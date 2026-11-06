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

For the live working plan, autonomous CI engine history, and portable handoff,
start with [CONTINUATION.md](docs/CONTINUATION.md) and the generated register in
[CONFORMANCE.md](CONFORMANCE.md).

**All twenty-eight source slices (`R4-01`..`R4-28`), the browser-native WebAssembly studio, and the Rule-30-modeled autonomous GitHub Actions engine (`Engine`) are implemented and deployed.**
This repository was generated through GitHub's template operation from
[`UOR-Foundation/template`](https://github.com/UOR-Foundation/template).
Twenty-eight source slices are implemented in LexLean and registered with the
PrismPM-template conformance model:

- Configuration bounds (`R4-01`), with eleven exact boundary theorems.
- Canonical finite geometry (`R4-02`), with 774 theorem controls for the 64 actions,
  sixteen-root orbit, composition, inverse and ordered transport construction.
- Scalar objectives (`R4-03`), with 91 controls for hinges, policies and checked weighted
  arithmetic, including overflow in an unselected branch.
- Canonical ternary binders and general forest validation (`R4-04`), with 165 controls
  including the maximum context and valid noncanonical forests.
- Identity-preserving admission and writer objectives (`R4-05`), with 72 controls for
  cell selection, exact ranking, virtual moves, eligibility and typed failures.
- Read configuration (`R4-06`), with 75 controls for validation order, initialization,
  mode metadata and checked gate inventories.
- Route traversal (`R4-07`), with 58 controls for root-port selection, bounded framed
  descent, ordered transport and occurrence-aware virtual changes.
- Finite learning transitions (`R4-08`), with 154 controls for complete candidate
  inventories, batch eligibility, shared trits and binding/epoch-guarded updates.
- Exact fixed-width word operations (`R4-09`), with 995 controls for arithmetic,
  bitwise operations, checked rotations/shifts and endian codecs.
- Complete Flock/Relation reads (`R4-10`), with 119 controls for transported forests,
  five relation edges, cumulative synchronous sweeps, causal selection,
  actual-count consensus, virtual changes and trace projections.
- Composed Engine (`R4-11`), with 100 controls for exact seeded initialization, carried
  causal windows, full batch admission, all read modes, actual paired
  counterfactual reads and writer scores, and guarded geometric transitions.
- Complete SHA-256 and configured BLAKE2b-96 node identities (`R4-12`), with thirty
  independent controls covering full digests, padding, multiblock chaining,
  logical counters, byte order and typed admission.
- Complete Unicode and UTF-8 text primitives (`R4-13`), with 3,865 controls for the
  pinned Unicode predicates, scalar and byte admission, strict and lossy
  decoding, encoding, and the reversible tokenizer byte alphabet.
- Typed tokenizer admission and the full rank-BPE kernel (`R4-14`), with 127 controls
  for added tokens, pretokenization, rank ties, all-occurrence merging, Unicode
  overlap, complete byte-alphabet use, decoding and token-length policies.
- Complete imported mechanism catalog and evidence descriptions (`R4-15`), with 627
  controls for all 27 mechanisms, shared capability obligations, finite graph
  admission, exact identity roles, report classifications and admitted queries.
- Exact JSON number admission (`R4-16`), with 608 controls for integer bounds, decimal
  scanning, exponent saturation, rounding ties and finite-number classification.
- Complete bounded JSON parsing and tokenizer extraction (`R4-17`), with 399 controls for
  syntax, duplicate keys, string decoding, document revalidation and staged
  tokenizer admission.
- Complete Snapshot storage decoding and exact typed/original-Value encodings (`R4-18`),
  with 476 controls for schema, numeric domains, error ordering, strict duplicate
  handling, complete traversal and output budgets.
- Exact codebook integer operations (`R4-19`), with 2,404 controls for checked and wrapping
  arithmetic, bit operations, signed ordering and endian codecs at widths
  32, 64 and 128.
- Exact multi-digit radix-`B` orbit arithmetic, `17`-root Icosian golden-field $\mathbb{Z}[\phi]$ snap kernel, and integer PRLL mass conservation (`R4-20`, [`src/R4/ExactArithmetic.lex.tex`](src/R4/ExactArithmetic.lex.tex)).
- Bijective `256`-byte BPE vocabulary (`32,000` tokens) and exact round-trip decoding (`R4-21`, [`src/R4/RawByteTokenizer.lex.tex`](src/R4/RawByteTokenizer.lex.tex)).
- Unkeyed BLAKE3-256 content addressing and `64`-hex digest custody (`R4-22`, [`src/R4/CryptographicCustody.lex.tex`](src/R4/CryptographicCustody.lex.tex)).
- `19`-table `M1` discrete trunk (`12,288` bits/token, `4`-block base-`17` orbit root packing, Level-`3` joint pointer scoring; `R4-23`, [`src/R4/DiscreteTrunk.lex.tex`](src/R4/DiscreteTrunk.lex.tex)).
- `M3` scoped durable turn memory (`PrimeAddress` GCD intersection sieve, $+3\sigma$ `2144/4096` manifold sketch gate; `R4-24`, [`src/R4/DurableMemory.lex.tex`](src/R4/DurableMemory.lex.tex)).
- `G1`–`G13` structural gate verification and `S1`–`S15` surface contract audit (`R4-25`, [`src/R4/VerificationSuites.lex.tex`](src/R4/VerificationSuites.lex.tex)).
- Browser-native WebAssembly studio runtime, `UORLUT01` integer lookup table execution (`crates/wasm`), and `2B` geometric language model engine (`R4-26`, [`src/R4/BrowserRuntime.lex.tex`](src/R4/BrowserRuntime.lex.tex)).
- Integer lookup-table stack (`UORLUT01` header, `/5` Basin, `/6` Walk, `/7` TurnMemory, `/8` PointKey `28`-bit anchor synchronization; `R4-27`, [`src/R4/IntegerStack.lex.tex`](src/R4/IntegerStack.lex.tex)).
- Rule-30-modeled autonomous self-improvement engine, curriculum frontier, bounded Hebbian strategy plasticity, and live `.uorlut` weight compilation (`R4-28`, [`src/R4/AutonomousEngine.lex.tex`](src/R4/AutonomousEngine.lex.tex) and [`tests/lexlean/AutonomousEngineControls.lex.tex`](tests/lexlean/AutonomousEngineControls.lex.tex)).

## The engine: the work runs on GitHub Actions

Modeled directly on [`afflom/rule30`](https://github.com/afflom/rule30)'s autonomous research engine (`R4-28`), the repository runs a scheduled and event-driven self-improvement loop via [`.github/workflows/engine.yml`](.github/workflows/engine.yml) (`cron: "17,47 * * * *"`, `workflow_dispatch`, and `push` to `main`) backed by [`bin/r4`](bin/r4), [`frontier.toml`](frontier.toml), [`model/engine.toml`](model/engine.toml), and [`model/hypotheses.toml`](model/hypotheses.toml):

```text
bin/r4 status
bin/r4 brief
bin/r4 chain
bin/r4 claim <ID>
bin/r4 frontier [list|check|set]
bin/r4 discover [--write]
bin/r4 audit
bin/r4 deduce [--write]
bin/r4 engine status
bin/r4 engine tick [--budget-minutes 75] [--commit] [--push]
bin/r4 engine hypotheses
bin/r4 engine prune [--apply]
bin/r4 api check
bin/r4 api sync
```

Each tick of `bin/r4 engine tick` executes six stages inside the immutable PrismPM SDK image (`prismpm.lock`):

1. **Model & Gate Check**: Verifies the `28`-component LexLean register (`check-model`) and `CONFORMANCE.md`.
2. **Induction (`discover`)**: Scans dialogue stems, STEM reasoning, and code synthesis domains against `frontier.toml` and `model/hypotheses.toml` to admit `observed` curriculum claims.
3. **Pruning (`audit`)**: Evaluates `observed` claims against structural and `28`-bit `PointKey` invariants, refuting falsified hypotheses (`status = "refuted"`).
4. **Priority-Scheduled Deduction (`deduce`)**: Schedules strategies descending by bounded Hebbian weights $w \in [1, 16]$ in [`model/engine.toml`](model/engine.toml), verifies candidate turns (`0` token repetition cycles, `0` orphaned `/8` rows), promotes verified claims to `status = "proved"`, and **compiles proved dialogue/STEM/code claims directly into the served WebAssembly `.uorlut` binary** (`crates/wasm/src/bootstrap_v8.uorlut`).
5. **Hebbian Plasticity & Hypothesis Minting**: Updates strategy/family weights in [`model/engine.toml`](model/engine.toml) and mints successor hypotheses into [`model/hypotheses.toml`](model/hypotheses.toml).
6. **Pre-Commit Gate & Atomic Push**: Runs `check-model`, `audit-deferral`, `audit-limits`, and `cargo test -p lexlean-r4-wasm` before pushing an atomic `r4-engine` commit to `main` and dispatching [`pages.yml`](.github/workflows/pages.yml).

## Live application deployment

The WebAssembly studio (`crates/wasm` + `web/app.js`) is deployed on GitHub Pages at:
<https://casey-allard.github.io/lexlean-r4/>.

## Design and delivery

- [Approved full-scope design](docs/specs/2026-10-21-lexlean-r4-design.md)
- [Implementation sequence and acceptance gates](docs/plans/2026-10-21-lexlean-r4.md)
- [500B parameter scaling & self-improvement report](docs/reports/scaling-500b-self-improvement.md)
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
