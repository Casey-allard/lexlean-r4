Feature: CI/CD Workflow & Publishing Turnaround Optimization (R4-48)

  @R4-48 @build
  Scenario: Layered target caching, fast-fail gate ordering, and decoupled weight manifests optimize publishing turnaround without weakening verification
    Given the CI/CD pipeline definitions in .github/workflows/pages.yml, Justfile recipe vv, and public/weights/manifest.json
    When GitHub Pages deployment workflows, normative acceptance gate ordering, and static weight shard distributions are evaluated
    Then layered Cargo target caching with Cargo.lock hash keys, zero-compilation fast-fail check sequencing (fmt-check -> template-check -> model -> lint -> test -> features -> deny), and decoupled manifest pointers for large weight shards are admitted
    And un-cached full rebuilds on unchanged lockfiles, inverted gate ordering executing slow test suites before zero-cost format checks, and un-manifested bloated static payload bundles are refused
