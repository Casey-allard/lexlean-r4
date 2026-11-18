Feature: Polyglot Wasm Container Runtime & Host Adapters (R4-58)

  @R4-58 @build
  Scenario: R4-58 MVM NeedsBridge dispatch and Worker adapters for node/npm/esbuild, SRI-pinned Pyodide, WASI Preview 1, and sqlite3
    Given an MVM-Wasm sandbox synced with StudioEngine and Worker host bridge adapters
    When the guest executes node, npm, npx, python, python3, pip, wasm run, or sqlite3 commands over /workspace
    Then MVM emits typed NeedsBridge requests and resumes deterministically via mvm_complete_bridge_json while keeping node_modules/**, *.db, and *.wasm in OPFS COW sectors and enforcing canonical protection on prism/*.lex.tex and app/state
    And node/npx executes isolated JS with shimmed fs/path/process/buffer/crypto/url/fetch and reuses host-configured Foundry esbuild.wasm (returning EsbuildAdapterUnavailable if unconfigured for TS/ESM)
    And npm install verifies SHA-512 SRI integrity (refusing tampered packages with IntegrityMismatch) and extracts valid tarballs into /workspace/node_modules/ while updating package.json
    And python/pip lazy-loads Pyodide strictly after exact PYODIDE_LOCK SHA-384 SRI verification into Cache API uor-pyodide-v1, wasm run executes WASI Preview 1 _start modules via wasi_snapshot_preview1, and sqlite3 persists binary /workspace/*.db state cleanly
