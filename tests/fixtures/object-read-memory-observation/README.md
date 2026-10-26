# Admitted-range memory diagnostic source preparation

This directory currently contains **partial host-tested parsing and journal helpers** for
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

From this directory, with an existing Python 3.10 or newer installation (select
that interpreter explicitly if `python3` resolves to an older version):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest -v test_observe
```

The current 35 tests use synthetic records and in-memory streams. They do not
exercise a guest observer, shared-filesystem durability, original proof cost,
the SDK or a new resource epoch. Recorded checks used host Python 3.15; the
selected guest Python 3.12 runtime has not executed this source.

[Verification evidence](evidence/RESULTS.md) includes the actual RED/GREEN
history, independent partial-source reviews and measured host line coverage.

The implementation sequence remains in the
[working plan](../../../docs/plans/2026-10-26-admitted-memory-observation.md):
ownership/race
checks, clock/ACK linkage, observer composition, and separately reviewed invocation preparation are still required.
The 1 MiB observation cap cannot guarantee coverage of the earlier 264-second
verification. No automatic retry, resource-limit change or acceptance waiver
is supplied by this directory.
