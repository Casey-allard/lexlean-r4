//! Complete Snapshot storage decoding and exact encoding controls.

mod support;

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn complete_snapshot_storage_r4_22() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let bytes = std::fs::read(root.join("tests/fixtures/snapshot/controls.json"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "3585063b5301c09545a945c6c39e6fcdab516bb32967bf7c0d684558838b6492",
        "reviewed Snapshot fixture inventory changed"
    );
    let inventory: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(inventory["spec"], "lexlean-r4/snapshot-json-controls/1");
    assert_eq!(inventory["model_id"], "R4-22");
    assert_eq!(inventory["controls"], 476);
    let fixtures = inventory["modules"].as_array().ok_or("missing fixtures")?;
    assert_eq!(fixtures.len(), 9);
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
            "R4.SnapshotJson",
            "R4.TokenizerText",
            "R4.TokenizerTextUnicode",
        ];
        modules.sort_unstable();
        support::verify_controls(path, module, digest, &modules)?;
    }
    assert_eq!(count, 476);
    Ok(())
}
