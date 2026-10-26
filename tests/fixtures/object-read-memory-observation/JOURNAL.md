# Bounded journal primitive (host tests only)

This change implements the append/synchronization core of plan step 3. It is not
an executable observer, full observation-schema validator or ACK
issuer. The resource-failed admitted proof remains unaccepted. No SDK run is
performed by these tests.

`Journal` takes a trusted caller-owned, empty, exclusive, unbuffered binary stream
and a trusted synchronous fsync callback returning `None`. The caller must enforce
those conditions; this primitive does not open paths, inspect ownership, flush a
buffered wrapper, support concurrent/reentrant use, or establish durability on the
selected shared filesystem. A successful callback is only its reported result.

Before any write the row must be immutable bytes containing one UTF-8 JSON array
and a final LF. The tag determines the pool and encoded-byte limit: `s` 16384,
`m` 32768; `i`, `e`, `h`, `b`, `g` 8192 each. This is envelope validation only;
field schemas, numeric meanings, sequence order and identity linkage remain the
future composer's obligations. JSON escaping and multibyte text consume their
actual encoded length. Boundary/gap rows use a conservative 8192-byte cap.

The fixed pools remain samples 786432 bytes, identities 196608, markers 57344 and
terminal 8192. The entire terminal pool and 26624 marker bytes for fixed retained
capacity are reserved at construction. No journal row can spend them. Each row's
full length is reserved before writing, with no loan from another pool, no refund
following a partial failure and no ring overwrite. `reserved` returns a snapshot.
The [fixed retention records](RETENTION.md) account for the root marker, host
copy, bounded retrieval receipt, control/temp capacity and both terminal slots.
Actual path/role ownership, control replacement and bounded process capture still
require integration before live composition.

Short writes continue only after a positive integer count no greater than the
remaining row. Invalid counts or exceptions stop the journal; exceptions propagate
unchanged, including interruption. `written_offset` counts only bytes reported by
successful writes, while `offset_certain=False` means the actual end may include
unreported bytes. `complete_offset` and `complete_rows` advance only for complete
rows; an incomplete suffix is retained but excluded from that prefix. The full
row remains charged even when its actual write extent is unknown.

Synchronization updates `synced_offset` only after the callback returns `None`.
A failure preserves the prior bound and stops further append/sync operations.
No failure becomes an empty successful sample or successful ACK. The caller may
use the reserved terminal budget to describe the stopped state; this primitive
cannot guarantee that a failed filesystem will accept that later report.

Tests use in-memory or constant-space counting streams. Filling a logical pool
in a test does not materialize a megabyte on disk. Whole-observer Python RSS,
allocation bounds, shared-file atomicity and power-loss behavior remain untested.
