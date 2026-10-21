# BLAKE3 source map for independent review

This is an **author-prepared review packet**, not an independent approval or a
universal equivalence proof. No implementation or accepted-project language
changes are made here. The complete isolated implementation has 91 declarations
and remains at the frozen source identities below.

Authority is [BLAKE3 1.5.0 reference_impl.rs](https://github.com/BLAKE3-team/BLAKE3/blob/5aa53f07f7188a569cadfc5daf1522972d9a9630/reference_impl/reference_impl.rs),
SHA-256 `2fae31faabb79094ebb8c8377e5f1d804ae46902f8cc2a8b31e14d2632948ca0`.
The local exact copy is `/tmp/r4-blake3-reference.rs`; implementation is
`/tmp/lexlean-r4-blake3-nat-20261021`. Every declaration is listed exactly once
in this map, checked mechanically against actual semantic source declarations.
Reference numbers below are line numbers in that pinned reference file.

A byte-preserving durable copy of the candidate, controls, prior inputs,
mutation manifests and retained receipts/audits is under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/blake3-codebook/`.
Its preservation manifest records original paths, sizes and SHA-256 hashes.
This location change promotes no verification or acceptance status; temporary
originals remain in place.

| Source | Declarations | SHA-256 |
| --- | ---: | --- |
| `src/R4/Blake3Nat.lex.tex` | 18 | `91df70496689c63732341fad46abd39a3c91f8d756db0665136f789b74e9a68b` |
| `src/R4/Blake3Core.lex.tex` | 22 | `64242d1e99d648b420f33ca190f949308a151b1e52eb6a58af559f3d8561c6a0` |
| `src/R4/Blake3.lex.tex` | 41 | `a940ced45024e3b3c73da5ec373c27a57afd9abdbd0f1e87ff52dbbd0878754e` |
| `src/R4/Blake3Input.lex.tex` | 10 | `00eab31ab88f170f242076e8bb791e65d9f9e7fc3d00776883f484cbae227198` |

## Complete declaration map

| Module and exact declarations | Reference lines | Review obligation |
| --- | --- | --- |
| R4.Blake3Nat: `B3NatError`, `B3CounterHalves`, `b3NatWordValid`, `b3NatPowTwo` | 42–50,74–120 | Port representation: exact Nat; typed word/counter domains; all divisors positive. |
| R4.Blake3Nat: `b3NatAddRaw`, `b3NatWrapAdd`, `b3NatXorBits`, `b3NatXorRaw`, `b3NatXor`, `b3NatRotateRaw`, `b3NatRotateRight` | 42–50 | Modulo2^32 addition, exactly 32 XOR bits, rotr amount0..31; checked wrappers left before right / word before amount. |
| R4.Blake3Nat: `b3NatWordBytesRaw`, `b3NatWordBytes`, `b3NatCount`, `b3NatValidateBytes`, `b3NatDecodeRaw`, `b3NatBytesWord`, `b3NatCounterParts` | 74–92,126–132,155–168 | LE 4-byte conversion; exact length before byte domain; split counter into low/high32 without wrapping. |
| R4.Blake3Core: `B3CoreState`, `B3CoreCV`, `B3CoreMix`, `B3CoreError`, `b3CoreIV`, `b3CoreStateValid`, `b3CoreCVValid`, `b3CoreValidateState`, `b3CoreValidateCV` | 35–39,74–92 | Fixed16/8/4 Nat records; exact IV; validate every word in field order. |
| R4.Blake3Core: `b3CoreMixRaw`, `b3CoreColumns`, `b3CoreDiagonals`, `b3CoreRoundRaw`, `b3CoreRound` | 42–63 | G sequencing16/12/8/7; column and diagonal disjoint write indices; checked round state then message. |
| R4.Blake3Core: `b3CorePermute`, `b3CoreInitialRaw`, `b3CoreInitial`, `b3CoreRounds`, `b3CoreFeedForward`, `b3CoreCompressRaw`, `b3CoreCompress`, `b3CoreFirstEight` | 66–124 | Gather permutation; direct metadata initialization; exactly 7 rounds; both feed-forward halves use original CV. |
| R4.Blake3: `B3Error`, `B3Hasher`, `B3Output`, `B3Digest` | 136–182,276–283 | Port records/errors; logical buffer and immutable values; no source flags in public hasher. |
| R4.Blake3: `blake3Append`, `blake3CountCVs`, `blake3CVsValid`, `blake3BufferValid`, `blake3CVEqual`, `blake3Popcount54`, `blake3ExpectedStackLength`, `blake3Init`, `blake3ValidateState` | 184–204,285–298,330–343 | List support and public-state refinement. Ordered 12 invariants below; stack length popcount(chunkCounter), newest-first. |
| R4.Blake3: `blake3CheckLength`, `blake3CheckBytes` | 346–363 | Port explicit length/budget/byte admission; whole update checked before immutable transitions. |
| R4.Blake3: `blake3ByteAt`, `blake3BufferWord`, `blake3BlockWords`, `blake3StartFlag`, `blake3ChunkOutput`, `blake3OutputCV`, `blake3ParentOutput` | 126–153,199–204,235–273 | Zero pad only word loads; actual length retained; chunk counter constant per chunk; parent IV,left||right,0,64,PARENT. |
| R4.Blake3: `blake3MergeStack`, `blake3BeforeByte`, `blake3AppendByte`, `blake3FeedBytes`, `blake3Update` | 207–232,330–363 | Commit only before next actual byte; trailing-zero merges pop newest left; full chunk takes priority over ordinary block. |
| R4.Blake3: `blake3FoldStack`, `blake3DigestFromOutput`, `blake3RootDigest`, `blake3Finalize256`, `blake3Hash256Bounded`, `blake3Hash256` | 155–170,366–382 | Fold newest stack entry as left child; final compression counter 0 with ROOT; return first8 words, no empty flush. |
| R4.Blake3: `blake3EncodeDigest`, `blake3HexNibble`, `blake3HexBytes`, `blake3DigestHex`, `blake3Hash256Bytes`, `blake3ChunkBytesCount`, `blake3FeedChunks`, `blake3HashChunks256Bytes` | 155–170,346–382 | Port wrappers: checked digest LE 32 bytes, lowercase hex 64; chunks route through actual update/finalize. |
| R4.Blake3Input: `B3LiteralInput`, `blake3LiteralLength`, `blake3UnpackBytes`, `blake3ValidatePacked`, `blake3ValidateLiteral` | port-only literal representation | Generic empty/packed/concat, count<=64,payload<256^count; LE decode exact count and retain zero bytes. |
| R4.Blake3Input: `blake3DecodeLiteralRaw`, `blake3DecodeLiteral`, `blake3FeedLiteralRaw`, `blake3UpdateLiteral`, `blake3HashLiteral256Bytes` | 346–382 plus port-only literal boundary | Validate full state/length/tree first; unpack each admitted leaf and invoke real update; never input-specific digest lookup. |

## Ordered state invariants and transition obligations

`blake3ValidateState` reports `invalidState(code)` in this exact order:

1. `totalBytes < 2^64`.
2. `totalBytes <= budget`.
3. `chunkCounter < 2^54`.
4. `blocksCompressed < 16`.
5. Logical buffer length is at most 64.
6. Every buffered natural is below 256.
7. Every current chaining word is below 2^32.
8. Every word in every stack CV is below 2^32.
9. `totalBytes = 1024*chunkCounter + 64*blocksCompressed + bufferLength`.
10. An empty buffer implies `totalBytes=0`.
11. Zero compressed blocks implies the current chaining value is IV.
12. Stack length equals the complete 54-bit popcount of `chunkCounter`.

These predicates constrain a supplied record structurally. They do **not**
prove that arbitrary supplied CV words arose from a particular input history.
Only a later application custody boundary can make an ongoing state authentic.
The immutable semantic APIs deliberately make no such custody claim.

For a valid state, update first checks the total additional length against
u64 and the caller budget, then every new byte in input order, then feeds it.
The byte error index includes the previous total length. Failure does not
return a partially advanced state. `blake3Init` accepts a natural budget;
a budget above u64 does not enlarge the independent u64 message domain.
An empty update preserves a pending full block/chunk exactly.

Before an actual next byte, a full buffer with 15 earlier blocks is finalized
as a non-ROOT chunk and merged; otherwise a full buffer becomes an ordinary
non-CHUNK_END block. Append then adds that next byte. The temporary empty
buffer created inside that transition is never returned as an admitted
nonempty final state. At the largest admitted total length, the public length
guard prevents advancing the largest chunk index beyond2^54-1.

When a chunk is committed, `totalChunks=chunkCounter+1` is positive.
Each trailing zero consumes the newest prior stack CV as the left child and
divides that count by 2. The structural popcount invariant gives enough entries
for this process; encountering an empty stack while the count is even reports
`internalInvariant(1)` instead of fabricating a CV. When the count is odd,
push the current merged CV at the front. On finalize, fold that newest-first
stack with `parent(left,current.CV)`. Review this against the reference's
array-backed reverse traversal, particularly the non-power-of-two chunk cases.

Unkeyed whole-hash paths construct only flags 1/2/3/4 and add 8 exactly once
for the final root compression. Their flag values have disjoint bits, so
addition equals bitwise OR here. Generic checked compression permits any
32-bit flags for low-level algorithm inspection; that helper is not a claimed
unkeyed application root. Likewise raw arithmetic/core/state helpers depend
on their documented internal domains. Source visibility does not make these
helpers authenticated or safe application entrypoints.

## Arithmetic and literal-input risk checklist

- Every modulo-add is exact Nat addition followed by modulo 2^32. The XOR
  fold always visits 32 bits; round rotations are 16,12,8,7. `amount=0` uses
  divisor 1 and has no exceptional shift. No division denominator is zero.
- Compression initialization assigns counter halves, length and flags
  directly. It never uses BLAKE2's IV-XOR initialization. The message
  permutation is the specified gather. Seven rounds and both feed-forward
  halves must remain present; the unused permutation after the last recursive
  round does not change the returned state.
- Word loads are little-endian. Out-of-buffer positions are genuine zero
  padding, while `blockLength` remains the logical buffer length. Neither
  SHA padding nor an extra empty chunk is introduced at exact multiples.
- The parent input CV is IV, message is left CV followed by right CV, length 64,
  counter 0, flags PARENT. The final root output counter is 0 even when the last
  chunk counter is nonzero. Digest serialization is exactly 32 LE bytes.
- Literal `packed(count,payload)` admits count 0..64 and payload strictly below
  `256^count`. Therefore count 0 requires payload 0. Division by 256 produces
  exactly count bytes, preserving zeros at either end. Concatenation feeds
  left before right through actual streaming update; no vector length or
  payload selects a digest. Full-tree validation precedes any feed result.
- Packed payloads can require 512 bits. They are formal test-input containers,
  not a claimed production byte ABI. Ordinary `List Nat` hash/update APIs
  remain available. Natural arithmetic can create values above u64 before
  reduction; checked-u64 generated Nat lowering therefore still requires a
  separate lawful production representation/export contract.
- Public recursive helpers and literal trees do not establish a wall-clock,
  stack-memory, constant-time or allocation bound. Structural totality and
  fixed word/count domains must not be reported as D11 serving compliance.
  No native hash, external oracle, foreign code, or digest lookup is used.

## Evidence and independent review record

The author prepared this map and its machine-checked 91-name inventory at
`/tmp/lexlean-r4-blake3-nat-20261021/source-review-map.json`.
It supplies review context; it is not independent evidence of correctness.
The implementation freeze remains SHA-256
`57c8e4da4ebbc00081cc0267cc7b716cfa12a6886a76fc8ba3f743f0243a3f01`.

On 2026-10-21 the parent agent independently read all four source modules as
semantic ASTs and the pinned reference. The parent reported no static
algorithm discrepancy in G/round wiring, permutations, seven rounds,
feed-forward, counter initialization, pending blocks/chunks, stack merge/fold,
parent/root domains, scalar/packed bounds and ordered admission. That is a
**parent source review**, not an author approval and not an execution result.
The parent explicitly retained the structural-state/provenance distinction
and Nat/packed-input production-lowering gaps.

At this review boundary 252 of 272 frozen formal controls have audited positive
receipts. The remaining 20 published cases, all nine semantic defect plants,
restoration, final source review closure and actual Rust/integration gate are
still required. The maximum published input is 102400 bytes. No independent
model execution, model capability or serving performance follows from these
cryptographic formal controls. See `../BLAKE3.md` for exact observed receipts.
