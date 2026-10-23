# Original Object257 nonempty continuation: source candidate

**All new native checks and proofs are NOT_RUN.** This candidate tests one
actual production continuation equality for the original first two atoms:

`ObjFoldAtoms [A,B] [] [] L = ObjFoldAtoms [B] S.frontier S.inventory L`.

Here A is bytes 0–31, B is bytes 32–63, and S/L are copied from the independently
verified first-atom state. No second-leaf ID, expected second state, parent
certificate or new runtime definition is introduced.

The proof-local context K term-matches a `Result ObjCarryState ObjError`, returns
an error unchanged, and on success calls the real `ObjFoldAtoms [B]` with the
carried frontier/inventory and original limits. Its Nat Prop family has the exact
accepted first-state equality at zero and K applied to both sides at successor.
The lifted proof uses the previously supported Nat Induction/Cases/Congruence
shape; the actual consumer applies it at 1. There is no proof Cases on Result.

The negative source was authored first. The same unrelated `0 = 0` theorem
appears in both phase modules. Their semantic data differs only at the zero-base
Apply theorem reference: the negative selects that unrelated theorem, while the
positive selects the accepted first-state theorem. All terms and the production
consumer remain identical. An import, prior-proof, resource or parser failure
does not count as the intended base type-mismatch negative.

## Portable phase preparation

`candidate.json` lists 17 shared stage inputs with repository-relative source
paths, exact hashes and stage-relative destinations: the ten runtime modules,
accepted terminal context, three first-state modules and three Lean workspace
files. The three new modules have matching `newStageInputs` mappings. Rehash all
inputs and copy them into an owned stage; do not modify shared fixture sources
or overwrite the original working drafts. No runtime or artifact tree is copied
into this fixture.

Select `phases/continuation-false/lexlean.toml` or
`phases/continuation-positive/lexlean.toml` as the stage's `lexlean.toml`. These
change only the entrypoint from the accepted first-state configuration, retaining
all source/language/Lean settings and the 300,000 ms child limit. The parent must
generate genuine development locks with the installed official compiler after
source review and fresh identity/process/storage checks. No lock is fabricated
or supplied by this source candidate.

Run the negative root first, then the positive root, from the prepared stage:

| Phase | Source check | Verification |
| --- | --- | --- |
| `continuation-false` | `lexlean --diagnostic-format json check tests/lexlean/Object257NonemptyContinuationFalseBase.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257NonemptyContinuationFalseBase.lex.tex` |
| `continuation-positive` | `lexlean --diagnostic-format json check tests/lexlean/Object257NonemptyContinuation.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257NonemptyContinuation.lex.tex` |

K(F) surrounds a completed one-atom fold with a match; the actual two-atom fold
nests its continuation inside production leaf/stage/carry matches. Their concrete
conversion is the unresolved obligation. **A passing lifted helper with a failing
production consumer leaves this bridge incomplete.** Verification may recompute
the first leaf hash or unfold B. No performance, caching or avoided-computation
claim follows from source inspection. Preserve exact emitted failures and resource
observations; do not automatically increase limits or retry an equivalent failure.

Native success requires the actual consumer, empty observed axioms, recorded
kernel replay and independent artifact review. Even success would establish
only this finite relation, not a second concrete state, general associativity,
full original257 atomization/fold/digest, Object/Hash qualification, production
PrismPM/template acceptance or Pages deployment.
