//! Complete Snapshot storage decoding and exact encoding controls.

mod support;

use serde_json::Value;

#[test]
fn complete_snapshot_storage_r4_22() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let bytes = std::fs::read(root.join("tests/fixtures/snapshot/controls.json"))?;
    assert_eq!(
        support::sha256_hex(&bytes),
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
        count += fixture["controls"]
            .as_u64()
            .ok_or("missing control count")?;
    }
    assert_eq!(count, 476);

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::scope(|s| {
        let workers = 3;
        let chunk_size = fixtures.len().div_ceil(workers);
        for chunk in fixtures.chunks(chunk_size) {
            let tx = tx.clone();
            s.spawn(move || {
                for fixture in chunk {
                    let path = match fixture["path"].as_str() {
                        Some(p) => p,
                        None => {
                            let _ = tx.send(Err("missing control path".to_string()));
                            return;
                        }
                    };
                    let module = match fixture["module"].as_str() {
                        Some(m) => m,
                        None => {
                            let _ = tx.send(Err("missing control module".to_string()));
                            return;
                        }
                    };
                    let digest = match fixture["sha256"].as_str() {
                        Some(d) => d,
                        None => {
                            let _ = tx.send(Err("missing control digest".to_string()));
                            return;
                        }
                    };
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
                    if let Err(e) = support::verify_controls(path, module, digest, &modules) {
                        let _ = tx.send(Err(e.to_string()));
                        return;
                    }
                }
                let _ = tx.send(Ok(()));
            });
        }
    });
    drop(tx);
    while let Ok(res) = rx.recv() {
        res.map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    }
    Ok(())
}
