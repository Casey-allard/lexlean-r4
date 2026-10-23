# Complete Engine composition

`R4.Engine` implements the integration contract in
[`specs/engine-integration-design.md`](specs/engine-integration-design.md).
The authority is UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`, specifically
`device.rs::{make_binding,new_with_initialization,carry,carry_windows,row_plans,encode_carried,admit_batch,forward_internal,propose_inner_with_reuse,apply}`,
`discrete.cu::{discrete_init,discrete_encode,read_vector,discrete_forward,discrete_counterfactual,discrete_choose,discrete_apply}`,
and `lib.rs::{Identity,training_admission}`. Source SHA-256 pins are retained
in the reviewed design. This is a pure semantic component; no handwritten
host model, host initializer, raw Lean implementation or FFI supplies behavior.

## Composition and public semantic operations

The model is `Learning.ModelState`, including its complete binding, UInt64
epoch, flattened root codebook, frame inventory and shared typed trit gates.
`admitModel` performs structural state admission. It does not authenticate a
source digest, establish artifact provenance or allocate a unique identity.
Those are separate generated application responsibilities.

`initializeModel(binding)` constructs the entire seed-derived state. A 32-step
Nat/Word32 bridge chooses each bit with exact comparisons and additions, then
calls the existing fixed-width Words xorshift operations. No subtraction over
a seed-sized unary natural is used. The four low bits choose a canonical root
for each flattened token/block index. Routes start at identity and all gate
storage is allocated even in Route mode. ActiveLeafV1 sets each payload block's
level-zero gate positive and keeps all other gates zero. The initialization
name avoids Lean's reserved `initialize` keyword.

`allocateInstance(next)` returns the old nonzero counter and checked successor;
MAX refuses. `initializeFresh(next,binding)` initializes with that allocated
identity. `admitFreshModel(next,model)` structurally admits imported state,
preserves its epoch and parameters, and replaces only the binding instance.
This is the fresh-identity part of snapshot reload; it does not implement or
claim the separate snapshot codec, schema/kernel/source/geometry validation,
authentication, durable storage or process-global counter effect.

`carry` retains the compatibility flat surface, including nonempty storage
that is not an admitted batch. Only exact multiples of context with 1..64 rows
receive plans. Raw count and coordinate count retain checked UInt32 bounds.
`carryWindows` constructs 1..64 canonical rows from nonempty bounded windows.
It preserves token, occurrence, record and version in order, pads physical
storage with the first identity, and encodes every physical token from the
same current codebook. Each row retains its unpadded identity prefix and
canonical Binder plan. Query selection uses that prefix, never padding.

`admitBatch` checks the model and then the host's binding, row-shape, target,
and encoding-epoch sequence. Its additional public-record consistency check
reconstructs canonical windows and compares every physical identity, encoded
block, logical row and plan. An orbit-valid changed block is rejected. The
host source can trust private buffers; this stronger pure-record boundary is
explicitly a port refinement. `planWords` validates plan shape/capacity before
projecting the exact header, four-word nodes, global node padding and sixteen
frontier slots.

`forward(model,batch,targets,trace)` dispatches Route, Flock or Relation to the
existing component, then calls the current Admission writer with ordinary
telemetry disabled. Writer MAX normalization and arithmetic errors remain
typed. The Forward record retains full binding, epoch, trace availability,
actual read outputs and writer receipts. `traceWords` emits all row/block
Flock words followed by Relation scores padded to the global node capacity.
Traced Route is refused before batch execution. An untraced result cannot be
serialized as a trace.

`propose(model,batch,targets,parameter)` accepts no losses, eligibility masks,
writer observations or read results. It first admits the parameter and batch.
It computes all sixteen geometric lanes or three active gate lanes from the
same immutable model. Vector moves are mapped to the same token/block/candidate
in both the reader and virtual writer. Route moves use the global stage split;
gates use the shared ordinal mapping. Every lane evaluates each baseline row
before considering its candidate. Baseline rows without token rivals skip
candidate evaluation. An unavailable candidate terminates that lane without
visiting later rows. An exact MAX accumulator continues, whereas addition
above MAX fails. The finite loss list is converted to a closed Learning
inventory and passed to `Learning.propose`; thirteen inactive gate lanes are
padded only in the returned physical observation.

`transition(model,windows,targets,parameter)` constructs its own batch and
proposal and applies only that proposal. Failures return the entire original
model. A selected move changes exactly one route, root or gate and advances
the epoch once. CURRENT preserves all state. `trainingAdmission` always returns
`trainingHeld`.

Internal decomposition functions, including `finishProposal`,
`proposalFromCosts`, `candidateRows` and `applyInternal`, are semantic helpers,
not generated external application roots. They do not establish authentic
proposal custody. In particular, exporting a helper that accepts a supplied
proposal or costs would violate this contract. The generated application must
retain immutable admitted records behind private producer handles and perform
an atomic epoch compare/update. Such generated custody and source/artifact
admission remain required and unaccepted until provided by the production SDK.

## Error and arithmetic contract

`EngineError` distinguishes structural model/word admission, exhausted instance
allocation, identity and storage count failures, window/row shape, full binding
mismatch, target mismatch, stale encoding, inconsistent redundant storage,
unsupported trace mode, the training hold, seed bridge failure, wrapped
parameter/application errors, row/candidate-specific Route and Flock errors,
writer failures and candidate aggregation failures. Writer arithmetic retains
its existing typed variant. Candidate unavailable is a MAX lane result, not
an operation exception. A deterministic first error is a semantic presentation;
it does not claim parity with CUDA's nondeterministic competing atomic statuses.

All arithmetic is exact, and every recursion retains its complete domain.
The 64-row flat-plan search implements the source admission bound without
restricting raw compatibility carry. The 32 bit-conversion steps implement
UInt32 exactly. These are not arbitrary fuel cutoffs. Relation keeps its
reviewed checked signed intermediates. This component does not establish
D11 resource, allocation, recursion-stack or runtime performance acceptance.

## Independent controls

The frozen fixture includes literal seed roots and UInt32 boundaries, fresh
allocation/reload identities, complete codebook copying and occurrence metadata,
canonical short-window padding, redundant-storage tamper rejection, ordered
binding/target/epoch errors, current forward results and every candidate lane.
A two-token Route fixture has literal loss 128 in all sixteen route lanes;
moving its sole rival makes all fifteen non-CURRENT vector lanes unavailable.

A causal Flock/Relation fixture has query root1 and target root0 in the first
block. The source coordinates are +/-2, so their L1 distance is 8. Its baseline
token hinge is 24 and address hinge is 16: `8*24 + 16*16 = 448`.
The positive leaf gate yields root0 and loss 128, with the exact three costs
`[448,448,128]`; the selected gate changes once and old batches/proposals become
stale. Two-row costs are `[576,576,256]`. A separate three-token Route fixture
selects geometric ordinal 1, changes only token0's first block to root14 and
increments the epoch once. These expectations were authored from source
geometry and writer arithmetic, not a host model execution.

Internal recurrence controls additionally test baseline skip before an invalid
candidate, unavailable before a demonstrably failing later row, exact MAX
continuing into that later row, and above-MAX addition. Such unreachable-from-
bounded-real-cost arithmetic cases specify the helper's checked domain; they
are not claimed as actual admitted model losses. Complete trace words are
literal, including per-block BFS actions and signed Relation score padding.

The conformance test uses the shared immutable fixture digest, fresh compiler
receipt, full transitive module identities, empty theorem axioms and independent
replay. Formal controls establish these finite claims, not a universal proof
of every model or model capability. Production SDK/template acceptance,
full `just vv`, generated external roots/custody, snapshot persistence, GPU
execution/parity, research authorization and Pages delivery remain separate
required gates.

## Review corrections before the final freeze

Self-review found that comparing only `Binder.nodeWords` did not compare a
node's semantic level: levels are derived by the physical ABI and are absent
from its four-word node projection. Batch consistency now compares context,
stages, frontier and every node's level, optional atom and optional child triple
structurally. A changed leaf level is rejected at batch admission, even when
its projected ABI words would match. The level-tampering control specifically
covers this earlier admission gap rather than relying on a later read failure.

Before freezing the independent expectations, fixture setup was corrected to
use the actual ObjectivePolicy constructor and allocate Route's base gate
inventory. The causal fixture's loss calculation was corrected for +/-2 carrier
coordinates. Trace expectations were corrected to the pinned header order,
BFS action 16 for root0-to-root1 transport, and final Relation scores. With
all edge gates zero, the initial `previous = 8-distance` signal contributes
nothing to the recurrence's zero-initialized cumulative state: final scores
are all zero. The initial score is not a final trace score. No source read,
writer, gate, score or trace algorithm was changed to match these fixture
corrections. Separate block slices and the full concatenated trace are checked.

## Frozen verification and planted defects

Final source SHA-256:
`9eaa56c03d587c2041cb162f90a4be023b7a210fb1b351c1a58ed49a26a79b6b`.
Independent 100-control fixture SHA-256:
`f7d77d5fc81768a7f9376e67cfc871ed82a39d00ff9e7b2b5cb2013df963ce9c`.
The unchanged development SDK
`ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21`
verified all 16 transitive modules with attestation
`95c7253823d94b93e1853b6493d6b5e321c0cfa2be259ed95beed72dd4b81f15`.

Each following well-formed source defect was planted independently against
that exact fixture, verified using `lexlean verify tests/lexlean/EngineControls.lex.tex`,
rejected with exit 1 and `LLV7002`, and restored byte-for-byte:

| Integration defect | Intended failing control |
| --- | --- |
| Widen the virtual writer's CURRENT branch from ordinal 0 to 0..15 | `virtualTargetAndQueryUnavailable`, `virtualSoleRivalUnavailable`, `allVectorCandidatesRecomputed`, and actual vector update controls |
| Use the padded suffix instead of the logical prefix for read entries | `paddedLastSlotNeverBecomesQuery` |
| Bypass encoding epoch equality | `staleBatchRefused`, `updatedModelRejectsOldBatch` |
| Compare carried block lists only by their lengths | `changedRootRefused` |
| Revert semantic node equality to four-word ABI projection equality | `alteredUnserializedLevelRefused` |

Mutation-instrument setup attempts that failed canonical JSON, unused-variable
or structural-recursion checks were excluded from this evidence. All five
counted defects reached actual independent theorem failures. The controls and
all dependencies were unchanged throughout these plants. No compiler budget,
policy limit, theorem statement, generated Lean or axiom was weakened.

The Rust conformance fixture is theorem-only, as required by the shared gate.
Transparent constant fixture definitions were expanded into the exact same
closed theorem statements before the final freeze; no shared harness rule was
relaxed. The targeted restored `cargo test --offline --locked -p repo-conformance
--test engine` passed one test with zero failed, ignored or filtered tests,
including fresh verification and independent replay (44.39 seconds).

After repeating all five theorem-failing plants against the final theorem-only
fixture, the exact restored source passed the targeted conformance test again:
one passed, zero failed/ignored/filtered, 42.18 seconds. The content-addressed
attestation remained `95c7253823d94b93e1853b6493d6b5e321c0cfa2be259ed95beed72dd4b81f15`;
the shared harness also checked fresh invocation receipts and replay. No source
or fixture mutation remains.
