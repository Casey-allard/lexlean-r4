# Read configuration and checked gate inventory

`src/R4/Configuration.lex.tex` models the pinned discrete engine's configuration
metadata. It imports the existing `R4.Types` Spec predicate and `R4.Objective`
structural natural multiplication helper. No carrier, kernel, research fixture,
training, or serving workload is executed by these configuration controls.

## Source and representation

The authority is `crates/uor-r4-discrete/src/lib.rs`, lines 57–188, at UOR-R4
revision `4611538bb4843947363ccb9cce5f388e32c46883`, SHA-256
`ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9`.
It defines Spec, ReadSpec, ReadInitialization and gate_count.

`ReadSpec` is a closed sum: Route; Flock with kept count; or Relation with kept
and sweep counts. `ReadInitialization` is ZeroGatesV1 or ActiveLeafV1. Model
natural values carry an explicit UInt32-domain refinement through `isWord`
and `readWordsAdmitted`; values above 4,294,967,295 receive a separate
`outsideWord` admission result. This is a local representation boundary,
not an invented error from the Rust/CUDA source, whose operands already have
u32 types. No conversion truncates an out-of-domain natural.

## Validation and projection

`validateRead` first runs the existing Spec predicate. Its `invalidSpec` result
precedes all read validation outcomes. After word admission, Route is accepted;
Flock kept must be in 1..8; Relation kept must be in 1..8 before sweeps is tested
against 1..5. Errors distinguish invalid Flock kept, invalid Relation kept and
invalid Relation sweeps. `validateReadBounds` is the source semantic check
after representation admission and exposes that ordering directly.

`words` projects the source's three-word metadata:

| Mode | Words |
| --- | --- |
| Route | `[0,0,0]` |
| Flock | `[1,kept,0]` |
| Relation | `[2,kept,sweeps]` |

As in the source, projection itself does not validate semantic ranges. Its
source-compatible word contract requires `readWordsAdmitted`; arbitrary
naturals outside that domain are not UInt32 metadata. The closed model has no
extra fields or unknown modes. JSON serialization and rejection of extra wire
fields are separate future artifact-codec obligations, not claimed here.

`validateInitialization` is deliberately independent: ZeroGatesV1 always passes;
ActiveLeafV1 rejects Route and passes Flock/Relation, even if their kept/sweep
counts violate read bounds. It does not call `validateRead` or perform word
admission. Its imported source interpretation assumes a represented ReadSpec;
callers must perform representation and configuration checks at their own
boundary. `isZeroGates` preserves the source predicate.

## Arithmetic and errors

`gateCount` admits its natural-backed read fields, block count and stage count
to the UInt32 domain, then checks the same arithmetic as the source:

1. Compute blocks × stages and reject a result above UInt32 maximum.
2. Route and Flock return that base count.
3. Relation additionally checks 5 × stages, then checks base plus that result.

`overflow` is distinct from `outsideWord`. All three arithmetic failures are
the same source gate-inventory overflow class. Kept and sweep range validation
is not part of gate counting. Nor is Spec validation: zero blocks or zero
stages are meaningful inputs to this API, even though zero blocks would fail
Spec. In particular, Relation with zero blocks still includes five gates per
stage; Route with zero blocks returns zero for every admitted stage count.

The natural `product` helper supplies the larger operand as the value and the
smaller as the multiplier to `Objective.scaleNat`. This preserves exact
multiplication and keeps boundary controls with one small operand reducible.
It is not a fixed cutoff or fallback: all natural inputs have the same total
structural arithmetic semantics. Reduction cost grows with the smaller operand
(the imported helper groups eight additions per recursive chunk). This slice
does not establish efficient evaluation of arbitrary large pairs or a native
checked-multiply projection. Production compiler/resource acceptance remains
separate; no compiler helper or generated Lean was modified.

## Independent controls and verification

The 75 theorem controls include Spec-before-read and kept-before-sweeps error
priority; kept 0/1/8/9 and sweeps 0/1/5/6; every mode and initialization pairing;
initialization independence from read bounds; exact three-word projection;
word maximum and first out-of-word values; and the following gate boundaries:

| Read / blocks / stages | Expected result |
| --- | --- |
| Route or Flock / 16 / 6 | 96 |
| Relation / 16 / 6 | 126 |
| Route / 4,294,967,295 / 1 | 4,294,967,295 |
| Relation / 4,294,967,290 / 1 | 4,294,967,295 |
| Relation / 4,294,967,291 / 1 | overflow |
| Relation / 0 / 858,993,459 | 4,294,967,295 |
| Relation / 0 / 858,993,460 | overflow in the relation product |
| Relation / 16 / 204,522,252 | 4,294,967,292 |
| Relation / 16 / 204,522,253 | overflow in the sum |
| Route / 4,294,967,296 / 0 | outsideWord, despite a zero mathematical product |

R4-06 and its scenario preceded implementation. The initial control failed SDK
checking with `LLR3005`: imported `R4.Configuration` did not exist. The first
positive formal verification passed all 75 controls with zero diagnostics,
attestation `e051ab75eb140376f86f03cbecad53f650148bb2dbe91e198cccf06e508b8fb0`.

A planted defect changed Relation's factor from five to four in the actual
source. `cargo test --offline --locked -p repo-conformance --test configuration`
failed with exit 101 and `LLV7002`: the 126-gate, exact-word-maximum, and overflow
theorems were false. The source was restored byte-for-byte before rerunning the
positive integration check. The test freezes all control statements and checks
their audits, fresh manifest/source bindings, and independent Lean replay
through the shared verifier. The restored command passed its single
`typed_configuration_r4_06` test in 5.84 seconds.

| Reviewed input | SHA-256 |
| --- | --- |
| Configuration source | `a3f01b7b49ce1a9d426a2abed5041d7afb5becf4e37a01fee0dcd72634a531d8` |
| Configuration controls | `73c945a6bff74ebbaa3dd5ce54c81b20549e749bede25ac1c509b3713963a5fb` |

These are development SDK formal checks. They do not resolve the production
SDK/template binding or establish read execution, GPU parity, or deployment.
