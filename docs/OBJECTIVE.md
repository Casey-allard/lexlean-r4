# Scalar objective model

`src/R4/Objective.lex.tex` owns the scalar arithmetic used to construct the
pinned discrete writer's two objectives. `R4-03` is a formal arithmetic slice;
it does not implement admission, token identity, the complete writer, or learning.

For a present rival the hinge is the natural truncated difference
`margin + targetDistance - rivalDistance`; absence produces zero. The explicit
absence flag is an abstract input. The future writer must derive it from the
source's `UINT32_MAX` rival sentinel and keep the separate rival count. Zero
hinge must never be interpreted as evidence that a supervised comparison exists.
Natural hinge arithmetic agrees with the source's UInt32 additions on the
validated geometric domain where those additions cannot overflow.

For token hinge T, address hinge A, rank violation R and width W:

- Nearest-address objective: `8T + WA`.
- Address-rank objective: `8T + WR`.
- Rank violation: `max(rank - keptCells, 0)`, with one-based rank.

The model checks all three products and both sums before selecting either
policy. A product equal to `UINT64_MAX` is a sentinel collision and produces
`WeightedOutcome.arithmeticOverflow`, as does a larger product or sum. A sum
exactly equal to that maximum passes this scalar arithmetic check; the original
callers subsequently interpret a selected maximum loss as failure. That sum
collision is unreachable for valid geometric widths, since all terms are
multiples of eight. The generic helper controls intentionally include wider
arithmetic cases to distinguish these behaviors.

The scalar wrapper models the source word domains explicitly. Hinge inputs
larger than UInt64 or a width larger than UInt32 produce
`WeightedOutcome.invalidOperand`. These are typed admission failures for the
natural-number model; the original CUDA parameter types impose those limits
before the function is entered. `ObjectivePolicy` has exactly two constructors.
Actual application widths must still come from validated `Spec`: 16 through
128 in multiples of eight. Generic word-domain arithmetic is not a valid
configuration admission path.

Policy 0 without telemetry leaves the source writer's rank at one; policy 1
or telemetry computes the actual rank. This scalar function accepts an explicit
rank violation and does not change that future writer obligation. Baseline
eligibility, a candidate losing its supervised rival, and whole-batch accumulation
remain separate learning contracts.

## Proof implementation and limits

The pinned development SDK emits unexposed helper bodies for primitive
subtraction and multiplication. Imported proofs could not reduce them. This
module therefore defines its natural arithmetic directly in LexLean. It uses
structural predecessor/subtraction and an addition accumulator. To stay within
the unchanged kernel's reduction depth, scaling consumes eight multipliers per
chunk. Its zero-fuel branch computes the complete residual multiplication with
the linear accumulator; it never substitutes a default result. The public
wrapper supplies fuel equal to the multiplier. Every recursive call decreases
the explicitly declared structural argument.

This representation has linear work in a generic multiplier. It is formal
source development, not a claim of CUDA-equivalent cost, a production backend,
or D11 serving compliance. The model contains no learned dense operation,
floating-point arithmetic, or squashing operation.

The 91 independently specified controls cover hinge/rank boundaries, both
policies, unselected-branch overflow, product and sum sentinel distinctions,
word-domain rejection, every residual size from zero through sixteen at three
fuel values, and all fifteen permitted geometric widths. Their exact source
bytes are bound by the conformance test before the SDK independently verifies
the entire selected graph. No changed expected constant or additional axiom was
needed to resolve the compiler reduction limits.
