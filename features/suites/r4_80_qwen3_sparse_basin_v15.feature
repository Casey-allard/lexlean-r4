Feature: R4-80 Qwen3-distilled 64-basin UORR4V15 sparse geometric wave engine
  As the LexLean-R4 Studio and Chat serving runtime
  I want an instant-mounting (<= 150 ms) 64-basin UORR4V15 sparse geometric wave engine sustaining >= 10000 tok/s with full /workspace and <tool_call> intelligence
  So that default conversational and coding turns respond instantaneously without 410 MB dense weight stalls

  @R4-80 @build
  Scenario: Sub-150ms UORR4V15 sparse basin wave generation at >= 10000 tok/s with zero floats
    Given the formal LexLean module R4.Qwen3SparseBasinV15 and its control theorems are verified
    When the UORR4V15 sparse basin engine mounts and executes 8-token resonant waves over <= 512 KiB active basins
    Then float_ops, dense_matmul_ops, softmax_ops, and icosian_drift_lsb equal 0 and throughput is at least 10000 tok/s
    And Studio /workspace tool calls and PlanReadOnly refusals are honored across multi-hop continuations
