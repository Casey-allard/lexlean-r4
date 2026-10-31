//! Complete bounded JSON-number observation and exact finite arithmetic.
//! Raw JSON values and production export remain separate acceptance gates.

mod support;

#[test]
fn exact_json_number_r4_18() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    assert_eq!(
        support::sha256_hex(&std::fs::read(
            root.join("tests/fixtures/tokenizer/number-arithmetic.json"),
        )?),
        "71ddc658b5793e10a553bf36263e8f4e07826c5fd34a15bd6f6013389d257d27",
        "reviewed exact integer arithmetic facts changed"
    );
    support::verify_controls(
        "tests/lexlean/JsonNumberArithmeticControls.lex.tex",
        "JsonNumberArithmeticControls",
        "92d6ef7689f4370840aac9ffc38e02bf540782e68067155076482b80a87da6df",
        &["JsonNumberArithmeticControls", "R4.JsonNumberArithmetic"],
    )?;
    for (path, module, digest) in [
        (
            "tests/lexlean/JsonNumberSyntaxControls.lex.tex",
            "JsonNumberSyntaxControls",
            "1641eb1060f792031c42ef1fcb60a4bc52a8bf4d0b5c93b2c18c18ce9197491e",
        ),
        (
            "tests/lexlean/JsonNumberExponentControls.lex.tex",
            "JsonNumberExponentControls",
            "6b597c6638fcad5151ca45aef501dcdf5e21d51b52d9e95d9ca8b48937951e52",
        ),
        (
            "tests/lexlean/JsonNumberBoundaryControls.lex.tex",
            "JsonNumberBoundaryControls",
            "2e9999ef4d3495c5be8aeffff376e6f723c699edab6e4026e456d34e2d6d5b8b",
        ),
        (
            "tests/lexlean/JsonNumberCombineControls.lex.tex",
            "JsonNumberCombineControls",
            "d2f96367e5483f097a5fabb74f047b55390ea523dcd4bf875031ce61c2664960",
        ),
    ] {
        support::verify_controls(
            path,
            module,
            digest,
            &[module, "R4.JsonNumber", "R4.JsonNumberArithmetic"],
        )?;
    }
    Ok(())
}
