# Admitted-range memory diagnostic source preparation

This directory currently contains **partial host-tested parsing, journal, owned-file and boundary helpers** for
the one diagnostic described in the [reviewed plan](../../../docs/continuation/2026-10-25/read-admitted-memory-diagnostic/README.md).
There is no live observer entry point or compiler invocation here. The LexLean
artifact's resource outcome remains unaccepted because `memory.events.max`
increased to 6325. Parsing tests do not change that result or accept Read33.

`bindings.json` preserves a null live epoch and byte-identical copies of the
concrete contract, placement decision and implementation SPEC. Those historical
documents retain their original external evidence paths; their source-level
approval does not imply that all referenced evidence is bundled here or that
the unfinished observer can execute.

## Implemented scope

- Canonical unsigned 64-bit counters stay decimal strings. Unknown valid event
  keys are retained; missing, duplicate and malformed fields reject.
- A caller-owned binary stream is read through a finite cap plus one overflow
  byte. Short reads continue to EOF; failures retain their original exceptions.
  This helper neither opens a path nor establishes its ownership.
- A cgroup census is bounded to 4096 bytes and 64 positive, distinct PID values.
  Duplicate entries are conservatively rejected as an unusable census; this is
  not a claim that the kernel can never produce duplicates during a changing
  census.
- Bounded stat/status records decode PID, parent PID, start ticks, up to 64
  namespace coordinates and optional RSS/HWM in KiB. Missing optional values
  remain unknown. Parsing a record alone does not establish a stable incarnation,
  namespace correspondence, ancestry or successful exec.
- Unified cgroup membership remains exact opaque path bytes for comparison.
  Never open or normalize a filesystem path derived from that return value.

The [journal primitive](JOURNAL.md) reserves whole encoded rows before writing,
handles short writes, preserves the complete prefix after IO errors, and advances
its synchronized bound only after the trusted callback succeeds. Its fixed pools
exclude terminal and control capacity from ordinary rows. This is partial plan
step 3. The [fixed retention records](RETENTION.md) add protected copy slots,
typed retrieval/terminal encoding and independent one-attempt writers. Live
ownership, control replacement and process capture remain open.

The stat field layout follows the [Linux manual](https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html).
Cgroup discovery must respect the [kernel's cgroup v2 interface](https://docs.kernel.org/admin-guide/cgroup-v2.html).
The current parser reads only the supplied bytes; it performs no process scan.

## Host verification

From this directory, with an existing Python 3.11 or newer installation (select
that interpreter explicitly if `python3` resolves to an older version):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest discover -v
```

The current 104 tests use synthetic records, in-memory streams and tiny owned
host filesystem fixtures. They do not
exercise a guest observer, shared-filesystem durability, original proof cost,
the SDK or a new resource epoch. Recorded checks used host Python 3.15; the
selected guest Python 3.12 runtime has not executed this source.

[Current sample-composition results](evidence/SAMPLE-RESULTS.md) record the 104-test
checkpoint and its source reviews. [Earlier parser evidence](evidence/RESULTS.md)
retains that earlier checkpoint's RED/GREEN history and coverage.

The implementation sequence remains in the
[working plan](../../../docs/plans/2026-10-26-admitted-memory-observation.md):
live ownership and parent/root linkage, periodic scheduling, safe control
publication, observer composition and separately reviewed invocation preparation
are still required.
The 1 MiB observation cap cannot guarantee coverage of the earlier 264-second
verification. No automatic retry, resource-limit change or acceptance waiver
is supplied by this directory.

The [finite process and clock helpers](IDENTITY.md) add trusted-reader double-read
consistency and exact guest-clock arithmetic. They do not establish path ownership,
parent ancestry, a live clock domain or completed step 4. Control/ACK and live
composition remain required; standard test discovery includes `test_identity.py`.

The [owned-file acquisition helpers](OWNED-IO.md) implement bounded no-follow
directory walks, fixed-name reads, immediate-child checks and one census-listed
PID snapshot. Their trusted input pins must come from the later launch preflight.
Standard discovery includes `test_owned_io.py`; no helper discovers a live
container, scans unrelated processes, creates an observation output or runs a
compiler. Python 3.11 is required for `BaseExceptionGroup`, which retains primary
and cleanup failures without masking either. Existing host 3.15 and guest 3.12
need no installation; the guest has not run this code.

The earlier [parent and byte-only boundary helpers](BOUNDARIES.md) add a nested parent observation,
finite request/ACK codecs, host-clock wire correlation and synchronized journal
copies. `boundary.py` is a non-executable fixture-local module; it does not add a
second observer process. Those earlier helpers do not validate an ACK's underlying sample,
establish invocation ancestry, publish control files or authorize a compiler run.
Those composition obligations remain open. Independent code, Python and security reviews approve only these bounded helpers;
all three independently reran the then-current 84 host tests. See [earlier results](evidence/BOUNDARY-RESULTS.md).

The [forced sample composition](SAMPLES.md) now connects actual owned ordinary
and slow reads to retained ACK arguments inside one operation. Historical
nonzero counters block the sequence-7 pre-verification ACK; earlier sequences
retain such observations. Missing mandatory evidence, changed identity/census,
partial writes and failed synchronization block every boundary ACK in the fixtures.
The earlier wire helper remains byte-only. Root linkage, live preflight, periodic
coverage, safe channel publication and native invocation remain open.
