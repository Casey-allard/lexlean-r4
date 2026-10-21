# Verification record

This record concerns the first configuration-contract slice, **R4-01**, on
2026-10-21. It is not full-template acceptance, an executable R4 engine,
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

The Rust integration test checks the exact theorem statements against independent
literal boundary cases. It requests machine-readable verification, requires the
selected modules, binds the fresh four identity fields to the returned receipt,
requires each theorem's empty-axiom audit, and requires successful independent
kernel replay for both modules. It does not implement configuration logic.

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
