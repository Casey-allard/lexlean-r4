# Complete byte-BPE semantic kernel

R4-16 models typed tokenizer admission, segmentation, rank BPE and decoding.
The complete 115-declaration implementation is present; final current-SDK
verification, semantic defect checks and independent review are in progress.
No accepted JSON parser or tokenizer identity is claimed by this component. Raw JSON semantics and BLAKE3 identity
remain separately registered dependencies in `TOKENIZER.md`.

## Authority and input boundary

Authority is `crates/uor-r4-tokenizer/src/lib.rs` at UOR revision
`4611538bb4843947363ccb9cce5f388e32c46883`, SHA-256
`52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35`.
The implementation uses accepted `R4.TokenizerText` scalar admission, Unicode
predicates, UTF-8, lossy decoding and byte-alphabet operations. It does not call
host strings, regular expressions, Unicode libraries or a tokenizer oracle.

`KernelInput` is a typed extraction result, not raw JSON. Text is
a list of Unicode scalar naturals. Vocabulary/added entries contain text and
natural IDs. IDs must fit UInt32; model IDs must form a dense prefix. Model
vocabulary entry order is normalized by scalar lexicographic order, matching
UTF-8 String order for valid scalars, before checking ID occupancy. Duplicate
decoded content keys are rejected as a typed-input shape error: a parsed JSON
Value object already has unique keys after its last-wins collapse, so this
error does not change source-compatible JSON behavior. JSON parsing must
perform that collapse itself, including validation of overwritten values.

Optional model-type text represents the source's string-valued check; absent
or non-string type is extracted as None. Present text must equal `BPE`.
Missing/wrong-type model/vocabulary/merges fields remain JSON-extraction errors
rather than invented kernel data. Empty model content is legal; empty added
content is rejected. Identical repeated added ID/content is idempotent;
either alias direction is rejected. The final slot count is max ID plus one,
with holes explicit and distinct from nonempty token contents.

`KernelMerge` has text-key, two-text, and invalid-shape variants. A text key
must contain exactly one ASCII space; empty components are legal. Two texts
join with one literal ASCII space even when either component contains spaces.
Ranks retain original list indices; duplicate joined keys retain first rank.
No pair-record lookup may erase source key collisions.

`KernelStep` distinguishes Digits, ByteLevel and unsupported kinds. Their
optional Boolean values reproduce source defaults (Digits false, prefix true).
The optional step list represents extracted direct/Sequence pre-tokenizer
configuration. All preceding Digits steps update the policy; ByteLevel fixes
it; any subsequent step is rejected. Missing/malformed extraction cannot be
silently turned into a valid empty policy. JSON-specific missing-vs-wrong-type
diagnostics belong to the extraction layer; both may refuse before kernel use.

Public encode/decode wrappers admit the typed configuration and scalar domains
before using it. An internal constructed configuration is a convenience, not
permission to trust caller-supplied lookup tables or identity values. Strict
consumer admission (no prefix insertion, full alphabet coverage, exact ID and
byte budget) is separate from permissive source encoding/decoding.

## Algorithm and termination

Added matching scans valid scalar boundaries left to right; at the earliest
match it chooses greatest UTF-8 byte length, then lexical content. This agrees
with the source's earliest byte-position search on valid UTF-8. Empty content
is rejected so every special match advances. A matched added ID bypasses all
other segmentation and BPE.

Digits splits numeric/nonnumeric runs using the accepted Unicode predicate;
individual mode splits each numeric scalar. Prefix insertion applies to each
nonempty resulting split before ByteLevel, and only literal ASCII space
suppresses insertion. ByteLevel checks lowercase contractions only at its
current cursor, then optional ASCII-space attachment to alpha/numeric/punctuation
runs, then whitespace with one trailing whitespace retained when followed by
nonspace. It never restarts contraction detection inside a punctuation run.

BPE begins with the scalar alphabet encoding of each actual UTF-8 byte.
Select the earliest adjacent pair among minimum ranks; merge every disjoint
occurrence of that selected ordered pair in one pass. Each successful pass
removes at least one symbol. The structurally recursive driver starts with the initial symbol count. If
a ranked pair remains at zero fuel it returns `invariantFailure`, never an
unfinished success. No arbitrary
iteration cutoff, one-occurrence approximation or fixture-shaped algorithm
is accepted. Missing final symbols fall back by individual alphabet scalar,
skipping missing entries as the source does.

Raw decoding concatenates exact bytes: added IDs use literal UTF-8, normal
IDs reverse the alphabet where possible and otherwise UTF-8 encode the scalar.
Permissive out-of-range IDs and empty holes emit nothing. Lossy text conversion
runs only after concatenation. `token_byte_len` returns no length for empty,
hole or unknown IDs. Strict decoding checks ID validity and a cumulative byte
budget before producing output; it cannot reuse permissive skipping as success.
The compatibility encode-lossy count remains zero even when missing alphabet
symbols were skipped; it is not a losslessness witness.

## Initial independent controls and RED

`tests/lexlean/TokenizerKernelControls.lex.tex` initially contains 58 immutable
source-derived theorem statements, SHA-256
`25dfac59aac61b0a2ef12b5309ed343e04f525347a6832db72d7bc83236ea74e`
before the reviewed identifier-only repair described below.
The registered scenario is `features/suites/r4-tokenizer-kernel.feature`.
Controls use literal expected IDs, scalar pieces, raw bytes, summaries and
failures, including complete fixture encode/decode, rank priority, all-occurrence
merging, joined-key collision, policy defaults, aliases and source text cases.
Additional boundaries will be frozen before their corresponding definitions.
No candidate kernel or host reference was executed to obtain expectations.

Before kernel definitions existed, the old development SDK in the isolated
project `/tmp/tokenizer-kernel-dev` rejected the fixture with `LLR3005`:
`TokenizerKernelControls` imports absent `R4.TokenizerKernel`. This actual
missing-module RED followed a successful isolated lock. Log:
`/tmp/tokenizer-kernel-red.json` in container `lexlean-r4-dev`.
The host preparation is `/tmp/lexlean-r4-tokenizer-kernel-dev-20261021`.

Current acceptance still requires restored verification of every fixture,
semantic planted defects, independent review and the mandatory Rust receipt
gate under the parent's accepted SDK/configuration. This component does not
establish tokenizer identity, production deployment or model capability.

## Public surfaces and representation

| Surface | Behavior |
|---|---|
| `kernelAdmit`, `kernelDescribe` | Validate typed extraction; expose admitted records and summary |
| `kernelEncode` | Added matching, Digits, ByteLevel, exact rank BPE and fallback |
| `kernelEncodeLossy` | Same actual IDs with source-compatible replacement count zero |
| `kernelTokenId` | Exact content lookup, including a legal empty model token |
| `kernelDecodeBytes`, `kernelDecodeLossy` | Raw bytes, then optional concatenation-wide lossy UTF-8 |
| `kernelTokenByteLen`, `kernelTokenByteLengths` | Strict optional individual length; permissive slot-indexed table |
| `kernelDecodeStrict` | Reject empty/unknown IDs and cumulative byte-budget excess |
| `kernelStrictConsumer` | No-prefix and all-256-alphabet coverage admission |
| `kernelBpeSymbols`, `kernelBpeBounded` | Complete admitted BPE; explicitly bounded diagnostic surface |

The strict-consumer predicate establishes the two named configuration
conditions only. Durable-memory acceptance must additionally check actual
encode/decode byte equality and model-vocabulary binding at composition time.
For example, marking a byte-alphabet model token as an added token changes
its raw decode to literal UTF-8; alphabet coverage alone does not prove a
round trip. Raw JSON, listing serialization, BLAKE3 identities and consumer
composition remain independent obligations.

Configuration stores a sparse list of admitted assignments plus exact slot
count, preserving source holes without allocating an artificial million-slot
list. Lookup distinguishes an empty token, an unassigned hole and an unknown
ID where the public API distinguishes them. Model entries are sorted by
scalar lexicographic order before source-domain checks. Scalar order agrees
with UTF-8 byte order on admitted Unicode scalars. Rich error constructors
are explicit port diagnostics; the pinned Rust constructor returns only
`None` for these failures. Arithmetic uses unbounded naturals internally with
explicit source UInt32 ID/rank and 2^20 vocabulary admission. Each supplied
scalar text must have a UTF-8 byte length representable by the documented
64-bit source target; larger abstract inputs return sourceTextTooLarge.
This is an explicit source-representation boundary, not a UInt32 text cap.

All list scans recurse on an immediate input tail. Pretoken and added-token
scans advance one scalar per structural step and use a remaining-skip count
for the already-emitted chunk. They do not call a scanner recursively on a
computed suffix. Empty added contents are rejected and every segmentation
chunk is nonempty. The only iteration budget is the mathematically sufficient
BPE measure: a selected adjacent pair exists in the current list, its
nonoverlapping pass merges at least one occurrence, and the symbol count
strictly decreases. A list initially containing n symbols therefore needs
at most n-1 successful passes; the public driver gives n. This is the
termination/adequacy argument, while the checked finite controls separately
exercise connected complete computations, exhausted diagnostic budgets,
empty-symbol pairs and multiround rank changes. No universal source-parity
theorem is claimed from finite controls.

## Frozen controls and identifier repair

The first actual Lean build exposed that the SDK emits field names verbatim:
`prefix` is a Lean notation keyword. The parent and independent reviewer
approved the narrow schema alpha-renaming to `addPrefixSpace` in the
Summary, Policy and Config records. An exact AST comparison of all 58
original controls found only seven field-label changes in summary literals
(declarations 34 through 40); all values, theorem names, calls, proofs and
axiom lists were unchanged. The original bytes and exact diff are retained
in `/tmp/lexlean-r4-tokenizer-kernel-evidence-20261021`. No SDK or generated
Lean was modified.

| Fixture | Count | SHA-256 |
|---|---:|---|
| TokenizerKernelControls | 58 | dfa8e53738a4d2c12a57b6800185e0136e009a5de53ba4342f8a0ce13af052c9 |
| TokenizerKernelBoundaryControls | 35 | b70eb35d74ca6f6eeacc590d21f8803d1b8edae07fad30849be1e81e7d511da4 |
| TokenizerKernelSurfaceControls | 15 | c46d8dac6721578f6281de87b13431a600eb2be9fc322dc35ad00502df130bd3 |
| TokenizerKernelProgressControls | 1 | 8286c01b3c7092b8940f2163a2a07e29dd3ecafa0326a4912c634cb56ccb89a9 |
| TokenizerKernelLengthControls | 13 | dc74c7be04391e9264356ec3e053c9440580ffceab4ec92f097d158d6254fde5 |
| TokenizerKernelUnicodeControls | 5 | 4e84bd416d6daa435fe46c959000f217c92efa7e24b3edf1f84c6ad06388a91a |

The surface fixture includes the complete literal 256-entry byte alphabet.
Its input uses balanced concatenation of ordered literal chunks to respect
the semantic JSON parser's nesting limit; it does not replace admission,
lookup or byte encoding with stored answers. The full configuration runs
through actual admission, sorting, lookup and coverage. Source-derived
expected IDs for non-ASCII text remain literal.

The old development SDK verified the original 58 and boundary 35 controls
under the empty-axiom policy. Its fixed recursion depth rejected precisely
three full-alphabet surface proofs; the other 12 elaborated. This is retained
as a compiler-cost diagnostic. The unchanged 15 surface statements passed against the parent's official
current SDK on the 108-declaration source, receipt
6b660147edd4fb326a1a2a31d6d6568e30431d84733133015c716e452d0df549,
with its genuine fresh isolated lock and unchanged fixed limits. The final
115-declaration length and Unicode corrections requires the parent's fresh complete
current-SDK Rust gate; that integration result is separate.

## Source length semantics

The pinned source intentionally exposes different length domains. Individual
`token_byte_len` uses checked 64-bit usize accumulation and returns None on
overflow. The bulk `token_byte_lengths` uses an `as u32` conversion, so each
raw decoded byte length is modulo 2^32. `kernelTokenLength` therefore applies
`kernelCheckedRawLength`; `kernelLengthTable` folds each actual decoded byte
through a wrapping UInt32 counter. The counter starts at zero and advances
once per byte, with MAX→0. It does not truncate UTF-8 or count display scalars.

The additional 13 `TokenizerKernelLengthControls` were frozen before these
definitions and rejected with a missing-function diagnostic. They check
UInt64 MAX/+1, UInt32 MAX/+1 accumulation, a connected three-byte carry across
MAX, and invalid accumulator/byte domains. Existing actual API table and
individual-token controls remain unchanged. These small symbolic checks
validate the arithmetic boundary; they do not claim execution of giant
materialized text fixtures.

Current source SHA-256: `ffc51a6e011534602cebaf723cdb0e60fa9a8693533ae39f92edbd77414e7eee`.
Length-controls SHA-256: `dc74c7be04391e9264356ec3e053c9440580ffceab4ec92f097d158d6254fde5`.
Progress-controls SHA-256: `8286c01b3c7092b8940f2163a2a07e29dd3ecafa0326a4912c634cb56ccb89a9`.

The separate progress control was added after a well-formed one-occurrence
mutation revealed that repeated full BPE iterations eventually repair the
simple repeated-aa examples. The distinguishing case starts with aaaa and
ranks (aa,a)=0 before (a,a)=1. Correct all-occurrence processing produces
[aa,aa]; merging only the first occurrence lets the new lower-rank pair
intervene, producing [aaa,a]. The new theorem actually rejected that mutation
before restoration. All earlier frozen expectations remain unchanged.

## Unicode overlap regression

Independent review found that using the initial category classifier for run
membership incorrectly stopped a numeric run before a character that is both
Numeric and Alphabetic. U+2160 ROMAN NUMERAL ONE is such a character in the
pinned Unicode table. The source deliberately chooses the Alphabetic branch
first at a run's start, but its Numeric loop tests Numeric independently.
Thus 1Ⅰ is one numeric pretoken, while Ⅰ1 is two pretokens. A leading ASCII
space attaches to the complete numeric run.

The separate five-theorem TokenizerKernelUnicodeControls fixture was frozen
before correction. The old source failed the three numeric-overlap cases;
the repaired source passed all five under none axioms, receipt
8b319e9a7fdc7c816ed97a414186964a9d38bfc238344db0a9e61eede0188e20.
The fix adds independent run-membership predicates while preserving the
initial branch order. Reverse-order, alphabetic-run and leading-whitespace
controls protect that distinction. Fixture SHA-256:
4e84bd416d6daa435fe46c959000f217c92efa7e24b3edf1f84c6ad06388a91a.
The earlier 122 theorem statements are unchanged; the full harness now
requires all 127 statements across six files.

## Final semantic defects and restoration

The following well-formed source mutations were applied only in the isolated
old-SDK stage to the final 115-declaration source. Each passed source admission
and failed the unchanged semantic theorem statements with LLV7002; no parser,
missing-name or axiom-policy failure is counted as detection.

| Mutation | Rejected controls |
|---|---|
| Reverse minimum-rank preference | rankAndAdded, duplicateFirstRank |
| Merge only the first selected occurrence per iteration | allOccurrencesBeforeNewLowerRank |
| Treat an initial tab as suppressing prefix-space insertion | prefixBeforeTab |
| Drop ordinary reverse-alphabet bytes during raw decode | 10 raw-byte, lossy, length and strict-budget controls |

The original source was restored byte-identically and all five old-SDK-sized
fixture modules were freshly verified under none axioms: 112 statements in
58 main, 35 boundary, 1 progress, 13 length and 5 Unicode controls. Restored
receipts are:

| Module | Restored attestation |
|---|---|
| TokenizerKernelControls | 5efa344fb383e203b34a3ea2973e60debddf4fb21237fc4be42e9006e3b60f2b |
| TokenizerKernelBoundaryControls | 4b7331941287d3ef96f1ebbc8d751d4a6b6ca363b800c41296d9bf51309fb915 |
| TokenizerKernelProgressControls | 4cd71687f9bf645d15952e5bd8399a4b3016e8b206cefe9a0486a11d468232f3 |
| TokenizerKernelLengthControls | dec9a75ec64add7ea9be6592b4a8f74d7589a76c2aaf0edbba8ae39bf67a146c |
| TokenizerKernelUnicodeControls | 8b319e9a7fdc7c816ed97a414186964a9d38bfc238344db0a9e61eede0188e20 |

Exact logs and mutation identities are retained under
`/tmp/lexlean-r4-tokenizer-kernel-evidence-20261021`, with
`final115-mutant-*.json`, `restored115-*.json` and `mutants.json`.
The six-fixture Rust gate freezes all 127 theorem statements and delegates to
the shared current-source/configuration/lock/module/axiom receipt verifier.
It contains no tokenizer implementation. The final current-SDK six-fixture
Rust result and shared default-root integration are parent-owned acceptance
steps; this document does not replace them with the older partial-size run.

## Integrated construction-SDK verification

The actual mandatory `cargo test --offline --locked -p repo-conformance
--test tokenizer_kernel` passed in 124.48 seconds: one test, zero failures,
ignored or filtered tests. It checked the final 115-declaration source
`ffc51a6e011534602cebaf723cdb0e60fa9a8693533ae39f92edbd77414e7eee`
and all six frozen fixture modules above, including the complete byte alphabet.
Each invocation checked current input identities, an exact four-module closure,
empty observed axioms under the none policy, and four independent kernel replays.
Across the six invocations there are 1,117 audit rows and 24 kernel replays;
source declarations recur across these receipts, so these are not unique counts.

| Fixture | Controls | Attestation |
|---|---:|---|
| TokenizerKernelControls | 58 | `0df62d7efd4294f7efb3c37abfe6149c070b3d048a8599cf834b85be311efadd` |
| TokenizerKernelLengthControls | 13 | `8909d0e5856f2aa31df6462456fd1e4de7bee4c95b46e1d2dfa3cdb8e6fe9cf8` |
| TokenizerKernelProgressControls | 1 | `9ab54f609ffea0a0f99c1edf749dc4f9f2b18fef12df8cefc8f459947279976e` |
| TokenizerKernelUnicodeControls | 5 | `b9c00a5e603ebe4acd5d39b6c092c89830d7d9afb97338afd0ef24f21579490d` |
| TokenizerKernelBoundaryControls | 35 | `f36a2ca29ca3b345d24d16680d302e982146fb94c91baf2dd478e1ebe381329d` |
| TokenizerKernelSurfaceControls | 15 | `fc368fffca3fbc34c88d37146b471ec4d9ffbc08b777291d41a0ec7c5b3b37ed` |

Configuration SHA-256: `61608e2debc1e76b89258bd2c74d8188f26d7d2454a8d6f40894f7b161a6ae2e`.
Lock SHA-256: `1b35d04ceb567c6fbe03885ff1814be73a60e0863b47056843bea13663365c6e`.

The isolated fourteen-component snapshot passed model generation, formatting,
lock checking, all-target/all-feature Clippy with warnings denied, and default
source checking of 67 modules. The combined 6,636-control proof replay is not
yet run; the last full combined replay remains twelve components. Full
`just vv` passed model consistency and ownership, then failed at template-check
line 17 because `prismpm.lock` is missing. The SDK is construction-only.

An earlier integration attempt correctly failed current-configuration identity:
its array spacing did not match the compiler canonical representation. The
final source uses comma-space arrays and passes the unchanged raw-input hash
validator. This was an input-identity failure, not a tokenizer counterexample.
Raw JSON, BLAKE3 tokenizer identity, production acceptance and deployment
remain separate requirements.
