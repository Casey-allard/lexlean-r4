# UOR-R4 CUDA GPU Training Pipeline & Cost Analysis

This directory contains the pure-CUDA (`compute_75` through `compute_89` Ada RTX 4090 and `compute_120` Blackwell RTX 5090) and pure-Rust (`rustc`) training, quantization, and static `RV64IMAC` guest package pipeline for the **UOR-R4 Geometric Language Model** (`UOR-R4-GLLM v10` / `UORGLLM9` and `UR4GLM06`). Runtime inference executes purely in integer Rust/WebAssembly and CUDA (`dense_matmul_ops == 0`, `softmax_ops == 0`, `turn_memory_entries == 0`, **zero Transformer components**), with offline corpus and `ELF64` package build scripts (`gpu/build_v10_corpus.py`, `gpu/mvm_guest/build_pkgs.py`) used only at build/training time.

## 1. Primary Local GPU Target (`ssh wick` — Owned RTX 4090, `\$0.00/hr`)

The primary local GPU training and verification target is the owned workstation **`ssh wick`**:

- **Hardware**: `1× NVIDIA GeForce RTX 4090` (`24,564 MiB` / `24 GB` VRAM, Ada Lovelace `SM 8.9`, `-arch=sm_89`)
- **Cost**: **`\$0.00 / hr`** (owned local hardware — zero cloud rental pods required)
- **Toolchain**: One-shot containerized CUDA toolkit via Docker (`docker run --rm --gpus all ... nvidia/cuda:12.8.0-devel-ubuntu24.04 nvcc -O3 -std=c++17 -arch=sm_89`) in an isolated `/tmp` work directory without disturbing active host workloads.

```bash
# Build 50,000-sequence v10 multi-turn + tool-calling UORCORP3 corpus & train on ssh wick (RTX 4090, sm_89):
python3 gpu/build_v10_corpus.py /tmp/uor_v10/v10_corpus.bin
nvcc -O3 -std=c++17 -arch=sm_89 gpu/train_gllm_v10.cu -o /tmp/uor_v10/train_gllm_v10
/tmp/uor_v10/train_gllm_v10 \
  /tmp/uor_v10/v10_corpus.bin \
  crates/wasm/src/bootstrap_v9.uorgllm \
  gpu/weights/v10 \
  gpu/rtx5090_gllm_v10_receipt.json
```

## 2. CUDA, Corpus & Guest Package Inventory

| File | Subsystem | Role in GPU Training, Quantization & Sandbox Build |
| --- | --- | --- |
| `train_gllm_v10.cu` | `R4.CompositionalGllm` / `R4.ConnectedStudio` (`UORGLLM9` `v10`) | Pure-geometric integer CUDA trainer (`-arch=sm_89` on `ssh wick` RTX 4090 / `-arch=sm_120` on RTX 5090) with 100% bit-exact Wasm parity (`MAX_SEQ = 1024`, `Cl(0,6)` FWHT-64 integer butterflies, 256-entry Q12 T-MAC LUTs, rational PRLL `1,000,000`-quantum causal KV attention, 81-state balanced-nonary readout, zero dense matmul, zero softmax). Exports both the `6L 64D` fast-start checkpoint (`crates/wasm/src/bootstrap_v9.uorgllm`, `3.44 MB <= 10 MB`) and the `8L 128D` 13-shard OPFS pack (`gpu/weights/v10/`, `12.90 MB <= 14 MB`). |
| `build_v10_corpus.py` | `UORCORP3` Offline Corpus Builder | Offline deterministic corpus generator building the `50,001`-sequence `1024`-token multi-turn conversational, compositional code, `MVM-Wasm`, `git`, and live-web `<tool_call>`/`<tool_response>` training split (`v10_corpus.bin`), `35,578`-sequence fast-start split (`v10_boot_corpus.bin`), disjoint held-out validation splits, and `UORVOCB9` raw byte table. |
| `mvm_guest/build_pkgs.py` & `mvm_guest/pkg_*.c` | `R4.MvmWasm` (`R4-37`) | Offline freestanding `RV64IMAC` (`clang --target=riscv64-unknown-elf -march=rv64imac -mabi=lp64 -nostdlib -static`) package builder producing all 10 static `ELF64` (`EM_RISCV = 243`) guest binaries and SHA-256 `gpu/weights/mvm-pkgs/index.json` catalog (`jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, `http`). |
| `rtx5090_gllm_v10_receipt.json` | `uor-r4.gpu-training-receipt/10` | Hardware training and post-quantization held-out evaluation receipt for the shipped `v10` `6L 64D` bootstrap and `8L 128D` OPFS checkpoints (`turn_memory_entries == 0`, `dense_matmul_ops == 0`, `softmax_ops == 0`). |
| `cuda_stack.cu` | `R4.SpiralCore` / `R4.Flock` | Multi-layer geometric stream forward/backward pass, 16-head `RETAIN_LAMBDA_Q31` geometric retention, and stream depth-2 / depth-6 Hamming readouts. |
| `spiralcore_fbs.cu` | `R4.SpiralCoreFbs` (`R4-24`) | 17-root Icosian $\mathbb{Z}[\phi]$ snap (`5 * a^2 < d`), GoldenQ12 inner-product compatibility, and quaternion Hamilton products on GPU. |
| `spiralcore_fbs_arms.cu` | `R4.ExactArithmetic` (`R4-27`) | 5-arm causal ablation and radix-$B$ place-value carry kernels. |
| `spiralcore_fbs_arms_prelude.cu` | Device Prelude | Fixed-point `Q12` / `Q24` / `Q31` device constants and warp reductions. |
| `discrete.cu` | `R4.DiscreteTrunk` (`R4-25`) | 19-table M1 discrete trunk (`12,288` bits/token) and `evaluateCoupledGateVector` discrete coordinate sweeps. |
| `general_glm.cu` | `R4.Learning` / `UR4GLM06` | Multi-domain Generalized GLM 4-kernel CUDA trainer (`train_glm_vocab_embedding_kernel`, `train_general_glm_manifold_kernel`, `train_glm_token_horizons_kernel`, `train_glm_horizon_transitions_kernel`): 1,024-epoch coupled `(gate, vectorBlock)` coordinate sweeps over 8-lane `Q12` coordinates, `Hopf S^3 -> S^2` base/fiber projections, 128-bit `VSA` hypervectors, `E8`/`Cl(0,6)`/`Icosian`/`H4` lattice indices, `/7` horizon-to-horizon geodesic continuation edges (`next_horizon_id`, `alt_horizon_id`, `transition_energy`), and 40-lane ternary gates (`{-1, 0, +1}`). |
| `glm_corpus.rs` | `UR4GLM06` Knowledge & Prose Corpus | Pure-Rust corpus defining 205 multi-domain natural prose basins (`RAW_GLM_CONCEPTS`), 7 formal logic puzzle solvers (`RAW_GLM_PUZZLES`), and 135 conversational & common-sense Q&A basins (`RAW_GLM_CONV_QA`). |
| `glm_reasoner_template.rs` | `UR4GLM06` Inference Engine | Pure-Rust natural-prose geometric decoder (`synthesize_general_glm_response`) with multi-turn pragmatic discourse state, pronoun/locative anaphora resolution (`GLM_RETAIN_LAMBDA_Q31[6]`), `/7` horizon geodesic continuation, `Hopf` fiber alignment, 128-bit `VSA` Hamming resonance, `Q12` cosine similarity, and `SWAR` quaternion sign parity. |
| `train_general_glm.rs` | `UR4GLM06` Pure-Rust Compiler | Standalone Rust binary (`rustc --edition 2021 -O gpu/train_general_glm.rs`) that emits CUDA binary targets (`--emit-cuda-targets`) and compiles the trained `gpu/rtx5090_glm_weights.bin` directly into `crates/wasm/src/glm_weights.rs` (`--compile-rust`). |
| `rtx5090_glm_weights.bin` | `UR4GLM06` Trained CUDA Tensor | Binary device weight pack (magic `UR4GLM06`, `205` basins, `9,393` vocabulary stems, `1,742` token horizons, `3,484` `/7` transitions) produced by `general_glm.cu` on an NVIDIA GeForce RTX 5090 (`sm_120`). |
| `rtx5090_training_receipt.json` | `uor-r4/cuda-glm-training-receipt/4` | Hardware execution receipt from the NVIDIA GeForce RTX 5090 (`1,024` manifold epochs, `512` vocab epochs, `256` horizon epochs, `256` transition epochs, `90.48 ms`). |

## 3. Reproducing the Pure-Rust + CUDA `UR4GLM06` Pass

```bash
# 1. Compile the pure-Rust trainer and emit binary CUDA targets
rustc --edition 2021 -O gpu/train_general_glm.rs -o /tmp/train_general_glm
/tmp/train_general_glm --emit-cuda-targets /tmp/glm_targets.bin

# 2. Compile and run the CUDA kernel on ssh wick (RTX 4090 sm_89) or RTX 5090 (sm_120)
nvcc -O3 -arch=sm_89 -DGLM_STANDALONE_TRAINER gpu/general_glm.cu -o /tmp/train_glm_cuda
/tmp/train_glm_cuda /tmp/glm_targets.bin gpu/rtx5090_glm_weights.bin gpu/rtx5090_training_receipt.json

# 3. Compile the GPU-trained binary weights into the Rust/Wasm crate
/tmp/train_general_glm --compile-rust gpu/rtx5090_glm_weights.bin crates/wasm/src/glm_weights.rs
```

## 4. Measured GPU Training Cost Analysis (`ssh wick` Local RTX 4090 vs. Cloud Rental)

Because UOR-R4 replaces dense $O(N^2 \cdot d)$ floating-point attention matrices with integer `Cl(0,6)` Walsh-Hadamard butterflies, rational PRLL causal KV attention, and discrete `Q12` T-MAC lookup tables, full dual-checkpoint `v10` training runs locally on **`ssh wick` (`1× NVIDIA GeForce RTX 4090`, `24 GB`, `-arch=sm_89`) at `\$0.00 / hr`** with zero cloud rental pods:

| Run Tier | Dataset | Steps | Tokens Processed | Hardware | Rate | Wall-Clock Time | **Total GPU Cost** |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **GLLM v10 Dual Pass (`bootstrap_v9.uorgllm` `6L 64D` + `gpu/weights/v10/` `8L 128D`)** | `50,001` full + `35,578` boot multi-turn & tool sequences (`MAX_SEQ=1024`) | `13` + `12` QAT epochs | `~42M` | **`ssh wick`: 1× RTX 4090 (`24 GB`, `sm_89`)** | **`\$0.00 / hr` (owned)** | `~20 min` | **`\$0.000`** |
| **UR4GLM06 Manifold (`rtx5090_glm_weights.bin`)** | 205 Basins + 135 Conv QA + 9,393 Vocab + 1,742 Horizons | `1,024` epochs | `205 × 1,024 + 9,393 × 512 + 1,742 × 512` | 1× RTX 5090 (32 GB) | `\$0.503 / hr` | `90.5 ms` | **`< \$0.0002`** |
| **B-s1 Smoke (`short.uorlut`)** | `mw-balp` + `TinyStories` | `256` (`2×128`) | `32,768` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `32 sec` | **`\$0.005`** |
| **Served-s1 (`served-s1.uorlut`)** | `mw-balp` + `/7` + `/8` tables | `256` + `573,801` reply rows | `3.1M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `6.4 min` | **`\$0.060`** |
| **Long Production (`long.uorlut`)** | `mw-balp` + `TinyStories` + `OpenWebText` | `8,192` | `16.7M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `16.0 min` | **`\$0.150`** |
| **Full Corpus Sweep (`65,536` steps)** | Full multi-domain + code corpus | `65,536` | `134.2M` | 1× RTX 5090 (32 GB) | `\$0.563 / hr` | `2.1 hrs` | **`\$1.180`** |

## 5. Bundled Artifacts in `gpu/`

1. **`crates/wasm/src/bootstrap_v9.uorgllm`** (`3.44 MB`, `6L 64D` `UORGLLM9` `v10` fast-start checkpoint), **`gpu/weights/v10/manifest.json` + `shard_000.bin..shard_012.bin`** (`12.90 MB`, `8L 128D` 13-shard OPFS checkpoint), and **`gpu/rtx5090_gllm_v10_receipt.json`**.
2. **`gpu/weights/mvm-pkgs/index.json` + `*.elf`** (`10` static freestanding `RV64IMAC` `ELF64` `EM_RISCV = 243` binaries built by `gpu/mvm_guest/build_pkgs.py`).
3. **`gpu/rtx5090_glm_weights.bin`** (`UR4GLM06` binary weight tensor trained on NVIDIA GeForce RTX 5090 `sm_120`) and **`gpu/rtx5090_training_receipt.json`**.
4. **`gpu/short.uorlut`** (`2.1 MB`, `UORLUT01` binary container, schema `uor-r4.lut-stack/6`, `vocab_size = 4,096`, `width = 256`, `n_layers = 6`, `n_heads = 16`, `head_dim = 16`, `n_quat_blocks = 64`). Can be loaded directly into the browser workbench via the `.uorlut` weight pack loader.
5. **`gpu/served-s1.header.json`** (`uor-r4.lut-stack/8` full header manifest for the `573,801`-row `/7` + `33,219`-point `/8` GPU-trained stack).
6. **`gpu/tokenizer.json`** (`109 KB`, SHA-256 `d36d3e8700a123e620012df77de195f244fbdb4d05d9e1aa7e77fa9407590f89`, canonical 4,096-piece number-aware byte-level BPE tokenizer).
