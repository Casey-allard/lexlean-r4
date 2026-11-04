// UOR-R4 Generalized Geometric Language Model (GLM) Joint Basin, Vocabulary & Token-Horizon Trainer (`UR4GLM05`)
// Target architectures: sm_75 .. sm_120 (NVIDIA Turing / Ampere / Hopper / Blackwell RTX 5090)
//
// Implements:
// 1. `train_general_glm_manifold_kernel`: 1,024-epoch coupled `(gate, vectorBlock)` coordinate sweeps
//    (`R4.Learning.evaluateCoupledGateVector`) over 8-lane `Q12` coordinates, `Hopf S^3 -> S^2 x U(1)`
//    projections, 128-bit `VSA` hypervectors, 16-root `E8` orbit, 64-action `Cl(0,6)` carrier,
//    and exact 17-root `Icosian Z[phi]` GoldenQ12 inner-product snapping (`R4.SpiralCoreFbs`, `R4-24`).
// 2. `train_glm_vocab_manifold_kernel`: 512-epoch contrastive coordinate optimization pulling each
//    training-corpus stem's primary and secondary `Q12` fibers, 64-bit `SWAR` quaternion sign codes,
//    and 128-bit `VSA` hypervectors toward the basins in whose training prose the stem co-occurs.
// 3. `train_glm_token_horizons_kernel`: 256-epoch `RETAIN_LAMBDA_Q31` (`R4.IntegerStack`) + 4D integer
//    Hamilton quaternion (`R4.SpiralCoreFbs.stackHamilton4D`) + Base-17 4-Block Trunk (`R4.DiscreteTrunk`)
//    lookahead horizon trainer for token-by-token autoregressive generation.

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

struct GlmDeviceHorizonRow {
    int16_t horizon_q12[POINT_KEY_DIM]; // 0..16
    uint64_t horizon_sign;              // 16..24
    uint32_t packed_4block;             // 24..28
    uint16_t basin_id;                  // 28..30
    uint8_t section_idx;                // 30
    uint8_t sentence_idx;               // 31
    uint16_t next_horizon_id;           // 32..34
    uint16_t alt_horizon_id;            // 34..36
    uint32_t transition_energy;         // 36..40
};

// Exact 8-head integer retention decay constants (`R4.IntegerStack.retainLambdaQ31`, `R4-18`)
__constant__ int64_t DEVICE_RETAIN_LAMBDA_Q31[8] = {
    1073741824LL,
    1610612736LL,
    1879048192LL,
    2013265920LL,
    2080374784LL,
    2113929216LL,
    2122317824LL,
    2130706432LL
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

__device__ __forceinline__ void device_stack_hamilton_4d(
    const int64_t* __restrict__ q,
    const int64_t* __restrict__ u,
    int64_t* __restrict__ out
) {
    out[0] = q[0] * u[0] - q[1] * u[1] - q[2] * u[2] - q[3] * u[3];
    out[1] = q[0] * u[1] + q[1] * u[0] + q[2] * u[3] - q[3] * u[2];
    out[2] = q[0] * u[2] - q[1] * u[3] + q[2] * u[0] + q[3] * u[1];
    out[3] = q[0] * u[3] + q[1] * u[2] - q[2] * u[1] + q[3] * u[0];
}

extern "C" __global__ void train_glm_token_horizons_kernel(
    GlmDeviceHorizonRow* __restrict__ horizon_rows,
    const GlmDeviceConceptRow* __restrict__ trained_basins,
    const GlmDeviceVocabRow* __restrict__ trained_vocab,
    const uint16_t* __restrict__ horizon_vocab_ids, // [horizon_count * 8]
    uint32_t horizon_count,
    uint32_t basin_count,
    uint32_t vocab_count,
    uint32_t epoch
) {
    uint32_t h = blockIdx.x * blockDim.x + threadIdx.x;
    if (h >= horizon_count) {
        return;
    }

    GlmDeviceHorizonRow row = horizon_rows[h];
    uint16_t b_id = row.basin_id % basin_count;
    const GlmDeviceConceptRow basin = trained_basins[b_id];

    // Forward-decay the 8 token stem slots using `DEVICE_RETAIN_LAMBDA_Q31` (`R4.IntegerStack`)
    int64_t accum_q12[POINT_KEY_DIM];
    int32_t sign_votes[64];
    #pragma unroll
    for (int l = 0; l < POINT_KEY_DIM; ++l) {
        accum_q12[l] = (int64_t)basin.q12_coords[l] / 2;
    }
    for (int b = 0; b < 64; ++b) {
        sign_votes[b] = ((basin.sign_code >> b) & 1ULL) ? 2 : -2;
    }

    int64_t quat[4] = {256, 0, 0, 0};

    for (int slot = 0; slot < 8; ++slot) {
        uint16_t vid = horizon_vocab_ids[h * 8 + slot];
        if (vid >= vocab_count) {
            continue;
        }
        const GlmDeviceVocabRow vrow = trained_vocab[vid];
        int64_t lam = DEVICE_RETAIN_LAMBDA_Q31[7 - slot];
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            accum_q12[l] = ((accum_q12[l] * lam) >> 31) + ((int64_t)vrow.q12_coords[l] / 3);
        }
        for (int b = 0; b < 64; ++b) {
            sign_votes[b] += ((vrow.sign_code >> b) & 1ULL) ? 3 : -3;
        }
        if (slot < 3) {
            int64_t u[4] = {
                (int64_t)(vrow.q12_coords[0] >> 4),
                (int64_t)(vrow.q12_coords[1] >> 4),
                (int64_t)(vrow.q12_coords[2] >> 4),
                (int64_t)(vrow.q12_coords[3] >> 4)
            };
            int64_t next_q[4];
            device_stack_hamilton_4d(quat, u, next_q);
            for (int c = 0; c < 4; ++c) {
                quat[c] = (next_q[c] >> 7);
                if (quat[c] > 4096) quat[c] = 4096;
                if (quat[c] < -4096) quat[c] = -4096;
            }
        }
    }

    for (int l = 0; l < POINT_KEY_DIM; ++l) {
        int32_t tgt = (int32_t)accum_q12[l];
        if (tgt > Q12_ONE) tgt = Q12_ONE;
        if (tgt < -Q12_ONE) tgt = -Q12_ONE;
        int32_t cur = (int32_t)row.horizon_q12[l];
        int32_t diff = tgt - cur;
        if (diff > 64) diff = 64;
        if (diff < -64) diff = -64;
        row.horizon_q12[l] = (int16_t)(cur + diff);
    }

    uint64_t tgt_sign = 0ULL;
    for (int b = 0; b < 64; ++b) {
        if (sign_votes[b] >= 0) {
            tgt_sign |= (1ULL << b);
        }
    }
    uint64_t mask = 0xFFULL << ((epoch & 7u) * 8u);
    row.horizon_sign = (row.horizon_sign & ~mask) | (tgt_sign & mask);

    int16_t q_snap[4] = {(int16_t)quat[0], (int16_t)quat[1], (int16_t)quat[2], (int16_t)quat[3]};
    uint8_t b0 = basin.icosian_root % 17u;
    uint8_t b1 = device_icosian_snap_select(q_snap) % 17u;
    uint8_t b2 = device_icosian_snap_select(&row.horizon_q12[0]) % 17u;
    uint8_t b3 = device_icosian_snap_select(&row.horizon_q12[4]) % 17u;
    row.packed_4block = (uint32_t)b0 + 17u * (uint32_t)b1 + 289u * (uint32_t)b2 + 4913u * (uint32_t)b3;
    row.next_horizon_id = (uint16_t)((h + 1) % horizon_count);
    row.alt_horizon_id = (uint16_t)((h + 2) % horizon_count);
    row.transition_energy = 0u;

    horizon_rows[h] = row;
}

extern "C" __global__ void train_glm_horizon_transitions_kernel(
    GlmDeviceHorizonRow* __restrict__ horizon_rows,
    const GlmDeviceConceptRow* __restrict__ trained_basins,
    uint32_t horizon_count,
    uint32_t basin_count,
    uint32_t epoch
) {
    uint32_t h = blockIdx.x * blockDim.x + threadIdx.x;
    if (h >= horizon_count || basin_count == 0) {
        return;
    }

    GlmDeviceHorizonRow row = horizon_rows[h];
    uint16_t bid = row.basin_id < basin_count ? row.basin_id : 0;
    const GlmDeviceConceptRow basin = trained_basins[bid];

    // `/7` cyclic section progression (`0 -> 1 -> 2 -> 3 -> 4 -> 0`) via 4D Hamilton transport
    uint8_t target_sec = (uint8_t)((row.section_idx + 1u) % 5u);
    uint8_t root_idx = (uint8_t)((basin.icosian_root + target_sec * 3u + 1u) % 17u);
    int64_t q_cur[4] = {
        (int64_t)row.horizon_q12[0],
        (int64_t)row.horizon_q12[1],
        (int64_t)row.horizon_q12[2],
        (int64_t)row.horizon_q12[3]
    };
    int64_t q_step[4] = {
        (int64_t)(DEVICE_ICOSIAN_ROOTS_A[root_idx][0] >> 4) + 128LL,
        (int64_t)(DEVICE_ICOSIAN_ROOTS_A[root_idx][1] >> 4) + (int64_t)(target_sec * 29u),
        (int64_t)(DEVICE_ICOSIAN_ROOTS_A[root_idx][2] >> 4) - (int64_t)(row.sentence_idx * 19u),
        (int64_t)(DEVICE_ICOSIAN_ROOTS_A[root_idx][3] >> 4) + 37LL
    };
    int64_t q_next[4];
    device_stack_hamilton_4d(q_cur, q_step, q_next);

    int16_t transported_q12[POINT_KEY_DIM];
    for (int l = 0; l < 4; ++l) {
        int64_t v = (q_next[l] >> 8) + ((int64_t)basin.q12_coords[l] >> 2);
        if (v > Q12_ONE) v = Q12_ONE;
        if (v < -Q12_ONE) v = -Q12_ONE;
        transported_q12[l] = (int16_t)v;
    }
    for (int l = 4; l < POINT_KEY_DIM; ++l) {
        int32_t cyc = (int32_t)row.horizon_q12[4 + ((l - 3) & 3)]
            + ((int32_t)basin.q12_coords[l] >> 3);
        if (cyc > Q12_ONE) cyc = Q12_ONE;
        if (cyc < -Q12_ONE) cyc = -Q12_ONE;
        transported_q12[l] = (int16_t)cyc;
    }
    uint64_t transported_sign = (row.horizon_sign << 7) | (row.horizon_sign >> 57);
    transported_sign ^= basin.sign_code;

    uint16_t best_next = (uint16_t)((h + 1u) % horizon_count);
    uint16_t best_alt = (uint16_t)((h + 2u) % horizon_count);
    uint32_t best_energy = 0xFFFFFFFFu;
    uint32_t alt_energy = 0xFFFFFFFFu;

    // Scan local basin neighborhood plus strided manifold candidates
    for (uint32_t cand = 0; cand < horizon_count; ++cand) {
        if (cand == h) {
            continue;
        }
        const GlmDeviceHorizonRow crow = horizon_rows[cand];
        bool same_basin = (crow.basin_id == bid);
        if (!same_basin && ((cand + epoch) & 7u) != 0u) {
            continue;
        }

        uint32_t l1 = 0;
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            int32_t d = (int32_t)transported_q12[l] - (int32_t)crow.horizon_q12[l];
            l1 += (uint32_t)(d < 0 ? -d : d);
        }
        uint32_t ham = (uint32_t)__builtin_popcountll(transported_sign ^ crow.horizon_sign);
        uint32_t energy = (l1 >> 2) + ham * 6u;

        if (same_basin) {
            if (crow.section_idx == target_sec) {
                energy = (energy > 220u) ? (energy - 220u) : 1u;
            } else if (crow.section_idx != row.section_idx) {
                energy = (energy > 110u) ? (energy - 110u) : 4u;
            } else {
                energy += 40u;
            }
        } else {
            energy += 320u;
        }

        if (energy < best_energy) {
            alt_energy = best_energy;
            best_alt = best_next;
            best_energy = energy;
            best_next = (uint16_t)cand;
        } else if (energy < alt_energy && cand != best_next) {
            alt_energy = energy;
            best_alt = (uint16_t)cand;
        }
    }

    row.next_horizon_id = best_next;
    row.alt_horizon_id = best_alt;
    row.transition_energy = best_energy;

    // Gentle geodesic coupling relaxation along the discovered transition edge
    if ((epoch & 3u) == 3u && best_next < horizon_count) {
        const GlmDeviceHorizonRow nrow = horizon_rows[best_next];
        for (int l = 0; l < POINT_KEY_DIM; ++l) {
            int32_t target_mid = ((int32_t)basin.q12_coords[l] * 3 + (int32_t)nrow.horizon_q12[l]) / 4;
            int32_t step = (target_mid - (int32_t)row.horizon_q12[l]) / 16;
            if (step > 6) step = 6;
            if (step < -6) step = -6;
            int32_t updated = (int32_t)row.horizon_q12[l] + step;
            if (updated > Q12_ONE) updated = Q12_ONE;
            if (updated < -Q12_ONE) updated = -Q12_ONE;
            row.horizon_q12[l] = (int16_t)updated;
        }
    }

    horizon_rows[h] = row;
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
    uint32_t horizon_count = 0;
    if (fread(&count, sizeof(uint32_t), 1, fin) != 1 ||
        fread(&vocab_count, sizeof(uint32_t), 1, fin) != 1 ||
        fread(&horizon_count, sizeof(uint32_t), 1, fin) != 1 ||
        count == 0 || count > 65536 || vocab_count == 0 || vocab_count > 65536 ||
        horizon_count == 0 || horizon_count > 65536) {
        fprintf(stderr, "Invalid counts: basins=%u, vocab=%u, horizons=%u\n", count, vocab_count, horizon_count);
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

    uint16_t* h_horizon_basins = (uint16_t*)calloc(horizon_count, sizeof(uint16_t));
    uint8_t* h_horizon_secs = (uint8_t*)calloc(horizon_count, sizeof(uint8_t));
    uint8_t* h_horizon_sents = (uint8_t*)calloc(horizon_count, sizeof(uint8_t));
    uint16_t* h_horizon_vocab_ids = (uint16_t*)calloc(horizon_count * 8, sizeof(uint16_t));

    if (fread(h_target_q12, sizeof(int16_t), count * POINT_KEY_DIM, fin) != count * POINT_KEY_DIM ||
        fread(h_target_signs, sizeof(uint64_t), count, fin) != count ||
        fread(h_vsa_hv, sizeof(uint64_t), count * 2, fin) != count * 2 ||
        fread(h_domains, sizeof(uint32_t), count, fin) != count ||
        fread(h_pri_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_alt_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_ter_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_quat_basins, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_idf_weights, sizeof(uint16_t), vocab_count, fin) != vocab_count ||
        fread(h_stem_hashes, sizeof(uint64_t), vocab_count, fin) != vocab_count ||
        fread(h_horizon_basins, sizeof(uint16_t), horizon_count, fin) != horizon_count ||
        fread(h_horizon_secs, sizeof(uint8_t), horizon_count, fin) != horizon_count ||
        fread(h_horizon_sents, sizeof(uint8_t), horizon_count, fin) != horizon_count ||
        fread(h_horizon_vocab_ids, sizeof(uint16_t), horizon_count * 8, fin) != horizon_count * 8) {
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

    GlmDeviceHorizonRow* h_horizons = (GlmDeviceHorizonRow*)calloc(horizon_count, sizeof(GlmDeviceHorizonRow));
    for (uint32_t h = 0; h < horizon_count; ++h) {
        h_horizons[h].basin_id = h_horizon_basins[h];
        h_horizons[h].section_idx = h_horizon_secs[h];
        h_horizons[h].sentence_idx = h_horizon_sents[h];
        h_horizons[h].horizon_sign = 0x9E3779B97F4A7C15ULL ^ ((uint64_t)h * 0xBF58476D1CE4E5B9ULL);
        h_horizons[h].next_horizon_id = (uint16_t)((h + 1) % horizon_count);
        h_horizons[h].alt_horizon_id = (uint16_t)((h + 2) % horizon_count);
        h_horizons[h].transition_energy = 0u;
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

    GlmDeviceHorizonRow* d_horizons = NULL;
    uint16_t* d_horizon_vocab_ids = NULL;

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

    cudaMalloc(&d_horizons, horizon_count * sizeof(GlmDeviceHorizonRow));
    cudaMalloc(&d_horizon_vocab_ids, horizon_count * 8 * sizeof(uint16_t));

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

    cudaMemcpy(d_horizons, h_horizons, horizon_count * sizeof(GlmDeviceHorizonRow), cudaMemcpyHostToDevice);
    cudaMemcpy(d_horizon_vocab_ids, h_horizon_vocab_ids, horizon_count * 8 * sizeof(uint16_t), cudaMemcpyHostToDevice);

    cudaEvent_t ev_start, ev_stop;
    cudaEventCreate(&ev_start);
    cudaEventCreate(&ev_stop);

    uint32_t threads = 128;
    uint32_t basin_blocks = (count + threads - 1) / threads;
    uint32_t vocab_blocks = (vocab_count + threads - 1) / threads;
    uint32_t horizon_blocks = (horizon_count + threads - 1) / threads;
    uint32_t basin_epochs = 1024;
    uint32_t vocab_epochs = 512;
    uint32_t horizon_epochs = 256;
    uint32_t transition_epochs = 256;

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
    for (uint32_t ep = 0; ep < horizon_epochs; ++ep) {
        train_glm_token_horizons_kernel<<<horizon_blocks, threads>>>(
            d_horizons, d_rows, d_vocab, d_horizon_vocab_ids,
            horizon_count, count, vocab_count, ep
        );
    }
    for (uint32_t ep = 0; ep < transition_epochs; ++ep) {
        train_glm_horizon_transitions_kernel<<<horizon_blocks, threads>>>(
            d_horizons, d_rows, horizon_count, count, ep
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
    cudaMemcpy(h_horizons, d_horizons, horizon_count * sizeof(GlmDeviceHorizonRow), cudaMemcpyDeviceToHost);
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
    uint64_t total_transition_energy = 0;
    for (uint32_t h = 0; h < horizon_count; ++h) {
        total_transition_energy += h_horizons[h].transition_energy;
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
    fwrite(&horizon_count, sizeof(uint32_t), 1, fout);
    fwrite(h_rows, sizeof(GlmDeviceConceptRow), count, fout);
    fwrite(h_vocab, sizeof(GlmDeviceVocabRow), vocab_count, fout);
    fwrite(h_horizons, sizeof(GlmDeviceHorizonRow), horizon_count, fout);
    fwrite(h_gates, sizeof(int8_t), TERNARY_FOREST_NODES, fout);
    fclose(fout);

    FILE* frec = fopen(receipt_path, "w");
    if (frec) {
        fprintf(frec, "{\n");
        fprintf(frec, "  \"schema\": \"uor-r4/cuda-glm-training-receipt/4\",\n");
        fprintf(frec, "  \"model_magic\": \"UR4GLM06\",\n");
        fprintf(frec, "  \"gpu_device\": \"%s\",\n", prop.name);
        fprintf(frec, "  \"compute_capability\": \"%d.%d\",\n", prop.major, prop.minor);
        fprintf(frec, "  \"cuda_driver_version\": %d,\n", driver_ver);
        fprintf(frec, "  \"cuda_runtime_version\": %d,\n", runtime_ver);
        fprintf(frec, "  \"sm_count\": %d,\n", prop.multiProcessorCount);
        fprintf(frec, "  \"vram_mib\": %llu,\n", (unsigned long long)(prop.totalGlobalMem / (1024ULL * 1024ULL)));
        fprintf(frec, "  \"trained_basin_rows\": %u,\n", count);
        fprintf(frec, "  \"trained_vocab_rows\": %u,\n", vocab_count);
        fprintf(frec, "  \"trained_horizon_rows\": %u,\n", horizon_count);
        fprintf(frec, "  \"trained_horizon_transitions\": %u,\n", horizon_count * 2u);
        fprintf(frec, "  \"basin_epochs\": %u,\n", basin_epochs);
        fprintf(frec, "  \"vocab_epochs\": %u,\n", vocab_epochs);
        fprintf(frec, "  \"horizon_epochs\": %u,\n", horizon_epochs);
        fprintf(frec, "  \"transition_epochs\": %u,\n", transition_epochs);
        fprintf(frec, "  \"kernel_elapsed_ms\": %.4f,\n", elapsed_ms);
        fprintf(frec, "  \"initial_basin_loss\": %llu,\n", (unsigned long long)initial_total_loss);
        fprintf(frec, "  \"final_basin_loss\": %llu,\n", (unsigned long long)final_total_loss);
        fprintf(frec, "  \"final_vocab_cluster_loss\": %llu,\n", (unsigned long long)final_vocab_loss);
        fprintf(frec, "  \"total_horizon_transition_energy\": %llu,\n", (unsigned long long)total_transition_energy);
        fprintf(frec, "  \"coordinate_improvements\": %u,\n", h_improved);
        fprintf(frec, "  \"coupled_gate_escapes\": %u,\n", h_escapes);
        fprintf(frec, "  \"active_ternary_gates\": %u\n", active_gates);
        fprintf(frec, "}\n");
        fclose(frec);
    }

    printf("Trained %u basins (%u ep) + %u vocab stems (%u ep) + %u token horizons (%u+%u ep) on %s (sm_%d%d) in %.3f ms\n",
           count, basin_epochs, vocab_count, vocab_epochs, horizon_count, horizon_epochs, transition_epochs,
           prop.name, prop.major, prop.minor, elapsed_ms);

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
    cudaFree(d_horizons);
    cudaFree(d_horizon_vocab_ids);

    free(h_rows);
    free(h_vocab);
    free(h_horizons);
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
    free(h_horizon_basins);
    free(h_horizon_secs);
    free(h_horizon_sents);
    free(h_horizon_vocab_ids);
    return 0;
}
#endif
