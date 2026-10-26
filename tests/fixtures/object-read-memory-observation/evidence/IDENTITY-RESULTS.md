# Identity and clock helper verification

Scope: partial step 4 of the one-off admitted-range diagnostic. No live reader,
SDK run, path/epoch custody, root attribution, ACK issuance or new product proof.
The previous resource outcome remains unaccepted at `memory.events.max=6325`.

- Fresh host discovery: **46 tests passed** on existing Python 3.15.0; 35 existing
  tests remain unchanged, with 11 new focused tests in `test_identity.py`.
- Seven observed RED/GREEN pairs cover missing snapshot behavior, raw-reader
  bounds, rejection before reads, interval rounding/invalid values and absolute
  scheduling/invalid values. The first REDs intentionally include missing-function
  errors; bounds/type/schedule negatives then exercise actual predecessor bodies.
- Fourteen separately planted in-memory defects were rejected by the focused
  tests. They weaken census, identity/membership/status, raw bounds or clock
  predicates. They are synthetic mutation tests, not LexLean semantic negatives.
- Independent source reviews passed: [CODE](identity-code-review.json),
  [security](identity-security-review.json), [Python](identity-python-review.json).
  Each reran 46 tests and respectively 433, 334 and 166 additional bounded probes.
- Final measured positive source-line coverage: **352/408 = 86.27%**. New helpers:
  **40/44** reported executable lines. The four uncovered helper lines are
  function declaration/annotation lines outside the traced test execution.
  The module was imported before tracing. This denominator includes declarations;
  branch coverage is **NOT_MEASURED**. Guest Python 3.12 execution is **NOT_RUN**;
  Python review's 3.12 AST parsing is syntax evidence only.

Run from the repository with the existing installed compatible interpreter:

```sh
/opt/homebrew/bin/python3 -B -m unittest discover \
  -s tests/fixtures/object-read-memory-observation -v
```

The compressed `identity-tdd.jsonl.gz` retains original record text, size and
SHA-256. Early records preserve outputs without all intermediate source pins;
only the final source and explicitly pinned records carry exact source identity.
No stronger byte-for-byte historical source reconstruction is claimed.

Preserved failures: the first coverage instrument failed sorting `None` against
integers. Python 3.15's stdlib trace includes `None` and line zero in its executable
line map. Separate Phase1 investigation diagnosed the measurement failure. The
corrected measurement validates all keys, explicitly excludes only `None`/zero,
and uses exact positive integer source lines. This was not a reader-code failure.
CODE/security reviewers also preserve initial system Python 3.9 import failures
at pre-existing union annotations before their explicit Python 3.15 reruns.
Nothing was installed or deleted.

These synthetic tests do not establish bounded physical memory, cross-process
ownership, snapshot atomicity or absence of unobserved ABA changes. The caller
must still provide owned paths/census, bounded reads, unchanged epoch and a common
guest clock. Parent/root linkage, descendant detection, controls and reader
composition remain required before an independently reviewed native diagnostic.
