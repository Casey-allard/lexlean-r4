//! Shared checks for the exact reviewed LexLean control source and SDK evidence.

use std::collections::BTreeSet;
use std::error::Error;
use std::process::Command;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Verify an immutable control fixture and its complete selected source graph.
/// The digest binds the independently authored statements, not implementation bytes.
pub fn verify_controls(
    control_path: &str,
    control_module: &str,
    expected_digest: &str,
    modules: &[&str],
) -> Result<(), Box<dyn Error>> {
    let root = repo_model::repo_root();
    let source = std::fs::read_to_string(root.join(control_path))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(source.as_bytes())),
        expected_digest,
        "reviewed control statements changed: {control_path}"
    );
    let data = source
        .split_once("\\semanticdata{")
        .ok_or("missing semantic data")?
        .1
        .split_once("\n\\end{semanticmodule}")
        .ok_or("missing semantic module end")?
        .0
        .strip_suffix('}')
        .ok_or("missing semantic data end")?;
    let controls: Value = serde_json::from_str(data)?;
    let declarations = controls["declarations"]
        .as_array()
        .ok_or("missing declarations")?;
    assert!(!declarations.is_empty(), "empty control fixture");
    let mut expected_names = BTreeSet::new();
    for declaration in declarations {
        assert_eq!(declaration["kind"], "theorem");
        assert_eq!(declaration["axioms"], json!([]));
        let name = declaration["name"].as_str().ok_or("missing theorem name")?;
        assert!(expected_names.insert(format!("LexLeanR4.{control_module}.{name}")));
    }
    let output = Command::new("lexlean")
        .current_dir(&root)
        .args(["--diagnostic-format", "json", "verify", control_path])
        .output()?;
    assert!(
        output.status.success(),
        "LexLean verification failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(result["spec"], "lexlean/command-result/1");
    assert_eq!(result["command"], "verify");
    assert_eq!(result["success"], true);
    assert_eq!(result["exit_code"], 0);
    assert_eq!(result["diagnostics"], json!([]));
    assert_eq!(result["modules"], json!(modules));
    let id = result["attestation_id"]
        .as_str()
        .ok_or("missing attestation ID")?;
    assert!(id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()));
    let directory = format!(".lexlean/verified/{id}");
    assert_eq!(result["artifacts"], json!([directory]));
    let receipt: Value = serde_json::from_slice(&std::fs::read(
        root.join(&directory).join("attestation.json"),
    )?)?;
    assert_eq!(receipt["spec"], "lexlean/attestation/1");
    assert_eq!(receipt["status"], "verified");
    for field in ["attestation_id", "build_id", "source_id", "semantic_id"] {
        assert!(result[field].is_string(), "missing identity {field}");
        assert_eq!(receipt[field], result[field], "receipt identity {field}");
    }
    let rows = receipt["declarations"]
        .as_array()
        .ok_or("missing audit rows")?;
    let mut audited_names = BTreeSet::new();
    for row in rows {
        let name = row["name"].as_str().ok_or("missing audit name")?;
        assert!(
            audited_names.insert(name.to_owned()),
            "duplicate audit row {name}"
        );
        assert_eq!(row["observed"], json!([]));
        assert_eq!(row["policy"], json!({"kind": "none", "axioms": []}));
        assert_eq!(row["result"], "ok");
    }
    assert!(
        expected_names.is_subset(&audited_names),
        "missing audited controls"
    );
    let processes = receipt["processes"]
        .as_array()
        .ok_or("missing process rows")?;
    for module in modules {
        let qualified = format!("LexLeanR4.{module}");
        assert_eq!(
            processes
                .iter()
                .filter(|p| p["tool"] == "leanchecker"
                    && p["module"] == qualified
                    && p["exit_code"] == 0)
                .count(),
            1,
            "missing independent kernel replay for {module}"
        );
    }
    let manifest_bytes = std::fs::read(root.join(directory).join("build-manifest.json"))?;
    assert_eq!(
        receipt["build_manifest"]["sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    assert_eq!(
        receipt["build_manifest"]["byte_length"],
        json!(manifest_bytes.len())
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    assert_eq!(manifest["spec"], "lexlean/build-manifest/1");
    for field in ["build_id", "source_id", "semantic_id"] {
        assert_eq!(manifest[field], result[field]);
    }
    let inputs = manifest["inputs"]
        .as_array()
        .ok_or("missing build inputs")?;
    let canonical_root = root.canonicalize()?;
    for input in inputs.iter().filter(|i| i["kind"] != "lexicon") {
        let path = root
            .join(input["path"].as_str().ok_or("missing input path")?)
            .canonicalize()?;
        assert!(
            path.starts_with(&canonical_root),
            "build input escapes project"
        );
        let bytes = std::fs::read(path)?;
        assert_eq!(
            input["sha256"],
            format!("{:x}", Sha256::digest(&bytes)),
            "stale verified input"
        );
        assert_eq!(input["byte_length"], json!(bytes.len()));
    }
    assert_eq!(
        inputs
            .iter()
            .filter(|i| i["kind"] == "source"
                && i["path"] == control_path
                && i["sha256"] == expected_digest)
            .count(),
        1,
        "wrong or stale control source in receipt"
    );
    Ok(())
}
