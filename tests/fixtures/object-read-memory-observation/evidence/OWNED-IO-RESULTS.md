# Owned-file acquisition host results

Scope: partial step 4 of the one-off admitted-range memory diagnostic. No live
reader, epoch binding, ACK issuance, invocation, SDK execution or product proof.
The previous `memory.events.max=6325` resource outcome remains unaccepted.

- Final source SHA-256: `5950e324cdcc642e7c8a72bb4dc9cdb476d234813f5cddb26166de6f230b2920`.
- Focused test SHA-256: `366e66d6b657de84c09199c029464d03885e86b4bf4452584f89191cd302ad57`.
- **64 full-suite tests passed** on existing host Python 3.15: 46 prior tests
  unchanged and 18 owned-file integration/fault tests. No package was installed.
- Eleven observed RED/GREEN pairs precede final boundary coverage. Missing-function
  REDs cover initial acquisition/file/children/snapshot entry points; actual
  predecessor defects cover intermediate symlink following, invalid requests,
  error masking, FIFO/directory handling and post-read identity drift. Snapshot
  RED initially called six positional arguments; the final API groups each root
  fd and pin as a tuple. Its behavioral fixture is unchanged. This small API
  change is disclosed in the corresponding GREEN record.
- Nine separately planted in-memory source defects were rejected by the 18
  focused tests: following intermediate aliases, omitting pin checks, suppressing
  cleanup errors, relaxing file caps/types, omitting the post-read check, allowing
  65 children, allowing directories/symlinks and opening an unlisted PID. They
  are synthetic defects, not LexLean semantic negatives or historical native runs.
- Measured host positive source-line coverage: **466/549 = 84.88%**. Import occurs
  before tracing; declaration/annotation lines remain in the denominator. Python
  trace keys are validated, and only `None` and line zero sentinels are excluded.
  Branch coverage is **NOT_MEASURED**; this line result does not establish it.

Independent reviews bind the exact source/test hashes:

- [CODE](owned-io-code-review.json): 29 focused owned-IO/identity tests plus six
  independent adversarial tests passed, including falsey/nested errors, cleanup
  interruptions, actual synthetic PID reuse and symlink rejection.
- [Python](owned-io-python-review.json): 18 owned-IO tests passed on host Python
  3.15 and host Python 3.12.14; four additional probes passed. The guest was not
  used. The required minimum Python version is now documented as **3.11**, since
  compound failure retention uses `BaseExceptionGroup`.
- [Security](owned-io-security-review.json): 18 owned-IO tests and five additional
  probes passed, including concurrent child insertion and pathname replacement.

CODE/Python reports preserve their initial default-interpreter import failures:
bare `python3` selected Apple Python 3.9 and rejected existing union annotations
before tests. Explicit existing compatible interpreters then ran the tests.
No source repair or tool installation was needed for that command-selection
failure. Host tests do not establish Linux guest proc/cgroup semantics.
The [separate debugger investigation](owned-io-interpreter-debugger.json)
independently reproduced the same failure at a pre-existing annotation and
successful imports with explicit host 3.15 and 3.12. It occurred after the
reviewers' reruns; no retroactive debugging-gate chronology is claimed.

The [source-only acquisition decision](owned-io-adr.json) is retained unchanged;
its external paths and recommended status describe the historical architecture
review. The implemented subset and remaining caller obligations are documented
in [OWNED-IO.md](../OWNED-IO.md).

`owned-io-tdd.jsonl.gz` retains 26 exact original JSON record texts, each with
byte count and SHA-256; compressed SHA-256 is
`c7010e7e127b4aed3dc737b9a4d37fa280b881b999d8119197519016c9546760`.
Intermediate records pin their then-current source but do not bundle every
intermediate body. No stronger historical source reconstruction is claimed.
Final source is delivered as ordinary reviewable text. Reproducible temporary
test fixtures are removed by their owners; all unique official evidence remains.

Run from the repository with an existing compatible interpreter:

```sh
/opt/homebrew/bin/python3 -B -m unittest discover \
  -s tests/fixtures/object-read-memory-observation -v
```

Preparation used bounded host fixtures and no build target. A sampled allocation
check found 233,472 bytes across then-current new source/evidence files; that is
not continuous high-water measurement or live observer RSS. Host available space
fell during the turn; a bounded read-only check did not fully attribute the
decline. The recorded final check still had about 38.2 GiB free with a 10 GiB
reserve; no other work was changed or deleted. Later native work retains the
separate fresh storage, epoch, source, invocation and immediate-zero gates.

Parent/root attribution, control replacement and ACK linkage, bounded failure
serialization, observation boundary rewalks and reader composition remain open.
No continuous filesystem identity, atomic snapshot, complete live capture,
operational proof acceptance, production binding or Pages result follows.
