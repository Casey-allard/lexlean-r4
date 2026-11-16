Feature: Holo v4 Kappa-Addressed OPFS Store & HTTP 206 Range Piece Loader (R4-50)

  @R4-50 @build
  Scenario: Verified HOLD-buffered OPFS piece admission and HTTP 206 Range streaming reject corrupt pieces and storage aborts
    Given the formal LexLean module R4.HoloKappaStore and generated Rust verifier r4_holo_kappa_store
    When 1 MiB pieces, 64 KiB HOLD-tail cryptographic gates, HTTP 206 Range windows, IndexedDB transaction commit boundaries, and Stitch 2-of-3 parity shards are evaluated
    Then complete pieces verified before tail release with valid safetensors headers, completed IDB transactions, and sufficient parity shards are admitted
    And early tail release before SHA-256 verification, digest mismatches, missing HTTP 206 ranges, corrupt safetensors headers, IDB aborts, insufficient parity shards, and oversized pieces are refused
