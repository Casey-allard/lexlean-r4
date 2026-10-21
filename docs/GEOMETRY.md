# Exact finite geometry

`src/R4/Geometry.lex.tex` owns the finite construction; its control source is
`tests/lexlean/GeometryControls.lex.tex`. This slice models static operator
metadata. It does not execute a learned carrier, train, run parity evaluation,
or establish a production serving implementation.

## Representation and failure behavior

An `Index` has exactly eight constructors. An `Axis` contains an `Index` and a
sign. An `Action` has eight output-row selectors. `validAction` checks pairwise
distinct source indices; the finite index type supplies the range bound.
`admitAction` and `composeChecked` return `Option Action`: invalid permutations
produce `none`. `decodeAxis` accepts precisely the sixteen encoded selectors;
its output is `Option Axis`, with no default index for malformed input.

A finite `Coordinate` is negative two, zero, or positive two. A `Root` has eight
such coordinates. This exactly represents the selected orbit; it is not the
representation of general accumulated model carriers. Coordinate negation is a
closed constructor match. Neither this model nor its verifier invokes matrix
multiplication, floating-point arithmetic, or a surrogate learned model.

`compose(left,right)` means applying `right` first and then `left`. It gathers
through the right action using the left selector, and combines signs by XOR.
`apply` uses the same output-row gather convention on finite roots. Public
state/root lookup functions and indexed composition/transport retain `Option`
failure for absent entries. A root with legal coordinate values but outside the
sixteen-element orbit is rejected by `rootIndex`.
`inverseState` searches the finite action inventory for a two-sided inverse and
also retains `Option` failure for an invalid state index.

## Exact ordering

The six left actions preserve the pinned oriented Fano cycles. The fifteen
bivectors are composed in lexicographic order of pairs from those six actions.
The sixty-four state definitions refer to earlier states and generators, with
identity at index zero, generator states at indices one through fifteen, and
negative identity at index sixteen.

Action discovery uses FIFO expansion and **right composition** of each current
action with the ordered generators. Root transport uses FIFO expansion and
**left composition** of each generator with the accumulated path. Both retain
first discovery. These distinct orders are part of the model.

The root list puts `[2,2,0,0,0,0,0,0]` first, followed by the other fifteen roots
in signed numeric lexicographic order. It is the selected sixteen-root orbit,
not the complete E8 root system. Ternary branch ports apply identity, the first
bivector, and its inverse, in that order.

## Formal certificate construction

The source supplies the actual discovery/transport transition functions and
closed finite traces. Every trace state contains the complete ordered `seen`
and `queue` fields. Controls prove direct record equality for every transition:
64 discovery steps and 16 transport steps for each of the sixteen starting
roots. Separate theorems establish initial seeds, the final ordered action
inventory, empty final queues, complete root reachability and correct transport.
Seventeen composed conjunction certificates apply all the corresponding
transition and endpoint theorems. There are no missing steps or independently
unconnected examples in these certificates.

The final checked transport states supply the finite metadata table. They are
LexLean model data validated against the modeled algorithm, not an external
handwritten runtime. There is no second monolithic loop hidden behind the
certificates.

Controls additionally cover all 4,096 pairwise compositions through 64 closure
rows; all 64 two-sided inverses; independent literal generator, root, neighbor,
and branch-port vectors; noncommutative order-sensitive compositions; all
sixteen selector decodes; and malformed selectors, duplicate axes, invalid
indices, and roots outside the orbit. These are finite construction obligations,
not a general theorem about arbitrary learned carriers or an empirical claim.
The controls also check every inverse lookup and all 1,024 action/root pairs
for preservation of the selected orbit.

The first monolithic discovery proof reached Lean's default reduction-depth
limit. Decomposing it into the connected transition certificates preserved the
full obligation without changing the SDK or increasing its limits. Balanced
conjunction trees also preserve the same obligations within the JSON parser's
nesting limit.

## Provenance

The upstream authority is UOR-R4 commit
`4611538bb4843947363ccb9cce5f388e32c46883`:

| Source | SHA-256 |
| --- | --- |
| `crates/uor-r4-core/src/spiralcore_operator.rs` | `1e17032031620e87d705b56a4529f4d04488a613449ec7cb0a9a7ccb862e744b` |
| `crates/uor-r4-discrete/build.rs` | `5353d7dda155244147f77dbcf7ae06b3ac67a98007b6f40a7815894870b13427` |

The imported authority and local R4-02 claim remain separate in the template
registers. Development proof evidence does not resolve the production SDK,
GPU-target or browser-runtime dependencies described in [UPSTREAM.md](UPSTREAM.md).
