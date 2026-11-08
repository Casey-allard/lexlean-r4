Feature: Complete the FIPS 180-4 SHA-256 Merkle SecOps audit chain and container workspace sandbox
  FIPS 180-4 SHA-256 Merkle audit chaining (uor.secops.audit/1), MCP policy gating across safe/confirm/deny modes, and bounded in-memory container workspace admission preserve exact integer semantics.
  Formal controls establish no cryptographic collision resistance or external container runtime isolation.

  @R4-30 @build
  Scenario: Complete the FIPS 180-4 SHA-256 Merkle SecOps audit chain and container workspace sandbox
    Given the pure LexLean SecOps audit and sandbox model in "src/R4/SecopsAudit.lex.tex"
    And the independent formal control theorems in "tests/lexlean/SecopsAuditControls.lex.tex"
    When the LexLean kernel verifies the SecOps audit and sandbox control suite
    Then every FIPS 180-4 SHA-256 Merkle SecOps audit link, MCP policy gate, and container workspace sandbox admission invariant holds
