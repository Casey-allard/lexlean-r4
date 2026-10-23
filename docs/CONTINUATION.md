# LexLean R4: current working plan and continuation

Updated 2026-10-23. This is the repository entry point for continuing the work
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
| HashWire | Positive baseline for 78 roots; restoration accepted 16/78; mutation qualification 2/7. Incomplete. | [HashWire](HASH-WIRE.md), [checkpoint](../tests/fixtures/hash-wire/finite-checkpoint.json). Direct10 starts the remaining 62. |
| Object implementation | Five modules and 403 controls authored, not integrated or accepted as a component. | [Draft source snapshot](continuation/2026-10-23/pending-manifest.json); do not confuse it with active model/source. |
| Whole Object257 digest | Original reflexivity proof timed out at 300 seconds; the 900-second experiment exhausted the 4 GiB container. No accepted whole257 result. | [300s result](continuation/2026-10-23/evidence/original257-timeout300.json), [OOM review](continuation/2026-10-23/evidence/original257-timeout900-review.json). Do not repeat equivalent limit increases. |
| Final partial leaf and reuse | Actual leaf certificate, direct Apply, parent context and inventory-staging transport verified, with intended negative controls. | [Finite source fixture](../tests/fixtures/object257-leaf-transport/README.md), [results](../tests/fixtures/object257-leaf-transport/results.json) |
| Terminal fold transition | Actual `ObjFoldAtoms [[0]] frontier inventory limits` transition verified for arbitrary incoming state. Certified leaf staging, depth-38 carry, same leaf ID, root scale 0, errors and empty-tail fold are retained. | Commit `0e3544a85454276394f58344dfc93772dcd4c444`; [native review](../tests/fixtures/object257-leaf-transport/evidence/terminal-fold-native-review.json): 769 empty-axiom declarations, 11 kernel replays, positive 23,010 ms; wrong-base negative rejected. |
| Production binding and deployment | Missing production SDK/template binding. `just vv`, generated application and Pages remain incomplete. | [Upstream dependencies](UPSTREAM.md), CI observation below. |

The local Object proof experiments can proceed before full Hash acceptance.
That permits diagnostic source/proof development; it does not promote Hash or
Object into production conformance. No training, GPU qualification or language
capability follows from these finite proofs.

## Next executable product sequence

1. **Certify the first concrete state of the original 257-byte fold.** Review
   the [bounded proposal](continuation/2026-10-23/next-first-atom-proposal.json)
   and its [internal SPEC approval](continuation/2026-10-23/first-atom-spec-review.json)
   before authoring. Source review passed; the intended theorem is actual
   `ObjFoldAtoms [[0,1,...,31]] [] [] originalLimits = ok firstState`. Compute
   the original Record0 identity through a genuine leaf certificate, reuse the
   accepted terminal context, and require the actual consumer to reduce staging,
   carry and the empty-tail fold to the concrete singleton state. The proposed
   negative changes only the expected inventory ID byte 63 to 64. These new
   native tests are **NOT_RUN**. Retained literals are expected outputs, not
   trusted inputs or accepted certificates.
2. **Connect finite states to the original theorem.** Preserve the original
   input, digest, runtime definitions and empty-axiom policy. Demonstrate a
   supported proof bridge before expanding all thirteen node certificates.
   Direct proof `Cases` on built-in `Result` is unsupported in the inspected
   compiler; do not assume that bridge exists. The accepted Nat-family,
   Induction/Cases/Congruence/Apply pattern establishes the recorded local
   transitions only. Atomization, earlier transitions, success/error continuation
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
continues with the first concrete fold state described above.

This reconciles the enumerated project source/plan locations, not every local
file. Raw compiler bundles, build/VM caches, compiler checkouts and unrelated
plugin changes remain excluded and preserved locally; acceptance cannot rely on
missing raw artifacts. The inventories state their traversal and comparison
boundaries. No original local work was deleted during preservation.
