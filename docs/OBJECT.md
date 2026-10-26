# Content-addressed object model

Updated 2026-10-26. The five canonical pure LexLean Object modules are integrated.
Independent development verification accepts **331 of the original 403 Object
controls**: Store20 and Codec311. The remaining Read33 and Construction39,
including the original 257-byte construction, are unfinished. Local connected
Read certificates do not discharge the original Read control suite.

| Evidence | Accepted scope | Remaining obligation |
| --- | --- | --- |
| [Store20](continuation/2026-10-23/object-store-integration/RESULTS.md) | Twenty original store controls in the pinned development closure | Full component integration and semantic defect qualification |
| [Codec311](continuation/2026-10-24/object-codec-integration/RESULTS.md) | All 311 canonical codec controls | Required component defects, restoration and integration |
| [Current continuation](CONTINUATION.md) | Separately reviewed local Read/construction certificates and classified failures | Original Read33, Construction39 and whole257; no equivalent timeout/OOM retry |

Integration must also consume the actual native `core-module/1` Read receipts
alongside semantic receipts, preserving all original goal mappings, source/lock
identities, empty-axiom audits and kernel replays. The current semantic-only
receipt checker does not establish that integration.

R4-13 model and scenario additions remain unfinished drafts. The committed model
is the authority for registered claims; neither this page nor a development
fixture promotes R4-13 to a verified build claim. The genuine production SDK
binding, full `just vv`, generated application and Pages remain required.

This page's original draft is retained [byte-for-byte in the historical
snapshot](continuation/2026-10-23/pending/docs/OBJECT.md). The
[preservation-context successor](continuation/2026-10-26/draft-disposition/preservation-context.json)
separates that historical copy from this maintained documentation. It changes
no compiler input, proof statement, lock, artifact or acceptance count. A future
Read invocation must explicitly review the successor preservation context.

The contract is [the reviewed memory protocol design](specs/memory-protocol-design.md),
using UOR-R4 revision `4611538bb4843947363ccb9cce5f388e32c46883`.
Its `crates/uor-r4-object-store/src/{lib,record,manifest,store}.rs` define
the selected byte layouts, deterministic construction and immutable-store
behavior. The protocol and retained fixtures independently supply known answers.

## Implemented component boundaries

1. `R4.ObjectTypes` defines distinct node, whole-object digest and manifest
   version identities; closed schema, integrity, missing, range, binding and
   budget errors; explicit caller limits; roots, records, manifests, inventory
   and statistics; and distinct closure, range and whole-verification results.
   Public object names use an `Obj` prefix to avoid known generated-exporter
   namespace collisions. Byte lists admit only values 0 through 255. Natural
   representations of wire integers are checked against their exact widths.
2. `R4.ObjectCodec` implements checked byte spans, little-endian lengths,
   canonical descending base-three frontier scales, exact record and manifest
   codecs, actual node hashing and actual manifest-version hashing. It rejects
   trailing bytes, invalid widths and scale 38. The total padded-span check is
   an explicit strengthening of the pinned manifest validator: upstream checks
   individual root spans there and checks accumulated offsets later during
   reads. No stronger default size limit is invented.
3. `R4.ObjectStore` implements canonical 32-byte leaves, final zero padding,
   ordered ternary carry, construction-visit preflight, sorted distinct record
   inventory, immutable staging and raw import/export. Identical duplicates add
   no retained records or bytes; conflicting record contents under one ID
   fail integrity. Limits count existing plus staged inventory. Import parses
   all complete records, permits identical duplicates and rejects fragments.
   Failed operations return a typed error and leave the original inventory unchanged.
4. `R4.ObjectRead` implements authenticated external-record fetches, checked
   offsets, intersecting-occurrence preflight, ordered traversal, final-leaf
   padding checks, exact range bytes/remainders and separate whole-object
   rehashing. Empty ranges perform no record fetch. Statistics distinguish
   repeated occurrences, distinct IDs and wire bytes over occurrences.
   Closure discovery tracks `(id, expectedScale)`, descends present parents,
   stops at missing parents and rejects inconsistent scale reuse. Sorted
   missing IDs and deterministic first-traversal missing IDs remain distinct
   APIs. Closure completeness never implies whole-object digest verification.
5. `R4.Object` exposes admitted operations over these components. Externally
   supplied records, inventory and manifests are validated at the boundary.
   No operation accepts supplied hash results as a substitute for computation.

The algorithms accept explicit caller budgets, with the design's defaults as a
convenience. Structural recursion follows input lists or decreasing scales;
administrative fuel must not silently shorten a successful result. Arithmetic
must reject width overflow before constructing an invalid span or offset.

## Required verification sequence

The scenario and theorem-only controls are written before implementation and
must fail against the absent module. Independent expected literals are frozen
from the pinned source fixtures. The first construction set covers empty,
1, 31, 32, 33, 64, 65, 96, 255, 256 and 257 bytes, plus the independent
96-byte protocol vector and its exact record, manifest and version bytes.
Boundary controls additionally cover 95/97 bytes, repeated content, ordered
children and more than one carry. Larger published vectors remain imported
source facts until their actual verification is recorded; retaining their
identities does not establish a locally verified implementation claim.

Negative controls cover every codec truncation point, trailing bytes, invalid
byte values, scale 37/38, unsigned-width and padded-span overflow, canonical
frontier mismatch, each relevant budget, missing descendants, wrong child
scales, corrupt external records, inconsistent scale reuse, nonzero final
padding despite a correct trimmed digest, invalid ranges and atomic failure.
Repeated content must retain all source occurrences during construction and
reads. A full-length range still returns the range verification type.

Controls invoke the actual SHA-256 and configured BLAKE2b-96 model. If reduction
requires decomposed certificates, each certificate must connect the actual
production transitions completely; a literal expected digest cannot enter an
implementation path as trusted identity. Compiler limits, generated Lean and
axioms remain unchanged.

Representative planned planted defects are reversed child order, hashing
padded rather than logical bytes, omitting final-tail validation, deduplicating
read occurrences, ignoring existing-inventory capacity and stopping closure
discovery at an already present parent. Each negative must be followed by a
fresh restored verification and the shared Rust receipt-wrapper test
`canonical_content_objects_r4_13`.

## Claim boundary

This component models content-addressed byte objects. Peer framing and sessions,
peer transactions, tokenizer occurrence bindings (`UORBND01`), durable browser
compare-and-publish, operation receipts, recovery and continuation cursors are
separate required components. No local proof establishes browser durability,
production throughput, generated-runtime acceptance or learned capability.
Hashes are integrity commitments under collision resistance, not signatures,
authorization or an injectivity axiom.

## Implementation surface and retained development history

The public facade supplies `ObjFold`, `ObjImport`, `ObjExport`, `ObjRangeRead`,
`ObjWholeRead`, `ObjDiscoverMissing` and `ObjValidateClosure`. A store is a sorted
unique record inventory plus explicit limits. A read backend is a sorted unique
list of `(NodeId, raw record bytes)` entries; each visited payload passes the
actual record decoder and hash check. Backend key validation does not trust
payloads or claim to fetch them. Internal `Unchecked` encoders are used only
under admitted public boundaries or to construct bytes that are subsequently
authenticated. They are not proposed application entrypoints.

The range's upper endpoint is named `stop` in semantic records because `end`
is reserved in the generated Lean syntax. The wire protocol is unchanged.

The chronology below records the original 2026-10-21 experiments and their
then-current outcomes. Its temporary paths and pending statements are historical;
the dated status and linked continuation above govern current progress. The exact
original text remains in the snapshot, including every failed experiment.

The initial RED command verified `ObjectCodecControls` before any object source
existed and failed `LLR3005`: `R4.ObjectTypes` was not a project module.
Subsequent source parsing/linking/build generation passed for the complete
eight-module object closure in an isolated workspace. A source build is not
formal acceptance.

The initial codec aggregate exceeded the unchanged 300,000 ms child timeout.
The unchanged singleton `objSpanLast`, asserting the exact scale-37 byte span,
also exceeded that limit (observed 300,006 ms). Inspection of emitted code
identified deferred repeated tripling of an accumulated natural expression.
This was an instrument failure, not a failed object contract. Timed-out child
Lean processes left by the old SDK were identified by their exact staging
paths and terminated after the verifier returned.

The span implementation now selects from all 38 protocol-admitted values.
Controls anchor scale zero, connect every adjacent entry by multiplication by
three expressed as additions, check the literal scale-37 value, and connect
its next multiplication to u64 overflow while rejecting scale 38. The exact
previously timed-out singleton passed with attestation
`eb2a5159c445d303229cd34401943d11e02e34f3b0d7134ae6e97b2469014d5a`.

Generic division retains a complete bounded u64 algorithm. It forces the
current natural constructors while constructing divisor powers and stops
constructing larger powers once they exceed the numerator. No table of
particular input answers or new input restriction is introduced. Independently
isolated frontier-33 and large little-endian encode literals passed. A separate
maximum-length decode hit the unchanged recursion-depth limit because unary
byte multiplication nested eight 255-step reductions. The decoder now uses
all 64 positional bit weights, with every one-hot position and the full u64
maximum controlled independently.

The first complete set of 246 codec controls passed on the unchanged old SDK
with attestation
`dbd0c14ab9a790db5c213a3d54b4b2d3dffd4ae72665d9b5c6bc490536e53951`.
Additional parent-record truncation and typed-error controls were added after
that receipt and are not covered by it. New-SDK integration, actual composed
hash controls, planted defects and independent final review remain pending.

The expanded set of 311 codec controls passed on the same old SDK with
attestation `5fa73e0c6fed992b91e91971706f82a5f1a44e539a7d7d6005e0e317d41cc9d6`.
It adds parent truncations, typed hash-input refusal, maximum aligned frontier,
maximum-u64 division by 3 and 256, and subtraction boundaries. The generic
subtraction retains its complete 64-bit greedy algorithm, with exact early
returns for `left <= right` and admitted `right = 0`. These avoid deferred
expressions in nested visit planning without restricting inputs. Fourteen
unchanged facade shape/budget/missing controls then passed with attestation
`d7b6827fd03b30c69909e99cf1929d074fc41e80f436766fd77dd4b247c3721c`.
These receipts do not cover successful composed hashing or final integration.

The current authored inventory contains 397 theorem declarations: 311 codec,
39 construction, 31 read/closure and 16 store/import controls. The construction
set additionally observes actual folds at 95/97 bytes and repeated zero leaves;
the read set adds complete protocol reconstruction, cross-leaf output and
occurrence counts, closure completion and wrong expected-scale integrity
failure. These additions are pending formal receipts.

The first unchanged new-SDK public composition control, `objFoldDigest0`, passed
through all eight source modules with attestation
`a4d2cdf41b66b8f8ea20cb043138b440335aae7238951990755e0d9d8e2305eb`.
The scratch workspace first required normal Lake manifest creation and its own
lock regeneration to satisfy the strict identity probe. That setup failure is
separate from the subsequent formal result.

The unchanged one-byte public fold digest control also passed on the new SDK,
attestation `4fdbe1aadbdd8ee45d0960c8d8db94b14dc7d8b11d8508b00259a28549269e83`.
Both successful composition controls retain their original reflexivity proofs,
actual hash calls and independent digest literals. They do not establish the
remaining construction or read cases.

Independent source review found and repaired two defects before acceptance.
Typed record admission had serialized fields before checking their individual
boundaries, allowing an 11-byte ID plus a 33-byte payload to alias a valid
12-byte-ID/32-byte-payload leaf wire. Admission now validates ID width/domain,
scale, exact payload width and payload byte domain before the existing decoder
and hash check. Four controls cover both directions of aliasing, directly and
through inventory export admission. The short-ID control failed with a genuine
`LLV7002` equality mismatch before the repair.

Read accounting had checked wire-byte overflow before authenticating the
current record. Pinned `store.rs` checks visits, authenticates the record, then
adds its wire size. The model now follows that order. Two finite internal-state
controls at maximum u64 wire bytes require missing/malformed current records
to retain their source error priority; both failed with genuine `LLV7002`
equality mismatches before the repair. All six repair controls passed on the
new SDK, attestation
`54d502203356e474f432594f05fa6c202d356c0fecbf6e79d6bb94b9611f5c08`.
The current authored inventory is consequently 403 declarations. Earlier
receipts identify their historical source bytes and are not final acceptance
of this repaired source.

The unchanged 96-byte public fold digest control hit a compiler resource
failure on the new SDK. It started at 09:16:49 UTC on 2026-10-21; Docker recorded
an OOM event at 09:21:20 UTC in the existing 4 GiB container. The verifier
returned `LLV7002` with empty diagnostics, and no child process remained. The
bounded Docker event is retained locally as
`/tmp/lexlean-r4-object-fold96-oom-20261021.jsonl`. This is an instrument failure,
not a semantic counterexample. The fixed compiler timeout, container memory,
source semantics and original expected statement were unchanged. The larger
257-byte composition has not yet run against this unresolved resource issue.

A proof-only decomposition is being tested: independent equations for the
published sequential first/second leaf IDs and the published 96-byte SHA,
followed by the exact original public-fold statement using theorem-reference
simplification. The runtime functions continue to call the actual hash model;
no supplied-hash parameter or trusted identity cache is introduced. Neither
the source build of this decomposition nor its staged equations establish
formal acceptance before the kernel receipts pass.

Two initial proof-only decompositions failed: broad unfolding exceeded the
simplifier's unchanged step budget; a narrower version left unresolved goals
after expanding branches around unknown intermediate hashes. These failures
did not change the original statements or establish a counterexample. The
next decomposition uses complete closed transition equations, so each actual
node hash, record construction, inventory insertion, carry and finalization
connects to the original public fold.

After those measured resource failures, the parent authorized a bounded Rust
cryptographic fixture test for only the frozen sequential 96- and 257-byte
inputs. Independent review approved its source before execution. It uses the
byte-identical `hash.rs` from the pinned upstream commit, no external
dependencies, and retains both original final-root assertions. The exact one
test passed, emitting four and thirteen chronological record occurrences.
These intermediate identities are fixture-authoring facts, not LexLean
acceptance; each must be proved through the unchanged actual hash model and
connected to the original fold theorem.

The temporary reference source SHA-256 is
`e2bc92877c581f923e51ec728b9bb62926652121c37b95a9c1c23d7d885e189d`;
the pinned hash source is
`f82cc1b4d4ef9ed2f0f0b933a9c8009611fe774de3666299a5f977fc0ed94b5f`.
Rust 1.97.1 (`8bab26f4f68e0e26f0bb7960be334d5b520ea452`) built the test
executable with SHA-256
`4416a804cd3385a1c054fbe76b530d73672a9ff160a89dff6143e26189a47549`.
Its exact output has SHA-256
`ae10d2cfc7d104f92bb4ac5b52a4329e61cadf1c82cae9e43b2131d4da69f368`.
Source, compiler identity and output remain in the local evidence directory
`/tmp/lexlean-r4-object-reference-20261021`; no Rust object implementation is
added to the product.

The first complete 96-byte transition attempt compiled all four actual node
hash equations, their record-wrapper equations and the sorted insertion
equations. Its carry simplification left the closed natural-equality guard
unreduced. An explicit reflexivity equation for that guard then reduced it to
`if True`, which the fixed `simp only` proof form still left unresolved.
Neither attempt produced an accepted aggregate receipt. The next staged proof
uses direct reflexivity for each bounded carry and a separate definitional
bridge through the admitted preflight checks; only result matches remain in
the subsequent certificate composition. The original digest, root and record
count statements for both 96 and 257 bytes are unchanged.

The standalone preflight bridge also exhausted the fixed container memory:
the process exited 137 with `Killed`, and Docker recorded the fresh OOM event
at Unix time 1792577249. Reflexivity still reduces the hash-dependent match
discriminator; the bridge did not isolate that work. Its emitted source and
exact event are retained locally as
`/tmp/lexlean-r4-object-trace96-bridge-emitted-20261021.lean` and
`/tmp/lexlean-r4-object-bridge-oom-20261021.jsonl`. The full 96/257 traces remain
unaccepted. A narrowly scoped Boolean-branch normalization is proposed for
independent review; no such source change is covered by the earlier receipts.

The proposed final Rust gate uses the existing shared verifier on one aggregate
control root. It first checks the four original fixture byte digests, all 403
unique theorem names, empty axiom policies and their complete statement and
parameter trees. Each aggregate theorem must preserve that exact statement and
use only an argument-free `Apply` to its registered case theorem. Every case
and connected helper source has a frozen byte digest; the complete case mapping
must be bijective over the original names. The gate rejects missing, duplicate
or extra registered cases and unregistered case-module files. All qualified
references remain part of the statement comparison.

The aggregate imports the actual case/helper graph. The shared verifier checks
the exact selected modules, source/configuration/lock identities, every audited
declaration's empty axiom policy and independent kernel replay for every module.
Proof decomposition therefore partitions compiler work without selecting a
subset of the original assertions. This is the integration design; the Rust
gate, complete passing aggregate and planted-defect evidence remain pending.

Independent AST review approved exactly five Boolean-branch normalizations in
`ObjCarryTriples` (two) and `ObjFoldAdmitted` (three): each semantic `if` becomes
an exhaustive `Bool.false`/`Bool.true` match with identical scrutinee and
respective branch bodies. Mechanical inverse transformation reproduces the
entire original module AST. Before adopting the change, four generic equations
proved both Boolean constructor cases for arbitrary success/error result
values of both affected return types. The attempted generic `cases Bool`
proof was rejected by the language's proof-case admission; the exhaustive
constructor equations then verified without axioms, with all six module kernel
replays successful, at attestation
`e77579e6b3c8ec506372ded38ffbb694104d030d8dfac1089ad8273723ab95ee`.

The normalized ObjectStore source SHA-256 is
`e6dd71f943f75623d1007b3655a8cf65c50fae864df2854cbf16071874922221`.
All four original control files, containing all 403 statements, remain
byte-identical. This source change enables direct constructor reduction in the
fixed simplification proof form; it adds no input shortcuts, hash caches,
unchecked assumptions or resource-limit changes. Refreshed complete construction
proofs and aggregate acceptance are still required.

The normalized carry proof compiled all three closed carry equations, but direct
unfolding of the recursive atom fold exhausted memory at Docker OOM event
1792577885. A proof-only decomposition then established the universally
quantified nil and cons equations of the actual `ObjFoldAtoms` definition.
Both equations verified without axioms, with all six module kernel replays,
at attestation
`900ce5512a82cc069e221fe49441ee844388b630856b00e61baffb54b8f317ff`.
No runtime implementation or original assertion changed.

Using those equations as explicit rewrites allowed the next complete 96-byte
attempt to compile every node hash, carry, full atom-fold equation, finalization
with the actual whole-object SHA-256 and preflight equation. The admitted-fold
proof was then rejected with `LLV7006` because its simplification list included
an unused empty-store admission theorem. That theorem is needed only by the
outer public fold. Removing the unused proof reference preserves the proposition
and all runtime behavior; refreshed aggregate verification remains pending.

After that unused reference was removed, the admitted-fold equation compiled
without warnings. The next, original public-observation module exhausted memory
at Docker event 1792579094, retained in
`/tmp/lexlean-r4-object-public-root-oom-20261021.jsonl`. The next controlled
proof-only decomposition adds the universal defining equation of the public
fold, then a closed equality for its full returned value, so the unchanged
original digest/root/count observations can rewrite that equality directly.
This is an instrument correction under the same limits; no runtime source or
original control statement changes. No complete construction receipt is claimed
until this entire chain, original observations, axiom audit and replay finish.

The controlled public-equation retry completed elaboration and reached the final
axiom audit, which rejected `objTrace96Admitted` with `LLV7005`: its observed
axiom set contained `propext`. This is a proof-policy failure, distinct from the
preceding memory failures; successful elaboration or kernel replay does not make
this an accepted construction proof. The exact emitted build is preserved at
`/tmp/lexlean-r4-object-propext-build-20261021`, build ID
`40b4bfc1678762927cac82c3a5071c3ec617b5a86907372c21e2b90ceb83a4df`.
The audit's first reported declaration does not alone identify whether the
axiom originated in that proof or an imported certificate. Small independent
simplification probes are staged to isolate that dependency. No original
statement, runtime source, axiom policy or compiler limit changed in this retry.

Three isolated, tiny proof-policy controls then separated the simplifier from
all hashing and object construction. A source module containing three literal
`rfl` equations passed with empty axioms at attestation
`3f428562e2cc43b8ab6ab3eaa0a3ff28fe9b3040108067a760e5ea9650bb70b4`.
Using one of those equations with the official `Simplify` proof form failed
`LLV7005` with observed `[propext]` in each independent setting: natural equality
under addition, Boolean equality under exhaustive match, and result equality
under constructor match. The preserved project, logs, source receipt and emitted
builds are in `/tmp/lexlean-r4-object-proof-policy-observed-20261021`.
This isolates simple equality rewriting through that proof form as a sufficient
cause of the policy failure, independently of the cryptographic dependencies.
It does not license changing the none-axiom policy or treating the unfinished
connected object certificates as accepted.
