# Discrete proposal and transition contract

Status: source-contract notes for the approved full-scope port. No learning
implementation or execution is claimed by this document. The authority is
UOR-R4 revision `4611538bb4843947363ccb9cce5f388e32c46883`, especially
`crates/uor-r4-discrete/src/discrete.cu` lines 388–454 and
`crates/uor-r4-discrete/src/device.rs` (`Binding`, `parameter`, `admit_batch`,
`propose_inner_with_reuse`, and `apply`). The existing writer and configuration slices
are prerequisites; read reconstruction remains a separate prerequisite.

## Inventory and virtual effects

The parameter sum type has route, vector-block and gate cases. Its source wire
kinds are respectively 0, 1 and 3; kind 2 means no virtual change and is not an
update parameter. Route parameters are unavailable in Flock and Relation.
Gate parameters are unavailable in Route. A vector-block parameter is indexed
by `token * blocks + block`; the route inventory is `blocks * stages`; the gate
inventory is the checked count defined by the read configuration.

Route and vector parameters have sixteen candidate ordinals. Zero is CURRENT;
ordinals 1 through 15 select the corresponding ordered bivector generator.
Route moves compose `generator * currentRoute`; vector moves act on the current
block. Every occurrence of a moved token in a carried batch must use that same
virtual block, and the writer must derive addresses from the virtual codebook.

Gate parameters have exactly three active ordinals. Zero is CURRENT. The other
two enumerate `[0,-1,+1]` with the current trit removed, in that order. Thus the
complete ordinal tables are:

| Current | Ordinal 0 | Ordinal 1 | Ordinal 2 |
| --- | --- | --- | --- |
| 0 | 0 | -1 | +1 |
| -1 | -1 | 0 | +1 |
| +1 | +1 | 0 | -1 |

The physical sixteen-lane loss buffer pads gate lanes 3–15 with UInt64 MAX.
Those padded lanes do not participate in selection or sensitivity. A semantic
three-candidate type can project to this physical buffer, but cannot treat the
padding as thirteen additional proposals.

## Baseline-fixed row eligibility and accumulation

Read and score the baseline before considering its candidate counterpart.
Baseline read failure is an operation error. A baseline writer failure is an
operation error, with arithmetic overflow retaining source status 12 rather
than generic status 8. If the baseline has no token rivals, skip that row for
every candidate, regardless of its loss or address rivals.

For every baseline-eligible row, reconstruct the candidate read and score it
against the consistently viewed codebook. Candidate read failure is an
operation error. Candidate arithmetic failure is an operation error. An
otherwise failed candidate writer, or a candidate with zero token rivals,
makes that candidate unavailable; removing the supervised competitor never
counts as improvement. The writer eligibility bridge must preserve these
distinctions instead of mapping every failure to an infinite loss.

Accumulate only baseline-eligible row losses with checked UInt64 addition.
Overflow is status 12. An exactly MAX sum is arithmetically representable but
collides with the candidate-unavailable sentinel. If no baseline-eligible row
exists, the whole operation is unavailable (source status 8). The model should
retain typed semantic outcomes and expose sentinel words only at the ABI edge.

## Exact selection

Selection starts at candidate zero. Gate selection replaces the incumbent only
on strictly lower loss. Equal noncurrent gate losses retain the first ordinal.
Route/vector selection additionally permits a tie replacement only when the
incumbent is already noncurrent and the new candidate's resulting port is lower.
For route moves the resulting port is the composed action index; for vector
moves it is the exact resulting root index. A remaining port tie retains the
earlier candidate. No candidate displaces CURRENT at equal loss.

The sensitivity field asks whether any active candidate cost differs from
CURRENT. It is independent of improvement: all-worse costs are sensitive,
all-equal costs are not, and ignored gate padding cannot manufacture sensitivity.
The device receipt has five words: chosen ordinal, baseline loss, chosen loss,
observed epoch, and a zero/one sensitivity word.

Controls must distinguish all-tied, all-worse, one strict improvement, competing
strict improvements, equal noncurrent losses with different resulting ports,
equal resulting ports, MAX-unavailable candidates, and inactive gate padding.
The comparisons are exact integer order, with no normalized score or probability.

## Binding and guarded application

The host source binds a proposal to the process model instance, complete Spec,
ReadSpec, objective policy, initialization policy, seed, geometry digest and
source digest. It separately stores the epoch in the proposal metadata and the
device receipt. Loading identical snapshot bytes creates a fresh model instance;
an old instance's proposal or carried batch does not become admissible.

Preserve the ordered host checks: admit parameter and index first, require the
same full binding, then require both proposal epoch copies to equal the current
model epoch. Kernel checks also compare its receipt epoch, enforce mode/index/
candidate bounds, and retain typed status distinctions. A candidate-zero request
returns unchanged only after these earlier checks. For a nonzero candidate,
reject chosen loss greater than or equal to baseline and reject epoch MAX.
Only then change exactly the selected parameter and increment the epoch once.

The result is either an unchanged state, one valid updated state, or a typed
failure preserving the entire prior state. Model/source/geometry identity,
stale epoch, inactive parameter, bad ordinal/index, non-improvement and epoch
overflow need separate negative controls. Candidate zero at epoch MAX is a
valid no-op after binding and index validation; a strict nonzero move is not.

The source's proposal buffers are private and device-authored. A public
LexLean record alone does not reproduce that trust boundary. The generated
application must expose a checked proposal-producing transition and its bound
application path, with no external root that accepts fabricated loss receipts.
Source equality is not authentication, and a hash is not an owner release.

## Training and integration obligations

This semantic transition does not release corpus training. Public training
admission remains held unless the actual owner-controlled experiment path admits
it. Formal examples, source review and a passing build do not supply that input.
GPU execution requires the separately modeled generic backend and source-bound
pre-registration, review and authorization.

The corpus driver additionally owns schedule/cursor behavior, checkpoint binding,
codebook initialization, Relation frozen-zero versus learn policy, coordinate
blocks and counterfactual execution paths. These contracts must be inventoried
before claiming the complete learning application. Lane/row execution strategies
may project the same model, but their parity is an execution claim. Neither a
selector-only module nor a single controlled state transition completes training.
