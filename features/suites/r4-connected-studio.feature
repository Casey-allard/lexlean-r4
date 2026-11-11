Feature: Connected Chat-First Studio, Worker Turn Isolation, and SpiralCore wss Bridge Admission
  Connected Studio enforces TLS or loopback SpiralCore CID 2 bridge transport under MCP policy, isolates GitHub secret placeholder substitution strictly to allowlisted GitHub hosts with zero guest RAM contamination, verifies SHA-256 and 64-bit EM_RISCV ELF64 headers before installing external packages, and restricts multi-turn causal KV generation to non-blocking Web Worker turns within the sequence window.
  Formal controls establish no remote Cloudflare Worker availability or external GitHub API liveness.

  @R4-39 @build
  Scenario: Enforce SpiralCore CID 2 TLS bridge transport, GitHub secret host allowlisting, EM_RISCV package verification, and non-blocking Worker sequence bounds
    Given the pure LexLean Connected Studio module and formal control theorems in "tests/lexlean/R4/ConnectedStudio.lex.tex"
    And the generated Rust verifier in "crates/wasm/src/lexlean_generated.rs" under module "r4_connected_studio"
    When the Connected Studio conformance suite evaluates positive and negative transport, secret, package, and worker inputs
    Then SpiralCore CID 3 to CID 2 egress admits WssRelay, HttpsDirect, and LoopbackDev under Safe or Confirm policy while refusing Deny policy, PlainHttp, PlainWs, and invalid vsock CIDs
    And GitHub secret placeholder substitution admits allowlisted GitHub hosts with zero guest RAM exposure while refusing guest RAM secret contamination, missing placeholders, and non-allowlisted hosts
    And package installation admits SHA-256-verified 64-bit EM_RISCV machine 243 binaries within quota while refusing digest mismatches, non-RISC-V or malformed ELF headers, and quota overflows
    And Worker turn admission requires execution off the main UI thread within the 1024-token multi-turn sequence window while refusing main-thread blocking and sequence window overflows
