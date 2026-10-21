# CodebookWords exact integer component

R4-21 is registered for the complete integer component specified by
[codebook-integer-api.md](specs/codebook-integer-api.md). The isolated component has passed all 2,404 formal controls, seven source-valid
mutation rejections and full restored verification. The complete nineteen-component default graph and mandatory Rust receipt test
have now passed, with actual receipt and static-check audits. Prime construction, permutations, software binary64, complete
codebooks and production export remain separate required components.

The parent independently reviewed the integer API before this packet was
created, then independently checked all 2,404 expected results using binary
schoolbook carry/product, bit digit arrays, Horner codecs and canonical signed
domain/order reasoning. The control author separately audited theorem structure
and literal expected values; that author audit does not replace the independent
expectation review.
The source authority and exact checksums are recorded in the API and
[codebook-artifact-refinement.md](specs/codebook-artifact-refinement.md).

## Frozen candidate packet

All 2404 controls have no parameters, an empty axiom allowance and reflexivity
proofs. Every expected result is literal constructor data; no implementation
call appears on the expected side. These are exact mathematical word/scalar
identities, not outputs from a candidate, prime constructor, PRNG or model.
The minimal committed inventory is
[word-controls.json](../tests/fixtures/codebook/word-controls.json); it binds
R4-21, the source module, every fixture path/module/count/hash and the 2,404 total.

| Fixture | Controls | SHA-256 |
| --- | ---: | --- |
| CodebookArithmetic32Controls | 298 | `21123003814318e1ed6dc817b3632d7e68837a689ec6e81bfbb16a660c7760f2` |
| CodebookCodec32Controls | 80 | `34769088179b5b7a9b4d6c015725a2a065f79aec5651687be3789780fe85b30d` |
| CodebookArithmetic64Controls | 490 | `cfcae7c13536244a6dc24668500aff199a8c406e6582aa6e2ddc57913af2a3e8` |
| CodebookCodec64Controls | 148 | `796a93a44271fbf4b29a83cebff38d052895bce3065d31b57f6c213123c781fe` |
| CodebookArithmetic128Controls | 874 | `535f7db97cf7470ee740cc5c9e466009a124c121472a5cc6452c5042bce63c4b` |
| CodebookCodec128Controls | 284 | `e700b5f1c133c7bc46e6f9abbf1101a54137ea9daf67920670bb6d1591b70d2b` |
| CodebookSignedControls | 230 | `837dba65a5fb004e395a9bb1058928be2a8c2d83fddf5db7baf757ef15f5ccfe` |

The preparation inventory is retained at
`/tmp/r4-codebook-words-controls-inventory.json`, SHA-256
`61bbd6102f14357b04f791b694989e7a24d75fbb6b4bdb2c75f3edaabf79dc0d`.
The authoring program is `/tmp/r4-codebook-words-controls-author.mjs`, SHA-256
`fbfa9381783377c7c1bf23214ea569393ac4199edf88d54e9ab53d968cf3f126`.
The checked fixture bytes in tests/lexlean are the proposition authority.

Coverage includes every basis bit, every admitted shift amount, complete
codec positions and distinct positional byte order at widths 32, 64 and 128;
checked versus wrapping overflow; first invalid operand/amount/length/byte;
zero divisors; canonical signed32 endpoints and inverse codecs; and total
signed ordering outside the i32 domain. Exact BigInt arithmetic was used only
to spell finite integer identity literals before any implementation existed.

## Source and verification

Independent packet review and genuine missing-definition RED preceded semantic
implementation. After a missing-lock setup failure and copied-owner permission
failure were corrected in the isolated stage, source check failed with LLR3005:
CodebookArithmetic32Controls imports R4.CodebookWords, which is not a project
module. The setup failures are retained and are not counted as RED. The genuine
result is `/tmp/r4-codebook-words-missing-module-red.json`.

The candidate `/tmp/lexlean-r4-codebook-words-20261021/src/R4/CodebookWords.lex.tex`
has 30 declarations implementing all 17 public functions, SHA-256
`8ac6f9360570026742c2d766f80d39a98bbad7b2a09d733337055aa14ea094fd`.
The official SDK source check passed its 11-module closure with no diagnostics;
`/tmp/r4-codebook-words-sourcecheck.json` records that static result. It is not
a proof receipt or shared-project acceptance. Independent code/security review
approved these exact source bytes after checking all 17 APIs and their raw-helper
domains, error order, full-width arithmetic and signed inverse codecs. The baseline and final restored verification both passed all 2,404 controls
across seven roots. Each complete receipt audit covers 8,270 declaration rows
and 35 independent kernel replays, with empty observed axiom sets and the
unchanged none-axiom policy. All restored attestation identities equal baseline.

The source's raw helpers are internal arithmetic machinery, not checked
application admission roots. `codebookRawXor` and `codebookRawOr` operate for
the supplied bit count; checked APIs supply exactly 32, 64 or 128 and admitted
operands. `codebookRawEncodeLE` extracts its requested count of low bytes;
checked callers first establish that the value fits the requested width.
`codebookRawDecodeLE` is a positional sum; checked callers first establish
exact length and every byte's 0..255 domain. `codebookPowerTwo` has a positive
result, and all divisors used by checked APIs are positive after their guards.
No arbitrary raw helper call is evidence of successful checked admission.

Seven source-valid defects were rejected at their frozen controls: removed
multiply wrap, inclusive checked-add bound, shortened XOR width, inclusive shift
bound, omitted negative-predecessor offset, wrong positional decode weight, and
byte validation before exact length. The final audit rechecked three retained
semantic negatives and executed the remaining four, then reran every control
against the restored source. It does not claim that all seven were rerun in the
last batch.

Two instrument faults remain preserved separately. The original XOR and decode
weight mutation ASTs required canonical JSON key order; that repair changed no
semantic node. A later audit rejected the compiler's terminal maximum-error
summary after 100 distinct mapped reflexivity failures. The reviewed successor
accepts that exact terminal summary only in position 101 after those 100 semantic
failures; setup, timeout, memory and other diagnostics still cannot count as a
negative. Synthetic invalid variants were rejected before the successor ran.
Neither repair changed any production source, expected control, resource limit
or axiom policy.

The final mutation audit SHA-256 is
`24b2a430571374c02e005d808a5439395780d653c61bbf61bd150b922a7c14cd`;
the restored audit SHA-256 is
`681d37ddc2507c46dd402e4f682a23b4f1e667cc1e43086f161d243cbf2aedb4`.
Durable evidence is under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/codebook-words/`.
These formal results use the recorded development compiler and fixed proof
limits; they do not provide production PrismPM/template binding.

## Development integration

The mandatory `exact_codebook_words_r4_21` Rust gate binds the immutable inventory,
all seven fixture modules and every actual transitive dependency receipt. Its
target invocation passed two tests with no failures, ignored or filtered tests.
The seven actual target bundles discharge all 2,404 controls and are individually
re-audited. The complete default graph passed 11,150 theorem controls across
116 modules and 88 roots, with 13,472 exact none-axiom declaration audits
and 116 independent kernel replays.

Model generation/checking, formatting, warnings-denied Clippy and all-feature/
all-target checking passed. The full combined audit checks 118 current local
source/config/lock inputs, every manifest-listed output and the exact selected
olean set. It does not independently derive an SDK output-path inventory or
validate embedded lexicon input rows against local project files.

Integrated target and full-graph verification ran at seven GiB and two CPUs in
the existing eight-GiB VM. Earlier isolated baseline/mutation/restoration proofs
used four GiB. Source, expected controls, compiler semantics, none-axiom policy
and child proof limits were unchanged by this integration. No new whole-workspace
test run is claimed. Actual `just model` and `just vv` still fail
because `prismpm.lock` is absent; production binding and export remain unresolved.

Combined attestation: `0e0d39f5e52029aaaba9ecb694e232eb0dd357b02ae058fe551cd8d530e5a3c9`.
Configuration SHA-256: `572bbd3606d49b9d44788ff483ed9249695428727cbb872b898ed388608e46d4`.
Lock SHA-256: `f6585e5d7d68b8486bb06cdf3b62ae5a5c12210b5d5283174bbf5da55e3deb73`.
Retained evidence: `/Users/casey.allard/lexlean-r4-evidence/2026-10-21/codebook-words/integration-19`.
Retention report SHA-256: `28b5e99e36b531909c315b66614ad99a239fa37cafb8b0807f92388dd1e537ac`.

The original integrated formatting check failed solely on a wrapped Rust test
assignment. The exact whitespace-only repair was independently reviewed; the
mandatory target reran successfully and the complete static-check sequence passed. All 118 formal
inputs are byte-identical to the successful combined receipt, which was retained
without rerunning the combined proof. The original target and failed formatting
evidence are preserved alongside the repaired execution.
Format-repair manifest SHA-256: `4a950fa0b70d7567d8b4449a79da35d888a3f335a5bff3a7a1bb8e6fb9d52127`.

Prime and permutation construction, software binary64, complete codebooks,
model execution, GPU parity and production lowering remain required and are not
established by this integer component.
