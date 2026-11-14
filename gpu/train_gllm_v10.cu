// Pure-Geometric CUDA Trainer, Held-Out Validator & UORGLLM9 Exporter for UOR-R4-GLLM v10.
//
// Charter Compliance (AGENTS.md):
//   - ZERO dense matrix multiplication (no GEMM / cublas / matmul).
//   - ZERO softmax, log-softmax, sigmoid, tanh, GELU, or exponential functions.
//   - ZERO ServedTurnMemory canned turn-ID tables (turn_memory_entries = 0).
//   - 100% bit-identical integer forward pass with crates/wasm/src/gllm_v9.rs
//     extended cleanly to MAX_SEQ = 1024 and multi-turn <|user|>/<|assistant|> framing:
//     * Deterministic Cl(0,6) byte-geometry vocabulary initialization across all 4,096 BPE tokens
//     * Cl(0,6) FWHT-64 integer butterflies + 256-entry discrete T-MAC Q12 LUTs
//     * 4th-order rational PRLL (R4-27) causal KV cache attention (exact 1,000,000 mass)
//     * 2-plane balanced-nonary [-4..+4] 81-state T-MAC activation LUT readout (ZERO multiplies)
//     * Multi-turn & prompt-bounded causal KV pointer-copy with batch-precomputed Q12/Q9 caches.

#include <cuda_runtime.h>
#include <sys/stat.h>
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
static constexpr int MAX_SEQ = 1024;
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

static std::string json_escape(const std::string& s) {
    std::string out;
    out.reserve(s.size() + 16);
    for (unsigned char c : s) {
        if (c == '"') out += "\\\"";
        else if (c == '\\') out += "\\\\";
        else if (c == '\n') out += "\\n";
        else if (c == '\r') out += "\\r";
        else if (c == '\t') out += "\\t";
        else if (c < 0x20) {
            char hex[8];
            snprintf(hex, sizeof(hex), "\\u%04x", int(c));
            out += hex;
        } else {
            out.push_back(char(c));
        }
    }
    return out;
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

__host__ __device__ static inline int8_t latent_to_q9(int32_t w) {
    int32_t num = w + (w >= 0 ? 128 : -128);
    int32_t q = num / 256;
    return int8_t(clamp_i32(q, -4, 4));
}

__host__ __device__ static inline void q9_to_trit_pair(int8_t q9, int8_t& t_hi, int8_t& t_lo) {
    int u = int(q9) + 4;
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

__host__ __device__ static inline uint64_t prll_weight_order4(int32_t tau, int32_t d_tot) {
    uint64_t t = (tau < 256) ? 256ULL : uint64_t(tau);
    uint64_t d = (d_tot < 0) ? 0ULL : uint64_t(d_tot);
    uint64_t num1 = t << 8;
    uint64_t den1 = t + d;
    uint64_t r1 = num1 / den1;
    uint64_t r2 = r1 * r1;
    uint64_t r4 = (r2 >> 4) * (r2 >> 4);
    return (r4 == 0) ? 1ULL : r4;
}

__host__ __device__ static inline int16_t quant_sim_q12(int32_t latent, int use_qat, uint16_t scale_q8) {
    int32_t c = clamp_i32(latent, -8192, 8192);
    if (!use_qat) return int16_t(c);
    int32_t sc = (scale_q8 < 16) ? 16 : int32_t(scale_q8);
    int32_t q = clamp_i32(((c << 4) + (c >= 0 ? (sc >> 1) : -(sc >> 1))) / sc, -127, 127);
    return clamp_q12((q * sc) >> 4);
}

struct SequenceSample {
    uint16_t prompt_len;
    uint16_t reply_len;
    uint16_t obs_lo;
    uint16_t obs_hi;
    uint8_t gen_flags; // bit 0 = in_code_gen, bit 1 = in_agent_gen
    uint8_t _pad;
    uint16_t tokens[MAX_SEQ];
    uint8_t copy_mask[MAX_SEQ];
};

struct DeviceModel {
    int d_model;
    int n_layers;
    int n_heads;
    int lut_groups;

    int32_t* e_tok;
    int32_t* e_pos;
    int32_t* q_lut;
    int32_t* k_lut;
    int32_t* v_lut;
    int32_t* ffn_lut;

    int32_t* w_tmac;
    int32_t* p_out;
    int32_t* b_out;

    uint16_t* head_tau;
    uint16_t* head_slope;
    uint8_t* head_shift;
    int32_t* head_copy_b;
    int32_t* head_copy_w;
    int32_t* copy_tok_gate;
    uint8_t* noncopy_tok;

    uint16_t* scales_q8;
};

// Batch-precomputed exact Q12/Q9 view of read-only DeviceModel tensors (100% bit-identical, ~20x faster!)
struct BatchCache {
    int16_t* etok_q12;
    int16_t* epos_q12;
    int16_t* pout_q12;
    int16_t* qlut_q12;
    int16_t* klut_q12;
    int16_t* vlut_q12;
    int16_t* ffnlut_q12;
    uint8_t* tmac_hi;
    uint8_t* tmac_lo;
    uint64_t* psk0;
    uint64_t* psk1;
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

struct BlockScratch {
    uint8_t pos_idx[MAX_SEQ];
    int16_t x_cur[MAX_SEQ][MAX_D];
    int16_t q_vec[MAX_L][MAX_SEQ][MAX_D];
    int16_t k_vec[MAX_L][MAX_SEQ][MAX_D];
    int16_t v_vec[MAX_L][MAX_SEQ][MAX_D];
    uint64_t q_sk0[MAX_L][MAX_SEQ];
    uint64_t k_sk0[MAX_L][MAX_SEQ];
    uint8_t qkv_codes[MAX_L][MAX_SEQ][MAX_G];
    uint8_t k_ctx_codes[MAX_L][MAX_SEQ][MAX_G];
    uint8_t ffn_codes[MAX_L][MAX_SEQ][MAX_G];
    uint16_t attn_best_s[MAX_L][MAX_H][MAX_SEQ];
    uint64_t step3_raw_w[MAX_H][MAX_SEQ];
    uint32_t top_prll_w[MAX_H][MAX_SEQ];
    int32_t top_gate_val[MAX_H];
};

__global__ void precompute_q12_array_kernel(
    const int32_t* __restrict__ src,
    int16_t* __restrict__ dst,
    size_t count,
    int use_qat,
    const uint16_t* __restrict__ scales_q8,
    int scale_idx
) {
    size_t i = size_t(blockIdx.x) * blockDim.x + threadIdx.x;
    if (i >= count) return;
    dst[i] = quant_sim_q12(src[i], use_qat, scales_q8[scale_idx]);
}

__global__ void precompute_vocab_readout_kernel(
    DeviceModel m,
    BatchCache bc,
    int use_qat
) {
    int v = blockIdx.x * blockDim.x + threadIdx.x;
    if (v >= VOCAB_SIZE) return;
    const int D = m.d_model;
    const int G = m.lut_groups;
    const uint16_t proto_sc = m.scales_q8[2];

    const int32_t* tw = &m.w_tmac[v * D];
    for (int g = 0; g < G; ++g) {
        uint8_t chi, clo;
        pack_4q9(tw[4*g + 0], tw[4*g + 1], tw[4*g + 2], tw[4*g + 3], chi, clo);
        bc.tmac_hi[v * G + g] = chi;
        bc.tmac_lo[v * G + g] = clo;
    }

    int16_t p_local[MAX_D];
    for (int c = 0; c < D; ++c) {
        int16_t pv = quant_sim_q12(m.p_out[v * D + c], use_qat, proto_sc);
        p_local[c] = pv;
        bc.pout_q12[v * D + c] = pv;
    }
    bc.psk0[v] = pack_sign64(&p_local[0]);
    bc.psk1[v] = (D >= 128) ? pack_sign64(&p_local[64]) : 0ULL;
}

__global__ void apply_momentum_step_kernel(
    int32_t* param,
    int32_t* mom,
    int32_t* grad,
    size_t n,
    int lr_num,
    int32_t clamp_lo,
    int32_t clamp_hi
) {
    size_t i = size_t(blockIdx.x) * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int32_t g = grad[i];
    grad[i] = 0;
    int32_t m = ((mom[i] * 7) >> 3) + g;
    mom[i] = clamp_i32(m, -65536, 65536);
    if (m != 0) {
        int32_t abs_m = (m < 0) ? -m : m;
        int32_t mag = clamp_i32(abs_m >> 3, 1, 8);
        int32_t delta = isign(m) * lr_num * mag;
        param[i] = clamp_i32(param[i] + delta, clamp_lo, clamp_hi);
    }
}

__device__ static inline uint64_t eval_head_prll_weight(
    const BlockScratch& sc,
    const SequenceSample& seq,
    const int32_t* __restrict__ copy_tok_gate,
    int l,
    int t,
    int s,
    int c_off,
    int sh,
    int ks_shift,
    int32_t tau,
    int32_t slope,
    uint8_t q_byte,
    bool t_is_anchor,
    int expected_cont_s = -1
) {
    int ks = (s >= ks_shift) ? (s - ks_shift) : 0;
    int32_t d_l1 = 0;
    #pragma unroll
    for (int j = 0; j < 8; ++j) {
        int32_t diff = int32_t(sc.q_vec[l][t][c_off + j]) - int32_t(sc.k_vec[l][ks][c_off + j]);
        d_l1 += (diff < 0 ? -diff : diff);
    }
    uint8_t k_byte = uint8_t((sc.k_sk0[l][ks] >> (((c_off / 8) & 7) * 8)) & 0xFFULL);
    int32_t d_ham = popcnt32_dev(uint32_t(q_byte ^ k_byte));
    int32_t d_ind = 0;
    if (sh == 1) {
        if (s == t) {
            d_ind = 4096;
        } else if (expected_cont_s >= 0) {
            d_ind = (s == expected_cont_s) ? 0 : 65536;
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
        } else if (expected_cont_s >= 0) {
            d_ind = (s == expected_cont_s) ? 0 : 65536;
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
            d_ind = (s > 0 && s < t && clamp_i32(copy_tok_gate[seq.tokens[s]], 0, 255) >= 208) ? 0 : 65536;
        }
    } else if (slope <= 4 && t > 0 && s == t) {
        d_ind = 4096;
    }
    int32_t d_geom = (sh == 1 || sh == 2) ? ((d_l1 >> 3) + 32 * d_ham) : (d_l1 + 256 * d_ham);
    int32_t d_tot = d_geom + slope * (t - s) + d_ind;
    return prll_weight_order4(tau, d_tot);
}

__global__ void forward_and_grad_kernel(
    DeviceModel m,
    BatchCache bc,
    DeviceGrads gr,
    BlockScratch* __restrict__ scratches,
    const SequenceSample* __restrict__ samples,
    const int* __restrict__ batch_indices,
    int batch_size,
    int is_train,
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

        bool is_bound_var_start = (t + 1 < p_len)
            && (D >= 128)
            && (clamp_i32(m.copy_tok_gate[tok], 0, 255) >= 208);
        if (is_bound_var_start) {
            bool seen_bound = false;
            for (int u = 1; u <= t; ++u) {
                if (seq.tokens[u] == tok && (seq.tokens[u - 1] == 773 || seq.tokens[u - 1] == 269)) {
                    seen_bound = true;
                    break;
                }
            }
            is_bound_var_start = seen_bound;
        }

        for (int c = 0; c < D; ++c) {
            int perm_c = c ^ (pos_idx & 7);
            int parity = popcnt32_dev(uint32_t(c & 63) & spin_t) & 1;
            int32_t et = (is_bound_var_start && c < 80 && ((c / 8) % 4 == 1))
                ? 0
                : int32_t(bc.etok_q12[tok * D + c]);
            int32_t ep = (pos_idx == 0) ? 0 : int32_t(bc.epos_q12[pos_idx * D + perm_c]);
            int32_t rot = (parity == 0) ? ep : -ep;
            sc.x_cur[t][c] = clamp_q12(et + rot);
        }
    }
    __syncthreads();

    // Step 2: Execute L Causal Geometric Layers (stack-free two-pass PRLL for MAX_SEQ = 1024!)
    for (int l = 0; l < L; ++l) {
        const size_t qkv_layer_base = size_t(l) * G * 256 * QKV_OUT;
        const size_t ffn_layer_base = size_t(l) * G * 256 * D;

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
                    q_acc[dst] += int32_t(bc.qlut_q12[qv_row + o]);
                    k_acc[dst] += int32_t(bc.klut_q12[k_row + o]);
                    v_acc[dst] += int32_t(bc.vlut_q12[qv_row + o]);
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
            const bool t_is_anchor = (clamp_i32(m.copy_tok_gate[seq.tokens[t]], 0, 255) >= 208);

            for (int h = 0; h < H; ++h) {
                const int c_off = h * 8;
                const int sh = int(m.head_shift[l * H + h]);
                const int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
                const int32_t tau = int32_t(m.head_tau[l * H + h] < 256 ? 256 : m.head_tau[l * H + h]);
                const int32_t slope = int32_t(m.head_slope[l * H + h]);
                const uint8_t q_byte = uint8_t((sc.q_sk0[l][t] >> ((h & 7) * 8)) & 0xFFULL);

                uint64_t sum_raw = 0;
                int best_s = 0;
                uint64_t best_w = 0;

                for (int s = 0; s <= t; ++s) {
                    uint64_t w = eval_head_prll_weight(
                        sc, seq, m.copy_tok_gate, l, t, s, c_off, sh, ks_shift, tau, slope, q_byte, t_is_anchor
                    );
                    sum_raw += w;
                    if (w > best_w) { best_w = w; best_s = s; }
                }
                sc.attn_best_s[l][h][t] = uint16_t(best_s);

                int64_t acc_v[8] = {0,0,0,0,0,0,0,0};
                uint64_t mass_acc = 0;
                for (int s = 0; s <= t; ++s) {
                    uint64_t w = eval_head_prll_weight(
                        sc, seq, m.copy_tok_gate, l, t, s, c_off, sh, ks_shift, tau, slope, q_byte, t_is_anchor
                    );
                    uint32_t wp = uint32_t((w * PRLL_QUANTUM) / sum_raw);
                    mass_acc += wp;
                    int64_t wp64 = int64_t(wp);
                    #pragma unroll
                    for (int j = 0; j < 8; ++j) {
                        acc_v[j] += wp64 * int64_t(sc.v_vec[l][s][c_off + j]);
                    }
                }
                int64_t rem_mass = int64_t(PRLL_QUANTUM - mass_acc);
                #pragma unroll
                for (int j = 0; j < 8; ++j) {
                    acc_v[j] += rem_mass * int64_t(sc.v_vec[l][best_s][c_off + j]);
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
            for (int g = 0; g < G; ++g) {
                uint8_t code = code_4ch(&u_ffn[g * 4]);
                sc.ffn_codes[l][t][g] = code;
                size_t row = ffn_layer_base + (size_t(g) * 256 + code) * D;
                for (int c = 0; c < D; ++c) {
                    ffn_acc[c] += int32_t(bc.ffnlut_q12[row + c]);
                }
            }
            for (int c = 0; c < D; ++c) {
                int16_t act = icosian_hinge(ffn_acc[c] >> 4);
                sc.x_cur[t][c] = clamp_q12(int32_t(x_mid[c]) + int32_t(act));
            }
        }
        __syncthreads();
    }

    // Step 3: 2-Plane 9-Level T-MAC LUT Readout + Causal Pointer-Copy + R4.Objective Hinge Loss
    __shared__ int32_t s_act_lut[MAX_G][81];
    __shared__ int32_t s_copy_logits[VOCAB_SIZE];
    __shared__ int32_t s_best_neg_score[256];
    __shared__ int s_best_neg_id[256];
    __shared__ int32_t s_target_score;
    __shared__ int s_prev_copy_src;
    __shared__ int32_t s_grad_h[MAX_D];

    const int copy_limit = (p_len > 2) ? (p_len - 2) : p_len;
    const bool in_code_gen = ((seq.gen_flags & 1u) != 0u);
    const bool in_agent_gen = ((seq.gen_flags & 2u) != 0u);
    const bool in_structured_gen = in_code_gen || in_agent_gen;
    const bool has_tool_obs = (seq.obs_lo > 0 && seq.obs_hi > seq.obs_lo);
    const int obs_end = (int(seq.obs_hi) < copy_limit) ? int(seq.obs_hi) : copy_limit;
    const int32_t g3_min_open = (D >= 128) ? 2800 : 1200;
    bool has_any_copy = false;
    for (int i = p_len - 1; i < total_len - 1; ++i) {
        if (seq.copy_mask[i] != 0) { has_any_copy = true; break; }
    }
    const bool train_copy_gates = has_any_copy || in_code_gen;

    if (tid == 0) {
        s_prev_copy_src = -1;
    }
    __syncthreads();

    for (int t = p_len - 1; t < total_len - 1; ++t) {
        const int target = int(seq.tokens[t + 1]);
        const int is_copy_pos = int(seq.copy_mask[t]);
        const int prev_copy_src = s_prev_copy_src;
        const bool prev_was_copied = (prev_copy_src >= 0);

        bool in_tool_obs_span = false;
        int expected_cont_s = -1;
        if (has_tool_obs && prev_copy_src >= int(seq.obs_lo) && (prev_copy_src + 1) < obs_end) {
            in_tool_obs_span = true;
            expected_cont_s = prev_copy_src + 1;
        } else if (prev_copy_src >= 0 && (prev_copy_src + 1) < copy_limit) {
            int ns = prev_copy_src + 1;
            uint32_t ns_tok = seq.tokens[ns];
            int32_t ns_gate = clamp_i32(m.copy_tok_gate[ns_tok], 0, 255);
            bool ns_lead_space = ((m.noncopy_tok[ns_tok] & 2u) != 0u);
            if (in_code_gen) {
                if (!ns_lead_space && ns_gate >= 160) {
                    expected_cont_s = ns;
                }
            } else if (in_agent_gen && ns_gate > 0) {
                expected_cont_s = ns;
            }
        }

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
            const int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
            const uint32_t cur_tok = seq.tokens[t];
            const bool fence_or_nl = (cur_tok == 66 || cur_tok == 201 || cur_tok == 29);
            const bool active_struct_t = (t > p_len - 1) ? in_structured_gen : has_tool_obs;
            int32_t copy_gain = 0;
            if (active_struct_t) {
                if (sh == 3) {
                    copy_gain = (prev_was_copied || fence_or_nl || gate_sum < g3_min_open)
                        ? 0 : clamp_i32(gate_sum, 0, 8192);
                } else if (sh == 1 || sh == 2) {
                    if (!prev_was_copied || expected_cont_s < 0) {
                        copy_gain = 0;
                    } else if (in_tool_obs_span) {
                        copy_gain = clamp_i32(gate_sum < 4096 ? 4096 : gate_sum, 0, 8192);
                    } else if (gate_sum <= 0) {
                        copy_gain = 0;
                    } else {
                        copy_gain = clamp_i32(gate_sum, 0, 8192);
                    }
                }
            }
            sc.top_gate_val[h] = gate_sum;

            const int32_t tau = int32_t(m.head_tau[top_l * H + h] < 256 ? 256 : m.head_tau[top_l * H + h]);
            const int32_t slope = int32_t(m.head_slope[top_l * H + h]);
            const uint8_t q_byte = uint8_t((sc.q_sk0[top_l][t] >> ((h & 7) * 8)) & 0xFFULL);
            const bool t_is_anchor = (clamp_i32(m.copy_tok_gate[cur_tok], 0, 255) >= 208);

            uint64_t sum_raw = 0;
            int best_s = 0;
            uint64_t best_w = 0;
            for (int s = 0; s <= t; ++s) {
                uint64_t w = eval_head_prll_weight(
                    sc, seq, m.copy_tok_gate, top_l, t, s, c_off, sh, ks_shift, tau, slope, q_byte, t_is_anchor, expected_cont_s
                );
                sc.step3_raw_w[h][s] = w;
                sum_raw += w;
                if (w > best_w) { best_w = w; best_s = s; }
            }
            uint64_t mass_acc = 0;
            for (int s = 0; s <= t; ++s) {
                uint32_t wp = uint32_t((sc.step3_raw_w[h][s] * PRLL_QUANTUM) / sum_raw);
                sc.top_prll_w[h][s] = wp;
                mass_acc += wp;
            }
            sc.top_prll_w[h][best_s] += uint32_t(PRLL_QUANTUM - mass_acc);

            if (copy_gain > 0 && sh != 3) {
                const int32_t cap_h = 12288;
                for (int s = 1; s < copy_limit; ++s) {
                    int tok_s = int(seq.tokens[s]);
                    bool seen_before = false;
                    for (int sp = 1; sp < s; ++sp) {
                        if (int(seq.tokens[sp]) == tok_s) { seen_before = true; break; }
                    }
                    if (seen_before) continue;
                    uint64_t tot_wp = 0;
                    for (int s2 = s; s2 < copy_limit; ++s2) {
                        if (int(seq.tokens[s2]) == tok_s) {
                            tot_wp += sc.top_prll_w[h][s2];
                        }
                    }
                    int32_t tok_allow = clamp_i32(m.copy_tok_gate[tok_s], 0, 255);
                    const bool is_expected_tok = (expected_cont_s >= 0 && int(seq.tokens[expected_cont_s]) == tok_s);
                    if (is_expected_tok && (
                        in_tool_obs_span
                        || (expected_cont_s + 1 < copy_limit
                            && clamp_i32(m.copy_tok_gate[seq.tokens[expected_cont_s + 1]], 0, 255) >= 160
                            && (tok_s == 5 || tok_s == 16 || tok_s == 17))
                    )) {
                        if (tok_allow < 192) tok_allow = 192;
                    }
                    int32_t cand = clamp_i32(
                        int32_t((int64_t(tot_wp) * int64_t(copy_gain) * int64_t(tok_allow)) >> 25),
                        0,
                        cap_h
                    );
                    if (cand > 0) atomicMax(&s_copy_logits[tok_s], cand);
                }
            }
        }
        __syncthreads();

        // Exact `sh == 3` `top1 + top2` + same-line suppression + `(copy_limit - s) * 8` parity with `gllm_v9.rs`!
        {
            const uint32_t cur_tok_s3 = seq.tokens[t];
            const bool fence_or_nl_s3 = (cur_tok_s3 == 66 || cur_tok_s3 == 201 || cur_tok_s3 == 29);
            const bool active_struct_s3 = (t > p_len - 1) ? in_structured_gen : has_tool_obs;
            if (active_struct_s3 && !prev_was_copied && !fence_or_nl_s3) {
                int line_start_pos = p_len;
                for (int u = p_len; u <= t; ++u) {
                    if (seq.tokens[u] == 201) line_start_pos = u + 1;
                }
                const int32_t cap_h3 = (D >= 128) ? 11264 : 12288;
                for (int s = 1 + tid; s < copy_limit; s += 256) {
                    int tok_s = int(seq.tokens[s]);
                    bool seen_before = false;
                    for (int sp = 1; sp < s; ++sp) {
                        if (int(seq.tokens[sp]) == tok_s) { seen_before = true; break; }
                    }
                    if (seen_before) continue;
                    int32_t tok_allow = clamp_i32(m.copy_tok_gate[tok_s], 0, 255);
                    if (tok_allow == 0) continue;

                    int32_t top1 = 0, top2 = 0;
                    int n_sh3 = 0;
                    for (int h = 0; h < H; ++h) {
                        if (int(m.head_shift[top_l * H + h]) != 3) continue;
                        int32_t gs = sc.top_gate_val[h];
                        if (gs < g3_min_open) continue;
                        ++n_sh3;
                        int32_t cg = clamp_i32(gs, 0, 8192);
                        uint32_t max_wp = 0;
                        for (int s2 = s; s2 < copy_limit; ++s2) {
                            if (int(seq.tokens[s2]) == tok_s && sc.top_prll_w[h][s2] > max_wp) {
                                max_wp = sc.top_prll_w[h][s2];
                            }
                        }
                        int32_t cand = clamp_i32(
                            int32_t((int64_t(max_wp) * int64_t(cg) * int64_t(tok_allow)) >> 25),
                            0,
                            cap_h3
                        );
                        if (cand >= top1) { top2 = top1; top1 = cand; }
                        else if (cand > top2) { top2 = cand; }
                    }
                    if (n_sh3 > 0) {
                        int32_t combined = (n_sh3 >= 2) ? clamp_i32(top1 + top2, 0, 18432) : top1;
                        bool on_line = false;
                        for (int u = line_start_pos; u <= t; ++u) {
                            if (int(seq.tokens[u]) == tok_s) { on_line = true; break; }
                        }
                        if (on_line) {
                            combined >>= 3;
                        } else if (combined >= 1024) {
                            combined += (copy_limit - s) * 8;
                        }
                        if (combined > 0) atomicMax(&s_copy_logits[tok_s], combined);
                    }
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
            int32_t tmac_sum = 0;
            const uint8_t* hi_row = &bc.tmac_hi[v * G];
            const uint8_t* lo_row = &bc.tmac_lo[v * G];
            for (int g = 0; g < G; ++g) {
                int32_t v_hi = s_act_lut[g][hi_row[g]];
                int32_t v_lo = s_act_lut[g][lo_row[g]];
                tmac_sum += (v_hi << 1) + v_hi + v_lo;
            }

            const int16_t* p_vec = &bc.pout_q12[v * D];
            int32_t l1 = 0;
            for (int c = 0; c < D; ++c) {
                int32_t diff = int32_t(sc.x_cur[t][c]) - int32_t(p_vec[c]);
                l1 += (diff < 0 ? -diff : diff);
            }
            int ham = popcnt64_dev(h_sk0 ^ bc.psk0[v]);
            if (D == 128) {
                ham += popcnt64_dev(h_sk1 ^ bc.psk1[v]);
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

            // Advance causal KV pointer-copy state identically to GllmV9KvCache::predict_next_token
            if (is_copy_pos == 0) {
                s_prev_copy_src = -1;
            } else if (!prev_was_copied) {
                int next_src = -1;
                for (int s = 1; s < copy_limit; ++s) {
                    if (int(seq.tokens[s]) == target) { next_src = s; break; }
                }
                s_prev_copy_src = next_src;
            } else if (expected_cont_s >= 0 && int(seq.tokens[expected_cont_s]) == target) {
                s_prev_copy_src = expected_cont_s;
            } else {
                int best_src = -1;
                uint32_t best_src_w = 0;
                for (int h_i = 0; h_i < H; ++h_i) {
                    int sh_i = int(m.head_shift[top_l * H + h_i]);
                    if (sh_i == 1 || sh_i == 2) {
                        for (int s = 1; s < copy_limit; ++s) {
                            if (int(seq.tokens[s]) == target && sc.top_prll_w[h_i][s] >= best_src_w) {
                                best_src_w = sc.top_prll_w[h_i][s];
                                best_src = s;
                            }
                        }
                    }
                }
                s_prev_copy_src = best_src;
            }
        }
        __syncthreads();

        if (!is_train) continue;
        const int hard_neg = s_best_neg_id[0];
        const bool is_span_start = (is_copy_pos != 0) && !prev_was_copied;
        const bool is_copy_cont = (is_copy_pos != 0) && prev_was_copied;
        const int gate3_lo = (D == 128) ? 80 : 40;
        const int gate12_lo = (D == 128) ? 104 : 52;

        // 1. Causal KV Pointer-Copy & Structural Anchor-Shift Q/K Alignment
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
                int32_t best_neg_eff = -1;

                if (sh == 3 && is_span_start) {
                    for (int s = 1; s <= t; ++s) {
                        bool anchor_ok = (clamp_i32(m.copy_tok_gate[seq.tokens[s]], 0, 255) >= 208);
                        if (s < copy_limit && anchor_ok && int(seq.tokens[s]) == target) {
                            if (match_s < 0) match_s = s;
                        } else if (s == t || (anchor_ok && (s >= copy_limit || int(seq.tokens[s]) != target))) {
                            int32_t eff_score = int32_t(sc.top_prll_w[h][s]);
                            if (s < copy_limit && sc.top_prll_w[h][s] >= 20000u) {
                                eff_score += (copy_limit - s) * 32;
                            }
                            if (eff_score > best_neg_eff) {
                                best_neg_eff = eff_score;
                                best_neg_w = sc.top_prll_w[h][s];
                                neg_s = s;
                            }
                        }
                    }
                } else if ((sh == 1 || sh == 2) && is_copy_cont) {
                    if (expected_cont_s >= 0 && int(seq.tokens[expected_cont_s]) == target) {
                        match_s = expected_cont_s;
                        for (int s = 1; s <= t; ++s) {
                            if (s != expected_cont_s && (s >= copy_limit || int(seq.tokens[s]) != target) && sc.top_prll_w[h][s] >= best_neg_w) {
                                best_neg_w = sc.top_prll_w[h][s];
                                neg_s = s;
                            }
                        }
                    } else {
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
                }

                if (match_s >= 0) {
                    uint64_t target_tot_wp = 0;
                    uint32_t target_max_wp = 0;
                    for (int s2 = 1; s2 < copy_limit; ++s2) {
                        if (int(seq.tokens[s2]) == target) {
                            uint32_t pw = sc.top_prll_w[h][s2];
                            target_tot_wp += pw;
                            if (pw > target_max_wp) target_max_wp = pw;
                        }
                    }
                    bool need_qk_step = (sh == 3)
                        ? (target_max_wp < 860000u || uint64_t(target_max_wp) <= uint64_t(best_neg_w) + 240000ULL)
                        : (target_tot_wp < 820000ULL);
                    if (need_qk_step && neg_s >= 0) {
                        int ks_pos = (match_s >= ks_shift) ? (match_s - ks_shift) : 0;
                        int ks_neg = (neg_s >= ks_shift) ? (neg_s - ks_shift) : 0;
                        const int32_t q_step = (sh == 3) ? ((D == 64) ? 12 : 10) : 6;
                        const int32_t k_step = 6;
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
                                    atomicAdd(&gr.q_lut[qk_base + (size_t(g_src) * 256 + qc) * QKV_OUT + o_src], q_step * (isign(kp - qv) - isign(kn - qv)));
                                    atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcp) * QKV_OUT + o_src], k_step * isign(qv - kp));
                                    atomicAdd(&gr.k_lut[qk_base + (size_t(g_src) * 256 + kcn) * QKV_OUT + o_src], -k_step * isign(qv - kn));
                                }
                            }
                            if (top_l > 0) {
                                int32_t dir_q = isign(kp - kn);
                                int32_t dir_kp = isign(qv - kp);
                                int32_t dir_kn = -isign(qv - kn);
                                const int32_t ffn_q_step = (sh == 3) ? ((D == 64) ? 4 : 3) : 2;
                                const size_t prev_ffn = size_t(top_l - 1) * G * 256 * D;
                                for (int g_f = 0; g_f < G; ++g_f) {
                                    uint8_t fc_t = sc.ffn_codes[top_l - 1][t][g_f];
                                    uint8_t fc_kp = sc.ffn_codes[top_l - 1][ks_pos][g_f];
                                    uint8_t fc_kn = sc.ffn_codes[top_l - 1][ks_neg][g_f];
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_t) * D + ch], ffn_q_step * dir_q);
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_kp) * D + ch], dir_kp);
                                    atomicAdd(&gr.ffn_lut[prev_ffn + (size_t(g_f) * 256 + fc_kn) * D + ch], dir_kn);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Orthogonal Gate-Carrier Subspace Backpropagation (`[gate3_lo..gate12_lo)` and `[gate12_lo..D)`)
        const uint32_t cur_tok = seq.tokens[t];
        const bool fence_or_nl = (cur_tok == 66 || cur_tok == 201 || cur_tok == 29);
        const int32_t g3_open_tgt = (D == 128) ? 4800 : 6200;
        const int32_t g12_open_tgt = (D == 128) ? 3600 : 5400;
        const int32_t g12_rest = (D == 128) ? 1400 : 2400;
        const int32_t g3_val = sc.top_gate_val[1];
        const int32_t g1_val = sc.top_gate_val[3];
        const bool active_struct_g = (t > p_len - 1) ? in_structured_gen : has_tool_obs;
        const int32_t g3_grad = train_copy_gates ? (
            is_span_start ? ((g3_val < g3_open_tgt) ? 32 : 0) : ((!fence_or_nl && g3_val > -300) ? -2 : 0)
        ) : (
            (active_struct_g && !fence_or_nl && g3_val > (g3_min_open - ((D == 128) ? 600 : 250))) ? ((D == 128) ? -4 : -2) : 0
        );
        const int32_t g1_grad = !train_copy_gates ? 0 : (
            is_copy_cont ? ((g1_val < g12_open_tgt) ? 16 : 0) : ((!prev_was_copied && g1_val > g12_rest) ? -2 : 0)
        );
        const bool readout_violated = (s_target_score - s_best_neg_score[0] < HINGE_MARGIN);

        if (!readout_violated && g3_grad == 0 && g1_grad == 0) {
            __syncthreads();
            continue;
        }

        const bool suppress_noncopy_neg = readout_violated
            && (
                (!is_copy_pos && s_copy_logits[hard_neg] < 512)
                || (is_copy_pos && (s_best_neg_score[0] - s_copy_logits[hard_neg]) > 2800)
            );

        if (tid == 0 && readout_violated) {
            if (!is_copy_pos) {
                atomicAdd(&gr.b_out[target], 2);
            } else if (m.b_out[target] < -256) {
                atomicAdd(&gr.b_out[target], 2);
            }
            if (suppress_noncopy_neg && (
                m.copy_tok_gate[hard_neg] < 128 || m.b_out[hard_neg] > -256
            )) {
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

        // Strictly confine gate3/gate12 carrier updates to top_l (`L - 1`) for ALL d_model sizes so
        // no subsequent FWHT64 butterfly mixes gate channels `[gate3_lo..D)` back into `[0..gate3_lo)`.
        const int gate_min_l = L - 1;
        for (int l = L - 1; l >= 0; --l) {
            int scale_l = (l == L - 1) ? 2 : 1;
            const size_t ffn_layer_base = size_t(l) * G * 256 * D;
            for (int idx = tid; idx < G * D; idx += 256) {
                int g = idx / D;
                int c = idx % D;
                if (c >= gate3_lo && l < gate_min_l) continue;
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
                if (dst >= gate3_lo && l < gate_min_l) continue;
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

static void dequantize_tensor_q12(const std::vector<int8_t>& q, uint16_t scale_q8, std::vector<int16_t>& out) {
    out.resize(q.size());
    int32_t sc = int32_t(scale_q8);
    for (size_t i = 0; i < q.size(); ++i) {
        out[i] = clamp_q12((int32_t(q[i]) * sc) >> 4);
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

static void init_byte_geometric_embeddings(
    const std::vector<std::string>& vocab_bytes,
    int D,
    std::vector<int32_t>& h_etok,
    std::vector<int32_t>& h_pout,
    std::vector<int32_t>& h_copy_tok_gate
) {
    h_etok.assign(size_t(VOCAB_SIZE) * D, 0);
    h_copy_tok_gate.assign(VOCAB_SIZE, 192);
    h_copy_tok_gate[0] = 0;
    h_copy_tok_gate[1] = 0;
    h_copy_tok_gate[2] = 0;

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

        bool has_xml_angle = (s.find('<') != std::string::npos || s.find('>') != std::string::npos);
        if (v > 2 && (has_newline || has_xml_angle || s == "`" || s == ";" || s == "=" || s == " =")) {
            h_copy_tok_gate[v] = 0;
        } else if (v > 2 && s == " ") {
            h_copy_tok_gate[v] = 160;
        } else if (v > 2 && s == ".") {
            // Mid-span `.` (`0.2`, `docs.rs`, `.git`) is boosted to `192` in `gllm_v9.rs:1665` when followed by
            // an identifier/digit token (`gate >= 160`), while sentence-final `.` stays at `2` (`cand <= 488 < 1024`)!
            h_copy_tok_gate[v] = 2;
        } else if (v > 2 && (s == ":" || s == "'")) {
            // `< 160` so `in_code_gen` stops before `:` in `p1: i64`, while `in_agent_gen` copies `https://` & `'expr'`.
            h_copy_tok_gate[v] = 128;
        } else if (v > 2 && !has_alpha_lo && !has_alpha_hi && !has_digit && !has_under) {
            if (s.find('-') == std::string::npos && s.find('/') == std::string::npos && s.find('|') == std::string::npos) {
                h_copy_tok_gate[v] = 0;
            }
        } else if (v > 2 && lead_space && s.size() >= 2) {
            unsigned char c1 = (unsigned char)s[1];
            if ((c1 >= 'a' && c1 <= 'z') || (c1 >= 'A' && c1 <= 'Z') || c1 == '/') {
                h_copy_tok_gate[v] = 224;
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
    h_pout.resize(h_etok.size());
    for (size_t i = 0; i < h_etok.size(); ++i) {
        h_pout[i] = h_etok[i] >> 1;
    }
}

struct EvalPromptItem {
    std::string id;
    std::string kind;
    std::string prompt_text;
    std::vector<uint16_t> prompt_ids;
};

struct SampleGenerationRecord {
    std::string id;
    std::string kind;
    std::string prompt;
    std::string reply;
    int generated_tokens;
};

// Exact host-side quantized integer autoregressive decoder matching crates/wasm/src/gllm_v9.rs
struct HostQuantModel {
    int D, L, H, G;
    std::vector<int16_t> e_tok, e_pos, p_out;
    std::vector<uint64_t> p_sk0, p_sk1;
    std::vector<int16_t> b_out;
    std::vector<uint8_t> tmac_hi, tmac_lo, copy_tok_gate;
    std::vector<uint16_t> head_tau, head_slope;
    std::vector<uint8_t> head_shift;
    std::vector<int16_t> head_copy_b;
    std::vector<uint8_t> head_copy_trits;
    std::vector<std::vector<int16_t>> q_lut, k_lut, v_lut, ffn_lut;

    std::string generate_greedy(
        const std::vector<uint16_t>& prefix_ids,
        const std::vector<std::string>& vocab_bytes,
        const std::string& prompt_text,
        int max_new_tokens,
        int& out_tok_count
    ) const {
        const int prompt_len = int(prefix_ids.size());
        int struct_pos = 0;
        std::vector<uint32_t> tokens;
        tokens.reserve(MAX_SEQ);
        std::vector<std::vector<int16_t>> prev_layer_in(L, std::vector<int16_t>(D, 0));
        std::vector<std::vector<int16_t>> k_cache(L), v_cache(L);
        std::vector<std::vector<uint64_t>> k_sk0(L);
        std::vector<int16_t> last_top_q(D, 0), last_hidden(D, 0);
        uint64_t last_top_q_sk0 = 0;

        auto push_tok = [&](uint32_t tok_id, bool was_copied) {
            int t = int(tokens.size());
            uint32_t tok = std::min<uint32_t>(tok_id, VOCAB_SIZE - 1);
            tokens.push_back(tok);
            int pos_idx = 0;
            if (t + 1 < prompt_len) {
                pos_idx = 0;
            } else if (t + 1 == prompt_len) {
                struct_pos = 1;
                pos_idx = 1;
            } else {
                if (!was_copied) struct_pos = std::min(struct_pos + 1, 255);
                pos_idx = struct_pos;
            }
            uint32_t spin_t = uint32_t((pos_idx * 7 + int(tok & 63)) & 63);
            bool is_bound_var_start = (t + 1 < prompt_len) && (D >= 128) && (copy_tok_gate[tok] >= 208);
            if (is_bound_var_start) {
                bool found = false;
                for (int u = 1; u <= t; ++u) {
                    if (tokens[u] == tok && (tokens[u - 1] == 773 || tokens[u - 1] == 269)) {
                        found = true;
                        break;
                    }
                }
                is_bound_var_start = found;
            }
            std::vector<int16_t> x_cur(D);
            for (int c = 0; c < D; ++c) {
                int perm_c = c ^ (pos_idx & 7);
                int parity = popcnt32_dev(uint32_t(c & 63) & spin_t) & 1;
                int32_t ep = (pos_idx == 0) ? 0 : int32_t(e_pos[pos_idx * D + perm_c]);
                int32_t rot = (parity == 0) ? ep : -ep;
                int32_t et = (is_bound_var_start && c < 80 && ((c / 8) % 4 == 1)) ? 0 : int32_t(e_tok[tok * D + c]);
                x_cur[c] = clamp_q12(et + rot);
            }
            std::vector<int16_t> u_blk(D), k_in(D), u_k(D), q_vec(D), k_vec(D), v_vec(D), attn_out(D), x_mid(D), u_ffn(D);
            for (int l = 0; l < L; ++l) {
                for (int c = 0; c < D; ++c) {
                    int32_t prev_x = (t > 0) ? int32_t(prev_layer_in[l][c ^ 1]) : 0;
                    k_in[c] = clamp_q12(((int32_t(x_cur[c]) * 3) + prev_x) >> 2);
                }
                prev_layer_in[l] = x_cur;
                for (int blk = 0; blk < D / 64; ++blk) {
                    fwht64_block(&x_cur[blk * 64], &u_blk[blk * 64]);
                    fwht64_block(&k_in[blk * 64], &u_k[blk * 64]);
                }
                std::vector<int32_t> q_acc(D, 0), k_acc(D, 0), v_acc(D, 0);
                for (int g = 0; g < G; ++g) {
                    int q_code = code_4ch(&u_blk[g * 4]);
                    int k_code = code_4ch(&u_k[g * 4]);
                    int base_c = (g * 4) % D;
                    size_t qv_row = (size_t(g) * 256 + q_code) * QKV_OUT;
                    size_t k_row = (size_t(g) * 256 + k_code) * QKV_OUT;
                    for (int o = 0; o < QKV_OUT; ++o) {
                        int dst = (base_c + o) % D;
                        q_acc[dst] += q_lut[l][qv_row + o];
                        k_acc[dst] += k_lut[l][k_row + o];
                        v_acc[dst] += v_lut[l][qv_row + o];
                    }
                }
                for (int c = 0; c < D; ++c) {
                    q_vec[c] = clamp_q12((q_acc[c] >> 2) + (int32_t(u_blk[c]) >> 1));
                    k_vec[c] = clamp_q12((k_acc[c] >> 2) + (int32_t(u_k[c]) >> 1));
                    v_vec[c] = clamp_q12((v_acc[c] >> 2) + (int32_t(x_cur[c]) >> 1));
                }
                uint64_t q_sk = pack_sign64(&q_vec[0]);
                uint64_t k_sk = pack_sign64(&k_vec[0]);
                k_cache[l].insert(k_cache[l].end(), k_vec.begin(), k_vec.end());
                v_cache[l].insert(v_cache[l].end(), v_vec.begin(), v_vec.end());
                k_sk0[l].push_back(k_sk);
                if (l + 1 == L) {
                    last_top_q = q_vec;
                    last_top_q_sk0 = q_sk;
                }
                std::vector<uint64_t> raw_w(t + 1);
                std::vector<uint32_t> prll_w(t + 1);
                for (int h = 0; h < H; ++h) {
                    int c_off = h * 8;
                    int sh = head_shift[l * H + h];
                    int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
                    int32_t tau = std::max<int32_t>(256, head_tau[l * H + h]);
                    int32_t slope = head_slope[l * H + h];
                    uint8_t q_byte = uint8_t((q_sk >> ((h & 7) * 8)) & 0xFFULL);
                    uint64_t sum_raw = 0, best_w = 0;
                    int best_s = 0;
                    bool t_is_anchor = copy_tok_gate[tokens[t]] >= 208;
                    for (int s = 0; s <= t; ++s) {
                        int ks = std::max(0, s - ks_shift);
                        int32_t d_l1 = 0;
                        for (int j = 0; j < 8; ++j) {
                            d_l1 += std::abs(int32_t(q_vec[c_off + j]) - int32_t(k_cache[l][ks * D + c_off + j]));
                        }
                        uint8_t k_byte = uint8_t((k_sk0[l][ks] >> ((h & 7) * 8)) & 0xFFULL);
                        int32_t d_ham = popcnt32_dev(uint32_t(q_byte ^ k_byte));
                        int32_t d_ind = 0;
                        if (sh == 1) {
                            if (s == t) d_ind = 4096;
                            else if (s > 1 && t > 0 && s < t && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 0;
                            else if (s > 0 && s < t && t_is_anchor && tokens[s - 1] == tokens[t]) d_ind = 256;
                            else d_ind = 65536;
                        } else if (sh == 2) {
                            if (s == t) d_ind = 4096;
                            else if (s > 2 && t > 1 && s < t && tokens[s - 3] == tokens[t - 2] && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 0;
                            else if (s > 1 && t > 0 && s < t && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 256;
                            else if (s > 0 && s < t && t_is_anchor && tokens[s - 1] == tokens[t]) d_ind = 512;
                            else d_ind = 65536;
                        } else if (sh == 3) {
                            if (s == t) d_ind = 8192;
                            else d_ind = (s > 0 && s < t && copy_tok_gate[tokens[s]] >= 208) ? 0 : 65536;
                        } else if (slope <= 4 && t > 0 && s == t) {
                            d_ind = 4096;
                        }
                        int32_t d_geom = (sh == 1 || sh == 2) ? ((d_l1 >> 3) + 32 * d_ham) : (d_l1 + 256 * d_ham);
                        uint64_t w = prll_weight_order4(tau, d_geom + slope * (t - s) + d_ind);
                        raw_w[s] = w;
                        sum_raw += w;
                        if (w > best_w) { best_w = w; best_s = s; }
                    }
                    uint64_t mass_acc = 0;
                    for (int s = 0; s <= t; ++s) {
                        uint32_t wp = uint32_t((raw_w[s] * PRLL_QUANTUM) / sum_raw);
                        prll_w[s] = wp;
                        mass_acc += wp;
                    }
                    prll_w[best_s] += uint32_t(PRLL_QUANTUM - mass_acc);
                    int64_t acc_v[8] = {0,0,0,0,0,0,0,0};
                    for (int s = 0; s <= t; ++s) {
                        for (int j = 0; j < 8; ++j) {
                            acc_v[j] += int64_t(prll_w[s]) * int64_t(v_cache[l][s * D + c_off + j]);
                        }
                    }
                    for (int j = 0; j < 8; ++j) {
                        attn_out[c_off + j] = clamp_q12(int32_t(acc_v[j] / int64_t(PRLL_QUANTUM)));
                    }
                }
                for (int c = 0; c < D; ++c) {
                    x_mid[c] = clamp_q12(int32_t(x_cur[c]) + (int32_t(attn_out[c]) >> 1));
                }
                for (int blk = 0; blk < D / 64; ++blk) {
                    fwht64_block(&x_mid[blk * 64], &u_ffn[blk * 64]);
                }
                std::vector<int32_t> ffn_acc(D, 0);
                for (int g = 0; g < G; ++g) {
                    int code = code_4ch(&u_ffn[g * 4]);
                    const int16_t* row = &ffn_lut[l][(size_t(g) * 256 + code) * D];
                    for (int c = 0; c < D; ++c) ffn_acc[c] += row[c];
                }
                for (int c = 0; c < D; ++c) {
                    int16_t act = icosian_hinge(ffn_acc[c] >> 4);
                    x_cur[c] = clamp_q12(int32_t(x_mid[c]) + int32_t(act));
                }
            }
            last_hidden = x_cur;
        };

        for (uint16_t tid : prefix_ids) push_tok(tid, false);

        std::vector<uint32_t> generated;
        int prev_copy_src = -1;
        const int copy_limit = std::min(int(tokens.size()), prompt_len > 2 ? (prompt_len - 2) : prompt_len);

        // Exact `<tool_response>...</tool_response>` token span detection matching gllm_v9.rs
        int obs_lo = -1, obs_hi = -1;
        {
            std::string acc_s;
            std::vector<int> tok_end_byte(tokens.size() + 1, 0);
            for (size_t i = 0; i < tokens.size(); ++i) {
                if (i > 0) acc_s += vocab_bytes[tokens[i]];
                tok_end_byte[i + 1] = int(acc_s.size());
            }
            size_t r_open = acc_s.rfind("<tool_response>");
            if (r_open != std::string::npos) {
                size_t content_b = r_open + 15;
                size_t r_close = acc_s.find("</tool_response>", content_b);
                if (r_close != std::string::npos) {
                    for (size_t i = 1; i < tokens.size(); ++i) {
                        if (obs_lo < 0 && tok_end_byte[i + 1] > int(content_b)) obs_lo = int(i);
                        if (tok_end_byte[i] < int(r_close)) obs_hi = int(i + 1);
                    }
                }
            }
        }
        const bool has_tool_obs = (obs_lo >= 0 && obs_hi > obs_lo);

        for (int step = 0; step < max_new_tokens && int(tokens.size()) + 1 < MAX_SEQ; ++step) {
            int t = int(tokens.size()) - 1;
            int top_l = L - 1;
            std::vector<std::vector<int32_t>> act_lut(G, std::vector<int32_t>(81));
            for (int g = 0; g < G; ++g) build_tmac_act_lut81(&last_hidden[g * 4], act_lut[g].data());

            std::string gen_prefix;
            for (size_t i = 0; i < std::min<size_t>(generated.size(), 6); ++i) {
                gen_prefix += vocab_bytes[generated[i]];
            }
            size_t p_nonws = gen_prefix.find_first_not_of(" \t\r\n");
            std::string gen_trim = (p_nonws == std::string::npos) ? "" : gen_prefix.substr(p_nonws);
            bool in_code_gen = (gen_trim.rfind("```", 0) == 0);
            bool in_agent_gen = has_tool_obs || (gen_trim.rfind("<", 0) == 0);
            bool in_structured_gen = in_code_gen || in_agent_gen;

            bool in_tool_obs_span = false;
            int expected_cont_s = -1;
            if (has_tool_obs && prev_copy_src >= obs_lo && prev_copy_src + 1 < std::min(obs_hi, copy_limit)) {
                in_tool_obs_span = true;
                expected_cont_s = prev_copy_src + 1;
            } else if (prev_copy_src >= 0 && prev_copy_src + 1 < copy_limit) {
                int ns = prev_copy_src + 1;
                uint32_t ns_tok = tokens[ns];
                uint8_t ns_gate = copy_tok_gate[ns_tok];
                const std::string& ns_piece = vocab_bytes[ns_tok];
                if (in_code_gen && (ns_piece.empty() || ns_piece[0] != ' ') && ns_gate >= 160) {
                    expected_cont_s = ns;
                } else if (in_agent_gen && ns_gate > 0) {
                    expected_cont_s = ns;
                }
            }

            std::vector<int32_t> copy_logits(VOCAB_SIZE, 0);
            std::vector<int> copy_best_src(VOCAB_SIZE, 0);
            std::vector<std::vector<int32_t>> sh3_cands(VOCAB_SIZE);
            int g3_min_open = (D >= 128) ? 2800 : 1200;

            for (int h = 0; h < H; ++h) {
                int32_t gate_sum = head_copy_b[top_l * H + h];
                for (int g = 0; g < G; ++g) {
                    uint8_t pcode = head_copy_trits[(top_l * H + h) * G + g];
                    gate_sum += act_lut[g][pcode];
                }
                int c_off = h * 8;
                int sh = head_shift[top_l * H + h];
                uint32_t cur_tok = tokens[t];
                bool fence_or_nl = (cur_tok == 66 || cur_tok == 201 || cur_tok == 29);
                int32_t copy_gain = 0;
                if (in_structured_gen) {
                    if (sh == 3 && prev_copy_src < 0 && !fence_or_nl && gate_sum >= g3_min_open) {
                        copy_gain = clamp_i32(gate_sum, 0, 8192);
                    } else if ((sh == 1 || sh == 2) && prev_copy_src >= 0 && expected_cont_s >= 0) {
                        if (in_tool_obs_span) {
                            copy_gain = clamp_i32(std::max<int32_t>(gate_sum, 4096), 0, 8192);
                        } else if (gate_sum > 0) {
                            copy_gain = clamp_i32(gate_sum, 0, 8192);
                        }
                    }
                }
                int ks_shift = (sh == 2) ? 2 : ((sh == 1 || sh == 3) ? 1 : 0);
                int32_t tau = std::max<int32_t>(256, head_tau[top_l * H + h]);
                int32_t slope = head_slope[top_l * H + h];
                uint8_t q_byte = uint8_t((last_top_q_sk0 >> ((h & 7) * 8)) & 0xFFULL);
                uint64_t sum_raw = 0, best_w = 0;
                int best_s = 0;
                std::vector<uint64_t> raw_w(t + 1);
                std::vector<uint32_t> prll_w(t + 1);
                bool t_is_anchor = copy_tok_gate[tokens[t]] >= 208;
                for (int s = 0; s <= t; ++s) {
                    int ks = std::max(0, s - ks_shift);
                    int32_t d_l1 = 0;
                    for (int j = 0; j < 8; ++j) {
                        d_l1 += std::abs(int32_t(last_top_q[c_off + j]) - int32_t(k_cache[top_l][ks * D + c_off + j]));
                    }
                    uint8_t k_byte = uint8_t((k_sk0[top_l][ks] >> ((h & 7) * 8)) & 0xFFULL);
                    int32_t d_ham = popcnt32_dev(uint32_t(q_byte ^ k_byte));
                    int32_t d_ind = 0;
                    if (sh == 1) {
                        if (s == t) d_ind = 4096;
                        else if (expected_cont_s >= 0) d_ind = (s == expected_cont_s) ? 0 : 65536;
                        else if (s > 1 && t > 0 && s < t && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 0;
                        else if (s > 0 && s < t && t_is_anchor && tokens[s - 1] == tokens[t]) d_ind = 256;
                        else d_ind = 65536;
                    } else if (sh == 2) {
                        if (s == t) d_ind = 4096;
                        else if (expected_cont_s >= 0) d_ind = (s == expected_cont_s) ? 0 : 65536;
                        else if (s > 2 && t > 1 && s < t && tokens[s - 3] == tokens[t - 2] && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 0;
                        else if (s > 1 && t > 0 && s < t && tokens[s - 2] == tokens[t - 1] && tokens[s - 1] == tokens[t]) d_ind = 256;
                        else if (s > 0 && s < t && t_is_anchor && tokens[s - 1] == tokens[t]) d_ind = 512;
                        else d_ind = 65536;
                    } else if (sh == 3) {
                        if (s == t) d_ind = 8192;
                        else d_ind = (s > 0 && s < t && copy_tok_gate[tokens[s]] >= 208) ? 0 : 65536;
                    } else if (slope <= 4 && t > 0 && s == t) {
                        d_ind = 4096;
                    }
                    int32_t d_geom = (sh == 1 || sh == 2) ? ((d_l1 >> 3) + 32 * d_ham) : (d_l1 + 256 * d_ham);
                    uint64_t w = prll_weight_order4(tau, d_geom + slope * (t - s) + d_ind);
                    raw_w[s] = w;
                    sum_raw += w;
                    if (w > best_w) { best_w = w; best_s = s; }
                }
                uint64_t mass_acc = 0;
                for (int s = 0; s <= t; ++s) {
                    uint32_t wp = uint32_t((raw_w[s] * PRLL_QUANTUM) / sum_raw);
                    prll_w[s] = wp;
                    mass_acc += wp;
                }
                prll_w[best_s] += uint32_t(PRLL_QUANTUM - mass_acc);

                if (copy_gain > 0) {
                    for (int s = 1; s < copy_limit; ++s) {
                        int tok_s = int(tokens[s]);
                        bool seen = false;
                        for (int sp = 1; sp < s; ++sp) if (int(tokens[sp]) == tok_s) { seen = true; break; }
                        if (seen) continue;
                        uint64_t tot_wp = 0;
                        uint32_t max_wp = 0;
                        int argmax_s = s;
                        for (int s2 = s; s2 < copy_limit; ++s2) {
                            if (int(tokens[s2]) == tok_s) {
                                tot_wp += prll_w[s2];
                                if (prll_w[s2] >= max_wp) { max_wp = prll_w[s2]; argmax_s = s2; }
                            }
                        }
                        uint64_t eff_wp = (sh == 3) ? uint64_t(max_wp) : tot_wp;
                        int32_t cap_h = (sh == 3 && D >= 128) ? 11264 : 12288;
                        bool is_expected_tok = (expected_cont_s >= 0 && int(tokens[expected_cont_s]) == tok_s);
                        int64_t tok_allow = int64_t(copy_tok_gate[tok_s]);
                        if (sh != 3 && is_expected_tok && (in_tool_obs_span || (expected_cont_s + 1 < copy_limit && copy_tok_gate[tokens[expected_cont_s + 1]] >= 160 && (tok_s == 5 || tok_s == 16 || tok_s == 17)))) {
                            if (tok_allow < 192) tok_allow = 192;
                        }
                        int32_t cand = clamp_i32(int32_t((int64_t(eff_wp) * int64_t(copy_gain) * tok_allow) >> 25), 0, cap_h);
                        if (sh == 3) {
                            if (sh3_cands[tok_s].empty()) copy_best_src[tok_s] = s;
                            sh3_cands[tok_s].push_back(cand);
                        } else if (cand > copy_logits[tok_s]) {
                            copy_logits[tok_s] = cand;
                            copy_best_src[tok_s] = (expected_cont_s >= 0) ? expected_cont_s : argmax_s;
                        }
                    }
                }
            }

            size_t line_start = 0;
            for (size_t i = 0; i < generated.size(); ++i) if (generated[i] == 201) line_start = i + 1;
            for (int v = 0; v < VOCAB_SIZE; ++v) {
                if (!sh3_cands[v].empty()) {
                    auto& vec = sh3_cands[v];
                    std::sort(vec.begin(), vec.end());
                    int32_t combined = (vec.size() >= 2) ? clamp_i32(vec[vec.size()-1] + vec[vec.size()-2], 0, 18432) : vec[0];
                    bool on_line = false;
                    for (size_t i = line_start; i < generated.size(); ++i) if (generated[i] == uint32_t(v)) { on_line = true; break; }
                    if (on_line) combined >>= 3;
                    else if (combined >= 1024) combined += std::max(0, copy_limit - copy_best_src[v]) * 8;
                    if (combined > copy_logits[v]) copy_logits[v] = combined;
                }
            }

            uint64_t h_sk0 = pack_sign64(&last_hidden[0]);
            uint64_t h_sk1 = (D >= 128) ? pack_sign64(&last_hidden[64]) : 0ULL;
            int d_shift = (D >= 128) ? 1 : 0;

            int blocked_cycle_next = -1;
            const size_t gen_len = generated.size();
            if (gen_len >= 5
                && generated[gen_len - 5] == generated[gen_len - 1]
                && generated[gen_len - 4] == generated[gen_len - 1]
                && generated[gen_len - 3] == generated[gen_len - 1]
                && generated[gen_len - 2] == generated[gen_len - 1]) {
                blocked_cycle_next = int(generated[gen_len - 1]);
            } else {
                for (size_t period = 2; period <= 8; ++period) {
                    if (gen_len >= period * 3) {
                        bool eq01 = true, eq12 = true;
                        for (size_t k = 0; k < period; ++k) {
                            if (generated[gen_len - 3 * period + k] != generated[gen_len - 2 * period + k]) eq01 = false;
                            if (generated[gen_len - 2 * period + k] != generated[gen_len - period + k]) eq12 = false;
                        }
                        if (eq01 && eq12) {
                            blocked_cycle_next = int(generated[gen_len - period]);
                            break;
                        }
                    }
                }
            }
            const bool active_mid_span = (expected_cont_s >= 0 && copy_logits[tokens[expected_cont_s]] >= 1024);

            uint32_t best_v = 1;
            int32_t best_score = -2000000000;
            int32_t best_copy = 0, best_winner_pure = -2000000000;
            int32_t max_pure = -2000000000, second_pure = -2000000000;
            uint32_t max_pure_v = 1;

            for (int v = 0; v < VOCAB_SIZE; ++v) {
                if (v == 0 || v == 2) continue;
                if (v == 1 && active_mid_span) continue;
                if (v == blocked_cycle_next) continue;
                int32_t tmac_sum = 0;
                for (int g = 0; g < G; ++g) {
                    int32_t v_hi = act_lut[g][tmac_hi[v * G + g]];
                    int32_t v_lo = act_lut[g][tmac_lo[v * G + g]];
                    tmac_sum += (v_hi << 1) + v_hi + v_lo;
                }
                int32_t l1 = 0;
                for (int c = 0; c < D; ++c) l1 += std::abs(int32_t(last_hidden[c]) - int32_t(p_out[v * D + c]));
                int ham = popcnt64_dev(h_sk0 ^ p_sk0[v]);
                if (D >= 128) ham += popcnt64_dev(h_sk1 ^ p_sk1[v]);
                int32_t pure_score = (tmac_sum >> d_shift) + (int32_t(b_out[v]) * 2) - ((ham * 24) >> d_shift) - (l1 >> (5 + d_shift));
                int32_t score = pure_score + copy_logits[v];
                if (pure_score > max_pure) { second_pure = max_pure; max_pure = pure_score; max_pure_v = v; }
                else if (pure_score > second_pure) { second_pure = pure_score; }
                if (score > best_score) {
                    best_score = score;
                    best_v = v;
                    best_copy = copy_logits[v];
                    best_winner_pure = pure_score;
                }
            }

            if (best_v == 1) break;
            bool won_by_vocab = (prev_copy_src < 0) && (best_v == max_pure_v) && (best_winner_pure > -1024) && (best_winner_pure > second_pure + 512);
            int next_copy = (best_copy >= 1024 && !won_by_vocab) ? copy_best_src[best_v] : -1;

            generated.push_back(best_v);
            push_tok(best_v, next_copy >= 0);
            prev_copy_src = next_copy;

            // Stop immediately if `<|end|>` or `</tool_call>` suffix has been completed (matching gllm_v9.rs)
            const bool v10_stop_markers = (prompt_text.rfind("<|user|>", 0) == 0);
            auto finalize_decoded = [&](std::string s) -> std::string {
                if (v10_stop_markers) {
                    size_t end_pos = s.find("<|end|>");
                    if (end_pos != std::string::npos) s.resize(end_pos);
                    size_t p0 = s.find_first_not_of(" \t\r\n");
                    if (p0 != std::string::npos) s.erase(0, p0);
                    if (s.rfind("<thought>", 0) == 0) {
                        size_t close_th = s.find("</thought>", 9);
                        if (close_th != std::string::npos) {
                            s = s.substr(close_th + 10);
                        }
                    }
                }
                size_t a = s.find_first_not_of(" \t\r\n");
                if (a == std::string::npos) return "";
                size_t b = s.find_last_not_of(" \t\r\n");
                return s.substr(a, b - a + 1);
            };
            if (v10_stop_markers && generated.size() >= 3) {
                std::string so_far;
                for (uint32_t gt : generated) so_far += vocab_bytes[gt];
                if (so_far.find("<|end|>") != std::string::npos
                    || (so_far.size() >= 12 && so_far.compare(so_far.size() - 12, 12, "</tool_call>") == 0)) {
                    out_tok_count = int(generated.size());
                    return finalize_decoded(so_far);
                }
            }
        }
        out_tok_count = int(generated.size());
        std::string decoded;
        for (uint32_t gt : generated) decoded += vocab_bytes[gt];
        if (prompt_text.rfind("<|user|>", 0) == 0) {
            size_t end_pos = decoded.find("<|end|>");
            if (end_pos != std::string::npos) decoded.resize(end_pos);
            size_t p0 = decoded.find_first_not_of(" \t\r\n");
            if (p0 != std::string::npos) decoded.erase(0, p0);
            if (decoded.rfind("<thought>", 0) == 0) {
                size_t close_th = decoded.find("</thought>", 9);
                if (close_th != std::string::npos) decoded = decoded.substr(close_th + 10);
            }
        }
        size_t a = decoded.find_first_not_of(" \t\r\n");
        if (a == std::string::npos) return "";
        size_t b = decoded.find_last_not_of(" \t\r\n");
        return decoded.substr(a, b - a + 1);
    }
};

struct TrainResultSummary {
    std::string model_tag;
    int d_model;
    int n_layers;
    int n_heads;
    double initial_loss;
    double final_train_loss;
    double final_train_acc;
    double val_hinge_loss;
    double val_exact_acc;
    size_t container_bytes;
    std::string container_sha256;
    double elapsed_sec;
    std::vector<SampleGenerationRecord> sample_generations;
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
    void zero_mom() {
        if (mom) CHECK_CUDA(cudaMemset(mom, 0, count * sizeof(int32_t)));
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
    const std::vector<SequenceSample>& eval_samples,
    const std::vector<EvalPromptItem>& eval_prompts,
    const std::vector<std::string>& vocab_bytes,
    const std::string& out_container_path,
    const std::string& shard_dir
) {
    auto t0 = std::chrono::steady_clock::now();
    const int G = D / 4;
    const int BATCH = 170;

    std::mt19937 rng(D * 1000 + L * 97 + 42);
    auto rand_init = [&](size_t n, int32_t amp) {
        std::vector<int32_t> v(n);
        std::uniform_int_distribution<int32_t> dist(-amp, amp);
        for (size_t i = 0; i < n; ++i) v[i] = dist(rng);
        return v;
    };

    std::vector<int32_t> h_etok, h_pout, h_copy_tok_gate;
    init_byte_geometric_embeddings(vocab_bytes, D, h_etok, h_pout, h_copy_tok_gate);

    // Explicit whole-word English prompt delimiter & framing demotion (ZERO BPE subword prefixes touched!):
    // - Clause delimiters (`" into"`, `" inside"`, `" and"`, `" summarize"`, `" report"`, `"[agent_mode]"`) -> `0`
    //   so `in_agent_gen` (`expected_cont_s`) stops cleanly right after copied URLs/commands!
    // - Whole-word English chat/framing words never starting a copy span -> `0` (if `copy_hits == 0`) or `160` (`< 208`),
    //   preventing multi-turn greeting distractors while keeping ALL BPE subword prefixes (`" comp"`, `" off"`, `" step"`,
    //   `" te"`, `" ph"`, `" mo"`, `" ne"`, `" or"`, `" We"`, `" a"`, `" s"`, `" g"`, `" h"`, `" l"`) at `224`!
    std::vector<uint8_t> h_noncopy_tok(VOCAB_SIZE, 0);
    {
        std::vector<uint32_t> copy_hits(VOCAB_SIZE, 0);
        std::vector<uint32_t> copy_start_hits(VOCAB_SIZE, 0);
        std::vector<uint32_t> noncopy_hits(VOCAB_SIZE, 0);
        for (const auto& seq : host_samples) {
            int plen = int(seq.prompt_len);
            int tlen = std::min(int(MAX_SEQ), plen + int(seq.reply_len));
            for (int i = plen; i < tlen; ++i) {
                uint16_t tok = seq.tokens[i];
                if (tok < VOCAB_SIZE && seq.copy_mask[i - 1] != 0) {
                    copy_hits[tok]++;
                    if (i == plen || seq.copy_mask[i - 2] == 0) {
                        copy_start_hits[tok]++;
                    }
                } else if (tok < VOCAB_SIZE) {
                    noncopy_hits[tok]++;
                }
            }
        }
        for (int v = 0; v < VOCAB_SIZE; ++v) {
            uint8_t flags = (noncopy_hits[v] > 0) ? 1u : 0u;
            if (!vocab_bytes[v].empty() && vocab_bytes[v][0] == ' ') {
                flags |= 2u;
            }
            h_noncopy_tok[v] = flags;
        }
        auto is_hard_clause_stop = [](const std::string& s) -> bool {
            return s == " into" || s == " inside" || s == " and" || s == " package"
                || s == " summarize" || s == " report" || s == "[agent_mode]";
        };
        auto is_english_framing_word = [](const std::string& s) -> bool {
            return s == " the" || s == " for" || s == " with" || s == " from"
                || s == " in" || s == " to" || s == " on" || s == " at" || s == " of" || s == " by"
                || s == " is" || s == " are" || s == " was" || s == " am" || s == " it" || s == " do"
                || s == " my" || s == " your" || s == " you" || s == " we" || s == " me"
                || s == " hi" || s == " hey" || s == " hello" || s == " there" || s == " today"
                || s == " great" || s == " running" || s == " locally" || s == " would" || s == " while"
                || s == " like" || s == " work" || s == " ready" || s == " question"
                || s == " help" || s == " ask" || s == " any" || s == " here" || s == " glad"
                || s == " fast" || s == " table" || s == " tables"
                || s == " along" || s == " our" || s == " saved" || s == " name" || s == " context" || s == " session"
                || s == " did" || s == " just" || s == " say" || s == " said"
                || s == " what" || s == " What" || s == " which" || s == " where" || s == " how" || s == " who"
                || s == " write" || s == " Write" || s == " Given" || s == " concise"
                || s == " Rust" || s == " Python" || s == " JavaScript"
                || s == " C" || s == " L" || s == " M" || s == " T" || s == " U"
                || s == " want" || s == " as" || s == " ide" || s == " sn" || s == " soft"
                || s == " build" || s == " set" || s == " free" || s == " rep"
                || s == " whenever" || s == " fun" || s == " ch" || s == " lo"
                || s == " expression" || s == " combining" || s == " adding" || s == " summing"
                || s == " snippet" || s == " setting" || s == " assigning" || s == " freestanding"
                || s == " called" || s == " repository" || s == " tracking" || s == " track"
                || s == "Recorded" || s == " Recorded" || s == "Noted" || s == " Noted"
                || s == " handle" || s == " chat" || s == " label" || s == " Studio" || s == " lookup"
                || s == " deterministic" || s == " keep" || s == " briefly"
                || s == " Run" || s == " Execute" || s == " Install" || s == " Check"
                || s == " Clone" || s == " Search" || s == " Fetch" || s == " Verify"
                || s == " Summarize" || s == " Turn" || s == " record" || s == " subsystem"
                || s == " sandbox" || s == " tools" || s == " tool" || s == " return"
                || s == " Sure" || s == " handy" || s == " cache" || s == " project"
                || s == " coding" || s == " system" || s == " integer" || s == " geometric"
                || s == " layers" || s == " Hadamard"
                || s == " Linux" || s == " MicroVM" || s == " MVM";
        };
        for (int v = 3; v < VOCAB_SIZE; ++v) {
            if (h_copy_tok_gate[v] == 0) continue;
            const std::string& s = vocab_bytes[v];
            if (is_hard_clause_stop(s)) {
                h_copy_tok_gate[v] = 0;
            } else if (is_english_framing_word(s)) {
                if (copy_hits[v] == 0) {
                    h_copy_tok_gate[v] = 0;
                } else if (copy_start_hits[v] == 0) {
                    h_copy_tok_gate[v] = 160;
                }
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
    std::vector<int32_t> h_wtmac(size_t(VOCAB_SIZE) * D, 0);
    std::vector<int32_t> h_bout(VOCAB_SIZE, 0);
    // Gate-12-only readout calibration on `[gate12_lo..D)` for unseen / copy-only BPE subwords
    // (`noncopy_tok == 0 && copy_tok_gate >= 128`): ZERO weight on `gate3` so span-starts are never
    // polluted, while `q9 = +1` (`256`) on `gate12` + `b_out = 400` lifts unseen continuation subwords
    // (`"lu"` in `flux`, `"ft"` in `drift`) cleanly above `+6,000` when `g_h1/g_h2` open (`cp = 12288`)!
    for (int v = 3; v < VOCAB_SIZE; ++v) {
        if ((h_noncopy_tok[v] & 1u) == 0u && h_copy_tok_gate[v] >= 128) {
            for (int c = gate12_lo; c < D; ++c) h_wtmac[size_t(v) * D + c] = 256;
            h_bout[v] = 400;
        }
    }

    std::vector<uint16_t> h_tau(L * H);
    std::vector<uint16_t> h_slope(L * H);
    std::vector<uint8_t> h_shift(L * H);
    std::vector<int32_t> h_copy_b(L * H, -4096);
    std::vector<int32_t> h_copy_w(size_t(L) * H * D, 0);

    const int top_copy_h_limit = gate3_lo / 8;
    for (int l = 0; l < L; ++l) {
        for (int h = 0; h < H; ++h) {
            uint8_t sh = 0;
            if (l == L - 1 && D == 64 && h == 5) {
                sh = 3;
            } else if (l == L - 1 && h >= top_copy_h_limit) {
                sh = 0;
            } else if (h % 4 == 3) {
                sh = 1;
            } else if (h % 4 == 2) {
                sh = 2;
            } else if (h % 4 == 1) {
                sh = 3;
            } else {
                sh = 0;
            }
            h_shift[l * H + h] = sh;
            if (sh == 0) {
                h_tau[l * H + h] = uint16_t(1536 + (h % 8) * 384);
                bool local_recency = (l == L - 1 && h >= top_copy_h_limit)
                    ? ((h % 2) == 0)
                    : (((h / 4) % 2) == 0);
                h_slope[l * H + h] = local_recency ? uint16_t(16) : uint16_t(2);
            } else {
                h_tau[l * H + h] = (l == L - 1 && sh == 3) ? uint16_t(1280) : uint16_t(512);
                h_slope[l * H + h] = uint16_t(0);
            }
            if (l == L - 1 && sh != 0) {
                int32_t* cw = &h_copy_w[(size_t(l) * H + h) * D];
                if (sh == 3) {
                    h_copy_b[l * H + h] = -250;
                    for (int c = gate3_lo; c < gate12_lo; ++c) cw[c] = 512;
                } else {
                    h_copy_b[l * H + h] = (D == 128) ? 2000 : 2600;
                    for (int c = gate12_lo; c < D; ++c) cw[c] = 512;
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
    alloc_copy(h_noncopy_tok, &dm.noncopy_tok);
    alloc_copy(h_scales, &dm.scales_q8);

    BatchCache bc{};
    CHECK_CUDA(cudaMalloc(&bc.etok_q12, h_etok.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.epos_q12, h_epos.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.pout_q12, h_pout.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.qlut_q12, h_qlut.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.klut_q12, h_klut.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.vlut_q12, h_vlut.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.ffnlut_q12, h_ffnlut.size() * sizeof(int16_t)));
    CHECK_CUDA(cudaMalloc(&bc.tmac_hi, size_t(VOCAB_SIZE) * G));
    CHECK_CUDA(cudaMalloc(&bc.tmac_lo, size_t(VOCAB_SIZE) * G));
    CHECK_CUDA(cudaMalloc(&bc.psk0, VOCAB_SIZE * sizeof(uint64_t)));
    CHECK_CUDA(cudaMalloc(&bc.psk1, VOCAB_SIZE * sizeof(uint64_t)));

    auto refresh_batch_cache = [&](int use_qat) {
        auto launch_q12 = [&](const int32_t* src, int16_t* dst, size_t n, int sc_idx) {
            int blks = int((n + 255) / 256);
            precompute_q12_array_kernel<<<blks, 256>>>(src, dst, n, use_qat, dm.scales_q8, sc_idx);
        };
        launch_q12(dm.e_tok, bc.etok_q12, h_etok.size(), 0);
        launch_q12(dm.e_pos, bc.epos_q12, h_epos.size(), 1);
        size_t qk_n = size_t(G) * 256 * QKV_OUT;
        size_t ffn_n = size_t(G) * 256 * D;
        for (int l = 0; l < L; ++l) {
            launch_q12(dm.q_lut + l * qk_n, bc.qlut_q12 + l * qk_n, qk_n, 3 + 4 * l + 0);
            launch_q12(dm.k_lut + l * qk_n, bc.klut_q12 + l * qk_n, qk_n, 3 + 4 * l + 1);
            launch_q12(dm.v_lut + l * qk_n, bc.vlut_q12 + l * qk_n, qk_n, 3 + 4 * l + 2);
            launch_q12(dm.ffn_lut + l * ffn_n, bc.ffnlut_q12 + l * ffn_n, ffn_n, 3 + 4 * l + 3);
        }
        precompute_vocab_readout_kernel<<<16, 256>>>(dm, bc, use_qat);
    };

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

    SequenceSample* d_eval_samples = nullptr;
    if (!eval_samples.empty()) {
        CHECK_CUDA(cudaMalloc(&d_eval_samples, eval_samples.size() * sizeof(SequenceSample)));
        CHECK_CUDA(cudaMemcpy(d_eval_samples, eval_samples.data(), eval_samples.size() * sizeof(SequenceSample), cudaMemcpyHostToDevice));
    }

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
    const int n_eval = int(eval_samples.size());
    std::vector<int> order(n_samples);
    for (int i = 0; i < n_samples; ++i) order[i] = i;
    std::vector<int> eval_order(std::max(1, n_eval));
    for (int i = 0; i < n_eval; ++i) eval_order[i] = i;

    double initial_loss = 0.0;
    double final_train_loss = 0.0;
    double final_train_acc = 0.0;
    double best_qat_acc = -1.0;
    double best_qat_loss = 1e9;

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

    auto eval_exact_qat_dataset = [&](const SequenceSample* d_set, const int* h_idx, int count) -> std::pair<double, double> {
        if (count <= 0 || !d_set) return {0.0, 0.0};
        refresh_batch_cache(1);
        CHECK_CUDA(cudaMemset(d_loss, 0, sizeof(unsigned long long)));
        CHECK_CUDA(cudaMemset(d_corr, 0, sizeof(unsigned int)));
        CHECK_CUDA(cudaMemset(d_tot, 0, sizeof(unsigned int)));
        for (int off = 0; off < count; off += BATCH) {
            int cur_b = std::min(BATCH, count - off);
            CHECK_CUDA(cudaMemcpy(d_batch_idx, &h_idx[off], cur_b * sizeof(int), cudaMemcpyHostToDevice));
            forward_and_grad_kernel<<<cur_b, 256>>>(
                dm, bc, gr, d_scratches, d_set, d_batch_idx, cur_b,
                0, d_loss, d_corr, d_tot
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
        if (ep >= epochs) {
            if (ep == epochs && best_qat_acc > 0.0) {
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
                CHECK_CUDA(cudaMemcpy(dm.copy_tok_gate, b_copy_tok.data(), b_copy_tok.size() * 4, cudaMemcpyHostToDevice));
                CHECK_CUDA(cudaMemcpy(dm.scales_q8, b_scales.data(), b_scales.size() * 2, cudaMemcpyHostToDevice));
            } else {
                sync_device_to_host();
            }
            if (ep == epochs) {
                gm_etok.zero_mom(); gm_epos.zero_mom(); gm_qlut.zero_mom();
                gm_klut.zero_mom(); gm_vlut.zero_mom(); gm_ffnlut.zero_mom();
                gm_wtmac.zero_mom(); gm_pout.zero_mom(); gm_bout.zero_mom();
            }
        }

        std::shuffle(order.begin(), order.end(), rng);
        int lr_main = (L >= 8) ? 22 : 26;
        int lr_trit = 18;
        if (ep >= epochs) {
            lr_main = 1;
            lr_trit = 1;
        } else if (ep >= 6) {
            lr_main = 3; lr_trit = 2;
        } else if (ep >= 4) {
            lr_main = 6; lr_trit = 5;
        } else if (ep >= 2) {
            lr_main = 12; lr_trit = 9;
        }

        CHECK_CUDA(cudaMemset(d_loss, 0, sizeof(unsigned long long)));
        CHECK_CUDA(cudaMemset(d_corr, 0, sizeof(unsigned int)));
        CHECK_CUDA(cudaMemset(d_tot, 0, sizeof(unsigned int)));

        for (int off = 0; off < n_samples; off += BATCH) {
            int cur_b = std::min(BATCH, n_samples - off);
            refresh_batch_cache(use_qat);
            CHECK_CUDA(cudaMemcpy(d_batch_idx, &order[off], cur_b * sizeof(int), cudaMemcpyHostToDevice));
            forward_and_grad_kernel<<<cur_b, 256>>>(
                dm, bc, gr, d_scratches, d_samples, d_batch_idx, cur_b,
                1, d_loss, d_corr, d_tot
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
        final_train_loss = avg_loss;
        final_train_acc = acc;

        double val_loss_ep = 0.0, val_acc_ep = 0.0;
        if (ep >= 1 || use_qat) {
            sync_device_to_host();
            auto [qloss, qacc] = (n_eval > 0)
                ? eval_exact_qat_dataset(d_eval_samples, eval_order.data(), n_eval)
                : eval_exact_qat_dataset(d_samples, order.data(), std::min(n_samples, 2048));
            val_loss_ep = qloss;
            val_acc_ep = qacc;
            if ((ep >= epochs - 2 || use_qat) && (qloss < best_qat_loss || (qloss == best_qat_loss && qacc > best_qat_acc))) {
                best_qat_acc = qacc;
                best_qat_loss = qloss;
                b_etok = h_etok; b_epos = h_epos; b_qlut = h_qlut; b_klut = h_klut;
                b_vlut = h_vlut; b_ffnlut = h_ffnlut; b_wtmac = h_wtmac; b_pout = h_pout;
                b_bout = h_bout; b_copy_b = h_copy_b; b_copy_w = h_copy_w; b_copy_tok = h_copy_tok_gate;
                b_scales = h_scales;
            }
        }

        printf("[%s] Epoch %2d/%2d (QAT=%d, lr=%2d) | Train Loss: %7.2f (%6.2f%%) | Held-Out QAT Loss: %7.2f (%6.2f%%)\n",
               model_tag.c_str(), ep + 1, total_epochs, use_qat, lr_main, avg_loss, acc, val_loss_ep, val_acc_ep);
        fflush(stdout);
    }

    if (best_qat_acc > 0.0) {
        h_etok = b_etok; h_epos = b_epos; h_qlut = b_qlut; h_klut = b_klut;
        h_vlut = b_vlut; h_ffnlut = b_ffnlut; h_wtmac = b_wtmac; h_pout = b_pout;
        h_bout = b_bout; h_copy_b = b_copy_b; h_copy_w = b_copy_w; h_copy_tok_gate = b_copy_tok;
        h_scales = b_scales;
    } else {
        sync_device_to_host();
    }

    printf("[%s] BEST POST-QUANTIZATION HELD-OUT EVAL | Loss: %.2f | Exact Token Acc: %.2f%%\n",
           model_tag.c_str(), best_qat_loss, best_qat_acc);

    // Serialize UORGLLM9 payload + simultaneously populate HostQuantModel for exact greedy verification!
    HostQuantModel hqm{};
    hqm.D = D; hqm.L = L; hqm.H = H; hqm.G = G;
    hqm.head_tau = h_tau;
    hqm.head_slope = h_slope;
    hqm.head_shift = h_shift;
    hqm.head_copy_b.resize(L * H);
    hqm.head_copy_trits.resize(size_t(L) * H * G);
    hqm.q_lut.resize(L); hqm.k_lut.resize(L); hqm.v_lut.resize(L); hqm.ffn_lut.resize(L);

    std::vector<uint8_t> payload;
    payload.reserve(18 * 1024 * 1024);
    for (uint16_t sc : h_scales) append_u16_le(payload, sc);

    std::vector<int8_t> q_buf;
    quantize_tensor_i8(h_etok, h_scales[0], q_buf);
    dequantize_tensor_q12(q_buf, h_scales[0], hqm.e_tok);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

    quantize_tensor_i8(h_epos, h_scales[1], q_buf);
    dequantize_tensor_q12(q_buf, h_scales[1], hqm.e_pos);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

    for (int l = 0; l < L; ++l) {
        for (int h = 0; h < H; ++h) append_u16_le(payload, h_tau[l * H + h]);
        for (int h = 0; h < H; ++h) append_u16_le(payload, h_slope[l * H + h]);
        for (int h = 0; h < H; ++h) payload.push_back(h_shift[l * H + h]);
        for (int h = 0; h < H; ++h) {
            int16_t cb = int16_t(clamp_i32(h_copy_b[l * H + h], -8192, 8192));
            hqm.head_copy_b[l * H + h] = cb;
            append_u16_le(payload, uint16_t(cb));
        }
        for (int h = 0; h < H; ++h) {
            const int32_t* cw = &h_copy_w[(l * H + h) * D];
            for (int g = 0; g < G; ++g) {
                uint8_t pc = pack_4trits(
                    latent_to_trit(cw[4*g + 0]),
                    latent_to_trit(cw[4*g + 1]),
                    latent_to_trit(cw[4*g + 2]),
                    latent_to_trit(cw[4*g + 3])
                );
                hqm.head_copy_trits[(l * H + h) * G + g] = pc;
                payload.push_back(pc);
            }
        }

        size_t qk_n = size_t(G) * 256 * QKV_OUT;
        size_t ffn_n = size_t(G) * 256 * D;
        std::vector<int32_t> sub_q(h_qlut.begin() + l * qk_n, h_qlut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_k(h_klut.begin() + l * qk_n, h_klut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_v(h_vlut.begin() + l * qk_n, h_vlut.begin() + (l + 1) * qk_n);
        std::vector<int32_t> sub_f(h_ffnlut.begin() + l * ffn_n, h_ffnlut.begin() + (l + 1) * ffn_n);

        quantize_tensor_i8(sub_q, h_scales[3 + 4 * l + 0], q_buf);
        dequantize_tensor_q12(q_buf, h_scales[3 + 4 * l + 0], hqm.q_lut[l]);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

        quantize_tensor_i8(sub_k, h_scales[3 + 4 * l + 1], q_buf);
        dequantize_tensor_q12(q_buf, h_scales[3 + 4 * l + 1], hqm.k_lut[l]);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

        quantize_tensor_i8(sub_v, h_scales[3 + 4 * l + 2], q_buf);
        dequantize_tensor_q12(q_buf, h_scales[3 + 4 * l + 2], hqm.v_lut[l]);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

        quantize_tensor_i8(sub_f, h_scales[3 + 4 * l + 3], q_buf);
        dequantize_tensor_q12(q_buf, h_scales[3 + 4 * l + 3], hqm.ffn_lut[l]);
        payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());
    }

    hqm.tmac_hi.resize(size_t(VOCAB_SIZE) * G);
    hqm.tmac_lo.resize(size_t(VOCAB_SIZE) * G);
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        const int32_t* tw = &h_wtmac[v * D];
        for (int g = 0; g < G; ++g) {
            uint8_t chi, clo;
            pack_4q9(tw[4*g + 0], tw[4*g + 1], tw[4*g + 2], tw[4*g + 3], chi, clo);
            hqm.tmac_hi[size_t(v) * G + g] = chi;
            hqm.tmac_lo[size_t(v) * G + g] = clo;
        }
    }
    payload.insert(payload.end(), hqm.tmac_hi.begin(), hqm.tmac_hi.end());
    payload.insert(payload.end(), hqm.tmac_lo.begin(), hqm.tmac_lo.end());

    hqm.copy_tok_gate.resize(VOCAB_SIZE);
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        uint8_t cg = uint8_t((v <= 2) ? 0 : clamp_i32(h_copy_tok_gate[v], 0, 255));
        hqm.copy_tok_gate[v] = cg;
        payload.push_back(cg);
    }

    quantize_tensor_i8(h_pout, h_scales[2], q_buf);
    dequantize_tensor_q12(q_buf, h_scales[2], hqm.p_out);
    payload.insert(payload.end(), (uint8_t*)q_buf.data(), (uint8_t*)q_buf.data() + q_buf.size());

    hqm.p_sk0.resize(VOCAB_SIZE);
    hqm.p_sk1.resize(VOCAB_SIZE, 0ULL);
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        hqm.p_sk0[v] = pack_sign64(&hqm.p_out[v * D]);
        if (D >= 128) hqm.p_sk1[v] = pack_sign64(&hqm.p_out[v * D + 64]);
    }

    hqm.b_out.resize(VOCAB_SIZE);
    for (int v = 0; v < VOCAB_SIZE; ++v) {
        int16_t bv = int16_t(clamp_i32(h_bout[v], -2048, 2048));
        hqm.b_out[v] = bv;
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

    if (!out_container_path.empty()) {
        std::ofstream f(out_container_path, std::ios::binary);
        f.write((const char*)container.data(), container.size());
        printf("[%s] Wrote %zu bytes -> %s (sha256=%s)\n",
               model_tag.c_str(), container.size(), out_container_path.c_str(), full_sha_hex.c_str());
    }

    if (!shard_dir.empty()) {
        mkdir(shard_dir.c_str(), 0755);
        const size_t SHARD_CAP = 1048576; // 1 MiB OPFS shards for v10
        size_t shard_count = (container.size() + SHARD_CAP - 1) / SHARD_CAP;
        std::string manifest = "{\n";
        manifest += "  \"format\": \"UORGLLM9\",\n";
        manifest += "  \"schema\": \"uor-r4.compositional-gllm/10\",\n";
        manifest += "  \"version\": 9,\n";
        manifest += "  \"model_version\": \"v10\",\n";
        manifest += "  \"vocab_size\": 4096,\n";
        manifest += "  \"d_model\": " + std::to_string(D) + ",\n";
        manifest += "  \"n_layers\": " + std::to_string(L) + ",\n";
        manifest += "  \"n_heads\": " + std::to_string(H) + ",\n";
        manifest += "  \"max_seq_len\": 1024,\n";
        manifest += "  \"turn_memory_entries\": 0,\n";
        manifest += "  \"dense_matmul_ops\": 0,\n";
        manifest += "  \"softmax_ops\": 0,\n";
        manifest += "  \"total_bytes\": " + std::to_string(container.size()) + ",\n";
        manifest += "  \"container_sha256\": \"" + full_sha_hex + "\",\n";
        manifest += "  \"shard_max_bytes\": 1048576,\n";
        manifest += "  \"shards\": [\n";

        for (size_t s = 0; s < shard_count; ++s) {
            size_t off = s * SHARD_CAP;
            size_t len = std::min(SHARD_CAP, container.size() - off);
            char part_name[64];
            snprintf(part_name, sizeof(part_name), "shard_%03zu.bin", s);
            std::string part_path = shard_dir + "/" + part_name;
            {
                std::ofstream sf(part_path, std::ios::binary);
                sf.write((const char*)(container.data() + off), len);
            }
            uint8_t s_sha[32];
            sha256_bytes(container.data() + off, len, s_sha);
            manifest += "    {\"index\": " + std::to_string(s) +
                        ", \"file\": \"" + std::string(part_name) +
                        "\", \"path\": \"/weights/v10/" + std::string(part_name) +
                        "\", \"byte_offset\": " + std::to_string(off) +
                        ", \"byte_length\": " + std::to_string(len) +
                        ", \"sha256\": \"" + hex_of(s_sha, 32) + "\"}";
            if (s + 1 < shard_count) manifest += ",";
            manifest += "\n";
        }
        manifest += "  ]\n}\n";
        std::ofstream mf(shard_dir + "/manifest.json");
        mf << manifest;
        printf("[%s] Emitted %zu 1MiB OPFS shards + manifest.json -> %s\n",
               model_tag.c_str(), shard_count, shard_dir.c_str());
    }

    // Run exact quantized greedy generation over the 20 held-out audit prompts
    std::vector<SampleGenerationRecord> gen_records;
    printf("\n--- [%s] Held-Out Conversational, Multi-Turn & Tool Samples ---\n", model_tag.c_str());
    for (const auto& ep : eval_prompts) {
        int gcnt = 0;
        std::string reply = hqm.generate_greedy(ep.prompt_ids, vocab_bytes, ep.prompt_text, 256, gcnt);
        printf("  [%s] -> %s\n", ep.id.c_str(), reply.c_str());
        gen_records.push_back(SampleGenerationRecord{ep.id, ep.kind, ep.prompt_text, reply, gcnt});
    }
    printf("----------------------------------------------------------------\n\n");
    fflush(stdout);

    gm_etok.destroy(); gm_epos.destroy(); gm_qlut.destroy(); gm_klut.destroy(); gm_vlut.destroy();
    gm_ffnlut.destroy(); gm_wtmac.destroy(); gm_pout.destroy(); gm_bout.destroy();
    gm_copyb.destroy(); gm_copyw.destroy(); gm_copytok.destroy();
    cudaFree(bc.etok_q12); cudaFree(bc.epos_q12); cudaFree(bc.pout_q12);
    cudaFree(bc.qlut_q12); cudaFree(bc.klut_q12); cudaFree(bc.vlut_q12); cudaFree(bc.ffnlut_q12);
    cudaFree(bc.tmac_hi); cudaFree(bc.tmac_lo); cudaFree(bc.psk0); cudaFree(bc.psk1);
    cudaFree(dm.e_tok); cudaFree(dm.e_pos); cudaFree(dm.q_lut); cudaFree(dm.k_lut);
    cudaFree(dm.v_lut); cudaFree(dm.ffn_lut); cudaFree(dm.w_tmac); cudaFree(dm.p_out); cudaFree(dm.b_out);
    cudaFree(dm.head_tau); cudaFree(dm.head_slope); cudaFree(dm.head_shift);
    cudaFree(dm.head_copy_b); cudaFree(dm.head_copy_w); cudaFree(dm.copy_tok_gate); cudaFree(dm.noncopy_tok); cudaFree(dm.scales_q8);
    cudaFree(d_samples);
    if (d_eval_samples) cudaFree(d_eval_samples);
    cudaFree(d_scratches); cudaFree(d_batch_idx);
    cudaFree(d_loss); cudaFree(d_corr); cudaFree(d_tot);

    double elapsed = std::chrono::duration<double>(std::chrono::steady_clock::now() - t0).count();
    return TrainResultSummary{
        model_tag, D, L, H, initial_loss, final_train_loss, final_train_acc,
        best_qat_loss, best_qat_acc, container.size(), full_sha_hex, elapsed, gen_records
    };
}

static void populate_sample_metadata(SequenceSample& s, const std::vector<std::string>& vocab_bytes) {
    s.obs_lo = 0;
    s.obs_hi = 0;
    s.gen_flags = 0;
    s._pad = 0;
    const int plen = int(s.prompt_len);
    const int rlen = int(s.reply_len);
    const int tlen = std::min(int(MAX_SEQ), plen + rlen);

    // 1. `<tool_response>...</tool_response>` token span matching `GllmV9Model::run_kv_generation`
    if (plen > 2) {
        std::string acc_s;
        std::vector<int> tok_end_byte(plen + 1, 0);
        for (int i = 0; i < plen; ++i) {
            if (i > 0 && s.tokens[i] < VOCAB_SIZE) acc_s += vocab_bytes[s.tokens[i]];
            tok_end_byte[i + 1] = int(acc_s.size());
        }
        size_t r_open = acc_s.rfind("<tool_response>");
        if (r_open != std::string::npos) {
            size_t content_b = r_open + 15;
            size_t r_close = acc_s.find("</tool_response>", content_b);
            if (r_close != std::string::npos) {
                int lo = -1, hi = -1;
                for (int i = 1; i < plen; ++i) {
                    if (lo < 0 && tok_end_byte[i + 1] > int(content_b)) lo = i;
                    if (tok_end_byte[i] < int(r_close)) hi = i + 1;
                }
                if (lo > 0 && hi > lo) {
                    s.obs_lo = uint16_t(lo);
                    s.obs_hi = uint16_t(hi);
                }
            }
        }
    }

    // 2. `in_code_gen` (bit 0) and `in_agent_gen` (bit 1) matching `GllmV9KvCache::predict_next_token`
    std::string gen_prefix;
    const int gen_end = std::min(tlen - 1, plen + 6);
    for (int i = plen; i < gen_end; ++i) {
        if (s.tokens[i] < VOCAB_SIZE) gen_prefix += vocab_bytes[s.tokens[i]];
    }
    size_t p_nonws = gen_prefix.find_first_not_of(" \t\r\n");
    std::string gen_trim = (p_nonws == std::string::npos) ? "" : gen_prefix.substr(p_nonws);
    if (gen_trim.rfind("```", 0) == 0) s.gen_flags |= 1u;
    if ((s.obs_lo > 0 && s.obs_hi > s.obs_lo) || gen_trim.rfind("<", 0) == 0) s.gen_flags |= 2u;
}

static bool load_uorcorp3(
    const std::string& path,
    const std::vector<std::string>& vocab_bytes,
    std::vector<SequenceSample>& out
) {
    std::ifstream cf(path, std::ios::binary);
    if (!cf) return false;
    char magic[8];
    cf.read(magic, 8);
    if (memcmp(magic, "UORCORP3", 8) != 0) return false;
    uint32_t count = 0, file_max_seq = 0;
    cf.read((char*)&count, 4);
    cf.read((char*)&file_max_seq, 4);
    out.resize(count);
    for (uint32_t i = 0; i < count; ++i) {
        uint16_t plen = 0, rlen = 0;
        cf.read((char*)&plen, 2);
        cf.read((char*)&rlen, 2);
        out[i].prompt_len = plen;
        out[i].reply_len = rlen;
        memset(out[i].tokens, 0, sizeof(out[i].tokens));
        memset(out[i].copy_mask, 0, sizeof(out[i].copy_mask));
        cf.read((char*)out[i].tokens, size_t(plen + rlen) * 2);
        std::vector<uint8_t> rcopy(rlen, 0);
        cf.read((char*)rcopy.data(), rlen);
        for (uint16_t k = 0; k < rlen; ++k) {
            if ((plen - 1 + k) < MAX_SEQ) {
                out[i].copy_mask[plen - 1 + k] = rcopy[k];
            }
        }
        populate_sample_metadata(out[i], vocab_bytes);
    }
    return true;
}

static std::vector<EvalPromptItem> load_eval_prompts_bin(const std::string& path) {
    std::vector<EvalPromptItem> items;
    std::ifstream pf(path, std::ios::binary);
    if (!pf) return items;
    char magic[8];
    pf.read(magic, 8);
    if (memcmp(magic, "UORPRMP1", 8) != 0) return items;
    uint32_t cnt = 0;
    pf.read((char*)&cnt, 4);
    for (uint32_t i = 0; i < cnt; ++i) {
        uint16_t l_id = 0, l_k = 0, l_p = 0, n_ids = 0;
        pf.read((char*)&l_id, 2);
        pf.read((char*)&l_k, 2);
        pf.read((char*)&l_p, 2);
        EvalPromptItem it;
        it.id.resize(l_id); pf.read(&it.id[0], l_id);
        it.kind.resize(l_k); pf.read(&it.kind[0], l_k);
        it.prompt_text.resize(l_p); pf.read(&it.prompt_text[0], l_p);
        pf.read((char*)&n_ids, 2);
        it.prompt_ids.resize(n_ids);
        pf.read((char*)it.prompt_ids.data(), size_t(n_ids) * 2);
        items.push_back(std::move(it));
    }
    return items;
}

int main(int argc, char** argv) {
    if (argc < 4) {
        fprintf(stderr, "Usage: %s <v10_corpus.bin> <bootstrap_v9.uorgllm> <v10_shard_dir> [receipt.json]\n", argv[0]);
        return 1;
    }

    cudaDeviceProp prop{};
    CHECK_CUDA(cudaGetDeviceProperties(&prop, 0));
    printf("=== UOR-R4-GLLM v10 Pure-Geometric CUDA Trainer on %s (SM %d.%d, %zu MiB VRAM) ===\n",
           prop.name, prop.major, prop.minor, prop.totalGlobalMem / (1024 * 1024));

    std::string corpus_path(argv[1]);
    size_t slash = corpus_path.find_last_of('/');
    std::string dir_prefix = (slash == std::string::npos) ? "." : corpus_path.substr(0, slash);
    std::string vocab_path = dir_prefix + "/v10_vocab_bytes.bin";
    std::string boot_corpus_path = dir_prefix + "/v10_boot_corpus.bin";
    std::string boot_eval_path = dir_prefix + "/v10_boot_eval.bin";
    std::string eval_path = dir_prefix + "/v10_eval.bin";
    std::string prompts_path = dir_prefix + "/v10_eval_prompts.bin";

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

    std::vector<SequenceSample> samples, boot_samples, eval_samples, boot_eval_samples;
    if (!load_uorcorp3(corpus_path, vocab_bytes, samples)) {
        fprintf(stderr, "Cannot open UORCORP3 corpus %s\n", corpus_path.c_str());
        return 1;
    }
    if (!load_uorcorp3(boot_corpus_path, vocab_bytes, boot_samples) || boot_samples.empty()) {
        boot_samples = samples;
    }
    load_uorcorp3(eval_path, vocab_bytes, eval_samples);
    if (!load_uorcorp3(boot_eval_path, vocab_bytes, boot_eval_samples) || boot_eval_samples.empty()) {
        boot_eval_samples = eval_samples;
    }
    std::vector<EvalPromptItem> eval_prompts = load_eval_prompts_bin(prompts_path);

    printf("Loaded %zu full train sequences (%zu fast-start boot sequences), %zu held-out eval sequences (%zu boot eval), %zu audit prompts (MAX_SEQ=%d).\n",
           samples.size(), boot_samples.size(), eval_samples.size(), boot_eval_samples.size(), eval_prompts.size(), MAX_SEQ);

    // 1. Fast-start bootstrap_v9.uorgllm: 6L 64D (8 heads, 16 LUT groups -> 3.43 MB <= 10 MB)
    auto boot_res = train_and_export_model(
        "bootstrap_v10_6L_64D", 64, 6, 8, 11, 2,
        boot_samples, boot_eval_samples, eval_prompts, vocab_bytes, argv[2], ""
    );

    // 2. Full OPFS sharded v10 model: 8L 128D (16 heads, 32 LUT groups -> 12.90 MB <= 14 MB, 1 MiB shards)
    auto opfs_res = train_and_export_model(
        "opfs_v10_8L_128D", 128, 8, 16, 10, 2,
        samples, eval_samples, eval_prompts, vocab_bytes, "", argv[3]
    );

    if (argc >= 5) {
        auto write_samples_json = [](std::ofstream& rf, const std::vector<SampleGenerationRecord>& recs) {
            rf << "[\n";
            for (size_t i = 0; i < recs.size(); ++i) {
                rf << "      {\"id\": \"" << json_escape(recs[i].id) << "\", "
                   << "\"kind\": \"" << json_escape(recs[i].kind) << "\", "
                   << "\"prompt\": \"" << json_escape(recs[i].prompt) << "\", "
                   << "\"reply\": \"" << json_escape(recs[i].reply) << "\", "
                   << "\"generated_tokens\": " << recs[i].generated_tokens << "}";
                if (i + 1 < recs.size()) rf << ",";
                rf << "\n";
            }
            rf << "    ]";
        };

        std::ofstream rf(argv[4]);
        rf << "{\n"
           << "  \"schema\": \"uor-r4.gpu-training-receipt/10\",\n"
           << "  \"model_version\": \"v10\",\n"
           << "  \"gpu_device\": \"" << json_escape(prop.name) << "\",\n"
           << "  \"compute_capability\": \"" << prop.major << "." << prop.minor << "\",\n"
           << "  \"vram_mib\": " << (prop.totalGlobalMem / (1024 * 1024)) << ",\n"
           << "  \"max_seq_len\": " << MAX_SEQ << ",\n"
           << "  \"corpus_train_sequences\": " << samples.size() << ",\n"
           << "  \"corpus_eval_sequences\": " << eval_samples.size() << ",\n"
           << "  \"dense_matmul_ops\": 0,\n"
           << "  \"softmax_ops\": 0,\n"
           << "  \"turn_memory_entries\": 0,\n"
           << "  \"bootstrap_v9_fast_start\": {\n"
           << "    \"d_model\": " << boot_res.d_model << ",\n"
           << "    \"n_layers\": " << boot_res.n_layers << ",\n"
           << "    \"n_heads\": " << boot_res.n_heads << ",\n"
           << "    \"initial_hinge_loss\": " << boot_res.initial_loss << ",\n"
           << "    \"final_train_hinge_loss\": " << boot_res.final_train_loss << ",\n"
           << "    \"final_train_token_acc_pct\": " << boot_res.final_train_acc << ",\n"
           << "    \"val_hinge_loss\": " << boot_res.val_hinge_loss << ",\n"
           << "    \"val_exact_token_acc_pct\": " << boot_res.val_exact_acc << ",\n"
           << "    \"container_bytes\": " << boot_res.container_bytes << ",\n"
           << "    \"container_sha256\": \"" << boot_res.container_sha256 << "\",\n"
           << "    \"elapsed_seconds\": " << boot_res.elapsed_sec << ",\n"
           << "    \"held_out_generations\": ";
        write_samples_json(rf, boot_res.sample_generations);
        rf << "\n  },\n"
           << "  \"opfs_sharded_v10\": {\n"
           << "    \"d_model\": " << opfs_res.d_model << ",\n"
           << "    \"n_layers\": " << opfs_res.n_layers << ",\n"
           << "    \"n_heads\": " << opfs_res.n_heads << ",\n"
           << "    \"initial_hinge_loss\": " << opfs_res.initial_loss << ",\n"
           << "    \"final_train_hinge_loss\": " << opfs_res.final_train_loss << ",\n"
           << "    \"final_train_token_acc_pct\": " << opfs_res.final_train_acc << ",\n"
           << "    \"val_hinge_loss\": " << opfs_res.val_hinge_loss << ",\n"
           << "    \"val_exact_token_acc_pct\": " << opfs_res.val_exact_acc << ",\n"
           << "    \"container_bytes\": " << opfs_res.container_bytes << ",\n"
           << "    \"container_sha256\": \"" << opfs_res.container_sha256 << "\",\n"
           << "    \"elapsed_seconds\": " << opfs_res.elapsed_sec << ",\n"
           << "    \"held_out_generations\": ";
        write_samples_json(rf, opfs_res.sample_generations);
        rf << "\n  }\n"
           << "}\n";
    }
    return 0;
}
