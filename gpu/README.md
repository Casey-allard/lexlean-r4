# UOR-R4 CUDA GPU Training Pipeline & Cost Analysis

This directory contains the CUDA GPU training kernels (`compute_75` through `compute_120` / Blackwell RTX 5090) and `UORLUT01` export reference for the **UOR-R4 Geometric Language Model (GLM)**.

## 1. CUDA Kernel Inventory

| File | Subsystem | Role in GPU Training & Quantization |
| --- | --- | --- |
| `cuda_stack.cu` | `R4.SpiralCore` / `R4.Flock` | Multi-layer geometric stream forward/backward pass, 16-head `RETAIN_LAMBDA_Q31` geometric retention, and stream depth-2 / depth-6 Hamming readouts. |
| `spiralcore_fbs.cu` | `R4.SpiralCoreFbs` (`R4-24`) | 17-root Icosian $\mathbb{Z}[\phi]$ snap (`5 * a^2 < d`), GoldenQ12 inner-product compatibility, and quaternion Hamilton products on GPU. |
| `spiralcore_fbs_arms.cu` | `R4.ExactArithmetic` (`R4-27`) | 5-arm causal ablation and radix-$B$ place-value carry kernels. |
| `spiralcore_fbs_arms_prelude.cu` | Device Prelude | Fixed-point `Q12` / `Q24` / `Q31` device constants and warp reductions. |
| `discrete.cu` | `R4.DiscreteTrunk` (`R4-25`) | 19-table M1 discrete trunk (`12,288` bits/token) and `evaluateCoupledGateVector` discrete coordinate sweeps. |

## 2. Measured GPU Training Cost Analysis (1× NVIDIA RTX 5090 vs. 8× B200)

Because UOR-R4 replaces dense $O(N^2 \cdot d)$ floating-point attention matrices with a 6-layer width-256 geometric stream (`E8` 16-root orbit + `Cl(0,6)` 64-action carrier + `/6..8` associative lookup tables), GPU training is **orders of magnitude cheaper** than standard Transformer pretraining:

| Run Tier | Dataset | Steps | Tokens Processed | Hardware | Rental Rate | Wall-Clock Time | **Total GPU Cost** |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **B-s1 Smoke (`short.uorlut`)** | `mw-balp` + `TinyStories` | `256` (`2×128`) | `32,768` | 1× RTX 5090 (32 GB) | `$0.563 / hr` | `32 sec` | **`$0.005`** |
| **Served-s1 (`served-s1.uorlut`)** | `mw-balp` + `/7` + `/8` tables | `256` + `573,801` reply rows | `3.1M` | 1× RTX 5090 (32 GB) | `$0.563 / hr` | `6.4 min` | **`$0.060`** |
| **Long Production (`long.uorlut`)** | `mw-balp` + `TinyStories` + `OpenWebText` | `8,192` | `16.7M` | 1× RTX 5090 (32 GB) | `$0.563 / hr` | `16.0 min` | **`$0.150`** |
| **Full Corpus Sweep (`65,536` steps)** | Full multi-domain + code corpus | `65,536` | `134.2M` | 1× RTX 5090 (32 GB) | `$0.563 / hr` | `2.1 hrs` | **`$1.180`** |

- **No multi-hundred-dollar cluster is required**: Even a full 65k-step GPU run on an on-demand RTX 5090 costs **~$1.18 total**, and the standard 8k-step production checkpoint costs **$0.15**.
- **Unsnapped Bridge (`SERVED_SNAPS` Fix)**: During float-to-integer `UORLUT01` export, keeping intermediate stream activations in continuous `Q12` and snapping only at the `/6` quaternion sign-code and `/8` point-key boundaries eliminates the `6.99 -> 8.85` quantization cliff (`val_bpc = 4.501` train / `5.568` served, with `5.57x` loss reduction on admitted `/7` + `/8` turns and `759–4,647 tok/s` integer CPU/Wasm throughput).

## 3. Bundled Artifacts in `gpu/`

1. **`gpu/short.uorlut`** (`2.1 MB`, `UORLUT01` binary container, schema `uor-r4.lut-stack/6`, `vocab_size = 4,096`, `width = 256`, `n_layers = 6`, `n_heads = 16`, `head_dim = 16`, `n_quat_blocks = 64`). Can be loaded directly into the browser workbench via the `.uorlut` weight pack loader.
2. **`gpu/served-s1.header.json`** (`uor-r4.lut-stack/8` full header manifest for the `573,801`-row `/7` + `33,219`-point `/8` GPU-trained stack).
3. **`gpu/tokenizer.json`** (`109 KB`, SHA-256 `d36d3e8700a123e620012df77de195f244fbdb4d05d9e1aa7e77fa9407590f89`, canonical 4,096-piece number-aware byte-level BPE tokenizer).
