# Connected Chat Studio & GLLM v10 Closure Verification (`R4-39`)

## Scope

This project-owned verification record documents the integrated `R4-39` closure across:

1. **LexLean Model & Conformance (`R4-39`)**:
   - Verified `.lex.tex` model translation (`crates/wasm/src/lexlean_generated.rs`) and `CONFORMANCE.md` (`39` registered IDs) checked by `cargo run -p xtask -- validate` and `cargo run -p xtask -- check-model`.
   - Typed `R5` refusal admission for `InsecureTransportRejected`, `HostNotAllowlistedForSecret`, `PolicyDenied`, `InvalidVsockCid`, `GuestRamSecretContamination`, `PackageDigestMismatch`, `PackageInvalidElf64`, and `PackageSizeExceedsQuota`.
2. **SpiralCore Edge & Local Relay (`relay/`)**:
   - Live Cloudflare Worker (`wss://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs`) and local Node relay (`relay/local-relay.mjs`), verified by `node --test relay/relay.test.mjs` (`9/9` passing).
3. **UOR-R4-GLLM v10 (`16L × 256D` OPFS Shards + `6L × 128D` Bootstrap Wasm `< 15 MiB`, Pure-Rust Tooling)**:
   - Sharded `UORGLLM9` v10 `16L × 256D` container (`gpu/weights/v10/manifest.json`, `82,427,606` bytes across `79` × `1 MiB` shards `shard_000.bin..shard_078.bin`, SHA-256 `c886437310d9c817bf745f97b71b78dee09bd7dda1a1ff149bc9749cb4b0c408`, `98.0466%` post-QAT eval accuracy) and `10/10` SHA-256-verified static `riscv64-linux-musl` (`EM_RISCV = 243`) binaries in `gpu/weights/mvm-pkgs/` built with pure Rust (`gpu/mvm_guest/build_pkgs.rs`, zero `.py` files under `gpu/`).
   - Embedded `6L × 128D` bootstrap (`crates/wasm/src/bootstrap_v9.uorgllm`, `10,010,406` bytes, SHA-256 `9c5b38cbbe1e4d7122dddb1867ad9c085abd68b22cd8f983725932786c955d9b`, `99.0115%` post-QAT eval accuracy) yielding `public/lexlean_r4_bg.wasm` (`12,533,609` bytes / `11.95 MiB < 15 MiB`) bound by SHA-256 (`2cf02dda82007fd2dc0671232d187ab0ea4e4650d1bb50d2ca3cb791c8535cc4`) in `public/provenance.json`.
4. **Zero-Freeze Dual-Mode Worker UI & Strict 6-File Closure (`public/*`)**:
   - Chat-First default landing (`body[data-layout-mode="chat"]`) with `0 ms` main-thread Wasm boot freeze via dedicated `uor-r4-engine-worker` (`public/lexlean_r4.js`), instant `Studio IDE` toggle (`#toggle-studio-ide-btn`), clean Linux MicroVM bottom terminal (`casey@mvm:/workspace$`), full-height `#inspectors-modal` (`86vh`), live `brain_pulse` 3D cortical & Corpus Callosum animation, and host-vault `GITHUB_TOKEN` isolation (`0` secret bytes in guest RAM).

---

## Planted-Defect Gate Evidence (`AGENTS.md` Gate Quality)

### Planted Defect 1 — 7th File in `public/` Closure

- **Planted defect**: Created `public/planted_defect.tmp` (`echo "defect" > public/planted_defect.tmp`) violating the strict 6-file browser closure (`app.css`, `app.js`, `index.html`, `lexlean_r4.js`, `lexlean_r4_bg.wasm`, `provenance.json`).
- **Failing command**:
  ```sh
  cargo test -p lexlean-r4-wasm --test browser_closure
  ```
- **Diagnostic**:
  ```text
  test test_browser_closure_contains_exact_six_files ... FAILED
    left: ["app.css", "app.js", "index.html", "lexlean_r4.js", "lexlean_r4_bg.wasm", "planted_defect.tmp", "provenance.json"]
  test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
  ```
- **Restored result**: Removed `public/planted_defect.tmp` and re-ran `cargo test -p lexlean-r4-wasm --test browser_closure` -> `10 passed; 0 failed`.

### Planted Defect 2 — Non-Loopback Cleartext `http://` Endpoint in `public/app.js`

- **Planted defect**: Appended `// http://evil.example.com/cleartext` to `public/app.js`, violating the `R4-39` TLS-only (`https://` / `wss://` or loopback `127.0.0.1` / `localhost`) browser transport boundary.
- **Failing command**:
  ```sh
  cargo test -p lexlean-r4-wasm --test browser_closure
  ```
- **Diagnostic**:
  ```text
  test test_browser_closure_zero_external_network_telemetry ... FAILED
  public/app.js contains non-loopback unencrypted http:// reference at byte 494962
  test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
  ```
- **Restored result**: Restored `public/app.js` byte-for-byte and re-ran `cargo test -p lexlean-r4-wasm --test browser_closure` (`10 passed; 0 failed`) and `node tests/e2e/challenger_m1_2_adversarial_stress.mjs` (`VERDICT: APPROVE`, `0` defects).

### Planted Defect 3 — Zero-Python Gate (`gpu/defect.py`)

- **Planted defect**: Created `gpu/defect.py` (`printf '# defect\n' > gpu/defect.py`) violating the zero-Python pure-Rust rule in `gpu/`.
- **Failing command**:
  ```sh
  cargo test -p lexlean-r4-wasm --test frontier_e2e tier1_feature::test_f14_synthetic_generator_cli_flags_invocation
  ```
- **Diagnostic**:
  ```text
  thread 'tier1_feature::test_f14_synthetic_generator_cli_flags_invocation' (25) panicked at crates/wasm/tests/frontier_e2e/tier1_feature.rs:855:17:
  unexpected Python file remaining under gpu/: /work/gpu/defect.py
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.00s
  ```
- **Restored result**: Removed `gpu/defect.py` and re-ran `cargo test -p lexlean-r4-wasm --test frontier_e2e tier1_feature::test_f14_synthetic_generator_cli_flags_invocation` -> `1 passed; 0 failed`.

### Planted Defect 4 — `v10` 79-Shard Manifest Digest Integrity (`gpu/weights/v10/manifest.json`)

- **Planted defect**: Flipped the first hex digit of `"container_sha256"` in `gpu/weights/v10/manifest.json` from `c8864373...` to `d8864373...`.
- **Failing command**:
  ```sh
  cargo test -p lexlean-r4-wasm --lib test_v10_6l_bootstrap_and_8l_opfs_shards_end_to_end_r4_39
  ```
- **Diagnostic**:
  ```text
  thread 'tests::test_v10_6l_bootstrap_and_8l_opfs_shards_end_to_end_r4_39' (24) panicked at crates/wasm/src/lib.rs:10729:14:
  gllm_load_v10_shards must verify and load 79 v10 shards: ContainerDigestMismatch { expected: "d886437310d9c817bf745f97b71b78dee09bd7dda1a1ff149bc9749cb4b0c408", actual: "c886437310d9c817bf745f97b71b78dee09bd7dda1a1ff149bc9749cb4b0c408" }
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 78 filtered out; finished in 9.14s
  ```
- **Restored result**: Restored `gpu/weights/v10/manifest.json` byte-for-byte from backup and re-ran `cargo test -p lexlean-r4-wasm --lib test_v10_6l_bootstrap_and_8l_opfs_shards_end_to_end_r4_39` -> `1 passed; 0 failed`.
