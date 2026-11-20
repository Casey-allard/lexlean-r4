Feature: Verdant M15 Bidirectional Self-Improvement Ratchet-Cycle and Upstream Authority Refresh (R4-69)

  @R4-69 @build
  Scenario: Source-tree SHA-256 binding, 7-category lifted lifecycle, bidirectional floor/ceiling auto-tightening, and DR-01/DR-02 catalog coverage
    Given the formal R4.VerdantRatchetCycle module in src/R4/VerdantRatchetCycle.lex.tex and controls in tests/lexlean/VerdantRatchetCycleControls.lex.tex
    When evaluating live Rust and JS source-tree SHA-256 bindings, 7 lifecycle categories, lifted-function floors, residue and file-line ceilings, and Prism-DevRel DR-01/DR-02 coverage
    Then synchronized source trees across all 7 lifecycle categories within floor/ceiling watermarks admit as RatchetCycleAdmitted and complete facet catalogs admit as CatalogCoveredAdmitted
    And stale Rust/JS source-tree SHA-256 digests refuse with RefusedStaleSourceTreeSha256, residue or file-line ceiling inflation refuses with RefusedWatermarkCeilingInflation, and uncovered DevRel records refuse with RefusedUncoveredSubjectRecord
