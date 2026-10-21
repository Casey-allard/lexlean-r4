Feature: Complete byte-BPE semantic tokenizer kernel
  The pure LexLean kernel admits typed tokenizer configuration and preserves exact text behavior.
  Raw JSON parsing and content identities remain separately verified components.

  @R4-16 @build
  Scenario: Exact admission, segmentation, merging and decoding
    Given source-derived independent configuration and Unicode text controls
    When the SDK verifies the complete semantic tokenizer kernel
    Then added tokens match earliest position and longest content
    And Digits, prefix spaces and ByteLevel preserve source segmentation
    And the lowest first-occurrence merge rank merges every nonoverlapping selected pair
    And fallback and raw, lossy and strict decoding preserve their distinct contracts
    And malformed configurations, invalid scalars and exceeded byte budgets fail explicitly
    And planted rank, merge-pass, segmentation and decoding defects are rejected
    And overlapping Unicode predicates preserve independent run membership and initial branch priority
    And raw bulk lengths wrap at UInt32 while individual lengths retain checked UInt64 semantics
