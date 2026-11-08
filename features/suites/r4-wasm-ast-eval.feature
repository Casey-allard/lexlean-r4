Feature: Complete the in-Wasm Rust AST parser, typechecker, interpreter, and live test runner
  In-Wasm Rust AST module admission with exact line-and-column syntax and type diagnostics, fuel-bounded deterministic expression and statement evaluation, and live #[test] assertion verification preserve exact integer semantics.
  Formal controls establish no full rustc borrow-checker equivalence or unbounded Turing-complete termination.

  @R4-32 @build
  Scenario: Complete the in-Wasm Rust AST parser, typechecker, interpreter, and live test runner
    Given the pure LexLean in-Wasm AST evaluation and live test runner model in "src/R4/WasmAstEval.lex.tex"
    And the independent formal control theorems in "tests/lexlean/WasmAstEvalControls.lex.tex"
    When the LexLean kernel verifies the in-Wasm AST evaluation and live test runner control suite
    Then every in-Wasm AST parse admission, fuel-bounded step evaluation, and live test assertion invariant holds
