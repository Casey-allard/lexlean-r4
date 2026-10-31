//! Complete hash controls, partitioned without changing any reviewed declaration.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;

use serde_json::{json, Value};

const ORIGINAL: &str = "tests/fixtures/hash/HashControls.lex.tex";
const ORIGINAL_SHA: &str = "0603c79620ff19e4a30239a7b180b214af5941d925f60161479a0049b8acd60e";
const INVENTORY: &str = "tests/fixtures/hash/inventory.json";
const INVENTORY_SHA: &str = "66f2661bb885aa61c36365329cf267fe21ff7c245ec3a0d98e9751257e3899d7";

fn frozen_source(root: &Path, path: &str, digest: &str) -> Result<String, Box<dyn Error>> {
    let source = std::fs::read_to_string(root.join(path))?;
    assert_eq!(
        support::sha256_hex(source.as_bytes()),
        digest,
        "reviewed hash fixture changed: {path}"
    );
    Ok(source)
}

fn declarations(source: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let data = source
        .split_once("\\semanticdata{")
        .ok_or("missing semantic data")?
        .1
        .split_once("\n\\end{semanticmodule}")
        .ok_or("missing module end")?
        .0
        .strip_suffix('}')
        .ok_or("missing data end")?;
    let module: Value = serde_json::from_str(data)?;
    Ok(module["declarations"]
        .as_array()
        .ok_or("missing declarations")?
        .clone())
}

#[test]
fn complete_content_hashes_r4_11() -> Result<(), Box<dyn Error>> {
    let root = repo_model::repo_root();
    let original = declarations(&frozen_source(&root, ORIGINAL, ORIGINAL_SHA)?)?;
    assert_eq!(original.len(), 30);
    let mut required = BTreeMap::new();
    for declaration in original {
        assert_eq!(declaration["kind"], "theorem");
        assert_eq!(declaration["axioms"], json!([]));
        let name = declaration["name"]
            .as_str()
            .ok_or("missing original name")?
            .to_owned();
        assert!(
            required.insert(name, declaration).is_none(),
            "duplicate original theorem"
        );
    }
    let inventory: Value = serde_json::from_str(&frozen_source(&root, INVENTORY, INVENTORY_SHA)?)?;
    assert_eq!(inventory["original_source_sha256"], ORIGINAL_SHA);
    let groups = inventory["groups"].as_array().ok_or("missing groups")?;
    assert_eq!(groups.len(), 30);
    let mut partitioned = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut modules = BTreeSet::new();
    for (index, group) in groups.iter().enumerate() {
        let module = group["module"].as_str().ok_or("missing module")?;
        let path = group["path"].as_str().ok_or("missing path")?;
        let digest = group["source_sha256"]
            .as_str()
            .ok_or("missing source digest")?;
        let name = group["name"].as_str().ok_or("missing theorem name")?;
        assert_eq!(module, format!("HashCase{:02}", index + 1));
        assert_eq!(path, format!("tests/lexlean/{module}.lex.tex"));
        assert!(paths.insert(path.to_owned()) && modules.insert(module));
        let source = frozen_source(&root, path, digest)?;
        assert!(source.starts_with(&format!("\\begin{{lexlean}}{{{module}}}\n")));
        let rows = declarations(&source)?;
        assert_eq!(
            rows.len(),
            1,
            "each module must preserve one complete theorem"
        );
        let declaration = rows.into_iter().next().ok_or("empty group")?;
        assert_eq!(declaration["name"], name);
        assert_eq!(
            declaration,
            *required.get(name).ok_or("unexpected theorem")?,
            "statement, input, proof, or axiom drift in {module}"
        );
        assert!(
            partitioned.insert(name.to_owned(), declaration).is_none(),
            "duplicate partition theorem"
        );
    }
    assert_eq!(
        partitioned, required,
        "exact complete declaration multiset required"
    );
    let actual_paths: BTreeSet<_> = std::fs::read_dir(root.join("tests/lexlean"))?
        .filter(|entry| {
            entry.as_ref().map_or(true, |entry| {
                entry.file_name().to_string_lossy().starts_with("HashCase")
            })
        })
        .map(|entry| -> Result<String, Box<dyn Error>> {
            let entry = entry?;
            assert!(
                entry.file_type()?.is_file(),
                "unexpected hash fixture directory or symlink"
            );
            Ok(format!(
                "tests/lexlean/{}",
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "non-UTF8 fixture")?
            ))
        })
        .collect::<Result<_, _>>()?;
    assert_eq!(
        actual_paths, paths,
        "missing or unregistered hash control source"
    );
    // Audit the entire frozen partition before invoking any verifier; every case is mandatory.
    // Verify the complete 30-case partition across 3 import-closure batches in parallel.
    let batches = [
        (
            "tests/lexlean/HashBatch01.lex.tex",
            "HashBatch01",
            "221b0a08698a53557eb15089c3dd20ef40f11a2631788aa5e4d54ff75ad56ca8",
            &groups[0..10],
        ),
        (
            "tests/lexlean/HashBatch02.lex.tex",
            "HashBatch02",
            "7b44311ff5e60a1b6f985e34f3f8acbbd6749e5a0e34bc90828caa22378d6ce2",
            &groups[10..20],
        ),
        (
            "tests/lexlean/HashBatch03.lex.tex",
            "HashBatch03",
            "2542f41144915bf07ee87eed7cae3257ecd4fd55e82a021439edcf2551ceab13",
            &groups[20..30],
        ),
    ];

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::scope(|s| {
        for batch in batches {
            let tx = tx.clone();
            s.spawn(move || {
                let (batch_path, batch_module, batch_digest, batch_groups) = batch;
                let mut controls = Vec::new();
                let mut modules = vec![
                    batch_module,
                    "R4.Hash",
                    "R4.HashNat",
                    "R4.HashNatCore",
                    "R4.Words",
                ];
                for group in batch_groups {
                    let module = match group["module"].as_str() {
                        Some(m) => m,
                        None => {
                            let _ = tx.send(Err("missing module".to_string()));
                            return;
                        }
                    };
                    let path = match group["path"].as_str() {
                        Some(p) => p,
                        None => {
                            let _ = tx.send(Err("missing path".to_string()));
                            return;
                        }
                    };
                    let digest = match group["source_sha256"].as_str() {
                        Some(d) => d,
                        None => {
                            let _ = tx.send(Err("missing digest".to_string()));
                            return;
                        }
                    };
                    controls.push((path, module, digest));
                    modules.push(module);
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
        res.map_err(|e| -> Box<dyn Error> { e.into() })?;
    }
    Ok(())
}
