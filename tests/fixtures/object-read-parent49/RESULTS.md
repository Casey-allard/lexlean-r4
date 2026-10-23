# Original parent49 decode: verified local equation

The actual LexLean theorem `LexLeanR4.ObjectReadParent49.objParent49Decode` passed independent native review. It proves the full equation `ObjDecodeRecord(originalParentWire49) = Result.ok(expectedRecord)`, retaining the original 12-byte identity beginning 128, scale 1 and 36-byte payload. The original input `ObjAppendBytes` expression and production decoder are unchanged; no hash was substituted or assumed.

| Actual phase | Observed result | Whole verify time |
|---|---|---|
| False expected ID | Source check 0; verify 1, LLV7002 intended `rfl` mismatch after changing only expected ID byte 0 from 128 to 129 | 35,529 ms |
| Original expected ID | Source check 0; verify 0, actual artifact independently accepted | 268,903 ms |

Both phases used 15 pinned inputs and genuine development locks. Both locks hash to `84a382eb352d60a3daed5f107f79f3cb73f1961fb2230124774375cd93b767db`; source/semantic identities differ and are recorded separately. Both postcapture error lists were empty. These whole-command durations include compilation/audit/replay work and imply no performance improvement. Limits remained 300,000 ms per child and 4 GiB / 2 CPU.

Positive attestation: `43bcff1d853a153c4d3e10c2e503fb794e5033995d47e205454862508d114033`. Build: `dc06dfefa33ee9d2d5dece4199037bff325f767ca5301696fca2437df96b8d56`. Source: `ef10bab5a1527e869cc4359d2802666d38f903a482777b6ca622ccbf7a8bfe24`. Semantic: `99f8245de1a7c555a8ed174b19631827fcdaaa4ebf9fe8191b3d284f7d21c1cf`.

The positive audit checked **804 declarations with none policy and empty observed axioms, 34 successful process records, 11 kernel replays and 115 artifact files**. The emitted theorem uses `rfl`. [positive-native-review.json](evidence/positive-native-review.json) has SHA256 `7a10d49b23f5cff53e84662be64c5090996e77cdf817530af5789b21b7face28`; [negative-native-review.json](evidence/negative-native-review.json) has SHA256 `168520b90a33e8d0d5b098e7fc26bb263e7b8744d8e70970f6c5c03a322e9aa8`. Selected raw receipts and artifact texts are retained; the complete file census pins omitted binaries/build artifacts without adding them to Git.

**This accepts one intermediate decode equation and zero original Read33 controls.** Mandatory next is the actual authenticated fetch over all four original backend entries, with lookup, missing/error propagation, scale and ID checks intact. Its proposal and SPEC describe future work; no fetch source or result is established here. Child authentication, traversal, range finishing, manifest-version hashing, whole 96-byte digest checking and the original whole-read consumer remain open.

The original 403 Object obligations still comprise separately accepted Store20, all 33 Read controls and 350 other controls requiring completion, alongside 20 separate Blake3 primitives. Required production semantic mutants/restoration, full 257 proof, Hash qualification/binding, model/conformance/full `just vv`, PrismPM and Pages remain open. The local false-ID equation is not a substitute for a production mutant. Earlier aggregate and singleton timeouts remain preserved.

[results.json](results.json) is the current machine-readable result; [source-bindings.json](source-bindings.json) provides repository-resolvable 15-input mappings for both phases. Frozen preparation records retain historical NOT_RUN/pending language. [positive-review-custody.json](evidence/positive-review-custody.json) binds the later actual review without changing those records. No production PrismPM/template lock, runtime duplication or new repository runner is included.
