Feature: Autonomous 7-Stage Engine Application Synthesis & Atomic Rollback (R4-47)

  @R4-47 @build
  Scenario: Stage 6 application synthesis and Stage 7 atomic worktree rollback maintain strict clean-tree homeostasis
    Given a clean repository worktree with Stage 1 through Stage 5 engine frontier updates
    When Stage 6 runs Verdant-Resonance lift-register, 4-facet synthesis, conformance regeneration, and Wasm rebuild checks followed by Stage 7 gate verification
    Then passing ticks commit only allowlisted synthesis artifacts and failing ticks execute rollback_failed_tick_worktree (git checkout, reset, clean) restoring zero dirty or untracked files
    And partial unverified application edits, dirty working trees on gate refusal, and out-of-allowlist commits are refused
