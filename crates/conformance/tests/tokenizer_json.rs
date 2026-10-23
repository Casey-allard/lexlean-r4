//! Complete raw JSON parsing, document revalidation and tokenizer extraction.
//! The actual LexLean verifier executes every frozen control with its full closure.

mod support;

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn complete_tokenizer_json_r4_20() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let inventory_bytes = std::fs::read(root.join("tests/fixtures/tokenizer/json-controls.json"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&inventory_bytes)),
        "2fdee4bcee209f5442cab4c6e510b42bea14063c68df6ed82b34e4161e716f12",
        "reviewed raw JSON fixture inventory changed"
    );
    let inventory: Value = serde_json::from_slice(&inventory_bytes)?;
    assert_eq!(inventory["spec"], "lexlean-r4/tokenizer-json-controls/1");
    assert_eq!(inventory["model_id"], "R4-20");
    assert_eq!(inventory["controls"], 399);
    let fixtures = inventory["modules"].as_array().ok_or("missing fixtures")?;
    assert_eq!(fixtures.len(), 12);
    let mut count = 0;
    for fixture in fixtures {
        let path = fixture["path"].as_str().ok_or("missing control path")?;
        let module = fixture["module"].as_str().ok_or("missing control module")?;
        let digest = fixture["sha256"].as_str().ok_or("missing control digest")?;
        count += fixture["controls"]
            .as_u64()
            .ok_or("missing control count")?;
        let mut modules = vec![
            module,
            "R4.Json",
            "R4.JsonNumber",
            "R4.JsonNumberArithmetic",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ];
        if module.starts_with("TokenizerJson") || module == "JsonAdditionalControls" {
            modules.extend(["R4.TokenizerJson", "R4.TokenizerKernel"]);
        }
        modules.sort_unstable();
        support::verify_controls(path, module, digest, &modules)?;
    }
    assert_eq!(count, 399);
    Ok(())
}
