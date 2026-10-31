Feature: 19-table M1 discrete trunk specification orbit-root causal successor and Level-3 joint pointer read
  The 19-table M1 discrete trunk layout, base-17 4-block root packing, causal prefix orbit-root lookup, and Level-3 joint pointer read preserve exact integer semantics.
  Formal controls establish no CUDA kernel parity, learned trunk weights or open-ended generation quality.

  @R4-25 @build
  Scenario: 19-table M1 discrete trunk base-17 packing causal orbit-root successor and Level-3 joint pointer read
    Given the pinned UOR-R4 discrete trunk and D11 contracts with independent literal controls
    When the digest-pinned development SDK verifies the discrete trunk controls
    Then the 19-table M1 trunk specification preserves exact level bits per token and the 12288-bit total across levels 0 through 5
    And base-17 4-block root packing and unpacking round-trip valid roots and reject out-of-range blocks
    And causal prefix orbit-root successor lookup selects the most recent matching predecessor and rejects out-of-vocabulary tokens
    And Level-3 joint pointer scoring combines L1 residual distance with 2*min-Hamming distance and breaks ties to smaller token IDs
