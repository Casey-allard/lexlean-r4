# Invocation-root content linkage

`root_linkage.join_invocation_root` connects the retained root marker, retrieval
receipt, before/after-verify request and ACK copies, and the version-3 sample
journal. It performs no IO and grants no execution authority. Its only positive
result is a **matched observation**, conditional on later validation of the
actual capture operations and namespace mapping. Every descendant remains
`owned_unattributed`; it does not identify the process responsible for a memory
counter increase.

The marker is at most 4096 bytes, with four LF-terminated lines: `R4ROOT/1`, the
64-character invocation nonce, the canonical inner PID, and the original one-line
proc stat. CR, NUL, additional lines, PID disagreement and malformed numeric
fields reject. The future wrapper must separately establish exclusive mode-0600
creation and unchanged PID/start time through `exec`; this parser cannot do so.

The existing receipt codec must round-trip exactly. Successful complete retrieval
must contain those exact marker bytes and the expected nonce/epoch binding. Its
host start time must follow the supplied retained host ACK-receipt timestamp.
That timestamp is a conditional input: decoding an ACK does not establish when
the host received it. Guest sample times are never compared with host times.

The reader scans immutable journal bytes once, with existing total/row caps. It
checks the epoch header, exact request/ACK copies and byte offsets, re-parses raw
stat-before/stat-after and membership, and checks sample references. Only completed
samples after the before-verify sample and through the return sample can supply
root evidence. Dictionary entries alone cannot. The child namespace vector is
the sampler's parsed value; child raw status is not retained and is not replayed
by this reader. Matching therefore remains conditional on the actual sampler and
the separately exercised namespace coordinate/depth.

Exactly one guest PID/start incarnation must match the marker's inner PID/start,
and that incarnation must have the exact NUL-delimited pinned LexLean verify argv
in the selected sample window. Missing namespace, no match, multiple matches or
missing argv evidence stays unknown. A missing namespace reading never fills
itself from an earlier sample, and it cannot erase a conflicting known namespace.
Only bounded identity metadata and argv-match bits are retained, not another
complete trace or all command lines. Parsed Python runtime memory still requires
measurement; encoded quotas are not an RSS bound.

This is deliberately not a generic journal validator: unrelated memory payloads
and nested parent evidence are not reinterpreted. Live identity/clock preflight,
sample-local ancestry, periodic scheduling, fixed-slot publication, actual wrapper
and retrieval custody, invocation review and fresh resource gates remain open.
The live epoch remains null. No SDK run, proof acceptance, resource-limit change
or production binding is established.

## Host verification

The final host suite passes 133 tests, including 17 new root-linkage tests using
actual `SampleBoundary` captures over synthetic owned filesystem inputs. Six
separately planted source omissions each produce an assertion failure: nonce,
start ticks, exact argv, host receipt ordering, header numeric type and boundary
status type. These are planted host defects, not historical compiler negatives.
Line tracing covers 220 of 249 executable lines in the new reader; branch coverage
was not measured.

The first 12 REDs were missing-capability assertions. A tuple/list fixture setup
mistake then prevented eleven cases from reaching the reader; it is preserved as
an instrument error. Subsequent actual tests exposed namespace-absence handling,
boolean/integer aliasing and nonfinite-JSON error typing. Independent debugger
reports distinguish those failures: nonfinite input was already rejected, but
initially escaped as a raw `ValueError`. The corrected suite and original failures
are retained in the companion evidence archive. Review results are recorded in
`root-linkage-results.json`; this page makes no native acceptance claim.
