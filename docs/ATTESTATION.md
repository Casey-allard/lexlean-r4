# Formal control receipt compatibility

The common Rust control gate binds the reviewed source digest before invoking
LexLean. Its semantic-module schema selects the only accepted language and
receipt pair: semantic-module/1 requires language 1.1 and attestation/1;
semantic-module/2 requires language 1.2 and attestation/2. Unknown, missing,
mistyped or crossed versions fail. This repository does not use language 1.0.

This follows the exact official LexLean revision
`0865df4e540ca643b090a29e9df6d319d6906da6`,
[`crates/lexlean/src/verify/mod.rs`](https://github.com/UOR-Foundation/LexLean/blob/0865df4e540ca643b090a29e9df6d319d6906da6/crates/lexlean/src/verify/mod.rs#L2215)
(source SHA-256 `487a582ffc55ce12e204729191bc3a36baad521a4e2e5eb90b9774f88bd02b29`).
The pinned attestation schemas have identical shared fields apart from the
version constant. Version 2 additionally permits optional `compiler_input`
and `preservation` digests for production roots. Ordinary formal-control
receipts need neither. The common control gate rejects either field's presence,
including null, until a separate production gate validates those artifacts.
Receipt version 2 does not establish production acceptance.

All existing identity, complete selected control coverage, none-axiom policy,
independent kernel replay, build-manifest digest and current source-input
checks remain required. Compatibility does not relax those checks.

## Observed repair evidence

The staged 84-module formal verification succeeded, but its actual Number
integration test failed at the former hardcoded attestation/1 assertion when
the official SDK returned attestation/2. This was a gate compatibility failure,
not a failed mathematical equation. Before the repair, the new binding test
failed to compile because the required validator was absent (E0425).

In the official SDK construction, the filtered Rust test
`cargo test -p repo-conformance --test json_number receipt_language_binding`
then passed. Three single-condition defects independently disabled receipt
version, language, and production-evidence rejection. Each compiled and failed
that test with exit 101. Restoring the exact reviewed source passed with exit 0.
Independent Rust, code and security review accepted the narrow change.

These filtered runs qualify the validator repair only. They are not the full
Number acceptance target, the complete project gate, a language migration
acceptance, or production qualification. Those checks remain separately required.
