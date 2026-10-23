Feature: Exact fixed-width cryptographic word algebra
  Boolean word models supply arithmetic and codecs for future hash definitions.
  Formal controls establish no hash, model execution or production acceptance.

  @R4-09 @build
  Scenario: Fixed words preserve modular arithmetic and exact byte order
    Given fixed-shape Boolean bytes and 32-bit and 64-bit words
    When the digest-pinned development SDK verifies independent word controls
    Then wrapping addition retains every carry below the word boundary
    And Boolean operations and bounded shifts and rotations preserve bit positions
    And little-endian and big-endian codecs consume exactly the required bytes
    And invalid byte values, lengths and shift amounts have typed failures
