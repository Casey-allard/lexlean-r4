# LexLean-R4 Seamless Completion Roadmap (`R4-65`–`R4-67`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task inline in the main session with 100% remote builds/tests on `wick` (zero subagent sprawl, zero local Mac compiles). Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all 7 remaining post-Wave-3 engineering, performance, cold-start, toolchain, CI-stability, and UX gaps across three formal capabilities (`R4-65`, `R4-66`, `R4-67`) without breaking any existing `R4-01..R4-64` guarantee or exhausting local MacBook RAM/disk.

**Architecture:** Wave 4 (`R4-65`) serializes concurrent 200-socket loopback flood tests to eliminate 4-vCPU CI SYN-backlog flakes, connects `WebGpuHoloEngine` with bound `GPUBindGroup`s and `<= 128 MiB` tensor-offset layer slabs directly into live Worker turn stepping, unlocks `SharedArrayBuffer` multi-worker Wasm on GitHub Pages via a `COEP: credentialless` Service Worker in `site/`, and adds a 2-tier structural + adaptive speculative drafter. Wave 5 (`R4-66`) bundles `UORR4V13` weights into `<= 48 MiB` streaming packs on the orphan weights branches, blacklists HTTP-404 piece mirrors after 1 probe, and replaces silent Turn-1 `NativeQ31` downgrades with live progress queuing and single-truth header badges. Wave 6 (`R4-67`) adds in-MVM `cargo` (`check`/`test`/`clippy`/`build`), complete `npm` lifecycle subcommands (`test`/`start`/`build`/`ls`/`uninstall`), and C99 `<stdint.h>`/`<stdbool.h>`/`<string.h>` support in `mvm_cc`.

**Tech Stack:** LexLean `0.3.0` (`lexlean/semantic-module/2`), Rust (`wasm32-unknown-unknown +simd128`), WebGPU pure-integer WGSL (`i32`/`u32`/`vec2<u32>`), Service Worker (`COOP: same-origin` + `COEP: credentialless`), `HOLOSLAB2` OPFS `SyncAccessHandle`, `RV64IMAC` ELF64 MicroVM, remote SSH execution on `wick` (`RTX 4090`, 32 cores, 59 GiB RAM).

**Spec:** `docs/superpowers/specs/2026-11-19-lexlean-r4-mvm-studio-redesign-design.md` (and post-Wave-3 gap audit).

## Global Constraints

- **Zero Local Mac Memory/Disk Bloat:** NEVER run `cargo`, `rustc`, `wasm-bindgen`, `docker`, or `node tests/e2e/*` on `/Users/casey.allard/lexlean-r4`; NEVER create a local `target/` directory; NEVER spawn `sdd-implementer`/`sdd-reviewer`/`research` subagents (use at most 1 `browser` subagent with `Model: "flash"` for the final live GitHub Pages check). Run 100% of builds, tests, pack generation, and locked-SDK Docker `just vv` over SSH on `wick:/tmp/lexlean-r4-ci` (`CARGO_TARGET_DIR=/tmp/lexlean-r4-sdk-target`).
- **Pure-Integer Serving Invariant (`R4-44`, `R4-51`, `R4-61`, `R4-64`):** `float_ops = 0` and `matmul_ops = 0` across all Rust/Wasm serving paths and all WGSL shaders (`Quality::measureFloatServingOps <= 0`, `Quality::measureDenseMatmulOps <= 0`).
- **6-File `public/` Browser Closure (`R4-48`):** During `just vv`, `public/` MUST contain the exact 6 files (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm <= 13.5 MiB`, `provenance.json`) with zero URL literals, `fetch(`, `parseFloat(`, or `parseInt(` in `public/app.js`, `public/index.html`, or `public/app.css`. Deployment-time static companions (`app.json`, `coi-serviceworker.js`, `icons/`) live in `site/` and are copied into `public/` inside `.github/workflows/pages.yml` after `just vv`.
- **Legacy DOM ID Preservation:** Preserve `id="claude-chat-input"`, `id="toggle-inspectors-btn"`, `id="inspectors-modal"`, and `id="settings-offline-q31-toggle"` for existing conformance/E2E suites while adding canonical `data-studio-id` aliases and `unwired_ui = 0`.
- **Byte-Bound Universal Policy Files (`template.lock`):** Never modify `.devcontainer/devcontainer.json`, `.github/workflows/bootstrap.yml`, `AGENTS.md`, `VERIFICATION.md`, `prismpm.lock`, or `template-contract.json`.
- **Shrink-Only Ratchet (`UE-16`):** `kappa_residue <= 342`, `score_milli >= 987`, `defects = 0`, zero `.py` files or Python CLI strings under `gpu/`, zero deferral markers (`TODO`, `FIXME`, `XXX`, `unimplemented!`, `todo!`).

## Review Focus

1. **COEP Mode Breaking Cross-Origin Shards/GitHub API:** Setting `Cross-Origin-Embedder-Policy: require-corp` in `site/coi-serviceworker.js` would block cross-origin `raw.githubusercontent.com` weight packs and `api.github.com` calls; the Service Worker MUST emit `Cross-Origin-Embedder-Policy: credentialless` and guard against infinite reload loops via `sessionStorage` (`uor_coi_reloaded_once`).
2. **Same-Origin `manifest.json` Without `.bin` Causing 404 Storms:** Because `pages.yml` copies `gpu/weights/**/manifest.json` to `public/weights/` while deleting `*.bin`/`*.pack`, `hydrate_holo_model` MUST blacklist any mirror base for binary fetches on the very first HTTP `404`/`403` response (`missingBlobMirrors.add(mBase)`) so pieces `1..N` go straight to the live mirror.
3. **Unbound WebGPU Compute Pass Validation Errors on Real Browsers:** Calling `pass.dispatchWorkgroups(...)` in `WebGpuHoloEngine` without calling `pass.setBindGroup(0, ...)` for every `@group(0)` binding declared by the active WGSL pipeline causes a WebGPU validation error on Chrome/Edge; every GPU dispatch MUST bind a concrete `GPUBindGroup` matching its shader layout.
4. **CPU Latency Regression When Speculative Drafts Miss:** Evaluating `bt = 5` tokens on single-core CPU when novel prose misses at draft slot 0 wastes 4 token evaluations; `HoloTok13SpeculativeDrafter` MUST track rolling acceptance (`accept_ema_permille`) and throttle CPU `max_drafts` to `0..=1` on repeated misses while allowing `K = 4` on WebGPU or high-acceptance code blocks.
5. **Corrupt or Truncated Pack Slice During Streaming Pack Hydration:** If a `pack_XX.pack` stream drops mid-transfer or fails per-piece SHA-256 verification, `hydrate_holo_model` MUST keep already-verified earlier pieces from that pack and seamlessly fall back to per-piece fetch + Stitch 2-of-3 XOR parity recovery for the remaining pieces without aborting the turn.

---

### Task 1: Deterministic Loopback Flood Serialization (`Gap #6`) & Formal `R4-65` Model (`LiveWebGpuTurnBridge`)

**Files:**
- Modify: `xtask/tests/empirical_suite.rs` (or shared helper in `xtask/tests/empirical_suite/m3_api.rs`, `m4_provenance_concurrency.rs`, `m4_iter2_chal2.rs`)
- Create: `src/R4/LiveWebGpuTurnBridge.lex.tex`
- Create: `tests/lexlean/LiveWebGpuTurnBridgeControls.lex.tex`
- Modify: `lexlean.toml`, `model/ids.toml`, `model/routes.toml`, `model/measures.toml`, `crates/conformance/tests/bdd.rs`, `crates/wasm/src/lexlean_generated.rs`, `crates/wasm/tests/overhaul_e2e/{tier1_feature.rs,tier3_combinations.rs,tier4_scenarios.rs}`
- Create: `features/suites/r4_65_live_webgpu_turn_bridge.feature`
- Create: `crates/conformance/tests/r4_65_live_webgpu_turn_bridge.rs`

**Interfaces:**
- Produces:
  - `pub fn acquire_loopback_hammer_guard() -> std::sync::MutexGuard<'static, ()>` in `xtask/tests/empirical_suite.rs`
  - `R4.LiveWebGpuTurnBridge` (`admitLiveGpuBoundStep`, `admitCoiCredentiallessMode`, `admitAdaptiveSpeculativeBudget`, `verifyLiveWebGpuTurnBridgeAdmitted`) and Rust mirror `crates/wasm/src/lexlean_generated.rs::r4_live_webgpu_turn_bridge`

- [ ] **Step 1: Add `acquire_loopback_hammer_guard()` to `xtask/tests/empirical_suite.rs` and acquire it in the 200-worker TCP flood tests**
  Acquire `let _hammer_guard = crate::acquire_loopback_hammer_guard();` at the start of `m3_api::challenge_11_live_daemon_concurrency_hammer_200_requests`, `m4_provenance_concurrency::test_challenge_5_concurrent_api_flooding_socket_stability`, and `m4_iter2_chal2::test_stress_api_concurrency_hammer_and_client_disconnects` (preserving all 200 concurrent workers within each test while preventing simultaneous multi-daemon SYN-backlog saturation).

- [ ] **Step 2: Author `src/R4/LiveWebGpuTurnBridge.lex.tex`, `tests/lexlean/LiveWebGpuTurnBridgeControls.lex.tex`, `features/suites/r4_65_live_webgpu_turn_bridge.feature`, and failing `crates/conformance/tests/r4_65_live_webgpu_turn_bridge.rs`**
  Model:
  - `LiveGpuStepVerdict` (`LiveGpuBoundStepAdmitted`, `FallbackWasmSimdAdmitted`, `RefusedFloatOpInGpuBridge`, `RefusedUnboundComputePass`, `RefusedGpuCpuLogitDrift`)
  - `CoiIsolationVerdict` (`CrossOriginCredentiallessAdmitted`, `SingleWorkerFallbackAdmitted`, `RefusedRequireCorpBreaksMirrors`)
  - `AdaptiveDrafterVerdict` (`AdaptiveTileAdmitted`, `ThrottledCpuDraftsAdmitted`, `RefusedUnthrottledCpuMissStorm`)

- [ ] **Step 3: Verify `lexlean check` + `lexlean verify` inside locked SDK container on `wick`, wire `r4_live_webgpu_turn_bridge` + `R4-65` (`R4-UR16`), and run `just model-write` + `cargo test -p repo-conformance --test r4_65_live_webgpu_turn_bridge` + `cargo test -p xtask --test empirical_suite` on `wick`**
  Expected: `verified 2 modules`, `CONFORMANCE.md equals the model, 65 ids`, `16 routes`, `empirical_suite` `43 passed; 0 failed`.

- [ ] **Step 4: Commit Task 1**
  Commit message: `feat(r4-65): model LiveWebGpuTurnBridge and serialize 200-worker loopback flood tests`

---

### Task 2: Bound Live WebGPU Turn Stepping, `site/coi-serviceworker.js` (`COEP: credentialless`), and 2-Tier Adaptive `HoloTok13` Drafter (`Gaps #1, #3, #4`)

**Files:**
- Modify: `crates/wasm/src/holo_geometric_runtime.rs`
- Modify: `crates/wasm/src/assistant.rs`
- Modify: `crates/wasm/src/lib.rs`
- Modify: `crates/wasm/js/worker_host_bridge.js`
- Create: `site/coi-serviceworker.js`
- Modify: `.github/workflows/pages.yml`
- Create: `tests/e2e/r4_65_live_webgpu_coi_drafter_e2e.mjs`

**Interfaces:**
- Consumes: `r4_live_webgpu_turn_bridge` from Task 1.
- Produces:
  - `HoloTok13SpeculativeDrafter::propose_adaptive_draft_tokens(..., gpu_active: bool, accept_ema_permille: u16) -> Vec<u32>` (Tier-1 3/2-gram suffix + Tier-2 Studio/code structural BPE continuations + CPU miss throttle when `!gpu_active && accept_ema_permille < 250`).
  - `#[wasm_bindgen] pub fn studio_export_holo_vocab_sigs_u32(model_id: &str) -> Vec<u32>` and `#[wasm_bindgen] pub fn studio_export_holo_layer_slab_bytes(model_id: &str, layer_idx: u32) -> Vec<u8>` (`UORLSLB1` header + 16B-aligned tensors `<= 128 MiB`).
  - `#[wasm_bindgen] pub fn holo_peek_foundry_turn_qsig512(turn_id: u64) -> Vec<u32>` and `#[wasm_bindgen] pub fn holo_step_foundry_turn_gpu_shortlist(turn_id: u64, shortlist_ids: &[u32]) -> String`.
  - `WebGpuHoloEngine.mountResidentLayerTensors(...)` + `WebGpuHoloEngine.stepLiveTurnShortlistAsync(qSig512U32, shortlistK)` (creates real `GPUBindGroup`s for every compute pass, verifies `bound_dispatches === total_dispatches`, `mapAsync` reads distances/shortlist, feeds `holo_step_foundry_turn_gpu_shortlist`).
  - `site/coi-serviceworker.js` (`COOP: same-origin`, `COEP: credentialless`, one-shot reload guard) + `registerStudioCoiServiceWorker()` in `worker_host_bridge.js` + `cp site/coi-serviceworker.js public/coi-serviceworker.js` in `.github/workflows/pages.yml`.

- [ ] **Step 1: Write failing Rust unit tests in `holo_geometric_runtime.rs` and failing Node E2E `tests/e2e/r4_65_live_webgpu_coi_drafter_e2e.mjs`**
  Test:
  1. Tier-2 structural drafter proposes `<prism-write` / `pub fn` continuations on Turn 1 (`recent` without prior repetition) and throttles `max_drafts` to `<= 1` when `!gpu_active && accept_ema_permille < 250`.
  2. `WebGpuHoloEngine` rejects any unbound compute pass (`RefusedUnboundComputePass`), binds real `GPUBindGroup`s on every dispatch, and drives `execute_inprism_foundry_turn` via `holo_step_foundry_turn_gpu_shortlist` with `0-LSB` token parity against CPU Wasm `simd128`.
  3. `site/coi-serviceworker.js` emits `Cross-Origin-Embedder-Policy: credentialless` (never `require-corp`) and guards single reload via `sessionStorage`.

- [ ] **Step 2: Implement adaptive 2-tier drafter + GPU shortlist/layer exports in Rust (`holo_geometric_runtime.rs`, `assistant.rs`, `lib.rs`), bound `WebGpuHoloEngine` live turn integration + COI registration in `worker_host_bridge.js`, and `site/coi-serviceworker.js` + `pages.yml` copy**

- [ ] **Step 3: Rebuild Wasm (`xtask build-wasm` + `build-holo --sync-site-manifest`), plant Defect #12 (`RefusedUnboundComputePass` bypass), verify failure & restore in `OVERHAUL-VERIFICATION.md`, and run Rust + Node E2E tests on `wick`**
  Expected: `r4_65_live_webgpu_coi_drafter_e2e.mjs` PASS, `float_ops = 0`, `matmul_ops = 0`, exact 6 files in `public/`.

- [ ] **Step 4: Commit Task 2**
  Commit message: `feat(r4-65): wire bound WebGPU turn shortlist stepping, COEP credentialless SW, and adaptive 2-tier drafter`

---

### Task 3: `<= 48 MiB` Streaming Pack Hydration, 404-Mirror Blacklisting, Queue-Not-Downgrade First Turn & Single-Truth Header UI (`R4-66`, `Gaps #2 & #7`)

**Files:**
- Create: `src/R4/FastPackHydration.lex.tex`
- Create: `tests/lexlean/FastPackHydrationControls.lex.tex`
- Create: `features/suites/r4_66_fast_pack_hydration.feature`
- Create: `crates/conformance/tests/r4_66_fast_pack_hydration.rs`
- Modify: `lexlean.toml`, `model/ids.toml`, `model/routes.toml`, `model/measures.toml`, `crates/conformance/tests/bdd.rs`, `crates/wasm/src/lexlean_generated.rs`, `crates/wasm/tests/overhaul_e2e/{tier1_feature.rs,tier3_combinations.rs,tier4_scenarios.rs}`
- Modify: `gpu/weights/v13_qwen3_0_6b/manifest.json`, `gpu/weights/v13_qwen3_1_7b/manifest.json` (plus push `pack_XX.pack <= 48 MiB` to `origin/weights-v13-qwen3-{0.6b,1.7b}` from `wick` with zero `.pack` bytes stored on Mac!)
- Modify: `crates/wasm/js/worker_host_bridge.js`
- Modify: `public/index.html`, `public/app.js`
- Create: `tests/e2e/r4_66_fast_pack_single_truth_ui_e2e.mjs`

**Interfaces:**
- Produces:
  - `R4.FastPackHydration` (`admitPackManifest`, `admitMirror404Pruning`, `admitTurnHydrationQueue`, `verifyFastPackHydrationAdmitted`).
  - `manifest.json` `packs: [{ index, file, byte_offset, byte_length, piece_start, piece_count, sha256 }]` (`byte_length <= 50_331_648` / `48 MiB`) on `weights-v13-qwen3-0.6b` (`9` packs) and `weights-v13-qwen3-1.7b` (`23` packs), preserving `container_sha256` and `pieces[]` unchanged.
  - Streaming `ReadableStream` pack reader in `hydrate_holo_model` + immediate `missingBlobMirrors.add(mBase)` on first HTTP `404`/`403` + `CONCURRENCY = 12` piece fallback + Stitch 2-of-3 mid-pack recovery.
  - Queue-not-downgrade Turn-1 behavior (`bootNativeQ31Fallback` stays `false` unless `uor_offline_emergency_q31 === '1'` or user explicitly selects Offline Emergency Q31) + single-truth `#weight-cache-badge` + canonical `data-studio-id` aliases (`chat-input`, `hud-dropdown-toggle`, `neural-hud-dropdown`).

- [ ] **Step 1: Author `R4-66` LexLean model & Controls, verify inside locked SDK on `wick`, and write failing `r4_66_fast_pack_single_truth_ui_e2e.mjs`**
  Test:
  1. Same-origin mirror returning HTTP `404` on `pack_00.pack` / `piece_0000.bin` is probed **once** (`<= 1` 404 fetch, not `410`) before switching all remaining fetches to the live mirror.
  2. Streaming `pack_00.pack` slices pieces in-flight, verifies SHA-256 per piece, and if `pack_01.pack` corrupts mid-stream, keeps verified pieces and recovers the corrupt piece via `pieces[]` / Stitch 2-of-3 XOR parity.
  3. Sending a prompt while `uor-r4.qwen3-0.6b-holo` is hydrating awaits hydration and answers with `uor-r4.qwen3-0.6b-holo` (never silently downgrading to `uor-r4.native-q31-d85` unless Offline Emergency Q31 is explicitly enabled), and `#weight-cache-badge` never displays `Offline Q31 · Emergency` while Qwen3 is selected and online.

- [ ] **Step 2: Build `<= 48 MiB` `pack_XX.pack` files on `wick` from existing verified pieces on `origin/weights-v13-qwen3-{0.6b,1.7b}`, push to GitHub orphan branches (`*.pack` excluded from `public/weights` in `pages.yml`), pull updated `manifest.json`s, and clean `/var/tmp` on `wick`**

- [ ] **Step 3: Implement streaming pack hydration, 404-mirror pruning, queue-not-downgrade turn routing, single-truth `#weight-cache-badge`, and canonical `data-studio-id` aliases (`worker_host_bridge.js`, `public/index.html`, `public/app.js`, `pages.yml`)**

- [ ] **Step 4: Rebuild Wasm/Holo on `wick`, plant Defect #13 (`RefusedPiece404MirrorRetry` / `RefusedSilentEmergencyDowngrade`), record in `OVERHAUL-VERIFICATION.md`, and pass all E2E suites**

- [ ] **Step 5: Commit Task 3**
  Commit message: `feat(r4-66): add <=48 MiB streaming pack hydration, 404 mirror pruning, and single-truth model UX`

---

### Task 4: MVM Polyglot Toolchain Completion — In-MVM `cargo`, Full `npm` Lifecycle & Extended C99 `mvm_cc` (`R4-67`, `Gap #5`) + Final CI & Live Verification

**Files:**
- Create: `src/R4/MvmPolyglotToolchain.lex.tex`
- Create: `tests/lexlean/MvmPolyglotToolchainControls.lex.tex`
- Create: `features/suites/r4_67_mvm_polyglot_toolchain.feature`
- Create: `crates/conformance/tests/r4_67_mvm_polyglot_toolchain.rs`
- Modify: `lexlean.toml`, `model/ids.toml`, `model/routes.toml`, `model/measures.toml`, `crates/conformance/tests/bdd.rs`, `crates/wasm/src/lexlean_generated.rs`, `crates/wasm/tests/overhaul_e2e/{tier1_feature.rs,tier3_combinations.rs,tier4_scenarios.rs}`
- Modify: `crates/wasm/src/mvm.rs`
- Modify: `crates/wasm/src/mvm_cc.rs`
- Modify: `crates/wasm/js/worker_host_bridge.js`
- Create: `tests/e2e/r4_67_mvm_polyglot_toolchain_e2e.mjs`

**Interfaces:**
- Produces:
  - `R4.MvmPolyglotToolchain` (`admitCargoWorkspaceCommand`, `admitNpmLifecycleSubcommand`, `admitC99StdlibProgram`, `verifyMvmPolyglotToolchainAdmitted`).
  - `MvmGuest::exec_cargo_command` (`cargo check`, `cargo test [filter]`, `cargo clippy`, `cargo build`, `cargo --version`, `rustc --version`) evaluating live `/workspace/Cargo.toml` + `/workspace/src/**/*.rs` via `wasm_ast_eval` with real syntax/item/test diagnostics and exit codes (`0` / `101`).
  - Extended `handleNpmBridge` (`npm test`/`t`, `npm start`, `npm build`, `npm ls`/`list`, `npm uninstall`/`rm`/`remove`) over `/workspace/package.json` + `/workspace/node_modules`.
  - Extended `mvm_cc` C99 support (`<stdint.h>`, `<stdbool.h>`, `<stddef.h>`, `<string.h>`, `typedef` fixed-width integers + `bool`, built-in `strlen`/`strcmp`/`strncmp`/`memcpy`/`memset`) compiling directly to `RV64IMAC` ELF64 (`EM_RISCV = 243`).

- [ ] **Step 1: Author `R4-67` LexLean model & Controls, verify inside locked SDK on `wick`, and write failing Rust unit tests + `tests/e2e/r4_67_mvm_polyglot_toolchain_e2e.mjs`**
  Test:
  1. `cargo check` and `cargo test` inside MVM Terminal succeed (`exit_code == 0`) on valid `/workspace/src/lib.rs` and fail (`exit_code == 101`) when a syntax/assertion error is written into `/workspace/src/lib.rs`.
  2. `npm init`, `npm test`, `npm ls`, and `npm uninstall` succeed cleanly in `/workspace`.
  3. A C99 program in `/workspace/src/vec.c` using `#include <stdint.h>`, `#include <stdbool.h>`, `#include <string.h>`, `uint32_t`, `bool`, `strlen`, and `memcpy` compiles via `cc /workspace/src/vec.c -o /workspace/vec` into a genuine `EM_RISCV` ELF64 binary and prints expected output on `RV64IMAC`.

- [ ] **Step 2: Implement `exec_cargo_command` in `mvm.rs`, C99 headers/intrinsics/`typedef`s in `mvm_cc.rs`, and full `npm` subcommands in `worker_host_bridge.js`**

- [ ] **Step 3: Rebuild Wasm/Holo (`build-wasm`, `build-holo --sync-site-manifest`, `lift-register --write`, `synthesize --write`, `ratchet-check`), plant Defect #14 (`RefusedCargoBrokenSyntaxPass`), record in `OVERHAUL-VERIFICATION.md`, and run FULL locked-SDK Docker `just vv` on `wick`**
  Expected: `vv: the acceptance gate passed` (`67` IDs, `18` routes, `float_ops = 0`, `matmul_ops = 0`, `residue <= 342`, `score_milli >= 987`).

- [ ] **Step 4: Merge to `main`, push to `origin/main`, verify `pages.yml` + `bootstrap.yml` + `deploy watch` all green, host-verify live GitHub Pages with 1 `browser` subagent (`Model: "flash"`), and clean `/tmp/lexlean-r4-*` on `wick`**
