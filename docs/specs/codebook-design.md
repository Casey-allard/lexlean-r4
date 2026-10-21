# PrimeZeta and A8 codebook source contract

Status: source-contract design, not an implementation or execution result.
This design retains the complete pure LexLean PrimeZeta constructor, its
floating phase provenance, exact integer export, and A8 address construction.
A component accepting arbitrary alignment scores or externally supplied
quantized spinors is only a boundary component; it does not satisfy the
complete PrimeZeta obligation. No model, reference codebook, training run,
or qualification was executed for this document.

## 1. Pinned authorities

All UOR references below mean commit
`4611538bb4843947363ccb9cce5f388e32c46883`; line references are to that commit,
read with `git show`, independently of the original checkout's working tree.

| Source | SHA-256 |
| --- | --- |
| `crates/uor-r4-discrete/src/prime_zeta.rs` | `2f1ab07fe6094be6c3887c3b700868f89f8a63fcf311652c16ed4227be8b4e54` |
| `crates/uor-r4-discrete/src/address_cells.rs` | `22dec8e967f7d2bfd410f83067ee69646993bffe0aa3be4996375896e33fa75e` |
| `crates/uor-r4-core/src/zeta_zeros.rs` | `abd8012d6760b07338d72bb306b92745d34af6ded44b8320b35839f8fe739920` |
| `crates/uor-r4-discrete/src/training.rs` | `a189d3c9db4cf1e2d4e19be3264a2a36029c7fc88dae8b0bbc414f89e49049bf` |
| `crates/uor-r4-discrete/src/lib.rs` | `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9` |

Root identity/order is the already modeled sixteen-root orbit in `R4.Geometry`,
including its distinguished first root and signed-coordinate order. It is not
the entire E8 root system. The historical source's construction authorization
comments do not release a new execution, platform, or qualification.

## 2. Prime ranks, modes, and floating phase source

`generate_codebook` (prime_zeta.rs:275) first calls `Spec.validate`: vocabulary
V in 2..8192, blocks B in 2..16, context in 1..243, kept cells in
1..min(16,V). `first_n_primes` independently admits count in 1..8192, generates
2 then odd candidates, tests divisibility by previously discovered primes
through p*p <= candidate, and returns the first V primes in order. Prime
construction must be defined, not replaced by an arbitrary rank map. Published
source controls include p1=2, p2=3, p3=5, p4=7, p256=1619, p4096=38873.
These are source assertions, not freshly executed observations.

The zeta source contains exactly 512 decimal f64 ordinates attributed to
Odlyzko's zeros1 database. Their exact source literal bytes and resulting
binary64 encodings are part of the compatibility boundary. Replacing them
with higher-precision zeros changes the source mechanism.

There are two distinct seeds. The top-level `seed: u32` is stored as output
metadata and does not change construction. `NormalNull { seed: null_seed }`
controls the actual prime permutation and phase/amplitude perturbations.
PrimeZeta and TangentNull use unpermuted primes regardless of metadata seed.
Mode labels are `prime_zeta`, `tangent_null_delta_{delta:.1}`, and
`normal_null_seed_{null_seed}`. The one-decimal tangent label is not an injective
encoding of delta; preserve the full binary64 parameter in model provenance
rather than treating that display label as complete identity.

For each token in ascending token ID and each block b in ascending order,
compute ln_p = binary64 ln(prime), then four channels c=0..3:

- channel = (4*b+c) modulo 512.
- PrimeZeta gamma = ZETA_ZEROS[channel].
- TangentNull first rejects nonfinite or zero delta, then gamma = ordinate +
  delta. This check occurs after Spec admission and prime construction.
- NormalNull uses ordinate index `(channel+17+(null_seed mod 31)) mod 512`
  plus `((wrapping_u32(null_seed+c) mod 7)+1)*0.37` in binary64.
- theta = `(gamma*ln_p + PI).rem_euclid(2*PI) - PI`, preserving source
  binary64 operation order and the Rust PI constant.
- PhaseQ29 is recorded by calling `from_radians(theta)`, which checks finite
  input, wraps again, multiplies by 2^29, rounds ties away from zero, checks
  the signed i32 interval, and casts to i32.

The rounded integer phase does **not** feed trigonometric construction.
The original floating theta feeds `cos` and `sin`. PrimeZeta and TangentNull
use modulus 1. NormalNull selects sigma_shift from `[0.35,-0.35,0.20,-0.20]`
using `(c+(token_id mod 4)+(null_seed mod 4)) mod 4`, then computes
`prime.powf(sigma_shift)`. For each channel, real = modulus*cos(theta),
imaginary = modulus*sin(theta), in that order of operations.

Channels 0 and 1 become the left vector `[re0,im0,re1,im1]`; channels 2 and 3
become `[re2,im2,re3,im3]`. `UnitS3Q30::from_r4` (73–99) rejects nonfinite
inputs, sums the four squared coordinates in source iteration order, rejects
norm_sq <= 1e-24, takes sqrt, and rounds each `(value/norm)*2^30` to i32
with the same range checks and ties-away convention. It rejects an all-zero
quantized vector. Preserve signed zero, intermediate overflow, and exceptional
binary64 behavior in the floating model; checking only finite inputs is not
proof that every intermediate is finite. For example, a generic finite
huge input can yield infinite norm_sq and an all-zero rounded vector.

The source Q29 interval description is about wrapped real radians; rounding
can produce an endpoint grid value outside a literal open-ended real interval.
Do not add an unsupported strict integer interval test. `PhaseQ29(pub i32)`
and `UnitS3Q30(pub [i32;4])` also permit raw construction upstream; a complete
constructor witness is stronger than the representational i32 check.

### Exact NormalNull permutation

prime_zeta.rs:226–243 initializes u64 state as
`((u64(null_seed) XOR 0x9e3779b97f4a7c15) * 0xbf58476d1ce4e5b9 mod 2^64) OR 1`.
For i=V−1 down to 1, apply XOR with state>>12, then state<<25, then state>>27,
each in the u64 domain; multiply modulo 2^64 by 0x2545f4914f6cdd1d.
The source casts this word to `usize` **before** remainder by i+1, then swaps
i with that index. If V>1 and the final permutation equals the input, swap
positions 0 and V−1. This prevents identity, not collisions between all seeds.

The cast makes the literal algorithm host-width-sensitive. A 64-bit reference
profile preserves the full word; a 32-bit profile truncates before remainder.
The full pure model must take an explicit reference word-width profile or
bind compatibility to the named 64-bit construction platform. Do not silently
claim cross-platform equivalence or replace this with A8's different PRNG.

## 3. Exact integer boundary and greedy orbit assignment

The integer boundary consists of the actual Q29 phases and two Q30 spinors
per token/block, linked to the full floating construction. Embed spinors
l and r as `[l0,l1,l2,r0,l3,r2,r3,r1]` (252–255). For each canonical root,
compute the exact signed alignment sum over the eight coordinates (260–266).
The root's two nonzero coordinates are ±2, so even arbitrary i32 component
inputs have magnitude at most 2^33 per alignment; two-block score sums fit i64.
Use signed ordering, not unsigned casts, absolute alignment, or normalized
scores. The phase records are diagnostic witnesses; the spinors determine
these scores.

Assignment (357–476) processes tokens in ascending token ID. There are 256
outer writer cells `(r_first,r_last)`, encoded `16*r_first+r_last`, each with
quota ceil(V/256). For each token:

1. Enumerate all outer pairs; sort by descending sum of first- and last-block
   alignments, then ascending `(r_first,r_last)`.
2. Sort each interior block's sixteen roots by descending alignment, then
   ascending root ID.
3. For B>=4, enumerate the first two interior roots' 256 pairs, sort by
   descending sum of their block-1 and block-2 scores, then ascending IDs.
   The implementation does not optimize a total score over all B blocks.
4. Scan outer pairs in that fixed order, skipping cells whose load reached
   quota. B=2 tries only `[r_first,r_last]`; B=3 scans its single interior
   root; B>=4 scans its two-root pairs and sets every additional interior
   block to that block's individually highest-ranked root.
5. Accept the first candidate allowed by the uniqueness rule, otherwise
   report assignment failure. Increment its cell load; append selected root
   IDs and their exact eight integer coordinates in block order.

`enforce_unique_full_vector = (B>=3) OR (V<=16^B)`. B=2 with V>256 deliberately
allows aliases and does not insert candidates into the used-vector set.
For B>=3 the used set rejects previous complete B-root tuples, even when the
claimed total orbit capacity is insufficient. B=3,V=8192 is admitted by
Spec but cannot assign 8192 distinct tuples from 4096 possibilities: the
constructor must return the source assignment error, not silently disable
uniqueness. B>=4 explores only two varying interior roots for each token;
additional blocks do not authorize a larger Cartesian search than the source.

Full cells have a ceiling quota, not necessarily equal occupancy when V is
not a multiple of 256. Report actual occupied-cell count. Recount distinct
phase chunks of length4B, spinor chunks of length2B (each entry four i32s),
and table chunks of length8B; do not infer these metrics from vocabulary or
an intended invariant. The constructor's output order is token, block, channel
or coordinate, with left spinor before right.

The three digests are SHA-256 of concatenated signed-i32 **two's-complement
little-endian** bytes: phases in flat order, spinors flattened including all
four coordinates, table in flat order. No domain tag, length, mode, seed,
root-ID list, or Spec bytes are added to these digests (211–220,490–493).
Those other identities need a separate complete artifact binding. Witness
extraction slices those exact arrays for a checked token ID. Structural
comparison requires identical full Spec, then counts differing token vectors,
root slots and writer cells; coordinate Hamming, coordinate L1, and phase L1
are separate quantities (515–596). Raw decoded codebooks need shape admission
before these helpers, which upstream assumes receive constructed data.

## 4. A8 counts and distinct admission contract

A8 `address_cells::build` does **not** call `Spec.validate`. Its own initial
predicate (142) is B>=2, V>0, V divisible by256, and V/256<=16. Thus quota is
1..16 and V is256..4096; source build can accept B>16 or invalid context/kept
fields. A port may expose a fully admitted model wrapper, but must not claim
that wrapper's additional rejection is behavior of the low-level builder.
It next checks both counts arrays and fallback array have length V.

`BigramCounts::count(tokens,eligible,V)` validates each eligible p in caller
order: 0<p<tokens.length, then both tokens[p−1] and tokens[p] within vocabulary.
It increments pair `(target,predecessor)` and target frequency, using u64
counts. Duplicate eligible positions count again. It sorts output by target,
then predecessor via the BTreeMap. It does not itself inspect document
boundaries; same-document eligibility is a caller contract. Only referenced
tokens are checked. Counter overflow needs explicit checked admission in the
pure model; source unchecked increments have profile-dependent failure/wrap
behavior outside the representable count domain.

Counts canonical bytes (79–90) are all u64 little-endian: frequency length
as vocabulary; for each target in by_target order, its frequency, edge count,
then every predecessor widened from u32 and its u64 count. No tag or separator
is inserted. Strict canonical count objects require matching lengths, sorted
unique in-range predecessor IDs, positive counts, checked sums, and frequency
equal to the edge-count sum. Those properties hold for successful `count`.
They are not all checked by `build`, whose public raw fields also accept
inconsistent frequencies, duplicate/unsorted edges, zeros, and invalid IDs.

Model two explicitly different inputs: a refined counted object for normal
construction, and a raw compatibility object with checked scalar domains and
array shapes. In raw builder parity, out-of-vocabulary predecessor IDs are
filtered out by failed `relabel.get`, zero counts are filtered out, and
frequency determines order independently of edge sums. Do not replace this
filtering with a claimed upstream error. If a strict artifact API rejects
noncanonical raw counts, identify the extra refinement and test it separately.
Likewise, upstream `canonical_bytes` can index a short frequency array and
serializes raw edge order; the safe canonical API must reject mismatched
shapes rather than reproducing a panic or silently sorting imported bytes.

## 5. A8 placement, shuffle, and full token table

`StructuredV1` uses identity predecessor relabeling. `ShuffledV1` starts u64
state at its own seed. For each Fisher-Yates step from V−1 to1, add
0x9e3779b97f4a7c15 modulo2^64; transform z with
`(z XOR (z>>30))*0xbf58476d1ce4e5b9`, then
`(z XOR (z>>27))*0x94d049bb133111eb`, each modulo2^64; output z XOR(z>>31).
Take u64 remainder by i+1 **before** converting to an index and swapping.
There is no forced-nonidentity final swap in A8. Its seed is u64, not the
NormalNull u32 seed.

Placement order sorts tokens by descending frequency, then ascending token
ID. For current target t, map each predecessor s through the optional
permutation, retain it only if that relabeled token was already placed, then
retain only positive counts. Eligibility uses the relabeled identity's placement
state: relabeled self/future identities do not attract t. An original self or
future predecessor can map to an already placed token and attract t; an
originally placed predecessor can map to self/future and be ignored.
Relabel predecessor identity only: do not permute target order, fallback,
frequency, count weights, or the final table.

For each candidate cell with load<quota:

- If no positive placed predecessor remains, compute cell L1 from this
  token's PrimeZeta fallback cell. Only in this branch is fallback>=256 an
  error; an unused invalid fallback value is not rejected by source build.
- Otherwise sum `count * cell_l1(placed_cell,candidate)` in u128; fallback
  contributes nothing, including no tie break or regularization term.
- Minimize `(cost,cell_id)`, so ties choose ascending numeric cell ID.

`cell_l1` is coordinate L1 of the first roots plus coordinate L1 of the last
roots, not Hamming distance between IDs. One root distance is at most8 and
one cell distance at most16. Refined counts with checked total mass <=u64::MAX
therefore have cost <=16*u64::MAX, safely inside u128. Arbitrary raw arrays
need a checked u128 cost accumulation or an explicit safe-total refinement;
a new overflow error is safer than claiming unspecified host overflow is
exact arithmetic. Cell decoding must reject invalid IDs before indexing.

After placement, table generation iterates **ascending token ID**, not
placement/frequency order. Maintain a rank for each assigned cell; its k-th
token in token-ID order has first root cell/16, last root cell%16, and root
**k repeated in every interior block**. Do not encode rank as base16 digits,
use frequency rank, or copy PrimeZeta interior roots. With quota<=16 each
interior root is valid. Recount full-vector distinctness, then check every
cell load equals quota, in that error order (214–218).

B=2,quota>1 necessarily fails the distinct-vector check because it has no
interior block to encode rank. B=2,quota=1 can succeed; B>=3 supplies an
interior root that distinguishes the at-most16 tokens per cell. Successful
full placement has all256 cells occupied at exact quota. Preserve typed
`NoCellQuota`, `TokenUnplaced`, `NonInjectiveTable`, and `QuotaMismatch`
branches even if well-formed constructor invariants make some unreachable.

`table_file_bytes` (231–241) emits compact Serde JSON with schema
`uor-r4.codebook-table/1`, Spec, mode, table, table_sha256, distinct_vectors
in the struct's declaration order. `CodebookTable::load` checks schema,
only vocabulary/blocks equality against requested Spec, exact8VB word length,
i32-byte digest, and recomputed distinctness. It does not require full Spec
equality or perform root membership on host; root admission belongs to the
existing integer admission model. Strict JSON decoding rejects unknown fields.
Preserve byte serialization separately from mathematical table identity.

## 6. Official LexLean numeric capability and compatibility gap

The inspected source is LexLean `0865df4e540ca643b090a29e9df6d319d6906da6`
vendored in PrismPM `dfdc4df770d038ee5fb700d374aca883d1c9ec7a`. Exact files
were retrieved from that immutable GitHub tree, not the earlier local01088
checkout or a different vendor revision:

| Vendored source | SHA-256 |
| --- | --- |
| `vendor/lexlean/src/ir/semantic.rs` | `e273dcc28a3d1b426bb97bef43cb426e2ccb7cbd205aab2a3b4ccf38c5623fbf` |
| `vendor/lexlean/src/backend/semantic.rs` | `d747f2819115fa4fb31b4c9c6e77ba0b0b26705d93a7f75ada209ca27f76295e` |

`SemanticType` (IR:223–305) provides Nat, Int, signed/unsigned8/16/32/64-bit
integers, Bool, bytes/strings and structured types. It has no Float32,
Float64, Real, or rational numeric scalar. `SemanticPrimitive` (IR:52 onward)
is closed; its arithmetic is integer arithmetic and it has no ln, powf,
sin, cos, sqrt, floating round, or floating rem_euclid. ParseDecimal and
FormatDecimal are explicitly constrained to integers (IR:6953–6970).
The exhaustive type lowering (backend:358–418) and primitive lowering
(backend:665 onward) confirm no hidden float backend mapping. A
source-defined named structure can represent binary64 bits, but that alone
implements none of the required arithmetic.

Consequently, exact whole PrimeZeta requires the following source-authored
numeric work, not just an alignment-table input:

1. Binary64 bit representation, classification, signed zero, finite and
   exceptional values, integer conversion, IEEE rounding for elementary
   arithmetic, and the source's explicit ties-away `round` conversion.
2. Exact bit encodings of PI,512 zeta decimal literals, thresholds,0.37,
   and the four signed sigma literals. Decimal-to-binary conversion must
   agree with the reference compiler, not treat literals as exact rationals
   during downstream calculations.
3. Reference-profile-compatible ln, sin, cos, sqrt, powf, and rem_euclid,
   including source operation order, argument reduction, overflow, and
   target/compiler math-library behavior. Rust's use of these f64 methods
   does not by itself select one universal bit-identical transcendental
   implementation for every supported platform.
4. Quantization controls at ties, wrap discontinuities, finite/error bounds,
   normalization threshold, overflow and all-zero collapse; then complete
   connected phase→spinor→alignment→assignment controls.

A viable pure approach is a source-authored binary64 engine plus a precisely
pinned math-function algorithm/profile, with proofs and literal reference
bits at each boundary. Pinning only an approximate real formula is
insufficient: a small numerical difference can change quantized coordinates,
score ties, greedy choices, or later tokens' capacity. Arbitrary-precision
rational or interval arithmetic may support proof of rounding cells, but it
is equivalent only after proving the output bits agree with the selected
binary64 reference operations, including their errors. Rational approximations
cannot be advertised as matching platform ln/powf merely because both
approximate the same real function.

Before choosing a full numerical implementation, capture the intended Rust
reference platform/compiler/math-library identity and its actual guarantees.
If that evidence does not exist, mark exact floating compatibility unresolved;
do not invent a compatible libm or reuse an unrelated execution release.
A separately reviewed generic LexLean numeric facility could improve this
path, but no such facility is present in the inspected source. A foreign
Rust or browser math call is not a pure LexLean implementation.

## 7. Proposed pure model phases and acceptance contracts

These are proposed components, not newly registered capabilities. Every phase
requires its own ID, scenario, independent RED control, implementation,
review and planted defect before a built claim. No implementation or shared
registry changes are included in this source-contract task.

| Phase | Complete responsibility | Honest claim boundary |
| --- | --- | --- |
| A: word and ordered-data support | wrapping64 multiplication, left/right shifts, bounded remainder, checked u64/u128 accumulation, signed i32 LE encoding, complete stable total orders and swaps | Generic exact arithmetic/data support; no codebook provenance |
| B: ranks and permutations | constructive prime sequence; both exact PRNG/Fisher-Yates variants, NormalNull identity correction, explicit host-width profile | Integer precursor semantics only |
| C: binary64 phase constructor | all three modes, all512 exact ordinate bits, reference-profile math, double wrap/Q29 recording, unrounded-theta complex channels, normalization/Q30 | Required before full PrimeZeta claim; currently unresolved numeric dependency |
| D: integer boundary constructor | checked complete spinor shapes and i32 domains, plane embedding, canonical root scores, exact greedy search, typed exhaustion, witnesses/metrics/i32 hashes | Boundary component if C missing; accepting external scores alone is weaker still |
| E: A8 count/placement constructor | eligibility-to-count transition, canonical bytes, raw/refined admission distinction, full shuffle/placed-mass/capacity ranking and injective token table | A8 integer construction; real fallback provenance still requires PrimeZeta composition |
| F: end-to-end artifact composition | full PrimeZeta output feeds A8 fallback extraction; exact table bytes/digest/metadata; existing root admission and Engine integration | Complete codebook construction only after all dependencies accepted |

Use a closed `CodebookMode` with a binary64 delta representation, distinct
metadata seed and mode seed, explicit width profile, and typed errors.
`ConstructedPrimeCodebook` should carry shapes and derivation provenance;
`QuantizedBoundaryInput` should plainly identify externally supplied witnesses.
An untrusted raw source record must not acquire a constructed witness solely
because its dimensions or checksum match. Integer downstream computation
must recompute alignment from quantized spinors, rather than trusting caller
scores. Complete acceptance must additionally connect those spinors to phase
construction C.

A proposed low-level interface division is:

- `primeRanks(count)` and `normalPrimePermutation(profile,seed,primes)`.
- `constructPhaseWitness(spec,mode)` returning exact per-token phases and
  spinor pairs, or typed numeric/admission failure.
- `assignQuantizedWitness(spec,witness)` recomputing scores and the complete
  deterministic token assignment. Optional externally supplied boundary
  input is separately named and cannot report full-constructor acceptance.
- `countEligible(tokens,positions,V)`, `admitCanonicalCounts(raw)` and
  `buildAddressCellsRaw(spec,counts,fallback,mode)`, whose refinement/parity
  contracts distinguish source filtering from added strict artifact checks.
- `constructPrimeCodebook(spec,metadataSeed,mode,profile)`, then
  `constructA8FromPrimeCodebook(...)` that extracts first/last selected roots
  as fallbacks and checks Spec/table shape before placement.

Model counts as Nat with explicit source-word admission or fixed words;
model exact signed scores with the existing signed representation carefully
(`neg n` denotes −(n+1) where that representation is used). Modular PRNG
arithmetic and checked counts/cost arithmetic are different operations.
No undocumented wrap, zero-index default, swallowed exhaustion, or fallback
root is permitted. Tail-recursive total algorithms need explicit structural
termination and realistic proof-cost experiments before promising full-scale
normalization; a bounded verifier timeout is not a permitted algorithmic
truncation.

## 8. Independent controls and defect obligations

Before implementation, freeze source-derived literals and independently
specified small cases. Do not execute the Rust constructor on host to make
new model/parity fixtures. New reference executions need their own applicable
authorization and platform binding. Published Rust unit-test expectations
remain imported assertions until actual port controls are checked.

Required controls include:

- Prime count0/1/8192/8193 boundaries; source prime-position literals; both
  seed domains, NormalNull forced swap, and the usize-before-mod distinction.
- All512 ordinate bit encodings; distinct metadata/mode seeds; tangent
  nonfinite and ±zero errors; overflow while adding a finite delta or
  multiplying gamma by ln_p; exact channel-offset wrap and u32 seed wrap.
- Binary64 reference controls for ln,powf,sin,cos,sqrt, rem_euclid; positive
  and negative halfway rounding; theta wrapped twice for Q29 but its original
  floating value used for spinors; S3 threshold and collapse cases.
- Each plane embedding basis, signed alignment bounds, root order, outer
  score/ID ties, interior root ties, pair ties, and a case distinguishing
  first-two-interior search from a full Cartesian optimizer.
- Capacity exhaustion including B3,V8192 as a theorem of impossible complete
  injectivity; B2,V>256 alias permission; exact quota ceiling at nonmultiples
  of256; duplicate metrics from actual arrays, not presumed cardinalities.
- i32LE −1 and signed minimum bytes, flattening order, no extra hash tag,
  distinct phase/spinor/table byte streams; checked witness token bounds and
  mismatching full-Spec comparison.
- A8 count immediate predecessor, duplicate eligible position, invalid
  position/token priority, sorted emitted edges, canonical-byte length/order;
  safe overflow admission and explicit raw noncanonical count handling.
- A8 low-level B>16/context/kept distinction from Spec admission; quota
  boundaries; descending frequency then token ID; invalid predecessor
  filtering and positive-count filtering before selecting fallback branch.
- Shuffle of predecessor identity only; self/future filtering after relabeling,
  including an original self/future predecessor mapped to an already placed
  identity and the converse;
  fallback used only with no positive placed mass; invalid fallback rejected
  only when used; weighted L1, cell-ID ties, saturated nearest cell rerouting.
- Token-order within-cell rank distinct from frequency placement rank;
  repeated interior rank for B>3; B2,quota>1 injectivity failure and B2,quota1
  success; recount-before-quota failure priority and exact table JSON format.

Representative planted defects must target real connected output assertions:
change the NormalNull seed source; dequantize Q29 before sin/cos; swap axes3/4;
reverse a score tie; scan additional interior combinations; silently relax
B3 uniqueness; use placement rank for A8 interiors; include an unplaced
predecessor; apply fallback during positive-mass placement; substitute root-ID
Hamming for coordinate L1; use truncating full hash or big-endian i32 bytes.
Compilation failure is not evidence that these semantic gates work.

## 9. Delivery and unresolved obligations

The exact integer algorithms are expressible using existing structural
LexLean types, with additional complete modular multiplication/remainder and
safe wide-cost support where needed. Their full finite construction may be
large to verify and needs connected transition certificates if direct
normalization exceeds the fixed SDK limits. Certificates must prove actual
transitions and final outputs; disconnected examples and arbitrary supplied
score tables do not establish the end-to-end result.

The primary unresolved dependency is source-compatible binary64 phase
construction, especially platform ln/powf and argument-reduction behavior.
It remains in the full implementation scope. Completing phases A/B/D/E can
make useful, separately named progress but cannot close full PrimeZeta or
full A8-with-PrimeZeta-provenance acceptance. No host computation, training,
GPU qualification, exported runtime performance, browser support, or deployed
capability follows from this design or its static source audit.
