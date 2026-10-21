# Tokenizer JSON syntax and extraction API

Status: independently reviewed implementation refinement, 2026-10-21.
Parent source/contract review approved the parser profile, string-error timing,
postorder/duplicate semantics, exact public types and staged extraction order.
Frozen controls, missing-definition RED and implementation remain required.
This specifies the complete raw admission component already required by
[TOKENIZER.md](../TOKENIZER.md). It introduces no source implementation,
accepted parser claim, tokenizer identity, or production runtime claim.
The parser profile is serde_json 1.0.150 with `std`, without
`float_roundtrip`, `arbitrary_precision`, `preserve_order`, `raw_value`, or
`unbounded_depth`. Consumers must bind the effective feature closure explicitly.
The numeric dependency is the independently reviewed
[JsonNumber scanner](tokenizer-number-scanner-api.md), including its exact
[finite-number arithmetic](tokenizer-number-admission.md).

Module ownership is exact: `R4.Json` owns all `Json...` types below, parser,
`jsonSourceProfile`, `jsonParse`, and `jsonCheckDocument`;
`R4.TokenizerJson` owns all `TokenizerJson...` types, staged extraction and
`tokenizerFromJson`. The former depends on `R4.JsonNumber`,
`R4.JsonNumberArithmetic`, and `R4.TokenizerText`; the latter additionally
depends on `R4.TokenizerKernel`. Existing Text/Unicode and Kernel source is
language 1.1: any isolated 1.2 import closure must use a separately audited
version-only migration preserving every semantic declaration and control
statement, followed by complete replay. This document does not migrate the
accepted project or infer mixed-version compatibility.

## Inspected authority

UOR authority is revision `4611538bb4843947363ccb9cce5f388e32c46883`,
[`crates/uor-r4-tokenizer/src/lib.rs`](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-tokenizer/src/lib.rs),
SHA-256 `52949ae97d4043547d8a3897d04298e0bed64fc5538c835c077feaa0537cff35`.
Constructor lines 91–233 and `parse_pre_tokenizer` lines 590–628 were inspected
directly. No tokenizer, serde parser, model, or GPU computation was executed.

The locally inspected serde_json package checksum is
`e8014e44b4736ed0538adeecded0fce2a272f22dc9578a7eb6b2d9993c74cfb9`.
Its source files have these SHA-256 pins:

| File | SHA-256 | Relevant contract |
| --- | --- | --- |
| `src/de.rs` | `4998d7e252eb513be85b298e287048b4fb283882e9ba7df905f572f07200f3f3` | Value dispatch, numbers, depth, punctuation, whitespace, completion |
| `src/read.rs` | `17a2efe49e5a6937c8d7d4a5c0b9eac84453de0f86bf8e7ee421387e82341bb3` | Slice reader, string bytes, escapes, surrogate pairs, UTF-8 |
| `src/value/de.rs` | `ceda1bf415aafde89caa83bf275811b70d393717d4e1038e0c61822d2da02993` | Complete Value deserialization before map insertion |
| `src/map.rs` | `b5a2fd1ee865e6cfd83ef2da0a2e1057f892465ac7ede8a78a11354a60526f33` | Default BTreeMap order and replacement |
| `src/value/index.rs` | `f1404ed12e795a5553da4bdf72835c441a5e0cc0f6e794fee875eaa887d65319` | String-key lookup on objects only |

The applicable reader is `SliceRead`, because the tokenizer uses `from_slice`.
Do not use the subtly different cursor behavior of `IoRead` as authority.
`de.rs:1372–1385` decrements an initial remaining depth of 128 before entering
either container and refuses zero: at most 127 simultaneously open arrays and
objects are accepted. The rejected 128th opening byte remains unconsumed.
`value/de.rs:119–145` parses each value before inserting it into the map.

## Public values and capacity profile

Use distinct `Json...` names and scalar-list text, reusing TokenizerText's
strict UTF-8 and scalar operations. Natural byte values are checked, never
truncated. No host String, map, JSON parser, Unicode classifier, or floating
conversion supplies semantics.

`JsonProfile` contains four natural bounds:

- `maxInputBytes`, admitted in `0..2147483647`;
- `maxValueNodes`, admitted in `0..2147483647`;
- `maxDecodedStringBytes`, admitted in `0..2147483647`;
- `maxContainerDepth`, admitted in `0..127`.

These are explicit application limits, independent of compiler proof limits.
`jsonSourceProfile(inputLimit)` checks the input limit and returns a profile
with both node and decoded-string limits equal to it and depth 127. Every
value needs at least one input byte; every decoded UTF-8 string byte is charged
to raw string content/escape bytes, so this profile introduces no additional
node/string rejection inside the admitted input bound. These inequalities are
implementation proof obligations, not assumed performance facts. A smaller
caller limit is a named application refinement. Zero depth permits scalar
JSON roots. No hidden small default excludes real vocabularies.

Public entry points are:

1. `jsonParse(profile, bytes : List Nat) -> JsonParseResult` parses exactly one
   complete JSON value with permitted surrounding JSON whitespace.
2. `jsonCheckDocument(profile, bytes, candidate) -> JsonCheckResult` reparses
   and compares the entire canonical document, rejecting any changed node,
   span, ordering, count, or root. It is an explicit diagnostic for externally
   supplied tapes, never a caller-owned validity flag.
3. `tokenizerFromJson(profile, bytes) -> TokenizerJsonResult` calls the actual
   parser, ordered extraction, and `TokenizerKernel.kernelAdmit`. Its result
   contains the original bytes, extracted `KernelInput`, and actual admitted
   `KernelConfig`. It has no CID or trusted identity field.

The exact result records/sums are:

- `JsonProfileResult.profile(JsonProfile)` or
  `.error(JsonError, cursor : Nat)` for `jsonSourceProfile`.
- `JsonParseResult.parsed(JsonDocument)` or `.error(JsonError, cursor : Nat)`.
- `JsonCheckResult.checked(JsonDocument)` or `.error(JsonError, cursor : Nat)`.
  A successful check returns the freshly reparsed document. Parser errors take
  precedence over comparison; a comparison mismatch returns `documentMismatch`
  at cursor zero.
- `TokenizerJsonPrepared` has `rawBytes : List Nat`,
  `input : R4.TokenizerKernel.KernelInput`, and
  `config : R4.TokenizerKernel.KernelConfig`.
- `TokenizerJsonResult.admitted(TokenizerJsonPrepared)` or
  `.error(TokenizerJsonError)`, with the exact extraction error sum below.

These are semantic data types, not opaque handles or authenticated receipts.

Internal extraction functions consume only the freshly parsed document. If a
standalone document-extraction API is exposed later, it must perform the same
raw-byte reparse/equality check first. Public constructors in a formal language
are not unforgeable application custody. Later identity composition must start
from actual raw admission and actual BLAKE3 calls, and generated application
entry points must enforce their own custody boundary.

## Canonical flat document

Avoid a recursively nested custom Value type. `JsonNodeId` is an inductive
with exactly one constructor, `JsonNodeId.index(Nat)`. `JsonBinding` is a
record with fields `key : List Nat`, `keyStart : Nat`, `keyEnd : Nat`,
`value : JsonNodeId`, in that order. `JsonNodeValue` is a closed sum:

- `nullValue`;
- `boolean(Bool)`;
- `number(JsonNumberClass)`;
- `text(List Nat)` containing admitted Unicode scalars;
- `array(List JsonNodeId)` in occurrence order;
- `object(List JsonBinding)` in strictly increasing decoded-key order.

`JsonNode` is a record with fields `value : JsonNodeValue`, `start : Nat`,
`end : Nat`, in that order; the latter two form the half-open raw span.
`JsonDocument` is a record with fields `nodes : List JsonNode`,
`root : JsonNodeId`, `inputLength : Nat`, `valueOccurrences : Nat`,
`stringOccurrences : Nat`, `decodedStringBytes : Nat`, in that order.
IDs are exactly zero-based node indices assigned in completion/postorder:
each child ID is smaller than its enclosing container ID. A root scalar has
ID zero. All parsed value occurrences remain in the tape, including values
later overwritten by a duplicate key; these may be unreachable from the final
root. Rejecting all unreachable nodes would incorrectly reject lawful source
documents. Keys are strings but are not separate value nodes.

Array child order is unchanged. On each completed object member, replace the
previous binding with the same decoded key, then retain bindings sorted in the
default source's String order. Valid UTF-8 lexicographic order equals Unicode
scalar lexicographic order; use a source-owned comparator with controls for
ASCII, BMP and supplementary characters. Escape spelling is irrelevant to key
equality. The surviving binding retains the last occurrence's key span/value.
Empty keys, embedded decoded NUL, and canonically equivalent but unequal scalar
sequences remain distinct according to exact String equality; no normalization.

`valueOccurrences` equals the tape length. `stringOccurrences` and
`decodedStringBytes` include every string value and every object key occurrence,
including overwritten keys/values. UTF-8 byte counts, not scalar counts, govern
that budget. Original raw bytes remain with the caller/result for later identity;
the tape is not a canonical serialization of raw tokenizer identity.

## Boundary validation and error order

`JsonProfileField` has exactly `inputBytes`, `valueNodes`, `decodedStringBytes`,
`containerDepth`, in that validation order. `JsonContext` has exactly `value`,
`string`, `array`, `object`, identifying the source EOF category.
`JsonInvariant` has exactly `malformedContinuation`, `nodeReference`,
`nonProgress`, `accounting`.

`JsonError` is a closed sum with representation errors
`invalidProfile(JsonProfileField)`,
`inputCapacity(limit : Nat, actual : Nat)`, `invalidByte(index : Nat)`,
`valueCapacity(limit : Nat, attemptedCount : Nat)`,
`stringCapacity(limit : Nat, attemptedBytes : Nat)`, `documentMismatch`;
syntax errors `unexpectedEnd(JsonContext)`, `expectedValue`,
`expectedLiteral`, `invalidStringControl`, `invalidEscape`,
`unexpectedHexEnd`, `loneSurrogate`, `invalidUtf8(decodedOffset : Nat)`, `expectedColon`,
`keyMustBeString`, `expectedArrayCommaOrEnd`, `expectedObjectCommaOrEnd`,
`trailingComma`, `depthExceeded(limit : Nat, attemptedDepth : Nat)`,
`trailingBytes`; and `numberError(R4.JsonNumber.JsonNumberError)`,
`textError(R4.TokenizerText.TokenizerTextError)`,
`invariantFailure(JsonInvariant)` for checked dependency/internal failures.
Unannotated constructors have no payload. A strict string-decoder
`invalidUtf8(offset)` maps to `JsonError.invalidUtf8(offset)`; other Text
failures remain `textError` and never become accepted text. Every parse/check
error result additionally carries an absolute first-unconsumed cursor.
An invalid byte also carries its separate absolute index. The precise enum
spelling above is frozen with controls before definitions.

At public admission validate profile fields in the order listed above, then
actual whole input length, then every byte in order. These failures have cursor
zero. Do not validate the entire document as UTF-8 before parsing: source
allows arbitrary raw bytes outside strings to reach punctuation/value errors,
and string UTF-8 validation has specific timing described below.

Parse syntax in source order. Check depth immediately before consuming an
opening container. Complete lexical/semantic validation of a string before
charging its UTF-8 bytes and string occurrence; check string capacity before
storing it. Complete a value before checking/incrementing value-node capacity
and assigning its postorder ID. This makes the extra application-capacity
ordering explicit; it is not attributed to upstream allocation failures.
Depth, malformed input and capacity errors return no partial accepted document.
Preserve the first parser error, rather than replacing it during cleanup.

Detailed cursor labels refine the upstream tokenizer's `None` result. They
capture the first failure's consumed slice bytes, not serde_json's line/column
error presentation or the reader position after error cleanup. In particular,
`deserialize_any` can call `end_seq`/`end_map` even after its visitor failed;
cleanup may consume more bytes while the original error wins. The public
cursor is captured before that cleanup and never advances to describe it.
No claim equates this new diagnostic record with serde_json's full Error type.
All syntax/range errors must precede tokenizer shape errors, including errors
inside ignored or subsequently overwritten fields.

## Syntax and complete consumption

JSON whitespace is exactly ASCII space, tab, LF, CR. BOM, comments, NBSP and
other Unicode whitespace have no special treatment. The parser accepts any
JSON root kind; tokenizer extraction later requires object fields. `null`,
`true`, and `false` match their complete lowercase literals with consuming
byte comparisons. Wrong bytes are consumed by literal matching; EOF does not
advance the cursor. Numbers delegate to the actual prefix scanner and retain
its classification/cursor. A suffix after a number is checked by its enclosing
frame or complete-document logic, never silently consumed as numeric text.

A parser frame records array-first/array-next or object-first/object-next,
pending key/colon/value phase, accumulated children/bindings and opening span.
Every transition either consumes input, completes a value and decreases the
pending continuation stack, or terminates. Use structural suffix recursion or
a proved decreasing measure; an arbitrary fuel cap cannot return an unfinished
document. EOF after `[`/`{` differs from EOF after a comma; preserve the source
context categories. Closing brackets must match the frame. Trailing commas,
non-string keys, missing colon, missing comma and missing value are rejected.
Empty arrays and objects still consume one depth level. Sibling containers
release depth; the bound is simultaneous nesting, not total container count.

After one root value, consume only JSON whitespace and require EOF. Unlike
`jsonCompleteNumber`, a complete JSON document accepts surrounding whitespace.
Do not accept concatenated roots or ignored non-whitespace suffixes.

## String, escape and UTF-8 behavior

After consuming an opening quote, accumulate ordinary raw bytes and decoded
escape bytes until a closing quote. Unescaped bytes below 0x20 are consumed and
fail `invalidStringControl`; DEL is allowed. Backslash escapes accept exactly
`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\uXXXX`.
Hex digits accept either case. An invalid escape designator is consumed.
For `SliceRead::decode_hex_escape`, if four bytes remain, consume all four
before deciding whether they are hex; if fewer remain, consume to EOF and
report string EOF. Do not stop at the first invalid hex digit.

A non-surrogate u16 becomes its exact scalar and UTF-8 bytes. A low surrogate
DC00–DFFF by itself fails after its four hex bytes. A high surrogate D800–DBFF
requires an immediately adjacent second `\u` escape and low surrogate. If the
next present byte is not backslash, consume it then fail `unexpectedHexEnd`;
after backslash, similarly consume a present non-`u` byte before that failure.
EOF at either point is string EOF without fictitious consumption. Decode all
four bytes of the second escape before checking its low-surrogate range.
Combine a lawful pair as `0x10000 + (high-0xD800)*1024 + low-0xDC00`.
Do not accept lone surrogates or invoke the raw-byte/WTF-8 reader path: Value
keys and strings require the validating `parse_str` path.

Crucially, SliceRead validates UTF-8 only after it consumes the closing quote
(`read.rs:494–539`, `as_str` at 868). Invalid raw UTF-8 followed by an invalid
escape or control character therefore reports that earlier parser failure;
an unterminated string reports EOF even if earlier bytes were invalid UTF-8.
Accumulate the complete decoded byte string, then run TokenizerText strict
decode at the consumed closing-quote cursor. Expose that source cursor for
`invalidUtf8`; any decoded-buffer offset is additional detail, not a fabricated
raw-file offset. Raw multibyte sequences, escapes, and surrogate-pair output
must together form valid UTF-8. No lossy replacement is permitted here.

## Ordered tokenizer extraction and kernel connection

Only after complete syntax admission resolve fields from the final decoded-key
maps. All generic string-key lookups on nonobjects return absence. Maintain
source order among these stages; richer errors are a diagnostic refinement:

1. Look up root `model`; absence is `missingModel`. Check a string-valued
   `model.type`: only `BPE` is accepted; absence/non-string skips that check.
   Require object-valued `model.vocab`, else `missingVocabulary`. Empty or
   more than 2^20 final vocabulary entries fails before entry extraction.
2. Traverse the unique final vocabulary keys in sorted String order. Each
   value must be `JsonNumberClass.unsigned` and at most u32::MAX. Reuse kernel
   model admission to enforce the dense prefix, ID range and aliases at each
   entry before proceeding. Decoded keys may be empty strings.
3. Missing/null/nonarray `added_tokens` means empty. An actual array is visited
   in input order. Each entry must provide string `content`, nonempty content,
   and unsigned-u32 `id`, in that order. Reuse the actual kernel added-entry
   validation against already admitted entries, so an earlier alias cannot be
   masked by a later malformed entry. Repeated identical pairs are idempotent.
   Then enforce final slot count at most 2^20, as the source does.
4. Require array-valued `model.merges`. Preserve original order. A string maps
   to `KernelMerge.text`; an exact two-string array maps to `.pair`; other
   shapes produce the indexed shape error. Reuse kernel key/rank admission,
   including exactly one ASCII space for string entries, joined-key collisions,
   first duplicate rank, and u32 rank bounds. Do not split array entries or
   normalize their text.
5. Require non-null `pre_tokenizer` with string `type`. `Sequence` requires
   array `pretokenizers`, preserving order; every other string type selects
   a one-step list. A step with type `Digits` maps to `.digits(optionBool)`;
   `ByteLevel` maps to `.byteLevel(optionBool)`; other/missing/non-string
   step types map to `.unsupported`. OptionBool is absent for a missing or
   non-Boolean option. Reuse actual kernel policy: repeated Digits before
   ByteLevel retain the last setting; every later step fails; no ByteLevel
   fails. Unknown types and malformed sequences never become defaults.
6. Construct the complete `KernelInput` and call actual `kernelAdmit`, returning
   its actual configuration. No caller-supplied summary or admission result
   substitutes for this call. Earlier staged checks must agree with this
   final admission; any disagreement is an internal failure, not success.

To preserve stage error order, do not extract all later shapes first and then
invoke kernel admission only at the end. Reuse current kernel validators
(`kernelModelType`, `kernelValidateModel`, `kernelAddedStep`, `kernelMergeKey`,
`kernelPolicy`) with their exact initial/accumulator invariants, or introduce a
separately reviewed generic staged API. No placeholder valid configurations
or forged final summaries are permitted. Tests must pair an earlier semantic
failure with a later shape failure to establish the intended order.

`TokenizerJsonField` has exactly `vocabularyId`, `addedContent`, `addedId`.
`TokenizerJsonStage` has exactly `modelType`, `modelVocabulary`, `addedTokens`,
`finalVocabulary`, `merges`, `preTokenizer`, `finalAdmission`.
`TokenizerJsonInvariant` has `kernelDisagreement(KernelError)` and
`documentReference` (no payload). The exact `TokenizerJsonError` sum is:

- `parseError(JsonError, cursor : Nat)`;
- `missingModel`, `missingVocabulary`, `missingMerges`, `missingPreTokenizer`,
  `missingSequence` (each without payload);
- `invalidField(TokenizerJsonField, index : Nat)`;
- `kernelError(TokenizerJsonStage, R4.TokenizerKernel.KernelError)`;
- `invariantFailure(TokenizerJsonInvariant)`.

`vocabularyId` indices are zero-based sorted final vocabulary-entry ordinals;
added-field indices are original added-array ordinals. An absent/non-unsigned
ID is an `invalidField`, whereas an unsigned value exceeding u32 is passed to
the kernel ID check and retains its `invalidId(value)` error in that stage.
Missing/non-string content is `invalidField(addedContent,index)`; empty string
content retains the kernel `emptyAddedContent` before ID extraction. Missing,
null, or absent/non-string top-level pre-tokenizer type is
`missingPreTokenizer`; `Sequence` with missing/nonarray `pretokenizers` is
`missingSequence`. Unsupported string types and malformed nested steps remain
kernel policy errors. Malformed merge shapes are represented by
`KernelMerge.invalidShape` and immediately checked at their original rank,
preserving `malformedMerge(rank)`. No raw shape error preempts an earlier
admission error. A final `kernelAdmit` failure after all staged checks is
`invariantFailure(kernelDisagreement(error))`, never a successful partial config.
Freeze these payloads and stage priorities in the predefinition fixture.
Ignored normalizer/postprocessor/chat-template/other metadata remains
fully parsed and part of raw identity; it does not gain execution semantics.
This component admits the source's incomplete-byte-alphabet fallback profile.
The existing strict memory-consumer checks remain a separate API, not a
silently strengthened constructor.

## Required independent controls and review

Freeze theorem-only literal expectations before definitions, partitioned by
string decoding, container/tape construction, duplicate semantics, boundaries,
and tokenizer extraction. At minimum include:

- All scalar root kinds, all four whitespace bytes, BOM/NBSP/comments, empty
  input, malformed literals, missing punctuation, mismatched ends, trailing
  commas and multiple roots. Complete JSON whitespace must differ from the
  complete-number wrapper. Mix nested arrays/objects at depths 126/127/128,
  including empty innermost containers and many shallow siblings.
- Every escape, both hex cases, invalid first and fourth hex digits with full
  four-byte cursor advancement, truncated escapes, paired boundary surrogates
  D800/DC00 and DBFF/DFFF, lone/reversed surrogates, missing second slash/u,
  unescaped controls versus escaped controls, NUL, DEL, valid UTF-8 boundaries,
  overlong/surrogate/out-of-range UTF-8, invalid UTF-8 before escape/control/EOF.
- `{"a":0,"\u0061":1}`: one final key with value1, two numeric value nodes,
  one object node, both key-string occurrences charged. Different raw bytes
  with the same final document semantics remain distinct raw identities.
  A first duplicate value `1e9999` fails before replacement; a first finite
  wrong-typed value can disappear. Include nested overwritten objects and
  duplicates in `model`, `vocab`, `added_tokens`, `type`, and options.
- Object ordering across ASCII/BMP/supplementary keys, composed versus
  decomposed Unicode keys, empty keys, escaped NUL, exact postorder IDs, spans,
  overwritten unreachable nodes, duplicate occurrence budgets, every capacity
  boundary and precedence. Tamper each field of an externally checked tape.
- The established full tokenizer fixture, model-type permissiveness, empty
  vocabulary key, float/negative/negative-zero IDs, u32 overflow, sparse and
  duplicate model IDs after key collapse; idempotent/conflicting added tokens;
  string/array merges and collisions; default option types, repeated Digits,
  unsupported/reordered steps and every missing/nonarray field distinction.
- Invalid syntax/UTF-8/numeric metadata in ignored and overwritten fields;
  earlier vocabulary/added semantic failure paired with later malformed merges
  or pre-tokenizer; valid parsed nonobject roots rejected only by extraction.

Plant decoded-key/raw-key confusion, first-wins replacement, skipping ignored
or overwritten values, eager UTF-8 validation, surrogate acceptance, off-by-one
depth, float-ID coercion, altered default option, and bypassed final kernel
admission. Each accepted plant rejection must be a genuine semantic control
failure, not a compiler resource/setup failure. Complete source and immutable
fixtures require independent review plus mandatory receipt verification.

No formal proof here resolves the current Nat/List production lowering,
steady-state allocation or D11 instruction obligations. BLAKE3 identity,
browser persistence, generated application roots, and actual serving remain
separate required work; none is inferred from a parsed or admitted tokenizer.
