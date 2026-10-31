Feature: M3 scoped durable turn memory PrimeAddress GCD sieve and 3-sigma manifold sketch gate
  Scoped durable turn-memory codecs (UORMEM01 and UR4SESS1), sorted-merge PrimeAddress GCD shared-prime intersection, and the +3-sigma manifold sketch gate preserve exact integer semantics.
  Formal controls establish no browser IndexedDB durability, network synchronization or cryptographic collision resistance.

  @R4-26 @build
  Scenario: Scoped durable turn memory codecs sorted-merge PrimeAddress GCD sieve and manifold sketch gate
    Given the pinned UOR-R4 durable memory PrimeAddress and manifold sketch contracts with independent literal controls
    When the digest-pinned development SDK verifies the durable memory controls
    Then UORMEM01 v1 and v2 and UR4SESS1 v1 and v2 headers enforce magic version and explicit turn-memory opt-in rules
    And scoped durable turn-memory lookup isolates user and session scopes and resolves most-recent opening and reply-continuation targets
    And sorted-merge PrimeAddress GCD shared-prime intersection counts and extracts exact common prime factors in linear traversal
    And the 4096-bit manifold sketch gate admits overlaps at or above the +3-sigma threshold of 2144 bits and rejects lower overlaps
