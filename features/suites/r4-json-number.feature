Feature: Source-bound tokenizer JSON number admission
  Numeric metadata is parsed even when ignored or later overwritten.
  The standalone default parser profile is distinct from float_roundtrip.

  @R4-18 @build
  Scenario: Exact numeric scanning and finite acceptance preserve ID distinctions
    Given pinned default-profile number branches and independent arithmetic controls
    When pure LexLean scans the complete bounded numeric lexeme
    Then unsigned IDs remain distinct from negative zero decimal and exponent syntax
    And overflow digits and source exponent saturation retain their exact order
    And binary64 finite acceptance uses exact integer rounding and midpoint comparison
    And malformed or out-of-range numbers return typed errors before replacement
    And no host floating-point oracle or generated acceptance is inferred
