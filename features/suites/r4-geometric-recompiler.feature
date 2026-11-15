Feature: Direct Teacher-to-R4 Geometric Vector Recompilation & Multiplier-Free Serving (R4-44)

  @R4-44 @build
  Scenario: Recompiled UORR4V12 container admits multiplier-free SmolLM2 geometry and refuses floats, dense matmuls, or turn-memory hijacks
    Given the formal LexLean module R4.GeometricRecompiler and generated Rust verifier r4_geometric_recompiler
    When UORR4V12 container headers, GQA head mappings, and nibble-pair 256-entry LUT integer Q14 RoPE steps are evaluated
    Then compliant SmolLM2-135M integer configurations (30 layers, 9 query heads, 3 KV heads, head_dim=64, d_model=576, vocab_size=49152, tied_lm_head=true, rms_norm_preserved=true) with zero floats, zero dense matmuls, zero softmax ops, and zero turn-memory hijacks are admitted
    And bad container versions, non-zero float serving ops, non-zero dense matmul ops, non-zero softmax ops, canned turn-memory entries, mismatched GQA dimensions, missing tied/RMS norm flags, invalid LUT tables, and Q14 overflows are refused
