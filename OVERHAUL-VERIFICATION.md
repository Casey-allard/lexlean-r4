# LexLean-R4 `v0.13.0` Overhaul Verification Log

Authoritative verification evidence for **Track 1** (`Qwen/Qwen3-0.6B` and `Qwen/Qwen3-1.7B` GPTQ `UORR4V13` pure-integer Wasm `simd128` + `OpfsKappaStore` + HTTP `206` Safetensors Range streaming) and **Track 2** (`D85` Native `Q31` SpiralCore/FBS `UORQ31V1` lane), plus planted-defect gate proof (`R1`–`R6`, `R4-46`–`R4-53`).

## RTX 4090 `UORR4V13` Out-of-Domain Parity Receipts

| Model | Weights Format | Float Ops | Dense Matmuls | Top-1 Parity | Top-5 Parity | Mean KL (nats) | Orphan Weights Branch |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `Qwen/Qwen3-0.6B` (`28L x 1024D`) | `UORR4V13` pure int4 | `0` | `0` | `91.67%` (`11/12`) | `100.00%` (`12/12`) | `0.025384` | `weights-v13-qwen3-0.6b` (`410` pieces, `409.59 MiB`) |
| `Qwen/Qwen3-1.7B` (`28L x 2048D`) | `UORR4V13` pure int4 | `0` | `0` | `91.67%` (`11/12`) | `100.00%` (`12/12`) | `0.030286` | `weights-v13-qwen3-1.7b` (`1088` pieces, `1087.43 MiB`) |
| `D85` Native `Q31` (`4L x 256D`) | `UORQ31V1` (`847 KiB`) | `0` | `0` | Bit-exact CPU/CUDA checksum `7271024129310762567` | — | — | Embedded in Wasm (`crates/wasm/src/native_q31_d85.uorq31`) |

## Build & Normative Gate Measurements (`v0.13.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` = `10,486,024` bytes / `10.00 MiB` < `15 MiB`, `provenance.json`), `wasm=39a6e1d7cc5c`, `js=9e49a01e0b04`, `core=6647c48416a6`.
- **`check-model`**: `CONFORMANCE.md` equals `model/`, `53` conformance IDs (`R4-01..R4-53`).
- **`lift-register`**: `total=2498`, `lifted=1778`, `effect=236`, `foreign=140`, `residue=344` (`<= 355`).
- **`ratchet-check`**: shrink-only + Peirce triadic admitted (`r30_gen=223`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=344`, `float=0`, `matmul=0`, `unwired_ui=0`).
- **`check-routes`**: `9` routes and `12` test vectors checked under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=344`, `routes=9`, `provenance_sources=22`).
- **`synthesize`**: `score_milli=986`, `complexity=344`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

## Planted-Defect Gate Evidence

Each defect below was planted in an isolated workspace copy (`/var/tmp/lexlean-r4-defect-test`) with its own compiled `xtask` and conformance binary, observed failing at the intended gate, and then cleanly restored and re-checked.

### 1. Retired Scaffold Leak (`R4-46` / `R4-51`)
- **Planted defect**: Appended `// planted-defect: "uor-r4.seed-fast"` to `public/app.js`.
- **Failing command 1**: `cargo test -p repo-conformance --test holo_geometric_runtime`
  - **Exit code**: `101`
  - **Diagnostic**: `panicked at crates/conformance/tests/holo_geometric_runtime.rs:192:5: assertion failed: !app_js.contains("uor-r4.seed-fast")`
- **Failing command 2**: `./target/debug/xtask synthesize`
  - **Exit code**: `1`
  - **Diagnostic**: `gate failed: synthesize refused (RefusedNonZeroDefect): total_defects=1 > 0`
- **Restored result**: Reverted `public/app.js`; both commands exited `0` (`test result: ok. 1 passed` and `synthesize check ok: score_milli=986 complexity=344 defects=0 strategy=pure-integer-lut-serving verdict=CandidateAdmitted`).

### 2. Ambiguous UORust Route (`R4-53`)
- **Planted defect**: Appended a duplicate `[[route]]` entry (`id = "R4-UR99"`, `rust_fact = "crate-bin-target"`, `prism_record = "Foundation.Arch::Component"`, `facet = "arch"`) to `model/routes.toml`.
- **Failing command**: `./target/debug/xtask check-routes`
  - **Exit code**: `1`
  - **Diagnostic**: `gate failed: RefusedAmbiguousRoute: duplicate rust_fact \`crate-bin-target\` in model/routes.toml`
- **Restored result**: Reverted `model/routes.toml`; `./target/debug/xtask check-routes` exited `0` (`check-routes: 9 routes and 12 vectors verified (uorust/routes/1)`).

### 3. Floating-Point Serving Op Regression (`R4-44` / `R4-51` / `R4-53`)
- **Planted defect**: Set `"float_ops": 1` in `gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json`.
- **Failing command**: `./target/debug/xtask check-measures`
  - **Exit code**: `1`
  - **Diagnostic**: `gate failed: check-measures failed on \`Quality::measureFloatServingOps\` (R4.UorustPrismDevrel.floatServingOps): verdict=RefusedMeasureThresholdViolated, measured=1, threshold=0, missing_provenance=[]`
- **Restored result**: Reverted `gpu/rtx4090_holo_v13_qwen3_0_6b_receipt.json`; `./target/debug/xtask check-measures` exited `0` (`check-measures: 6 measures admitted (uncovered=0, undeclared=0, float_ops=0, matmul_ops=0, kappa_residue=344, routes=9, provenance_sources=22)`).

### 4. Corrupt BLAKE3 Footer Bypass on Hologram v4 Archive (`R4-54`)
- **Planted defect**: Bypassed `actual_footer == expected_footer` (`let blake3_footer_valid = true;`) in `crates/wasm/src/prism_holo_compiler.rs`.
- **Failing command**: `cargo test -p repo-conformance --test prism_holo_identity`
  - **Exit code**: `101`
  - **Diagnostic**: `panicked at crates/conformance/tests/prism_holo_identity.rs:301:43: called Result::unwrap_err() on an Ok value: HoloV4Summary { verdict: PrismHoloAdmitted, ... }`
- **Restored result**: Restored `let blake3_footer_valid = actual_footer == expected_footer;` in `crates/wasm/src/prism_holo_compiler.rs`; `cargo test -p repo-conformance --test prism_holo_identity` exited `0` (`test result: ok. 1 passed`).
