# LexLean R4

A pure [LexLean](https://github.com/UOR-Foundation/LexLean) implementation of
[UOR-R4](https://github.com/UOR-Foundation/uor-r4), fully modeled with
[PrismPM](https://github.com/UOR-Foundation/PrismPM).

The intended product is a geometric language model with exact addressed memory,
integer serving, and a generated browser application deployed to GitHub Pages.
The application, model mechanics, typed failures, proofs, state transitions,
View, and deployment contract belong in authoritative `.lex.tex` sources.
Generated Lean, Rust, WebAssembly, and browser files are projections of those
sources; handwritten implementations of application behavior are excluded.

## Current state

**Repository established; implementation and deployment are not complete.**
This repository was generated through GitHub's template operation from
[`UOR-Foundation/template`](https://github.com/UOR-Foundation/template).
Six source slices are implemented in LexLean and registered with the inherited
PrismPM-template conformance model:

- Configuration bounds, with eleven exact boundary theorems.
- Canonical finite geometry, with 774 theorem controls for the 64 actions,
  sixteen-root orbit, composition, inverse and ordered transport construction.
- Scalar objectives, with 91 controls for hinges, policies and checked weighted
  arithmetic, including overflow in an unselected branch.
- Canonical ternary binders and general forest validation, with 165 controls
  including the maximum context and valid noncanonical forests.
- Identity-preserving admission and writer objectives, with 72 controls for
  cell selection, exact ranking, virtual moves, eligibility and typed failures.
- Read configuration, with 75 controls for validation order, initialization,
  mode metadata and checked gate inventories.

Each slice has independent kernel replay and planted-defect evidence. The
conformance verifier binds reviewed control bytes and current model inputs to
the fresh proof receipt. This is formal development evidence; the integrated
R4 engine, learning, memory and browser application remain under implementation.
See the [verification record](docs/VERIFICATION.md),
[geometry contract](docs/GEOMETRY.md), [objective contract](docs/OBJECTIVE.md)
[binder contract](docs/BINDER.md), [admission contract](docs/ADMISSION.md)
and [configuration contract](docs/CONFIGURATION.md). The default verification
roots cover all six slices: 1,188 theorem controls across thirteen modules.

The current upstream template has no `prismpm.lock` or `template.lock` and is
not bound to a released SDK. Its full `just vv` boundary therefore cannot pass
as supplied. A published development SDK candidate has been identified; it is
not a production-accepted release. See the [source and dependency review](docs/UPSTREAM.md).

The planned site address is
<https://casey-allard.github.io/lexlean-r4/>. **It is not deployed.** Publication
requires a verified generated application closure; this README is not a demo.

## Design and delivery

- [Approved full-scope design](docs/specs/2026-10-21-lexlean-r4-design.md)
- [Implementation sequence and acceptance gates](docs/plans/2026-10-21-lexlean-r4.md)
- [Pinned source review and toolchain gaps](docs/UPSTREAM.md)
- [Inherited repository rules](AGENTS.md)
- [Template contract](TEMPLATE-CONTRACT.md)

UOR-R4 is pre-alpha. Port correctness, formal proof, finite construction checks,
GPU qualification, trained language capability, and product performance are
separate claims. A browser demonstration cannot establish a training result.

Use atomic conventional commits. Each capability starts with its model,
registered claim, behavioral scenario, and failing test, followed by its
implementation and a planted-defect check. `just model-write` generates
conformance evidence. `just vv` in the exact locked SDK is the full acceptance
boundary; it must not be replaced with a reduced release gate.

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
