# Flock and Relation over explicit carried occurrences

`R4-10` implements the complete per-row Flock and Relation read component in
`R4.ReadArithmetic`, `R4.Relation`, and `R4.Flock`. Its imported authority is
`crates/uor-r4-discrete/src/discrete.cu` at UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`, source SHA-256
`8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.
The relevant functions are `gate_value`, `snap`, `frame_block`, and
`flock_vector`. The approved source contract is in
`docs/specs/flock-relation-design.md`.

Every algorithm is authoritative LexLean source. Rust checks the frozen
independent theorem bytes and fresh formal-verification receipts. There is no
handwritten host implementation or measured model result.

## Public input and admission

`Flock.run` accepts `Types.Spec`, `Configuration.ReadSpec`, a general
`Binder.BinderPlan`, ordered `Route.Entry` occurrences, gate words, a virtual
view, and a trace flag. It returns all payload-block receipts in block order
or one typed error, without a partial output vector. `outputBlocks` projects
the carried output. Entries retain token identities separately from geometry;
duplicate identities are permitted and are never deduplicated.

Admission checks Spec bounds, Flock/Relation kept and sweep bounds, general
Binder validity, and canonical global-context node capacity. Row length must
fit the model context and match the occurrence inventory. The existing Route
admission checks every token, payload dimension, and geometric block. Gate
inventory is exactly `blocks * globalStages`, plus `5 * globalStages` in
Relation. Global stages derive from the canonical model context, including
for a short or valid noncanonical row forest.

The public wire gate inventory accepts exactly `0`, `4294967295`, and `1`,
corresponding to signed trits zero, negative and positive. `R4.Gates` owns the
single trit type and candidate table shared with Learning. Candidate zero
means CURRENT. Candidates one and two enumerate `[0,-1,+1]` after removing
the current trit. Trace `gate+1` uses different words: zero maps to one,
negative to zero, and positive to two.

Full admission is stronger than CUDA's visited-field validation. Error order
is this explicit interface's order, not a claim of identical diagnostics on
malformed raw CUDA arrays. Gate shape is checked before virtual-view bounds;
all gate words are decoded before execution. Route virtual moves are refused.
Vector moves use Route's exact occurrence-aware input helper for both query
and leaves, including frame endpoints and every matching token occurrence.

## Exact arithmetic and per-block forest

Read arithmetic reuses `Carrier.Scalar`: `nonnegative(n)` means n and
`negative(n)` means -(n+1). Addition, ordering, cancellation, block sums and
repeated scaling are exact. `snap(query,n)` scans all sixteen ordered roots
and minimizes L1 displacement from n copies of the root, then coordinate
Hamming distance, retaining the earlier root on a tie. Zero scale is rejected;
an empty selection never calls snap.

For payload block zero, the frame is the final block; every other payload
block uses frame block zero. The final occurrence supplies query frame and
payload. Each block rebuilds its own forest in stored node order. Leaf payloads
and frames come from the virtual occurrence. A parent retains its first
child's frame, transports each child with the actual BFS action into that
frame, sums the three payloads and applies snap with scale three. Metadata
retains the first and last atom positions and derived node level.

These are structurally recursive exact definitions. They make no claim of
efficient generic large-integer reduction or generated serving throughput.

## Relation recurrence

Relation initializes previous scores to eight minus frame L1 distance and
accumulated scores to zero, separately for every payload block. Each sweep
reads a complete immutable previous vector and adds signed gated edge sums
to the retained accumulated vector. The next previous vector becomes the
complete new accumulated vector only after the whole sweep finishes.

The five edges are parent/children, adjacent siblings, preceding same-level
span, latest earlier leaf with the same token identity, and latest earlier
same-level node with the same frame root. The last two overwrite on each
match. The first three sum contributions. Gates are indexed by the receiving
node's level. Propagation includes any node whose final atom is inside the row,
including query-containing nodes; causal selection uses a different predicate.

Every contributing score addition, trit sign application, and cumulative
addition is checked against signed 32-bit range. Failure propagates as a typed
Relation arithmetic error; no wrapping, clipping, or substituted score occurs.
This is an explicit source-safe arithmetic refinement. A conservative forest
argument gives at most nine contributions, initial magnitude at most eight,
first-sweep magnitude at most 72 and five-sweep magnitude at most 720,000.
The generic forest prerequisites and bound have not yet been formalized as
universal theorems; finite controls do not replace them. No undefined signed
overflow behavior is imported as a source contract.

## Selection, residual and trace

For each global level, eligible nodes have that level and `last + 1 < length`.
Selection orders by score descending, frame L1 ascending, Hamming ascending,
last position descending, and node index ascending. Flock uses zero scores.
The ordered selection retains at most K nodes, carries each payload into the
query frame with its actual BFS action, and snaps their sum using the actual
retained count. Empty selection contributes nothing.

A positive gate adds consensus to the query residual; a negative gate
subtracts it. A nonzero gate with nonempty selection increments the term count,
which starts at one. Final output snaps the residual at that term count.
The same execution computes observations whether or not they are requested;
the trace flag only retains or drops the resulting semantic records. It cannot
disable admission, recurrence, selection, or output computation.

Each block receipt contains output, original query frame and payload roots,
term count, level traces and optional Relation scores. Zero gates still record
selection and nonempty consensus when tracing. Empty levels retain absent
consensus, zero eligible/found counts and empty picks. `traceWords` projects
the four-word header and 28-word level records, with three eight-slot arrays
padded by UInt32 MAX. `relationWords` projects signed scores and pads unused
global-node-capacity slots; it rejects truncation and out-of-range scores.
Active score -1 projects to UInt32 MAX too: node count and position distinguish
an active signed score from padding. These projections consume valid semantic
receipts; they are not replacement public admission paths for arbitrary records.

## Typed failures

`ReadError` distinguishes invalid Spec/read configuration, invalid Binder plan
(retaining its error), plan capacity, row shape, row admission (retaining the
Route error), gate shape, invalid gate position, invalid virtual view and
inactive Route moves. Internal helpers explicitly report missing nodes,
invalid geometry, unavailable transport/snap, missing gates or mismatched score
inventories. Relation failures retain missing node/score/gate or arithmetic
classification. Some internal failures cannot arise after successful full
admission and the verified finite geometry tables; they remain explicit rather
than silently choosing a root, score or payload.

## Formal evidence and remaining obligations

The initial controls and scenario preceded implementation. The explicit
development verifier failed with exit 1 and `LLR3005` because
`FlockControls` imported the absent `R4.ReadArithmetic` module.

The independent controls use literal outcomes, including a full public
Relation example that changes the selected token and final output relative
to Flock; different Relation scores for separate payload blocks; empty and
zero-gate traces; actual-count consensus; nested parent construction; all five
edge kinds; synchronous cumulative multi-sweep scores; virtual duplicate,
query, frame and interior-block changes; exact error paths; maximum configured
context, block count, kept count, sweep count and token identity. They do not
compute expected read results through the implementation.

The final fixture contains 119 axiom-free closed theorems. The selected graph
contains FlockControls, Binder, Carrier, Configuration, Flock, Gates, Geometry,
Objective, ReadArithmetic, Relation, Route and Types. The exact reviewed inputs
are:

| Input | SHA-256 |
| --- | --- |
| ReadArithmetic | `c92c001cb8982c11ce9558040149a9249e6ad14c698c2c8d4f61946fb2437b0d` |
| Relation | `1c9e4b02b07241463f5f49674e8811bacb3d569beeba1288da40bb1a85940784` |
| Flock | `b0dc66d88c0e004f1a70eff679f5ce2c4cc518cf76530beec70d74a245a057fa` |
| FlockControls | `b5d21afc86c3a6feccda2c79d006caad7d257713536a5749d66497558a66992c` |

Five well-formed defects were planted separately in the actual source with
the control bytes unchanged. Each explicit development verifier invocation
exited 1 with `LLV7002`, rejecting control equalities:

| Planted defect | Observed contract disagreement |
| --- | --- |
| Append every token/frame match instead of replacing the latest link | Contributor inventories and multi-sweep Relation scores |
| Retain the initial zero accumulator at each recursive sweep | Cumulative multi-sweep scores and public Relation receipts |
| Admit nodes whose last atom is the query | Causal selection and public read results |
| Add consensus under a negative residual gate | Exact signed residual subtraction |
| Return root zero consensus for an empty level | Public per-block trace absence |

The first attempt to plant cumulative reset used an unavailable forward helper
reference and failed linking with `LLT4001`. That instrument fault is excluded
from semantic evidence. The corrected plant was well-formed and failed the
theorem check above. Every mutation was restored byte-for-byte.

Verification uses the already pinned network-disconnected development image
`ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21`.
The targeted command is
`cargo test --offline --locked -p repo-conformance --test flock`; it invokes
the same formal verifier and checks fresh source, config, lock, build and Lean
receipt identities. The initial final-fixture run passed one test with zero
failures, zero ignored tests and zero filtered tests in 21.33 seconds.
After all mutations were restored, the same command again passed one test
with zero failed, ignored or filtered tests in 31.29 seconds. The matching
restored-source attestation is
`6cbb9e3703de334d83145e479455e9e22e1b8f9744f0fc2073ea02e3b11558f4`.
Final source/control hashes matched the table and `git diff --check` passed.

Production acceptance still requires the absent production `prismpm.lock`. Full carried
batch provenance, counterfactual orchestration, GPU parity, generated runtime
performance and serving qualification remain separate obligations. This
component establishes no scientific or language capability.

One proposed mutation replaced actual retained count m with configured K in
consensus snap and survived the 119 controls. That survival is recorded rather
than reported as a detected defect. The source retains the exact actual-count
argument. A static explanation is available for this orbit: its roots occupy
four disjoint coordinate pairs. For a sum of m roots and any scale n >= m,
each coordinate magnitude is at most 2m <= 2n, so L1 ordering is independent
of n after the common 4n term is removed. An exact coordinate match at n=m
requires all m roots to share that coordinate and sign, forcing one coordinate
pair to be the unique L1 winner. Its remaining sign tie, if any, has zero in
that coordinate and stays tied. Otherwise exact scaled-coordinate matches do
not occur and Hamming ordering is independent of n too. This mathematical
argument has not been encoded as a universal theorem. No artificial scale
observation is added to the public receipt just to distinguish this mutation.
