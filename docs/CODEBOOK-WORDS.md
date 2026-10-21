# CodebookWords integer precursor controls

R4-21 is registered for the complete integer component specified by
[codebook-integer-api.md](specs/codebook-integer-api.md). This is a control
preparation record: source definitions exist only in an isolated candidate and
no formal acceptance is claimed. Prime construction, permutations, software binary64, complete
codebooks and production export remain separate required components.

The parent independently reviewed the integer API before this packet was
created, then independently checked all2404 expected results using binary
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
R4-21, the source module, every fixture path/module/count/hash and the2404 total.

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
codec positions and distinct positional byte order at widths32/64/128;
checked versus wrapping overflow; first invalid operand/amount/length/byte;
zero divisors; canonical signed32 endpoints and inverse codecs; and total
signed ordering outside the i32 domain. Exact BigInt arithmetic was used only
to spell finite integer identity literals before any implementation existed.

## Required next gates

Independent packet review and genuine missing-definition RED preceded semantic
implementation. After a missing-lock setup failure and copied-owner permission
failure were corrected in the isolated stage, source check failed with LLR3005:
CodebookArithmetic32Controls imports R4.CodebookWords, which is not a project
module. The setup failures are retained and are not counted as RED. The genuine
result is `/tmp/r4-codebook-words-missing-module-red.json`.

The candidate `/tmp/lexlean-r4-codebook-words-20261021/src/R4/CodebookWords.lex.tex`
has30 declarations implementing all17 public functions, SHA-256
`8ac6f9360570026742c2d766f80d39a98bbad7b2a09d733337055aa14ea094fd`.
The official SDK source check passed its11-module closure with no diagnostics;
`/tmp/r4-codebook-words-sourcecheck.json` records that static result. It is not
a proof receipt or shared-project acceptance. Independent code/security review
approved these exact source bytes after checking all17 APIs and their raw-helper
domains, error order, full-width arithmetic and signed inverse codecs. The
coordinated formal-verification window remains pending.

The source's raw helpers are internal arithmetic machinery, not checked
application admission roots. `codebookRawXor` and `codebookRawOr` operate for
the supplied bit count; checked APIs supply exactly32/64/128 and admitted
operands. `codebookRawEncodeLE` extracts its requested count of low bytes;
checked callers first establish that the value fits the requested width.
`codebookRawDecodeLE` is a positional sum; checked callers first establish
exact length and every byte's0..255 domain. `codebookPowerTwo` has a positive
result, and all divisors used by checked APIs are positive after their guards.
No arbitrary raw helper call is evidence of successful checked admission.

Seven defect candidates are staged only in isolated scratch directories:
removed multiply wrap, inclusive checked-add bound, shortened XOR width,
inclusive shift bound, omitted negative-predecessor offset, wrong positional
decode weight, and byte validation before exact length. None has yet been
source-checked or formally rejected. `/tmp/r4-codebook-words-mutation-plan.json`
binds their candidate bytes; only a source-valid semantic rejection can count.

Complete none-axiom proof/replay,
source-valid planted defects and restored positives must follow implementation.
The assigned Rust receipt test is `exact_codebook_words_r4_21` in
`crates/conformance/tests/codebook_words.rs`; its actual dependency closure
must be bound after verification. No placeholder test or implementation is
introduced by this packet. Parent owns final default-root/build-gate integration.

The accepted SDK proof limits remain fixed. No formal verification, model execution,
GPU result, cross-platform parity or production-lowering success is claimed here.
