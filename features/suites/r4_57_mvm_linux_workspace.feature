Feature: Minimal Linux Coreutils & Filtered Bidirectional /workspace Sync (R4-57)

  @R4-57 @build
  Scenario: R4-57 MVM shell AST executor, native Rust coreutils applets, and filtered OPFS-COW workspace delta sync
    Given an MVM-Wasm sandbox and StudioEngine workspace synced via mvm_sync_workspace_in_json
    When a shell script runs pipelines, heredocs, redirects, environment expansion, and native Rust coreutils applets over MvmCowSnapshot inodes
    Then cat <<'EOF' | grep -E "foo|bar" | sort | uniq -c | awk '{print $2,$1}' > /workspace/out.txt and head, tail, wc, tr, cut, sed, find, cp, mv, touch, chmod, ln, printf, env, export, which, ps, uname, date, tar, gzip, gunzip, base64, diff, bc, and jq execute deterministically without panicking on missing or binary files
    And mvm_export_workspace_diff_json emits minimal base64 writes and deletes while strictly excluding node_modules/**, .venv/**, **/__pycache__/**, .git/objects/**, non-UTF-8 binaries (*.db, *.wasm, >1 MiB), and canonical protected rule paths (^prism/.*\.lex\.tex$, ^app/state$)
    And excluded heavy directories and binaries remain persisted in MVM OPFS COW sectors while invalid sync payloads, traversal paths, and guest writes to canonical rule files produce typed refusals
