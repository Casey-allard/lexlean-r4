Feature: R4-85 Continuous GitHub r4-engine Dual-Surface Ratchet Pipeline

  @R4-85 @build
  Scenario: Scheduled and dispatched GitHub r4-engine ticks co-evolve proofs and application directly onto main with bounded cadence
    Given .github/workflows/engine.yml running inside the immutable prismpm.lock SDK container with commit-pinned .lexlean cache
    When an autonomous tick executes dual_surface_ratchet, builds Wasm and .holo, tightens model/ratchet.toml watermarks, and passes just vv
    Then R4.AutonomousGithubRatchetLoop admits AutonomousTickAdmitted pushing directly to main and triggering pages.yml without runaway self-retrigger loops
    And refuses stale main tips, recursive push triggers authored by r4-engine, missing .lexlean caches, untightened watermarks, or unthrottled self-retriggers
