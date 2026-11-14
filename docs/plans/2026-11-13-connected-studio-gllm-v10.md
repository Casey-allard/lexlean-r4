# Connected Chat Studio & GLLM v10 Closure Verification (`R4-39`)

## Scope

This project-owned verification record documents the integrated `R4-39` closure across:

1. **LexLean Model & Conformance (`R4-39`)**:
   - Verified `.lex.tex` model translation (`crates/wasm/src/lexlean_generated.rs`) and `CONFORMANCE.md` (`39` registered IDs) checked by `cargo run -p xtask -- validate` and `cargo run -p xtask -- check-model`.
   - Typed `R5` refusal admission for `InsecureTransportRejected`, `HostNotAllowlistedForSecret`, `PolicyDenied`, `InvalidVsockCid`, `GuestRamSecretContamination`, `PackageDigestMismatch`, `PackageInvalidElf64`, and `PackageSizeExceedsQuota`.
2. **SpiralCore Edge & Local Relay (`relay/`)**:
   - Live Cloudflare Worker (`wss://lexlean-r4-spiralcore-relay.swarm-relay-cloud-8d31a122.workers.dev/spvs`) and local Node relay (`relay/local-relay.mjs`), verified by `node --test relay/relay.test.mjs` (`9/9` passing).
3. **UOR-R4-GLLM v10 (`8L 128D` OPFS Shards + `< 15 MiB` Slim Bootstrap Wasm)**:
   - Sharded `UORGLLM9` v10 container (`gpu/weights/v10/manifest.json`, `3` shards) and `10/10` SHA-256-verified static `riscv64-linux-musl` (`EM_RISCV = 243`) binaries in `gpu/weights/mvm-pkgs/`.
   - Slim `public/lexlean_r4_bg.wasm` (`5,959,591` bytes / `5.68 MiB < 15 MiB`) bound by SHA-256 in `public/provenance.json`.
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
