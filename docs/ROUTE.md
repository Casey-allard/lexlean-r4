# Bounded Route over explicit carried row roots

`src/R4/Route.lex.tex` owns R4-07. Its `run` definition accepts a bounded
`Types.Spec`, an independently validated `Binder.BinderPlan`, ordered row
entries, a complete per-block frame inventory, and a current or virtual view.
Each entry contains its token identity and current carried `Carrier.Block`
values. The result is a typed error or one `BlockReceipt` per payload block.
`outputBlocks` projects the complete carried vector for subsequent consumers.

This is the complete Route component over that explicit interface. Codebook
encoding, construction of rows from model/epoch batches, counterfactual proposal
selection, and runtime parity remain separate obligations. CPU verification
here checks formal source contracts; it is not a model execution, GPU result,
serving qualification, or performance result.

## Source authority and representation

The imported authority is UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-discrete/src/discrete.cu`, SHA-256
`8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e`.
The relevant functions are `route_vector`, `input_block`, `move_action`,
`root_cost`, `action`, and `compose`. Geometry constants and ordered BFS paths
come from the separately verified `R4.Geometry` component.

The row carries raw signed-integer blocks so malformed geometry has a typed
admission path. Every coordinate and block is checked with `Carrier.decodeRoot`.
Thus admitted blocks are exactly the sixteen roots, each with two nonzero
coordinates of magnitude two. Signed permutation actions preserve that orbit.
The L1 cost is at most eight and Hamming cost at most four; all Route cost
arithmetic on this admitted domain is safely within the source's UInt32 range.
The representation adds no float, dense product, or host model implementation.

A row identity must be less than the vocabulary bound. Repeated identities are
allowed, including identical geometries under different identities. Supplied
carried blocks of repeated identities need not be identical: the explicit
interface does not invent a codebook or assert epoch provenance. A later
integration must establish that external provenance before claiming it.

## Admission, budgets and ordering

`run` applies the following ordered checks:

1. `Types.validSpec` admits vocabulary 2..8192, blocks 2..16, context 1..243,
   and the existing kept-cell bounds.
2. `Binder.validate` admits the supplied general ordered forest. Its exact
   typed error is retained inside `RouteError.invalidPlan`.
3. `Binder.construct(spec.context)` supplies the global node allocation and
   stage budget. The row context must not exceed the specification context,
   and its node inventory must fit the global allocation.
4. Entry count equals row context. Every entry has exactly `spec.blocks`
   blocks, a bounded identity, and valid geometry in every block.
5. The frame table has exactly `spec.blocks` rows, each containing exactly the
   global number of stages. Every frame is an action ID in 0..63.
6. The virtual view's token or block/stage coordinates and candidate ordinal
   are in range.

This public artifact boundary checks the full supplied input and route table,
including unvisited rows, interior blocks, and frames after an early leaf.
CUDA's helper checks some fields only when visited. The full validation is an
explicit interface refinement, not a claim that the raw CUDA helper rejects
all the same malformed artifacts or uses the same diagnostic order.

The global budget is derived from the full specification context, never the
actual row length or the supplied forest's `stages`. A context-nine model has
three frames per block even for a one-token row; context 243 has six. A valid
noncanonical forest is retained as supplied, rather than replaced with the
canonical row constructor. Row plan validation and global allocation are
separate conditions.

## Traversal and transport

Each block starts with the final row occurrence as its query, root zero and
identity transport. Every stage transforms that same original (possibly
virtual) query with the stage frame. The query is not cumulatively transformed.

At the first stage, the root choices are the canonical root prefix with length
equal to the frontier count. The selected port indexes the frontier node list;
its geometric root identity is the **frontier ordinal**, not the node ID.
At later stages the three root choices are `Geometry.branchRoots(oldRoot)`;
the selected ordinal indexes the previous node's ordered children.

Selection minimizes L1 displacement, then coordinate Hamming distance, then
retains the earliest port on a complete tie. It does not rank by token ID,
node ID, or the absolute root ID of a later-stage choice.

After selection the accumulated action is
`BFS[oldRoot][selectedRoot] * total`, with the new BFS action on the left.
The old root is then replaced by the selected root. A leaf accepts any atom
less than row length, including the query occurrence itself. Its possibly
virtual carried block is transformed by the accumulated action. An internal
node continues with the remaining global frames. Exhausting them before
reaching a leaf returns `stageExhausted`.

`BlockReceipt` contains the resulting block, selected atom, final geometric
root, and accumulated action. The last three fields are transparent model
observations used by formal controls; they do not claim an upstream CUDA Route
trace API. Results remain in increasing block order, and an error propagates
without returning a partial vector.

## Virtual views

`View.current` retains every input and frame.
`View.routeMove block stage candidate` left-composes the selected generator
with that one frame. Candidate zero is identity; candidates 1..15 select the
ordered generators. A valid stage can lie after an early leaf; in that case
its transformation has no effect on the returned carrier.

`View.vectorMove token block candidate` applies the generator to the supplied
carried block of every matching token occurrence at the selected block.
`inputBlock` is used both for query acquisition and selected-leaf acquisition.
Consequently repeated identities, the query occurrence and a selected query
leaf all receive the same virtual transformation. Other identities and blocks
retain their current carried values.

The nested routes representation corresponds exactly to flattened source
parameter `block * globalStages + stage`. Separate checked coordinates avoid
an ambiguous or overflowing externally supplied flattened index.

## Typed failures

| Error | Meaning and control boundary |
| --- | --- |
| `invalidSpec` | Specification violates the existing bounds. |
| `invalidPlan reason` | General binder validation fails; the exact binder error is retained. |
| `planCapacity` | Row context or node count exceeds the canonical model allocation. |
| `rowShape` | Entry count or per-entry block count is wrong. |
| `invalidToken position` | Row identity is outside the vocabulary. |
| `invalidGeometry position block` | A supplied block is outside the sixteen-root orbit. |
| `routesShape` | Frame-table block or global-stage count is wrong. |
| `invalidFrame block stage` | A frame is outside action IDs 0..63. |
| `invalidMove` | A virtual index or candidate is out of range. |
| `stageExhausted` | The checked traversal has no remaining frame before a leaf. |
| `invalidNode` | A low-level traversal link is absent or an internal node has no children. |
| `invalidAtom` | A low-level traversal leaf has no corresponding entry or payload block. |
| `invalidPort` | A low-level traversal has no candidate or matching link. |
| `transportFailure` | A low-level transport endpoint is outside the orbit or cannot resolve an action. |

The last five traversal errors are tested directly through `walk` or
`transport` on malformed helper inputs. Public admission rules out these
malformations for a valid bounded forest, but the helpers still fail explicitly.
`walk` is a low-level traversal definition and does not replace `run`'s full
artifact admission. It is not an alternate public admission path.

## Formal evidence and limits

The independent control root contains 58 closed theorems, each without axioms.
It covers frontier ordinals versus node IDs, all three selection keys, original
query reuse, branch ports and transport, left composition, old-root BFS across
three stages, early query-self leaves, duplicate identity virtual updates,
noncanonical forests, all error variants, maximum context and block dimensions,
and the maximum admitted token identity.

Literal outcomes were derived from the pinned source and fixed finite geometry
metadata. The three-stage witness names its two expected actions explicitly as
`state42 * state1`; it does not compute an expected Route result through the
implementation. No handwritten host operator supplies an oracle.

The initial RED command was the explicit development verifier on
`tests/lexlean/RouteControls.lex.tex`, before `R4.Route` existed. It returned
exit 1 and `LLR3005`: the imported module was not a project module.

Stable source SHA-256:
`5baa4dd5cd5ba7cb8cd265565add6c35341e3b41fd47745471790eab890ec44e`.
Control SHA-256:
`cb6125350b75d06db18b28d881fb23f34387ae636c0026bb69d0aecf341cc0ef`.

The digest-pinned development SDK formally verified this control root with
no diagnostics and attestation
`1355cd39cd12026f4748c54917c70cccbca5a42a25a298db60fec786e744dacc`.
The selected source graph is Binder, Carrier, Geometry, Objective, Route, Types,
and RouteControls. The Rust conformance test checks the exact control digest,
full selected-module inventory, fresh source/build identities and Lean receipts.

Production acceptance remains blocked by the absent production `prismpm.lock`.
Development verification is not `just vv` acceptance and establishes no GPU
parity or deployed behavior.

A planted defect reversed `frameAction`'s virtual composition from
`generator * frame` to `frame * generator`. The same explicit verifier exited
1 with `LLV7002`: Lean rejected the noncommuting control
`routeMoveCompositionIsLeft`, whose expected action is `state6`. The unmodified
source was then restored byte-for-byte before the final conformance test.

After restoration, the offline locked Rust conformance command
`cargo test --offline --locked -p repo-conformance --test route` passed its one
unfiltered test, with zero failures and zero ignored tests. That test invoked
the same formal verifier and checked the exact receipt graph. `git diff --check`
also passed. These commands ran in the already pinned, network-disconnected
development SDK container; no runtime model evaluation was performed.
