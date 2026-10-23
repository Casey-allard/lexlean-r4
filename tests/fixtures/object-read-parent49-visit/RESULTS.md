# Parent49 visit transition results

**Accepted locally: the exact production parent visit transition and its intended wrong-base negative.** This is not a final walked state, child-read acceptance, or acceptance of any original Read33 control.

| Phase | Observed result | Independent conclusion |
|---|---|---|
| visit-false | Source check passed; verify exit 1, intended LLV7002; 37,832 ms | Wrong unrelated-zero base rejected while verifying the full consumer root |
| visit-positive | Source check passed for 17 modules; verify exit 0; 120,377 ms | Exact full production visitor transition accepted |

The transition starts from the original four-entry backend and parent ID at scale 1, base 0, empty initial walk state, logical length 96, range [0,96), original limits, and collect=true. Its right side retains the complete production Result continuation, including all three child visits, record accounting, range/offset checks and error propagation. The proof reuses the accepted full parent fetch through Nat induction, Cases, Congruence and the mandatory consumer Apply at one. The actual proof and kernel replay established the required Nat.succ0/Nat1 conversion for this exact equation.

Positive attestation: `8348d05ac25a56d8d0d9a29419b9dbdb6819f6eae688e05793fe238ad43ed943`. Independent review `816bbb8327a847c5511647227730e9527e5fc123e93d94cda7097ddaa33a8d3c` checked 175 artifact files, 85 build outputs, 814 declarations with no observed axioms, 52 successful processes, and 17 kernel replays. Source/lock/tool/epoch/idle/storage checks passed, memory events were zero, and postcapture errors were empty. This establishes neither continuous resource bounds nor a speedup.

The reviewer census digest `c51aa0b37f44a80f82eb5d132c72d46ab514cf2d5ac1cb2a68d64b1d2d1d850c` and portable census file SHA `9e4a8164239acef105883418e1dd62bd759e81285b0f6aa1f4ae794e298aa952` hash different serializations. The first is the reviewer's sorted census encoding; the second includes the portable JSON wrapper and file rows. Do not substitute either for the attestation ID or conflate the two hashes.

Children and the final walk state remain unproved, along with root planning/wrapping, concrete accounting/statistics, range finishing and manifest-version computation, whole digest comparison, original wholeRead and all 33 Read controls. Full Object/Hash qualification, PrismPM integration and generated Pages remain open.

Frozen source README/metadata retain preparation-time NOT_RUN. Historical raw custody retains pending-review labels. This document and `results.json` record the later accepted outcome. Shared inputs resolve through `source-bindings.json`; selected emitted proof/audit text and the complete artifact census are portable, while full compiled artifacts remain locally preserved and pinned.
