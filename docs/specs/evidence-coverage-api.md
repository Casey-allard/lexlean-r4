# Evidence coverage admission API

This refines the approved Evidence design for the next independently controlled
module, `R4.EvidenceCoverage`. Independent architecture review approved this
contract on 2026-10-21; it is not an implemented or accepted coverage claim.
The catalog remains `R4.Evidence`.

## Values and meaning

Use the catalog's closed mechanism enumeration and a closed execution-surface
enumeration: source construction, formal verification, generated execution,
research qualification, product serving and live deployment. A surface names
an obligation; its presence is not evidence that the obligation was met.

A capability has a UInt32-admitted natural ID, a surface, a nonempty ordered
list of source-root SHA-256 byte identities, and dependency capability IDs.
Every digest has exactly 32 admitted bytes. Zero bytes are structurally valid;
they grant no authenticity. Repeated source-root digests or dependencies in
one capability are rejected. Distinct capabilities may share a root digest.

An obligation has a UInt32-admitted natural ID, a mechanism and a capability
ID. A mechanism-coverage entry has a mechanism and a nonempty ordered list of
obligation IDs. Different obligations may reference the same capability,
including across different mechanisms. Every obligation must occur exactly
once in its own mechanism's coverage entry. No obligation or capability record
contains an implemented, verified, qualified or trusted flag.

The input contains the capability table, obligation table and mechanism
coverage table. Every one of the 27 mechanisms must occur exactly once in the
coverage table, including all isolated and PARKED mechanisms. This describes
the declared obligation graph; it does not prove that the declaration fully
specifies each mechanism's semantics. Semantic completeness remains subject
to the approved port design and independent implementation review.

## Budgets and deterministic validation

Budgets bound total records, source-root bytes, references and graph visits.
Records count all capability, obligation and coverage records. Source-root
bytes count the actual supplied digest-list lengths before width admission.
References count dependency IDs, obligation capability IDs and coverage
obligation IDs. These are logical resource counters, not an invented wire
encoding or a performance measurement. Count with unbounded Nat; compare
against the explicitly supplied natural budgets without machine overflow.

Ordered validation phases are:

1. Record, source-byte and reference budgets, in that order.
2. Duplicate mechanism entries, then missing mechanisms in catalog order.
3. Duplicate capability definitions, then duplicate obligation definitions.
4. UInt32 domains for every ID-bearing field, including dependency and coverage
   references, then source-root presence and digest byte/width domains.
5. Missing capability/dependency/obligation references, mechanism mismatch,
   unlisted obligations and empty mechanism obligations.
6. Repeated dependency, source-root and mechanism-obligation references.
7. Self-dependency, then complete graph traversal with explicit fuel.

Each listed check category is a global subphase, not an interleaved per-record
check. In phase 5, check all obligation capability references first, then all
capability dependency references, all coverage obligation references, mechanism
mismatches, unlisted obligations and empty obligation lists. Phase 6 checks
repeated dependencies, then repeated roots, then repeated coverage references.
Within each subphase, inspect source tables and fields in their declared order.
Each error is a distinct closed constructor. Duplicate-mechanism validation
precedes missing-mechanism validation so replacing a READY entry with another
copy reports the duplicate rather than hiding it behind the resulting gap.
This explicitly refines the broad "complete mechanism set" phase of the
approved design; it changes no upstream behavior or status.

Graph traversal starts from every capability, including unreferenced and
disconnected vertices. An explicit work list carries each vertex and its
current ancestor path. A repeated ancestor is a cycle. Dependency order and
root order match the tables; each entered work item consumes one graph visit.
An empty work list succeeds even with zero remaining fuel. A nonempty work
list with zero fuel fails with a graph-budget error, even if the next item
would expose a cycle, and produces no partial
successful description. Reject self-dependencies in the preceding phase.
Shared DAG descendants may be visited more than once; the visit budget is
explicit about that cost and must not turn an incomplete traversal into
success. Structural recursion on fuel establishes termination.

The separately callable graph helper returns a typed `missingDependency` for
an unresolved vertex. It never treats an absent capability as a leaf. Its
acyclic result describes graph structure only; it does not replace complete
coverage admission, ID-domain checks or the preceding global validations.

## Results and queries

Successful admission returns `described` with the exact supplied tables.
It means only that their shapes, references and classifications are consistent.
No public Boolean derives research or deployment authority from these values.
Errors return no description.

A public mechanism-obligation query first performs the same full admission.
It then returns that mechanism's obligations in capability-table order, with
obligation-table order within each capability. Input coverage-reference order
is preserved in the description, but does not silently change this documented
query ordering. Unknown mechanisms cannot be constructed through the closed
enumeration; external string lookup uses the catalog's typed lookup result.

## Controls before definitions

Freeze a valid complete 27-mechanism fixture with shared capability references,
then separate controls for every ordered failure and exact budget boundary.
Include an unreferenced disconnected two-vertex cycle, a converging DAG, a
zero-fuel empty graph helper case, nonempty zero-fuel refusal, duplicate READY
coverage, missing isolated/PARKED coverage, duplicate global definitions,
legitimate shared capabilities, malformed and all-zero digest cases, and
stable query ordering. Multi-fault controls must freeze phase precedence.
The full API cannot accept a coverage input lacking all capabilities and
obligations merely because every mechanism name is present.

Independent contract/control review and missing-definition RED precede source
definitions. Plant ignored disconnected cycles, collapsed shared references,
omitted isolated mechanisms and unfinished traversal success. The complete
Evidence component additionally requires evidence-report description admission;
neither this module nor the catalog alone satisfies R4-17.
