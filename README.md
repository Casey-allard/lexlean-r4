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
The first source slice implements the discrete engine's configuration predicate
in LexLean, with eleven exact boundary theorems, independent kernel replay,
and a registered PrismPM-template conformance scenario. Both a wrong bound and
a vacuous replacement theorem were planted and rejected. This establishes
bounded formal development evidence; the R4 engine, learning, memory and
application are not implemented yet. See the [verification record](docs/VERIFICATION.md).

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
