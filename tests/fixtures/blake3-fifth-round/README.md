# Fifth BLAKE3 round: source preparation

All three official compiler phases are **NOT_RUN**. These files propose one fixed fifth-round certificate and its mandatory original compression consumer, following the accepted fourth-round result. Source checks and host arithmetic are not proof acceptance.

The positive theorem states `b3CoreRoundRaw(R4, M4) = R5`, where R4 is copied exactly from the accepted fourth certificate and M4 is copied exactly from its actual consumer. R5 has all sixteen literal words. `derive-expected.py` interprets the pinned production AST and checks the result against an independent integer G schedule; its output remains untrusted authoring data until official kernel reflexivity succeeds. The negative changes only expected s0 from 1361083347 to 1361083348.

The mandatory consumer preserves the original `b3CoreCompressRaw` left side and exposes `b3CoreFeedForward(b3CoreRounds(2, R5, M5), IV)`, with exactly one additional permutation of M4. It applies the accepted fourth original-consumer theorem. Actual compiler conversion, certificate reuse and resource cost remain unproved. A helper pass alone cannot accept this consumer.

The three phases are round5-negative, round5-positive and original-consumer, individually reviewed and run in that order without automatic advancement. Their exact import closures contain 4/4/15 modules and 8/8/19 stage inputs, respectively; decoded input sizes are 140433/140433/233786 bytes. Shared runtime, accepted proofs and metadata are pinned by committed source or gzip references in candidate.json, not copied from removed recoveries. No development lock is fabricated.

Four structural predecessor checks fail and pass on this successor; 105 source assertions pass, including canonical JSON, direct imports, original left-side preservation, sole negative delta, supported proof constructors, phase closure and decoded byte counts. A memory-only missing-fifth-permutation defect is rejected and the unchanged checker passes after restoration. Repeated derivation bytes agree. Coverage has not been measured. These are source preparation results, not native results.

Original runtime definitions, digest, empty axioms and 300000 ms limits are unchanged. Two remaining rounds, full compression, S128, final digest, BLAKE3 counts and broader UOR-R4 integration remain open. This packet grants no native launch authority and introduces no additional-round batch. The exact approved proposal and independent SPEC are retained alongside the source.
