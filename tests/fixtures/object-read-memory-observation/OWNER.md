# Fixed observer owner: finite host source checkpoint

`observer_owner.main` connects the existing seven-argument config adapter to one
fixed observer child and two exclusive files, `observer-launch.json` and
`observer-exit.json`. It owns the fork-return PID, GO pipe, output handles and
wait result. The host-input descriptor is borrowed. There is no configured PID,
executable, source directory, callback, environment or process-group interface.
The compiler is never signaled. This checkpoint does not authorize a live or
indefinite observer or change the existing Node null gate.

Before fork, config and its independently supplied parent pin are validated by
`observer_entry.load_config`; the exact eight-module manifest and source custody
are checked, and both empty exclusive 0600 receipts are opened beneath the pinned
0700 output directory. Only one fork attempt is allowed. The child blocks on an
anonymous GO pipe, closes unrelated descriptors, and directs stdin, stdout and
stderr to `/dev/null`. Output cannot saturate a host pipe or impersonate receipts.
EOF or a non-G byte exits125 before exec. The fixed child argv uses
`/usr/bin/python3.12 -I -B -c` and the source-pinned bootstrap. The bootstrap
preloads stdlib, verifies bounded nofollow reads of the eight sources, rejects
applicable and legacy bytecode caches, and checks module paths and source bytes
after import. These checks assume trusted directory custody, not a hostile writer
with the same UID. Parent source/runtime deployment pins need independent review.

The parent reads boot, init, own and fork-child identity through the supplied
pinned Linux proc/cgroup roots. Child PPID must match the parent, and both owner
and child must be outside the target cgroup. The parent writes and fsyncs launch
before GO. Launch is a pre-exec incarnation receipt; it does not prove exec,
subsequent h3 identity, namespace mapping, compiler ancestry or proof success.

Only exact `waitpid(fork_pid, WNOHANG)` final exit/signal statuses consume custody.
`poll` enters a nonsignalable transition before wait and owns the entire span
through status publication. Only a proven zero result, accepted stopped/continued
status, or genuine syscall `EINTR` restores `OWNED`. Terminal status marks
`REAPED` before fallible decoding. Uncertain wait, decoding or publication errors
permanently prohibit signals and preserve the original error with unknown status.
`ECHILD` also permanently loses custody. No signal follows reap or custody loss.

Local input EOF or owner error causes TERM and, only while still owned, KILL.
One monotonic start fixes the polling deadlines at start plus 2 and 4 seconds;
interruptions never reset them. A sleep or kill interruption cannot reap under
the sole-reaper assumption: it is retained while bounded cleanup continues.
After KILL, a known-owned sleep interruption resumes final polling against the
same start-plus-4 deadline. Signals are sent only once. Deadline expiry still
performs the final nonblocking poll. Loss of the final-phase clock permits one
no-sleep fallback poll, even after a prior sleep interruption; an exception in
that fallback ends the attempt and cannot cause an endless retry.
An unavailable clock switches to only the two existing signal/nonblocking-poll
phases, with no sleep, reacquisition, or elapsed-time assertion. Unreaped clock
fallback is unknown/owner_error; a measured final deadline is unknown/reap_timeout.
ESRCH is not an exit receipt. GO close, receipt and independent output-close errors
are preserved; no failure authorizes a retry, replacement or fallback file.
Default SIGCHLD and sole thread are checked. Exclusive reaper ownership remains
an invocation obligation. Parent death, Node blocked on spawnSync, real Colima
EOF transport, shared-filesystem durability and native custody remain unresolved.

Receipts are compact ensure_ascii JSON arrays with one LF, exact decode/reencode
bytes, and a512-byte cap. `ol` is
`["ol",1,nonce,digest,[parentPID,parentStart],[childPID,childStart],boot]`.
`oe` is `["oe",1,nonce,digest,PIDOrNull,startOrNull,[state,code,signal],reason]`.
Nonce/digest are lowerhex64; boot is canonical UUID; PID/start are u64 decimal
strings, positive PID; version/code/signal are strict JSON integers, never bool.
The digest binds Docker projection, boot, init start, directory pins and target;
it correlates content and grants no authority. Final states and reasons are:

| State | Status fields | Reasons |
|---|---|---|
| not_run | PID/start/code/signal null | not_started, fork_failed |
| exited | positive PID; code0..255; signal null | natural, host_eof, owner_error |
| signaled | positive PID; code null; signal1..64 | natural, host_eof, owner_error |
| unknown | positive PID; code/signal null | lost_custody, reap_timeout, owner_error |

Start may be null after fork if capture failed. Reason names identify the owner's
trigger, not inferred child causality; code127 alone is not exec-failure proof.
EOF after final wait does not rewrite the reason. Partial or absent files remain
unknown; encoding cannot assert its own future fsync. `main` returns the recorded
process tuple and propagates errors, including compound cleanup failures. `cli`
returns0 only for an actual exited0 tuple, preserves config64/65/74, otherwise70;
diagnostics are fixed and bounded.

`owner-retention/1` reserves the same1024 bytes in every child Journal and gives
the parent exactly two512-byte slots. Total logical cap remains1048576; fixed
markers27648, marker journal29696, total external35840 and journal max1012736.
No multiple-ledger or empty-slot capacity is added. h3 emission and root-linkage
validation require the profile. Historical artifacts use their unchanged pinned
predecessor source/profile. Physical allocation and Python RSS are separate.

Tests execute only finite harmless host children with independent alarms and an
owned watchdog. Private test substitutions never become a production command API.
Synthetic proc files test parsing and binding to real fork handles; they do not
establish actual Linux proc/start semantics. The real isolated bootstrap reaches
the existing entry with invalid arguments and never runs its observer loop.
No SDK, VM, compiler or epoch reset is exercised; the retained max6325 failure and
all original invocation inputs, argv, limits and independent guards remain intact.
