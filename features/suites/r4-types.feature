Feature: Exact R4 configuration boundaries
  Configuration metadata preserves the pinned discrete engine's bounds.
  This feature establishes no executed model or research capability.

  @R4-01 @build
  Scenario: The specification admits precisely the source configuration range
    Given the LexLean specification and independently authored boundary theorems
    When the digest-pinned development SDK verifies the source through Lean and axiom replay
    Then both valid endpoints are accepted
    And every out-of-range dimension and excessive cell count is rejected
