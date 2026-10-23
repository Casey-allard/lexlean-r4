# Verified Object leaf and parent-context proofs

This fixture preserves actual LexLean source for the final partial leaf of the 257-byte Object example, direct reuse of its certificate, and transport into the production scale-1 parent operation. The two negative modules are intentional failures. The five source checks and verification outcomes are recorded in results.json; independent reviews and emitted proof text are in evidence/.

The positive parent proof uses induction: the zero branch applies the leaf certificate, the successor uses Congruence, and the consumer applies the lifted theorem at one. All reviewed declarations have empty observed axioms, with official kernel replay recorded. The expected record is a theorem conclusion, not an implementation input.

## Scope

This is a finite development fixture, not the canonical Object implementation or a production acceptance path. Its exact runtime source closure differs from the current working Object modules. Keeping it self-contained preserves reproducibility without replacing those unfinished modules. It does not establish the original whole-257 digest theorem, all Object controls, Hash qualification, PrismPM integration or Pages. No model claim or generated conformance status changes.

The compiler-generated lexlean.lock files are genuine development locks from the recorded stages. They are not production prismpm.lock or template.lock files. No compiler implementation or compiled artifact is vendored. Full verified bundles remain in the evidence location recorded by results.json, bound by their attestation IDs and independent review hashes.

## Replaying a phase

Use the exact recorded compiler/toolchain in an owned workspace with the documented storage and memory budget. Copy project/ into that workspace, then copy the selected phases/<phase>/lexlean.toml and lexlean.lock over its default files. For a fresh run, generate a genuine lock and retain any identity change; never hand-edit acceptance into a lock. From that workspace run:

```sh
lexlean --diagnostic-format json lock
lexlean --diagnostic-format json check
lexlean --diagnostic-format json verify tests/lexlean/Object257ParentTransport.lex.tex
```

Replace the final root with the phase root listed in results.json. The default project selects transport-positive. The leaf-false phase must reject expected ID byte129; transport-false must reject the unrelated0=0 base theorem. Timeout, OOM or source-admission failure is not an accepted negative. Run phases sequentially without overlapping compiler jobs.

The original stages were executed and reviewed; this repository relocation was verified byte-for-byte, not rerun under its new path. manifest.json inventories the preserved source and evidence.
