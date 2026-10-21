Feature: Exact read configuration and gate inventory
  Configuration is artifact metadata with ordered typed validation.
  Formal checks execute no carrier, kernel, training or serving workload.

  @R4-06 @build
  Scenario: Read configuration retains source order and checked word arithmetic
    Given the pinned Spec, read modes and independent initialization policy
    When the digest-pinned development SDK verifies configuration controls
    Then invalid Spec precedes read errors and Relation kept precedes sweeps
    And mode words and initialization outcomes match the source contracts
    And exact UInt32 gate inventory limits reject overflow without truncation
    And out-of-word natural operands have a separate admission error
