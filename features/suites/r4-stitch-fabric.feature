Feature: Stitch 2-of-3 XOR Parity Custody and SHA-256 Verification (R4-42)

  @R4-42 @build
  Scenario: Stitch 2-of-3 XOR parity custody recovers missing shard halves and refuses insufficient or tampered shards
    Given the formal LexLean module R4.StitchFabric over stitch/xor-2-of-3/1 manifests and shard groups
    When any 2-of-3 combination of (left, right, parity) is supplied with matching FIPS 180-4 SHA-256 digests
    Then exact byte-for-byte shard reconstruction succeeds for both paired groups and odd singleton trailing groups
    And insufficient shards (< 2 present), length mismatches, bad threshold/count metadata, and SHA-256 digest tampering are refused
