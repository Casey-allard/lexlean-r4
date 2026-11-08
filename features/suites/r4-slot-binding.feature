Feature: Complete De Bruijn variable slot-binding and live kernel execution in the decoder
  De Bruijn variable slot-binding across integer, algebraic expression, identifier, and file-path entities, live execution of exact R4-27 arithmetic, R4-24 Icosian, and AST kernels over bound VSA register slots, and /7 token-stream substitution preserve exact integer semantics.
  Formal controls establish no unbounded natural-language semantic understanding or unverified external solver completeness.

  @R4-31 @build
  Scenario: Complete De Bruijn variable slot-binding and live kernel execution in the decoder
    Given the pure LexLean De Bruijn slot-binding and live kernel model in "src/R4/SlotBinding.lex.tex"
    And the independent formal control theorems in "tests/lexlean/SlotBindingControls.lex.tex"
    When the LexLean kernel verifies the De Bruijn slot-binding and live kernel control suite
    Then every De Bruijn register binding, live kernel evaluation, and /7 token-stream substitution invariant holds
