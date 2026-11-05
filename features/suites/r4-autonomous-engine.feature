Feature: Autonomous homeostasis engine Hebbian strategy plasticity and hypothesis minting
  Bounded Hebbian strategy weight plasticity in [1, 16], priority strategy scheduling, workflow homeostasis, and non-redundant hypothesis minting preserve exact integer semantics.
  Formal controls establish no external runner liveness or unbounded empirical convergence.

  @R4-28 @build
  Scenario: Autonomous engine verifies Hebbian plasticity priority scheduling workflow homeostasis and hypothesis minting
    Given the pure LexLean autonomous engine model in "src/R4/AutonomousEngine.lex.tex"
    And the independent formal control theorems in "tests/lexlean/AutonomousEngineControls.lex.tex"
    When the LexLean kernel verifies the autonomous engine control suite
    Then Hebbian strategy weights stay bounded in [1, 16] under +2/-1 outcome updates
    And priority strategy scheduling orders strategies descending by learned family weight
    And workflow homeostasis admits canonical 17,47 cron and 75-minute tick budgets with >=15-minute gate reserve
    And non-redundant hypothesis minting clamps support to [1, 16] and refuses duplicate attempts or exhausted budgets
