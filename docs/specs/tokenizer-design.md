# Exact byte-BPE and tokenizer identity contract

Status: source research for the approved full-scope implementation. No tokenizer
implementation is claimed. Authority is UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-tokenizer/src/lib.rs`, SHA-256
`52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35`.
The source repository pins Rust 1.97.1. Unicode classification tables are part
of that implementation identity and must be pinned before claiming exact
non-ASCII segmentation; ASCII-only examples do not establish Unicode parity.

## Admission and vocabularies

The source accepts a JSON object with a model vocabulary and merge array. If
`model.type` is a string it must equal `BPE`; absent or non-string values do
not trigger that particular rejection. The model vocabulary is nonempty and
at most 2^20 entries. Its IDs are distinct UInt32 values forming a dense prefix
of exactly its entry count. Added-token IDs may extend the final ID-indexed
table sparsely, but maximum ID plus one must still be at most 2^20.
The model vocabulary may contain an empty string. Absent, null or non-array
`added_tokens` is treated as empty; element validation applies only to an
actual array.

An added token has nonempty content and a UInt32 ID. Repeating the same
content/ID is idempotent, including an exact model-vocabulary entry. One ID
cannot alias different content, and one content cannot alias different IDs.
Sort atomic match candidates by descending UTF-8 byte length, then lexical
content. Empty added content is rejected before matching, ensuring progress.

Merge entries may be a string with exactly one ASCII space separator or an
array containing exactly two strings. Preserve the first occurrence of a
duplicate merge key; its rank is its original position in the merge list.
Reject an original merge-list index above UInt32 MAX.
Do not rank merges by resulting token ID. The source accepts empty components
and array components containing spaces through its string-key construction;
any stronger structural restriction must be an explicit strict-admission
profile, rather than an unannounced compatibility change.

The supported pre-tokenizer is one ByteLevel step, optionally preceded by
Digits steps inside a Sequence. Every Digits step before ByteLevel updates
the pending policy, so multiple such steps retain the last value. Any step
after ByteLevel, duplicate ByteLevel, unknown step or missing ByteLevel is
rejected. Missing/non-Boolean `individual_digits` defaults to false;
missing/non-Boolean `add_prefix_space` defaults to true. A null or absent
pre-tokenizer is rejected. Model all of these permissive parser distinctions
separately from strict application admission.

The runtime applies no normalizer, post-processor, BOS/EOS insertion or chat
template. Those source fields may be present in the input bytes without being
executed by this adapter. Dialogue framing is a separate explicit consumer.

## Byte alphabet and atomic matching

Map bytes 0x21..0x7e, 0xa1..0xac and 0xae..0xff to equal-valued Unicode
scalars. Map the remaining bytes, in increasing byte order, to successive
scalars beginning at U+0100. This creates the full invertible 256-byte alphabet.
It is not UTF-8 decoding of the input bytes.

Before pre-tokenization, find the leftmost added-token occurrence. At a shared
position prefer longest content, retaining the established lexical tie order.
Emit preceding plain text, then the atomic ID, and continue after its complete
UTF-8 content. Added tokens bypass digit splitting and BPE.

## Segmentation and merging

For each non-special segment, optional Digits splitting uses the pinned Rust
`char::is_numeric` predicate. Split numeric and nonnumeric runs; individual
mode makes each numeric scalar its own split. Prefix-space insertion occurs
**after** this split, independently for every nonempty piece, when enabled
and the piece does not begin with literal ASCII space. A tab does not satisfy
that literal-space check.

ByteLevel pre-tokenization follows the actual source state machine:

1. At an apostrophe, recognize the first matching lowercase contraction from
   `'s`, `'t`, `'re`, `'ve`, `'m`, `'ll`, `'d` before the other branches.
2. Optionally attach one literal ASCII space to a following alphabetic,
   numeric or punctuation run. Alphabetic and numeric use the pinned Rust
   predicates, not an assumed ASCII or differently versioned regex category.
3. Punctuation runs consume scalars that are neither whitespace, alphabetic
   nor numeric. Contraction detection is at the current cursor; punctuation
   consumption does not restart it at each internal apostrophe.
4. Consume a whitespace run. When a non-whitespace scalar follows a run of
   more than one scalar, leave its final whitespace scalar for the next
   iteration. Only ASCII space is eligible to attach in the next branch.

Encode each pre-token to UTF-8 bytes and map every byte to its alphabet scalar.
Repeatedly find the adjacent symbol pair with the least merge rank, preserving
the earliest pair on equal rank. Then merge **all nonoverlapping occurrences
of that selected symbol pair** from left to right in one pass. Repeat on the
new symbol list until no pair has a rank. A complete recursive model decreases
symbol count on every successful pass; no arbitrary iteration cutoff is valid.

For each final symbol, use its exact vocabulary ID if present. Otherwise the
source tries each alphabet character separately and skips characters absent
from the vocabulary. The parser does not enforce a complete byte alphabet,
despite comments describing total encoding for well-formed byte vocabularies.
Preserve this behavior in a compatibility surface, and require complete
alphabet coverage plus exact round-trip checks for the durable-memory adapter.
An omitted byte must never be accepted as a lossless conversation encoding.
The compatibility `encode_lossy` still returns `(encode(text), 0)` even when
missing symbols were skipped; its zero replacement count is not a losslessness
certificate. The pinned object-store `bindings.rs::PinnedTokenizer::new`
explicitly rejects `add_prefix_space=true` before encoding. Complete-alphabet
admission is an additional strict profile beyond that constructor's checks.

The bounded source cache affects reuse only. Its semantics can be omitted from
the authoritative encoding result; a generated cache requires independent
identity/capacity and observable-equivalence checks before performance claims.

## Decoding and limits

Raw decode preserves bytes. Added tokens emit their literal UTF-8 content;
ordinary tokens reverse the byte alphabet, and any character outside that
alphabet emits its ordinary UTF-8 encoding. Unassigned empty slots emit no
bytes, and out-of-range IDs are skipped by the permissive batch decoder.
Text decode applies lossy UTF-8 conversion only after concatenating all bytes,
so two token fragments can complete one scalar before replacement decisions.

`token_byte_len` is deliberately stricter: unassigned, empty and out-of-range
IDs return no length. Model a strict serving decode that uses these lengths to
admit byte budgets before allocation. Do not silently replace the permissive
compatibility behavior with that strict surface or discard partial UTF-8 bytes.
Input scalar validity, UTF-8 encoding, malformed JSON and bounded allocation
need typed errors at their respective layers.

## Identity domains

Tokenizer CID is `blake3:<lowercase hex>` over the **raw tokenizer JSON bytes**.
Formatting changes therefore change the CID. SHA-256 is not a replacement.
A pure BLAKE3 implementation remains a separate dependency beyond node
BLAKE2b-96 and SHA-256.

The adapter family is `hf-byte-bpe`, version 1. Sort added tokens by ID and
concatenate each as ASCII decimal `id:byteLength:`, exact content bytes, then
newline. Hash that listing with BLAKE3. Preserve byte length even if content
contains a colon or newline.

The adapter digest hashes a fixed UTF-8 line record with tag
`uor-r4-tokenizer-adapter/1`, then `key=value` lines in this exact order:
family, version, tokenizer_cid, policy.normalizer, policy.pre_tokenizers,
policy.byte_fallback, policy.added_tokens_count, policy.added_tokens_digest,
policy.bos, policy.eos, policy.chat_template_policy. Every line ends in newline.
Pre-tokenizer policy strings are comma-joined in application order:
`digits(individual_digits=true|false)` when present, then
`byte-level(add_prefix_space=true|false)`, with the chosen Boolean spelled
literally. Multiple preceding Digits steps collapse to their final effective
single policy entry. Exact fixed values are `normalizer=none`,
`byte_fallback=byte-level-alphabet`, `bos=none`, `eos=none` and
`chat_template_policy=not-interpreted`. The adapter digest is itself rendered
as `blake3:<lowercase hex>` over those canonical bytes. The fixed
policy values and raw source CID bind unexecuted source metadata without making
it application behavior. This identity is distinct from the memory adapter's
SHA-256 domain described in [memory-protocol-design.md](memory-protocol-design.md).

## Required acceptance

Controls must distinguish rank from merged-token ID, simultaneous nonoverlapping
merges from one-pair merging, duplicate rank retention, leftmost-longest special
matches, digit-prefix ordering, Unicode classifications, contraction positions,
whitespace tails, partial UTF-8 decode, strict/permissive ID handling, sparse
added IDs, aliases, invalid parser shapes and byte budgets. Independent raw
JSON/CID and adapter-listing/digest vectors must bind identity serialization.
Compiler-generated native text/byte projection, complete Unicode tables,
BLAKE3, model-vocabulary binding and dialogue policy remain integration
obligations. A presegmented ASCII BPE fixture does not complete this tokenizer.
