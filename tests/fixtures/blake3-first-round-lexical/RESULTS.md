# Actual first-round certificate and compression decomposition

The official compiler accepted the full sixteen-word first-round certificate and the mandatory production compression consumer. The wrong-s0 companion was rejected with the intended `LLV7002` reflexivity diagnostic. Independent artifact reviews accept this bounded decomposition only; the six remaining rounds still appear as an unevaluated expression.

| Phase | Source modules | Verify exit | Verify ms | Independent outcome |
| --- | --- | --- | --- | --- |
| round-negative | 4 PASS | 1 | 10995 | PASS_INTENDED_WRONG_S0_NEGATIVE_LOCAL_ONLY |
| round-positive | 4 PASS | 0 | 21434 | PASS_ACTUAL_FIRST_ROUND_FULL16WORD_CERTIFICATE_ONLY |
| lift-positive | 6 PASS | 0 | 10450 | PASS_ACTUAL_NAT_LIFT_HELPER_ONLY |
| consumer | 7 PASS | 0 | 12523 | PASS_ACTUAL_FIRST_ROUND_COMPRESSION_DECOMPOSITION_ONLY |

The actual consumer proves:

```text
CompressRaw(IV, BlockWords(original bytes 0..63), 0, 64, 1)
  = FeedForward(Rounds(6, literalR1, Permute(BlockWords(original bytes 0..63))), IV)
```

The positive certificate checks all sixteen literal words against actual production `RoundRaw`; the consumer applies the supported Nat lift at one and retains exactly one message permutation before the six remaining rounds. No trusted intermediate hash, changed runtime definition, extra axiom or raised timeout was introduced. The wrong-word companion changes only expected s0 from 2216531236 to 2216531237.

The certificate, lift and consumer bundles contain 45/65/75 files, 82/85/86 empty-axiom declarations, 4/6/7 kernel replays and 13/19/22 successful process receipts respectively. Actual identities and review pins are in [results.json](results.json). All nine postguards and all six zero-memory counters passed for each phase. These are retained official results and independent audits; packaging did not rerun the compiler.

The original attempt failed source admission with `LLL1004`: `First` was not a phrase concept in the title `First round`; verification never ran. Prior source review missed that lexical constraint. The separately frozen lexical successor changes exactly two wrapper lines in each of five files: adds the explicit Boolean glossary before Nat and replaces that title with `Boolean`. Semantic JSON bytes, ASTs, imports, proof terms, runtime, configurations and the negative word remain identical. Original source, source-review miss, actual failure, independent diagnosis and later successful admission are preserved. Instrument failures and their independent diagnoses remain separately classified in the archive; none is counted as a proof negative.

[evidence.jsonl.gz](evidence.jsonl.gz) uses the existing exact-evidence JSONL format: first header, then SHA-256/byte-length/base64 members with original absolute and portable aliases. Every member and original alias was byte-checked before writing. It includes both source versions, source/invocation reviews, all four actual phase receipts, genuine development locks and artifact proof/axiom/kernel text. [artifact-inventory.json](artifact-inventory.json) inventories all 185 actual artifact files. The 17 compiled `.olean` files (5,880,544 bytes) remain local and are explicitly excluded from the archive; this checkpoint does not claim complete binary reconstruction. Frozen source preparation `NOT_RUN` labels are historical; this supplemental outcome record records their later execution without rewriting frozen evidence.

This does not prove the remaining six rounds as a literal compression result, S128, the original 2048-byte digest, whole Object257, efficiency or an OOM remedy. Counts remain 18/35 published vectors, 255/272 controls and Object331/403. Hash restoration is complete: 78/78 roots and 636 controls. Mutation qualification remains 2/7 and full Hash acceptance remains open. The seven pending drafts, Read33/Construction39, serving, durable memory, learning, genuine production PrismPM binding and Pages remain open. Any next round/compression proof requires a source-backed independent specification and separately selected invocation; this checkpoint grants no further native authority or production-gate waiver.
