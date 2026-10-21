# Source and dependency review

Reviewed on 2026-10-21. These are source observations, not executable acceptance.
No research compute, parity evaluation, training, or deployment ran.

## Source identities

| Source | Reviewed commit | Role |
| --- | --- | --- |
| [UOR-R4](https://github.com/UOR-Foundation/uor-r4/tree/4611538bb4843947363ccb9cce5f388e32c46883) | `4611538bb4843947363ccb9cce5f388e32c46883` | Mechanics, artifact and protocol semantics, scientific constraints |
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

There is a further inconsistency: `holo/browser_application.rs::require_runtime`
rejects a browser application only when a View label equals `Runtime unavailable`.
Removing that label cannot establish accepted storage, custody, or recovery.
The actual public SDK build and runtime acceptance must demonstrate the required
behavior. A stateless text demo does not satisfy durable conversation memory.

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
