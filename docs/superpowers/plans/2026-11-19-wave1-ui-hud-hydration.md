# Wave 1 (`R4-62`): Compact Cursor-Grade UI, Non-Blocking Dropdown Neural HUD & Zero-Prompt Auto-Hydration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restore a crisp, non-overlapping Cursor-grade web studio UI, replace the blackout Observatory modal with a floating non-blocking topbar Dropdown Neural HUD, clean up user-facing model dropdowns to the 2 active Qwen3 models (`0.6B` & `1.7B`), and deliver zero-prompt first-message auto-hydration with resilient shard parity retry and bounded JS heap usage.

**Spec:** [2026-11-19-lexlean-r4-mvm-studio-redesign-design.md](file:///Users/casey.allard/lexlean-r4/docs/superpowers/specs/2026-11-19-lexlean-r4-mvm-studio-redesign-design.md) (Wave 1 / `R4-62`).

---

## Mandatory Disk & RAM Guardrails

1. Run **one Cargo crate/test binary at a time** with `CARGO_BUILD_JOBS=2` (never `cargo test --workspace` locally) and `-- --test-threads=2` on large integration binaries.
2. Run `rm -rf target && df -h /` immediately after every Cargo verification batch and Wasm build, keeping `>= 20 GiB` free on `/`.
3. Run Node E2E scripts sequentially (`NODE_OPTIONS="--max-old-space-size=1536"`) — never in parallel — and null out downloaded 1 MiB piece buffers immediately after pushing to Wasm (`res.bytes = null`).

---

### Task 1: Authoritative `R4-62` LexLean Model, Route/Measure Ledgers & Conformance Suite (`R1`, `R2`, `R3`, `R5`)

- Extend [StudioFrame.lex.tex](file:///Users/casey.allard/lexlean-r4/src/R4/StudioFrame.lex.tex) (`R4.StudioFrame`) and [StudioFrameControls.lex.tex](file:///Users/casey.allard/lexlean-r4/tests/lexlean/StudioFrameControls.lex.tex) with `HydrationOriginPolicy`, `CatalogHydrationVerdict`, `HudPresentationMode`, `HudBackdropVerdict`, `maxCatalogHydrationConcurrency`, `admitCatalogHydration`, `admitHudPresentation`, `verifyCompactStudioAutoHydrationAdmitted`, and `breaks*` counterexample theorems.
- Mirror enums/functions in [lexlean_generated.rs](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/lexlean_generated.rs) and add `studio_admit_catalog_hydration_json` (`/// Formal R4.StudioFrame`) in [crates/wasm/src/lib.rs](file:///Users/casey.allard/lexlean-r4/crates/wasm/src/lib.rs).
- Register `R4-62` in [model/ids.toml](file:///Users/casey.allard/lexlean-r4/model/ids.toml), `R4-UR13` (`compact-studio-auto-hydration`) in [model/routes.toml](file:///Users/casey.allard/lexlean-r4/model/routes.toml), bump `Quality::measureRouteTableTotal` to `13` in [model/measures.toml](file:///Users/casey.allard/lexlean-r4/model/measures.toml), regenerate [CONFORMANCE.md](file:///Users/casey.allard/lexlean-r4/CONFORMANCE.md) via `xtask -- check-model --write`, create `features/suites/r4_62_compact_studio_auto_hydration.feature`, add `crates/conformance/tests/r4_62_compact_studio_auto_hydration.rs` (`test_r4_62_compact_studio_auto_hydration_r4_62`), and update the ID count (`61 -> 62`) in `bdd.rs` & `overhaul_e2e`.

### Task 2: Zero-Prompt Auto-Hydration, `CONCURRENCY = 3` Retry, Stale Manifest Self-Healing & Heap-Safe Streaming

- Update [worker_host_bridge.js](file:///Users/casey.allard/lexlean-r4/crates/wasm/js/worker_host_bridge.js) (`/** Formal R4.StudioFrame */` on every new helper):
  - Admit first-party catalog hydration (`0.6B` & `1.7B`) without blocking consent modals unless `allowOffOriginHydration: false` or untrusted external mirror URLs are used (`RefusedExternalOriginConsentRequired`).
  - Eliminate silent sticky `uor_allow_off_origin_hydration === 'declined'` downgrades to `uor-r4.native-q31-d85` (`RefusedSilentModelDowngrade`) unless Settings Offline Emergency Mode (`uor_offline_emergency_q31 === '1'`) or an explicit request `offlineFallback` flag is set.
  - Lower download batch concurrency from `6` to `3`, add 3-attempt backoff retry in `holoFetchWithHoldTail`, fetch manifests with `{ cache: 'no-cache' }`, overwrite stale `uor_holo_kappa_v1` IndexedDB entries on `container_sha256` drift, null `res.bytes = null` immediately after streaming to Wasm, and emit accurate `downloadedBytes` / `totalBytes` / `percent` progress events.
- Update and verify `tests/e2e/foundry_worker_stepper_unit.mjs` and `tests/e2e/foundry_observatory_e2e.mjs`.

### Task 3: 2-Model Picker, Honest Status/Progress Pill & Settings Offline Emergency Toggle

- Restrict `COMPOSER_MODELS`, `#model-select`, and `#composer-model-select` in [composer.ts](file:///Users/casey.allard/lexlean-r4/crates/wasm/ts/composer.ts), [index.html](file:///Users/casey.allard/lexlean-r4/public/index.html), and [app.js](file:///Users/casey.allard/lexlean-r4/public/app.js) to `uor-r4.qwen3-0.6b-holo` (`Qwen3 0.6B · Fast (410 MB)`) and `uor-r4.qwen3-1.7b-holo` (`Qwen3 1.7B · Reasoning (1.1 GB)`), keeping `uor-r4.native-q31-d85` & `uor-r4.hf-repo-holo` only for Settings Emergency Mode and programmatic `set-model` API calls.
- Wire `#settings-offline-q31-toggle` and clickable `#weight-cache-badge` (`410 MB · Auto-downloads on send` -> `Downloading XX% · YY / 410 MB` -> `Ready · WebGPU` / `Ready · Wasm SIMD` + inline chat progress bar).
- Update & verify `empirical_challenge_chatbox.rs`, `empirical_chatbox_harness.mjs`, `honest_model_plumbing_e2e.mjs`, and `github_roundtrip_and_stitch_e2e.mjs`.

### Task 4: Cursor-Grade Compact CSS Geometry, 3-Bar Studio Workbench & Non-Blocking Dropdown Neural HUD

- Fix desktop sizing in [app.css](file:///Users/casey.allard/lexlean-r4/public/app.css) (`24–28px` buttons/inputs/selects, `38px` topbar, `30px` tab bar, `22px` breadcrumb+diagnostics bar, `26px` terminal header, `22px` status bar; scope `44px` touch rules strictly to `@media (max-width: 599px), (pointer: coarse) and (hover: none)`; hide `#observatory-dock-split` `0px` so Studio chat fills `100%` column height).
- Turn `#inspectors-modal` (`[⚡ HUD ▾]`) into a floating non-blocking right-aligned Dropdown Neural HUD (`76px` compact live ribbon `#neural-hud-compact-strip` + expandable 4-canvas deck via `#neural-hud-expand-btn` / `#neural-hud-close-btn`, zero backdrop overlay in Studio/Chat modes, `<= 64`-frame ring & `requestAnimationFrame` canvas coalescing).
- Update & verify `empirical_gui_state_machine_challenge.mjs`, `foundry_observatory_e2e.mjs`, `empirical_challenge_m3_closure.rs`, and `browser_closure.rs`.

### Task 5: Wasm Bundle Rebuild, Planted-Defect Evidence, Full Sequential Verification, Review, Push & Live Verification

- Run `build-wasm`, `build-holo --sync-site-manifest`, `lift-register --write` (`residue <= 344`), `synthesize --write`, `ratchet-check`.
- Plant & log `R4-62` negative gate defect in [OVERHAUL-VERIFICATION.md](file:///Users/casey.allard/lexlean-r4/OVERHAUL-VERIFICATION.md).
- Run all Rust & sequential Node E2E checks + `rm -rf target`, pass fresh `pro` code review, fast-forward `main` & push, monitor CI via `schedule` (`pages` + `bootstrap`), and verify live deployed UI across `1440×900`, `1280×720`, and `390×844` via `browser` subagent before entering Wave 2 (`R4-63`).
