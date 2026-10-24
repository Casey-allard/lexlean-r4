# Actual whole-read inner connection checkpoint

Current status is **COMPILER_ARTIFACT_VERIFIED_RUN_GATE_FAILED**. See [RESULTS.md](RESULTS.md) and [results.json](results.json): the exact original ObjWholeAdmitted = C consumer passed official verification and independent kernel audit, while the positive memory postgate failed (max 0 to 46; no OOM or kill). Overall run acceptance is not granted.

The intended negative was rejected in the transport dependency, before consumer elaboration. The frozen [candidate](candidate/README.md), [readiness](readiness.json), and earlier custody records preserve their preparation-time and pending-review observations. They are historical, not current NOT_RUN claims. The previous README is preserved under evidence/history.

[source-bindings.json](source-bindings.json) maps all 22 phase inputs using repository references. Runtime, digest, limits and empty axioms are unchanged. Full successful inner/outer Result composition, original ordinal26, Read33 and full integration remain open. No native execution authority follows from this checkpoint.
