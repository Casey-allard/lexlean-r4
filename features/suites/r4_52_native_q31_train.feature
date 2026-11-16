Feature: Pure-From-Scratch D85 Q31 SpiralCore/FBS Neural Lane & CUDA/Wasm Parity (R4-52)

  @R4-52 @build
  Scenario: Pure Q31 Cl(0,6) E8 Spin-Orbit codec and G2/D19 lane enforce bit-identical CUDA and Wasm integer replay
    Given the formal LexLean module R4.NativeQ31Train and generated Rust verifier r4_native_q31_train
    When UORQ31V1 container magic, orbital group bounds (64 Cl(0,6) states, 16 E8 roots, 8D spin), SHA-256/BLAKE3 checksums, integer-only updates, and CUDA vs Wasm logit checksums are evaluated
    Then pure Q31 checkpoints with bit-identical CUDA/Wasm replay, valid orbital invariants, and zero float training ops are admitted
    And invalid magic, non-integer training steps, checksum mismatches, out-of-domain orbital configurations, and CUDA/Wasm parity drift are refused
