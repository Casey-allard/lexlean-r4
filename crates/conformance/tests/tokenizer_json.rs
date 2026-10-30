//! Complete raw JSON parsing, document revalidation and tokenizer extraction.
//! The actual LexLean verifier executes every frozen control with its full closure.

mod support;

use serde_json::Value;

#[test]
fn complete_tokenizer_json_r4_20() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_model::repo_root();
    let inventory_bytes = std::fs::read(root.join("tests/fixtures/tokenizer/json-controls.json"))?;
    assert_eq!(
        support::sha256_hex(&inventory_bytes),
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
        count += fixture["controls"]
            .as_u64()
            .ok_or("missing control count")?;
    }
    assert_eq!(count, 399);

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        let workers = 4;
        for _ in 0..workers {
            let tx = tx.clone();
            let next_idx = &next_idx;
            s.spawn(move || {
                loop {
                    let i = next_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= fixtures.len() {
                        break;
                    }
                    let fixture = &fixtures[i];
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
                        "R4.TokenizerText",
                        "R4.TokenizerTextUnicode",
                    ];
                    if module.starts_with("TokenizerJson") || module == "JsonAdditionalControls" {
                        modules.extend(["R4.TokenizerJson", "R4.TokenizerKernel"]);
                    }
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
