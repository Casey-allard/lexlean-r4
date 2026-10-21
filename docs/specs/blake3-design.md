# Pure unkeyed BLAKE3-256 for tokenizer identity

Status: complete algorithm design for independent review; no new source,
scenario, registered capability or verified tokenizer identity is introduced.
The scope is the ordinary unkeyed BLAKE3 hash with exactly 32 output bytes,
including arbitrary finite admitted input, incremental updates, all chunk/tree
boundaries, and final ROOT compression. Keyed hashing, key derivation and a
public arbitrary-length XOF are separate APIs outside this tokenizer need.
Their exclusion does not permit omitting any part of the unkeyed hash tree.
SHA-256 and BLAKE2b-96 are different algorithms and cannot supply this identity.

## 1. Exact authorities and evidence custody

UOR authority is revision `4611538bb4843947363ccb9cce5f388e32c46883`.
Its Cargo.lock pins `blake3` 1.5.0 with package checksum
`0231f06152bf547e9c2b5194f247cd97aacf6dcd8b15d8e5ec0663f64580da87`.
The retained local `.crate` archive's SHA-256 matches that checksum. The
package's portable implementation and common implementation were read;
no BLAKE3 library or reference executable was run.

The official [1.5.0 source revision](https://github.com/BLAKE3-team/BLAKE3/tree/5aa53f07f7188a569cadfc5daf1522972d9a9630)
is commit `5aa53f07f7188a569cadfc5daf1522972d9a9630`, resolved from the official
1.5.0 tag. Its readable reference implementation and published test vectors
provide independent authority for the proposed LexLean implementation.
The [official specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/ac784c9f22a48327782f042ec2f4d8126b3b1744/blake3.tex)
is separately pinned to `ac784c9f22a48327782f042ec2f4d8126b3b1744`; its
compression/chunk/parent/tree descriptions were compared with the 1.5.0 code.

| Exact source bytes | SHA-256 |
| --- | --- |
| UOR `crates/uor-r4-tokenizer/src/lib.rs` | `52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35` |
| blake3-1.5.0 package `src/lib.rs` | `c30bdfbee02c4715cc619e365a59f5278eb022498092f8d8e7f4ed06fae2d717` |
| blake3-1.5.0 package `src/portable.rs` | `59a1c66492af606105c51fb2f59acf0d3a79e99f614bb46cc02878987f9c9e2c` |
| Official 1.5.0 `reference_impl/reference_impl.rs` | `2fae31faabb79094ebb8c8377e5f1d804ae46902f8cc2a8b31e14d2632948ca0` |
| Official 1.5.0 `test_vectors/test_vectors.json` | `dcb91ea8accc77e6d6e632af7cdc1a99a9f3ae78cf648da595c7d064db32f624` |
| Official 1.5.0 `test_vectors/src/lib.rs` | `92ac641e0e47c11bfa0e68d6a815bef09d0c5a7c24845210cfa862ccceb35e84` |
| Pinned specification `blake3.tex` | `db9151f3eca2f63cfbed2c685b90e2e7e913d455eddbec20158c89613ee7e887` |

Research copies are retained under `/tmp/r4-blake3-*`. Before implementation,
freeze the full published vector file, its projected input/digest inventory,
source identities and licenses in the reviewed project fixture packet. These
are imported inputs, not generated serving tables or algorithm substitutes.

## 2. Compression function

Use fixed32-bit words with arithmetic modulo 2^32. The newer pinned SDK admits
language-1.1 `UInt32`/`UInt64`, checked arithmetic, bit operations and checked
shifts. These pure LexLean primitives are the preferred representation if
none-axiom controls confirm their actual compiler behavior. The existing
`R4.Words` Boolean representation remains an available exact fallback; its
legacy necessity does not impose it on this new algorithm. Neither choice
changes the independent digest expectations or permits a native hash call.

There is no `WrapAdd` primitive in the inspected semantic IR, and ordinary
`Add` accepts Nat only. Define modulo addition using admitted pure operations,
for example checked widening/addition/masking/narrowing for 32-bit words, or
checked overflow decomposition. In the latter form, if checked `a+b` overflows,
`a - (MAX-b) - 1` is the exact wrapped result and both subtractions are in
range. Checked conversions/shifts must propagate typed unexpected failures,
never return a zero/default word. Prove boundary/carry cases before selecting
the representation and record its compiler identity; do not assume it is faster
without actual proof-cost diagnostics. BLAKE3 needs right rotations16,12,8,7
constructed from bounded shifts and OR. It requires no float, transcendental,
dense multiplication or BLAKE2b64-bit mixer.

Define a fixed eight-word chaining value and sixteen-word message/state. The
unkeyed IV is, in word order:

```text
6a09e667 bb67ae85 3c6ef372 a54ff53a
510e527f 9b05688c 1f83d9ab 5be0cd19
```

Compression accepts input CV h[0..7], sixteen little-endian message words,
a 64-bit counter t, actual block length b, and internal flags d. Initialize
state as `[h0..h7, IV0,IV1,IV2,IV3, low32(t),high32(t),b,d]`.
These final four words are assigned directly. They are not XORed with the
remaining IV words as in BLAKE2.

Each G(a,b,c,d,x,y) performs this ordered sequence:

```text
a = a+b+x; d = rotr(d XOR a,16)
c = c+d;   b = rotr(b XOR c,12)
a = a+b+y; d = rotr(d XOR a,8)
c = c+d;   b = rotr(b XOR c,7)
```

Every addition wraps at 32 bits. A round first applies G to columns
(0,4,8,12), (1,5,9,13), (2,6,10,14), (3,7,11,15), then diagonals
(0,5,10,15), (1,6,11,12), (2,7,8,13), (3,4,9,14), using successive
message-word pairs0/1 through 14/15. Perform exactly seven rounds. Between
rounds, replace message word i with the previous message word at
`[2,6,3,10,7,0,4,13,1,11,12,5,9,14,15,8][i]`. This is a gather, not its inverse.
No permutation is needed after round seven.

Return sixteen words: output[i] = state[i] XOR state[i+8], and
output[i+8] = state[i+8] XOR original_h[i], for i0..7. A chaining value uses
the first eight; compression retains the complete defined result. Output
byte encoding is little-endian. Giving the first half alone a digest type
before ROOT processing would be a domain error.

Internal flags are CHUNK_START=1, CHUNK_END=2, PARENT=4, ROOT=8. Ordinary
unkeyed mode has base flags 0 and IV as its key words. KEYED_HASH=16 and the
two derive-key flags 32/64 must never be set by this public API. Use closed
internal roles or proved flag constructors so arbitrary caller flags cannot
turn a tokenizer hash into a different mode.

## 3. Blocks, chunks, tree, and root output

Partition bytes into contiguous1024-byte chunks and chunks into64-byte
blocks. Only the entire empty message has an empty final chunk/block. Exact
multiples of64 or1024 do not append an empty block or chunk. Short final
blocks are zero-filled only for loading16 words; actual length remains the
number of message bytes. There is no0x80 marker or appended bit length.

A chunk starts with IV. Its every block compression uses the same zero-based
chunk index as counter, not byte count or block index. CHUNK_START marks
only its first block; CHUNK_END marks its final block, including a full64-byte
final block. A one-block chunk has both. Empty input has length 0, counter 0,
zero message words, and both flags. Intermediate blocks update CV from the
first eight output words. Retain the input to the final compression as an
`OutputNode` containing input CV, block words, chunk counter, length and flags.
This supports the essential choice between an internal CV and ROOT output.

A parent combines exactly two child CVs, left then right, as its16 message
words. Its input CV is IV, counter 0, block length 64, flags PARENT. Do not use
the left child CV as input CV or recursively hash a serialized child record.
The tree is left-complete: for n>1 chunks, split off the largest power of two
strictly less than n as the left subtree; the right subtree contains the
remainder. Thus three leaves group `((0,1),2)`, five group
`(((0,1),(2,3)),4)`, and seven group
`(((0,1),(2,3)),((4,5),6))`. A naïve left fold differs from the required tree
at four leaves and beyond.

After obtaining the final chunk or parent OutputNode, invoke compression
with its flags OR ROOT and output-block counter 0. Encode the first eight
output words little-endian to produce the 32-byte digest. Do not return a
previous non-ROOT CV, hash the CV again, or perform an extra parent hash for a
single chunk. Fixed32-byte output is the prefix of the specification's root
output stream; it needs only output block 0. Full XOF/seek is not claimed.

## 4. Streaming construction and bounded totality

Proposed public interfaces, with globally distinctive BLAKE3 names, are
`blake3Hash256(input)`, `blake3Hash256Bounded(maxBytes,input)`,
`blake3Init(maxBytes)`, `blake3Update(state,bytes)`,
`blake3Finalize256(state)`, and `blake3EncodeDigest(digest)`. This is a
complete streaming alternative to assembling a huge tokenizer record in one
allocation; whole-list and incremental results must agree for any admitted
partition. Public input is natural-valued bytes with explicit0..255 admission.
Digest representation is a dedicated eight-Word32 record with LE encoding,
not the SHA digest encoder's BE convention.

Retain current chunk CV, its index,0..15 compressed-block count, a zero-filled
64-byte block with actual length 0..64, total admitted byte count, configured
budget, and a stack of completed left-subtree CVs. A full block is compressed
without CHUNK_END only when another byte is known to follow within the chunk.
A full1024-byte chunk is converted to its non-ROOT CV and pushed/merged only
when further input is known to follow. Therefore an update ending exactly
on a block/chunk boundary leaves that final block/chunk pending. An empty
update is an identity, including in those states.

When committing chunk index q, set completed count q+1. While this count is
even, pop the latest left CV, form parent(left,current), and halve the count.
Push the resulting CV. The stack represents the binary decomposition of
completed chunks and preserves left-to-right subtree order. On finalization,
start from the pending chunk OutputNode; consume stack entries from newest to
oldest, making each parent(stack_left,current_output.CV). Apply ROOT only to
the ultimate output. Finalization is pure and does not consume the state;
repeated finalization agrees, and a later update resumes the unfinalized state.

The specification admits byte lengths0<=L<2^64. The public model accepts at
most18446744073709551615 bytes and rejects its successor. Counts are checked
naturals or refined words; never wrap the cumulative byte count. This permits
at most2^54 chunks, with largest valid chunk index2^54−1. An eager-merge stack
of 54 CVs suffices for the source reference algorithm. A list implementation
may retain a proved bound instead of relying on an unchecked fixed stack.
The optimized crate uses a different lazy-merging allocation; reproducing
its transient 55-slot layout is unnecessary if tree/output equivalence holds.

Proposed ordered failures for an update are `MessageTooLong(total)` first,
then `BudgetExceeded(max,total)`, then the first `InvalidByte(absoluteIndex,value)`.
The index includes previously accepted updates. Pure update returns an error
without returning a partially committed state; prior valid state remains
available. A budget may exceed the standard limit as metadata, but cannot
expand algorithm admission. Whole-list hashing is init plus one update plus
finalization; it preserves the same order. These typed errors are an explicit
safe LexLean boundary; Rust `blake3::hash(&[u8])` does not have invalid-byte
inputs or this Result API.

State constructors should be internal to the admitted API where possible.
If raw states are exportable/importable, a separate typed structural validator
must check lengths, counter relation, stack shape and zero-filled unused
bytes. Structural validity alone is not proof of a state's prefix history;
trusted tokenizer identity only accepts states derived by actual init/update
or a separately authenticated and verified checkpoint protocol. Malformed
state, empty-stack underflow, bad length/counter/flag shape, and unexpected
codec failure must never select zero/default data. Input-derived recursion
must process every byte; fixed verifier cost is not an algorithmic fuel limit.

## 5. Independent published vectors

The pinned official JSON has 35 cases. Each input is the first L bytes of the
repeating sequence0,1,...,250,0,1,...; each `hash` field contains 131 bytes of
unkeyed output. For this API, freeze its first 32 bytes, exactly as the official
vector commentary instructs for default output. Do not use keyed_hash or
derive_key fields. The literals below were extracted from the file; no
candidate or host hash produced them.

| L | BLAKE3-256 hex |
| --- | --- |
| 0 | `af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262` |
| 1 | `2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213` |
| 63 | `e9bc37a594daad83be9470df7f7b3798297c3d834ce80ba85d6e207627b7db7b` |
| 64 | `4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98` |
| 65 | `de1e5fa0be70df6d2be8fffd0e99ceaa8eb6e8c93a63f2d8d1c30ecb6b263dee` |
| 1023 | `10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11` |
| 1024 | `42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7` |
| 1025 | `d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444` |
| 2048 | `e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a` |
| 2049 | `5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030` |
| 3072 | `b98cb0ff3623be03326b373de6b9095218513e64f1ee2edd2525c7ad1e5cffd2` |
| 4096 | `015094013f57a5277b59d8475c0501042c0b642e531b0a1c8f58d2163229e969` |

The complete length inventory is
`0,1,2,3,4,5,6,7,8,63,64,65,127,128,129,1023,1024,1025,2048,2049,3072,3073,4096,4097,5120,5121,6144,6145,7168,7169,8192,8193,16384,31744,102400`.
All35 belong in the acceptance inventory, not merely the displayed subset.
Large fixture inputs may be stored in bounded literal chunks with proved
ordered concatenation and length, avoiding the JSON parser's nesting limit.
An input pattern generator must itself be audited against the exact pattern;
it must never compute expected digest values.

## 6. Tokenizer-specific identity fixtures and evidence gap

UOR tokenizer lib.rs:231 hashes the exact raw JSON byte slice, rendering
`blake3:` followed by64 lowercase hexadecimal digits. Whitespace, final newline,
key order and ignored metadata remain part of that identity. Hashing parsed
or normalized JSON is wrong even when the resulting tokenizer behavior agrees.

At375–405, added entries are sorted by numeric token ID. Each listing entry is
ASCII decimal `id:UTF8ByteLength:`, exact content bytes, and newline. Unicode
character count must not replace byte length. Colons/newlines in content are
preserved literally; the length prefix disambiguates them. Empty added entries
produce an empty listing, so this particular application's added-list digest
already has an independent published answer:
`blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262`.

The adapter hashes its canonical line record (824–856), with a final newline
on every line, in this order:

```text
uor-r4-tokenizer-adapter/1
family=hf-byte-bpe
version=1
tokenizer_cid=<actual raw-JSON CID>
policy.normalizer=none
policy.pre_tokenizers=<effective policy strings joined by comma>
policy.byte_fallback=byte-level-alphabet
policy.added_tokens_count=<decimal count>
policy.added_tokens_digest=<actual listing digest>
policy.bos=none
policy.eos=none
policy.chat_template_policy=not-interpreted
```

The angle-bracket fields above describe values; they are not fixture bytes.
The adapter_digest field is not included in its own hashed record. BLAKE3's
unkeyed flags remain ordinary 0: these application strings are message bytes,
not a request to use BLAKE3 derive-key context mode. Added-list digest, raw CID,
and final adapter digest must be connected through real BLAKE3 calls and
correct lowercase serialization, not caller-supplied trusted fields.

The existing tokenizer test at893 computes its expected CID with the same
Rust blake3 library at runtime, and its adapter assertion calls
`declared_digest` again. Static inspection found no literal independent CID
or adapter digest in that crate's tests or the inspected object-store test
paths. This is an evidence gap, not a reason to fabricate literals or treat
candidate output as an oracle. The standard pattern vectors do not by
themselves independently bind a concrete tokenizer JSON/listing/adapter.

Before the identity-composition component begins, freeze a packet with:

1. Minimal valid JSON M from docs/TOKENIZER.md, exact UTF-8 bytes with no final
   newline, plus M with one final newline. They have the same admitted
   semantic body but distinct raw inputs and independently pinned CID values.
2. The pinned upstream multiline FIXTURE from tokenizer lib.rs:864–872,
   retaining indentation/newlines and non-ASCII bytes exactly; plus empty
   and nonempty added-list serializations. The source fixture's `<end>` entry
   yields the exact listing bytes `8:5:<end>\n`.
3. A listing with entries(10,`a:b`) and(2,`é\n`), whose exact UTF-8 bytes are
   `2:3:é\n\n10:3:a:b\n` in escaped notation. Assert serialization separately
   before asserting its independently pinned digest.
4. Fully literal canonical adapter bytes for M and the upstream fixture,
   including independently sourced raw/listing digest strings, exact policy
   order and final newline. Assert the composed constructor equals those
   bytes and its actual hash equals the independent adapter digest.
5. At least one larger JSON input spanning1024 bytes and another spanning an
   irregular multi-chunk tree, with independent complete raw-byte/digest
   records; preserve unexecuted metadata and spelling in their CID.

Preferred evidence is an already published exact artifact plus trustworthy
independently recorded digest whose original bytes are obtainable. If none
exists, obtain a separately reviewed and authorized reference-vector packet,
with source/executable/input identities and a genuinely independent checker,
then freeze it before candidate identity code. A compatible external tool's
output is not independently pinned evidence unless its identity and exact
input bytes are retained and reviewed. This design does not authorize or run
host reference hashing to manufacture that packet. Standard vectors and
empty-listing identity can proceed without closing this additional gap.

## 7. Formal controls, cost, and partition plan

Register the complete component and source authorities before adding source
or scenarios. Proposed future ownership is `src/R4/Blake3.lex.tex`, theorem
modules `tests/lexlean/Blake3CaseNN.lex.tex`, a frozen original vector/inventory packet,
`crates/conformance/tests/blake3.rs`, and its registered feature suite; final
paths/IDs remain parent decisions. The Rust harness must only check fixture
coverage, invoke actual LexLean verification, and audit fresh receipts,
source/config/lock bindings, none-axiom policies, and kernel replays.

Freeze all 35 official unkeyed32-byte expectations before definitions, then
capture missing-definition RED. Start proof-cost investigation with one full
compression, empty input,64/65 bytes,1024/1025 bytes, then a three-chunk case.
This sequence evaluates feasibility; it does not reduce the final inventory.
Do not imply the previously observed SHA/BLAKE2 proof times predict BLAKE3
throughput. BLAKE3 processes seven rounds per compression, but102400 bytes
requires1600 message-block compressions plus its tree, so large direct
normalization may exceed a300-second module budget even when one case is
compiled separately.

Initially put each complete public digest theorem in its own module, as for
R4-11, with unchanged input and expected digest. Group only cheap structural
controls. If large complete theorems exceed the fixed SDK limits, retain their
statements and develop connected block/chunk/tree certificates. Before
expanding a certificate inventory, demonstrate an actual two-transition proof
that composes supported proof constructors under the none-axiom policy.
Previous hash work showed that blanket simplification or unsupported proof
sequencing can fail; source feasibility cannot be inferred from a plan.

A sufficient certificate chain starts from exact admitted bytes/initial state,
proves every actual compression and state transition, binds pending buffers,
lengths/counters/flags and full CV stack at every adjacency, then proves the
ROOT finalization and exact independent digest. Intermediate literals may
come from compiler diagnostics, but each transition must be proved and those
literals are not independent oracles. A conjunction of detached examples or
an audit of matching intermediate strings is insufficient without the actual
end-to-end digest theorem. Shared prefix lemmas are reusable only with exact
message words, chunk counter, flags and incoming CV identities.

Controls in addition to the 35 digests must cover:

- G carry/wrap, all rotations, column/diagonal order, seven-round permutation
  orientation, direct counter/length/flag initialization, and both feed-forward
  halves. Projection controls distinguish high and low counter words without
  pretending to hash a physically enormous input.
- Empty, exact block, exact chunk and one-byte-over transitions; empty update
  in every pending-buffer state; no extra block/chunk on finalization; exact
  lengths and zero fill without a SHA padding marker.
- Chunk-index persistence across all sixteen blocks, index increment only
  between chunks, parent counter 0, IV input CV, ordered children, and the
  correct3/4/5/7-chunk reduction shape.
- Final flags 11 on empty or one-block single-chunk roots; flags 10 on the last
  block of a multi-block single-chunk root; flags 12 on a parent root. Prior
  non-root CVs must remain flags without ROOT.
- Whole-input versus one-byte, irregular,63+1,64+1,1023+1 and1024+1 updates,
  each connected to the same published digest. Multiple finalizations agree;
  finalize then update agrees with an independently admitted longer input.
- Exact standard length predicate at2^64−1 and2^64; budgets below/equal/above
  actual length; first invalid byte and cumulative index; length/budget before
  invalid-byte priority; malformed imported state if that API exists.
- Final LE 32-byte encoding and lowercase64-digit hex formatting, including
  leading zero bytes. No caller digest/identity truth flag is admitted.

Required semantic plants include dropping ROOT, changing the root output
counter from zero to one, changing length to padded64, appending an empty block/chunk, changing
chunk counter to block/byte count, replacing parent IV with left CV, reversing
children or folding four chunks left-associatively, wrong message permutation,
and using BE words. Tokenizer composition additionally plants normalized-JSON
hashing, scalar instead of byte length, omitted listing/adapter newline,
derive-key mode, SHA substitution, and trust in a supplied digest. Each plant
must remain wellformed, fail the intended actual equality gate, then restore
exact reviewed bytes and rerun a positive control. Syntax/depth/timeout
failures do not satisfy the negative semantic obligation.

Partition manifests must positively enumerate all 35 original declarations and
any additional named controls; compare complete declarations against frozen
sources and reject missing, duplicated, filtered or modified cases. No timeout
increase, axiom allowance, ignored case, alternate reduced acceptance flag,
fixture lookup runtime, or host hash fallback can close an unresolved proof.

## 8. Implementation order and completion boundary

1. Register scope/authorities and freeze all original vectors plus typed
   admission/state controls; demonstrate RED before implementation.
2. Define fixed representations, compression and output-node semantics with
   existing Words operations; verify complete short published digests.
3. Implement complete pending block/chunk transitions, exact stack reduction
   and ROOT finalization; establish streaming/whole-input equivalence controls.
4. Verify every original vector, including irregular and large trees, through
   unchanged direct theorems or genuinely connected complete certificates.
   Preserve proof-cost blockers explicitly if the SDK cannot compose them.
5. Review the algorithm, Rust gate, immutable fixture coverage and semantic
   plants; run all required fresh acceptance checks under parent integration.
6. After independent tokenizer identity fixtures exist, connect the actual
   BLAKE3 API to raw JSON, listing and adapter bytes, with separate composition
   controls and provenance review.

Complete unkeyed BLAKE3 semantics and finite formal conformance do not prove
cryptographic collision resistance, production runtime performance, GPU parity,
or browser execution. Tokenizer identity remains incomplete until its exact
byte construction and actual hash calls are accepted together. The design
preserves those obligations and performs no reference hash, model evaluation,
compiler job, source implementation, or registration change.
