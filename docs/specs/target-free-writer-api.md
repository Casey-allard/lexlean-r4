# Target-free geometric writer refinement

Status: independently reviewed API refinement of the approved full-scope design
and [artifact/serving design](artifact-serving-design.md). Architecture and
mathematical/error-domain reviews passed on 2026-10-21. The target-free source,
31 frozen controls, missing-module RED and seven semantic plants now have a
verified development checkpoint. General identity-projection length and exact
nonempty-list finalizer receipt proofs also passed; see the
[verification record](../PREDICTION.md). Full admitted-domain refinement and
mandatory integration remain required. No serving or model execution is claimed.

## Authority and separation

Authority is UOR-R4 revision `4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-discrete/src/discrete.cu`, SHA-256
`8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`,
especially `writer` at lines 307–345 and `discrete_evaluate`'s global ranking.
The existing modeled implementation is `R4.Admission`; its checked source
domain and ordered errors are documented in [ADMISSION.md](../ADMISSION.md).

The new module is `R4.Prediction`. It accepts an explicit carrier and complete
codebook, selects occupied cells by endpoint distance/cell ID, then ranks every
identity in those cells by full-width distance/token ID. It accepts no target,
objective policy, training proposal, mutable model handle or telemetry switch.
It is a pure projection required by serving; the complete frozen session,
read composition, artifact custody, production lowering and runtime audit are
separate obligations. It must not call the supervised writer with an invented
target or suppress an arbitrary supervised error.

## Public API and admission

`PredictionReceipt` has `prediction : Nat`, `admittedCells : List Nat`, and
`rankedTokens : List Nat`. Lists preserve complete distinct token identities,
including identities with equal vectors, without a fixed member-count cutoff.
Cells are unique and ordered by `(addressDistance, cellID)`; ranked tokens are
ordered by `(fullDistance, tokenID)`. The first ranked token is the prediction.
An error returns no partial receipt.

`PredictionError` is closed: `invalidSpec`, `carrierShape`, `codebookShape`,
`invalidMove`, `unsafeCarrier`, `invalidGeometry(token : Nat, block : Nat)`,
`noPrediction`, and `arithmeticFailure`. It has no invalid-target, loss, rank-
telemetry or training-eligibility error. The arithmetic constructor preserves
failures from checked shared distance helpers; public lawful-input proofs must
establish its unreachability where relied upon.

`PredictionResult` is `value(PredictionReceipt)` or `error(PredictionError)`.
`predict(spec : R4.Types.Spec, query : List R4.Carrier.Block,
book : List (List R4.Carrier.Block), view : R4.Admission.View)` checks, in order:

1. Existing `validSpec` bounds.
2. Query block count equals `spec.blocks`.
3. Complete vocabulary count and every token's block count match the spec.
4. Current/virtual view token, block and move ordinal admission.
5. Existing sufficient `Carrier.sourceArithmeticSafe(query, blocks)` domain.
6. Every original codebook block is an exact canonical root, in token/block
   order, before any virtual transformation.

The current view is used by frozen serving. Retaining the existing virtual
view in this mathematical API allows a complete projection relationship with
the already modeled writer and exercises endpoint/interior transformation
semantics. It does not authorize parameter updates in a serving session.

Reuse `Admission.scoreBook`, `occupied`, `takeCells`, `rankTokens`, and identity
projections where their contracts apply. In particular, scoring must retain
all codebook entries and check interior roots. A target-free API does not
justify checking only the eventual winning cells. New wrapper errors preserve
the existing root location and distance failures instead of inventing success.
No top-eight padding is included in the receipt: rankedTokens is the complete
logical inventory, not a claim that a production implementation allocates it.

The shared `ScoresOutcome` error type also contains supervised-only
`invalidTarget` and `sentinelFailure` constructors, although the score traversal
does not produce them. A total wrapper may classify those constructors as
`arithmeticFailure`, a defensive invariant failure, and must never turn them
into success. The general refinement must establish their unreachability from
the actual traversal. This mapping is not an error-equivalence claim for
arbitrary externally fabricated `ScoresOutcome` values.

## General refinement obligations

The required relationship ranges over every admitted spec, safe carrier,
complete root codebook, valid view, target below vocabulary, both objective
policies, and both telemetry flags. For each such input:

- The supervised writer succeeds; target lookup, geometry, prediction and loss
  failures are unreachable under these preconditions.
- Its prediction, admittedCells and rankedTokens equal the target-free result.
- Changing the target, policy or telemetry does not change those three fields.
- Every admitted cell is occupied, no eligible token identity is omitted, and
  the prediction is the global distance/identity minimum over those cells.

Finite examples alone cannot discharge these universally quantified claims.
The source dependency argument is that target affects only rival exclusion,
target distance/rank, hinges and telemetry. Cell selection uses the complete
codebook and query; best-token updates use distance and identity only. The
modeled implementation must connect that argument to checked general lemmas,
including successful completion of the supervised loss path. A source diagram
or equality of two independently computed example values is insufficient.

For nonadmitted inputs, retain the public wrapper's explicit domain and error
order. Do not claim identical errors when a supervised invalid target occurs
earlier than some target-free geometry error. The refinement's target bound is
an explicit precondition, rather than a hidden repair of the source interface.

## Loss-path safety argument to formalize

Write `Q` for the sum of all query-coordinate magnitudes, `B` for block count,
and `U = 4294967295`. Existing admission gives `2 <= B <= 16`, per-coordinate
magnitude at most `2147483645`, and `Q + 12*B < U`. Each canonical root has L1
norm four and coordinate magnitude at most two; virtual moves preserve roots.

Every full token distance is at most `Q + 4*B`; adding width `8*B` is strictly
below U. Every address distance uses first/last blocks, so it is at most
`Q + 8`; adding margin eight is below U because `B >= 2`. Thus both hinges
are nonnegative and strictly below U, without UInt32 wrap or collision with
the absent-rival sentinel. Empty-rival hinges are zero by definition.

At most 256 cells are occupied; one-based target rank is at most 256, and
rank violation is at most 255 because keptCells is positive. Width is at most
128. Consequently weighted token hinge is below `8*U`, weighted address hinge
below `128*U`, and weighted rank at most `32640`. Both objective sums are
strictly below `136*U`, which is below `2^40` and hence far below UInt64 MAX.
The unrequested telemetry branch's local rank one also obeys these bounds.
Neither checked-scaled-add overflow nor selected-loss sentinel can occur.

These are proposed proof bounds inferred from the source and existing domain,
not an assertion that the CUDA implementation checks them at runtime. General
proofs must also connect score-list presence to exact vocabulary indexing and
positive occupied-cell admission, establishing a nonempty ranked result.

## Independent controls and integration

Before definitions, freeze literal controls for invalid spec/shape/view/safety
priority; invalid first/interior/last root location; all-zero/equal-distance
identity ties; cell ties; a crowded cell retaining all members; differing cell
order versus global token order; kept-cells truncation with no member truncation;
endpoint and interior virtual moves; duplicate-vector distinct IDs; and both
ends of the sufficient carrier domain. Add supervised projection controls
across every valid target of small complete fixtures, both policies and both
telemetry flags, alongside the general proof obligations above.

Required source-valid plants include choosing the first admitted cell's winner
instead of the global winner, reversing identity ties, dropping duplicate-vector
identities, ignoring an interior invalid root, and using target-dependent rival
exclusion in the prediction. Restore exact source bytes before acceptance.

The component must enter the PrismPM model/authority/scenario/ledger and exact
mandatory receipt gate. It does not close complete Engine inference, fixed-memory
addressed access, instruction acceptance, GPU research parity, qualification,
or GitHub Pages deployment. Those full-scope obligations remain required.
