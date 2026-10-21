# Proposed complete bounded Snapshot JSON codec API

Status: approved API for R4-22 independent control preparation, 2026-10-21. Parent and independent architecture review approved the exact contract at SHA256 `ca21d9308ca83ed5460a7e9e08a7c31cdf36ac414ca6833ff84fb194b784692a`. Implementation, formal verification, and acceptance remain pending. No candidate parser/serializer, model, host reference, or floating conversion was executed to prepare this document. Independent control review and missing-definition RED must precede source and acceptance. This contract does not alter the artifact-serving design, source snapshot schema, runtime admission, production SDK, or deployment requirements.

## 1. Authority and intended boundary

Pinned UOR-R4 revision: `4611538bb4843947363ccb9cce5f388e32c46883`.

- `crates/uor-r4-discrete/src/snapshot.rs`, SHA256 `c03dcb7bc0e0e966951285a52cf76bf672ee74bf136c42403b40d1a4b5da1170`: Snapshot fields at 10–35; typed-to-Value entry at 50–53; Value-to-Snapshot and two independent compact encodings at 55–67; independent source serialization golden at 76–121.
- `lib.rs`, SHA256 `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9`: Spec fields 55–62; initialization 82–88; ReadSpec Serialize and strict custom Wire Deserialize 105–137.
- `objective.rs`, SHA256 `b813e855be6c4ea460c8711493527d52f1caf1a02bb09b046145f88c7f4ee4d0`: objective names/default 12–25.
- Serde/derive 1.0.228 and serde_json 1.0.150, as pinned by Cargo.lock. Relevant files/source hashes are in the existing snapshot-json-codec-notes.md and tokenizer-json-api.md. Additional inspected derive source pins: `de.rs` SHA256 `08fc2db0db3e7263ccfb8999ecc6249e5a8c3d9d3a3b85ea3b8995e865f08c83`; `de/enum_internally.rs` SHA256 `9d898dbc1c46262f695ef7ba1660b2af70de48e07bbac8f44b740326b7c5b303`; `de/identifier.rs` SHA256 `b653c26f4274e5c323e3f893c6f9a2b21bd5c873abdc2a801f0289a5e56a9f6c`.

The modeled Serde profile is the existing JSON contract's default map/number/depth profile: sorted BTreeMap keys, no arbitrary_precision/preserve_order/raw_value/float_roundtrip/unbounded_depth behavior. Effective features of a production storage consumer remain an external binding obligation, not inferred from Cargo.lock alone.

Proposed module: `R4.SnapshotJson`. It depends on the actual complete `R4.Json` parser/document revalidation, its `R4.JsonNumber` classes, and `R4.TokenizerText` scalar/UTF-8 operations. It uses the exact storage-specific types below; checked correspondence with runtime configuration is separate. No runtime validator is silently called by the storage codec.

The complete component supplies:

1. Exact Snapshot deserialization from the surviving Value represented by a fully parsed raw document, within explicit application budgets.
2. Exact canonical typed Snapshot bytes.
3. Exact compact original-Value bytes for values that actually deserialize as Snapshot, preserving that Value's original surviving representations.
4. A separately named strict raw duplicate-rejecting refinement.

This is not arbitrary serde_json Value serialization: successful Snapshot extraction proves every reachable number is an exact admissible integer. A surviving floating number fails typed admission. Overwritten unreachable values must still parse successfully, but are absent from Value serialization. No float formatter is needed for this precise successful-input domain; a general Value serializer remains separate required work if later exposed.

## 2. Public values and fields

`SnapshotText = List Nat` denotes Unicode scalar lists; every caller-supplied scalar is checked by the actual scalar predicate before encoding. No host String is semantic authority.

`SnapshotSigned32` has constructors `nonnegative(Nat)` and `negative(Nat)`. `negative(n)` means `-(n+1)`, matching the established signed predecessor representation and excluding a second representation of zero. Both constructor payloads must be at most 2147483647. In particular `negative(2147483647)` encodes -2147483648 without signed negation overflow.

`SnapshotValue` has these fifteen fields, in exact declaration and typed serialization order:

| Index | Semantic field | JSON name | Type/domain | Absence/default |
|---|---|---|---|---|
| 0 | schema | schema | scalar text | required |
| 1 | spec | spec | SnapshotSpec below | required |
| 2 | readSpec | read_spec | route / flock(kept) / relation(kept,sweeps) | required |
| 3 | objectivePolicy | objective_policy | nearestAddressMarginV1 / occupiedCellRankHingeV1 | nearestAddressMarginV1 |
| 4 | readInitialization | read_initialization | zeroGatesV1 / activeLeafV1 | zeroGatesV1 |
| 5 | seed | seed | Nat, 0..4294967295 | required |
| 6 | epoch | epoch | Nat, 0..18446744073709551615 | required |
| 7 | table | table | List SnapshotSigned32 | required |
| 8 | routes | routes | List Nat, each 0..4294967295 | required |
| 9 | gates | gates | List SnapshotSigned32 | required |
| 10 | operatorKappa | operator_kappa | scalar text | required |
| 11 | compositionKappa | composition_kappa | scalar text | required |
| 12 | kernelSha256 | kernel_sha256 | scalar text | required |
| 13 | sourceSha256 | source_sha256 | scalar text | required |
| 14 | geometrySha256 | geometry_sha256 | scalar text | required |

`SnapshotSpec` has `vocabulary`, `blocks`, `context`, `keptCells` in that order; each is Nat in 0..4294967295. The last JSON field is `kept_cells`.

Every ReadSpec kept/sweeps payload is UInt32-domain Nat; this storage layer accepts zero and the full UInt32 domain. Runtime kept/sweeps restrictions are not part of deserialization.

Closed enum spellings are exactly:

- objective: `nearest_address_margin_v1`, `occupied_cell_rank_hinge_v1`;
- initialization: `zero_gates_v1`, `active_leaf_v1`;
- read kind: `route`, `flock`, `relation`.

No field validates schema equality, digest spelling/length, kappa meaning, root membership, gate trits, route <64, configuration dimensions, array inventory, active-leaf/read compatibility, instance/epoch custody, or compiled identity. Those remain artifact/runtime admission obligations. Storage success is not authority or execution permission.

## 2A. Exact closed declarations and constructor payloads

The following names, field order, constructor order, and positional payload order are normative for independently frozen controls. They are semantic type notation, not executable definitions. Every type in this section belongs to `R4.SnapshotJson` unless fully qualified. `SnapshotText` and `SnapshotPath` are documentation abbreviations for `List Nat` and `List SnapshotPathPart`; they are not additional named wrapper types in the AST.

```
SnapshotSigned32 = nonnegative(Nat) | negative(Nat)
SnapshotSpec = { vocabulary:Nat, blocks:Nat, context:Nat, keptCells:Nat }
SnapshotReadSpec = route | flock(Nat) | relation(Nat,Nat)
SnapshotObjective = nearestAddressMarginV1 | occupiedCellRankHingeV1
SnapshotInitialization = zeroGatesV1 | activeLeafV1

SnapshotValue = {
  schema:List Nat,
  spec:SnapshotSpec,
  readSpec:SnapshotReadSpec,
  objectivePolicy:SnapshotObjective,
  readInitialization:SnapshotInitialization,
  seed:Nat,
  epoch:Nat,
  table:List SnapshotSigned32,
  routes:List Nat,
  gates:List SnapshotSigned32,
  operatorKappa:List Nat,
  compositionKappa:List Nat,
  kernelSha256:List Nat,
  sourceSha256:List Nat,
  geometrySha256:List Nat
}
SnapshotJsonProfile = { json:R4.Json.JsonProfile, maxEncodedBytes:Nat }

SnapshotField =
  schema | spec | readSpec | objectivePolicy | readInitialization |
  seed | epoch | table | routes | gates | operatorKappa | compositionKappa |
  kernelSha256 | sourceSha256 | geometrySha256 |
  vocabulary | blocks | context | keptCells | kind | kept | sweeps
SnapshotPathPart = field(SnapshotField) | index(Nat) | variantPayload
SnapshotExpectedType =
  snapshotStruct | specStruct | readSpecContainer | text | list |
  enumRepresentation | unit | exactInteger
SnapshotIntegerDomain = uint32 | uint64 | int32
SnapshotEncodingDomain = storage | snapshot
SnapshotInvariant = observerEvent | observerDocument | nodeReference | numericRepresentation

SnapshotJsonError =
  jsonError(R4.Json.JsonError,Nat) |
  invalidOutputLimit |
  duplicateRawKey(Nat,Nat,List Nat) |
  wrongType(List SnapshotPathPart,SnapshotExpectedType) |
  unknownField(List SnapshotPathPart,List Nat) |
  missingField(List SnapshotPathPart,SnapshotField) |
  sequenceTooShort(List SnapshotPathPart,Nat) |
  sequenceTooLong(List SnapshotPathPart,Nat) |
  unknownVariant(List SnapshotPathPart,List Nat) |
  enumMapArity(List SnapshotPathPart) |
  invalidIntegerClass(List SnapshotPathPart) |
  integerOutOfRange(List SnapshotPathPart,SnapshotIntegerDomain) |
  invalidScalar(List SnapshotPathPart,Nat) |
  encodedByteLimit(SnapshotEncodingDomain) |
  invariantFailure(SnapshotInvariant)

SnapshotDecoded = {
  rawBytes:List Nat,
  document:R4.Json.JsonDocument,
  snapshot:SnapshotValue
}
SnapshotEncodings = {
  snapshot:SnapshotValue,
  storageBytes:List Nat,
  snapshotBytes:List Nat
}
SnapshotKeyEvent = { objectStart:Nat, keyStart:Nat, keyEnd:Nat, key:List Nat }

SnapshotDecodeResult = decoded(SnapshotDecoded) | error(SnapshotJsonError)
SnapshotBytesResult = bytes(List Nat) | error(SnapshotJsonError)
SnapshotEncodingsResult = encodings(SnapshotEncodings) | error(SnapshotJsonError)
SnapshotKeyEventsResult = events(List SnapshotKeyEvent) | error(SnapshotJsonError)
```

Payload meanings are exact:

- `duplicateRawKey(objectStart,keyStart,key)` names the enclosing `{` byte offset, repeated key's opening quote offset, and fully decoded scalar key. It does not report the first occurrence's offset or a synthetic parser cursor.
- `sequenceTooShort(path,position)` names the zero-based next required positional field that was absent; defaults at exhausted positions are processed before this error. `sequenceTooLong(path,count)` supplies the admitted number of positional fields (15 Snapshot, 4 Spec, or full ReadSpec array length including the tag: 1/2/3).
- `invalidScalar(path,index)` identifies the zero-based scalar index within that text; the value is recoverable from the caller input and is not a second payload.
- `jsonError(reason,cursor)` embeds the exact two payloads returned by actual JsonParseResult.error or JsonCheckResult.error, including nested JsonNumber errors. The outer Result.error has exactly one payload, SnapshotJsonError. No outer cursor is appended. Invalid JsonProfile maps to `jsonError(invalidProfile(field),0)`, and document mismatch to `jsonError(documentMismatch,0)`, retaining existing Json conventions.
- Only `jsonError` carries an inherited parser cursor. Typed field errors use their path; duplicate errors use named source offsets; invalidOutputLimit and encodedByteLimit have no invented cursor.
- `invariantFailure` is an explicit total internal-failure branch, never an alternative successful value. observerEvent means an expected decoded key-closing transition lacks its required object/pending-key shape; observerDocument means a successful replay does not reproduce the first fresh document; nodeReference means a supposedly admitted document edge/index cannot be resolved; numericRepresentation means a reached number cannot be serialized as the exact integer class guaranteed by successful typed extraction. These branches cannot admit malformed input or substitute zero/empty values. They are not newly asserted user errors on source-lawful inputs.

Paths are root-to-leaf lists. The root path is `[]`. A recognized map field appends `field(...)`; a positional struct field appends `index(position)`; a list element appends `index(elementIndex)`. A map unknown/missing field error uses the containing object's path and its separate key/field payload. A sequence missing/extra error uses the containing array path. An enum singleton-map payload appends `variantPayload`; its variant-name/arity failure uses the enum value's path. A ReadSpec map tag uses `field(kind)`, sequence tag uses `index(0)`; sequence variant-content errors append original array positions 1 or 2, and the expectedCount payload includes the tag. A missing ReadSpec tag uses the ReadSpec container path and `SnapshotField.kind`.

For caller-constructed typed values, paths use semantic field names even though no JSON object exists. Nested Spec fields append their own field selectors. ReadSpec kept/sweeps validation appends `field(kept)`/`field(sweeps)` to the readSpec path. The branch selector itself is a closed enum and needs no externally supplied validity flag.

The new storage types are explicit, rather than aliases to runtime configuration types; correspondence to R4.Configuration is a later checked mapping and cannot insert runtime restrictions into deserialization.

## 3. Budgets and public operations

Proposed `SnapshotJsonProfile` fields, in validation order:

1. `json : R4.Json.JsonProfile`, with its existing ordered domain checks.
2. `maxEncodedBytes : Nat`, admitted in 0..2147483647, applying independently to each complete output byte list.

No hidden length cap is imposed on tables/routes/gates beyond the explicit input/output budgets. Input parser budgets account for every occurrence, including discarded duplicate values and repeated key strings. Larger production capacity profiles require explicit review; this bounded API does not claim all source usize-sized inputs are representable.

Exact public operations (parameters listed in declaration order):

- `snapshotFromJson(profile : SnapshotJsonProfile, rawBytes : List Nat) -> SnapshotDecodeResult`: actual complete parse followed by exact Value-compatible typed extraction. Duplicates use the existing parser's decoded-key last-wins semantics.
- `snapshotFromStrictJson(profile : SnapshotJsonProfile, rawBytes : List Nat) -> SnapshotDecodeResult`: actual complete parse, complete raw duplicate observation/rejection, then the same typed extraction. Strictness changes duplicate policy only; it does not silently forbid positional structs or accepted enum-map forms.
- `snapshotCheckDocument(profile : SnapshotJsonProfile, rawBytes : List Nat, candidateDocument : R4.Json.JsonDocument) -> SnapshotDecodeResult`: actual jsonCheckDocument first, then same extraction. An externally supplied tape or cached prepared value is never trusted.
- `snapshotTypedBytes(profile : SnapshotJsonProfile, value : SnapshotValue) -> SnapshotBytesResult`: validate caller-supplied semantic value in declaration/element order, then exact typed serialization with checked complete output budget.
- `snapshotEncodingPair(profile : SnapshotJsonProfile, rawBytes : List Nat) -> SnapshotEncodingsResult`: actual compatible raw decode, then original reachable-Value compact encoding and typed Snapshot compact encoding. Output order/budget precedence: storage encoding first, then typed encoding. The result is published only when both succeed.
- `snapshotStrictEncodingPair(profile : SnapshotJsonProfile, rawBytes : List Nat) -> SnapshotEncodingsResult`: same pair after strict duplicate rejection.
- `snapshotNativeEncodingPair(profile : SnapshotJsonProfile, value : SnapshotValue) -> SnapshotEncodingsResult`: validate typed value, construct its exact source `to_value` representation (including default omission), then serialize that Value in sorted-key order and the same typed value in declaration order. This models `snapshot_hash_domains`' input conversion, not raw file identity.

- `snapshotKeyEvents(profile : SnapshotJsonProfile, rawBytes : List Nat) -> SnapshotKeyEventsResult`: complete fresh parsing and exact transition replay return all decoded key events in input order, including duplicates and overwritten objects. This diagnostic does not reject duplicates or attempt Snapshot extraction. It validates the same entire profile, including output-limit domain, but emits no serialized bytes; maxEncodedBytes does not cap the event list. Events are bounded by parsed key occurrences and the admitted input/string budgets. Strict admission computes these events internally and never accepts caller-supplied events.

`SnapshotDecoded` contains `rawBytes`, the fresh `JsonDocument`, and `snapshot : SnapshotValue`. These are ordinary semantic data, not unforgeable receipts. No public method accepts this record as proof of admission; composite operations repeat their actual checks or remain internal to the same admitted call.

`SnapshotEncodings` contains `snapshot`, `storageBytes`, and `snapshotBytes`. These fields hold actual complete encodings, not expected digests. The exact Result constructors and all payloads are fixed in section 2A.

Future hash composition must call the actual verified SHA256 algorithm separately on each exact byte list. The source uses bare SHA256 of those bytes, with no added separator, salt, newline, or framing. The labels distinguish domains; they are not bytes prepended by snapshot.rs. Raw FilePin is SHA256 of raw file bytes in a separate domain. No digest string supplied by a Snapshot is accepted as the computed digest, and no hash comparison authenticates provenance.

## 4. Proposed diagnostic types and exact precedence

Diagnostics are typed port diagnostics, not claimed byte-for-byte reproductions of Serde's English messages or line/column errors. Preserve acceptance/rejection and explicit phase order. Parser failures retain their actual JsonError and cursor.

`SnapshotPathPart` is `field(SnapshotField)` / `index(Nat)` / `variantPayload`; `SnapshotField` includes all Snapshot/Spec/ReadSpec fields. Unknown decoded keys are separately carried as scalar lists. Section 2A fixes all constructors, payloads, and exact map/array path conventions. A path names a position, not an authority.

`SnapshotJsonError` variants:

- `jsonError(JsonError,cursor)` for profile/input/document/parser failures;
- `invalidOutputLimit`;
- `duplicateRawKey(objectStart,keyStart,key : List Nat)`;
- `wrongType(path, expected : SnapshotExpectedType)`;
- `unknownField(path,key)`;
- `missingField(path,field)`;
- `sequenceTooShort(path,expectedFieldIndex)`;
- `sequenceTooLong(path,expectedCount)`;
- `unknownVariant(path,name)`;
- `enumMapArity(path)`;
- `invalidIntegerClass(path)`;
- `integerOutOfRange(path,domain)`;
- `invalidScalar(path,scalarIndex)` for caller-constructed typed values;
- `encodedByteLimit(domain)` where domain is storage or snapshot;
- `invariantFailure(reason)` with the exact closed internal invariant reasons in section 2A.

`SnapshotExpectedType` distinguishes Snapshot struct, Spec struct, ReadSpec map-or-sequence, text, list, enum string-or-singleton-map, unit/null, and exact integer. `SnapshotIntegerDomain` distinguishes UInt32, UInt64, Int32. Closed enums prevent arbitrary expected-type flags from changing semantics.

Global phases for raw operations:

1. Validate profile: JsonProfile fields using existing order, then output limit. Invalid profile beats raw byte errors.
2. Complete jsonParse/jsonCheckDocument, including complete input consumption. Parser failures beat every duplicate or typed error, even if a typed error would occur earlier in direct `from_slice::<Snapshot>`.
3. Strict operations only: inspect every raw object-key occurrence; reject first repeated decoded key in input key-opening order, including in overwritten subtrees. This is an explicitly proposed strict policy. Compatible operations omit this phase.
4. Deserialize root as Snapshot with the recursive encounter-order rules below. No global prepass for all unknown fields, missing fields, numeric domains, or runtime constraints.
5. If encoding requested, emit complete storage and/or typed bytes; enforce output limits without truncation or partial success.

For typed-only encoding: profile checks, then all semantic fields in declaration order (nested record fields in their declaration order, list values in index order), scalar/domain failure at first encountered field/element; then output accounting. Output budget does not hide a later invalid semantic field. This ordering is a new explicit semantic boundary for public Nat/scalar records; Serde's Rust types already exclude such invalid values.

## 5. Exact accepted Serde representations and local order

### Snapshot and Spec objects

Use surviving decoded-key bindings in increasing Rust String order, already modeled by Json's scalar-list comparator. For every binding: classify field key; unknown key fails immediately; deserialize its value immediately and propagate its failure before proceeding to later keys. Only after all bindings finish, resolve required/defaulted fields in declaration order. A missing earlier field does not precede a malformed present value encountered during iteration.

Snapshot map required/default distinction is exact: missing objective/init defaults; present null fails enum type; no other missing field defaults. Spec requires all four fields. Unknown nested fields are rejected. Strings are arbitrary admitted Unicode, including empty strings and NUL.

Compatible raw duplicates have already collapsed to the last fully parsed value; typed duplicate errors cannot occur in this Value surface. Earlier finite wrong-type values can disappear. Earlier malformed/infinite-number values still cause complete parser failure before collapse. Do not reproduce direct raw typed struct duplicate behavior under this API.

### Snapshot and Spec arrays

Serde's Value deserializer accepts arrays for structs (`value/de.rs:482–495`). Snapshot positions are exactly the fifteen declaration positions above. Spec positions are its four declaration positions. Deserialize position by position; an element error precedes missing later elements. At the first missing required position, report sequenceTooShort with that position; defaulted positions use their defaults only when the sequence is already exhausted. Later required fields still fail, so a successful Snapshot positional array always has all fifteen elements. Omitting defaulted middle positions does not shift later fields or create holes. Null is an explicit value, not omission.

Once all required positions are decoded successfully, any extra position yields sequenceTooLong; malformed early fields precede extra-element errors. The entire raw JSON has already parsed, so this does not suppress prior parser errors. Value and Content sequence drivers check exhaustion (`value/de.rs:195–211`, Serde private/de.rs:1097–1099,1524ff).

### ObjectivePolicy and ReadInitialization

Accepted form is a recognized exact variant string, or an object with exactly one decoded key equal to a recognized variant name and value null. Object arity is checked before variant name; an empty/multikey object returns enumMapArity even if its first key is unknown. For exactly one key, unknownVariant precedes unit-payload validation. Known key with non-null payload gives wrongType(unit/null). Arrays and numeric variant indices are not admitted by the serde_json Value enum surface (`value/de.rs:234–265,328–340,556–560`). Serialization always emits the string form.

### ReadSpec

Accepted map form uses `kind` plus mode fields. The custom Wire variants are strict struct variants, including empty Route. No source permissive unit-Route behavior is used.

The internal tag is gathered before content is deserialized (Serde private/de.rs:845–889 and derive de/enum_internally.rs:35–70). In the already collapsed map, locate `kind`; if absent, missingField(kind) wins over content unknown/missing/type errors. If present, decode its exact string variant; wrong tag type/unknown variant wins over content errors. Then process the remaining content keys in sorted order, reject unknown keys immediately, decode known values immediately, and resolve missing required mode fields in mode declaration order. Route has no allowed content keys; Flock has kept; Relation has kept then sweeps. Stored map order can place kept before kind, but kept deserialization is still deferred until the variant is known.

Accepted sequence form starts with the exact kind string and contains positional remaining variant fields: `["route"]`, `["flock",kept]`, `["relation",kept,sweeps]`. Empty sequence gives missingField(kind). Tag type/name errors precede content length/type checks. Then apply the same left-to-right field and exhaustion rules; extra elements fail after required fields. No sequence uses `{"kind":...}` as its first element. The generated internally tagged struct visitor retains visit_seq; its Content sequence checks complete consumption. These representations require independent controls before implementation.

## 6. Exact numeric bridge, including negative magnitude

Use the actual freshly parsed JsonNumberClass, never a host floating conversion or a numeric spelling heuristic:

- UInt32: accept `unsigned(n)` iff n<=4294967295.
- UInt64: accept `unsigned(n)` iff n<=18446744073709551615.
- Int32: accept `unsigned(n)` iff n<=2147483647; accept `negativeInteger(m)` iff 1<=m<=2147483648, yielding `negative(m-1)`.
- Every `finiteOther` rejects as invalidIntegerClass, including bare -0, 0.0, 1.0, exponent forms such as 1e0, integral-valued floats, long integer fallback, negative overflow, and underflow-to-zero. No coercion into integer fields.
- Negative integers reject unsigned roles as integerOutOfRange; out-of-bound integer magnitudes reject integerOutOfRange in the declared role. A string/bool/null/container at a numeric position is wrongType(exact integer), not a range error.

The current scanner preserves negativeInteger magnitude through 2^63, enough for every admitted Int32 negative. No new signed-number scanner is necessary for these fields. finiteOther lacks complete floating/overflowed-negative magnitude, and therefore cannot support a general Value serializer. Raw numeric spans and raw bytes remain essential to full document revalidation and any future broader numeric extension; they must not be discarded or reinterpreted as typed integer evidence.

An externally supplied document's numeric constructor cannot forge this admission: every public document surface reparses raw bytes and requires exact complete document equality first.

## 7. Complete canonical output

Typed output is a compact object in Snapshot declaration order, using the exact JSON field names above. Spec is an object in vocabulary/blocks/context/kept_cells order. ReadSpec emits kind first, then kept, then sweeps as applicable. Objective/init fields are omitted exactly when their typed values equal their defaults; nondefaults occupy their declared positions. Route's inactive gates are still fully serialized.

Stored-Value output retains the surviving representation from input: objects sort decoded keys; arrays retain every element and position; singleton enum maps remain maps with null; explicit default fields remain present; ReadSpec arrays remain arrays. It serializes only nodes reachable from the final root, so overwritten values are absent. The typed output normalizes these forms to objects/variant strings and default omission. Native typed-to-Value output first applies those typed choices, then recursively sorts keys for the storage encoding.

For a complete object with explicit defaults, the sorted storage field order is composition_kappa, epoch, gates, geometry_sha256, kernel_sha256, objective_policy, operator_kappa, read_initialization, read_spec, routes, schema, seed, source_sha256, spec, table. The nested sorted Spec order is blocks, context, kept_cells, vocabulary. Relation storage keys sort kept, kind, sweeps, distinct from typed kind, kept, sweeps. Omitted default fields are simply absent from the stored object only when absent in that original Value.

Every output byte follows the pinned CompactFormatter:

- no BOM, whitespace separators, indentation, or trailing newline;
- comma between members/elements, colon after each quoted key;
- decimal unsigned integers without plus/leading zero; negative integers use one minus and the exact magnitude; zero is `0`;
- quote and backslash become `\"` and `\\`;
- U+0008/9/A/C/D use `\b`, `\t`, `\n`, `\f`, `\r`;
- every other ASCII control U+0000..001F uses lowercase `\u00xx`;
- slash is raw `/`; DEL, non-ASCII, U+2028/U+2029, HTML punctuation are raw UTF-8;
- no Unicode normalization or escaping every non-ASCII scalar.

Output accounting charges exact final UTF-8/escape bytes. Equality at maxEncodedBytes succeeds; one byte over yields domain-specific error and no prefix/result pair. Complete array contents and all strings are serialized; no fixed list truncation, omitted inactive arrays, or supplied hash substitution is allowed.

Source-independent implementation obligations include termination over finite validated documents/lists, complete traversal of every admitted node/element, decoded-key ordering, exact decimal generation for UInt64/Int32 endpoints, and preservation of successful storage representation. Structural lower node IDs or explicit list work stacks may provide termination; caller fuel may not convert unfinished work into success.

## 8. Strict duplicate observation through actual Json transitions

JsonDocument contains every value occurrence but only the last key span for each final object binding. Keys are not standalone nodes. Therefore neither surviving bindings nor node reachability establishes absence of duplicate keys.

The observer reuses the actual `R4.Json` state machine without changing its source, JsonDocument schema, or any frozen 399 control. The inspected Json source is SHA256 `b18031a8626363bfcfb6799424e1527da77fc8b2131a1ebfa75d6390a9da16fe`; a separately reviewed version-only language migration may change its source hash and must be pinned before implementation verification.

The exact observer procedure is:

1. Validate the SnapshotJsonProfile and run the actual complete `jsonParse(profile.json,rawBytes)`. Return any parser error unchanged inside SnapshotJsonError.jsonError. No event, duplicate error, or typed extraction is exposed before this completes. A standalone supplied document is never accepted as a substitute.
2. Set JsonEnvironment to the same admitted profile, raw bytes, and actual total length. Initialize the replay JsonState exactly as jsonParseSized: position/nodeCount/stringCount/stringBytes/depth/skip zero; nodes/frames empty; mode value; root none.
3. Recurse structurally through the entire raw byte list. For each byte, call the actual `jsonStepByte(byte,environment,state)`. On continue, retain every returned state field and advance physical position to the prior position plus one, exactly as jsonDrive. On error, propagate the actual error/cursor; successful first parsing implies such a replay discrepancy should not occur, but the total definition must not hide it. Number skip and logical-end handling are therefore the same actual transitions, not reimplemented syntax.
4. An event occurs exactly when the prior state has skip zero and mode `JsonMode.string(stringState)`, stringState.isKey true, stringState.phase normal, and current byte 34 (closing quote), and the actual step succeeds. Require the next state to have mode colon and a head `JsonFrame.objectFrame(objectStart,bindings,some key)`. Emit `SnapshotKeyEvent{objectStart,keyStart:key.keyStart,keyEnd:key.keyEnd,key:key.key}`. The value is the actual decoded JsonKey produced by jsonCloseString → jsonFinishString → jsonStoreKey; do not decode the raw bytes independently. Check keyStart equals the prior string opening and keyEnd equals prior position plus one; unexpected shape/accounting is invariantFailure(observerEvent).
5. No event is emitted for value strings, escaped quotes, ordinary intermediate string bytes, skipped number bytes, whitespace in colon mode, or later consumption of the pending key. In particular, checking only next mode colon would emit the same key again on whitespace and is forbidden. Object identity is the opening `{` byte offset stored in the actual frame, not a mutable stack depth or postorder node ID.
6. Retain events in source order, including keys from every overwritten value/object. A temporary reversed list is allowed only with complete final reversal. Walk the entire raw input; no caller fuel or early success. At EOF call actual jsonEndInput and require its successful complete document to equal the first parsed document with actual jsonDocumentEqual. Any mismatch returns invariantFailure(observerDocument). Pending frames, unfinished strings, or nonzero numeric skip must never pass EOF through a separate observer shortcut.
7. `snapshotKeyEvents` returns the complete ordered event list after this check. Strict operations scan that complete list from first to last, remembering prior pairs `(objectStart,decodedKey)`. The first event whose pair was already seen yields duplicateRawKey with that repeated event's objectStart, keyStart, and decodedKey. No duplicate yields the same typed extraction as the compatible operation. The event scan uses actual scalar-list equality, never normalized text or raw escape spelling.

Key strings cannot overlap or contain nested JSON objects. Thus source key-closing order equals source key-opening order, and the first repeated event gives the declared earliest duplicate. Keys in distinct objects may be equal without duplication. `"a"` and `"\u0061"` within one object duplicate; canonically equivalent but unequal scalar sequences remain distinct.

Public admission never consumes a caller-authored JsonState, event list, duplicate-free flag, or successful SnapshotDecoded record as proof. All public strict/observer paths perform the actual complete parse and replay. Low-level helper behavior on fabricated replay states must remain explicit and cannot weaken those entrypoints.

Independent controls must freeze full event lists with exact object/key offsets and decoded keys for empty/multiple/nested objects, arrays containing objects, value strings containing quote/colon text, escaped quotes and keys, whitespace after keys, equal keys in distinct objects, numeric skip paths, and objects later overwritten. Include malformed later input suppressing event/duplicate results, duplicate order across nested objects, exact EOF document agreement, and a source-valid plant that mistakes every colon-state transition for a new event. No expected event list is generated by executing the observer or a host parser.

## 9. Independent frozen-control plan

No expected output below is to be derived by executing the candidate, a host serializer/parser, or a floating oracle. Author literal expected scalars/bytes, independently review against pinned source, then freeze per-module hashes and an inventory before source/RED. Hash fixtures, if added, need separately identified independently available digest evidence; the existing source golden supplies exact bytes, not published digest constants.

Suggested theorem-only partitions:

1. **SnapshotSchemaControls:** complete source snapshot.rs golden, every read kind, both objectives/initializations, exact omission and retained inactive Route gates; zero/max storage dimensions and short arrays accepted despite runtime invalidity; all required fields individually missing, both defaults absent/present/null, every unknown nested field, empty metadata.
2. **SnapshotRepresentationControls:** Snapshot arrays 0..16 lengths and wrong-type earlier elements; exact Spec array4; all ReadSpec array forms plus short/extra/malformed first-tag/content; enum strings/singleton-null maps/empty/multikey maps, known/unknown names and bad payloads; explicit all15 Snapshot positions versus omitted-middle shifts.
3. **SnapshotNumericControls:** each UInt32/UInt64/Int32 endpoint, one outside; -2147483648 predecessor conversion; -1 versus -0; 0.0/1.0/1e0/-1e0/long integer and negative-overflow classes; wrong scalar/container numeric shapes; externally fabricated document class/span rejected by actual revalidation.
4. **SnapshotOrderControls:** present bad value before missing field; sorted-key order independent of raw field order; earlier unknown key versus later bad known value; ReadSpec missing/unknown/wrong tag before bad kept/sweeps; valid tag then content unknown/type/missing order; enum arity before name, name before payload; early positional error before missing/extra; full raw malformed suffix before all typed failures.
5. **SnapshotStoredValueControls:** whitespace/key-order/escape spelling collapse to identical storage bytes; source explicit-default example yields equal typed bytes and different storage bytes; object/positional and string/enum-map alternatives retain distinct storage bytes with same typed value/typed bytes; last-wins duplicate changes only reachable values; discarded finite float allowed, discarded invalid/infinite number parse failure; raw bytes versus compact Value bytes kept distinct.
6. **SnapshotStringControls:** all32 ASCII controls independently frozen, quote/backslash/slash, DEL, BMP/supplementary extrema, U+2028/U+2029, normalization-distinct strings, empty strings, embedded NUL; scalar surrogate/out-of-range rejection in caller typed records; exact UTF-8 byte budgets and escape expansion.
7. **SnapshotStrictControls:** duplicate each schema/nested field; escape-equivalent keys, nested duplicates in overwritten values, equal names in separate objects, multiple repeats and earliest duplicate key position; malformed later input precedes duplicate under the declared parse-first policy; compatible counterpart retains source last-wins success.
8. **SnapshotBudgetControls:** every profile domain endpoint, invalid profile before bad raw byte, parser budgets inherited unchanged, exact output boundaries for storage and typed outputs separately, one domain fits/other does not, no partial pair, empty-list serialization completeness, long complete arrays with chunked literal AST audited by expansion.
9. **SnapshotRoundtripControls:** literal expected typed value → exact bytes; raw valid alternative → typed value → exact canonical typed bytes; native encoding pair versus compatible raw pair from exact native Value; all public entrypoints recheck domains. Finite examples supplement general roundtrip and complete-traversal obligations.

Required well-formed source plants include default-field retention/omission reversal, first-wins replacement, wrong sorted/declaration order, float-to-integer coercion, Int32-min off-by-one, dropped final array element/inactive gates, wrong string escape, metadata hash substitution, alternate-representation normalization in storage bytes, and missed overwritten-subtree duplicate. Each must pass source checking, fail unchanged relevant formal controls, and be followed by restoration/full replay. The existing Json/Number controls remain unchanged.

## 10. Review decisions before implementation

1. Approve the complete Value-compatible surface and separately named strict duplicate policy, including parse-first diagnostic precedence; direct raw typed Serde error-string/cursor parity is not claimed.
2. Approve positional/enum representations and their frozen source-derived controls, including exact ReadSpec tag-first sequence forms.
3. Approve the exact closed constructors, parameter/result payloads, paths and cursor packaging in section 2A, plus the two-field capacity profile and semantic typed-value validation order; larger source-platform capacities remain explicit profiles, not truncation.
4. Approve the actual-jsonStepByte replay observer and its checked public event diagnostic in section 8, preserving the accepted JSON399 source/output contract.
5. Bind exact Serde source/feature profile and source/control inventory. A Cargo.lock package version alone is insufficient production attestation.
6. Keep bare-SHA256 byte-domain composition, artifact runtime admission, authenticity/custody, binary freeze codec, generated instruction/allocation evidence, and Pages deployment as distinct remaining obligations. This codec alone establishes none of them.
