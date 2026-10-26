# One admitted-range memory observation

This is diagnostic tooling for the actual LexLean admitted-range proof, not an
application implementation or another verification framework. The full pure
LexLean/PrismPM/Pages objective and all remaining controls stay open. The earlier
compiler artifact remains kernel-verified and its resource outcome remains
unaccepted at `memory.events.max=6325`.

## Authority and boundaries

The [reviewed source design](../continuation/2026-10-25/read-admitted-memory-diagnostic/README.md)
permits concretization only. The architecture decision must be frozen and read
before implementation. This plan supplies no executable resource epoch or SDK
launch authority. Source preparation is covered by the user's existing full-scope
authorization and autonomous top-session review; a new native phase still needs
independent source, invocation and fresh-resource review.

Use the existing guest runtime selected by that decision; install nothing in the
VM. Any Python is diagnostic-only, outside the measured container and outside the
LexLean application/SDK. Preserve existing host command capture and independent
postguards. No source, digest, axiom, timeout or resource-bar change; no equivalent
qualification retry. The diagnostic may run at most once after its separate gates.

## Files and bounded sequence

All new diagnostic source belongs under
`tests/fixtures/object-read-memory-observation/`. Preserve the nine pending drafts,
old fixtures, old epochs and all unique failure evidence. Source, synthetic tests,
temporary copies and test reports together must remain below 1 MiB during host
preparation. Do not create a build target, install packages or run the SDK.

1. **`README.md`, `bindings.json`:** record exact proposal, SPEC, observation
   contract, architecture decision and capability identities. Keep the live epoch
   null. Distinguish actual capability results from synthetic byte scenarios.
   Dependency: frozen architecture decision. Risk: low; no native authority.
2. **`test_observe.py`, then `observe.py`:** implement finite numeric and proc-file
   decoding, with one observed RED before each behavior's implementation. Reject
   overlong input, malformed keys, PID reuse and incomplete mandatory readings;
   preserve unknown valid counter keys. Keep individual changes/functions small
   (at most 50 lines per implementation step). Risk: medium, because silently
   substituting zero or stale identity would corrupt the diagnostic.
3. **Same two files:** test and implement the bounded journal, separate quota
   pools, partial-write accounting, terminal reserve and synchronization records.
   Retain complete prefixes; no ring overwrite. Fault tests cover short writes,
   quota exhaustion and failed synchronization. Dependency: step 2. Risk: medium.
4. **`observe.py`, `boundary.py`, `test_identity.py`, `test_owned_io.py`,
   `test_family.py`, `test_boundary.py` (with existing test discovery):** test and implement owned-cgroup census and double-read
   identity checks, common guest-clock samples, missed ticks and marker/ACK
   validation. Processes without proven invocation ancestry remain explicitly
   unattributed. Use captured records only as fixtures, never invented measured
   process histories. Dependency: steps 2–3. Risk: high; races and coverage gaps
   must remain visible. The separate focused test file keeps the original
   668-line test suite unchanged and below the review file-size limit. Owned
   filesystem fixtures use the second focused file; the descriptor acquisition
   decision is preserved in `OWNED-IO.md`. Compound cleanup errors use Python
   3.11 `BaseExceptionGroup`, supported by the existing host/guest runtimes.
   The reviewed control-layout refinement puts pure codecs, host wire correlation
   and retained request/ACK copies in an imported `boundary.py` to keep source
   files bounded. It adds no observer process. Nested parent consistency is not
   root attribution; wire correlation and journal copies are not semantic ACK
   admission. Actual sample/dictionary/identity validation and fixed-slot
   publication remain mandatory before step 5 can execute.
5. **Same source and focused test files:** compose the one-off reader. Direct bounded shared-file
   retention must not depend on the host event loop draining stdout during
   `spawnSync`. Stop observation only on failure/cap; never signal or extend a
   healthy compiler. Null epoch and invalid configuration must reject before
   observation, output creation or process activity; reading the bounded
   configuration itself is permitted. Dependency: steps 2–4. Risk: high.
6. **`invocation-preparation.mjs`, focused tests:** only after reader review, adapt
   the existing single-phase invocation at exact observation boundaries. If the
   architecture decision permits a marker-and-exec wrapper, preserve exact
   compiler argv/stdout/stderr and separately test retrospective root binding;
   a pre-exec marker alone does not prove exec. Leave epoch null and do not run
   the SDK. Preserve all original postguards. Risk: high.
7. **`RESULTS.md`, exact delivery manifest:** independently review CODE, Python,
   security and actual test evidence. Merge only the bounded source checkpoint
   through the usual exact-head PR path. Host tests establish reader behavior on
   their fixtures, not live capture completeness or a product proof.

## Verification and later native gate

Use the existing standard-library test runtime, with bytecode caches disabled.
Record each intended RED, resulting GREEN, relevant planted defects and the
actual final test/coverage output. Do not claim an unavailable coverage metric.
Use synthetic filesystem fixtures below the preparation budget; preserve unique
failure reports and don't rerun writers inside frozen evidence directories.

The 100 ms cadence and 1 MiB observation cap do not guarantee coverage of the
previous 264-second run. The measured argv corpus supports size calculations, not
an observed concurrency history. Truncation, unknown ancestry and missing boundary
samples must limit conclusions. Observer runtime memory is separate from encoded
payload capacity; it must be accounted for before any live run.

Only a later independent pre-run review may bind a new resource epoch, exact
runtime/source/tool identities, shared-path custody and fresh storage. Require
the unchanged 20 GiB host launch floor, 10 GiB peak estimate plus 10 GiB reserve,
container-artifact 11 GiB before/10 GiB after, idle ownership and complete absolute
zero counters immediately before verification. Do not confuse the guest root,
Docker data volume and shared host volume. Preserve the old nonzero epoch before
any separately authorized restart. No automatic retry or control promotion follows
from recurrence, non-recurrence or an incomplete diagnostic.
