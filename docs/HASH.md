# Pure content hashes

`R4.Hash` implements SHA-256 and the configured twelve-byte BLAKE2b digest
used by UOR object nodes. Every word operation, message schedule, compression
round, padding byte and output byte is defined in LexLean. Rust only audits
immutable controls and compiler evidence; it does not compute application
hashes. This component does not establish object-store persistence, tokenizer
identity, runtime performance, or production SDK acceptance.

## Construction and interfaces

`sha256(input : List Nat)` returns `Result HashError Digest256`.
`sha256Bounded(maxBytes, input)` adds an explicit caller byte budget.
`Digest256` wraps eight `R4.Words.Word32` values in `ShaState`; `encodeDigest256`
returns their 32 bytes in big-endian order. The admitted SHA input length is
at most 2^61−1 bytes, so the byte-aligned bit length is strictly below 2^64.
A list is consumed structurally; the implementation has no fixture-dependent
message bound or truncated compression loop.

SHA-256 appends byte 0x80, zeros to length 56 modulo 64, then the exact 64-bit
big-endian bit length. Each 64-byte block supplies sixteen big-endian words.
The sliding sixteen-word schedule computes all 64 required words. All 64
rounds use the standard constants, choose/majority, big/small sigma functions,
and wrapping 32-bit addition. The completed working state is added to the
incoming chaining state before processing the next block. The public result
is the final state after every padded block. Small explicit fixed records
avoid unchecked list indexing and invalid-index defaults.

`nodeId(scale, payload)` returns `Result HashError NodeId12`.
Scale zero requires exactly 32 payload bytes. Scales 1 through 37 require
exactly 36 bytes, representing three ordered twelve-byte child identifiers.
It hashes the exact message `scale || payload`, including the scale byte.
`NodeId12` has twelve fixed `Byte8` fields; `encodeNodeId` returns those fields
in order. The hash component checks shape and byte domains; validation of
referenced children and canonical object structure belongs to the object model.

`blake2b96SingleBlock(input)` implements the complete unkeyed single-block
construction for 0 through 128 bytes. It initializes the BLAKE2b parameter
word with `0x0101000c`: digest length 12, key length 0, fanout 1, depth 1.
The logical byte count is XORed into counter word v12, v14 is complemented
for the final block, and the message is zero-filled to 128 bytes only for
little-endian word loading. The count excludes those fill bytes. Twelve
rounds use the ten standard sigma rows cyclically, eight column/diagonal
mixes per round, and rotations 32, 24, 16, and 63. Final chaining XORs yield
eight bytes from h0 and four from h1, both little-endian. Configuring a
64-byte digest and taking its first twelve bytes gives a different hash.
Normal node messages are exactly 33 or 37 bytes, within this complete bound.

## Typed admission and helper errors

All public numeric byte inputs are checked against 0..255. Natural values
are not silently narrowed to bytes. Admission order is intentional:

| API | Ordered failures |
| --- | --- |
| `sha256` | `messageTooLong(count)`; first `invalidByte(index,value)` |
| `sha256Bounded` | `messageTooLong(count)`; `budgetExceeded(max,count)`; first invalid byte |
| `nodeId` | `invalidScale(scale)` above 37; `invalidLength(expected,actual)`; first invalid byte |
| `blake2b96SingleBlock` | `blockTooLong(128,count)`; first invalid byte |

Byte indices are zero based in the logical hash message. For `nodeId`, the
scale is at index zero, so payload byte zero is reported at index one.
Internal block loaders preserve `invalidLength` for wrong shapes and
`wordFailure(WordError)` for failed word codecs. Complete public admission,
padding, and fixed block construction establish those helper preconditions;
these error constructors are propagated rather than converted to a zero
word or digest. The length predicate controls test the exact SHA maximum
and its successor without allocating an impossibly large list. They do not
claim an enormous message was evaluated.

The structural Boolean representation is exact but expensive to normalize.
Proof times measure compiler reduction and kernel replay on finite constants,
not serving throughput. Arbitrary admitted input semantics do not imply that
all such messages fit a particular verifier's time or memory policy.

## Authorities and immutable controls

The node construction is imported from upstream
`crates/uor-r4-object-store/src/hash.rs` at commit
`4611538bb4843947363ccb9cce5f388e32c46883`, file SHA-256
`f82cc1b4d4ef9ed2f0f0b933a9c8009611fe774de3666299a5f977fc0ed94b5f`.
Protocol inputs and independent literals come from that revision's
`tests/protocol_vectors.rs` (SHA-256
`188fd4fac9f56fc4cb33b039d57f78eed690a7db8b33462d24668636017e95a1`)
and retained `tests/fixtures/python_vectors.json` (SHA-256
`0930044db928f0389dfa35e603812e4c765e0c8c7ced3182f6616adf31299031`).
Those reference fixtures were read as data; no host reference hash algorithm
was run to produce expected values.

SHA standard constants and mechanics follow FIPS 180-4 and RFC 6234.
NIST's byte-oriented SHA test archive has SHA-256
`929ef80b7b3418aca026643f6f248815913b60e01741a44bba9e118067f4c9b8`.
Its SHA256ShortMsg.rsp and SHA256LongMsg.rsp files have respective SHA-256
`75e1cb83994638481808e225b9eb0c1ebd0c232d952ac42b61abce6363be283c`
and `6fac36f37360bcf74ffcf4465c18e30d6d5a04cc90885b901fc3130c16060974`.
The first `abc` compression-round state is the NIST worked example. The
independent full digest expectations remain literal theorem right-hand sides.

`tests/fixtures/hash/HashControls.lex.tex` preserves the original thirty
controls byte for byte, SHA-256
`0603c79620ff19e4a30239a7b180b214af5941d925f60161479a0049b8acd60e`.
The active files `tests/lexlean/HashCase01.lex.tex` through
`HashCase30.lex.tex` each contain one unchanged original theorem. The frozen
`tests/fixtures/hash/inventory.json` maps every theorem, path and source hash;
its SHA-256 is
`15eb88f7ac49aa3910ab52ed6b322f6907a8edca21b1c97fc9bd248b6da44dfd`.

| Cases | Complete original obligations |
| --- | --- |
| 01 | Independent NIST first SHA `abc` round state |
| 02–08 | SHA empty, `abc`, NIST 55/56/63/64-byte boundaries and 163-byte multiblock input |
| 09–10 | Published complete object and manifest SHA digests |
| 11–15 | Zero leaf, three published leaves, and their published parent ID |
| 16–19 | Configured BLAKE2b-96 independent messages of lengths 0, 1, 2, 3 |
| 20–24 | SHA length maximum/successor, invalid byte, budget precedence and exact budget |
| 25–30 | Invalid node scale/leaf or parent length, and oversized BLAKE block |

`crates/conformance/tests/hash.rs::complete_content_hashes_r4_11` freezes
both original and inventory bytes, checks unique names and exact thirty-case
coverage, compares every full JSON declaration (including statement, input,
proof and axiom policy), and rejects extra, missing or duplicate case files.
Only then does it run all thirty controls sequentially through the common
verifier. Each invocation must produce successful current-source evidence,
empty observed/allowed axiom sets, and independent kernel replays for the
control module, `R4.Hash`, and `R4.Words`. File and module partitioning changes
compilation workload, not any mathematical statement. The original aggregate
fixture is outside active source roots to avoid also compiling its expensive
thirty-theorem duplicate as one job.

## Development verification and limits

The initial missing-module and missing-definition controls failed before
implementation. The prior SDK candidate at LexLean `01088ef...` checked the
complete source but could not normalize full digest controls within its
fixed recursion settings. Three experimental round-certificate probes did
not resolve that limitation and are excluded from the accepted 181-declaration
algorithm. No public digest statement, input, proof or axiom allowance was
changed to obtain acceptance.

The official construction artifact from PrismPM run 39492842177 vendors
LexLean `0865df4e540ca643b090a29e9df6d319d6906da6`, Lean 4.32.1.
Its immutable OCI manifest is
`716370f20b6b01948700a16380668f5328b854e3764188916fc8802ef27ca7c6`;
LexLean executable SHA-256 is
`107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5`.
Its version banner reports compatibility-language 1.0 semantic identity
`95deb33a8d416d7bf60f02a36e251a71c3ee6f046b7e475d7bae2fc5ddc3767d`.
The actual language 1.1 project lock records semantic identity
`4658bc8564c14b99f771a758276c578e8acd5ef5f24a3c43e69e4e6eb7df370e`.
The artifact's own fixed recursion/heartbeat limits were used unchanged.
The individual-control runs used fixed 4 GiB RAM, two CPUs, no network, and
the existing 300-second child timeout. The earlier aggregate fixture exceeded
that timeout; its container started at 2 GiB and was raised to 4 GiB during
the attempt, so that failure is not a controlled performance comparison.
Each of its thirty unchanged individually compiled controls passed, with
empty axiom audits and all module replays passing. The longest individual
job was 88 seconds; the sum of observed job times was 760 seconds.

The mandatory Rust target was then run against the integrated twelve-component
snapshot with the regenerated lock. It independently audited the frozen full
partition and verified every case through the unchanged shared evidence gate:
`cargo test --offline --locked -p repo-conformance --test hash` passed its
single mandatory test in 1010.80 seconds, with zero failed, ignored or filtered
tests. This is a separate integrated run, not the sum of the diagnostic jobs.

The partition gate was also attacked in a separate staged copy. An extra
`HashCase31` was rejected as an unregistered control; removing `HashCase30`
failed with a missing-file error; appending one byte to that frozen fixture
failed its expected source digest. These three failures occurred during the
pre-verification inventory audit. The original case bytes were restored, and
the integration snapshot remained unchanged throughout these plants.

Representative defects were planted only in isolated source copies. Every
mutant first passed source checking, then failed the intended full digest
control with `LLV7002` and an actual reflexivity mismatch:

| Defect | Required rejection |
| --- | --- |
| SHA padding marker 0x80 changed to zero | `sha256Abc` |
| BLAKE logical counter changed to padded length 128 | `protocolParent` |
| Digest configuration changed consistently from 12 to 64 bytes | `protocolParent` |
| BLAKE input word decoding changed from little to big endian | `protocolParent` |

No mutant was accepted; these failures were not syntax, timeout, recursion,
or axiom-policy failures. The exact original algorithm SHA-256
`d2783f37e0ae537c13802f4454411cf131304ac5bcb41f43bf47fd0903416c02`
was restored and SHA `abc` and the published parent passed again. These are
finite formal conformance results on a development construction artifact.
Production `prismpm.lock`/`template.lock` binding, full `just vv`, deployed
Pages, runtime performance and complete durable-memory behavior remain
separate acceptance obligations. Tokenizer BLAKE3 identity is also a separate
algorithm and must not use either digest here as a substitute.


## Isolated HashWire verified finite baseline

[HashWire](HASH-WIRE.md) preserves an isolated fixture project whose exact 78-root/636-control positive baseline has passed independent artifact review and durable sealing. Status is BASELINE_VERIFIED_MUTATION_INCOMPLETE: two per-mutant semantic negatives are partial observations; the third mutant hit a whole-module LLS8002 child timeout, and full restoration was still running at the recorded checkpoint. All seven mutants/ten witnesses, complete restoration and final independent review remain required. The candidate Hash facade differs from the canonical module and is not adopted by this packaging. Original Hash regression, separate Nat controls/eleven mutants/restoration, Object consumers and modeled integration remain required; checked-u64 production lowering is a separate gate.
