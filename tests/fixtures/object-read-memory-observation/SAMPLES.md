# Forced sample and retained ACK composition

`SampleBoundary.capture_boundary(request)` acquires one ordinary and slow sample
through the owned readers, retains its evidence, and derives the ACK arguments
internally. This is source preparation for the single admitted-range memory
observation. It does not launch a reader or compiler, publish an ACK, establish
root ancestry, or accept the previous `memory.events.max=6325` outcome.

The caller supplies an empty exclusive `Journal`, borrowed pinned cgroup/proc
descriptors, a fixed concrete identity tuple, and trusted synchronous identity and
clock readers. The tuple contains container ID, immutable image digest, StartedAt,
guest boot ID, target cgroup path/device/inode/mode, and observer PID/start ticks/
cgroup. The cgroup pin must agree with the borrowed directory identity and the
observer must be outside the target subtree. SHA-256 of the bounded canonical
encoding of this tuple becomes the ACK binding; callers cannot supply a valid
boolean, sample number, offset, prepared sample or independent binding string.
Live acquisition and validation of those preflight inputs remain separate work.

Each request must be the next exact nonce/sequence/label. The operation checks
current identity, acquires its guest time, verifies no child cgroups, reads the
whole bounded census, memory fields and every listed PID through the existing
owned double-read path, checks census/topology again, and observes identity and
time again. It retains ordinary and slow rows, checks counter history, and checks
identity once more before the trusted journal retention/sync operations. The
returned ACK references a complete synchronized prefix containing both samples.
The separate ACK-copy row is also synchronized before return.

## Retained rows

Version 3 header and the following compact JSON-array rows use the existing
journal pools and 1 MiB cap. Integers from kernel counters remain canonical decimal
strings. Times are exact relative microseconds, start rounded down and end up.

- `h`: schema version, nonce, boot, container/image/start identity, cgroup and
  observer tuples, clock origin/domain and flag/dictionary limits.
- `e`: increasing keyset ID and sorted event keys. Every unknown valid event key
  is retained; a changed set gets a new ID. At most 32 sets are admitted.
- `i`: increasing identity ID, PID/start/parent fields, explicit
  `owned_unattributed` role, null root binding, base64 command bytes, optional
  namespace coordinates, base64 membership and both actual raw stat reads,
  followed by the nested parent evidence described in [FAMILY.md](FAMILY.md).
  Those raw records are captured by the same reads used for validation. At most
  1024 identities and 64 current PID cache entries are admitted.
- `s`: sample sequence, time interval, dictionary ID and matching event values,
  current/swap/optional peak bytes, compact per-PID identity/RSS/HWM/flags,
  zero missed ticks for this forced operation, and sample flags.
- `m`: the same sequence/interval, required raw `memory.stat`, optional raw
  pressure/local-events records and slow flags. Unknown stat/local keys survive.
- `g`: failed attempt sequence, any known interval endpoints, bounded reason
  and fixed field name/errno. Arbitrary exception text is never serialized.
- `b`: the existing request and ACK copies described in [BOUNDARIES.md](BOUNDARIES.md).

The encoded caps are 8 KiB for header/dictionary/identity/gap, 16 KiB for ordinary
sample and 32 KiB for slow sample. They do not enlarge the existing pool limits.
RSS/HWM/namespace/parent unknown flags are 1/2/4/8. Peak unsupported is 1;
pressure/local unsupported flags are 1/2. A missing optional file is accepted
only for ENOENT after topology and identity rechecks. Permission, IO, syntax and
cleanup failures remain failures. Unknown parent start is not an ancestry claim.

Counter disappearance/decrease rejects. Any observed nonzero global or local
counter remains sticky; sequence 7 (`before-verify`) rejects if any such value
was ever observed. This is an absolute historical-zero check, not a delta test.
Forced boundary samples establish neither 100 ms periodic coverage nor the
absence of changes between observations. Optional peak/pressure/local absence
is explicit; mandatory events/current/swap/stat readings cannot be absent.

## Failure and ownership

Every failed operation poisons the sampler and journal. A typed gap is retained
when the journal is still safely writable; a partial/uncertain write or changed
owner can prevent even that gap. Original failures are re-raised, and failure
while retaining a gap preserves both causes in order. No failed instance retries
or returns an ACK. Invalid end clocks remain unknown in the gap.

Strong references pin the original stream and synchronization callable, checked
with object identity before acquisition/writes and after retention. Detectable
replacement cannot return an ACK or redirect a later gap. A callback that changes
ownership during retention can leave partial unaccepted bytes; this does not
promise rollback. Inputs and callbacks remain trusted, single-owner and
non-reentrant. These checks do not prove immutable filesystem content, protect
against malicious in-process mutation, close cross-process races, or establish
power-loss durability.

## Remaining product blocker

Before the one informational native diagnostic: complete retrospective invocation-root linkage; bind live identity/clock acquisition;
implement bounded periodic scheduling and safe fixed-slot control publication;
review the narrow invocation, observer overhead and fresh resource epoch. Keep
all original compiler/axiom/time/resource limits and postguards. This checkpoint
cannot promote Read33, the nine pending drafts, PrismPM binding or Pages.
