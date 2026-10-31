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

    let fixture_map: std::collections::BTreeMap<&str, &Value> = fixtures
        .iter()
        .filter_map(|f| f["module"].as_str().map(|m| (m, f)))
        .collect();
    assert_eq!(fixture_map.len(), 9);

    let batches = [
        (
            "tests/lexlean/SnapshotBatch01.lex.tex",
            "SnapshotBatch01",
            "a927c0d5c847cfacceb6d4522c43b5015647ccf1053e72eec0a924724525cfcb",
            &[
                "SnapshotStringControls",
                "SnapshotStrictControls",
                "SnapshotBudgetControls",
            ][..],
        ),
        (
            "tests/lexlean/SnapshotBatch02.lex.tex",
            "SnapshotBatch02",
            "2679bc4437cbbafdd572ab0e6d4ecda54c93e9f3d63a716fa5028a7bb0b7fbcd",
            &[
                "SnapshotNumericControls",
                "SnapshotRoundtripControls",
                "SnapshotSchemaControls",
            ][..],
        ),
        (
            "tests/lexlean/SnapshotBatch03.lex.tex",
            "SnapshotBatch03",
            "75ffe36a7ef930c3c53ccb7518fecadce08f3151dc1e9a50e19076a1409a9c9a",
            &[
                "SnapshotRepresentationControls",
                "SnapshotOrderControls",
                "SnapshotStoredValueControls",
            ][..],
        ),
    ];

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::scope(|s| {
        for batch in batches {
            let tx = tx.clone();
            let fixture_map = &fixture_map;
            s.spawn(move || {
                let (batch_path, batch_module, batch_digest, batch_modules) = batch;
                let mut controls = Vec::new();
                let mut modules = vec![
                    batch_module,
                    "R4.Json",
                    "R4.JsonNumber",
                    "R4.JsonNumberArithmetic",
                    "R4.SnapshotJson",
                    "R4.TokenizerText",
                    "R4.TokenizerTextUnicode",
                ];
                for mod_name in batch_modules {
                    let fixture = match fixture_map.get(mod_name) {
                        Some(f) => f,
                        None => {
                            let _ = tx.send(Err(format!("missing fixture {mod_name}")));
                            return;
                        }
                    };
                    let path = match fixture["path"].as_str() {
                        Some(p) => p,
                        None => {
                            let _ = tx.send(Err("missing path".to_string()));
                            return;
                        }
                    };
                    let digest = match fixture["sha256"].as_str() {
                        Some(d) => d,
                        None => {
                            let _ = tx.send(Err("missing digest".to_string()));
                            return;
                        }
                    };
                    controls.push((path, *mod_name, digest));
                    modules.push(mod_name);
                }
                modules.sort_unstable();
                if let Err(e) = support::verify_control_batch(
                    batch_path,
                    batch_module,
                    batch_digest,
                    &controls,
                    &modules,
                ) {
                    let _ = tx.send(Err(e.to_string()));
                    return;
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
