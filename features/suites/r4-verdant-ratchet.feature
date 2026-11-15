Feature: Verdant-Resonance Peirce Triadic Homeostasis and Shrink-Only Ratchet Gate (R4-41)

  @R4-41 @build
  Scenario: Peirce triadic homeostasis and shrink-only ratchet enforce monotonic bounds and witness checks
    Given model/ratchet.toml and the standardized formal LexLean modules R4.AutonomousEngine, R4.PrismFactory, and R4.Rule30Evolution
    When the engine executes an Abduction -> Deduction -> Induction cycle with Rule-30 discovery and shrink-only ratchet auditing
    Then verified witnesses, preserved invariants, and non-regressing metric values are admitted
    And missing witnesses, violated invariants, zero Rule-30 chaos seeds, and metric regressions above ceiling are refused
