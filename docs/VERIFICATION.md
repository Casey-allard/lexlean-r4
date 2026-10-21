# Verification record

This record concerns the configuration and subsequent formal development
slices on 2026-10-21. It is not full-template acceptance, an executable R4 engine,
research qualification, a trained artifact, or a Pages deployment.

## Environment

Formal verification used the public development SDK by immutable index:

```text
ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:60226bc791d4c0e5613402a6be7e63f4963d3faf7f327befcf56fc0e41d0ce21
```

The selected arm64 child is
`sha256:2f82a04e8f8141636e03a219d3960e30d98a6f304828341dd47b553b3c23baeb`.
Publication and upstream source identities are in [UPSTREAM.md](UPSTREAM.md).
The container uses the SDK's LexLean 0.3.0, Lean 4.32.1 and independent
leanchecker. Project language 1.1, builtins, compiler semantics and Lake
workspace inputs are recorded in `lexlean.lock`. These language pins do not
stand in for the absent production PrismPM/template bindings.

The first verification attempt rejected Lake's additional output while Lake
created its missing manifest (`LLV7001`). Initializing Lake, retaining its
manifest and regenerating the language lock resolved this setup problem.
The identity check and verifier were not modified.

## Development evidence

The first red source check reported `LLT4001`: the boundary tests referred to
`R4.Types::validSpec` before its implementation existed. After implementation,
LexLean checked and built two modules and independently verified all eleven
boundary theorems without axioms.

The Rust integration test freezes the reviewed independent literal theorem file
by SHA-256 before verification. It requests machine-readable verification, requires the
selected modules, binds the fresh four identity fields to the returned receipt,
requires each theorem's empty-axiom audit, and requires successful independent
kernel replay for both modules. It does not implement configuration logic.

The shared verifier additionally checks the receipt's copied build manifest
against its hash and length, then binds every non-lexicon input to current
workspace bytes. Canonical paths must stay inside the project. The selected
control file must occur exactly once with its reviewed digest, and the audit
must contain every unique control theorem. The build audit inspects the shared
helper as well as itself and each required entrypoint. The Types control digest
is `38671c44f31d5475abb570c6dc8c4b82f57fc7937277d9d35d0a63442a31be1a`.
After this refactoring, replacing `minimumSpec` with `0 = 0` again failed the
actual integration test (exit 101, `reviewed control statements changed`). The
original fixture was restored with its digest unchanged. Independent Rust and
security review found no blocking issue in this source-binding change.

The following representative defects were planted and restored:

| Defect | Command | Required rejection observed |
| --- | --- | --- |
| Increase vocabulary maximum from 8192 to 8193 in the model | `cargo test --offline --locked -p repo-conformance --test lexlean` | Exit 101; `LLV7002`: `decide` proved `validSpec 8193 2 1 1 = false` is false |
| Replace `minimumSpec`'s assertion with `0 = 0`, preserving its name | Same command | Exit 101; `missing or altered boundary statement minimumSpec` |
| Add `#[ignore]`, `#[cfg(any())]`, `#[cfg_attr(all(), ignore)]`, or `#[should_panic]` to the required Rust test | Same command, one mutation per run | Each fails the build-time acceptance audit with its rejected attribute |
| Add file-level `#![cfg(any())]` to the integration test | Same command | Fails the build-time acceptance audit before compilation can omit the test |

Each change was restored byte-for-byte. Review first reproduced the ignored-test
bypass against the inherited test-name scanner. The project-owned build audit
now parses the mandatory test and its own source, rejects conditional/ignored/
expected-panic attributes, and requires one top-level bare test entrypoint.
It does not certify arbitrary filtered Cargo invocations or changes that remove
the audit itself; reviewed source and the full unfiltered template command remain
the trust boundary. Subsequent workspace tests passed.
The receipt for the restored two-module source is:

| Identity | Value |
| --- | --- |
| Source | `33810a116b5ceb689b22b7c13c14c9df46ca35d2f4decb0252ae491dcdeaa317` |
| Semantics | `0b1442e7046d4d84a820788e3f5058cf106eb0cbfa5c8b6a80cafef5c8d38fa2` |
| Build | `1f4eddddd06a26c93c2754610c9513e56857d13c9cfeaa344110b6a7432c5522` |
| Attestation | `f6b4aee59c51d60c0b0855ba651833619c73ae48915d48b079fb5d4b35942936` |

These are observed development identities, not a replacement for rerunning the
verifier. Generated `.lexlean/` evidence is ignored; no compiled artifacts are
committed. The test never accepts an arbitrary old attestation from that directory.

After restoring the defects, the following development checks passed with
networking disconnected: `cargo fmt --all -- --check`,
`cargo clippy --offline --locked --workspace --all-targets -- -D warnings`,
`cargo test --offline --locked --workspace` (23 Rust tests, no ignored tests;
the bootstrap test also runs two Node checks), and
`cargo check --offline --locked --workspace --all-features --all-targets`.
`just model-write` regenerated `CONFORMANCE.md`. The broader `just model`
also stops at the missing SDK lock; its bootstrap checks were not waived.

## Finite geometry: R4-02

The [geometry verification record](GEOMETRY-VERIFICATION.md) records the 774
theorem controls, exact source and receipt identities, passing Cargo integration
test and three rejected composition defects. Independent specification and
code reviews approved the final source. The connected certificates cover every
discovery and transport step; the finite controls additionally cover closure,
inverse lookup and orbit preservation. These are static operator metadata.

## Scalar objective: R4-03

The registered scenario and controls preceded the implementation; the first
source check rejected the absent module with `LLR3005`. The completed source
has 91 independent theorem controls. Its receipt audits all 103 declarations
(12 source declarations and 91 controls) with empty observed axiom sets and
successful independent replay of both modules. The targeted Cargo integration
test passed with no ignored or filtered tests.

The objective preserves absent-rival hinge behavior, truncated rank violations,
policy selection, all three sentinel-aware product checks and both sum checks
before selection. Source-defined natural arithmetic avoids opaque SDK helper
reduction. The chunked scaling recursion computes the complete result for every
fuel value; zero fuel executes the complete residual. Controls cover all eight
residue classes, small and zero fuel, and every valid geometric width. See
[OBJECTIVE.md](OBJECTIVE.md) for the exact source-domain and integration limits.

| Planted defect | Command | Required rejection observed |
| --- | --- | --- |
| Remove the address sum overflow check, leaving the rank policy selected | `lexlean --diagnostic-format json verify tests/lexlean/ObjectiveControls.lex.tex` | Exit 1, `LLV7002` at `unselectedAddressSum` |
| Accept an address product exactly equal to the UInt64 sentinel by replacing `>=` with `>` | Same command | Exit 1, `LLV7002` at `productSentinelCollision` |

Both mutations were restored byte-for-byte. Independent specification and code
reviews approved source digest
`43af667dc55f173ef1629192156315ca90b7ff0f599f901250e21c42c5ea12ce`
and control digest
`9934a492d9d751ed0fde07f193b5a2582e70219b8a3f2eb72b595f955b2d77fc`.

| Identity | Value |
| --- | --- |
| Source | `58d64647419459772d06707063431108f16c0e00c2d8b7b4287f6e85263d6510` |
| Semantics | `27fcba8a8c815314317f38ed2f0a0764401a626f0100ffb24e5defb0e0ca8759` |
| Build | `9dce1d2819dbd3b46ee5300877839d76016b3d031fa92159bfd0f6ef17a56a39` |
| Attestation | `92cabf5852d44a8964c3e316475fb14b177fc5e178f1857e2bc31291c9bfc0dd` |

This establishes scalar formal evidence. Writer eligibility, policy-dependent
rank computation, caller status conversion, learning, GPU parity and performance
remain independent obligations. No model or training execution was run.

## Clean committed-source replay

Commit `44088c2` was exported with `git archive` into a new directory inside the
network-disconnected development container. This excluded uncommitted binder
work and all previously generated project proof/build artifacts. On that exact
snapshot, formatting, warning-denying Clippy, all workspace tests, all-feature
and all-target compilation, and generated-model equality passed. The run
contained 25 Rust tests and two Node checks, with none ignored or filtered.
Its three formal integration tests reverified the 876 theorem controls from
fresh project outputs. Dependency caches and the installed SDK remained shared;
this is a clean source/build replay, not a new SDK build or supply-chain audit.

## Complete acceptance remains blocked

`just vv` was invoked inside the candidate with networking disconnected. It
passed the model equality check, then rejected bootstrap acceptance:

```text
gate failed: prismpm.lock is required: No such file or directory (os error 2)
error: recipe `template-check` failed on line 17 with exit code 1
```

The universal gate is retained unchanged. Passing development checks does not
waive this failure. Production delivery needs the accepted SDK and matching
rendered template contract. The published development candidate cannot provide
the current template renderer. [The design](specs/2026-10-21-lexlean-r4-design.md)
also requires a generic GPU execution target and accepted generated browser
persistence. Those dependencies remain unfulfilled; no substitute runtime or
placeholder Pages application has been published.
