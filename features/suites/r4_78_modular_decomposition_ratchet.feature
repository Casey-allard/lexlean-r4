Feature: R4-78 Verdant M15 modular Rust and JavaScript decomposition and residue ratchet
  As the LexLean-R4 repository acceptance boundary
  I want oversized Rust and JavaScript files decomposed into focused domain modules and unlifted residue ratcheted <= 200
  So that every source module stays within strict line ceilings while preserving the 6-file browser closure

  @R4-78 @build
  Scenario: Modular Rust/JS decomposition, residue <= 200, and 6-file browser closure sync
    Given the formal LexLean module R4.ModularDecompositionRatchet and its control theorems are verified
    When the repository ratchet and lift register are evaluated across modular Rust and JavaScript sources
    Then max_residue_functions is at most 200 and max_rust_file_lines is at most 11400 and max_js_file_lines is at most 12800
    And the 6-file public browser closure remains deterministically synchronized with modular JS sources
