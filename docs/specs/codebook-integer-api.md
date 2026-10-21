# Codebook integer precursor API

Design proposal dated 2026-10-21, before controls or definitions. This refines
[codebook-design.md](codebook-design.md) and
[codebook-artifact-refinement.md](codebook-artifact-refinement.md). Their complete
PrimeZeta, A8, numeric-profile and artifact obligations remain required. These
two modules establish integer precursors only; they do not accept caller
scores or establish floating construction, full codebook provenance, or serving.

## Authorities and module ownership

UOR revision is `4611538bb4843947363ccb9cce5f388e32c46883`.

- `crates/uor-r4-discrete/src/prime_zeta.rs`, SHA-256
  `2f1ab07fe6094be6c3887c3b700868f89f8a63fcf311652c16ed4227be8b4e54`:
  lines 110–138 first primes; 211–220 signed word bytes;
  226–243 NormalNull permutation.
- `crates/uor-r4-discrete/src/address_cells.rs`, SHA-256
  `22dec8e967f7d2bfd410f83067ee69646993bffe0aa3be4996375896e33fa75e`:
  lines 79–90 u64 count bytes; 108–125 A8 permutation.
- `R4.Carrier.Scalar` is the existing canonical integer representation:
  `nonnegative(n)` denotes n; `negative(n)` denotes −(n+1). There is one zero,
  `nonnegative(0)`, and no negative-zero normalization choice.

`R4.CodebookWords` owns scalar admission, exact word operations, byte codecs
and total orders. `R4.CodebookRanks` owns actual prime construction, complete
permutations and their closed reference-width profile. Public declaration
names below are proposed exact names. Data are immutable source values.
Reuse of an accepted arithmetic implementation is permissible, but does not
relax these contracts or introduce a foreign arithmetic callback. Official
language 1.2 arithmetic is the planned formal dependency; project migration
and its complete replay are separately required before integration.

## CodebookWords types

Closed types:

```
CodebookWordWidth = word32 | word64 | word128
CodebookOrder = less | equal | greater
CodebookWordError =
  invalidUnsigned(width: CodebookWordWidth, value: Nat)
  | invalidSigned32(value: Carrier.Scalar)
  | invalidShift(width: CodebookWordWidth, amount: Nat)
  | zeroDivisor
  | overflow(width: CodebookWordWidth)
  | invalidLength(expected: Nat, actual: Nat)
  | invalidByte(index: Nat, value: Nat)
```

All checked operations return `Result<T, CodebookWordError>`. Nat parameters
are mathematical naturals, not an implicit machine word. `word128` is needed
for checked A8 raw costs; it is not a new claim of native UInt128 lowering.

| Function | Parameters → success type | Exact contract |
| --- | --- | --- |
| `codebookAdmitUnsigned` | width, value → Nat | return value iff value < 2^width |
| `codebookAdmitSigned32` | Scalar → Scalar | nonnegative n: n<2^31; negative n: n<2^31 |
| `codebookCompareUnsigned` | width, left, right → CodebookOrder | admit left then right; compare mathematical Nat |
| `codebookCompareScalar` | left Scalar, right Scalar → CodebookOrder | total on all Scalars, no i32 narrowing |
| `codebookWrapAdd` | width, left, right → Nat | admitted operands; (left+right) mod 2^width |
| `codebookWrapMultiply` | width, left, right → Nat | admitted operands; left·right mod 2^width |
| `codebookCheckedAdd` | width, left, right → Nat | admitted operands; overflow iff sum ≥2^width |
| `codebookCheckedMultiply` | width, left, right → Nat | admitted operands; overflow iff product ≥2^width |
| `codebookXor` | width, left, right → Nat | complete width-bit XOR |
| `codebookOr` | width, left, right → Nat | complete width-bit OR |
| `codebookShiftLeft` | width, value, amount → Nat | admit value then amount<width; (value·2^amount) mod 2^width |
| `codebookShiftRight` | width, value, amount → Nat | admit value then amount<width; floor(value/2^amount) |
| `codebookRemainder` | width, value, divisor → Nat | admit value then divisor to width; reject zero divisor; value mod divisor |
| `codebookEncodeUnsignedLE` | width, value → List Nat | admit value; emit exactly width/8 bytes, least significant first |
| `codebookDecodeUnsignedLE` | width, bytes → Nat | exact width/8 length first, then byte validation left to right |
| `codebookEncodeSigned32LE` | Scalar → List Nat | admit signed32 then emit exact four two's-complement bytes |
| `codebookDecodeSigned32LE` | bytes → Scalar | exact four-byte/byte-domain admission, then inverse signed mapping |

Binary operations admit left then right before arithmetic;
an overflow is reported only after both operands are valid. No valid word is
silently truncated at a checked boundary. Shifts reject amount=width rather
than applying a host shift-count mask. Right shifts are logical unsigned shifts.
Remainder's divisor domain is explicit; its callers always supply positive
bounded index counts, not an unbounded host usize.

Scalar order is: every negative precedes every nonnegative; compare two
nonnegatives by n; compare `negative(a)` and `negative(b)` by the reverse
order of a and b. For signed32 encoding, nonnegative n maps to n and negative
n maps to `2^32−(n+1)`. Decoding unsigned u<2^31 returns nonnegative(u);
otherwise return negative(`2^32−u−1`). This includes the minimum value
negative(2147483647); negative(2147483648) is rejected. Decode is lossless
and canonical, with no alternative zero encoding.

Bytes are positional: byte j is `(u / 256^j) mod 256`. Encode32(−1) is
`[255,255,255,255]`; minimum is `[0,0,0,128]`; maximum is
`[255,255,255,127]`. Unsigned decoders sum the admitted positional bytes.
These functions do not add digest domains, tags, lengths or other metadata.
Stream hashing remains a separate operation over the actual resulting bytes.

The mathematical multiplication bound is exact: admitted w-bit operands have
product ≤(2^w−1)^2<2^(2w); admitted addition is <2^(w+1). Thus wrapping64
needs up to128-bit intermediate arithmetic, checked128 multiplication can
need256 bits to determine overflow, and formal Nat must not be advertised as
checked-u64 production code. A later bounded limb lowering may use16-bit limbs
and fully accounted carries, but it needs its own proof and D11 instruction
audit. The present API does not assume machine multiplication is permitted in
the exported serving path.

## CodebookRanks types and checked entry points

```
CodebookIndexProfile = usize32 | usize64
CodebookRankError =
  invalidPrimeCount(count: Nat)
  | invalidPermutationLength(maximum: Nat, actual: Nat)
  | invalidSeed(width: CodebookWordWidth, value: Nat)
  | invalidState(value: Nat)
  | invalidIndex(length: Nat, index: Nat)
  | primeBoundInvariant
  | permutationInvariant

CodebookPrngStep = { state: Nat, word: Nat }
```

The two invariant errors diagnose a violated internal construction obligation;
successful supported public calls must prove them unreachable. They are not
ordinary cutoff outcomes or permission to return incomplete success. Helpers
used on malformed explicit states remain total and return typed failures.

| Function | Parameters → result | Admission and result |
| --- | --- | --- |
| `codebookPrimeRanks` | count → Result<List Nat, RankError> | count in1..8192; exactly first count ascending primes |
| `codebookNormalInitialState` | seed → Result<Nat, RankError> | seed<2^32; exact initialized u64 state |
| `codebookNormalStep` | state → Result<CodebookPrngStep, RankError> | state<2^64; actual xorshift state and multiplied output word |
| `codebookA8Step` | state → Result<CodebookPrngStep, RankError> | state<2^64; incremented state and splitmix output word |
| `codebookNormalPermutation` | profile, seed, size → Result<List Nat, RankError> | size0..8192 then seed<2^32; permutation of0..size−1 including identity correction |
| `codebookNormalPrimeRanks` | profile, seed, count → Result<List Nat, RankError> | prime count admission then seed admission; construct primes, then permute actual ranks |
| `codebookA8Permutation` | seed, size → Result<List Nat, RankError> | size≤2^32−1 then seed<2^64; permutation of0..size−1 |
| `codebookSwap` | values:List Nat, left:Nat, right:Nat → Result<List Nat, RankError> | validate left then right against actual length; swap exact values, including same-index identity |

These helper domains include all source constructor calls. Normal's exposed
size bound follows the first-prime count bound; its zero-size helper is useful
for the empty-loop equation, not an admitted PrimeZeta vocabulary. A8's helper
does not pretend to reproduce the private Rust `(0..n as u32)` behavior for
unrepresentable n. Its specified size domain includes the entire u32 index
range relevant to a safe index permutation. The actual A8 builder still has
the narrower vocabulary/quota predicates from the complete codebook contract.
Additional Nat-domain errors are explicit port boundaries for fields typed
u32/u64/usize in Rust. They are not invented source errors on typed inputs.

Lists never use zero as a missing-index default. A swap may use two structural
passes to find both old values and replace their positions; no host array or
mutable state is authoritative. Iterate Fisher–Yates with the structural Nat
`size−1`: at step i>0 swap i with j, then recurse at i−1. A size0/1 call
performs no PRNG steps. Validation precedes this loop even for empty inputs.

## Complete prime construction and termination gate

The desired observable result is the source trial-division sequence: 2, then
ascending odd candidates; test prior discovered primes until p·p>candidate,
reject on divisibility, append only primes, stop after count. The implementation
must construct the sequence rather than accept a caller list or look up a
fixture of the first8192 answers.

Proposed practical finite realization: a complete source-owned sieve of the
inclusive interval2..100000, followed by the requested prefix. Every traversal
is structurally finite. Before adopting100000 as a runtime bound, require an
actual none-axiom certificate that this complete sieve has at least8192 entries,
and correctness of the sieve/prefix interpretation. This document does not
claim that certificate has been run or passed. Merely choosing100000 or adding
a `primeBoundInvariant` error is not sufficient; acceptance is blocked until
the adequacy obligation is proved. A literal prime inventory may support
independent proofs, but is never the runtime source of primes.

The sieve's mathematical specification is complete: start from ascending integers2..100000;
emit the first remaining integer p, then remove subsequent multiples of p,
and continue until empty. A structural implementation can walk a fixed
descending counter for100000 rounds while carrying the remaining list: a
nonempty step removes at least its head, and an empty step remains empty.
This counter bounds work by the initial list length; it does not bound the
mathematics by a caller-chosen fuel. A more efficient candidate marking sieve
is allowed only with the same complete candidate interval and proved result.
The ascending list and removal invariant establish prime soundness and that
no prime≤100000 is omitted. The separate adequacy certificate establishes
that the requested prefix is complete for every admitted count.

The simple whole-list sieve is a specification, not a promise to elaborate a
100000-deep semantic literal or normalize a100000-deep recursive call. The
implementation design should use bounded segments or a balanced traversal.
For example, the99999 candidates2..100000 divide into390 full256-candidate
segments and a final159-candidate segment, with391 outer steps. Within each
segment, mark composites using already constructed base primes and newly
discovered primes as necessary, then emit unmarked values in order. The
refinement must establish that every composite has a marking divisor, primes
are never marked, and the concatenated outputs equal the complete mathematical
sieve. A mere length test, a supplied primality predicate, or a count asserted
by the implementation cannot replace these obligations. Concrete segment/state
representation and the certificate decomposition need review before source
authoring. SDK recursion, heartbeat and child-timeout limits remain unchanged;
failure to verify within them is not permission to waive completeness.

An unconditional mathematical alternative avoids any fixed empirical bound:
retain the complete ascending prime prefix and its product P. A next prime
exists at most P+1, since a prime divisor of P+1 divides none of that prefix;
because the prefix contains every prime through its last element, that divisor
is larger. Exhaustive candidate search through P+1 therefore terminates
structurally. This is a valid constructive termination argument, not a proposed
efficient generated representation: P grows very large. It also does not by
itself prove the first8192 primes fit u32. The practical finite certificate
above resolves both the requested prefix and the source scalar bound. No
fallback to unverified data, partial prefix or arbitrary fuel is allowed.

## Exact distinct PRNG transitions and permutation order

NormalNull initialization is
`((seed XOR 0x9e3779b97f4a7c15) * 0xbf58476d1ce4e5b9 mod2^64) OR1`.
Each step performs, in order, u64 XOR with state>>12, state<<25 truncated to
u64, and state>>27. The next stored state is that xorshift result; the output
word is its product with `0x2545f4914f6cdd1d` modulo2^64. Do not store the
multiplied output as the next state.

For Normal's Fisher–Yates step i, `usize64` uses j=word mod(i+1);
`usize32` uses j=(word mod2^32) mod(i+1). Truncation precedes remainder.
The selected profile is explicit and cannot be inferred from the current
LexLean execution host. After all descending swaps, compare the entire output
to the original sequence; if identical and size>1 swap positions0 and size−1.
For the index permutation, comparison to identity is equivalent to comparing
the permuted distinct prime list to its original. `codebookNormalPrimeRanks`
must construct primes and apply that actual permutation, with checked index
lookup; it never accepts a claimed prime list from a caller.

A8 stores `state'=(state+0x9e3779b97f4a7c15) mod2^64`. Set z=state', then
z=`(z XOR(z>>30))*0xbf58476d1ce4e5b9 mod2^64`, followed by
z=`(z XOR(z>>27))*0x94d049bb133111eb mod2^64`; the output word is
z XOR(z>>31). The stored state remains state', not scrambled z or output.
Take j=word mod(i+1) in u64 **before** converting to an index. Both usize
profiles agree on this safe index domain, so A8 does not accept an irrelevant
profile argument. It has no forced-nonidentity swap.

Full permutation success preserves exact length, values0..size−1, uniqueness
and deterministic order. Tests must distinguish stored state from output,
each constant/shift, Normal cast order, the forced correction and A8's lack of
one. Testing only sorted outputs or a single seed would miss these semantics.

## Required control and proof preparation

No controls or definitions are authored by this design. Before implementation,
freeze independent literals for signed endpoints/byte order, every width's
carry/multiply/shift/domain precedence, unsigned versus negative-successor
ordering, both PRNG state/output transitions, usize32/64 divergence, descending
swaps, identity correction, size0/1 behavior and seed limits. Prime controls
include count0/1/8192/8193 and source prime-position assertions, accompanied
by the complete bound certificate and generic sieve/prefix obligations.

The accepted count bound is8192; proof-cost problems may change the exact
source algorithm or proof decomposition after review, but never truncate that
domain. Formal success, source-profile parity, generated code and model
execution are distinct claims. No model or reference constructor execution
was performed to produce this proposal.
