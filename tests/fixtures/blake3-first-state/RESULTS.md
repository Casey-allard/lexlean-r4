# Actual first64 BLAKE3 state certificate

The official compiler rejected the intentionally wrong `totalBytes = 65` statement
and verified the exact `totalBytes = 64` statement. Independent reviews accepted
both outcomes and their source, resource, command and artifact custody. This is
one local full-state certificate for the original case19 first packed leaf.
BLAKE3 acceptance remains **18/35 vectors and 255/272 controls**.

The actual theorem is `Blake3Case19FirstState.first64_full_state`: feeding the
original bytes0–63 through `blake3FeedLiteralRaw` from
`blake3Init 18446744073709551615` yields all seven expected state fields:
original IV, chunk counter0, compressed blocks0, buffer0–63, total bytes64,
original maximum budget, and empty CV stack. The positive proof uses reflexivity
with no axioms. Runtime definitions and the original final digest are unchanged.

| Actual phase | Official outcome | Independent review |
| --- | --- | --- |
| Wrong65 negative | verify1, intended `LLV7002` rfl mismatch,12281ms; no artifact | `d4440c217bf753464a82c4c1873e6a82af74f86921fd687db4e63fc66ddad734` |
| Exact64 positive | verify0,30954ms; artifact retained | `a8e1cf019cfaaedd888ab68a748717de8b4e27a8725536427a84954e77405d81` |

Positive attestation:
`3ef21cc30dd1fe63d2cd7610e08a07c4f78dbb4557b9225daf056e99ba32ac00`.
The independent audit accounted for55 actual artifact files,5 compiled modules,
5 kernel replays,92 unique declarations with empty observed axioms, and16 successful
process records. Both phases passed all nine independent postguards. Source,
genuine generated development locks, tools and the separately reviewed
`2026-10-26T20:22:36.749456407Z` epoch stayed unchanged; sampled memory counters
were all zero. Storage samples passed the resource floors; the10GiB peak
allowance remains an estimate, not a measured peak or continuous reserve proof.

[results.json](results.json) records exact identities, scope and raw review aliases.
[artifact-inventory.json](artifact-inventory.json) inventories every actual artifact
file. [evidence.jsonl.gz](evidence.jsonl.gz) retains content-deduplicated exact text
with relative aliases: source/invocation/resource reviews, all command receipts,
stdout/stderr, generated locks, recovered input bytes, artifact text, and the
pending model reconciliation. Its first JSONL row is metadata; following rows
contain SHA256, byte length, aliases and base64 bytes. Five `.olean` binaries stay
in local evidence and are explicitly excluded; this is not a new replay claim.
The rejected v1 invocation's eleven decoded members are preserved, while its
original compressed-container hash is provenance rather than copied gzip bytes.

The frozen README, candidate and source manifest retain their historical
source-only `NOT_RUN` status. These separate results supersede that status.
V1's success-shaped failure-envelope assumption was rejected before native work;
its source, reviews and tests are preserved alongside the narrow v2 repair and
actual negative. Host tests are not substituted for either compiler outcome.

No compression occurs at this first cut. This result establishes neither an OOM
remedy nor an efficiency improvement, whole2048 digest, Object completion,
production PrismPM binding, `just vv`, or Pages acceptance. The prior whole-case19
OOM failure remains preserved and unaccepted.

Next prove the five fixed error-preserving feed continuations with symbolic
right subtrees of sizes64,128,256,512,1024. Use the supported Nat-family transport
and unrelated `0 = 0` base-Apply negative, then require both the actual generic
production feed consumer and the exact original balanced-tree consumer. Helper
success alone is insufficient. Outer validation, length, finalization, ROOT and
digest encoding remain obligations; no trusted intermediate CV or limit retry.

The archived `pending-model-reconciliation.json` identifies concrete work for all
seven remaining drafts: component IDs/scenarios and real acceptance tests may be
authored before implementation, but rendering conformance cannot promote any full
claim. Prediction work is independent of Read33; successful receipt/loss-bound
and scoreBook completeness proofs remain required. Object Read33/Construction39,
whole257, Hash mutations, serving, durable memory, learning, production binding
and generated Pages remain in scope.
