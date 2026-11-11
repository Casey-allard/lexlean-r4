Feature: Compositional Geometric Language Model Runtime
  UORGLLM9 enforces zero stamped ServedTurnMemory entries, zero dense matmul ops, zero softmax ops, causal PRLL KV cache masking, and 8-bit discrete T-MAC Q12 LUT read bounds.
  Formal controls establish no external network weight streaming or empirical benchmark fluency.

  @R4-38 @build
  Scenario: UORGLLM9 enforces zero stamped turn-memory, matmul-free T-MAC LUT mixing, and causal PRLL KV attention
    Given the pure LexLean module "src/R4/CompositionalGllm.lex.tex"
    And the independent formal control theorems in "tests/lexlean/CompositionalGllmControls.lex.tex"
    When the LexLean kernel verifies the CompositionalGllmControls control suite
    Then a pure v9 geometric container with zero turn-memory entries, zero matmuls, and zero softmax ops is admitted
    And any container carrying non-zero turn-memory entries, dense matmul ops, or softmax ops is refused
    And causal PRLL KV attention refuses future-token leaks and mass drift while discrete Q12 LUT reads enforce 8-bit code bounds
