Feature: Enforce MVM-Wasm RV64IMAC ELF admission and SpiralCore virtio-vsock zero-NIC routing
  Zero-NIC guest socket admission restricts MicroVM communication exclusively to AF_VSOCK (domain 40) framed in 16-byte SpiralCore Cl(0,6) x E8 headers, verifies 64-bit EM_RISCV ELF64 executables, and enforces host secret placeholder substitution without guest RAM secret exposure.
  Formal controls establish no hardware KVM hypervisor privilege or unrestricted external internet routing.

  @R4-37 @build
  Scenario: Enforce MVM-Wasm RV64IMAC ELF admission and SpiralCore virtio-vsock zero-NIC routing
    Given the pure LexLean SpiralCore virtio-vsock and MVM admission model in "src/R4/SpiralCoreVsock.lex.tex"
    And the independent formal control theorems in "tests/lexlean/SpiralCoreVsockControls.lex.tex"
    When the LexLean kernel verifies the SpiralCore virtio-vsock control suite
    Then AF_INET sockets are refused, valid 16-byte SpiralCore AF_VSOCK frames and EM_RISCV ELF64 headers are admitted, and guest secret leakage is rejected
