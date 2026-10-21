# Exact natural-word primitive contract for the isolated Hash candidate

Status: proposed API and independent predefinition controls, 2026-10-21.
This is an isolated language-1.2/spec2 candidate dependency. Accepted Hash,
Words, Object sources and their original statements remain unchanged.
No source definitions, primitive verification, performance improvement or
replacement acceptance are established by this document.

The accepted Hash source pin is
`d2783f37e0ae537c13802f4454411cf131304ac5bcb41f43bf47fd0903416c02`;
current Words source pin is
`995863b7cfcc0d481fb92452f5d75e35178a1a63c17af479ce89e6f0b3f99945`.
UOR authority remains revision `4611538bb4843947363ccb9cce5f388e32c46883`.
SHA-256 mechanics follow the accepted Hash contract/FIPS 180-4; configured
BLAKE2b-96 follows the accepted Hash contract and RFC 7693, including its
12-byte output parameter. This layer defines exact finite word algebra; it
contains neither hash algorithm nor digest fixture table.

## Types and operations

`R4.HashNat` owns `HashWordWidth.w32 | .w64` (no fields) and:

`HashNatError.invalidWord(width : HashWordWidth, value : Nat)`,
`.invalidAmount(width : HashWordWidth, amount : Nat)`,
`.invalidLength(expected : Nat, actual : Nat)`,
`.invalidByte(index : Nat, value : Nat)`.

Write w=32 or64, M=2^w, and mask=M-1. Word-valued natural operands must be
strictly below M. These bounds are checked before every public operation;
no silent truncation admits a malformed operand. Binary operations validate
left, then right. The Result error is the exact first malformed value.

All of the following return `Result Nat HashNatError`:

- `hashNatWrapAdd(width,left,right)`: (left+right) modulo M.
- `hashNatXor(width,left,right)`, `hashNatAnd(width,left,right)`, and
  `hashNatOr(width,left,right)`: exact per-bit operations over all w bits.
- `hashNatNot(width,value)`: mask-value.
- `hashNatRotateRight(width,value,amount)`: cyclic right rotation. Amount is
  admitted in 0..w-1; zero is identity. Check word before amount.
- `hashNatShiftRight(width,value,amount)`: logical right shift.
- `hashNatShiftLeft(width,value,amount)`: logical left shift modulo M.
  Both shifts admit 0..w-1 and check word before amount. Amount w is an error,
  not a disguised zero result. No hash algorithm requires that amount.

`hashNatEncodeLE(width,value)` and `hashNatEncodeBE(width,value)` return
`Result (List Nat) HashNatError`, exactly w/8 bytes, retaining leading/trailing
zeros. They check the word domain first. `hashNatDecodeLE(width,bytes)` and
`hashNatDecodeBE(width,bytes)` return `Result Nat HashNatError`: check exact
list length first, then every byte in order, and only then decode. No accepted
prefix, implicit padding, ignored suffix, truncation, or native host cast.

The legacy boundary adapters are:

- `hashNatToWord32(value) -> Result R4.Words.Word32 HashNatError`;
- `hashNatToWord64(value) -> Result R4.Words.Word64 HashNatError`;
- `hashNatFromWord32(value : R4.Words.Word32) -> Nat`;
- `hashNatFromWord64(value : R4.Words.Word64) -> Nat`;
- `hashNatToByte8(value) -> Result R4.Words.Byte8 HashNatError`;
- `hashNatFromByte8(value : R4.Words.Byte8) -> Nat`.

The word adapters use the corresponding invalidWord constructor when out of
range; the byte adapter uses invalidByte(0,value) for value>=256. From-adapters
are total because the input records contain fixed Boolean fields, not
unchecked integers. In Byte8, b0 is least significant; in Word32/64, a0 is
the least significant byte. Preserve those exact existing types and fields.

## Implementation and proof boundaries

Use exact exposed Nat arithmetic from the official 0865 SDK under language1.2.
Do not use native UInt/Float operations whose observed axiom audit failed.
Bit folds must consume exactly 32/64 positions over already admitted operands;
there is no caller-selected fuel or unfinished-result success. Rotations use
exact quotient/remainder and powers of two; bounded word conversions visit
every legacy Boolean field. No exponential unary recursion on a word value.

Internal unchecked helpers may assume their documented word/shift domains;
all external candidate hash paths derive or validate those domains. They do
not accept expected results as arguments. No compiler patch, injected Lean,
axiom allowance, host hash oracle or resource-limit change is part of the plan.

Candidate SHA/BLAKE inner state may use Nat records, but existing ShaState,
Digest256, NodeId12, shaRound and every externally referenced Hash definition
must preserve its signature and semantics through actual boundary adapters.
All original 30 Hash and 403 Object propositions remain required, in addition
to these primitives and independent round/compression vectors. BLAKE2b remains
the configured single-block API with its original bounds; BLAKE3 is unrelated.
Complete source and compatibility review precede replacement acceptance.

## Independent control construction

The frozen controls use literal expected numbers, byte lists and legacy
Boolean records, not calls to the candidate implementation on the right side.
Their basis is exact positional integer identities: 2^i, complementary masks,
disjoint bit pairs, all rotation/shift amounts, radix256 place values, and
carry/wrap boundaries. Nontrivial alternating/asymmetric patterns distinguish
direction and byte order. Static authoring may expand these identities using
host exact integers; it does not run a hash, parser, model, or candidate source.

Cover every one-hot bit for both widths through Boolean operations and both
directions of word/byte codecs; every legal rotation amount and both logical
shifts; zero, maxima, full-width carry and mixed-word carry; malformed left and
right operands, word-before-amount priority, exact byte lengths before byte
domain errors, and the first invalid byte. Literal test statements are frozen
before missing-module RED and definitions. General algebra/adapter claims
remain separate proof obligations; basis controls alone are not a universal
equivalence proof or production execution benchmark.

## Frozen round interface and published controls

`R4.HashNatCore` owns `HashNatBlakeState` with Nat fields `v0` through `v15`
and `HashNatBlock64` with Nat fields `w0` through `w15`, both in numeric order.
The internal functions `hashNatBlakeRound0` through `hashNatBlakeRound11`
take `(block : HashNatBlock64, state : HashNatBlakeState)` and return a
`HashNatBlakeState`. `hashNatBlakeRounds(block,state)` composes all twelve in
order. These are the actual core functions used by the configured 12-byte
hash facade; their word-domain invariant is every field below 2^64.

Thirteen additional independent controls preserve all 208 literal state
words at boundaries i=0 through i=12 from
[RFC 7693 Appendix A](https://www.rfc-editor.org/rfc/rfc7693#appendix-A).
Twelve controls invoke one actual round on its published input; the thirteenth
invokes the complete round composition on state i=0 and compares all sixteen
words with i=12. No implementation computes the expected states. The RFC
trace uses BLAKE2b-512 initialization for `abc`; this tests generic round
mechanics and never replaces the original configured BLAKE2b-96 vectors.
The unchanged public SHA round-zero control must likewise invoke the actual
Nat SHA round through the preserved legacy adapters.

The five primitive fixtures contain 1681 statements, inventory SHA-256
`bf05c6095c809cd3a7521b08c738ca7c4cb259dbabd9470fbd0301487467fafb`.
The separate `HashNatBlakeRoundsControls` fixture has SHA-256
`b164ce16a47b35bc52ea9d4c1517a740c722c27872e3183cf44f25d0c7ba0df2`.
Its direct published-state inventory has SHA-256
`ad96bbc2501a53f1ca2bac1f3aaafd4de89b65249bb6e557b18c3df07636cafb`;
the retained RFC source has SHA-256
`c943754888364fe29bbd0bb3c71c6658ef392371497eaa4a9d9dde015b0721e5`.
All 1694 statements precede their new definitions. They require independent
review, genuine missing-definition rejection, and subsequent formal checks.
