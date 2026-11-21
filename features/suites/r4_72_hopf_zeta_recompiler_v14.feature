Feature: R4-72 UORR4V14 4D Signed Hopf, Golden-Ratio Poincare, Zeta-Zero, Icosian, and C1/S5 Teacher Recompiler
  As a UOR-R4 geometric model architect
  I want to recompile Qwen3-0.6B and Qwen3-1.7B teacher representations with C1/S5 distance-split whole-context counts into UORR4V14 harmonic atlases
  So that all 16 signed Hopf orthants and the S1 storage fiber angle chi are preserved with zero float ops and bounded <= 1 MiB active context memory

  @R4-72 @build
  Scenario: Dual-tier Qwen3-0.6B and Qwen3-1.7B UORR4V14 recompilation preserves 4D signed Hopf storage angle and <= 1 MiB C1/S5 context budget
    Given the LexLean module "src/R4/HopfZetaRecompilerV14.lex.tex" and negative controls "tests/lexlean/HopfZetaRecompilerV14Controls.lex.tex"
    When the xtask recompile-v14 pipeline compiles Qwen3-0.6B and Qwen3-1.7B harmonic atlases
    Then ablated 2D TurboQuant, unsigned orthants, float exports, and > 1 MiB context tables are refused and verified UORR4V14 containers emit deterministic BLAKE3 kappa digests
