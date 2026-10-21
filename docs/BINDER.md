# Ordered ternary binder metadata

`src/R4/Binder.lex.tex` owns canonical binder construction, independent general
forest validation, and four-word node serialization for `R4-04`. Its authority is
`crates/uor-r4-discrete/src/metadata.rs` at UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`, SHA-256
`27e53e4feecb391adb4a4db3462712a8d9375aed09c0e57086581134cb6e1e32`.
Every algorithm in this slice is LexLean source. The Rust conformance entrypoint
checks the frozen independent controls and invokes the digest-pinned verifier.

## Construction and validation are distinct

Canonical construction admits contexts 1 through 243. Each atom appends a level
zero leaf. Whenever the newest three frontier roots have equal levels, their
ordered triple becomes the next node at the next level, and closure continues.
Node identifiers are indices in the append order. The final frontier retains its
original order, including incomplete tails; stages are one plus the greatest
level. Thus three atoms produce leaves 0, 1, 2 followed by parent 3, while nine
atoms end at parent 12 with children 3, 7, 11.

The general validator deliberately accepts more than the canonical constructor
produces. Three separate leaves with frontier `[0, 1, 2]`, context 3, and one
stage are valid. It never reconstructs the canonical plan and compares against
it. Construction and validation are independently callable in this model;
the upstream constructor's final self-validation is represented by separate
construction/validation controls, not an additional call inside `construct`.

Validation preserves the pinned source's first-error order:

1. `inventory`: context outside 1 through 243, no nodes, no frontier, or more
   than sixteen frontier entries.
2. The node scan proceeds in append order. `nodeShape` rejects a node that is
   neither the next level-zero atom nor a positive-level complete triple.
   `childLink` rejects a child that is not prior, is at the wrong level, or has
   already been consumed. The three children are checked in their listed order,
   updating consumption between them, so a repeated child in one triple fails.
3. `contextStages`: atom count or greatest-level-plus-one disagrees with the
   plan's context or stages.
4. `frontierRoot`: a frontier entry is outside the node inventory, repeated, or
   already consumed by a parent.
5. `omittedRoot`: the frontier is not exactly the set of unconsumed nodes.
6. `atomOrder`: left-to-right forest traversal loses sequential atom identity.
7. `atomCoverage`: completed traversal does not cover the context.

The model replaces source array marks with exact list membership and retains
the same ordering of observable error classes. Traversal has structural fuel
equal to the node count. The earlier checks establish prior-only child edges,
one parent per child, and exactly all unconsumed roots. Consequently traversal
visits each node once and this fuel covers the entire validated forest. Direct
helper calls with insufficient fuel return a typed coverage failure. That helper
failure is not an extra rejection condition for a structurally valid forest.

## Representation and words

Public indices, atoms, levels, and counts use natural numbers. Admitted plans
have at most 243 atoms and 364 nodes, so their indices and atoms fit the source's
UInt32 fields; their levels also fit the source's machine-size integers. The
natural model does not emulate malformed machine-word wraparound. Serialization
emits each node as its three child indices followed by its atom index, using
`4294967295` for each absent field. It does not serialize levels, add padding, or
invent implicit nodes. The low-level `nodeWords` helper is a projection and does
not validate its input; callers that accept external plans must validate first.

Construction accumulates nodes and frontier in reverse order, restoring public
order at completion. Its internal node count is a little-endian list of binary
digits. Structural increment carries through true digits; decoding doubles the
higher value by addition and adds the low digit. Starting from the empty zero
counter preserves canonical digits. Exposed helpers also have defined numeric
semantics for leading zero digits, covered by controls.

The pinned verifier exceeded its unchanged reduction depth when the largest
closure retained a deferred chain of natural-number increments. The binary
counter and exhaustive decomposition/reconstruction of its first digit after
each atom insertion keep reduction shallow. This reconstruction is the identity
on the state. No expected value, theorem statement, compiler limit, or axiom was
changed to make the boundary pass.

Construction consumes three atoms per structural fuel step. Counting and
reversal consume eight list entries per step. Every zero-fuel branch computes
the complete residual through its linear structural implementation; none returns
a default or truncates input. The fixed counting/reversal chunk budget is an
evaluation strategy, not an accepted-inventory limit. These metadata algorithms
make no backend performance or serving-throughput claim.

## Evidence and remaining scope

The 165 independent formal controls include exact complete plans and successful
validation for contexts 1, 2, 3, 4, 6, 8, and 9; rejection of context 0 and 244;
valid noncanonical forests; malformed identities, child links, stages, roots,
coverage and traversal order; competing-error precedence; exact serialized
words; and count/stage/frontier boundaries at 27, 81, 242, and 243 atoms. The
largest boundary has 364 nodes, six stages, and frontier `[363]`. Additional
controls exercise binary carries through 255/256 and 363/364, leading zeros,
all construction residuals zero through nine at three fuel values, and list
count/reversal residuals zero through nine with nonempty accumulators and both
zero and nonzero fuel.

These are finite compiler-checked statements, not a universal theorem that every
admitted context validates. They establish neither carrier computation nor GPU
qualification, learning, generalization, Route/Flock behavior, window-plan
framing, integer serving, or a browser execution backend. Those remain separate
modeled capabilities and delivery obligations.
