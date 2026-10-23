Feature: Complete unkeyed BLAKE3 tokenizer identity hash
  Pure LexLean computes exact BLAKE3-256 from every admitted input byte.
  Standard hash conformance does not substitute for tokenizer identity composition.

  @R4-15 @build
  Scenario: Published outputs bind complete chunks, tree and root domain
    Given all 35 immutable official unkeyed BLAKE3 known answers
    And an admitted exact word representation with no additional proof axioms
    When the development SDK verifies every complete digest and streaming control
    Then every message block and chunk contributes in its defined order
    And tree reduction, chunk counters and ROOT output preserve the standard
    And exact length, byte and budget errors reject invalid inputs
    And wellformed counter, root flag, tree order and endian defects are rejected
