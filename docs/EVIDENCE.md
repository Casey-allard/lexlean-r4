# Mechanism catalog and evidence implementation

The approved [design](specs/evidence-design.md) governs R4-17. This component is
registered before its definitions. Implementation and formal acceptance remain
pending; neither this document nor imported metadata is an execution receipt.

## Frozen catalog controls

The exact upstream board is retained as
`tests/fixtures/evidence/upstream-board.json`, SHA-256
`357cdfc926a220239193b19c6d2fd86beea4e6c0d843926066e6c185e1fdc5d2`.
It comes directly from the pinned revision named in the design. No upstream
mechanism status or historical result has been locally reclassified.

Before source definitions, 358 source-derived catalog controls were authored
in `tests/lexlean/EvidenceCatalogControls.lex.tex`, SHA-256
`171feb942c88b96189667afad97eefb4613b13d252124f55d52c6845cfb36324`.
They retain every label, imported status, canonical name, alias, plain-language
description, replacement slot, status rationale, next/reopening text and
lineage metadata. They also retain the complete 21-edge ordered family tree,
all incoming/outgoing projections, all slot-map entries and all 18 recorded
contradictions. The imported counts are independently checked as 10/4/13.
Five unknown-label controls require exact lookup without trimming or case
folding. A static projection check confirms each source `lineage_detail`
equals the corresponding ordered family-tree projection.

These expectations fix the catalog API before implementation. Mechanisms,
statuses and relations use closed enumerations. Metadata uses typed records
and string literals; source labels remain exact. The one edge inventory
supplies incoming/outgoing queries. The complete source board remains an
authority fixture, while application queries must use LexLean definitions.

The isolated old-SDK project observed missing-definition RED: `LLR3005`,
`EvidenceCatalogControls` imports absent `R4.Evidence`. The result is retained
at `/tmp/r4-evidence-catalog-red.json` in the development container. An earlier
staging attempt omitted the Lake files and failed project setup; that failure
is not the missing-definition result. No definitions were present in either.

Independent catalog projection review, graph/evidence admission controls,
implementation, planted defects and mandatory receipt verification are still
required. The 358 catalog controls alone are not full
R4-17 coverage and are not added to the default accepted proof roots.

## Catalog implementation and isolated replay

Independent review compared all 358 expected assertions against the pinned
board. A subsequent source review checked all 247 literal assertion bodies
against the fixture and inspected the derived queries separately. The source
has 34 declarations and SHA-256
`d25bfb1c0a68953bb03262248e5daab1ff3fb88e1a9cfc9fc7db0b3d9a9f23c8`.
It defines the closed mechanism/status/relation types, complete metadata,
one ordered edge inventory and structurally recursive exact lookup, status
counts and lineage projections. String equality was independently verified
under both official language versions without axioms; this source stays 1.1.

The isolated old-SDK verification passed all 358 controls, with receipt
`f5616e86cc3a1a99771802184a726a077f58c59dbef6633a148492f1925b6f0b`.
The parent checked all 392 unique declaration audits, every control's presence,
empty observed/allowed axiom lists, both independent kernel replays and the
manifest bytes/length. All four non-lexicon manifest inputs match the frozen
isolated project. This is catalog-only formal evidence; full R4-17 admission
and application integration remain pending.

The next [coverage admission contract](specs/evidence-coverage-api.md) has
independent architecture approval. Its graph and evidence-report controls must
be frozen before their respective definitions; the catalog's success does not
substitute for either obligation.

Four catalog semantic plants were tested against the unchanged fixture:
removing isolated VSA from the catalog; reversing the first lineage edge;
promoting Hopf from PARKED to ACTIVE; and substituting a default mechanism
for an unknown label. Every mutant passed source checking and failed the
intended formal assertions with `LLV7002`. The original source was restored
byte-for-byte and passed again with the same `f5616e86...` attestation above.
Sources, source-check results, proof diagnostics and the restoration result
are retained under `/tmp/lexlean-r4-evidence-plants-20261021`. These are catalog
defect checks; they do not stand in for the required graph/report defects.

## Coverage admission implementation

`R4.EvidenceCoverage` implements the independently approved
[coverage admission API](specs/evidence-coverage-api.md) with 73 semantic
declarations. The source SHA-256 is
`9f5876e60d381b96244952b67f87086f45be201a325faa01314014d6238d16c8`.
All application behavior is authored in LexLean. The module imports the
catalog's closed mechanism enumeration and complete 27-member inventory.

The public functions are `describeCoverage`, `coverageObligationsFor` and
`coverageGraph`. Complete admission returns the exact supplied capability,
obligation and coverage tables, or a distinct typed error without a partial
description. The query performs the same complete admission before selecting
obligations in capability-table order and then obligation-table order within
each capability. It does not use coverage-reference order as query order.
The graph helper checks only graph traversal, including typed refusal of a
missing vertex; its `acyclic` result does not replace complete admission or
admit arbitrary IDs, roots or coverage tables.

Admission enforces all seven ordered phases of the contract. Counts use
unbounded naturals and compare actual list sizes against explicit record,
source-byte and reference budgets. Every ID-bearing field is admitted to
UInt32 before any root-shape check. Each source digest must contain exactly
32 values in the byte domain; byte-domain failure precedes width failure for
that digest. All-zero digests remain structurally valid. The entire set of
27 mechanisms is required exactly once, including isolated and PARKED
mechanisms. Capabilities and obligations have unique global IDs. Each
obligation must be listed exactly once under its own mechanism. Distinct
obligations may share a capability, and distinct capabilities may share a
source root. Duplicate dependencies and roots are rejected within each
capability, and duplicate obligation references within each coverage entry.

Global validation subphases preserve the explicitly reviewed precedence:
missing capability, missing dependency, missing obligation, mechanism
mismatch, unlisted obligation, then empty obligation list. Duplicate-reference
checks inspect dependencies before source roots before coverage references.
Self-dependency is rejected before graph traversal. Every capability starts
a traversal, including unreferenced and disconnected vertices. Work items
carry their ancestor paths; encountering an ancestor detects a cycle without
mistaking a shared DAG descendant for a cycle. Dependency and root order are
preserved. Each entered work item consumes one visit. Empty work succeeds
with zero fuel; nonempty work with zero fuel returns `graphBudget`, even if
the next item would expose a cycle. Recursion decreases the supplied fuel.
There is no global visited-set shortcut or successful unfinished traversal.

The 57 original controls were frozen before definitions at SHA-256
`475fb9138594c96b5d485f19025d7219359e8ad0e59169f0fab4cae522acb28e`.
Six additional boundary controls were independently reviewed and frozen at
`f602de7fc17acfe413ac55251ada3251cbdbed1f2e046ec758e946b6a7040ecb`.
Both observed missing-module `LLR3005` before source definitions. The controls
include exact budgets, all six execution surfaces, all ID-field domains,
duplicate definitions and references, missing isolated/PARKED coverage,
legitimate sharing, malformed digests, disconnected cycles, converging DAG
visit counts, zero-fuel cases, query order and multi-fault precedence. The six
additions specifically cover raw malformed-root byte-budget precedence,
within-capability query order, two global subphase orderings, all ID domains
before root shape, and the graph helper's missing-vertex error.

The isolated development SDK verified all 63 unchanged controls. The original
57 produced attestation
`37e805a894da3da17e1568f0f2b218fcaf048cb8753bab7ec29ec2a3df2a98e8`;
the additional six produced
`67fdd2c3a91c90f182f77ea5d2ba02bdd36d12c73410954ee4ef91d6bfd20dd5`.
This is finite metadata formal verification, not execution of a learned model,
research qualification, serving acceptance or deployment evidence. Source
identities and surface classifications describe obligations; no trusted,
implemented or verified flag is inferred from them. Independent final source
review, aggregate receipt acceptance and evidence-report description admission
remain separate requirements before full R4-17 integration.

Four source-valid semantic plants were rejected by the unchanged original
controls with `LLV7002`: starting traversal from only the first capability
(three failures); treating shared capability references as duplicate obligation
definitions (39 failures); skipping isolated VSA in complete-set validation
(one failure); and accepting nonempty work at zero fuel (four failures).
The original source was restored byte-for-byte and both control modules
reverified with the exact attestations above. The isolated first mutation
serialization attempt was noncanonical JSON and failed source checking;
that authoring fault was corrected before all four source-valid experiments
and is not counted as semantic rejection. The mutant sources, successful
source checks, proof diagnostics and restoration receipts are retained under
`/tmp/lexlean-r4-evidence-coverage-evidence-20261021`. These checks exercise
coverage semantics and do not establish evidence-report acceptance.

## Evidence-description implementation

`R4.EvidenceDescription` implements the reviewed
[description admission API](specs/evidence-description-api.md) with 103 semantic
declarations. Source SHA-256 is
`7c1140e81f5410afc174084e455a24ab895b5e5008804923a2abea936d554781`.
Its six payloads are imported fact, formal receipt reference, run report
reference, product receipt reference, interpretation and hypothesis. Distinct
record IDs share one admitted UInt32 domain. Source revision, source graph,
configuration, executable, artifact and deployment identities occupy distinct
fields. Optional identity bindings represent absence explicitly. Required
missing bindings return their closed identity-role error, without zero defaults.

The module derives an ordered field inventory from each typed payload. Its
internal field tags select byte, width, UTF-8, label and presence checks; callers
cannot supply those tags through `EvidenceInput`. Complete admission derives
that inventory itself and scans the entire record table separately for every
global validation category. Twenty-byte Git revision and 32-byte SHA-256 roles
are distinct. Byte accounting uses actual supplied lengths before admission,
including malformed optional identities and the negative configuration binding.
Strict text admission calls the existing `R4.TokenizerText.tokenizerDecodeUtf8`;
there is no host decoder, lossy repair, normalization, filesystem lookup or
external source fetch. Admitted original bytes and ordering are preserved.

Formal references bind SDK executable, compiler semantics, source graph,
configuration, lock, receipt and every declared selected root. Root labels are
nonempty, NUL-free and unique; distinct labels may share a digest. The complete
declared selection is traversed. The correctness of that declaration against
an actual SDK manifest and source closure remains an external receipt-validation
obligation. Merely supplying a root inventory cannot authenticate its completeness.

Run validation preserves every row of the reviewed status/outcome table.
`notRun`, `unavailable` and running-without-observation reports have no output.
Every observation requires its output identity, and every status requires the
registered sheet, configuration, data, executable and source identities. Status
compatibility, instrument classification, forbidden output and negative binding
are independent global phases. A configuration negative requires byte-exact
equality with the run's configuration and a reported valid instrument; an
instrument fault cannot become a mechanism negative. Completion is reported
completion, not accepted quality. Live product deployment additionally requires
a distinct deployment identity; another product surface does not become live
because an optional deployment field is present.

Interpretations and hypotheses require nonempty typed reference lists. The model
checks target existence, exact target kind, duplicate targets within each list,
self references and cycles. Distinct records may cite the same target and may
cross mechanism boundaries. All-record ancestor-path traversal preserves reference
order, inspects disconnected components, revisits shared DAG descendants and
refuses unfinished work at zero fuel. No trusted status is inferred from a
reference role. The record, mechanism, outgoing and incoming queries all perform
complete admission first; query ID bounds and unknown IDs follow that admission.
A successful description returns the exact input and cannot change the imported
catalog or authorize any research, training, serving or deployment action.

### Frozen description controls

The parent authored the expectations independently; a non-author reviewer checked
all original 204 and the two additional cross-record controls before definitions.
All four files observed `LLR3005` importing absent `R4.EvidenceDescription`.

| Fixture | Controls | SHA-256 |
| --- | ---: | --- |
| `EvidenceDescriptionIdentityControls.lex.tex` | 65 | `2e8f4f92d9148a063fbff3f84247e63eb37164a2e0ccecfdaec000fc73fd0b44` |
| `EvidenceDescriptionRunControls.lex.tex` | 91 | `355418af17e04fe34087182db878378f88c1414b2083b8efc971ca9ee7b791fb` |
| `EvidenceDescriptionGraphControls.lex.tex` | 48 | `6fd332a1910efc32632da64ff6d671278d50bea19c71e3eb8f36045d7727812d` |
| `EvidenceDescriptionBoundaryControls.lex.tex` | 2 | `c39d353c5a1812ab1805415fe0dbb6c80595fe3dd113b395784469411d3cb3c7` |

These controls include all 75 status/outcome/instrument combinations, every
required identity role, exact 54/193/225-byte fixtures, UTF-8 negatives,
second-root identity validation, all reference roles, a disconnected cycle,
converging DAG visit count 10, shared references, query ordering and admission
precedence. The added controls establish that a later record's invalid byte
precedes an earlier width error, and a later status mismatch precedes an earlier
instrument mismatch. All 206 literal statements and proofs remain unchanged.

All four suites passed isolated formal verification on the development SDK:

| Fixture | Attestation |
| --- | --- |
| Identity | `17e229311ba40e1d011e6d06cc2e4c767df7d76450a9b3cdce1fd6f2a678551e` |
| Run | `85095631bc98339c8083f2aa28cdfcfb7093562e02a65c528bb9f8088d8227f7` |
| Graph | `40eac520bc6b5d664a6f983ecb9e2d636ff92badf496589ccc8f9391ae097aa1` |
| Boundary | `a6df2cbe78d83ef05f2d01b1867fc509224497eda7b60aaca8acf2cb8e552291` |

This is finite report/metadata formal verification. Independent final source
review, semantic-plant restoration and the parent-owned aggregate Rust receipt
gate are separate checks; production binding and generated application delivery
remain outstanding requirements of the full project.

Independent final source/spec review and subsequent code/security review found
no issues at the source identity above. The reviewer inspected all 103
declarations and verified that the authoring representation produces the exact
source bytes. Seven isolated semantic plants each passed source checking and
failed the unchanged controls with `LLV7002`:

| Planted defect | Rejected assertions |
| --- | ---: |
| Ignore formal-root fields after the first root | 1 |
| Admit observations under `notRun` | 12 |
| Ignore instrument consistency for configuration negatives | 3 |
| Ignore negative-configuration equality | 1 |
| Accept the wrong reference target kind | 7 |
| Traverse only the first record, skipping disconnected components | 2 |
| Accept unfinished graph work at zero fuel | 2 |

The original source was restored byte-for-byte; all four suites passed again
with the same attestations listed above. Frozen fixtures, missing-definition
RED, mutant sources, successful source checks, proof diagnostics, restored
attestations and audit summaries are retained under
`/tmp/lexlean-r4-evidence-description-evidence-20261021`. These results establish
finite description semantics. The parent-owned aggregate receipt gate and
project integration remain separate from the isolated verification.

## Integrated construction-SDK acceptance

The mandatory `cargo test --offline --locked -p repo-conformance --test evidence`
passed in 66.57 seconds: one test, zero failures, ignored or filtered tests.
The gate freezes the full upstream board and all seven independent fixtures,
then invokes the unchanged current-input/source/receipt/kernel validator.
All 627 controls passed with 1,623 none-policy audit rows and 28 independent
kernel replays across seven invocations. These totals include repeated source
declarations and replays; they are not unique declarations across the project.

| Fixture | Controls | Attestation |
|---|---:|---|
| EvidenceCatalogControls | 358 | `3f6bdfbbeb989246c3e817db9d86190137e3de0b35f9ca66200839291f3b00cf` |
| EvidenceCoverageBoundaryControls | 6 | `c7f7e1c0098a523bc97561beedd802b4a6d4e2faa0bdbfbb35dbfe35b9bae3c8` |
| EvidenceCoverageControls | 57 | `e9122fdb42e7367c7211d8e57957eba37b22a2d318c2cabda3a3c08b63ebfe2b` |
| EvidenceDescriptionBoundaryControls | 2 | `d2917ee2067602c10e88bd971c65184331021c72bc007461eabf45476d0fbcc7` |
| EvidenceDescriptionGraphControls | 48 | `a53ff20efbd31a57a613111b9157c3008e915e8c9fcca1045c7b24219bd119e9` |
| EvidenceDescriptionIdentityControls | 65 | `60c098362898d01d36988c7684a8c0988e6d4671850b3bd26c4ab5ef82395ade` |
| EvidenceDescriptionRunControls | 91 | `2972a7b10a74b61c99c55749346f9f86d9e8440264a94ded222cb557f66079f0` |

An independent post-run audit checked every theorem, all process exits, empty
observed/policy axioms, exact manifest identity/length, the executable and
language-1.1 compiler semantics, and every non-lexicon input against the current
worktree bytes. Configuration SHA-256 is `2d09b3cfbaffd840e89cd4d93c5e02fcc98427f69f36f00a6114c7905591a497`;
lock SHA-256 is `39d2c63edb4979e9aa6357ad63034302e9dcbce3e855bde9e90aac2d8a24e551`.

Fifteen-ID model generation, formatting, lock checking, all-target/all-feature
Clippy with warnings denied, and source checking of the complete 77-module graph
passed. The combined 7,263-control proof replay remains pending. Full `just vv`
passed model consistency and dependency ownership, then failed at template-check
line 17 because `prismpm.lock` is absent. An earlier configuration attempt
correctly rejected unsorted entrypoints with LLC0101; sorting the same 55 unique
roots and regenerating the lock corrected it before proof execution. The failed
configuration is retained separately.

These results establish the complete R4-17 development component. They do not
establish that every declared application obligation is implemented, that a
caller-supplied receipt is authentic, or that research/product/deployment
authority exists. Full generated application coverage and the accepted SDK
remain separate requirements.
