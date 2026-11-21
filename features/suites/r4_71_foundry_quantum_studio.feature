Feature: R4-71 Foundry Hologram Quantum PC, InTabProver FK-01..09 Lifecycle, and Canonical UOR Kappa
  As a UOR-R4 Holo Studio developer
  I want float-forbidding unique-key canonical JSON kappa addressing, sealed MLS cross-device channels, and FK-01..09 InTabProver proof-gated .holo publication
  So that every Studio application is content-addressed by BLAKE3 kappa and verified sound on propext and Quot.sound before it can ship

  @R4-71 @build
  Scenario: Foundry Quantum PC canonical kappa and FK-01..09 InTabProver proof-gated Studio lifecycle pass verification
    Given the LexLean module "src/R4/FoundryQuantumStudio.lex.tex" and negative controls "tests/lexlean/FoundryQuantumStudioControls.lex.tex"
    When workspace files are edited, checked, proved in-tab, and published through the Quantum PC Studio session
    Then floats and duplicate JSON keys are rejected, edits reset proof state to Drafting, unsound axioms and unproved publishes are refused, and sealed MLS envelopes authenticate cleanly
