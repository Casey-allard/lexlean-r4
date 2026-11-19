# LexLean-R4 Studio, MVM Linux & SpiralCore Overhaul — Design Specification

**Date:** 2026-11-19  
**Target Live URL:** `https://casey-allard.github.io/lexlean-r4/`  
**Repository:** `/Users/casey.allard/lexlean-r4` (`uor/template-contract/1`)  
**Status:** Ready for User Approval

---

## 1. Intent & Verified Root-Cause Diagnosis

### Why We Are Redesigning
LexLean-R4 is meant to operate as a self-hosted browser IDE, autonomous agent studio, and verifiable pure-integer inference workbench—comparable to a web-hosted Cursor or ChatGPT container workspace—backed by an isolated in-browser Linux MicroVM (`MVM-Wasm` over `RV64IMAC`), `SpiralCore` networking, live neural observability, and local WebGPU-accelerated `UORR4V13` weights.

An end-to-end codebase and CI audit surfaced five concrete regressions and architectural bottlenecks that block that experience today:

1. **CI Queue Contention & 37–90 Minute Deploy Delays:**
   - [`.github/workflows/engine.yml`](file:///Users/casey.allard/lexlean-r4/.github/workflows/engine.yml) runs on a `17,47 * * * *` cron schedule with a 90-minute timeout (`44553321781` failed at `1h30m21s`) and pushes to `main`, queueing repeated Pages builds.
   - [`.github/workflows/pages.yml`](file:///Users/casey.allard/lexlean-r4/.github/workflows/pages.yml) sets `concurrency: { group: 'pages', cancel-in-progress: false }` across the entire workflow, forcing newer pushes to sit behind stale 37-minute builds (`44553321802` took `36m 31s` build + `18s` deploy).
   - `pages.yml` keys its Cargo cache on `r4-pages-target-${{ hashFiles('Cargo.lock') }}`. Because `Cargo.lock` rarely changes while Rust sources and `crates/wasm/src/bootstrap_v8.uorlut` (`40.3 MB`) change frequently, GitHub Actions hits the frozen primary cache key (`not saving cache`) and recompiles stale targets every run.
   - [`Cargo.toml`](file:///Users/casey.allard/lexlean-r4/Cargo.toml) configures `[profile.dev]` (`debug = false`, default `opt-level = 0`) vs `[profile.test]` (`opt-level = 2`, `debug = false`), causing `just vv` (`clippy --all-targets`, `cargo test`, `cargo check --all-targets`) to compile third-party crates twice (`4m 24s` extra in CI).
   - `xtask/tests/empirical_challenge_*.rs` (`m2`, `m2_iter2`, `m3_api`, `m4_iter2_chal2`, `m4_provenance_concurrency`) include `#[path = "../src/engine.rs"] mod engine;`, compiling `engine.rs` + `lut_stack_exec.rs` + the `40.3 MB` `.uorlut` five times and executing `engine::tests` (~100 tests) five duplicate times (`~6m` wasted).
   - `crates/wasm/tests/` compiles ~25 separate integration test binaries (each linking an ~80 MB executable and causing local Mac SSD/RAM bloat when running `--workspace`), and `LutStackModel::parse` (`crates/wasm/src/lut_stack_exec.rs`) is invoked repeatedly without process-wide `OnceLock` sharing in `api.rs` (`EMBEDDED_BOOTSTRAP_UORLUT`) and test helpers (`~12–14m` wasted).

2. **Broken First-Message Auto-Hydration & Confusing Model Picker:**
   - [`public/index.html`](file:///Users/casey.allard/lexlean-r4/public/index.html) defaults `#model-select` to `uor-r4.qwen3-0.6b-holo` (`Qwen3-0.6B Holo v13 (28L 1024D OPFS Int4)`) right beside `#weight-cache-badge` displaying `Native Q31 · Ready`, giving outsiders zero explanation of codenames or download size.
   - [`crates/wasm/js/worker_host_bridge.js`](file:///Users/casey.allard/lexlean-r4/crates/wasm/js/worker_host_bridge.js) classifies our own repo's weight branch (`https://raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b/` and `weights-v13-qwen3-1.7b/`) as unapproved off-origin, raising `ConsentRequiredForWeightDownload` instead of hydrating automatically.
   - Once `uor_allow_off_origin_hydration === 'declined'` is stored in `localStorage`, [`crates/wasm/js/worker_host_bridge.js`](file:///Users/casey.allard/lexlean-r4/crates/wasm/js/worker_host_bridge.js) silently hijacks turns to `uor-r4.native-q31-d85` even while Qwen3 remains selected in the dropdown. Meanwhile, stale IndexedDB entries (`uor_holo_kappa_v1`) plus `CONCURRENCY = 6` parallel raw GitHub fetches trigger `RefusedInsufficientParityShards: failed piece 0`.
   - Unused internal model choices (`uor-r4.native-q31-d85`, `uor-r4.hf-repo-holo`) still clutter `#model-select` and `#composer-model-select`.

3. **Oversized Controls, Chrome Overlap & Blackout Modal HUD:**
   - [`public/app.css`](file:///Users/casey.allard/lexlean-r4/public/app.css) sets `button { min-height: 36px }` globally and expands buttons, tabs, inputs, and `.cursor-mini-btn` to `44×44px` under `@media (pointer: coarse)`—which matches touch-capable Windows/Mac laptops and blows past `28px` status/breadcrumb bars, wrapping toolbars and pushing usable space off screen.
   - Studio stacks 10 separate horizontal chrome bars inside the viewport and docks `#observatory-dock-split` across `42%` of the chat column height.
   - Clicking `Observatory` opens `#inspectors-modal` with a dark `rgba(5, 8, 15, 0.72)` `.inspectors-modal-backdrop`, blacking out the chat stream, token output, and editor while the user wants to watch live stats change during inference.

4. **Split-Brain `/workspace`, JS Fake Terminal & Missing SpiralCore Networking Loop:**
   - [`crates/wasm/src/lib.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/lib.rs) maintains `StudioEngine.workspace_files: Vec<WorkspaceFile>` separately from `StudioEngine.mvm` (`MvmSandbox`), syncing text snapshots via `sync_workspace_into_mvm()` / `sync_workspace_from_mvm()`.
   - [`public/app.js`](file:///Users/casey.allard/lexlean-r4/public/app.js) intercepts terminal input through a JS `switch (cmd)` shell (`Linux wasm-container ... wasm32`), only delegating specific pipe binaries to `mvm_exec`.
   - `mvm_initramfs.bin` (`UORMVM01`) in [`crates/wasm/src/mvm.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/mvm.rs) ships only ~10 core utilities (`gpu/mvm_guest/`), `mvm_cc.rs` accepts a single source string with `#define` only (no `#include` or multi-TU linking), and `SYS_socket` (`admit_guest_socket_domain`) rejects standard `AF_INET`/`AF_INET6` sockets in Zero-NIC mode without transparent SpiralCore encapsulation or unprompted CORS-preempting public web egress.
   - No interactive `[▶ Run]` / `[Edit]` / `[Auto-run]` collaboration flow connects agent file preparation on `/workspace` to visible execution in the MVM terminal.

5. **Single-Threaded Wasm32 Inference Bottleneck & Harness Token Overhead:**
   - `WebGpuHoloEngine` in [`crates/wasm/js/worker_host_bridge.js`](file:///Users/casey.allard/lexlean-r4/crates/wasm/js/worker_host_bridge.js) compiles 4 WGSL shaders and reports `webgpu-i8-dot4`, **but never uploads model weight slabs to `GPUBuffer`s or dispatches GPU compute passes during token generation**. Every forward pass still streams ~410 MiB–1.06 GiB through single-threaded Wasm32 CPU loops (`holo_geometric_runtime.rs`), making long system prompts and `0.6B`/`1.7B` generation dramatically slower than the 10k+ tok/s UOR-R4 LUT engine.

---

## 2. Goals, Non-Goals, & Anti-Goals

### Goals
1. **Zero CI Queue Contention & Fast Builds (Wave 0):** Eliminate background `Engine` cron queue contention via `R4_ENGINE=off`; cut `pages.yml` `just vv` wall-clock time from ~37 min to **~6–9 min** while running 100% of `just vv` inside the locked SDK container; keep local incremental rebuild footprints compact (`< 4 GB`).
2. **Cursor-Grade Compact UI & Non-Blocking Dropdown HUD (Wave 1):** Fit all Studio panes cleanly inside `100dvh` with `0` overlapping controls and `0` horizontal/vertical chrome clipping across `1440×900`, `1280×720`, `768×1024`, and `390×844`. Replace the blackout Observatory modal with a floating, non-blocking topbar **Dropdown Neural HUD** (`#neural-hud-dropdown`).
3. **2-Model Picker & Effortless Auto-Hydration (Wave 1):** Expose only `Qwen3 0.6B · Fast (410 MB)` and `Qwen3 1.7B · Reasoning (1.1 GB)` in the primary dropdown with crystal-clear download/cache states, and automatically download & verify weights on first send for both models with inline progress and resilient retries.
4. **Real Single-Source MVM Linux Container & CORS-Preempting SpiralCore Net (Wave 2):** Make `MvmSandbox`'s 64 KB OPFS-backed CoW `/workspace` the single live filesystem (removing `Vec<WorkspaceFile>` split-brain and the JS fake terminal); expand `mvm_initramfs.bin` into a compact compressed `riscv64` developer userland (`sh`, coreutils, `rg`, `sed`, `awk`, `jq`, `diff`, `patch`, `tar`, `gzip`, `git`, `curl`, `make`, upgraded `#include`/multi-TU `mvm_cc`); bridge guest TCP/HTTP/DNS transparently over `AF_VSOCK = 40` with unprompted public CORS preemption via `relay/worker.mjs`; and ship interactive `[▶ Run]` / `[Edit]` / `[Auto-run]` collaboration + a KV-pinned `<= 120`-token state header.
5. **High-Throughput WebGPU Serving & Weight-Native Studio Awareness (Wave 3):** Execute the 28-layer `UORR4V13` forward pass on per-layer (`<= 128 MiB`) resident GPU VRAM buffers with `packed_4x8_integer_dot_product` + `HoloTok13`-native speculative decoding (`4–6` draft tokens verified in one GPU batch tile at `0-LSB` parity), then train Studio/MVM SFT/LoRA weights remotely on `wick` (or Vast.ai GPU pod fallback), push updated `weights-v13-*` shards, and shrink the static harness prompt to `< 40` tokens in lockstep.

### Non-Goals
- Full x86/x86_64 OS emulation (QEMU, v86, container2wasm).
- Sending user credentials, GitHub PATs, or private headers through any CORS proxy or SpiralCore relay (`RelayForbiddenForCredentialedHost` stays strictly enforced).
- Building or publishing a Chrome Web Store extension in this repo (expose only the host-side `window.__uorSpiralCoreExtensionBridge` hook for optional power-user raw sockets).
- Editing any universal template file byte-bound in [`template.lock`](file:///Users/casey.allard/lexlean-r4/template.lock).

### Anti-Goals
- Reducing or skipping `just vv` checks in `pages.yml` (`browser_closure.rs` requires `exec just vv` inside the locked SDK image).
- Introducing floating-point inference ops (`max_float_serving_ops = 0`) or dense matmul ops (`max_dense_matmul_ops = 0`) in shipped crates (`crates/wasm`).
- Raising `model/ratchet.toml` `max_residue_functions` or inflating `synthesis.rs` `complexity` past `344`.
- Silent fallback from a selected Qwen3 model to `Native Q31` without explicit user action.
- Blackout backdrops behind live token/neural visualizations.

---

## 3. Repository & Operational Constraints

1. **Formal Contract (`AGENTS.md` / `uor/template-contract/1`):**
   - `R1–R6` strictly apply: capabilities `R4-62` (Wave 1), `R4-63` (Wave 2), and `R4-64` (Wave 3) each start from a LexLean module (`src/R4/*.lex.tex`) + control tests (`tests/lexlean/*.lex.tex`), `model/ids.toml`, `model/routes.toml`, `CONFORMANCE.md` regenerated solely via `just model-write`, Gherkin scenario (`features/suites/*.feature`), failing conformance test, planted-defect log in `OVERHAUL-VERIFICATION.md`, and full verification.
   - `xtask -- ratchet-check` and `xtask -- synthesize` enforce `complexity <= 344` (`ratchet.toml` ceiling `355`), `float_serving_ops = 0`, `dense_matmul_ops = 0`, `unwired_ui_controls = 0`, and `public/lexlean_r4_bg.wasm <= 15 MiB` (`max_wasm_bytes = 15 * 1024 * 1024`). Every new shipped Rust/JS function must carry `/// Formal R4.<Module>` (or be lifted) while consolidating legacy helpers so `complexity` stays `<= 344` across all waves.
   - Universal policy files bound in [`template.lock`](file:///Users/casey.allard/lexlean-r4/template.lock) (`.devcontainer/devcontainer.json`, `.github/workflows/bootstrap.yml`, `AGENTS.md`, `VERIFICATION.md`, `prismpm.lock`, `template-contract.json`) must **never** be modified.
   - Compatibility note for `R4-37`: [`crates/wasm/tests/mvm_spiralcore_sandbox.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/tests/mvm_spiralcore_sandbox.rs) and [`crates/wasm/tests/overhaul_e2e/tier1_feature.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/tests/overhaul_e2e/tier1_feature.rs) assert that stateless `admit_guest_socket_domain(2)` (`AF_INET`) returns `SocketAdmission::RefusedNicDisabled` and `MvmSandbox::boot_default()` isolates raw guest NICs (`-EAFNOSUPPORT = -97`). Therefore `MvmSandbox` **keeps** `boot_default()` Zero-NIC isolated for unit tests and only activates transparent `AF_INET` → `AF_VSOCK = 40` (`SPVS`) encapsulation when Studio/SpiralCore Net Mode is explicitly enabled (`Net: SpiralCore`), preserving `R4-37` intact.
2. **Local Disk & RAM Protection:**
   - Maintain `>= 30 GiB` free on `/`. Locally run targeted crate/test commands with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2` and run `rm -rf target` immediately after verification. Never download multi-GB PyTorch checkpoints onto the local Mac; Wave 3C pulls HuggingFace checkpoints and trains strictly on remote `wick` / Vast.ai pods.

---

## 4. Approaches Considered & Trade-Offs

| Approach | Summary | Trade-Offs | Decision |
| --- | --- | --- | --- |
| **A. 4-Wave Phased Overhaul (`Wave 0 → 1 → 2 → 3`)** | Fix CI queue/build speed first (**Wave 0**), ship UI repolish + non-blocking HUD + auto-hydration (**Wave 1**), unify MVM `/workspace` + SpiralCore CORS-preempting Net + Interactive Run (**Wave 2**), then ship Resident WebGPU + `HoloTok13` Speculative Serving + remote LoRA weights (**Wave 3**). | Slightly more intermediate PR/deploy checkpoints, but eliminates the 1h+ CI bottleneck on day 1, restores a clean usable app in Wave 1, and ensures fast serving before baking fine-tuned weights in Wave 3. | **Selected** |
| **B. Kernel & WebGPU Serving First, UI & CI Last** | Tackle WebGPU forward pass and MVM `/workspace` before touching CI or UI/hydration. | Leaves every test deploy stuck behind 37–90 min CI waits and keeps the live site visually crowded and unhydrated while deep kernel/GPU changes are iterated. | Rejected |
| **C. Single Monolithic All-At-Once Rewrite** | Change CI, UI CSS/JS, MVM VFS, networking, WebGPU kernels, and LoRA shards on one branch. | Large blast radius across 61+ formal conformance gates and browser E2E suites; difficult to isolate WebGPU/OPFS regressions from CSS/MVM regressions. | Rejected |

---

## 5. Architecture & Wave Specifications

### Wave 0: CI Zero-Queue & ~4× `just vv` Build/Test Speedup
**Formal Scope:** Build/workflow/harness optimization only (zero relaxation of assertions, zero skipped/ignored/filtered tests, preserves `exec just vv` in SDK image per [`crates/wasm/tests/browser_closure.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/tests/browser_closure.rs)).

1. **Queue Elimination:**
   - Keep GitHub repository variable `R4_ENGINE=off` set while waves are under active development (activates `engine.yml` line 39 `vars.R4_ENGINE != 'off'`), re-enabling after Wave 3 lands if desired.
   - Update [`.github/workflows/pages.yml`](file:///Users/casey.allard/lexlean-r4/.github/workflows/pages.yml) concurrency so the `build` job uses `concurrency: { group: 'pages-build-${{ github.ref }}', cancel-in-progress: true }` (cancelling obsolete pushes immediately) while the 18-second `deploy` job uses `concurrency: { group: 'pages-deploy', cancel-in-progress: false }` to protect live Pages publication ordering.
2. **Rolling Cargo Cache (Matching `overhaul_e2e` F01 Contract):**
   - Preserve exact prefix `r4-pages-target-` and `hashFiles('Cargo.lock')` required by [`crates/wasm/tests/overhaul_e2e/tier1_feature.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/tests/overhaul_e2e/tier1_feature.rs), changing key to:
     - `key: r4-pages-target-v2-${{ hashFiles('Cargo.lock') }}-${{ hashFiles('crates/**/src/**/*.rs', 'xtask/src/**/*.rs', 'src/R4/**/*.lex.tex') }}`
     - `restore-keys: |` -> `r4-pages-target-v2-${{ hashFiles('Cargo.lock') }}-` and `r4-pages-target-v2-`
   - In `Prune Cargo incremental cache`, prune stale old test executables (`find target/debug/deps -maxdepth 1 -type f -executable -mtime +1 -delete || true`) alongside `incremental/` directories so the saved cache remains bounded and fresh.
3. **Single-Compile Profile Alignment (`Cargo.toml`):**
   - Align third-party dependency optimization across dev and test (`[profile.dev.package."*"] opt-level = 2`, `[profile.test.package."*"] opt-level = 2`, and shared `lexlean-r4-wasm` `opt-level = 2` across both `dev` and `test` if kernel math runtime requires optimization) so crates compile **once** across `clippy`, `test`, and `check` instead of twice at `opt-0` and `opt-2`.
4. **Deduplicate `xtask` Tests (`xtask/src/lib.rs`):**
   - Introduce `xtask/src/lib.rs` exporting `audit`, `bootstrap`, `engine`, `mechanizer`, `ratchet`, `routes_and_measures`, and `synthesis`, keeping `xtask/src/main.rs` as a thin CLI entrypoint.
   - Update `xtask/tests/empirical_challenge_*.rs` to import `use xtask::...` instead of `#[path = "../src/engine.rs"] mod engine;` — eliminating 4 duplicate compilations of `engine.rs` + `lut_stack_exec.rs` + `40.3 MB` `.uorlut` and 4× duplicate runs of `engine::tests` (~100 unit tests).
5. **Process-Wide `OnceLock` Immutable Model Sharing & Test Binary Consolidation:**
   - Wrap immutable embedded model loading in `OnceLock<Arc<...>>` everywhere `LutStackModel::parse` or `NativeQ31Model::from_bytes` is constructed for read-only / CoW-cloned usage (notably `crates/wasm/src/api.rs:1151` and `crates/wasm/tests/` helpers).
   - Consolidate standalone `crates/wasm/tests/empirical_challenge_*.rs` test files (preserving every `#[test]` function verbatim in submodules under a single `crates/wasm/tests/empirical_suite.rs` harness) so Cargo links one binary instead of ~17 separate 80+ MB binaries, executing all empirical tests in parallel across CI vCPUs.
   - **Wave 0 Exit Criteria:** `just vv` passes cleanly inside SDK image, all `tier1_feature.rs` F01 & `browser_closure.rs` gates pass, total `pages.yml` build time drops materially toward `< 10 min`, and local `rm -rf target` leaves `>= 30 GiB` free.

---

### Wave 1 (`R4-62`): Compact Cursor-Grade UI, Non-Blocking Dropdown HUD & Auto-Hydration

**Formal Specification (`src/R4/StudioFrame.lex.tex` / `R4-62`):**
- Add `R4-62` superseding `R4-60`'s unconditional off-origin refusal for first-party repo weight mirrors:
  - `HydrationOriginPolicy::FirstPartyCatalog` (both `0.6B` `weights/v13_qwen3_0_6b/` + `raw.githubusercontent.com/Casey-allard/lexlean-r4/weights-v13-qwen3-0.6b/` and `1.7B` `weights/v13_qwen3_1_7b/` + `weights-v13-qwen3-1.7b/`) **admits automatic hydration on send/click without a consent modal** (`HydrationAdmitted`).
  - `HydrationOriginPolicy::ExternalUntrustedOrigin` requires explicit user confirmation (`RefusedExternalOriginConsentRequired`).
  - `OfflineFallbackPolicy` activates **only** when the user explicitly toggles Offline Emergency Mode in Settings (or invokes internal Settings/programmatic emergency switching) — never via sticky background `declined` flags when Qwen3 is selected (`RefusedSilentModelDowngrade`).
  - `HudPresentationMode` (`DropdownCompactStrip`, `DropdownExpandedDeck`, `ChromelessEmbed`) forbids blocking full-screen backdrops during live neural streaming (`RefusedBlockingBackdropOnLiveHud`).
- In lockstep under `R1`, update `model/ids.toml`, `CONFORMANCE.md` (`just model-write`), `crates/conformance/tests/bdd.rs`, `crates/conformance/tests/r4_60_studio_frame.rs`, `crates/conformance/tests/honest_plumbing.rs`, `crates/wasm/tests/overhaul_e2e/`, `crates/wasm/tests/empirical_challenge_chatbox.rs`, `crates/wasm/tests/empirical_challenge_m3_closure.rs`, `site/app.json` (`build-holo --sync-site-manifest`), and `tests/e2e/{foundry_observatory_e2e,foundry_worker_stepper_unit,honest_model_plumbing_e2e,prism_holo_me_e2e}.mjs` to reflect `R4-62`'s 2-model picker, zero-prompt catalog hydration, and desktop/mobile touch split.

#### 1A. Model Picker & Seamless Auto-Hydration
- **User-Facing Model Options (`public/index.html`, `public/app.js`):**
  - Replace the 4 options in `#model-select` and `#composer-model-select` with exactly two clear choices:
    - `<option value="uor-r4.qwen3-0.6b-holo" selected>Qwen3 0.6B · Fast (410 MB)</option>`
    - `<option value="uor-r4.qwen3-1.7b-holo">Qwen3 1.7B · Reasoning (1.1 GB)</option>`
  - Remove `uor-r4.native-q31-d85` and `uor-r4.hf-repo-holo` from user-facing `<select>` menus. Keep an explicit **Offline Emergency Mode (Zero-Download Q31)** checkbox inside `#settings-modal` (plus backend `?embed=me` `uor-workspace-bridge/1:set-model` / `studio_dispatch` API compatibility) and keep `NativeQ31Model` wired internally in Rust (`crates/wasm/src/lib.rs`) so `xtask/src/synthesis.rs` and offline emergencies remain intact without cluttering or hijacking the main UX.
- **Honest Status Pill & Inline Hydration Bar (`#weight-cache-badge`):**
  - Boot immediately runs `readWarmSlabSeal(selectedModel)` in OPFS (`HOLOSLAB2`) and updates `#weight-cache-badge`:
    - Uncached: `410 MB · Auto-downloads on send` / `1.1 GB · Auto-downloads on send` (clickable to start background download immediately).
    - Downloading: Inline sleek progress fill + percentage/MB counter (`Downloading 42% · 172 / 410 MB`) inside the topbar pill AND inline at the top of the pending assistant response turn.
    - Cached & Mounted: `Ready · WebGPU` or `Ready · Wasm SIMD`.
- **Resilient Hydration Transport (`crates/wasm/js/worker_host_bridge.js`):**
  - Admit automatic hydration on send/click for both `uor-r4.qwen3-0.6b-holo` (`410 MB`) and `uor-r4.qwen3-1.7b-holo` (`1.1 GB`); purge/ignore legacy `localStorage.getItem('uor_allow_off_origin_hydration') === 'declined'` hijacks on boot unless Settings Offline Emergency Mode is enabled.
  - Fix `RefusedInsufficientParityShards: failed piece 0`:
    1. Reduce parallel piece fetch concurrency against `raw.githubusercontent.com` from `CONCURRENCY = 6` to `CONCURRENCY = 3` with exponential backoff + jitter (3 retries per transient HTTP/network error) before invoking Stitch 2-of-3 XOR parity recovery.
    2. Validate any cached IndexedDB `uor_holo_kappa_v1` manifest against the live mirror's `container_sha256` on fetch failure and automatically refresh stale manifests so stale cache metadata never bricks piece 0.
    3. Keep external arbitrary-URL confirmation gating testable via `requireExplicitConsentForTesting` / non-catalog URLs (`R5` negative coverage).

#### 1B. Cursor-Grade Compact Viewport Layout (`public/app.css`, `public/index.html`)
- **Top-Level Viewport Layout:**
  ```text
  ┌─ [≡] R4 LexLean  [Chat|Studio] ── [Qwen3 0.6B · Fast (410 MB) ▾] [410 MB · Ready ▾] [⚡ HUD ▾] ── [Net: SpiralCore] [GitHub] [⚙] ─┐ 38px
  │ ┌──────────────────────────────────────────────────────────────────┐                                                               │
  │ │ #neural-hud-dropdown (Floating Topbar Panel — ZERO backdrop)     │  Right-aligned below topbar, never dims background            │
  │ │ [Compact 76px live ribbon] ⇄ [Expanded 4-Canvas + 3D Deck]       │  Live TTFT · tok/s · Layer Waterfall · Logit Bars · Geodesic  │
  │ └──────────────────────────────────────────────────────────────────┘                                                               │
  ├─ Sidebar (232px) ─┼─ Center Workbench (flex 1fr) ───────────────────────┼─ Chat & Agent Column (100% height, no bottom split) ─────┤
  │  Explorer (MVM)   │  [tab1 ×] [tab2 ×]  [Code|Preview|Diff] [Cmd+K] 30px│  Transcript + Thinking Stream + Staged MVM Action Cards  │
  │  SCM / Search     │  src › main.c  ·  ✓ R4-32 · Auto-saved          22px│                                                           │
  │                   │  1 │ ... (Code Editor fills remaining height)       │                                                           │
  │                   ├─────────────────────────────────────────────────────┤  ┌─ Ask or build in /workspace ... ──────────────────────┐  │
  │                   │  TERMINAL · MVM RV64 (/workspace) [Auto-run ○][×]26px│  │                                                     │  │
  │                   │  lexlean@mvm:/workspace$ _                          │  │ [+] [@] [/]   [Agent|Plan] [Auto-run ○] [Send ↵]    │  │
  └─ ● MVM RV64 · main · 0 errors · 0 uncommitted ───────── Ln 1, Col 1 · C ──────────────────── Net: Direct · WebGPU ─────────────────┘ 22px
  ```
- **Concrete CSS Geometry Fixes:**
  - Global desktop sizing tokens (`min-width: 600px`): buttons `min-height: 26px; padding: 0.18rem 0.5rem; font-size: 0.76rem;`, compact mini/icon buttons `22–24px`, selects/inputs `26–28px`, topbar `38px` (`flex-wrap: nowrap` on `>= 840px` with responsive truncation on model badge), editor tab strip `30px`, breadcrumbs strip `22px`, bottom status bar `22px`.
  - Fix `@media (pointer: coarse)` bloat: apply `44px` touch targets (including `.cursor-status-btn`, `.cursor-cmdk-close`, `.canvas-close-btn`) **strictly** inside `@media (max-width: 599px), (pointer: coarse) and (hover: none)` so desktop browsers on Macs/Windows with touchscreens/trackpads never trigger `44px` giant buttons inside `22–38px` toolbars.
  - Consolidate the 10 vertical bars in `#dockable-canvas-pane`:
    - Merge editor tabs and workbench switcher (`Code | Preview | Diff | Cmd+K | Check | Terminal`) into one `30px` row with horizontal tab scroll.
    - Merge breadcrumbs + `R4-32` diagnostics badge + save status into one `22px` bar (removing redundant `.pane-subheader`, static `#cursor-problems-strip`, and standalone Save button row; surface ghost-text inline only when active).
    - Merge terminal drawer header and bar into one `26px` bar (`TERMINAL · MVM RV64 (/workspace)`, `Auto-run` pill, `Clear`, `Reset`, `Hide`).
  - Remove `#observatory-dock-split` (`flex: 0 1 42%`) from `#prism-right-column` so the Chat + Agent column has full vertical height in Studio mode.

#### 1C. Non-Blocking Dropdown Neural HUD (`#neural-hud-dropdown`)
- **Replace Blocking Modal with Topbar Dropdown Panel:**
  - Remove `.inspectors-modal-backdrop` blocking overlay from live telemetry (`#settings-modal`, `#github-connect-modal`, `#new-path-modal`, and `#cursor-command-palette` keep crisp focused modal dialogs when opened intentionally).
  - `#neural-hud-dropdown` mounts as a floating non-modal panel (`position: fixed; top: 42px; right: 12px; width: min(620px, calc(100vw - 24px)); max-height: min(68dvh, 540px); z-index: 650`) anchored to `[⚡ HUD ▾]` (`#toggle-inspectors-btn`) in the topbar, **never obscuring the chat stream on the right/left or intercepting clicks outside its bounds**.
  - **Two modes inside `#neural-hud-dropdown`:**
    1. **Compact Live Strip (default when toggled during streaming):** Low-profile `76px` translucent dark-glass HUD bar showing live `TTFT`, `tok/s`, active backend (`WebGPU` / `Wasm SIMD`), mini 28-layer residual sparkline canvas, Top-4 candidate token probabilities, and live geodesic action route—updating on every `chat_chunk` frame at `requestAnimationFrame` cadence without layout thrashing.
    2. **Expanded Observatory Tabs:** One-click expand reveals the full 4-canvas Holo Observatory deck (`28L Residual Waterfall`, `151,936-Vocab RHT Logit Race`, `Flock Sparse Attention`, `OPFS + MVM CoW Geometry`), 3D Riemannian Manifold WebGL2 canvas, OTel Traces, ML Coordinates, Diagnostics, and Self-Improve panels.
  - `?surface=observatory` chromeless iframe embed stays intact for headless/iframe consumers.

---

### Wave 2 (`R4-63`): Single-Source MVM Linux `/workspace`, SpiralCore CORS-Preempting Net & Interactive Run

**Formal Specification (`src/R4/MvmLinuxContainer.lex.tex` / `R4-63`):**
- Model single-source MVM VFS consistency (`WorkspaceSingleSource`), transparent SpiralCore socket encapsulation (`SpiralCoreSocketAdmission`: isolated `-EAFNOSUPPORT = -97` when `Net: Isolated` / `boot_default()`; `AF_VSOCK` `SPVS` encapsulation when `Net: SpiralCore`), `R4-59` public web override allowing unauthenticated CORS preemption via direct `fetch` -> `relay/worker.mjs` with zero domain allowlist blocks or per-site popups while preserving SSRF CIDR & `Authorization`/`GITHUB_TOKEN` relay refusal (`RefusedCredentialedCorsProxy`, `PrivateAddressBlocked`), and interactive command gate precedence (`InteractiveApproval`, `AutoRunAdmitted`, `StagedPendingUserGo`, `RefusedInPlanOrDenyPolicy`).

#### 2A. Single-Source MVM `/workspace` (Zero Split-Brain)
- Remove `StudioEngine.workspace_files: Vec<WorkspaceFile>` (`crates/wasm/src/lib.rs`) as an independent store; `StudioEngine.mvm` (`MvmSandbox` in [`crates/wasm/src/mvm.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/mvm.rs)) becomes the sole live CoW inode tree under `/workspace` (storing both UTF-8 source files and binary artifacts: ELF64 binaries, `*.db`, `*.wasm`, `.git/**`, `node_modules/**`), persisted incrementally to OPFS (`uor-mvm-cow-v1.bin`).
- Keep `sync_workspace_from_mvm()` / `sync_workspace_into_mvm()` as thin compatibility shims reading directly from MVM (or updating `mvm_spiralcore_sandbox.rs` directly alongside direct MVM VFS checks) with zero duplicate storage.
- All Studio APIs (`studio_dispatch` `workspace/list`, `workspace/read`, `workspace/write`, `workspace/delete`, `workspace/rename`, Explorer UI, Editor tabs, Git SCM diff, and Agent tools) read/write `self.mvm` directly, returning `changed_paths` and `cow_state_sha256` so Explorer, Editor, Diff, and Terminal stay instantaneously consistent.

#### 2B. Real MVM Interactive Shell, Compressed Userland (`<= 12 MiB` Wasm) & Multi-TU `mvm_cc`
- Delete the JS `switch (cmd)` fake terminal in [`public/app.js`](file:///Users/casey.allard/lexlean-r4/public/app.js) (`Linux wasm-container ... wasm32`, fake `cd`/`pwd`/`ls`).
- Every command entered in Terminal routes through `MvmSandbox`'s persistent shell session (`cwd`, `env`, `$?`, background jobs, pipelines `|`, logical chains `&&`/`||`, globs, heredocs, quoting), printing real `riscv64` kernel identity (`uname -a` -> `Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64`).
- **Stay well under the `15 MiB` Wasm ceiling (`public/lexlean_r4_bg.wasm` currently `10.96 MiB`):**
  - Build the expanded Linux developer userland via compact in-kernel Rust applets + freestanding multi-call `riscv64` C applets (`gpu/mvm_guest/mvmbox.c` et al.) packaged in a **compressed `UORMVM01` initramfs (`< 512 KiB`)** decoded cleanly at `MvmSandbox::boot()`, keeping `public/lexlean_r4_bg.wasm` `<= ~12 MiB` (while `node`, `pyodide`, and extra packages continue using lazy OPFS/Cache hydration under `R4-58`):
    - POSIX `sh` + coreutils (`ls`, `cp`, `mv`, `rm`, `mkdir`, `rmdir`, `touch`, `chmod`, `ln`, `cat`, `head`, `tail`, `wc`, `sort`, `uniq`, `tr`, `cut`, `tee`, `find`, `xargs`, `env`, `export`, `which`, `ps`, `df`, `free`, `uname`, `whoami`, `id`, `date`, `base64`, `sha256sum`, `xxd`, `bc`).
    - Text & archive tools (`grep`/`rg`, `sed`, `awk`, `jq`, `diff`, `patch`, `tar`, `gzip`/`gunzip`).
    - Developer & repo tools (`git` integrated with SpiralCore Smart-HTTP / GitHub REST, `curl`/`http`, `make`, upgraded [`crates/wasm/src/mvm_cc.rs`](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/mvm_cc.rs) supporting `#include <...>` / `#include "..."` preprocessing against `/usr/include/` headers (`stdio.h`, `stdlib.h`, `string.h`, `unistd.h`, `sys/socket.h`, `netdb.h`), basic `struct`/pointer layout, and multi-file TU linking/symbol resolution).
  - **Self-Hosting MVM Source Pack:** Bundle `/usr/src/mvm/` headers & sample guest tools inside the compressed initramfs so the browser agent + user can edit, compile with `cc`/`make`, run, and improve MVM guest programs directly inside MVM.

#### 2C. Transparent SpiralCore `AF_INET` Encapsulation & Unprompted Public CORS Preemption (`R4-59` Override)
- **Zero-NIC Compatibility Preserved:** `MvmSandbox::boot_default()` and stateless `admit_guest_socket_domain(2)` keep Zero-NIC isolation (`-EAFNOSUPPORT = -97`) so all `R4-37` security unit tests stay green.
- **Transparent `Net: SpiralCore` Mode (Default in Studio):** Guest `socket(AF_INET | AF_INET6, ...)`, `getaddrinfo()`, `connect()`, `send()`, `recv()`, `/bin/curl`, `/bin/git`, and `cc`-compiled C programs transparently encapsulate outbound IP/HTTP/DNS traffic through signed 16-byte `SPVS` (`AF_VSOCK = 40`, `CID 3 -> CID 2`) frames handled by [`crates/wasm/js/worker_host_bridge.js`](file:///Users/casey.allard/lexlean-r4/crates/wasm/js/worker_host_bridge.js):
  1. **Override `R4-59` Domain Allowlist & Consent Popup:** When `Net: SpiralCore` is ON in Studio, **supersede** `R4-59`'s `Sandboxed` 10-domain allowlist and `RelayConsentRequired` prompt (`CapabilityDenied`) so unauthenticated public web requests to arbitrary non-private hosts (`example.com`, docs, package registries, web searches) run automatically without popups or allowlist blocks.
  2. **Tier 1 — Direct Browser APIs:** DNS-over-HTTPS (`cloudflare-dns.com` / `dns.google`) + direct Worker `fetch()` for CORS-enabled HTTPS APIs (`api.github.com`, `raw.githubusercontent.com`, registries, JSON APIs).
  3. **Tier 2 — Automatic Stateless CORS Preemption via `relay/worker.mjs`:** Whenever direct `fetch()` rejects with a browser CORS `TypeError` on a non-loopback, non-RFC1918 public HTTP/HTTPS request carrying **no** user credentials (`Authorization` header, cookies, or Vault PAT absent), the Worker automatically retries through `DEFAULT_SPIRALCORE_RELAY_HTTP_URL` (`https://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs`, backed by [`relay/worker.mjs`](file:///Users/casey.allard/lexlean-r4/relay/worker.mjs)), expanding `relay/worker.mjs` if needed for general `GET`/`HEAD`/safe unauthenticated text/HTML responses with `Access-Control-Allow-Origin: *`. Any request carrying `Authorization`, cookies, `GITHUB_TOKEN`, or targeting `api.github.com` / private/loopback CIDRs stays **strictly blocked** from proxy/relay failback (`RelayForbiddenForCredentialedHost`, `PrivateAddressBlocked`).
  4. **Tier 3 — Custom WSS Relay / Host Extension Hook (No Extension Deliverable):** For raw non-HTTP TCP sockets or custom local proxies, support user-configured `wss://` relays (`#spiralcore-relay-input`) and detect an optional host-side companion bridge (`window.__uorSpiralCoreExtensionBridge` / `postMessage`)—without building or shipping a Chrome Web Store extension in this repo.

#### 2D. Interactive Agent ↔ User Execution Precedence & Compact KV-Pinned State
- **Unified Fail-Closed Policy Precedence (`[Agent | Plan]` × `[Auto-run]` × `#mcp-policy-select`):**
  1. `#mcp-policy-select = deny` **blocks all tool execution** (`Denied`) regardless of `[Auto-run]`.
  2. `Plan` mode (`PlanReadOnly`) auto-runs read-only inspection (`workspace_read`, `workspace_search`, `ls`, `cat`, `rg`, `git status`, `git diff`, `uname`, `pwd`) and **refuses all workspace writes and mutating `[▶ Run]` commands** (`RefusedInPlanMode`) even if `[Auto-run: On]` is toggled.
  3. In `Agent` mode (`AgentFull`), the composer/terminal **`[Auto-run: Off | On]`** toggle syncs cleanly 1:1 with `#mcp-policy-select` (`Off` ↔ `confirm` for mutating/build/network shell execution while still auto-running read-only inspection; `On` ↔ `safe` auto-running all admitted commands).
- **Interactive "Prep & Hit Go" Flow:** When `Auto-run: Off`, the agent writes/diffs any preparation files into MVM `/workspace` immediately and emits a `stage_mvm_command` turn pause rendering a live **Staged MVM Action Card** in chat + mirroring the pre-filled command into the bottom MVM terminal prompt:
  - Controls: **`[▶ Run]`**, **`[Edit]`**, **`[Skip]`**.
  - Clicking **`[▶ Run]`** executes inside the live MVM terminal with streaming output and automatically resumes the agent turn (`foundry_resume_staged_command`) with `{ exit_code, stdout, stderr, changed_files }`.
- **Minimal KV-Pinned Live State Header (`<= 120` tokens):** Structure prompts as `[FIXED_CACHEABLE_PREFIX]` + `[MVM_LIVE_STATE_HEADER]` (`cwd`, active tab, dirty files, `auto_run`, `net`, `last_exit`), pinning `[FIXED_CACHEABLE_PREFIX]` in KV cache so subsequent turns pay near-zero instruction prefill latency.

---

### Wave 3 (`R4-64`): True Resident WebGPU Serving, `HoloTok13` Speculative Decoding & Remote LoRA Training

**Formal Specification (`src/R4/WebGpuResidentRuntime.lex.tex` / `R4-64`):**
- Model per-layer (`<= 128 MiB`) resident VRAM buffer admission (`ResidentSlabAdmitted`, `RefusedInsufficientGpuBufferLimits`), `HoloTok13` speculative draft verification `0-LSB` equivalence (`SpeculativeExact0LsbAdmitted`, `RefusedSpeculativeParityDrift`), and remote SFT/LoRA `UORR4V13` provenance + `< 40`-token prompt shrink lockstep (`StudioLoraUorV13Admitted`, `RefusedFloatOpInLoraExport`).

#### 3A. True Resident WebGPU Integer Forward Pass (Fixes `0.6B`/`1.7B` Speed First)
- Update `WebGpuHoloEngine` (`crates/wasm/js/worker_host_bridge.js`) and Wasm bridge (`crates/wasm/src/holo_geometric_runtime.rs`, `crates/wasm/src/lib.rs`):
  1. **Limits & Per-Layer (`<= 128 MiB`) Buffer Partitioning:** During `WebGpuHoloEngine.init()`, request `requiredLimits` up to `adapter.limits.maxBufferSize` and `adapter.limits.maxStorageBufferBindingSize`, and request `'packed_4x8_integer_dot_product'` and `'subgroups'` when supported (with pure-integer `u32` WGSL fallback otherwise). Partition `0.6B` (`~410 MiB`) and `1.7B` (`~1.06 GiB`) into **28 per-layer storage buffers (`~15–38 MiB` per layer, safely below WebGPU's `128 MiB` default `maxStorageBufferBindingSize`)** plus chunked embedding/LM-head slabs uploaded in `<= 4 MiB` `writeBuffer` chunks once on mount, then release duplicate JS staging arrays. Emit `RefusedInsufficientGpuBufferLimits` → Wasm `simd128` CPU fallback cleanly only if device buffer/VRAM allocation fails.
  2. **Persistent GPU KV Cache & Single-Submit 28-Layer Decode:** Allocate persistent GPU KV ring buffers and residual ping-pong buffers; encode all 28 layers (`B = 1` or `B <= 32` batched tile) into **one `GPUCommandEncoder`** submission per step (`rmsnorm_q16`, `gemm_i4_dot4_packed`, `rope_flock_attn_i8`, `swiglu_geodesic_topk` supporting both `GeodesicSparse1536` and `DenseExact`, `rht512_vocab_shortlist`), reading back only a `<= 256`-byte result/HUD summary buffer per step.
  3. **End-to-End 28-Layer `0-LSB` Parity:** Using `vec2<u32>` hi/lo 64-bit integer emulation in WGSL where wide integer accumulation/scaling occurs (`RMSNorm` variance, Q30 score scaling, SwiGLU), verify **exact 28-layer token ID & top-K logit equivalence (`0-LSB`)** against CPU `HoloV13Model::step_token` across both `GeodesicSparse1536` and `DenseExact` modes (with Wasm `simd128` retained for WebGPU-less browsers/tests).

#### 3B. `HoloTok13`-Native Speculative Decoding (`2.5×–4×` Throughput Multiplier)
- Because `LutStackModel` uses `ByteBpe4096` (`4,096` tokens) while Qwen3 `UORR4V13` uses `HoloTok13` (`151,936` tokens), avoid cross-tokenizer text round-trips or `.wasm` table bloat by implementing a **`HoloTok13`-native suffix N-gram + `/workspace` identifier & `<tool_call>` grammar trie drafter** on CPU (`< 0.5 ms` draft latency):
  - Proposes `K = 4..6` `HoloTok13` candidate IDs per step grounded in prompt/context N-grams, open workspace identifiers, and JSON/shell grammar transitions.
  - Feeds `[accepted_tip, draft_1..draft_K]` directly into Wave 3A's `(K + 1)` batched WebGPU verification pass (`MAX_PREFILL_TILE = 32` from `R4-61`).
  - Accepts the longest exact matching token prefix (`0-LSB` lossless equivalence with autoregressive Qwen3) + bonus token in one GPU pass, rewinding the GPU KV write pointer cleanly on divergence.

#### 3C. Remote SFT/LoRA Training (`wick` / Vast.ai) → `UORR4V13` Weight Deployment
1. **Deterministic Trajectory Generator (`xtask -- generate-studio-sft-dataset`):** Generate ~1,500 verified multi-turn JSONL training episodes covering MVM `/workspace` file operations (`<tool_call>` JSON), `mvm_exec` & `[stage_mvm_command]`/`[Auto-run]` discipline, SpiralCore web/API/git tasks, MVM guest C compilation/debugging, and concise tool-result synthesis.
2. **Remote GPU Execution Pipeline (`wick` first, `/Users/casey.allard/.local/bin/vastai` fallback):**
   - Run `gpu/train_studio_lora.py` on SSH host `wick` if reachable/available, or provision one GPU pod via `/Users/casey.allard/.local/bin/vastai` (enforcing `trap "vastai destroy instance ..." EXIT` cleanup immediately after completion so zero idle rental spend remains).
   - Pull base HuggingFace `Qwen/Qwen3-0.6B` and `Qwen/Qwen3-1.7B` checkpoints strictly on the remote GPU instance (zero multi-GB PyTorch files downloaded to the local Mac), run LoRA SFT + merge to `safetensors`, evaluate held-out tool-call/argument accuracy, and run our pure-integer Rust `UORR4V13` compiler (`xtask -- compile-holo-v13` / `gpu/recompile_holo_v13.rs` + Stitch 2-of-3 parity split).
   - Push verified `UORR4V13` 1 MiB shards + `manifest.json` + `manifest.stitch.json` directly to `origin/weights-v13-qwen3-0.6b` and `origin/weights-v13-qwen3-1.7b`.
3. **Lockstep Prompt Shrink (`< 40` tokens):** Only once the newly trained `container_sha256` manifests are pushed and pinned in the repo, shrink `STUDIO_SELF_KNOWLEDGE_PREFIX` / `format_foundry_system_prompt` to `< 40` tokens in the same Wave 3 release commit so base weights never run without full instructions and fine-tuned weights achieve minimal prefill latency.

---

## 6. What Is Left to the Builder & Re-Plan Triggers

### Delegated to Builder During Implementation Planning
- Exact DOM ID preservation/aliasing (keeping legacy E2E hook selectors functional while simplifying visible chrome into 3 compact bars + `#neural-hud-dropdown`).
- Exact internal `OnceLock<Arc<...>>` cache wrapper struct layout and submodule organization inside `crates/wasm/tests/empirical_suite.rs`.
- Exact WGSL compute workgroup tile dimensions (`64` vs `128`) tuned against `device.limits.maxComputeInvocationsPerWorkgroup` in Wave 3.
- Exact freestanding POSIX C header subset under `/usr/include/` for `mvm_cc` (`R4-63`).
- Specific LoRA rank (`r = 16` vs `32`), learning rate, and epoch schedule in `gpu/train_studio_lora.py` (`R4-64`).

### Re-Plan Triggers
Stop and re-enter Plan Mode if any of the following occurs:
1. A Rust consolidation or Wave 1–3 refactor unexpectedly causes `xtask -- ratchet-check` or `xtask -- synthesize` to exceed `complexity <= 344` or `score_milli < 987` even with `/// Formal` annotations applied.
2. `public/lexlean_r4_bg.wasm` exceeds `13.5 MiB` (approaching the `15 MiB` `synthesis.rs` ceiling) during Wave 2 userland packaging, requiring offloading optional guest tools to OPFS slabs.
3. Safari/Chrome WebGPU `maxStorageBuffersPerShaderStage` (`8–10`) or remote `wick`/Vast.ai GPU availability interrupts Wave 3 execution.

---

## 7. Builder Review Calls Settled

The following 6 engineering decisions were settled during builder review (folded directly into Waves 0–3 above so planners do not need to guess):

1. **Superseding breaking test assertions & shimming legacy workspace entrypoints (Call 1):** Under `AGENTS.md` `R1`, registering `R4-62`..`R4-64` updates `model/ids.toml`, `CONFORMANCE.md` (`just model-write`), and superseded conformance/E2E assertions (`bdd.rs`, `tier1_feature.rs`, `tier3_combinations.rs`, `r4_60_studio_frame.rs`, `honest_plumbing.rs`, `mvm_spiralcore_sandbox.rs`, `empirical_challenge_chatbox.rs`, `empirical_challenge_m3_closure.rs`, `site/app.json`, and `tests/e2e/*.mjs`). Both `0.6B` (`410 MB`) and `1.7B` (`1.1 GB`) auto-hydrate on first send/click without consent prompts; `uor-r4.native-q31-d85` moves out of `<select>` menus to Settings ("Offline Emergency Mode") + programmatic `set-model` API compatibility; `@media (pointer: coarse)` 44px rules scope strictly to mobile/touch-only viewports; and `sync_workspace_from_mvm()` / `sync_workspace_into_mvm()` remain thin shims if referenced externally.
2. **Strict `complexity <= 344` Pareto maintenance (Call 2):** Every newly added pure function across Waves 0–3 carries `/// Formal R4.<Module>` (contributing `0` unlifted residue) while removing the legacy JS fake shell and split-brain workspace code lowers residue—keeping `complexity <= 344` monotonically satisfied without altering `model/ratchet.toml`.
3. **15 MiB Wasm ceiling & `mvm_cc` scope (Call 3):** Expanded POSIX/dev utilities run as compact in-kernel Rust applets + freestanding multi-call `riscv64` binaries in a compressed `< 512 KiB` `UORMVM01` initramfs (`lexlean_r4_bg.wasm` kept `<= ~12 MiB` under the `15 MiB` cap), leaving heavy Node/Pyodide blobs lazy-cached (`R4-58`). `mvm_cc.rs` scopes to `#include` preprocessing against bundled `/usr/include/` headers, basic `struct`/pointer layouts, and multi-TU resolution.
4. **Fail-closed tool policy precedence (Call 4):** `deny` blocks everything → `Plan` auto-runs read-only inspection only and refuses mutating edits/`[▶ Run]` even if `Auto-run: On` → in `Agent` mode, `[Auto-run: Off | On]` syncs 1:1 with `#mcp-policy-select` (`confirm` vs `safe`) while always auto-running read-only inspection.
5. **Per-layer `<= 128 MiB` WebGPU storage buffers & full 28-layer `0-LSB` parity (Call 5):** Partition weights into 28 per-layer storage buffers (`~15–38 MiB` each, under WebGPU's `128 MiB` default binding cap) requesting max adapter limits, and enforce full 28-layer end-to-end integer token & logit equivalence (`0-LSB`) against CPU `HoloV13Model::step_token` across both `GeodesicSparse1536` and `DenseExact` using `vec2<u32>` hi/lo 64-bit accumulators where needed.
6. **`HoloTok13`-native speculative drafter (Call 6):** Use a `HoloTok13`-native suffix N-gram + `/workspace` identifier & `<tool_call>` grammar trie drafter (`< 0.5 ms` CPU, zero `.wasm` table bloat, immune to `ByteBpe4096` vs `HoloTok13` tokenizer mismatch) verified losslessly by Qwen3's GPU `(K + 1)` tile.
