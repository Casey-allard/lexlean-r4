Feature: Complete the two-level hierarchical Radix /8 index and 5-tier OTel span deduplication
  Two-level hierarchical Radix /8 bucket indexing (16 * 64 * 17 = 17,408 buckets, 17,409 directory offsets) and 5-tier OTel u16 span deduplication preserve exact integer semantics.
  Formal controls establish no external network trace export or unbounded sub-span table capacity.

  @R4-29 @build
  Scenario: Complete the two-level hierarchical Radix /8 index and 5-tier OTel span deduplication
    Given the pure LexLean Radix and OTel model in "src/R4/RadixOtel.lex.tex"
    And the independent formal control theorems in "tests/lexlean/RadixOtelControls.lex.tex"
    When the LexLean kernel verifies the Radix /8 and OTel control suite
    Then every two-level hierarchical Radix /8 bucket index, directory offset range, and 5-tier OTel u16 span deduplication invariant holds
