# Third BLAKE3 round: source preparation only

All three official compiler phases are **NOT_RUN**. This packet proposes one fixed full sixteen-word equality for production `b3CoreRoundRaw(R2,M2)` and a mandatory consumer of the exact original `b3CoreCompressRaw(IV,BlockWords([0..63]),0,64,1)` expression. It does not establish a proof or alter production runtime definitions, digest, empty-axiom policy, or the 300000 ms limit.

`proposal.json` and `spec-review.json` are exact copies of the approved source plan. Accepted R2 and M2 are copied as ASTs from the committed second-round certificate and consumer. `derive-expected.py` evaluates the pinned production AST once from literal R2; a separate fixed integer G schedule checks the same sixteen values. Both computations provide untrusted authoring data, not trusted compression or kernel evidence. `derivation.json` retains the actual source pins, inputs, output and cross-check. Repeating the exact recipe produced byte-identical output.

The negative differs only in expected s0: 3971108617 becomes 3971108618. Its proof remains reflexivity. The required production consumer preserves the original left-hand side and has `FeedForward(Rounds(4,R3,Permute(M2)),IV)` on the right. Its only proof is `Apply Blake3SecondRoundConsumer.second_round_compress_consumer` with empty arguments. The new certificate is imported; no rewrite or transitivity constructor is claimed. Actual definitional conversion and resource cost remain unproved until the mandatory consumer passes.

The source preparation has four intended predecessor structural failures, four current structural passes, 95 full source assertions, one intended in-memory missing-third-permutation rejection, then 95 restored assertions. These are source checks, not semantic compiler negatives or execution coverage. Every wrapper uses the previously admitted Boolean title and explicit Boolean/Nat glossaries, canonical JSON and direct imports. The twelve shared inputs are pinned references, never substituted locks or copied runtime variants.

| Future phase | Modules | Staged inputs | Decoded bytes |
|---|---:|---:|---:|
| round3-negative | 4 | 8 | 140250 |
| round3-positive | 4 | 8 | 140250 |
| original-consumer | 11 | 15 | 199601 |

Independent source, invocation and fresh resource reviews precede any parent-selected compiler phase. Negative, positive and consumer must each stop for actual independent outcome review; there is no automatic advancement. Unexpected admission failures, timeout, OOM or generic diagnostics are not the intended wrong-word negative.

Four remaining rounds, full compression, literal S128, the original balanced2048 digest, BLAKE3 acceptance, Object integration, seven pending drafts, genuine PrismPM binding and Pages remain open. Counts stay at 18/35 vectors and 255/272 controls; Object331/403. Hash restoration is complete at78/78roots636controls; mutation qualification remains2/7 and fullHash acceptance open. No efficiency or OOM remedy is claimed.
