# Source and dependency review

Reviewed on 2026-10-21. These are source observations, not executable acceptance.
No research compute, parity evaluation, training, or deployment ran.

## Source observation update, 2026-10-24

The [bounded refresh](continuation/2026-10-24/upstream-binding-refresh/refresh.json)
and [independent source review](continuation/2026-10-24/upstream-binding-refresh/refresh-spec-review.json)
preserve fetched metadata and exact source bytes. At captured PrismPM revision
`dd8b2f0f9f536bffa3acd8180851b9fb234db194`,
[RELEASE-STATUS.md](https://github.com/UOR-Foundation/PrismPM/blob/dd8b2f0f9f536bffa3acd8180851b9fb234db194/RELEASE-STATUS.md)
explicitly declares production SDK/ecosystem acceptance incomplete and Browser
`PP2011` closed. This updates the earlier document observation below; it is
not a new execution audit of those upstream capabilities.

The fetched release collection still returns `v0.2.0`, whose notes describe an
unsigned development bootstrap. Its API metadata says `immutable: false`;
release-note wording does not establish GitHub-enforced immutability. Template
main remains `1bea460bac6ea50bae53a7eeb674589d7900e6cb` in this observation.

The captured [development lock](https://github.com/UOR-Foundation/PrismPM/blob/dd8b2f0f9f536bffa3acd8180851b9fb234db194/sdk/golden-development.lock.json)
now names index `sha256:42213da87c564d859e327ab7df2459357d674fa538050f13905caa24c352588e`
at source `6ebbef86a300ff4490c4c37f821e50585e211481`, with scope
`source-golden-review-only`. This refresh did not inspect its publication receipt,
image inventory or installed behavior. It establishes no production binding,
compatibility or authority to replace the compiler used for the current proofs.
The installed compiler, earlier source identities and failure history below
remain unchanged. The captured collection is not an exhaustive audit of every
publication channel.

## Source identities

| Source | Reviewed commit | Role |
| --- | --- | --- |
| [UOR-R4](https://github.com/UOR-Foundation/uor-r4/tree/4611538bb4843947363ccb9cce5f388e32c46883) | `4611538bb4843947363ccb9cce5f388e32c46883` | Mechanics, artifact and protocol semantics, scientific constraints (`R4-01` .. `R4-22`) |
| [UOR-R4 Integer Stack](https://github.com/UOR-Foundation/uor-r4/tree/4195ce2bf83d703b1acd3eb868c888d1da053479/crates/uor-r4-integer/src/stack) | `4195ce2bf83d703b1acd3eb868c888d1da053479` | Integer LUT serving stack (`/6` Hamming read, `/7` turn memory, `/8` point-key L1 plurality, Q31 retention lambdas, whole-table nearest readout, `R4-23`) |
| [LexLean](https://github.com/UOR-Foundation/LexLean/tree/6bcc8d59836506d6365dcfa4c755d068771c4f93) | `6bcc8d59836506d6365dcfa4c755d068771c4f93` | Authoring language, proof and production contracts |
| [PrismPM](https://github.com/UOR-Foundation/PrismPM/tree/590ec8a9c41d46cd61dea567130f8f8ef9bca26e) | `590ec8a9c41d46cd61dea567130f8f8ef9bca26e` | Model projection, application profiles, SDK and deployment contracts |
| [Template](https://github.com/UOR-Foundation/template/tree/1bea460bac6ea50bae53a7eeb674589d7900e6cb) | `1bea460bac6ea50bae53a7eeb674589d7900e6cb` | Repository policy and bootstrap |
| [Calculator example](https://github.com/UOR-Foundation/calculator-example/tree/5051dc6adb18e9ca2b6ae4ca7090e638329831ef) | `5051dc6adb18e9ca2b6ae4ca7090e638329831ef` | Example of model-owned application and generated Pages closure |

GitHub's generated repository starts at
`9f437a6e1e9a941236ab5b4d607706890f52a8a4`; its repository metadata identifies
`UOR-Foundation/template` as `template_repository`. This is a template instance,
not a fork of the Rust implementation.

## Toolchain findings

### Template locks are absent

At the reviewed template revision, `prismpm.lock` and `template.lock` do not
exist. The devcontainer is named `PrismPM SDK release binding required` and
contains no image. `README.md` explicitly documents the unbound state.
The universal `template-contract.json` requires both locks. Therefore the
template's complete `just vv` cannot establish acceptance as supplied.

The renderer requires real multi-platform image inventories and a full PrismPM
Action revision, not just a string containing a digest. These facts must be
extracted and validated by the SDK before committing bindings.

### Published release and documentation disagree

PrismPM's `RELEASE-STATUS.md` says 0.3.0 acceptance is complete. GitHub's latest
published release observed in this review is
[v0.2.0](https://github.com/UOR-Foundation/PrismPM/releases/tag/v0.2.0), whose
release notes explicitly describe an unsigned development bootstrap. The
calculator README says its production SDK migration is not yet released.
These statements do not establish a usable production-accepted 0.3.0 SDK.

The PrismPM repository does identify a real development candidate in
`sdk/golden-development.lock.json` and `sdk/golden-development-amd64.lock.json`:

```text
ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21
linux/arm64 child sha256:2f82a04e8f8141636e03a219d3960e30d98a6f304828341dd47b553b3c23baeb
linux/amd64 child sha256:c2e0e50437e13d9b2e382d3af4ae7a962b469721b9b80f14f215d9254e8ed78f
```

A [successful workflow run](https://github.com/UOR-Foundation/PrismPM/actions/runs/39492842177)
was observed for source `dfdc4df770d038ee5fb700d374aca883d1c9ec7a`. Its actual
jobs and artifact names identify **unpublished native SDK construction** for
both architectures, not a candidate publication. This run does not establish
publication of the candidate above. That binding must be read from a real
publication receipt. The current candidate workflow's own contract explicitly
says development-only. A successful workflow is not a production acceptance
receipt. The GitHub package-list API returned HTTP 403
because the current CLI credential lacks `read:packages`; that does not prove
the candidate image is unavailable to anonymous OCI pulls.

The actual candidate publication is
[run 35065092990](https://github.com/UOR-Foundation/PrismPM/actions/runs/35065092990),
at source `d0174e1d64339f73091fe4c59d5d6bf532a37d1f`.
Its retained `sdk-candidate-reference` artifact names the exact image above.
Anonymous OCI retrieval confirmed the index and both child manifests; the
arm64 image was pulled and inspected. Its LexLean dependency is
`01088ef00d9d0d0227775e0346a852c2d19cb67b`, with compiler semantics
`49561889f77c9c3571da51e8e53233da9df5ab8f46d16165159425ba2e82dc44`.
It accepts language 1.1 canonical semantic modules, rather than current main's
language 1.2 authoring syntax. Its `prismpm template` CLI has `check` and `update`,
but lacks the renderer needed by the current template's binding procedure.

Local Docker and Colima were installed and an isolated `lexlean-r4` VM was
created without changing the default Docker context. Formal source verification
runs inside that immutable development image. No host compiler or handwritten
SDK substitute is used. This is development evidence only: the candidate's
own recorded scope is `source-golden-review-only`, and neither a production
`prismpm.lock` nor `template.lock` has been fabricated from it.

### Newer unpublished construction diagnostic

The complete digest controls exposed a normalizer-depth limit in the published
development candidate. Independent source review found a concrete upstream
change: newer semantic backends emit fixed finite Lean budgets of 100,000
recursion depth and 1,000,000,000 heartbeats. The candidate uses the older
defaults. This is a compiler-identity difference; no project-local option,
generated Lean edit, axiom permission or golden digest was changed.

The official [native construction run 39492842177](https://github.com/UOR-Foundation/PrismPM/actions/runs/39492842177)
retains a complete runtime OCI archive. Its exact identities are:

| Input | Identity |
| --- | --- |
| PrismPM/workflow source | `dfdc4df770d038ee5fb700d374aca883d1c9ec7a` |
| Vendored LexLean source | `0865df4e540ca643b090a29e9df6d319d6906da6` |
| GitHub arm64 artifact | `12302860930` |
| Downloaded ZIP SHA-256 | `5b703f6196a646cc7bb8701c778dfd224485dfae3f8328eabd11ec6177a8ea61` |
| OCI archive SHA-256 | `d408f9895baf484db72d5b90b617cb60651770cc327eab93df490393d987d8ad` |
| OCI manifest SHA-256 | `716370f20b6b01948700a16380668f5328b854e3764188916fc8802ef27ca7c6` |
| OCI config SHA-256 | `3d45853bec499af76db83e2ceefdd96edaf8d79742cfe7ba77faea8ee4ce8a3a` |
| Installed LexLean binary SHA-256 | `107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5` |
| `--version` compatibility-language 1.0 semantics | `95deb33a8d416d7bf60f02a36e251a71c3ee6f046b7e475d7bae2fc5ddc3767d` |
| Regenerated language 1.1 project-lock semantics | `4658bc8564c14b99f771a758276c578e8acd5ef5f24a3c43e69e4e6eb7df370e` |

The downloaded ZIP digest matched GitHub's artifact metadata. Its construction
record, ten exact smoke-evidence files, 5,332,154,880 archive bytes, TAR member
inventory, all OCI blobs, reachable descriptor graph, manifest/config and
architecture/source labels were validated before import. The validation code
received independent security review; its initial incomplete TAR checks were
repaired and the stronger validation rerun successfully. The immutable image
was imported into the isolated task VM. Its installed compiler digest matches
the recorded SDK inventory; its Lean toolchain is still 4.32.1.

The two semantic identities serve different language selections. At the exact
vendored revision, `src/cli.rs::version_text` reports the compatibility default
language 1.0, while `src/lock.rs` calls `compiler_semantics_id_for` with the
project's declared language. `src/lib.rs` hashes a language-specific embedded
source inventory. The language 1.1 value above was observed in the regenerated
isolated project lock and is not the default version-banner identity. Neither
semantic identity replaces the separately verified executable digest.

This construction vendors LexLean `0865df4`, rather than the separately reviewed
standalone `6bcc8d5`; the reviewed PrismPM `590ec8a` source separately vendors
`9c1456d34b896ab77db01c5e8ebbfed584edee26`. These identities must not be conflated.
The construction's own scope is `construction-only-not-acceptance`. It expressly
does not establish full VV, installed dual-native qualification, signing,
release or product readiness. Its isolated diagnostic gets a fresh genuine
compiler lock; it does not create production SDK or template bindings.

Neither this newer semantic AST nor language 1.2 provides a general ordered
rewrite-and-finish proof constructor. The structured prose proof IR is a
different authoring surface. Therefore the diagnostic tests the actual fixed
compiler budgets against unchanged complete algorithms and golden controls;
it does not presume that a language-version change supplies missing proofs.

### CUDA research is a separate missing execution target

LexLean `SPEC.md` §17.13 registers `rust-core` and `rust-std` production targets.
UOR-R4's active `crates/uor-r4-discrete/src/lib.rs` explicitly has no host model
implementation; `discrete.cu` executes its model and research operations.
The reviewed production-target contract supplies no CUDA target. A formal
learning transition in LexLean is possible without establishing executable GPU
training. Generated CPU model evaluation must not substitute for GPU parity or
qualification under the source charter.

### Portable Pages is not persistent browser memory

PrismPM `SPEC.md` §3 explicitly excludes persistence from its portable text
profile. Its separate browser application profile models storage and operation
recovery, but §12.9 describes a declaration-only public boundary, while
`crates/prismpm/src/browser_build.rs` describes a private compiler.

At the initially reviewed PrismPM revision `590ec8a`,
`holo/browser_application.rs::require_runtime` conditionally rejected a browser
application when a View label equaled `Runtime unavailable`; removing that
label could not establish accepted storage, custody or recovery. The newer
construction revision `dfdc4df` closes that inconsistency: it unconditionally
rejects every Browser application with `PP2011`, explicitly naming the missing
runtime behavior. Neither revision establishes the required public runtime.
See the [exact production contract](specs/production-export-contract.md).
A stateless text demo does not satisfy durable conversation memory.

### Integer output is not D11 acceptance

LexLean's fixed-width types and Rust target profiles do not guarantee absence
of multiply/divide instructions or steady-state allocation. Generated native
and Wasm decode closures require instruction inspection and allocation evidence.
Use fixed-capacity representations for frozen inference, and distinguish
loading/tokenization/storage work from the measured decode boundary.

## Source-review discipline

The UOR working checkout had existing changes and an older charter. The review
uses the pinned remote commit above rather than those working files. The pinned
charter includes later governance changes, including different-vendor code/model
review and updated release procedures. This new repository inherits the
template's rules; it does not rewrite the upstream charter, operate its labs,
release held research, or promote mechanism status.

The user's explicit language choice is LexLean for this separate project.
Rust remains a generated target or inherited verification tool, not a second
handwritten implementation of R4 behavior.

## Source observation update, 2026-10-31 (`R4-23` integer serving stack)

At upstream `UOR-Foundation/uor-r4` revision `4195ce2bf83d703b1acd3eb868c888d1da053479`,
`crates/uor-r4-integer/src/stack/` defines the integer LUT serving stack modeled
by `R4-23` (`src/R4/IntegerStack.lex.tex` and `tests/lexlean/IntegerStackControls.lex.tex`):

| Upstream path (`4195ce2bf83d703b1acd3eb868c888d1da053479`) | SHA-256 | Modeled mechanics |
| --- | --- | --- |
| `crates/uor-r4-integer/src/stack/mod.rs` | `58144609d5b1b492f332e0f9edc3af4bad9de2424db439fdb7776049d135c103` | Stack container versions (`/1`..`/8`) and whole-table nearest L1 readout |
| `crates/uor-r4-integer/src/stack/hamming.rs` | `b7c1cb291346de0a4f7b7caacd1d1155168b802d02a8e0c1bb89b024b4695cba` | `/6` induction-keyed Hamming read and margin-gated weight |
| `crates/uor-r4-integer/src/stack/memory.rs` | `5e812162dadecb466a7208bb07b0dc23f8cff892f11b92c14ddf46a722cbab85` | `/7` opening and reply-continuation tables and readout interpolation |
| `crates/uor-r4-integer/src/stack/point_key.rs` | `5a3692b27f0111e2f82eab8602b6fe2d87a9aceb88424814cf92ff664cd8a751` | `/8` continuous point-key L1 plurality fallback and tie-breaking |
| `crates/uor-r4-integer/src/stack/session.rs` | `0e051017b5d9845053cdbc164617aa814ec40f9898a16fa7282ff92bbff644d4` | `RETAIN_LAMBDA_Q31` geometric retention constants and snapshot turn-memory guard |
| `crates/uor-r4-integer/src/stack/chat.rs` | `ff33e7bf5659c83e35a203fa7f7f4b0666d83fa29e21a1db3767533916eed698` | Bounded greedy prose reply token loop stopping at EOS (`1`) or token cap |
