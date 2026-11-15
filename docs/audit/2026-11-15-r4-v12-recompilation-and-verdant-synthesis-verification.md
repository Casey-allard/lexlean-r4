# UOR-R4 v12 Recompilation, Verdant Synthesis, and Honest Plumbing Verification

**Date**: 2026-11-15  
**Target Repository**: `Casey-allard/lexlean-r4`  
**Governing Standard**: `uor/template-contract/1` (`AGENTS.md` Rules `R1`–`R6`, `template.lock`)  
**Capabilities**: `R4-44`, `R4-45`, `R4-46`

## 1. Registered Capabilities & Formal Verification

1. **`R4-44` — Direct Teacher Recompilation into End-to-End No-Matmul R4 Vector Geometry**
   - Formalized in `src/R4/GeometricRecompiler.lex.tex` and `tests/lexlean/GeometricRecompilerControls.lex.tex`, generated into `crates/wasm/src/lexlean_generated.rs::r4_geometric_recompiler`, and checked in `crates/conformance/tests/geometric_recompiler.rs`.
   - Teacher recompilation source & receipt (`gpu/recompile_r4_teacher.rs`, `gpu/rtx4090_r4_recompiled_v12_receipt.json`):
     - Teacher model: `HuggingFaceTB/SmolLM2-135M-Instruct` (`config.json` SHA-256 `8eb740e8bbe4cff95ea7b4588d17a2432deb16e8075bc5828ff7ba9be94d982a`, `model.safetensors` SHA-256 `5af571cbf074e6d21a03528d2330792e532ca608f24ac70a143f6b369968ab8c`).
     - Full 30-layer container (`uor-r4.geometric-vector-stack/12`, `container_version = 12`): `75,684,407` bytes across `73` data shards and `37` Stitch 2-of-3 XOR parity groups under `gpu/weights/v10/`, SHA-256 `fa4d31131213309bcadae0155067a86c7ef15d6383cc4bce5d23953a7dad30c3` (`n_layers = 30`, `d_model = 576`, `n_heads = 9`, `n_kv_heads = 3`, `vocab_size = 49152`, `rope_theta = 100000`, `tied_lm_head = true`, `rms_norm_preserved = true`, quantized via `block32-wht+group32-dyadic-4bit-per-row-exp+conway-sloane-e8-audit`).
     - Compact distilled embedded seed (`crates/wasm/src/bootstrap_v12_seed.uorr4`, protected by `*.uorr4 binary` in `.gitattributes`): `4,433,277` bytes, SHA-256 `31dfbeb82ea14ed4218f0ab5445f5e4e3a246e808c8f400ea72eec0ba7fa8fa5` (`n_layers = 8`, `d_model = 256`, `n_heads = 4`, `n_kv_heads = 2`, `vocab_size = 6144`).
     - Serving invariants in `crates/wasm/src/r4_geometric_stack.rs`: `float_ops = 0`, `dense_matmul_ops = 0`; teacher parity over 12 prompts: `top1_logit_parity_pct = 75.00%`, `top5_logit_parity_pct = 100.00%`, `mean_kl_divergence = 0.635270`, `total_nibble_lut_lookups = 81738989568`, `total_lorentz_geodesic_evals = 22543380`.

2. **`R4-45` — Verdant AST Mechanizer, UORust κ Lift-Register, 4-Facet Synthesis & Shrink-Only Ratchet**
   - Formalized in `src/R4/VerdantSynthesis.lex.tex` and `tests/lexlean/VerdantSynthesisControls.lex.tex`, generated into `crates/wasm/src/lexlean_generated.rs::r4_verdant_synthesis`, and checked in `crates/conformance/tests/verdant_synthesis.rs`.
   - Implemented in `xtask/src/{mechanizer,lift_register,synthesis,ratchet,engine,main}.rs` with outputs `model/lift_register.json` (`total=2337`, `lifted=1644`, `effect=214`, `foreign=130`, `residue=349 <= 355`), `model/ratchet.toml` (`max_residue_functions = 355`, `max_float_serving_ops = 0`, `max_dense_matmul_ops = 0`, `max_unwired_ui_controls = 0`), and `model/synthesis_report.json` (`score_milli=985`, `complexity=349`, `defects=0`, `strategy=pure-integer-lut-serving`, `verdict=CandidateAdmitted`).

3. **`R4-46` — Honest Model Routing, UI Cleanup, OPFS Shard Hydration & `.uorc` Watchdog**
   - Formalized in `src/R4/HonestPlumbing.lex.tex` and `tests/lexlean/HonestPlumbingControls.lex.tex`, generated into `crates/wasm/src/lexlean_generated.rs::r4_honest_plumbing`, and checked in `crates/conformance/tests/honest_plumbing.rs`.
   - Wired `#composer-model-select` (`uor-r4.recompiled-v12-agent`, `uor-r4.recompiled-v12-chat`, `uor-r4.seed-fast`, `uor-r4.lut-symbolic`), eager OPFS `/weights/v10/manifest.json` hydration await in `crates/wasm/js/worker_host_bridge.js`, 3-strike `.uorc` rollback admission in `crates/wasm/src/lib.rs`, and the exact 6-file `public/` closure (`lexlean_r4_bg.wasm` = `9,375,758` bytes < 15 MiB budget).

4. **SDK Formal Pipeline (`ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e`)**
   - `lexlean lock`, `lexlean fmt` (0 diffs), and `lexlean verify` passed on all 6 new `.lex.tex` modules (`attestation 611cb0a0f774cc33a77c0a36bfaf0bfc4cccaeec5447a62bb9864e0dce857f91`).
   - `xtask api sync` (`claims=243, theorems=235`), `xtask check-model --write`, and `xtask check-wasm-sync` (`core=de11d242acde wasm=951713971572 js=cc7ec45e887c`) passed.

---

## 2. Planted-Defect Gate Proofs (`AGENTS.md` Gate Quality)

Every gate below was executed inside `ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:36da25ef924d84899453f24ec0c117a7a220e708037933eb8b499caa7723d99e`, observed to fail on a concrete planted defect, and verified green again after restoring the production file.

| Gate / Capability | Planted Defect | Failing Command | Observed Diagnostic & Exit Code | Restored Green Result |
| --- | --- | --- | --- | --- |
| `R4-44` (`ratchet-check`) | Inserted `pub fn planted_float_regression_r4_44(x: u32) -> f32 { (x as f32) + 1.0_f32 }` before `#[cfg(test)]` in `crates/wasm/src/r4_geometric_stack.rs` | `cargo run -q -p xtask -- ratchet-check` | `exit 1`: `gate failed: shrink-only ratchet refused float_serving_ops: measured=3 ceiling=0 verdict=RefusedResidualRegression` | `exit 0`: `ratchet-check: shrink-only + Peirce triadic verified (r30_gen=212, orphans=0, ungrounded=0, ignored=0, residue=349, float=0, matmul=0, unwired_ui=0)` |
| `R4-45` (`ratchet-check`) | Appended `<button id="planted-dead-btn">x</button>` to `public/index.html` | `cargo run -q -p xtask -- ratchet-check` | `exit 1`: `gate failed: shrink-only ratchet refused unwired_ui_controls: measured=1 ceiling=0 verdict=RefusedResidualRegression; unwired_controls=["planted-dead-btn"]` | `exit 0`: `unwired_ui=0` |
| `R4-46` (`synthesize`) | Replaced `template_override_used: false` with `template_override_used: true` at `crates/wasm/src/lib.rs:5583` | `cargo run -q -p xtask -- synthesize` | `exit 1`: `gate failed: synthesize refused (RefusedNonZeroDefect): total_defects=1 > 0` | `exit 0`: `synthesize check ok: score_milli=985 complexity=349 defects=0 strategy=pure-integer-lut-serving verdict=CandidateAdmitted` |
