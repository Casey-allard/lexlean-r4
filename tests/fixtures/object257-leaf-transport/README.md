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

## Inventory-staging transport

The staging-positive phase carries the same accepted leaf equality through actual `ObjStageRecord inventory leaf limits`, preserving errors and arbitrary inventory/limits. This changes the equality operands from a record result to an inventory-list result. Its proof-local Prop family uses the original leaf equation at zero and the staging equation at successors; induction, predecessor cases and Congruence close the lifted theorem, which the consumer applies at one.

The real compiler accepted this proof. Independent review audited 769 declarations with empty observed axioms and 11 recorded kernel replays. The staging-false phase changes only the base Apply target and was rejected for the intended type mismatch. The first positive setup attempt failed while copying file ownership, before compiler invocation; the preserved receipt and review distinguish that setup failure from proof results. A fresh copy with identical18source/configuration hashes succeeded.

Select phases/staging-positive or phases/staging-false for these two additional controls; their roots and exact outcomes are in results.json. The default project remains parent transport. The inventory-staging theorem is local: it does not establish the full carry/fold sequence or the original whole-257 digest. The repository relocation remains byte-verified rather than rerun.

## Actual terminal fold transition

The terminal-positive phase proves an equation with actual `ObjFoldAtoms [[0]] frontier inventory limits` on the left. On the right, the certified leaf passes through `ObjStageRecord`, then actual `ObjCarryTriples 38` with root scale 0, then `ObjFoldAtoms []` using the carried frontier and inventory. The same bound leaf is staged and supplies the carry ID; errors propagate unchanged. Incoming frontier, inventory and limits remain arbitrary.

The real compiler accepted the consumer and lifted proof in 23,010 ms. Internal independent review checked 769 declarations with empty observed axioms and 11 recorded kernel replays. The terminal-false phase changes the base Apply to unrelated `0 = 0` and fails with the intended type mismatch. Select phases/terminal-positive or phases/terminal-false; the default project remains parent transport.

The auxiliary allocation probes captured the data disk but returned exit 1 for an absent guessed system-disk path. The failed probes and corrected postrun observation are preserved; the latter is not a retroactive preflight measurement. Independent host and guest capacity checks passed.

This establishes one real terminal transition, not the earlier transitions, concrete incoming state, atomization, whole-object SHA or original 257-byte digest theorem. The runtime definitions and expected digest are unchanged. Repository relocation is byte-verified, not rerun.
