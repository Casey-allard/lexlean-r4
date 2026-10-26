# Original BLAKE3 case19: failed verification with observed OOM

The official compiler admitted the exact five-module source closure, then the
original `published_unkeyed_2048` verification failed. Independent outcome review
classifies this as a resource-associated failure of this configuration, **not a
false theorem, intended semantic negative, or accepted proof**.

| Actual operation | Outcome |
| --- | --- |
| Genuine development lock | Exit 0; new lock SHA256 `edabce6baaf572e30341b2eae31a98bcfcd0b06a33311130ccdcc77581195bd0` |
| Source check | Exit 0, exactly five modules, no diagnostics |
| Original case19 verify | Exit 1 after 144,543 ms; `LLV7002`, empty rejection detail |
| Container memory events | Before verify all six zero; afterward `max=12602`, `oom=1`, `oom_kill=1`; other three zero |
| Final guards | All nine attempted; eight passed, absolute-zero memory predicate failed |
| Successful verified artifact | None returned or copied; guest intermediate-file absence is not claimed |

The unchanged original theorem computes the 2,048-byte published vector, with
the original four pure-Nat runtime modules, expected digest and empty axioms.
The only configuration change selects case19 as the default entrypoint; the
300,000 ms SDK child limit remains unchanged. The 144,543 ms observation is whole
verification wall time, not a successful proof timing or timeout. The exact
killed PID, failing subprocess and causal chain from OOM to `LLV7002` are unknown.

All nine source inputs remained byte-identical in four stage checks. The genuine
lock remained identical in three checks. Tools, container epoch/configuration,
guest boot, final idle state, and sampled host/artifact reserves passed. The
VM-root, artifact filesystem and shared host remain distinct volumes. No
continuous peak or continuous reserve measurement is claimed.

Historical acceptance remains **18/35 published vectors and 255/272 controls**.
Case19 adds zero accepted proofs. No full BLAKE3, Hash, Object, PrismPM or Pages
acceptance follows. The Read observer is not a compiler/import prerequisite for
this independent source closure.

The [machine-readable result](results.json) names the exact source, invocation
and review pins. `evidence.jsonl.gz` preserves content-deduplicated exact bytes
with every relative alias, byte count and SHA256, including all raw command
receipts/stdout/stderr, the genuine development lock, source/resource/invocation
reviews, rejected invocation predecessors and their narrow regression history.
The original [source manifest](manifest.json) and its eight payloads are unchanged;
the four runtime gzip dependencies remain referenced by exact committed paths.
Archived scripts are historical invocation evidence, not installed SDK tools or
authorization to run again. The supplemental delivery manifest also pins this
result and the current plan updates.

The original failed stage and nonzero epoch are preserved. There is no automatic
retry, larger resource limit, epoch reset or changed proof. The next source-backed
investigation must identify a concrete supported proof change and retain the
original statement, runtime and expected digest; any candidate requires its own
review and falsifiable official compiler checks before acceptance.

The archived `failure-investigation.json` proposes one exact first-64-byte state
certificate and a connected symbolic continuation along the original balanced
input tree. This is **a proposal pending independent SPEC**, with no candidate or
native tests. Its first state performs no compression; it neither identifies the
OOM hotspot nor establishes reduced cost or the final digest. A concrete original
input consumer and the later validation/finalization connection remain mandatory.
