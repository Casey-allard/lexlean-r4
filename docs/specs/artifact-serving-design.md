# Artifact admission and frozen integer serving

Status: proposed implementation design derived by source inspection on
2026-10-21. This document defines no accepted export, executable model,
training release, serving qualification, or compatibility with UORLUT01.
No model, CPU/GPU reference, or research execution was performed. It supplements
the approved full-scope design and
[production export contract](production-export-contract.md); their outstanding
compiler, runtime and deployment requirements remain in scope.

All existing behavior below is pinned to UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`. New choices are explicitly marked
**proposed**. The artifact is a frozen view of the discrete model, not a way
to activate historical dense stack implementations.

## Source identity inventory

Hashes below identify source-file bytes from the pinned Git object, not
compiled runtime identities.

| Pinned path | SHA256 |
| --- | --- |
| `crates/uor-r4-discrete/src/snapshot.rs` | `c03dcb7bc0e0e966951285a52cf76bf672ee74bf136c42403b40d1a4b5da1170` |
| `crates/uor-r4-discrete/src/lib.rs` | `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9` |
| `crates/uor-r4-discrete/src/device.rs` | `47acbd7398ac95e87d9e6b58594012e14d91016941adb90fa72563e41afdd6ae` |
| `crates/uor-r4-discrete/src/discrete.cu` | `8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e` |
| `crates/uor-r4-discrete/src/objective.rs` | `b813e855be6c4ea460c8711493527d52f1caf1a02bb09b046145f88c7f4ee4d0` |
| `crates/uor-r4-discrete/src/metadata.rs` | `27e53e4feecb391adb4a4db3462712a8d9375aed09c0e57086581134cb6e1e32` |
| `crates/uor-r4-discrete/src/prime_zeta.rs` | `2f1ab07fe6094be6c3887c3b700868f89f8a63fcf311652c16ed4227be8b4e54` |
| `crates/uor-r4-discrete/build.rs` | `5353d7dda155244147f77dbcf7ae06b3ac67a98007b6f40a7815894870b13427` |
| `crates/uor-r4-integer/src/native_geometry.rs` | `c78772a0495269f43ff562ff5d56bf6b73b97562d9e69254f377c520aecd9b7a` |
| `crates/uor-r4-integer/src/wasm_serve.rs` | `7e535ef0045107995cc7f13d5b64ab7b3c27d58ed8e4fa7bd5223bbdae531787` |
| `crates/uor-r4-integer/src/stack/format.rs` | `e44ac46e050cdffa17f501afeda54c94aee002dab04ea8f24fc0a273841a4018` |
| `crates/uor-r4-tokenizer/src/lib.rs` | `52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35` |
| `docs/mechanisms/lut-serving.md` | `a200e565684cad05b031594fe4f35d12be99a4d65839bc47ffc06736a74e540f` |
| `CHARTER.md` | `6afcc623507d0bb42603978d045c37b17a20b87c1f927ddae5813a89c79271fe` |

## Existing snapshot serialization

The exact schema string is `uor-r4.discrete-spiralcore-fbs/2`.
Snapshot and Spec reject unknown fields. ReadSpec has a custom strict
deserializer: even Route rejects extra fields, using an empty struct variant
to avoid the permissive internally tagged unit-variant behavior.

Typed snapshot serialization uses this declared field order:

1. schema
2. spec: vocabulary, blocks, context, kept_cells
3. read_spec
4. objective_policy, when nondefault
5. read_initialization, when nondefault
6. seed
7. epoch
8. table
9. routes
10. gates
11. operator_kappa
12. composition_kappa
13. kernel_sha256
14. source_sha256
15. geometry_sha256

ReadSpec is `{"kind":"route"}`,
`{"kind":"flock","kept":K}`, or
`{"kind":"relation","kept":K,"sweeps":S}`, with that declared serialization
order. ObjectivePolicy defaults to `"nearest_address_margin_v1"`; its other
value is `"occupied_cell_rank_hinge_v1"`. ReadInitialization defaults to
`"zero_gates_v1"`; its other value is `"active_leaf_v1"`. Missing defaulted
fields are admitted, and typed serialization omits them when default.
No schema-1 or missing-ReadSpec/gates migration is present.

Seed, Spec fields and routes are UInt32; epoch is UInt64; table and gates
are Int32 arrays. Strings and numeric syntax must reproduce the pinned
serde_json serialization when claiming byte compatibility. Reordered input
keys or whitespace are not part of typed serialization identity.

There are three distinct storage domains:

| Domain | Exact source operation |
| --- | --- |
| Raw file identity | Hash of actual file bytes, maintained separately by a FilePin |
| `model_storage_sha256` | SHA256 of `serde_json::to_vec(original stored Value)` |
| `model_snapshot_sha256` | Deserialize that Value into Snapshot, then SHA256 of `serde_json::to_vec(typed Snapshot)` |

The storage hash deliberately uses the original stored Value, not the typed
round-trip. Explicit default fields therefore change storage identity while
leaving typed identity unchanged. The stored-Value domain is not raw input
JSON: whitespace and parser-discarded distinctions cannot be recovered from
a Value. Object-map ordering must be pinned with the serde_json package and
effective features; the inspected Cargo manifests do not request
`preserve_order`. A pure LexLean compatibility codec needs independent
nested-object, string-escaping and exact-integer controls, rather than an
assumption that arbitrary JSON encoders emit these bytes.

**Proposed input boundary:** accept a versioned, bounded byte packet with
duplicate object keys rejected before Value construction, exact integer
domains, bounded nesting/string/array lengths, and complete input consumption.
This stricter raw-byte admission is a new interface policy. It must not be
misreported as recovering raw bytes from the existing stored-Value hash.

## Existing snapshot admission and runtime identities

The source `Device::from_snapshot` sequence is:

1. Validate ReadSpec and Spec; construct canonical BinderPlan for context.
2. Require exact schema, operator/composition kappa, ordinary PTX hash,
   compiled source binding and packed geometry hash.
3. Require table length `vocabulary * blocks * 8`, route length
   `blocks * stages`, and mode-specific gate count.
4. Construct a new model through `new_with_initialization`, which also
   checks initialization/read compatibility and exact CUDA target admission.
5. Load table/routes/gates/epoch, then run `discrete_validate` before exposing
   encode or forward capability.

Source bounds are vocabulary 2..8192, blocks 2..16, context 1..243,
kept cells 1..min(16,vocabulary), Flock kept 1..8, and Relation kept 1..8
with sweeps 1..5. Active-leaf initialization is invalid for Route.
Canonical stages are one plus maximum binder-node level, at most six.

Route and Flock gate inventory is `blocks * stages`; Relation additionally
has `5 * stages` edge gates. Route still serializes and validates its complete
inactive gate inventory. The maximum admitted route count is 96 and gate
count is 126. At context 243, the canonical ternary tree has 364 nodes.

CUDA validation checks every codebook block against one of the sixteen
ordered roots, every route index below 64, and every gate in {-1,0,+1}.
It also checks mode/configuration metadata and objective policy word.
Failures use source statuses 6 (root membership), 7 (route state), and
10 (gate/configuration). Multiple invalid entries race through atomic status
publication; a deterministic semantic validator may return an ordered typed
error, but must not claim that order reproduces CUDA's concurrent first error.

A fresh process model instance is allocated even when snapshot bytes are
identical. `NEXT_MODEL` uses checked increment and refuses exhaustion.
Snapshot epoch is restored; instance identity is not. Existing carried batches
and proposals bind the full instance/config/source/geometry tuple and epoch,
and do not become valid after reload. A public natural-number handle is not
custody or authentication.

The three source runtime hashes must remain distinct:

- `kernel_sha256` hashes ordinary PTX bytes.
- Geometry hashing concatenates operator-kappa bytes, composition-kappa bytes,
  then each packed UInt32 geometry word in little-endian order. Packed geometry
  contains exactly 5120 words: ROOTS, AXES, COMPOSE, BFS, BRANCH, GENERATORS,
  INVERSE, then IDENTITY. Signed root coordinates use their Int32 bit pattern.
- `source_sha256` is the exact ordered build binding in device.rs, not Git SHA.
  Its initial unframed sequence includes SCHEMA, device.rs, snapshot.rs,
  discrete.cu, metadata.rs, lib.rs, objective.rs, proposal_timing.rs, build.rs,
  decomposition_source.rs, decomposition.rs, decomposition_device.rs,
  baseline_reuse_source.rs, baseline_reuse.rs, baseline_reuse_device.rs,
  diagnostic PTX, and ordinary PTX. It then appends three versioned domains:
  `uor-r4.cuda-images/1` binds format, target, ordinary/decomposition cubins
  and both resource reports;
  `uor-r4.baseline-reuse-image/1` binds reuse CUDA source, PTX, cubin and
  resource report; `uor-r4.rows-image/1` binds rows source wrappers, CUDA
  source, PTX, cubin and resource report. Every named entry in these domains
  is framed by UInt64-LE name length, name bytes, UInt64-LE data length,
  and data bytes, in the source order.

Those built bytes are required to reproduce the runtime identity. Reading a
Git revision, source-file hashes, or a snapshot's claimed hash cannot invent
them. Loading a CUDA snapshot under a pure LexLean runtime is consequently a
new explicitly bound import/refinement operation, not the original
`Device::from_snapshot` acceptance.

## Proposed table-native freeze format

Use a distinct versioned format, provisionally named
`lexlean-r4.table-freeze/1`, with eight-byte magic `LEXR4F01`.
This name and magic are proposed application format choices, not an existing
UOR format or an assertion of UORLUT01 compatibility. Final schema registration
and byte fixtures must precede implementation.

Admission expectations come from a reviewed runtime/deployment-owned policy,
independently of the incoming packet: exact schema and kappa identifiers,
canonical ordered geometry, approved source/SDK/executable bindings, and
supported capacity profiles. The packet's claimed identities and embedded
evidence are inputs to validation, never the authority that chooses expected
values. Recomputed content hashes establish consistency, not provenance or
permission. An unknown binding remains unaccepted even if its supplied bytes
and hashes agree. Structural geometry laws supplement exact canonical table
identity; an isomorphic reordered table cannot replace the admitted ordering.
Packet length and resource budgets are enforced against runtime policy before
trusting or allocating from packet claims.

Proposed encoding is a bounded canonical binary packet: magic, UInt32-LE
version (1), UInt32-LE flags (0), UInt64-LE payload length, then a fixed ordered
section sequence. Each section has a UInt32-LE tag and UInt64-LE byte length.
No optional unrecognized sections, duplicate tags, trailing data or padding
are admitted. Length arithmetic must be checked before slicing/allocation.

| Ordered section | Proposed contents and obligations |
| --- | --- |
| 1: provenance | Versioned original snapshot domain IDs; actual available CUDA binding evidence or explicit native-LexLean origin; source revision; accepted model-source/SDK/generated-executable identities; no fabricated CUDA identity |
| 2: configuration | All Spec, ReadSpec, objective and initialization fields explicitly encoded, seed and frozen epoch; no implicit defaults in the new format |
| 3: geometry | Exact ordered root/action/composition/transport tables and kappa identifiers; verify identity and structural laws, not lengths alone |
| 4: parameters | One root-index byte per token/block in token-major order (0..15), one action-index byte per route (0..63), one trit byte per gate using a declared 0/1/2 codec; retain inactive fields |
| 5: tokenizer | Exact tokenizer asset bytes or an immutable included content-addressed asset, its complete semantics/version/identity and exact token-ID mapping |
| 6: decoder contract | Explicit EOS option, context-window policy, token-output policy, capacity-profile identity and deterministic stopping rules |
| 7: derived layout | Occupied-cell/member indices, fixed offsets, binder layout and preallocation descriptor; validate or reconstruct from admitted authoritative parameters |
| 8: evidence | Source-to-freeze refinement identity, generated-root/ABI identity, instruction/allocation acceptance references, and provenance classification |

The frozen artifact identity is a domain-separated SHA256 of the complete
framed packet, with the digest supplied in an external immutable manifest;
there is no self-referential digest field. Any embedded evidence has its own
declared hash domain. Trust roots and signatures, where needed, are separate
from content hashes. The exact framing and domain-separator bytes require
literal controls before this proposal becomes a codec.

An executable/ABI identity embedded in the packet denotes independently built
parameter-loading code whose bytes do not embed that packet. If deployment
later bundles code and artifact, the complete bundle identity belongs in an
external manifest. Do not create a cycle in which the packet includes the
digest of an executable that itself includes the packet.

Root-index encoding is lossless only after exact membership and uniqueness
in the ordered sixteen-root table are established. Expansion must reproduce
every original Int32 table coordinate; routes, gates, seed and epoch must
round-trip exactly. New-format trit bytes must not be confused with Gates'
signed Int32 wire words or its `gate+1` trace encoding.

The authority/provenance tag distinguishes a source-bound CUDA import from
a newly constructed pure-LexLean model. A native model has its own source
and executable binding. It cannot inherit a CUDA scientific qualification,
or become source-compatible by copying a hash string.

## Proposed frozen decoder

A serving session holds the immutable artifact identity, a fresh runtime-owned
instance/handle generation, exact token/occurrence identities, bounded context,
preallocated read/writer scratch, output counters and a closed stop status.
ReadSpec, gates, roots and routes are artifact configuration, never request
overrides. Serving performs no parameter or epoch update.

The proposed deterministic step is:

1. Validate handle custody, artifact identity, session state and all capacity
   reservations. Reject empty prompts unless an explicitly declared start-token
   policy exists; do not insert an invented BOS.
2. Admit exact tokenizer IDs and select the trailing valid context of at most
   Spec.context tokens under the declared sliding-window policy. Preserve exact
   occurrence/record/version identities and canonical causal binder semantics.
   For a nonempty logical window of length L, query row position L-1 while
   retaining that occurrence's original identity metadata. Construct
   the canonical prefix plan for L while retaining the global stage inventory
   derived from Spec.context. Any physical padding required by the carried
   representation is excluded from query selection, eligible candidates and
   logical occurrence counts; the final padded slot is never the query.
3. Execute the actual Route/Flock/Relation read with frozen parameters and the
   shared geometric writer. Rank occupied cells by address distance, then cell
   ID, and admit the first min(kept_cells, occupied-cell count) cells. Across
   every token identity in those cells, select minimum full token distance,
   then minimum token ID. Do not use the legacy
   stack's probability masses, softmax tables or argmax semantics.
4. Commit one token and its exact occurrence to the session only if output
   token, output-byte, context and counter capacities suffice. On capacity or
   computation failure, preserve the complete prior session.
5. Stop on the declared EOS, explicit cancellation at a token boundary, or
   the caller's admitted maximum new-token count. Zero requested tokens returns
   unchanged with a budget stop. EOS is retained in the exact token transcript
   and stops before another prediction; the text renderer's handling is part
   of the explicit decoder contract. If EOS and token budget coincide, EOS
   takes precedence. A terminal session cannot silently resume.

These are new bounded generation semantics, not an already implemented
discrete greedy loop. The tokenizer source inserts no BOS/EOS, and its assets
do not by themselves choose a generation stop ID. EOS must be absent or an
explicit admitted exact token ID; no hard-coded token number or automatically
executed chat template is permitted.

The CUDA writer accepts a supervised target and computes loss after prediction.
A serving target-free projection therefore needs a proof: for every admitted
query/codebook and every valid target, cell selection and predicted token are
identical, and removing objective telemetry cannot suppress an applicable
source error. Fixed root/domain bounds must establish loss arithmetic safety
where that argument relies on it. Supplying an arbitrary target without this
refinement is not an established inference contract.

Tokenizer bytes can span UTF-8 boundaries. Preserve exact generated token and
byte streams; incremental text rendering needs a modeled pending-byte state or
an explicit existing decoder policy. Re-decoding each isolated token with
replacement characters is not equivalent to decoding the full byte sequence.

## Capacities, D11 and custody

Use a versioned capacity profile, admitted before loading/session creation.
It must specify artifact/asset/request/response bytes, sessions, context,
generated-token budget, decoded-byte budget, memory records/ranges, counters,
scratch and recursion/stack bounds. Source dimensions above are mandatory;
additional product capacities are explicit configuration choices, not new
scientific bars. Reject an unsupported profile rather than truncating data.

The admitted maximum table has 131072 root indices (or 1048576 expanded
coordinates). These source-derived finite counts do not establish runtime
allocation behavior. Allocate fixed layouts at load/session construction and
prove that the steady-state step reuses them. Precompute offsets or use
incremental addresses: a runtime `token * blocks` is not justified merely
because the result is bounded. Generated instruction audits must exclude
multiply, divide, float and transcendental operations from the required
artifact/runtime closure, including helper and allocator paths.

A frozen occupied-cell index may replace the source's bounded full-vocabulary
scans only with a refinement proving exact cell occupancy, complete duplicate-
preserving token membership, ordering, and the same selection/argmin result.
Never cap crowded-cell members and lose identities. A degenerate cell may
still contain the entire vocabulary; an indexed representation alone proves
neither D5 sparsity nor cache residency or tokens/s.

The runtime, not an externally supplied integer, owns private model/session
handles. Load is all-or-nothing; corrupt assets never replace a live model.
Unload/reload invalidates old handles. Repeated identical-byte loading creates
a distinct instance. Snapshot/restore binds the artifact, tokenizer, decoder
contract, capacity profile and exact state, but establishes a new runtime
handle generation. Recovery and durable-memory import require the accepted
PrismPM runtime's atomic admission/custody path. Artifact hashes do not release
training or authenticate mutable proposals.

## Offline prime-zeta boundary

Pinned prime_zeta.rs explicitly permits one-time offline host codebook
preparation, not model forward/routing/scoring/learning. It uses prime logs,
Odlyzko zero ordinates, trigonometric functions and normalization before
PhaseQ29 and UnitS3Q30. NormalNull additionally uses `powf`; mode/seed/null
parameters and exact zeta/prime input identities must be retained.

After quantization, the source uses exact integer embedding/alignment,
deterministic root ordering, writer-cell quotas and full-vector non-aliasing
assignment. QuantizedCodebook binds phases, spinors and table using SHA256
over consecutive Int32 little-endian words; it also carries selected root
indices, mode, Spec, seed and diagnostic counts. The mode label alone is not
a sufficient identity for every floating parameter.

The frozen runtime consumes only the admitted integer roots/tables. It does
not embed prime-log, sin/cos/sqrt/powf code or regenerate float preparation at
load. The pure-LexLean offline preparation model, source/input/rounding
correspondence, lawful execution surface, and independently admitted export
gap remain required work. Accepting a quantized input artifact is not a claim
that these upstream obligations are already implemented.

## Controls and remaining acceptance obligations

Required corruption and boundary controls include:

- Unknown/duplicate/missing fields, explicit-default versus omitted-default
  hash domains, reordered keys, escaped strings, integer endpoints, negative
  unsigned values, truncation, excess bytes, and domain substitution.
- Every schema/kappa/geometry/kernel/source mismatch, including a changed
  resource report, target, reuse image or rows image; unavailable compiled
  evidence must not be replaced by Git identity.
- Each Spec/ReadSpec/init boundary, route64, gate outside {-1,0,+1}, wrong array
  lengths, non-root coordinates, complete inactive Route gates, relation edge
  inventory, and reload-instance/counter exhaustion.
- Root-index and signed-coordinate round-trip, trit-codec confusion, corrupted
  tokenizer map, ID outside vocabulary, swapped cells, missing/repeated token
  membership, crowded cells, equal-distance tie order and derived-layout
  substitution.
- Empty prompt, a short nonempty prompt under a larger configured context
  (logical-last query versus physical padding), context 1/243 and window
  rollover, zero/one token budget,
  absent/invalid/EOS-at-last-slot stop, output-byte exhaustion before commit,
  cross-token UTF-8, cancellation, stale/forged handle, terminal resume,
  checkpoint mismatch and exact restore continuation.
- Positive full accepted artifact round-trip and genuine end-to-end step,
  with planted wrong-table, wrong-tie, wrong-EOS and dropped-identity defects.
  These finite controls do not replace general refinement or runtime audits.

Acceptance requires: completed hash/protocol verification; modeled strict
snapshot JSON and new binary codecs; full source-safe bounded arithmetic;
the actual Engine and target-free writer refinement; canonical frozen layout
and selected-address reads; modeled tokenizer/byte decoding; an accepted
production SDK with collision-free complete names and required fixed-memory
lowering; generated instruction/allocation evidence; admitted runtime custody,
persistence and recovery; and the complete generated Pages application.
The production-export contract supplies the exact currently known compiler
and browser-runtime blockers.

Pinned lut-serving.md explicitly reports no current lawful discrete export,
no addressed-memory integer kernel, and an open table-native UORLUT schema.
The default integer crate points at native_geometry.rs's fixed geometry/scalar
surface; serving-wasm includes the preserved serving root and its legacy stack
loader. Neither is the missing discrete export. Historic formats or their
old positive audits must not be activated or cited as acceptance of this new
artifact. Research parity, quality/export-gap claims and training keep their
separate GPU/fitness/authorization gates; ordinary client product serving is
the charter's distinct allowed execution boundary.
