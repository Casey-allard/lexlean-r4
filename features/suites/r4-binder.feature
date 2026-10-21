Feature: Canonical ordered ternary binder and general forest validation
  Fixed binder metadata preserves exact atom identity and construction order.
  This feature establishes no carrier execution or window-plan framing.

  @R4-04 @build
  Scenario: Canonical construction and general validation preserve their distinct contracts
    Given the pinned ordered ternary binder metadata specification
    When the digest-pinned SDK verifies the independent binder controls
    Then canonical construction preserves exact leaf, triple, and frontier order
    And general validation accepts valid noncanonical forests
    And malformed identities and references produce the first typed source error
    And retained nodes serialize as three child words followed by one atom word
