Feature: Exact tokenizer text and Unicode primitives
  Complete Unicode 17 imported facts retain the pinned Rust 1.97.1 identity.
  LexLean defines scalar and byte admission, UTF-8, and the byte alphabet.
  These primitives establish neither a complete tokenizer nor a tokenizer CID.

  @R4-14 @build
  Scenario: Text primitives preserve exact bytes and complete scalar classifications
    Given the reviewed complete Unicode inventory and independent transition boundaries
    When the digest-pinned SDK verifies every frozen text control
    Then Alphabetic Numeric and White_Space use their complete pinned properties
    And UTF-8 encoding and strict decoding preserve every valid scalar form
    And invalid scalar byte and UTF-8 inputs have typed failures
    And lossy decoding preserves Rust invalid-sequence grouping after byte concatenation
    And every byte alphabet image has its exact inverse
