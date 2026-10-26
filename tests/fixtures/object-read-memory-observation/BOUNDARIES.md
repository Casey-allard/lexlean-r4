# Parent observations and finite boundary records

This is partial diagnostic source preparation. There is no live observer entry
point, native invocation or new resource epoch. The original resource outcome
at `memory.events.max=6325` remains unaccepted. The LexLean definitions, theorem
statements, compiler and limits are unchanged.

## Parent observation

`process_family_snapshot` uses the caller's bounded census and trusted reader.
After the child's initial identity, membership, status and command reads, it
observes one census-listed parent through the existing double-read function.
It then performs the child's final identity and membership reads. Both snapshots
must remain consistent before a parent start tick is returned. It never follows
a grandparent or reads a parent outside the supplied census. All snapshots remain
`owned_unattributed`; parent consistency is not invocation-root attribution.

An ordinary parent IO or observation failure returns an unknown parent plus the
original exception, while the child checks continue. A subsequent child failure
preserves both exceptions in order. Fatal parent exceptions propagate. The caller
must encode a bounded typed gap rather than serialize arbitrary exception text.
An unobserved reparenting ABA remains possible. This helper does not itself acquire
or pin the filesystem descriptors; composition with the owned readers is required.

## Wire format and host correlation

`boundary.py` is an imported helper, not a process or service. Records are exact
canonical compact ASCII JSON arrays followed by one LF, at most 512 bytes:

```text
["q",1,nonce,sequence,label]
["a",1,nonce,sequence,label,sampleSequence,completeOffset,syncedOffset,binding]
```

Nonce and binding are 64 lowercase hexadecimal characters. The binding is intended
to identify the retained invocation identity tuple; matching a supplied string
alone does not validate that identity. Sample sequence is a positive integer.
Offsets are canonical decimal strings, positive and at most 1013760, with synced
equal to complete. Each of eleven sequences has exactly one label, from
`staging-before` through `postguard-complete`, fixed by `LABELS`.

`BoundaryWire` allows one outstanding request. It correlates nonce, sequence,
label and binding, requires increasing sample/prefix values, and uses supplied
exact integer host timestamps for a two-second deadline. An unchanged previous
ACK may be polled without consuming it again. Malformed or mismatched records,
backward clocks, late responses and invalid state permanently fail the instance.
It assumes one trusted, non-reentrant owner; it acquires no clock itself.

Successful correlation advances wire state only. The caller must independently
resolve the retained sample and dictionary, current identity, mandatory readings
and pre-verification all-zero counters. It must reject semantic failures before
using any ACK to advance a native phase. This semantic admission is not implemented
by the wire helper.

## Retained copies

`retain_ack_copy` preflights both boundary rows against the existing marker pool.
It pre-encodes the ACK for quota accounting, referencing the planned request
prefix. It appends and synchronizes the request copy, checks that the prefix is
complete and synchronized, then appends and synchronizes the exact ACK copy.
Only then does it return the ACK bytes for later publication. The referenced
prefix intentionally precedes the ACK's own copy, avoiding a circular offset or
an assertion that a future write already succeeded. Both copies use base64 in
the bounded boundary flags; the observation cap and pool sizes do not change.

Any write, synchronization or validation failure stops further journal use.
Partial bytes and the last complete/synchronized bounds remain available. A
successful synchronization callback is not evidence of power-loss durability.
The helper trusts the existing exclusive stream and synchronous callback contract.

This function tests retention, not sample semantics: tiny tests deliberately use
an opaque sample envelope. No live evidence can be admitted from that envelope.
Actual sample/dictionary validation, root linkage, control-slot publication,
replacement and later observer composition remain required. No ACK is published
by this source.

## Current verification boundary

The author observed 84 host tests passing with explicit Python 3.15 after the last
source change. The tests cover nested read order, unknown and fatal parent errors,
identity changes, canonical wire rejection, replay and deadlines, exact retained
copies and failures at both write/sync positions. Independent code, Python and security source reviews passed within this scope;
see [the retained results](evidence/BOUNDARY-RESULTS.md). No guest run, live process sampling or native proof result is asserted.
