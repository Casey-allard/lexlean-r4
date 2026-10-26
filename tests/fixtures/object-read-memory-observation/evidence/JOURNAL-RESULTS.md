# Partial host journal verification

The append/synchronization primitive and existing parsers passed all **26 host
unittest methods**, without skips. Scope is the [journal contract](../JOURNAL.md),
not an executable observer or full plan step 3. The original admitted-range
artifact remains resource-unaccepted at `memory.events.max=6325`.

Source SHA256: `abf04baddad7e91dae38a3fce340b72d787bca7676a3ea373ddced719dea505b`.
Tests SHA256: `7ac370aa834225b3cf539086279a84b8bc2637a65cb3e807ea1b015e94dd21ed`.

`journal-tdd.jsonl.gz` preserves all 18 immutable external result records as UTF-8
JSON text, with original compressed and decompressed hashes. Seven observed
RED/GREEN pairs cover the API, short writes, uncertain partial failures, invalid
write counts, synchronization failures, row envelopes and quota boundaries.
Records 15–16 are additional regression/final checks, not newly claimed REDs.
The existing 16 parser tests remain in the final 26-test suite.

Eight separately planted source defects were each rejected: dropped short-write
suffix, disabled quota, omitted row reservation, Unicode character undercharging,
accepted false synchronization, premature sync bound, failure without stopping,
and missing terminal reserve. These were in-memory source variants; the actual
source file was not changed. Their mutations and outcomes are in record 17.

Record 17's zero-line coverage is an **instrumentation failure**, preserved as
such. The independent Python reviewer, acting under the eligible debugger role,
identified the missing `__file__` on the in-memory module: stdlib Trace excludes
that module. Record 18 sets the exact source filename and traces initialization
plus tests, with the same source/test hashes. It measures **187/188 executable
lines (99.47%)**; the uncovered line is the multiline return annotation. This is
line coverage, not branch coverage. No coverage package was installed.

A constant-space sink accepting 24 reused 32768-byte slow rows measured a host
`tracemalloc` peak of 69097 bytes. The row was allocated before tracing. This is a
single host allocation probe, excluding interpreter baseline and row construction;
it is not a guest RSS or whole-observer memory bound.

The independent CODE, Python and security reports are preserved alongside this
record. Their scope excludes full observation field validation, depth/object
limits, the refined root-marker/copy/retrieval ledger, terminal serialization,
ACK linkage, filesystem ownership and durability. Deep valid JSON and exponent
`1e9999` can pass the envelope layer; the future typed composer must reject fields
that violate its schema. No sampling may begin before those obligations pass.

No SDK run, guest observer, VM restart, process scan, resource limit change,
counter reset or control promotion occurred. Unique prior failures and all nine
pending product drafts are preserved. Continue the remaining journal lifecycle
accounting and terminal writer, then identity/clock composition and separately
reviewed invocation work; the full LexLean/PrismPM/Pages objective remains open.
