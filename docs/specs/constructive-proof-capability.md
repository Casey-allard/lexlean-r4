# Route induction: official 0865 constructive-proof capability review

2026-10-21; source-only review. No compiler execution, source algorithm change, generated-Lean edit, or axiom-policy change.

## Conclusion

The retained positive theorem is mathematically constructive, but no supported, justified none-axiom expression of its remaining induction step was found in the official 0865 closed proof schema. The current evidence establishes a proof-interface dependency, not a false theorem or a need to change Route/Admission algorithms. Do not repeat speculative `simp`/`omega` wrappers or count successful elaboration as verification.

A minimal generic upstream capability is definitional conversion/unfolding followed by checked equality congruence using a local induction hypothesis, with kernel checking and the existing none-axiom audit. It must be supplied by an officially identified SDK; it is not an application-local compiler patch or raw-Lean escape hatch.

## Exact authority and observed evidence

Official LexLean revision: `0865df4e540ca643b090a29e9df6d319d6906da6`.

- Retained semantic IR: `/tmp/r4-0865-semantic-ir.rs`, SHA256 `e273dcc28a3d1b426bb97bef43cb426e2ccb7cbd205aab2a3b4ccf38c5623fbf`.
- Retained semantic backend: `/tmp/r4-0865-semantic-backend.rs`, SHA256 `d747f2819115fa4fb31b4c9c6e77ba0b0b26705d93a7f75ada209ca27f76295e`.
- Parent-retained complete probe evidence: `/Users/casey.allard/lexlean-r4-evidence/2026-10-21/prediction/route-general-probes/`.
- Positive obligation is unchanged: the length of actual Admission.identitiesTokens tokens under actual Route.lengthNat equals actual Admission.lengthTokens tokens, universally over the token list.
- Original `Induction` plus terminal `Congruence` sourcechecks but leaves the cons goal unsolved (`LLV7002`).
- Named-definition `LinearArithmetic` sourcechecks and closes elaboration, then fails `LLV7005`: `routeLengthOfTokenIdentities` observes `[Quot.sound, propext]` under none policy. This is read directly from the retained `r4-route-linear-positive-verify.json`.
- False companion remains source-valid and fails its nil equality `0 = 1` (`LLV7002`). It does not make the positive result acceptable.

## Constructive mathematical step

Let `a = Route.lengthNat (Admission.identitiesTokens tail)` and `b = Admission.lengthTokens tail`. The cons branch has local induction hypothesis `a = b`. Definitional unfolding of the three actual recurrences changes its goal to `1 + a = 1 + b`.

Equality elimination/congruence for the ordinary function `n ↦ 1+n` supplies that goal from the induction hypothesis. This uses neither arithmetic decision procedures, quotient soundness, nor propositional extensionality. It does not change the recurrence to a new successor-based implementation. The nil branch remains definitional reflexivity.

This paragraph describes the required proof rule, not an authored or verified Lean proof artifact.

## What the closed schema actually permits

The complete `SemanticProof` enum is at IR lines 609–655; backend rendering is at lines 1105–1254.

- `Reflexivity` emits terminal `rfl`; it does not consume the local equality.
- `Decide` emits terminal `decide`; it supplies no reduction step followed by use of the induction hypothesis for an open arbitrary list.
- `Congruence` emits exactly terminal `congr 1` (backend line 1188). It has no named definitions, continuation, or explicit hypothesis argument. The actual unchanged goal has already demonstrated that this form is insufficient here.
- `Simplify` emits `simp only [named...]`. It can name local Prop hypotheses (IR 7875–7895), but there is no constructive equality-rewrite selection or axiom guarantee. Earlier isolated imported-equality probes already observed `propext`; this is not an established lawful fallback.
- `LinearArithmetic` emits intros, substitution, `simp only [...] at *`, and `omega`. The current positive theorem empirically violates the none policy. The source does not justify assigning each observed axiom to a specific individual internal step; the aggregate generated proof is rejected.
- `Apply` checks a MemberRef against `env.proof_rules`, requiring a prior document theorem and checked arguments (IR 7814–7871); it emits `exact priorTheorem arguments`. It cannot directly name the local induction hypothesis or arbitrary `congrArg`/`Eq.rec` as an unregistered theorem. Adding an application helper without proving its equality-elimination premise would merely move the same dependency.
- `Cases` and `Induction` accept Nat, List, or registered document inductives (IR 8053–8177). List induction binds its IH internally as SemanticType::Prop. `Cases` on that IH is rejected: Prop is not one of the supported elimination scrutinee types. Naming a document inductive `Eq` would not identify the built-in equality proof or create proof transport.
- `Constructor` sequences subproofs only for constructor-generated goals; it is not a generic tactic sequence or definitional conversion wrapper.
- `BooleanReflection` is a fixed natural-deduction bridge for particular list/record Boolean predicates, with constrained Nat/List-Nat/record inputs (IR 7956–8050; backend 1189–1236). The current equality-of-lengths goal is not that bridge. Replacing the theorem with a reflected, unrelated proposition would not prove the required obligation.
- Semantic terms contain data lambdas/applications and proposition syntax, but no general checked proof-term `Eq.rec`, local proof application, or equality congruence constructor. `SemanticType::Prop` denotes the type of propositions; it is not a dependent binder for a proof of a specified equality.

This is a source-bound limitation finding for the needed path, not a proof that no arbitrarily elaborate encoding can ever be written in the language.

## Exact upstream requirement and acceptance gate

Request a generic closed, typed proof capability supporting both:

1. Definitional conversion/unfolding of a sorted list of existing named definitions with a nested proof continuation; the new target must be definitionally equal to the old target.
2. Local equality use via equality elimination or congruence, with checked local hypothesis and an existing typed pure function/lambda. For this case, its inferred conclusion is `f a = f b`, with `f(n)=1+n`, checked against the converted target. No arbitrary source text, asserted equality, generated proof file, or caller-provided success flag is admitted.

An equivalent general natural-deduction proof-term design is acceptable if it checks the same typing and scope obligations. A specialized Route theorem recognizer is unnecessary and would not supply the broader Prediction proof infrastructure.

Before depending on an upstream implementation: retain its official source/executable/semantic identities; rerun this exact positive and false companion; require complete none-axiom audits and fresh module kernel replays; additionally reject missing/wrong-typed hypotheses, nonconvertible target changes, and mismatched congruence functions. Preserve existing source/control statements and original negative evidence. This small theorem remains only an enabling diagnostic; complete target-free prediction projection, success, and ranking proofs are separate mandatory obligations.
