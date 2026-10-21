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
Fifteen source slices are implemented in LexLean and registered with the inherited
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
- Route traversal, with 58 controls for root-port selection, bounded framed
  descent, ordered transport and occurrence-aware virtual changes.
- Finite learning transitions, with 154 controls for complete candidate
  inventories, batch eligibility, shared trits and binding/epoch-guarded updates.
- Exact fixed-width word operations, with 995 controls for arithmetic,
  bitwise operations, checked rotations/shifts and endian codecs.
- Complete Flock/Relation reads, with 119 controls for transported forests,
  five relation edges, cumulative synchronous sweeps, causal selection,
  actual-count consensus, virtual changes and trace projections.
- Composed Engine, with 100 controls for exact seeded initialization, carried
  causal windows, full batch admission, all read modes, actual paired
  counterfactual reads and writer scores, and guarded geometric transitions.
- Complete SHA-256 and configured BLAKE2b-96 node identities, with thirty
  independent controls covering full digests, padding, multiblock chaining,
  logical counters, byte order and typed admission.
- Complete Unicode and UTF-8 text primitives, with 3,865 controls for the
  pinned Unicode predicates, scalar and byte admission, strict and lossy
  decoding, encoding, and the reversible tokenizer byte alphabet.
- Typed tokenizer admission and the full rank-BPE kernel, with 127 controls
  for added tokens, pretokenization, rank ties, all-occurrence merging, Unicode
  overlap, complete byte-alphabet use, decoding and token-length policies.
- Complete imported mechanism catalog and evidence descriptions, with 627
  controls for all 27 mechanisms, shared capability obligations, finite graph
  admission, exact identity roles, report classifications and admitted queries.

Each slice has independent kernel replay and planted-defect evidence. The
conformance verifier binds reviewed control bytes and current model inputs to
the fresh proof receipt. This is formal development evidence; the integrated
training workflow, artifact custody, memory and browser application remain
under implementation.
See the [verification record](docs/VERIFICATION.md),
[geometry contract](docs/GEOMETRY.md), [objective contract](docs/OBJECTIVE.md)
[binder contract](docs/BINDER.md), [admission contract](docs/ADMISSION.md)
and [configuration contract](docs/CONFIGURATION.md). The default verification
roots include all fifteen slices, containing 7,263 theorem controls across
77 modules. [Route](docs/ROUTE.md), [learning transitions](docs/LEARNING.md),
[word operations](docs/WORDS.md) and [Flock/Relation](docs/FLOCK.md) each passed their refreshed isolated-snapshot
integration test after review. The [Engine](docs/ENGINE.md) supplies the actual
read/writer outcomes and complete candidate losses to the learning component;
its integration test and five planted-defect checks pass. This establishes
formal composition, without establishing runtime custody or corpus training.
The [hash component](docs/HASH.md) preserves every original theorem while
compiling expensive digest controls in separate modules. The newer SDK passed
the complete twelve-component default graph, including all 57 independent
kernel replays. The later [text component](docs/TOKENIZER.md) passed its
mandatory integration test and three independent kernel replays. The
[tokenizer kernel](docs/TOKENIZER-KERNEL.md) passed all six fixture checks
through its mandatory Rust integration test. The expanded 77-module graph
passed source checking; combined fifteen-component proof verification
remains pending. Raw JSON admission and tokenizer identity remain separate
required components. The [evidence component](docs/EVIDENCE.md) passed its
mandatory integration gate across all seven fixtures. Structural report
admission preserves descriptions and does not authenticate or authorize them.

The current upstream template has no `prismpm.lock` or `template.lock` and is
not bound to a released SDK. Its full `just vv` boundary therefore cannot pass
as supplied. A published development SDK candidate has been identified; it is
not a production-accepted release. A newer official unpublished SDK construction
supports further development verification. See the [source and dependency review](docs/UPSTREAM.md)
and [production export contract](docs/specs/production-export-contract.md) for
the distinct compiler, browser runtime and bounded serving dependencies.

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
