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

## Wave 1 `R4-60` Prism Studio Frame, Live Mini-HUD & Consent Gates (`v0.14.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` = `11,466,357` bytes / `10.94 MiB` < `15 MiB`, `provenance.json`), `wasm=890006206349`, `js=c515c626a2e1`, `core=9d7855e56dfa`.
- **`build-holo --sync-site-manifest` / `--verify-site-manifest`**: verified Hologram v4 archive (`12,913,686` bytes <= `33,554,432` B / `32 MiB`, `blake3=e3ed520578776ffb`, `wasmSha256=89000620634905f2`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **`check-model`**: `CONFORMANCE.md` equals `model/`, `60` conformance IDs (`R4-01..R4-60`, including `R4-60` / `R4.StudioFrame`).
- **`lift-register`**: `total=2749`, `lifted=1972`, `effect=269`, `foreign=164`, `residue=344` (`<= 355`, `UE-16` monotone).
- **`ratchet-check`**: shrink-only + Peirce triadic verified (`r30_gen=257`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=344`, `float=0`, `matmul=0`, `unwired_ui=0`).
- **`check-routes`**: `11` routes and `14` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=344`, `routes=11`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=987`, `complexity=344`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 5. Unwired UI Control Gate (`R4-60`)
- **Planted defect**: Inserted `<button id="dead-probe" type="button">Dead Probe</button>` into `public/index.html`.
- **Failing command**: `cargo run -q -p xtask -- ratchet-check`
  - **Exit code**: `1`
  - **Diagnostic**: `gate failed: shrink-only ratchet refused \`unwired_ui_controls\`: measured=1 ceiling=0 verdict=RefusedResidualRegression; unwired_controls=["dead-probe"]`
- **Restored result**: Reverted `public/index.html` (`git checkout -- public/index.html`, empty `git diff`); `cargo run -q -p xtask -- ratchet-check` exited `0` (`ratchet-check: shrink-only + Peirce triadic verified (r30_gen=255, orphans=0, ungrounded=0, ignored=0, residue=344, float=0, matmul=0, unwired_ui=0)`).

### 6. Off-Origin Weight Download Consent Gate (`R4-60`)
- **Planted defect**: Bypassed `options.allowOffOriginHydration` / `storedConsent` verification in `hydrate_holo_model` inside `crates/wasm/js/worker_host_bridge.js` (`const allowOffOriginHydration = true;`).
- **Failing command**: `node tests/e2e/foundry_observatory_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `FOUNDRY OBSERVATORY E2E FAILED: AssertionError [ERR_ASSERTION]: The validation function is expected to return "true". Received false` (`Error: Unable to fetch UORR4V13 manifest for uor-r4.qwen3-0.6b-holo at async testStudioBridgeConsentAndGitDeletions_R4_60 (file:///Users/casey.allard/lexlean-r4/tests/e2e/foundry_observatory_e2e.mjs:1322:5)`) instead of rejecting with `ConsentRequiredForWeightDownload`.
- **Restored result**: Reverted `crates/wasm/js/worker_host_bridge.js` (`git checkout -- crates/wasm/js/worker_host_bridge.js`, empty `git diff`); `node tests/e2e/foundry_observatory_e2e.mjs` exited `0` (`=== ALL R4-59/R4-60 OBSERVATORY & STUDIO BRIDGE E2E CHECKS PASSED ===`).

## Wave 2 `R4-61` Pure-Integer WebGPU WGSL, Geodesic SwiGLU & HOLOSLAB2 Gates (`v0.14.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm`, `provenance.json`), `wasm=87950fde301d`, `js=4b2821b65884`, `core=9f48685d06d1`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,001,366` bytes <= `33,554,432` B / `32 MiB`, `blake3=e8ae2bbb03825bee`, `wasmSha256=87950fde301d7f20`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **`check-model`**: `CONFORMANCE.md` matches `model/` with `R4-61` (`R4.WebGpuGeodesicRuntime`) admitted.
- **`lift-register`**: `total=2782`, `lifted=2007`, `effect=271`, `foreign=160`, `residue=344` (`<= 355`, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=344`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `12` routes (including `R4-UR12` / `webgpu-geodesic-slab`) and `15` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=344`, `routes=12`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=987`, `complexity=344`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 7. WGSL Pure-Integer & 0-LSB Parity Gate (`W2-1`, `R4-61`)
- **Planted defect**: Inserted `var bad_float: f32 = 0;` inside `WGSL_HOLO_SHADERS.holo_int4_gemv_gemm` in `crates/wasm/js/worker_host_bridge.js`.
- **Failing command**: `node tests/e2e/webgpu_geodesic_slab_r4_61_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `R4-61 E2E FAILED: AssertionError [ERR_ASSERTION]: expected WGSL shader purity ok, got {"ok":false,"verdict":"RefusedFloatOpInWgsl","float_ops":1,"offenders":["holo_int4_gemv_gemm"]}`
- **Restored result**: Removed `var bad_float: f32 = 0;` from `crates/wasm/js/worker_host_bridge.js`; `node tests/e2e/webgpu_geodesic_slab_r4_61_e2e.mjs` exited `0` (`=== ALL R4-61 WEBGPU WGSL + GEODESIC SWIGLU + HOLOSLAB2 E2E CHECKS PASSED ===`).

### 8. HOLOSLAB2 Merkle Seal & Piece Integrity Gate (`W2-2`, `R4-61`)
- **Planted defect**: Replaced `seal_matches` verification with `let seal_matches = true;` inside `validate_holoslab2_mount` in `crates/wasm/src/holo_kappa_store.rs`.
- **Failing command**: `cargo test -p lexlean-r4-wasm --lib holoslab2`
  - **Exit code**: `101`
  - **Diagnostic**: `thread 'holo_kappa_store::tests::holoslab2_seal_enforces_quota_merkle_and_piece_hashes' panicked at crates/wasm/src/holo_kappa_store.rs:776:9: assertion failed: !bad_size.ok`
- **Restored result**: Restored full `seal_matches` validation expression in `crates/wasm/src/holo_kappa_store.rs`; `cargo test -p lexlean-r4-wasm --lib holoslab2` exited `0` (`test result: ok. 1 passed; 0 failed`).

## Wave 1 Redesign (`R4-62`): Compact Desktop Studio UI, Non-Blocking Dropdown Neural HUD & Zero-Prompt Auto-Hydration (`v0.14.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` = `11,514,089` bytes / `10.98 MiB` < `15 MiB`, `provenance.json`), `wasm=7795d62f624b`, `js=1adae764ebcb`, `core=89fa1186b92b`.
- **`build-holo --sync-site-manifest` / `--verify-site-manifest`**: verified Hologram v4 archive (`13,045,043` bytes <= `33,554,432` B / `32 MiB`, `blake3=54611d8807b6b2e6`, `wasmSha256=7795d62f624b1afc`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `62` conformance IDs (`R4-01..R4-62`, including `R4-62` / `R4.StudioFrame` catalog auto-hydration & floating HUD policy).
- **`lift-register`**: `total=2795`, `lifted=2016`, `effect=276`, `foreign=160`, `residue=343` (`< 344` incumbent, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=343`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `13` routes (including `R4-UR13` / `compact-studio-auto-hydration`) and `16` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=343`, `routes=13`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=987`, `complexity=343`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 9. Silent Sticky Q31 Downgrade Regression Gate (`R4-62`)
- **Planted defect**: Short-circuited `purgeLegacyDeclinedHydrationFlag()` in `crates/wasm/js/worker_host_bridge.js` to return `false` without removing `uor_allow_off_origin_hydration = 'declined'`.
- **Failing command**: `NODE_OPTIONS="--max-old-space-size=1024" node tests/e2e/foundry_worker_stepper_unit.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Stale declined flag must be purged` at `tests/e2e/foundry_worker_stepper_unit.mjs:1538`
- **Restored result**: Restored `safeStorageRemove('uor_allow_off_origin_hydration')` inside `purgeLegacyDeclinedHydrationFlag()`; `NODE_OPTIONS="--max-old-space-size=1024" node tests/e2e/foundry_worker_stepper_unit.mjs` exited `0` (`tests 12, pass 12, fail 0`).

## Wave 2 Redesign (`R4-63`): Single-Source MVM Linux `/workspace`, Unprompted Public CORS Preemption & Interactive Command Gates (`v0.14.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` `<= 12 MiB` < `15 MiB`, `provenance.json`), `wasm=58091ff3c397`, `js=af77f86c2840`, `core=4893c103fc2c`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,168,900` bytes <= `33,554,432` B / `32 MiB`, `blake3=c3465c6f93b88b86`, `wasmSha256=58091ff3c397eb76`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `63` conformance IDs (`R4-01..R4-63`, including `R4-63` / `R4.MvmLinuxContainer`).
- **`lift-register`**: `total=2831`, `lifted=2055`, `effect=277`, `foreign=157`, `residue=342` (`< 343` incumbent, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=342`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `14` routes (including `R4-UR14` / `mvm-linux-spiralcore-container`) and `17` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=342`, `routes=14`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=987`, `complexity=342`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 10. Credentialed CORS Proxy Refusal Gate (`R4-63`)
- **Planted defect**: Replaced `return 'RefusedCredentialedCorsProxy';` with `return 'StatelessRelayCorsPreempted';` inside `admit_public_cors_preemption()` in `crates/wasm/js/worker_host_bridge.js`.
- **Failing command**: `node tests/e2e/r4_63_mvm_container_cors_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: + 'StatelessRelayCorsPreempted' - 'RefusedCredentialedCorsProxy'` at `tests/e2e/r4_63_mvm_container_cors_e2e.mjs:44:10`
- **Restored result**: Restored `return 'RefusedCredentialedCorsProxy';` inside `admit_public_cors_preemption()`; `node tests/e2e/r4_63_mvm_container_cors_e2e.mjs` exited `0` (`[PASS] R4-63 single-source /workspace, self-hosted make+cc, and chat UI cleanliness verified`).

## Wave 3 Redesign (`R4-64`): True Resident WebGPU `<= 128 MiB` Per-Layer Serving, `0-LSB` `HoloTok13` Speculative Decoding & Studio LoRA Lockstep (`v0.14.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` `<= 12 MiB` < `15 MiB`, `provenance.json`), `wasm=d915ce8df957`, `js=1795af22d885`, `core=02afed6732f6`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,210,921` bytes <= `33,554,432` B / `32 MiB`, `blake3=02133f9a4f8cdef4`, `wasmSha256=d915ce8df957e4b6`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json` (`qwen3WeightsKappa = sha256:617ea77fd9c3d15285e351bd4f094cc0845cf5bc038e042a3fb5aadf0e406bda`).
- **Remote RTX 4090 Studio LoRA SFT + `UORR4V13` Deployment (`wick`)**:
  - `Qwen3-0.6B` (`origin/weights-v13-qwen3-0.6b` @ `5c003a3`): `container_sha256=617ea77fd9c3d15285e351bd4f094cc0845cf5bc038e042a3fb5aadf0e406bda`, `429,484,190` B (`410` pieces, `205` Stitch parity groups), `top1_parity_pct=100.00%`, `top5_parity_pct=100.00%`, `mean_kl_nats=0.027825` (`<= 0.05`).
  - `Qwen3-1.7B` (`origin/weights-v13-qwen3-1.7b` @ `9dc6d9e`): `container_sha256=2c362d73046851ff69dfa7e1de197d4f0fee14031e9e53e29bf278c41b6bc8bf`, `1,140,250,782` B (`1,088` pieces, `544` Stitch parity groups), `top1_parity_pct=91.67%`, `top5_parity_pct=100.00%`, `mean_kl_nats=0.033533` (`<= 0.05`).
  - Lockstep `STUDIO_SELF_KNOWLEDGE_PREFIX` shrunk to `34` Qwen3 BPE tokens (`< 40` tokens).
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `64` conformance IDs (`R4-01..R4-64`, including `R4-64` / `R4.WebGpuResidentRuntime`).
- **`lift-register`**: `total=2845`, `lifted=2068`, `effect=278`, `foreign=157`, `residue=342` (`< 343` baseline, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=342`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `15` routes (including `R4-UR15` / `webgpu-resident-speculative-lora`) and `18` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=342`, `routes=15`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=987`, `complexity=342`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 11. Unpinned Studio LoRA Prompt-Shrink Refusal Gate (`R4-64`)
- **Planted defect**: Replaced `if (!fineTunedManifestPinned) return 'RefusedPromptShrinkWithoutFineTunedManifest';` with `if (!fineTunedManifestPinned) return 'StudioLoraUorV13Admitted';` inside `admitStudioLoraPromptShrink()` in `crates/wasm/js/worker_host_bridge.js`.
- **Failing command**: `node tests/e2e/r4_64_webgpu_speculative_lora_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: + 'StudioLoraUorV13Admitted' - 'RefusedPromptShrinkWithoutFineTunedManifest'` at `tests/e2e/r4_64_webgpu_speculative_lora_e2e.mjs:186:10`
- **Restored result**: Restored `if (!fineTunedManifestPinned) return 'RefusedPromptShrinkWithoutFineTunedManifest';` inside `admitStudioLoraPromptShrink()`; `node tests/e2e/r4_64_webgpu_speculative_lora_e2e.mjs` exited `0` (`[PASS] R4-64 resident WebGPU <=128 MiB slabs, 28-layer 0-LSB speculative tiles, and Studio LoRA lockstep verified.`).

## Seamless Completion Wave (`R4-65`): Bound Live WebGPU Turn Shortlist Bridge, `COEP: credentialless` Cross-Origin Isolation & Adaptive 2-Tier `HoloTok13` Drafter (`v0.15.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` = `11,680,856` B < `15 MiB`, `provenance.json`), `wasm=bbd520a245e8`, `js=977c02e8a6d7`, `core=5b77b8621dca`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,231,401` bytes <= `33,554,432` B / `32 MiB`, `blake3=28f80513d5b63c86`, `wasmSha256=bbd520a245e820b1`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **GitHub Pages `COEP: credentialless` Service Worker (`site/coi-serviceworker.js`)**: injects `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: credentialless` (never `require-corp`) with single-reload guard (`sessionStorage['uor_coi_reloaded_v1']`) and deployed via `.github/workflows/pages.yml` post-`just vv` so `public/` preserves its exact 6-file closure.
- **API Readiness Race Closure (`xtask/src/engine.rs` + `m4_iter2_chal2.rs`)**: `api::default_model_backend()` warmed before `TcpListener::bind`, `1ms` nonblocking accept poll, and HTTP `200 /v1/traces` warmup probe (`43/43` `empirical_suite` green).
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `65` conformance IDs (`R4-01..R4-65`, including `R4-65` / `R4.LiveWebGpuTurnBridge`).
- **`lift-register`**: `total=2859`, `lifted=2081`, `residue=342` (`< 343` baseline, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=342`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `16` routes (including `R4-UR16` / `live-webgpu-coi-adaptive-drafter`) and `19` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=342`, `routes=16`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=988`, `complexity=342`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 12. Unbound WebGPU Compute Pass Refusal Gate (`R4-65`)
- **Planted defect**: Replaced `if (Number(boundDispatches) < Number(totalDispatches)) return 'RefusedUnboundComputePass';` with `if (Number(boundDispatches) < Number(totalDispatches)) return 'LiveGpuBoundStepAdmitted';` inside `admitLiveGpuBoundStep()` in `crates/wasm/js/worker_host_bridge.js`.
- **Failing command**: `node tests/e2e/r4_65_live_webgpu_coi_drafter_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: + 'LiveGpuBoundStepAdmitted' - 'RefusedUnboundComputePass'` at `tests/e2e/r4_65_live_webgpu_coi_drafter_e2e.mjs:46:10`
- **Restored result**: Restored `if (Number(boundDispatches) < Number(totalDispatches)) return 'RefusedUnboundComputePass';` inside `admitLiveGpuBoundStep()`; `node tests/e2e/r4_65_live_webgpu_coi_drafter_e2e.mjs` exited `0` (`[PASS] R4-65 bound WebGPU turn bridge, COEP credentialless SW, and adaptive drafter verified.`).

## Seamless Completion Wave (`R4-66`): `<= 48 MiB` Streaming Pack Hydration, Immediate `404` Mirror Pruning & Single-Truth UI (`v0.15.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` = `11,680,856` B < `15 MiB`, `provenance.json`), `wasm=bbd520a245e8`, `js=a07c1d7700a0`, `core=5fe28acbe2bb`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,239,142` bytes <= `33,554,432` B / `32 MiB`, `blake3=9465af4a5a8d0e0e`, `wasmSha256=bbd520a245e820b1`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **Deployed `<= 48 MiB` (`50,331,648` B) Streaming Weight Packs**:
  - `origin/weights-v13-qwen3-0.6b` (`081cb4f`): `9` packs (`pack_00.pack..pack_08.pack`, max `50,331,648` B) covering all `410` verified pieces with `container_sha256` preserved (`617ea77fd9c3...`).
  - `origin/weights-v13-qwen3-1.7b` (`b35a6dc`): `23` packs (`pack_00.pack..pack_22.pack`, max `50,331,648` B) covering all `1,088` verified pieces with `container_sha256` preserved (`2c362d730468...`).
- **Immediate `404`/`403` Mirror Pruning & Single-Truth Badge**: `OpfsKappaStore.missingBlobMirrors` + `sessionPackMissingMirrors` prune missing-blob mirrors on the first probe; `admitSingleTruthBadgeUi` prevents `Offline Q31 · Emergency` from rendering when a `Qwen3` model is active while preserving legacy IDs alongside `data-studio-id` aliases (`browser_closure` `10/10`, `empirical_suite` `99/99`).
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `66` conformance IDs (`R4-01..R4-66`, including `R4-66` / `R4.FastPackHydration`).
- **`lift-register`**: `total=2867`, `lifted=2088`, `residue=342` (`< 343` baseline, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=342`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `17` routes (including `R4-UR17` / `fast-pack-single-truth-hydration`) and `20` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=342`, `routes=17`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=988`, `complexity=342`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 13. Contradictory Emergency Q31 Badge Refusal Gate (`R4-66`)
- **Planted defect**: Replaced `return 'RefusedEmergencyBadgeOnQwenSession';` with `return 'SingleTruthBadgeAdmitted';` inside `admitSingleTruthBadge()` in `crates/wasm/js/worker_host_bridge.js` / `public/lexlean_r4.js`.
- **Failing command**: `node tests/e2e/r4_66_fast_pack_single_truth_ui_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: + 'SingleTruthBadgeAdmitted' - 'RefusedEmergencyBadgeOnQwenSession'` at `tests/e2e/r4_66_fast_pack_single_truth_ui_e2e.mjs:40:8`
- **Restored result**: Restored `return 'RefusedEmergencyBadgeOnQwenSession';` inside `admitSingleTruthBadge()`; `node tests/e2e/r4_66_fast_pack_single_truth_ui_e2e.mjs` exited `0` (`[PASS] R4-66 <=48 MiB streaming packs, 404 mirror pruning, and single-truth UI verified.`).

## Seamless Completion Wave (`R4-67`): MVM Polyglot Toolchain Completion (`cargo`/`rustc`, `npm` Lifecycle & C99 `<stdlib.h>`/`<stdbool.h>`, `v0.15.0`)

- **Browser Wasm closure (`public/`)**: 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm` < `15 MiB`, `provenance.json`), `wasm=69b5bc0b4ce7`, `js=90e621d303c8`, `core=9d7962390730`.
- **`build-holo --sync-site-manifest`**: verified Hologram v4 archive (`13,257,917` bytes <= `33,554,432` B / `32 MiB`, `blake3=8bc967bdf4759e08`, `wasmSha256=69b5bc0b4ce7f6e3`, `vectors=5/5`, `view_files=6`) synchronized in `site/app.json`.
- **Polyglot MVM Toolchain**:
  - In-guest `cargo` (`check`, `test`, `clippy`, `build`, `run`, `--version`) & `rustc` wired directly to `wasm_ast_eval::execute_workspace_cargo` over `/workspace/**/*.rs` with canonical-path protection.
  - `npm` (`init`, `install`, `run`, `test`, `start`, `build`, `ls`, `uninstall`, `--version`) in `SpiralCoreHostBridge.handleNpmBridge`.
  - Extended C99 `<stdlib.h>` (`malloc`, `calloc`, `free` via 8-byte aligned `SYS_brk = 214`, `atoi`, `abs`) & `<stdbool.h>` (`bool`, `true`, `false`) compiled natively to RV64 ELF64 (`EM_RISCV = 243`).
- **`check-model`**: `CONFORMANCE.md` matches `model/`, `67` conformance IDs (`R4-01..R4-67`, including `R4-67` / `R4.MvmPolyglotToolchain`).
- **`lift-register`**: `total=2874`, `lifted=2095`, `residue=341` (`< 342` prior wave, `<= 355` threshold, `UE-16` monotone).
- **`ratchet-check`**: `r30_gen=261`, `orphans=0`, `ungrounded=0`, `ignored=0`, `residue=341`, `float=0`, `matmul=0`, `unwired_ui=0`.
- **`check-routes`**: `18` routes (including `R4-UR18` / `mvm-polyglot-cargo-npm-c99`) and `21` test vectors verified under `uorust/routes/1`.
- **`check-measures`**: `6` measures admitted under `prism/measures/1` (`uncovered=0`, `undeclared=0`, `float_ops=0`, `matmul_ops=0`, `kappa_residue=341`, `routes=18`, `provenance_sources=24`).
- **`synthesize`**: `score_milli=988`, `complexity=341`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`.

### 14. Unsupported Polyglot Toolchain Subcommand Refusal Gate (`R4-67`)
- **Planted defect**: Replaced `return 'RefusedUnsupportedToolchainSubcommand';` with `return 'NpmSubcommandAdmitted';` inside `admitCargoTerminalSubcommand()` / `admitNpmTerminalSubcommand()` in `crates/wasm/js/worker_host_bridge.js` / `public/lexlean_r4.js`.
- **Failing command**: `node tests/e2e/r4_67_mvm_polyglot_toolchain_e2e.mjs`
  - **Exit code**: `1`
  - **Diagnostic**: `AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: + 'NpmSubcommandAdmitted' - 'RefusedUnsupportedToolchainSubcommand'` at `tests/e2e/r4_67_mvm_polyglot_toolchain_e2e.mjs:24:8`
- **Restored result**: Restored `return 'RefusedUnsupportedToolchainSubcommand';` inside `admitCargoTerminalSubcommand()` / `admitNpmTerminalSubcommand()`; `node tests/e2e/r4_67_mvm_polyglot_toolchain_e2e.mjs` exited `0` (`[PASS] R4-67 MVM cargo/rustc, npm lifecycle subcommands, and C99 <stdlib.h>/<stdbool.h> verified.`).
