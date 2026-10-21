# Proposed design: UOR-R4 in LexLean and PrismPM

Status: approved by Casey in this chat on 2026-10-21: “Approve the full-scope design.”
This document is a design, not the product's
semantic authority and not a claim that any module exists or verifies.
Source identities and dependency findings are in [UPSTREAM.md](../UPSTREAM.md).

## Requested outcome

Create `casey-allard/lexlean-r4` from the UOR template; implement UOR-R4 in pure
LexLean; model the complete product with PrismPM; generate and verify the
implementation and browser application; deploy its accepted output to GitHub
Pages; deliver atomic conventional commits.

Pure means each application rule is defined once in `.lex.tex`. No handwritten
Lean proofs or definitions, Rust model implementation, JavaScript model or UI
behavior, copied CUDA engine, or FFI bridge to the original model substitutes
for those definitions. Generic compiler/runtime facilities remain dependencies
and must be identity-bound. Generated code is a projection, not an authority.

## Alternatives and recommendation

1. **Full semantic port with separately verified execution surfaces (recommended).**
   Model the active discrete engine, learning law, artifact, tokenizer, exact
   memory, application, and deployment in one imported source graph. Complete
   each executable surface against the toolchain that can actually support it.
   GPU research and durable browser memory remain required scope, with explicit
   upstream dependencies; neither is silently removed to ship a demonstration.
2. **Serving-only port.** Import a qualified frozen artifact and implement its
   integer inference and portable Pages interface. This is smaller but excludes
   executable training and may exclude durable browser memory. It does not meet
   the full request without an explicit scope change.
3. **Toolchain expansion first.** Extend the generic LexLean/PrismPM production
   toolchain for GPU execution and accepted persistent browser orchestration,
   then port R4. This addresses the complete execution problem but is a larger
   cross-repository program; framework changes need separate upstream review.

The recommended approach retains the full requested scope. No phase alone earns
the name “complete UOR-R4 implementation.”

## One source graph

The planned prefix is `LexLeanR4`. Choose the source language version only after
the exact SDK demonstrates compatibility; current LexLean 1.2 authoring syntax
must not be assumed available in a candidate built from an older source.

| Planned source | Responsibility | Principal source obligation |
| --- | --- | --- |
| `src/R4/Types.lex.tex` | Bounds, identities, tagged errors, specification | `discrete/src/lib.rs` and artifact schemas |
| `src/R4/Geometry.lex.tex` | Root/action tables, signed-axis action, composition, inverse, deterministic transport | `discrete/build.rs`, `core::spiralcore_operator` |
| `src/R4/Binder.lex.tex` | Causal ternary binder, node spans, frontier, structural validity | `discrete/src/metadata.rs`, binder construction |
| `src/R4/Route.lex.tex` | Framed tree descent and carrier transport | `discrete.cu::route_vector` |
| `src/R4/Flock.lex.tex` | Causal neighbor selection, transport, consensus, trit gating | `discrete.cu::flock_vector` |
| `src/R4/Relation.lex.tex` | Five edge types, bounded sweeps, exact-token relations | Relation branch of `flock_vector` |
| `src/R4/Admission.lex.tex` | Occupied cells, bounded admission, crowded-cell identity retention | `token_cell`, `cell_precedes`, `writer` |
| `src/R4/Objective.lex.tex` | Token/address/rank hinge, exact weighted loss, overflow | `writer`, `objective.rs` |
| `src/R4/Learning.lex.tex` | Candidate enumeration, selection, state transition, admission and epoch binding | `discrete_choose`, `discrete_apply`, `training.rs` |
| `src/R4/Codebook.lex.tex` | Prime-zeta input identity, discrete construction, A8 cell placement | `prime_zeta.rs`, `address_cells.rs` |
| `src/R4/Tokenizer.lex.tex` | Exact byte-BPE semantics, token IDs, special tokens, input limits | `uor-r4-tokenizer/src/lib.rs` |
| `src/R4/Artifact.lex.tex` | Bound tables/config/tokenizer/geometry, canonical encoding and strict loading | discrete snapshot and integer artifact formats |
| `src/R4/Serving.lex.tex` | Frozen state, shared vector writer, bounded decode and stop behavior | active discrete semantics and integer serving contracts |
| `src/R4/Memory.lex.tex` | Content IDs, ternary DAG, manifests, ranges, atomic import | `uor-r4-object-store/PROTOCOL.md` and reference source |
| `src/R4/Conversation.lex.tex` | Session state, memory commits, retention, errors and recovery | explicit new integration over modeled primitives |
| `src/R4/Evidence.lex.tex` | Source/mechanism status, proof/run/acceptance distinctions, artifact identity | charter and mechanism board |
| `src/R4/Application.lex.tex` | Named roots, byte protocol, acceptance vectors, View and finite limits | PrismPM application profile |
| `src/R4/System.lex.tex` | Resources, persistence grants, lifecycle, release and Pages target | PrismPM system/browser contracts |

Artifact details and tokenizer assets are adopted only after their complete
format review. They are not replaced with a guessed format or a byte-only toy
tokenizer. Parameters, dictionaries and geometry assets need exact identities.

## Mechanics that must survive translation

### Geometry and identities

The current discrete routing atlas is a **sixteen-root E8 orbit with 64 finite
actions**. It is not the entire E8 root system and is not evidence of full
H4/icosian learnability. The action is a signed coordinate gather over eight
coordinates; composition and path transport use finite tables. Preserve the
ordered generator set and deterministic graph traversal.

Root selection minimizes `(L1 distance, coordinate Hamming, root index)`.
Consensus selects the root closest to the unnormalized sum against a repeated
sum of that root. No softmax, normalized exponential, sigmoid, dense matrix
operation, or renamed equivalent enters the semantic or runtime graph.

Token, occurrence, record, and version are exact identities. They remain
distinct from geometric address. Input, carried values, and output use the same
vector representation. The first and final blocks determine the 16 by 16
product cell; distinct tokens may occupy the same cell.

### Route, Flock and Relation

Validate source bounds: vocabulary 2..8192, whole 8D blocks 2..16, context
1..243, admitted cells 1..min(16, vocabulary). Flock retains 1..8 neighbors;
Relation adds 1..5 sweeps. These are source configuration bounds, not a new
scientific bar.

Route follows the causal binder with frame transforms and composes actual BFS
transport actions. Flock's frame block is the opposite endpoint for block zero
and block zero for other blocks. Each candidate is carried into the query frame
before consensus. Preserve the causal leaf-span rule and deterministic ordering
by relation score descending, distance ascending, Hamming ascending, recency
descending, then node index ascending.

Relation retains all five source edge definitions and their accumulation and
overwrite behavior across sweeps. An approximate graph walk is not equivalent.
Trits are exactly `{-1, 0, +1}`. Invalid read-mode/parameter combinations fail
with typed errors. A zero gate must preserve the defined read behavior without
inventing learned attention capability.

### Admission, prediction and objective

Rank occupied cells by `(address distance, cell ID)` and open only the requested
number. Score every token identity in those cells; do not cap tokens by the
cell count or collapse same-address identities. Resolve prediction ties by
token ID. The source currently scans the bounded vocabulary; this is not a
claim of per-token sparse access.

Keep `NearestAddressMarginV1` and `OccupiedCellRankHingeV1` distinct. Token hinge
uses a width-dependent margin, address hinge uses margin eight, and rank hinge
is `max(target occupied-cell rank - kept cells, 0)`. Absent rivals have explicit
eligibility/count semantics. Weighted sums require checked integer arithmetic.
Do not invent probabilities or report cross-entropy/BPB from raw ranks.

### Learning and codebook construction

Enumerate the legal finite route/vector moves and gate siblings with the
source's exact candidate order and tie rules. Apply only strict improvement
against the bound current state. Reject stale epochs, foreign geometry/model
identity, invalid indices, mode mismatches, and epoch overflow. Prospective
virtual changes must affect both the codebook and derived cells consistently.

Model training admission and experiment identity without manufacturing owner
release. Formal proofs and finite metadata checks authorize no training run.
Prime-zeta preprocessing stays separate from downstream discrete operations;
do not place complex/float preprocessing in the frozen serving closure.

### Exact memory

Implement the actual UOROBJ01 contract: 32-byte atoms; ternary parents;
12-byte BLAKE2b node IDs over the scale and logical payload; object SHA-256;
canonical frontier; 45-byte leaf and 49-byte parent records; exact length and
manifest identity. A generic key/value map is insufficient.

Bound requests and traversal. Reject corrupted, foreign, missing, duplicated,
truncated and trailing records. Verify the complete staged object before
atomic import. A failed operation leaves prior state intact. Model browser
storage, reload, uncertain-operation recovery and version conflicts explicitly.
Network replication and authentication are separate from integrity; content
hashes alone grant no access authority.

## Mechanism coverage

The upstream board's 27 mechanisms are all represented in the port inventory.
Their imported status does not become a local implementation claim.

| Imported category | Mechanisms | Port treatment |
| --- | --- | --- |
| ACTIVE | Discrete learning; prime-zeta codebook; A8 address cells; Route; Flock; Relation; admission; rank-margin hinge; D11 serving; tokenizer | Exact semantic port and executable acceptance appropriate to each target |
| READY | Hamilton gate; recurrence gates; SpiralCore/FBS projection; D19 object store | Separately modeled modules and contracts; integrate only with explicit evidence; D19 is required for durable memory |
| PARKED | Carrier credit; legal construction; E8 orbit memory; exact memory; pointer key; read binding; Hopf observation; VSA; spin weights; ternary/XOR; geometric stack; native geometric engine; R4G1 graph | Preserve status, lineage, evidence and reopening conditions; do not compile historical banned implementations into the product |

The project does not claim that every historical experiment is a lawful active
model. Full modeling includes precise status and dependencies, not automatic
activation of every historical mechanism.

## Production and Pages

The deterministic core consumes a frozen, validated artifact and bounded session
state. It emits the next state, tokens, provenance and a closed error type.
Generation limits, empty input, EOS, capacity exhaustion, invalid artifacts,
memory conflicts, and reload behavior are model-owned.

The PrismPM model supplies the View and all requested effects. Generated host
code adapts those effects; it does not implement model semantics or fabricate
answers. The user interface exposes loaded artifact identity, input, generation,
stop, and durable-memory operations with their actual outcome. Sample traces
must be labeled as finite fixtures, never presented as trained conversation.

CI pins the exact SDK and Action, checks the authoritative sources, runs Lean
verification and axiom auditing, builds the named production roots, validates
generated native/Wasm behavior and planted defects, then packages the exact
accepted browser closure. The Pages job deploys those bytes without rebuilding
application behavior. Verify the live URL and exact served bytes at the selected
source commit. An HTML status page alone does not satisfy deployment completion.

D11 acceptance requires emitted native/Wasm instruction checks and measured
steady-state allocation at the decode boundary. Fixed-width integer source is
necessary but insufficient. Formal verification does not prove tokens/s or
language quality.

## Acceptance and blocking dependencies

| Deliverable | Required evidence | Current state |
| --- | --- | --- |
| Repository identity | GitHub template metadata and source repository | Established |
| Source model | LexLean check/build/verify, exact axiom audit, coverage, no second implementation | Not implemented |
| Mechanism equivalence | Exact pinned semantics; adversarial finite controls; authorized GPU parity for model execution | Not run |
| Learning execution | Modeled GPU backend, source-bound pre-registration and authorization | Backend dependency unresolved; not run |
| D11 serving | Generated named roots, artifact validation, instruction and allocation evidence | Not implemented |
| Durable browser memory | Public SDK effect support, actual reload/recovery/corruption/concurrency acceptance | SDK acceptance unresolved; not run |
| Pages | Complete `just vv`, generated closure, deployment receipt and live-byte verification | Not deployed |

Resolve the exact SDK/template binding and its supported source language first.
A development candidate may support investigation but cannot be silently shipped
as an accepted dependency under template R6. Resolve GPU generation and public
persistent-browser acceptance without handwritten application fallbacks or
changing the source charter. These are dependencies of the requested result,
not reduced completion criteria.
