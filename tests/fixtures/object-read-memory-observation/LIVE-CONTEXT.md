# Owned context and one serial diagnostic reader

The [finite owner checkpoint](OWNER.md) connects the existing configuration
adapter to one fixed child and parent-owned lifecycle receipts. The child reserves
the same two 512-byte slots in `owner-retention/1`; it never writes those files.
Local finite process tests do not qualify Linux launch identity, Colima transport,
parent-death cleanup, shared-filesystem overhead or actual compiler invocation.

This checkpoint connects the existing memory sampler, cadence and fixed control
files to actual owned-file operations and one serial loop. It is host-tested
diagnostic source for the admitted-range proof's `memory.events.max=6325`
failure. It is not a LexLean application component, SDK implementation, live
capture result or accepted Read33 control. The resource result remains unaccepted.

`live_context.ContextSpec` carries the exact retained Docker projection, boot ID,
init start ticks, target membership and three directory pins. The reader rejects
an absent or malformed configuration before opening files. It opens only the
specified pinned roots, its own process and the census-listed container init;
it does not scan unrelated processes. Actual bounded boot, stat, status and
membership reads must agree. The init census precedes both initial acquisition
and later numeric-directory reopening. Identity and path replacement, missing
init, nested cgroups, clock errors and reentry stop the context. The guest
`CLOCK_BOOTTIME` primitive supplies the origin and subsequent times; there is no
fallback clock or supplied self-PID callback.

These checks are conditional on the selected mount/proc namespace and retained
Docker capture. They do not discover Docker state or authenticate host metadata.
Live namespace, mount, wrapper/exec and host-receipt custody remain prerequisites.

`ObserverSession` borrows that context and the existing `SampleBoundary`,
`Cadence`, `OwnedControlSlots` and `Retention`. Every forced ACK follows the
actual sample, retained request/ACK rows and journal synchronization. Re-reading
the same request allows periodic sampling without repeating its ACK. The last
boundary ends the reader. Failures stop observation, preserve original and
cleanup exceptions, and attempt one bounded terminal record. Reentrant calls
poison the operation even if a callback catches their exception.

`run_owned` owns the concrete context, output directory and two exclusive 0600
files, `journal.jsonl` and `guest-terminal.json`, beneath a pinned 0700 directory.
It uses unbuffered file objects and actual `fsync` callbacks. Existing files or
symlinks are never overwritten. Partial setup may leave empty exclusive files;
that is failure evidence, not a completed observation. Every acquired descriptor
and stream has an independent cleanup attempt, with ownership discharged before
close and no retry of uncertain closes. Temporary test fixtures are removed.

This is an imported function, not a command-line launch adapter. It neither
starts nor signals a compiler. Host retrieval must separately record the actual
observer exit, verify custody and interpret any missing or partial terminal.
The guest terminal reports SDK process state as unknown and detailed coverage
as incomplete. Its `ended` state means only that the final observation boundary
was reached; it does not assert proof success or resource acceptance.

The [results](live-context-results.json) and lossless evidence archive retain the
actual RED/GREEN sequence, independent reviews and instrumentation limitations.
No guest Python execution, shared-filesystem durability check, observer overhead
measurement, fresh epoch, compiler invocation or native acceptance is supplied.
The [working plan](../../../docs/plans/2026-10-26-admitted-memory-observation.md)
still requires those gates before the one diagnostic may run.
