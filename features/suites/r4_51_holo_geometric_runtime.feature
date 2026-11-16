Feature: Pure-Integer Holo UORR4V13 Geometric Runtime & Worker RPC Isolation (R4-51)

  @R4-51 @build
  Scenario: Block-32 RHT Cholesky-GPTQ Qwen3 inference enforces pre-RoPE QK-norm, zero floats/matmuls, and Worker-only Wasm dispatch
    Given the formal LexLean module R4.HoloGeometricRuntime and generated Rust verifier r4_holo_geometric_runtime
    When Qwen3-0.6B/1.7B attention normalization ordering, hardware integer arithmetic counts, Hamming shortlist bounds, RTX 4090 OOD parity bounds, and Web Worker RPC boundaries are evaluated
    Then Worker-isolated pure-integer execution with pre-RoPE q_norm and k_norm, group-32 alignment, zero float or dense matmul ops, and OOD parity within threshold is admitted
    And main-thread Wasm dispatch, post-RoPE normalization, non-zero float or dense matmul counts, misaligned heads, out-of-range shortlists, and excess KL divergence are refused
