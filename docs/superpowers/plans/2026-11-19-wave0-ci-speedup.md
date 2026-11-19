# Wave 0: CI Zero-Queue & ~4× `just vv` Build/Test Speedup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate GitHub Pages queue waits (`R4_ENGINE=off` + cancelled superseded `build` jobs) and cut `just vv` build + test runtime from ~37 minutes down to ~6–9 minutes while keeping 100% of conformance assertions intact and local disk usage compact.

**Architecture:** Split `.github/workflows/pages.yml` concurrency (`cancel-in-progress: true` on `build`, `false` on 18-second `deploy`) and switch `actions/cache` to a rolling source-hash key with safe executable pruning. Align Cargo optimization profiles across `dev` and `test` so shared dependencies compile once, extract a zero-function `xtask/src/lib.rs` to eliminate 4× duplicate `engine::tests` runs, cache `StudioEngine`'s initialized baseline in a `OnceLock`, and consolidate standalone `empirical_challenge_*.rs` binaries into multi-module suites.

**Tech Stack:** GitHub Actions YAML, Cargo workspaces & build profiles, Rust (`std::sync::{Arc, Mutex, OnceLock}`).

**Spec:** [2026-11-19-lexlean-r4-mvm-studio-redesign-design.md](file:///Users/casey.allard/lexlean-r4/docs/superpowers/specs/2026-11-19-lexlean-r4-mvm-studio-redesign-design.md) (Wave 0, Section 5).

---

## Global Constraints

- Universal files listed in `template.lock` (`.devcontainer/devcontainer.json`, `.github/workflows/bootstrap.yml`, `AGENTS.md`, `VERIFICATION.md`, `prismpm.lock`, `template-contract.json`) must never be modified.
- `.github/workflows/pages.yml` must preserve `runs-on: ubuntu-24.04`, 40-character lowercase SHA action pins, `fetch-depth: 0`, `persist-credentials: false`, `exec just vv` inside the locked SDK container, `path: public`, and `if: github.ref == 'refs/heads/main'` (`crates/wasm/tests/browser_closure.rs:235-330`).
- `.github/workflows/pages.yml` cache step must keep a single-line `key:` starting with `r4-pages-target-` and containing `hashFiles('Cargo.lock')`, `path: target`, and `restore-keys:` entries starting with `r4-pages-target-`, leaving `timeout-minutes:` intact (`crates/wasm/tests/overhaul_e2e/common.rs:317-385`, `tier1_feature.rs:24-95`).
- Zero tests may be skipped, ignored, filtered, or omitted (`AGENTS.md` Gate quality): all `42` `#[test]` functions across the 5 `xtask/tests/empirical_challenge_*.rs` files and all `98` `#[test]` functions across the 17 standalone `crates/wasm/tests/empirical_challenge_*.rs` files must run under `cargo test`.
- `xtask -- ratchet-check` and `xtask -- synthesize` must pass with `complexity <= 344` (`residue <= 344`), `float_serving_ops = 0`, `dense_matmul_ops = 0`, `unwired_ui_controls = 0`, and `UE-16` zero regressions.
- Local Mac disk & RAM safety: execute all local cargo commands with `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2` and run `rm -rf target` after verification so `>= 30 GiB` remains free on `/`.

---

## Review Focus

1. **Cold-runner missing `target/debug/deps` during `if: always()` cache prune (Task 1):** On a fresh cache miss or early step failure, `find target/debug/deps` returns non-zero and fails the job if `target/debug/deps` does not exist; guard with `[ -d target/debug/deps ]` and pin in `test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency`.
2. **Half-deployed Pages race if `cancel-in-progress: true` is left global (Task 1):** Moving `cancel-in-progress: true` to workflow level could cancel `actions/deploy-pages` mid-upload; scope `cancel-in-progress: true` strictly to `jobs.build` (`pages-build-${{ github.ref }}`) while keeping `cancel-in-progress: false` on `jobs.deploy` (`pages-deploy`), pinned in `test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency`.
3. **`lift_register` function-path shift (`UE-16`) & `missing_docs = "deny"` when creating `xtask/src/lib.rs` (Task 2):** Moving functions out of `xtask/src/main.rs` alters `lift_register`'s path keys, and workspace `missing_docs = "deny"` rejects undocumented `pub` items; keep `xtask/src/lib.rs` limited strictly to `pub type Fail` + `#[allow(missing_docs)] pub mod ...` re-exports (`0` functions) and leave CLI functions inside `xtask/src/main.rs`, verified by `xtask -- lift-register`, `ratchet-check`, and `synthesize`.
4. **TCP socket / CPU starvation across consolidated `xtask` daemon hammer tests (Task 2):** `m3_api`, `m4_iter2_chal2`, and `m4_provenance_concurrency` each spawn live `xtask api serve` daemons and 20 client threads; running them concurrently in one binary triggers 2s connection timeouts on 4-vCPU runners — serialize daemon hammer tests with `crate::XTASK_DAEMON_LOCK`.
5. **Cross-module race on global `STUDIO_ENGINE` inside consolidated `crates/wasm/tests/empirical_suite.rs` (Task 3):** `empirical_challenge_m2_iter3_chal1` and `m2_iter4_chal1` previously held separate file-local `STUDIO_DISPATCH_LOCK` mutexes; unify both onto a single `crate::STUDIO_DISPATCH_LOCK` so `cargo test` parallel threads never collide on global `STUDIO_ENGINE` state.

---

## File Structure Map

| File | Action | Responsibility |
| --- | --- | --- |
| `.github/workflows/pages.yml` | Modify | Split `build`/`deploy` job concurrency, rolling cache key, guarded `target/debug/deps` prune |
| `crates/wasm/tests/overhaul_e2e/tier1_feature.rs` | Modify | Pin rolling cache & job concurrency contract in `F01` |
| `Cargo.toml` | Modify | Align `[profile.dev.package.*]` & `lexlean-r4-wasm` `opt-level = 2` with `[profile.test]` |
| `xtask/src/lib.rs` | Create | Zero-function crate library root exporting `xtask` modules & `pub type Fail` |
| `xtask/src/main.rs` | Modify | Consume `xtask::{...}` library modules instead of compiling a duplicate module tree |
| `xtask/tests/empirical_suite.rs` + `xtask/tests/empirical_suite/*.rs` | Create / Move | Unified runner for all 5 `xtask` empirical suites + `OnceLock` LUT + `XTASK_DAEMON_LOCK` + completeness check |
| `crates/wasm/src/lib.rs` + `crates/wasm/src/mvm.rs` | Modify | Derive `Clone` on `MvmSandbox`/sub-structs & `OnceLock`-cache `StudioEngine::new_default()` baseline |
| `crates/wasm/tests/empirical_suite.rs` + `crates/wasm/tests/empirical_suite/*.rs` | Create / Move | Unified runner for all 17 standalone Wasm empirical suites + shared `BOOTSTRAP_BYTES` / `STUDIO_DISPATCH_LOCK` |

---

### Task 1: Split `pages.yml` Job Concurrency & Rolling Cargo Cache

**Files:**
- Modify: `crates/wasm/tests/overhaul_e2e/tier1_feature.rs:85-104`
- Modify: `.github/workflows/pages.yml:12-80`

**Interfaces:**
- Preserves existing `PagesWorkflowSpec::parse(&content)` parser in `crates/wasm/tests/overhaul_e2e/common.rs:317-409`.
- Adds `test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency` in `crates/wasm/tests/overhaul_e2e/tier1_feature.rs`.

- [ ] **Step 1: Write failing F01 test for split concurrency & rolling cache key**

Add `test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency` to `crates/wasm/tests/overhaul_e2e/tier1_feature.rs` after `test_t1_f01_pages_workflow_execution_order`, asserting on `.github/workflows/pages.yml` text + `PagesWorkflowSpec::parse(&content)`:
- `spec.cache_key.starts_with("r4-pages-target-v2-")`
- `spec.cache_key.contains("hashFiles('Cargo.lock')") && spec.cache_key.contains("crates/**/src/**/*.rs")`
- `content` contains `"group: 'pages-build-${{ github.ref }}'"` and `"group: 'pages-deploy'"`, with `build` job containing `"cancel-in-progress: true"` and `deploy` job containing `"cancel-in-progress: false"`
- `content` contains `"if [ -d target/debug/deps ]; then"` before pruning old executables in `Prune Cargo incremental cache`.

- [ ] **Step 2: Run targeted `overhaul_e2e` test to verify it fails**

Run:
```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --test overhaul_e2e test_t1_f01_pages_workflow_rolling_cache_and_job_concurrency
```
Expected: FAIL asserting `spec.cache_key.starts_with("r4-pages-target-v2-")` against current `r4-pages-target-${{ hashFiles('Cargo.lock') }}`.

- [ ] **Step 3: Update `.github/workflows/pages.yml`**

Edit `.github/workflows/pages.yml`:
1. Remove top-level `concurrency:` block (`lines 12-14`) and add job-level concurrency:
   - Under `jobs.build`: `concurrency: { group: 'pages-build-${{ github.ref }}', cancel-in-progress: true }`
   - Under `jobs.deploy`: `concurrency: { group: 'pages-deploy', cancel-in-progress: false }`
2. Update `Restore Cargo target cache` (`lines 31-38`) keeping single-line `key:` and existing `restore-keys:` format:
   ```yaml
      - name: Restore Cargo target cache
        uses: actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0
        with:
          path: target
          key: r4-pages-target-v2-${{ hashFiles('Cargo.lock') }}-${{ hashFiles('crates/**/src/**/*.rs', 'xtask/src/**/*.rs', 'src/R4/**/*.lex.tex') }}
          restore-keys: |
            r4-pages-target-v2-${{ hashFiles('Cargo.lock') }}-
            r4-pages-target-v2-
   ```
3. Update `Prune Cargo incremental cache` (`lines 74-79`) to safely prune stale deps executables alongside incremental dirs:
   ```yaml
      - name: Prune Cargo incremental cache
        if: always()
        shell: bash
        run: |
          rm -rf target/debug/incremental target/wasm32-unknown-unknown/release/incremental || true
          if [ -d target/debug/deps ]; then
            find target/debug/deps -maxdepth 1 -type f -executable -mtime +1 -delete || true
          fi
   ```

- [ ] **Step 4: Run `overhaul_e2e` F01 & `browser_closure` `pages` checks to verify pass**

Run:
```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --test overhaul_e2e test_t1_f01_pages_workflow && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --test browser_closure pages_workflow
```
Expected: All `test_t1_f01_pages_workflow*` tests and `browser_closure` `pages.yml` contract tests PASS.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/pages.yml crates/wasm/tests/overhaul_e2e/tier1_feature.rs
git commit -m "ci(wave0): split pages concurrency and use rolling cargo target cache"
```

---

### Task 2: Align `Cargo.toml` Profiles & Deduplicate `xtask` Engine Test Compilation

**Files:**
- Modify: `Cargo.toml:33-43`
- Create: `xtask/src/lib.rs`
- Modify: `xtask/src/main.rs:8-34, 160-165`
- Create: `xtask/tests/empirical_suite.rs`
- Move/Modify: `xtask/tests/empirical_challenge_{m2,m2_iter2,m3_api,m4_iter2_chal2,m4_provenance_concurrency}.rs` -> `xtask/tests/empirical_suite/{m2,m2_iter2,m3_api,m4_iter2_chal2,m4_provenance_concurrency}.rs`

**Interfaces:**
- Produces `xtask::Fail` (`pub type Fail = Box<dyn std::error::Error>;`) and `pub mod { audit, bootstrap, engine, lexlean_generated, lut_stack_exec, otel, prism_holo_compiler, routes_and_measures, wasm_ast_eval };` in `xtask/src/lib.rs` (`0` `fn` items).
- Produces `crate::XTASK_DAEMON_LOCK: std::sync::Mutex<()>` and `crate::shared_bootstrap_lut() -> std::sync::Arc<xtask::lut_stack_exec::LutStackModel>` (`OnceLock`) in `xtask/tests/empirical_suite.rs`.

- [ ] **Step 1: Move the 5 `xtask/tests/` files into `xtask/tests/empirical_suite/` and write completeness test**

1. Create `xtask/tests/empirical_suite.rs` with:
   - `pub(crate) static XTASK_DAEMON_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());`
   - `pub(crate) static BOOTSTRAP_BYTES: &[u8] = include_bytes!("../../crates/wasm/src/bootstrap_v8.uorlut");`
   - `pub(crate) fn shared_bootstrap_lut() -> std::sync::Arc<xtask::lut_stack_exec::LutStackModel>` backed by `OnceLock`
   - Submodule declarations with outer `#[allow(dead_code, unused_imports, clippy::all)] mod m2; mod m2_iter2; mod m3_api; mod m4_iter2_chal2; mod m4_provenance_concurrency;`
   - `#[test] fn test_wave0_xtask_empirical_suite_preserves_all_42_tests()` scanning `xtask/tests/empirical_suite/*.rs` to verify `count >= 42` for `#[test]`, `0` occurrences of `#[ignore]`, and `0` occurrences of `#[path = "../src/engine.rs"]`.
2. Move `xtask/tests/empirical_challenge_*.rs` into `xtask/tests/empirical_suite/` (using `git mv`), remove crate-level `#![allow(...)]` and `#[path = ...]` `mod engine;` / `mod lut_stack_exec;` / `mod otel;` includes, switch imports to `xtask::{engine, lut_stack_exec, otel}`, use `crate::shared_bootstrap_lut()` / `crate::BOOTSTRAP_BYTES`, and guard `TestServerDaemon::spawn` / spawned `xtask` server tests in `m3_api`, `m4_iter2_chal2`, and `m4_provenance_concurrency` with `let _daemon_guard = crate::XTASK_DAEMON_LOCK.lock().unwrap_or_else(|e| e.into_inner());`.

- [ ] **Step 2: Run `cargo test -p xtask --test empirical_suite` to verify failure before `xtask/src/lib.rs` exists**

Run:
```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p xtask --test empirical_suite
```
Expected: FAIL with `unresolved import xtask` (`xtask/src/lib.rs` does not exist yet).

- [ ] **Step 3: Create `xtask/src/lib.rs`, wire `xtask/src/main.rs`, and align `Cargo.toml` profiles**

1. Create `xtask/src/lib.rs` containing ONLY `#![allow(missing_docs)]`, `pub type Fail = Box<dyn std::error::Error>;`, and the 9 `pub mod` declarations matching `xtask/src/main.rs:13-31` (`audit`, `bootstrap`, `engine`, `routes_and_measures`, plus the `#[allow(dead_code, clippy::all)] #[path = "../../crates/wasm/src/..."] pub mod ...` exports for `lexlean_generated`, `lut_stack_exec`, `otel`, `prism_holo_compiler`, `wasm_ast_eval`). Zero `fn` items in `lib.rs`.
2. Update `xtask/src/main.rs` to replace `mod audit; ... mod wasm_ast_eval;` and local `type Fail` with `use xtask::{audit, bootstrap, engine, lexlean_generated, lut_stack_exec, otel, prism_holo_compiler, routes_and_measures, wasm_ast_eval, Fail};`. Promote any `pub(crate)` item in `xtask/src/engine.rs` or sibling modules accessed by `main.rs` or `empirical_suite` (such as `serve_listener_until_stopped` and `api`) to `pub` without moving CLI `fn`s out of `main.rs`.
3. Update `Cargo.toml` profiles (`lines 33-43`):
   ```toml
   [profile.dev]
   debug = false

   [profile.dev.package."*"]
   opt-level = 2

   [profile.dev.package.lexlean-r4-wasm]
   opt-level = 2

   [profile.test]
   opt-level = 0
   debug = false

   [profile.test.package."*"]
   opt-level = 2

   [profile.test.package.lexlean-r4-wasm]
   opt-level = 2
   ```

- [ ] **Step 4: Verify consolidated `xtask` suite + `lift-register` / `ratchet-check` / `synthesize` pass**

Run:
```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p xtask --test empirical_suite && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p xtask --lib && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo run -q -p xtask -- lift-register && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo run -q -p xtask -- ratchet-check && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo run -q -p xtask -- synthesize
```
Expected: `empirical_suite` prints `43 passed; 0 failed` (`42 + 1`), `xtask --lib` runs `engine::tests` once cleanly, `lift-register` / `ratchet-check` pass with zero UE-16 regressions, and `synthesize check ok` reports `complexity=344 defects=0`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml xtask/
git commit -m "perf(wave0): align cargo dev/test profiles and deduplicate xtask engine tests"
```

---

### Task 3: Cache `StudioEngine` Baseline Snapshot & Consolidate 17 Wasm Empirical Suites

**Files:**
- Modify: `crates/wasm/src/mvm.rs:694-725` (plus nested private state structs in `crates/wasm/src/mvm.rs`)
- Modify: `crates/wasm/src/lib.rs:486-538, 1359-1436`
- Create: `crates/wasm/tests/empirical_suite.rs`
- Move/Modify: 17 standalone `crates/wasm/tests/empirical_challenge_{3d_math,chatbox,m1,m1_challenger,m2_challenger1,m2_iter2_chal1,m2_iter3_chal1,m2_iter3_chal2,m2_iter4_chal1,m2_iter4_chal2,m3_chal2,m3_challenger1,m3_closure,m4_budget_stress,m4_iter2_chal1,m4_otel,terminal}.rs` -> `crates/wasm/tests/empirical_suite/*.rs` (keep `empirical_challenge_next_token_arch.rs` + `next_token_arch/` intact).

**Interfaces:**
- Produces `OnceLock`-cached baseline state in `crates/wasm/src/lib.rs` (tagged `/// Formal R4.StudioFrame`) so `StudioEngine::new_default()` clones already-registered tokens, 20 admitted knowledge turns, starter workspace files, and booted `MvmSandbox` in `< 0.5 ms`.
- Produces `crates/wasm/tests/empirical_suite.rs` exporting `pub(crate) static STUDIO_DISPATCH_LOCK: std::sync::Mutex<()>`, `pub(crate) static BOOTSTRAP_BYTES: &[u8]`, and `pub(crate) fn shared_bootstrap_lut() -> std::sync::Arc<lexlean_r4_wasm::lut_stack_exec::LutStackModel>`.

- [ ] **Step 1: Create `crates/wasm/tests/empirical_suite.rs` with 98-test preservation guard & move the 17 submodules**

1. Create `crates/wasm/tests/empirical_suite.rs` with:
   - Outer `#[allow(...)]` attributes covering all moved suites
   - `pub(crate) static STUDIO_DISPATCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());`
   - `pub(crate) static BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");`
   - `pub(crate) fn shared_bootstrap_lut() -> std::sync::Arc<lexlean_r4_wasm::lut_stack_exec::LutStackModel>` returning `lexlean_r4_wasm::api::default_model_backend()` (on native `bootstrap_v8.uorlut`)
   - 17 `#[path = "empirical_suite/..."] mod ...;` declarations
   - `#[test] fn test_wave0_wasm_empirical_suite_preserves_all_98_tests()` scanning `crates/wasm/tests/empirical_suite/*.rs` to verify `count >= 98` for `#[test]` and `0` `#[ignore]` attributes.
2. `git mv` all 17 standalone `empirical_challenge_*.rs` files into `crates/wasm/tests/empirical_suite/`, remove crate-level `#![allow(...)]` headers, replace per-file `BOOTSTRAP_BYTES` / `load_bootstrap` / `load_test_model` / `get_test_model` definitions with `crate::BOOTSTRAP_BYTES` / `crate::shared_bootstrap_lut().as_ref().clone()`, and switch `m2_iter3_chal1` + `m2_iter4_chal1` `STUDIO_DISPATCH_LOCK` references to `crate::STUDIO_DISPATCH_LOCK`.

- [ ] **Step 2: Add `/// Formal R4.StudioFrame` baseline cache test in `crates/wasm/src/lib.rs` (TDD)**

Inspect `crates/wasm/src/lib.rs` tests and add a unit test `test_wave0_studio_engine_new_default_clones_cached_baseline_deterministically` verifying:
- Calling `StudioEngine::new_default()` twice produces identical `turns.len()`, `workspace_files` receipts, and `mvm.cow_state_sha256()`, and mutating `e1.workspace_files` or `e1.mvm` leaves `e2` completely unmutated.
Run targeted unit test before/during implementation to confirm compilation & isolation contract.

- [ ] **Step 3: Derive `Clone` on `MvmSandbox` and cache `StudioEngine::new_default()` baseline via `OnceLock`**

1. Add `#[derive(Clone)]` on `MvmSandbox` and its private helper types (`GuestInode`, `HostSecretRule`, `InstalledPackageMeta`, etc.) in `crates/wasm/src/mvm.rs`.
2. In `crates/wasm/src/lib.rs`, introduce a private `#[derive(Clone)] struct StudioBaselineSnapshot` (populated once inside a `static STUDIO_BASELINE: OnceLock<StudioBaselineSnapshot>` initialized on first `StudioEngine::new_default()`, pre-running `register_special_tokens`, `admit_turn` for `default_knowledge_turns()`, `init_default_workspace()`, `sync_workspace_into_mvm()`, and `mvm.record_workspace_baseline_from_map(...)`), tagged `/// Formal R4.StudioFrame` so zero `residue` functions are added.
3. Have `StudioEngine::new_default()` assemble a fresh `StudioEngine` directly by cloning `STUDIO_BASELINE` (`vocab`, `turns`, `opening_table`, `reply_table`, `point_table`, `occupied_cells`, `workspace_files`, `mvm`, plus shared `Arc` handles `lut_stack`, `gllm_v9`, etc.).

- [ ] **Step 4: Run consolidated Wasm empirical suite + full gate checks + cleanup `target/`**

Run:
```bash
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --lib test_wave0_studio_engine_new_default_clones_cached_baseline_deterministically && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --test empirical_suite && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo test -p lexlean-r4-wasm --test overhaul_e2e && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo run -q -p xtask -- validate && \
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
  cargo clippy --workspace --all-features --all-targets -- -D warnings && \
rm -rf target && df -h /
```
Expected:
- `empirical_suite`: `99 passed; 0 failed` (`98 + 1` completeness test)
- `overhaul_e2e`: all tests pass
- `xtask -- validate`: `synthesize check ok: score_milli=987 complexity=344 defects=0 ... verdict=CandidateAdmitted`
- `clippy --workspace --all-features --all-targets -- -D warnings`: `0` warnings
- `df -h /` shows `>= 30 GiB` free after `rm -rf target`.

- [ ] **Step 5: Commit Spec, Plan & Wave 0, then push & time GitHub Pages CI run**

```bash
git add docs/superpowers/ crates/wasm/src/ crates/wasm/tests/
git commit -m "perf(wave0): cache StudioEngine baseline and consolidate wasm empirical suites"
git push origin HEAD:main
```
Wait for `pages` GitHub Actions workflow on the pushed SHA to complete (`gh run watch`) and verify total build time drops cleanly from ~36m 31s (before opening Wave 1).
