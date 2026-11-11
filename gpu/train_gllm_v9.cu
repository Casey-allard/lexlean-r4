// Pure-Geometric CUDA Trainer & UORGLLM9 Exporter for UOR-R4-GLLM v9 (R4-38, rev 3).
//
// Charter Compliance (AGENTS.md):
//   - ZERO dense matrix multiplication (no GEMM / cublas / matmul).
//   - ZERO softmax, log-softmax, sigmoid, tanh, GELU, or exponential functions.
//   - ZERO ServedTurnMemory canned turn-ID tables (turn_memory_entries = 0).
//   - 100% bit-identical integer forward pass with crates/wasm/src/gllm_v9.rs:
//     * Deterministic Cl(0,6) byte-geometry vocabulary initialization across all 4,096 BPE tokens
//     * Cl(0,6) FWHT-64 integer butterflies + 256-entry discrete T-MAC Q12 LUTs
//     * 4th-order rational PRLL (R4-27) causal KV cache attention with 1/2/3-gram & anchor-shift heads
//     * 2-plane balanced-nonary [-4..+4] 81-state T-MAC activation LUT readout (ZERO multiplies!)
//     * Prompt-bounded causal KV pointer-copy with repeated-occurrence mass summation
//     * R4.Objective multi-margin hinge loss with staged LR decay & best-QAT checkpoint retention.

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <chrono>
#include <vector>
#include <string>
#include <fstream>
#include <algorithm>
#include <random>

#define CHECK_CUDA(call) do { \
    cudaError_t err = (call); \
    if (err != cudaSuccess) { \
        fprintf(stderr, "CUDA error at %s:%d: %s\n", __FILE__, __LINE__, cudaGetErrorString(err)); \
        exit(1); \
    } \
} while (0)

static constexpr int VOCAB_SIZE = 4096;
static constexpr int MAX_SEQ = 256;
static constexpr int MAX_D = 128;
static constexpr int MAX_L = 8;
static constexpr int MAX_H = 16;
static constexpr int MAX_G = 32;
static constexpr int QKV_OUT = 16;
static constexpr uint64_t PRLL_QUANTUM = 1000000ULL;
static constexpr int HINGE_MARGIN = 1200;

struct Sha256Ctx {
    uint32_t h[8];
    uint8_t block[64];
    size_t block_len;
    uint64_t total_bits;
};

static const uint32_t SHA256_K[64] = {
    0x428a2f98u,0x71374491u,0xb5c0fbcfu,0xe9b5dba5u,0x3956c25bu,0x59f111f1u,0x923f82a4u,0xab1c5ed5u,
    0xd807aa98u,0x12835b01u,0x243185beu,0x550c7dc3u,0x72be5d74u,0x80deb1feu,0x9bdc06a7u,0xc19bf174u,
    0xe49b69c1u,0xefbe4786u,0x0fc19dc6u,0x240ca1ccu,0x2de92c6fu,0x4a7484aau,0x5cb0a9dcu,0x76f988dau,
    0x983e5152u,0xa831c66du,0xb00327c8u,0xbf597fc7u,0xc6e00bf3u,0xd5a79147u,0x06ca6351u,0x14292967u,
    0x27b70a85u,0x2e1b2138u,0x4d2c6dfcu,0x53380d13u,0x650a7354u,0x766a0abbu,0x81c2c92eu,0x92722c85u,
    0xa2bfe8a1u,0xa81a664bu,0xc24b8b70u,0xc76c51a3u,0xd192e819u,0xd6990624u,0xf40e3585u,0x106aa070u,
    0x19a4c116u,0x1e376c08u,0x2748774cu,0x34b0bcb5u,0x391c0cb3u,0x4ed8aa4au,0x5b9cca4fu,0x682e6ff3u,
    0x748f82eeu,0x78a5636fu,0x84c87814u,0x8cc70208u,0x90befffau,0xa4506cebu,0xbef9a3f7u,0xc67178f2u
};

static inline uint32_t rotr32(uint32_t x, int n) { return (x >> n) | (x << (32 - n)); }

static void sha256_compress(uint32_t h[8], const uint8_t b[64]) {
    uint32_t w[64];
    for (int i = 0; i < 16; ++i) {
        w[i] = (uint32_t(b[4*i]) << 24) | (uint32_t(b[4*i+1]) << 16) | (uint32_t(b[4*i+2]) << 8) | uint32_t(b[4*i+3]);
    }
    for (int i = 16; i < 64; ++i) {
        uint32_t s0 = rotr32(w[i-15], 7) ^ rotr32(w[i-15], 18) ^ (w[i-15] >> 3);
        uint32_t s1 = rotr32(w[i-2], 17) ^ rotr32(w[i-2], 19) ^ (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    uint32_t a=h[0],bb=h[1],c=h[2],d=h[3],e=h[4],f=h[5],g=h[6],hh=h[7];
    for (int i = 0; i < 64; ++i) {
        uint32_t S1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
        uint32_t ch = (e & f) ^ ((~e) & g);
        uint32_t t1 = hh + S1 + ch + SHA256_K[i] + w[i];
        uint32_t S0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
        uint32_t maj = (a & bb) ^ (a & c) ^ (bb & c);
        uint32_t t2 = S0 + maj;
        hh = g; g = f; f = e; e = d + t1; d = c; c = bb; bb = a; a = t1 + t2;
    }
    h[0]+=a; h[1]+=bb; h[2]+=c; h[3]+=d; h[4]+=e; h[5]+=f; h[6]+=g; h[7]+=hh;
}

static void sha256_bytes(const uint8_t* data, size_t len, uint8_t out[32]) {
    Sha256Ctx ctx;
    ctx.h[0]=0x6a09e667u; ctx.h[1]=0xbb67ae85u; ctx.h[2]=0x3c6ef372u; ctx.h[3]=0xa54ff53au;
    ctx.h[4]=0x510e527fu; ctx.h[5]=0x9b05688cu; ctx.h[6]=0x1f83d9abu; ctx.h[7]=0x5be0cd19u;
    ctx.block_len = 0;
    ctx.total_bits = uint64_t(len) * 8ULL;
    for (size_t i = 0; i < len; ++i) {
        ctx.block[ctx.block_len++] = data[i];
        if (ctx.block_len == 64) { sha256_compress(ctx.h, ctx.block); ctx.block_len = 0; }
    }
    ctx.block[ctx.block_len++] = 0x80;
    if (ctx.block_len > 56) {
        while (ctx.block_len < 64) ctx.block[ctx.block_len++] = 0;
        sha256_compress(ctx.h, ctx.block);
        ctx.block_len = 0;
    }
    while (ctx.block_len < 56) ctx.block[ctx.block_len++] = 0;
    for (int i = 0; i < 8; ++i) ctx.block[56 + i] = uint8_t(ctx.total_bits >> (56 - 8 * i));
    sha256_compress(ctx.h, ctx.block);
    for (int i = 0; i < 8; ++i) {
        out[4*i] = uint8_t(ctx.h[i] >> 24);
        out[4*i+1] = uint8_t(ctx.h[i] >> 16);
        out[4*i+2] = uint8_t(ctx.h[i] >> 8);
        out[4*i+3] = uint8_t(ctx.h[i]);
    }
}

static std::string hex_of(const uint8_t* bytes, size_t n) {
    char buf[3];
    std::string s;
    s.reserve(n * 2);
    for (size_t i = 0; i < n; ++i) {
        snprintf(buf, sizeof(buf), "%02x", bytes[i]);
        s += buf;
    }
    return s;
}

__host__ __device__ static inline int16_t clamp_q12(int32_t v) {
    return v < -8192 ? int16_t(-8192) : (v > 8192 ? int16_t(8192) : int16_t(v));
}

__host__ __device__ static inline int32_t clamp_i32(int32_t v, int32_t lo, int32_t hi) {
    return v < lo ? lo : (v > hi ? hi : v);
}

__host__ __device__ static inline int32_t isign(int32_t x) {
    return (x > 0) - (x < 0);
}

__host__ __device__ static inline int8_t latent_to_trit(int32_t w) {
    return w > 96 ? int8_t(1) : (w < -96 ? int8_t(-1) : int8_t(0));
}

// Balanced-nonary [-4..+4] decomposition into (t_hi, t_lo) in {-1,0,+1}^2 so 3*t_hi + t_lo == q9!
__host__ __device__ static inline int8_t latent_to_q9(int32_t w) {
    int32_t num = w + (w >= 0 ? 128 : -128);
    int32_t q = num / 256;
    return int8_t(clamp_i32(q, -4, 4));
}

__host__ __device__ static inline void q9_to_trit_pair(int8_t q9, int8_t& t_hi, int8_t& t_lo) {
    int u = int(q9) + 4; // 0..8
    t_hi = int8_t((u / 3) - 1);
    t_lo = int8_t((u % 3) - 1);
}

__host__ __device__ static inline uint8_t pack_4trits(int8_t t0, int8_t t1, int8_t t2, int8_t t3) {
    return uint8_t((t0 + 1) + 3 * (t1 + 1) + 9 * (t2 + 1) + 27 * (t3 + 1));
}

__host__ __device__ static inline void pack_4q9(
    int32_t w0, int32_t w1, int32_t w2, int32_t w3,
    uint8_t& code_hi, uint8_t& code_lo
) {
    int8_t h0, l0, h1, l1, h2, l2, h3, l3;
    q9_to_trit_pair(latent_to_q9(w0), h0, l0);
    q9_to_trit_pair(latent_to_q9(w1), h1, l1);
    q9_to_trit_pair(latent_to_q9(w2), h2, l2);
    q9_to_trit_pair(latent_to_q9(w3), h3, l3);
    code_hi = pack_4trits(h0, h1, h2, h3);
    code_lo = pack_4trits(l0, l1, l2, l3);
}

// Pure addition/subtraction construction of 81-entry T-MAC activation lookup table (ZERO multiplies!)
__host__ __device__ static inline void build_tmac_act_lut81(const int16_t* x4, int32_t out81[81]) {
    int32_t a0[3] = {-int32_t(x4[0]), 0, int32_t(x4[0])};
    int32_t a1[3] = {-int32_t(x4[1]), 0, int32_t(x4[1])};
    int32_t a2[3] = {-int32_t(x4[2]), 0, int32_t(x4[2])};
    int32_t a3[3] = {-int32_t(x4[3]), 0, int32_t(x4[3])};
    int idx = 0;
    for (int c3 = 0; c3 < 3; ++c3) {
        for (int c2 = 0; c2 < 3; ++c2) {
            int32_t s23 = a3[c3] + a2[c2];
            for (int c1 = 0; c1 < 3; ++c1) {
                int32_t s123 = s23 + a1[c1];
                out81[idx + 0] = (s123 + a0[0]) >> 5;
                out81[idx + 1] = (s123 + a0[1]) >> 5;
                out81[idx + 2] = (s123 + a0[2]) >> 5;
                idx += 3;
            }
        }
    }
}

__host__ __device__ static inline uint8_t q2_level(int32_t v) {
    if (v < -1024) return 0;
    if (v < 0) return 1;
    if (v < 1024) return 2;
    return 3;
}

__host__ __device__ static inline uint8_t code_4ch(const int16_t* u4) {
    return uint8_t(q2_level(u4[0]) | (q2_level(u4[1]) << 2) | (q2_level(u4[2]) << 4) | (q2_level(u4[3]) << 6));
}

__host__ __device__ static inline int16_t icosian_hinge(int32_t v) {
    int32_t y = (v < -2048) ? ((v >> 2) - 1536) : v;
    return clamp_q12(y);
}

__host__ __device__ static inline void fwht64_block(const int16_t* in64, int16_t* out64) {
    int32_t u[64];
    for (int i = 0; i < 64; ++i) u[i] = in64[i];
    for (int len = 1; len < 64; len <<= 1) {
        for (int i = 0; i < 64; i += (len << 1)) {
            for (int j = 0; j < len; ++j) {
                int32_t a = u[i + j];
                int32_t b = u[i + j + len];
                u[i + j] = a + b;
                u[i + j + len] = a - b;
            }
        }
    }
    for (int i = 0; i < 64; ++i) {
        out64[i] = clamp_q12(u[i] >> 3);
    }
}

__host__ __device__ static inline int popcnt64_dev(uint64_t v) {
#ifdef __CUDA_ARCH__
    return __popcll((unsigned long long)v);
#else
    return __builtin_popcountll((unsigned long long)v);
#endif
}

__host__ __device__ static inline int popcnt32_dev(uint32_t v) {
#ifdef __CUDA_ARCH__
    return __popc(v);
#else
    return __builtin_popcount(v);
#endif
}

__host__ __device__ static inline uint64_t pack_sign64(const int16_t* v64) {
    uint64_t bits = 0;
    for (int i = 0; i < 64; ++i) {
        if (v64[i] >= 0) bits |= (1ULL << i);
    }
    return bits;
}

__host__ __device__ static inline int16_t quant_sim_q12(int32_t w_q12, int use_qat, uint16_t scale_q8) {
    int32_t c = clamp_i32(w_q12, -8192, 8192);
    if (!use_qat) return int16_t(c);
    int32_t sc = (scale_q8 < 16) ? 16 : int32_t(scale_q8);
    int32_t q_i8 = clamp_i32(((c << 4) + (c >= 0 ? (sc >> 1) : -(sc >> 1))) / sc, -127, 127);
    return clamp_q12((q_i8 * sc) >> 4);
}

// 4th-order rational PRLL (R4-27) attention weight: ZERO exp, ZERO softmax, ZERO float!
__host__ __device__ static inline uint64_t prll_weight_order4(int32_t tau, int32_t d_tot) {
    uint64_t denom = uint64_t(tau < 256 ? 256 : tau) + uint64_t((d_tot > 0 ? d_tot : 0) >> 1);
    uint64_t q1 = 1073741824ULL / (denom > 0 ? denom : 1ULL);
    uint64_t q2 = (q1 * q1) >> 20;
    return ((q2 * q2) >> 16) + 1ULL;
}

struct SequenceSample {
    uint16_t prompt_len;
    uint16_t reply_len;
    uint16_t tokens[MAX_SEQ];
    uint8_t copy_mask[MAX_SEQ]; // [total_len]: 1 if tokens[idx] is an in-context copied span
};

struct BlockScratch {
    int16_t x_cur[MAX_SEQ][MAX_D];
    int16_t q_vec[MAX_L][MAX_SEQ][MAX_D];
    int16_t k_vec[MAX_L][MAX_SEQ][MAX_D];
    int16_t v_vec[MAX_L][MAX_SEQ][MAX_D];
    uint64_t q_sk0[MAX_L][MAX_SEQ];
    uint64_t k_sk0[MAX_L][MAX_SEQ];
    uint8_t qkv_codes[MAX_L][MAX_SEQ][MAX_G];
    uint8_t k_ctx_codes[MAX_L][MAX_SEQ][MAX_G];
    uint8_t ffn_codes[MAX_L][MAX_SEQ][MAX_G];
    uint32_t top_prll_w[MAX_H][MAX_SEQ];
    int32_t top_gate_val[MAX_H];
    uint8_t pos_idx[MAX_SEQ];
};

struct DeviceModel {
    int d_model;
    int n_layers;
    int n_heads;
    int lut_groups;

    int32_t* e_tok;         // [VOCAB_SIZE * D]
    int32_t* e_pos;         // [256 * D]
    int32_t* q_lut;         // [L * G * 256 * QKV_OUT]
    int32_t* k_lut;         // [L * G * 256 * QKV_OUT]
    int32_t* v_lut;         // [L * G * 256 * QKV_OUT]
    int32_t* ffn_lut;       // [L * G * 256 * D]
    int32_t* w_tmac;        // [VOCAB_SIZE * D] latent 9-level weights -> quantized via latent_to_q9!
    int32_t* p_out;         // [VOCAB_SIZE * D]
    int32_t* b_out;         // [VOCAB_SIZE]
    uint16_t* head_tau;     // [L * H]
    uint16_t* head_slope;   // [L * H]
    uint8_t* head_shift;    // [L * H]
    int32_t* head_copy_b;   // [L * H]
    int32_t* head_copy_w;   // [L * H * D]
    int32_t* copy_tok_gate; // [VOCAB_SIZE]
    uint16_t* scales_q8;    // [3 + 4 * L]
};

struct DeviceGrads {
    int32_t* e_tok;
    int32_t* e_pos;
    int32_t* q_lut;
    int32_t* k_lut;
    int32_t* v_lut;
    int32_t* ffn_lut;
    int32_t* w_tmac;
    int32_t* p_out;
    int32_t* b_out;
    int32_t* head_copy_b;
    int32_t* head_copy_w;
    int32_t* copy_tok_gate;
};

__global__ void apply_momentum_step_kernel(
    int32_t* __restrict__ param,
    int32_t* __restrict__ mom,
    int32_t* __restrict__ grad,
    size_t n,
    int lr_q4,
    int32_t clamp_lo,
    int32_t clamp_hi
) {
    size_t i = size_t(blockIdx.x) * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int32_t g = grad[i];
    grad[i] = 0;
    int32_t g_bounded = (g == 0) ? 0 : (4 * isign(g) + clamp_i32(g / 2, -60, 60));
    int32_t m_prev = mom[i];
    int32_t m_new = (m_prev * 5 + g_bounded * 3) / 8; // truncates toward 0 -> never traps at -1!
    mom[i] = m_new;
    if (m_new != 0 || g != 0) {
        int32_t delta = (m_new * lr_q4 + (m_new > 0 ? 8 : (m_new < 0 ? -8 : 0))) / 16;
        if (delta == 0 && g != 0) delta = isign(g);
        param[i] = clamp_i32(param[i] + delta, clamp_lo, clamp_hi);
    }
}

__global__ void forward_and_grad_kernel(
    DeviceModel m,
    DeviceGrads gr,
    BlockScratch* scratches,
    const SequenceSample* __restrict__ samples,
    const int* __restrict__ batch_indices,
    int batch_size,
    int is_train,
    int use_qat,
    unsigned long long* __restrict__ out_loss_sum,
    unsigned int* __restrict__ out_correct_toks,
    unsigned int* __restrict__ out_total_toks
) {
    const int b_idx = blockIdx.x;
    if (b_idx >= batch_size) return;
    const int tid = threadIdx.x; // 0..255

    const SequenceSample& seq = samples[batch_indices[b_idx]];
    const int p_len = seq.prompt_len;
    const int r_len = seq.reply_len;
    const int total_len = p_len + r_len;
    if (total_len < 2 || total_len > MAX_SEQ) return;

    BlockScratch& sc = scratches[b_idx];
    const int D = m.d_model;
    const int L = m.n_layers;
    const int H = m.n_heads;
    const int G = m.lut_groups;

    const uint16_t tok_scale = m.scales_q8[0];
    const uint16_t pos_scale = m.scales_q8[1];
    const uint16_t proto_scale = m.scales_q8[2];

    // Step 1: Parallel Embedding + Prompt & Span-Length-Invariant Cl(0,6) Structural Phase Rotation
    for (int t = tid; t < total_len; t += 256) {
        uint32_t tok = seq.tokens[t];
        int pos_idx = 0;
        if (t >= p_len - 1) {
            int sp = 1;
            for (int u = p_len; u <= t; ++u) {
                if (seq.copy_mask[u - 1] == 0) ++sp;
            }
            pos_idx = clamp_i32(sp, 0, 255);
        }
        sc.pos_idx[t] = uint8_t(pos_idx);
        uint32_t spin_t = uint32_t((pos_idx * 7 + int(tok & 63)) & 63);
        for (int c = 0; c < D; ++c) {
            int perm_c = c ^ (pos_idx & 7);
            int parity = popcnt32_dev(uint32_t(c & 63) & spin_t) & 1;
            int16_t et = quant_sim_q12(m.e_tok[tok * D + c], use_qat, tok_scale);
            int16_t ep = (pos_idx == 0) ? int16_t(0) : quant_sim_q12(m.e_pos[pos_idx * D + perm_c], use_qat, pos_scale);
            int32_t rot = (parity == 0) ? int32_t(ep) : -int32_t(ep);
            sc.x_cur[t][c] = clamp_q12(int32_t(et) + rot);
        }
    }
    __syncthreads();

    // Step 2: Execute L Causal Geometric Layers
    for (int l = 0; l < L; ++l) {
        const uint16_t q_sc = m.scales_q8[3 + 4 * l + 0];
        const uint16_t k_sc = m.scales_q8[3 + 4 * l + 1];
        const uint16_t v_sc = m.scales_q8[3 + 4 * l + 2];
        const uint16_t ffn_sc = m.scales_q8[3 + 4 * l + 3];

        for (int t = tid; t < total_len; t += 256) {
            int16_t u_blk[MAX_D];
            int16_t k_in[MAX_D];
            int16_t u_k[MAX_D];
            for (int c = 0; c < D; ++c) {
                int16_t prev_x = (t > 0) ? sc.x_cur[t - 1][c ^ 1] : int16_t(0);
                k_in[c] = clamp_q12(((int32_t(sc.x_cur[t][c]) * 3) + int32_t(prev_x)) >> 2);
            }
            for (int blk = 0; blk < D / 64; ++blk) {
                fwht64_block(&sc.x_cur[t][blk * 64], &u_blk[blk * 64]);
                fwht64_block(&k_in[blk * 64], &u_k[blk * 64]);
            }
            int32_t q_acc[MAX_D] = {0};
            int32_t k_acc[MAX_D] = {0};
            int32_t v_acc[MAX_D] = {0};

            const size_t qkv_layer_base = size_t(l) * G * 256 * QKV_OUT;
            for (int g = 0; g < G; ++g) {
                uint8_t q_code = code_4ch(&u_blk[g * 4]);
                uint8_t k_code = code_4ch(&u_k[g * 4]);
                sc.qkv_codes[l][t][g] = q_code;
                sc.k_ctx_codes[l][t][g] = k_code;
                int base_c = (g * 4) % D;
                size_t qv_row = qkv_layer_base + (size_t(g) * 256 + q_code) * QKV_OUT;
                size_t k_row = qkv_layer_base + (size_t(g) * 256 + k_code) * QKV_OUT;
                for (int o = 0; o < QKV_OUT; ++o) {
                    int dst = (base_c + o) % D;
                    q_acc[dst] += quant_sim_q12(m.q_lut[qv_row + o], use_qat, q_sc);
                    k_acc[dst] += quant_sim_q12(m.k_lut[k_row + o], use_qat, k_sc);
                    v_acc[dst] += quant_sim_q12(m.v_lut[qv_row + o], use_qat, v_sc);
                }
            }
            for (int c = 0; c < D; ++c) {
                sc.q_vec[l][t][c] = clamp_q12((q_acc[c] >> 2) + (int32_t(u_blk[c]) >> 1));
                sc.k_vec[l][t][c] = clamp_q12((k_acc[c] >> 2) + (int32_t(u_k[c]) >> 1));
                sc.v_vec[l][t][c] = clamp_q12((v_acc[c] >> 2) + (int32_t(sc.x_cur[t][c]) >> 1));
            }
            sc.q_sk0[l][t] = pack_sign64(&sc.q_vec[l][t][0]);
            sc.k_sk0[l][t] = pack_sign64(&sc.k_vec[l][t][0]);
        }
        __syncthreads();

        for (int t = tid; t < total_len; t += 256) {
            int16_t attn_out[MAX_D];
            for (int h = 0; h < H; ++h) {
                const int c_off = h * 8;
                const int sh = int(m.head_shift[l * H + h]);
                const int32_t tau = int32_t(m.head_tau[l * H + h] < 256 ? 256 : m.head_tau[l * H + h]);
                const int32_t slope = int32_t(m.head_slope[l * H + h]);
                const uint8_t q_byte = uint8_t((sc.q_sk0[l][t] >> ((h & 7) * 8)) & 0xFFULL);

                uint64_t raw_w[MAX_SEQ];
                uint64_t sum_raw = 0;
                int best_s = 0;
                uint64_t best_w = 0;

                for (int s = 0; s <= t; ++s) {
                    int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
                    int ks = (s >= ks_shift) ? (s - ks_shift) : 0;
                    int32_t d_l1 = 0;
                    #pragma unroll
                    for (int j = 0; j < 8; ++j) {
                        int32_t diff = int32_t(sc.q_vec[l][t][c_off + j]) - int32_t(sc.k_vec[l][ks][c_off + j]);
                        d_l1 += (diff < 0 ? -diff : diff);
                    }
                    uint8_t k_byte = uint8_t((sc.k_sk0[l][ks] >> ((h & 7) * 8)) & 0xFFULL);
                    int32_t d_ham = popcnt32_dev(uint32_t(q_byte ^ k_byte));
                    bool t_is_anchor = (clamp_i32(m.copy_tok_gate[seq.tokens[t]], 0, 255) >= 208);
                    int32_t d_ind = 0;
                    if (sh == 1) {
                        if (s == t) {
                            d_ind = 4096;
                        } else if (s > 1 && t > 0 && s < t && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                            d_ind = 0;
                        } else if (s > 0 && s < t && t_is_anchor && seq.tokens[s - 1] == seq.tokens[t]) {
                            d_ind = 256;
                        } else {
                            d_ind = 65536;
                        }
                    } else if (sh == 2) {
                        if (s == t) {
                            d_ind = 4096;
                        } else if (s > 2 && t > 1 && s < t && seq.tokens[s - 3] == seq.tokens[t - 2] && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                            d_ind = 0;
                        } else if (s > 1 && t > 0 && s < t && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                            d_ind = 256;
                        } else if (s > 0 && s < t && t_is_anchor && seq.tokens[s - 1] == seq.tokens[t]) {
                            d_ind = 512;
                        } else {
                            d_ind = 65536;
                        }
                    } else if (sh == 3) {
                        if (s == t) {
                            d_ind = 8192;
                        } else {
                            d_ind = (s > 0 && s < t && clamp_i32(m.copy_tok_gate[seq.tokens[s]], 0, 255) >= 208) ? 0 : 65536;
                        }
                    } else if (slope <= 4 && t > 0 && s == t) {
                        d_ind = 4096;
                    }
                    int32_t d_geom = (sh == 1 || sh == 2) ? ((d_l1 >> 3) + 32 * d_ham) : (d_l1 + 256 * d_ham);
                    int32_t d_tot = d_geom + slope * (t - s) + d_ind;
                    uint64_t w = prll_weight_order4(tau, d_tot);
                    raw_w[s] = w;
                    sum_raw += w;
                    if (w > best_w) { best_w = w; best_s = s; }
                }

                int64_t acc_v[8] = {0,0,0,0,0,0,0,0};
                uint64_t mass_acc = 0;
                uint32_t prll_w_local[MAX_SEQ];
                for (int s = 0; s <= t; ++s) {
                    uint32_t wp = uint32_t((raw_w[s] * PRLL_QUANTUM) / sum_raw);
                    prll_w_local[s] = wp;
                    mass_acc += wp;
                }
                prll_w_local[best_s] += uint32_t(PRLL_QUANTUM - mass_acc);

                for (int s = 0; s <= t; ++s) {
                    int64_t wp = int64_t(prll_w_local[s]);
                    #pragma unroll
                    for (int j = 0; j < 8; ++j) {
                        acc_v[j] += wp * int64_t(sc.v_vec[l][s][c_off + j]);
                    }
                }
                #pragma unroll
                for (int j = 0; j < 8; ++j) {
                    attn_out[c_off + j] = clamp_q12(int32_t(acc_v[j] / int64_t(PRLL_QUANTUM)));
                }
            }

            int16_t x_mid[MAX_D];
            for (int c = 0; c < D; ++c) {
                x_mid[c] = clamp_q12(int32_t(sc.x_cur[t][c]) + (int32_t(attn_out[c]) >> 1));
            }
            int16_t u_ffn[MAX_D];
            for (int blk = 0; blk < D / 64; ++blk) {
                fwht64_block(&x_mid[blk * 64], &u_ffn[blk * 64]);
            }
            int32_t ffn_acc[MAX_D] = {0};
            const size_t ffn_layer_base = size_t(l) * G * 256 * D;
            for (int g = 0; g < G; ++g) {
                uint8_t code = code_4ch(&u_ffn[g * 4]);
                sc.ffn_codes[l][t][g] = code;
                size_t row = ffn_layer_base + (size_t(g) * 256 + code) * D;
                for (int c = 0; c < D; ++c) {
                    ffn_acc[c] += quant_sim_q12(m.ffn_lut[row + c], use_qat, ffn_sc);
                }
            }
            for (int c = 0; c < D; ++c) {
                int16_t act = icosian_hinge(ffn_acc[c] >> 4);
                sc.x_cur[t][c] = clamp_q12(int32_t(x_mid[c]) + int32_t(act));
            }
        }
        __syncthreads();
    }

    // Step 3: 2-Plane 9-Level T-MAC LUT Readout + Prompt-Bounded Pointer-Copy + R4.Objective Hinge Loss
    __shared__ int32_t s_act_lut[MAX_G][81];
    __shared__ int32_t s_copy_logits[VOCAB_SIZE];
    __shared__ int32_t s_best_neg_score[256];
    __shared__ int s_best_neg_id[256];
    __shared__ int32_t s_target_score;
    __shared__ int32_t s_grad_h[MAX_D];

    const int copy_limit = (p_len > 2) ? (p_len - 2) : p_len;

    for (int t = p_len - 1; t < total_len - 1; ++t) {
        const int target = int(seq.tokens[t + 1]);
        const int is_copy_pos = int(seq.copy_mask[t]); // note: r_copy[t - (p_len - 1)] stored at copy_mask[t]!
        const bool prev_was_copied = (t > p_len - 1) && (seq.copy_mask[t - 1] != 0);

        if (tid < G) {
            build_tmac_act_lut81(&sc.x_cur[t][tid * 4], s_act_lut[tid]);
        }
        for (int v = tid; v < VOCAB_SIZE; v += 256) {
            s_copy_logits[v] = 0;
        }
        __syncthreads();

        const int top_l = L - 1;
        if (tid < H) {
            int h = tid;
            int32_t gate_sum = clamp_i32(m.head_copy_b[top_l * H + h], -8192, 8192);
            const int32_t* cw_row = &m.head_copy_w[(top_l * H + h) * D];
            for (int g = 0; g < G; ++g) {
                uint8_t pcode = pack_4trits(
                    latent_to_trit(cw_row[4*g + 0]),
                    latent_to_trit(cw_row[4*g + 1]),
                    latent_to_trit(cw_row[4*g + 2]),
                    latent_to_trit(cw_row[4*g + 3])
                );
                gate_sum += s_act_lut[g][pcode];
            }
            const int c_off = h * 8;
            const int sh = int(m.head_shift[top_l * H + h]);
            const uint32_t cur_tok = seq.tokens[t];
            const bool fence_or_nl = (cur_tok == 66 || cur_tok == 201 || cur_tok == 29);
            int32_t copy_gain = (sh == 3 && fence_or_nl) ? 0
                : (((sh == 1 || sh == 2) && !prev_was_copied) ? 0 : clamp_i32(gate_sum, 0, 8192));
            sc.top_gate_val[h] = gate_sum;
            const int32_t tau = int32_t(m.head_tau[top_l * H + h] < 256 ? 256 : m.head_tau[top_l * H + h]);
            const int32_t slope = int32_t(m.head_slope[top_l * H + h]);
            const uint8_t q_byte = uint8_t((sc.q_sk0[top_l][t] >> ((h & 7) * 8)) & 0xFFULL);

            uint64_t raw_w[MAX_SEQ];
            uint64_t sum_raw = 0;
            int best_s = 0;
            uint64_t best_w = 0;
            for (int s = 0; s <= t; ++s) {
                int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
                int ks = (s >= ks_shift) ? (s - ks_shift) : 0;
                int32_t d_l1 = 0;
                for (int j = 0; j < 8; ++j) {
                    int32_t diff = int32_t(sc.q_vec[top_l][t][c_off + j]) - int32_t(sc.k_vec[top_l][ks][c_off + j]);
                    d_l1 += (diff < 0 ? -diff : diff);
                }
                uint8_t k_byte = uint8_t((sc.k_sk0[top_l][ks] >> ((h & 7) * 8)) & 0xFFULL);
                int32_t d_ham = popcnt32_dev(uint32_t(q_byte ^ k_byte));
                bool t_is_anchor = (clamp_i32(m.copy_tok_gate[seq.tokens[t]], 0, 255) >= 208);
                int32_t d_ind = 0;
                if (sh == 1) {
                    if (s == t) {
                        d_ind = 4096;
                    } else if (s > 1 && t > 0 && s < t && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                        d_ind = 0;
                    } else if (s > 0 && s < t && t_is_anchor && seq.tokens[s - 1] == seq.tokens[t]) {
                        d_ind = 256;
                    } else {
                        d_ind = 65536;
                    }
                } else if (sh == 2) {
                    if (s == t) {
                        d_ind = 4096;
                    } else if (s > 2 && t > 1 && s < t && seq.tokens[s - 3] == seq.tokens[t - 2] && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                        d_ind = 0;
                    } else if (s > 1 && t > 0 && s < t && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]) {
                        d_ind = 256;
                    } else if (s > 0 && s < t && t_is_anchor && seq.tokens[s - 1] == seq.tokens[t]) {
                        d_ind = 512;
                    } else {
                        d_ind = 65536;
                    }
                } else if (sh == 3) {
                    if (s == t) {
                        d_ind = 8192;
                    } else {
                        d_ind = (s > 0 && s < t && clamp_i32(m.copy_tok_gate[seq.tokens[s]], 0, 255) >= 208) ? 0 : 65536;
                    }
                } else if (slope <= 4 && t > 0 && s == t) {
                    d_ind = 4096;
                }
                int32_t d_geom = (sh == 1 || sh == 2) ? ((d_l1 >> 3) + 32 * d_ham) : (d_l1 + 256 * d_ham);
                int32_t d_tot = d_geom + slope * (t - s) + d_ind;
                uint64_t w = prll_weight_order4(tau, d_tot);
                raw_w[s] = w;
                sum_raw += w;
                if (w > best_w) { best_w = w; best_s = s; }
            }
            uint64_t mass_acc = 0;
            for (int s = 0; s <= t; ++s) {
                uint32_t wp = uint32_t((raw_w[s] * PRLL_QUANTUM) / sum_raw);
                sc.top_prll_w[h][s] = wp;
                mass_acc += wp;
            }
            sc.top_prll_w[h][best_s] += uint32_t(PRLL_QUANTUM - mass_acc);

            if (copy_gain > 0) {
                for (int s = 1; s < copy_limit; ++s) {
                    int tok_s = int(seq.tokens[s]);
                    bool seen_before = false;
                    for (int sp = 1; sp < s; ++sp) {
                        if (int(seq.tokens[sp]) == tok_s) { seen_before = true; break; }
                    }
                    if (seen_before) continue;
                    uint64_t tot_wp = 0;
                    for (int s2 = s; s2 < copy_limit; ++s2) {
                        if (int(seq.tokens[s2]) == tok_s) tot_wp += sc.top_prll_w[h][s2];
                    }
                    int32_t tok_allow = clamp_i32(m.copy_tok_gate[tok_s], 0, 255);
                    int32_t cand = clamp_i32(
                        int32_t((int64_t(tot_wp) * int64_t(copy_gain) * int64_t(tok_allow)) >> 25),
                        0,
                        12288
                    );
                    if (cand > 0) atomicMax(&s_copy_logits[tok_s], cand);
                }
            }
        }
        __syncthreads();

        const uint64_t h_sk0 = pack_sign64(&sc.x_cur[t][0]);
        const uint64_t h_sk1 = (D == 128) ? pack_sign64(&sc.x_cur[t][64]) : 0ULL;
        const int d_shift = (D >= 128) ? 1 : 0;

        int32_t local_best_neg = -2000000000;
        int local_best_neg_id = 0;

        for (int v = tid; v < VOCAB_SIZE; v += 256) {
            if (v == 0 || v == 2) continue;
            // 2-plane balanced-nonary [-4..+4] T-MAC LUT readout (3*hi + lo via shifts/adds, ZERO multiplies!)
            int32_t tmac_sum = 0;
            const int32_t* tw = &m.w_tmac[v * D];
            for (int g = 0; g < G; ++g) {
                uint8_t code_hi, code_lo;
                pack_4q9(tw[4*g + 0], tw[4*g + 1], tw[4*g + 2], tw[4*g + 3], code_hi, code_lo);
                int32_t v_hi = s_act_lut[g][code_hi];
                int32_t v_lo = s_act_lut[g][code_lo];
                tmac_sum += (v_hi << 1) + v_hi + v_lo;
            }

            int16_t p_vec[MAX_D];
            int32_t l1 = 0;
            for (int c = 0; c < D; ++c) {
                int16_t pv = quant_sim_q12(m.p_out[v * D + c], use_qat, proto_scale);
                p_vec[c] = pv;
                int32_t diff = int32_t(sc.x_cur[t][c]) - int32_t(pv);
                l1 += (diff < 0 ? -diff : diff);
            }
            uint64_t p_sk0 = pack_sign64(&p_vec[0]);
            int ham = popcnt64_dev(h_sk0 ^ p_sk0);
            if (D == 128) {
                uint64_t p_sk1 = pack_sign64(&p_vec[64]);
                ham += popcnt64_dev(h_sk1 ^ p_sk1);
            }

            int32_t b_val = clamp_i32(m.b_out[v], -2048, 2048);
            int32_t score = (tmac_sum >> d_shift) + (b_val * 2) - ((ham * 24) >> d_shift) - (l1 >> (5 + d_shift)) + s_copy_logits[v];

            if (v == target) {
                s_target_score = score;
            } else if (score > local_best_neg) {
                local_best_neg = score;
                local_best_neg_id = v;
            }
        }

        s_best_neg_score[tid] = local_best_neg;
        s_best_neg_id[tid] = local_best_neg_id;
        __syncthreads();

        if (tid == 0) {
            int32_t best_neg = s_best_neg_score[0];
            int neg_id = s_best_neg_id[0];
            for (int i = 1; i < 256; ++i) {
                if (s_best_neg_score[i] > best_neg) {
                    best_neg = s_best_neg_score[i];
                    neg_id = s_best_neg_id[i];
                }
            }
            s_best_neg_score[0] = best_neg;
            s_best_neg_id[0] = neg_id;

            atomicAdd(out_total_toks, 1u);
            if (s_target_score > best_neg) {
                atomicAdd(out_correct_toks, 1u);
            }
            int32_t loss = HINGE_MARGIN - (s_target_score - best_neg);
            if (loss > 0) {
                atomicAdd(out_loss_sum, (unsigned long long)loss);
            }
        }
        __syncthreads();

        if (!is_train) continue;
        const int hard_neg = s_best_neg_id[0];
        const bool is_span_start = (is_copy_pos != 0) && (t == p_len - 1 || seq.copy_mask[t - 1] == 0);
        const bool is_copy_cont = (is_copy_pos != 0) && !is_span_start;
        const int gate3_lo = (D == 128) ? 80 : 40;
        const int gate12_lo = (D == 128) ? 104 : 52;

        // 1. Always-On Causal KV Pointer-Copy & Structural Anchor-Shift Q/K Alignment!
        if (tid < H) {
            int h = tid;
            const int sh = int(m.head_shift[top_l * H + h]);
            if (sh > 0) {
                const int ks_shift = (sh == 2) ? 2 : 1;
                const int c_off = h * 8;
                const size_t qk_base = size_t(top_l) * G * 256 * QKV_OUT;

                int match_s = -1;
                int neg_s = -1;
                uint32_t best_neg_w = 0;

                if (sh == 3 && is_span_start) {
                    for (int s = 1; s < copy_limit; ++s) {
                        bool anchor_ok = (clamp_i32(m.copy_tok_gate[seq.tokens[s]], 0, 255) >= 208);
                        if (!anchor_ok) continue;
                        if (int(seq.tokens[s]) == target) {
                            if (match_s < 0) match_s = s; // Lock onto invariant 1st syntactic anchor!
                        } else if (sc.top_prll_w[h][s] >= best_neg_w) {
                            best_neg_w = sc.top_prll_w[h][s];
                            neg_s = s;
                        }
                    }
                } else if ((sh == 1 || sh == 2) && is_copy_cont) {
                    bool t_is_anchor = (clamp_i32(m.copy_tok_gate[seq.tokens[t]], 0, 255) >= 208);
                    bool have_2gram = false;
                    for (int s = 1; s < copy_limit; ++s) {
                        bool is_2g = (s > 1 && t > 0 && seq.tokens[s - 2] == seq.tokens[t - 1] && seq.tokens[s - 1] == seq.tokens[t]);
                        bool mode_ok = is_2g || (t_is_anchor && seq.tokens[s - 1] == seq.tokens[t]);
                        if (int(seq.tokens[s]) == target && mode_ok) {
                            if (is_2g && !have_2gram) {
                                match_s = s;
                                have_2gram = true;
                            } else if (match_s < 0) {
                                match_s = s;
                            }
                        } else if (int(seq.tokens[s]) != target && sc.top_prll_w[h][s] >= best_neg_w) {
                            best_neg_w = sc.top_prll_w[h][s];
                            neg_s = s;
                        }
                    }
                }

                if (match_s >= 0) {
                    uint64_t target_tot_wp = 0;
                    for (int s2 = 1; s2 < copy_limit; ++s2) {
                        if (int(seq.tokens[s2]) == target) target_tot_wp += sc.top_prll_w[h][s2];
                    }
                    if (target_tot_wp < 820000ULL && neg_s >= 0) {
                        int ks_pos = (match_s >= ks_shift) ? (match_s - ks_shift) : 0;
                        int ks_neg = (neg_s >= ks_shift) ? (neg_s - ks_shift) : 0;
                        for (int j = 0; j < 8; ++j) {
                            int ch = c_off + j;
                            int32_t qv = int32_t(sc.q_vec[top_l][t][ch]);
                            int32_t kp = int32_t(sc.k_vec[top_l][ks_pos][ch]);
                            int32_t kn = int32_t(sc.k_vec[top_l][ks_neg][ch]);
                            for (int grp_k = 0; grp_k < 4; ++grp_k) {
                                int g_src = ((ch / 4) + G - grp_k) % G;
                                int o_src = (ch + D - g_src * 4) % D;
                                if (o_src < QKV_OUT) {
                                    uint8_t qc = sc.qkv_codes[top_l][t][g_src];
                                    uint8_t kcp = sc.k_ctx_codes[top_l][ks_pos][g_src];
                                    uint8_t kcn = sc.k_ctx_codes[top_l][ks_neg][g_src];
                                    atomicAdd(&gr.q_lut[qk_base + (size_t(g_src) * 256 + qc) * QKV_OUT + o_src], 6 * (isign(kp - qv) - isign(kn - qv)));
                                    atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcp) * QKV_OUT + o_src], 6 * isign(qv - kp));
                                    atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcn) * QKV_OUT + o_src], -6 * isign(qv - kn));
                                }
                            }
                            if (top_l > 0) {
                                int32_t dir_q = isign(kp - kn);
                                int32_t dir_kp = isign(qv - kp);
                                int32_t dir_kn = -isign(qv - kn);
                                const size_t prev_ffn = size_t(top_l - 1) * G * 256 * D;
                                for (int g_f = 0; g_f < G; ++g_f) {
                                    uint8_t fc_t = sc.ffn_codes[top_l - 1][t][g_f];
                                    uint8_t fc_kp = sc.ffn_codes[top_l - 1][ks_pos][g_f];
                                    uint8_t fc_kn = sc.ffn_codes[top_l - 1][ks_neg][g_f];
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_t) * D + ch], 2 * dir_q);
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_kp) * D + ch], dir_kp);
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_kn) * D + ch], dir_kn);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Asymmetric Orthogonal Gate-Carrier Subspace Backpropagation (`[gate3_lo..gate12_lo)` and `[gate12_lo..D)`)!
        // - `gate3` (`sh == 3`, span-start): CLOSED (`< 0`) by default on unseen tokens; OPEN (`>= g3_open_tgt`) on seen syntax triggers.
        // - `gate12` (`sh == 1, 2`, continuation): OPEN by default on unseen sub-tokens; CLOSED (`<= -300`) on seen non-continuation tokens.
        const uint32_t cur_tok = seq.tokens[t];
        const bool fence_or_nl = (cur_tok == 66 || cur_tok == 201 || cur_tok == 29);
        const int32_t g3_open_tgt = (D == 128) ? 3800 : 1200;
        const int32_t g12_open_tgt = (D == 128) ? 3400 : 1800;
        const int32_t g3_val = sc.top_gate_val[1]; // `h = 1` has `sh == 3`
        const int32_t g1_val = sc.top_gate_val[3]; // `h = 3` has `sh == 1`
        const int32_t g3_grad = is_span_start ? ((g3_val < g3_open_tgt) ? 32 : 0) : ((!fence_or_nl && g3_val > -300) ? -2 : 0);
        const int32_t g1_grad = is_copy_cont
            ? ((g1_val < g12_open_tgt) ? 14 : 0)
            : (prev_was_copied ? ((g1_val > -400) ? -10 : 0) : ((g1_val > 1800) ? -1 : 0));
        const bool readout_violated = (s_target_score - s_best_neg_score[0] < HINGE_MARGIN);

        if (!readout_violated && g3_grad == 0 && g1_grad == 0) {
            __syncthreads();
            continue;
        }

        const bool suppress_noncopy_neg = readout_violated && (
            (!is_copy_pos && s_copy_logits[hard_neg] < 512)
            || (is_copy_pos && (s_best_neg_score[0] - s_copy_logits[hard_neg]) > 2800)
        );

        // Clean Separation of Readout Vocabulary (`0..gate3_lo`) vs Gate Carriers (`gate3_lo..D`)!
        if (tid == 0 && readout_violated) {
            if (!is_copy_pos) {
                atomicAdd(&gr.b_out[target], 2);
            } else if (m.b_out[target] < -256) {
                atomicAdd(&gr.b_out[target], 2);
            }
            if (suppress_noncopy_neg && (m.copy_tok_gate[hard_neg] < 128 || m.b_out[hard_neg] > -256)) {
                atomicAdd(&gr.b_out[hard_neg], -2);
            }
        }

        if (tid < D) {
            int c = tid;
            if (c < gate3_lo) {
                if (readout_violated) {
                    int32_t x_val = int32_t(sc.x_cur[t][c]);
                    int32_t x_dir = clamp_i32(x_val >> 9, -8, 8);

                    if (!is_copy_pos) {
                        atomicAdd(&gr.w_tmac[target * D + c], x_dir);
                        int32_t pt = clamp_i32(m.p_out[target * D + c], -8192, 8192);
                        atomicAdd(&gr.p_out[target * D + c], isign(x_val - pt));
                    }
                    if (suppress_noncopy_neg) {
                        atomicAdd(&gr.w_tmac[hard_neg * D + c], -x_dir);
                        int32_t pn = clamp_i32(m.p_out[hard_neg * D + c], -8192, 8192);
                        atomicAdd(&gr.p_out[hard_neg * D + c], -isign(x_val - pn));
                    }

                    int8_t q9_pos = is_copy_pos ? int8_t(0) : latent_to_q9(m.w_tmac[target * D + c]);
                    int8_t q9_neg = suppress_noncopy_neg ? latent_to_q9(m.w_tmac[hard_neg * D + c]) : int8_t(0);
                    int32_t pt = clamp_i32(m.p_out[target * D + c], -8192, 8192);
                    int32_t pn = clamp_i32(m.p_out[hard_neg * D + c], -8192, 8192);
                    s_grad_h[c] = int32_t(q9_pos - q9_neg)
                        + (is_copy_pos ? 0 : isign(pt - x_val))
                        - (suppress_noncopy_neg ? isign(pn - x_val) : 0);
                } else {
                    s_grad_h[c] = 0;
                }
            } else if (c < gate12_lo) {
                s_grad_h[c] = g3_grad;
            } else {
                s_grad_h[c] = g1_grad;
            }
        }
        __syncthreads();

        // Backpropagate s_grad_h through FFN_lut & V_lut (gate channels focus cleanly on top 2 layers!)
        for (int l = L - 1; l >= 0; --l) {
            int scale_l = (l == L - 1) ? 2 : 1;
            const size_t ffn_layer_base = size_t(l) * G * 256 * D;
            for (int idx = tid; idx < G * D; idx += 256) {
                int g = idx / D;
                int c = idx % D;
                if (c >= gate3_lo && l < L - 2) continue;
                int32_t gh = s_grad_h[c] * scale_l;
                if (gh != 0) {
                    uint8_t code = sc.ffn_codes[l][t][g];
                    atomicAdd(&gr.ffn_lut[ffn_layer_base + (size_t(g) * 256 + code) * D + c], gh);
                }
            }
            const size_t v_layer_base = size_t(l) * G * 256 * QKV_OUT;
            if (tid < G * QKV_OUT) {
                int g = tid / QKV_OUT;
                int o = tid % QKV_OUT;
                int dst = ((g * 4) + o) % D;
                if (dst >= gate3_lo && l < L - 2) continue;
                int32_t gh = s_grad_h[dst];
                if (gh != 0) {
                    uint8_t q_code = sc.qkv_codes[l][t][g];
                    atomicAdd(&gr.v_lut[v_layer_base + (size_t(g) * 256 + q_code) * QKV_OUT + o], gh);
                    if (t > 0) {
                        uint8_t prev_code = sc.qkv_codes[l][t - 1][g];
                        atomicAdd(&gr.v_lut[v_layer_base + (size_t(g) * 256 + prev_code) * QKV_OUT + o], gh);
                    }
                    if (l == top_l && dst < gate3_lo) {
                        int h_dst = dst / 8;
                        for (int s = 1; s < copy_limit; ++s) {
                            uint32_t wp = sc.top_prll_w[h_dst][s];
                            if (wp >= 25000) {
                                int32_t w_scale = int32_t((wp + 32768u) >> 16);
                                if (w_scale < 1) w_scale = 1;
                                uint8_t s_code = sc.qkv_codes[l][s][g];
                                atomicAdd(&gr.v_lut[v_layer_base + (size_t(g) * 256 + s_code) * QKV_OUT + o], gh * w_scale);
                            }
                        }
                    }
                }
            }
        }

        // Update e_tok/e_pos on readout channels (`c < gate3_lo`) and Cl(0,6) spin-bound e_pos on `gate3` (`gate3_lo..gate12_lo`)!
        if (tid < gate3_lo && readout_violated) {
            int c = tid;
            int32_t gh = s_grad_h[c];
            if (gh != 0) {
                uint32_t tok_t = seq.tokens[t];
                int pos_idx = int(sc.pos_idx[t]);
                uint32_t spin_t = uint32_t((pos_idx * 7 + int(tok_t & 63)) & 63);
                int perm_c = c ^ (pos_idx & 7);
                int parity = popcnt32_dev(uint32_t(c & 63) & spin_t) & 1;
                atomicAdd(&gr.e_tok[tok_t * D + c], gh);
                atomicAdd(&gr.e_pos[pos_idx * D + perm_c], (parity == 0) ? gh : -gh);
                int h_dst = c / 8;
                for (int s = 1; s < copy_limit; ++s) {
                    if (sc.top_prll_w[h_dst][s] >= 40000) {
                        atomicAdd(&gr.e_tok[uint32_t(seq.tokens[s]) * D + c], gh);
                    }
                }
            }
        } else if (tid >= gate3_lo && tid < gate12_lo) {
            int c = tid;
            int32_t gh = s_grad_h[c];
            if (gh != 0) {
                uint32_t tok_t = seq.tokens[t];
                int pos_idx = int(sc.pos_idx[t]);
                uint32_t spin_t = uint32_t((pos_idx * 7 + int(tok_t & 63)) & 63);
                int perm_c = c ^ (pos_idx & 7);
                int parity = popcnt32_dev(uint32_t(c & 63) & spin_t) & 1;
                atomicAdd(&gr.e_pos[pos_idx * D + perm_c], (parity == 0) ? (gh * 2) : -(gh * 2));
            }
        }

        // Exact Chain-Rule Semantic V-Alignment for Global Context & Gate-Carrier Heads (`sh == 0`)
        if (tid < H) {
            int h = tid;
            const int sh = int(m.head_shift[top_l * H + h]);
            const int c_off = h * 8;
            const size_t qk_base = size_t(top_l) * G * 256 * QKV_OUT;

            if (sh == 0 && copy_limit > 2 && (!is_copy_pos || c_off >= gate3_lo)) {
                int best_pos_s = -1, best_neg_s = -1;
                int32_t max_align = -2000000000, min_align = 2000000000;
                for (int s = 1; s < copy_limit; ++s) {
                    int32_t align = 0;
                    for (int j = 0; j < 8; ++j) {
                        align += int32_t(sc.v_vec[top_l][s][c_off + j]) * s_grad_h[c_off + j];
                    }
                    if (align > max_align) { max_align = align; best_pos_s = s; }
                    if (align < min_align) { min_align = align; best_neg_s = s; }
                }
                if (best_pos_s >= 0 && best_neg_s >= 0 && best_pos_s != best_neg_s && max_align > 0) {
                    for (int j = 0; j < 8; ++j) {
                        int ch = c_off + j;
                        int32_t qv = int32_t(sc.q_vec[top_l][t][ch]);
                        int32_t kp = int32_t(sc.k_vec[top_l][best_pos_s][ch]);
                        int32_t kn = int32_t(sc.k_vec[top_l][best_neg_s][ch]);
                        for (int grp_k = 0; grp_k < 4; ++grp_k) {
                            int g_src = ((ch / 4) + G - grp_k) % G;
                            int o_src = (ch + D - g_src * 4) % D;
                            if (o_src < QKV_OUT) {
                                uint8_t qc = sc.qkv_codes[top_l][t][g_src];
                                uint8_t kcp = sc.k_ctx_codes[top_l][best_pos_s][g_src];
                                uint8_t kcn = sc.k_ctx_codes[top_l][best_neg_s][g_src];
                                atomicAdd(&gr.q_lut[qk_base + (size_t(g_src) * 256 + qc) * QKV_OUT + o_src], isign(kp - qv) - isign(kn - qv));
                                atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcp) * QKV_OUT + o_src], isign(qv - kp));
                                atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcn) * QKV_OUT + o_src], -isign(qv - kn));
                            }
                        }
                    }
                }
            }
        }
        __syncthreads();
    }
}

static uint16_t compute_optimal_scale_q8(const std::vector<int32_t>& w) {
    int32_t max_abs = 64;
    for (int32_t v : w) {
        int32_t c = std::abs(clamp_i32(v, -8192, 8192));
        if (c > max_abs) max_abs = c;
    }
    int32_t sc = ((max_abs << 4) + 126) / 127;
    if (sc < 16) sc = 16;
    if (sc > 4096) sc = 4096;
    return uint16_t(sc);
}

static void quantize_tensor_i8(const std::vector<int32_t>& w, uint16_t scale_q8, std::vector<int8_t>& out) {
    out.resize(w.size());
    int32_t sc = (scale_q8 < 16) ? 16 : int32_t(scale_q8);
    for (size_t i = 0; i < w.size(); ++i) {
        int32_t c = clamp_i32(w[i], -8192, 8192);
        int32_t q = clamp_i32(((c << 4) + (c >= 0 ? (sc >> 1) : -(sc >> 1))) / sc, -127, 127);
        out[i] = int8_t(q);
    }
}

static void append_u16_le(std::vector<uint8_t>& buf, uint16_t v) {
    buf.push_back(uint8_t(v & 0xFF));
    buf.push_back(uint8_t((v >> 8) & 0xFF));
}

static void append_u32_le(std::vector<uint8_t>& buf, uint32_t v) {
    buf.push_back(uint8_t(v & 0xFF));
    buf.push_back(uint8_t((v >> 8) & 0xFF));
    buf.push_back(uint8_t((v >> 16) & 0xFF));
    buf.push_back(uint8_t((v >> 24) & 0xFF));
}

// Deterministic Cl(0,6) byte-geometric embedding initializer across ALL 4,096 BPE tokens!
static void init_byte_geometric_embeddings(
    const std::vector<std::string>& vocab_bytes,
    int D,
    std::vector<int32_t>& h_etok,
    std::vector<int32_t>& h_pout,
    std::vector<int32_t>& h_copy_tok_gate
) {
    h_etok.assign(size_t(VOCAB_SIZE) * D, 0);
    h_copy_tok_gate.assign(VOCAB_SIZE, 192);
    h_copy_tok_gate[0] = 0; // <|bos|>
    h_copy_tok_gate[1] = 0; // <|eos|>
    h_copy_tok_gate[2] = 0; // <|unk|>

    for (int v = 0; v < VOCAB_SIZE; ++v) {
        const std::string& s = vocab_bytes[v];
        bool lead_space = (!s.empty() && s[0] == ' ');
        bool has_newline = (s.find('\n') != std::string::npos);
        bool has_under = (s.find('_') != std::string::npos);
        bool has_alpha_lo = false, has_alpha_hi = false, has_digit = false, has_punct = false;
        for (size_t i = (lead_space ? 1 : 0); i < s.size(); ++i) {
            unsigned char ch = (unsigned char)s[i];
            if (ch >= 'a' && ch <= 'z') has_alpha_lo = true;
            else if (ch >= 'A' && ch <= 'Z') has_alpha_hi = true;
            else if (ch >= '0' && ch <= '9') has_digit = true;
            else if (ch != '_' && ch != ' ') has_punct = true;
        }

        // Suppress pointer-copy on pure structural syntax/whitespace/tag tokens (\n, ```, ;, =, <, >)
        bool has_xml_angle = (s.find('<') != std::string::npos || s.find('>') != std::string::npos);
        if (v > 2 && (has_newline || has_xml_angle || s == "`" || s == ";" || s == "=" || s == " =")) {
            h_copy_tok_gate[v] = 0;
        } else if (v > 2 && s == " ") {
            h_copy_tok_gate[v] = 160; // Exact 2-gram mid-span space (e.g. before `|` in pipes), never span-start (`< 208`)
        } else if (v > 2 && !has_alpha_lo && !has_alpha_hi && !has_digit && !has_under) {
            if (s.find('-') == std::string::npos && s.find('/') == std::string::npos && s.find('|') == std::string::npos) {
                h_copy_tok_gate[v] = 32;
            }
        } else if (v > 2 && lead_space && s.size() >= 2) {
            unsigned char c1 = (unsigned char)s[1];
            if ((c1 >= 'a' && c1 <= 'z') || (c1 >= 'A' && c1 <= 'Z') || c1 == '/') {
                h_copy_tok_gate[v] = 224; // Span-start anchor eligible (`sh == 3`, >= 208; excludes numeric RHS literals like ` 63`!)
            }
        }

        int32_t* row = &h_etok[size_t(v) * D];
        row[0] = lead_space ? 900 : -900;
        row[1] = has_newline ? 900 : -900;
        row[2] = has_under ? 900 : -900;
        row[3] = has_punct ? 900 : -900;
        row[4] = has_alpha_lo ? 900 : -900;
        row[5] = has_alpha_hi ? 900 : -900;
        row[6] = has_digit ? 900 : -900;
        row[7] = (int(s.size()) >= 4) ? 700 : -700;

        // Deterministic Cl(0,6) Walsh character-gram hash across readout coordinates `8..gate3_lo` (zero on gate carriers `gate3_lo..D`!)
        const int gate3_lo = (D == 128) ? 80 : 40;
        uint64_t h1 = 14695981039346656037ULL ^ uint64_t(v * 1315423911ULL);
        for (unsigned char ch : s) {
            h1 ^= uint64_t(ch);
            h1 *= 1099511628211ULL;
        }
        for (int c = 8; c < gate3_lo; ++c) {
            h1 ^= (h1 >> 12);
            h1 ^= (h1 << 25);
            h1 ^= (h1 >> 27);
            uint64_t mix = h1 * 2685821657736338717ULL;
            int32_t val = int32_t((mix >> 48) % 1101ULL) - 550;
            row[c] = val;
        }
        for (int c = gate3_lo; c < D; ++c) {
            row[c] = 0;
        }
    }
    // Moderate prototype initialization so unseen & seen tokens start on equal footing!
    h_pout.resize(h_etok.size());
    for (size_t i = 0; i < h_etok.size(); ++i) {
        h_pout[i] = h_etok[i] >> 1;
    }
}

struct TrainResultSummary {
    std::string model_tag;
    int d_model;
    int n_layers;
    int n_heads;
    double initial_loss;
    double final_loss;
    double qat_exact_acc;
    size_t container_bytes;
    std::string container_sha256;
    double elapsed_sec;
};

struct GradMomBuf {
    int32_t* grad = nullptr;
    int32_t* mom = nullptr;
    size_t count = 0;

    void alloc(size_t n) {
        count = n;
        CHECK_CUDA(cudaMalloc(&grad, n * sizeof(int32_t)));
        CHECK_CUDA(cudaMalloc(&mom, n * sizeof(int32_t)));
        CHECK_CUDA(cudaMemset(grad, 0, n * sizeof(int32_t)));
        CHECK_CUDA(cudaMemset(mom, 0, n * sizeof(int32_t)));
    }
    void step(int32_t* param, int lr, int32_t lo, int32_t hi) {
        int blocks = int((count + 255) / 256);
        apply_momentum_step_kernel<<<blocks, 256>>>(param, mom, grad, count, lr, lo, hi);
    }
    void destroy() {
        if (grad) cudaFree(grad);
        if (mom) cudaFree(mom);
    }
};

static TrainResultSummary train_and_export_model(
    const std::string& model_tag,
    int D,
    int L,
    int H,
    int epochs,
    int qat_epochs,
    const std::vector<SequenceSample>& host_samples,
    const std::vector<std::string>& vocab_bytes,
    const std::string& out_container_path,
    const std::string& shard_dir
) {
    auto t0 = std::chrono::steady_clock::now();
    const int G = D / 4;
    const int BATCH = 128;

    std::mt19937 rng(D * 1000 + L * 97 + 42);
    auto rand_init = [&](size_t n, int32_t amp) {
        std::vector<int32_t> v(n);
        std::uniform_int_distribution<int32_t> dist(-amp, amp);
        for (size_t i = 0; i < n; ++i) v[i] = dist(rng);
        return v;
    };

    std::vector<int32_t> h_etok, h_pout, h_copy_tok_gate;
    init_byte_geometric_embeddings(vocab_bytes, D, h_etok, h_pout, h_copy_tok_gate);

    // Corpus-derived span-start vs continuation eligibility (`{0, 16, 96, 192, 224}`):
    // - Pure prompt framing words never copied (`prompt_hits > 50 && copy_hits == 0`) -> `0`
    // - Prompt framing tokens copied only mid-span (`prompt_hits > 50 && copy_start_hits == 0`) -> `96` (`< 208`)
    // - Unseen word-initial identifiers (`lead_space && alpha`, `prompt_hits <= 50`) -> remain `224` (`>= 208`)
    {
        std::vector<uint32_t> prompt_hits(VOCAB_SIZE, 0);
        std::vector<uint32_t> copy_hits(VOCAB_SIZE, 0);
        std::vector<uint32_t> copy_start_hits(VOCAB_SIZE, 0);
        for (const auto& seq : host_samples) {
            int plen = int(seq.prompt_len);
            int tlen = std::min(int(MAX_SEQ), plen + int(seq.reply_len));
            for (int i = 0; i < plen && i < MAX_SEQ; ++i) {
                uint16_t tok = seq.tokens[i];
                if (tok < VOCAB_SIZE) prompt_hits[tok]++;
            }
            for (int i = plen; i < tlen; ++i) {
                uint16_t tok = seq.tokens[i];
                if (tok < VOCAB_SIZE && seq.copy_mask[i - 1] != 0) {
                    copy_hits[tok]++;
                    if (i == plen || seq.copy_mask[i - 2] == 0) {
                        copy_start_hits[tok]++;
                    }
                }
            }
        }
        for (int v = 3; v < VOCAB_SIZE; ++v) {
            if (h_copy_tok_gate[v] == 0) continue;
            bool is_word_or_tag = (!vocab_bytes[v].empty() && vocab_bytes[v][0] == ' ') || vocab_bytes[v] == "[agent_mode]";
            if (!is_word_or_tag) continue;
            if (prompt_hits[v] > 50 && copy_hits[v] == 0) {
                h_copy_tok_gate[v] = 0;
            } else if (prompt_hits[v] > 50 && copy_start_hits[v] == 0) {
                h_copy_tok_gate[v] = 160; // Exclude from `sh == 3` (`>= 208`) while preserving exact 2-gram mid-span continuation gain!
            }
        }
    }

    std::vector<int32_t> h_epos = rand_init(256 * size_t(D), 450);
    std::vector<int32_t> h_qlut = rand_init(size_t(L) * G * 256 * QKV_OUT, 400);
    std::vector<int32_t> h_klut = rand_init(size_t(L) * G * 256 * QKV_OUT, 400);
    std::vector<int32_t> h_vlut = rand_init(size_t(L) * G * 256 * QKV_OUT, 450);
    std::vector<int32_t> h_ffnlut = rand_init(size_t(L) * G * 256 * D, 400);
    const int gate3_lo = (D == 128) ? 80 : 40;
    const int gate12_lo = (D == 128) ? 104 : 52;
    for (int i = 0; i < 256; ++i) {
        for (int c = gate3_lo; c < D; ++c) h_epos[size_t(i) * D + c] = 0;
    }
    for (size_t r = 0; r < size_t(G) * 256; ++r) {
        for (int c = gate3_lo; c < gate12_lo; ++c) {
            h_ffnlut[(size_t(L - 1) * G * 256 + r) * D + c] = -256;
        }
        for (int c = gate12_lo; c < D; ++c) {
            h_ffnlut[(size_t(L - 1) * G * 256 + r) * D + c] = 0;
        }
    }
    std::vector<int32_t> h_wtmac(size_t(VOCAB_SIZE) * D, 0); // ZERO trits until trained -> zero unseen-token bias!
    std::vector<int32_t> h_bout(VOCAB_SIZE, 0);

    std::vector<uint16_t> h_tau(L * H);
    std::vector<uint16_t> h_slope(L * H);
    std::vector<uint8_t> h_shift(L * H);
    std::vector<int32_t> h_copy_b(L * H, -4096);
    std::vector<int32_t> h_copy_w(size_t(L) * H * D, 0);

    const int top_copy_h_limit = gate3_lo / 8;
    for (int l = 0; l < L; ++l) {
        for (int h = 0; h < H; ++h) {
            if (l == L - 1 && h >= top_copy_h_limit) {
                h_shift[l * H + h] = 0; // Gate-carrier causal context reader
            } else if (h % 4 == 3) {
                h_shift[l * H + h] = 1; // 1/2-gram continuation induction
            } else if (h % 4 == 2) {
                h_shift[l * H + h] = 2; // 2/3-gram continuation induction
            } else if (h % 4 == 1) {
                h_shift[l * H + h] = 3; // Structural anchor-shift (ks = s - 1)
            } else {
                h_shift[l * H + h] = 0; // Semantic context reader
            }
            h_tau[l * H + h] = (h_shift[l * H + h] == 0) ? uint16_t(768 + (h % 8) * 192) : uint16_t(256);
            h_slope[l * H + h] = (h_shift[l * H + h] == 0) ? uint16_t((h % 8 < 4) ? 20 : 2) : 0;
            if (l == L - 1 && h < top_copy_h_limit && (h % 4 != 0)) {
                int32_t* cw = &h_copy_w[(size_t(l) * H + h) * D];
                if (h % 4 == 1) {
                    h_copy_b[l * H + h] = -250;
                    for (int c = gate3_lo; c < gate12_lo; ++c) cw[c] = 512; // +1 trit on span-start carrier
                } else {
                    h_copy_b[l * H + h] = (D == 128) ? 1400 : 900;
                    for (int c = gate12_lo; c < D; ++c) cw[c] = 512;        // +1 trit on continuation carrier
                }
            }
        }
    }

    std::vector<uint16_t> h_scales(3 + 4 * L, 1024);

    DeviceModel dm{};
    dm.d_model = D; dm.n_layers = L; dm.n_heads = H; dm.lut_groups = G;

    auto alloc_copy = [](const auto& vec, auto** ptr) {
        CHECK_CUDA(cudaMalloc((void**)ptr, vec.size() * sizeof(vec[0])));
        CHECK_CUDA(cudaMemcpy(*ptr, vec.data(), vec.size() * sizeof(vec[0]), cudaMemcpyHostToDevice));
    };

    alloc_copy(h_etok, &dm.e_tok);
    alloc_copy(h_epos, &dm.e_pos);
    alloc_copy(h_qlut, &dm.q_lut);
    alloc_copy(h_klut, &dm.k_lut);
    alloc_copy(h_vlut, &dm.v_lut);
    alloc_copy(h_ffnlut, &dm.ffn_lut);
    alloc_copy(h_wtmac, &dm.w_tmac);
    alloc_copy(h_pout, &dm.p_out);
    alloc_copy(h_bout, &dm.b_out);
    alloc_copy(h_tau, &dm.head_tau);
    alloc_copy(h_slope, &dm.head_slope);
    alloc_copy(h_shift, &dm.head_shift);
    alloc_copy(h_copy_b, &dm.head_copy_b);
    alloc_copy(h_copy_w, &dm.head_copy_w);
    alloc_copy(h_copy_tok_gate, &dm.copy_tok_gate);
    alloc_copy(h_scales, &dm.scales_q8);

    GradMomBuf gm_etok, gm_epos, gm_qlut, gm_klut, gm_vlut, gm_ffnlut, gm_wtmac, gm_pout, gm_bout, gm_copyb, gm_copyw, gm_copytok;
    gm_etok.alloc(h_etok.size());
    gm_epos.alloc(h_epos.size());
    gm_qlut.alloc(h_qlut.size());
    gm_klut.alloc(h_klut.size());
    gm_vlut.alloc(h_vlut.size());
    gm_ffnlut.alloc(h_ffnlut.size());
    gm_wtmac.alloc(h_wtmac.size());
    gm_pout.alloc(h_pout.size());
    gm_bout.alloc(h_bout.size());
    gm_copyb.alloc(h_copy_b.size());
    gm_copyw.alloc(h_copy_w.size());
    gm_copytok.alloc(h_copy_tok_gate.size());

    DeviceGrads gr{
        gm_etok.grad, gm_epos.grad, gm_qlut.grad, gm_klut.grad, gm_vlut.grad,
        gm_ffnlut.grad, gm_wtmac.grad, gm_pout.grad, gm_bout.grad, gm_copyb.grad, gm_copyw.grad, gm_copytok.grad
    };

    SequenceSample* d_samples = nullptr;
    CHECK_CUDA(cudaMalloc(&d_samples, host_samples.size() * sizeof(SequenceSample)));
    CHECK_CUDA(cudaMemcpy(d_samples, host_samples.data(), host_samples.size() * sizeof(SequenceSample), cudaMemcpyHostToDevice));

    BlockScratch* d_scratches = nullptr;
    CHECK_CUDA(cudaMalloc(&d_scratches, BATCH * sizeof(BlockScratch)));

    int* d_batch_idx = nullptr;
    CHECK_CUDA(cudaMalloc(&d_batch_idx, BATCH * sizeof(int)));

    unsigned long long* d_loss = nullptr;
    unsigned int* d_corr = nullptr;
    unsigned int* d_tot = nullptr;
    CHECK_CUDA(cudaMalloc(&d_loss, sizeof(unsigned long long)));
    CHECK_CUDA(cudaMalloc(&d_corr, sizeof(unsigned int)));
    CHECK_CUDA(cudaMalloc(&d_tot, sizeof(unsigned int)));

    const int n_samples = int(host_samples.size());
    std::vector<int> order(n_samples);
    for (int i = 0; i < n_samples; ++i) order[i] = i;

    double initial_loss = 0.0;
    double best_qat_acc = -1.0;
    double best_qat_loss = 1e9;

    // Best-checkpoint host buffers
    std::vector<int32_t> b_etok, b_epos, b_qlut, b_klut, b_vlut, b_ffnlut, b_wtmac, b_pout, b_bout, b_copy_b, b_copy_w, b_copy_tok;
    std::vector<uint16_t> b_scales;

    auto sync_device_to_host = [&]() {
        CHECK_CUDA(cudaMemcpy(h_etok.data(), dm.e_tok, h_etok.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_epos.data(), dm.e_pos, h_epos.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_pout.data(), dm.p_out, h_pout.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_qlut.data(), dm.q_lut, h_qlut.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_klut.data(), dm.k_lut, h_klut.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_vlut.data(), dm.v_lut, h_vlut.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_ffnlut.data(), dm.ffn_lut, h_ffnlut.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_wtmac.data(), dm.w_tmac, h_wtmac.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_bout.data(), dm.b_out, h_bout.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_copy_b.data(), dm.head_copy_b, h_copy_b.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_copy_w.data(), dm.head_copy_w, h_copy_w.size() * 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_copy_tok_gate.data(), dm.copy_tok_gate, h_copy_tok_gate.size() * 4, cudaMemcpyDeviceToHost));

        h_scales[0] = compute_optimal_scale_q8(h_etok);
        h_scales[1] = compute_optimal_scale_q8(h_epos);
        h_scales[2] = compute_optimal_scale_q8(h_pout);
        for (int l = 0; l < L; ++l) {
            size_t qk_n = size_t(G) * 256 * QKV_OUT;
            size_t ffn_n = size_t(G) * 256 * D;
            std::vector<int32_t> sub_q(h_qlut.begin() + l * qk_n, h_qlut.begin() + (l + 1) * qk_n);
            std::vector<int32_t> sub_k(h_klut.begin() + l * qk_n, h_klut.begin() + (l + 1) * qk_n);
            std::vector<int32_t> sub_v(h_vlut.begin() + l * qk_n, h_vlut.begin() + (l + 1) * qk_n);
            std::vector<int32_t> sub_f(h_ffnlut.begin() + l * ffn_n, h_ffnlut.begin() + (l + 1) * ffn_n);
            h_scales[3 + 4 * l + 0] = compute_optimal_scale_q8(sub_q);
            h_scales[3 + 4 * l + 1] = compute_optimal_scale_q8(sub_k);
            h_scales[3 + 4 * l + 2] = compute_optimal_scale_q8(sub_v);
            h_scales[3 + 4 * l + 3] = compute_optimal_scale_q8(sub_f);
        }
        CHECK_CUDA(cudaMemcpy(dm.scales_q8, h_scales.data(), h_scales.size() * 2, cudaMemcpyHostToDevice));
    };

    auto eval_exact_qat = [&]() -> std::pair<double, double> {
        CHECK_CUDA(cudaMemset(d_loss, 0, sizeof(unsigned long long)));
        CHECK_CUDA(cudaMemset(d_corr, 0, sizeof(unsigned int)));
        CHECK_CUDA(cudaMemset(d_tot, 0, sizeof(unsigned int)));
        for (int off = 0; off < n_samples; off += BATCH) {
            int cur_b = std::min(BATCH, n_samples - off);
            CHECK_CUDA(cudaMemcpy(d_batch_idx, &order[off], cur_b * sizeof(int), cudaMemcpyHostToDevice));
            forward_and_grad_kernel<<<cur_b, 256>>>(
                dm, gr, d_scratches, d_samples, d_batch_idx, cur_b,
                0, 1, d_loss, d_corr, d_tot
            );
        }
        CHECK_CUDA(cudaDeviceSynchronize());
        unsigned long long eval_loss = 0;
        unsigned int eval_corr = 0, eval_tot = 0;
        CHECK_CUDA(cudaMemcpy(&eval_loss, d_loss, sizeof(unsigned long long), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&eval_corr, d_corr, sizeof(unsigned int), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&eval_tot, d_tot, sizeof(unsigned int), cudaMemcpyDeviceToHost));
        double l_val = double(eval_loss) / double(std::max(1u, eval_tot));
        double a_val = 100.0 * double(eval_corr) / double(std::max(1u, eval_tot));
        return {l_val, a_val};
    };

    const int total_epochs = epochs + qat_epochs;
    for (int ep = 0; ep < total_epochs; ++ep) {
        int use_qat = (ep >= epochs) ? 1 : 0;
        if (ep == epochs) {
            // Restore best pre-QAT checkpoint onto GPU and freeze optimal int8 scales for QAT polish!
            if (best_qat_acc > 0.0) {
                CHECK_CUDA(cudaMemcpy(dm.e_tok, b_etok.data(), b_etok.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.e_pos, b_epos.data(), b_epos.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.p_out, b_pout.data(), b_pout.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.q_lut, b_qlut.data(), b_qlut.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.k_lut, b_klut.data(), b_klut.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.v_lut, b_vlut.data(), b_vlut.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.ffn_lut, b_ffnlut.data(), b_ffnlut.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.w_tmac, b_wtmac.data(), b_wtmac.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.b_out, b_bout.data(), b_bout.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.head_copy_b, b_copy_b.data(), b_copy_b.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.head_copy_w, b_copy_w.data(), b_copy_w.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.scales_q8, b_scales.data(), b_scales.size() * 2, cudaMemcpyHostToDevice));
            } else {
                sync_device_to_host();
            }
        }

        std::shuffle(order.begin(), order.end(), rng);
        int lr_main = (L >= 8) ? 22 : 28;
        int lr_trit = 20;
        if (ep >= epochs) {
            lr_main = (ep < epochs + qat_epochs / 2) ? 3 : 1;
            lr_trit = (ep < epochs + qat_epochs / 2) ? 2 : 1;
        } else if (ep >= 9) {
            lr_main = 5; lr_trit = 4;
        } else if (ep >= 5) {
            lr_main = 12; lr_trit = 9;
        }

        CHECK_CUDA(cudaMemset(d_loss, 0, sizeof(unsigned long long)));
        CHECK_CUDA(cudaMemset(d_corr, 0, sizeof(unsigned int)));
        CHECK_CUDA(cudaMemset(d_tot, 0, sizeof(unsigned int)));

        for (int off = 0; off < n_samples; off += BATCH) {
            int cur_b = std::min(BATCH, n_samples - off);
            CHECK_CUDA(cudaMemcpy(d_batch_idx, &order[off], cur_b * sizeof(int), cudaMemcpyHostToDevice));
            forward_and_grad_kernel<<<cur_b, 256>>>(
                dm, gr, d_scratches, d_samples, d_batch_idx, cur_b,
                1, use_qat, d_loss, d_corr, d_tot
            );
            gm_etok.step(dm.e_tok, std::max(1, lr_main / 2), -8192, 8192);
            gm_epos.step(dm.e_pos, std::max(1, lr_main / 2), -8192, 8192);
            gm_qlut.step(dm.q_lut, lr_main, -8192, 8192);
            gm_klut.step(dm.k_lut, lr_main, -8192, 8192);
            gm_vlut.step(dm.v_lut, lr_main, -8192, 8192);
            gm_ffnlut.step(dm.ffn_lut, lr_main, -8192, 8192);
            gm_wtmac.step(dm.w_tmac, lr_trit, -1152, 1152);
            gm_pout.step(dm.p_out, std::max(1, lr_main / 2), -8192, 8192);
            gm_bout.step(dm.b_out, lr_trit, -2048, 2048);
        }
        CHECK_CUDA(cudaDeviceSynchronize());

        unsigned long long h_loss = 0;
        unsigned int h_corr = 0, h_tot = 0;
        CHECK_CUDA(cudaMemcpy(&h_loss, d_loss, sizeof(unsigned long long), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_corr, d_corr, sizeof(unsigned int), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_tot, d_tot, sizeof(unsigned int), cudaMemcpyDeviceToHost));

        double avg_loss = double(h_loss) / double(std::max(1u, h_tot));
        double acc = 100.0 * double(h_corr) / double(std::max(1u, h_tot));
        if (ep == 0) initial_loss = avg_loss;

        printf("[%s] Epoch %2d/%2d (QAT=%d, lr=%2d) | R4.Objective Hinge Loss: %8.2f | Next-Token Exact Acc: %6.2f%% (%u/%u)\n",
               model_tag.c_str(), ep + 1, total_epochs, use_qat, lr_main, avg_loss, acc, h_corr, h_tot);
        fflush(stdout);

        if (ep >= 3) {
            if (!use_qat) sync_device_to_host();
            auto [qloss, qacc] = eval_exact_qat();
            if (qacc > best_qat_acc) {
                best_qat_acc = qacc;
                best_qat_loss = qloss;
                sync_device_to_host();
                b_etok = h_etok; b_epos = h_epos; b_qlut = h_qlut; b_klut = h_klut;
                b_vlut = h_vlut; b_ffnlut = h_ffnlut; b_wtmac = h_wtmac; b_pout = h_pout;
                b_bout = h_bout; b_copy_b = h_copy_b; b_copy_w = h_copy_w; b_copy_tok = h_copy_tok_gate;
                b_scales = h_scales;
            }
        }
    }

    // Restore best exact post-quantization snapshot
    if (best_qat_acc > 0.0) {
        h_etok = b_etok; h_epos = b_epos; h_qlut = b_qlut; h_klut = b_klut;
        h_vlut = b_vlut; h_ffnlut = b_ffnlut; h_wtmac = b_wtmac; h_pout = b_pout;
        h_bout = b_bout; h_copy_b = b_copy_b; h_copy_w = b_copy_w; h_copy_tok_gate = b_copy_tok;
        h_scales = b_scales;
    } else {
        sync_device_to_host();
        auto [qloss, qacc] = eval_exact_qat();
        best_qat_loss = qloss;
        best_qat_acc = qacc;
    }

    printf("[%s] BEST POST-QUANTIZATION EXACT INTEGER EVAL | Loss: %.2f | Exact Acc: %.2f%%\n",
           model_tag.c_str(), best_qat_loss, best_qat_acc);

    // Serialize UORGLLM9 payload
    std::vector<uint8_t> payload;
    payload.reserve(18 * 1024 * 1024);
    for (uint16_t sc : h_scales) append_u16_le(payload, sc);

    std::vector<int8_t> q_buf;
    quantize_tensor_i8(h_etok, h_scales[0], q_buf);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

    quantize_tensor_i8(h_epos, h_scales[1], q_buf);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

    for (int l = 0; l < L; ++l) {
        for (int h = 0; h < H; ++h) append_u16_le(payload, h_tau[l * H + h]);
        for (int h = 0; h < H; ++h) append_u16_le(payload, h_slope[l * H + h]);
        for (int h = 0; h < H; ++h) payload.push_back(h_shift[l * H + h]);
        for (int h = 0; h < H; ++h) {
            int16_t cb = int16_t(clamp_i32(h_copy_b[l * H + h], -8192, 8192));
            append_u16_le(payload, uint16_t(cb));
        }
        for (int h = 0; h < H; ++h) {
            const int32_t* cw = &h_copy_w[(l * H + h) * D];
            for (int g = 0; g < G; ++g) {
                payload.push_back(pack_4trits(
                    latent_to_trit(cw[4*g + 0]),
                    latent_to_trit(cw[4*g + 1]),
                    latent_to_trit(cw[4*g + 2]),
                    latent_to_trit(cw[4*g + 3])
                ));
            }
        }

        size_t qk_n = size_t(G) * 256 * QKV_OUT;
        size_t ffn_n = size_t(G) * 256 * D;
        std::vector<int32_t> sub_q(h_qlut.begin() + l * qk_n, h_qlut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_k(h_klut.begin() + l * qk_n, h_klut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_v(h_vlut.begin() + l * qk_n, h_vlut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_f(h_ffnlut.begin() + l * ffn_n, h_ffnlut.begin() + (l + 1) * ffn_n);

        quantize_tensor_i8(sub_q, h_scales[3 + 4 * l + 0], q_buf);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
        quantize_tensor_i8(sub_k, h_scales[3 + 4 * l + 1], q_buf);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
        quantize_tensor_i8(sub_v, h_scales[3 + 4 * l + 2], q_buf);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
        quantize_tensor_i8(sub_f, h_scales[3 + 4 * l + 3], q_buf);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
    }

    // 2-plane balanced-nonary T-MAC readout weights: hi plane [VOCAB_SIZE * G], lo plane [VOCAB_SIZE * G]
    std::vector<uint8_t> tmac_hi(size_t(VOCAB_SIZE) * G);
    std::vector<uint8_t> tmac_lo(size_t(VOCAB_SIZE) * G);
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        const int32_t* tw = &h_wtmac[v * D];
        for (int g = 0; g < G; ++g) {
            uint8_t chi, clo;
            pack_4q9(tw[4*g + 0], tw[4*g + 1], tw[4*g + 2], tw[4*g + 3], chi, clo);
            tmac_hi[size_t(v) * G + g] = chi;
            tmac_lo[size_t(v) * G + g] = clo;
        }
    }
    payload.insert(payload.end(), tmac_hi.begin(), tmac_hi.end());
    payload.insert(payload.end(), tmac_lo.begin(), tmac_lo.end());

    for (int v = 0; v < VOCAB_SIZE; ++v) {
        int32_t cg = (v <= 2) ? 0 : clamp_i32(h_copy_tok_gate[v], 0, 255);
        payload.push_back(uint8_t(cg));
    }

    quantize_tensor_i8(h_pout, h_scales[2], q_buf);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        int16_t bv = int16_t(clamp_i32(h_bout[v], -2048, 2048));
        append_u16_le(payload, uint16_t(bv));
    }

    uint8_t payload_sha[32];
    sha256_bytes(payload.data(), payload.size(), payload_sha);

    std::vector<uint8_t> container;
    container.reserve(80 + payload.size());
    const char magic[8] = {'U','O','R','G','L','L','M','9'};
    container.insert(container.end(), magic, magic + 8);
    append_u32_le(container, 9u);
    append_u32_le(container, uint32_t(VOCAB_SIZE));
    append_u32_le(container, uint32_t(D));
    append_u32_le(container, uint32_t(L));
    append_u32_le(container, uint32_t(H));
    append_u32_le(container, uint32_t(G));
    append_u32_le(container, 0u);
    append_u32_le(container, 0u);
    append_u32_le(container, 0u);
    append_u32_le(container, 8u);
    container.insert(container.end(), payload_sha, payload_sha + 32);
    container.insert(container.end(), payload.begin(), payload.end());

    uint8_t full_sha[32];
    sha256_bytes(container.data(), container.size(), full_sha);
    std::string full_sha_hex = hex_of(full_sha, 32);

    {
        std::ofstream f(out_container_path, std::ios::binary);
        f.write((const char*)container.data(), container.size());
    }
    printf("[%s] Wrote %zu bytes -> %s (sha256=%s)\n",
           model_tag.c_str(), container.size(), out_container_path.c_str(), full_sha_hex.c_str());

    if (!shard_dir.empty()) {
        const size_t SHARD_CAP = 524288;
        size_t shard_count = (container.size() + SHARD_CAP - 1) / SHARD_CAP;
        std::string manifest = "{\n";
        manifest += "  \"format\": \"UORGLLM9\",\n";
        manifest += "  \"schema\": \"uor-r4.compositional-gllm/9\",\n";
        manifest += "  \"version\": 9,\n";
        manifest += "  \"vocab_size\": 4096,\n";
        manifest += "  \"d_model\": " + std::to_string(D) + ",\n";
        manifest += "  \"n_layers\": " + std::to_string(L) + ",\n";
        manifest += "  \"n_heads\": " + std::to_string(H) + ",\n";
        manifest += "  \"turn_memory_entries\": 0,\n";
        manifest += "  \"dense_matmul_ops\": 0,\n";
        manifest += "  \"softmax_ops\": 0,\n";
        manifest += "  \"total_bytes\": " + std::to_string(container.size()) + ",\n";
        manifest += "  \"container_sha256\": \"" + full_sha_hex + "\",\n";
        manifest += "  \"shard_max_bytes\": 524288,\n";
        manifest += "  \"shards\": [\n";

        for (size_t s = 0; s < shard_count; ++s) {
            size_t off = s * SHARD_CAP;
            size_t len = std::min(SHARD_CAP, container.size() - off);
            char part_name[64];
            snprintf(part_name, sizeof(part_name), "uor_r4_gllm_v9.uorgllm.part%03zu", s);
            std::string part_path = shard_dir + "/" + part_name;
            {
                std::ofstream sf(part_path, std::ios::binary);
                sf.write((const char*)(container.data() + off), len);
            }
            uint8_t s_sha[32];
            sha256_bytes(container.data() + off, len, s_sha);
            manifest += "    {\"index\": " + std::to_string(s) +
                        ", \"file\": \"" + std::string(part_name) +
                        "\", \"byte_offset\": " + std::to_string(off) +
                        ", \"byte_length\": " + std::to_string(len) +
                        ", \"sha256\": \"" + hex_of(s_sha, 32) + "\"}";
            if (s + 1 < shard_count) manifest += ",";
            manifest += "\n";
        }
        manifest += "  ]\n}\n";
        std::ofstream mf(shard_dir + "/manifest.json");
        mf << manifest;
        printf("[%s] Emitted %zu 512KB OPFS shards + manifest.json -> %s\n",
               model_tag.c_str(), shard_count, shard_dir.c_str());
    }

    gm_etok.destroy(); gm_epos.destroy(); gm_qlut.destroy(); gm_klut.destroy(); gm_vlut.destroy();
    gm_ffnlut.destroy(); gm_wtmac.destroy(); gm_pout.destroy(); gm_bout.destroy();
    gm_copyb.destroy(); gm_copyw.destroy(); gm_copytok.destroy();
    cudaFree(dm.e_tok); cudaFree(dm.e_pos); cudaFree(dm.q_lut); cudaFree(dm.k_lut);
    cudaFree(dm.v_lut); cudaFree(dm.ffn_lut); cudaFree(dm.w_tmac); cudaFree(dm.p_out); cudaFree(dm.b_out);
    cudaFree(dm.head_tau); cudaFree(dm.head_slope); cudaFree(dm.head_shift);
    cudaFree(dm.head_copy_b); cudaFree(dm.head_copy_w); cudaFree(dm.copy_tok_gate); cudaFree(dm.scales_q8);
    cudaFree(d_samples); cudaFree(d_scratches); cudaFree(d_batch_idx);
    cudaFree(d_loss); cudaFree(d_corr); cudaFree(d_tot);

    double elapsed = std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count();
    return TrainResultSummary{
        model_tag, D, L, H, initial_loss, best_qat_loss, best_qat_acc,
        container.size(), full_sha_hex, elapsed
    };
}

int main(int argc, char** argv) {
    if (argc < 4) {
        fprintf(stderr, "Usage: %s <corpus.bin> <bootstrap_v9.uorgllm> <opfs_shard_dir> [receipt.json]\n", argv[0]);
        return 1;
    }

    cudaDeviceProp prop{};
    CHECK_CUDA(cudaGetDeviceProperties(&prop, 0));
    printf("=== UOR-R4-GLLM v9 Pure-Geometric CUDA Trainer (rev 17) on %s (SM %d.%d, %zu MiB VRAM) ===\n",
           prop.name, prop.major, prop.minor, prop.totalGlobalMem / (1024 * 1024));

    std::string corpus_path(argv[1]);
    size_t slash = corpus_path.find_last_of('/');
    std::string dir_prefix = (slash == std::string::npos) ? "." : corpus_path.substr(0, slash);
    std::string vocab_path = dir_prefix + "/v9_vocab_bytes.bin";

    std::vector<std::string> vocab_bytes(VOCAB_SIZE);
    {
        std::ifstream vf(vocab_path, std::ios::binary);
        if (!vf) { fprintf(stderr, "Cannot open vocab bytes %s\n", vocab_path.c_str()); return 1; }
        char vmagic[8];
        vf.read(vmagic, 8);
        if (memcmp(vmagic, "UORVOCB9", 8) != 0) { fprintf(stderr, "Invalid vocab magic\n"); return 1; }
        uint32_t vcnt = 0;
        vf.read((char*)&vcnt, 4);
        for (uint32_t i = 0; i < vcnt && i < VOCAB_SIZE; ++i) {
            uint8_t blen = 0;
            vf.read((char*)&blen, 1);
            vocab_bytes[i].resize(blen);
            if (blen > 0) vf.read(&vocab_bytes[i][0], blen);
        }
    }

    std::ifstream cf(corpus_path, std::ios::binary);
    if (!cf) { fprintf(stderr, "Cannot open corpus %s\n", corpus_path.c_str()); return 1; }
    char magic[8];
    cf.read(magic, 8);
    if (memcmp(magic, "UORCORP2", 8) != 0) { fprintf(stderr, "Invalid corpus magic\n"); return 1; }
    uint32_t count = 0;
    cf.read((char*)&count, 4);

    std::vector<SequenceSample> samples(count);
    for (uint32_t i = 0; i < count; ++i) {
        uint16_t plen = 0, rlen = 0;
        cf.read((char*)&plen, 2);
        cf.read((char*)&rlen, 2);
        samples[i].prompt_len = plen;
        samples[i].reply_len = rlen;
        memset(samples[i].tokens, 0, sizeof(samples[i].tokens));
        memset(samples[i].copy_mask, 0, sizeof(samples[i].copy_mask));
        cf.read((char*)samples[i].tokens, (plen + rlen) * 2);
        std::vector<uint8_t> rcopy(rlen, 0);
        cf.read((char*)rcopy.data(), rlen);
        // Store rcopy[k] at copy_mask[(plen - 1) + k] so at query t, target = tokens[t+1] has flag copy_mask[t]!
        for (uint16_t k = 0; k < rlen; ++k) {
            if ((plen - 1 + k) < MAX_SEQ) {
                samples[i].copy_mask[plen - 1 + k] = rcopy[k];
            }
        }
    }
    printf("Loaded %u compositional ByteBPE-4096 sequences + 4,096 byte-geometric vocab entries.\n", count);

    auto boot_res = train_and_export_model(
        "bootstrap_v9_6L_128D", 128, 6, 16, 12, 6, samples, vocab_bytes, argv[2], ""
    );

    std::string opfs_full_bin = std::string(argv[3]) + "_full.uorgllm";
    auto opfs_res = train_and_export_model(
        "opfs_v9_8L_128D", 128, 8, 16, 12, 6, samples, vocab_bytes, opfs_full_bin, argv[3]
    );
    remove(opfs_full_bin.c_str());

    if (argc >= 5) {
        std::ofstream rf(argv[4]);
        rf << "{\n"
           << "  \"schema\": \"uor-r4.gpu-training-receipt/9\",\n"
           << "  \"gpu_device\": \"" << prop.name << "\",\n"
           << "  \"compute_capability\": \"" << prop.major << "." << prop.minor << "\",\n"
           << "  \"vram_mib\": " << (prop.totalGlobalMem / (1024 * 1024)) << ",\n"
           << "  \"corpus_sequences\": " << count << ",\n"
           << "  \"dense_matmul_ops\": 0,\n"
           << "  \"softmax_ops\": 0,\n"
           << "  \"turn_memory_entries\": 0,\n"
           << "  \"bootstrap_v9\": {\n"
           << "    \"d_model\": " << boot_res.d_model << ",\n"
           << "    \"n_layers\": " << boot_res.n_layers << ",\n"
           << "    \"n_heads\": " << boot_res.n_heads << ",\n"
           << "    \"initial_hinge_loss\": " << boot_res.initial_loss << ",\n"
           << "    \"final_hinge_loss\": " << boot_res.final_loss << ",\n"
           << "    \"post_quant_exact_token_acc_pct\": " << boot_res.qat_exact_acc << ",\n"
           << "    \"container_bytes\": " << boot_res.container_bytes << ",\n"
           << "    \"container_sha256\": \"" << boot_res.container_sha256 << "\",\n"
           << "    \"elapsed_seconds\": " << boot_res.elapsed_sec << "\n"
           << "  },\n"
           << "  \"opfs_sharded_v9\": {\n"
           << "    \"d_model\": " << opfs_res.d_model << ",\n"
           << "    \"n_layers\": " << opfs_res.n_layers << ",\n"
           << "    \"n_heads\": " << opfs_res.n_heads << ",\n"
           << "    \"initial_hinge_loss\": " << opfs_res.initial_loss << ",\n"
           << "    \"final_hinge_loss\": " << opfs_res.final_loss << ",\n"
           << "    \"post_quant_exact_token_acc_pct\": " << opfs_res.qat_exact_acc << ",\n"
           << "    \"container_bytes\": " << opfs_res.container_bytes << ",\n"
           << "    \"container_sha256\": \"" << opfs_res.container_sha256 << "\",\n"
           << "    \"elapsed_seconds\": " << opfs_res.elapsed_sec << "\n"
           << "  }\n"
           << "}\n";
    }
    return 0;
}
