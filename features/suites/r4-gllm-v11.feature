Feature: GLLM v11 Multi-Turn Integer RoPE, 16-Channel PRLL Attention, and Agentic Git Loop (R4-43)

  @R4-43 @build
  Scenario: GLLM v11 container admission supports versions 9, 10, and 11 with integer RoPE, 16-ch PRLL, and bounded git tool loop
    Given the formal LexLean module R4.CompositionalGllm and generated Rust verifier r4_compositional_gllm
    When container headers carry version 9, 10, or 11 with valid Cl(0,6) alignment and zero turn-memory stamping
    Then admit_gllm_container admits versions 9, 10, and 11 while rejecting versions 0, 8, and 12
    And 16-channel integer RoPE PRLL attention and SpiralCore CID 2 agentic git tool iterations enforce causality, mass conservation, and secret isolation
