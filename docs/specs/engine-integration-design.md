# Engine integration contract

Status: design elaboration within the approved full-scope port. No Engine
implementation, model execution, GPU parity or training admission is claimed.
This document specifies the next complete integration component; it does not
replace the requirements of the independently reviewed source components.

Authority is UOR-R4 `4611538bb4843947363ccb9cce5f388e32c46883`:

- `crates/uor-r4-discrete/src/device.rs`: `make_binding`,
  `new_with_initialization`, `from_snapshot`, `carry`, `carry_windows`,
  `row_plans`, `encode_carried`, `admit_batch`, `forward_internal`,
  `propose_inner_with_reuse`, and `apply`; SHA-256
  `47acbd7398ac95e87d9e6b58594012e14d91016941adb90fa72563e41afdd6ae`.
- `crates/uor-r4-discrete/src/discrete.cu`: `discrete_init`,
  `discrete_encode`, `read_vector`, `discrete_forward`,
  `discrete_counterfactual`, `discrete_choose`, and `discrete_apply`; SHA-256
  `8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.
- `crates/uor-r4-discrete/src/lib.rs`: `Spec`, `ReadInitialization`,
  `Identity`, and the unconditionally held `training_admission`; SHA-256
  `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9`.

## Complete component and dependencies

`R4.Engine` must compose codebook encoding, canonical causal rows, read-mode
dispatch, writer scoring, complete candidate computation, selection, and
atomic guarded application. Its proposal-producing public semantic entrypoint
accepts the model, a carried batch or windows, targets and a parameter. It
must not accept caller-provided losses, writer receipts, read outcomes or
eligibility masks. `Learning.propose` remains an internal integration step.

Reuse `Types`, `Configuration`, `Binder`, `Geometry`, `Carrier`, `Admission`,
`Route`, `Gates` and `Learning`. The full component additionally requires the
independently accepted Flock/Relation read and signed arithmetic components.
Their in-progress names below describe the currently visible interfaces;
freeze their reviewed hashes before implementing the Engine bridge. No host
Rust, JavaScript, raw Lean or FFI path may supply missing read or writer logic.

The coherent public operations are: admit a model, construct encoded storage,
construct an admitted window batch, forward that batch, compute a proposal
from actual candidate reads, and a composed semantic transition that applies
only its internally produced proposal. Seed initialization is a complete
additional entrypoint specified below; importing a separately admitted
codebook is supported independently and must not imply seeded initialization.

## Identities and model state

Reuse the complete `Learning.ModelState` and `Learning.Binding`. The binding
contains model instance, every Spec field, full ReadSpec including kept/sweeps,
objective policy, initialization policy, seed, geometry digest and source
digest. Retain the existing UInt32/UInt64 admission and full eight-word digest
equality. Do not introduce a smaller Engine identity or compare only digests.

An occurrence identity contains exactly `token: UInt32`, `occurrence: UInt64`,
`record: UInt64`, and `version: UInt64`, as in the host source. All four fields
are preserved in caller order. Only token selects codebook geometry and the
same-token Relation link. Record, version and occurrence are neither a metric
nor a model epoch. Do not sort, deduplicate, infer document boundaries, demand
unique occurrence fields, or reinterpret record version as model epoch.

The flattened codebook is ordered by `token * blocks + block`. Each entry is
a complete admitted root, every route is in 0..63, and every gate has the
shared `Gates.Trit` type. Inventories are exactly vocabulary*blocks roots,
blocks*globalStages routes, and Configuration's mode-specific gate count.
Global stages and node capacity come from `Binder.construct(spec.context)`;
Learning's boundary helper agrees on admitted contexts but is not a separate
source of row construction. State epoch is UInt64.

The source allocates model instances using a process counter starting at one,
returning the old counter only when checked increment succeeds. Thus a modeled
allocator admits next-instance values 1..MAX-1; MAX refuses allocation. A
snapshot reload obtains a fresh instance even if every saved byte agrees.
A semantic binding field does not itself ensure fresh allocation or custody.

## Codebook encoding and row construction

`encodeOccurrences(model, physicalIdentities)` copies, in occurrence order,
all blocks of `model.codebook[token]`; no transform or distance is involved.
An occurrence's original metadata is retained separately from its encoded
root blocks. Equal tokens copy equal current blocks; distinct identities may
still share equal roots. Token and count admission precede indexed copying.

Preserve two source surfaces:

1. Compatibility flat `carry`: nonempty valid identities can be encoded even
   when their count is not a positive multiple of context, or gives more than
   64 full rows. Such storage has no usable row plans and cannot be passed to
   forward or propose. Count and count*width still require checked UInt32
   admission. Do not silently discard a remainder or fabricate a partial row.
2. `carryWindows`: require 1..64 windows, each with 1..context valid identities.
   The caller already chose causal/document boundaries. For each window,
   preserve its logical identities, append copies of its first identity up to
   model context for physical storage, and construct the canonical Binder plan
   for the logical length. The last valid identity, not the final padded slot,
   is the query. Padding must never enter a row plan, selection or writer target.

The first surface may be represented as `EncodedStorage` with optional row
metadata; the second returns `CarriedBatch`. Preserve logical identities,
physical IDs/blocks, valid lengths and per-row plans, full binding and encoding
epoch. Do not claim that absence of a public record constructor makes either
record unforgeable in a generated runtime.

A physical plan row is `[length, nodeCount, frontierCount]`, then four words
per stored node, then ABSENT words up to global node capacity, then frontier
IDs and ABSENT padding to sixteen. Stride is `3 + 4*globalNodes + 16`.
Use `Binder.nodeWords` only after construction/validation. A semantic row can
retain the unpadded valid prefix, but the explicit projection must reproduce
physical padding, offsets, order and capacities exactly.

Standalone Route/Flock accept general valid forests. Engine-created rows are
canonical because the source `row_plans` calls `BinderPlan::new(length)`.
Engine must not erase the standalone components' broader contract or accept
an arbitrary valid forest as evidence that it came from Engine construction.

## Batch admission and custody

Preserve host check ordering for forward: full binding equality; 1..64 rows,
nonempty bounded lengths, plans present and matching physical ID count; one
valid target per row; then encoding epoch equals current model epoch.
For proposal, parameter/mode/index admission precedes batch admission.
Additional natural-word and model-shape checks are explicit port boundaries,
not fabricated CUDA status codes.

The original host can trust private vectors and plans constructed by its own
methods. Pure public records need an explicit semantic consistency check:
logical rows, first-identity padding, canonical plans and every carried root
must agree with the admitted identities and the current codebook. Reject a
changed block even if it remains an orbit root and binding/epoch match.
Reject altered identities, lengths or plans whose redundant projections
contradict one another. This is a stronger external-record admission boundary,
not a claim the host source recomputes all its private buffers on every call.

Prefer a generated application API that holds immutable admitted models and
batches in private storage and returns opaque handles minted only by successful
constructors. Handle lookup must bind the full model instance and epoch and
must not permit caller replacement of stored fields. When the SDK cannot
supply this custody/effect boundary, keep runtime acceptance blocked while
retaining the complete pure semantic consistency checks. No custom handwritten
application backend may impersonate the missing generated custody mechanism.

Proposal metadata epoch and observed device epoch are distinct from batch
encoding epoch: the batch must match the state when producing a proposal, and
both proposal epoch copies must match again at application. Any successful
nonzero route, vector or gate change increments the model epoch once and makes
all older batches/proposals stale, even if their copied vectors happen to be
unchanged. A CURRENT no-op preserves epoch and admitted batch usability.

## Read dispatch and virtual views

Split the complete route inventory into exactly blocks rows of globalStages
frames for Route. Convert the shared trits to Flock's checked gate-word surface
with `Gates.wireWord`, or use a reviewed typed adapter if available. Root blocks
convert through `Carrier.fromRoot`. Every logical row passes its own plan and
valid entries while retaining the full Spec context for global capacities.

Dispatch ReadSpec Route to `Route.run`; dispatch Flock and Relation to the
reviewed `Flock.run(spec, read, plan, entries, gates, view, trace)`. Preserve
read-specific typed errors. Project each returned block's actual output into
one ordered writer carrier. Do not reconstruct output from diagnostic traces.

An Engine parameter maps to one consistent virtual view:

| Parameter | Read view | Writer view |
| --- | --- | --- |
| Route(flatIndex), candidate | checked block/stage split using globalStages; generator*frame | current codebook |
| VectorBlock(flatIndex), candidate | checked token/block split using spec.blocks; transform every matching occurrence | same token/block/candidate in Admission's virtual codebook |
| Gate(index), candidate | shared three-way gate ordinal and same flattened index | current codebook |

CURRENT is exactly ordinal zero. Route moves are inactive in Flock/Relation;
gates are inactive in Route. Vector views change payload and frame occurrences,
including the query and selected leaves; the writer changes target geometry,
occupied cells, rivals and addresses using that same virtual block. Applying a
virtual read change with a current writer codebook is a mandatory negative
control. Each candidate starts from the same original state; never compose
successive candidate moves with one another or mutate the baseline state.

## Forward result

`forward` admits batch and targets, runs the current read for every row, and
calls `Admission.writer` with the admitted current codebook, row target,
selected objective and telemetry false. A read failure is an operation failure;
a failed writer or selected MAX sentinel also prevents a successful Forward.
Preserve arithmetic failure separately from the generic writer failure.

Return binding, observed epoch, carried output vectors, predictions, UInt64
losses, token/address rival counts and sixteen admitted-cell slots per row.
Pad unused admitted-cell slots with UInt32 MAX, as `writer` does. Targets
supervise scoring; they do not enter read construction or candidate geometry.
Optional richer writer telemetry belongs to a separately named evaluation
surface and must not silently change the ordinary forward projection.

`forwardTraced` rejects Route before batch execution, matching the host API.
Flock/Relation traces must come from the same read result. Preserve global
stage/node padding, row/block order and the separate signed Relation score
projection. Traced and untraced output vectors and writer results must agree.
No trace, diagnostic buffer or cache can become a second implementation.

## Complete candidate computation

For each proposal use one immutable admitted state, batch, target vector and
parameter. Construct exactly sixteen geometric candidates or three gate
candidates. Every active cost is computed through actual read and writer
composition. The fixed physical gate buffer pads thirteen inactive lanes with
UInt64 MAX; those lanes do not become candidates or sensitivity inputs.

Baseline read/writer results may be shared as immutable semantic results if
proven identical to the ordinary source path. The source recomputes them per
lane; sharing is an equivalence obligation, not a performance claim or license
to accept caller-supplied baseline records. Retain the same row/target order
for every candidate. No API may inject a different eligibility mask or select
its own baseline per lane.

For each candidate process rows in source order. Evaluate the baseline first;
fail for a baseline read/writer failure. Skip a zero-token-rival baseline row
without performing its candidate read or inspecting a candidate error. For an
eligible baseline, perform that candidate read and writer with the paired
views above, then use Learning's exact eligibility and checked accumulation.
Candidate unavailable ends that lane immediately; later rows must not produce
an observable error for that lane. Exactly MAX accumulated by valid additions
continues through later rows, potentially overflowing or failing, and becomes
unavailable only at successful completion. This is distinct from an unavailable
writer result. No eligible baseline is an operation failure.

The implementation can share `Learning.rowOutcome` and `aggregateCandidate`
only if its producer preserves this conditional/prefix behavior. Building all
candidate outcomes eagerly and throwing later errors before the aggregation
step is wrong. Returning an error as inert semantic data is different from
raising the operation failure before the source would inspect that row.

Any operation error prevents selection and proposal issuance. CUDA lanes
publish errors concurrently with atomic exchanges, so the numerical winning
status among multiple failing lanes is not deterministic source behavior.
A semantic error can retain all observed lane/row causes, or expose an
unspecified source-status projection; a deterministic presentation order must
not be claimed as CUDA error-priority parity.

With complete successful lane aggregation, create the closed Learning loss
inventory internally and call `Learning.propose`. Its actual resulting-port
tie-break, sensitivity, full binding and both epoch copies are authoritative.
No caller-supplied loss vector or proposal observation is an accepted Engine
proposal-producing root.

## Application and the training hold

A complete pure `transition(model, windows, targets, parameter)` constructs and
admits the batch, computes all required read/writer outcomes, selects, and
calls `Learning.applyProposal` on its internally produced proposal. Every
failure returns the entire unchanged input model. A strict chosen move changes
one root, route or trit and increments the epoch exactly once. CURRENT retains
the whole state. Never apply partial candidate state or commit while another
candidate/row has failed.

A split generated API may expose propose then apply through private producer
handles. At apply, recheck parameter admission, complete binding and both
proposal epochs against the current model, and atomically compare-and-update
that state. Two proposals for the same old epoch cannot both update. Semantic
equality and digest comparison alone do not provide this ownership, exclusion
or authentic-loss property. A raw public `Learning.Proposal` must not become
an external generated application root accepting fabricated losses.

The source public corpus `training_admission` always returns TrainingHeld.
Preserve it. The pure transition is a formal component for a separately
admitted execution path, not an exposed unrestricted training command. No
Boolean in a user-supplied record, synthesized owner phrase, model proof or
unit-test pass releases this hold. Corpus scheduling, optimizer policy,
checkpoint serialization and research authorization remain separately required
integrations under the existing full-scope plan.

## Complete seed initialization and model admission

Two explicit model origins are supported. `admitModel` accepts a complete
separately supplied codebook/routes/gates/epoch after the existing full state
checks and external artifact identity admission; it makes no claim those
values were derived from the recorded seed. `initialize` derives every value
from admitted seed and policies, not from placeholder literal geometry.

For flattened root index j=token*blocks+block, use exact UInt32 arithmetic:

    x0 = seed XOR j XOR 0x9e3779b9
    x1 = x0 XOR (x0 << 13)
    x2 = x1 XOR (x1 >> 17)
    x3 = x2 XOR (x2 << 5)
    root = canonicalRoots[x3 AND 15]

Every left shift truncates to 32 bits, every right shift is logical. Reuse
Words.Word32 operations; add an explicit checked Nat/word bridge if required.
Do not substitute mathematical unbounded shifts, an alternate PRNG, or a host
initializer. All eight coordinates of one block come from the same selected
root. Routes initialize to identity. Gates initialize to zero. ActiveLeafV1
then sets only each payload block's level-zero gate at block*globalStages to
positive, leaving other levels and all Relation edge gates zero. Epoch remains
zero. Configuration rejects ActiveLeafV1 with Route.

Source/geometry digest production and artifact custody are distinct from this
scalar initializer. Source geometry digest includes the exact operator and
composition kappa strings followed by all 5120 geometry words in little-endian
UInt32 form. Import verified binding bytes or implement the complete named
digest producer before claiming provenance; never hash an invented substitute
serialization. Snapshot import must validate its full schema/source/kernel/
geometry identity and allocate a fresh instance. Checkpoint codec construction
is not silently included in a state-admission helper.

## Domains, capacity and failures

All occurrence metadata and epochs fit UInt64; tokens, seeds, flattened
parameters, buffer offsets and ABI words fit UInt32. Checked public records
must reject out-of-domain naturals before deriving pointers or inventories.
Under admitted forward bounds, width<=128, rows*context<=15,552 and encoded
coordinate count<=1,990,656; the largest codebook has 131,072 root blocks and
1,048,576 coordinates. Global nodes<=364, stages<=6, routes<=96 and Relation
gates<=126. The physical plan stride is at most 1475 words and total 94,400.

Read outputs are snapped orbit roots, so the existing writer's source-safe
integer refinement holds. Preserve the reviewed Relation arithmetic admission
and all source-safe-domain errors; do not infer signed overflow behavior from
unbounded mathematical arithmetic. Exact integer semantics do not establish
production preallocation, runtime stack use, storage durability or resource
acceptance. The raw flat compatibility carry has its separate checked UInt32
inventory bound and is not limited to a forward-admitted batch.

Typed Engine failures should retain: model/word admission; instance exhaustion;
identity/token/count/shape admission; missing or malformed canonical plans;
full batch binding mismatch; target count/value; stale batch epoch; inactive
parameter or bad index; unsupported trace mode; wrapped Route/Flock read
failures with row and candidate; baseline/candidate writer versus arithmetic
failures; no eligible rows; candidate unavailability; and wrapped Learning
application refusal. Candidate unavailability is a lane result, not a whole
operation exception. Generated custody lookup, revoked handles, concurrent
state change and storage failures require distinct effect-level failures.

## Independent controls and delivery boundary

Start with registered scope, independent expectations and RED before source.
At minimum freeze controls for:

- Seed 0x9e3779b9 with vocabulary 2 / blocks 2: flattened indices 0..3 produce
  roots 0,1,2,3. For j=1 the xorshift result is 270369 (0x00042021), whose low
  nibble is 1. These are literal seed-initializer checks, not a host oracle.
  Verify zero/active-leaf gate placement for multilevel Relation and UInt32
  wrap/high-bit cases independently.
- Distinct occurrence/record/version fields survive exact codebook copying;
  repeated token identities retain all occurrences. Short windows pad with
  their first identity while plans and queries use only the valid prefix.
- Nonmultiple flat carry is encoded but refused by forward; windows 0/65,
  empty/oversized rows, bad token/target words, plan tampering and geometry-
  valid carried-vector tampering are rejected explicitly.
- Full binding mismatch and stale batch epoch; fresh-instance reload;
  two valid proposals at one epoch with the second refused after the first
  update; both proposal epoch guards; CURRENT leaves old batches usable.
- Context 1 Route, two identical two-block root0 codebook tokens, keptCells 1,
  query token 0 and target 0: prediction 0, tokenRivals 1, token hinge 16,
  loss 128. Every route candidate has cost 128 and CURRENT is insensitive. Moving token 1's first
  block makes its candidate unavailable by losing the sole rival, despite
  the carrier/target remaining unchanged; it must never become an improvement.
- Virtual vector changes shared query/frame/leaf occurrences and writer cells
  together; nonmatching identities remain unchanged. Compare a deliberately
  inconsistent current-writer/virtual-reader implementation against literals.
- Every read mode dispatches through the actual component; trace mode changes
  observations only. Global context budget and logical row length remain
  distinct. All three gate ordinals and all sixteen geometric ordinals are
  produced, and gate padding never affects sensitivity or selection.
- Baseline-skipped rows do not evaluate candidate failures; unavailable lanes
  stop before later errors; exactly MAX sum continues; above MAX fails; identical
  baseline row/target pairing is used across all active candidates.
- Whole-state preservation on any failure, actual one-field geometric update,
  no mutation during proposal, and no acceptance of externally supplied losses.

Plant defects at the composition boundary, not only inside already tested
components: skip virtual writer mapping, use padded query position, weaken
batch epoch admission, accept a modified carried root, or bypass actual
candidate reads with supplied costs. Require intended-control failure before
restoring the exact source. Bind all transitive modules in the conformance
receipt, add mandatory test audit and default control root, regenerate locks,
and obtain independent spec then code/security review.

Formal component verification remains separate from production SDK/template
acceptance, generated-root custody tests, GPU execution/parity, research
authorization and GitHub Pages application delivery. These remain required
parts of the overall implementation; the Engine source component does not
claim them complete.
