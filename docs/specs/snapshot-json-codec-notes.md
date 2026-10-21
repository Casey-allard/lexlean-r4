# Snapshot JSON codec source-contract notes

Status: source-only refinement, 2026-10-21. This is not a registered codec API,
accepted implementation, serialization execution, or model result. The raw JSON
399-control verification remains the immediate implementation priority. These
notes refine [artifact-serving-design.md](artifact-serving-design.md), preserving
its distinction between storage metadata and runtime admission.

## Authority and three entry surfaces

UOR source is revision `4611538bb4843947363ccb9cce5f388e32c46883`.
`crates/uor-r4-discrete/src/snapshot.rs` SHA-256 is
`c03dcb7bc0e0e966951285a52cf76bf672ee74bf136c42403b40d1a4b5da1170`.
Cargo.lock lines 1743–1794 pins serde/serde_derive 1.0.228 and serde_json 1.0.150.
Source line references below refer to these exact versions.

1. **Typed serialization:** `snapshot_hash_domains` first converts a typed
   Snapshot to Value (snapshot.rs50–53), then uses the stored-Value entrypoint.
2. **Stored-Value hashing:** `stored_snapshot_hash_domains` first deserializes a
   clone of the supplied Value into Snapshot, then hashes the original Value's
   compact bytes and the typed Snapshot's compact bytes separately
   (snapshot.rs55–67). Failure to deserialize returns no successful domain pair.
3. **Proposed strict raw-byte import:** the serving design proposes rejection
   of duplicate keys and bounded complete JSON parsing before typed admission.
   That is a new interface policy. It must not be labeled identical to either
   direct `from_slice::<Snapshot>` or the existing Value-taking function.

Value construction inserts object entries into a map with later equal keys
replacing earlier entries (serde_json value/de.rs121–142). With
`preserve_order` disabled, that map is a BTreeMap (map.rs1–35). Once a Value has
been constructed, original duplicate occurrences, whitespace and spelling of
escapes are unavailable. The raw parser's postorder tape can preserve evidence
needed for a new duplicate-rejecting boundary, but the surviving object bindings
alone cannot.

## Typed fields, defaults and semantic boundaries

Snapshot's fifteen fields and declared serialization order are in
snapshot.rs10–35: schema, spec, read_spec, objective_policy,
read_initialization, seed, epoch, table, routes, gates, operator_kappa,
composition_kappa, kernel_sha256, source_sha256, geometry_sha256.
Only objective_policy and read_initialization have defaults; only their default
variants are omitted on typed serialization. Explicit null is not absence.
There is no default read_spec, gate inventory, schema migration or generic
unknown-field allowance.

Spec contains four UInt32 fields in vocabulary/blocks/context/kept_cells order
(lib.rs55–62). Seed and routes are UInt32, epoch UInt64, table and gates Int32.
These domains belong to typed deserialization. Spec.validate, ReadSpec.validate,
initialization compatibility, schema/hash checks and table/root/gate inventory
checks are later runtime admission obligations, not prerequisites enforced by
the storage hash function. The source's own serialization golden uses a table
and route inventory too short for runtime admission (snapshot.rs76–121).
Similarly, stored schema/kappa/hash strings are not validated as hexadecimal,
compiled identities or trusted provenance by this function.

ReadSpec uses a custom internally tagged Wire enum with deny_unknown_fields
(lib.rs115–136). Route is an empty struct variant, making its extra map fields
invalid. Flock requires kept; Relation requires kept and sweeps. Initialization
names are zero_gates_v1/active_leaf_v1 (lib.rs82–88); objective names are
nearest_address_margin_v1/occupied_cell_rank_hinge_v1 (objective.rs12–18).

## Duplicate, unknown and failure order distinctions

Derived struct map visitors consume entries in the deserializer's iteration
order, reject a duplicate recognized field before reading its repeated value,
and only after exhausting the map resolve missing/defaulted fields in declaration
order (serde_derive de/struct_.rs250–327). Unknown fields are rejected by the
field-identifier visitor; deny_unknown_fields supplies no ignored-value arm.
For `from_value`, map order is sorted decoded keys under the stated feature
profile. For direct raw typed deserialization, map order is input order and an
early field failure may precede malformed later JSON. A complete Value-first
parser must parse the whole JSON before this typed stage. These are different
error contracts; a new deterministic typed error order must name its surface.

ReadSpec first gathers a tag and content; a second kind field is rejected while
gathering, absent kind is reported after collection, and variant content is then
deserialized (serde private/de.rs816–889). Do not flatten these phases into
Snapshot's declared field order or assume all shape errors precede every nested
semantic error. Raw duplicate detection must compare decoded keys, including
escaped equivalent spellings, before Value replacement.

The exact enum and struct acceptance surface is broader than canonical output:
serde_json Value deserialization accepts both arrays and objects for structs
(value/de.rs482–495), and enum maps with exactly one key as well as variant
strings (234–265,328–340). Unit-variant map payloads deserialize as unit
(556–560), permitting the null payload form. Thus a complete source-compatible
decoder must consider positional Snapshot/Spec arrays and forms such as
`{"zero_gates_v1":null}`. Typed serialization still emits objects and variant
strings. TaggedContentVisitor also accepts a sequence whose first element is
the tag (serde private/de.rs845–856); generated internally tagged struct visitors
retain sequence handling (serde_derive de/struct_.rs72–96,154–156). This indicates
additional ReadSpec sequence forms and requires independent literal controls
before their exact accepted domain and errors are frozen. An object-only/string-
enum-only codec must explicitly be a strict refinement instead.

## Exact compact byte output

Both successful hash domains use `serde_json::to_vec`, not pretty output or raw
file bytes. CompactFormatter contributes no whitespace (ser.rs1940–1944), object
separators are comma/colon (1882–1916), and to_vec delegates to to_writer
(2213–2221). No trailing newline or BOM is added.

Typed output has declared field order, Spec's declared subfield order and
ReadSpec's kind/kept/sweeps order. Defaulted policies disappear. Stored-Value
output recursively preserves array order and sorts object keys by decoded
String order under the BTreeMap profile (map.rs493–503). Explicit defaults and
alternative admitted struct/enum representations remain in that original Value
and therefore can change storage bytes while leaving typed bytes unchanged.

Strings are quoted UTF-8 with exact escaping (ser.rs1757–1801,2079–2165): quote
and backslash are escaped; U+0008/U+0009/U+000A/U+000C/U+000D use b/t/n/f/r;
other ASCII controls use lowercase `\u00xx`. Slash is not escaped by the actual
escape table. Non-ASCII scalars, DEL, U+2028/U+2029 and HTML punctuation are
emitted as UTF-8, with no Unicode normalization. Typed integer output is plain
base-ten with a minus sign only for negative signed values, no leading plus,
leading zeros, fraction or exponent. Array elements use these same rules and
retain complete order. Signed Int32 minimum needs its exact magnitude codec;
it cannot be negated in an Int32 intermediate.

The accepted source Snapshot fields contain only exact integer numeric domains.
A general stored-Value serializer would additionally need serde_json's exact
floating number representation and formatting; R4.Json's finiteOther class
deliberately does not retain that information. It also loses the original
magnitude of negative integer lexemes that took the source floating fallback;
it is not a complete signed-number representation. The document retains each
numeric span and the raw input remains available. Any additional typed signed
decoder must reconstruct and classify the exact span, or introduce a separately
reviewed observation extension where the existing class lacks the required
information. Therefore do not claim arbitrary Value serialization from the
observation tape. A Snapshot-refined serializer
must establish that all surviving values belong to its complete admitted schema
before omitting this otherwise required floating serialization dependency.

## Required contract decisions and controls

- Freeze whether the next API is exact Value-to-Snapshot compatibility, strict
  raw import, or both separately named operations. Rejecting duplicates and
  alternative Serde representations is an explicit strict policy.
- Pin effective Cargo feature closure, not just package version. Confirm
  preserve_order, arbitrary_precision, raw_value, float_roundtrip and
  unbounded_depth for the actual storage consumer. The current inspection uses
  the same default-profile assumptions as the approved JSON contract; it is
  not a measured build-feature attestation.
- Specify typed errors independently from Serde's human-readable error strings
  unless exact string/cursor reproduction is separately modeled. Include
  encounter-order field errors, post-map missing fields, internal tag phases,
  duplicate decoded keys and parse-before-extraction distinctions.
- Freeze independent complete bytes for all three ReadSpec modes, both policy
  values and explicit/omitted defaults, positional and tagged alternatives if
  compatibility includes them, nested sorted maps, every control escape, UTF-8
  boundaries and UInt32/UInt64/Int32 endpoints. Include unknown/missing/duplicate
  fields, null defaults, wrong numeric classes, extra sequence elements and
  incompatible runtime values that nevertheless deserialize as storage types.
- Require checked output budgets and complete serialization; no truncation,
  implicit reordering of arrays, dropped inactive gates, fabricated compiled
  identities or acceptance based solely on matching content hashes.

Additional inspected source SHA-256 pins: serde_json ser.rs
`f2b8cc0b97e30c49dbc58698fa22c14a7d56ea616cdb57b067835a59af49a75f`;
serde_json value/de.rs
`ceda1bf415aafde89caa83bf275811b70d393717d4e1038e0c61822d2da02993`;
serde_derive de/struct_.rs
`7b26982ce59504a9bb5a18c3ab56eee2d75e70fd465774ff035d15c50452c118`;
serde private/de.rs
`68ee17774d3bbc693d9f19803747fa2c3afdb7046482e6d94ac6cf7db5a4fe20`.
