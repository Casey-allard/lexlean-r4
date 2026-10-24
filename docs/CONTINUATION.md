# LexLean R4: current working plan and continuation

Updated 2026-10-24. This is the repository entry point for continuing the work
without the original chat or its temporary directories. The approved scope is
the [full design](specs/2026-10-21-lexlean-r4-design.md); the detailed phase plan
is [here](plans/2026-10-21-lexlean-r4.md). The model remains the authority for
registered claims. This page records work and evidence, not additional claims.

Casey requested that reviewed development checkpoints and this plan be merged
through PRs to `main`, with GitHub as the source of truth. Do not leave progress
only in a long-lived branch or local handoff. Merge does not mean production
acceptance, a release, or a completed implementation. Use this chat's internal
independent reviewers; no external lab or another session owns this work.

## What is implemented and what is proved

| Area | Recorded state | Repository evidence / next obligation |
| --- | --- | --- |
| Integrated development graph | Nineteen registered components; last recorded combined graph has 11,150 controls, 116 modules, 13,472 empty-axiom declaration audits and 116 kernel replays. Not a fresh whole-workspace test claim. | [Verification](VERIFICATION.md), [codebook](CODEBOOK-WORDS.md), model records and `CONFORMANCE.md` |
| HashWire | Historical 78-root baseline and 16 retained roots re-audited. Parent restoration session 70584 was running through 28/78 in the timestamped observation; full audit pending, mutation qualification 2/7. | [Recovery history](continuation/2026-10-24/hash-restoration-recovery/RESULTS.md), [running observation](continuation/2026-10-24/object-read-native-controls/restoration-observation.json). The earlier storage hold remains historical; no full restoration acceptance follows. |
| Object implementation | Five canonical modules reconciled to the reviewed development runtime; 331 of 403 Object controls independently verified through Store20 and Codec311 in pinned development closures. Component integration and qualification remain incomplete. | [Store results](continuation/2026-10-23/object-store-integration/RESULTS.md), [Codec results](continuation/2026-10-24/object-codec-integration/RESULTS.md), [original snapshot](continuation/2026-10-23/pending-manifest.json). Read33 and Construction39 account for the remaining 72 Object controls; The separate 20-control UInt/ByteArray experiment now has an [independently reviewed archival decision](continuation/2026-10-24/blake3-primitives-disposition/README.md); physical disposition remains pending and no controls are accepted by that decision. Full BLAKE3 qualification and required semantic negatives remain open. |
| Object Read33 integration | Canonical native candidate preserves all 33 names and propositions; explicit design, source and invocation reviews passed static readiness. All official native phases are NOT_RUN; no new Read control accepted. | [Source checkpoint](continuation/2026-10-24/object-read-native-controls/RESULTS.md), [preserved semantic timeout](continuation/2026-10-23/object-read-integration/RESULTS.md). |
| Parent-record decode certificate | Actual original49-byte parent record decodes to the complete expected record; false expected ID128→129 is rejected. Independent positive artifact audit:804 empty-axiom declarations and11 kernel replays. This is a local certificate, not a Read control or authenticated-fetch result. | [Decode results](../tests/fixtures/object-read-parent49/RESULTS.md), [connected-fetch design](../tests/fixtures/object-read-parent49/evidence/authenticated-fetch-spec-review.json). |
| Connected parent authenticated fetch | Actual `ObjAuthenticatedFetch` with the original four-entry backend, parent ID and scale1 equals the full expected record. Wrong-base proof rejected; positive artifact independently audited with809 empty-axiom declarations and14 kernel replays. Local fetch equation only; zero original Read33 controls discharged. | [Fetch results](../tests/fixtures/object-read-parent49-fetch/RESULTS.md). Original source-admission failure and exact two-field repair are preserved separately. |
| Parent visitor transition | Actual original `ObjVisitNode` equals its complete continuation with the certified parent record substituted. Wrong-base proof rejected; positive independently audited:814 empty-axiom declarations and17 kernel replays. All three child calls remain; no final walked state or Read33 acceptance. | [Visitor results](../tests/fixtures/object-read-parent49-visit/RESULTS.md). |
| Concrete parent accounting and child entry | Actual parent state is empty output, one visit, 49 wire bytes and the original parent ID in seen. The full original visitor equals its three-child continuation with this state; false expected wire 50 is rejected. Independent audit: 816 empty-axiom declarations, 19 kernel replays. | [Accounting results](../tests/fixtures/object-read-parent49-accounting/RESULTS.md). At that checkpoint, child results remained open; the later child and complete-parent rows record the completed local walk. |
| First actual child visit and parent continuation | The original first leaf authenticates and returns its 32-byte payload, two visits, 94 wire bytes and the sorted parent/child seen IDs. False expected wire 95 is rejected. The original parent visitor equals the continuation with both later child calls retained. Independent audit: 818 empty-axiom declarations and 21 kernel replays. | [First-child results](../tests/fixtures/object-read-first-child/RESULTS.md). At that checkpoint, two later child results remained open; see the complete-parent row for current progress. Original Read33 remains open. |
| Second actual child visit and parent continuation | Authenticated 64-byte output, three visits, 139 wire bytes and sorted parent/first/second IDs; false expected wire count 140 rejected. Mandatory original parent retains final third-child call. Independent audit: 820 empty-axiom declarations, 23 kernel replays, 204,491 ms. | [Second-child results](../tests/fixtures/object-read-second-child/RESULTS.md). At that checkpoint, the final third-child result remained open; see the complete-parent row. Whole-data hashing was open at that checkpoint; see the later accepted digest and complete post-range result. At that checkpoint the complete outer whole-read Result remained open; see the later installed-format native result. Canonical ordinal 26 and Read33 remain open. |
| Complete original parent visitor | Final authenticated child returns S184: 96 original output bytes, four visits, 184 wire bytes and sorted third/parent/first/second IDs. False expected wire count 185 is rejected. The full original parent equation returns Result.ok S184 with no child call remaining. Independent audit: 822 empty-axiom declarations, 25 kernel replays, 212,087 ms. | [Final-child results](../tests/fixtures/object-read-final-child/RESULTS.md). At that checkpoint root traversal, range finish and manifest/version remained open; see the later rows for their subsequent proofs. Whole-data hashing was open at that checkpoint; see the later accepted digest and complete post-range result. At that checkpoint the complete outer whole-read Result remained open; see the later installed-format native result. Canonical ordinal 26 and Read33 remain open. |
| Original singleton-root traversal | Actual original ObjVisitRoots returns full Result.ok S184 with original manifest projections, backend, range and limits. False expected wire count 185 is rejected. Independent audit: 265 files, 823 empty-axiom declarations, 26 kernel replays; aggregate positive verification 421,070 ms. | [Root-traversal results](../tests/fixtures/object-read-root-traversal/RESULTS.md). At that checkpoint the connected range equation remained open; see the next row. See later version and contextual-finish rows; the later admitted-range result establishes composition and the later digest/post-range checkpoints establish local hashing and comparison. The outer admission equation is accepted in the later outer-shell checkpoint; the complete outer Result was open at that checkpoint; see the latest installed-format native result for its local proof. Canonical ordinal 26 and Read33 remain open. |
| Actual connected range equation | The original ObjRangeAdmitted call equals the genuine ObjRangeFinish call on S184; actual consumer Apply1 is kernel checked. Wrong-base negative rejected the transport dependency. Independent audit: 295 files, 828 empty-axiom declarations, 29 kernel replays; aggregate positive verification 570,901 ms. | [Connected-range results](../tests/fixtures/object-read-connected-range/RESULTS.md). At that checkpoint no concrete finish or version result followed from the equation; see the version row for subsequent progress. The later contextual-finish row records the concrete finish proof; see the admitted-range result row for subsequent successful composition. |
| Original manifest-version result | Full original ObjManifestVersion returns the exact ObjVersion through real admission, encoding and HashWire hashing; false expected first byte 147 is rejected. Independent audit: 85 files, 740 empty-axiom declarations, 8 kernel replays; aggregate positive verification 38,752 ms. | [Version results](../tests/fixtures/object-manifest-version/RESULTS.md). Fifteen inputs supplied 11 source files; 8 modules were selected and verified. At that checkpoint concrete finish remained open; see the contextual-finish row. Whole-data hashing was open at that checkpoint; see the later accepted digest and complete post-range result. At that checkpoint the complete outer whole-read Result remained open; see the later installed-format native result. Canonical ordinal 26 and Read33 remain open. |
| Concrete range-finish direct proof | Full-result wrong-wire phase passed source admission for 9 selected modules from 11 supplied source files, then timed out in target Lean at 300,013 ms against the unchanged 300,000 ms limit. Aggregate verification 425,362 ms; no artifact, intended negative not established, positive NOT_RUN. | [Failure checkpoint](../tests/fixtures/object-concrete-range-finish/RESULTS.md). This configuration remains unaccepted. The distinct contextual successor is recorded in the next row. |
| Contextual complete range finish | The complete original ObjRangeFinish Result is verified; expected wire statistic 185 is rejected. Independent audit: 125 files, 776 empty-axiom declarations, 12 kernel replays; positive aggregate 49,645 ms. | [Contextual-finish results](../tests/fixtures/object-contextual-range-finish/RESULTS.md). The subsequent admitted-range result is accepted; whole-read obligations remain open. |
| Original admitted-range full Result | The actual original ObjRangeAdmitted call returns all 96 bytes, exact version, range, empty remainder and statistics 4/4/184. The intended full-consumer negative is rejected; successful positive conversion is separately kernel checked. Independent audit: 135 files, 777 empty-axiom declarations, 13 kernel replays; aggregate 289,504 ms. | [Admitted-range results](../tests/fixtures/object-admitted-range-result/RESULTS.md). Data hashing and digest comparison were open at that checkpoint; see the later accepted local digest and complete post-range result. The outer admission equation is accepted in the later outer-shell checkpoint; the complete outer Result was open at that checkpoint; see the latest installed-format native result for its local proof. Canonical ordinal 26 and Read33 remain open. |
| Original whole-read continuation attempt | Intended transport negative rejected; sourcecheck18 passed, but the actual whole-read consumer timed out at300004ms. Memory max450159 with oom0/oom_kill0; no artifact or continuation acceptance. | [Failure checkpoint](../tests/fixtures/object-whole-read-continuation/RESULTS.md). Preserve the nonzero resource epoch; no equivalent limit retry. The subsequent standalone digest proof does not repair this failed connection. |
| Genuine projected whole-data digest | The production ObjHashBytes call on the original range record's96 projected bytes returns the complete expected digest Result. Expected first byte201→202 is rejected at the mandatory consumer. Positive independently audited:744 empty-axiom declarations,11 kernel replays,115 artifact files,113189ms. | [Digest reuse results](../tests/fixtures/object-whole-digest-reuse/RESULTS.md). The old outer whole-read continuation remains failed; the later outer-shell checkpoint proves admission/backend conversion, while the complete outer Result was open at that checkpoint; see the latest installed-format native result for its local proof. Canonical ordinal 26 and Read33 remain open; the post-range comparison/wrapping is accepted in the next checkpoint. |
| Original whole-read full Result | Exact original concrete four-record ObjWholeRead returns the full successful record with 96 bytes, version and statistics 4/4/184. Intended unrelated-proof negative rejected at the mandatory native root. Positive independent audit: 285 files, 827 empty-axiom declarations, 28 kernel replays and all run gates; 363,107 ms aggregate. | [Installed-format native results](../tests/fixtures/object-whole-native-format/RESULTS.md). Canonical semantic ordinal 26 and Read33 remain open because native-to-semantic proof import is not supported by the installed compiler. |
| Whole Object257 digest | Original reflexivity proof timed out at 300 seconds; the 900-second experiment exhausted the 4 GiB container. No accepted whole257 result. | [300s result](continuation/2026-10-23/evidence/original257-timeout300.json), [OOM review](continuation/2026-10-23/evidence/original257-timeout900-review.json). Do not repeat equivalent limit increases. |
| Final partial leaf and reuse | Actual leaf certificate, direct Apply, parent context and inventory-staging transport verified, with intended negative controls. | [Finite source fixture](../tests/fixtures/object257-leaf-transport/README.md), [results](../tests/fixtures/object257-leaf-transport/results.json) |
| Terminal fold transition | Actual `ObjFoldAtoms [[0]] frontier inventory limits` transition verified for arbitrary incoming state. Certified leaf staging, depth-38 carry, same leaf ID, root scale 0, errors and empty-tail fold are retained. | Commit `0e3544a85454276394f58344dfc93772dcd4c444`; [native review](../tests/fixtures/object257-leaf-transport/evidence/terminal-fold-native-review.json): 769 empty-axiom declarations, 11 kernel replays, positive 23,010 ms; wrong-base negative rejected. |
| First concrete Object257 atom state | Actual first32-byte leaf and `ObjFoldAtoms [[0,1,...,31]] [] [] originalLimits` singleton state verified. Wrong expected inventory ID rejected at the consumer. | [Actual results](../tests/fixtures/object257-first-atom/RESULTS.md): leaf 66,570 ms; state 48,659 ms, 769 empty-axiom declarations and 12 kernel replays. Local prefix only. |
| Nonempty two-atom continuation | Actual `Fold [A,B] [] [] L = Fold [B] firstState.frontier firstState.inventory L` verified, with A bytes0–31 and B bytes32–63. Unrelated-base proof rejected. | [Results](../tests/fixtures/object257-nonempty-continuation-canonical/RESULTS.md): positive 93,920 ms, 774 empty-axiom declarations and 14 kernel replays. Actual consumer conversion passed; no second-state, whole257 or efficiency claim. |
| Original257 atomized fold-core probe | Sourcecheck admitted 14 modules; negative phase timed out in its final Lean module at the unchanged 300-second child limit. Positive not run. | [Timeout record](../tests/fixtures/object257-atomized-core/RESULTS.md). This is not the intended proof negative or an accepted core bridge. |
| Split atomized continuation | Intended negative and positive helper independently verified; mandatory actual consumer failed with a recorded cgroup OOM kill. Bridge remains unaccepted. | [Split results](../tests/fixtures/object257-atomized-split/RESULTS.md): helper773 empty axioms/14 replays; consumer resource failure, no semantic counterexample. |
| Symbolic second-atom continuation | Intended wrong-base negative, parameterized lift, generic production consumer and exact original concrete specialization independently verified. | [Actual results](../tests/fixtures/object257-symbolic-continuation/RESULTS.md): concrete775 empty axioms/16 kernel replays,25,240ms. Exact original statement retained. First-state continuation only, not full257 digest or Object acceptance. |
| Production binding and deployment | Missing production SDK/template binding. `just vv`, generated application and Pages remain incomplete. | [Upstream dependencies](UPSTREAM.md), CI observation below. |

The local Object proof experiments can proceed before full Hash acceptance.
That permits diagnostic source/proof development; it does not promote Hash or
Object into production conformance. No training, GPU qualification or language
capability follows from these finite proofs.

## Next executable product sequence

**Hash restoration history and running observation:** The [recovery checkpoint](continuation/2026-10-24/hash-restoration-recovery/RESULTS.md) recovered all seven original mutant sources byte-identically and audited the historical 78-root/636-control baseline and 16 retained roots. Its reviewed successor had not executed at that checkpoint, and its storage hold remains historical evidence. After separate eligibility checks, the parent launched restoration session 70584. The [timestamped parent observation](continuation/2026-10-24/object-read-native-controls/restoration-observation.json), recorded at 2026-10-24T08:02:52.965Z, reports progress through HashWireCross15Controls (28/78), still running with no terminal runner record. This is not final restoration acceptance or an independent artifact audit; mutation qualification remains 2/7. Fresh operational gates, the 20 GiB launch floor, 10 GiB reserve and unchanged per-root 11/10 GiB gates remain required. No Read native phase may overlap that job.

**Current Read33 source checkpoint:** [Reviewed native source and invocation](continuation/2026-10-24/object-read-native-controls/RESULTS.md) preserve all 33 original qualified names and propositions in supported core-module/1. The explicit design review revises the earlier metadata-only restriction for this module; source and invocation CODE reviews establish static readiness only. All official Read native phases remain **NOT_RUN**, so accepted controls remain 331/403. The parent must enforce independent focused-negative review, focused-positive review, then the mandatory full 33-control phase after fresh eligibility; the one-phase runner does not enforce prior-phase history. Earlier semantic timeouts and the nine pending drafts remain unresolved. Native-to-semantic proof imports are still unsupported; this candidate uses the separately reviewed native representation without changing the runtime or original goals.

**Current Codec311 checkpoint:** [All 311 canonical controls now pass](continuation/2026-10-24/object-codec-integration/RESULTS.md) in the pinned development closure: eight selected modules, 1,050 empty-axiom declarations, eight kernel replays and 78,702 ms aggregate. The actual legacy bool-version admission negative is preserved. Together with Store20 this is 331/403 existing Object controls with local root verification; Read33+Construction39 are the remaining 72. Nine original drafts remain unfinished after this delivery. Direct native-to-semantic proof imports remain unavailable. The reviewed native Read33 candidate above is not yet kernel-verified; Construction39 includes the exact known failed whole257 obligations. Any official checked proof dependency or remedy requires separate review, not an unchanged retry or new limits. Full Hash/Object/model/scenario/PrismPM/Pages obligations remain open. Historical counts and NOT_RUN statements below describe their original checkpoints; this paragraph and the linked actual results give current Codec status.


The preceding [installed-format native result](../tests/fixtures/object-whole-native-format/RESULTS.md) independently proves the exact original 96-byte/four-record whole-read full Result, with 827 empty-axiom declarations, 28 kernel replays and all run gates passing. Canonical ordinal 26 and Read33 remain open: installed LexLean 0865 excludes imported native theorems from the semantic proof environment. The [source investigation](../tests/fixtures/object-whole-native-format/evidence/next-ordinal26-investigation.json) is not executable authorization. The [independent sequencing SPEC](../tests/fixtures/object-whole-native-format/evidence/next-integration-spec-review.json) explicitly permits bounded Codec311 compatibility work before Read33 completion. Preserve all 311 declarations while changing only the three reviewed metadata literals. At that design checkpoint the proposed closure had 15 inputs, 11 supplied and eight statically reachable modules, with tests NOT_RUN. The later Codec checkpoint above records written source/invocation review and actual acceptance of all 311 controls under fresh operational gates. This adjusts scheduling without waiving Read33 or interoperability.

The immediate next work is the 17-file draft integration package requested by
Casey. The atomized first-state bridge and existing20-control Store root are now verified. Complete the actual
Object control roots and their dependencies, then reconcile the model, scenarios
and documentation; retain the remaining whole257 obligation within that work.
The [source-only inventory](../tests/fixtures/object257-symbolic-continuation/evidence/draft-integration-next-step.json)
identifies 403 Object controls (Codec311, Store20, Read33, Construction39) and
20 separate Blake3 primitive controls. The runtime compatibility changes and
three-literal Store control metadata migration passed independent source review.
The original control produced the intended language-version admission rejection;
the migrated root then verified all20 controls with unchanged statement/proof ASTs.
This compatibility negative is not a semantic defect test. The [actual results](continuation/2026-10-23/object-store-integration/RESULTS.md)
record823 empty-axiom declarations and11 kernel replays. Read33 source migration
is now reviewed, but its aggregate verification timed out as recorded below;
retain the whole257 obligation and required Hash qualification. Do not repeat the known whole257 reflexivity
timeout/OOM configuration. The earlier source-only inventory remains historical.
The [remaining-controls proposal](continuation/2026-10-23/object-store-integration/remaining-controls-proposal.json)
pins the minimal metadata migrations and proposed Read33 followed by Codec311.
Codec311 was NOT_RUN at that proposal checkpoint; its later full-root acceptance is recorded above. Read33 retains the preserved timeout below.
Construction39 still contains the known whole257 reflexivity
obligations and must not be replayed unchanged as an equivalent limit experiment.
At that earlier inventory checkpoint, the separate Blake3 primitive suite imported
a missing canonical `R4.Blake3Primitives` module. The later [reviewed disposition](continuation/2026-10-24/blake3-primitives-disposition/README.md)
identifies it as a rejected optional representation already superseded by the Nat
design. Its physical archival action remains pending Read33; restoring that failed
wrapper is not the current implementation task. Full BLAKE3 requirements remain open.

Read33 has now been attempted: its exact three metadata edits passed independent
source review and actual source admission, but the complete root timed out in
Lean. The earlier NOT_RUN proposal is historical, not the current Read result.
The [Read failure review](continuation/2026-10-23/object-read-integration/positive-failure-review.json)
confirms no verified artifact, zero recorded memory events, unchanged identities
and successful postchecks. Do not rerun the same aggregate or raise its limits.
At the Read migration checkpoint, seven original draft files had intentional
integration changes and ten remained unchanged. After Codec integration, eight
have intentional changes and nine remain unfinished; all seventeen original
snapshots remain preserved. This counts source integration, not completed component
obligations. Store20 and Codec311 now have accepted complete canonical roots in
their pinned development closures. Bounded connected
Read proofs now establish all three child states and the complete original parent
visitor equation. The singleton-root traversal and actual connected range
equation also pass. The original manifest-version full Result is now independently verified.
The direct concrete range-finish negative timed out without establishing the intended
rejection. Its distinct contextual successor now proves the complete original finish Result; successful admitted-range composition is now independently verified.
All 33 Read declarations and their eventual integration remain mandatory.
The [source investigation](continuation/2026-10-23/object-read-integration/timeout-investigation.json)
and [independent specification review](continuation/2026-10-23/object-read-integration/timeout-investigation-spec-review.json)
select only the first diagnostic: isolate exact ordinal26, `objProtocolWholeBytes`,
with the same complete declaration AST, module, path, imports, runtime and limits.
The independently reviewed extraction and invocation have now run. Source admission
passed11 modules, but the exact singleton also timed out: target Lean300010ms
against the unchanged300000ms limit, whole command341805ms. No artifact was
produced; all15 inputs, lock, tools, epoch, zero memory events and post-reserves
passed independent audit. See the [isolated result](continuation/2026-10-23/object-read-isolated26/RESULTS.md).
The other32 declarations are therefore not necessary for this configuration to
exceed its limit; this does not locate the earlier aggregate stopping point or
identify an expensive subterm. All 33 controls remain unaccepted. The original semantic
source is preserved byte-exactly; the current canonical source is the reviewed native
candidate described above, preserving all 33 names and propositions. No limits changed.
The source investigation follows the actual authenticated-record, range-walk,
manifest-version and whole-data hashing chain. The parent49 decode certificate
now passes as recorded above. The authenticated-fetch and original parent visitor
transition also pass. Concrete parent accounting and entry into the original
three-child continuation now pass as well. All three authenticated child results
and the complete original parent visitor now pass. The singleton-root traversal
also passes. The connected range equation now retains actual outer validation
and planning through to the real finishing call. The specific original manifest-version
computation now passes. Concrete range success, genuine whole-data hashing and the complete post-range Result now pass at the later checkpoints below. Later native composition now proves the exact local whole-read full Result; canonical ordinal 26 and the original Read33 controls remain open.

1. **Verified checkpoint: first state connected to the original atomized fold core.**
   The [two-atom continuation](../tests/fixtures/object257-nonempty-continuation-canonical/RESULTS.md)
   now passes actual official verification and independent artifact review.
   Its concrete consumer establishes the nested-match conversion, beyond the K
   helper. The unrelated `0 = 0` zero-base negative is rejected as intended.
   An earlier source-admission failure came from three noncanonical JSON objects;
   the isolated key-order repair preserved all parsed semantics and both controls.
   Both the rejected source and actual failures remain in the repository.
   The next bounded task is the independently reviewed
   [original atomized-core proposal](continuation/2026-10-23/atomized-core-proposal.json)
   and [source SPEC review](continuation/2026-10-23/atomized-core-spec-review.json):
   connect the actual `ObjAtomize original257 [] 0` fold core to the remaining225-byte
   suffix from the certified first state, without trusted intermediate hashes.
   Candidate construction must take S from the accepted state theorem's
   `right.arguments[0]` (the ObjCarryState); `right` itself is already `ok S`.
   Do not double-wrap it. The [candidate and actual timeout](../tests/fixtures/object257-atomized-core/RESULTS.md)
   are now preserved: source admission passed, but the negative module exceeded
   the 300,000 ms child limit (observed 300,006 ms). The intended unrelated-base
   diagnostic was not returned. The positive remains **NOT_RUN**, and the larger
   consumer/atomization-prefix conversion remains unproved. The exact expensive
   declaration or conversion is not localized by this result. Inspect the retained
   build emission. The reviewed [three-root split](../tests/fixtures/object257-atomized-split/RESULTS.md)
   returned the intended negative and verified the positive helper, but the mandatory
   actual consumer failed with generic LLV7002 and a recorded container OOM kill.
   That split's bridge remains unaccepted and its failed resource epoch is preserved.
   The separately reviewed symbolic successor abstracts only the complete second
   atom as a list parameter, then specializes the generic production consumer at
   the original bytes32–63. Runtime, original statement, digest and limits remain
   unchanged. It uses a separately reviewed fresh container epoch, preserving the
   earlier OOM evidence. The negative, lift and generic consumer passed independent
   native review. The concrete specialization also passed independent artifact
   review:775 empty-axiom declarations and16 kernel replays, with the entire
   original theorem statement unchanged. This accepts the atomized first-state
   continuation bridge; the final digest and subsequent concrete states remain open.
   The suffix uses the original bounded ObjAppendBytes tree minus its first32-byte
   leaf, with all225 bytes checked. That source representation refinement was
   independently reviewed and does not assert an atomization result.
   Require the actual consumer, unchanged limits and intended wrong-base control;
   helper-only success is insufficient. Possible hash recomputation remains open.
   Frozen source-preparation NOT_RUN text remains historical; current fixture
   RESULTS.md/results.json records the later outcomes.
   After this bridge checkpoint, the next work package is completion of the 17
   preserved drafts requested by Casey. Begin with their actual Object control
   roots and source reconciliation, rather than extending verification tooling.
2. **Complete the connection to the original theorem.** Preserve the original
   input, digest, runtime definitions and empty-axiom policy. Demonstrate a
   supported proof bridge before expanding all thirteen node certificates.
   Direct proof `Cases` on built-in `Result` is unsupported in the inspected
   compiler; do not assume that bridge exists. The accepted Nat-family,
   Induction/Cases/Congruence/Apply pattern establishes the recorded local
   transitions only. Atomization, remaining transitions, success/error continuation
   composition and whole-object SHA still need proofs.
3. **Finish Hash qualification and Object integration.** Restore the remaining
   HashWire roots and independently inspect actual artifacts and baseline
   identities; complete the remaining mutation controls. Verify all 403 Object
   controls, their required defects and restoration before adopting draft source,
   model records and scenarios into the active graph. Regenerate conformance
   through the official applicable toolchain.
4. **Complete the approved product.** Finish codebook construction, integer
   artifact export/serving, durable memory and runtime custody, learning
   constraints, named application roots/effects and View/System composition.
   Resolve official compiler dependencies and production PrismPM/template
   binding, then generate the browser application and deploy GitHub Pages.

Keep source changes small, prove the relevant negative and positive with the
real compiler, obtain an internal non-author review of source and actual
artifacts, then push atomic conventional commits and merge reviewed checkpoints
through PRs. Update this page when the next action, blocker or result changes.
Do not restart the abandoned general verification-harness integration project.

Casey explicitly requested that the 17 preserved unfinished drafts be finished
after the current atomized-fold proof work. Treat them as the next integration
work package: five Object modules, five control suites, three feature scenarios,
three model records, and Object documentation. Resolve the actual verification
failures and required Hash/Object qualification, run all 403 Object controls and
required negatives, reconcile the PrismPM model and scenarios, and merge reviewed
commits to main. The archived snapshot preserves provenance; it is not completion
or permission to promote unverified drafts into the active graph.
At the Store checkpoint, six source files from that package were reconciled and
delivered: the five Object modules and Store controls; eleven drafts then remained
unchanged. Read metadata and Codec integration now bring the intentional changes
to eight, leaving nine unfinished drafts. Component-level obligations, model registration,
feature scenarios, documentation and full403-control qualification are pending.

## Portable source and evidence map

The [finite proof fixture](../tests/fixtures/object257-leaf-transport/README.md)
contains the exact development runtime closure, proof sources, phase configs,
genuine development locks, compiler receipts, selected emitted Lean and internal
reviews. Its manifest inventories all payload bytes. The repository relocation
was byte-verified, not rerun. The default fixture project still selects the parent
transport phase; select `phases/terminal-positive` for the latest accepted proof.

The dated [continuation packet](continuation/2026-10-23/manifest.json) preserves:

- `pending/`: the 17 unfinished working files, byte-identical to their recorded
  hashes. These are inert drafts outside the active source roots and model.
  `pending-manifest.json` gives each destination path, hash and tracked base hash.
  Compare before applying anything in a clean owned checkout; never overwrite
  differing work or install the draft model merely to make a check pass.
- `reference/ObjectTrace257Node0Controls.lex.tex` and
  `reference/ObjectTrace257Carry0Controls.lex.tex`: the original Record0/Carry0
  statements used by the next proposal. Their old proof text is retained as
  reference, not claimed as a newly verified certificate.
- `original257/ObjectNatDigest257Controls.lex.tex` and `lexlean.toml`: the
  original diagnostic target/configuration. Use the finite fixture's matching
  runtime closure if reconstructing it; generate a genuine development lock.
- Original failure receipts/reviews, the next proposal and the CI diagnosis.

Historical JSON contains original absolute paths for custody. For the next proof,
use the relative files above and the fixture's `project/` files; rehash them
instead of depending on `/tmp` existing. New source, tests and status must be
recorded in Git, not solely in a chat or local packet.

Complete historical compiler bundles are not all committed. Reviews and
attestation identities are historical evidence, not substitutes for those raw
bundles. If a new machine cannot retrieve the exact artifacts, rerun the required
proofs and audit the new artifacts; do not silently count missing restored roots
as accepted. In particular, resuming 16/78 Hash restoration requires the retained
artifacts and exact baseline identity checks, or a newly recorded full run.

## Development toolchain and execution

See [UPSTREAM.md](UPSTREAM.md) for acquisition/build identities. The recorded
development compiler SHA-256 is
`107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`,
semantics `09f4b95d5994ca00acd413e6f9e8381c8a18d4fd778249d2c8654f7dc99dce86`,
Lean `4.32.1`. These are development identities, not a production SDK binding.
Read the fixture reviews for exact Lean/Lake/checker identities and limits.

In a fresh owned workspace, copy the fixture's `project/` source, select a phase
configuration and run the official compiler sequentially:

```sh
lexlean --diagnostic-format json lock
lexlean --diagnostic-format json check
lexlean --diagnostic-format json verify tests/lexlean/Object257TerminalFoldTransition.lex.tex
```

Retain the genuine lock and all source/tool hashes, raw result, observed axioms,
generated artifact census and kernel replays. Run the intended false companion
as a separate phase; an OOM, timeout or source-admission error is not an accepted
semantic negative. Never manufacture `prismpm.lock` or edit proof acceptance.

The existing host's reusable VM is `lexlean-r4` (5 GiB), with the existing
`lexlean-r4-current-sdk` container (4 GiB, 2 CPU, network none, no mounts).
At this checkpoint no SDK job is active. Treat that as a dated observation:
inspect processes before launching, preserve healthy jobs and allow only one
SDK/compiler job. Reuse the installed compiler and bounded caches.

Before native phases record fresh host/guest available and allocated storage,
including paging in the peak estimate. Keep 10 GiB after the estimate, use the
20 GiB conservative launch planning floor, and retain 11 GiB pre-root / 10 GiB
post-root gates. Investigate changes over 2 GiB. Small bounded host work may
continue while its reserve holds. Never touch unrelated work or processes.

## Known CI failure and checkpoint merge boundary

[Run 39993043946](https://github.com/Casey-allard/lexlean-r4/actions/runs/39993043946)
failed the inherited template gate on both runners. The inspected amd64 job log
confirms `sed: can't read prismpm.lock: No such file or directory`, then exit 2,
before Docker or `just vv`. This upgrades the earlier source-only inference to
a log-confirmed observation for that job. See the [dated record](continuation/2026-10-23/ci-observation.json).

The user's requested merge preserves development progress on `main` while this
production gate remains red. It does not waive `just vv` for a release. No CI gate
is disabled and no production lock is fabricated. The implementation checkpoint
(PR #3) did not change universal files; the subsequent Actions dependency update
is described below.
Resolve the real immutable SDK/template dependency before claiming product
acceptance or deployment. The inherited template and production pipeline remain
the acceptance boundary.

## Merged development checkpoints and dependency maintenance

PR #3 merged the implementation, finite proof fixtures and portable continuation
packet into `main` at `9456614d9fe13ef56a736f69e7cda71a5fae6db9`. The merged
tree matched the reviewed head, all 80 finite-fixture and 31 continuation payloads
were rehashed from fetched `origin/main`, and the original 17 unfinished working
files remained unchanged. This was tree/custody verification, not a fresh `just vv`.
The old implementation branch was deleted after merge.

[PR #1](https://github.com/Casey-allard/lexlean-r4/pull/1) merged at
`5389a4248fd2c768ef3bf8c47c67527adad971b9` after internal infrastructure/security
review of all five immutable Action pins and 31 substitutions across two workflows.
Inputs, permissions, conditions and shared PrismPM calls were unchanged. These
universal workflow updates still require genuine template-policy reconciliation
before production acceptance; the merge is a development checkpoint. The existing
`.template-update.patch` upload lacks `include-hidden-files: true` and remains a
separate inherited template-update defect, not a new defect from the pin update.

Dependabot replaced closed PR #2 with [PR #4](https://github.com/Casey-allard/lexlean-r4/pull/4).
Its original head `c5b5c0e4e5fe3f1546f849994c351b27bba121e8` passed library and
xtask tests but failed all-target compilation: sha2 0.11 digest outputs no longer
implement `LowerHex`. The compatibility repair renders each SHA-256 byte as two lowercase hexadecimal
digits in shared test support and preserves all expected digest constants. The
dedicated empty/abc/leading-zero regression, four library tests, 14 xtask tests
(including two nested JavaScript tests), all-target/all-feature compilation,
model equality and dependency-update audit passed with Rust 1.97.1. These scoped
host checks do not execute LexLean proofs or establish production acceptance.
The original compilation failure is preserved. Internal non-author Rust/code and
security reviews passed for repair commit `14aaa92219a57a0781e50570a305890193303e3d`.
A planted leading-zero-loss defect failed the dedicated regression and the exact
restored formatter passed. Merge remains a development checkpoint under the same
production binding boundary.

The [dependency-maintenance evidence](continuation/dependency-refresh-2026-10-23/manifest.json)
contains the exact initial failures, restored host results, source pins and
internal reviews. Absolute paths inside historical receipts identify the original
execution; the adjacent files and manifest preserve their bytes on GitHub.

PR #4 merged at `31ec155d064cdf70c1db1aa01d18ac744b6d74c6`; the dependency
branch was deleted. All six scoped host checks above passed again on that fetched
main revision. [Historical product work](continuation/product-history-2026-10-23/README.md)
preserves the additional 324 small text contents found in the bounded local
inventory, with source/alias mappings and exact hashes. Active product drafts
and all 24 inspected candidate/evidence LexLean source occurrences already matched
committed bytes. This archive preserves provenance; it adds no accepted capability.

### Local-history reconciliation

The [earlier source inventory](continuation/earlier-history-2026-10-23/README.md)
adds 1,821 deduplicated historical text payloads (60,597,422 original bytes),
covering 3,729 source/alias locations across 83 named project roots. It compares
exact committed files and decoded source strings before archival. The
[large JSON archive](continuation/large-history-2026-10-23/README.md) closes all
14 large-inventory exclusions with seven losslessly compressed contents
(99,221,243 original bytes). All eight inspected large Engine/Words source copies
already matched main. Original and compressed hashes are recorded separately.

These archives preserve old source versions, rejected candidates, negative
controls, plans and observations. Twenty upstream compiler excerpts are labeled
historical authority references. They do not install dependencies, revive the
abandoned harness work or establish new acceptance. Current product development
continues with the bounded continuation bridge described above.

This reconciles the enumerated project source/plan locations, not every local
file. Raw compiler bundles, build/VM caches, compiler checkouts and unrelated
plugin changes remain excluded and preserved locally; acceptance cannot rely on
missing raw artifacts. The inventories state their traversal and comparison
boundaries. No original local work was deleted during preservation.

The reviewed parent49 decode step has now passed actual negative/positive
verification and independent artifact review. See the [decode checkpoint](../tests/fixtures/object-read-parent49/RESULTS.md):
wrong expected ID rejected in35,529ms; exact positive verified in268,903ms.
Production input, runtime and empty axioms remain unchanged. The connected
[authenticated-fetch proof](../tests/fixtures/object-read-parent49-fetch/RESULTS.md)
now passes: the intended wrong-base proof is rejected in25,138ms and the actual
full consumer verifies in43,852ms. The original candidate failed source linking
before verification; removing exactly two self-module qualifiers repaired local
references without changing runtime, proof terms, configuration or limits.
Independent review checked145 artifact files,809 empty-axiom declarations and14
kernel replays, including the original backend, parent ID, scale1 and full Result.
This local equation does not discharge any original Read33 control. The
[original parent visitor transition](../tests/fixtures/object-read-parent49-visit/RESULTS.md)
now also passes: wrong-base rejection in37,832ms and actual positive verification
in120,377ms. Independent review checked175 artifact files,85 build outputs,814
empty-axiom declarations,52 successful process records and17 kernel replays.
The exact production successor branch retains all three recursive child calls,
error propagation, accounting and checked offsets. The equation substitutes the
certified fetch result into that continuation; it does not assert the completed
walked state or child results. All21 inputs, genuine development lock, tools,
resource epoch and postchecks passed, with zero memory events.
The [concrete parent accounting and child-entry step](../tests/fixtures/object-read-parent49-accounting/RESULTS.md)
now passes too: the false expected wire count 49→50 is rejected in 10,533 ms, and
the correct state plus mandatory full consumer verifies in 136,745 ms. Independent
review checked 195 artifact files, 95 outputs, 816 empty-axiom declarations, 58
successful process records and 19 kernel replays. All 23 inputs and operational
postchecks passed with zero memory events. The concrete incoming state has empty
output, one visit, 49 wire bytes and the original parent ID in seen.
The consumer directly applies the accepted visitor theorem through definitional
conversion; it does not rewrite with the accounting certificate or establish
cached computation. All three actual child calls remain in its continuation.
The [first actual child visit](../tests/fixtures/object-read-first-child/RESULTS.md)
now also passes: the false wire count 94→95 is rejected in 20,606 ms; the positive
child state and mandatory original-parent consumer verify in 180,381 ms.
Independent review checked 215 artifact files, 105 outputs, 818 empty-axiom
declarations, 64 successful process records and 21 kernel replays. All 25 inputs
and operational postchecks passed with zero memory events. The state contains
the actual 32-byte leaf output, two visits, 94 wire bytes and both sorted seen IDs.
The original-parent consumer retains the two later child calls. Direct Apply
uses definitional conversion; the child certificate is not a rewrite or cache.
The initial unsupported equality tag was corrected before native execution;
the original preparation and exact correction are preserved separately.
The [second actual child checkpoint](../tests/fixtures/object-read-second-child/RESULTS.md)
now passes: intended wire count 139→140 rejection took 28,220 ms; actual positive took
204,491 ms. Independent audit checked 235 artifact files, 115 outputs, 820 empty-axiom
declarations, 70 successful processes and 23 kernel replays. All 27 inputs and
source/lock/tools/epoch/memory/storage postguards passed. S139 contains the
original 64 output bytes, three visits, 139 wire bytes and sorted parent/first/second
seen IDs. The mandatory unchanged original parent retains the final third-child call,
checked offset and errors. Direct Apply establishes actual definitional conversion;
no child certificate rewrite, cache or efficiency claim follows.
The [final-child checkpoint](../tests/fixtures/object-read-final-child/RESULTS.md)
now passes the final authenticated child S139 → S184 and the full original parent
visitor returning Result.ok S184. The intended expected wire count 184→185 negative
is rejected in 15,004 ms; positive verification took 212,087 ms. Independent audit
checked 255 artifact files, 125 outputs, 822 empty-axiom declarations, 76 successful
processes and 25 kernel replays. All 29 inputs and operational postguards passed.
The actual 96 bytes retain traversal order; the seen IDs are sorted as third,
parent, first and second. No child visitor remains on the equation's right side.
Direct Apply establishes actual definitional conversion, not a certificate rewrite,
cache or efficiency claim.

The [root-traversal checkpoint](../tests/fixtures/object-read-root-traversal/RESULTS.md)
now establishes the exact original singleton-root consumer returning Result.ok S184.
The intended false wire count 185 is rejected; negative verification took 550,982 ms
in aggregate. Positive verification took 421,070 ms in aggregate. Independent
review checked 265 artifact files, 130 outputs, 823 empty-axiom declarations,
79 successful processes and 26 kernel replays. All 30 inputs, genuine lock, tools,
epoch, zero memory events and independent postguards passed. These durations cover
the imported closure; no separate final-module duration, cache or efficiency claim
is made.

The [connected-range checkpoint](../tests/fixtures/object-read-connected-range/RESULTS.md)
now proves the actual original ObjRangeAdmitted equation equals the real
ObjRangeFinish call on S184. The consumer applies the checked contextual transport
at Nat1; all original checks, planner and error branches remain. The wrong-base
negative rejected the transport dependency in 403,160 ms aggregate, without
claiming consumer elaboration. Actual positive consumer verification took
570,901 ms aggregate. Independent review checked 295 files, 145 outputs, 29 oleans,
828 empty-axiom declarations, 88 successful processes and 29 kernel replays.
All 33 inputs and operational postguards passed. No target-module duration, cache
or efficiency improvement is claimed.

The connected-range equation itself did not establish a concrete finished result.
The [manifest-version checkpoint](../tests/fixtures/object-manifest-version/RESULTS.md)
now independently proves the full original version Result with the real admitted
encoding and HashWire computation. The sole expected first-byte 146→147 negative
is rejected. Positive verification took 38,752 ms in aggregate; the audit checked
85 files, 40 outputs, 740 empty-axiom declarations and 8 kernel replays. Eleven
source files were supplied, but only eight modules were selected and verified.

The [direct concrete-finish checkpoint](../tests/fixtures/object-concrete-range-finish/RESULTS.md)
now preserves an actual target timeout. The first wrong-wire phase supplied 15
inputs and 11 source files; source admission selected nine modules. Lean timed
out at 300,013 ms against the unchanged 300,000 ms child limit, with 425,362 ms
aggregate verification time. No artifact was produced. Independent failure
review confirmed clean source/lock/tool/epoch/resource postconditions and zero
memory events. The intended semantic negative is NOT_ESTABLISHED and the
positive remains NOT_RUN. Those postconditions are not proof acceptance.

The [contextual-finish checkpoint](../tests/fixtures/object-contextual-range-finish/RESULTS.md)
now independently proves the complete original ObjRangeFinish result: 96 output
bytes, the real version, original range, empty remainder, four visits, four
distinct records and 184 wire bytes. The supported context and transport retain
the real length guard and error branches. The mandatory consumer applies the
lifted theorem at Nat 1; helper-only success is not the acceptance boundary.
The full-consumer expected wire statistic 185 is rejected in 62,057 ms aggregate.
Positive verification took 49,645 ms aggregate. Independent audit checked 125
artifact files, 60 outputs, 776 empty-axiom declarations and 12 kernel replays.
Eighteen inputs supplied fourteen source files; twelve modules were selected.
All recorded source, lock, tool, epoch, memory and storage postconditions passed.
The prior direct-proof timeout remains a failed configuration, and neither
hash avoidance, caching nor a general timing improvement is claimed.

The [admitted-range checkpoint](../tests/fixtures/object-admitted-range-result/RESULTS.md)
now independently proves the complete original ObjRangeAdmitted Result. The
actual consumer directly applies the contextual finish theorem; its successful
kernel check establishes the required definitional conversion for these inputs.
The negative consumer was rejected in 108,728 ms aggregate. Its diagnostic alone
does not establish conversion or isolate mismatch causation to the false statistic;
the independently verified positive supplies that conversion evidence.
Positive verification took 289,504 ms aggregate. Audit covered 135 files, 65
outputs, 777 empty-axiom declarations and 13 kernel replays. Both source checks
admitted thirteen modules from fifteen supplied source files and nineteen inputs;
only the positive verified the complete module closure successfully.
All source, lock, tool, epoch, idle, memory-event and storage postconditions passed.
Recorded resource observations are samples, not measured peaks. No cache or
avoided-work claim follows from the successful direct Apply.

The attempted next connection is the original ordinal 26 call to R4.Object.ObjWholeRead,
retaining its real whole-data hash and digest-comparison continuation after the
admitted-range result. ObjRangeRead is a sibling API, not this original call.
The [next whole-read continuation proposal](continuation/2026-10-23/next-whole-read-continuation-proposal.json),
[independent SPEC](continuation/2026-10-23/next-whole-read-continuation-spec-review.json)
and [portable bindings](continuation/2026-10-23/whole-read-continuation-plan-bindings.json)
remain the frozen source design. The actual candidate and both official phases
are now preserved in the [whole-read continuation checkpoint](../tests/fixtures/object-whole-read-continuation/RESULTS.md).
The wrong zero-base Apply was rejected in the transport dependency in 203,247 ms;
this does not establish negative-consumer elaboration. Positive source admission
passed eighteen modules, but the mandatory original whole-read consumer timed
out at 300,004 ms against the unchanged 300,000 ms child limit, with 400,487 ms
aggregate verification time and no artifact. Independent review checked all
22 inputs, the genuine development lock, tools, source, epoch, idle and reserves.
The sole postcapture guard failure was memory-after: max450159, oom0, oom_kill0.
This records timeout with memory-limit pressure, not an OOM kill, semantic
falsity, an identified expensive subterm or proof that pressure caused the timeout.
The recorded 3.609 GiB observation is a sample, not peak usage. That historical epoch
has nonzero counters and failed the unchanged zero-event gate. The later separately
reviewed 05:31 epoch and accepted native-format run had zero events; neither
changes the failed historical gate nor establishes ongoing live eligibility.

That failed whole-read continuation configuration remains unaccepted. The successful
whole-read Result was open at this checkpoint; later native composition proves it
for the exact local 96-byte/four-record case. Canonical ordinal 26 and Read33 remain open.
Do not repeat the equivalent run, raise its limits or silently reset its resource
history. The next hash investigation initially overlooked an exact existing
96-byte wire proof because its list decoder omitted R4.Hash.append. Preserve
that rejected census and its independent review. A corrected reuse proposal
examines the actual retained Direct09 proof and real ObjHashBytes wrapper;
the [corrected proposal](../tests/fixtures/object-whole-read-continuation/evidence/next-whole-digest-reuse-successor.json)
and [independent SPEC](../tests/fixtures/object-whole-read-continuation/evidence/next-whole-digest-reuse-spec-review.json)
permit preparing this standalone lemma without requiring the failed whole
connection to pass first. It must retain the exact projected input, genuine
Result wrapper and expected-byte201→202 negative. Written-source review, a new
official closure/lock and a separately reviewed fresh resource epoch are still
required before native work. At that design checkpoint all wrapper tests were
NOT_RUN; the subsequent digest checkpoint below records their actual results. Historical native
artifact binaries remain local; the new closure must be verified from source.
This does not retroactively prove the failed whole-read connection.

At that historical checkpoint, whole-data hashing and digest comparison remained open;
the later digest and complete post-range checkpoints below record their local acceptance.
The later outer-shell checkpoint establishes admission/backend conversion. The complete outer Result was open at that checkpoint and is now proved locally by the later installed-format native composition. Original ordinal 26, all Read33 and full integration remain open.
Keep the nine remaining drafts in the completion sequence and use fresh resource
gates before native work. All earlier failed configurations remain preserved.

## Genuine whole-data digest checkpoint

The [standalone digest lemma](../tests/fixtures/object-whole-digest-reuse/RESULTS.md)
has now passed official verification and independent artifact review. It proves
the production ObjHashBytes call on the original admitted range record's96
projected bytes equals the complete expected successful digest Result. The
exact Direct09 source is imported and reverified through the real Result wrapper
and supported transport; no historical binary or lock is transplanted.

Both source checks admitted11 modules from14 supplied source modules and18
inputs. The expected first digest byte201→202 negative is rejected in the full
consumer in115437ms. Positive verification took113189ms; independent review
checked115 artifact files,55 outputs,11 oleans,744 empty-axiom declarations,
34 successful processes and11 kernel replays. All source, genuine development
lock, tool, epoch, idle, memory-event and storage checks passed. The negative
resource observation is post-run; the positive885.9MiB observation is an active
Direct09 replay sample, not peak consumption or a general efficiency claim.

The old whole-read timeout and nonzero resource history remain preserved. These
new phases used the separately reviewed2026-10-24T03:23:35.957043396Z epoch with
unchanged container, compiler, limits and failed stages. The prior source-only
preparation records remain historical; the new RESULTS and reviews record the
actual narrower acceptance.

At the digest checkpoint, the next step was a separately reviewed connection from
the genuine post-range hash/digest continuation to the complete ObjWholeVerified
Result, retaining comparison, errors and wrapping. The complete production post-range
checkpoint below now records that step's actual acceptance. It does not prove the
failed outer ObjWholeRead consumer. The later outer-shell checkpoint establishes admission/backend conversion. The complete
whole Result was open at that checkpoint and is now proved for the exact local case
by the later installed-format native composition; canonical ordinal 26, Read33,
nine drafts and full integration remain required.

The exact [next continuation proposal](../tests/fixtures/object-whole-digest-reuse/evidence/next-whole-result-investigation.json) and [independent SPEC](../tests/fixtures/object-whole-digest-reuse/evidence/next-whole-result-spec-review.json) were accepted as source design only at that checkpoint, when the new candidate and native tests were NOT_RUN. Their mandatory consumer retains the production post-range expression returning the complete ObjWholeVerified Result, with only expected wire184→185 changed in the negative. The complete production post-range checkpoint below records the subsequent actual proof and intended negative. This was a distinct step, not an equivalent retry of the failed outer consumer.

## Complete production post-range result

The [actual post-range continuation](../tests/fixtures/object-whole-postrange-result/RESULTS.md) now verifies through the official compiler and independent artifact audit. It preserves genuine ObjHashBytes error propagation, the original digest comparison and integrity-failure branch, and returns the complete original96-byte ObjWholeVerified result with exact version and statistics4/4/184. This is the computation after the accepted range record, not yet the original outer ObjWholeRead call.

Both phases admitted14 modules from17 supplied sources and21 inputs. The only negative change, expected visitedRecordBytes184→185, was rejected at the mandatory full consumer (LLV7002,345061ms aggregate). The positive verified in494719ms aggregate, attestationc8a21fe7466cd404ccea624de08f094f0f60b6d0be20735b1410295986ecbd43. Independent review checked145 actual artifact files,70 outputs,14 oleans,748 empty-axiom declarations,43 successful processes and14 kernel replays. Parent rehashed the actual145 artifacts and49 raw receipts against the review. These are aggregate timings and discrete memory samples, not peak-memory or efficiency claims.

All21 inputs, genuine development locks, tool identities, unchanged resource epoch, zero memory-event counters, idle checks and storage reserves passed. No SDK job remains active. The immutable preparation history remains preserved; current RESULTS and reviews establish the narrower acceptance. The earlier negative packager record marked positive NOT_RUN_AT_CAPTURE without observing the parent job; additive results correct that knowledge limitation because the parent positive was already running.

At the post-range checkpoint, the next step was to connect outer admission and backend-key validation to the actual admitted call. That narrower equation is accepted in the outer-shell checkpoint below. The earlier whole-read-to-continuation timeout remains failed and preserved. No equivalent retry, limit increase, manufactured lock or assumed equality is authorized by this result. The original ordinal26/Read33, nine remaining drafts, fullObject/Hash integration, production PrismPM binding and generated Pages remain required.

The [next outer-shell proposal](../tests/fixtures/object-whole-postrange-result/evidence/next-outer-whole-investigation.json) had passed [independent SPEC](../tests/fixtures/object-whole-postrange-result/evidence/next-outer-whole-spec-review.json) as design only at that checkpoint. It proposed the actual admission/backend-key shell with a symbolic Result tail, followed by the mandatory original ObjWholeRead = ObjWholeAdmitted consumer. The later source and invocation reviews and official phases now establish the mandatory concrete admission conversion; its false schemaMismatch helper is independently accepted as a dependency-level negative. The original design-only/NOT_RUN records remain historical; current actual results follow below. This does not establish the final successful outer result or repeat the failed whole-to-hash-continuation goal.

## Original whole-read admission and backend connection

The [outer-shell checkpoint](../tests/fixtures/object-whole-outer-shell/RESULTS.md) now proves the exact original ObjWholeRead call equals the original ObjWholeAdmitted call. The real manifest admission and backend-key validation remain in the computation. Independent inspection reconstructed the emitted helper and mandatory consumer, including its concrete admitted-binder conversion and exact Apply argument. This is an equality to the admitted call, not yet its complete successful Result.

Both phases admitted all12 supplied source modules from16 inputs. The false helper alone changes the expected symbolic tail to typed schemaMismatch and is rejected by rfl with LLV7002 in14819ms; this is a dependency-level negative, not negative consumer elaboration. Positive official verification took19772ms, attestationf905d7e39823ac7939f78ccd8b8f73cba4f2042f8ccce6e2cb40e15cf789d7fc. Independent review audited125 artifacts,60 outputs,12 oleans,806 empty-axiom declarations,37 successful processes and12 kernel replays. Parent rehashed the125 actual artifacts and43 raw receipts against that audit.

All16 input hashes, genuine development locks, tools, unchanged epoch, zero memory events, idle checks and storage reserves passed. Historical readiness and source-preparation records remain unchanged; current RESULTS identify the accepted scope. No fresh full Read33 control or production claim follows.

At the outer-shell checkpoint, the next step was to connect the original admitted call to the accepted post-range computation. The later inner-connection checkpoint below now records the compiler proof and its failed operational gate. At that checkpoint, complete whole Result composition and the ordinal26 projection remained open. The later installed-format native checkpoint proves the full Result; canonical ordinal26 remains open. The prior whole-to-continuation timeout remains failed. Nine unfinished drafts, full Object/Hash integration, genuine production PrismPM binding and generated Pages remain required; no equivalent limit retry or manufactured lock.

The [next inner-connection proposal](../tests/fixtures/object-whole-outer-shell/evidence/next-inner-connection-investigation.json) had passed [independent SPEC](../tests/fixtures/object-whole-outer-shell/evidence/next-inner-connection-spec-review.json) as source design only at that checkpoint. It retains the existing range context and supported transport, with a mandatory actual ObjWholeAdmitted = C consumer, where C is the accepted post-range theorem's left expression. The wrong-base negative is dependency-level; the new tests were NOT_RUN at preparation time. The subsequent actual consumer proof and failed resource gate are recorded below. The later installed-format checkpoint now supplies the complete Result composition; a supported connection to canonical ordinal26 is still required. Source and invocation reviews preceded those actual phases; no operational acceptance or further execution follows from the failed resource gate.

## Inner admitted-call connection: verified artifact, failed run gate

The [inner-connection results](../tests/fixtures/object-whole-inner-connection/RESULTS.md) record **COMPILER_ARTIFACT_VERIFIED_RUN_GATE_FAILED**. Official verification and independent kernel audit establish the complete original ObjWholeAdmitted = C equation, retaining the genuine post-range hash/digest comparison, errors and wrapping. Positive verification returned 0 in 264273 ms aggregate, attestation 4e1e8a432c06fb92985d8c470b43deabb71772b9d92aebad565281e29fd79f20. The audit checked 185 files, 90 outputs, 18 oleans, 814 empty-axiom declarations, 55 successful processes and 18 kernel replays.

Overall phase acceptance remains failed: memory.events.max increased from 0 to 46, with no OOM or kill events, so the launcher returned 1. All other captured source, lock, tool, epoch, idle and storage checks passed. These are discrete observations, not peak-memory or cause attribution. The intended wrong-base negative was rejected in the transport dependency with LLV7002 in 288296 ms aggregate; it did not elaborate the mandatory consumer. Both phases admitted all 18 supplied sources from 22 inputs.

The immutable preparation and pending-review records remain historical. The compiler artifact and failed resource gate are both preserved; neither erases the other. No counter reset, restart, native retry or limit increase is authorized by these results. The [next composition proposal](../tests/fixtures/object-whole-inner-connection/evidence/next-composition-investigation.json) and [independent SPEC](../tests/fixtures/object-whole-inner-connection/evidence/next-composition-spec-review.json) at that checkpoint permitted bounded conditional source preparation only: official LexLean core-module/2 authoring with two kernel Eq.trans applications composing O=A, A=C and C=R into the mandatory original O=R goal. At preparation time all new tests were NOT_RUN. The later composition admission attempt below failed before verification; the earlier inner run stays resource-gate failed. No automatic restart, counter reset, retry, limit increase or waiver is authorized. At that checkpoint, the successful complete inner/outer Result remained required; the later installed-format native result now proves it locally. Original ordinal26, Read33, nine unfinished drafts, full Object/Hash integration, production PrismPM binding and generated Pages remain required.


### Native composition: installed-source admission failure

The [composition results](../tests/fixtures/object-whole-composed-result/RESULTS.md) preserve the corrected native candidate and rejected preparation history. Official lock generation succeeded, but source check exited 1 in 441 ms with LLP2003: expected `\coredata`, found `imports` at line 14. Verification did not run, no artifact was produced, the intended unrelated-proof negative was not established, and the positive phase did not run. Independent review checked 32 inputs, 57 raw records and 52 command receipts; all postconditions passed with zero memory events.

The installed development compiler vendors LexLean `0865df4e540ca643b090a29e9df6d319d6906da6`; raw notation was reviewed against newer `5967d9f` source. Exact installed-source inspection identifies a required coredata envelope and core-module/1 schema versus intended /2. Only the envelope rejection was observed natively. That failure led to independent exact capability review and the separately reviewed singleton /1 migration recorded below. It was not an envelope-only retry or an unreviewed schema downgrade. Full original whole Result composition is now proved locally; canonical ordinal26, Read33, nine unfinished drafts, Object/Hash integration, production PrismPM binding and generated Pages remain required. The earlier inner kernel artifact and failed resource gate remain preserved separately.


### Installed-format composition: original whole-read full Result accepted locally

The [actual native result](../tests/fixtures/object-whole-native-format/RESULTS.md) is independently accepted with all run gates. The installed-format migration retains the entire singleton proof DAG and uses canonical coredata /1, omitting only the reviewed /2 singleton all field. Actual verification checks the original 96-byte/four-record O=R goal through two kernel Eq.trans applications; the singleton is present exactly once in the actual declaration audit and attestation. Positive verification took 363,107 ms aggregate. Audit covered 285 files, 140 build outputs, 28 oleans, 827 empty-axiom declarations, 85 successful processes and 28 kernel replays. All 32 input identities, genuine development lock, tools, epoch, zero memory events, idle checks and storage reserves passed. The intended unrelated0=0 premise was rejected at the mandatory native root in the negative phase. No runtime, language, limit or axiom policy was weakened.

This is a local full Result proof, not canonical ordinal26 or full Read33 acceptance. The [next source investigation](../tests/fixtures/object-whole-native-format/evidence/next-ordinal26-investigation.json) identifies an installed semantic/native theorem-import boundary. At that checkpoint, its mathematical projection recipe was not an authored, reviewed executable plan, and a native projection alone could not discharge the existing semantic declaration. The current Read33 source checkpoint above now supplies an explicitly reviewed native representation and invocation; all of its official native tests remain NOT_RUN. The separate [sequencing SPEC](../tests/fixtures/object-whole-native-format/evidence/next-integration-spec-review.json) explicitly adjusts the earlier Read33-first ordering to allow independent Codec311 work next. At that design checkpoint only bool/nat glossary 1.1-to-1.2 and semantic-module/1-to-/2 metadata changes were proposed, preserving all 311 declaration ASTs; the later canonical Codec checkpoint records their actual implementation and full-root proof. The proposed source is pinned as 17ca6e3d… and config as 75fe2142… in the [portable binding](../tests/fixtures/object-whole-native-format/evidence/next-integration-bindings.json). Those next tests were NOT_RUN at that design checkpoint; the Codec results above supersede that status with actual reviewed evidence. Canonical ordinal 26, Read33, the nine remaining drafts and full product obligations remain required.

Earlier /2 admission and authoring failures, the original whole-read timeout, and the earlier inner run's max46 operational failure remain preserved. This successful fresh run does not relabel those failures. The full Object/Hash qualification, nine unfinished drafts, original ordinal26 and Read33, production PrismPM binding and generated Pages remain open. Timing is aggregate, not a measured kernel peak, individual-theorem cost or efficiency claim.
