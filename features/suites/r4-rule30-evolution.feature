Feature: Complete the Rule 30 chaotic frontier discovery and monotonic weight evolution controller
  Chaotic 64-cell Rule 30 cellular automaton stepping over cross-shard manifold basins and non-vacuous monotonic L1 weight consolidation preserve zero orphaned PointKey rows and benchmark non-regression.
  Formal controls establish no unbounded non-terminating chaotic search or unverified weight mutation outside the admitted residual envelope.

  @R4-33 @build
  Scenario: Complete the Rule 30 chaotic frontier discovery and monotonic weight evolution controller
    Given the pure LexLean Rule 30 chaotic discovery and monotonic weight evolution model in "src/R4/Rule30Evolution.lex.tex"
    And the independent formal control theorems in "tests/lexlean/Rule30EvolutionControls.lex.tex"
    When the LexLean kernel verifies the Rule 30 chaotic discovery and weight evolution control suite
    Then every Rule 30 chaotic candidate step and monotonic L1 weight consolidation invariant holds
