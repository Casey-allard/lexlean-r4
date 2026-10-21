# Evidence description admission API

Status: parent-reviewed and approved refinement of the approved
[Evidence design](evidence-design.md), 2026-10-21. The review accepts the
explicit provenance requirements, status table, reference graph policy and
deterministic validation phases. Independently frozen controls are next.
It is not a source implementation, accepted report, research result or release.
No definitions may precede contract approval, independently frozen expectations
and genuine missing-definition RED. The proposed module is
`R4.EvidenceDescription`; the catalog and coverage modules remain separate.

## Representation and identity roles

`EvidenceInput` contains one ordered list of `EvidenceRecord` values. Each record
has `recordId : Nat`, `mechanism : EvidenceMechanism`, and one of the six payloads
below. Record IDs and every reference target ID must be in `0..4294967295`.
IDs are globally unique across all six kinds, rather than unique within a kind.
The mechanism is a catalog identity, never a locally promoted status. References
may cross mechanisms; they do not imply that one mechanism's result applies to
another. Empty input is a valid empty description, not complete port coverage.

A `DigestBinding` is `Option (List Nat)`. `none` explicitly means missing; a
present digest has exactly 32 bytes. `RevisionBinding` has the same optional
representation but a present value has exactly 20 bytes: this is the pinned
upstream Git SHA-1 revision role, not a SHA-256 source identity. Other Git object
formats require a separately versioned contract. No hex strings, zero sentinel,
implicit truncation, default digest or automatic hashing is used. All-zero
values have valid shape and do not prove authenticity. A record ID, Git revision,
source digest, configuration digest and deployment digest remain distinct roles.

Every text value is an ordered `List Nat` of raw UTF-8 bytes. Admission first
checks the byte domain and then calls the strict UTF-8 semantics of
`R4.TokenizerText.tokenizerDecodeUtf8`. Ill-formed UTF-8 is rejected, never
lossily replaced. Valid input bytes are preserved, including canonically
distinct Unicode spellings. This is the same admitted scalar/UTF-8 behavior as
the existing Text component, without tokenizer segmentation or vocabulary work.
Paths and module labels are nonempty and NUL-free. Other required textual fields
are nonempty; embedded NUL in ordinary claim/diagnostic text is retained as a
valid scalar. No trimming, case folding, Unicode normalization, path resolution,
filesystem lookup, module lookup or URL fetch occurs during admission.

`IdentityRole` is a closed diagnostic enumeration:
`sourceRevision`, `sourceDigest`, `sdkExecutable`, `compilerSemantics`,
`sourceGraph`, `configuration`, `lock`, `selectedRoot`, `formalReceipt`,
`registeredSheet`, `data`, `runExecutable`, `runOutput`, `negativeConfiguration`,
`generator`, `runtime`, `productSource`, `artifact`, `productTestReceipt`,
`deployment`. This labels shape/missing-binding errors; it does not grant trust.

## Exact record payloads

The six payload constructors have these fields, in the stated traversal order.
Field order below also fixes validation tie-breaking within each global phase.

| Payload | Exact fields |
| --- | --- |
| `ImportedFact` | `revision : RevisionBinding`; `path : Utf8`; `source : DigestBinding`; `claim : Utf8` |
| `FormalReceiptReference` | `sdkExecutable : DigestBinding`; `compilerSemantics : DigestBinding`; `sourceGraph : DigestBinding`; `configuration : DigestBinding`; `lock : DigestBinding`; `roots : List FormalRoot`; `receipt : DigestBinding` |
| `RunReportReference` | `registeredSheet : DigestBinding`; `configuration : DigestBinding`; `data : DigestBinding`; `executable : DigestBinding`; `source : DigestBinding`; `output : DigestBinding`; `status : ObservationStatus`; `instrument : InstrumentClassification`; `outcome : RunOutcome` |
| `ProductReceiptReference` | `surface : ProductSurface`; `generator : DigestBinding`; `runtime : DigestBinding`; `source : DigestBinding`; `artifact : DigestBinding`; `testReceipt : DigestBinding`; `deployment : DigestBinding` |
| `Interpretation` | `text : Utf8`; `references : List EvidenceReference` |
| `Hypothesis` | `text : Utf8`; `references : List EvidenceReference` |

`FormalRoot` is `{ moduleLabel : Utf8, source : DigestBinding }`. The roots list
is nonempty and contains the complete *declared* ordered root selection. Every
entry has a nonempty NUL-free label and a required source digest. Labels are
unique by exact bytes; two different labels may share source bytes/digests.
There is no independent list of names with absent identities. `sourceGraph`
identifies the complete source graph separately from this selected-root list.
Admission checks every declared root, rather than only its first element.
Actual agreement with the SDK manifest, transitive source closure and true
completeness of the claimed selection are external receipt-validation duties;
caller-supplied roots cannot prove their own completeness.

`ProductSurface` is closed: `generatedExecution`, `productServing`, or
`liveDeployment`. It cannot be replaced with an arbitrary display string or a
research-qualification surface. Every product record requires generator,
runtime, source, artifact and product-test receipt identities. A live-deployment
record additionally requires its distinct deployment identity. For the other
two surfaces deployment may be missing or present; a supplied binding still
must have valid shape and is retained without inferring a live deployment.
The word “receipt” in the type is a reference role, not an accepted-receipt flag.

An imported fact requires its revision and source digest. A formal reference
requires every listed digest and every root digest. A run reference requires
registered-sheet, configuration, data, executable and source identities for
**every** status. This makes a fully described `NotRun` or `Unavailable` report
a bound plan/report whose operation did not run or could not be observed. A
raw report with missing provenance remains representable in `EvidenceInput`,
but full description returns a missing-binding error. Neither a sheet digest
nor a field containing owner text authorizes execution.

## Run status, outcome and instrument consistency

`ObservationStatus` is exactly `notRun`, `running`, `completed`, `failed`,
`unavailable`. `InstrumentClassification` is exactly `notAssessed`,
`instrumentValid`, `instrumentFault`. These are explicit reported classifications,
not derived credentials or measurements of instrument quality.

`RunOutcome` is a closed sum with these exact payloads:

| Outcome | Payload |
| --- | --- |
| `noObservation` | none |
| `partialObservation` | `observation : Utf8` |
| `completedObservation` | `observation : Utf8` |
| `configurationNegative` | `configuration : DigestBinding`, then `observation : Utf8` |
| `instrumentFailure` | `diagnostic : Utf8` |

Outcome text is an attributed report, not a parsed quantity or an authenticated
measurement. Diagnostic quantities may remain in the instrument-failure text.
The output digest names the exact output artifact or output manifest; it does
not assume a single output file. Multiple files can be bound by that manifest.

The exact compatibility rules are:

| Status | Outcome | Instrument | Output binding |
| --- | --- | --- | --- |
| `notRun`, `unavailable` | `noObservation` | `notAssessed` | absent |
| `running` | `noObservation` | `notAssessed` | absent |
| `running` | `partialObservation` | `instrumentValid` | required |
| `completed` | `completedObservation` | `instrumentValid` | required |
| `completed` | `configurationNegative` | `instrumentValid` | required |
| `completed` | `instrumentFailure` | `instrumentFault` | required |
| `failed` | `instrumentFailure` | `instrumentFault` | required |

Every unlisted combination fails. In particular, completion does not require a
positive result, a planned configuration-negative early stop is `completed`,
and an operation failure is never a mechanism-negative outcome. A negative's
explicit configuration binding is required and must equal the run's bound
configuration byte-for-byte. It identifies this tested configuration, never a
terminal mechanism state. It cannot remove or relabel a catalog mechanism.
An instrument-fault classification cannot be paired with a configuration
negative, even if that outcome has a configuration digest and numerical text.
No-observation records with a supplied output fail `unexpectedOutput`; they do
not silently reinterpret output bytes as a measurement.

The presence requirements for output and negative configuration are determined
by the outcome constructor, before compatibility validation: each outcome other
than `noObservation` requires output, and `configurationNegative` requires its
configuration. Thus missing mandatory provenance precedes an incompatible status.
A malformed supplied optional output fails shape admission even when the later
compatibility rule would prohibit any output.

## Directed reference roles and graph semantics

`EvidenceReference` is `{ role : ReferenceRole, targetId : Nat }`. Closed roles
map one-to-one to target payload kinds:

| Role | Required target kind |
| --- | --- |
| `importedSource` | `ImportedFact` |
| `formalSupport` | `FormalReceiptReference` |
| `runObservation` | `RunReportReference` |
| `productObservation` | `ProductReceiptReference` |
| `interpretationBasis` | `Interpretation` |
| `hypothesisBasis` | `Hypothesis` |

Only interpretation and hypothesis records contain reference lists. Both require
at least one reference. Their directed edge is from the interpreting/hypothesizing
record to its cited record. A role asserts the expected record *kind*, not the
truth of that record or a requirement that its operation completed. For example,
a hypothesis may cite a `notRun` run report through `runObservation` without
turning that report into a result. A hypothesis may cite another hypothesis.
Each target must exist and match its exact role. Duplicate target IDs within one
list fail, including the same target repeated with different roles; wrong-kind
validation occurs before duplicate validation. References from different records
to the same target are valid. Input order is retained; there is no silent dedup.

Self references and directed cycles are forbidden. After global self-reference
validation, visit all records in table order, including disconnected components.
Each work item carries its ID and ancestor path. Dependencies are its reference
targets in list order. Entry consumes one graph visit, ancestor repetition is a
cycle, and shared DAG descendants can consume multiple visits without being
cycles. Empty pending work succeeds at zero fuel; nonempty pending work at zero
fuel fails `graphBudget` before inspecting the next vertex. No partial graph
success is returned. Structural recursion decreases fuel. An internal unresolved
vertex must produce `danglingReference`, never a successful leaf, even though
normal complete admission checks references before traversal.

## Budgets, global phases and exact errors

`EvidenceBudget` has natural fields `maxRecords`, `maxBytes`, `maxReferences`,
and `maxGraphVisits`. Record count is the input list length. Byte count is the
sum of **all actual supplied list lengths** in every text field, revision,
present digest, formal-root label/digest and outcome field, including redundant
negative configuration bytes. Missing bindings count zero. Reference count is
the total interpretation/hypothesis reference-list length; formal roots are
identities, not evidence-record references. Counts use unbounded Nat. There is
no overflow, assumed fixed width before admission, wire serialization claim or
performance qualification. Graph fuel is enforced only in phase 9.

Each listed phase/subphase below scans the entire record table before the next
subphase begins. Within a scan, record order, payload field order above and list
order determine the first error. The single phase-2 domain scan checks a record's
ID followed by its reference IDs before moving to the next record.

1. Record budget, byte budget, then reference budget.
2. UInt32 domains for every record and reference ID.
3. Duplicate global record definitions.
4. All byte domains, then all identity widths, then strict UTF-8 validity, then
   nonempty required texts, then NUL-free path/module labels. Absent identity
   bindings are skipped here. Revision width is 20; every digest width is 32.
5. Required identity presence, then nonempty formal-root selections, then
   nonempty interpretation/hypothesis reference lists. For an absent complete
   roots list, phase 5 has no per-root digest to inspect. Presence checks follow
   field order and the surface/outcome rules above, independent of later status
   compatibility. No ordinary optional deployment binding is required.
6. Dangling references, wrong target kinds, duplicate reference target IDs, then
   duplicate formal-root module labels. Duplicate labels use exact admitted bytes.
7. Status/outcome compatibility, then outcome/instrument compatibility, then
   prohibited output presence, then negative-configuration equality. These are
   global subphases; each run is checked using its explicit constructor tags.
8. Self references across the whole table.
9. Complete graph traversal with the supplied graph-visit budget.

`EvidenceError` is closed with constructors `recordBudget`, `byteBudget`,
`referenceBudget`, `outsideWord`, `duplicateRecord`, `invalidByte`,
`invalidRevisionWidth`, `invalidDigestWidth`, `invalidUtf8`, `emptyText`,
`nulInLabel`, `missingBinding(IdentityRole)`, `emptyRoots`, `emptyReferences`,
`danglingReference`, `wrongReferenceKind`, `repeatedReference`,
`repeatedRootLabel`, `statusOutcomeMismatch`, `instrumentOutcomeMismatch`,
`unexpectedOutput`, `negativeConfigurationMismatch`, `selfReference`, `cycle`,
`graphBudget`, and query-only `unknownRecord`. Missing-binding role follows the
identity field mapping stated above: run `source` uses `sourceDigest`, product
`source` uses `productSource`, and each root `source` uses `selectedRoot`.
No error constructor denotes authorization or mechanism retirement.

## Public results and queries

`describeEvidence(budget, input)` returns `EvidenceResult.described(input)` with
all exact supplied bytes, tags and order, or `EvidenceResult.error(error)`.
There is no successful partial inventory and no implicit promotion of an
imported, formal, run or product record to a trusted receipt.

Every public query first performs the same complete admission. For queries
with a record ID, only after that succeeds does it validate the query ID's
UInt32 domain and look up that ID; an absent in-domain ID gives `unknownRecord`.
It never substitutes record zero or an empty record. The proposed queries are:

- `evidenceRecordFor(budget, input, id)`: the exact selected record.
- `evidenceForMechanism(budget, input, mechanism)`: matching records in input
  table order, including hypotheses and `notRun` reports, without status filtering.
- `evidenceOutgoingFor(budget, input, id)`: that record's ordered reference list;
  the four non-narrative kinds yield an empty list after successful admission.
- `evidenceIncomingFor(budget, input, id)`: ordered `{ sourceId, role } values,
  scanning the record table then each reference list. The selected target must
  exist; records with no incoming edge have an empty result.

Each operation has a closed typed success/error result for its stated value.
They expose no `isTrusted`, `qualified`, `accepted`, `authorized`, `canTrain` or
`canDeploy` Boolean. External cryptographic/custody/source/SDK validation and
live artifact checks remain distinct required integrations. Imported ACTIVE
labels, reported formal proofs and structurally complete root bindings do not
release research or deployment.

## Independently frozen controls before implementation

Control families must include all six record payloads and every admitted row of
the status table, every instrument outcome, valid all-zero digests, exact
20/32-byte widths, exact budget boundaries and raw malformed-list byte accounting.
Test every missing mandatory identity role, optional nonlive deployment, mandatory
live deployment, all root entries beyond the first, empty/duplicate root labels,
distinct labels sharing one source digest, and malformed optional bindings.

Freeze UTF-8 ASCII/multibyte/path cases and malformed byte, overlong sequence,
surrogate, invalid continuation and truncated sequence cases using the existing
strict Text contract. Include nonempty and NUL label boundaries without inventing
normalization. Freeze all record/reference UInt32 fields at MAX and MAX+1,
duplicate IDs across different kinds, dangling targets, every wrong-kind role,
within-list duplicate refs, valid across-record sharing, cross-mechanism refs,
self-reference, disconnected cycle, converging DAG visit counts, and zero-fuel
empty/nonempty cases. Test all query orders, full-admission-before-query errors,
unknown IDs and query ID domain errors.

Multi-fault controls must exercise the global phases, especially malformed
identity before missing binding, missing binding before dangling reference,
wrong reference kind before duplicates, missing output before incompatible
status, status before instrument mismatch, and self reference before graph fuel.
A `notRun` report with a measurement, an instrument fault labeled as a bound
configuration negative, a negative bound to the wrong configuration, and a
completed report missing source/executable/configuration must each fail exactly.
Valid reported completion remains only `described` and cannot change the catalog.

Required semantic plants include ignoring the second formal-root identity,
accepting `notRun` measurement, conflating instrument failure with configuration
negative, skipping negative-configuration equality, accepting a wrong-kind or
dangling reference, and accepting a disconnected cycle or unfinished graph.
Contract review, frozen-control review and RED precede definitions. Source review,
all literal controls, planted defects, restoration, mandatory receipt auditing
and project integration follow. None substitutes for complete R4-17 delivery or
for the separately required generated application.
