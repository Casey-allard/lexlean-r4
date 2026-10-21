# Production export contract

Status: source review on 2026-10-21; proposed implementation contract, not an
export, execution, deployment or production acceptance receipt. No R4 native
inference, model evaluation, training or GPU computation was performed for this
review. Formal source acceptance does not establish executable acceptance.

## Exact authorities

The mechanics remain [UOR-R4 `4611538bb4843947363ccb9cce5f388e32c46883`](https://github.com/UOR-Foundation/uor-r4/tree/4611538bb4843947363ccb9cce5f388e32c46883).
The initial language review used [LexLean `6bcc8d59836506d6365dcfa4c755d068771c4f93`](https://github.com/UOR-Foundation/LexLean/tree/6bcc8d59836506d6365dcfa4c755d068771c4f93)
and [PrismPM `590ec8a9c41d46cd61dea567130f8f8ef9bca26e`](https://github.com/UOR-Foundation/PrismPM/tree/590ec8a9c41d46cd61dea567130f8f8ef9bca26e).
The executable-construction source inspected here is instead
[PrismPM `dfdc4df770d038ee5fb700d374aca883d1c9ec7a`](https://github.com/UOR-Foundation/PrismPM/tree/dfdc4df770d038ee5fb700d374aca883d1c9ec7a),
the source of [unpublished construction run 39492842177](https://github.com/UOR-Foundation/PrismPM/actions/runs/39492842177).
Its [vendored LexLean identity](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/.cargo_vcs_info.json)
is `0865df4e540ca643b090a29e9df6d319d6906da6`, not the standalone reviewed
LexLean revision. All compiler links below select this exact construction
revision. Construction integrity and development verification are distinct from
publication and production acceptance; see [UPSTREAM.md](../UPSTREAM.md).

## Two compiler contracts must both be satisfied

LexLean's language-1.2 production registry exposes `rust-core` and `rust-std`.
Both realize naturals in 64 bits; the former excludes heap allocation, while
the latter permits it. `Nat`, `Bool`, lists and recursive definitions have
runtime dispositions, but lists admit allocation and recursion admits an
input-dependent stack effect. Required effects must be explicitly admitted by
the production root. A list-rich closure is therefore not an unchanged
`rust-core` program. These are eligibility rules, not an assertion that a
particular complete R4 closure exports successfully.
[Registry](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/language/production-1.2.toml),
[eligibility analysis](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/src/production/eligibility.rs#L2858-L2908).

PrismPM's modeled library and browser construction also depend on its separate
vendored `lean4-prod` exporter and `prod-codegen`. Eligibility in LexLean's
registry is not a successful export through that pipeline. Its actual codegen
maps `Nat` to `u64`, rejects mathematical `Int`, uses checked arithmetic, and
propagates fallibility through the call graph. No bignum fallback exists.
Application signed-natural representations remain subject to the bounds on
their natural magnitudes and intermediate expressions.
[Actual codegen contract](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lean4-prod/rust/prod-codegen/src/lib.rs#L1-L104).

The application has completed its independently reviewed official language-1.2
development migration: glossary/schema changes and 23 local binding renames
preserve declaration semantics, controls and the none-axiom policy. The
seventeen-component checkpoint at `eeb3990` passed 8,270 controls, 10,419
none-axiom declaration audits and 98 kernel replays. The prior sixteen-component
workspace suite and the later JSON target tests have distinct receipts; see
[the migration record](../LANGUAGE-MIGRATION.md) and [JSON integration](../JSON.md).
This migration does not establish named production roots, effect admission or
export acceptance. The inspected SDK already admits `UInt8`, other fixed
integers and `Bytes` in language 1.1; byte codecs alone were not the migration
requirement. Production-root annotations and the `executable` declaration flag
are gated to language 1.2 and still require explicit application authoring and
complete production-closure checks. [Type-version gates](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/src/ir/semantic.rs#L2629-L2662),
[production declaration gates](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lexlean/src/ir/semantic.rs#L4934-L4949).

## Concrete generic exporter blocker

The named-root validator strips namespaces from every included definition and
rejects duplicate leaf names with `PP5004`. The code generator also rejects
different named types with the same final component. This affects the complete
reachable closure, not merely the selected entry function.
[Definition-name validation](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/crates/prismpm/src/library_build.rs#L120-L149),
[type-name validation](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lean4-prod/rust/prod-codegen/src/lib.rs#L370-L389).

For example, the reviewed Engine runtime read dispatch reaches both
`R4.Route.run` and `R4.Flock.run`. Their distinct meanings share the leaf
`run`. Other common helper and type names need the same closure audit. This is
a source-derived export blocker under the pinned naming rule; no failing R4
export was executed to establish it. A new uniquely named outer wrapper does
not remove collisions in its dependencies.

Preferred upstream repair: preserve injective full-name identities throughout
Lean lowering, IR definitions and named types, constructors and references,
generated Rust symbols, named-root coverage and ABI selection. Add independent
cross-namespace function/type collision controls to that generic compiler
change. Its acceptance requires a newly identified official SDK. The possible
application-only alternative is a complete semantics-preserving rename of all
colliding source declarations and references, with regenerated proofs and
locks. That is a broad source migration, not an adapter or waiver. Do not
silently drop reachable branches, patch generated Rust, or bypass coverage.

## Required authoritative application roots

The following belongs in pure LexLean application source and the PrismPM
application model, rather than handwritten host computation:

1. Declare exact named external roots and their complete reachable closures.
   Do not expose internal supplied-loss, supplied-proposal or unchecked helper
   constructors as application capabilities. Route all public requests through
   Engine admission and actual read/writer/counterfactual computation.
2. Define bounded, versioned byte request/response codecs, including scalar
   widths, lengths, endian rules, identity fields and typed errors. Decode,
   validate, call the authoritative Engine root, and serialize its result in
   LexLean. Reuse reviewed Words and accepted hash/protocol implementations;
   do not create a host mirror of those algorithms.
3. State a source-safe arithmetic domain for each external root. Bounds must
   cover every intermediate expression, not only accepted results. A formal
   `Nat` expression that adds before comparing to UInt64 MAX can fail in
   generated arithmetic before its modeled error branch is reached. Prove the
   admitted domain excludes that overflow, or refine the source operation to
   checked bounded behavior and prove the correspondence. Reject out-of-domain
   byte values before narrowing.
4. Separate serialized application errors from compiler/resource failures.
   Generic overflow, output exhaustion, stack failure and allocation failure
   cannot be represented as a successful application receipt or silently
   reclassified as an Engine domain error.
5. Bind requests to the exact admitted model instance, batch identity and both
   epochs. Preserve current virtual vector/gate/frame pairing and complete
   candidate flow. Semantic binding checks already modeled by Engine are not
   unforgeable custody of an external mutable model handle.

The current core-Wasm generator accepts exactly a single
`List UInt8 -> List UInt8` or `Bytes -> Bytes` entry. Engine's structured,
multiargument functions do not themselves have this ABI. The generated ABI
has explicit input/output capacities and memory limits; checked computation
failures trap at that boundary. The byte wrapper must serialize expected
application failures before returning; it cannot turn a trap into a valid
result. [Wasm entry and failure contract](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lean4-prod/rust/prod-codegen/src/core_wasm.rs#L55-L127).

## Generic browser runtime dependency

At the reviewed construction revision, `require_runtime` unconditionally
rejects every Browser application with `PP2011`: runtime orchestration,
effective-grant admission, credential custody and durable uncertain-operation
recovery are unimplemented. The browser compiler remains private and
explicitly unaccepted. This is a generic upstream dependency; changing an
application label or calling a private builder would not satisfy it.
[Runtime gate](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/crates/prismpm/src/holo/browser_application.rs#L25-L31),
[private browser compiler](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/crates/prismpm/src/browser_build.rs#L1-L28).

The required generated runtime must own private model and batch handles,
atomic conditional updates, durable record custody and recovery, and storage
and capability admission. The application must supply the modeled rules and
protocol roots for those actions. Neither alone substitutes for the other.
An immutable Pages viewer or isolated byte-function demo does not establish
the requested durable browser product. Training remains held independently.

## Memory, instruction and execution boundaries

The separate code generator lowers list parameters to borrowed slices and
list returns to caller-provided buffers, but nested owned list fields use
`Vec`. Some owned self-tail patterns become loops; other shapes may remain
recursive or be rejected. General total `Nat`/`List` source is therefore not
a guarantee that this complete closure exports, uses bounded stack, or meets
the source project's D11 serving restrictions. The Wasm allocator also has
no-op deallocation; per-call ABI capacity does not establish indefinite
instance lifetime without a reviewed reset/lifetime strategy.
[List and recursion policy](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lean4-prod/rust/prod-codegen/src/lib.rs#L14-L104),
[Wasm allocator](https://github.com/UOR-Foundation/PrismPM/blob/dfdc4df770d038ee5fb700d374aca883d1c9ec7a/vendor/lean4-prod/rust/prod-codegen/src/core_wasm.rs#L220-L268).

Where D11 requires a fixed serving layout, author a bounded/preallocated
LexLean representation and a correspondence proof to the admitted semantic
model. Inspect the generated instruction and allocation paths under their own
acceptance procedure. Do not replace the algorithms with a Rust implementation
or claim that generic compiler support proves D11 compliance. Neither the
LexLean production registry nor this Rust/Wasm pipeline provides a CUDA target.
Research evaluation/parity and training retain their separate GPU, fitness and
owner-release requirements; static export/build checks do not clear them.

## Delivery gates

The artifact format is a separate unresolved contract. Pinned UOR explicitly
records that no lawful current discrete model has been exported through its
integer container, that an addressed-memory integer read kernel is absent,
and that a table-native `UORLUT01` schema remains an open question. The historic
dense stack format is not a current Engine snapshot serialization and must not
be adapted as though that semantic bridge already exists.
[Pinned export boundary](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/docs/mechanisms/lut-serving.md),
[historic stack format](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-integer/src/stack/format.rs).

A separate versioned table-native freeze contract must bind the complete
discrete state, source/kernel/geometry identities and its admission rules to
the generated serving representation. Preserve the distinction between typed
state identity and storage-byte integrity identity from the pinned discrete
snapshot path; load must establish the appropriate new instance and admission
conditions, not trust a copied runtime handle. Generic fixed-container codegen
may support that contract but does not define it.
[Discrete snapshots](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/snapshot.rs),
[device snapshot admission](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/device.rs#L1036-L1118).

The generated serving artifact must specifically exclude multiply, divide,
float and transcendental instructions, exclude steady-state allocation, and
retain selected addressed reads. A `rust-std` build or fixed-width arithmetic
proof alone proves none of these machine-level properties. This is the
existing charter bar, not a new implementation restriction.
[Charter section 3.6](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/CHARTER.md#L29).

| Gate | Required evidence | Present review |
| --- | --- | --- |
| Semantic source | Independent controls, exact source and axiom policy | Existing slice evidence, separately recorded |
| Production authoring | Official migration, explicit roots/effects, full closure | Development migration passed; production roots/effects and closure acceptance remain unperformed |
| Generic export | Injective names, supported IR shapes, exact coverage | Naming blocker identified; no export executed |
| Bounded realization | Input/intermediate bounds and representation correspondence | Required contract, not established |
| Generated artifact | Official SDK identity, Rust/Wasm compile and ABI receipts | No R4 artifact built in this review |
| Stateful browser | Accepted generic runtime plus modeled custody/storage rules | PP2011 blocks current source |
| Production/Pages | Production locks, complete template checks and deployment receipt | Development construction is insufficient |
| Research capability | Separately authorized GPU evidence | No execution or capability claim |

The next export-oriented work should first settle the naming dependency and
the authoritative byte/root contract. Preserve the current formal model while
that generic compiler work is reviewed. Exact closure export and compilation
may then expose additional supported-shape boundaries; those must be reported
as compiler findings without weakening the application's semantics.
