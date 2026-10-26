# Fixed retention verification

The current source passes **35 host unittest methods**. It adds fixed retention
reservations, typed terminal/retrieval encoding and five one-attempt file roles
to the prior journal. This is partial diagnostic source, with no live observer,
compiler invocation or resource acceptance.

Final source SHA256: `2e83f9d63494e5ebec608920c96b1d21f1ad1f50eeae53784538cae08c036362`.
Tests SHA256: `833075e12209a9eed4db3e6b8e61d6f877e886712759fa041cc7358654e189d2`.

`retention-tdd.jsonl.gz` is compressed text containing 22 original named records,
their text, original-file SHA256 and decoded-text SHA256. All rows were compared
with the retained originals. Records preserve intended predecessor failures,
subsequent passes and these separate failures:

- Record 02 has **status 1 despite its green filename**: another old test still
  expected the previous 2048-byte marker reservation. Independent Phase 1 review
  identified that stale expectation; the approved reservation is 26624 bytes.
  The later complete suite passes after that expectation was corrected.
- Record 17 is a coverage instrumentation failure, not a source failure or
  accepted coverage result. Python 3.15's trace helper includes non-source
  `None` and zero entries. Independent debugging reproduced the issue. Records
  18 and 21 explicitly exclude those entries; the final run also validates key
  kinds and source bounds.
- The first design was rejected for absent unknown-quota representation and
  inconsistent retrieval reasons. Both the original and rejection are preserved.
  The reviewed v2 permits wholly unknown quota snapshots and constrains named
  reasons without inferring provenance from supplied flags.
- Python review requested public signature annotations. Its original report
  remains preserved. The narrow follow-up independently recovered the exact
  preceding source and confirmed annotation-stripped AST equality, then ran the
  final 35 tests. This was a profile-policy correction, not a behavior defect.

On the final source, nine separately planted defects were rejected: reduced fixed
reservation, invented zero quota snapshot, repeat attempts, premature sync
success, contradictory retrieval reason, overlong capture, premature decimal
copy, false extent certainty and a guest claiming its future exit. These are
in-memory source variants; no repository or native source was mutated by them.

Final trace-derived **positive source-line coverage is 352/364 (96.70%)**. The
raw key census has 366 entries, including the disclosed `None` and zero. Branch
coverage was not measured. Annotation/source-position entries affect this line
metric; no branch or whole-observer metric is inferred.

An initial allocation regression probe used a 1 MiB in-memory decimal input;
its retained failure was repeated with a 131072-byte input before the fix.
That smaller predecessor run allocated a traced peak of 133601 bytes during the
call; the final test rejects before making the encoded copy and stays below its
65536-byte assertion. The input is created before tracing. This is a host
allocation test, not a total Python RSS or live observer memory bound. No such
large input was written as a filesystem fixture.

Independent CODE and security reports approve the pre-annotation behavior;
Python's follow-up verifies the final annotation-only change. Source review
probes include 216 retrieval status/reason/completeness combinations, hostile
subclasses, an 11119-byte receipt carrying maximum-size binary captures, combined pool
saturation, synchronization interruption and preservation of original errors.
Exact-delivery/head review remains a separate delivery gate.

All approvals retain the trusted caller-owned stream/synchronization contract.
Actual role/path ownership, control replacement, bounded process capture,
source/identity samples, clock/ACK linkage, observer composition and the separate
native pre-run gate remain open. `max=6325` stays resource-unaccepted; no SDK,
VM restart, counter reset, resource change or production binding occurred here.
