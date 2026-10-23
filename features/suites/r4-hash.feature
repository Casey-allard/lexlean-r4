Feature: Exact content digests
  Pure LexLean implements complete node and whole-object hash algorithms.
  Hash conformance does not establish runtime or durable-memory acceptance.

  @R4-11 @build
  Scenario: Published digests bind exact rounds and byte encodings
    Given fixed Boolean word algebra and independent published hash vectors
    When the development SDK verifies complete hash controls
    Then SHA-256 preserves padding, bit lengths and block chaining
    And node identities use configured twelve-byte BLAKE2b with exact counters
    And typed admission failures preserve invalid lengths, bytes and scales
    And planted configuration, counter and endian defects are rejected
