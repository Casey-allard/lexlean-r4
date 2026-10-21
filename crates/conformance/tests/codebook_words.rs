//! Complete checked integer and signed-scalar precursor conformance.
//! This gate does not establish prime, floating-phase or codebook construction.

mod support;

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn exact_codebook_words_r4_21() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let inventory_bytes = std::fs::read(root.join("tests/fixtures/codebook/word-controls.json"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&inventory_bytes)),
        "d591ea7a83579738c1aef20b0758b6d74482026850d3feb7b6862b929256a886",
        "reviewed codebook integer fixture inventory changed"
    );
    let inventory: Value = serde_json::from_slice(&inventory_bytes)?;
    assert_eq!(inventory["schema"], "lexlean-r4/codebook-word-controls/1");
    assert_eq!(inventory["conformance_id"], "R4-21");
    assert_eq!(inventory["source_module"], "R4.CodebookWords");
    assert_eq!(inventory["control_count"], 2404);
    let fixtures = inventory["fixtures"].as_array().ok_or("missing fixtures")?;
    assert_eq!(fixtures.len(), 7);
    let mut count = 0;
    for fixture in fixtures {
        let path = fixture["path"].as_str().ok_or("missing control path")?;
        let module = fixture["module"].as_str().ok_or("missing control module")?;
        let digest = fixture["sha256"].as_str().ok_or("missing control digest")?;
        count += fixture["theorem_count"]
            .as_u64()
            .ok_or("missing control count")?;
        support::verify_controls(
            path,
            module,
            digest,
            &[
                module,
                "R4.Carrier",
                "R4.CodebookWords",
                "R4.Geometry",
                "R4.Objective",
            ],
        )?;
    }
    assert_eq!(count, 2404);
    Ok(())
}
