# Actual first-atom proof results

The original first 32-byte leaf certificate and its **concrete production fold
state passed official compiler verification and independent artifact review**.
The intended wrong-inventory-ID control was rejected at the consumer theorem.
This establishes one local prefix state, not the full Object257 digest.

The frozen `README.md`, `candidate.json`, `structural-audit.json` and source
`manifest.json` retain their preparation-time **NOT_RUN** status unchanged.
The latest actual status is recorded here and in [results.json](results.json).
The separate `delivery-manifest.json` inventories the source and later evidence.

| Phase | Actual result | Verify time | Independent audit |
| --- | --- | --- | --- |
| `leaf-positive` | Check passed; verify exit 0 | 66,570 ms | 764 empty-axiom declarations, 9 kernel replays |
| `state-false` | Check passed; verify exit 1, `LLV7002` at Apply1 | 25,097 ms | Expected inventory ID starts 64; frontier remains 63; intended mismatch confirmed |
| `state-positive` | Check passed; verify exit 0 | 48,659 ms | 769 empty-axiom declarations, 12 kernel replays |

The actual leaf proof is Reflexivity over `ObjMakeLeaf [0,1,...,31]`, computing
the retained original record. The concrete consumer proves
`ObjFoldAtoms [[0,1,...,31]] [] [] originalLimits` equals the singleton inventory
and frontier with ID `[63,164,31,162,68,35,68,195,38,116,35,81]`. Its emitted
proof applies the checked Nat-family transport at 1, establishing the concrete
staging, carry and empty-tail conversion for this prefix.

The leaf attestation is
`f99cb5bd10766d0de7af04ba66cc25a1ab710a2fa1767cd5703f94f550d12bdd`.
The concrete-state attestation is
`57a540c76f1edcfbd4f8f58582f23e54e47ddbf7f4e55733ab09281ff324516e`.
See the complete [leaf review](evidence/leaf-native-review.json),
[state/negative review](evidence/state-native-review.json), and
[source review](evidence/code-review.json).

Selected actual check/verify receipts, source and lock observations, resource
postconditions, build manifests and emitted proof text are retained in `evidence/`.
The genuine phase development locks are adjacent to their configs. All three
phases have empty `postcapture-errors.json`; independent reviews found stable
source/locks, the same container epoch, zero memory events and passing sampled
storage reserves. These are sampled observations, not continuous high-water claims.

Full artifact trees and compiled binaries are excluded from this Git fixture.
The complete 95-file leaf and 125-file state artifact censuses and identities are
retained in the independent reviews; `results.json` identifies their local source
locations. The publication copies exact bytes and does not claim a new kernel
replay. Future re-verification needs retained artifacts or a fresh official run.

The next product task is one bounded continuation bridge from this certified
prefix state to the actual remaining fold, preserving all errors and original
definitions. That bridge remains unproved. Eight later atoms, parent/carry states,
atomization, whole-object finish and the original257 digest remain open. Complete
Hash qualification, Object integration, production PrismPM/template checks and
Pages deployment remain separate obligations. No wholesale thirteen-certificate
expansion or production acceptance follows from this local result.
