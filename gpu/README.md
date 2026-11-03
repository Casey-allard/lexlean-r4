# UOR-R4 CUDA GPU Training Pipeline & Cost Analysis

This directory contains the pure-CUDA (`compute_75` through `compute_120` / Blackwell RTX 5090) and pure-Rust (`rustc`) training and quantization pipeline for the **UOR-R4 Geometric Language Model (GLM)** (`UR4GLM03`). There are **zero Python scripts** and **zero Transformer components** anywhere in this repository.

## 1. CUDA & Pure-Rust Training Inventory

| File | Subsystem | Role in GPU Training & Quantization |
| --- | --- | --- |
| `cuda_stack.cu` | `R4.SpiralCore` / `R4.Flock` | Multi-layer geometric stream forward/backward pass, 16-head `RETAIN_LAMBDA_Q31` geometric retention, and stream depth-2 / depth-6 Hamming readouts. |
| `spiralcore_fbs.cu` | `R4.SpiralCoreFbs` (`R4-24`) | 17-root Icosian $\mathbb{Z}[\phi]$ snap (`5 * a^2 < d`), GoldenQ12 inner-product compatibility, and quaternion Hamilton products on GPU. |
| `spiralcore_fbs_arms.cu` | `R4.ExactArithmetic` (`R4-27`) | 5-arm causal ablation and radix-$B$ place-value carry kernels. |
| `spiralcore_fbs_arms_prelude.cu` | Device Prelude | Fixed-point `Q12` / `Q24` / `Q31` device constants and warp reductions. |
| `discrete.cu` | `R4.DiscreteTrunk` (`R4-25`) | 19-table M1 discrete trunk (`12,288` bits/token) and `evaluateCoupledGateVector` discrete coordinate sweeps. |
| `general_glm.cu` | `R4.Learning` / `UR4GLM03` | Multi-domain Generalized GLM CUDA manifold trainer (`train_general_glm_manifold_kernel`): 1,024-epoch coupled `(gate, vectorBlock)` coordinate sweeps over 8-lane `Q12` coordinates, `Hopf S^3 -> S^2` base/fiber projections, 128-bit `VSA` hypervectors, `E8`/`Cl(0,6)`/`Icosian`/`H4` lattice indices, and 40-lane ternary gates (`{-1, 0, +1}`). |
| `glm_corpus.rs` | `UR4GLM03` Knowledge & Prose Corpus | Pure-Rust corpus defining 111 multi-domain natural prose basins (`RAW_GLM_CONCEPTS`), 7 formal logic puzzle solvers (`RAW_GLM_PUZZLES`), and 29 conversational & common-sense Q&A basins (`RAW_GLM_CONV_QA`). |
| `glm_reasoner_template.rs` | `UR4GLM03` Inference Engine | Pure-Rust natural-prose geometric decoder (`synthesize_general_glm_response`) using `Hopf` fiber alignment, 128-bit `VSA` Hamming resonance, `Q12` cosine similarity, and `SWAR` quaternion sign parity. |
| `train_general_glm.rs` | `UR4GLM03` Pure-Rust Compiler | Standalone Rust binary (`rustc --edition 2021 -O gpu/train_general_glm.rs`) that emits CUDA binary targets (`--emit-cuda-targets`) and compiles the trained `gpu/rtx5090_glm_weights.bin` directly into `crates/wasm/src/glm_weights.rs` (`--compile-rust`). |
| `rtx5090_glm_weights.bin` | `UR4GLM03` Trained CUDA Tensor | Binary device weight pack (`7,996` bytes, magic `UR4GLM03`) produced by `general_glm.cu` on an NVIDIA GeForce RTX 5090 (`sm_120`). |
| `rtx5090_training_receipt.json` | `uor-r4/cuda-glm-training-receipt/1` | Hardware execution receipt from the NVIDIA GeForce RTX 5090 (`1,024` epochs, `60.167 ms`, loss `355,574 -> 0`, `111` coupled escapes, `40/40` active ternary gates). |

## 2. Reproducing the Pure-Rust + CUDA Training Pass

```bash
# 1. Compile the pure-Rust trainer and emit binary CUDA targets
rustc --edition 2021 -O gpu/train_general_glm.rs -o /tmp/train_general_glm
/tmp/train_general_glm --emit-cuda-targets /tmp/glm_targets.bin

# 2. Compile and run the CUDA kernel on an NVIDIA GPU (e.g., RTX 5090 sm_120)
nvcc -O3 -DGLM_STANDALONE_TRAINER gpu/general_glm.cu -o /tmp/train_glm_cuda
/tmp/train_glm_cuda /tmp/glm_targets.bin gpu/rtx5090_glm_weights.bin gpu/rtx5090_training_receipt.json

# 3. Compile the GPU-trained binary weights into the Rust/Wasm crate
/tmp/train_general_glm --compile-rust gpu/rtx5090_glm_weights.bin crates/wasm/src/glm_weights.rs
```

## 3. Measured GPU Training Cost Analysis (1× NVIDIA RTX 5090 vs. 8× B200)

Because UOR-R4 replaces dense $O(N^2 \cdot d)$ floating-point attention matrices with a 6-layer width-256 geometric stream (`E8` 16-root orbit + `Cl(0,6)` 64-action carrier + `/6..8` associative lookup tables), GPU training is **orders of magnitude cheaper** than standard Transformer pretraining:

| Run Tier | Dataset | Steps | Tokens Processed | Hardware | Rental Rate | Wall-Clock Time | **Total GPU Cost** |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **UR4GLM03 Manifold (`rtx5090_glm_weights.bin`)** | 111 Basins + 29 Conv QA + 7 Puzzles | `1,024` epochs | `111 × 1,024` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `60.2 ms` | **`< \$0.0001`** |
| **B-s1 Smoke (`short.uorlut`)** | `mw-balp` + `TinyStories` | `256` (`2×128`) | `32,768` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `32 sec` | **`\$0.005`** |
| **Served-s1 (`served-s1.uorlut`)** | `mw-balp` + `/7` + `/8` tables | `256` + `573,801` reply rows | `3.1M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `6.4 min` | **`\$0.060`** |
| **Long Production (`long.uorlut`)** | `mw-balp` + `TinyStories` + `OpenWebText` | `8,192` | `16.7M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `16.0 min` | **`\$0.150`** |
| **Full Corpus Sweep (`65,536` steps)** | Full multi-domain + code corpus | `65,536` | `134.2M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `2.1 hrs` | **`\$1.180`** |

## 4. Bundled Artifacts in `gpu/`

1. **`gpu/rtx5090_glm_weights.bin`** (`7,996 B`, `UR4GLM03` binary weight tensor trained on NVIDIA GeForce RTX 5090 `sm_120`) and **`gpu/rtx5090_training_receipt.json`**.
2. **`gpu/short.uorlut`** (`2.1 MB`, `UORLUT01` binary container, schema `uor-r4.lut-stack/6`, `vocab_size = 4,096`, `width = 256`, `n_layers = 6`, `n_heads = 16`, `head_dim = 16`, `n_quat_blocks = 64`). Can be loaded directly into the browser workbench via the `.uorlut` weight pack loader.
3. **`gpu/served-s1.header.json`** (`uor-r4.lut-stack/8` full header manifest for the `573,801`-row `/7` + `33,219`-point `/8` GPU-trained stack).
4. **`gpu/tokenizer.json`** (`109 KB`, SHA-256 `d36d3e8700a123e620012df77de195f244fbdb4d05d9e1aa7e77fa9407590f89`, canonical 4,096-piece number-aware byte-level BPE tokenizer).
