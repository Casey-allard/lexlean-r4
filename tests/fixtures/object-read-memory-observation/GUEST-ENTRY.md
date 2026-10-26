# Functional guest reader entry

`observer_entry.py` connects a bounded configuration file to the existing
`ContextSpec` and actual `run_owned` reader. It adds no compiler launcher or
second disabling flag. The host invocation's null epoch stays unchanged.
Configuration validity is not permission to run the diagnostic or proof.

The command accepts exactly seven tokens after the script:

```text
--config-directory ABSOLUTE_GUEST_PATH --parent-pin DEV INO UID MODE
```

The caller must independently obtain the guest directory identity; a host
inode or a tuple supplied by the configuration itself is not that identity.
The directory must belong to the current user with mode0700. The fixed file
`observer-config.json` must be regular, mode0600, current-user-owned, single-link
and 1–16384 bytes. Componentwise opens reject symlinks. Bounded reads compare
file metadata before/after and reopen the parent and fixed filename to detect
observed pathname replacement. This does not establish mount authenticity or
perpetual filesystem stability.

The JSON schema is `lexlean-r4/observer-config/1`, with exactly `schema`, `nonce`,
`context` and `output_directory`. Context contains exactly `docker`, `boot`,
`init_start`, `cgroup`, `proc`, `boot_directory` and `target`. See the test's
explicit valid fixture for the complete shape. Directory pins use canonical
unsigned decimal strings on the wire and exact Python integers in memory,
preserving values above2^53. Duplicate keys, unknown fields, numeric pins,
booleans, malformed encoding, oversized data and invalid paths reject.

Every config handle closes before the reader starts. Cleanup retains the
primary error and every close error, including falsey errors and interruptions.
The CLI emits no stdout and only fixed, bounded categories on handled failures:

| Exit | Meaning |
| --- | --- |
| 0 | Reader and cleanup returned; no proof or resource acceptance implied. |
| 64 | Invalid arguments or asserted parent policy, before config I/O. |
| 65 | Invalid decoded configuration after successful custody and cleanup. |
| 74 | Actual config acquisition, custody, read or cleanup failure. |
| 70 | Reader failure or unexpected internal failure. |

Tests run the real `main → load_config → decode_config → run_owned` path through
all eleven boundaries on owned synthetic files. The synthetic BOOTTIME source
and serial sleep hook drive test requests; real file operations, journal, ACKs,
terminal creation and cleanup remain in use. The reader is never replaced by a
successful stub. These are host tests, not guest-runtime measurements.

The final suite passed235 tests, including23 adapter tests. Eight planted host
defects were rejected. Adapter line coverage is138/143 (96.50%); branch coverage
is unmeasured. Independent CODE, Python and security reviews passed for this
source scope. The Python reviewer independently reran23 tests and two compound
cleanup probes; actual guest Python execution remains NOT_RUN.

Run from this directory with `PYTHONDONTWRITEBYTECODE=1` and
`/opt/homebrew/bin/python3 -m unittest test_observer_entry` on the development
host. Do not invoke the CLI against real guest paths yet. The [results](guest-entry-results.json)
and lossless evidence archive preserve source identities, reviews, intended
negatives and instrument failures. Earlier result counts are historical, not
additional tests in the final235.

Next connect the existing one-off host invocation to this reader, with independently
owned launch/PID/start/exit evidence, control/abort ordering and namespace/exec
custody. Shared-filesystem behavior, overhead, fresh resource review and actual
native execution remain required. The original max6325 result stays unaccepted;
Read33, Construction39 and production PrismPM/Pages remain open. This Python
coverage does not waive the preceding JavaScript coverage gap or make PR65
merge-ready.
