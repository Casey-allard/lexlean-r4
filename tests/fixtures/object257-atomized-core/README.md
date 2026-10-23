# Original257 atomized fold-core bridge: source candidate

**All new native checks and proofs are NOT_RUN.** This candidate tests exactly

`ObjFoldAtoms (ObjAtomize I [] 0) [] [] L = ObjFoldAtoms (ObjAtomize R [] 0) S.frontier S.inventory L`.

I is the unchanged original257 input expression. R is its remaining 225-byte
suffix, bytes 32 through 255 followed by 0. S is the actual carry-state record
inside the accepted first-state theorem's `Result.ok`; L and the entire base
equality are copied unchanged. No new expected node hash, state certificate or
runtime implementation is introduced.

The suffix preserves the original bounded `ObjAppendBytes` source tree and
32/33-byte literal leaves, removing only the first 32-byte leaf and collapsing
its append. It does not use a deeply nested flat 225-cons JSON chain. Structural
checks compare all suffix bytes and the original input AST. Neither source
decoding nor the preserved expression is claimed as native atomization proof.

The context K term-matches a carry-state Result: errors propagate unchanged;
success calls the real suffix atomizer and fold with the carried fields. The Nat
Prop family and Induction/Cases/Congruence/Apply-at-1 proof use the already tested
constructors. The actual production consumer is mandatory; proving only the
transported K equality is insufficient. Nested-match and original atomization
conversion, and their resource cost, remain unestablished.

The negative was authored first. Both phase modules contain the same unrelated
`0 = 0` theorem. Only the lifted zero-base Apply target differs: the negative
selects that theorem instead of the accepted first-state theorem. A parser,
canonicality, dependency, timeout or OOM failure does not qualify as the intended
base mismatch. All semantic JSON is recursively key-sorted and byte-checked.

## Portable staging and execution boundary

`candidate.json` gives exact repository-relative source/hash/destination mappings
for the 17 unchanged shared inputs and three new modules. Reuse the accepted
runtime and first-state closure by reference; never overwrite original sources
or failed stages. Historical absolute evidence references in the proposal have
verified repository aliases in the candidate. They are not runtime dependencies.

Select `phases/atomized-false/lexlean.toml` or
`phases/atomized-positive/lexlean.toml` in a new owned stage. Configurations change
only the prior accepted entrypoint and retain 300,000 ms and all original limits.
After independent source review and fresh identity/process/storage checks, the
parent generates genuine development locks with the installed official compiler.
No lock is fabricated here.

| Phase | Source check | Verification |
| --- | --- | --- |
| `atomized-false` | `lexlean --diagnostic-format json check tests/lexlean/Object257AtomizedCoreFalseBase.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257AtomizedCoreFalseBase.lex.tex` |
| `atomized-positive` | `lexlean --diagnostic-format json check tests/lexlean/Object257AtomizedCore.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257AtomizedCore.lex.tex` |

Preserve actual diagnostics, source/lock identities, resource counters and all
postconditions. No automatic larger-limit or equivalent retry follows from a
failure. Native success requires the production consumer, empty observed axioms,
recorded kernel replay and independent artifacts. Even success would establish
only this exact fold-core prefix relation: outer admission, remainder evaluation,
parents/carries, whole-object finish, the original digest, complete Hash/Object
qualification, production PrismPM and Pages remain open.
