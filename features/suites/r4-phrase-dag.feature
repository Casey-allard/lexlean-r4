Feature: Complete the compositional multi-basin PhraseDag planner and topological clause bridge verifier
  Bounded acyclic PhraseDag node admission over ServedPointKey rows, multi-basin Icosian and Hamming edge composition, and cycle-free topological synthesis preserve stem coverage and zero unresolved slot references.
  Formal controls establish no cyclic graph traversal, out-of-bounds PointKey indexing, or degenerate self-loop basin transitions.

  @R4-34 @build
  Scenario: Complete the compositional multi-basin PhraseDag planner and topological clause bridge verifier
    Given the pure LexLean compositional multi-basin PhraseDag model in "src/R4/PhraseDag.lex.tex"
    And the independent formal control theorems in "tests/lexlean/PhraseDagControls.lex.tex"
    When the LexLean kernel verifies the compositional multi-basin PhraseDag control suite
    Then every PhraseDag node admission, multi-basin edge composition, and topological synthesis invariant holds
