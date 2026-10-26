# Bounded diagnostic composition

This checkpoint connects three parts needed to investigate the admitted-range
proof's retained `memory.events.max=6325` failure. It changes no LexLean theorem,
runtime definition, compiler, resource limit or production acceptance rule.
There is still no live observer entry point or executable invocation binding.

## Sample-local ancestry

`ancestry.iter_invocation_families` first validates the existing root-content
connection, then streams the bounded journal again. It reparses each identity's
nested raw parent evidence and follows only identities present in the same sample.
PID/start-tick identity, parent metadata and the configured namespace coordinate
must agree. Missing parents, reused PIDs, cycles and namespace collisions remain
`owned_unattributed`. This is discrete observed ancestry; it makes no claim about
continuous ancestry, memory attribution or the cause of a resource event. Lazy
iteration can fail after earlier rows, which the caller must retain as incomplete.

## Owned request and ACK slots

`control_slots.OwnedControlSlots` uses a borrowed pinned directory owned by the
current UID with mode0700. Fixed request/ACK names and temporary slots are bounded
to512 bytes, regular0600 files with one link. Roles restrict host publication to
requests and guest publication to ACKs. A publication performs an exclusive temp
create, complete write, readback, file sync and close; it checks the previous
destination and prepared bytes again before the final same-directory rename.
No fallible operation follows successful rename. Independent close failures and
original errors survive together, and failed owners cannot retry.

This provides file-synced atomic visibility. It is not a compare-and-swap against
a malicious same-UID writer and does not promise power-loss durability. Missing
peer slots are ordinary only when opening the slot returns ENOENT; a later read
error is fatal. Sequence/nonce admission remains the boundary protocol's job.

## One absolute schedule

`cadence.Cadence` calls the actual owned `SampleBoundary`. It has no thread, sleep,
process launcher or compiler handle. Ordinary samples use absolute100ms slots;
slow fields use absolute5s slots. Forced boundary samples take priority and
always read slow fields. Skipped slow reads retain earlier local-counter history.
Owned journal synchronization is targeted every1s and at existing boundaries;
blocking IO can delay it, so this is not a real-time guarantee.

The next unaccounted deadline and pending missed count are explicit state. A
periodic sample receives credit for its due slot; a forced sample receives no
periodic credit. Read-end accounting enters the sample row. Slots elapsed during
later writes/synchronization carry to the next row or remain exposed as incomplete
coverage at stop. There is no catch-up loop or recursive gap-writing loop. Any
failed sample/sync preserves the previous schedule, stops further scheduling and
leaves emitted partial evidence unaccepted. Backward clocks and swallowed
reentrant sampling calls fail closed. The original forced-only API remains
available without schedule accounting.

## Verification and remaining connection

Run `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover` in this directory.
The [reviewed results](composition-results.json) record175 passing host tests and
86.67–93.41% line coverage for the four affected modules; branch coverage was not
measured. They distinguish missing-capability REDs, real defects,
test-instrument errors and final GREEN. The first cadence run had three method
shadowing errors and two test callbacks incompatible with memoryview; it is not
retroactively described as a successful timing test. Independent review reports
and unique failure evidence are preserved in the accompanying history archive.
Independent security review found a lifecycle reentry defect after the first170
tests passed. Five additional actual predecessor failures preceded the repair:
driver and sampler ownership now fail closed together, and caught nested errors
cannot permit an outer request to return an ACK after termination.

Actual boot/self/init/cgroup and BOOTTIME preflight, serial control-loop and
terminal composition, marker/exec plus host receipt/namespace custody, placement,
shared-filesystem and overhead tests remain required. Only after those checks and
a fresh independently reviewed resource epoch may the one diagnostic invocation
run. No native test, epoch reset, SDK run or new Object control is claimed here.
