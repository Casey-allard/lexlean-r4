Feature: Complete the fuel-bounded ReAct self-correcting agent controller and verified patch gate
  Fuel- and iteration-bounded ReAct phase transitions, compiler and test-runner diagnostic self-correction, and SecOps-audited patch commit admission preserve deterministic termination and zero untested patch commits.
  Formal controls establish no unbounded agent retry loops, SecOps policy bypasses, or commits with failing or absent tests.

  @R4-35 @build
  Scenario: Complete the fuel-bounded ReAct self-correcting agent controller and verified patch gate
    Given the pure LexLean fuel-bounded ReAct agent controller model in "src/R4/ReactAgent.lex.tex"
    And the independent formal control theorems in "tests/lexlean/ReactAgentControls.lex.tex"
    When the LexLean kernel verifies the ReAct self-correcting agent controller control suite
    Then every ReAct phase transition, diagnostic self-correction retry, and verified patch commit invariant holds
