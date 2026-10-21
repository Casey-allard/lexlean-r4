# Codebook construction and typed artifact refinement

Design review dated 2026-10-21. This elaborates the approved complete scope in
[codebook-design.md](codebook-design.md); it is not implementation acceptance,
a platform-parity result, or authorization for model execution. No constructor,
model, training, or GPU evaluation was executed for this review. Integer
precursors are useful complete components, but do not replace the required
PrimeZeta floating construction or its composition with A8.

## Pinned authorities

All UOR line numbers refer to commit
`4611538bb4843947363ccb9cce5f388e32c46883`, read through `git show` independently
of working-tree changes. Links select that immutable source.

| Authority | SHA-256 | Relevant lines |
| --- | --- | --- |
| [prime_zeta.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/prime_zeta.rs) | `2f1ab07fe6094be6c3887c3b700868f89f8a63fcf311652c16ed4227be8b4e54` | 48–138 numeric wrappers/primes; 211–266 bytes/permutation/embedding; 275–493 construction |
| [address_cells.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/address_cells.rs) | `22dec8e967f7d2bfd410f83067ee69646993bffe0aa3be4996375896e33fa75e` | 51–125 counts/bytes/distance/shuffle; 135–241 placement/table file |
| [zeta_zeros.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-core/src/zeta_zeros.rs) | `abd8012d6760b07338d72bb306b92745d34af6ded44b8320b35839f8fe739920` | complete 512-ordinate literal inventory |
| [training.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/training.rs) | `a189d3c9db4cf1e2d4e19be3264a2a36029c7fc88dae8b0bbc414f89e49049bf` | 97–176 pinned initialization and typed table loader |
| [lib.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/lib.rs) | `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9` | 56–61 typed Spec fields and unknown-field policy |
| [corpus.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/corpus.rs) | `826bdaba978b3c54542147656c8aa3cf24d1aa39b6711528b02b545c4fda9f7e` | 20–47 lowercase digest admission, file read, actual raw-byte digest check |

The inspected SDK is LexLean `0865df4e540ca643b090a29e9df6d319d6906da6`
vendored by PrismPM `dfdc4df770d038ee5fb700d374aca883d1c9ec7a`.
Its `vendor/lexlean/src/ir/semantic.rs` SHA-256 is
`e273dcc28a3d1b426bb97bef43cb426e2ccb7cbd205aab2a3b4ccf38c5623fbf`;
`vendor/lexlean/src/backend/semantic.rs` is
`d747f2819115fa4fb31b4c9c6e77ba0b0b26705d93a7f75ada209ca27f76295e`.
IR lines 223–305 and the closed primitive inventory provide integer types,
Nat, Int, Boolean/byte/string and structured data, but no floating scalar or
ln, powf, sin, cos, sqrt, floating remainder or floating round primitive.
Integer ParseDecimal/FormatDecimal do not supply floating formatting.

## Complete numerical obligation and reference profile

The missing native primitives are not a proof that a pure implementation is
inexpressible. A source-owned binary64 representation and arithmetic engine
can be modeled with exact integer and Boolean operations. They are substantial
remaining work, and a record containing 64 bits implements none of that work.

Before claiming bit parity, bind a reference profile containing Rust/compiler
version, target/architecture, usize width, effective features, math-library
implementation and relevant floating evaluation behavior. Cargo.lock and the
UOR source pin do not determine one universal result for every platform's ln
or powf. This profile is an implementation dependency to resolve within the
approved scope, not permission to choose an arbitrary numerically close libm.
Until resolved, exact transcendental compatibility remains unresolved.

The complete pure constructor must provide:

- Exact binary64 literal bits for all 512 ordinates, PI, thresholds, 0.37 and
  the four sigma values; classification, signed zero, subnormal/exceptional
  behavior, elementary arithmetic rounding and checked integer conversions.
- The selected reference algorithms for ln, sin, cos, sqrt, powf and
  rem_euclid, preserving source operation order. Rational/interval methods
  are valid proof techniques only when they establish the required output
  bits, rather than replace platform operations by ideal real arithmetic.
- Phase Q29's finite check, wrap, multiplication, ties-away round, i32 check;
  the source generator already wraps theta, so the recorded phase wraps it
  a second time. The unquantized theta still supplies the trigonometry.
- Q30 normalization with ordered sum of squares, threshold check, sqrt,
  per-coordinate division/scale/round/range check and all-zero rejection.
  Finite inputs alone do not guarantee finite intermediates. Huge generic
  inputs can reach an infinite norm and then the all-zero rejection.
- Exact mode behavior and distinct seed roles. Metadata seed does not alter
  construction. NormalNull's u32 seed does; TangentNull must retain full
  binary64 delta even though its one-decimal display label is not injective.
  That label's formatting is also a source compatibility obligation.

The complete path is phase → complex channels → normalized spinors → signed
root alignments → deterministic assignment. Accepting caller-provided scores,
spinors, roots, or a table with a matching checksum cannot establish that path.
An explicitly named quantized-boundary API may accept spinors, but must
recompute all alignments and must not return a full-constructor claim.

## Integer precursor APIs and exact scalar domains

These are proposed API responsibilities, not existing declarations or new
accepted claims. Each needs independently frozen controls and its own normal
review/verification gates before implementation.

| Component | Complete proposed operation | Required boundary |
| --- | --- | --- |
| `CodebookWords` | checked i32 admission, exact signed ordering/LE bytes; wrapping u32/u64 operations; checked u64 counts and u128 costs | distinguish modular arithmetic from checked accumulation |
| `CodebookRanks` | `primeRanks(count)` constructs the first count primes; NormalNull permutation and A8 shuffle | count 1..8192; explicit usize profile for NormalNull; complete termination, no arbitrary fuel |
| `CodebookQuantized` | admit full spinor shapes; derive embedding, all scores, outer/inner ordering, capacity search, roots/table, witnesses and metrics | externally supplied witnesses remain boundary inputs |
| `AddressCounts` | count actual caller-ordered eligible positions and serialize canonical counts | validate referenced positions/tokens; duplicates count again; checked count overflow |
| `AddressCells` | complete raw-compatible and strict-counted A8 builders | preserve filtering/lazy fallback/source error order; strict refinements separately named |
| `CodebookArtifact` | typed schema admission, exact serialization, word digest and distinct recount; strict root/Engine adapter | raw file hash, typed table identity and constructed provenance remain distinct |
| `PrimeCodebook` | complete numerical construction plus assignment | depends on the exact selected numeric profile, not caller scores |
| `PrimeAddressArtifact` | actual PrimeCodebook fallback cells → A8 → actual table encoding/digest/admission | no full construction claim until every dependency is accepted |

For exact signed i32 serialization, admit −2^31 ≤ x < 2^31; map negative
x to u = 2^32+x and nonnegative x to u=x. Emit, for j=0..3,
`floor(u / 256^j) mod 256`. This is two's-complement little endian: −1 is
`ff ff ff ff`, minimum i32 is `00 00 00 80`, and maximum is
`ff ff ff 7f`. A signed-magnitude implementation must canonicalize zero;
an Int negative-successor representation must account for its offset.

Hash the actual concatenated bytes with SHA-256. PrimeZeta's phase, flattened
spinor and table streams have no domain tag, length, seed, mode or Spec prefix.
Table order is token, block, coordinate. Those omitted metadata fields need a
separate complete binding, not an invented change to the source word hash.

Both shuffles need wrapping multiplication with potentially 128-bit Nat
intermediates before reduction to u64. NormalNull casts the generated word to
usize before modulo and forces a final identity permutation to swap endpoints;
A8 performs u64 modulo before index conversion and does not force a swap.
Constructive primes need a valid structural algorithm or proved sufficient
finite bound for all 8192 entries; a caller list or unproved retry cap is not
the specified prime constructor.

Root alignment uses the exact embedding `[l0,l1,l2,r0,l3,r2,r3,r1]`. The two
nonzero ±2 coordinates bound one signed alignment by 2^33 in magnitude and
two-block sums by 2^34, including arbitrary admitted i32 components. Preserve
descending signed score and ascending root-ID ties. The outer quota is
ceil(V/256); B≥4 varies only the first two interior roots, not an invented
full Cartesian optimizer. B=2 aliases and B=3 exhaustion remain source cases.

A8 canonical counted mass has checked u64 total, so a cell cost is at most
16·u64::MAX, safely within u128. Raw publicly constructible arrays can violate
that invariant: raw compatibility must use an explicit safe arithmetic
domain or report checked overflow as a port refinement. It cannot silently
equate unbounded Nat sums with profile-dependent Rust overflow behavior.
Raw frequency may differ from edge sums, invalid predecessors and zero edges
are filtered, and fallback validity is checked only if no placed positive
predecessor remains. Strict canonical-count admission is a separate API.

## Typed JSON artifact is a separate parser profile

`CodebookTable::load` directly deserializes a deny-unknown-fields Serde struct;
it does not first deserialize `serde_json::Value`. Its declared fields are
schema, spec, mode, table, table_sha256, distinct_vectors; nested Spec also
rejects unknown fields. Derived struct deserialization rejects duplicate
fields, and may reject an unknown or duplicate key before consuming its
value. Missing, type and duplicate errors therefore have a staged order.

The tokenizer JSON model deliberately parses all values and collapses decoded
duplicate keys by last-wins Value semantics. Reusing that collapsed map for
CodebookTable would erase required evidence. Reuse its lexical/string/number
machinery through a separately specified typed-event profile or retain an
occurrence/event tape that can reproduce typed source admission and priority.
Do not claim raw acceptance parity merely because the final map looks right.
Freeze the effective serde/serde_json feature closure and reference usize
width for this profile; the tokenizer's default standalone profile is not
automatically the codebook producer/consumer profile.

Numeric admission must distinguish signed integer i32 table entries, unsigned
u32 Spec fields and usize distinct count from floating numeric forms, including
negative zero. The tokenizer's as_u64 observation quotient does not alone
supply every typed signed decoder or its source diagnostics. Exact JSON string
escaping, integer spelling, compact separators and struct declaration order
belong to byte serialization; semantic object equality is weaker.

The source loader order is raw FilePin validation/read/digest, typed JSON,
schema, vocabulary/blocks equality, exact 8VB words, actual word SHA, then
actual distinct-vector count. It compares only vocabulary and blocks with
the requested Spec, not context/kept. Host loading does not check root
membership; upstream performs that during GPU reload. A fully admitted pure
Engine adapter must perform complete shape/root/model checks, identified as
the stronger application boundary rather than behavior of this host loader.

FilePin requires lowercase 64-hex SHA and compares the actual raw file bytes.
The table's declared word hash is compared against a recomputed lowercase
SHA string. Matching either digest neither authenticates the claimed source
revision nor proves the claimed mode produced the data. Model-owned successful
derivation results and later application-root custody must remain distinct
from publicly constructible witness records.

## Required acceptance and production boundaries

Independent controls must distinguish both PRNGs and cast orders, exact i32
endian/minimum/zero cases, actual score/ID ties, B2 aliases/B3 exhaustion,
count duplicates and raw filtering, lazy invalid fallback, token-ID interior
rank versus frequency order, complete JSON duplicate/unknown/type priority,
wrong raw versus word hash, and source loader versus strict root admission.
All original numerical obligations remain mandatory, including every ordinate
encoding and connected phase-to-artifact cases. No runtime table of expected
answers may replace construction.

Language 1.2's formal Nat arithmetic can express the needed integer operations;
that does not establish generated Rust/Wasm support for arbitrary-size Nat,
128-bit intermediates, software binary64, or allocation-free containers.
The production contract additionally requires bounded representations, exact
lowering and D11 instruction/allocation audits. Source multiplication used
to describe wrapping arithmetic is not GEMM, but a generated multiply/divide
instruction still needs the separate serving restriction resolved by lawful
lowering. This design neither changes D11 nor asserts an exported runtime.

No historic dense artifact format or generic accepted integer table closes
the complete current construction/export dependency. The proposed serving
format and deployment remain governed by the separate artifact-serving and
production-export contracts, after the complete mathematical components and
their exact bindings are accepted.
