# Mechanism and evidence model

Status: approved by independent architecture review, 2026-10-21. This is the
Evidence component of the approved full-scope design. It introduces no local
mechanism promotion, research result, acceptance receipt or permission to run.
Registration, independently frozen controls and RED precede definitions.

## Source authority

The imported catalog is the complete mechanism board at UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`:

| Source | SHA-256 |
| --- | --- |
| `docs/mechanisms/README.md` | `8aefb9331b3f740d5d012f139b4a1b3755d870957c47beb74338e6db882de367` |
| `docs/labs/repo-renormalization-2026-10-19/audit/mechanisms/board.json` | `357cdfc926a220239193b19c6d2fd86beea4e6c0d843926066e6c185e1fdc5d2` |
| `CHARTER.md` | `6afcc623507d0bb42603978d045c37b17a20b87c1f927ddae5813a89c79271fe` |

The snapshot has 27 mechanisms: ten ACTIVE, four READY and thirteen PARKED.
These are imported upstream statuses at that revision. They do not describe
whether this port implements a mechanism. Measurements and reopening text in
the board remain attributed historical statements. No source citation alone
establishes a local measurement or recreates an authorization.

The evidence rules follow charter section 4: distinguish instrument failures
from mechanism results; a negative concerns a configuration; NOT_RUN is not
success; hypotheses are distinct from measurements. Operational authorization
remains outside this component. In particular, neither an imported charter
snapshot nor a record containing owner text can grant a release.

## Complete catalog and typed lineage

`src/R4/Evidence.lex.tex` will define a closed `EvidenceMechanism` enumeration,
not an unvalidated string ID. Stable external labels match the board exactly.
The finite catalog contains every identifier below, in the source's order.

| Imported status | Exact identifiers |
| --- | --- |
| ACTIVE | `address-cells`, `admission`, `discrete-learning`, `flock-read`, `lut-serving`, `prime-zeta-codebook`, `rank-margin`, `relation-read`, `route-read`, `tokenizer` |
| READY | `hamilton-gate`, `object-store`, `recurrence-gates`, `spiralcore-fbs` |
| PARKED | `carrier-credit`, `e8-orbit-memory`, `exact-memory`, `geometric-stack`, `hopf-observation`, `legal-construction`, `native-geometric`, `pointer-key`, `r4g1-graph`, `read-binding`, `spin-weights`, `ternary-xor`, `vsa` |

The source's `family_tree` contains 21 directed edges over 22 mechanisms.
Relations are exactly `successor`, `discrete-form-of` and `specialization`.
Store that edge set once; derive incoming/outgoing projections from it. An edge
describes historical lineage, never implementation dependency or permission to
activate the parent. The five isolated vertices are admission, tokenizer,
Hopf observation, read binding and VSA. Do not drop them from coverage because
they have no edge.

Import the complete source metadata inventory, including canonical names,
aliases, status rationale, next/reopening text, slot map, lineage notes and
recorded contradictions. Keep its bytes/checksum as evidence input; do not
reinterpret a `next` field as a successful result. Presentation text may be
projected from these model-owned literals. A fixture or Markdown table must
not become an independent runtime catalog.

The two explicit board caveats remain attached to their corresponding entries:
D11's ACTIVE label describes an end-of-pipeline contract without a lawful
export, and Hopf's PARKED label groups the failed sector configuration with a
kept-fibre map whose language test was not run. This port imports the labels
and caveats together and does not resolve either status question.

## Local coverage is a separate relation

Normalize this relation into a unique global `Capability` table and separately
identified `PortObligation` records. A capability owns its stable ID, required
source-root identities, execution surface and dependency capability IDs. An
obligation owns a unique obligation ID, a mechanism ID and a capability
reference. The same capability may serve several mechanisms, and one mechanism
may need several capabilities. Do not interpret one verified helper as full coverage.
Keep source construction, finite formal verification, generated execution,
qualified research, product serving and live deployment as separate surfaces.

Every mechanism receives a disposition in the complete coverage inventory:

- ACTIVE: full semantic port and the appropriate generated execution surface
  remain required by the approved design.
- READY: retain the exact contract and its integration prerequisites. D19
  object storage is specifically required for durable conversation memory;
  the other READY mechanisms do not become active merely by being modeled.
- PARKED: retain identity, lineage, evidence and reopening conditions. The
  catalog cannot cause historical banned implementations to enter a runtime
  import closure.

Admission checks the complete 27-entry coverage inventory, unique mechanism
entries, unique global capability definitions and unique obligation identities.
Repeated capability references across different obligations are legal.
Dependency references must exist;
self-dependencies and cycles fail with a typed error. The finite graph check
must inspect all vertices, including disconnected components. A node with no
dependencies is not automatically verified or accepted.

## Evidence descriptions and trust

Records describe evidence; they do not authenticate themselves. Separate:

1. `ImportedFact`: exact source revision/path/digest and an attributed claim.
2. `FormalReceiptReference`: SDK executable and semantics identities, source,
   config and lock identities, complete root selection and receipt digest.
3. `RunReportReference`: registered sheet/configuration/data/executable and
   output identities, observation status and instrument classification.
4. `ProductReceiptReference`: accepted generator/runtime/source/artifact and
   product-test receipt identities for a named execution surface.
5. `Interpretation` and `Hypothesis`: explicitly tagged text with references
   to the records being interpreted, never a replacement measured value.

Every digest and external ID must pass its stated width/encoding admission.
Missing identities are represented as missing and fail any operation that
requires them; an all-zero digest is not a sentinel for validated evidence.
An all-zero digest remains structurally a digest, not cryptographic proof.
Record identity, source identity and deployment identity are different fields.

Reference lists have explicit duplicate policy: capability dependency IDs,
capability source-root identities, obligation IDs within a mechanism, and
evidence-record references within an interpretation reject duplicates. A
capability may be referenced from several different obligations, and a record
may be referenced by several different interpretations. Ordered source metadata
such as alias text and lineage notes preserves its source inventory exactly;
generic deduplication must not rewrite it.

Use closed observation cases `NotRun`, `Running`, `Completed`, `Failed` and
`Unavailable`. Completion only says a referenced operation reports finishing;
it does not encode successful quality or acceptance. An instrument failure
cannot be represented as a mechanism-negative outcome. A mechanism-negative
record names the exact tested configuration, never a terminal mechanism state.
The imported-status enumeration has no retired or closed constructor.

For `RunReportReference`, freeze this status/outcome compatibility table. The
observation cases describe the report, never trusted acceptance:

| Reported operation status | Admitted outcome descriptions |
| --- | --- |
| `NotRun`, `Unavailable` | `NoObservation` only |
| `Running` | `NoObservation` or `PartialObservation` |
| `Completed` | `CompletedObservation`, bound `ConfigurationNegative`, or `InstrumentFailure` |
| `Failed` | `InstrumentFailure` only |

A planned early stop that yields its registered configuration-negative result
is a completed observation, distinct from an operation failure. Diagnostic
quantities may be retained inside `InstrumentFailure`; their presence does not
turn the outcome into a mechanism negative. Import, formal and product records
have their own typed provenance roles and cannot bypass this run-report table
by changing a display label.

The component validates shape, reference consistency and explicit classification
and returns a typed `EvidenceDescription`. It exposes no `isTrusted`,
`qualified`, `authorized`, `canTrain` or `canDeploy` Boolean computed from
caller-supplied receipt fields. Actual SDK receipt validation, signature/custody,
external source verification and live-byte checks remain required integrations.
Do not rename a structurally admitted report into an accepted receipt.

Application presentation can display "reported", "source verified" or
"not run" only from the corresponding modeled distinction. Trusted application
acceptance, when available, must enter through the reviewed generic SDK/runtime
contract bound to the exact artifact; this proposal creates no such bridge.

## Deterministic query operations and typed failures

Public operations return the complete catalog, exact label lookup, incoming
and outgoing typed lineage, imported status, port obligations by mechanism,
and structurally admitted evidence descriptions. Byte/text encoding delegates
to the admitted text/codec component; unknown labels return `UnknownMechanism`
rather than an arbitrary default mechanism.

Proposed ordered failures:

| Boundary | Ordered checks |
| --- | --- |
| Catalog import | source identity, complete ID set/order, duplicates, status/edge equality, metadata inventory identity |
| Port coverage | capacity, complete mechanism set, repeated capability definition or obligation ID, invalid root identity, missing reference/dependency, repeated reference within a set-valued field, self-dependency, cycle |
| Evidence description | byte/capacity admission, malformed identity, missing required binding, dangling reference, wrong record kind, observation/outcome contradiction |

Budgets bound record count, metadata bytes, reference count and graph traversal.
A budget refusal returns no partial successful inventory. Query order is
explicit and stable: catalog/source order for mechanism lists, source edge
order for lineage, registered capability order for obligations. Duplicate
references are either rejected at admission or retained by a documented field
contract; never silently erased when multiplicity carries meaning.

## Frozen controls and implementation sequence

Before definitions, independently project the pinned board's complete literal
inventory. A Rust conformance entrypoint checks that frozen source-derived
control identity and invokes LexLean; it never computes runtime classifications.
The protocol source, local projection and fixtures retain separate hashes.

Required controls include:

- All 27 label round trips, exact imported statuses and the 10/4/13 totals.
- Every one of the 21 directed typed edges, all incoming/outgoing projections,
  and the five isolated vertices. Reversing an edge changes the expectation.
- Completeness and uniqueness: missing PARKED mechanism, duplicate READY
  mechanism, absent dependency, self-loop, disconnected cycle and over-budget
  graph each fail with its exact typed error.
- Valid finite acyclic coverage with two distinct mechanisms referencing the
  same global capability ID must pass. A duplicate capability definition must
  fail. Shared helpers and multiple obligations remain distinct from verified
  or accepted coverage.
- NOT_RUN with a claimed measurement, instrument failure classified as a
  mechanism negative, missing executable/configuration/source bindings,
  malformed digest and dangling interpretation reference fail as specified.
- A structurally valid completed report stays a report; imported ACTIVE status
  and a finite formal proof reference grant no training or deployment capability.
- Port-side source verification cannot alter upstream PARKED status or erase
  reopening text. A negative record cannot remove a catalog entry.

Register the complete scoped capability and imported authorities, freeze these
controls and observe missing-definition RED. Implement the finite catalog,
typed lineage and full admission algorithms, then independently review source
and error coverage. Plant a dropped isolated vertex, reversed edge, ignored
cycle and status/coverage conflation; each must fail the intended control.
Finally run the mandatory source/receipt/kernel gate and integrate with the
default proof graph. The generated application later consumes this model;
a documentation dashboard alone is not the requested deployed application.
