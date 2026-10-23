# Verified two-atom fold continuation

The official compiler verified the actual equality

`ObjFoldAtoms [A, B] [] [] L = ObjFoldAtoms [B] firstState.frontier firstState.inventory L`

where A is bytes 0–31, B is bytes 32–63, L is the original limits, and
firstState is the independently verified first-atom singleton state. The emitted
consumer applies the supported Nat-family theorem at 1. Its term-level Result
continuation preserves the error branch. Runtime definitions and the original
257-byte input/digest are unchanged.

| Phase | Actual result | Whole verify wall time |
| --- | --- | --- |
| Original source admission | LLT4001: noncanonical JSON; verify never ran | Not applicable |
| Canonical false-base control | LLV7002: unrelated 0 = 0 cannot prove the required first-state base | 548,369 ms |
| Canonical actual consumer | Verify exit 0, no diagnostics | 93,920 ms |

The source repair changes only the ordering of three JSON objects' keys. Parsed
semantic ASTs, both consumers and both phase configs are unchanged. The original
[source candidate](../object257-nonempty-continuation/README.md) and its actual
admission failure remain preserved. That failure is not the proof negative.

Independent [native review](evidence/native-review.json) inspected the actual
consumer and 145-file artifact: **774 empty-axiom declarations, 43 successful
processes and 14 kernel replays**, with all 21 source inputs, genuine development
lock, tool/epoch identities, zero memory events and storage postconditions checked.
Attestation: `7ec728a1b4b9ecb94aeea15ec0d7bf62e57918f9d12163ab525b46b15165ab27`.

[results.json](results.json) records identities and links to independent source,
negative and native reviews. The original frozen README/candidate/manifest retain
preparation-time NOT_RUN text; these results record the later actual runs.

This establishes a local continuation equality. It does **not** yet certify the
second concrete state, remaining atoms, original257 digest, full Object/Hash
qualification, production PrismPM binding or Pages. No claim of avoided hash
recomputation or performance improvement follows from the wall times; these
cover multiple child processes, not a single-child timeout measurement.

The source closure is shared by repository-relative pins with the earlier leaf
and first-atom fixtures. Select a phase config, materialize every mapped input
from candidate.json, and generate/verify with the official compiler. The locks
here are actual development outputs, not manufactured production bindings.
Selected raw receipts, emitted module texts and full reviewer census are portable.
Full binary/olean artifact trees remain local and are explicitly excluded; rerun
required proofs when those exact raw artifacts are unavailable.
