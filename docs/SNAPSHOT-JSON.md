# Snapshot JSON codec — R4-22

Status: isolated source verification is complete: all 476 frozen controls pass baseline and both restoration runs, and the reviewed successor accounts for all 11 source-valid semantic negatives. The original first mutation batch remains preserved as INCOMPLETE because it advertised one insensitive witness. The mandatory Rust target, complete 108-module default graph, receipt audit and static integration checks now pass. Production binding and deployment remain pending.

The [normative API](specs/snapshot-json-codec-api.md) is the approved ca21d9308ca83ed5460a7e9e08a7c31cdf36ac414ca6833ff84fb194b784692a proposal; [source notes](specs/snapshot-json-codec-notes.md) record its derivation. The published API changes only its status paragraph. All 476 independently reviewed literal controls and their nine module hashes remain unchanged in `tests/fixtures/snapshot/controls.json`. The predefinition run produced actual missing-R4.SnapshotJson LLR3005 diagnostics for all nine roots after a successful fresh lock; the earlier unsorted-config LLC0101 is a separately retained setup failure.

The source has 143 declarations. Authoring only constructs semantic AST and canonical JSON; it does not execute any candidate/parser/serializer/reference model. The source imports exactly R4.Json, R4.JsonNumber, and R4.TokenizerText, with the complete existing transitive five-source dependency closure copied byte-identically from the RED stage.

## Traversal and admission

Fixed record maps visit every surviving binding in the parser's decoded-key sort order. Each recognized field is decoded immediately. Unknown fields fail immediately. Required-field/default resolution follows the complete binding scan in declaration order. Positional records parse left to right, including exhausted default positions before reporting the next absent required position; extra elements are rejected after all required values.

ReadSpec finds and decodes its tag before examining any remaining content. Unit enum maps check singleton arity before the variant name, then validate the recognized variant's null payload. Numeric extraction uses actual JsonNumberClass; finiteOther always fails exact integer admission. Int32 negatives retain the predecessor representation. Typed-only encoding validates every semantic field, nested field, scalar and list item before output length accounting.

Every public raw operation checks the full Snapshot profile and performs actual complete parsing. The public document method calls actual jsonCheckDocument, then extracts from the newly checked document. Strict methods replay actual jsonStepByte over every input byte with the same physical-position advancement as jsonDrive, observe only a successful normal-phase key-closing transition, check actual key spans, and call jsonEndInput plus jsonDocumentEqual at EOF. Duplicate scanning occurs only after the whole replay and compares exact decoded keys scoped by object opening offset. No public method trusts supplied events, parser state, or an admission flag.

## Serialization and termination

Typed encoding visits every fixed field and complete list, omitting only the two exact default enum fields. Native storage encoding recursively uses the exact sorted order of the typed-to-Value representation. Stored raw Value encoding preserves all surviving object/array/enum-map forms.

For the raw stored representation, a complete descending postorder pass starts with the root identity and marks each reachable node's children. Actual parser edges always point to earlier completed nodes, so every newly marked child is still ahead in the descending pass. A complete ascending pass emits only marked nodes and resolves child encodings from already emitted identities. A missing identity is an explicit invariant error. Overwritten unreachable values are never encoded, including finiteOther values; all raw values were still parsed before typed admission. This avoids both a floating formatter and arbitrary traversal fuel.

All list traversals recurse on their immediate list tail. Decimal generation alone uses a fixed internal 20-step digit bound, covering every admitted UInt64 and signed Int32 magnitude; its zero-fuel branch is explicit numericRepresentation failure, not truncation or zero substitution. Complete bytes are counted separately in storage-first, typed-second order. Failed output budgets expose no prefix or partial pair.

## Static checks

Host structural checks found no unbound local variables or unresolved/local-call arity errors. Every constructor, field order, and public call referenced by all 476 frozen controls matches the source's declared surface. Canonical JSON has maximum object/array depth 102, below the known parser nesting limit. These are static source checks only, not formal acceptance.

The only pre-review source revision was an internal parameter alpha-renaming in snapshotKeyAfterStep from next to advancedState, performed before any SDK measurement. The original source was recovered by exact inverse and matched its previously recorded SHA. The evidence directory contains both author versions, the original source, and the exact three AST string differences. No future source edit should occur silently during review.

## Observed verification and limits

Independent source/spec review and subsequent code/security review approved the same frozen 143-declaration source SHA cf26668c78b89f1114dc3dbece6c5a3645e90d423bb844789105f4a52f314430. Official SDK sourcecheck passed every root. Both baseline and first-batch restored verification passed all 476 controls across nine roots. Each audit binds 3,302 exact declaration rows, 63 kernel replays, the complete current input hashes, process outputs, and the none-axiom policy with empty observed axiom lists. Every restored attestation, build, source, and semantic identity equals its baseline counterpart.

The official compiler executable SHA is 107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5; language-1.2 compiler semantics are 09f4b95d5994ca00acd413e6f9e8381c8a18d4fd778249d2c8654f7dc99dce86. Throughout the isolated component runs, Lean 4.32.1, the fixed 300-second child policy, four-GiB/two-CPU container limits, and disabled network were unchanged. The later combined integration required the memory increase recorded below. No compiler/library/generated-Lean patch or axiom allowance was used. Durations are execution records, not a performance benchmark.

| Fixture | Frozen controls |
| --- | ---: |
| SnapshotSchemaControls | 44 |
| SnapshotRepresentationControls | 74 |
| SnapshotNumericControls | 94 |
| SnapshotOrderControls | 35 |
| SnapshotStoredValueControls | 21 |
| SnapshotStringControls | 104 |
| SnapshotStrictControls | 47 |
| SnapshotBudgetControls | 33 |
| SnapshotRoundtripControls | 24 |
| Total | 476 |

The first mutation runner accepted nine source-valid semantic negatives and stopped at the tenth under its unchanged strict classifier. That tenth source mutation was rejected by strictEventsWhitespaceColon, but its advertised strictEventsNestedWhitespace association was insensitive: the latter input has whitespace after the colon in value mode. The sensitive input has whitespace before the colon; the mutant attempts key validation at the wrong endpoint and triggers invariant refusal. This is a witness-association instrument fault, not a production-source correction or a change to the frozen expected values. The original batch remains INCOMPLETE and its final dependency mutation, actual-json-first-wins, remains NOT_RUN in that batch. Both original sources were restored exactly, and the runner's outer finally completed the full unchanged 476-control replay despite the primary failure.

An independent diagnosis and parent review approved a successor that changes only that association, preserves every mutant and control byte and the exact negative classifier, and re-audits the retained first ten results. The successor has now actually executed the remaining actual-json-first-wins dependency mutation: sourcecheck passed, and formal verification produced five precisely mapped LLV7002 rfl failures including every required unchanged witness. Both sources were restored exactly; the successor replayed all 476 controls and independently audited all 3,302 declaration rows and 63 kernel replays. Every restored identity matches the baseline, all observed axiom sets are empty, and all current inputs and process outputs are bound. The successor summary is PASS for ten Snapshot-owned mutations plus one actual-parser dependency mutation. The first ten negative results were re-audited from their retained exact bytes under the reviewed corrected association; those ten were not re-executed. The original incomplete record remains unchanged.

Setup faults are retained separately: the initial unsorted configuration, copied-directory ownership failure and unsuccessful metadata repair, and the initial mutation runner's name regex rejecting a digit before any SDK execution. Their approved repairs did not alter the source, control values, compiler settings, or semantic-negative classifier. The later insensitive-witness fault is likewise retained separately from restored positive verification.

Durable evidence is under /Users/casey.allard/lexlean-r4-evidence/2026-10-21/snapshot/. Baseline audit SHA: 3d70007bf37a4616faad8069755563ca7ef6f8e417e9b450a3291c40a2db3dae. First-batch restored audit SHA: 5bf576d8ece79869f86832bbe0c27eb20f76cd08ef419f062ce695d7df494987. Original incomplete mutation summary SHA: 9606297e3620733f8e9f72938fe896ff7b92b1203d867ae3da6af27048cb3fc1. The retention record confirms 305 copied files match their originals byte-for-byte, including exact original inputs, negative diagnostics, source restoration censuses, and all nine restored receipt/manifests.

The successor evidence is retained in the witness-successor subdirectory. Its summary SHA is 9e29eaf06df035101d759b6ee93527458c7e7a3fe74ecfe6875c0fd6842ba67a; its restored audit SHA is 3bb1762a1a2431534667e89ae8730cbb05421c7808e9c86acf06a7a71b186840. All 171 successor command/receipt/input files match their original evidence bytes. The complete nine actual verified directories were separately copied from the container: 675 files totaling 76,182,532 bytes, including every one of 315 manifest outputs and all 63 attested .olean files. Full file-tree hashes match the container; every output and .olean byte length/hash matches its actual manifest or attestation. This includes generated source, coverage, maps, lexicon closure, process/probe records, and compiled proof artifacts, rather than only receipts. No compiler or SDK archive was copied as a substitute.

## Remaining acceptance boundaries

The component is integrated in the 18-component development graph. The fixed inventory-bound Rust target ran both tests successfully (476 controls in nine fixture verifications; no ignored or filtered tests). The complete default graph passed 8,746 controls across 108 modules and 81 roots, with 11,038 exact none-axiom declaration audits and 108 independent kernel replays. The independent audit checks 110 current non-lexicon inputs, every manifest-listed output and the exact attested compiled-module set. It does not independently derive the complete expected output-path set or check lexicon input rows against local project files.

Model generation/checking, formatting, warnings-denied Clippy and all-feature/all-target checking passed. The preceding full workspace suite is a separate historical result; this integration ran its mandatory target and combined formal graph. Actual `just model` and `just vv` both still fail on the missing `prismpm.lock`. No production acceptance was substituted.

The target passed at four GiB. The combined graph twice exhausted compiler front-end memory (exit 137 at four and five GiB); kernel evidence confirms those resource failures. It passed at six GiB in the same container, with the existing VM increased to eight GiB. CPU count, disconnected network, compiler, source, controls, axiom policy and child proof limits were unchanged. The initial target invocation also failed before compilation because it used the default Cargo home; selecting the retained offline Cargo cache corrected the runner only. All failed attempts are preserved.

Combined attestation: `e532e1cfd597ae6d18ab4e33b601bf12c031217ad63ae80617ca680c36ee4afd`. Configuration SHA-256: `cd87a5aa342663566f53e4089f8252e0c66de98725a01961c35df3218f5fbcdb`; lock SHA-256: `f35f51860d64c0cd703b35a6ecb912ed4c116399d5d920433a663907fd1da04e`. Complete integration evidence is retained under `/Users/casey.allard/lexlean-r4-evidence/2026-10-21/snapshot/integration-18/`.

Hash composition, runtime Snapshot admission, effective Serde feature binding, custody/authenticity, binary freeze/export, generated runtime, and Pages deployment remain separate full-scope obligations. This codec's structural success describes and serializes exact data; it does not authenticate those data or establish model behavior.
