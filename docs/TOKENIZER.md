# Exact tokenizer implementation plan

Status: implementation plan and complete Unicode inventory approved by independent
architecture review, 2026-10-21. The Text component is implemented and its
mandatory integration test passes; complete tokenization and verified tokenizer
identity remain pending. This plan implements
the approved [tokenizer contract](specs/tokenizer-design.md) and full-scope
design; it does not replace the tokenizer with an ASCII or presegmented demo.
Parent registered R4-14 before its scenario and frozen controls. The Text source
followed an observed missing-module RED; full tokenizer claims remain pending.

## Authority and discovered dependencies

The semantic authority is UOR-R4 revision
`4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-tokenizer/src/lib.rs`, SHA256
`52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35`.
Its public encoder, decoder, admission and identity construction were read
directly, without running a Rust behavior oracle.

The repository pins Rust 1.97.1. Its installed source distribution identifies
`1.97.1 (8bab26f4f 2026-07-14)` and Unicode **17.0.0**. The local rustup manifest
names the 2026-07-16 rust-src archive with SHA256
`e9a1e616d04c6845895c827a178b9227f7c7199f3f4a80af81ab3aff7b80156b`
for the xz archive. This is inspected manifest metadata, not a fresh download
or signature verification. Before claiming imported-table provenance, record
the accepted distribution/source authority and complete literal inventories.

| Source relative to Rust `library/` | Inspected SHA256 |
| --- | --- |
| `core/src/unicode/unicode_data.rs` | `4438831f7c584a35457022c4016ab8a295f242ff809b81a5c81b9bb14dc5e166` |
| `core/src/char/methods.rs` | `6794677490411e762ee66df96438a112fb6ca5bdc0a1ae11a9933e93eee77909` |
| `core/src/str/lossy.rs` | `62c3794f5ddbeb0e72bc3342a1648c58fe033c2f33409cac03b6b661d1a52e94` |
| `core/src/str/validations.rs` | `932abb6839ebd86e67ddd2377549ffbb4d91a3fa8ae441181392b2b158b4fa6b` |
| `alloc/src/string.rs` | `68d7464cc83b0064bac9b600b72b4feb7c90eb5efc92def549b7eb586c6537ed` |

Alphabetic and Numeric use the source's compressed skip tables, with explicit
ASCII fast paths. The generated table comments report 759 non-ASCII Alphabetic
ranges and 145 Numeric ranges. White_Space uses the complete 256-byte lookup
map plus its high-byte branches and ASCII fast path. These are imported facts;
the LexLean lookup, scalar validation and segmentation are built claims.
No host Unicode library, locale, regex, or approximate range set is an oracle.

Cargo.lock pins serde_json 1.0.150, package checksum
`e8014e44b4736ed0538adeecded0fce2a272f22dc9578a7eb6b2d9993c74cfb9`,
and blake3 1.5.0, package checksum
`0231f06152bf547e9c2b5194f247cd97aacf6dcd8b15d8e5ec0663f64580da87`.
Inspected serde_json source hashes are:

| Path | SHA256 |
| --- | --- |
| `src/de.rs` | `4998d7e252eb513be85b298e287048b4fb283882e9ba7df905f572f07200f3f3` |
| `src/value/de.rs` | `ceda1bf415aafde89caa83bf275811b70d393717d4e1038e0c61822d2da02993` |
| `src/read.rs` | `17a2efe49e5a6937c8d7d4a5c0b9eac84453de0f86bf8e7ee421387e82341bb3` |

There is a real parser-profile dependency: the tokenizer crate requests default
serde_json features, while pinned `uor-r4-core` and `uor-r4-graph-cli` enable
`float_roundtrip`. Cargo feature unification can select the latter parser in
a consuming binary. Cargo.lock alone does not determine that feature closure.
The initial proposed compatibility profile is the standalone tokenizer default
(`std`, no `float_roundtrip`, `arbitrary_precision`, `preserve_order`, or
`unbounded_depth`). A source-bound consumer must declare and bind its actual
profile; it cannot inherit this profile merely from the crate version. A
second profile requires its own exact number-admission definition and controls.

## Proposed separately registered components

Parent assigns final R4 IDs and registers authority/ledger entries. The symbols
below describe scope, not allocated IDs. Every component has a complete public
contract and its own theorem fixture; the final tokenizer claim requires their
composition. Pending dependencies do not turn into successful placeholder
results or caller-supplied assertions of validity.

The parent has now registered the Text component as **R4-14**, with authority
`AUTH-TOKENIZER-TEXT`, source authority `UOR-TOKENIZER-TEXT`, and suite
`r4-tokenizer-text`. Its dedicated verifier is
`crates/conformance/tests/tokenizer_text.rs` and its scenario file is
`features/suites/r4-tokenizer-text.feature`. Other component IDs remain pending.

| Component | Owned source | Complete scoped behavior |
| --- | --- | --- |
| Text primitives | `src/R4/TokenizerText.lex.tex` | Valid scalar/byte domains, strict UTF-8 and exact Rust lossy decoding, UTF-8 encoding, complete pinned Unicode predicates, exact byte alphabet/inverse |
| Semantic kernel | `src/R4/TokenizerKernel.lex.tex` | Typed vocabulary/merge/pre-tokenizer admission; added-token matching; Digits; prefix spacing; ByteLevel state machine; complete rank BPE; raw/permissive/strict decode and token lengths |
| Raw admission | `src/R4/TokenizerJson.lex.tex` | Complete bounded JSON syntax and chosen source parser profile; duplicate last-wins Value semantics; tokenizer field extraction; exact source admission over the parsed result |
| Identity/composition | `src/R4/Tokenizer.lex.tex` | Raw JSON BLAKE3 CID, added-list BLAKE3 digest, canonical adapter bytes/digest, connection from admitted raw bytes through all semantic operations |

Controls respectively live at `tests/lexlean/TokenizerTextControls.lex.tex`,
`TokenizerKernelControls.lex.tex`, `TokenizerJsonControls.lex.tex`, and
`TokenizerControls.lex.tex`. The future composed verifier entry is
`crates/conformance/tests/tokenizer.rs`; its scenarios are in
`features/suites/r4-tokenizer.feature`. Text has the separate registered files
named above and may use additional `TokenizerText*.lex.tex` source modules.
Only parent changes registrations,
mandatory-test audit, default entrypoints, locks, or generated conformance.

Pure generic BLAKE3 is a **required dependency** of the fourth component.
Existing SHA256 and configured BLAKE2b-96 are different algorithms. They cannot
replace BLAKE3, and a supplied CID/digest is not verification. A separately
reviewed shared BLAKE3 module must cover ordinary unkeyed 32-byte output,
64-byte block and 1024-byte chunk boundaries, chunk counters, tree reduction,
and ROOT output flags. Its final module ownership/registration is parent-owned;
this plan does not hide a second hash algorithm in tokenizer fixtures.

The first three components may establish their own complete claims without
BLAKE3. Their return types are text, admitted semantic configuration or parsed
body, never a trusted tokenizer identity or production-ready adapter. The
fourth component has no accepted public constructor until real hash calls and
independent known-answer controls connect it to the exact raw bytes.

## Representation and proof design

Use closed records and inductives with globally distinctive `tokenizer...`
names to avoid the production exporter's current naming collisions. Bytes
reuse `R4.Words.Byte8` where practical; scalar values are admitted naturals in
0..0x10FFFF excluding 0xD800..0xDFFF. Text is a scalar list with source-defined
UTF-8 projection. A raw-byte boundary checks every incoming natural before
conversion; it never truncates. No opaque host String operation supplies
Unicode, JSON, sorting, searching or BPE behavior.

The approved representation is a complete balanced interval decision tree in
`TokenizerTextUnicode.lex.tex`, with scalar-domain checks in TokenizerText.
Every interval comes from the reviewed complete maximal-range inventory, never
from boundary answers. Independent static decompression established equality
between the full Rust compressed tables plus ASCII and the independent UCD
ranges. This is reviewed imported-data provenance, not a model-proved conversion
or universal lookup theorem. The original compressed literal inventory remains
in `tests/fixtures/tokenizer/unicode-inventory.json`. Independent review must
flatten every emitted tree interval, compare the complete inventory, and check
ordering, disjointness and branch coverage. No host Rust predicate is used to
generate answers. Every transition boundary and adjacent scalar is controlled.

Represent vocabularies and merges as finite ordered lists with exact key
equality. Admission retains distinct `modelVocabularySize` and final slot count.
Sparse added slots are represented explicitly or by lookup returning an empty
slot; absence must not alias a token containing an empty string. Merge keys are
the actual concatenated `left + ASCII space + right` strings, including source
collisions involving spaces. A pair-record map would silently change admitted
array-merge semantics. Retain the first rank for a repeated joined key.

Represent JSON with a flat typed token/node tape, subtree extents and a list
of closed parser frames. This avoids unsupported recursive custom inductives.
Implement lexical scanning, string escape/surrogate handling, container
transition and value construction as separate structural folds. Complete
consumption, source recursion depth and explicit application byte/node limits
are independent checks. Duplicate object keys overwrite the preceding value
in compatibility mode. Key equality is equality of decoded String/UTF-8 content,
not raw JSON token spelling: escape-equivalent keys collapse. Every overwritten
value still passes full JSON parsing before replacement; a syntactically valid
wrong-typed value may disappear, but a parse error cannot. A strict application
profile may reject duplicate keys, but must
name that policy and cannot claim source-compatible acceptance.

Source number admission is part of JSON parsing even for ignored metadata.
Keep integer, negative integer, and floating forms distinct; `-0`, decimal and
exponent forms do not become UInt32 IDs. Unknown finite-valued metadata must
not be rejected merely because it is irrelevant to tokenization. Implement
the selected parser's finite-number acceptance using exact integer/bit
semantics, including its rounding/overflow path and exponent saturation, or
resolve an accepted generic source-defined parser dependency. Do not substitute
an approximate exponent threshold, skip numeric validation, or execute host
floating-point helpers. This is load-time syntax compatibility, not a model
float operation or an approved D11 runtime primitive.

Recursion is structural on remaining input/arrays or on a proved measure.
BPE's successful pass removes at least one symbol. A symbol-count budget may
be internal only if its adequacy and final no-pair condition are established;
zero fuel cannot return an unfinished symbol list as success. Lexer/parser
budgets similarly must derive from input and progress or return an explicit
application capacity error, never silently truncate. Keep proof-cost changes
separate from statement changes and preserve the none-axiom policy.

## Ordered public errors and source-compatible distinctions

The Rust constructor returns only `None`, so richer typed failures are a new
diagnostic refinement. Preserve accept/reject behavior and document the chosen
order; do not attribute detailed error labels to upstream. Proposed order is:

1. Application byte/capacity admission and byte/scalar validity.
2. JSON syntax/UTF-8/escape/number/depth/complete-consumption failure, at the
   first source-order failing cursor.
3. Missing model, wrong string-valued model type, missing/nonobject vocabulary,
   vocabulary size, then vocabulary ID/domain/density/alias errors in ordered
   object-key order after duplicate collapse.
4. Added-token array entries in input order: content type/empty, ID domain,
   ID-to-content or content-to-ID alias; then final slot-count limit.
5. Missing/nonarray merges, each malformed merge shape or rank overflow in
   original order, preserving first duplicate rank.
6. Missing/unsupported pre-tokenizer, bad Sequence shape, unsupported order or
   step, and missing final ByteLevel.
7. Strict consumer-only profile checks, including no prefix insertion,
   full byte-alphabet coverage, model-vocabulary binding, byte limits and
   exact admitted ID handling.
8. Actual raw-byte/adapter identity comparison to independently admitted
   expected identities, with no caller-owned truth flags.

Absent/non-string model.type is accepted by that check. Missing/null/nonarray
added_tokens is empty. Model vocabulary may contain the empty string; added
content may not. Repeated identical added content/ID is idempotent and can
mark an existing model ID as literal-decoding. Digits defaults false and
ByteLevel prefix defaults true for missing/non-Boolean fields. Multiple
Digits before ByteLevel retain the last policy. Every step after ByteLevel
is rejected. Unexecuted normalizer, postprocessor and chat-template fields
remain in raw identity but do not become behavior.

## Frozen literal controls before definitions

These expected results are derived from source and arithmetic, not a candidate
implementation. The fixture will contain declarations/theorems only, with
literal inputs and outputs; no fixture-side tokenizer algorithm or test helper
may compute expected results. The following is the initial frozen inventory.
Complete Unicode boundary and identity-vector inventories require review before
their respective implementation phase, rather than invented digest literals.

### Text and alphabet

| Input | Exact expected result |
| --- | --- |
| Scalars 0, 0x7F, 0x80, 0x7FF | UTF-8 `[00]`, `[7F]`, `[C2,80]`, `[DF,BF]` |
| Scalars 0x800, 0xD7FF, 0xE000 | `[E0,A0,80]`, `[ED,9F,BF]`, `[EE,80,80]` |
| Scalars 0x10000, 0x10FFFF | `[F0,90,80,80]`, `[F4,8F,BF,BF]` |
| Scalars 0xD800, 0xDFFF, 0x110000 | InvalidScalar |
| Bytes `[C0,80]`, `[ED,A0,80]`, `[F4,90,80,80]`, `[C3]` | Strict UTF-8 failure |
| Lossy `[C3,A9]`, `[C3]`, `[C3,28]` | Scalars `[00E9]`, `[FFFD]`, `[FFFD,0028]` |
| Lossy `[E2,82]`, `[E2,28,A1]` | `[FFFD]`, `[FFFD,0028,FFFD]` |
| Lossy `[ED,A0,80]` | `[FFFD,FFFD,FFFD]` |
| Alphabet bytes 0, 9, 32, 33, 126, 127, 160, 161, 172, 173, 174, 255 | Scalars 256,265,288,33,126,289,322,161,172,323,174,255 |
| Each byte 0..255 | Exact inverse of its literal alphabet image |
| Scalar U+00AA, U+00B2, U+4E09, U+0085, U+200B | `(Alpha,Numeric,White)` = `(T,F,F)`, `(F,T,F)`, `(T,F,F)`, `(F,F,T)`, `(F,F,F)` |

### Segmentation and merge kernel

The small vocabulary fixture F is exactly
`a:0,b:1,c:2,ab:3,bc:4,Ã:5,©:6,Ġ:7`; added `<end>:8`;
merges `b c` at rank0 and `a b` at rank1; no prefix and no Digits.

| Input/setup | Exact expected result |
| --- | --- |
| F encodes `abc<end>abc` | `[0,4,8,0,4]` |
| F encodes ` é` | `[7,5,6]` |
| F decodes `[0,4,8]` | Bytes of `abc<end>` |
| F raw-decodes `[5]`, `[6]`, `[5,6]` | `[C3]`, `[A9]`, `[C3,A9]` |
| F text-decodes `[5,6]` | U+00E9, not two replacement characters |
| F encodes `z` | `[]`; compatibility encode_lossy returns `([],0)` |
| F raw-decodes `[999,0]` | `[61]`; strict decode rejects ID999 |
| F token_byte_len(5), token_byte_len(999) | Some1, None |
| Pre-tokenize `can't` | `["can","'t"]` |
| Pre-tokenize `CAN'T` | `["CAN","'","T"]` |
| Pre-tokenize `!'s` | `["!'","s"]`; punctuation does not restart contraction matching |
| Pre-tokenize `  a` | `[" "," a"]` |
| Pre-tokenize `\t a` | `["\t"," a"]` |
| Pre-tokenize ` \ta` | `[" ","\t","a"]` |
| Pre-tokenize `a  ` | `["a","  "]` |
| Pre-tokenize `é²三` | `["é","²","三"]` |
| Digits individual `a12b`, prefix true | Pre-tokens `[" a"," 1"," 2"," b"]` |
| Digits runs `a12b`, prefix true | `[" a"," 12"," b"]` |
| Prefix true on `\ta`, no Digits | `[" ","\t","a"]` |
| Added `<a>:9`, `<ab>:10`; input `x<ab><a>` | Text `x`, Special10, Special9 |
| Added `a:9`, `ba:10`; input `aba` | Special9, Special10; earliest position wins over later length |
| Symbols `[a,a,a,a]`, merge `a a` | One pass `[aa,aa]`, preserving nonoverlap |
| Symbols `[a,a,a]`, merge `a a` | `[aa,a]` |
| Ranks `a b`:0, duplicate `a b`:1, `b c`:2 | `abc` merges to `[ab,c]` |
| Array merge keys `["a b","c"]`, `["a","b c"]` | Both store `a b c`; first rank retained |

### Admission and byte identity

Start with minimal valid JSON M:
`{"model":{"vocab":{"a":0},"merges":[]},"pre_tokenizer":{"type":"ByteLevel","add_prefix_space":false}}`.
Raw bytes are exactly the one-line UTF-8 spelling above, without final newline.
The expected parsed body has one model slot, no added entries or merges, false
prefix, and no Digits. The final BLAKE3 literal must come from an independently
pinned vector source before identity implementation; no guessed digest enters
the fixture. Identity tests also freeze M plus one final newline as different
raw bytes with the same semantic body and different independently known CID.

| Mutation or additional fixture | Exact expected result |
| --- | --- |
| M model.type null, number0, absent | Accepted semantic body |
| M model.type `Unigram` | WrongModelType |
| M added_tokens null/false/object | No added tokens |
| M ID `-0`, `0.0`, `0e0`, -1, 4294967296 | InvalidVocabularyId |
| Model vocab `a:0,b:0`; `a:0,b:2` | Duplicate ID; nondense ID |
| Empty model vocab | VocabularySize |
| Model vocab empty-string:0 | Accepted; token_byte_len(0)=None; raw decode empty |
| Added `a:0` twice over M | One added entry; ID0 literal decode |
| Added empty-content:1; `b:0`; `a:1` | EmptyAddedContent; AddedIdAlias; AddedContentAlias |
| Added `z:3` over M | Four slots; slots1/2 empty; strict lengths None |
| Added ID1048576 | FinalVocabularySize |
| String merge `a  b`; one-element array merge | MalformedMerge |
| String merge ` `; array merge `["a b",""]` | Accepted keys ` ` and `a b ` |
| Sequence Digits(true), Digits(false), ByteLevel(false) | Effective Digits(false), prefix false |
| Sequence ByteLevel, Digits; two ByteLevel steps | UnsupportedPreTokenizerOrder |
| ByteLevel prefix null/string; Digits individual null/string | Defaults true; defaults false |
| JSON duplicate vocabulary key `a:1,a:0` | Last value0; one dense slot accepted |
| JSON duplicate vocabulary key `a:"wrong",a:0` | First value parses, then disappears; accepted one-slot vocabulary |
| JSON duplicate vocabulary key `a:1e9999,a:0` | NumberOutOfRange while parsing first value; overwrite cannot recover |
| JSON vocabulary keys `"a":1,"\u0061":0` | Decoded keys are equal; one slot with ID0 accepted |
| JSON `"\uD83D\uDE00"` | Scalar U+1F600 |
| JSON unpaired high/low surrogate, unescaped control, trailing value | Typed syntax failure |
| JSON numeric ignored field `1e9999` | Source-profile NumberOutOfRange |
| JSON numeric ignored field `0e9999`, `1e-9999` | Finite-number syntax accepted in proposed default profile |
| Added listing entries (10,`a:b`), (2,`é\n`) | Bytes `2:3:é\n\n10:3:a:b\n`, sorted by ID |

Also freeze container-depth boundary controls directly from the selected
serde_json decrement-before-enter rule (initial remaining depth128), numeric
rounding/overflow boundaries, all JSON escape forms, aliases involving decoded
escape-equivalent keys, byte budgets before concatenation, and all adapter
field names/order/newlines. Final adapter literals use real independent
BLAKE3 fixtures for raw source bytes, added listing and canonical line record.

## Complete Unicode review packet

The preparation packet is outside the implementation tree at
`/tmp/lexlean-r4-tokenizer-unicode-20261021/`. It contains copied Rust literal
arrays, independent Unicode Character Database ranges, and expected boundary
facts. No candidate lookup or Rust classification function was executed to
obtain the expectations; text extraction, adjacent-range coalescing and literal
boundary projection are preparation of imported source data.

The independent sources are Unicode 17.0.0
[DerivedCoreProperties.txt](https://www.unicode.org/Public/17.0.0/ucd/DerivedCoreProperties.txt)
for Alphabetic,
[UnicodeData.txt](https://www.unicode.org/Public/17.0.0/ucd/UnicodeData.txt)
for general categories Nd/Nl/No, and
[PropList.txt](https://www.unicode.org/Public/17.0.0/ucd/PropList.txt)
for White_Space. Their complete downloaded-byte SHA256 values are respectively
`24c7fed1195c482faaefd5c1e7eb821c5ee1fb6de07ecdbaa64b56a99da22c08`,
`2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c`,
and `130dcddcaadaf071008bdfce1e7743e04fdfbc910886f017d9f9ac931d8c64dd`.

`unicode-inventory.json`, SHA256
`e5d09aaa3d8edd50b0fedd1ea887f779fc319787a9c5f9e74b108f4c6a251b7d`,
contains all 51 Alphabetic run headers and 1519 offset bytes, all 43 Numeric
headers and 291 offset bytes, and all 256 whitespace-map bytes with its four
high-byte cases. The independent UCD ranges include ASCII and contain 761
Alphabetic ranges/147421 scalars, 146 Numeric ranges/1924 scalars, and 10
White_Space ranges/25 scalars. Subtracting the Rust ASCII fast paths agrees
with the Rust generated inventory counts; this count check is not itself a
proof of table equivalence.

`unicode-boundaries.json`, SHA256
`e2d378cac11a10f0028a49c270181ee738d5b0506b76c8291c1f5780fcfb6655`,
freezes 2649 Alphabetic, 575 Numeric and 35 White_Space scalar/Boolean pairs.
Every maximal true interval contributes its first and last scalar and each
adjacent valid scalar; duplicate points are combined with a consistency check.
Scalar zero and U+10FFFF are included. Invalid surrogate/domain controls remain
separate typed failures. These are preimplementation review inputs, not accepted
runtime artifacts. Independent review of complete inventories and boundary
coverage precedes their literal theorem projection and source definitions.

## Execution sequence and checks

1. **Register and freeze controls.** Parent registers complete scoped IDs and
   authorities. Add scenarios and theorem-only literal fixture files above,
   then demonstrate missing-definition RED. Risk low; no source implementation
   before review. Rust entrypoint only invokes the exact SDK and validates
   current source/config/module/axiom/control attestation; it never tokenizes.
2. **Text primitives.** Implement scalar/byte admission and each UTF-8 branch
   in small reviewable groups; then exact lossy grouping, imported Unicode
   lookup, byte alphabet and inverse. Dependency: frozen text controls and
   reviewed source tables. Risk medium. Plant invalid-surrogate acceptance,
   wrong whitespace classification and swapped alphabet byte defects.
3. **Kernel.** Implement typed configuration admission, ordered added matching,
   Digits/prefix/ByteLevel segmentation, merge rank selection/all-occurrence
   pass, then fallback and both decode surfaces. Dependency: text component.
   Risk medium. Each small function group receives its own RED/GREEN controls.
   Plant rank-vs-ID, one-occurrence-only, wrong-space-prefix and lost-byte defects.
4. **JSON admission.** Implement tokenizer-independent lexical/container
   primitives in TokenizerJson, chosen source number profile, Value semantics,
   then exact tokenizer extraction. Dependency: text/kernel and parser profile
   review. Risk high. Prove structural progress and scoped capacity behavior.
   Plant float-ID acceptance, duplicate-first-wins, permissive surrogate,
   ignored out-of-range number and sparse model-ID defects.
5. **Identity composition.** Connect actual shared BLAKE3 to original raw bytes,
   exact listing and canonical adapter record. Dependency: reviewed generic
   BLAKE3 plus all preceding components and independently frozen digest vectors.
   Risk high. Plant normalized-JSON hashing, SHA substitution, wrong byte length,
   missing final newline and supplied-digest-trust defects.
6. **Consumer acceptance.** Connect strict no-prefix/full-alphabet/ID/byte-budget
   admission to durable memory and generated application roots through parent
   integration. Preserve raw byte decoding separately from lossy display.
   Dependency: full accepted tokenizer identity and production SDK/runtime.
   Risk high. No claim that finite compiler controls establish browser custody,
   generated allocation behavior, D11 instruction compliance or GPU parity.

The authoritative algorithms live only in LexLean. Documentation and fixtures
are neither runtime lookup tables of answers nor second implementations. Small
literal formal checks may use the isolated pinned development SDK while the
parent's new-SDK Hash job owns that container; final verification/attestations
must be refreshed under the accepted project SDK and current configuration.
Do not modify SDK limits, axioms, generated Lean, public expectations, or skip
controls to hide proof-cost failures. Parent performs the required complete
`just vv` and atomic conventional commits after independent review.

## Review decisions needed before definitions

- Approve the four complete claim boundaries and concrete source ownership.
- Bind the initial standalone default serde_json feature profile and require
  explicit distinct profiles for consuming feature closures.
- Accept the complete source-literal Unicode import method and independently
  review its inventories before claiming exact non-ASCII semantics.
- Allocate the shared BLAKE3 implementation/verification dependency and freeze
  independent identity vectors before the final composition begins.

These are architecture/registration decisions for the parent review, not a
request to reduce the approved full tokenizer scope. All four components,
raw-byte identity and their real production integration remain required.

## R4-14 Text implementation and evidence

The initial complete Text source defines scalar admission, Alphabetic/Numeric/
White_Space wrappers, byte alphabet and inverse, UTF-8 scalar/text encoding,
strict decoding and Rust-compatible lossy decoding. Public boundary names are
`tokenizerAlphabetic`, `tokenizerNumeric`, `tokenizerWhitespace`,
`tokenizerAlphabetEncode`, `tokenizerAlphabetDecode`, `tokenizerEncodeScalar`,
`tokenizerEncodeUtf8`, `tokenizerDecodeUtf8`, and `tokenizerDecodeUtf8Lossy`.
Internal codec helpers are called only after their scalar/byte/state domain
checks; they are not independent arbitrary-input admission surfaces.

Failures are `invalidScalar(value)`, `invalidByte(value)`, and
`invalidUtf8(sequenceStartOffset)`. Both byte decoders first validate the complete
byte domain, so an out-of-domain byte is rejected before UTF-8 interpretation.
Strict decoding rejects incomplete, overlong, surrogate, out-of-range, invalid
lead and interrupted continuation sequences. Lossy decoding emits one U+FFFD
per Rust invalid chunk: a bad continuation is processed again as the next lead,
and a truncated otherwise valid prefix produces one replacement. It never
discards an interrupted ASCII byte. Raw byte preservation and lossy display
remain separate later tokenizer surfaces.

UTF-8 encoding decomposes a valid scalar into four fixed bit groups using
bounded comparisons and addition. Decoding uses a structural byte fold and
finite complete payload-position maps; no host Unicode/string operations,
floating-point arithmetic, integer division or unary large-number subtraction
enter the source. These source facts do not establish generated D11 allocation
or instruction properties, and finite fixtures do not constitute a universal
encoding/classification theorem.

The original theorem-only fixture contains **3865** unique parameterless
reflexivity controls, all with empty axiom declarations: 3259 independently
frozen Unicode boundary facts, 512 alphabet forward/inverse cases, and 94
UTF-8/domain cases. SHA256 is
`ec3b221ab55f1cb5b36460d0eac2208d4e26de76b1b390ce8dbb44a4064a4044`.
The exact reviewed inventory and boundary JSON bytes are preserved under
`tests/fixtures/tokenizer/`. The fixture contains no definitions or expected-
value computation. The Rust verifier invokes and checks SDK attestations only.

After isolated Lake/lock setup, the original fixture produced expected
`LLR3005` missing `R4.TokenizerText`; the earlier missing local lock/Lake files
were setup failures, not behavioral RED evidence. Source check then passed.
Full development-SDK verification of the unchanged fixture passed all three
modules with attestation
`d44b7369fab19ae1f587e38d782e79d2481c03035d73d94f5ea0ecb8cc946300`,
source ID `eb6bc718a27a9c6eac25e88d63faed3d25bff1aa870a695c5432b0ae85656e92`,
and semantic ID
`36689a749a154a85dd6c8b02789fd6dc6730c0d6a434c7964b3f0244b0065d60`.
This is an isolated development receipt, not current production-SDK/Rust-gate
or full-project acceptance. Parent integration must refresh those checks under
the locked SDK and current project configuration.

Four isolated planted-source defects were rejected by the complete unchanged
fixture with `LLV7002`, exit1:

| Defect | Observed rejecting controls |
| --- | --- |
| Remove the surrogate exclusion from scalar admission | 18 failures across scalar encoding, classification and alphabet admission |
| Mark U+0085 as not whitespace | `unicode_white_space_133` |
| Change alphabet image of byte0 from U+0100 to U+0101 | `alphabetEncode_0` |
| Drop an interrupted continuation byte instead of processing it as a new lead | Six lossy cases, including `[195,40]`, `[237,160,128]`, and `[65,226,130,66]` |

Mutations affected only the isolated development copy. Shared source and frozen
controls remained unchanged. Logs are `/tmp/r4-text-mutant-surrogate.json`,
`/tmp/r4-text-mutant-whitespace.json`, `/tmp/r4-text-mutant-alphabet.json`, and
`/tmp/r4-text-mutant-lossy-drop.json`; these are local diagnostic evidence,
not durable release receipts. The full original development source was restored;
final full replay passed all 3865 controls and reproduced attestation
`d44b7369fab19ae1f587e38d782e79d2481c03035d73d94f5ea0ecb8cc946300`.
Its 3915 declaration audit rows all have empty observed axioms and each of the
three modules has successful independent leanchecker replay. Rust formatting
checks pass; actual dedicated Rust-gate
execution remains part of parent SDK integration.

### Integrated Text acceptance

Independent full source, Rust and input-boundary review passed after the
Unicode inventory/tree and fixture reviews. In the isolated thirteen-component
project, the actual newer-SDK test
`cargo test --offline --locked -p repo-conformance --test tokenizer_text`
passed in 65.20 seconds: one mandatory test, zero failures, ignored or filtered
tests. The common gate audited all 3,915 declaration rows and all three kernel
replays against the unchanged 3,865 controls and current project inputs.
Its attestation is
`150e1654ec1351ef0b483d7085d058ef5d75f650c0c0376258e1574ca7d7ac22`.
The project configuration SHA-256 is
`582dd99a4afa394ef4adccb717610982db1686ab9f7dfa6c8c0e2515a198e59c`;
the regenerated language lock SHA-256 is
`b6e36d0c495ddfd9732f70c643e3ec7e00d2ed97372269d546a3ceb648bd0c64`.

Formatting, thirteen-ID model consistency, lock checking and all-target/
all-feature Clippy passed. The mandatory-test audit now requires this exact
test and the default source selection includes TextControls once. This is a
targeted development integration result; it does not replace full `just vv`,
establish BPE/JSON/identity composition or accept a production SDK/runtime.
