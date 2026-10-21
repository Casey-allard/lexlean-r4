# Snapshot JSON codec controls

The [approved codec contract](specs/snapshot-json-codec-api.md) preserves all
fifteen Snapshot storage fields, full source integer domains, accepted Serde
representations, exact typed and original-Value compact encodings, and a
separately named strict duplicate policy. Storage decoding does not perform
runtime admission or authenticate metadata. The source investigation is retained
in [the codec notes](specs/snapshot-json-codec-notes.md).

The independently reviewed predefinition packet contains 476 frozen controls in
nine modules. `tests/fixtures/snapshot/controls.json` binds their exact bytes.
The original complete review packet has SHA-256
`2b71f6d66998ecd5f9869165c36dbe99c9d54aa82cd7f3f7bf64a4410662968a`.
It remains byte-identical in the retained evidence; the compact repository
inventory records the same nine module identities and counts.

| Partition | Controls |
| --- | ---: |
| Schema | 44 |
| Representations | 74 |
| Numeric domains | 94 |
| Error ordering | 35 |
| Stored Value encodings | 21 |
| Strings | 104 |
| Strict duplicate policy | 47 |
| Budgets | 33 |
| Round trips and complete decoded records | 24 |

Expectations were derived from the pinned source contract without executing a
candidate or reference parser, serializer, model or floating conversion. The
independent review checked complete literal inputs, typed outputs, encoding
bytes, key-event offsets, error precedence and returned-document goldens. The
two complete decoded records retain all nineteen nodes and exact spans, eighteen
edges, twenty-five strings, 158 decoded string bytes and 259 input bytes.

After review, the official development SDK created a fresh lock and rejected
each of the nine roots with `LLR3005`, specifically because `R4.SnapshotJson`
was absent. An earlier `LLC0101` rejection of unsorted entrypoints is retained
as a setup fault; sorting the same nine roots changed no control or dependency.
This is genuine predefinition RED, not a positive codec result.

Evidence is retained under
`/Users/casey.allard/lexlean-r4-evidence/2026-10-21/snapshot-json/`, including the
original packet, independent review, source-absent stage, lock and all nine
diagnostics. Implementation, formal positives, source-valid mutation checks,
restoration and mandatory integration remain required. These controls are not
part of the accepted default graph until those gates complete. Hash-domain
composition, runtime artifact admission, durable custody, generated serving and
browser deployment remain distinct obligations.
