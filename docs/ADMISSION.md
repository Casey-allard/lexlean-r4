# Exact admitted-identity writer

`src/R4/Carrier.lex.tex` and `src/R4/Admission.lex.tex` own R4-05. The component
accepts a specification, an explicit integer carrier, a complete codebook,
target identity, current or virtual codebook view, objective policy, and
telemetry request. It returns a complete typed receipt or a typed error.
`tests/lexlean/AdmissionControls.lex.tex` independently specifies its finite
formal controls. No handwritten host writer supplies expected results.

This is the complete writer component over explicit inputs. It does not create
the carrier through Route, Flock or Relation, run counterfactual reads, select
or apply proposals, aggregate training batches, or establish execution parity.
Those integrations remain distinct model obligations. CPU activity here is
source processing and formal checking; model execution/parity remains a future
GPU obligation.

## Integer representation and source domain

`Carrier.Scalar.nonnegative n` represents `n` and `Scalar.negative n` represents
`-(n+1)`. Both fields are mathematical natural numbers. Zero has exactly one
representation; `negative 0` means minus one, never negative zero. A `Block`
contains exactly eight scalars. A carrier is an ordered block list with its
length checked against the specification. This representation admits arbitrary
mathematical integers without restricting the query to the root alphabet.

Magnitude, negation, equality, distance and signed-axis gather are defined in
LexLean. Natural subtraction and scaling reuse `R4.Objective`'s transparent
definitions. Integer blocks are separate from the finite `Geometry.Root` type.
Root decoding admits exactly the sixteen canonical roots. Codebook admission
checks every block, including interior blocks, before using its virtual value.
Different token identities may have identical vectors and cells.

The CUDA source uses unchecked signed subtraction/absolute value and UInt32
distance/margin sums. The checked writer therefore requires the sufficient
`sourceArithmeticSafe` refinement:

- Every query coordinate has magnitude at most 2,147,483,645.
- The sum of query-coordinate magnitudes plus `12 * blocks` is strictly less
  than 4,294,967,295.

For any admitted root block, coordinate differences then have magnitude at
most `INT32_MAX`. A root has L1 norm four, so complete token distance is at
most query norm plus `4 * blocks`; adding the token margin `8 * blocks` stays
below the UInt32 sentinel. Endpoint address distance plus its margin eight is
at most query norm plus sixteen, also below that bound for at least two blocks.
This prevents arithmetic wrap and absent-rival sentinel collisions.

The predicate is a deliberately sufficient interface domain, not a claim that
CUDA rejects other queries, nor a characterization of every individually safe
CUDA input. The low-level mathematical distance/gather definitions remain
total outside it. Every root-valued carrier under the admitted specification
lies within it. A query outside the refinement returns `unsafeCarrier`.

`checkedTokenDistance` rejects unequal block-list lengths with `none`; it never
silently truncates either vector. Equal empty lists have mathematical distance
zero, while the public writer rejects an empty carrier through specification
and shape admission.

## Current and virtual views

`View.current` uses the codebook unchanged. `View.vectorMove token block ordinal`
checks token/block bounds and ordinal `0..15`. Ordinal zero retains the current
value; ordinals one through fifteen apply the corresponding ordered bivector.
The raw block must be a valid root before any virtual transformation.

The view affects full token distance and exact endpoint root lookup together.
The first and last viewed root indices `a,b` produce cell `16*a+b`; membership
is exact, with no nearest-root snapping. An interior move can change token
distance while retaining its cell. An endpoint move can change occupancy,
admission, target rank and the available rivals. Route and gate changes affect
the externally supplied query and use the current codebook view.

## Admission, ranking and receipts

The writer constructs occupied cells from all vocabulary identities, orders
them by `(endpoint L1 distance, cell ID)`, and admits up to `keptCells` occupied
cells. Hamming distance is available in `Carrier` but is not an admission key.
Every token in those cells participates. The global token order is
`(full-width L1 distance, token ID)`, independently of cell-admission order.
The first token is the prediction. `topEight` projects this global identity
order and pads missing entries with `UINT32_MAX`.

A receipt includes admitted cell IDs, the global ranked token IDs, prediction,
target-admission flag, both rival counts, both hinges, both objective losses,
selected loss, and optional telemetry. These lists are logical inventories,
not claims about production allocation, sparse lookup, or running time.

Token rivals exclude the exact target identity. Address rivals exclude the
target cell. The nearest rival is found in the corresponding sorted list and
is explicitly optional. The target's distance is computed even when the target
cell was not admitted. A target outside admission may still have supervised
rivals. A crowded cell can have many token rivals and zero address rivals.

The existing `Objective` supplies the width margin, margin-eight address hinge,
rank violation, and both checked weighted objectives. Both branches are checked
before policy selection. A selected loss exactly equal to `UINT64_MAX` becomes
`sentinelFailure`, matching the caller's status-8 failure interpretation. It is
distinct from arithmetic failure: the source permits an exact-maximum sum
without setting arithmetic status 12, whereas overflowing sums or a scaled
product colliding with the maximum are arithmetic failures. The generic receipt
helper also rejects a selected value greater than the maximum as arithmetic
failure. The exact-maximum case is unreachable under the checked writer's
bounded geometric arithmetic, but its helper and eligibility contracts retain
the source distinction.

Actual target-cell rank is one-based among all occupied cells, under the same
distance/ID ordering. Rank is computed for rank policy or requested telemetry.
Nearest-address policy without telemetry keeps the source's local rank one,
rank violation zero, and occupancy zero. The receipt does not invent an actual
rank field for that case. `lossTR` in this case reflects the uncomputed rank
term, even though `lossTA` remains the selected objective.

Present telemetry contains, in source order: rank violation, target-cell rank,
occupied-cell count, admitted-token count, rank-objective loss, selected loss.
`lossTA` corresponds to the evaluator's traditional total loss, whereas `loss`
is policy-selected. The two fields are not interchangeable under rank policy.

## Supervised eligibility bridge

`eligibility baseline candidate` models the writer-outcome decision inside the
counterfactual row loop. A baseline error propagates. A baseline with zero token
rivals skips the row, including when the candidate would fail. Otherwise a
candidate arithmetic error propagates, another candidate failure (including an
exact-maximum sentinel failure) is unavailable,
and a candidate with zero token rivals is rejected as `lostRival`. A remaining
candidate contributes its selected loss. The bridge never treats absent-rival
zero hinge as a supervised success and never lets a virtual move improve by
removing its competitor.

The bridge consumes explicit writer outcomes; it does not claim to bind a
model identity/epoch, recompute reads, or aggregate a batch. Proposal and
learning integration must provide those contracts.

## Typed failure mapping

| Writer error | Meaning and source relationship |
| --- | --- |
| `invalidSpec` | Existing R4-01 bounds failed |
| `carrierShape` | Carrier block count differs from the specification |
| `codebookShape` | Vocabulary or per-token block inventory differs |
| `invalidTarget` | Target index is outside the vocabulary |
| `invalidMove` | Virtual token, block or candidate ordinal is invalid |
| `invalidGeometry token block` | An exact raw codebook root is invalid; source snapshot validation uses status 6 |
| `unsafeCarrier` | Explicit sufficient arithmetic refinement failed; this is a port-interface refusal |
| `noPrediction` | No valid ranked prediction exists; source writer failure maps to status 8 |
| `sentinelFailure` | Selected loss is exactly `UINT64_MAX`; source caller status is 8, and an eligible candidate is unavailable rather than an arithmetic error |
| `arithmeticFailure` | Weighted arithmetic, operand, or above-maximum value failure; source arithmetic status is 12 |

The public writer checks specification, carrier shape, codebook shape, target,
view and safe arithmetic domain in that order, then checks raw geometry in
token/block order. It returns a whole result rather than partially initialized
CUDA output buffers. Host/interface preconditions are not assigned invented
CUDA status codes. Internal checked helpers use `Option` for absent entries.

## Provenance and formal evidence

Authority: UOR-R4 commit `4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-discrete/src/discrete.cu`, SHA-256
`8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.
The relevant definitions are `table_block`, `root_cost`, `token_cell`,
`address_distance`, `token_distance`, `writer`, the counterfactual eligibility
branch, and the global top-eight projection in `discrete_evaluate`.

TDD began with an independent cell tie-order theorem. The source check rejected
the absent `R4.Admission` module (`LLR3005`), then the single implemented ordering
function verified. Full writer fixtures next rejected absent `R4.Carrier`
(`LLR3005`) before implementation. The checked distance-shape control rejected
the absent `checkedTokenDistance` function (`LLT4001`) before that boundary was
implemented. An initial malformed import-header ordering was corrected before
the missing-module failure was counted as RED.

Custom result types use reflexivity controls, since the development SDK does
not supply their `DecidableEq` instances. The independent expected values were
not changed to resolve this proof-form issue. Generated Lean/Rust, local authoring
scripts, and verifier outputs are not semantic authority. Verification still
uses the digest-pinned development SDK; full production acceptance is blocked
by the independently recorded SDK and execution-surface dependencies.

The completed graph has 22 carrier declarations, 55 admission declarations and
72 independent theorem controls. All controls verified with empty observed
axiom sets and independent kernel replay of the six selected modules. The
restored verification attestation is
`01b8242179e01f155cdd2550773fc6d2fb85d7ff77aaad495265ca235e1eba63`.
The controls are bound by SHA-256
`cd5cfdf48ce3f31f26348f3de57ed3afd379fc8e832571463b1426d5a791dc16`.

| Planted defect | Gate and observed rejection |
| --- | --- |
| Reverse the cell-ID comparison when distances tie | SDK `lexlean --diagnostic-format json verify tests/lexlean/AdmissionControls.lex.tex` exited 1 with `LLV7002`; `cellTieUsesIdentity` and multiple full writer receipts failed reflexivity |
| Return the unchanged root in the selected virtual-move branch | The same verifier exited 1 with `LLV7002`; the fixed candidate controls and virtual writer receipts failed reflexivity |
| Restore the old classification of an exact-maximum selected sum as arithmetic failure | The same verifier exited 1 with `LLV7002`; `selectedSentinelTyped` and the composed `candidateSentinelUnavailable` control failed reflexivity |

After each defect the source was restored byte-for-byte. The final command
`cargo test --offline --locked -p repo-conformance --test admission` passed
inside the network-disconnected development SDK container: one test passed,
zero failed, zero ignored and zero filtered out. That test binds the exact
control digest, invokes the full selected-graph verifier, checks the receipt's
source/build identities and manifest hashes, audits all theorem axioms, and
requires successful independent Lean kernel replay for every selected module.
This is development formal-conformance evidence, not complete `just vv` or
production acceptance.

The independent specification review found that the original receipt helper
classified a selected exact-maximum sum as arithmetic failure. The new composed
eligibility control first failed against that actual old implementation with
`LLV7002`. The repair introduced `sentinelFailure`, retained propagation for a
baseline failure, and made the candidate unavailable without arithmetic status
12. Additional controls distinguish a selected value greater than the maximum.
The old classification was then deliberately replanted and rejected by the
unchanged repaired controls before the final source restoration and test.
