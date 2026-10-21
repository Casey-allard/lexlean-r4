# Finite learning transitions (R4-08)

R4-08 models complete finite candidate selection and guarded geometric state
updates in pure LexLean. Its checked entrypoints are `propose`,
`applyProposal`, `transition`, and `aggregateCandidate`. The 154 formal
controls establish their stated finite examples. They do not establish a
universal correctness theorem, counterfactual read execution, device proposal
authentication, GPU parity, or permission to train.

The authority is UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`:

- `crates/uor-r4-discrete/src/discrete.cu`, especially lines 384–454,
  SHA256 `8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.
- `crates/uor-r4-discrete/src/device.rs`, especially `Binding`,
  `parameter`, and `apply`,
  SHA256 `47acbd7398ac95e87d9e6b58594012e14d91016941adb90fa72563e41afdd6ae`.

## Representation and admission

A model state carries its full binding, UInt64 epoch, ordered codebook roots,
ordered route indices, and ordered ternary gates. The flattened codebook index
is `token * blocks + block`. Each root must belong to the modeled geometric
orbit, each route must be a valid action index, and all inventories must have
the sizes derived from the admitted Spec and ReadSpec. Route count is
`blocks * stages`; gate count reuses Configuration's checked mode-specific
formula. Canonical context stages use the exact admitted context boundaries
1, 3, 9, 27, 81 and 243. This helper is used only after Spec admission.

The binding includes the model instance, every Spec field, complete ReadSpec
including kept/sweeps, objective policy, initialization policy, seed, geometry
digest and source digest. Digests are closed tuples of eight admitted UInt32
words. Equality checks every word. This representation neither computes SHA256
nor authenticates external source bytes; a future artifact producer must supply
and verify the actual identities. A reloaded model's distinct instance identity
therefore remains relevant even when its source and geometry are equal.

Natural-number fields are explicitly admitted into the source UInt32/UInt64
domains. Local `outsideWord` and `invalidState` failures describe the checked
semantic interface; source operands already have machine-word types. They are
not additional CUDA status codes. Mode and index admission precedes proposal
binding checks, matching the source host's parameter admission.

## Candidate inventory and selection

Route and vector-block proposals contain exactly sixteen costs. Ordinal zero
is CURRENT; ordinals 1–15 apply the ordered Geometry generators. Route effects
compose the generator with the current route. Vector effects act on the
selected codebook root. Tie-breaking compares the actual resulting action or
root index, rather than the ordinal or a surrogate distance.

Selection starts at CURRENT. A strictly lower cost replaces the incumbent.
For route/vector candidates, equal costs can replace an already noncurrent
incumbent only when the new resulting port is lower. A remaining port tie
retains the earlier candidate. CURRENT is never displaced by an equal cost.

Gates share the `R4.Gates.Trit` type with other read components. Their complete
three-candidate inventory is:

| Current | Ordinal 0 | Ordinal 1 | Ordinal 2 |
| --- | --- | --- | --- |
| zero | zero | negative | positive |
| negative | negative | zero | positive |
| positive | positive | zero | negative |

This is the source sibling order zero, negative, positive with CURRENT removed.
Gate selection uses strictly lower costs only. The physical sixteen-word
projection pads thirteen inactive lanes with UInt64 MAX. Padding participates
in neither selection nor sensitivity. Sensitivity is true exactly when some
active cost differs from CURRENT, including the all-worse case.

Gates also provides checked word and trace projections. The signed Int32 wire
bit patterns for zero/negative/positive are 0/4294967295/1. The source trace
encoding `gate + 1` is 1/0/2 respectively. Reverse projections reject other
words. Gate ordinals outside 0–2 are rejected by the checked interface.

The observation projection has exactly five words: ordinal, baseline loss,
chosen loss, observed epoch, and sensitivity encoded as zero or one.
Loss inventories are explicit semantic data; the selector does not derive
them by executing a read or writer.

## Guarded update and complete refusal preservation

Application validates the whole input state and parameter, compares the full
binding, admits proposal words, then separately compares metadata epoch and
observed epoch with the current state. Metadata mismatch has precedence when
both epoch copies are stale. Candidate bounds are checked before no-op or
improvement handling.

Candidate zero returns the unchanged state after those checks, including at
epoch MAX. A nonzero candidate must have strictly lower loss and the current
epoch must be below MAX. Application then changes exactly one admitted route,
codebook root, or gate and increments the epoch once. Every typed refusal
returns the complete original state, not a reconstructed partial state.
Lookup and replacement functions return Options; failure never silently
truncates or substitutes a default geometric value.

Internal helpers such as `performMove` and `applyReady` implement steps under
their callers' assumptions. They are not standalone authenticated application
entrypoints. Even `applyProposal` checks a public semantic record: it does not
reproduce the source's private device-authored proposal buffers. A generated
application must connect an admitted, bound proposal producer to application
and must not expose an external root accepting fabricated losses or receipts.

## Explicit candidate-batch aggregation

`aggregateCandidate` accepts 1–64 paired baseline and candidate observations,
with exact list-length agreement. It represents one candidate lane. Inputs
are explicit read results and writer results; the model does not perform
counterfactual read reconstruction or supervise a GPU kernel.

For each row, baseline read or writer failure is an operation error. A valid
baseline with zero token rivals is skipped without inspecting the candidate,
even if that candidate reports a read or arithmetic error. An eligible
baseline fixes eligibility for its candidate counterpart through
Admission.eligibility. A candidate read failure or arithmetic writer failure
remains an operation error. Other candidate writer failure, or losing all token
rivals, makes this candidate unavailable and ends its accumulation.

Losses use checked UInt64 addition. A sum greater than MAX is arithmetic
overflow. An exactly MAX sum remains representable: subsequent rows are still
processed, so another positive loss can overflow and a later read failure
remains an error. If accumulation finishes at exactly MAX, the result is
unavailable because the value collides with the source sentinel. This differs
from an unavailable writer result, which ends the candidate immediately.

A writer value whose loss is already MAX is normalized to the source sentinel
failure before eligibility. Baseline failure retains its precedence even if
that synthetic receipt also has zero token rivals. No baseline-eligible rows
is a distinct operation error. Loss and token-rival words are admitted because
those fields are consumed here; validity of the remaining supplied receipt
fields belongs to their verified producer.

A future producer must reuse the same admitted baseline batch for all
candidate lanes and bind it to the model and virtual codebook. This component
does not invent a deterministic priority among concurrent lane errors; the
source's parallel atomic error publication does not establish such an order.
It also does not implement carried-batch authentication or baseline reuse.

## Verification and remaining integration

The scenario and an initial missing-module control were written before the
definitions. The immutable SDK rejected that initial source with LLR3005.
A later missing helper control failed LLT4001 before its generic structural
definition was added. The source field initially named `instance` exposed an
unescaped reserved Lean token during backend generation; renaming that field
to `modelInstance` corrected the source without changing any statement or
compiler settings.

Three planted source defects were each rejected by Lean with LLV7002 while
the control statements remained unchanged: removing the observed-epoch guard,
allowing equal-loss updates, and reversing the resulting-port tie comparison.
The first two failed their full-state refusal controls; the third failed exact
selection and composed-transition controls. The original source was restored
byte-for-byte after these negative checks.

The 154 controls cover all nine gate candidates and codec boundaries, exact
sixteen-candidate selection, resulting-port ties, active-only sensitivity,
single-parameter state effects, binding fields and error ordering, both epochs,
mode/index/word/ordinal bounds, epoch overflow, exact batch sentinel/overflow
behavior and 64/65-row bounds. Expected states and arithmetic values are
literal independent fixtures. Rust invokes the digest-pinned LexLean verifier,
binds the exact control digest and nine-module graph, and requires successful
proof and kernel replay through the shared conformance support.

Finite formal reduction is compiler verification, not a CPU model research
run. Corpus scheduling, optimizer coordinate schedules, counterfactual reads,
virtual codebook propagation, initialization, checkpoint/resume, private
proposal custody and device execution remain integration obligations. Relation
frozen-zero versus learn policy belongs to the learning driver. Training
admission remains HELD under the source contract. This component supplies no
owner identity phrase, release, or scientific capability claim.
