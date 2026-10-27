# Second-round source candidate

This bounded candidate contains one full sixteen-word second-round certificate, its wrong-s0 companion, and the mandatory original production CompressRaw consumer. All three official compiler phases are **NOT_RUN**. Host source checks and the numeric derivation do not establish a theorem.

The positive statement is `RoundRaw(R1, Permute(M0)) = R2`, where R1 is the exact AST of the accepted first-round certificate and M0 is BlockWords of original bytes0..63. The negative changes only expected s0 by one modulo2^32. Runtime definitions, IV/counter0/length64/flags1, digest, empty axioms and 300000ms limits are unchanged.

The consumer keeps the exact original CompressRaw left side and exposes `FeedForward(Rounds(5,R2,Permute(Permute(M0))),IV)` on the right. Its proof applies the accepted original first-round consumer with no arguments. Importing the new certificate does not rewrite the goal or perform transitivity. Kernel definitional conversion must establish the substitution again and might recompute more rounds; its success and cost are unproved. A helper-only result cannot accept this candidate.

The production AST evaluator in derive-expected.py is reused narrowly for one round with pinned R1 and the one-permuted message. derivation.json retains its actual input/output and remains untrusted authoring data until the official reflexivity proof passes. No supplied CV or host hash library is accepted as proof.

The phase order is round2-negative, independently audited round2-positive, then independently audited original-consumer, each separately selected after fresh source/invocation/resource gates. Candidate metadata records4/4/9 modules,8/8/13 staged inputs and exact bytes. Shared runtime/workspace inputs and accepted first-round closure are pinned by committed reference, not copied broadly. Every phase needs an official genuine development lock; none is manufactured here.

Source checks retain four predecessor shape failures, exact successor checks, a missing-second-permutation structural mutant rejection and identical same-source authoring re-evaluation. These are source preparation controls only, not semantic negatives or native results. Accepted Boolean wrapper/glossaries, direct imports, local reference rules and canonical JSON are checked.

No further round batch, full compression literal, S128, original2048 digest, count increase, efficiency/OOM remedy, production PrismPM binding or Pages acceptance is claimed. Independent source/code/security review and a separately reviewed invocation must precede native execution. Preserve the first-round source-admission failure and second-state timeout through their committed checkpoints.
