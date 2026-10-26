# Partial host reader verification

The seven parser/read functions in `observe.py` passed the 16 host unittest
methods in `test_observe.py`. This accepts bounded parsing on the tested inputs;
it does not establish a live observer, resource acceptance or a LexLean proof.

Source SHA256: `ec319b4808228dae4ad4dca71cf01a526aca4b5bb6da2fc16c5ac86dc06f507e`.
Tests SHA256: `48d1b0a2bd93e74258c0879b8d7796109c4a52bc0d0f9673872aab85d59c9b97`.

`host-tdd.jsonl.gz` is compressed text, not an executable artifact. Its 24 JSON
rows contain the original named RED/GREEN record text and SHA256. Decompression
and comparison with all 24 original files passed. Records retain source/test
snapshots, return statuses and output. API-absence failures are distinguished
from boundary tests that reject the preceding minimal implementation. The final
GREEN passed all 16 tests without skips.

Independent partial CODE, Python and security reports are included byte-for-byte.
They reproduced all 16 tests and added respectively 12, 17 and six groups of
in-memory checks. Their conclusions exclude path acquisition, ownership/races,
ancestry, persistence, complete memory accounting and live execution. The CODE
report preserves its first interpreter-selection failure under default Python
3.9 before rerunning successfully with the explicit host Python 3.15 binary.

The first line-coverage report failed while sorting Python 3.15's non-source
`None`/zero entries. Its failure and independent debugger diagnosis are retained.
The corrected fresh report validated and excluded those sentinels, then measured
**106 of 116 executable source lines (91.4%)**. Imports/function definitions
executed before tracing account for the ten uncovered lines. Branch coverage is
unavailable; no package was installed and no branch percentage is claimed.

Three synthetic single-call `tracemalloc` probes measured peaks of 49,531 bytes
for a cap-sized read, 148,055 bytes for a status record with many unknown fields,
and 24,190 bytes for a long stat record. These are host Python allocations for
those calls, **not** guest RSS, whole-observer bounds or runtime overhead evidence.

No SDK phase, process observation, VM/container restart, resource-limit change or
counter reset occurred during these tests. The original compiler artifact remains
kernel-verified with an **unaccepted** resource outcome (`max=6325`). The remaining
diagnostic implementation and its independent execution gates remain open.
