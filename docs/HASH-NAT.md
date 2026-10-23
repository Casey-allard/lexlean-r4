# Nat-word cryptographic refinement

Status: 1,694 independent controls frozen before new definitions. The existing
accepted hash implementation is unchanged. The proposed refinement and exact
interfaces are in [the API contract](specs/hash-nat-api.md).

The candidate retains all original 30 Hash and 403 Object propositions and
legacy word types. Its purpose is to express complete cryptographic rounds
with exact Nat arithmetic while preserving existing public behavior. No
performance result or replacement acceptance is established here.

Five primitive fixtures contain 1,681 literal controls. Their inventory is
[`control-inventory.json`](../tests/fixtures/hash-nat/control-inventory.json),
SHA-256 `bf05c6095c809cd3a7521b08c738ca7c4cb259dbabd9470fbd0301487467fafb`.
Independent review recomputed every expected value from positional bit and
byte identities, separately from the author's shift-based construction.
This is finite control construction, not execution of a candidate or hash.

Thirteen further controls directly project all 208 published BLAKE2b internal
state words from RFC 7693 Appendix A. The retained literal inventory is
[`rfc7693-appendix-a.json`](../tests/fixtures/hash-nat/rfc7693-appendix-a.json).
Independent review parsed the named RFC source and compared every input and
output word of all twelve individual rounds and the complete composition.
The RFC uses BLAKE2b-512 initialization; the original public BLAKE2b-96 controls
remain required to check the configured digest parameters and feedforward.

Before new definitions, official language-1.2 source checks after successful
lock creation rejected the absent `R4.HashNat` and `R4.HashNatCore` imports
with `LLR3005`. This records the genuine pre-implementation failing check.

Acceptance requires actual source and formal checks, source-valid defects,
restored replays, connected original Hash/Object controls and complete
integration. Axiom policy and compiler limits remain unchanged. Formal Nat
arithmetic, including possible 65-bit addition intermediates, does not establish
the checked-u64 production lowering or fixed-memory serving requirements.


## Isolated HashWire verified finite baseline

[HashWire](HASH-WIRE.md) preserves an isolated fixture project whose exact 78-root/636-control positive baseline has passed independent artifact review and durable sealing. Status is BASELINE_VERIFIED_MUTATION_INCOMPLETE: two per-mutant semantic negatives are partial observations; the third mutant hit a whole-module LLS8002 child timeout, and full restoration was still running at the recorded checkpoint. All seven mutants/ten witnesses, complete restoration and final independent review remain required. The candidate Hash facade differs from the canonical module and is not adopted by this packaging. Original Hash regression, separate Nat controls/eleven mutants/restoration, Object consumers and modeled integration remain required; checked-u64 production lowering is a separate gate.
