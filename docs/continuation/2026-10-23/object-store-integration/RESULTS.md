# Object Store integration — actual results

The canonical five Object runtime modules and the migrated Store20 control module passed the **existing 20 Object Store controls** under the pinned finite development dependency closure. The independent positive review accepted 823 declarations with empty observed axioms, 34 successful process records, 11 kernel replays and an exact 115-file artifact census. All 20 original control declaration ASTs are unchanged; their emitted proofs are `rfl`.

| Actual phase | Result | Scope |
|---|---|---|
| Original legacy Store20 source | Lock 0; check 1, LLR3001; verify not run | Intended source-admission negative: bool glossary1.1.0 does not match locked1.2.0. This is not a semantic planted-defect test. |
| Header-migrated Store20 source | Check 0 across 11 modules; verify 0; independently reviewed | Local 20-control proof under exact finite development dependencies. |

The positive attestation is `7ff21e35e542b6817ce0d67ba61d3e6739b1b2d896977a3a8adce92c7d3cc75e`. Its build is `298243fc326bfbbeb0c44ed1121b8b639bec0d96bfe4fc051f984ed17d02d49a`, source identity `120791b7dd6e31e7f2fb876cdb24d0c69e234ac96face7a28083a9a9ecb0a012`, and semantic identity `432ad85e96b7c68797619a2d9ba87a6db8abe6827a5c411598f73f67c60cfb78`. Both genuine phase locks hash to `f8fceeb925ba5a23cd02f192eca29ea32f5105f5679e2e2bd2eb490135b336a2`; source identities are recorded separately. Positive whole-command wall time was 373603 ms, across multiple child processes with the unchanged 300000 ms per-child limit. This is neither a timeout violation nor a speedup claim.

The negative review is [negative-native-review.json](negative-native-review.json) (SHA256 `554ba20c9fff829f33c3241eda981d7c7f58551506c77808509a42fb640ce014`); the positive review is [positive-native-review.json](positive-native-review.json) (SHA256 `0fd9bc23e46e264ca9d6ea4be7940247d8dfe747216b26a516c6d9acb13bc1bd`). Both retain actual source, resource, lock, epoch and postcapture evidence; both report no postcapture errors. Full census identities are in the positive review. Selected raw receipts, audit text, emitted Store20 Lean and the genuine lock are preserved here; compiled binaries and the full native artifact remain excluded and pinned.

The six integrated canonical paths are:

- `src/R4/ObjectTypes.lex.tex` — SHA256 `02ab380fb1bcb2cb6f0e267843028a88038f17d374b1d65b7b09f0dc1505b740`
- `src/R4/ObjectCodec.lex.tex` — SHA256 `66ffca5523f2ab6f9382c61fa4eb0c63d3b45e232019fc0146240a336ca8cdbd`
- `src/R4/ObjectStore.lex.tex` — SHA256 `37063c3ec6b6b709b042539e9f3d34aef683e52f084888b756c22ad48a6b9a61`
- `src/R4/ObjectRead.lex.tex` — SHA256 `30610bf511fa7354c76b5fe2655d7356235508a10127609b40bf1999f96257a7`
- `src/R4/Object.lex.tex` — SHA256 `f2ece9599ae63c301d124ffb2f51b47c59538483efaa6dd36f16b00c399e2a76`
- `tests/lexlean/ObjectStoreControls.lex.tex` — SHA256 `28324d3259fe927296707a99e4bcc1a64305703c7f160d289c0d214a26623687`

Runtime reconciliation changes two Codec hash dispatches to the finite-verified HashWire Nat-byte functions and eleven bound-input names (Types 1 / Store 1 / Read 9); the other 260 of 273 declarations are identical. All five runtime wrappers migrate bool/nat glossary versions to 1.2.0 and semantic-module/2. Store20 changes only those three metadata literals, preserving all 20 whole declaration ASTs, proof terms and empty-axiom policies. [source-reconciliation-prewrite.json](source-reconciliation-prewrite.json), [source-reconciliation.json](source-reconciliation.json), and [control-migration.json](control-migration.json) retain exact differences and source reviews.

The immutable originals remain in the already versioned `docs/continuation/2026-10-23/pending/` snapshot. [results.json](results.json) includes every current canonical path, its original snapshot path/hash, all eleven unchanged remaining drafts, and portable 15-input stage mappings in [current-portable-bindings.json](current-portable-bindings.json). The five Hash/runtime dependency sources are repository-pinned finite proof context; this does not establish canonical Hash/HashWire equivalence or production dependency acceptance. No original snapshot was overwritten.

**Remaining work:** 383 Object controls (311 Codec, 33 Read, 39 construction) and 20 separate Blake3 primitive controls; the six semantic planted-defect controls and restoration; the complete original 257 digest/root/distinct proof; full Hash qualification and production binding; model/feature/config/receipt conformance integration, full `just vv`, PrismPM integration and generated Pages acceptance. The admission negative does not discharge any semantic-defect requirement. This checkpoint does not establish full Object acceptance.

The initial [input-manifest.json](input-manifest.json) and [source-bindings.json](source-bindings.json) preserve preparation-time blocked/pending statements. **The current actual status is this RESULTS.md and results.json.** Their original statuses were not retroactively upgraded. [store20-header-migration-proposal.json](store20-header-migration-proposal.json) and the migration record bind the actual reviewed successor. A future compiler run still requires fresh source/tool/resource admission; these development locks are not production PrismPM/template locks.
