# Raw tokenizer JSON conformance

Status: contract and 399 independent controls frozen before implementation.
No raw parser implementation or accepted runtime is claimed by this record.
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

Required acceptance remains complete pure LexLean definitions, all frozen
controls, representative source-valid defects, restored formal replay and
mandatory receipt/source checks. Raw-byte BLAKE3 identity, production lowering,
full PrismPM acceptance and Pages deployment are separate required work.
