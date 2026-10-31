//! Complete checked integer and signed-scalar precursor conformance.
//! This gate does not establish prime, floating-phase or codebook construction.

mod support;

use serde_json::Value;

#[test]
fn exact_codebook_words_r4_21() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let inventory_bytes = std::fs::read(root.join("tests/fixtures/codebook/word-controls.json"))?;
    assert_eq!(
        support::sha256_hex(&inventory_bytes),
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
        count += fixture["theorem_count"]
            .as_u64()
            .ok_or("missing control count")?;
    }
    assert_eq!(count, 2404);

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let mut sorted_fixtures = fixtures.clone();
    sorted_fixtures.sort_by_key(|f| std::cmp::Reverse(f["theorem_count"].as_u64().unwrap_or(0)));
    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let workers = 4;
        for _ in 0..workers {
            let tx = tx.clone();
            let next_idx = &next_idx;
            let sorted_fixtures = &sorted_fixtures;
            s.spawn(move || {
                loop {
                    let i = next_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= sorted_fixtures.len() {
                        break;
                    }
                    let fixture = &sorted_fixtures[i];
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
                    if let Err(e) = support::verify_controls(
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
                    ) {
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
