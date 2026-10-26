# BLAKE3 second64 state and continuation candidate

This frozen source candidate targets the first actual compression followed by the second packed64 input. **All six native phases are NOT_RUN.** The state theorem is the real production `blake3FeedLiteralRaw(Q,S64)=ok(S128)` with a fully literal eight-word chain and all seven state fields. Its expected-data derivation is retained in `derive-expected.py` and `derivation.json`; those diagnostics are untrusted authoring data until official kernel Reflexivity with empty axioms succeeds.

The six fixed phases are state-negative (only expected c0 incremented), state-positive, lift-negative (only zero Apply changed to unrelated0=0), lift-positive, generic K5-to-K4 consumer and mandatory exact original-tree consumer. No phase auto-advances. Source CODE and separate exact invocation/resource reviews precede any native selection; each actual outcome must be independently reviewed before the next selection. A helper success does not accept either consumer.

`candidate.json` gives exact phase inputs and module closures. Its global sharedInputs is a catalog: use each phase.sharedInputs subset, plus its newStageInputs. Shared production runtime, the accepted first64 certificate and unchanged K5 source remain pinned by references to committed files/gzip source. No duplicate runtime implementation, production lock or generated acceptance was created. The new state imports the accepted first64 module as a retained dependency but proves its own concrete equality by Reflexivity.

`source-checks.json` records structural RED/GREEN and a planted wrong-c0 source-check defect, not native semantic negatives. Direct module imports, local reference rules, canonical semantic JSON, sole negative AST deltas, exact original-tree left/sibling terms and unchanged300s child limit were checked. Official source admission and theorem/resource behavior remain unproved.

Proposal and independent SPEC are copied byte-exact in this fixture. Preserve their instrument history and unavailable historical traces honestly. New compiler admission failures, unintended diagnostics, timeout/OOM/nonzero memory events or failed postguards stop the sequence; no equivalent retry or raised limits.

This is one bounded second-state/continuation step, not a32-certificate expansion. Final digest, other compression/chunk-tree work, validation/finalization/ROOT/encoding, existing vector/control counts, pending Object/model/scenario drafts and production PrismPM/Pages remain open.
