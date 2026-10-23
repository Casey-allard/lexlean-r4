# Constructive prime ranks: proposed source and certificate contract

Status: architecture proposal, 2026-10-21. No new source definitions, prime
constructor execution, reference constructor execution, compiler invocation,
or capacity result was produced for this proposal. The bound 100000 remains
unaccepted until the obligations below pass. Counts 1 through 8192 remain the
complete public domain. This refines the existing CodebookRanks design; it
neither replaces PrimeZeta's floating construction nor supplies a rank table.

## Exact authorities

UOR revision `4611538bb4843947363ccb9cce5f388e32c46883`,
`crates/uor-r4-discrete/src/prime_zeta.rs`, SHA-256
`2f1ab07fe6094be6c3887c3b700868f89f8a63fcf311652c16ed4227be8b4e54`,
lines 110–138: reject count zero or greater than8192; start candidate2;
scan previously emitted primes in ascending order; stop scanning at p*p>n;
otherwise reject n at its first divisor; emit n only if the scan accepts;
advance2 to3 and thereafter by2 after every tested candidate, including the
last emission; then exit at the next while guard when the requested count
has been reached.
The source tests independently freeze ranks0,1,2,3,255,4095 as
2,3,5,7,1619,38873. They do not freeze the8192nd value or establish the
proposed100000 capacity. Read the exact pinned file rather than mutable HEAD:
https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/prime_zeta.rs#L110

Official construction compiler: LexLean
`0865df4e540ca643b090a29e9df6d319d6906da6`, vendored in PrismPM
`dfdc4df770d038ee5fb700d374aca883d1c9ec7a`.
Exact retained IR `/tmp/r4-0865-semantic-ir.rs` SHA-256
`e273dcc28a3d1b426bb97bef43cb426e2ccb7cbd205aab2a3b4ccf38c5623fbf`;
backend `/tmp/r4-0865-semantic-backend.rs` SHA-256
`d747f2819115fa4fb31b4c9c6e77ba0b0b26705d93a7f75ada209ca27f76295e`.
Relevant source: IR609–658 proof schema;7790–7936 proof checking;
8052–8177 elimination checking; backend1105–1254 proof lowering.
https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/src/ir/semantic.rs#L609
https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/src/backend/semantic.rs#L1105

## Recommended constructor shape

Retain the source trial-division transition, rather than introducing a second
runtime primality decision. Segment its candidate schedule and immutable
storage. The prior sieve proposal remains an alternative refinement, but it
requires an additional equivalence proof and is not needed for this first
source-faithful option.

Use the exact logical interval2..100000 inclusive, partitioned into390 full
windows of256 integers and a final159-integer window:99999 integers total,
391 windows. Window j begins at2+256*j. Its successor begins at the exact
exclusive endpoint; no overlap, skipped endpoint, or final dummy value.
Only2 and subsequent odd integers invoke the source transition. Even numbers
are skipped exactly as in the source schedule. Each window consumes at most
256 physical positions and returns its emitted primes as a bounded chunk.
After every tested candidate, perform the source candidate increment, even
when that candidate emitted the final requested prime. Only then inspect
the count at the next loop guard, including inside a window, and stop before
testing any later candidate. The final progress value retains this advanced
nextCandidate; the public result projects only its constructed prime chunks.

Suggested internal source values, to finalize before controls:

- `PrimeChunk = List Nat`, bounded by256 entries in a supported transition.
- `PrimeChunks = List PrimeChunk`, ordered chronologically, at most391 chunks.
- `PrimeProgress = { nextCandidate: Nat, emitted: Nat, chunks: PrimeChunks }`.
- `PrimeAdvance = continuing(PrimeProgress) | complete(PrimeProgress)`.

These are internal representations, not caller-provided authorities. The
public `codebookPrimeRanks(count)` remains `Result<List Nat, CodebookRankError>`.
It validates count first, creates its own empty state, and flattens the actual
constructed chunks from the completed progress only at successful return.
No public root accepts primes,
prime counts, segment outputs, cached certificates, or divisibility answers
from the caller. If an inspection helper accepts malformed explicit state,
its domain/refusal contract must be separately specified; that helper is not
an admitted construction root.

Prefer two nested structural traversals of window count and physical width,
with no unbounded semantic recursion or caller fuel. Trial division walks the
chronological chunks and their entries, preserving the p*p test before the
remainder test and the first-divisor early stop. A chunk may be appended with
bounded outer traversal; do not disguise an8192-deep flat-list append as a
bounded chunk update. The full public flat result still has up to8192 entries:
chunking internal computation does not guarantee that normalizing an equality
against this result fits the official limits. That remains a measured gate.

An arithmetic optimization that tests only a separately constructed base-prime
prefix through316 could be equivalent, but would change the runtime algorithm
and add a proof obligation. Do not silently substitute it here.

## Arithmetic, termination, and exact source-domain obligations

The proposed fixed candidate interval is an implementation boundary only after
capacity is proved; `primeBoundInvariant` may diagnose malformed helpers but
must be unreachable for every admitted public count. Exhausting391 windows
with too few primes is a failed implementation, not a legitimate partial result.

The prime at317 gives a useful independent small bound on trial-square
arithmetic:317^2=100489>100000, and317 has no divisor among
2,3,5,7,11,13,17 (remainders1,2,2,2,9,5,11). Every composite317 would have a
prime divisor at most17. Therefore a correct increasing prior-prime scan
for candidates through100000 either ends earlier or crosses the square bound
no later than317; it does not multiply arbitrary large prior primes. Prove
this connection, rather than assuming arbitrary stored entries are small.
Source candidate advancement is at most100001 and tested p*p at most100489,
so both fit u32. Supported counts, window positions and offsets fit both
usize32 and usize64. Prime ranks themselves do not need a host-width profile;
the later NormalNull cast-before-remainder permutation still does.

Mathematical soundness/completeness uses strong induction on candidates:

1. The state contains exactly the ascending primes smaller than the next
   source candidate, until the requested prefix has been completed.
2. If the source rejects n, an earlier emitted p divides n, so n is composite.
3. If n is composite, a prime divisor p satisfies p*p<=n and p<n. By the
   induction invariant it is present, and the scan cannot stop before it.
4. If n is prime, no scanned p divides it; scanning accepts it. The even skip
   loses no prime beyond2. Ascending candidate advancement preserves order
   and uniqueness. After the last emission, the transition advances the
   candidate once more before the next guard returns exactly that prefix;
   no subsequent candidate is tested.

This is an independent mathematical argument, not a claimed kernel proof.
The least-prime-divisor/factor-pair lemma and the induction invariant remain
formal obligations, as does the finite capacity statement. No exact value of
pi(100000) or of prime8192 is asserted by this proposal.

## Finite certificate packet: exact claims and dependency edges

Freeze certificate expectations independently of the candidate implementation.
A literal checkpoint is proof-only data, never a runtime rank inventory.
Do not derive expected checkpoints by executing the proposed constructor.
An independent static arithmetic certificate can contain composite factors
and prime non-divisibility witnesses, but its checker must verify every claim
using the actual pure source operations. Host-generated assertions are not
accepted facts. No witness generation is authorized by this document alone.

The proposed packet has four layers, each with exact input/output bindings:

A. Schedule and local arithmetic. Prove closed interval endpoints, final159,
window adjacency, initial2/next3, odd advancement including the final
emission-to-completed-state advance, divisibility arithmetic,
small prime witnesses and u32 safety. Freeze adversarial cases for p*p=n,
p*p>n before remainder, a first divisor, candidate2, and invalid count order.
These are literal `rfl` obligations, not a new general number-theory theorem.

B. Complete local transition coverage. For every window j, bind its exact
incoming progress and the exact outgoing progress under the actual window
function. Include every emitted or rejected odd candidate, exact first-divisor
witness or exact exhausted/square-cutoff branch, not only an output length.
Where a window is too large, divide its transition into contiguous subwindows;
all new endpoints and incoming/outgoing states must match literally. The
incoming state for window j+1 must be byte/AST-identical to window j's outgoing
state in the certificate inventory. Invalid/missing/duplicate windows fail.
Capacity must count actual emitted entries, not a caller-supplied count field.

C. Independent semantic checks. Cover the entire candidate interval with a
second specification of primality/compositeness, ascending order, uniqueness,
and no omissions. A nonemitted composite has checked2<=d<n,2<=q<n,d*q=n.
An emitted candidate has complete trial-divisor coverage through its square
bound. The latter establishes mathematical primality only once the general
factor-pair bridge is proved. The actual count of all emitted entries must
be at least8192; count metadata equality alone is insufficient. Every count
1..8192 has the actual public result length and the exact requested prefix.

D. Connection to the public root. Prove that the canonical runtime construction
starting from its own initial state follows all certified transitions and
returns the certified prefix for every admitted count. The root cannot consume
the proof-only checkpoint sequence as an argument or substitute its output.
Theorems in B and C do not discharge D merely because adjacent literals agree.

Each layer uses a sorted, unique inventory with all source/control digests,
exact statement ASTs and coverage ranges. Freeze it before any respective
implementation. Do not settle for selected ranks, sampled divisors, a bound
asserted as a Boolean constant, or only a cardinality test.

## What the current proof language can and cannot connect

The official schema permits `Reflexivity`, `Decide`, `Constructor`, `Cases`,
`Induction`, `Congruence`, `Apply`, `Simplify`, `LinearArithmetic` and two
restricted Boolean-reflection shapes. There is no arbitrary Lean/tactic text,
proof sequence, general `rw`, named local `have`, equality-transitivity form,
or general introduction/exact-local-hypothesis form.

A concrete lawful finite packet root is a balanced propositional conjunction
of all exact local equations. Each internal proof is `Constructor` with its
two corresponding children; each leaf is argument-free `Apply` of its already
verified closed theorem. This builds an actual connected proof of the full
conjunction, without handwritten Lean, by using the existing backend's
`constructor` and `exact priorTheorem` lowering. A391-leaf grouping has at
most9 conjunction levels; it may be split into modules to respect source-size
limits. Its proposition is the full set of equations, not an implied universal
public-constructor theorem. All dependencies must pass none-axiom audit and
kernel replay. Source availability is not proof that this specific packet fits.

`Apply` validates a prior document theorem and typed data arguments; it is not
an API to pass a local equality proof or compose Eq.trans/Eq.rec. `Congruence`
lowers to `congr 1` alone, without supplied subproof branches. Cases/Induction
eliminate Nat/List/document inductives, not arbitrary equality hypotheses;
induction binds hypotheses but the general proof-consumption path is limited.
Boolean reflection handles only its exact Nat list/record comparison shapes,
not primality, arbitrary transition relations or factorization. Linear
arithmetic is not a nonlinear primality/factorization solver, and its use still
requires actual none-axiom audit.

Retained official probes at
`/tmp/lexlean-r4-object-proof-policy-observed-20261021/{Nat,Bool,Result}.log`
each reject imported-equality Simplify with LLV7005/observed[propext]. The
backend emits `simp only [...]`. Therefore do not promise that rewriting the
391 checkpoint equations into the public-root equality meets the current
none policy. Likewise, proof-of-checker=true is not a proof of the independent
number-theoretic interpretation until its soundness bridge is established.

## Acceptance paths and required early gates

Path1 uses only the current official compiler. First probe the small proof
shapes for actual source transition, closed connection, and independent
primality interpretation. Then attempt direct `rfl` public-root certificates
that recompute the actual constructor, retaining all8192 admitted counts.
An exhaustive finite conjunction of8192 closed public-call certificates is
possible as a specification of this finite domain; it is not automatically
a quantified Nat theorem. A separate admission/domain-coverage theorem is
required if the published claim is quantified over Nat. Direct certificates
may repeat large normalization and fail the fixed limits. No feasibility or
speed claim follows from the count of conjuncts. Do not author a full runtime
module before the proof-connection feasibility is reviewed.

Path2, if direct connection cannot verify under the fixed limits, is an
explicit generic upstream proof-language dependency: typed equality
elimination/transport, transitivity, congruence with checked proof operands,
and local hypothesis/induction-hypothesis application, with ordinary kernel
proof terms and none audits. A verified generic arithmetic/prime-divisor
library or sufficient generic proof construction is also needed for the
semantic bridge. This must be an official source/API/release change, not a
project-local compiler patch, handwritten generated Lean or an axiom waiver.
It would allow B's exact step proofs to compose into D without reevaluating
all previous segments, but that benefit itself needs a real diagnostic.

Do not report the100000 bound, universal prime correctness or successful
construction for all8192 counts as accepted while any connection is missing.
The existing integer-word component may proceed independently; PrimeZeta's
complete software-binary64/transcendental/reference-platform obligations and
all production lowering/D11 restrictions remain separate and unresolved.

## Parent review correction: final candidate advance

The pinned Rust loop increments candidate after every test, including after
`primes.push` reaches the requested count. The prior proposal incorrectly
said to stop immediately at that emission. This revision preserves the
advance in the completed PrimeProgress and checks the stopping guard next.
For count1 the completed state therefore has emitted1, prime chunk[2], and
nextCandidate3. This is a source-order correction, not an accepted capacity
result or a proof that a final-advance optimization is valid. The original
proposal is retained separately with its original digest.
