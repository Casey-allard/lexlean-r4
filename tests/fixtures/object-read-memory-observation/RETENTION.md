# Fixed retention records (host source checkpoint)

This finishes the fixed-slot and typed-record portion of plan step 3. It does
not start an observer, run LexLean, bind a process, or accept the resource-failed
proof. The existing `memory.events.max=6325` result remains unaccepted.

The unchanged logical cap is 1,048,576 bytes, split into samples 786432,
identities 196608, markers 57344 and terminals 8192. `Journal` now reserves
27648 marker bytes up front for the following fixed slots. The remaining 29696
marker bytes include the header, boundary rows and gap rows; a maximum 8192-byte
header leaves 21504. Unused fixed capacity cannot be borrowed by journal rows.

| Fixed slot | Encoded byte cap |
|---|---:|
| request, request temporary, ACK, ACK temporary | 512 each |
| container root marker | 4096 |
| retained host root marker | 4096 |
| retrieval receipt | 16384 |
| guest terminal | 4096 |
| host terminal | 4096 |
| observer launch and observer exit | 512 each |

`Retention` owns single attempts for the two roots, retrieval receipt and two
terminals and two owner receipts. The four control slots are reserved; their replacement protocol is
not implemented here. All files must be caller-owned, initially empty,
exclusive and unbuffered. Callers provide synchronous synchronization operations
returning `None`. This source opens no paths and does not enforce cross-process
role ownership, support concurrent/reentrant use, or guarantee filesystem
durability. Creating another ledger is not permission to create another copy.
The later invocation must map these fixed roles to exact owned paths.

An attempt is consumed before validation. Invalid data, a partial write, an
interruption or a synchronization failure never grants a retry or refund. Short
writes advance only by reported positive integer counts. An uncertain write
marks its extent unknown; completion and synchronization are separate flags.
Original exceptions propagate unchanged. One terminal's failure leaves the
other role available, but failure of both cannot guarantee a retained report.
No third fallback file or publication temporary is provided.

`encode_terminal` and `encode_retrieval` build the exact compact JSON arrays in
the reviewed refinement. Fixed arities, enums, primitive types and lengths are
checked before serialization. Counters remain canonical decimal strings;
remaining quota is derived from a validated snapshot. Missing prefix or quota
knowledge remains null, distinct from known zero. Terminal state describes the
observer lifecycle, not coverage completeness or SDK acceptance. Prior journal
synchronization is separate from the terminal writer's subsequent synchronization.
Observer and SDK process outcomes are independent fields. Supplied field
provenance is a future composer's obligation.

Retrieval captures are immutable byte prefixes, at most 4096 bytes per stream;
base64 copies are charged inside the 16384-byte receipt. Process status,
completeness and reason must satisfy the local predicates. This serializer does
not choose an observed cause or enforce priority among multiple observed causes.
Truncation cannot be inferred merely from an incomplete flag. The future bounded
capture must replace the existing broad 20 MiB command capture for this one
receipt; it may not retain extra raw or duplicate JSON output outside these slots.

The `owner-retention/1` profile reserves these same owner bytes in the child;
the parent alone writes the two actual files. Another ledger grants no additional
capacity. `ol` and `oe` are canonical fixed arrays outside the journal row pools;
their state/reason and custody rules are specified in [OWNER.md](OWNER.md).
Historical artifacts remain byte-identical and use their pinned predecessor
source/profile; they are not silently reinterpreted by the current validators.

The cap measures logical encoded bytes, not physical allocation or Python RSS.
With hypothetical 4096-byte blocks, the maximum journal and eleven fixed files
round to 1,073,152 data bytes before metadata. Real allocation, capture buffers,
control replacement, sample schemas, ownership/race checks, clock/ACK linkage
and live composition remain unverified. These host tests use in-memory streams;
there is no new native result or launch authority.
