Feature: R4-84 Dual-Surface Proof and Application Ratchet Co-Evolution

  @R4-84 @build
  Scenario: Constructive CAP0005 per-root falsifier proofs and live crates/wasm application residue lifts co-evolve monotonically
    Given an Observer Refinement Agenda targeting an unlifted pure residue function in crates/wasm
    When the Dual-Surface Ratchet synthesizes a constructive RatchetGenN LexLean specification, verifies CAP0005 per-root boolean falsifier negation, compiles Rust predicates into lexlean_generated.rs, lifts the application function, and emits an R4SPORE1 generation record
    Then R4.DualSurfaceRatchetEvolution admits GenerationPromoted with increased lifted floor, reduced residue ceiling, non-increasing Telos loss, and Wasm core <= 14155776 bytes
    And refuses unverified decide gates, vacuous CAP0005 falsifier roots, stagnant lift/residue counts, Telos loss regressions, or Wasm byte budget overflows
