# Object Read33 source migration and verification timeout

The existing Read33 root now passes official source admission after three metadata edits. Its verification timed out in the target Lean module, so **none of the 33 controls is accepted by this run**. The complete failed source, original compatibility negative, selected raw receipts and emitted module are preserved for diagnosis.

The source migration changes only the bool and nat glossary versions from1.1.0 to1.2.0 and semantic-module schema1 to2. All33 complete declaration ASTs, statements, parameters, reflexivity proofs and declared axiom lists are unchanged. [Written-source review](control-migration-code-review.json) and [current portable input bindings](current-portable-bindings.json) identify the exact source and15 inputs. Six merged Store integration sources and ten other unfinished drafts remain unchanged; original snapshots remain in the earlier pending archive.

| Phase | Actual result | Meaning |
| --- | --- | --- |
| Original unchanged Read33 | Genuine lock succeeded; sourcecheck exited1 with LLR3001 glossary-version mismatch; no verify | Compatibility admission negative only, not semantic defect evidence |
| Migrated Read33 sourcecheck | Exit0,11 modules, no diagnostics | Source admission only |
| Migrated Read33 verification | Exit4,LLS8002; target Lean child observed300009ms against300000ms limit | Closed timeout configuration; no verified artifact and no accepted Read33 subset |

The [actual verify receipt](positive/verify.json) reports timeout in LexLeanR4.ObjectReadControls. Whole verification wall time was316328ms across its work; the configured child limit remains300000ms. The diagnostic does not identify a particular theorem, tactic or subexpression. It does not demonstrate a false proposition, semantic counterexample or mechanism failure. No equivalent retry or automatic limit increase follows from this checkpoint.

Both sampled memory.events captures contain zero in all six counters, including oom and oom_kill. Source and lock identities, the four tools, epoch, idle state and storage postconditions were checked; [postcapture errors](positive/postcapture-errors.json) is empty. These observations distinguish the timeout from the prior OOM configurations. They are sampled observations, not a continuous storage or memory high-water guarantee.

The [independent failure review](positive-failure-review.json) records the completed result. [Selected positive custody](positive-custody.json) binds byte-preserved raw receipts, the genuine development lock, [emission capture](emission-capture.json), and [actual emitted module](emitted-ObjectReadControls.lean). The emitted Lean is diagnostic build output, not a verified artifact or a handwritten replacement for LexLean. No compiled binaries are included. Original local raw copy receipts and any other build intermediates remain retained locally; this directory is an explicitly selected portable record.

[Results](results.json) records exact identities. The earlier [source bindings](source-bindings.json) retain their historical positive-not-included state; the current successor records the completed timeout without changing those earlier bytes. The development locks are actual official compiler outputs and do not establish a production PrismPM binding.

The [Codec311 source-only next-step specification](codec-next-spec.json) preserves all311 ASTs under the same three proposed metadata changes and finds no whole257 construction reference or missing module in the pinned finite closure. Its originally proposed ordering was after Read acceptance and delivery. Read acceptance has not occurred: this is planning context, not authority to treat Read33 as passed or to launch Codec automatically. All Codec native tests remain NOT_RUN. Read timeout diagnosis and any revised ordering require explicit reviewed continuation; no new candidate or diagnosis is claimed here.

Only the earlier20 Store controls remain accepted in this Object integration sequence. Read33 remains unaccepted; Codec311 and Construction39,20 separate Blake3 primitive controls, required semantic defects and restoration, whole257, complete Hash qualification, model/scenario/wrapper integration, genuine production PrismPM/full just vv and generated Pages remain open.
