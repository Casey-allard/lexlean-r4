# Fourth BLAKE3 round: source preparation only

All three official compiler phases are **NOT_RUN**. This packet proposes one full sixteen-word equality for production `b3CoreRoundRaw(R3,M3)` and a mandatory consumer of the exact original `b3CoreCompressRaw(IV,BlockWords([0..63]),0,64,1)`. Runtime definitions, original digest, empty-axiom policy and300000ms limits remain unchanged.

Exact approved proposal and independent SPEC are retained. AcceptedR3 andM3 are copied from the accepted third-round certificate and consumer ASTs. `derive-expected.py` evaluates the pinned productionAST for exactly one round, then a separate fixed integer G schedule checks every derived word. Both calculations remain untrusted authoring data until actual official kernel verification. The retained recipe/output repeats byte-identically.

The sole negative changes expecteds0 from2484768672 to2484768673. All other fields/LHS/proof/config remain identical. The original consumer has `FeedForward(Rounds(3,R4,Permute(M3)),IV)` on the right, with exactly four permutations of originalM0. It uses only `Apply Blake3ThirdRoundConsumer.third_round_compress_consumer` with empty arguments. Imported fourth-round equality is not syntactic rewriting or transitivity: actual contextual kernel conversion and cost remain unproved.

Source preparation: four intended predecessor structural failures, four successor passes, full source assertions, one in-memory missing-fourth-permutation mutant rejected, restored assertions and byte-identical derivation. These are source checks, not compiler semantic negatives or runtime coverage. Admitted Boolean title/explicit bool+nat glossaries, canonicalJSON and directimports are preserved. Shared14 inputs are committed pinned references; no fabricated lock.

| Future phase | Modules | Inputs | Decoded bytes |
|---|---:|---:|---:|
| round4-negative | 4 | 8 | 140346 |
| round4-positive | 4 | 8 | 140346 |
| original-consumer | 13 | 17 | 216605 |

Independent source CODE/security-Python and exact invocation/resource review precede parent selection. Actual negative, positive and mandatory consumer require separate independent outcome audits; no automatic advancement. Admission error, timeout, OOM or blank diagnostic is not the intended wrong-word rejection.

Three remaining literal rounds, fullcompression/FirstEight/S128, finalization/ROOT/original2048digest, Read33/Construction39/wholeObject257, seven drafts, serving/memory/learning, genuinePrismPMbinding and generatedPages remain open. Counts stay18/35vectors255/272controls,Object331/403. Hashrestoration78/78roots636controls is COMPLETE; mutations2/7/fullHashacceptance open. No efficiency/OOMremedy or production acceptance.
