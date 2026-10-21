Feature: Complete raw tokenizer JSON admission
  Ignored and overwritten values remain subject to complete syntax admission.
  Source-compatible extraction runs only after complete document admission.

  @R4-20 @build
  Scenario: Raw bytes preserve parser and tokenizer semantics end to end
    Given the pinned SliceRead parser profile and independent literal controls
    When pure LexLean parses every byte into a bounded postorder document
    Then string escapes UTF-8 numeric syntax depth and punctuation preserve ordered errors
    And duplicate decoded keys replace bindings while retaining every admitted value node
    And complete document validation rejects forged node references and counters
    And tokenizer extraction preserves source order before the existing kernel admission
    And cryptographic identity and generated runtime acceptance require separate evidence
