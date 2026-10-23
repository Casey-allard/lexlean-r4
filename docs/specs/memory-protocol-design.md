# Exact memory protocol design

Status: source-contract research, 2026-10-21. No memory implementation or memory
acceptance is claimed. This elaborates the approved full-scope design using
UOR-R4 revision `4611538bb4843947363ccb9cce5f388e32c46883` and the exact development
LexLean revision `01088ef00d9d0d0227775e0346a852c2d19cb67b`. Original checkouts were
read through `git show`; no upstream files were changed and no model, training,
or parity evaluation was executed. Hashes below identify the inspected bytes.

The deliverable remains a content-addressed ternary byte DAG, exact range and
whole-object verification, transactional import, peer replication, and durable
browser recovery. A key/value map alone does not implement it. Application
rules belong in LexLean; generated adapters may perform generic effects but
must not substitute handwritten hashing, traversal, or transaction decisions.

## 1. Identities and hash domains

Keep these types distinct even where their representations have equal widths:

| Identity | Exact derivation | Meaning |
| --- | --- | --- |
| `NodeId` | 12 raw bytes, BLAKE2b configured for a 12-byte output over `scale || payload` | One node, including its scale and ordered payload |
| `WholeObjectSha256` | SHA-256 over exactly the original logical object bytes | Entire object bytes, excluding atom padding |
| `ObjectVersion` | SHA-256 over canonical encoded `UOROBJ01` manifest | Length, ordered frontier, and object digest together |
| `TokenizerIdentity` | SHA-256 over ASCII `uor-r4-object-store/lossless-byte-bpe/1` followed by NUL, followed by tokenizer address UTF-8 bytes | Existing exact-byte BPE adapter policy and tokenizer source identity |
| `OccurrenceIndex` | Consecutive u64 index within an object/tokenizer binding | An occurrence, distinct from token ID and shared node ID |

The node hash is **BLAKE2b-96 with output length configured to 12 bytes**, not
the first 12 bytes of BLAKE2b-512. It is unkeyed, with fanout 1 and depth 1.
Initialization XORs `h[0]` with `0x0101000c`: `0x0c` is the digest-length byte,
not the fanout byte. The protocol prose incorrectly labels that byte once;
its equivalence paragraph and `hash.rs` establish the actual parameters.

For a leaf the logical hash message is 33 bytes (`0` then 32 payload bytes);
for a parent it is 37 bytes (scale then three 12-byte child IDs). Copy that
message into one zero-filled 128-byte compression block. Decode sixteen u64
message words little-endian. Use the standard BLAKE2b IV and ten sigma rows,
with twelve rounds selecting sigma row `round mod 10`. Set the byte counter to
33 or 37, **not 128**, and set the final-block flag. Each mix uses modulo-2^64
addition and rotations right by 32, 24, 16, and 63. Output little-endian H0 and
the first four little-endian bytes of H1 after final XOR. A generic hash facility
must expose these output-length semantics; a truncation API is insufficient.

The scale prefix and distinct payload lengths provide node domain separation.
The manifest has its magic prefix, but whole-object SHA-256 has no added prefix:
adding a new prefix would break compatibility. The 32-byte peer selector is an
untagged union matching either object digest or manifest version, resolved by
the first matching manifest. Content hashes are integrity commitments under
their usual collision-resistance assumptions, not signatures or permissions.
Do not assert an axiom that hashes are injective.

## 2. Canonical tree and exact byte layouts

Split bytes in order into 32-byte atoms and zero-fill only the final partial
atom. Empty input has no atom, no record, and no frontier root. Append each leaf
to an ordered frontier; whenever its last three entries have equal scale,
replace those three, in order, with their parent at scale plus one. Repeat the
carry until no triple remains. Equal content shares a record ID, but repeated
root or child occurrences remain in order and must all contribute bytes.

For `atoms = len / 32 + (len mod 32 != 0)`, the descending base-three digits
uniquely determine frontier scales, with each digit contributing zero, one or
two roots. Example: 32 atoms = base-three `1012`, so scales are `[3,1,0,0]`.
The order of IDs within equal scales is source order, not ID sorting. Object
length, last-leaf zero padding, root scales, and every parent/child scale step
are separate validation obligations.

The implementation's byte span is `32 * 3^scale`, checked in u64. Thus admitted
record scales are 0 through 37: span(37) = 14409084988511915616, whereas span(38)
= 43227254965535746848 exceeds u64. `PROTOCOL.md` describes the atom-span check
as `3^scale`; `lib.rs::span` is stricter and is the selected compatibility bound.
All offset additions and total padded spans also require checked arithmetic.

| Record field | Leaf | Parent |
| --- | --- | --- |
| Offset 0 | scale 0, one byte | scale 1..37, one byte |
| Offset 1 | ID, 12 bytes | ID, 12 bytes |
| Offset 13 | Payload, 32 bytes | Three ordered child IDs, 36 bytes |
| Exact total | 45 bytes | 49 bytes |

Record decode checks scale, exact length and recomputed ID. It does not by itself
establish that children exist or have scale one lower. Closure/range validation
must do that. Extra bytes to a single-record decoder are invalid.

| Manifest offset | Bytes | Encoding |
| --- | --- | --- |
| 0 | 8 | ASCII `UOROBJ01`, hex `554f524f424a3031` |
| 8 | 8 | Logical object length, u64 little-endian |
| 16 | 32 | Whole-object SHA-256, raw digest bytes |
| 48 | 1 | Root count n, maximum 128 |
| 49 | 13n | Each root: ID 12 bytes, then scale one byte |

Manifest size must be exactly `49 + 13*n`; reject trailing bytes. Validate the
object limit and canonical frontier scales, including zero roots for empty
input. Duplicate root IDs are legal repeated content, not duplicate records.
Canonical encoding is the input to `ObjectVersion`. No JSON, hex strings,
platform-width integers, or added version fields appear in this binary format.

Raw store export concatenates complete records sorted lexicographically by raw
NodeId. It has no file header or manifest; the manifest is separately pinned.
Raw import parses to exhaustion, rejects incomplete/trailing fragments, and
stages every record before publishing. An identical duplicate is idempotent in
this API; a different record claiming an existing ID is `IntegrityFailure`.
Complete additional valid records are further import entries, not trailing
garbage. Import need not require a complete object closure. The peer response
API below has a deliberately stricter duplicate policy.

## 3. Framing, sessions, and replication

Each frame is `kind:u8 || bodyLength:u32 big-endian || body`. The body length is
capped at both 64 MiB and `max_record_bytes`, before allocation. Counts inside
FETCH and NODES are **u16 big-endian**. All manifest-length and ID-word
little-endian rules are independent of frame/count endianness.

| Decimal kind | Body | Required transition |
| --- | --- | --- |
| 20, MANIFEST_REQUEST | Empty, or exactly 32 selector bytes | Empty selects first object; selector matches version or object digest |
| 21, MANIFEST | Exact encoded manifest | Validate schema and every supplied caller pin before requesting records |
| 22, FETCH | u16 count followed by exactly count IDs | Require a selected object and IDs within that object's closure |
| 23, NODES | u16 count followed by exactly count records | Correlate with outstanding request; validate every record and consume all bytes |
| 24, DONE | Empty | End session |
| 25, ERROR | Short UTF-8 reason | Unknown selector is a per-request outcome; clear selection and allow a later request |

The strict model must distinguish clean EOF at a frame boundary from truncated
headers/bodies. DONE with a body is invalid. No frame version byte is added.
New application capabilities need separately specified kinds or selector
formats, rather than overloading v1 without a declared version decision.

Caller pins are optional length, manifest version, and whole-object digest;
every supplied pin must match. The Rust API distinguishes no ExpectedObject
argument from an explicitly empty ExpectedObject, which fails matching. A
selector alone does not implicitly create a caller pin. Durable conversation
imports must carry their selected expected identity explicitly.

The full pull model is: receive and pin manifest; discover missing closure;
request deduplicated frontier IDs; validate and stage responses; repeat; validate
all reachable records, scales and padding against the combined local/staged
view; reconstruct exact logical bytes and check SHA-256; validate all final
capacity limits; atomically publish. No validation error may occur after the
publication boundary. Response delivery may fail after commit and is modeled
as an uncertain outcome, not a promise that the old state remains.

Missing-frontier traversal stops at an absent node because its children are
unknown. It descends through every present parent, including local parents
whose children may be absent after raw import. Track `(id, expectedScale)` to
reject inconsistent scale reuse. Deduplicate requested IDs while retaining
deterministic first traversal order. `Store::missing` returns sorted missing
IDs through a BTreeSet; peer missing discovery uses first traversal order.
Expose these as different ordered results if both compatibility APIs are kept.

Default fetch batches derive from
`min(4096, floor((recordBudget-2)/49), floor((recordBudget-2)/12))`, saturating
the subtraction and clamping the result to at least one. A requested batch is
clamped to 1..65535 in the source. Model impossible one-record budgets as errors
before issuing an oversized request. Default fetch window is four, further
bounded by `max(1, floor(262144/(2+12*batch)))`; compute this before allocating
in-flight buffers. Responses are consumed in request order. Each response must
contain exactly the requested ordered IDs, with no duplicate, foreign, omitted,
or trailing records. These order checks implement protocol prose and the
approved strict design; the pinned client only checks set membership.

`verify` checks pins and local closure without fetching or payload rehashing.
Its result means closure completeness, not whole-object digest verification.
`verify_series` pipelines windows of 64 requests and reports verified/stale,
unknown selector, or binding mismatch in request order. `pull_series` commits
one fully verified object at a time and sends one DONE at the end; it is not
an all-or-nothing multi-object transaction. Server request count is bounded
by max_visits, and cumulative served record bytes by max_record_bytes.

## 4. Range reads, occurrence bindings, and cursors

Ranges are half-open `[start,end)`, with `0 <= start <= end <= manifest.len`.
Enforce max_returned_bytes and precompute intersecting occurrence visits before
reserving output. Traverse roots and three children in source order, validating
expected ID and scale on each fetched record. Authenticate every external
record through the decoder. For the final partial leaf, reject nonzero bytes
past manifest.len, even if the requested range ends earlier within that leaf.
An empty range needs no record fetch. Repeated DAG occurrences count as visits;
distinct IDs are a separate statistic. visited_record_bytes sums wire sizes
over occurrences rather than estimating network or cache traffic.

`RangeVerifiedAgainstRoot` carries exact bytes, manifest version, requested
range, read stats, and unverified remainder intervals `[0,start)` and
`[end,len)` when nonempty. It never claims a whole-object rehash, even for a
range equal to the entire object. `WholeObjectDigestVerified` requires the
separate full reconstruction and SHA-256 check. UTF-8 decoding is optional and
fallible; bytes, including embedded/trailing NULs, remain authoritative.

No serialized continuation cursor exists in the pinned object-store or
reconstruction-DAG API. A browser cursor is therefore an explicit new local
integration, not an imported UOROBJ01 wire format. Model it as an opaque
validated value binding object version, immutable store generation, requested
range, next offset, ordered DFS continuation `(id,scale,base,childPosition)`,
visited occurrence budget, and verification level. Validate every resumed
continuation against its bound manifest; caller-supplied cursor state must not
skip integrity checks. A version conflict rejects resumption. Only a complete
whole-object pass can upgrade verification; paginated range results cannot.
Freeze a separately versioned cursor encoding before any serialization API is
accepted. Do not invent a UOROBJ01 field for it.

Retain exact token occurrence metadata as a separate `UORBND01` format:

| Offset | Encoding |
| --- | --- |
| 0 | 8-byte ASCII `UORBND01` |
| 8 | 32-byte object manifest version |
| 40 | 32-byte tokenizer identity |
| 72 | Logical byte length, u64 little-endian |
| 80 | Occurrence count, u64 little-endian |
| 88 | count entries of 28 bytes: index u64 LE, token ID u32 LE, start u64 LE, end u64 LE |

Validate exact `88+28*count` size, limits, consecutive indices, start equal to
the previous end, positive spans, end within length, and final end equal to
length. Repeated token IDs still have distinct occurrences. Ingestion rejects
prefix-space tokenizers, requires valid source UTF-8, and checks that decoded
token byte strings concatenate exactly to the original bytes before folding.
Occurrence reads pin object version, tokenizer identity, length, span length,
and exact decoded token bytes; they return only range verification. Metadata
identity is not trusted provenance or authorization by itself.

## 5. Transactions, durability, and recovery

Pure state transitions return either an unchanged store with a typed failure,
or a fully validated new immutable record inventory and commit intent. Budgets
must account for existing plus staged distinct records and bytes, not only the
new input. Identical imports add zero records and bytes. No request-visible
state exposes partially validated records.

Pinned `Store::save` writes a uniquely named sibling temporary file, syncs its
contents, closes it, and renames it over the destination. It attempts temporary
cleanup on errors. It does not guarantee concurrent-writer safety or crash
durability across directory-metadata loss. `load` bounds file metadata length,
reads exactly that length, rejects extra bytes, then performs atomic raw import.
This is not a browser storage or recovery contract.

The approved browser integration requires a new modeled durable transaction:
bind operation ID, expected store generation, object version and staged content
identity; validate all content; request an atomic compare-and-publish effect
that includes records, manifest/index, new generation and operation receipt.
A conflict publishes nothing. A successful effect publishes all four together.
An interruption before a definitive receipt enters `OutcomeUnknown` and must
not be reported as rollback. Recovery queries the durable operation receipt:
the same operation and payload returns the original result; reuse with different
payload fails binding; a known aborted operation leaves the previous generation;
an unresolved effect remains unknown. Reload validates persisted schemas and
identities and quarantines invalid state rather than exposing it as memory.

The transaction/receipt format, grant model, crash boundaries and concurrent
generation semantics must be separately modeled and tested in the accepted
PrismPM browser effect surface. They do not change UOROBJ01 object bytes or
node IDs. Commit success followed by failed DONE/notification is a committed
operation with a delivery failure/uncertain caller outcome, not failed atomic
validation. Actual reload, crash, corruption and concurrent-tab tests remain
required; state-machine proofs alone do not establish durable execution.

## 6. Errors, bounds, and deliberate source corrections

Retain closed distinctions: Missing(NodeId), SchemaMismatch, IntegrityFailure,
InvalidRange, LimitExceeded(budget), BindingMismatch, UnknownObject, Utf8 and
classified Io(kind, optional OS code). Browser additions need explicit
VersionConflict, OutcomeUnknown and operation-binding outcomes, not prose
parsed into protocol errors. Decoder truncation is SchemaMismatch; wrong hash
is IntegrityFailure. Unexpected transport EOF must distinguish between-frame
closure and a truncated frame. Allocation and checked arithmetic failures are
bounded errors, never zero IDs or silently shortened byte strings.

| Default budget | Value |
| --- | --- |
| max_object_bytes | 16,777,216 |
| max_records | 1,000,000 |
| max_record_bytes | 67,108,864 |
| max_returned_bytes | 16,777,216 |
| max_visits | 1,000,000 |
| max_occurrences | 1,000,000 |
| Wire body cap | 67,108,864, also bounded by max_record_bytes |
| Manifest roots | 128 |
| Wire count | 65,535 |

Portable limits are explicit natural/fixed-width values, not target `usize`
wrapping behavior. Construction visits equal atom count plus successive
floor-divisions by three. Bound traversal occurrences independently of distinct
record storage so a highly deduplicated DAG cannot cause unbounded expansion.

These source discrepancies are intentionally corrected to meet the already
approved atomic-import/strict-framing design. They are not parity claims or
edits to upstream. All line numbers below are at the pinned revision.

| Pinned behavior | New-port obligation and negative control |
| --- | --- |
| `lib.rs::span` counts bytes; PROTOCOL section 3 describes only atom span | Use checked byte span; reject scale 38 even though 3^38 fits u64. |
| `peer.rs:235-240` treats all UnexpectedEof as clean end | Only EOF before the first header byte is clean; truncate at each header/body boundary and require refusal. |
| `peer.rs:297` accepts DONE regardless of body | Reject nonempty DONE; retain empty DONE. |
| `peer.rs:603-606` checks membership and duplicates, but not per-position response ID | Reject a permutation of requested records in NODES even when the set is correct. |
| `store.rs:132` stage allows identical duplicates; peer rejects duplicates across staged responses | Keep raw import idempotence and reject wire duplicates in their respective APIs; do not apply one policy globally. |
| `peer.rs:739-740` stops at any locally present parent | Descend local parents to discover absent descendants; test a parent-only raw import followed by pull. |
| `peer.rs:545-568` hashes truncated assembly, imports, sends DONE, then rebuilds; assembly at 756 does not carry expected scale or check zero tail | Validate scale, closure, zero padding, length and digest on staged view before commit; inject self-certifying but wrongly scaled children and nonzero tail with correct trimmed digest, require unchanged store. |
| `peer.rs:563-565` may fail writing DONE after import | Model committed state plus delivery failure/unknown outcome; replay same operation ID without double publication. |
| `store.rs:413` save lacks concurrent-writer/directory durability guarantees | Browser compare-and-publish, receipts and reload recovery are new integration obligations, not inherited guarantees. |

The strict client remains compatible with the reference server's normal ordered
responses. Reordered or malformed peers formerly tolerated by the reference
client are intentionally rejected. Raw import duplicate acceptance remains
wire-compatible. No new peer wire version is claimed merely for stronger
validation; any new persistent cursor/receipt format receives its own version.

## 7. Published independent known answers

These are copied from pinned source fixtures, not generated by a proposed port.
Future controls must freeze the input and expected literals before implementing
the relevant hash/codec. This research did not execute the fixtures.

`tests/protocol_vectors.rs` defines 96 bytes `payload[i]=(7*i+3) mod 256`:

```text
payload = 030a11181f262d343b424950575e656c737a81888f969da4abb2b9c0c7ced5dce3eaf1f8ff060d141b222930373e454c535a61686f767d848b9299a0a7aeb5bcc3cad1d8dfe6edf4fb020910171e252c333a41484f565d646b727980878e959c
whole SHA-256 = c9f1a5f79d7bea01a54f4edb41673722f627ee2e82dda324946b63cf4b9b16af
root ID (scale 1) = 80ae70cded63205a6ec66dc8
manifest = 554f524f424a30316000000000000000c9f1a5f79d7bea01a54f4edb41673722f627ee2e82dda324946b63cf4b9b16af0180ae70cded63205a6ec66dc801
manifest version = 924abee83e5bafbeed9d712604081105fde9e7bb1f71eb6405c2bdb522e7968e
parent record = 0180ae70cded63205a6ec66dc8cce0826423d1cf8912a7863de5eef536d8dd6b28bc323ca17dddcb6d6ee0fbd7694edd07
first leaf record = 00cce0826423d1cf8912a7863d030a11181f262d343b424950575e656c737a81888f969da4abb2b9c0c7ced5dc
```

The retained `tests/fixtures/python_vectors.json` includes general BLAKE2b-96
known answers (these messages are not themselves valid node-record payloads):

| Logical message hex | Configured 12-byte BLAKE2b output |
| --- | --- |
| empty | `b8e1dda3ac0aa3820ad2990b` |
| `01` | `2257f8892d7bb998d7aaed44` |
| `0263` | `03f5d5d26dc7eeb0145d6dd9` |
| `0364c5` | `8cd7b0db096b7fdd06e8a70b` |

Its empty fold has no roots, zero distinct nodes and whole SHA-256
`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
Its one-byte zero object has one scale-zero root
`809aa4906db59da44ac69d7c` and whole SHA-256
`6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d`.
Preserve all supplied partial-atom/carry vectors as independent future inputs.
The fixture README explains that original Showcase record transport order is
root-ordered, children-before-parent, whereas raw store export sorts IDs. Compare
the intended ordering explicitly; do not mislabel this distinction as a hash
failure or regenerate expected hashes from the new implementation.

## 8. Exact candidate LexLean capabilities and dependency gaps

The pinned language-1.1 semantic IR has Nat, Int, fixed signed/unsigned integers,
Bytes, lists, records, inductives, Option and Result. Structural recursion is
checked on Nat, List, or document inductives. It has checked fixed-width
arithmetic/conversion, bitwise operators, bounded shifts, byte/list append,
length, index and slice, UTF-8 encode/decode and byte comparison. Slice takes
start and **count**, not start and end. Shifts return none for a count at least
the word width. Ordinary semantic `add` and order terms are Nat-only.

| Requirement | Candidate finding and model path |
| --- | --- |
| SHA-256 and BLAKE2b-96 | No hash primitive in the closed SemanticPrimitive enum. Implement the rounds/padding in authoritative LexLean, or obtain a separately reviewed generic exact-hash compiler facility with equivalent semantics and parameterization. A host hash call cannot silently become the application authority. |
| Modulo word addition and rotate | CheckedAdd exists; wrapping-add/rotate primitives do not. Model fixed bit-vector addition and rotation explicitly, or obtain generic reviewed primitives. Checked overflow rejection cannot replace hash modular addition. |
| Integer/byte codecs | No dynamic byte singleton/list-to-Bytes, endian-word load/store, or fixed-integer-to-Nat primitive. Lists of modeled bytes/bits can express exact codecs now; native Bytes conversion needs an explicit implementation (even a complete finite singleton mapping), not an assumed API. |
| Imported proof reduction | Some backend primitives, including index, slice, quotient/remainder, UTF-8 and compareBytes, are noinline definitions without expose. This already affected arithmetic controls in this SDK. Start with small imported proof probes; use structural LexLean definitions or request a generic exposure fix, never edit generated Lean. |
| Byte-efficient hashes | Structural bit/word/list models are expressible, but production throughput and proof feasibility have not been measured. Require decomposed round/state proofs and independent literal vectors; do not replace computation with fixture lookup tables. |
| Persistent storage and networking | These are not pure semantic IR primitives. Accepted PrismPM effects must supply bounded record fetch, ordered stream I/O and atomic durable compare-and-publish/receipt lookup. Candidate production browser acceptance remains unresolved. |
| Production extraction and resources | A verified mathematical list implementation alone does not establish bounded runtime allocation, browser storage durability, Wasm behavior, or production binding. Preserve those as required dependencies. |

No present primitive gap proves the mathematical model impossible. The source
can model bytes as eight bits, words as fixed records, hashes as bounded rounds,
and DAG/state transitions with explicit fuel and limits. The gap is between
that exact model and an accepted efficient generated runtime/effect surface.
Do not introduce raw Lean, Rust, JavaScript, FFI, unchecked declarations or
hash-oracle axioms to claim that gap closed.

## 9. Concrete modeling phases and acceptance gates

1. **Bytes, bounds and codecs.** Register the memory IDs/scenarios and immutable
   control-source identities first. Define byte/word identities, exact-length
   wrappers, endian codecs, checked spans, frames and tagged errors. Red controls:
   malformed widths, 37/38 scale boundary, overflow, every truncation point,
   extra bytes, nonempty DONE and inconsistent body counts.
2. **Hash semantics.** Define modular addition/rotation, BLAKE2b configuration,
   twelve compression rounds, SHA-256 blocks/padding/length and exact output
   codecs. Freeze the published vectors above. Plant output-length-as-truncation,
   counter-128, endian and rotation defects and require rejection. Prove bounded
   round transitions; never claim hash injectivity.
3. **Records, fold and manifest.** Add self-certifying record decode, ternary
   carry, canonical frontier, manifest codec/version and exact padding. Controls
   include empty/1/31/32/33/95/96/97 bytes, repeated roots, multiple carries,
   ordered children, nonzero final padding and wrong root scales. Match all
   published roots/digests and distinguish export order from transport order.
4. **Immutable store and reads.** Model staged raw import/idempotence, combined
   budgets, missing closure, ordered range traversal, occurrence counts and
   separate verification result types. Inject missing descendants, same ID at
   conflicting scales, repeated-DAG expansion, corrupt backend data and out-of-
   range requests. Include UORBND01 and exact tokenizer binding once the full
   tokenizer model exists; do not substitute byte-only tokenization.
5. **Peer transaction machine.** Model selected-object scope, request/response
   correlation, pipelining, exact pins, staged full verification, per-object
   series commits and explicit postcommit delivery outcomes. Encode every source
   correction in section 6 as a distinct control; round-trip interoperability
   uses unchanged valid UOROBJ01 bytes, not parity with permissive failures.
6. **Durable browser memory and cursors.** Specify separate versioned local
   formats, PrismPM resources/grants, compare-and-publish, receipts, conflict,
   reload and uncertain recovery. Prove transition invariants and then run
   generated browser reload/crash/concurrent-tab/corruption tests on the accepted
   SDK. No manual browser runtime supplies these semantics.
7. **Conversation and delivery integration.** Bind memory versions/provenance
   to modeled conversation operations, exercise exact returned bytes and limits,
   retain the distinct range/whole claims, and deploy only the accepted generated
   application closure. Storage fixture tests establish no learning capability.

Each phase uses atomic conventional commits after independent review and fresh
formal checks. Completion of an early phase does not satisfy durable-memory or
full UOR-R4 acceptance. Keep strict validator corrections visible in PrismPM
authority/ledger records, separate from imported upstream implementation facts.

## 10. Source identity ledger

All object-store paths below are relative to `crates/uor-r4-object-store/` at
UOR-R4 `4611538bb4843947363ccb9cce5f388e32c46883`.

| Path | SHA-256 |
| --- | --- |
| PROTOCOL.md | `75b4384f0e1c178719820ab05a58fe87d4f84a59303f7f949ebd1029a37a5821` |
| src/lib.rs | `e82939bd3fe505a6a972d3107958aaa02157552683e69d67a6fe4cb351242114` |
| src/hash.rs | `f82cc1b4d4ef9ed2f0f0b933a9c8009611fe774de3666299a5f977fc0ed94b5f` |
| src/record.rs | `a0aaf33fc1fe24b38a492316cc478e13ac931bf74d0e087837ec754b75a2722d` |
| src/manifest.rs | `19f69c495e47895de6f7a5089fb2c9492c03d18997fd80e1ffa1edc44abe1290` |
| src/store.rs | `d7c384681a1f8e188c1a193b2bdd3481438f7d8f309f69b1e906b421bed04b86` |
| src/peer.rs | `58c54bce407b3d3d66af28d3ee0169772726beb7eb20925898bd4ccf6fba8045` |
| src/bindings.rs | `50a5adc5bf715d61ea3d33ab3dd4309b6b6237415dd7d6cf39986a0e34f04573` |
| tests/protocol_vectors.rs | `188fd4fac9f56fc4cb33b039d57f78eed690a7db8b33462d24668636017e95a1` |
| tests/fixtures/python_vectors.json | `0930044db928f0389dfa35e603812e4c765e0c8c7ced3182f6616adf31299031` |
| tests/fixtures/README.md | `aa33f6acdecec8b49b223dc06a266991b3c1bac18ad25d05578e7863affa2000` |

LexLean paths at `01088ef00d9d0d0227775e0346a852c2d19cb67b`:

| Path | SHA-256 |
| --- | --- |
| crates/lexlean/src/ir/semantic.rs | `17d07ae09ae3bb8ba33e77c8d46b8776db2a913900be4dcf22753dc433127de3` |
| crates/lexlean/src/backend/semantic.rs | `6a7bab1304b08658879a7879ab7826631651e2d743344bca8c6172a016ea65d2` |

The secondary `crates/uor-r4-recon-dag/src/lib.rs` was inspected to confirm
range-versus-whole verification distinctions and absence of a cursor API. Its
older text manifest and backend-specific limits are not adopted as UOROBJ01.
Its SHA-256 is
`c6638189eb00a7f693139a68db83566a1f7d0e2d1b561d148bcbd2f7c2bb1620`.
