//! In-Wasm Rust AST parser, typechecker, interpreter, and live `#[test]` runner controls.

mod support;

#[test]
fn complete_wasm_ast_eval_r4_32() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/WasmAstEvalControls.lex.tex",
        "WasmAstEvalControls",
        "26925ed50e4b53be961ab612ec8a9dd8f88ef6c3b3583da56c5f3b4bd65704a2",
        &["WasmAstEvalControls", "R4.WasmAstEval"],
    )
}
