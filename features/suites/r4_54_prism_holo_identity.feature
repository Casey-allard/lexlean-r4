Feature: Stitch Prism Identity, Hologram v4 .holo Compiler & UOR Foundry Selectable App (R4-54)

  @R4-54 @build
  Scenario: Stitch.PrismId canonical kappa chains, Hologram v4 .holo compilation, and UOR Foundry app descriptors admit valid archives and refuse tampered inputs
    Given the formal LexLean module R4.PrismHoloIdentity and generated Rust verifier r4_prism_holo_identity
    When Stitch.PrismId records (prism/device/1, prism/genesis/1, prism/bind/1, prism/revoke/1, prism/action/1, prism/persona/1) and Hologram v4 .holo archives are compiled and verified
    Then valid canonical integer kappa chains, zero-import Core-Wasm guests, sorted HOLOVIEW v1 bundles, BLAKE3 footers, and replayed UOR Foundry vectors are admitted
    And floating-point canonical JSON, kappa mismatches, recovery-signed actions, revoked signers, invalid HOLO headers, guest imports, unsorted entries, corrupt BLAKE3 footers, and vector divergences are refused
