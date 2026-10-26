# Admitted-range artifact and failed resource gate

**Overall phase: UNACCEPTED.** The official compiler and kernel checks succeeded, but the unchanged strict memory-event postcheck failed. These are separate observations; this checkpoint waives no gate and promotes no Object control.

| Observation | Actual evidence |
| --- | --- |
| Original admitted negative | Official verify exit1, intended `LLV7002` at `Eq.trans (Eq.refl 0)`, 184,265 ms; every Child/chain input was positive. Independent review checked 76 raw files and all operational guards. |
| Complete admitted positive | Official verify exit0, no diagnostics, exact33 selected modules, 264,017 ms; artifact `bbcaadebdf50fc99490203fc74c9fa0614dacf18e1d5cf72078dfd41580b4169`. |
| Artifact audit | 335 actual files, 33 oleans, 832 declarations with empty observed axioms, 100 successful process receipts including33 kernel replays. This was an inspection of actual receipts and artifact bytes, not a new compiler replay by the reviewer. |
| Blocking postcheck | `memory.events.max` rose0→6325. `low`, `high`, `oom`, `oom_kill` and `oom_group_kill` remained0. Launcher exit1 and `postcapture-errors` retain the memory failure. |
| Other postconditions | All50 source inputs, genuine development lock, exact tools/container epoch, idle snapshots and host/guest reserves passed inspection. |

The [admitted-negative review](admitted-negative-review.json), [positive artifact review](positive-artifact-review.json), and [read-only Phase1 investigation](positive-memory-phase1.json) preserve the evidence and its limits. The earlier [Child negative](../read-child-negative/README.md) is independently accepted. The frozen [source candidate](../../../../tests/fixtures/object-read-child-certificate-native/README.md) preserves the original statements, runtime, digest, empty-axiom policy and300-second child limits.

The allocation cause is unresolved. The two memory samples enclose staging, lock/check, verification, artifact copy and postcapture hashing. Inner process receipts contain no timestamps, PIDs, RSS or cgroup measurements. Successful kernel receipts and zero OOM counters do not satisfy the all-zero resource gate. They also do not identify a leak, a particular module, a continuous peak, or the point at which the limit was reached. The failed epoch and complete original artifacts remain preserved; there is no reset, retry or relaxed limit in this checkpoint.

## Portable evidence

[manifest.json](manifest.json) authenticates two lossless gzip JSON archives and the three independent reports. `raw-receipts.json.gz` retains all153 root evidence files for the negative and positive phases. `artifact-text.json.gz` retains all302 UTF-8 artifact files, including source, emissions, audit output and process receipts. Every entry contains its relative path, byte count, SHA256 and exact text. Decode with a standard gzip reader, parse JSON, and verify each UTF-8 text against its entry hash. The archive and decoded-document hashes are also pinned. Historical absolute paths in receipts are provenance, not a runnable installation.

The33 compiled `.olean` files are excluded from Git; their exact names, sizes and hashes are preserved in the artifact archive's `excludedOleans` inventory. The original335-file bundle remains at the external location recorded in the manifest. No compiled SDK or replacement application is installed by these archives.

Next: independently review a narrowly bounded diagnostic for the observed resource failure before any new native execution or resource-epoch change. Do not rerun unchanged qualification merely to seek a clean counter, and do not treat artifact inspection as permission to relax the gate. Original ordinal26, focused/full Read33, Construction39, nine pending drafts, required mutations, full integration, production PrismPM/template binding and generated Pages remain open. Accepted Object controls remain331/403.
