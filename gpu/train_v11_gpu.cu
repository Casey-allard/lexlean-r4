// UOR-R4-GLLM v11 Pure-Integer High-Throughput CUDA Trainer (`UORGLLM9` version = 11)
// Compiles with: /usr/local/cuda/bin/nvcc -O3 -arch=sm_89 gpu/train_v11_gpu.cu -o train_v11_gpu
// Zero Python; exact integer Q12 CORDIC RoPE, FWHT-64, 16-ch Q12 bilinear + sign-sketch PRLL,
// discrete Q12 T-MAC Q/K/V/FFN LUTs (`quant_bits=4` nibble FFN or `quant_bits=8`),
// transposed coalesced 81-state packed-ternary T-MAC readout + zero-atomic vocabulary gradient kernel.

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <vector>
#include <string>
#include <algorithm>
#include <chrono>
#include <numeric>

static constexpr int MAX_SEQ = 1024;
static constexpr int VOCAB = 4096;
static constexpr int QKV_OUT = 16;
static constexpr int MAX_POS_EMBED = 256;
static constexpr int D_MAX = 256;
static constexpr int H_MAX = 32;
static constexpr int G_MAX = 64;
static constexpr uint64_t PRLL_QUANTUM = 1000000ULL;

#define CHECK_CUDA(expr) do { \
    cudaError_t err__ = (expr); \
    if (err__ != cudaSuccess) { \
        fprintf(stderr, "CUDA error %s:%d: %s\n", __FILE__, __LINE__, cudaGetErrorString(err__)); \
        exit(2); \
    } \
} while(0)

static inline uint32_t rotr32(uint32_t x, uint32_t n) { return (x >> n) | (x << (32 - n)); }

static void sha256_compute(const uint8_t* data, size_t len, uint8_t out[32]) {
    static const uint32_t K[64] = {
        0x428a2f98u,0x71374491u,0xb5c0fbcfu,0xe9b5dba5u,0x3956c25bu,0x59f111f1u,0x923f82a4u,0xab1c5ed5u,
        0xd807aa98u,0x12835b01u,0x243185beu,0x550c7dc3u,0x72be5d74u,0x80deb1feu,0x9bdc06a7u,0xc19bf174u,
        0xe49b69c1u,0xefbe4786u,0x0fc19dc6u,0x240ca1ccu,0x2de92c6fu,0x4a7484aau,0x5cb0a9dcu,0x76f988dau,
        0x983e5152u,0xa831c66du,0xb00327c8u,0xbf597fc7u,0xc6e00bf3u,0xd5a79147u,0x06ca6351u,0x14292967u,
        0x27b70a85u,0x2e1b2138u,0x4d2c6dfcu,0x53380d13u,0x650a7354u,0x766a0abbu,0x81c2c92eu,0x92722c85u,
        0xa2bfe8a1u,0xa81a664bu,0xc24b8b70u,0xc76c51a3u,0xd192e819u,0xd6990624u,0xf40e3585u,0x106aa070u,
        0x19a4c116u,0x1e376c08u,0x2748774cu,0x34b0bcb5u,0x391c0cb3u,0x4ed8aa4au,0x5b9cca4fu,0x682e6ff3u,
        0x748f82eeu,0x78a5636fu,0x84c87814u,0x8cc70208u,0x90befffau,0xa4506cebu,0xbef9a3f7u,0xc67178f2u
    };
    uint32_t H[8] = {
        0x6a09e667u,0xbb67ae85u,0x3c6ef372u,0xa54ff53au,0x510e527fu,0x9b05688cu,0x1f83d9abu,0x5be0cd19u
    };
    uint64_t bitlen = uint64_t(len) * 8ULL;
    size_t rem = (len + 9) % 64;
    size_t pad_len = len + 9 + (rem == 0 ? 0 : (64 - rem));
    std::vector<uint8_t> buf(pad_len, 0);
    if (len > 0) memcpy(buf.data(), data, len);
    buf[len] = 0x80;
    for (int i = 0; i < 8; ++i) buf[pad_len - 8 + i] = uint8_t((bitlen >> (56 - 8 * i)) & 0xFF);
    for (size_t off = 0; off < pad_len; off += 64) {
        uint32_t w[64];
        for (int i = 0; i < 16; ++i) {
            w[i] = (uint32_t(buf[off + i*4]) << 24) | (uint32_t(buf[off + i*4 + 1]) << 16)
                 | (uint32_t(buf[off + i*4 + 2]) << 8) | uint32_t(buf[off + i*4 + 3]);
        }
        for (int i = 16; i < 64; ++i) {
            uint32_t s0 = rotr32(w[i - 15], 7) ^ rotr32(w[i - 15], 18) ^ (w[i - 15] >> 3);
            uint32_t s1 = rotr32(w[i - 2], 17) ^ rotr32(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        uint32_t a=H[0],b=H[1],c=H[2],d=H[3],e=H[4],f=H[5],g=H[6],h=H[7];
        for (int i = 0; i < 64; ++i) {
            uint32_t S1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
            uint32_t ch = (e & f) ^ ((~e) & g);
            uint32_t t1 = h + S1 + ch + K[i] + w[i];
            uint32_t S0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
            uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            uint32_t t2 = S0 + maj;
            h=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
        }
        H[0]+=a; H[1]+=b; H[2]+=c; H[3]+=d; H[4]+=e; H[5]+=f; H[6]+=g; H[7]+=h;
    }
    for (int i = 0; i < 8; ++i) {
        out[i*4+0] = uint8_t(H[i] >> 24); out[i*4+1] = uint8_t(H[i] >> 16);
        out[i*4+2] = uint8_t(H[i] >> 8);  out[i*4+3] = uint8_t(H[i]);
    }
}

static std::string bytes_to_hex(const uint8_t* b, size_t n) {
    static const char HEX[] = "0123456789abcdef";
    std::string out(n * 2, '0');
    for (size_t i = 0; i < n; ++i) { out[2*i] = HEX[b[i] >> 4]; out[2*i+1] = HEX[b[i] & 15]; }
    return out;
}

static std::string json_esc(const std::string& s) {
    std::string out = "\"";
    for (unsigned char c : s) {
        if (c == '\\') out += "\\\\";
        else if (c == '"') out += "\\\"";
        else if (c == '\n') out += "\\n";
        else if (c == '\r') out += "\\r";
        else if (c == '\t') out += "\\t";
        else if (c < 0x20) { char buf[8]; snprintf(buf, sizeof(buf), "\\u%04x", c); out += buf; }
        else out.push_back(char(c));
    }
    out += "\"";
    return out;
}

// ============================================================================
// Pure Integer Q12 CORDIC RoPE (Bit-exact to crates/wasm/src/gllm_v9.rs)
// ============================================================================
static const uint32_t CORDIC_ATAN_BAM[28] = {
    536870912u, 316933406u, 167458907u, 85004756u, 42667331u, 21354465u, 10679838u,
    5340245u, 2670163u, 1335087u, 667544u, 333772u, 166886u, 83443u, 41722u, 20861u,
    10430u, 5215u, 2608u, 1304u, 652u, 326u, 163u, 81u, 41u, 20u, 10u, 5u
};
static const uint32_t ROPE_OMEGA_BAM[8] = {
    20506958u, 6835653u, 2050696u, 683565u, 205070u, 68357u, 20507u, 6836u
};

static int16_t h_rope_cos[MAX_SEQ][8];
static int16_t h_rope_sin[MAX_SEQ][8];
__constant__ int16_t d_rope_cos[MAX_SEQ][8];
__constant__ int16_t d_rope_sin[MAX_SEQ][8];

static void cordic_cos_sin_q12(uint32_t bam, int16_t& out_c, int16_t& out_s) {
    uint32_t quad = bam >> 30;
    uint32_t rem = bam & 0x3FFFFFFFu;
    if (rem == 0) {
        if (quad == 0) { out_c = 4096; out_s = 0; }
        else if (quad == 1) { out_c = 0; out_s = 4096; }
        else if (quad == 2) { out_c = -4096; out_s = 0; }
        else { out_c = 0; out_s = -4096; }
        return;
    }
    int64_t x = 652032874LL, y = 0, z = int64_t(rem);
    for (int i = 0; i < 28; ++i) {
        int64_t dx = x >> i, dy = y >> i, dz = int64_t(CORDIC_ATAN_BAM[i]);
        if (z >= 0) { x -= dy; y += dx; z -= dz; }
        else        { x += dy; y -= dx; z += dz; }
    }
    int64_t rx = (x + (x >= 0 ? (1LL << 17) : -(1LL << 17))) >> 18;
    int64_t ry = (y + (y >= 0 ? (1LL << 17) : -(1LL << 17))) >> 18;
    int16_t c0 = int16_t(rx > 4096 ? 4096 : (rx < -4096 ? -4096 : rx));
    int16_t s0 = int16_t(ry > 4096 ? 4096 : (ry < -4096 ? -4096 : ry));
    if (quad == 0) { out_c = c0; out_s = s0; }
    else if (quad == 1) { out_c = -s0; out_s = c0; }
    else if (quad == 2) { out_c = -c0; out_s = -s0; }
    else { out_c = s0; out_s = -c0; }
}

static void init_rope_tables() {
    for (int pos = 0; pos < MAX_SEQ; ++pos) {
        for (int p = 0; p < 8; ++p) {
            uint32_t bam = uint32_t(pos) * ROPE_OMEGA_BAM[p];
            cordic_cos_sin_q12(bam, h_rope_cos[pos][p], h_rope_sin[pos][p]);
        }
    }
    CHECK_CUDA(cudaMemcpyToSymbol(d_rope_cos, h_rope_cos, sizeof(h_rope_cos)));
    CHECK_CUDA(cudaMemcpyToSymbol(d_rope_sin, h_rope_sin, sizeof(h_rope_sin)));
}

// ============================================================================
// Integer Primitives (Device + Host parity)
// ============================================================================
__host__ __device__ static inline int16_t clamp_q12(int32_t v) {
    return int16_t(v < -8192 ? -8192 : (v > 8192 ? 8192 : v));
}

__host__ __device__ static inline int16_t dequant_i8_q12(int8_t q, uint16_t scale_q8) {
    int32_t sc = int32_t(scale_q8 < 16 ? 16 : scale_q8);
    return clamp_q12((int32_t(q) * sc) >> 4);
}

__host__ __device__ static inline int16_t dequant_i4_q12(int8_t q4, uint16_t scale_q8) {
    int32_t sc = int32_t(scale_q8 < 16 ? 16 : scale_q8);
    return clamp_q12((int32_t(q4) * sc) >> 1);
}

__host__ __device__ static inline uint8_t q2_level(int16_t v) {
    int32_t x = int32_t(v);
    if (x < -1024) return 0;
    if (x < 0) return 1;
    if (x < 1024) return 2;
    return 3;
}

__host__ __device__ static inline uint8_t code_4ch(const int16_t* u4) {
    return uint8_t(q2_level(u4[0]) | (q2_level(u4[1]) << 2) | (q2_level(u4[2]) << 4) | (q2_level(u4[3]) << 6));
}

__host__ __device__ static inline int16_t icosian_hinge(int32_t v) {
    int32_t y = (v < -2048) ? ((v >> 2) - 1536) : v;
    return clamp_q12(y);
}

__host__ __device__ static inline uint32_t prll_weight_order4_v10(int32_t tau, int32_t d_tot) {
    uint64_t t = uint64_t(tau < 256 ? 256 : tau);
    uint64_t d = uint64_t(d_tot < 0 ? 0 : d_tot);
    uint64_t num1 = t << 8;
    uint64_t den1 = t + d;
    uint64_t r1 = num1 / (den1 < 1 ? 1 : den1);
    uint64_t r2 = r1 * r1;
    uint64_t r4 = (r2 >> 4) * (r2 >> 4);
    return r4 == 0 ? 1u : uint32_t(r4);
}

__host__ __device__ static inline uint64_t pack_sign64(const int16_t* v64) {
    uint64_t bits = 0ULL;
    for (int i = 0; i < 64; ++i) {
        if (v64[i] >= 0) bits |= (1ULL << i);
    }
    return bits;
}

__host__ __device__ static inline void build_tmac_act_lut81(const int16_t* x4, int32_t* out81) {
    int32_t a0[3] = { -int32_t(x4[0]), 0, int32_t(x4[0]) };
    int32_t a1[3] = { -int32_t(x4[1]), 0, int32_t(x4[1]) };
    int32_t a2[3] = { -int32_t(x4[2]), 0, int32_t(x4[2]) };
    int32_t a3[3] = { -int32_t(x4[3]), 0, int32_t(x4[3]) };
    int idx = 0;
    for (int i3 = 0; i3 < 3; ++i3) {
        for (int i2 = 0; i2 < 3; ++i2) {
            int32_t s23 = a3[i3] + a2[i2];
            for (int i1 = 0; i1 < 3; ++i1) {
                int32_t s123 = s23 + a1[i1];
                out81[idx + 0] = (s123 + a0[0]) >> 5;
                out81[idx + 1] = (s123 + a0[1]) >> 5;
                out81[idx + 2] = (s123 + a0[2]) >> 5;
                idx += 3;
            }
        }
    }
}

static void fwht64_host(const int16_t* in64, int16_t* out64) {
    int32_t u[64];
    for (int i = 0; i < 64; ++i) u[i] = int32_t(in64[i]);
    for (int len = 1; len < 64; len <<= 1) {
        for (int i = 0; i < 64; i += len * 2) {
            for (int j = 0; j < len; ++j) {
                int32_t a = u[i + j], b = u[i + j + len];
                u[i + j] = a + b;
                u[i + j + len] = a - b;
            }
        }
    }
    for (int i = 0; i < 64; ++i) out64[i] = clamp_q12(u[i] >> 3);
}

static void apply_rope_host(int16_t* vec, int D, int pos) {
    int p_idx = pos < MAX_SEQ ? pos : (MAX_SEQ - 1);
    for (int k = 0; k < D / 2; ++k) {
        if (((k >> 3) & 1) != 0) continue;
        int freq = k & 7;
        int32_t c = h_rope_cos[p_idx][freq];
        int32_t s = h_rope_sin[p_idx][freq];
        int32_t x0 = vec[2 * k], x1 = vec[2 * k + 1];
        vec[2 * k]     = clamp_q12((x0 * c - x1 * s) >> 12);
        vec[2 * k + 1] = clamp_q12((x0 * s + x1 * c) >> 12);
    }
}

__host__ __device__ static inline int head_qk_off(int h) {
    return ((h & ~3) * 8) + ((h & 2) ? 16 : 0);
}

__host__ __device__ static inline int32_t calc_d_ind(
    int sh, int s, int t, int plen, const uint16_t* toks, const uint8_t* copy_tok_gate, const uint8_t* tok_len
) {
    if (sh < 2) return 0;
    if (s == t) return (t > 0) ? 3072 : 0;
    if (sh == 2) {
        if (s + 1 < plen && copy_tok_gate[toks[s + 1]] == 0) return 2048;
        if (s >= 1 && t >= 1 && toks[s - 1] == toks[t - 1] && toks[s] == toks[t]) return 0;
        bool t_delim = (copy_tok_gate[toks[t]] == 0);
        bool t_multi_byte = (tok_len[toks[t]] >= 2);
        int tp = -1;
        if (t_delim) {
            if (t >= 1 && copy_tok_gate[toks[t - 1]] > 0) {
                tp = t - 1;
            } else if (t >= 2 && copy_tok_gate[toks[t - 2]] > 0) {
                tp = t - 2;
            } else if (t >= 3 && copy_tok_gate[toks[t - 3]] > 0) {
                tp = t - 3;
            }
        }
        if (tp >= 0) {
            if (tp >= 1 && s >= 1 && toks[s - 1] == toks[tp - 1] && toks[s] == toks[tp]) return 64;
            if (copy_tok_gate[toks[s]] == 0 &&
                ((s >= 1 && toks[s - 1] == toks[tp]) ||
                 (s >= 2 && toks[s - 2] == toks[tp]) ||
                 (s >= 3 && toks[s - 3] == toks[tp] &&
                  copy_tok_gate[toks[s - 1]] == 0 && copy_tok_gate[toks[s - 2]] == 0))) return 128;
            if (toks[s] == toks[t]) return 384;
            if (toks[s] == toks[tp]) return 448;
            return 640;
        }
        if ((t_delim || t_multi_byte) && toks[s] == toks[t]) return 384;
        return 640;
    }
    return 0;
}

// ============================================================================
// Corpus & Eval Prompts Loader (`UORCORP3` / `UORPRMP1` / `UORVOCB9`)
// ============================================================================
struct SequenceSample {
    uint16_t plen;
    uint16_t rlen;
    uint16_t tokens[MAX_SEQ];
    uint8_t copy_flags[256];
    uint64_t prompt_nocopy[12];
};

struct EvalPromptItem {
    std::string id;
    std::string kind;
    std::string prompt;
    std::vector<uint16_t> prompt_ids;
};

static bool load_uorcorp3(const char* path, std::vector<SequenceSample>& out) {
    FILE* f = fopen(path, "rb");
    if (!f) { fprintf(stderr, "Failed to open %s\n", path); return false; }
    char magic[8];
    uint32_t n_rec = 0, max_seq = 0;
    if (fread(magic, 1, 8, f) != 8 || memcmp(magic, "UORCORP3", 8) != 0) {
        fprintf(stderr, "Bad UORCORP3 magic in %s\n", path); fclose(f); return false;
    }
    if (fread(&n_rec, 4, 1, f) != 1 || fread(&max_seq, 4, 1, f) != 1) { fclose(f); return false; }
    out.resize(n_rec);
    for (uint32_t i = 0; i < n_rec; ++i) {
        uint16_t pl = 0, rl = 0;
        if (fread(&pl, 2, 1, f) != 1 || fread(&rl, 2, 1, f) != 1) { fclose(f); return false; }
        if (pl > 768 || rl > 256 || pl + rl > MAX_SEQ) { fclose(f); return false; }
        SequenceSample& s = out[i];
        s.plen = pl; s.rlen = rl;
        memset(s.tokens, 0, sizeof(s.tokens));
        memset(s.copy_flags, 0, sizeof(s.copy_flags));
        memset(s.prompt_nocopy, 0, sizeof(s.prompt_nocopy));
        if (fread(s.tokens, 2, pl, f) != pl) { fclose(f); return false; }
        if (fread(s.tokens + pl, 2, rl, f) != rl) { fclose(f); return false; }
        if (fread(s.copy_flags, 1, rl, f) != rl) { fclose(f); return false; }
    }
    fclose(f);
    return true;
}

static void mark_prompt_nocopy_samples(std::vector<SequenceSample>& samples, const std::vector<std::string>& vocab) {
    static const char* kNoCopyTags[] = {
        "<|user|>", "<|assistant|>", "<|end|>",
        "<tool_call>", "</tool_call>", "<tool_response>", "</tool_response>"
    };
    for (auto& s : samples) {
        memset(s.prompt_nocopy, 0, sizeof(s.prompt_nocopy));
        std::string joined;
        std::vector<std::pair<size_t, size_t>> spans(s.plen);
        for (int idx = 0; idx < int(s.plen); ++idx) {
            size_t b0 = joined.size();
            uint16_t tid = s.tokens[idx];
            if (tid < vocab.size()) joined += vocab[tid];
            spans[idx] = {b0, joined.size()};
        }
        for (const char* tag : kNoCopyTags) {
            std::string ts(tag);
            size_t pos = 0;
            while ((pos = joined.find(ts, pos)) != std::string::npos) {
                size_t t0 = pos, t1 = pos + ts.size();
                for (int idx = 0; idx < int(s.plen); ++idx) {
                    if (spans[idx].second > t0 && spans[idx].first < t1) {
                        s.prompt_nocopy[idx >> 6] |= (1ULL << (idx & 63));
                    }
                }
                pos = t1;
            }
        }
    }
}

static bool load_uorprmp1(const char* path, std::vector<EvalPromptItem>& out) {
    FILE* f = fopen(path, "rb");
    if (!f) return false;
    char magic[8];
    uint32_t cnt = 0;
    if (fread(magic, 1, 8, f) != 8 || memcmp(magic, "UORPRMP1", 8) != 0) { fclose(f); return false; }
    if (fread(&cnt, 4, 1, f) != 1) { fclose(f); return false; }
    out.resize(cnt);
    for (uint32_t i = 0; i < cnt; ++i) {
        uint16_t idl=0, kl=0, prl=0, tokc=0;
        if (fread(&idl, 2, 1, f) != 1 || fread(&kl, 2, 1, f) != 1 || fread(&prl, 2, 1, f) != 1) { fclose(f); return false; }
        out[i].id.resize(idl); out[i].kind.resize(kl); out[i].prompt.resize(prl);
        if (fread(&out[i].id[0], 1, idl, f) != idl) { fclose(f); return false; }
        if (fread(&out[i].kind[0], 1, kl, f) != kl) { fclose(f); return false; }
        if (fread(&out[i].prompt[0], 1, prl, f) != prl) { fclose(f); return false; }
        if (fread(&tokc, 2, 1, f) != 1) { fclose(f); return false; }
        out[i].prompt_ids.resize(tokc);
        if (fread(out[i].prompt_ids.data(), 2, tokc, f) != tokc) { fclose(f); return false; }
    }
    fclose(f);
    return true;
}

static bool load_uorvocb9(const char* path, std::vector<std::string>& tok_str) {
    FILE* f = fopen(path, "rb");
    if (!f) return false;
    char magic[8];
    uint32_t vsz = 0;
    if (fread(magic, 1, 8, f) != 8 || memcmp(magic, "UORVOCB9", 8) != 0) { fclose(f); return false; }
    if (fread(&vsz, 4, 1, f) != 1 || vsz != VOCAB) { fclose(f); return false; }
    tok_str.resize(vsz);
    for (uint32_t i = 0; i < vsz; ++i) {
        uint8_t blen = 0;
        if (fread(&blen, 1, 1, f) != 1) { fprintf(stderr, "vocab EOF at %u\n", i); fclose(f); return false; }
        tok_str[i].resize(blen);
        if (blen > 0 && fread(&tok_str[i][0], 1, blen, f) != blen) { fprintf(stderr, "vocab str EOF at %u\n", i); fclose(f); return false; }
    }
    fclose(f);
    return true;
}

// ============================================================================
// Device Buffers & Coalesced Transposed Readout Views
// ============================================================================
struct ModelCfg {
    int L, D, H, G, quant_bits;
    int attn_res_shift, ffn_shift, d_shift, sk_blocks;
};

struct DevBuf {
    float* data = nullptr;
    float* m = nullptr;
    float* v = nullptr;
    float* grad = nullptr;
    size_t len = 0;
    void alloc(size_t n) {
        len = n;
        CHECK_CUDA(cudaMalloc(&data, n * sizeof(float)));
        CHECK_CUDA(cudaMalloc(&m, n * sizeof(float)));
        CHECK_CUDA(cudaMalloc(&v, n * sizeof(float)));
        CHECK_CUDA(cudaMalloc(&grad, n * sizeof(float)));
        CHECK_CUDA(cudaMemset(m, 0, n * sizeof(float)));
        CHECK_CUDA(cudaMemset(v, 0, n * sizeof(float)));
        CHECK_CUDA(cudaMemset(grad, 0, n * sizeof(float)));
    }
    void free_all() {
        if (data) cudaFree(data);
        if (m) cudaFree(m);
        if (v) cudaFree(v);
        if (grad) cudaFree(grad);
    }
};

struct QuantDev {
    ModelCfg cfg;
    int16_t* e_tok;
    int16_t* q_lut;
    int16_t* k_lut;
    int16_t* v_lut;
    int16_t* ffn_lut;
    // Transposed [dim * VOCAB + v] layout for 100% coalesced warp reads in readout_scores_kernel
    uint8_t* tmac_hi_T;  // [G * VOCAB]
    uint8_t* tmac_lo_T;  // [G * VOCAB]
    int16_t* p_out_T;    // [D * VOCAB]
    uint64_t* p_sk_T;    // [sk_blocks * VOCAB]
    float* w_eff_T;      // [D * VOCAB]
    int16_t* b_out;      // [VOCAB]
    uint16_t* head_tau;
    uint16_t* head_slope;
    int16_t* head_copy_b;
    uint8_t* head_copy_gate;
    uint8_t* copy_tok_gate;
    uint8_t* tok_len;
};

struct GradDev {
    float* e_tok;
    float* q_lut;
    float* k_lut;
    float* v_lut;
    float* ffn_lut;
    float* w_out;
    float* p_out;
    float* b_out;
    float* head_copy_b;
    float* head_copy_w;
};

// Dynamic stride scratch workspace allocated once for max(batch_sz, L, MAX_SEQ, D)
struct TrunkScratchDev {
    int max_b, max_t, max_rep;
    int16_t*  q_rot;     // [B * L * T * D]
    int16_t*  k_rot;     // [B * L * T * D]
    int16_t*  v_vec;     // [B * L * T * D]
    int16_t*  x_in;      // [B * L * T * D]
    int16_t*  x_mid;     // [B * L * T * D]
    uint8_t*  q_code;    // [B * L * T * G]
    uint8_t*  k_code;    // [B * L * T * G]
    uint8_t*  ffn_code;  // [B * L * T * G]
    uint8_t*  ffn_hinge; // [B * L * T * D]
    float*    d_x;       // [B * T * D]
    float*    d_x_next;  // [B * T * D]
    float*    d_q;       // [B * T * D]
    float*    d_k;       // [B * T * D]
    float*    d_v;       // [B * T * D]
    int16_t*  rep_x_top;     // [max_rep * D]
    int16_t*  rep_copy_log;  // [max_rep * VOCAB]
    float*    rep_gv;        // [max_rep * VOCAB]
    int*      rep_b;         // [max_rep]
    int*      rep_t;         // [max_rep]
    int*      rep_k_rep;     // [max_rep]
    uint16_t* rep_target;    // [max_rep]
    uint8_t*  rep_is_copy;   // [max_rep]
    int*      step_rep_idx;  // [B * T]: -1 if not reply step, else rep index
    uint16_t* step_pred_v;   // [B * T]
    float*    step_prob_p;   // [B * T]
    float*    step_prob_t;   // [B * T]
    uint64_t* step_copy_act; // [B * T]: bit63=is_copy, bits 0..31=active copy heads
};

// ============================================================================
// Device Quantization & Optimizer Kernels
// ============================================================================
__global__ void quantize_array_i8_kernel(const float* src, int16_t* dst, size_t n, uint16_t scale_q8) {
    size_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        int32_t q = int32_t(lrintf((src[idx] * 16.0f) / float(scale_q8)));
        if (q < -127) q = -127;
        if (q > 127) q = 127;
        dst[idx] = dequant_i8_q12(int8_t(q), scale_q8);
    }
}

__global__ void quantize_array_i4_kernel(const float* src, int16_t* dst, size_t n, uint16_t scale_q8) {
    size_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < n) {
        int32_t q4 = int32_t(lrintf((src[idx] * 2.0f) / float(scale_q8)));
        if (q4 < -8) q4 = -8;
        if (q4 > 7) q4 = 7;
        dst[idx] = dequant_i4_q12(int8_t(q4), scale_q8);
    }
}

__global__ void quantize_tmac_readout_T_kernel(const float* w_out, uint8_t* hi_T, uint8_t* lo_T,
                                               float* w_eff_T, int16_t* p_out_T, uint64_t* p_sk_T,
                                               const float* p_out_f, const float* b_out_f, int16_t* b_out_i,
                                               uint16_t scale_p, int D, int G, int sk_blocks) {
    int v = blockIdx.x * blockDim.x + threadIdx.x;
    if (v >= VOCAB) return;
    b_out_i[v] = int16_t(fmaxf(-4096.0f, fminf(4096.0f, b_out_f[v])));
    int16_t local_p[D_MAX];
    for (int c = 0; c < D; ++c) {
        int32_t q = int32_t(lrintf((p_out_f[v * D + c] * 16.0f) / float(scale_p)));
        if (q < -127) q = -127;
        if (q > 127) q = 127;
        int16_t dq = dequant_i8_q12(int8_t(q), scale_p);
        local_p[c] = dq;
        p_out_T[size_t(c) * VOCAB + v] = dq;
    }
    for (int b = 0; b < sk_blocks; ++b) {
        p_sk_T[size_t(b) * VOCAB + v] = pack_sign64(local_p + b * 64);
    }
    for (int g = 0; g < G; ++g) {
        int hi_code = 0, lo_code = 0, mul = 1;
        for (int k = 0; k < 4; ++k) {
            int c = g * 4 + k;
            float wf = fmaxf(-4.0f, fminf(4.0f, w_out[v * D + c]));
            int lvl = int(lrintf(wf));
            if (lvl < -4) lvl = -4;
            if (lvl > 4) lvl = 4;
            int thi = (lvl >= 2) ? 1 : (lvl <= -2 ? -1 : 0);
            int tlo = lvl - 3 * thi;
            hi_code += (thi + 1) * mul;
            lo_code += (tlo + 1) * mul;
            mul *= 3;
            w_eff_T[size_t(c) * VOCAB + v] = float(lvl);
        }
        hi_T[size_t(g) * VOCAB + v] = uint8_t(hi_code);
        lo_T[size_t(g) * VOCAB + v] = uint8_t(lo_code);
    }
}

__global__ void quantize_copy_gates_kernel(const float* head_copy_w, const float* head_copy_b_f,
                                           uint8_t* gate_out, int16_t* b_out, int D, int H, int G) {
    int h = threadIdx.x;
    if (h >= H) return;
    b_out[h] = int16_t(fmaxf(-16000.0f, fminf(16000.0f, head_copy_b_f[h])));
    for (int g = 0; g < G; ++g) {
        int code = 0, mul = 1;
        for (int k = 0; k < 4; ++k) {
            float val = head_copy_w[h * D + g * 4 + k];
            int t = (val > 0.25f) ? 1 : (val < -0.25f ? -1 : 0);
            code += (t + 1) * mul;
            mul *= 3;
        }
        gate_out[h * G + g] = uint8_t(code);
    }
}

__global__ void clamp_copy_bias_kernel(float* cb, int H) {
    int h = threadIdx.x;
    if (h >= H) return;
    float v = cb[h];
    if ((h & 3) >= 2) {
        cb[h] = fmaxf(-192.0f, fminf(-24.0f, v));
    } else {
        cb[h] = fmaxf(-1024.0f, fminf(-512.0f, v));
    }
}

__global__ void adam_step_kernel(float* data, float* m_buf, float* v_buf, float* grad, size_t n,
                                 float lr, float beta1, float beta2, float bc1, float bc2,
                                 float eps, float max_step, float clip_lo, float clip_hi) {
    size_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;
    float g = grad[idx];
    grad[idx] = 0.0f;
    if (!isfinite(g)) g = 0.0f;
    float m = m_buf[idx];
    float v = v_buf[idx];
    if (g == 0.0f && fabsf(m) < 1e-6f && v < 1e-9f) {
        if (m != 0.0f) m_buf[idx] = 0.0f;
        if (v != 0.0f) v_buf[idx] = 0.0f;
        return;
    }
    m = beta1 * m + (1.0f - beta1) * g;
    v = beta2 * v + (1.0f - beta2) * g * g;
    if (fabsf(m) < 1e-7f && v < 1e-11f) {
        m_buf[idx] = 0.0f;
        v_buf[idx] = 0.0f;
        return;
    }
    m_buf[idx] = m;
    v_buf[idx] = v;
    float m_hat = m / bc1;
    float v_hat = v / bc2;
    float raw_step = lr * m_hat / (sqrtf(v_hat) + eps);
    float step = (max_step > 1e-6f) ? (max_step * tanhf(raw_step / max_step)) : raw_step;
    float w = data[idx] - step;
    if (w > clip_hi) w = clip_hi;
    if (w < clip_lo) w = clip_lo;
    data[idx] = w;
}

__device__ static void fwht64_shared(const int16_t* in_d, int16_t* out_d, int32_t* tmp_d, int tid, int D) {
    if (tid < D) tmp_d[tid] = int32_t(in_d[tid]);
    __syncthreads();
    int blk = tid / 64;
    int loc = tid % 64;
    for (int len = 1; len < 64; len <<= 1) {
        int32_t next_val = 0;
        if (tid < D) {
            int partner = loc ^ len;
            int32_t a = tmp_d[blk * 64 + loc];
            int32_t b = tmp_d[blk * 64 + partner];
            next_val = (loc < partner) ? (a + b) : (b - a);
        }
        __syncthreads();
        if (tid < D) tmp_d[blk * 64 + loc] = next_val;
        __syncthreads();
    }
    if (tid < D) out_d[tid] = clamp_q12(tmp_d[tid] >> 3);
    __syncthreads();
}

__device__ static void fwht64_t_float_shared(float* vec, int tid, int D) {
    // Exact transpose (inverse) of forward `fwht64_shared` (linear scaling `1/8` per 64-block).
    __syncthreads();
    int blk = tid / 64;
    int loc = tid % 64;
    for (int len = 32; len >= 1; len >>= 1) {
        float next_val = 0.0f;
        if (tid < D) {
            int partner = loc ^ len;
            float a = vec[blk * 64 + loc];
            float b = vec[blk * 64 + partner];
            next_val = (loc < partner) ? (a + b) : (b - a);
        }
        __syncthreads();
        if (tid < D) vec[blk * 64 + loc] = next_val;
        __syncthreads();
    }
    if (tid < D) vec[tid] *= 0.125f;
    __syncthreads();
}

// ============================================================================
// Kernel 1: Forward Causal Trunk (4-head parallel PRLL + copy_logits pack)
// ============================================================================
__global__ void fwd_trunk_kernel(
    const SequenceSample* samples,
    const int* batch_indices,
    const int* rep_offsets,
    int T_stride,
    QuantDev qm,
    TrunkScratchDev sc
) {
    int b = blockIdx.x;
    int tid = threadIdx.x;
    const SequenceSample& seq = samples[batch_indices[b]];
    const int L = qm.cfg.L;
    const int D = qm.cfg.D;
    const int H = qm.cfg.H;
    const int G = qm.cfg.G;
    const int attn_res_shift = qm.cfg.attn_res_shift;
    const int ffn_shift = qm.cfg.ffn_shift;
    const int plen = int(seq.plen);
    const int rlen = int(seq.rlen);
    const int T = plen + rlen;
    const int rep0 = rep_offsets[b];

    auto idx_ld = [&](int l, int t) -> size_t {
        return (((size_t(b) * L + l) * T_stride) + t) * D;
    };
    auto idx_lg = [&](int l, int t) -> size_t {
        return (((size_t(b) * L + l) * T_stride) + t) * G;
    };
    auto idx_td = [&](int t) -> size_t {
        return (size_t(b) * T_stride + t) * D;
    };

    __shared__ int16_t s_x[D_MAX], s_kin[D_MAX], s_u[D_MAX], s_uk[D_MAX];
    __shared__ int16_t s_q[D_MAX], s_k[D_MAX], s_v[D_MAX], s_attn[D_MAX], s_xmid[D_MAX], s_uffn[D_MAX];
    __shared__ int32_t s_tmp[D_MAX];
    __shared__ uint8_t s_qc[G_MAX], s_kc[G_MAX], s_fc[G_MAX];
    __shared__ int16_t s_act_lut[G_MAX][81]; // 10,368 B
    __shared__ uint32_t s_w4[4][MAX_SEQ];    // 16,384 B (4 heads parallel)
    __shared__ uint64_t s_sum4[4][64];
    __shared__ uint64_t s_bw4[4][64];
    __shared__ int s_bs4[4][64];
    __shared__ int32_t s_gate_sum[H_MAX];

    for (int t = 0; t < T; ++t) {
        if (tid == 0) sc.step_rep_idx[b * T_stride + t] = -1;
        if (tid < D) sc.d_x[idx_td(t) + tid] = 0.0f;
        uint32_t tok_t = seq.tokens[t];
        if (tid < D) s_x[tid] = qm.e_tok[tok_t * D + tid];
        __syncthreads();

        for (int l = 0; l < L; ++l) {
            size_t b_ld = idx_ld(l, t);
            size_t b_lg = idx_lg(l, t);
            if (tid < D) {
                sc.x_in[b_ld + tid] = s_x[tid];
                int32_t prev_x = (t > 0) ? int32_t(sc.x_in[idx_ld(l, t - 1) + (tid ^ 1)]) : 0;
                s_kin[tid] = clamp_q12(((int32_t(s_x[tid]) * 3) + prev_x) >> 2);
            }
            __syncthreads();

            fwht64_shared(s_x, s_u, s_tmp, tid, D);
            fwht64_shared(s_kin, s_uk, s_tmp, tid, D);

            if (tid < G) {
                s_qc[tid] = code_4ch(&s_u[tid * 4]);
                s_kc[tid] = code_4ch(&s_uk[tid * 4]);
                sc.q_code[b_lg + tid] = s_qc[tid];
                sc.k_code[b_lg + tid] = s_kc[tid];
            }
            __syncthreads();

            if (tid < D) {
                int32_t qa = 0, ka = 0, va = 0;
                const size_t lbase = size_t(l) * G * 256 * QKV_OUT;
                for (int k = 0; k < 4; ++k) {
                    int o = ((tid & 3) + k * 4);
                    int g = ((tid + D - o) % D) >> 2;
                    uint8_t qc = s_qc[g], kc = s_kc[g];
                    qa += int32_t(qm.q_lut[lbase + (size_t(g) * 256 + qc) * QKV_OUT + o]);
                    ka += int32_t(qm.k_lut[lbase + (size_t(g) * 256 + kc) * QKV_OUT + o]);
                    va += int32_t(qm.v_lut[lbase + (size_t(g) * 256 + qc) * QKV_OUT + o]);
                }
                s_q[tid] = clamp_q12((qa >> 2) + (int32_t(s_u[tid]) >> 1));
                s_k[tid] = clamp_q12((ka >> 2) + (int32_t(s_uk[tid]) >> 1));
                s_v[tid] = clamp_q12((va >> 2) + (int32_t(s_x[tid]) >> 1));
            }
            __syncthreads();

            if (tid < D / 2 && ((tid >> 3) & 1) == 0) {
                int p_idx = (t < MAX_SEQ) ? t : (MAX_SEQ - 1);
                int freq = tid & 7;
                int32_t c = d_rope_cos[p_idx][freq], s = d_rope_sin[p_idx][freq];
                int32_t q0 = s_q[2 * tid], q1 = s_q[2 * tid + 1];
                int32_t k0 = s_k[2 * tid], k1 = s_k[2 * tid + 1];
                s_q[2 * tid]     = clamp_q12((q0 * c - q1 * s) >> 12);
                s_q[2 * tid + 1] = clamp_q12((q0 * s + q1 * c) >> 12);
                s_k[2 * tid]     = clamp_q12((k0 * c - k1 * s) >> 12);
                s_k[2 * tid + 1] = clamp_q12((k0 * s + k1 * c) >> 12);
            }
            __syncthreads();

            if (tid < D) {
                sc.q_rot[b_ld + tid] = s_q[tid];
                sc.k_rot[b_ld + tid] = s_k[tid];
                sc.v_vec[b_ld + tid] = s_v[tid];
            }
            __syncthreads();

            // 4-head parallel PRLL attention (`h_loc = 0..3`, `h_tid = 0..63`)
            int h_loc = tid >> 6;
            int h_tid = tid & 63;
            for (int h0 = 0; h0 < H; h0 += 4) {
                int h = h0 + h_loc;
                int v_off = h * 8;
                int qk_off = head_qk_off(h);
                int sh = h & 3;
                int32_t tau = int32_t(qm.head_tau[l * H + h]);
                if (tau < 256) tau = 256;
                int32_t slope = int32_t(qm.head_slope[l * H + h]);
                uint16_t q_mask16 = 0;
                int32_t q16[16];
                for (int j = 0; j < 16; ++j) {
                    int32_t qv = int32_t(s_q[(qk_off + j) % D]);
                    q16[j] = qv;
                    if (qv >= 0) q_mask16 |= (1u << j);
                }
                uint64_t loc_min = UINT64_MAX;
                for (int s = h_tid; s <= t; s += 64) {
                    size_t k_off = idx_ld(l, s);
                    int64_t dot16 = 0;
                    int32_t d_l1 = 0;
                    uint16_t k_mask16 = 0;
                    for (int j = 0; j < 16; ++j) {
                        int32_t kv = int32_t(sc.k_rot[k_off + ((qk_off + j) % D)]);
                        dot16 += int64_t(q16[j]) * int64_t(kv);
                        int32_t diff = q16[j] - kv;
                        d_l1 += (diff < 0 ? -diff : diff);
                        if (kv >= 0) k_mask16 |= (1u << j);
                    }
                    int32_t ds = int32_t(dot16 >> 10);
                    ds = ds < -4096 ? -4096 : (ds > 4096 ? 4096 : ds);
                    int32_t d_ind = calc_d_ind(sh, s, t, int(seq.plen), seq.tokens, qm.copy_tok_gate, qm.tok_len);
                    int32_t d_tot = (4096 - ds) + (d_l1 >> 4) + (__popc(uint32_t(q_mask16 ^ k_mask16)) << 5) + (((t - s) * slope) >> 3) + d_ind;
                    if (d_tot < 0) d_tot = 0;
                    s_w4[h_loc][s] = uint32_t(d_tot);
                    uint64_t packed = (uint64_t(uint32_t(d_tot)) << 32) | uint32_t(s);
                    if (packed < loc_min) loc_min = packed;
                }
                s_bw4[h_loc][h_tid] = loc_min;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st && s_bw4[h_loc][h_tid + st] < s_bw4[h_loc][h_tid]) {
                        s_bw4[h_loc][h_tid] = s_bw4[h_loc][h_tid + st];
                    }
                    __syncthreads();
                }
                int32_t min_d = int32_t(uint32_t(s_bw4[h_loc][0] >> 32));
                int bs = int(uint32_t(s_bw4[h_loc][0] & 0xFFFFFFFFu));
                bool sharp_rel = (sh >= 2);
                uint64_t loc_sum = 0;
                for (int s = h_tid; s <= t; s += 64) {
                    int32_t d_eff = sharp_rel ? ((int32_t(s_w4[h_loc][s]) - min_d) << 2) : int32_t(s_w4[h_loc][s]);
                    uint32_t w = prll_weight_order4_v10(tau, d_eff);
                    s_w4[h_loc][s] = w;
                    loc_sum += w;
                }
                s_sum4[h_loc][h_tid] = loc_sum;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st) {
                        s_sum4[h_loc][h_tid] += s_sum4[h_loc][h_tid + st];
                    }
                    __syncthreads();
                }
                uint64_t sr = s_sum4[h_loc][0] > 0 ? s_sum4[h_loc][0] : 1ULL;
                uint64_t lmass = 0;
                for (int s = h_tid; s <= t; s += 64) {
                    uint32_t wp = uint32_t((uint64_t(s_w4[h_loc][s]) * PRLL_QUANTUM) / sr);
                    s_w4[h_loc][s] = wp;
                    lmass += wp;
                }
                s_sum4[h_loc][h_tid] = lmass;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st) s_sum4[h_loc][h_tid] += s_sum4[h_loc][h_tid + st];
                    __syncthreads();
                }
                if (h_tid == 0) s_w4[h_loc][bs] += uint32_t(PRLL_QUANTUM - s_sum4[h_loc][0]);
                __syncthreads();

                if (h_tid < 8) {
                    int64_t acc = 0;
                    int ch = v_off + h_tid;
                    for (int s = 0; s <= t; ++s) {
                        acc += int64_t(s_w4[h_loc][s]) * int64_t(sc.v_vec[idx_ld(l, s) + ch]);
                    }
                    s_attn[ch] = clamp_q12(int32_t(acc / int64_t(PRLL_QUANTUM)));
                }
                __syncthreads();
            }

            if (tid < D) {
                s_xmid[tid] = clamp_q12(int32_t(s_x[tid]) + (int32_t(s_attn[tid]) >> attn_res_shift));
                sc.x_mid[b_ld + tid] = s_xmid[tid];
            }
            __syncthreads();

            fwht64_shared(s_xmid, s_uffn, s_tmp, tid, D);
            if (tid < G) {
                s_fc[tid] = code_4ch(&s_uffn[tid * 4]);
                sc.ffn_code[b_lg + tid] = s_fc[tid];
            }
            __syncthreads();

            if (tid < D) {
                int32_t ffn_acc = 0;
                const size_t fbase = size_t(l) * G * 256 * D;
                for (int g = 0; g < G; ++g) {
                    ffn_acc += int32_t(qm.ffn_lut[fbase + (size_t(g) * 256 + s_fc[g]) * D + tid]);
                }
                int32_t ffn_pre = ffn_acc >> ffn_shift;
                sc.ffn_hinge[b_ld + tid] = (ffn_pre < -2048) ? 1 : 0;
                s_x[tid] = clamp_q12(int32_t(s_xmid[tid]) + int32_t(icosian_hinge(ffn_pre)));
            }
            __syncthreads();
        } // end layers

        if (t >= plen - 1 && t < T - 1) {
            int k_rep = t - (plen - 1);
            int r_idx = rep0 + k_rep;
            if (tid < D) sc.rep_x_top[size_t(r_idx) * D + tid] = s_x[tid];
            if (tid == 0) {
                sc.step_rep_idx[b * T_stride + t] = r_idx;
                sc.rep_b[r_idx] = b;
                sc.rep_t[r_idx] = t;
                sc.rep_k_rep[r_idx] = k_rep;
                sc.rep_target[r_idx] = seq.tokens[t + 1];
                sc.rep_is_copy[r_idx] = seq.copy_flags[k_rep];
            }

            if (tid < G) {
                int32_t buf81[81];
                build_tmac_act_lut81(&s_x[tid * 4], buf81);
                for (int q = 0; q < 81; ++q) s_act_lut[tid][q] = int16_t(buf81[q]);
            }
            __syncthreads();
            if (tid < H) {
                int32_t gs = int32_t(qm.head_copy_b[tid]);
                for (int g = 0; g < G; ++g) gs += int32_t(s_act_lut[g][qm.head_copy_gate[tid * G + g]]);
                s_gate_sum[tid] = gs;
            }
            __syncthreads();
            if (tid == 0) {
                uint64_t cm = 0ULL;
                for (int h = 0; h < H && h < 32; ++h) {
                    if (s_gate_sum[h] > 0) cm |= (1ULL << h);
                    if (s_gate_sum[h] > -224) cm |= (1ULL << (32 + h));
                }
                sc.step_copy_act[b * T_stride + t] = cm;
            }
            __syncthreads();

            int16_t* my_copy = &sc.rep_copy_log[size_t(r_idx) * VOCAB];
            for (int v = tid; v < VOCAB; v += 256) my_copy[v] = 0;
            __syncthreads();
            if (t > 0) {
                int h_loc = tid >> 6;
                int h_tid = tid & 63;
                int copy_s_lim = (plen > 1) ? (t < (plen - 1) ? t : (plen - 1)) : 0;
                int shift_bits = 20 + qm.cfg.d_shift;
                unsigned int* hacc = reinterpret_cast<unsigned int*>(&sc.rep_gv[size_t(r_idx) * VOCAB]);
                for (int h0 = 0; h0 < H; h0 += 4) {
                    int h = h0 + h_loc;
                    int qk_off = head_qk_off(h);
                    int sh = h & 3;
                    int32_t tau = int32_t(qm.head_tau[(L - 1) * H + h]); if (tau < 256) tau = 256;
                    int32_t slope = int32_t(qm.head_slope[(L - 1) * H + h]);
                    uint16_t q_mask16 = 0; int32_t q16[16];
                    for (int j = 0; j < 16; ++j) {
                        int32_t qv = int32_t(sc.q_rot[idx_ld(L - 1, t) + ((qk_off + j) % D)]);
                        q16[j] = qv; if (qv >= 0) q_mask16 |= (1u << j);
                    }
                    uint64_t loc_min = UINT64_MAX;
                    for (int s = h_tid; s <= t; s += 64) {
                        size_t k_off = idx_ld(L - 1, s);
                        int64_t dot16 = 0; int32_t d_l1 = 0; uint16_t k_mask16 = 0;
                        for (int j = 0; j < 16; ++j) {
                            int32_t kv = int32_t(sc.k_rot[k_off + ((qk_off + j) % D)]);
                            dot16 += int64_t(q16[j]) * int64_t(kv);
                            int32_t diff = q16[j] - kv; d_l1 += (diff < 0 ? -diff : diff);
                            if (kv >= 0) k_mask16 |= (1u << j);
                        }
                        int32_t ds = int32_t(dot16 >> 10); ds = ds < -4096 ? -4096 : (ds > 4096 ? 4096 : ds);
                        int32_t d_ind = calc_d_ind(sh, s, t, int(seq.plen), seq.tokens, qm.copy_tok_gate, qm.tok_len);
                        int32_t d_tot = (4096 - ds) + (d_l1 >> 4) + (__popc(uint32_t(q_mask16 ^ k_mask16)) << 5) + (((t - s) * slope) >> 3) + d_ind;
                        if (d_tot < 0) d_tot = 0;
                        s_w4[h_loc][s] = uint32_t(d_tot);
                        uint64_t packed = (uint64_t(uint32_t(d_tot)) << 32) | uint32_t(s);
                        if (packed < loc_min) loc_min = packed;
                    }
                    s_bw4[h_loc][h_tid] = loc_min;
                    __syncthreads();
                    for (int st = 32; st > 0; st >>= 1) {
                        if (h_tid < st && s_bw4[h_loc][h_tid + st] < s_bw4[h_loc][h_tid]) {
                            s_bw4[h_loc][h_tid] = s_bw4[h_loc][h_tid + st];
                        }
                        __syncthreads();
                    }
                    int32_t min_d = int32_t(uint32_t(s_bw4[h_loc][0] >> 32));
                    int bs = int(uint32_t(s_bw4[h_loc][0] & 0xFFFFFFFFu));
                    bool sharp_rel = (sh >= 2);
                    uint64_t loc_sum = 0;
                    for (int s = h_tid; s <= t; s += 64) {
                        int32_t d_eff = sharp_rel ? ((int32_t(s_w4[h_loc][s]) - min_d) << 2) : int32_t(s_w4[h_loc][s]);
                        uint32_t w = prll_weight_order4_v10(tau, d_eff);
                        s_w4[h_loc][s] = w;
                        loc_sum += w;
                    }
                    s_sum4[h_loc][h_tid] = loc_sum;
                    __syncthreads();
                    for (int st = 32; st > 0; st >>= 1) {
                        if (h_tid < st) s_sum4[h_loc][h_tid] += s_sum4[h_loc][h_tid + st];
                        __syncthreads();
                    }
                    uint64_t sr = s_sum4[h_loc][0] > 0 ? s_sum4[h_loc][0] : 1ULL;
                    uint64_t lmass = 0;
                    for (int s = h_tid; s <= t; s += 64) {
                        uint32_t wp = uint32_t((uint64_t(s_w4[h_loc][s]) * PRLL_QUANTUM) / sr);
                        s_w4[h_loc][s] = wp; lmass += wp;
                    }
                    s_sum4[h_loc][h_tid] = lmass;
                    __syncthreads();
                    for (int st = 32; st > 0; st >>= 1) { if (h_tid < st) s_sum4[h_loc][h_tid] += s_sum4[h_loc][h_tid + st]; __syncthreads(); }
                    if (h_tid == 0) s_w4[h_loc][bs] += uint32_t(PRLL_QUANTUM - s_sum4[h_loc][0]);
                    __syncthreads();

                    for (int sub = 0; sub < 4; ++sub) {
                        int hh = h0 + sub;
                        int32_t sgs = s_gate_sum[hh];
                        int32_t sgs_norm = (D >= 256) ? (sgs >> 1) : sgs;
                        int32_t scg = (sgs_norm > 0) ? (sgs_norm > 2048 ? 2048 : sgs_norm) : 0;
                        if (scg == 0 || copy_s_lim <= 0) continue;
                        for (int v = tid; v < VOCAB; v += 256) hacc[v] = 0u;
                        __syncthreads();
                        for (int s = tid; s < copy_s_lim; s += 256) {
                            bool s1_ok = ((seq.prompt_nocopy[(s + 1) >> 6] >> ((s + 1) & 63)) & 1ULL) == 0ULL;
                            int tok_n = int(seq.tokens[s + 1]);
                            if (s1_ok && tok_n >= 3 && s_w4[sub][s] > 0 && qm.copy_tok_gate[tok_n] > 0) {
                                uint32_t contrib = uint32_t((uint64_t(s_w4[sub][s]) * uint64_t(scg)) >> 6);
                                if (contrib > 0) atomicAdd(&hacc[tok_n], contrib);
                            }
                        }
                        __syncthreads();
                        for (int v = tid; v < VOCAB; v += 256) {
                            uint32_t ha = hacc[v];
                            uint32_t ctg = uint32_t(qm.copy_tok_gate[v]);
                            if (ha > 0 && ctg > 0) {
                                int32_t cand = int32_t((uint64_t(ha) * uint64_t(ctg)) >> shift_bits);
                                if (cand > 1536) cand = 1536;
                                int32_t next_v = int32_t(my_copy[v]) + cand;
                                my_copy[v] = int16_t(next_v > 4608 ? 4608 : next_v);
                            }
                        }
                        __syncthreads();
                    }
                }
            }
        }
        __syncthreads();
    }
}

// ============================================================================
// Kernel 2: Coalesced 100%-SM Readout Scores + Top `d_x` Gradient
// (`N_rep` blocks x `256` threads; 16 vocab items per thread in registers!)
// ============================================================================
__global__ void readout_scores_kernel(
    int N_rep,
    int T_stride,
    QuantDev qm,
    TrunkScratchDev sc,
    unsigned long long* d_loss_acc,
    unsigned int* d_corr_acc,
    unsigned int* d_tot_acc,
    int phase_ste
) {
    int r = blockIdx.x;
    if (r >= N_rep) return;
    int tid = threadIdx.x;
    const int D = qm.cfg.D;
    const int G = qm.cfg.G;
    const int d_shift = qm.cfg.d_shift;
    const int sk_blocks = qm.cfg.sk_blocks;
    static constexpr float SOFTMAX_TEMP = 256.0f;

    int b = sc.rep_b[r];
    int t = sc.rep_t[r];
    int k_rep = sc.rep_k_rep[r];
    int target_v = int(sc.rep_target[r]);

    __shared__ int16_t s_x[D_MAX];
    __shared__ int16_t s_act_lut[G_MAX][81];
    __shared__ uint64_t s_h_sk[4];
    __shared__ int32_t s_neg_sc[256];
    __shared__ int s_neg_id[256];
    __shared__ int32_t s_max_sc[256];
    __shared__ float s_red[256];
    __shared__ float s_dx[D_MAX];
    __shared__ int32_t s_ts;
    __shared__ int32_t s_mall;
    __shared__ float s_invz;

    if (tid < D) {
        s_x[tid] = sc.rep_x_top[size_t(r) * D + tid];
        s_dx[tid] = 0.0f;
    }
    if (tid == 0) s_ts = -2000000000;
    __syncthreads();
    if (tid < G) {
        int32_t buf81[81];
        build_tmac_act_lut81(&s_x[tid * 4], buf81);
        for (int q = 0; q < 81; ++q) s_act_lut[tid][q] = int16_t(buf81[q]);
    }
    if (tid < sk_blocks) s_h_sk[tid] = pack_sign64(&s_x[tid * 64]);
    __syncthreads();

    int32_t loc_sc[16];
    int32_t loc_best_neg = -2000000000;
    int loc_best_neg_id = 1;
    int32_t loc_max = -2000000000;
    int32_t targ_sc = -2000000000;
    const int16_t* copy_row = &sc.rep_copy_log[size_t(r) * VOCAB];

    for (int m = 0; m < 16; ++m) {
        int v = tid + m * 256;
        if (v == 0 || v == 2 || (v == 1 && k_rep == 0)) {
            loc_sc[m] = -2000000000;
            continue;
        }
        int32_t tmac = 0;
        for (int g = 0; g < G; ++g) {
            uint8_t hi = qm.tmac_hi_T[size_t(g) * VOCAB + v];
            uint8_t lo = qm.tmac_lo_T[size_t(g) * VOCAB + v];
            tmac += (s_act_lut[g][hi] << 1) + s_act_lut[g][hi] + s_act_lut[g][lo];
        }
        int32_t l1 = 0;
        for (int c = 0; c < D; ++c) {
            int32_t diff = int32_t(s_x[c]) - int32_t(qm.p_out_T[size_t(c) * VOCAB + v]);
            l1 += (diff < 0 ? -diff : diff);
        }
        int32_t ham = 0;
        for (int blk = 0; blk < sk_blocks; ++blk) {
            ham += __popcll(s_h_sk[blk] ^ qm.p_sk_T[size_t(blk) * VOCAB + v]);
        }
        int32_t pure_s = (tmac >> d_shift) + (int32_t(qm.b_out[v]) * 2) - ((ham * 24) >> d_shift) - (l1 >> (5 + d_shift));
        int32_t sc_tot = pure_s + int32_t(copy_row[v]);
        loc_sc[m] = sc_tot;
        if (sc_tot > loc_max) loc_max = sc_tot;
        if (v == target_v) {
            targ_sc = sc_tot;
        } else if (sc_tot > loc_best_neg) {
            loc_best_neg = sc_tot;
            loc_best_neg_id = v;
        }
    }
    s_neg_sc[tid] = loc_best_neg;
    s_neg_id[tid] = loc_best_neg_id;
    s_max_sc[tid] = loc_max;
    if (targ_sc > -1900000000) s_ts = targ_sc;
    __syncthreads();
    for (int st = 128; st > 0; st >>= 1) {
        if (tid < st) {
            if (s_neg_sc[tid + st] > s_neg_sc[tid]) {
                s_neg_sc[tid] = s_neg_sc[tid + st];
                s_neg_id[tid] = s_neg_id[tid + st];
            }
            if (s_max_sc[tid + st] > s_max_sc[tid]) {
                s_max_sc[tid] = s_max_sc[tid + st];
            }
        }
        __syncthreads();
    }
    if (tid == 0) s_mall = s_max_sc[0];
    __syncthreads();

    int32_t best_neg_sc = s_neg_sc[0];
    int best_neg_v = s_neg_id[0];
    int32_t t_sc = s_ts;
    int32_t m_all = s_mall;

    float loc_ex[16];
    float loc_sum = 0.0f;
    for (int m = 0; m < 16; ++m) {
        if (loc_sc[m] <= -1900000000) {
            loc_ex[m] = 0.0f;
        } else {
            float ex = expf(fmaxf(-32.0f, float(loc_sc[m] - m_all) / SOFTMAX_TEMP));
            loc_ex[m] = ex;
            loc_sum += ex;
        }
    }
    s_red[tid] = loc_sum;
    __syncthreads();
    for (int st = 128; st > 0; st >>= 1) {
        if (tid < st) s_red[tid] += s_red[tid + st];
        __syncthreads();
    }
    if (tid == 0) s_invz = 1.0f / fmaxf(s_red[0], 1e-8f);
    __syncthreads();

    float inv_z = s_invz;
    float p_t = (t_sc > -1900000000) ? (expf(fmaxf(-32.0f, float(t_sc - m_all) / SOFTMAX_TEMP)) * inv_z) : 1e-6f;
    p_t = fminf(1.0f, fmaxf(1e-6f, p_t));
    float err_f = fmaxf(0.0f, 1.0f - p_t);
    bool correct = (t_sc > best_neg_sc);

    if (tid == 0) {
        float ce_loss = -logf(fmaxf(p_t, 1e-6f));
        atomicAdd(d_loss_acc, (unsigned long long)(llroundf(ce_loss * 1000.0f)));
        atomicAdd(d_tot_acc, 1u);
        if (correct) atomicAdd(d_corr_acc, 1u);
        sc.step_pred_v[b * T_stride + t] = uint16_t(correct ? target_v : best_neg_v);
        sc.step_prob_p[b * T_stride + t] = err_f;
        sc.step_prob_t[b * T_stride + t] = p_t;
    }

    if (!phase_ste) return;

    float loc_pos_sum = 0.0f;
    for (int m = 0; m < 16; ++m) {
        int v = tid + m * 256;
        if (loc_ex[m] <= 0.0f || v == target_v) continue;
        float p_v = loc_ex[m] * inv_z;
        if (v == best_neg_v || p_v >= 0.0008f) loc_pos_sum += p_v;
    }
    s_red[tid] = loc_pos_sum;
    __syncthreads();
    for (int st = 128; st > 0; st >>= 1) {
        if (tid < st) s_red[tid] += s_red[tid + st];
        __syncthreads();
    }
    float pos_sum = s_red[0];
    float neg_scale = (pos_sum > 1e-8f) ? (err_f / pos_sum) : 0.0f;

    float tscale = 1.0f / float(1 << d_shift);
    float* gv_row = &sc.rep_gv[size_t(r) * VOCAB];
    for (int m = 0; m < 16; ++m) {
        int v = tid + m * 256;
        float gv = 0.0f;
        if (loc_sc[m] > -1900000000) {
            if (v == target_v) {
                gv = -err_f;
            } else {
                float p_v = loc_ex[m] * inv_z;
                if (v == best_neg_v || p_v >= 0.0008f) {
                    gv = (pos_sum > 1e-8f) ? (p_v * neg_scale) : (v == best_neg_v ? err_f : 0.0f);
                }
            }
        }
        gv_row[v] = gv;
        if (fabsf(gv) > 1e-7f) {
            float g_sc = gv * tscale;
            for (int c = 0; c < D; ++c) {
                int32_t d = int32_t(s_x[c]) - int32_t(qm.p_out_T[size_t(c) * VOCAB + v]);
                float l1s = (d > 0) ? -1.0f : (d < 0 ? 1.0f : 0.0f);
                atomicAdd(&s_dx[c], g_sc * (qm.w_eff_T[size_t(c) * VOCAB + v] + l1s));
            }
        }
    }
    __syncthreads();

    if (tid < D) {
        int32_t xv = int32_t(s_x[tid]);
        float norm_reg = (xv > 2600) ? (float(xv - 2600) * 0.0015f)
                       : ((xv < -2600) ? (float(xv + 2600) * 0.0015f) : 0.0f);
        sc.d_x[(size_t(b) * T_stride + t) * D + tid] = s_dx[tid] + norm_reg;
    }
}

// ============================================================================
// Kernel 3: Zero-Atomic Readout Parameter Gradients (`VOCAB` blocks x `D` threads)
// ============================================================================
__global__ void readout_weight_grad_exec_kernel(
    int N_rep, int D, int d_shift,
    const int16_t* p_out_T,
    TrunkScratchDev sc,
    GradDev gr
) {
    int v = blockIdx.x;
    int c = threadIdx.x;
    if (v >= VOCAB || c >= D) return;
    if (v == 0 || v == 2) return;

    int32_t pv = int32_t(p_out_T[size_t(c) * VOCAB + v]);
    float acc_w = 0.0f;
    float acc_p = 0.0f;
    float acc_b = 0.0f;
    float w_norm = 1.0f / 512.0f;

    for (int r = 0; r < N_rep; ++r) {
        float gv = sc.rep_gv[size_t(r) * VOCAB + v];
        if (gv == 0.0f) continue;
        acc_b += gv;
        int32_t xc = int32_t(sc.rep_x_top[size_t(r) * D + c]);
        acc_w += gv * float(xc) * w_norm;
        int32_t diff = xc - pv;
        float sg = (diff > 0) ? 1.0f : (diff < 0 ? -1.0f : 0.0f);
        acc_p += gv * sg;
    }

    if (c == 0 && acc_b != 0.0f) gr.b_out[v] += acc_b;
    if (acc_w != 0.0f) gr.w_out[size_t(v) * D + c] += acc_w;
    if (acc_p != 0.0f) gr.p_out[size_t(v) * D + c] += acc_p;
}

// ============================================================================
// Kernel 4: Analytic Multi-Layer Backward Trunk (`bn` blocks x `256` threads)
// ============================================================================
__global__ void bwd_trunk_kernel(
    const SequenceSample* samples,
    const int* batch_indices,
    int T_stride,
    QuantDev qm,
    GradDev gr,
    TrunkScratchDev sc
) {
    int b = blockIdx.x;
    int tid = threadIdx.x;
    const SequenceSample& seq = samples[batch_indices[b]];
    const int L = qm.cfg.L;
    const int D = qm.cfg.D;
    const int H = qm.cfg.H;
    const int G = qm.cfg.G;
    const int attn_res_shift = qm.cfg.attn_res_shift;
    const int ffn_shift = qm.cfg.ffn_shift;
    const int plen = int(seq.plen);
    const int rlen = int(seq.rlen);
    const int T = plen + rlen;

    auto idx_ld = [&](int l, int t) -> size_t {
        return (((size_t(b) * L + l) * T_stride) + t) * D;
    };
    auto idx_lg = [&](int l, int t) -> size_t {
        return (((size_t(b) * L + l) * T_stride) + t) * G;
    };
    auto idx_td = [&](int t) -> size_t {
        return (size_t(b) * T_stride + t) * D;
    };

    __shared__ uint32_t s_w4[4][MAX_SEQ];   // 16,384 B
    __shared__ float    s_g4[4][MAX_SEQ];   // 16,384 B
    __shared__ uint16_t s_drel4[4][MAX_SEQ]; //  8,192 B
    __shared__ uint64_t s_sum4[4][64];      //  2,048 B
    __shared__ float    s_f4[4][64];        //  1,024 B
    __shared__ float    s_f4b[4][64];       //  1,024 B
    __shared__ float    s_u1[256];          //  1,024 B
    __shared__ float    s_qsum4[4];         //     16 B
    __shared__ int      s_smin4[4];         //     16 B

    for (int l = L - 1; l >= 0; --l) {
        for (int t = 0; t < T; ++t) {
            if (tid < D) {
                size_t td = idx_td(t) + tid;
                sc.d_q[td] = 0.0f; sc.d_k[td] = 0.0f; sc.d_v[td] = 0.0f;
                sc.d_x_next[td] = sc.d_x[td];
            }
        }
        __syncthreads();

        const size_t fbase = size_t(l) * G * 256 * D;
        const size_t qkbase = size_t(l) * G * 256 * QKV_OUT;

        // 4a. FFN backward at active steps
        for (int t = 0; t < T; ++t) {
            float loc_abs = (tid < D) ? fabsf(sc.d_x[idx_td(t) + tid]) : 0.0f;
            s_u1[tid] = loc_abs;
            __syncthreads();
            for (int st = 128; st > 0; st >>= 1) {
                if (tid < st && s_u1[tid + st] > s_u1[tid]) s_u1[tid] = s_u1[tid + st];
                __syncthreads();
            }
            if (s_u1[0] < 1e-9f) continue;

            if (tid < D) {
                size_t b_ld = idx_ld(l, t);
                size_t b_lg = idx_lg(l, t);
                float d_act = sc.d_x[idx_td(t) + tid] * (sc.ffn_hinge[b_ld + tid] ? 0.25f : 1.0f);
                float df = d_act / float(1 << ffn_shift);
                for (int g = 0; g < G; ++g) {
                    atomicAdd(&gr.ffn_lut[fbase + (size_t(g) * 256 + sc.ffn_code[b_lg + g]) * D + tid], df);
                }
            }
            __syncthreads();
        }

        // 4b. Causal Attention + Pointer-Copy + RoPE Backward (4 heads in parallel)
        int h_loc = tid >> 6;
        int h_tid = tid & 63;
        float scale_attn = 1.0f / float(1 << attn_res_shift);

        for (int t = 0; t < T; ++t) {
            bool is_reply_step = (l == L - 1 && t >= plen - 1 && t < T - 1);
            float loc_abs = (tid < D) ? fabsf(sc.d_x[idx_td(t) + tid]) : 0.0f;
            s_u1[tid] = loc_abs;
            __syncthreads();
            for (int st = 128; st > 0; st >>= 1) {
                if (tid < st && s_u1[tid + st] > s_u1[tid]) s_u1[tid] = s_u1[tid + st];
                __syncthreads();
            }
            if (s_u1[0] < 1e-9f && !is_reply_step) continue;

            int target_v = is_reply_step ? int(seq.tokens[t + 1]) : -1;
            int pred_v = is_reply_step ? int(sc.step_pred_v[b * T_stride + t]) : -1;
            uint64_t copy_act_mask = is_reply_step ? sc.step_copy_act[b * T_stride + t] : 0ULL;
            bool is_copy = is_reply_step && (seq.copy_flags[t - (plen - 1)] != 0);
            bool is_copy_start = is_copy && (t == plen - 1 || !seq.copy_flags[t - plen]);
            float prob_t = is_reply_step ? sc.step_prob_t[b * T_stride + t] : 0.0f;
            float prob_p = is_reply_step ? sc.step_prob_p[b * T_stride + t] : 0.0f;

            int copy_s_lim = (plen > 1) ? (t < (plen - 1) ? t : (plen - 1)) : 0;
            for (int h0 = 0; h0 < H; h0 += 4) {
                int h = h0 + h_loc;
                int v_off = h * 8;
                int qk_off = head_qk_off(h);
                int sh = h & 3;
                int32_t tau = int32_t(qm.head_tau[l * H + h]); if (tau < 256) tau = 256;
                int32_t slope = int32_t(qm.head_slope[l * H + h]);
                uint16_t q_mask16 = 0; int32_t q16[16];
                size_t qt_off = idx_ld(l, t);
                for (int j = 0; j < 16; ++j) {
                    int32_t qv = int32_t(sc.q_rot[qt_off + ((qk_off + j) % D)]);
                    q16[j] = qv; if (qv >= 0) q_mask16 |= (1u << j);
                }

                uint64_t loc_min = UINT64_MAX;
                for (int s = h_tid; s <= t; s += 64) {
                    size_t ks_off = idx_ld(l, s);
                    int64_t dot16 = 0; int32_t d_l1 = 0; uint16_t k_mask16 = 0;
                    for (int j = 0; j < 16; ++j) {
                        int32_t kv = int32_t(sc.k_rot[ks_off + ((qk_off + j) % D)]);
                        dot16 += int64_t(q16[j]) * int64_t(kv);
                        int32_t diff = q16[j] - kv; d_l1 += (diff < 0 ? -diff : diff);
                        if (kv >= 0) k_mask16 |= (1u << j);
                    }
                    int32_t ds = int32_t(dot16 >> 10); ds = ds < -4096 ? -4096 : (ds > 4096 ? 4096 : ds);
                    int32_t d_ind = calc_d_ind(sh, s, t, int(seq.plen), seq.tokens, qm.copy_tok_gate, qm.tok_len);
                    int32_t d_tot = (4096 - ds) + (d_l1 >> 4) + (__popc(uint32_t(q_mask16 ^ k_mask16)) << 5) + (((t - s) * slope) >> 3) + d_ind;
                    if (d_tot < 0) d_tot = 0;
                    s_w4[h_loc][s] = uint32_t(d_tot);
                    uint64_t packed = (uint64_t(uint32_t(d_tot)) << 32) | uint32_t(s);
                    if (packed < loc_min) loc_min = packed;
                }
                s_sum4[h_loc][h_tid] = loc_min;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st && s_sum4[h_loc][h_tid + st] < s_sum4[h_loc][h_tid]) {
                        s_sum4[h_loc][h_tid] = s_sum4[h_loc][h_tid + st];
                    }
                    __syncthreads();
                }
                int32_t min_d = int32_t(uint32_t(s_sum4[h_loc][0] >> 32));
                if (h_tid == 0) s_smin4[h_loc] = int(uint32_t(s_sum4[h_loc][0] & 0xFFFFFFFFu));
                __syncthreads();
                bool sharp_rel = (sh >= 2);
                uint64_t loc_sum = 0;
                for (int s = h_tid; s <= t; s += 64) {
                    int32_t d_eff = sharp_rel ? ((int32_t(s_w4[h_loc][s]) - min_d) << 2) : int32_t(s_w4[h_loc][s]);
                    s_drel4[h_loc][s] = uint16_t(d_eff > 65535 ? 65535 : d_eff);
                    uint32_t w = prll_weight_order4_v10(tau, d_eff);
                    s_w4[h_loc][s] = w;
                    loc_sum += w;
                }
                s_sum4[h_loc][h_tid] = loc_sum;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st) s_sum4[h_loc][h_tid] += s_sum4[h_loc][h_tid + st];
                    __syncthreads();
                }
                uint64_t sr = s_sum4[h_loc][0] > 0 ? s_sum4[h_loc][0] : 1ULL;

                float d_attn_h[8];
                size_t dxt_off = idx_td(t);
                for (int j = 0; j < 8; ++j) d_attn_h[j] = sc.d_x[dxt_off + v_off + j] * scale_attn;

                bool eligible = (sh >= 2);
                float loc_wg = 0.0f, loc_ct = 0.0f, loc_cp = 0.0f, loc_qsum = 0.0f;
                int cur_tok_t = int(seq.tokens[t]);
                for (int s = h_tid; s <= t; s += 64) {
                    float wp_norm = float(s_w4[h_loc][s]) / float(sr);
                    float gs = 0.0f;
                    size_t vs_off = idx_ld(l, s);
                    size_t dvs_off = idx_td(s);
                    for (int j = 0; j < 8; ++j) {
                        gs += d_attn_h[j] * float(sc.v_vec[vs_off + v_off + j]);
                        sc.d_v[dvs_off + v_off + j] += wp_norm * d_attn_h[j]; // distinct channels per head `h0..h0+3`!
                    }
                    if (is_reply_step && s < copy_s_lim) {
                        bool s1_ok = ((seq.prompt_nocopy[(s + 1) >> 6] >> ((s + 1) & 63)) & 1ULL) == 0ULL;
                        int tok_n = int(seq.tokens[s + 1]);
                        if (s1_ok && is_copy && tok_n == target_v) {
                            if (eligible) {
                                gs -= (1.0f - prob_t) * 8.0f;
                                loc_ct += wp_norm;
                                if (sh == 3) {
                                    loc_qsum += 2.0f;
                                }
                            }
                        } else if (s1_ok && tok_n >= 3 && qm.copy_tok_gate[tok_n] > 0 && tok_n != target_v) {
                            if (tok_n == pred_v) {
                                gs += prob_p * (is_copy ? 4.0f : 2.0f);
                                loc_cp += wp_norm;
                            } else if (!is_copy) {
                                loc_cp += 0.30f * wp_norm;
                            }
                        }
                    }
                    s_w4[h_loc][s] = __float_as_uint(wp_norm);
                    s_g4[h_loc][s] = gs;
                    loc_wg += wp_norm * gs;
                }
                s_f4[h_loc][h_tid] = loc_wg;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st) s_f4[h_loc][h_tid] += s_f4[h_loc][h_tid + st];
                    __syncthreads();
                }
                float mean_g = s_f4[h_loc][0];

                if (is_reply_step) {
                    s_f4[h_loc][h_tid] = loc_ct; s_f4b[h_loc][h_tid] = loc_cp;
                    __syncthreads();
                    for (int st = 32; st > 0; st >>= 1) {
                        if (h_tid < st) {
                            s_f4[h_loc][h_tid] += s_f4[h_loc][h_tid + st];
                            s_f4b[h_loc][h_tid] += s_f4b[h_loc][h_tid + st];
                        }
                        __syncthreads();
                    }
                    if (h_tid == 0) {
                        float ct = s_f4[h_loc][0], cp = s_f4b[h_loc][0];
                        bool h_was_open = (h < 32) && ((copy_act_mask & (1ULL << h)) != 0ULL);
                        bool h_above_margin = (h < 32) && ((copy_act_mask & (1ULL << (32 + h))) != 0ULL);
                        float d_gate = 0.0f;
                        if (!eligible) {
                            d_gate = h_was_open ? 2.0f : (h_above_margin ? 0.50f : 0.0f);
                        } else if (is_copy_start) {
                            if (!h_was_open) {
                                d_gate = -3.2f * fmaxf(0.50f, 1.2f * (1.0f - prob_t)) * (0.80f + 3.0f * ct);
                            } else {
                                d_gate = (prob_t < 0.90f) ? (-1.5f * (1.0f - prob_t) * (0.60f + 3.0f * ct)) : -0.04f;
                            }
                        } else if (is_copy) {
                            if (!h_was_open) {
                                d_gate = -2.4f * fmaxf(0.45f, 1.1f * (1.0f - prob_t)) * (0.70f + 3.0f * ct);
                            } else {
                                d_gate = (prob_t < 0.90f)
                                             ? (-1.1f * (1.0f - prob_t) * (0.50f + 2.5f * ct) + prob_p * cp * 0.25f)
                                             : (prob_p * cp * 0.15f - 0.02f);
                            }
                        } else {
                            if (h_was_open) {
                                d_gate = (pred_v != target_v)
                                             ? (0.85f + (2.5f * prob_p + 0.8f * (1.0f - prob_t)) * cp)
                                             : (0.12f + 0.60f * (1.0f - prob_t) * cp);
                            } else {
                                d_gate = 0.0f;
                            }
                        }
                        s_f4b[h_loc][0] = d_gate;
                        if (fabsf(d_gate) > 1e-5f) {
                            float b_mult = eligible ? 1.2f : 4.0f;
                            atomicAdd(&gr.head_copy_b[h], d_gate * b_mult);
                        }
                    }
                    __syncthreads();
                    float d_gate_b = s_f4b[h_loc][0];
                    if (fabsf(d_gate_b) > 1e-5f) {
                        int r_idx = sc.step_rep_idx[b * T_stride + t];
                        const int16_t* gate_x = (r_idx >= 0) ? &sc.rep_x_top[size_t(r_idx) * D] : &sc.x_mid[idx_ld(l, t)];
                        bool cs_hint = (eligible && is_copy_start && !((copy_act_mask & (1ULL << h)) != 0ULL));
                        size_t dxt_gate = idx_td(t);
                        for (int c = h_tid; c < D; c += 64) {
                            atomicAdd(&gr.head_copy_w[h * D + c], d_gate_b * (float(gate_x[c]) / 1024.0f));
                            if (cs_hint) {
                                uint8_t code = qm.head_copy_gate[h * G + (c >> 2)];
                                int sub_k = c & 3;
                                int div = (sub_k == 0) ? 1 : (sub_k == 1 ? 3 : (sub_k == 2 ? 9 : 27));
                                int trit = ((int(code) / div) % 3) - 1;
                                if (trit != 0) {
                                    atomicAdd(&sc.d_x_next[dxt_gate + c], d_gate_b * float(trit) * 0.002f);
                                }
                            }
                        }
                    }
                    s_f4[h_loc][h_tid] = (is_copy && sh == 3) ? loc_qsum : 0.0f;
                    __syncthreads();
                    for (int st = 32; st > 0; st >>= 1) {
                        if (h_tid < st) s_f4[h_loc][h_tid] += s_f4[h_loc][h_tid + st];
                        __syncthreads();
                    }
                    if (h_tid == 0) s_qsum4[h_loc] = s_f4[h_loc][0];
                    __syncthreads();
                }

                float qsum = (is_reply_step && is_copy && sh == 3) ? s_qsum4[h_loc] : 0.0f;
                bool kl_active = (qsum > 0.0f);
                float kl_w = kl_active ? ((is_copy_start ? 32.0f : 16.0f) * fmaxf(0.40f, 1.0f - prob_t)) : 0.0f;

                float loc_gd = 0.0f;
                for (int s = h_tid; s <= t; s += 64) {
                    float wp_norm = __uint_as_float(s_w4[h_loc][s]);
                    float q_val = 0.0f;
                    bool s1_ok = (s < copy_s_lim) && (((seq.prompt_nocopy[(s + 1) >> 6] >> ((s + 1) & 63)) & 1ULL) == 0ULL);
                    if (kl_active && s1_ok && int(seq.tokens[s + 1]) == target_v) {
                        q_val = 2.0f;
                    }
                    float d_log_w = wp_norm * (s_g4[h_loc][s] - mean_g);
                    if (kl_active) {
                        d_log_w += kl_w * (wp_norm - (q_val / qsum));
                    }
                    float g_d = d_log_w * ((sharp_rel ? -16.0f : -4.0f) / float(tau + int32_t(s_drel4[h_loc][s])));
                    s_g4[h_loc][s] = g_d;
                    loc_gd += g_d;
                }
                s_f4[h_loc][h_tid] = loc_gd;
                __syncthreads();
                for (int st = 32; st > 0; st >>= 1) {
                    if (h_tid < st) s_f4[h_loc][h_tid] += s_f4[h_loc][h_tid + st];
                    __syncthreads();
                }
                if (h_tid == 0 && sharp_rel) {
                    s_g4[h_loc][s_smin4[h_loc]] -= s_f4[h_loc][0];
                }
                __syncthreads();

                float dq_loc[16] = {0};
                for (int s = h_tid; s <= t; s += 64) {
                    float d_d_tot = s_g4[h_loc][s];
                    if (fabsf(d_d_tot) < 1e-8f) continue;
                    size_t ks_off = idx_ld(l, s);
                    int64_t dot16 = 0;
                    for (int j = 0; j < 16; ++j) {
                        dot16 += int64_t(q16[j]) * int64_t(sc.k_rot[ks_off + ((qk_off + j) % D)]);
                    }
                    int32_t ds = int32_t(dot16 >> 10);
                    float d_dot = (ds >= 4096 || ds <= -4096) ? 0.0f : (d_d_tot * (-1.0f / 1024.0f));
                    float d_l1_coeff = d_d_tot * (1.0f / 16.0f);
                    size_t dks_off = idx_td(s);
                    for (int j = 0; j < 16; ++j) {
                        int ch = (qk_off + j) % D;
                        float qf = float(q16[j]), kf = float(sc.k_rot[ks_off + ch]);
                        float l1s = (qf > kf) ? 1.0f : (qf < kf ? -1.0f : 0.0f);
                        dq_loc[j] += d_dot * kf + d_l1_coeff * l1s;
                        atomicAdd(&sc.d_k[dks_off + ch], d_dot * qf - d_l1_coeff * l1s);
                    }
                }
                for (int j = 0; j < 16; ++j) {
                    if (fabsf(dq_loc[j]) > 1e-7f) atomicAdd(&sc.d_q[dxt_off + ((qk_off + j) % D)], dq_loc[j]);
                }
                __syncthreads();
            }
        }

        // 4c. Inverse RoPE (only rotated positional half-blocks) + Q/K/V LUT grads + residual
        for (int t = 0; t < T; ++t) {
            size_t td = idx_td(t);
            if (tid < D / 2 && ((tid >> 3) & 1) == 0) {
                int p_idx = (t < MAX_SEQ) ? t : (MAX_SEQ - 1);
                int freq = tid & 7;
                float c = float(d_rope_cos[p_idx][freq]) * (1.0f / 4096.0f);
                float s = float(d_rope_sin[p_idx][freq]) * (1.0f / 4096.0f);
                float gq0 = sc.d_q[td + 2 * tid], gq1 = sc.d_q[td + 2 * tid + 1];
                float gk0 = sc.d_k[td + 2 * tid], gk1 = sc.d_k[td + 2 * tid + 1];
                sc.d_q[td + 2 * tid]     = gq0 * c + gq1 * s;
                sc.d_q[td + 2 * tid + 1] = -gq0 * s + gq1 * c;
                sc.d_k[td + 2 * tid]     = gk0 * c + gk1 * s;
                sc.d_k[td + 2 * tid + 1] = -gk0 * s + gk1 * c;
            }
            __syncthreads();
            float* s_dq_f = s_g4[0];
            float* s_dk_f = s_g4[1];
            if (tid < D) {
                float dq_raw = sc.d_q[td + tid];
                float dk_raw = sc.d_k[td + tid];
                float dv_raw = sc.d_v[td + tid];
                s_dq_f[tid] = dq_raw;
                s_dk_f[tid] = dk_raw;
                float dq = dq_raw * 0.25f;
                float dk = dk_raw * 0.25f;
                float dv = dv_raw * 0.25f;
                if (fabsf(dq) > 1e-9f || fabsf(dk) > 1e-9f || fabsf(dv) > 1e-9f) {
                    size_t b_lg = idx_lg(l, t);
                    for (int k = 0; k < 4; ++k) {
                        int o = ((tid & 3) + k * 4);
                        int g = ((tid + D - o) % D) >> 2;
                        uint8_t qc = sc.q_code[b_lg + g], kc = sc.k_code[b_lg + g];
                        if (fabsf(dq) > 1e-9f) atomicAdd(&gr.q_lut[qkbase + (size_t(g) * 256 + qc) * QKV_OUT + o], dq);
                        if (fabsf(dk) > 1e-9f) atomicAdd(&gr.k_lut[qkbase + (size_t(g) * 256 + kc) * QKV_OUT + o], dk);
                        if (fabsf(dv) > 1e-9f) atomicAdd(&gr.v_lut[qkbase + (size_t(g) * 256 + qc) * QKV_OUT + o], dv);
                    }
                }
                if (fabsf(dv_raw) > 1e-9f) sc.d_x_next[td + tid] += dv_raw * 0.5f;
            }
            fwht64_t_float_shared(s_dq_f, tid, D);
            fwht64_t_float_shared(s_dk_f, tid, D);
            if (tid < D) {
                float dq_x = s_dq_f[tid];
                float dk_x = s_dk_f[tid];
                if (fabsf(dq_x) > 1e-9f) sc.d_x_next[td + tid] += dq_x * 0.5f;
                if (fabsf(dk_x) > 1e-9f) {
                    sc.d_x_next[td + tid] += dk_x * 0.375f;
                    if (t > 0) atomicAdd(&sc.d_x_next[idx_td(t - 1) + (tid ^ 1)], dk_x * 0.125f);
                }
            }
            __syncthreads();
        }

        for (int t = 0; t < T; ++t) {
            if (tid < D) sc.d_x[idx_td(t) + tid] = sc.d_x_next[idx_td(t) + tid];
        }
        __syncthreads();
    }

    for (int t = 0; t < T; ++t) {
        if (tid < D) {
            float dx = sc.d_x[idx_td(t) + tid];
            if (fabsf(dx) > 1e-9f) atomicAdd(&gr.e_tok[size_t(seq.tokens[t]) * D + tid], dx);
        }
    }
}

// ============================================================================
// Pure Integer Host Inference Simulator + Binary Exporter (`quant_bits` 4|8)
// ============================================================================
struct HostQuantModel {
    ModelCfg cfg;
    std::vector<uint16_t> scales_q8;
    std::vector<int8_t> q_e_tok, q_e_pos, q_p_out;
    std::vector<int16_t> e_tok, p_out, b_out;
    std::vector<uint64_t> p_sk;
    std::vector<uint8_t> tmac_hi, tmac_lo, copy_tok_gate;
    struct Layer {
        std::vector<uint16_t> head_tau, head_slope;
        std::vector<uint8_t> head_shift, head_copy_gate;
        std::vector<int16_t> head_copy_b;
        std::vector<int8_t> q_q, q_k, q_v, q_ffn;
        std::vector<int16_t> q_lut, k_lut, v_lut, ffn_lut;
    };
    std::vector<Layer> layers;

    std::string decode_tokens(const std::vector<uint16_t>& ids, const std::vector<std::string>& vocab) const {
        std::string s;
        for (uint16_t tid : ids) if (tid < vocab.size()) s += vocab[tid];
        size_t end_pos = s.find("<|end|>");
        if (end_pos != std::string::npos) s.resize(end_pos);
        size_t usr_pos = s.find("<|user|>");
        if (usr_pos != std::string::npos) s.resize(usr_pos);
        return s;
    }

    std::vector<uint16_t> greedy_generate(const std::vector<uint16_t>& prompt_ids, const std::vector<std::string>& vocab, int max_new) const {
        const int L = cfg.L, D = cfg.D, H = cfg.H, G = cfg.G;
        int plen = std::min<int>(int(prompt_ids.size()), 768);
        std::vector<uint16_t> tokens;
        tokens.reserve(plen + max_new);
        for (int i = 0; i < plen; ++i) tokens.push_back(prompt_ids[i]);
        std::vector<uint8_t> tok_len(VOCAB, 1);
        for (size_t v = 0; v < vocab.size() && v < VOCAB; ++v) {
            tok_len[v] = uint8_t(std::min<size_t>(255, vocab[v].size()));
        }

        std::vector<std::vector<int16_t>> prev_x(L, std::vector<int16_t>(D, 0));
        std::vector<std::vector<std::vector<int16_t>>> k_cache(L), v_cache(L);
        std::vector<int16_t> last_hidden(D, 0);
        std::vector<std::vector<uint32_t>> top_wp(H);
        std::vector<uint16_t> out_toks;
        std::string so_far;

        auto push_tok = [&](uint16_t tok, int t) {
            std::vector<int16_t> x_cur(e_tok.begin() + tok * D, e_tok.begin() + (tok + 1) * D);
            std::vector<int16_t> u_blk(D), k_in(D), u_k(D), q_vec(D), k_vec(D), v_vec(D), attn_out(D), x_mid(D), u_ffn(D);
            for (int l = 0; l < L; ++l) {
                for (int c = 0; c < D; ++c) {
                    int32_t px = (t > 0) ? int32_t(prev_x[l][c ^ 1]) : 0;
                    k_in[c] = clamp_q12(((int32_t(x_cur[c]) * 3) + px) >> 2);
                }
                prev_x[l] = x_cur;
                for (int b = 0; b < D / 64; ++b) {
                    fwht64_host(&x_cur[b * 64], &u_blk[b * 64]);
                    fwht64_host(&k_in[b * 64], &u_k[b * 64]);
                }
                std::vector<int32_t> qa(D, 0), ka(D, 0), va(D, 0);
                for (int g = 0; g < G; ++g) {
                    uint8_t qc = code_4ch(&u_blk[g * 4]);
                    uint8_t kc = code_4ch(&u_k[g * 4]);
                    int base_c = (g * 4) % D;
                    size_t qr = (size_t(g) * 256 + qc) * QKV_OUT;
                    size_t kr = (size_t(g) * 256 + kc) * QKV_OUT;
                    for (int o = 0; o < QKV_OUT; ++o) {
                        int dst = (base_c + o) % D;
                        qa[dst] += layers[l].q_lut[qr + o];
                        ka[dst] += layers[l].k_lut[kr + o];
                        va[dst] += layers[l].v_lut[qr + o];
                    }
                }
                for (int c = 0; c < D; ++c) {
                    q_vec[c] = clamp_q12((qa[c] >> 2) + (int32_t(u_blk[c]) >> 1));
                    k_vec[c] = clamp_q12((ka[c] >> 2) + (int32_t(u_k[c]) >> 1));
                    v_vec[c] = clamp_q12((va[c] >> 2) + (int32_t(x_cur[c]) >> 1));
                }
                apply_rope_host(q_vec.data(), D, t);
                apply_rope_host(k_vec.data(), D, t);
                k_cache[l].push_back(k_vec);
                v_cache[l].push_back(v_vec);

                for (int h = 0; h < H; ++h) {
                    int v_off = h * 8;
                    int qk_off = head_qk_off(h);
                    int sh = h & 3;
                    int32_t tau = std::max<int32_t>(256, layers[l].head_tau[h]);
                    int32_t slope = layers[l].head_slope[h];
                    int32_t q16[16]; uint16_t qm16 = 0;
                    for (int j = 0; j < 16; ++j) {
                        q16[j] = q_vec[(qk_off + j) % D];
                        if (q16[j] >= 0) qm16 |= (1u << j);
                    }
                    std::vector<uint32_t> rw(t + 1);
                    uint64_t sr = 0; int32_t min_d = INT32_MAX; int bs = 0;
                    for (int s = 0; s <= t; ++s) {
                        int64_t dot16 = 0; int32_t dl1 = 0; uint16_t km16 = 0;
                        for (int j = 0; j < 16; ++j) {
                            int32_t kv = k_cache[l][s][(qk_off + j) % D];
                            dot16 += int64_t(q16[j]) * int64_t(kv);
                            dl1 += std::abs(q16[j] - kv);
                            if (kv >= 0) km16 |= (1u << j);
                        }
                        int32_t ds = std::max(-4096, std::min(4096, int32_t(dot16 >> 10)));
                        int32_t ham = __builtin_popcount(uint32_t(qm16 ^ km16));
                        int32_t d_ind = calc_d_ind(sh, s, t, plen, tokens.data(), copy_tok_gate.data(), tok_len.data());
                        int32_t dtot = std::max<int32_t>(0, (4096 - ds) + (dl1 >> 4) + (ham << 5) + (((t - s) * slope) >> 3) + d_ind);
                        rw[s] = uint32_t(dtot);
                        if (dtot < min_d) { min_d = dtot; bs = s; }
                    }
                    bool sharp_rel = (sh >= 2);
                    for (int s = 0; s <= t; ++s) {
                        int32_t d_eff = sharp_rel ? ((int32_t(rw[s]) - min_d) << 2) : int32_t(rw[s]);
                        uint32_t w = prll_weight_order4_v10(tau, d_eff);
                        rw[s] = w; sr += w;
                    }
                    std::vector<uint32_t> wp(t + 1);
                    uint64_t ma = 0;
                    for (int s = 0; s <= t; ++s) {
                        wp[s] = uint32_t((uint64_t(rw[s]) * PRLL_QUANTUM) / std::max<uint64_t>(1, sr));
                        ma += wp[s];
                    }
                    wp[bs] += uint32_t(PRLL_QUANTUM - ma);
                    if (l == L - 1) top_wp[h] = wp;
                    for (int j = 0; j < 8; ++j) {
                        int64_t av = 0;
                        for (int s = 0; s <= t; ++s) av += int64_t(wp[s]) * int64_t(v_cache[l][s][v_off + j]);
                        attn_out[v_off + j] = clamp_q12(int32_t(av / int64_t(PRLL_QUANTUM)));
                    }
                }
                for (int c = 0; c < D; ++c) x_mid[c] = clamp_q12(int32_t(x_cur[c]) + (int32_t(attn_out[c]) >> cfg.attn_res_shift));
                for (int b = 0; b < D / 64; ++b) fwht64_host(&x_mid[b * 64], &u_ffn[b * 64]);
                std::vector<int32_t> fa(D, 0);
                for (int g = 0; g < G; ++g) {
                    uint8_t fc = code_4ch(&u_ffn[g * 4]);
                    const int16_t* row = &layers[l].ffn_lut[(size_t(g) * 256 + fc) * D];
                    for (int c = 0; c < D; ++c) fa[c] += row[c];
                }
                for (int c = 0; c < D; ++c) x_cur[c] = clamp_q12(int32_t(x_mid[c]) + int32_t(icosian_hinge(fa[c] >> cfg.ffn_shift)));
            }
            last_hidden = x_cur;
        };

        for (int t = 0; t < plen; ++t) push_tok(tokens[t], t);

        int prev_copy_src = -1;
        auto is_ws_prefix = [](const std::string& s) {
            return !s.empty() && (s[0] == ' ' || s[0] == '\t' || s[0] == '\n' || s[0] == '\r');
        };
        auto all_alnum_or = [](const std::string& s, bool allow_slug) {
            for (unsigned char ch : s) {
                bool ok = (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') ||
                          (ch >= '0' && ch <= '9') || ch == '_' ||
                          (allow_slug && (ch == '-' || ch == '/'));
                if (!ok) return false;
            }
            return !s.empty();
        };
        std::vector<uint8_t> prompt_nocopy(plen, 0);
        {
            static const char* kNoCopyTags[] = {
                "<|user|>", "<|assistant|>", "<|end|>",
                "<tool_call>", "</tool_call>", "<tool_response>", "</tool_response>"
            };
            std::string joined;
            std::vector<std::pair<size_t, size_t>> spans(plen);
            for (int idx = 0; idx < plen; ++idx) {
                size_t b0 = joined.size();
                uint16_t tid = tokens[idx];
                if (tid < vocab.size()) joined += vocab[tid];
                spans[idx] = {b0, joined.size()};
            }
            for (const char* tag : kNoCopyTags) {
                std::string ts(tag);
                size_t pos = 0;
                while ((pos = joined.find(ts, pos)) != std::string::npos) {
                    size_t t0 = pos, t1 = pos + ts.size();
                    for (int idx = 0; idx < plen; ++idx) {
                        if (spans[idx].second > t0 && spans[idx].first < t1) prompt_nocopy[idx] = 1;
                    }
                    pos = t1;
                }
            }
        }
        int shift_bits = 20 + cfg.d_shift;

        for (int step = 0; step < max_new; ++step) {
            int t = int(tokens.size()) - 1;
            if (t + 1 >= MAX_SEQ) break;
            std::vector<std::vector<int32_t>> act(G, std::vector<int32_t>(81, 0));
            for (int g = 0; g < G; ++g) build_tmac_act_lut81(&last_hidden[g * 4], act[g].data());
            std::vector<int32_t> cl(VOCAB, 0), cl_cand(VOCAB, 0), cl_src(VOCAB, 0);
            const auto& tl = layers[L - 1];
            int copy_s_lim = (plen > 1) ? std::min(t, plen - 1) : 0;
            for (int h = 0; h < H; ++h) {
                int32_t gs = tl.head_copy_b[h];
                for (int g = 0; g < G; ++g) gs += act[g][tl.head_copy_gate[h * G + g]];
                int32_t gs_norm = (D >= 256) ? (gs >> 1) : gs;
                // Avoid step-0 header echo where <assistant>\n attends to <user>\n.
                if (step == 0 || gs_norm <= 0 || copy_s_lim <= 0) continue;
                int64_t cg = std::min<int32_t>(2048, std::max<int32_t>(0, gs_norm));
                std::vector<uint64_t> ha(VOCAB, 0);
                std::vector<uint32_t> bw(VOCAB, 0);
                std::vector<int> bs_tok(VOCAB, 0);
                for (int s = 0; s < copy_s_lim; ++s) {
                    if (prompt_nocopy[s + 1]) continue;
                    uint16_t tn = tokens[s + 1];
                    if (tn < 3 || copy_tok_gate[tn] == 0) continue;
                    uint32_t w = top_wp[h][s];
                    if (w == 0) continue;
                    ha[tn] += (uint64_t(w) * uint64_t(cg)) >> 6;
                    if (w > bw[tn]) { bw[tn] = w; bs_tok[tn] = s + 1; }
                }
                for (int v = 0; v < VOCAB; ++v) {
                    if (ha[v] > 0 && copy_tok_gate[v] > 0) {
                        int32_t cand = int32_t(std::min<uint64_t>(1536ULL, (ha[v] * uint64_t(copy_tok_gate[v])) >> shift_bits));
                        if (cand > 0) {
                            cl[v] = std::min<int32_t>(4608, cl[v] + cand);
                            if (cand > cl_cand[v]) { cl_cand[v] = cand; cl_src[v] = bs_tok[v]; }
                        }
                    }
                }
            }
            bool in_code_gen = false;
            for (size_t i = 0; i < so_far.size(); ++i) {
                if (so_far[i] != ' ' && so_far[i] != '\n' && so_far[i] != '\r') {
                    in_code_gen = (so_far.compare(i, 3, "```") == 0);
                    break;
                }
            }
            auto piece_continues_span = [&](int ns) -> bool {
                const std::string& ns_p = (tokens[ns] < vocab.size()) ? vocab[tokens[ns]] : std::string();
                if (ns_p.empty() || is_ws_prefix(ns_p)) return false;
                bool is_ident_or_slug = all_alnum_or(ns_p, !in_code_gen);
                bool is_dotted_host = !in_code_gen && ns_p == "." && ns + 1 < plen &&
                                      ns + 1 < int(tokens.size()) &&
                                      (tokens[ns + 1] < vocab.size()) && !vocab[tokens[ns + 1]].empty() &&
                                      ((vocab[tokens[ns + 1]][0] >= 'a' && vocab[tokens[ns + 1]][0] <= 'z') ||
                                       (vocab[tokens[ns + 1]][0] >= 'A' && vocab[tokens[ns + 1]][0] <= 'Z') ||
                                       (vocab[tokens[ns + 1]][0] >= '0' && vocab[tokens[ns + 1]][0] <= '9'));
                bool is_url_scheme = !in_code_gen &&
                                     (ns_p == "://" ||
                                      (ns_p == ":" && ns + 1 < plen && ns + 1 < int(tokens.size()) &&
                                       (tokens[ns + 1] < vocab.size()) && !vocab[tokens[ns + 1]].empty() &&
                                       vocab[tokens[ns + 1]][0] == '/'));
                return is_ident_or_slug || is_dotted_host || is_url_scheme;
            };
            int cont_src = -1;
            if (prev_copy_src >= 0 && prev_copy_src + 1 < plen && prev_copy_src + 1 <= t) {
                int ns = prev_copy_src + 1;
                if (!prompt_nocopy[ns] && piece_continues_span(ns)) cont_src = ns;
            }
            if (cont_src >= 0) {
                uint16_t v_ns = tokens[cont_src];
                if (v_ns >= 3 && v_ns < VOCAB) {
                    std::fill(cl.begin(), cl.end(), 0);
                    cl[v_ns] = 4608;
                    cl_src[v_ns] = cont_src;
                }
            } else if (in_code_gen && !out_toks.empty()) {
                size_t ls = 0;
                for (size_t i = 0; i < out_toks.size(); ++i) {
                    if (out_toks[i] < vocab.size() && vocab[out_toks[i]].find('\n') != std::string::npos) ls = i + 1;
                }
                for (int v = 3; v < VOCAB; ++v) {
                    if (cl[v] <= 0) continue;
                    bool v_ws = (v < int(vocab.size())) && is_ws_prefix(vocab[v]);
                    for (size_t k = ls; k < out_toks.size(); ++k) {
                        if (int(out_toks[k]) == v) {
                            bool prev_alnum = false;
                            if (k > ls && out_toks[k - 1] < vocab.size() && !vocab[out_toks[k - 1]].empty()) {
                                unsigned char lc = static_cast<unsigned char>(vocab[out_toks[k - 1]].back());
                                prev_alnum = (lc >= 'a' && lc <= 'z') || (lc >= 'A' && lc <= 'Z') || (lc >= '0' && lc <= '9') || lc == '_';
                            }
                            if (k == ls || v_ws || !prev_alnum) { cl[v] >>= 2; break; }
                        }
                    }
                }
            }

            std::vector<uint64_t> hsk(cfg.sk_blocks);
            for (int b = 0; b < cfg.sk_blocks; ++b) hsk[b] = pack_sign64(&last_hidden[b * 64]);
            bool block_span_exit_alnum = false;
            if (cont_src < 0 && prev_copy_src >= 0 && prev_copy_src < plen && tokens[prev_copy_src] < vocab.size()) {
                const std::string& sp_p = vocab[tokens[prev_copy_src]];
                if (!sp_p.empty()) {
                    unsigned char lc = static_cast<unsigned char>(sp_p.back());
                    block_span_exit_alnum = (lc >= 'a' && lc <= 'z') || (lc >= 'A' && lc <= 'Z') || (lc >= '0' && lc <= '9') || lc == '_';
                }
            }
            int best_v = 1; int32_t best_sc = INT32_MIN; int32_t best_copy = 0; int32_t best_pure = INT32_MIN;
            int32_t max_pure = INT32_MIN, second_pure = INT32_MIN; int max_pure_v = 1;
            std::vector<int32_t> pure_scores(VOCAB, INT32_MIN);
            for (int v = 0; v < VOCAB; ++v) {
                if (v == 0 || v == 2 || (v == 1 && out_toks.empty())) continue;
                if (block_span_exit_alnum && v >= 3 && v < int(vocab.size()) && !vocab[v].empty() && !is_ws_prefix(vocab[v])) {
                    unsigned char fc = static_cast<unsigned char>(vocab[v][0]);
                    if ((fc >= 'a' && fc <= 'z') || (fc >= 'A' && fc <= 'Z') || (fc >= '0' && fc <= '9') || fc == '_') continue;
                }
                int32_t tmac = 0;
                for (int g = 0; g < G; ++g) {
                    tmac += (act[g][tmac_hi[v * G + g]] << 1) + act[g][tmac_hi[v * G + g]] + act[g][tmac_lo[v * G + g]];
                }
                int32_t l1 = 0, ham = 0;
                for (int c = 0; c < D; ++c) l1 += std::abs(int32_t(last_hidden[c]) - int32_t(p_out[v * D + c]));
                for (int b = 0; b < cfg.sk_blocks; ++b) ham += __builtin_popcountll(hsk[b] ^ p_sk[v * cfg.sk_blocks + b]);
                int32_t ps = (tmac >> cfg.d_shift) + (int32_t(b_out[v]) * 2) - ((ham * 24) >> cfg.d_shift) - (l1 >> (5 + cfg.d_shift));
                pure_scores[v] = ps;
                if (ps > max_pure) { second_pure = max_pure; max_pure = ps; max_pure_v = v; }
                else if (ps > second_pure) { second_pure = ps; }
            }
            bool uncertain_vocab = (max_pure < -1024);
            int32_t copy_span_thresh = uncertain_vocab ? 256 : 512;
            int lift_shift = uncertain_vocab ? 8 : 10;
            for (int v = 0; v < VOCAB; ++v) {
                int32_t ps = pure_scores[v];
                if (ps == INT32_MIN) continue;
                int32_t c_lift = 0;
                if (cl[v] >= copy_span_thresh && ps < -1408) {
                    bool already_gen = false;
                    for (uint16_t gt : out_toks) { if (int(gt) == v) { already_gen = true; break; } }
                    if (!already_gen) {
                        int64_t def = int64_t(-1408 - ps);
                        int64_t scaled = (def * int64_t(cl[v])) >> lift_shift;
                        c_lift = int32_t(scaled < def ? scaled : def);
                    }
                }
                int32_t sc_tot = ps + cl[v] + c_lift;
                if (sc_tot > best_sc) { best_sc = sc_tot; best_v = v; best_copy = cl[v]; best_pure = ps; }
            }
            int best_src = cl_src[best_v];
            bool mid_vocab_ident = false;
            if (prev_copy_src < 0 && best_v < int(vocab.size()) && !is_ws_prefix(vocab[best_v]) && !out_toks.empty()) {
                uint16_t pt = out_toks.back();
                bool matches_prev_prompt = (best_src > 0 && best_src <= plen && tokens[best_src - 1] == pt);
                if (!matches_prev_prompt && pt < vocab.size() && !vocab[pt].empty()) {
                    unsigned char lc = static_cast<unsigned char>(vocab[pt].back());
                    mid_vocab_ident = (lc >= 'a' && lc <= 'z') || (lc >= 'A' && lc <= 'Z') || (lc >= '0' && lc <= '9') || lc == '_';
                }
            }
            bool has_ident_cont = (best_copy >= copy_span_thresh) && (best_src + 1 < plen) &&
                                  !prompt_nocopy[best_src + 1] && !mid_vocab_ident && piece_continues_span(best_src + 1);
            bool won_vocab = (cont_src < 0) && !has_ident_cont && (best_v == max_pure_v) && (best_pure > -1024) && (best_pure > second_pure + 512);
            prev_copy_src = (best_copy >= copy_span_thresh && !won_vocab) ? best_src : -1;
            if (best_v == 1) break;
            out_toks.push_back(uint16_t(best_v));
            if (best_v < int(vocab.size())) so_far += vocab[best_v];
            if (so_far.find("<|end|>") != std::string::npos || so_far.find("<|user|>") != std::string::npos) break;
            tokens.push_back(uint16_t(best_v));
            push_tok(uint16_t(best_v), int(tokens.size()) - 1);
        }
        return out_toks;
    }

    std::vector<uint8_t> serialize_container() const {
        std::vector<uint8_t> payload;
        auto push_u16 = [&](uint16_t v) { payload.push_back(v & 0xFF); payload.push_back(v >> 8); };
        auto push_i8v = [&](const std::vector<int8_t>& vec) {
            for (int8_t x : vec) payload.push_back(uint8_t(x));
        };
        auto push_u8v = [&](const std::vector<uint8_t>& vec) {
            payload.insert(payload.end(), vec.begin(), vec.end());
        };

        for (uint16_t sc : scales_q8) push_u16(sc);
        push_i8v(q_e_tok);
        push_i8v(q_e_pos);
        for (int l = 0; l < cfg.L; ++l) {
            for (uint16_t v : layers[l].head_tau) push_u16(v);
            for (uint16_t v : layers[l].head_slope) push_u16(v);
            push_u8v(layers[l].head_shift);
            for (int16_t v : layers[l].head_copy_b) push_u16(uint16_t(v));
            push_u8v(layers[l].head_copy_gate);
            push_i8v(layers[l].q_q);
            push_i8v(layers[l].q_k);
            push_i8v(layers[l].q_v);
            if (cfg.quant_bits == 4) {
                for (size_t i = 0; i < layers[l].q_ffn.size(); i += 2) {
                    uint8_t lo = uint8_t(layers[l].q_ffn[i]) & 0x0F;
                    uint8_t hi = (uint8_t(layers[l].q_ffn[i + 1]) & 0x0F) << 4;
                    payload.push_back(lo | hi);
                }
            } else {
                push_i8v(layers[l].q_ffn);
            }
        }
        push_u8v(tmac_hi);
        push_u8v(tmac_lo);
        push_u8v(copy_tok_gate);
        push_i8v(q_p_out);
        for (int16_t v : b_out) push_u16(uint16_t(v));

        uint8_t digest[32];
        sha256_compute(payload.data(), payload.size(), digest);

        std::vector<uint8_t> out(80 + payload.size());
        memcpy(out.data(), "UORGLLM9", 8);
        uint32_t hdr_words[10] = {
            11u, uint32_t(VOCAB), uint32_t(cfg.D), uint32_t(cfg.L),
            uint32_t(cfg.H), uint32_t(cfg.G), 0u, 0u, 0u, uint32_t(cfg.quant_bits)
        };
        for (int i = 0; i < 10; ++i) {
            uint32_t w = hdr_words[i];
            out[8 + i*4 + 0] = uint8_t(w & 0xFF); out[8 + i*4 + 1] = uint8_t((w >> 8) & 0xFF);
            out[8 + i*4 + 2] = uint8_t((w >> 16) & 0xFF); out[8 + i*4 + 3] = uint8_t((w >> 24) & 0xFF);
        }
        memcpy(out.data() + 48, digest, 32);
        memcpy(out.data() + 80, payload.data(), payload.size());
        return out;
    }
};

static uint16_t compute_scale_i8(const std::vector<float>& w) {
    float ma = 64.0f;
    for (float x : w) ma = std::max(ma, fabsf(std::max(-8192.0f, std::min(8192.0f, x))));
    int32_t sc = int32_t((ma * 16.0f + 126.0f) / 127.0f);
    return uint16_t(std::max(16, std::min(65535, sc)));
}

int main(int argc, char** argv) {
    if (argc < 7) {
        fprintf(stderr, "Usage: %s <train.bin> <eval.bin> <vocab.bin> <prompts.bin> <out.uorgllm> <receipt.json> [--layers L] [--d_model D] [--epochs E] [--quant_bits 4|8] [--batch B]\n", argv[0]);
        return 1;
    }
    const char* train_path = argv[1];
    const char* eval_path = argv[2];
    const char* vocab_path = argv[3];
    const char* prompts_path = argv[4];
    const char* out_model_path = argv[5];
    const char* out_receipt_path = argv[6];

    int L = 6, D = 128, epochs = 5, quant_bits = 4, batch_sz = 64, max_train = 0;
    for (int i = 7; i < argc; ++i) {
        if (!strcmp(argv[i], "--layers") && i + 1 < argc) L = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--d_model") && i + 1 < argc) D = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--epochs") && i + 1 < argc) epochs = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--quant_bits") && i + 1 < argc) quant_bits = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--batch") && i + 1 < argc) batch_sz = atoi(argv[++i]);
        else if (!strcmp(argv[i], "--max_train") && i + 1 < argc) max_train = atoi(argv[++i]);
    }
    ModelCfg cfg;
    cfg.L = L; cfg.D = D; cfg.H = D / 8; cfg.G = D / 4; cfg.quant_bits = quant_bits;
    cfg.attn_res_shift = (L >= 16) ? 2 : 1;
    cfg.ffn_shift = (D >= 256) ? 5 : 4;
    cfg.d_shift = (D >= 256) ? 2 : (D >= 128 ? 1 : 0);
    cfg.sk_blocks = D / 64;

    init_rope_tables();

    cudaDeviceProp prop{};
    CHECK_CUDA(cudaGetDeviceProperties(&prop, 0));
    printf("[gpu-v11] Device=%s sm=%d.%d L=%d D=%d H=%d G=%d qbits=%d epochs=%d batch=%d\n",
           prop.name, prop.major, prop.minor, cfg.L, cfg.D, cfg.H, cfg.G, cfg.quant_bits, epochs, batch_sz);
    fflush(stdout);

    std::vector<SequenceSample> train_samples, eval_samples;
    std::vector<EvalPromptItem> prompts;
    std::vector<std::string> vocab;
    if (!load_uorcorp3(train_path, train_samples)) return 2;
    if (max_train > 0 && int(train_samples.size()) > max_train) train_samples.resize(max_train);
    if (!load_uorcorp3(eval_path, eval_samples)) return 2;
    if (!load_uorvocb9(vocab_path, vocab)) return 2;
    mark_prompt_nocopy_samples(train_samples, vocab);
    mark_prompt_nocopy_samples(eval_samples, vocab);
    load_uorprmp1(prompts_path, prompts);
    printf("[gpu-v11] Loaded train=%zu eval=%zu prompts=%zu\n", train_samples.size(), eval_samples.size(), prompts.size());
    fflush(stdout);

    auto rng_state = uint64_t(20261114ULL);
    auto next_f = [&](float amp) -> float {
        rng_state ^= rng_state << 13; rng_state ^= rng_state >> 7; rng_state ^= rng_state << 17;
        int32_t r = int32_t(rng_state & 0xFFFF) - 32768;
        return float(r) / 32768.0f * amp;
    };

    const size_t qkv_n = size_t(L) * cfg.G * 256 * QKV_OUT;
    const size_t ffn_n = size_t(L) * cfg.G * 256 * D;
    DevBuf b_etok, b_qlut, b_klut, b_vlut, b_ffnlut, b_wout, b_pout, b_bout, b_copyb, b_copyw;
    b_etok.alloc(size_t(VOCAB) * D);
    b_qlut.alloc(qkv_n); b_klut.alloc(qkv_n); b_vlut.alloc(qkv_n); b_ffnlut.alloc(ffn_n);
    b_wout.alloc(size_t(VOCAB) * D); b_pout.alloc(size_t(VOCAB) * D); b_bout.alloc(VOCAB);
    b_copyb.alloc(cfg.H); b_copyw.alloc(size_t(cfg.H) * D);

    std::vector<float> h_etok(b_etok.len), h_q(qkv_n), h_k(qkv_n), h_v(qkv_n), h_f(ffn_n);
    std::vector<float> h_wo(b_wout.len), h_po(b_pout.len), h_bo(VOCAB, 0.0f);
    std::vector<float> h_cb(cfg.H, -256.0f), h_cw(b_copyw.len, 0.0f);
    std::vector<uint8_t> h_copy_tok_gate(VOCAB, 96);
    std::vector<uint8_t> h_tok_len(VOCAB, 1);

    // Byte-geometric token embeddings + corpus frequency & copy-eligibility calibration
    {
        std::vector<uint64_t> resp_freq(VOCAB, 0), copy_hits(VOCAB, 0), copy_start_hits(VOCAB, 0), noncopy_hits(VOCAB, 0);
        uint64_t total_resp_toks = 0;
        for (const auto& seq : train_samples) {
            for (int k = 0; k < int(seq.rlen); ++k) {
                int pos = int(seq.plen) + k;
                if (pos >= MAX_SEQ) break;
                uint16_t tok = seq.tokens[pos];
                if (tok >= VOCAB) continue;
                resp_freq[tok]++;
                total_resp_toks++;
                if (seq.copy_flags[k]) {
                    copy_hits[tok]++;
                    if (k == 0 || !seq.copy_flags[k - 1]) copy_start_hits[tok]++;
                } else {
                    noncopy_hits[tok]++;
                }
            }
        }
        double mean_freq = double(std::max<uint64_t>(1, total_resp_toks)) / double(VOCAB);

        auto is_ascii_word = [](const std::string& s) -> bool {
            if (s.empty()) return false;
            size_t st = (s[0] == ' ') ? 1 : 0;
            if (st >= s.size()) return false;
            bool has_alnum = false;
            for (size_t i = st; i < s.size(); ++i) {
                unsigned char ch = static_cast<unsigned char>(s[i]);
                if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9')) {
                    has_alnum = true;
                } else if (ch != '_' && ch != '-' && ch != '/') {
                    return false;
                }
            }
            return has_alnum;
        };

        for (int v = 0; v < VOCAB; ++v) {
            const std::string& tok_s = (v < int(vocab.size())) ? vocab[v] : std::string();
            h_tok_len[v] = uint8_t(std::min<size_t>(255, tok_s.size()));
            uint64_t seed = 0x9e3779b97f4a7c15ULL ^ (uint64_t(v + 1) * 0xbf58476d1ce4e5b9ULL);
            double sum_sq = 0.0;
            std::vector<float> vec(D, 0.0f);
            bool is_space = (!tok_s.empty() && tok_s[0] == ' ');
            bool is_tag = (tok_s.find("<|") != std::string::npos || tok_s.find("tool_call") != std::string::npos ||
                           tok_s.find("tool_response") != std::string::npos);
            bool has_alpha = false, has_digit = false, has_upper = false, has_punct = false;
            for (unsigned char ch : tok_s) {
                if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')) has_alpha = true;
                if (ch >= '0' && ch <= '9') has_digit = true;
                if (ch >= 'A' && ch <= 'Z') has_upper = true;
                if ((ch >= '!' && ch <= '/') || (ch >= ':' && ch <= '@') || (ch >= '{' && ch <= '~')) has_punct = true;
            }
            bool has_alnum = has_alpha || has_digit;
            for (int c = 0; c < D; ++c) {
                float acc = 0.0f;
                for (size_t i = 0; i < tok_s.size(); ++i) {
                    uint8_t ch = static_cast<unsigned char>(tok_s[i]);
                    float phase = float(ch) * (0.19f + 0.07f * float(c % 13)) + float(i + 1) * (0.43f * float((c / 4) + 1));
                    float w = (i == 0 ? 1.4f : (i + 1 == tok_s.size() ? 1.2f : 1.0f));
                    acc += w * ((c & 1) ? sinf(phase) : cosf(phase));
                }
                if (c < 16) acc += (is_space ? 1.0f : -0.6f) * ((c & 2) ? 1.0f : -1.0f);
                if (c >= 16 && c < 32) acc += (has_digit ? 1.0f : -0.4f) * ((c & 1) ? 1.0f : -1.0f);
                if (c >= 32 && c < 48) acc += (has_upper ? 1.0f : -0.3f) * ((c & 2) ? -1.0f : 1.0f);
                if (c >= 48 && c < 64) acc += (has_punct ? 1.0f : -0.4f) * ((c & 1) ? -1.0f : 1.0f);
                if (is_tag && c >= 64 && c < 80) acc += ((c & 1) ? 1.5f : -1.5f);
                seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
                float jitt = (float(int32_t(seed & 0xFFFF) - 32768) / 32768.0f) * 0.65f;
                vec[c] = acc + jitt;
                sum_sq += double(vec[c]) * double(vec[c]);
            }
            float rms = sqrtf(float(sum_sq / double(D)));
            float sc = (rms > 1e-4f) ? (720.0f / rms) : 1.0f;
            for (int c = 0; c < D; ++c) {
                float e = fmaxf(-1600.0f, fminf(1600.0f, vec[c] * sc));
                h_etok[size_t(v) * D + c] = e;
                h_po[size_t(v) * D + c] = e;
                h_wo[size_t(v) * D + c] = fmaxf(-1.0f, fminf(1.0f, e / 1400.0f));
            }
            if (v == 0 || v == 2) {
                h_bo[v] = -512.0f;
                h_copy_tok_gate[v] = 0;
            } else {
                double logp = log((double(resp_freq[v]) + 0.5) / (mean_freq + 0.5));
                h_bo[v] = float(fmax(-384.0, fmin(384.0, 96.0 * logp)));
                bool is_conn_stop = (tok_s == " and" || tok_s == " or" || tok_s == " where" || tok_s == " with");
                if (is_tag || is_conn_stop || (!has_alnum && tok_s != "_" && tok_s != "-" && tok_s != "/")) {
                    h_copy_tok_gate[v] = 0;
                } else if (copy_start_hits[v] > 0 && copy_hits[v] >= noncopy_hits[v]) {
                    h_copy_tok_gate[v] = 240;
                } else if (copy_hits[v] > 0) {
                    h_copy_tok_gate[v] = 224;
                } else if (is_ascii_word(tok_s)) {
                    h_copy_tok_gate[v] = 208;
                } else {
                    h_copy_tok_gate[v] = 96;
                }
            }
        }
    }

    for (auto& x : h_q) x = next_f(384.0f);
    for (auto& x : h_k) x = next_f(384.0f);
    for (auto& x : h_v) x = next_f(384.0f);
    for (auto& x : h_f) x = next_f(cfg.quant_bits == 4 ? 140.0f : 256.0f);
    for (int h = 0; h < cfg.H; ++h) h_cb[h] = ((h & 3) >= 2) ? -48.0f : -1024.0f;
    for (auto& x : h_cw) x = next_f(0.32f);

    CHECK_CUDA(cudaMemcpy(b_etok.data, h_etok.data(), h_etok.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_qlut.data, h_q.data(), h_q.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_klut.data, h_k.data(), h_k.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_vlut.data, h_v.data(), h_v.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_ffnlut.data, h_f.data(), h_f.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_wout.data, h_wo.data(), h_wo.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_pout.data, h_po.data(), h_po.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_bout.data, h_bo.data(), h_bo.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_copyb.data, h_cb.data(), h_cb.size()*4, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(b_copyw.data, h_cw.data(), h_cw.size()*4, cudaMemcpyHostToDevice));

    QuantDev qm{}; qm.cfg = cfg;
    CHECK_CUDA(cudaMalloc(&qm.e_tok, b_etok.len * 2));
    CHECK_CUDA(cudaMalloc(&qm.q_lut, qkv_n * 2));
    CHECK_CUDA(cudaMalloc(&qm.k_lut, qkv_n * 2));
    CHECK_CUDA(cudaMalloc(&qm.v_lut, qkv_n * 2));
    CHECK_CUDA(cudaMalloc(&qm.ffn_lut, ffn_n * 2));
    CHECK_CUDA(cudaMalloc(&qm.tmac_hi_T, size_t(cfg.G) * VOCAB));
    CHECK_CUDA(cudaMalloc(&qm.tmac_lo_T, size_t(cfg.G) * VOCAB));
    CHECK_CUDA(cudaMalloc(&qm.p_out_T, size_t(D) * VOCAB * 2));
    CHECK_CUDA(cudaMalloc(&qm.p_sk_T, size_t(cfg.sk_blocks) * VOCAB * 8));
    CHECK_CUDA(cudaMalloc(&qm.w_eff_T, size_t(D) * VOCAB * 4));
    CHECK_CUDA(cudaMalloc(&qm.b_out, VOCAB * 2));
    CHECK_CUDA(cudaMalloc(&qm.head_tau, size_t(L) * cfg.H * 2));
    CHECK_CUDA(cudaMalloc(&qm.head_slope, size_t(L) * cfg.H * 2));
    CHECK_CUDA(cudaMalloc(&qm.head_copy_b, cfg.H * 2));
    CHECK_CUDA(cudaMalloc(&qm.head_copy_gate, size_t(cfg.H) * cfg.G));
    CHECK_CUDA(cudaMalloc(&qm.copy_tok_gate, VOCAB));
    CHECK_CUDA(cudaMemcpy(qm.copy_tok_gate, h_copy_tok_gate.data(), VOCAB, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMalloc(&qm.tok_len, VOCAB));
    CHECK_CUDA(cudaMemcpy(qm.tok_len, h_tok_len.data(), VOCAB, cudaMemcpyHostToDevice));

    std::vector<uint16_t> h_tau(size_t(L) * cfg.H), h_slope(size_t(L) * cfg.H);
    static const uint16_t TIER_SLOPE[4] = { 384, 96, 6, 2 };
    static const uint16_t TIER_TAU[4]   = { 384, 512, 448, 384 };
    for (int l = 0; l < L; ++l) {
        for (int h = 0; h < cfg.H; ++h) {
            h_slope[l * cfg.H + h] = TIER_SLOPE[h & 3];
            h_tau[l * cfg.H + h]   = TIER_TAU[h & 3];
        }
    }
    CHECK_CUDA(cudaMemcpy(qm.head_tau, h_tau.data(), h_tau.size()*2, cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMemcpy(qm.head_slope, h_slope.data(), h_slope.size()*2, cudaMemcpyHostToDevice));

    GradDev gr{
        b_etok.grad, b_qlut.grad, b_klut.grad, b_vlut.grad, b_ffnlut.grad,
        b_wout.grad, b_pout.grad, b_bout.grad, b_copyb.grad, b_copyw.grad
    };

    SequenceSample *d_train = nullptr, *d_eval = nullptr;
    CHECK_CUDA(cudaMalloc(&d_train, train_samples.size() * sizeof(SequenceSample)));
    CHECK_CUDA(cudaMemcpy(d_train, train_samples.data(), train_samples.size() * sizeof(SequenceSample), cudaMemcpyHostToDevice));
    CHECK_CUDA(cudaMalloc(&d_eval, eval_samples.size() * sizeof(SequenceSample)));
    CHECK_CUDA(cudaMemcpy(d_eval, eval_samples.data(), eval_samples.size() * sizeof(SequenceSample), cudaMemcpyHostToDevice));

    // Allocate dynamic-stride TrunkScratchDev comfortably within RTX 4090 23 GiB VRAM
    int max_rep = batch_sz * 256;
    size_t n_bld = size_t(batch_sz) * L * MAX_SEQ * D;
    size_t n_blg = size_t(batch_sz) * L * MAX_SEQ * cfg.G;
    size_t n_btd = size_t(batch_sz) * MAX_SEQ * D;
    size_t n_bt  = size_t(batch_sz) * MAX_SEQ;
    TrunkScratchDev sc{};
    sc.max_b = batch_sz; sc.max_t = MAX_SEQ; sc.max_rep = max_rep;
    CHECK_CUDA(cudaMalloc(&sc.q_rot, n_bld * 2));
    CHECK_CUDA(cudaMalloc(&sc.k_rot, n_bld * 2));
    CHECK_CUDA(cudaMalloc(&sc.v_vec, n_bld * 2));
    CHECK_CUDA(cudaMalloc(&sc.x_in,  n_bld * 2));
    CHECK_CUDA(cudaMalloc(&sc.x_mid, n_bld * 2));
    CHECK_CUDA(cudaMalloc(&sc.q_code, n_blg));
    CHECK_CUDA(cudaMalloc(&sc.k_code, n_blg));
    CHECK_CUDA(cudaMalloc(&sc.ffn_code, n_blg));
    CHECK_CUDA(cudaMalloc(&sc.ffn_hinge, n_bld));
    CHECK_CUDA(cudaMalloc(&sc.d_x, n_btd * 4));
    CHECK_CUDA(cudaMalloc(&sc.d_x_next, n_btd * 4));
    CHECK_CUDA(cudaMalloc(&sc.d_q, n_btd * 4));
    CHECK_CUDA(cudaMalloc(&sc.d_k, n_btd * 4));
    CHECK_CUDA(cudaMalloc(&sc.d_v, n_btd * 4));
    CHECK_CUDA(cudaMalloc(&sc.rep_x_top, size_t(max_rep) * D * 2));
    CHECK_CUDA(cudaMalloc(&sc.rep_copy_log, size_t(max_rep) * VOCAB * 2));
    CHECK_CUDA(cudaMalloc(&sc.rep_gv, size_t(max_rep) * VOCAB * 4));
    CHECK_CUDA(cudaMalloc(&sc.rep_b, size_t(max_rep) * 4));
    CHECK_CUDA(cudaMalloc(&sc.rep_t, size_t(max_rep) * 4));
    CHECK_CUDA(cudaMalloc(&sc.rep_k_rep, size_t(max_rep) * 4));
    CHECK_CUDA(cudaMalloc(&sc.rep_target, size_t(max_rep) * 2));
    CHECK_CUDA(cudaMalloc(&sc.rep_is_copy, size_t(max_rep)));
    CHECK_CUDA(cudaMalloc(&sc.step_rep_idx, n_bt * 4));
    CHECK_CUDA(cudaMalloc(&sc.step_pred_v, n_bt * 2));
    CHECK_CUDA(cudaMalloc(&sc.step_prob_p, n_bt * 4));
    CHECK_CUDA(cudaMalloc(&sc.step_prob_t, n_bt * 4));
    CHECK_CUDA(cudaMalloc(&sc.step_copy_act, n_bt * 8));

    int *d_bidx = nullptr, *d_repoff = nullptr;
    CHECK_CUDA(cudaMalloc(&d_bidx, batch_sz * sizeof(int)));
    CHECK_CUDA(cudaMalloc(&d_repoff, batch_sz * sizeof(int)));
    unsigned long long* d_loss = nullptr; unsigned int *d_corr = nullptr, *d_tot = nullptr;
    CHECK_CUDA(cudaMalloc(&d_loss, 8)); CHECK_CUDA(cudaMalloc(&d_corr, 4)); CHECK_CUDA(cudaMalloc(&d_tot, 4));

    std::vector<uint16_t> scales(3 + 4 * L, 64);
    scales[1] = 16;
    auto launch_quantize = [&](const std::vector<uint16_t>& sc_vec) {
        auto q8 = [&](const float* s, int16_t* d, size_t n, uint16_t qsc) {
            quantize_array_i8_kernel<<<(n + 255) / 256, 256>>>(s, d, n, qsc);
        };
        auto q4 = [&](const float* s, int16_t* d, size_t n, uint16_t qsc) {
            quantize_array_i4_kernel<<<(n + 255) / 256, 256>>>(s, d, n, qsc);
        };
        q8(b_etok.data, qm.e_tok, b_etok.len, sc_vec[0]);
        size_t q_layer_n = size_t(cfg.G) * 256 * QKV_OUT;
        size_t f_layer_n = size_t(cfg.G) * 256 * D;
        for (int l = 0; l < L; ++l) {
            q8(b_qlut.data + l * q_layer_n, qm.q_lut + l * q_layer_n, q_layer_n, sc_vec[3 + 4 * l + 0]);
            q8(b_klut.data + l * q_layer_n, qm.k_lut + l * q_layer_n, q_layer_n, sc_vec[3 + 4 * l + 1]);
            q8(b_vlut.data + l * q_layer_n, qm.v_lut + l * q_layer_n, q_layer_n, sc_vec[3 + 4 * l + 2]);
            if (cfg.quant_bits == 4)
                q4(b_ffnlut.data + l * f_layer_n, qm.ffn_lut + l * f_layer_n, f_layer_n, sc_vec[3 + 4 * l + 3]);
            else
                q8(b_ffnlut.data + l * f_layer_n, qm.ffn_lut + l * f_layer_n, f_layer_n, sc_vec[3 + 4 * l + 3]);
        }
        quantize_tmac_readout_T_kernel<<<(VOCAB + 127) / 128, 128>>>(
            b_wout.data, qm.tmac_hi_T, qm.tmac_lo_T, qm.w_eff_T, qm.p_out_T, qm.p_sk_T,
            b_pout.data, b_bout.data, qm.b_out, sc_vec[2], D, cfg.G, cfg.sk_blocks);
        quantize_copy_gates_kernel<<<1, cfg.H>>>(b_copyw.data, b_copyb.data, qm.head_copy_gate, qm.head_copy_b, D, cfg.H, cfg.G);
        CHECK_CUDA(cudaGetLastError());
    };

    auto refresh_scales = [&]() {
        CHECK_CUDA(cudaMemcpy(h_etok.data(), b_etok.data, h_etok.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_q.data(), b_qlut.data, h_q.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_k.data(), b_klut.data, h_k.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_v.data(), b_vlut.data, h_v.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_f.data(), b_ffnlut.data, h_f.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_po.data(), b_pout.data, h_po.size()*4, cudaMemcpyDeviceToHost));
        scales[0] = compute_scale_i8(h_etok);
        scales[1] = 16;
        scales[2] = compute_scale_i8(h_po);
        size_t q_ln = size_t(cfg.G) * 256 * QKV_OUT, f_ln = size_t(cfg.G) * 256 * D;
        for (int l = 0; l < L; ++l) {
            auto sub_sc = [&](const std::vector<float>& vec, size_t off, size_t n, bool q4) -> uint16_t {
                float ma = 64.0f;
                for (size_t i = off; i < off + n; ++i) {
                    float ax = fabsf(std::max(-8192.0f, std::min(8192.0f, vec[i])));
                    if (ax > ma) ma = ax;
                }
                int32_t sc_val = q4 ? int32_t((ma * 2.0f + 6.0f) / 7.0f) : int32_t((ma * 16.0f + 126.0f) / 127.0f);
                return uint16_t(std::max(16, std::min(65535, sc_val)));
            };
            scales[3 + 4 * l + 0] = sub_sc(h_q, l * q_ln, q_ln, false);
            scales[3 + 4 * l + 1] = sub_sc(h_k, l * q_ln, q_ln, false);
            scales[3 + 4 * l + 2] = sub_sc(h_v, l * q_ln, q_ln, false);
            scales[3 + 4 * l + 3] = sub_sc(h_f, l * f_ln, f_ln, cfg.quant_bits == 4);
        }
    };

    auto run_batch = [&](const std::vector<SequenceSample>& src, const SequenceSample* d_src,
                         const int* indices, int bn, int phase_ste) -> int {
        int T_max = 1, N_rep = 0;
        std::vector<int> rep_offs(bn);
        for (int k = 0; k < bn; ++k) {
            const auto& seq = src[indices[k]];
            int T = int(seq.plen) + int(seq.rlen);
            if (T > T_max) T_max = T;
            rep_offs[k] = N_rep;
            if (seq.rlen > 0) N_rep += int(seq.rlen);
        }
        CHECK_CUDA(cudaMemcpy(d_bidx, indices, bn * 4, cudaMemcpyHostToDevice));
        CHECK_CUDA(cudaMemcpy(d_repoff, rep_offs.data(), bn * 4, cudaMemcpyHostToDevice));
        fwd_trunk_kernel<<<bn, 256>>>(d_src, d_bidx, d_repoff, T_max, qm, sc);
        if (N_rep > 0) {
            if (phase_ste) {
                CHECK_CUDA(cudaMemset(sc.d_x, 0, size_t(bn) * T_max * D * sizeof(float)));
            }
            readout_scores_kernel<<<N_rep, 256>>>(N_rep, T_max, qm, sc, d_loss, d_corr, d_tot, phase_ste);
            if (phase_ste) {
                readout_weight_grad_exec_kernel<<<VOCAB, D>>>(N_rep, D, cfg.d_shift, qm.p_out_T, sc, gr);
                bwd_trunk_kernel<<<bn, 256>>>(d_src, d_bidx, T_max, qm, gr, sc);
            }
        }
        return N_rep;
    };

    auto run_eval = [&](const std::vector<SequenceSample>& ev_vec, const SequenceSample* d_src) -> std::pair<double, double> {
        unsigned long long h_l = 0; unsigned int h_c = 0, h_t = 0;
        CHECK_CUDA(cudaMemset(d_loss, 0, 8)); CHECK_CUDA(cudaMemset(d_corr, 0, 4)); CHECK_CUDA(cudaMemset(d_tot, 0, 4));
        int lim = int(ev_vec.size());
        for (int off = 0; off < lim; off += batch_sz) {
            int bn = std::min(batch_sz, lim - off);
            std::vector<int> idx(bn);
            for (int k = 0; k < bn; ++k) idx[k] = off + k;
            run_batch(ev_vec, d_src, idx.data(), bn, 0);
        }
        CHECK_CUDA(cudaDeviceSynchronize());
        CHECK_CUDA(cudaMemcpy(&h_l, d_loss, 8, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_c, d_corr, 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_t, d_tot, 4, cudaMemcpyDeviceToHost));
        return { double(h_l) / (1000.0 * std::max(1u, h_t)), double(h_c) / std::max(1u, h_t) };
    };

    auto export_host_model = [&]() -> HostQuantModel {
        CHECK_CUDA(cudaMemcpy(h_etok.data(), b_etok.data, h_etok.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_q.data(), b_qlut.data, h_q.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_k.data(), b_klut.data, h_k.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_v.data(), b_vlut.data, h_v.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_f.data(), b_ffnlut.data, h_f.size()*4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_po.data(), b_pout.data, h_po.size()*4, cudaMemcpyDeviceToHost));

        HostQuantModel out_hm;
        out_hm.cfg = cfg;
        out_hm.scales_q8 = scales;
        out_hm.e_tok.resize(b_etok.len);
        out_hm.q_e_tok.resize(b_etok.len);
        for (size_t i = 0; i < b_etok.len; ++i) {
            int32_t q = std::max(-127, std::min(127, int32_t(lrintf((h_etok[i] * 16.0f) / float(scales[0])))));
            out_hm.q_e_tok[i] = int8_t(q);
            out_hm.e_tok[i] = dequant_i8_q12(int8_t(q), scales[0]);
        }
        out_hm.q_e_pos.assign(size_t(MAX_POS_EMBED) * D, 0);

        out_hm.layers.resize(L);
        size_t q_ln = size_t(cfg.G) * 256 * QKV_OUT, f_ln = size_t(cfg.G) * 256 * D;
        for (int l = 0; l < L; ++l) {
            auto& ly = out_hm.layers[l];
            ly.head_tau.assign(h_tau.begin() + l * cfg.H, h_tau.begin() + (l + 1) * cfg.H);
            ly.head_slope.assign(h_slope.begin() + l * cfg.H, h_slope.begin() + (l + 1) * cfg.H);
            ly.head_shift.assign(cfg.H, 0);
            ly.head_copy_b.resize(cfg.H);
            CHECK_CUDA(cudaMemcpy(ly.head_copy_b.data(), qm.head_copy_b, cfg.H * 2, cudaMemcpyDeviceToHost));
            ly.head_copy_gate.resize(size_t(cfg.H) * cfg.G);
            CHECK_CUDA(cudaMemcpy(ly.head_copy_gate.data(), qm.head_copy_gate, ly.head_copy_gate.size(), cudaMemcpyDeviceToHost));

            auto quant_slice_i8 = [&](const std::vector<float>& src, size_t off, size_t len, uint16_t sc_v,
                                      std::vector<int8_t>& qo, std::vector<int16_t>& do_q12) {
                qo.resize(len); do_q12.resize(len);
                for (size_t i = 0; i < len; ++i) {
                    int32_t q = std::max(-127, std::min(127, int32_t(lrintf((src[off + i] * 16.0f) / float(sc_v)))));
                    qo[i] = int8_t(q); do_q12[i] = dequant_i8_q12(int8_t(q), sc_v);
                }
            };
            quant_slice_i8(h_q, l * q_ln, q_ln, scales[3 + 4 * l + 0], ly.q_q, ly.q_lut);
            quant_slice_i8(h_k, l * q_ln, q_ln, scales[3 + 4 * l + 1], ly.q_k, ly.k_lut);
            quant_slice_i8(h_v, l * q_ln, q_ln, scales[3 + 4 * l + 2], ly.q_v, ly.v_lut);
            if (cfg.quant_bits == 4) {
                ly.q_ffn.resize(f_ln); ly.ffn_lut.resize(f_ln);
                uint16_t sc_v = scales[3 + 4 * l + 3];
                for (size_t i = 0; i < f_ln; ++i) {
                    int32_t q4 = std::max(-8, std::min(7, int32_t(lrintf((h_f[l * f_ln + i] * 2.0f) / float(sc_v)))));
                    ly.q_ffn[i] = int8_t(q4); ly.ffn_lut[i] = dequant_i4_q12(int8_t(q4), sc_v);
                }
            } else {
                quant_slice_i8(h_f, l * f_ln, f_ln, scales[3 + 4 * l + 3], ly.q_ffn, ly.ffn_lut);
            }
        }

        std::vector<uint8_t> h_tmac_hi_T(size_t(cfg.G) * VOCAB), h_tmac_lo_T(size_t(cfg.G) * VOCAB);
        std::vector<uint64_t> h_p_sk_T(size_t(cfg.sk_blocks) * VOCAB);
        CHECK_CUDA(cudaMemcpy(h_tmac_hi_T.data(), qm.tmac_hi_T, h_tmac_hi_T.size(), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_tmac_lo_T.data(), qm.tmac_lo_T, h_tmac_lo_T.size(), cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(h_p_sk_T.data(), qm.p_sk_T, h_p_sk_T.size() * 8, cudaMemcpyDeviceToHost));

        out_hm.tmac_hi.resize(size_t(VOCAB) * cfg.G);
        out_hm.tmac_lo.resize(size_t(VOCAB) * cfg.G);
        for (int v = 0; v < VOCAB; ++v) {
            for (int g = 0; g < cfg.G; ++g) {
                out_hm.tmac_hi[size_t(v) * cfg.G + g] = h_tmac_hi_T[size_t(g) * VOCAB + v];
                out_hm.tmac_lo[size_t(v) * cfg.G + g] = h_tmac_lo_T[size_t(g) * VOCAB + v];
            }
        }
        out_hm.copy_tok_gate = h_copy_tok_gate;
        out_hm.q_p_out.resize(b_pout.len); out_hm.p_out.resize(b_pout.len);
        for (size_t i = 0; i < b_pout.len; ++i) {
            int32_t q = std::max(-127, std::min(127, int32_t(lrintf((h_po[i] * 16.0f) / float(scales[2])))));
            out_hm.q_p_out[i] = int8_t(q); out_hm.p_out[i] = dequant_i8_q12(int8_t(q), scales[2]);
        }
        out_hm.p_sk.resize(size_t(VOCAB) * cfg.sk_blocks);
        for (int v = 0; v < VOCAB; ++v) {
            for (int b = 0; b < cfg.sk_blocks; ++b) {
                out_hm.p_sk[size_t(v) * cfg.sk_blocks + b] = h_p_sk_T[size_t(b) * VOCAB + v];
            }
        }
        out_hm.b_out.resize(VOCAB);
        CHECK_CUDA(cudaMemcpy(out_hm.b_out.data(), qm.b_out, VOCAB * 2, cudaMemcpyDeviceToHost));
        return out_hm;
    };

    refresh_scales();
    launch_quantize(scales);
    CHECK_CUDA(cudaDeviceSynchronize());

    auto t_start = std::chrono::steady_clock::now();
    std::vector<int> order(train_samples.size());
    std::iota(order.begin(), order.end(), 0);

    double best_eval_acc = -1.0, last_train_loss = 0.0, last_train_acc = 0.0, last_eval_loss = 0.0;
    HostQuantModel best_hm{};
    bool has_best_hm = false;
    uint32_t steps_done = 0;
    const uint32_t approx_batches = uint32_t((train_samples.size() + batch_sz - 1) / batch_sz);
    const uint32_t total_steps_est = std::max<uint32_t>(1, uint32_t(epochs) * approx_batches);
    const uint32_t warmup_steps = std::min<uint32_t>(40, std::max<uint32_t>(4, total_steps_est / 20));

    for (int ep = 1; ep <= epochs; ++ep) {
        for (size_t i = order.size() - 1; i > 0; --i) {
            rng_state ^= rng_state << 13; rng_state ^= rng_state >> 7; rng_state ^= rng_state << 17;
            std::swap(order[i], order[rng_state % (i + 1)]);
        }
        // Length-bucket within super-batches of `16 * batch_sz` so T_stride stays compact while keeping randomness
        int mega = batch_sz * 16;
        for (size_t start = 0; start < order.size(); start += mega) {
            size_t end = std::min(order.size(), start + mega);
            std::sort(order.begin() + start, order.begin() + end, [&](int a, int b) {
                return (train_samples[a].plen + train_samples[a].rlen) < (train_samples[b].plen + train_samples[b].rlen);
            });
        }
        std::vector<std::pair<int, int>> batches;
        for (int off = 0; off < int(order.size()); off += batch_sz) {
            batches.push_back({off, std::min(batch_sz, int(order.size()) - off)});
        }
        for (size_t i = batches.size() - 1; i > 0; --i) {
            rng_state ^= rng_state << 13; rng_state ^= rng_state >> 7; rng_state ^= rng_state << 17;
            std::swap(batches[i], batches[rng_state % (i + 1)]);
        }

        CHECK_CUDA(cudaMemset(d_loss, 0, 8)); CHECK_CUDA(cudaMemset(d_corr, 0, 4)); CHECK_CUDA(cudaMemset(d_tot, 0, 4));

        for (size_t bi = 0; bi < batches.size(); ++bi) {
            int off = batches[bi].first, bn = batches[bi].second;
            int n_rep = run_batch(train_samples, d_train, order.data() + off, bn, 1);

            ++steps_done;
            double prog = (steps_done <= warmup_steps)
                ? (double(steps_done) / double(warmup_steps))
                : (0.20 + 0.80 * 0.5 * (1.0 + cos(3.14159265358979323846 * std::min(1.0, double(steps_done - warmup_steps) / double(std::max<uint32_t>(1, total_steps_est - warmup_steps))))));
            float sched = float(prog);
            (void)n_rep;

            const float beta1 = 0.85f;
            const float beta2 = 0.98f;
            const float eps = 1e-4f;
            float bc1 = 1.0f - powf(beta1, float(steps_done));
            float bc2 = 1.0f - powf(beta2, float(steps_done));
            auto step_adam = [&](DevBuf& buf, float base_lr, float max_step, float clip_lo, float clip_hi) {
                adam_step_kernel<<<(buf.len + 255) / 256, 256>>>(
                    buf.data, buf.m, buf.v, buf.grad, buf.len,
                    base_lr * sched, beta1, beta2, bc1, bc2, eps,
                    max_step * sched + 0.15f * max_step, clip_lo, clip_hi);
            };
            step_adam(b_etok,   12.0f, 16.0f, -1400.0f, 1400.0f);
            step_adam(b_qlut,   24.0f, 32.0f, -1600.0f, 1600.0f);
            step_adam(b_klut,   24.0f, 32.0f, -1600.0f, 1600.0f);
            step_adam(b_vlut,   18.0f, 24.0f, -1400.0f, 1400.0f);
            step_adam(b_ffnlut, cfg.quant_bits == 4 ? 28.0f : 18.0f,
                                cfg.quant_bits == 4 ? 36.0f : 24.0f,
                                cfg.quant_bits == 4 ? -640.0f : -1200.0f,
                                cfg.quant_bits == 4 ? 640.0f : 1200.0f);
            step_adam(b_wout,   0.12f, 0.25f,    -4.2f,    4.2f);
            step_adam(b_pout,   16.0f, 20.0f, -1400.0f, 1400.0f);
            step_adam(b_bout,    6.0f,  8.0f,  -384.0f,  384.0f);
            step_adam(b_copyb,   4.0f,  6.0f, -1024.0f,  192.0f);
            clamp_copy_bias_kernel<<<1, cfg.H>>>(b_copyb.data, cfg.H);
            step_adam(b_copyw,  0.10f, 0.16f,    -1.2f,    1.2f);

            if (steps_done == 1 || steps_done % 32 == 0) {
                refresh_scales();
            }
            launch_quantize(scales);
            if (bi == 0 || (bi + 1) % 25 == 0 || bi + 1 == batches.size()) {
                CHECK_CUDA(cudaDeviceSynchronize());
                unsigned long long h_l = 0; unsigned int h_c = 0, h_t = 0;
                CHECK_CUDA(cudaMemcpy(&h_l, d_loss, 8, cudaMemcpyDeviceToHost));
                CHECK_CUDA(cudaMemcpy(&h_c, d_corr, 4, cudaMemcpyDeviceToHost));
                CHECK_CUDA(cudaMemcpy(&h_t, d_tot, 4, cudaMemcpyDeviceToHost));
                double secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - t_start).count();
                printf("[gpu-v11] ep=%d/%d batch=%zu/%zu time=%.1fs tr_loss=%.3f tr_acc=%.2f%%\n",
                       ep, epochs, bi + 1, batches.size(), secs,
                       double(h_l) / (1000.0 * std::max(1u, h_t)),
                       100.0 * double(h_c) / std::max(1u, h_t));
                fflush(stdout);
            }
        }
        refresh_scales();
        launch_quantize(scales);
        CHECK_CUDA(cudaDeviceSynchronize());

        unsigned long long h_l = 0; unsigned int h_c = 0, h_t = 0;
        CHECK_CUDA(cudaMemcpy(&h_l, d_loss, 8, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_c, d_corr, 4, cudaMemcpyDeviceToHost));
        CHECK_CUDA(cudaMemcpy(&h_t, d_tot, 4, cudaMemcpyDeviceToHost));
        last_train_loss = double(h_l) / (1000.0 * std::max(1u, h_t));
        last_train_acc = double(h_c) / std::max(1u, h_t);
        auto [ev_l, ev_a] = run_eval(eval_samples, d_eval);
        last_eval_loss = ev_l;
        HostQuantModel snap{};
        bool snap_ready = false;
        if (ev_a >= best_eval_acc) {
            best_eval_acc = ev_a;
            snap = export_host_model();
            snap_ready = true;
            best_hm = snap;
            has_best_hm = true;
        }
        double secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - t_start).count();
        printf("[gpu-v11] EPOCH %d/%d DONE steps=%u time=%.1fs train_loss=%.4f train_acc=%.2f%% eval_loss=%.4f eval_acc=%.2f%%\n",
               ep, epochs, steps_done, secs, last_train_loss, last_train_acc * 100.0, ev_l, ev_a * 100.0);
        if (ep < epochs && !prompts.empty()) {
            if (!snap_ready) snap = export_host_model();
            for (size_t pi = 0; pi < prompts.size(); ++pi) {
                auto g = snap.greedy_generate(prompts[pi].prompt_ids, vocab, 48);
                printf("[ep%d-preview] %s => %s\n", ep, prompts[pi].id.c_str(), snap.decode_tokens(g, vocab).c_str());
            }
        }
        fflush(stdout);
    }

    HostQuantModel hm = has_best_hm ? std::move(best_hm) : export_host_model();

    std::vector<uint8_t> container = hm.serialize_container();
    FILE* mf = fopen(out_model_path, "wb");
    if (!mf) { fprintf(stderr, "Failed writing %s\n", out_model_path); return 2; }
    fwrite(container.data(), 1, container.size(), mf);
    fclose(mf);

    uint8_t model_sha[32];
    sha256_compute(container.data(), container.size(), model_sha);
    std::string sha_hex = bytes_to_hex(model_sha, 32);

    std::string prompts_json = "[\n";
    for (size_t pi = 0; pi < prompts.size(); ++pi) {
        auto gen_ids = hm.greedy_generate(prompts[pi].prompt_ids, vocab, 96);
        std::string text = hm.decode_tokens(gen_ids, vocab);
        prompts_json += "    {\"id\": " + json_esc(prompts[pi].id)
                     + ", \"kind\": " + json_esc(prompts[pi].kind)
                     + ", \"tokens\": " + std::to_string(gen_ids.size())
                     + ", \"reply\": " + json_esc(text) + "}"
                     + (pi + 1 == prompts.size() ? "\n" : ",\n");
        printf("[eval-prompt] %s => %s\n", prompts[pi].id.c_str(), text.c_str());
    }
    prompts_json += "  ]";
    fflush(stdout);

    double total_secs = std::chrono::duration<double>(std::chrono::steady_clock::now() - t_start).count();
    FILE* rf = fopen(out_receipt_path, "w");
    if (rf) {
        fprintf(rf,
            "{\n"
            "  \"format\": \"UORGLLM9\",\n"
            "  \"version\": 11,\n"
            "  \"gpu\": %s,\n"
            "  \"n_layers\": %d,\n"
            "  \"d_model\": %d,\n"
            "  \"n_heads\": %d,\n"
            "  \"lut_groups\": %d,\n"
            "  \"quant_bits\": %d,\n"
            "  \"max_seq_len\": %d,\n"
            "  \"container_bytes\": %zu,\n"
            "  \"container_sha256\": \"%s\",\n"
            "  \"train_samples\": %zu,\n"
            "  \"eval_samples\": %zu,\n"
            "  \"epochs\": %d,\n"
            "  \"steps\": %u,\n"
            "  \"duration_secs\": %.2f,\n"
            "  \"final_train_loss\": %.5f,\n"
            "  \"final_train_acc\": %.5f,\n"
            "  \"final_eval_loss\": %.5f,\n"
            "  \"best_eval_acc\": %.5f,\n"
            "  \"eval_generations\": %s\n"
            "}\n",
            json_esc(prop.name).c_str(), L, D, cfg.H, cfg.G, cfg.quant_bits, MAX_SEQ,
            container.size(), sha_hex.c_str(), train_samples.size(), eval_samples.size(),
            epochs, steps_done, total_secs, last_train_loss, last_train_acc, last_eval_loss, best_eval_acc,
            prompts_json.c_str());
        fclose(rf);
    }
    printf("[gpu-v11] Wrote %s (%zu bytes, sha256=%s) in %.2fs\n", out_model_path, container.size(), sha_hex.c_str(), total_secs);
    fflush(stdout);

    b_etok.free_all(); b_qlut.free_all(); b_klut.free_all(); b_vlut.free_all(); b_ffnlut.free_all();
    b_wout.free_all(); b_pout.free_all(); b_bout.free_all(); b_copyb.free_all(); b_copyw.free_all();
    cudaFree(qm.e_tok); cudaFree(qm.q_lut); cudaFree(qm.k_lut); cudaFree(qm.v_lut); cudaFree(qm.ffn_lut);
    cudaFree(qm.tmac_hi_T); cudaFree(qm.tmac_lo_T); cudaFree(qm.p_out_T); cudaFree(qm.p_sk_T); cudaFree(qm.w_eff_T);
    cudaFree(qm.b_out); cudaFree(qm.head_tau); cudaFree(qm.head_slope); cudaFree(qm.head_copy_b); cudaFree(qm.head_copy_gate); cudaFree(qm.copy_tok_gate); cudaFree(qm.tok_len);
    cudaFree(sc.q_rot); cudaFree(sc.k_rot); cudaFree(sc.v_vec); cudaFree(sc.x_in); cudaFree(sc.x_mid);
    cudaFree(sc.q_code); cudaFree(sc.k_code); cudaFree(sc.ffn_code); cudaFree(sc.ffn_hinge);
    cudaFree(sc.d_x); cudaFree(sc.d_x_next); cudaFree(sc.d_q); cudaFree(sc.d_k); cudaFree(sc.d_v);
    cudaFree(sc.rep_x_top); cudaFree(sc.rep_copy_log); cudaFree(sc.rep_gv);
    cudaFree(sc.rep_b); cudaFree(sc.rep_t); cudaFree(sc.rep_k_rep); cudaFree(sc.rep_target); cudaFree(sc.rep_is_copy);
    cudaFree(sc.step_rep_idx); cudaFree(sc.step_pred_v); cudaFree(sc.step_prob_p); cudaFree(sc.step_prob_t); cudaFree(sc.step_copy_act);
    cudaFree(d_train); cudaFree(d_eval); cudaFree(d_bidx); cudaFree(d_repoff);
    cudaFree(d_loss); cudaFree(d_corr); cudaFree(d_tot);
    return 0;
}
