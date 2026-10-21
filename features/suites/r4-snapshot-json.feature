Feature: Complete Snapshot JSON storage codec
  Typed storage metadata and original Value encodings preserve distinct domains.
  Strict duplicate rejection observes every decoded key through actual parser transitions.

  @R4-22 @build
  Scenario: Complete storage admission and exact compact bytes
    Given the pinned Snapshot and Serde contracts with independent literal controls
    When pure LexLean admits complete bounded raw bytes or checked typed values
    Then all fields defaults integer domains and Serde representations preserve ordered failures
    And typed and original Value encodings preserve exact bytes and complete inventories
    And strict admission observes overwritten objects and rejects the earliest repeated decoded key
    And public document entry points reparse and compare the entire supplied document
    And no partial encoding bypasses a byte budget
    And hash identity provenance runtime admission and serving require separate evidence
