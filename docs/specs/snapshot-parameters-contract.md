# SnapshotParameters: complete pure parameter admission and projection

Proposed normative contract, revision 2, 2026-10-21. The original reviewed revision remains byte-identical at `/tmp/r4-snapshot-parameters-api-proposal-20261021.md`, SHA-256 `9706b3a2f01d1f871336258c55fba9d2aef5c6c56764f5148d9eebe0bb1ffb9d`. This source correction separates root scale ±2 from gate scale ±1; no API names, output types or error order change. This elaborates the approved full-scope artifact work and `/tmp/r4-snapshot-artifact-admission-contract-20261021.md` (`3881bc94f805e0c8d40a421f1cb1b20c35df623fbd624aa17b4dabfce9cef8d4`). It is design only: no requirement registration, source definitions, compiler invocation, model/reference execution, universal proof or runtime acceptance is asserted. The parent must review the API before independent literal controls are frozen and missing-definition RED occurs.

## 1. Complete component and its boundary

`R4.SnapshotParameters` accepts a complete bounded storage-typed Snapshot through the actual SnapshotJson admission and encoding-pair APIs. It validates the complete pure parameter predicate, constructs the actual canonical Binder plan, converts every flat eight-coordinate root and every gate exactly, and returns the original Snapshot plus all Engine-compatible parameter fields. It never substitutes a root, drops a remainder, truncates an inventory, reinitializes parameters from the seed, or removes inactive gates/routes.

It returns no `Learning.Binding`, `Learning.ModelState`, `Engine.FreshModel`, carried batch, proposal, executable handle, trusted digest or instance counter. A successful value is an ordinary freely constructible semantic record. The component is complete as a parameter projection; it is not an implementation of effectful `Device::from_snapshot`. Hash-domain computation remains SnapshotDomains' separate operation. A caller cannot pass an asserted SnapshotDomains receipt to skip parsing or parameter admission here.

## 2. Source authority

Every UOR source link below selects revision `4611538bb4843947363ccb9cce5f388e32c46883`, inspected with `git show`; no upstream working-tree bytes were substituted.

| Source | SHA-256 | Relevant contract |
| --- | --- | --- |
| [snapshot.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/snapshot.rs#L10) | `c03dcb7bc0e0e966951285a52cf76bf672ee74bf136c42403b40d1a4b5da1170` | Lines 10–38: fifteen storage fields and exact Rust scalar domains; lines 40–73: hash domains distinct from admission |
| [device.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/device.rs#L1057) | `47acbd7398ac95e87d9e6b58594012e14d91016941adb90fa72563e41afdd6ae` | Lines 1057–1119: actual load order; 808–934: initialization and effects; 737–758: instance allocation; 1122–1139: exact GPU spec words |
| [lib.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/lib.rs#L57) | `ffa7d22a9471071a18aa33ff67376ee7f1fafefa4e774ddcd50210b7d54c2ad9` | Lines 57–78 Spec; 84–101 initialization; 110–187 ReadSpec validation/words/counts |
| [metadata.rs](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/metadata.rs#L27) | `27e53e4feecb391adb4a4db3462712a8d9375aed09c0e57086581134cb6e1e32` | Lines 27–71 canonical construction; 103–177 complete structural validation |
| [discrete.cu](https://github.com/UOR-Foundation/uor-r4/blob/4611538bb4843947363ccb9cce5f388e32c46883/crates/uor-r4-discrete/src/discrete.cu#L452) | `8cc15ccb605364e8ae4239a1852fc71a08fcfb747c515269b8cca7eecdbf190e` | Lines 452–463 complete runtime parameter predicate; 24–27 table layout; 78–89 and 181 gate layout |

Local source pins inspected for this contract:

| Module | SHA-256 |
| --- | --- |
| SnapshotJson (reviewed isolated stage) | `cf26668c78b89f1114dc3dbece6c5a3645e90d423bb844789105f4a52f314430` |
| Types | `c140409edf9a9fd2490a35f1692dfedb7577a6181165b9dc141773e609b5b456` |
| Configuration | `7c367ee55b39eb8028b277153fc4f8e573f7bc0dbfcae9a8b7f8f3196475dc88` |
| Geometry | `2b934cd1ff5eccf6c8d5b4af1c34901fa964d31958e8a435cdc05bb6e52c7167` |
| Binder | `2b66660f539c88fe2ce23229300c31bf7bbfba5ab83e9b23bd450bb237f6a449` |
| Gates | `92dfd0242c216b415420961eb55dab9be96d13676e503d616b4a76e622359f52` |
| Objective | `379e2a52851628a5cc762c6cd911a18d97a3d30df8b9eecdb86542b82fe441ec` |
| Learning | `3667322e441d1b689dee0eba8bfb7b512a456ae9781ee7063160dc7f98c25d32` |
| Engine | `fe1dd371f90db233d4b8675e5dbdddff982382f83c20f619d3ca5cf8acb31f40` |

SnapshotJson's approved API is `/tmp/r4-snapshot-json-codec-api-proposal.md`, `ca21d9308ca83ed5460a7e9e08a7c31cdf36ac414ca6833ff84fb194b784692a`. Its final integration evidence remains separately owned. Later source changes require a reviewed pin update; this proposal does not silently select an unreviewed derivative. Existing `docs/specs/artifact-serving-design.md` and `production-export-contract.md` retain all artifact/runtime obligations.

## 3. Exact public types and functions

All new declarations belong to `R4.SnapshotParameters`. Names below are normative. Reuse existing named types; do not introduce numeric encodings of the existing enums.

```text
SnapshotParameterArray = table | routes | gates

SnapshotParameterError =
    codec(R4.SnapshotJson.SnapshotJsonError)
  | configuration(R4.Configuration.Validation)
  | binderContext
  | inventory(SnapshotParameterArray, Nat expected, Nat actual)
  | root(Nat rootBlockIndex)
  | route(Nat flatIndex, Nat actual)
  | gate(Nat flatIndex, R4.SnapshotJson.SnapshotSigned32 actual)
  | invariantFailure

SnapshotParameterProjection = {
  snapshot: R4.SnapshotJson.SnapshotValue,
  spec: R4.Types.Spec,
  read: R4.Configuration.ReadSpec,
  policy: R4.Objective.ObjectivePolicy,
  initialization: R4.Configuration.ReadInitialization,
  seed: Nat,
  epoch: Nat,
  plan: R4.Binder.BinderPlan,
  codebook: List R4.Geometry.Root,
  routes: List Nat,
  gates: List R4.Gates.Trit
}

SnapshotParameterResult =
    parameters(SnapshotParameterProjection)
  | error(SnapshotParameterError)

snapshotProjectParameters(
  profile: R4.SnapshotJson.SnapshotJsonProfile,
  value: R4.SnapshotJson.SnapshotValue
) -> SnapshotParameterResult

snapshotDecodeParameters(
  profile: R4.SnapshotJson.SnapshotJsonProfile,
  rawBytes: List Nat
) -> SnapshotParameterResult

snapshotDecodeStrictParameters(
  profile: R4.SnapshotJson.SnapshotJsonProfile,
  rawBytes: List Nat
) -> SnapshotParameterResult
```

Constructor argument names in the error notation describe ordered payloads, not additional record fields. `configuration(accepted)` is never emitted; the constructor reuses the existing full Validation type and only actual nonaccepted results are wrapped. `binderContext` and `invariantFailure` provide total refusal for internal impossibilities; neither is a successful default. Successful public calls must not depend on reaching those branches.

No public function accepts a caller-authored plan, converted codebook, counted inventory, digest, instance, candidate projection, or expected runtime header. No `load`, `admitDevice`, `authenticate`, or `toModelState` alias is part of this module. Future consumers must either use the result from their own actual call or recheck all fields; its public constructor is not a seal.

## 4. Normative order and pure acceptance predicate

The same complete stages run for all three public roots after their respective stage 0. Every refusal returns only `SnapshotParameterResult.error`; there is no partial parameter record.

0. **Complete bounded storage admission.** `snapshotProjectParameters` calls actual `SnapshotJson.snapshotNativeEncodingPair`; compatible raw calls actual `snapshotEncodingPair`; strict raw calls actual `snapshotStrictEncodingPair`. Propagate the exact `SnapshotJsonError` in `codec`. Use the successful pair's `snapshot`, not an unvalidated original record. Both encoding budgets therefore apply before parameter validation. This deliberately makes the codec/profile boundary identical to SnapshotDomains and is a bounded application refinement, not source Device allocation/error order. The original earlier sketch allowed decode-only composition; this proposal explicitly chooses complete encoding-pair composition for all roots. No SHA is computed here, and the encodings need not be retained in the result. Raw duplicate/escape/whitespace semantics remain those of the corresponding actual entrypoint.
1. **Exact enum/field conversion and read/spec validation.** Convert the four Spec fields without change; map ReadSpec constructors preserving kept/sweeps. Call actual `Configuration.validateRead`. Reject its first nonaccepted result in `configuration`. Thus invalid Spec precedes mode limits; Relation kept precedes Relation sweeps. Storage admission already excluded non-u32 source fields.
2. **Canonical plan.** Call actual `Binder.construct(spec.context)`. On `Construction.invalidContext`, return `binderContext`. On `built(plan)`, use that complete plan; do not synthesize only a stage count or accept an equivalent noncanonical forest. Its stages must be the actual constructed `plan.stages`. A defensive mismatch against the admitted context-stage law may return `invariantFailure`, never replace the plan. Such a mismatch is a dependency defect, not an input-dependent source error.
3. **Complete exact inventories.** Derive root count `vocabulary*blocks`, table-coordinate count `8*vocabulary*blocks`, route count `blocks*plan.stages`; derive gate count by actual `Configuration.gateCount(read,blocks,plan.stages)`. Unexpected overflow/outsideWord is `invariantFailure` after these admitted bounds. Compare actual full list lengths in order table, routes, gates. The first mismatch returns `inventory(array,expected,actual)`. Counts are not provided by the caller. All inventory errors precede initialization or coordinate/route/gate contents. This deterministic three-way diagnostic refines one combined source allocation error.
4. **Initialization/read compatibility.** Map the storage initialization constructor, then call actual `Configuration.validateInitialization(initialization,read)`. Reject the nonaccepted result in `configuration`; activeLeafV1 + Route returns activeRoute. This check follows inventories, as the source load reaches initialization only through `new_with_initialization` after its inventory checks. Do not demand that restored gates match the initial policy or that restored table/routes match seeded initialization.
5. **Complete table projection.** Walk all table coordinates in consecutive groups of eight, token-major then block-major. For each group convert exactly 0, +2, -2 to Geometry coordinates; reject any other i32 value as `root(groupIndex)`. Construct the eight-field root and require actual `Geometry.rootIndex(root)` to return some canonical index. Reject off-orbit roots at the same group index. Preserve the actual coordinate record, its order and duplicates; do not canonicalize by nearest root, reorder by root ID, deduplicate, or normalize sign. A trailing partial group cannot occur after exact inventory checks; if a structural helper sees it, return `invariantFailure`. No arbitrary fuel or incomplete-success exit is allowed.
6. **Complete route admission.** Walk the entire original route list left-to-right. Reject the first value >=64 as `route(index,value)`, even in Flock or Relation mode. Preserve the exact list and all inactive entries.
7. **Complete gate projection.** Walk every gate left-to-right, mapping exactly 0, -1, +1 to Gates.zero, negative, positive. Reject any other value as `gate(index,originalSignedValue)`. Preserve all Route inactive gates, all per-block gates and all Relation extra gates.
8. **Return exact data.** Preserve the complete admitted Snapshot as `snapshot`, including all six uninterpreted header strings and every original typed scalar/list. Return its mapped spec/read/policy/initialization, unchanged seed and epoch, full canonical plan, codebook, unchanged routes and converted gates. No initializer, read, writer, score, prediction, parameter update, cryptographic assertion or runtime allocation is executed by this projection.

Header strings are six fields: schema, operatorKappa, compositionKappa, kernelSha256, sourceSha256, geometrySha256. Valid Unicode text is sufficient here; schema mismatch, empty/nonhex text or invented source strings do not cause rejection. That deliberate acceptance must be tested so an implementer cannot quietly turn an equality-to-caller-data check into authentication.

## 5. Exact scalar, shape and ordering laws

- SnapshotSigned32.nonnegative(n) denotes n, with n<=2147483647 after codec admission. SnapshotSigned32.negative(n) denotes -(n+1), with n<=2147483647. There is no negative-zero representation. Root-coordinate -2 is negative(1), +2 is nonnegative(2), and 0 is nonnegative(0). Gate -1 is negative(0), +1 is nonnegative(1), and 0 is nonnegative(0). Thus ±1 are refused as root coordinates, while ±2 are refused as gates. These distinct scales must never be collapsed.
- Seed is unchanged u32, including 0 and 4294967295. Epoch is unchanged u64, including 18446744073709551615. A loaded maximum epoch is admissible; a later improving transition has its separate increment refusal. No wrapping or artificial epoch reset is permitted.
- Spec: vocabulary2..8192, blocks2..16, context1..243, keptCells1..min(16,vocabulary). Flock kept1..8 and Relation kept1..8/sweeps1..5 are independent configured bounds: kept need not be <=vocabulary or the actual eligible count. The existing read algorithms handle actual found count.
- Stage count is `1 + floor(log_3(context))` on the admitted interval, defined constructively by the actual Binder plan: 1 for1..2; 2 for3..8; 3 for9..26; 4 for27..80; 5 for81..242; 6 for243. The source has no logarithm operation; this mathematical expression describes the count only. Every node, atom, child, level and frontier entry must be preserved in `plan`.
- Codebook index = token*blocks+block; coordinate index = (token*blocks+block)*8+d. The inverse coordinate-flattening of the returned roots must equal the entire admitted original table.
- Route index = block*stages+stage. Preserve all route words <64; do not replace inactive Flock/Relation routes with identity.
- Ordinary gate index = block*stages+level. **Relation extra gates are level-major:** base `blocks*stages + level*5 + edgeType`, with edgeType0..4. They are not `base + edgeType*stages + level`. The extra list is retained unchanged in this order; projection does not need to evaluate any edge.
- Route/Flock gates = blocks*stages. Relation gates = blocks*stages + 5*stages. Source gate_count has checked u32 products/addition; admitted bounds guarantee rootBlocks<=131072, tableCoordinates<=1048576, routeWords<=96 and gateWords<=126. These counts fit both 32- and 64-bit usize and u32; no platform-dependent wrapping is introduced. All scalar arithmetic here is mathematical Nat, not a D11 code-generation claim.
- Read mapping: route→Configuration.route; flock(k)→flock(k); relation(k,s)→relation(k,s). Objective nearestAddressMarginV1→Objective.nearestAddress; occupiedCellRankHingeV1→Objective.addressRank. Initialization maps both identically named cases exhaustively. No default catch-all is legal.

## 6. Exact relationship to Engine, without manufacturing custody

Let P be a successful projection. For any **separately supplied** Learning.Binding B whose spec/read/policy/initialization/seed equal P's mapped fields, whose modelInstance is within u64, and whose two Digests each contain eight admitted u32 words, define the mathematical record M with binding B, epoch=P.epoch, codebook=P.codebook, routes=P.routes and gates=P.gates.

The required structural relation is `Learning.stateAdmitted(M) = true`, so actual `Engine.admitModel(M)` returns the exact M. Establish this through the actual source predicates, including `Binder.plan.stages = Learning.contextStages(P.spec.context)` across all243 admitted contexts, all lengths, rootIndex membership and route admission. Do not claim a universal theorem merely from a finite set of example states. The source proof-language/none-axiom limitations remain applicable; general obligations must be identified and verified lawfully before being called proved.

This relation does **not** say B is authentic. `Learning.bindingAdmitted` currently checks a word64 modelInstance, allowing both0 andMAX as structural values; the fresh allocator is stricter. Its digest predicate checks eight u32 words, not actual compiled bytes, canonical hex, a source revision or an image. `Engine.admitModel` only calls stateAdmitted and returns its input. It has no schema/kappa/kernel fields to compare. A diagnostic control showing acceptance under arbitrary well-shaped digest words is useful evidence of that limited boundary, never a provenance test.

`Engine.admitFreshModel(next,M)` first requires stateAdmitted(M), then allocates only when 0<next<UInt64MAX, replaces the binding's instance and returns next+1. This source-semantic transition neither owns next nor reproduces Device's effectful failure order. This module will not call it and will not export a fake fresh instance.

Digest conversion is deliberately absent. Snapshot hash strings are preserved as text even when nonhex; converting them to Learning.Digest would add a stricter surface and an endian convention not present in this projection. Future runtime binding must separately establish actual compiled source/geometry identity and private fresh instance custody before any generated root exposes model use.

## 7. Source load obligations intentionally not claimed here

Actual Device load first validates read/spec, constructs the canonical plan, then checks six headers in short-circuit order against SCHEMA, generated kappas, actual ordinary PTX SHA, complete compiled source SHA and packed geometry SHA. It then checks inventories, calls new_with_initialization (which repeats read/init/plan checks), and allocates a fresh process model identity **before** CUDA context, target admission, module/function loading, storage allocation and initial kernel work. Later failures consume that instance; they do not roll back the atomic counter. The allocator starts at1, checked-add refuses MAX, and it never legitimately allocates0.

The constructor initializes table/routes/gates on CUDA and validates declared initial gate storage; load then overwrites complete table/routes/gates/epoch with the stored parameters and invokes discrete_validate. Active-leaf initial gates may therefore differ from the restored gates. Restored codebook/routes need not reproduce the seed. Those distinctions must remain in the eventual effect model.

CUDA validation checks the same root, route and trit acceptance predicates plus read/policy metadata. Failed roots publish6, routes7, gates/metadata10 through atomicExch. With multiple invalid entries the published nonzero status may race; this pure module's table→routes→gates and left-to-right error order is a deterministic diagnostic refinement, not CUDA multi-error status parity. After actual source read/spec/typed-enum checks, its metadata validity follows; initialization provenance is checked on the host constructor, not inferred from the validator.

Complete runtime import still needs actual built bytes (not a snapshot's own expected strings), target/image admission, resource effects/failures, fresh privately owned handles, full epoch/batch/proposal invalidation, and genuine GPU admission. This contract defines no table-native magic, version, exporter, decoder or serving root and does not resolve upstream UORLUT01. Full artifact, D11 lowering, browser durability and Pages obligations remain required.

## 8. Independent predefinition control plan

Freeze expectations by literal AST/data reasoning and pinned static metadata only. Do not execute a model, constructor, reader, writer, initializer or parity reference on the host. Each complete success control must compare the entire projection record, including the original Snapshot, mapped fields, full Binder plan and every converted list; checking only a count or Boolean is insufficient.

1. **Small complete Route base.** Literal Spec(vocabulary2,blocks2,context1,keptCells1), zero initialization, either policy, seed0, epoch0; table consists of four literal roots r0,r1,r2,r3 in token/block order; routes[0,63]; gates[0,-1]. Expected plan has context1, one atom0/level0 node/no children, frontier[0], stages1. Expected codebook is exactly those four root records, gates[zero,negative]. Set all six header strings to distinct arbitrary valid text. This must succeed, demonstrating inactive Route gates need not be zero and metadata is preserved without authenticity claims.
2. **All canonical roots and invalid eight-vectors.** Freeze all16 roots from the independently reviewed geometry inventory as literal coordinates, never by calling the proposed converter to make expectations. Exercise each root at first and later block positions; duplicates and ordering preserved. Reject zeros, one nonzero coordinate, valid {-2,0,2} coordinates at a noncanonical pair, three nonzeros, i32 extremes, ±1 and ±3. Distinguish first failing block and coordinate content from prior length refusal. A root group error reports its flat root-block index.
3. **Cross-token/block ordering.** Use at least vocabulary3/blocks3 with deliberately distinct ordered roots and one repeated root; expected flattening and full output are literal. This catches transpose, reversal, chunk overlap and deduplication without executing geometric operators.
4. **All64 route ordinals.** Complete snapshots with <=96 route capacity must preserve0..63, including Flock/Relation inactive routes. Refuse64/U32MAX at first/middle/last positions, never wrapping. Mixed root/route/gate defects exercise deterministic order.
5. **Every trit and Relation tail layout.** Complete Flock and Relation snapshots include all three signs in ordinary and extra positions. At context3/blocks2, stages2: four ordinary gates plus ten extra gates. Freeze distinct patterns for level0 edge0..4 followed by level1 edge0..4, so an edge-major permutation changes the full expected record. Refuse±2 and signed endpoints including the final Relation slot; preserve Route inactive signs. ActiveLeafV1 with Flock/Relation succeeds even when restored gates differ from active initialization.
6. **Read/spec/init bounds and error priority.** All first-outside bounds for vocabulary, blocks, context, keptCells, Flock kept and Relation kept/sweeps; Relation kept before sweeps; Spec before read; all inventories before activeRoute; table before routes before gates; initialization before root content. Flock/Relation kept8 with vocabulary2 is accepted. For malformed native numeric/scalar values, codec typing precedes invalidSpec/inventory. Full tuple combinations distinguish each adjacent priority stage.
7. **Complete canonical plans.** Independently freeze context1,2,3,8,9,26,27,80,81,242,243 full plans from already reviewed Binder literal controls/source-derived metadata. Also enumerate all243 context-to-stage expectations and bound relations as finite metadata controls, without inventing a caller-stage input. The actual constructed plan, including every node level, is compared in full where selected; stage-only controls do not replace plan construction.
8. **Inventories.** Empty, exact, one-short, one-long and trailing-partial table lengths; route/gate off-by-one for every mode; complete Relation extra count; maximum symbolic count laws131072/1048576/96/126 with no fake caller-count admission. Use feasible exact list fixtures for selected complete successes; state clearly if huge full-list controls are not run, never cap the implementation or claim all-size proof from symbolic lengths alone.
9. **Seed/epoch/policy preservation.** Both policy constructors, both allowed initialization constructors, seed0/MAX, epoch0/MAX. Seed changes must not reinitialize the codebook; epochMAX must not overflow or reset. Storage out-of-domain nats fail exact codec errors before projection. Gate signs are canonical signed predecessor values, including -1=negative0.
10. **Actual codec boundaries.** Reuse independently frozen raw/native Snapshot cases and exact error paths. Include invalid profile, invalid output budget, over-budget valid shape, malformed UTF8/JSON, duplicate compatibility success versus strict refusal, positional forms, unknown/missing fields and complete consumption. All three public roots use actual encoding-pair functions, so both stored/typed output-budget errors precede geometric admission. Successful raw forms preserve the same typed Snapshot but make no raw-byte or storage-hash claim.
11. **Structural Engine relation.** Small literal projection records paired with externally stated well-shaped Bindings, including instance0/MAX and arbitrary digest words, must exercise actual Engine.admitModel and preserve the exact state. Keep these explicitly structural. Add genuine malformed digest/instance companion controls against Engine predicates without implying the projection itself accepts such a binding. General validity/losslessness obligations are separate from these finite checks.
12. **Losslessness and refusals.** Freeze coordinate flattening and gate inverse literals independently; prove/verify they recover all source values on accepted projections. No public forged projection is accepted for bypass. Every failure leaves original inputs unchanged; the API is pure and returns no partially admitted value.

Potential source-valid plants after positive verification: accept any {-2,0,2}-valued eight-vector instead of actual root membership; convert negative0 to zero; transpose token/block order; omit Route gate validation; omit Relation tail; transpose Relation tail order; check initialization before inventory; reset epoch; initialize codebook from seed; reject arbitrary source text as fake authentication; and use a supplied stage/count instead of actual Binder construction. Each plant must target a concrete immutable distinguishing control, source-check successfully, then fail semantically under the unchanged none-axiom policy; preserve setup/resource failures and restore exact source.

## 9. Acceptance obligations and sequencing

This document is a complete contract proposal, not an accepted implementation. Review it, assign its own requirement/suite, freeze exact literal controls, observe genuine missing-module RED, then implement in isolation. Intrinsic spec and code/security review must precede formal integration. The official compiler, fixed limits, complete declaration audits, fresh input/build/olean hashes, kernel replays, mutation rejections/restored positives and mandatory Rust receipt gate remain required. Do not relax axioms, output limits, expected statements or source domains to force acceptance.

The mathematical general obligations are exact acceptance equivalence to the pure source predicate with identity/effects explicitly removed; source-field/flattening losslessness; stage law/overflow bounds; and conditional Learning.stateAdmitted preservation. Existing unsupported constructive proof composition is a real dependency if it prevents those general statements. Finite controls and source inspection must not be relabeled as universal proofs. No part of this component releases training, supplies CUDA parity, authenticates source hashes or establishes deployment.
