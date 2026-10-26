# Finite process consistency and guest-clock arithmetic

These helpers implement part of step 4 of the reviewed diagnostic plan. They do
not open paths, discover processes, sample a real clock or authorize an SDK run.
The admitted-range proof's `memory.events.max=6325` result remains unaccepted.

`process_snapshot(census, pid, target, read)` admits only a PID present in the
bounded supplied census. It calls a trusted bounded reader for exactly that PID:
stat, membership, status, cmdline, stat, membership. It checks initial membership
before reading status/argv, then rejects changed PID, start ticks, PPID, status
identity or final membership. Every returned raw value must be immutable bytes
within its individual cap. Original reader exceptions propagate unchanged and
stop further reads; the future composer must retain raced/vanished/denied outcomes
without inventing a successful sample. No previous process fields are cached.

The returned PPID is an observation, not a proven parent edge. `parent_start` stays
null and attribution stays `owned_unattributed`, including when argv resembles a
compiler command. Parent linkage, root binding and classification remain later
composition work. Missing RSS/HWM stays null; observed numeric zero stays zero.

The caller must obtain the census from the exact owned cgroup, establish stable
boot/cgroup/path ownership and confinement, reject unsupported child topology,
and enforce bounded reads before returning bytes. The helper's supplied paths and
census are not ownership evidence. Double reads cannot detect an unobserved ABA
membership change, prove an atomic snapshot, or bind an exec transition.

`guest_interval` uses exact integer nanoseconds and emits relative microseconds:
start rounds down, end rounds up. `next_tick` advances a 100 ms absolute grid to
the first slot strictly after completion and reports skipped slots; it never
requests catch-up work. Both reject invalid numeric types/order/ranges; scheduling
also rejects a misaligned due time and u64 overflow. The caller must establish the
same guest CLOCK_BOOTTIME domain and unchanged boot for all inputs. Numerically
valid host timestamps cannot be distinguished by arithmetic and must never be
passed here. These helpers do not sleep, poll, sample or issue ACKs.

Tests are synthetic byte scenarios, not captured process histories. The separate
`test_identity.py` keeps the existing 668-line test file unchanged and avoids
exceeding the review file-size limit. Standard unittest discovery runs both.
Control/ACK linkage, owned acquisition, descendant census, parent edges, reader
composition, invocation binding and real resource/placement checks remain open.
