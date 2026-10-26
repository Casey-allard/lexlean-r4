# Bounded acquisition for the one memory diagnostic

This is partial implementation of plan step 4. These helpers acquire files for
the proposed observation of the admitted-range proof's `memory.events.max=6325`
failure. They do not run the diagnostic or change that failed resource outcome.
The later observer still needs epoch/configuration binding, control/ACK linkage,
invocation ancestry, sample encoding and retention composition, and independent
invocation review. `bindings.json.liveEpoch` remains null.

## Ownership and finite operations

`open_pinned_directory(path, expected)` accepts a trusted absolute path and an
exact `(st_dev, st_ino, st_uid, st_mode)` directory pin. It rejects empty, dot and
dot-dot components, NUL, paths above 4096 encoded bytes or 64 components. Every
component opens relative to the preceding descriptor with `O_DIRECTORY`,
`O_NOFOLLOW` and `O_CLOEXEC`. At most 65 descriptors are held during the walk.
All ancestor descriptors close before ownership of the leaf transfers to the
caller. A failed ancestor close also causes the leaf to be closed and the entire
operation to fail. Unsupported flags or descriptor-relative APIs fail closed.

`read_observation_file(fd, pin, name)` borrows the supplied directory. Only the
13 fixed names and caps in `_OBSERVATION_CAPS` are accepted. It opens with
`O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK` and requires a regular file before reading;
the nonblocking flag avoids waiting on a FIFO before rejecting its type. It
reads through the existing cap-plus-one overflow check, without treating procfs
`st_size` as a payload limit. The parent descriptor is checked before opening
and after reading. The unbuffered wrapper does not own the raw descriptor;
wrapper and descriptor cleanup are separate, explicit attempts.

`check_no_descendants(fd, pin)` borrows the cgroup descriptor and iterates at
most 256 immediate entries plus one overflow probe. Each accepted entry uses a
fresh no-follow `stat` relative to the owned descriptor. A directory, symlink,
lookup failure or overflow rejects. It does not recurse, sort, materialize a
directory listing or trust cached `DirEntry` type information. Its iterator is
closed independently, including after failure. The borrowed descriptor stays
open.

The original 64-entry limit rejected the owned VM's ordinary cgroup control
files. A separately reviewed, bounded metadata census on 2026-10-26 exhausted
92 entries, all regular files, with matching directory pins. The
[entry-bound results](entry-bound-results.json) preserve that measurement, the
original incomplete census and the tests-first repair. This changes only the
directory enumeration bound; PID, event, retention and proof resource limits
are unchanged. The observer itself has not yet run in the guest.

`owned_process_snapshot(cgroup, proc, pid, target)` accepts each borrowed root
as `(fd, pin)`. It checks the cgroup's immediate children, acquires its bounded
`cgroup.procs`, and opens only a PID present in that census relative to the
pinned proc root. It calls the existing fixed-file double-read validator through
the owned PID descriptor, checks descriptor identities around the sample, then
rechecks cgroup children. It never enumerates the proc root, reads an unrelated
parent, or grants attribution. Returned processes remain `owned_unattributed`.

## Failure semantics and limits

Python 3.11 or newer is required. A single failure is raised as the same original
exception. Multiple failures form a `BaseExceptionGroup`, with the primary first
and cleanup failures in attempt order; nested groups, falsey errors,
`KeyboardInterrupt` and `SystemExit` are retained. Each explicitly owned resource
has one close attempt. A failed raw close is never retried because the descriptor
may already have been released. Success is returned only after successful
cleanup. Abrupt process termination or asynchronous interruption between Python
instructions is not made transactional by these helpers.

A pinned descriptor continues to reference its opened object after pathname
replacement. That property is tested and **does not establish the current
pathname's identity**. The later caller must repeat the component walk at
observation boundaries and compare the independently pinned boot, mount,
container and cgroup identities. Ancestor directories are checked as no-follow
directories, not forced to share the leaf owner or permissions. Pins do not
prove mount identity or constitute authority if supplied by an untrusted caller.

Checks are discrete. They cannot exclude unobserved ABA replacement, transient
descendants or membership changes between reads. A snapshot is not atomic and
does not establish invocation ancestry, successful exec, a common clock domain,
or continuous sample coverage. Unsupported or failed observations must later be
retained as failures, never substituted with zero or an empty valid sample.

## Host evidence

The focused suite exercises real temporary directories, symlinks and FIFOs,
exact/overflow file caps, renamed directories, caller-fd preservation and
synthetic open/read/stat/close failures. It uses only owned disposable fixtures;
all unique official RED/GREEN records are retained. Current evidence includes
64 full-suite tests, nine separately planted defects rejected, and measured host
line coverage. No guest observer, SDK, compiler or production gate was run.
See [owned IO results](evidence/OWNED-IO-RESULTS.md) for exact identities and
independent reviews. The finite preparation budget is not a measurement of a
live observer's memory cost or a guarantee that 1 MiB captures the whole proof.
