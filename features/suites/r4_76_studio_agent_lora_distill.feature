Feature: R4-76 Studio Agent Multi-Hop LoRA Corpus & UORR4V14 C1/S5 Coding Trajectory Distillation

  @R4-76 @build
  Scenario: Multi-hop MVM coding, <prism-write>, and .lex.tex trajectories compile into byte-bounded UORR4V14 C1/S5 atlases in lockstep with STUDIO_AGENT_TOOL_SCHEMA
    Given the formal LexLean module R4.StudioAgentLoraDistill and its proof controls StudioAgentLoraDistillControls
    When xtask studio-compile-v14-corpus compiles multi-hop MVM cargo/rustc/cc/npm, <prism-write>, and .lex.tex repair episodes
    Then >= 24 total episodes and >= 8 multi-hop tool-repair episodes are emitted in lockstep with STUDIO_SELF_KNOWLEDGE_PREFIX and STUDIO_AGENT_TOOL_SCHEMA
    And both Qwen3-0.6B and Qwen3-1.7B UORR4V14 C1/S5 atlases compile with 0 float ops, <= 1 MiB C1/S5 context bytes, and verified 2-of-3 Stitch parity
    And negative controls refuse float ops, insufficient multi-hop trajectories, C1/S5 byte overflows, broken Stitch/ChatML hop parity, and schema lockstep drift
