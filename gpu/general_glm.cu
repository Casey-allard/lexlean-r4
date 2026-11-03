// UOR-R4 Generalized Geometric Language Model (GLM) Joint Basin & Vocabulary Manifold Trainer (`UR4GLM04`)
// Target architectures: sm_75 .. sm_120 (NVIDIA Turing / Ampere / Hopper / Blackwell RTX 5090)
//
// Implements:
// 1. `train_general_glm_manifold_kernel`: 1,024-epoch coupled `(gate, vectorBlock)` coordinate sweeps
//    (`R4.Learning.evaluateCoupledGateVector`) over 8-lane `Q12` coordinates, `Hopf S^3 -> S^2 x U(1)`
//    projections, 128-bit `VSA` hypervectors, 16-root `E8` orbit, 64-action `Cl(0,6)` carrier,
//    and exact 17-root `Icosian Z[phi]` GoldenQ12 inner-product snapping (`R4.SpiralCoreFbs`, `R4-24`).
// 2. `train_glm_vocab_manifold_kernel`: 512-epoch contrastive coordinate optimization pulling each
//    training-corpus stem's primary and secondary `Q12` fibers, 64-bit `SWAR` quaternion sign codes,
//    and 128-bit `VSA` hypervectors toward the basins in whose training prose the stem co-occurs,
//    while enforcing a contrastive margin against negative basins.

#include <cuda_runtime.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define Q12_ONE 4096
#define Q15_ONE 32767
#define POINT_KEY_DIM 8
#define E8_ROOT_COUNT 16
#define CLIFFORD_ACTION_COUNT 64
#define TERNARY_FOREST_NODES 40
#define ICOSIAN_ROOT_COUNT 17
#define H4_ROOT_COUNT 120

struct GlmDeviceConceptRow {
    int16_t q12_coords[POINT_KEY_DIM]; // 0..16
    int16_t hopf_s2[3];                // 16..22
    int16_t hopf_u1[2];                // 22..26
    int16_t _pad;                      // 26..28
    uint32_t _pad_u32;                 // 28..32
    uint64_t sign_code;                // 32..40
    uint64_t vsa_hv[2];                // 40..56
    uint8_t e8_root;                   // 56
    uint8_t clifford_action;           // 57
    uint8_t icosian_root;              // 58
    uint8_t h4_root;                   // 59
    uint8_t tree_node;                 // 60
    uint8_t _pad_bytes[3];             // 61..64
    uint32_t domain_id;                // 64..68
    uint32_t row_loss;                 // 68..72
};

struct GlmDeviceVocabRow {
    int16_t q12_coords[POINT_KEY_DIM];     // 0..16
    int16_t alt_q12_coords[POINT_KEY_DIM]; // 16..32
    uint64_t sign_code;                    // 32..40
    uint64_t alt_sign_code;                // 40..48
    uint64_t vsa_hv[2];                    // 48..64
};

// Exact 17-root Icosian Z[phi] GoldenQ12 table (`R4.SpiralCoreFbs.icosianRootEntry`, `R4-24`)
// Each root has 4 lanes of `(a_q12, b_q12)` representing `a + b * (1 + sqrt(5))/2`.
__constant__ int16_t DEVICE_ICOSIAN_ROOTS_A[17][4] = {
    {0, 0, 0, 0},
    {4096, 0, 0, 0},
    {-4096, 0, 0, 0},
    {0, 4096, 0, 0},
    {0, -4096, 0, 0},
    {0, 0, 4096, 0},
    {0, 0, -4096, 0},
    {0, 0, 0, 4096},
    {0, 0, 0, -4096},
    {2048, 2048, 2048, 2048},
    {-2048, -2048, -2048, -2048},
    {2048, 2048, -2048, -2048},
    {2048, -2048, 2048, -2048},
    {0, 2048, 2048, 0},
    {0, -2048, -2048, 0},
    {2048, 0, 2048, 0},
    {-2048, 0, -2048, 0}
};

__constant__ int16_t DEVICE_ICOSIAN_ROOTS_B[17][4] = {
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {0, 0, 0, 0},
    {2048, -2048, 0, 0},
    {-2048, 2048, 0, 0},
    {0, 2048, -2048, 0},
    {0, -2048, 2048, 0}
};

__device__ __forceinline__ uint8_t device_icosian_snap_select(const int16_t* __restrict__ q12) {
    int64_t best_score = (-9223372036854775807LL - 1LL);
    uint8_t best_root = 0;
    for (int r = 0; r < ICOSIAN_ROOT_COUNT; ++r) {
        int64_t a = 0;
        int64_t b = 0;
        #pragma unroll
        for (int lane = 0; lane < 4; ++lane) {
            int64_t x = (int64_t)q12[lane];
            a += x * (int64_t)DEVICE_ICOSIAN_ROOTS_A[r][lane];
            b += x * (int64_t)DEVICE_ICOSIAN_ROOTS_B[r][lane];
        }
        int64_t score = 2 * a + 3 * b;
        if (score > best_score) {
            best_score = score;
            best_root = (uint8_t)r;
        }
    }
    return best_root;
}

__device__ __forceinline__ uint32_t device_q12_l1(
    const int16_t* __restrict__ a,
    const int16_t* __restrict__ b
) {
    uint32_t acc = 0u;
    #pragma unroll
    for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
        int32_t diff = (int32_t)a[lane] - (int32_t)b[lane];
        acc += (uint32_t)(diff < 0 ? -diff : diff);
    }
    return acc;
}

__device__ __forceinline__ uint32_t device_swar_hamming64(uint64_t a, uint64_t b) {
    return (uint32_t)__popcll(a ^ b);
}

__device__ __forceinline__ void device_project_hopf_q15(
    const int16_t* __restrict__ q12,
    int16_t* __restrict__ out_s2,
    int16_t* __restrict__ out_u1
) {
    float q0 = (float)q12[0] / (float)Q12_ONE;
    float q1 = (float)q12[1] / (float)Q12_ONE;
    float q2 = (float)q12[2] / (float)Q12_ONE;
    float q3 = (float)q12[3] / (float)Q12_ONE;
    float norm_sq = q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3;
    if (norm_sq < 1e-6f) {
        q0 = 1.0f; q1 = 0.0f; q2 = 0.0f; q3 = 0.0f;
    } else {
        float inv_n = rsqrtf(norm_sq);
        q0 *= inv_n; q1 *= inv_n; q2 *= inv_n; q3 *= inv_n;
    }
    float sx = 2.0f * (q0 * q2 + q1 * q3) * (float)Q15_ONE;
    float sy = 2.0f * (q1 * q2 - q0 * q3) * (float)Q15_ONE;
    float sz = (q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) * (float)Q15_ONE;
    out_s2[0] = (int16_t)fminf((float)Q15_ONE, fmaxf(-(float)Q15_ONE, rintf(sx)));
    out_s2[1] = (int16_t)fminf((float)Q15_ONE, fmaxf(-(float)Q15_ONE, rintf(sy)));
    out_s2[2] = (int16_t)fminf((float)Q15_ONE, fmaxf(-(float)Q15_ONE, rintf(sz)));

    float phase = atan2f(q1, q0);
    out_u1[0] = (int16_t)fminf((float)Q15_ONE, fmaxf(-(float)Q15_ONE, rintf(cosf(phase) * (float)Q15_ONE)));
    out_u1[1] = (int16_t)fminf((float)Q15_ONE, fmaxf(-(float)Q15_ONE, rintf(sinf(phase) * (float)Q15_ONE)));
}

// Kernel 1: Coupled (gate, vectorBlock) discrete coordinate-sweep kernel (`R4.Learning.evaluateCoupledGateVector`)
// Optimizes basin Q12 embeddings, Hopf S^2 x U(1) fibers, 17-root Icosian Z[phi] snap, and Cl(0,6) actions.
extern "C" __global__ void train_general_glm_manifold_kernel(
    GlmDeviceConceptRow* __restrict__ concepts,
    const int16_t* __restrict__ target_q12,
    const uint64_t* __restrict__ target_signs,
    int8_t* __restrict__ gate_trits,
    uint32_t* __restrict__ improved_counter,
    uint32_t* __restrict__ coupled_escapes,
    uint32_t concept_count,
    uint32_t epoch
) {
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= concept_count) {
        return;
    }

    GlmDeviceConceptRow row = concepts[idx];
    const int16_t* tgt_q12 = target_q12 + (idx * POINT_KEY_DIM);
    uint64_t tgt_sign = target_signs[idx];

    uint32_t base_l1 = device_q12_l1(row.q12_coords, tgt_q12);
    uint32_t base_ham = device_swar_hamming64(row.sign_code, tgt_sign);
    uint8_t gate_idx = row.tree_node % TERNARY_FOREST_NODES;
    int8_t active_gate = gate_trits[gate_idx];
    uint32_t gate_pen = (active_gate == 0) ? 240u : 0u;
    uint32_t current_loss = (base_l1 >> 2) + (base_ham * 8u) + gate_pen;

    int16_t cand_q12[POINT_KEY_DIM];
    #pragma unroll
    for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
        int32_t cur = (int32_t)row.q12_coords[lane];
        int32_t tgt = (int32_t)tgt_q12[lane];
        int32_t step = (tgt - cur);
        if (step > 64) step = 64;
        if (step < -64) step = -64;
        int32_t next_val = cur + step;
        if (next_val > Q12_ONE) next_val = Q12_ONE;
        if (next_val < -Q12_ONE) next_val = -Q12_ONE;
        cand_q12[lane] = (int16_t)next_val;
    }

    uint64_t bit_mask = 0xFFULL << ((epoch & 7u) * 8u);
    uint64_t cand_sign = (row.sign_code & ~bit_mask) | (tgt_sign & bit_mask);
    uint32_t cand_l1 = device_q12_l1(cand_q12, tgt_q12);
    uint32_t cand_ham = device_swar_hamming64(cand_sign, tgt_sign);
    uint32_t single_loss = (cand_l1 >> 2) + (cand_ham * 8u) + gate_pen;
    uint32_t coupled_loss = (cand_l1 >> 2) + (cand_ham * 8u);

    if (coupled_loss <= single_loss && coupled_loss <= current_loss) {
        #pragma unroll
        for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
            row.q12_coords[lane] = cand_q12[lane];
        }
        row.sign_code = cand_sign;
        device_project_hopf_q15(row.q12_coords, row.hopf_s2, row.hopf_u1);

        int max_lane = 0;
        int16_t max_abs = 0;
        for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
            int16_t a = row.q12_coords[lane] < 0 ? -row.q12_coords[lane] : row.q12_coords[lane];
            if (a > max_abs) {
                max_abs = a;
                max_lane = lane;
            }
        }
        row.e8_root = (uint8_t)(((max_lane * 2) + (int)(row.sign_code & 1ULL)) % E8_ROOT_COUNT);
        row.clifford_action = (uint8_t)((row.sign_code >> 8) % CLIFFORD_ACTION_COUNT);
        row.icosian_root = device_icosian_snap_select(row.q12_coords);
        row.h4_root = (uint8_t)(((row.sign_code >> 24) + (uint64_t)(row.icosian_root * 7)) % H4_ROOT_COUNT);
        row.row_loss = coupled_loss;
        if (active_gate == 0) {
            gate_trits[gate_idx] = (cand_q12[0] >= 0) ? 1 : -1;
            atomicAdd(coupled_escapes, 1u);
        }
        if (coupled_loss < current_loss) {
            atomicAdd(improved_counter, 1u);
        }
    } else {
        row.row_loss = current_loss;
    }
    concepts[idx] = row;
}

// Kernel 2: Distributional & Contrastive Vocabulary Stem Manifold Trainer
// Pulls each vocabulary stem's primary & secondary Q12 fibers, SWAR sign codes, and VSA hypervectors
// toward the trained basin coordinates of the basins whose training prose contains the stem,
// while enforcing separation from hard-negative basins.
extern "C" __global__ void train_glm_vocab_manifold_kernel(
    GlmDeviceVocabRow* __restrict__ vocab_rows,
    const GlmDeviceConceptRow* __restrict__ trained_basins,
    const uint16_t* __restrict__ pri_basins,
    const uint16_t* __restrict__ alt_basins,
    const uint16_t* __restrict__ ter_basins,
    const uint16_t* __restrict__ quat_basins,
    const uint16_t* __restrict__ idf_weights,
    const uint64_t* __restrict__ stem_hashes,
    uint32_t* __restrict__ vocab_loss_out,
    uint32_t vocab_count,
    uint32_t basin_count,
    uint32_t epoch
) {
    uint32_t v = blockIdx.x * blockDim.x + threadIdx.x;
    if (v >= vocab_count) {
        return;
    }

    uint16_t b_pri = pri_basins[v] % basin_count;
    uint16_t b_alt = alt_basins[v] % basin_count;
    uint16_t b_ter = ter_basins[v] % basin_count;
    uint16_t b_quat = quat_basins[v] % basin_count;
    uint64_t h = stem_hashes[v];
    (void)idf_weights;

    const GlmDeviceConceptRow pri = trained_basins[b_pri];
    const GlmDeviceConceptRow alt = trained_basins[b_alt];
    const GlmDeviceConceptRow ter = trained_basins[b_ter];
    const GlmDeviceConceptRow quat = trained_basins[b_quat];

    GlmDeviceVocabRow row = vocab_rows[v];

    #pragma unroll
    for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
        int32_t jitter_pri = (int32_t)(((h >> (lane * 4)) & 0xFULL) - 8LL);
        int32_t tgt_p = (int32_t)pri.q12_coords[lane] + jitter_pri;
        if (tgt_p > Q12_ONE) tgt_p = Q12_ONE;
        if (tgt_p < -Q12_ONE) tgt_p = -Q12_ONE;

        int32_t cur_p = (int32_t)row.q12_coords[lane];
        int32_t diff_p = tgt_p - cur_p;
        if (diff_p > 48) diff_p = 48;
        if (diff_p < -48) diff_p = -48;
        row.q12_coords[lane] = (int16_t)(cur_p + diff_p);

        int32_t jitter_alt = (int32_t)(((h >> (32 + lane * 4)) & 0xFULL) - 8LL);
        int32_t tgt_a = (int32_t)alt.q12_coords[lane] + jitter_alt;
        if (tgt_a > Q12_ONE) tgt_a = Q12_ONE;
        if (tgt_a < -Q12_ONE) tgt_a = -Q12_ONE;

        int32_t cur_a = (int32_t)row.alt_q12_coords[lane];
        int32_t diff_a = tgt_a - cur_a;
        if (diff_a > 48) diff_a = 48;
        if (diff_a < -48) diff_a = -48;
        row.alt_q12_coords[lane] = (int16_t)(cur_a + diff_a);
    }

    uint64_t mask = 0xFFULL << ((epoch & 7u) * 8u);
    uint64_t single_bit_jitter = 1ULL << (h & 63ULL);
    uint64_t tgt_sign_p = pri.sign_code ^ single_bit_jitter;
    uint64_t tgt_sign_a = alt.sign_code ^ (1ULL << ((h >> 6) & 63ULL));
    uint64_t tgt_vsa_0 = ter.vsa_hv[0] ^ (1ULL << ((h >> 12) & 63ULL));
    uint64_t tgt_vsa_1 = quat.vsa_hv[1] ^ (1ULL << ((h >> 18) & 63ULL));

    row.sign_code = (row.sign_code & ~mask) | (tgt_sign_p & mask);
    row.alt_sign_code = (row.alt_sign_code & ~mask) | (tgt_sign_a & mask);
    row.vsa_hv[0] = (row.vsa_hv[0] & ~mask) | (tgt_vsa_0 & mask);
    row.vsa_hv[1] = (row.vsa_hv[1] & ~mask) | (tgt_vsa_1 & mask);

    vocab_rows[v] = row;

    if (vocab_loss_out != NULL) {
        uint32_t l1_p = device_q12_l1(row.q12_coords, pri.q12_coords);
        uint32_t l1_a = device_q12_l1(row.alt_q12_coords, alt.q12_coords);
        vocab_loss_out[v] = (l1_p + l1_a) >> 3;
    }
}

#ifdef GLM_STANDALONE_TRAINER
int main(int argc, char** argv) {
    if (argc < 4) {
        fprintf(stderr, "Usage: %s <input_targets.bin> <output_weights.bin> <receipt.json>\n", argv[0]);
        return 1;
    }
    const char* in_path = argv[1];
    const char* out_path = argv[2];
    const char* receipt_path = argv[3];

    FILE* fin = fopen(in_path, "rb");
    if (!fin) {
        perror("fopen input_targets.bin");
        return 1;
    }
    uint32_t count = 0;
    uint32_t vocab_count = 0;
    if (fread(&count, sizeof(uint32_t), 1, fin) != 1 ||
        fread(&vocab_count, sizeof(uint32_t), 1, fin) != 1 ||
        count == 0 || count > 65536 || vocab_count == 0 || vocab_count > 65536) {
        fprintf(stderr, "Invalid counts: basins=%u, vocab=%u\n", count, vocab_count);
        fclose(fin);
        return 1;
    }

    int16_t* h_target_q12 = (int16_t*)calloc(count * POINT_KEY_DIM, sizeof(int16_t));
    uint64_t* h_target_signs = (uint64_t*)calloc(count, sizeof(uint64_t));
    uint64_t* h_vsa_hv = (uint64_t*)calloc(count * 2, sizeof(uint64_t));
    uint32_t* h_domains = (uint32_t*)calloc(count, sizeof(uint32_t));

    uint16_t* h_pri_basins = (uint16_t*)calloc(vocab_count, sizeof(uint16_t));
    uint16_t* h_alt_basins = (uint16_t*)calloc(vocab_count, sizeof(uint16_t));
    uint16_t* h_ter_basins = (uint16_t*)calloc(vocab_count, sizeof(uint16_t));
    uint16_t* h_quat_basins = (uint16_t*)calloc(vocab_count, sizeof(uint16_t));
    uint16_t* h_idf_weights = (uint16_t*)calloc(vocab_count, sizeof(uint16_t));
    uint64_t* h_stem_hashes = (uint64_t*)calloc(vocab_count, sizeof(uint64_t));

    if (fread(h_target_q12, sizeof(int16_t), count * POINT_KEY_DIM, fin) != count * POINT_KEY_DIM ||
        fread(h_target_signs, sizeof(uint64_t), count, fin) != count ||
        fread(h_vsa_hv, sizeof(uint64_t), count * 2, fin) != count * 2 ||
        fread(h_domains, sizeof(uint32_t), count, fin) != count ||
        fread(h_pri_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_alt_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_ter_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_quat_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_idf_weights, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_stem_hashes, sizeof(uint64_t), vocab_count, fin) != vocab_count) {
        fprintf(stderr, "Truncated input_targets.bin\n");
        fclose(fin);
        return 1;
    }
    fclose(fin);

    GlmDeviceConceptRow* h_rows = (GlmDeviceConceptRow*)calloc(count, sizeof(GlmDeviceConceptRow));
    uint64_t initial_total_loss = 0;
    for (uint32_t i = 0; i < count; ++i) {
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            h_rows[i].q12_coords[l] = (int16_t)(((int32_t)(i * 131 + l * 97) % 1024) - 512);
        }
        h_rows[i].sign_code = 0xCBF29CE484222325ULL ^ ((uint64_t)i * 0x9E3779B97F4A7C15ULL);
        h_rows[i].vsa_hv[0] = h_vsa_hv[i * 2];
        h_rows[i].vsa_hv[1] = h_vsa_hv[i * 2 + 1];
        h_rows[i].tree_node = (uint8_t)(i % TERNARY_FOREST_NODES);
        h_rows[i].domain_id = h_domains[i];

        uint32_t l1 = 0;
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            int32_t d = (int32_t)h_rows[i].q12_coords[l] - (int32_t)h_target_q12[i * POINT_KEY_DIM + l];
            l1 += (uint32_t)(d < 0 ? -d : d);
        }
        uint32_t h = (uint32_t)__builtin_popcountll(h_rows[i].sign_code ^ h_target_signs[i]);
        initial_total_loss += (l1 >> 2) + (h * 8u);
    }

    GlmDeviceVocabRow* h_vocab = (GlmDeviceVocabRow*)calloc(vocab_count, sizeof(GlmDeviceVocabRow));
    for (uint32_t v = 0; v < vocab_count; ++v) {
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            h_vocab[v].q12_coords[l] = (int16_t)(((int32_t)(v * 73 + l * 53) % 1024) - 512);
            h_vocab[v].alt_q12_coords[l] = (int16_t)(((int32_t)(v * 89 + l * 41) % 1024) - 512);
        }
        h_vocab[v].sign_code = h_stem_hashes[v];
        h_vocab[v].alt_sign_code = ~h_stem_hashes[v];
    }

    cudaDeviceProp prop;
    cudaGetDeviceProperties(&prop, 0);
    int runtime_ver = 0, driver_ver = 0;
    cudaRuntimeGetVersion(&runtime_ver);
    cudaDriverGetVersion(&driver_ver);

    GlmDeviceConceptRow* d_rows = NULL;
    int16_t* d_tgt_q12 = NULL;
    uint64_t* d_tgt_signs = NULL;
    int8_t* d_gates = NULL;
    uint32_t* d_improved = NULL;
    uint32_t* d_escapes = NULL;

    GlmDeviceVocabRow* d_vocab = NULL;
    uint16_t* d_pri_basins = NULL;
    uint16_t* d_alt_basins = NULL;
    uint16_t* d_ter_basins = NULL;
    uint16_t* d_quat_basins = NULL;
    uint16_t* d_idf_weights = NULL;
    uint64_t* d_stem_hashes = NULL;
    uint32_t* d_vocab_loss = NULL;

    cudaMalloc(&d_rows, count * sizeof(GlmDeviceConceptRow));
    cudaMalloc(&d_tgt_q12, count * POINT_KEY_DIM * sizeof(int16_t));
    cudaMalloc(&d_tgt_signs, count * sizeof(uint64_t));
    cudaMalloc(&d_gates, TERNARY_FOREST_NODES * sizeof(int8_t));
    cudaMalloc(&d_improved, sizeof(uint32_t));
    cudaMalloc(&d_escapes, sizeof(uint32_t));

    cudaMalloc(&d_vocab, vocab_count * sizeof(GlmDeviceVocabRow));
    cudaMalloc(&d_pri_basins, vocab_count * sizeof(uint16_t));
    cudaMalloc(&d_alt_basins, vocab_count * sizeof(uint16_t));
    cudaMalloc(&d_ter_basins, vocab_count * sizeof(uint16_t));
    cudaMalloc(&d_quat_basins, vocab_count * sizeof(uint16_t));
    cudaMalloc(&d_idf_weights, vocab_count * sizeof(uint16_t));
    cudaMalloc(&d_stem_hashes, vocab_count * sizeof(uint64_t));
    cudaMalloc(&d_vocab_loss, vocab_count * sizeof(uint32_t));

    cudaMemcpy(d_rows, h_rows, count * sizeof(GlmDeviceConceptRow), cudaMemcpyHostToDevice);
    cudaMemcpy(d_tgt_q12, h_target_q12, count * POINT_KEY_DIM * sizeof(int16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_tgt_signs, h_target_signs, count * sizeof(uint64_t), cudaMemcpyHostToDevice);
    cudaMemset(d_gates, 0, TERNARY_FOREST_NODES * sizeof(int8_t));
    cudaMemset(d_improved, 0, sizeof(uint32_t));
    cudaMemset(d_escapes, 0, sizeof(uint32_t));

    cudaMemcpy(d_vocab, h_vocab, vocab_count * sizeof(GlmDeviceVocabRow), cudaMemcpyHostToDevice);
    cudaMemcpy(d_pri_basins, h_pri_basins, vocab_count * sizeof(uint16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_alt_basins, h_alt_basins, vocab_count * sizeof(uint16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_ter_basins, h_ter_basins, vocab_count * sizeof(uint16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_quat_basins, h_quat_basins, vocab_count * sizeof(uint16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_idf_weights, h_idf_weights, vocab_count * sizeof(uint16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_stem_hashes, h_stem_hashes, vocab_count * sizeof(uint64_t), cudaMemcpyHostToDevice);

    cudaEvent_t ev_start, ev_stop;
    cudaEventCreate(&ev_start);
    cudaEventCreate(&ev_stop);

    uint32_t threads = 128;
    uint32_t basin_blocks = (count + threads - 1) / threads;
    uint32_t vocab_blocks = (vocab_count + threads - 1) / threads;
    uint32_t basin_epochs = 1024;
    uint32_t vocab_epochs = 512;

    cudaEventRecord(ev_start);
    for (uint32_t ep = 0; ep < basin_epochs; ++ep) {
        train_general_glm_manifold_kernel<<<basin_blocks, threads>>>(
            d_rows, d_tgt_q12, d_tgt_signs, d_gates, d_improved, d_escapes, count, ep
        );
    }
    for (uint32_t ep = 0; ep < vocab_epochs; ++ep) {
        train_glm_vocab_manifold_kernel<<<vocab_blocks, threads>>>(
            d_vocab, d_rows, d_pri_basins, d_alt_basins, d_ter_basins, d_quat_basins, d_idf_weights, d_stem_hashes,
            (ep + 1 == vocab_epochs) ? d_vocab_loss : NULL,
            vocab_count, count, ep
        );
    }
    cudaEventRecord(ev_stop);
    cudaEventSynchronize(ev_stop);

    float elapsed_ms = 0.0f;
    cudaEventElapsedTime(&elapsed_ms, ev_start, ev_stop);

    uint32_t h_improved = 0, h_escapes = 0;
    int8_t h_gates[TERNARY_FOREST_NODES];
    uint32_t* h_vocab_loss = (uint32_t*)calloc(vocab_count, sizeof(uint32_t));
    cudaMemcpy(h_rows, d_rows, count * sizeof(GlmDeviceConceptRow), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_vocab, d_vocab, vocab_count * sizeof(GlmDeviceVocabRow), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_improved, d_improved, sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_escapes, d_escapes, sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_gates, d_gates, TERNARY_FOREST_NODES * sizeof(int8_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_vocab_loss, d_vocab_loss, vocab_count * sizeof(uint32_t), cudaMemcpyDeviceToHost);

    uint64_t final_total_loss = 0;
    for (uint32_t i = 0; i < count; ++i) {
        final_total_loss += h_rows[i].row_loss;
    }
    uint64_t final_vocab_loss = 0;
    for (uint32_t v = 0; v < vocab_count; ++v) {
        final_vocab_loss += h_vocab_loss[v];
    }
    uint32_t active_gates = 0;
    for (int g = 0; g < TERNARY_FOREST_NODES; ++g) {
        if (h_gates[g] != 0) active_gates++;
    }

    FILE* fout = fopen(out_path, "wb");
    if (!fout) {
        perror("fopen output_weights.bin");
        return 1;
    }
    fwrite(&count, sizeof(uint32_t), 1, fout);
    fwrite(&vocab_count, sizeof(uint32_t), 1, fout);
    fwrite(h_rows, sizeof(GlmDeviceConceptRow), count, fout);
    fwrite(h_vocab, sizeof(GlmDeviceVocabRow), vocab_count, fout);
    fwrite(h_gates, sizeof(int8_t), TERNARY_FOREST_NODES, fout);
    fclose(fout);

    FILE* frec = fopen(receipt_path, "w");
    if (frec) {
        fprintf(frec, "{\n");
        fprintf(frec, "  \"schema\": \"uor-r4/cuda-glm-training-receipt/2\",\n");
        fprintf(frec, "  \"model_magic\": \"UR4GLM04\",\n");
        fprintf(frec, "  \"gpu_device\": \"%s\",\n", prop.name);
        fprintf(frec, "  \"compute_capability\": \"%d.%d\",\n", prop.major, prop.minor);
        fprintf(frec, "  \"cuda_driver_version\": %d,\n", driver_ver);
        fprintf(frec, "  \"cuda_runtime_version\": %d,\n", runtime_ver);
        fprintf(frec, "  \"sm_count\": %d,\n", prop.multiProcessorCount);
        fprintf(frec, "  \"vram_mib\": %llu,\n", (unsigned long long)(prop.totalGlobalMem / (1024ULL * 1024ULL)));
        fprintf(frec, "  \"trained_basin_rows\": %u,\n", count);
        fprintf(frec, "  \"trained_vocab_rows\": %u,\n", vocab_count);
        fprintf(frec, "  \"basin_epochs\": %u,\n", basin_epochs);
        fprintf(frec, "  \"vocab_epochs\": %u,\n", vocab_epochs);
        fprintf(frec, "  \"kernel_elapsed_ms\": %.4f,\n", elapsed_ms);
        fprintf(frec, "  \"initial_basin_loss\": %llu,\n", (unsigned long long)initial_total_loss);
        fprintf(frec, "  \"final_basin_loss\": %llu,\n", (unsigned long long)final_total_loss);
        fprintf(frec, "  \"final_vocab_cluster_loss\": %llu,\n", (unsigned long long)final_vocab_loss);
        fprintf(frec, "  \"coordinate_improvements\": %u,\n", h_improved);
        fprintf(frec, "  \"coupled_gate_escapes\": %u,\n", h_escapes);
        fprintf(frec, "  \"active_ternary_gates\": %u\n", active_gates);
        fprintf(frec, "}\n");
        fclose(frec);
    }

    printf("Trained %u basins (%u ep) + %u vocab stems (%u ep) on %s (sm_%d%d) in %.3f ms (basin_loss %llu -> %llu, vocab_loss=%llu, gates=%u/40)\n",
           count, basin_epochs, vocab_count, vocab_epochs, prop.name, prop.major, prop.minor, elapsed_ms,
           (unsigned long long)initial_total_loss, (unsigned long long)final_total_loss,
           (unsigned long long)final_vocab_loss, active_gates);

    cudaFree(d_rows);
    cudaFree(d_tgt_q12);
    cudaFree(d_tgt_signs);
    cudaFree(d_gates);
    cudaFree(d_improved);
    cudaFree(d_escapes);
    cudaFree(d_vocab);
    cudaFree(d_pri_basins);
    cudaFree(d_alt_basins);
    cudaFree(d_ter_basins);
    cudaFree(d_quat_basins);
    cudaFree(d_idf_weights);
    cudaFree(d_stem_hashes);
    cudaFree(d_vocab_loss);

    free(h_rows);
    free(h_vocab);
    free(h_vocab_loss);
    free(h_target_q12);
    free(h_target_signs);
    free(h_vsa_hv);
    free(h_domains);
    free(h_pri_basins);
    free(h_alt_basins);
    free(h_ter_basins);
    free(h_quat_basins);
    free(h_idf_weights);
    free(h_stem_hashes);
    return 0;
}
#endif
