# Flock and Relation read contract

Status: source-contract research for the approved full-scope port. This document
claims no implementation or execution. Authority: UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-discrete/src/discrete.cu`, especially `gate_value`, `snap`,
`frame_block`, `flock_vector` and `read_vector`. The complete CUDA source digest
is `8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.

## Inputs and admission

Use the admitted Spec, ReadSpec, ordered Binder forest and explicit carried
token occurrences. Preserve occurrence order and token identity separately.
Context bounds determine the global stage and node capacities; an individual
row may have a shorter valid forest. Require all payload blocks to be canonical
roots, all token identities in range, a complete gate inventory and a valid
virtual view. Full input admission is a stronger interface than the CUDA
function's visited-field checks and must be documented as such.

Flock and Relation require kept count 1..8 and at least two blocks. Relation
requires 1..5 sweeps. Route moves are invalid in either mode. Vector moves act
on every occurrence of the selected token/block. Gate moves use CURRENT and
the two remaining trits in `[0,-1,+1]` order, shared with the learning contract.
The read inventory contains `blocks * stages` gates and, in Relation, another
`5 * stages` gates. Gate ordinal, index, mode and shape errors remain typed.

## Exact snap and node construction

`snap_n(x)` selects the root minimizing the tuple
`(L1(x - n*r), Hamming(x, n*r), rootIndex)`. Scaling means exact repeated
addition. There is no normalized score, floating arithmetic or approximation.

For payload block zero, its frame is the final block. Every other payload block
uses block zero as its frame. Thus payload and frame are always distinct.
The final occurrence is the query; use its frame root and payload block.

Visit forest nodes in stored order, with children before parents. A leaf has
level zero and first/last positions equal to its atom. A parent's level is its
first child's level plus one, its first position comes from its first child,
and its last position comes from its third child. General forest admission
establishes compatible levels and ordered contiguous children.

Each leaf's frame and payload come from that occurrence. A parent retains its
first child's frame. Carry each child's payload using the actual BFS transport
from that child's frame to the first child's frame, sum the three carried
payloads, then apply `snap_3`. Using a same-endpoint action with a different
transport history is not source parity.

## Relation recurrence

Initialize `rel[i] = 0` for every node. Initialize the previous vector to
`prev[i] = 8 - L1(queryFrame - nodeFrame[i])`. For each sweep, every node adds
its signed, gated edge sums computed from the entire previous vector to its
retained `rel[i]`. Only after all nodes have been visited does `prev` become
the complete new `rel` vector. **Do not reset `rel` between sweeps.** The
recurrence is synchronous with cumulative scores, not an in-place traversal.

The five edge gates are indexed by the receiving node's level:

| Edge | Neighbors contributing to the sum |
| --- | --- |
| 0 | All three children of a parent, plus every later parent containing this node |
| 1 | Adjacent siblings: left gets middle, middle gets left and right, right gets middle |
| 2 | Every earlier node at the same level whose last position plus one equals this node's first position |
| 3 | For a leaf, only the last earlier leaf with the same token identity |
| 4 | Only the last earlier node at the same level with the same frame root |

Edges 3 and 4 overwrite the temporary edge contribution on each match; they
are latest-match links, not sums over all matches. Edge 2 sums all matches.
All contributing nodes must have last position less than the row length.
That includes the query leaf during relation propagation. The later candidate
selection excludes query-containing nodes; these are different predicates.
Each gate adds, subtracts or omits the edge sum according to its trit.

Use exact signed integers internally. Before asserting source arithmetic
parity, establish that the admitted finite graph and sweep count keep the
source's signed 32-bit additions in range. If that bound cannot be proved,
expose a checked arithmetic refinement and document its compatibility domain;
never silently wrap or claim undefined signed overflow as specified behavior.

A candidate static bound uses the validated forest: at most four parent/child,
two adjacent-sibling, one preceding-span, one latest-token and one latest-frame
contributions per node. With initial magnitude at most eight, the first sweep
has magnitude at most 72 and each later cumulative sweep multiplies that bound
by at most ten. Five sweeps therefore have magnitude at most 720,000. Formalize
the equal-level disjoint-span and single-parent prerequisites before adopting
this argument as an arithmetic guarantee; it is not execution evidence.

## Per-level selection and residual

For each level and payload block, admit only nodes at that level with
`last + 1 < rowLength`: every selected leaf must precede the query. Rank by
`(relationScore descending, frame L1 ascending, frame Hamming ascending,
lastPosition descending, nodeIndex ascending)`. Flock uses all-zero relation
scores. Retain at most the configured kept count.

Carry each retained node payload from its frame into the query frame using
the BFS action. Sum the carried payloads and apply `snap_found`, where `found`
is the actual retained count, not configured K. An empty selection contributes
nothing. A positive gate adds the consensus root to the running query payload;
a negative gate subtracts it. Either nonzero gate with a nonempty selection
increments the term count, initialized to one. Zero gates never increment it.
Finally output `snap_terms(residual)`.

Diagnostics preserve the same semantic computation. With tracing, even a zero
gate records selection and, when nonempty, consensus. Empty selection leaves
consensus absent, with found and eligible counts zero; never compute `snap_0`.
Without tracing, the source may skip
that work, but output remains identical. A trace contains query frame, output
root, term count, query payload root, and per-level gate-plus-one, found count,
consensus, eligible count, selected node IDs, actual transport actions and
carried root IDs. Fixed eight-slot arrays pad unused entries with UInt32 MAX.
Relation additionally records per-node final scores separately for every row
and payload block. Scores are reset and recomputed inside each block iteration;
unused global node-capacity slots are absent. Preserve signed values
semantically before any UInt32 wire projection.

## Required independent controls

Controls must exercise frame endpoint selection; parent transport and snap;
L1/Hamming/index snap ties; query exclusion; every selection tie field;
actual versus configured K; empty levels; positive, negative and zero gates;
traced zero-gate selection; vector moves in query, repeated-token leaves and
frame blocks; and typed invalid inputs. Relation controls must distinguish
all five edge kinds, last-match versus sum, synchronous versus in-place
updates, cumulative versus reset sweeps, receiving-level gate indexing,
query-inclusive propagation and query-exclusive output selection.

The result remains a pure source read component. Full batch binding, training
counterfactual execution, GPU parity and generated runtime cost are separate
integration and execution obligations.
