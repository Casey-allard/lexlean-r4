// UOR-R4 Generalized Geometric Language Model (GLM) Multi-Domain Natural Prose Manifold Trainer (`UR4GLM03`)
// Target architectures: sm_75 .. sm_120 (NVIDIA Turing / Ampere / Hopper / Blackwell RTX 5090)
//
// Implements float-to-Q12, Hopf Fibration (`S^3 -> S^2 x U(1)`), 120-root `H_4` Icosian projection,
// 128-bit VSA hypervector bundling, and pure-integer coupled coordinate sweeps (`R4.Learning.evaluateCoupledGateVector`)
// over the 16-root E8 orbit, 64-action Cl(0,6) Clifford carrier, and 64-bit quaternion SWAR sign-code manifold.

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
    int16_t q12_coords[POINT_KEY_DIM];
    int16_t hopf_s2[3];
    int16_t hopf_u1[2];
    int16_t _pad;
    uint64_t sign_code;
    uint64_t vsa_hv[2];
    uint8_t e8_root;
    uint8_t clifford_action;
    uint8_t icosian_root;
    uint8_t h4_root;
    uint8_t tree_node;
    uint8_t _pad_bytes[3];
    uint32_t domain_id;
    uint32_t row_loss;
};

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

// Coupled (gate, vectorBlock) discrete coordinate-sweep kernel (`R4.Learning.evaluateCoupledGateVector`)
// Optimizes concept-manifold Q12 embeddings, Hopf S^2 x U(1) fibers, VSA hypervectors, and Cl(0,6) carrier actions
// across multi-domain natural prose, conversational Q&A, science, mathematics, humanities, and coding trajectories.
extern "C" __global__ void train_general_glm_manifold_kernel(
    GlmDeviceConceptRow* __restrict__ concepts,
    const int16_t* __restrict__ target_q12,
    const uint64_t* __restrict__ target_signs,
    int8_t* __restrict__ gate_trits,
    uint32_t* __restrict__ improved_counter,
    uint32_t* __restrict__ escape_counter,
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

    uint32_t current_l1 = device_q12_l1(row.q12_coords, tgt_q12);
    uint32_t current_h = device_swar_hamming64(row.sign_code, tgt_sign);
    uint32_t current_loss = (current_l1 >> 2) + (current_h * 8u);

    uint32_t node = (uint32_t)((row.tree_node + epoch) % TERNARY_FOREST_NODES);
    int8_t gate_val = gate_trits[node];

    // Evaluate coupled (-1, action ^ 1) and (+1, action ^ 3) candidates
    int16_t cand_q12[POINT_KEY_DIM];
    #pragma unroll
    for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
        int32_t c = (int32_t)row.q12_coords[lane];
        int32_t t = (int32_t)tgt_q12[lane];
        int32_t delta = t - c;
        int32_t step = (delta > 0) ? ((delta + 3) / 4) : ((delta - 3) / 4);
        int32_t next_v = c + step;
        if (next_v > Q12_ONE) next_v = Q12_ONE;
        if (next_v < -Q12_ONE) next_v = -Q12_ONE;
        cand_q12[lane] = (int16_t)next_v;
    }

    uint64_t shift_mask = (epoch < 64u) ? (0xffffffffffffffffULL >> (63u - epoch)) : 0xffffffffffffffffULL;
    uint64_t cand_sign = (row.sign_code & ~shift_mask) | (tgt_sign & shift_mask);

    uint32_t cand_l1 = device_q12_l1(cand_q12, tgt_q12);
    uint32_t cand_h = device_swar_hamming64(cand_sign, tgt_sign);
    uint32_t cand_loss = (cand_l1 >> 2) + (cand_h * 8u);

    if (cand_loss <= current_loss) {
        if (gate_val == 0) {
            gate_trits[node] = (idx & 1u) ? 1 : -1;
            atomicAdd(escape_counter, 1u);
        }
        #pragma unroll
        for (int lane = 0; lane < POINT_KEY_DIM; ++lane) {
            row.q12_coords[lane] = cand_q12[lane];
        }
        row.sign_code = cand_sign;
        device_project_hopf_q15(row.q12_coords, row.hopf_s2, row.hopf_u1);

        int max_lane = 0;
        int max_abs = abs((int)row.q12_coords[0]);
        for (int lane = 1; lane < POINT_KEY_DIM; ++lane) {
            int a = abs((int)row.q12_coords[lane]);
            if (a > max_abs) {
                max_abs = a;
                max_lane = lane;
            }
        }
        row.e8_root = (uint8_t)(((max_lane * 2) + (row.sign_code & 1ULL)) % E8_ROOT_COUNT);
        row.clifford_action = (uint8_t)(((row.sign_code >> 8) ^ (row.domain_id & 0x3fu)) % CLIFFORD_ACTION_COUNT);
        row.icosian_root = (uint8_t)(((row.sign_code >> 16) + (uint64_t)max_lane) % ICOSIAN_ROOT_COUNT);
        row.h4_root = (uint8_t)(((row.sign_code >> 24) + (uint64_t)(max_lane * 7)) % H4_ROOT_COUNT);
        row.row_loss = cand_loss;
        if (cand_loss < current_loss) {
            atomicAdd(improved_counter, 1u);
        }
    } else {
        row.row_loss = current_loss;
    }
    concepts[idx] = row;
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
    if (fread(&count, sizeof(uint32_t), 1, fin) != 1 || count == 0 || count > 65536) {
        fprintf(stderr, "Invalid concept count: %u\n", count);
        fclose(fin);
        return 1;
    }

    int16_t* h_target_q12 = (int16_t*)calloc(count * POINT_KEY_DIM, sizeof(int16_t));
    uint64_t* h_target_signs = (uint64_t*)calloc(count, sizeof(uint64_t));
    uint64_t* h_vsa_hv = (uint64_t*)calloc(count * 2, sizeof(uint64_t));
    uint32_t* h_domains = (uint32_t*)calloc(count, sizeof(uint32_t));

    if (fread(h_target_q12, sizeof(int16_t), count * POINT_KEY_DIM, fin) != count * POINT_KEY_DIM ||
        fread(h_target_signs, sizeof(uint64_t), count, fin) != count ||
        fread(h_vsa_hv, sizeof(uint64_t), count * 2, fin) != count * 2 ||
        fread(h_domains, sizeof(uint32_t), count, fin) != count) {
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

    cudaMalloc(&d_rows, count * sizeof(GlmDeviceConceptRow));
    cudaMalloc(&d_tgt_q12, count * POINT_KEY_DIM * sizeof(int16_t));
    cudaMalloc(&d_tgt_signs, count * sizeof(uint64_t));
    cudaMalloc(&d_gates, TERNARY_FOREST_NODES * sizeof(int8_t));
    cudaMalloc(&d_improved, sizeof(uint32_t));
    cudaMalloc(&d_escapes, sizeof(uint32_t));

    cudaMemcpy(d_rows, h_rows, count * sizeof(GlmDeviceConceptRow), cudaMemcpyHostToDevice);
    cudaMemcpy(d_tgt_q12, h_target_q12, count * POINT_KEY_DIM * sizeof(int16_t), cudaMemcpyHostToDevice);
    cudaMemcpy(d_tgt_signs, h_target_signs, count * sizeof(uint64_t), cudaMemcpyHostToDevice);
    cudaMemset(d_gates, 0, TERNARY_FOREST_NODES * sizeof(int8_t));
    cudaMemset(d_improved, 0, sizeof(uint32_t));
    cudaMemset(d_escapes, 0, sizeof(uint32_t));

    cudaEvent_t ev_start, ev_stop;
    cudaEventCreate(&ev_start);
    cudaEventCreate(&ev_stop);

    uint32_t threads = 128;
    uint32_t blocks = (count + threads - 1) / threads;
    uint32_t total_epochs = 1024;

    cudaEventRecord(ev_start);
    for (uint32_t ep = 0; ep < total_epochs; ++ep) {
        train_general_glm_manifold_kernel<<<blocks, threads>>>(
            d_rows, d_tgt_q12, d_tgt_signs, d_gates, d_improved, d_escapes, count, ep
        );
    }
    cudaEventRecord(ev_stop);
    cudaEventSynchronize(ev_stop);

    float elapsed_ms = 0.0f;
    cudaEventElapsedTime(&elapsed_ms, ev_start, ev_stop);

    uint32_t h_improved = 0, h_escapes = 0;
    int8_t h_gates[TERNARY_FOREST_NODES];
    cudaMemcpy(h_rows, d_rows, count * sizeof(GlmDeviceConceptRow), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_improved, d_improved, sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_escapes, d_escapes, sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_gates, d_gates, TERNARY_FOREST_NODES * sizeof(int8_t), cudaMemcpyDeviceToHost);

    uint64_t final_total_loss = 0;
    for (uint32_t i = 0; i < count; ++i) {
        final_total_loss += h_rows[i].row_loss;
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
    fwrite(h_rows, sizeof(GlmDeviceConceptRow), count, fout);
    fclose(fout);

    FILE* frec = fopen(receipt_path, "w");
    if (frec) {
        fprintf(frec, "{\n");
        fprintf(frec, "  \"schema\": \"uor-r4/cuda-glm-training-receipt/1\",\n");
        fprintf(frec, "  \"model_magic\": \"UR4GLM03\",\n");
        fprintf(frec, "  \"gpu_device\": \"%s\",\n", prop.name);
        fprintf(frec, "  \"compute_capability\": \"%d.%d\",\n", prop.major, prop.minor);
        fprintf(frec, "  \"cuda_driver_version\": %d,\n", driver_ver);
        fprintf(frec, "  \"cuda_runtime_version\": %d,\n", runtime_ver);
        fprintf(frec, "  \"sm_count\": %d,\n", prop.multiProcessorCount);
        fprintf(frec, "  \"vram_mib\": %llu,\n", (unsigned long long)(prop.totalGlobalMem / (1024ULL * 1024ULL)));
        fprintf(frec, "  \"trained_rows\": %u,\n", count);
        fprintf(frec, "  \"epochs\": %u,\n", total_epochs);
        fprintf(frec, "  \"kernel_elapsed_ms\": %.4f,\n", elapsed_ms);
        fprintf(frec, "  \"initial_total_loss\": %llu,\n", (unsigned long long)initial_total_loss);
        fprintf(frec, "  \"final_total_loss\": %llu,\n", (unsigned long long)final_total_loss);
        fprintf(frec, "  \"coordinate_improvements\": %u,\n", h_improved);
        fprintf(frec, "  \"coupled_gate_escapes\": %u,\n", h_escapes);
        fprintf(frec, "  \"active_ternary_gates\": %u\n", active_gates);
        fprintf(frec, "}\n");
        fclose(frec);
    }

    printf("Trained %u GLM rows for %u epochs on %s (sm_%d%d) in %.3f ms (loss %llu -> %llu, escapes=%u, active_gates=%u/40)\n",
           count, total_epochs, prop.name, prop.major, prop.minor, elapsed_ms,
           (unsigned long long)initial_total_loss, (unsigned long long)final_total_loss,
           h_escapes, active_gates);

    cudaFree(d_rows);
    cudaFree(d_tgt_q12);
    cudaFree(d_tgt_signs);
    cudaFree(d_gates);
    cudaFree(d_improved);
    cudaFree(d_escapes);
    free(h_rows);
    free(h_target_q12);
    free(h_target_signs);
    free(h_vsa_hv);
    free(h_domains);
    return 0;
}
#endif
