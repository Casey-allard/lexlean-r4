Feature: R4-79 Content-addressed GitHub Actions LexLean verification cache
  As the LexLean-R4 continuous integration and GitHub Pages pipeline
  I want .lexlean verification artifacts cached under full-commit-pinned actions/cache bound to prismpm.lock, lexlean.lock, and all .lex.tex sources
  So that unchanged Lean proof artifacts skip redundant recompilation without modifying universal bootstrap.yml

  @R4-79 @build
  Scenario: Commit-pinned .lexlean cache in pages.yml with immutable bootstrap.yml
    Given the formal LexLean module R4.ContentAddressedCiCache and its control theorems are verified
    When .github/workflows/pages.yml and .github/workflows/bootstrap.yml are audited against template.lock
    Then pages.yml caches path .lexlean using a 40-hex commit-pinned actions/cache reference bound to prismpm.lock, lexlean.toml, lexlean.lock, src/R4/**/*.lex.tex, and tests/lexlean/**/*.lex.tex
    And universal .github/workflows/bootstrap.yml remains byte-for-byte identical to template.lock
