Feature: Canonical content-addressed byte objects
  Pure byte-object semantics preserve content integrity and every ordered occurrence.

  @R4-13 @build
  Scenario: Canonical objects retain exact bytes across construction and authenticated reads
    Given explicit object budgets and the pinned independent object-store vectors
    When the pinned SDK verifies the object controls
    Then exact record and manifest codecs reject malformed or unauthenticated bytes
    And ordered ternary construction preserves occurrences while deduplicating records
    And raw import validates all records and budgets before publishing any inventory
    And closure discovery descends present parents and distinguishes missing-ID orderings
    And range reads authenticate visited records and final padding with occurrence statistics
    And whole verification separately checks the original logical bytes with SHA-256
    And no peer transaction, tokenizer binding or durable browser execution is claimed
