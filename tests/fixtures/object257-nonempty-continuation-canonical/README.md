# Canonical serialization repair: nonempty continuation

This source-only successor preserves the exact proof and runtime semantics of
the [original candidate](../object257-nonempty-continuation/README.md).
**New native source checks and proofs are NOT_RUN.**

The original negative phase stopped at source admission with `LLT4001`:
`phase link: semantic-module JSON is not canonical`. Verification did not run;
this is not the intended unrelated-base negative. The original frozen fixture,
native stage and failure evidence remain unchanged. The actual check receipt and
empty postcapture-error record are copied into this successor's `evidence/`.

`repair.json` traces the failure to the official parser's exact canonical-byte
comparison. Its canonical serializer orders object keys by UTF-8 bytes. Three
new K match objects had insertion-order keys instead: the match body and its two
branches. This repair orders those keys without changing any parsed value,
array, field, declaration, theorem, proof or module header. Both consumer modules
and both phase configurations are byte-identical to the original candidate.
There is no evidence for a missing/default schema field in this failure.

The existing structural audit checked semantics but omitted serialization order.
The successor adds a source comparison of canonical bytes and exact parsed AST
equality. That comparison passes locally; it does not substitute for an actual
official compiler check or establish proof success.

`candidate.json` preserves all 17 shared stage input mappings and selects this
successor's three modules in `newStageInputs`. Use a new independently owned
stage; never overwrite the failed original stage. Select the corresponding phase
config as `lexlean.toml`, rehash every input, and generate a genuine development
lock through the installed official compiler. No lock is fabricated here.

| Phase | Source check | Verification |
| --- | --- | --- |
| `continuation-false` | `lexlean --diagnostic-format json check tests/lexlean/Object257NonemptyContinuationFalseBase.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257NonemptyContinuationFalseBase.lex.tex` |
| `continuation-positive` | `lexlean --diagnostic-format json check tests/lexlean/Object257NonemptyContinuation.lex.tex` | `lexlean --diagnostic-format json verify tests/lexlean/Object257NonemptyContinuation.lex.tex` |

Run only after independent source review and fresh resource/identity checks.
The 300,000 ms limit, source/tool/epoch/storage checks and complete postcapture
obligations remain unchanged. Negative success requires rejection of the zero-base
Apply to unrelated `0 = 0`; parser/import/resource failures do not qualify.
Positive success requires the actual `Fold[A,B][][]L = Fold[B]S.frontier
S.inventory L` consumer, not only the K equality. Nested-match conversion and
its resource cost remain unresolved. No automatic limit increase or equivalent
retry is implied. Whole257, complete Hash/Object qualification, PrismPM and Pages
acceptance remain open.
