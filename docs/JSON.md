# Raw tokenizer JSON conformance

Status: complete source definitions are independently reviewed; all 399 frozen
controls passed formal verification before and after nine detected semantic
defects were planted in isolated copies. The mandatory integrated Rust gate and
the complete 17-component formal graph passed. This record does not claim an
accepted runtime.
The complete interface is [the JSON contract](specs/tokenizer-json-api.md).

The twelve control modules and their exact byte hashes are recorded in
[`json-controls.json`](../tests/fixtures/tokenizer/json-controls.json), SHA-256
`2fdee4bcee209f5442cab4c6e510b42bea14063c68df6ed82b34e4161e716f12`.
Their independent source review covered syntax and first-error cursors,
surrogate and UTF-8 timing, depth limits, duplicate-key replacement, complete
postorder descriptions and ordered tokenizer extraction. A separate static
audit checked every theorem and literal input, including 90 expected parsed
documents containing 747 nodes. Literal list chunks are only a source
representation: flattening preserves every byte and node.

Before either new source module existed, each of the twelve frozen roots was
checked using the unchanged official language-1.2 SDK after successful lock
creation. Every check rejected the absent `R4.Json` import with `LLR3005`.
The initial directory-permission and missing-lock setup failure was retained
separately and was not counted as this failing-test evidence.

The named profile is serde_json 1.0.150 with `std`, excluding `float_roundtrip`,
`arbitrary_precision`, `preserve_order`, `raw_value` and `unbounded_depth`.
All ignored or overwritten values must be parsed completely before extraction.
The independently frozen 608 number controls remain unchanged. No host parser,
floating-point oracle, model evaluation or reference tokenizer supplied the
expected results.

The implementation consists of 67 declarations in
[`R4.Json`](../src/R4/Json.lex.tex), source SHA-256
`b18031a8626363bfcfb6799424e1527da77fc8b2131a1ebfa75d6390a9da16fe`,
and 43 declarations in
[`R4.TokenizerJson`](../src/R4/TokenizerJson.lex.tex), source SHA-256
`30b0497fc7afb40ed8919f02ebb67be64673d8129e61247cf9086a706808fd9a`.
Independent specification review followed by code and security review passed
at these exact source identities. The official SDK's source checks and complete
selected-control verification passed for both modules, with the same source
identities retained after every mutation and the final restored replay.

The parser advances structurally through the complete input list. Its explicit
continuation stores pending containers, lexical mode, postorder nodes and
resource accounting. It has no caller-supplied fuel that could accept a prefix
as a complete document. Number scanning calls the admitted exact number
component and records its logical endpoint immediately; subsequent input steps
consume precisely the already-scanned numeric bytes. A document cannot finish
with pending numeric bytes or containers.

String decoding retains source error order: escape and control-byte failures
occur during scanning, and strict UTF-8 decoding occurs on the closing quote.
Decoded keys are ordered lexicographically and later equal keys replace earlier
bindings. Every parsed value remains in the postorder tape, including values
overwritten by duplicate keys. String and value budgets count those occurrences.
`jsonCheckDocument` reparses the raw bytes and compares every supplied document
field, node, binding and reference against the reconstructed document.

Tokenizer extraction operates on that freshly parsed document. It preserves
the source's staged ordering across model type, vocabulary, added tokens,
final vocabulary size, merges and pre-tokenizers. Each stage invokes the actual
admitted tokenizer-kernel operations where source semantics require them. The
final preparation contains the exact raw bytes, extracted kernel input and
configuration returned by the complete `kernelAdmit` call. Internal reference
helpers do not establish admission for arbitrary externally supplied documents.

All twelve selected control roots passed actual verification under the official
LexLean 0865df4e SDK, language 1.2 and Lean 4.32.1, with unchanged limits and
none-axiom policy. Receipt audits checked each frozen theorem name, all declared
source inputs, exact configuration and lock bytes, build/semantic/source IDs,
the language-bound attestation/2 schema and each independent kernel replay.
The graph contains 399 control declarations and 329 implementation declarations;
the twelve selections produced 3,083 audited declaration rows and 80 module
replays in each complete baseline/restored pass. Every observed axiom list was
empty and every process succeeded. `JsonAdditionalControls` and the three
`TokenizerJson*` roots require eight modules each; the other eight roots require
six each. An independent recursive import audit checks this exact distinction.

The nine isolated variants below each passed source checking and then failed
formal verification with `LLV7002` against unchanged, independently authored
controls. None was counted from a syntax or type failure. The original source
was restored after each variant; the final complete 399-control replay and
receipt audit passed at the original reviewed source identities.

| Planted defect | Frozen detecting fixture | Failing diagnostics |
| --- | --- | ---: |
| Malformed literal bytes accepted, including ignored metadata | TokenizerJsonOrderControls | 2 |
| First duplicate decoded key wins | JsonDuplicateControls | 3 |
| Premature UTF-8 validation before escape processing | JsonStringBoundaryControls | 33 |
| One extra container depth admitted | JsonDepthControls | 2 |
| Floating-representation IDs coerced to zero | TokenizerJsonErrorControls | 7 |
| Model-type validation deferred past its required stage | TokenizerJsonOrderControls | 1 |
| Final kernel receives corrupted merge input | TokenizerJsonAdmissionControls | 2 |
| Complete document comparison omitted | JsonDocumentControls | 14 |
| Numeric skipping consumes one extra byte | JsonContainerControls | 12 |

The first verification attempt encountered `LLV7001` when Lake's initial
manifest-creation messages polluted the tool identity probe. The new manifest
then required an isolated lock refresh (`LLC0102`). These setup faults are
retained separately; neither is a semantic negative. No source, expected
result, proof policy, recursion limit or timeout changed to obtain the passes.

The new mandatory target
[`tokenizer_json.rs`](../crates/conformance/tests/tokenizer_json.rs) binds the
exact inventory and delegates all twelve fixtures to the shared verifier.
Its corrected exact module mapping passed independent Rust/security review.
The required build guard is registered. Actual Cargo execution passed both
the 399-control target and its receipt-language binding check, with no ignored
or filtered tests. A second execution in a separate Cargo target directory
passed in 153.97 seconds after a generator path-binding fault was found.

The complete integrated graph passed with 8,270 controls across 98 modules and
72 roots. Its attestation is
`c5655eb0948e913077cc6e51172a09f90bc0baef586a4b672e45b817302a3e00`.
Independent audit checked all 10,419 declaration rows under the unchanged
none-axiom policy, 98 kernel replays and 100 exact current input bindings. A
separate output audit checked all 490 build outputs and 98 compiled module
files against their recorded hashes and lengths. The configuration hash is
`3dcf1d812591ac582fce96a82d3a4a01115b0f92bac11d4945b439e6d7c88671`;
the freshly generated lock hash is
`f5fe85b3953a0414c6e0018cf286bcc0b25239d15d41e0ae555eacd8b061aa7a`.

Model generation and comparison, formatting, lint and all-feature checks
passed in the exact JSON integration stage. An earlier shared Cargo target
reused a model generator whose compile-time repository root pointed to the
previous sixteen-component stage. That output was excluded. A fresh, separate
target generated and checked the correct seventeen-component document. An
independent audit of 73 actual formal receipts confirmed their source/config/
lock bindings; the generator fault did not change those proof inputs.
The full 54-test workspace run belongs to the preceding sixteen-component
checkpoint; the JSON integration adds its two executed target tests and the
complete 98-module formal run, not a claimed new whole-workspace test run.

Bounded evidence is retained outside the repository at
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/json/`, including source and
review inputs, the frozen fixtures, actual receipts/manifests, setup faults,
source-valid mutation logs and restoration records. Retention manifests bind
the copied bytes; no SDK archives or compiled Lake object trees are required
to understand this result.

The unchanged `just model` and `just vv` commands checked the generated model
and then failed at the missing production `prismpm.lock`. Development component
checks passed; full template acceptance remains blocked by that dependency.
Raw-byte BLAKE3 identity, production lowering, full PrismPM acceptance and Pages
deployment are separate required work.
