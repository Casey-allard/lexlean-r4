//! Shared checks for the exact reviewed LexLean control source and SDK evidence.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::process::Command;

use serde_json::{json, Value};
mod sha256;
pub use sha256::sha256_hex;

// The immutable control source selects the language, not the returned receipt.
// LexLean 0865df4e emits attestation/2 for language 1.2; its ordinary
// declaration audits and independent kernel replays retain their meaning.
fn validate_receipt_language(
    controls: &Value,
    manifest: &Value,
    receipt: &Value,
) -> Result<(), &'static str> {
    let (language, schema) = match controls["spec"].as_str() {
        Some("lexlean/semantic-module/1") => ("1.1", "lexlean/attestation/1"),
        Some("lexlean/semantic-module/2") => ("1.2", "lexlean/attestation/2"),
        Some("lexlean/core-module/1") => ("1.2", "lexlean/attestation/2"),
        _ => return Err("unsupported control source schema"),
    };
    if manifest["language"] != language {
        return Err("control source and build language differ");
    }
    if receipt["spec"] != schema {
        return Err("control source and attestation schema differ");
    }
    // Production evidence has additional obligations. This formal-control gate
    // must not silently accept it without validating those artifacts.
    if receipt.get("compiler_input").is_some() || receipt.get("preservation").is_some() {
        return Err("production evidence requires a production acceptance gate");
    }
    Ok(())
}

#[test]
fn receipt_language_binding() {
    for (source, language, receipt) in [
        ("lexlean/semantic-module/1", "1.1", "lexlean/attestation/1"),
        ("lexlean/semantic-module/2", "1.2", "lexlean/attestation/2"),
        ("lexlean/core-module/1", "1.2", "lexlean/attestation/2"),
    ] {
        let controls = json!({"spec": source});
        let manifest = json!({"language": language});
        let attestation = json!({"spec": receipt});
        assert!(validate_receipt_language(&controls, &manifest, &attestation).is_ok());
        for wrong in [Value::Null, json!("1.0"), json!("1.3"), json!(0)] {
            assert!(validate_receipt_language(
                &controls,
                &json!({"language": wrong}),
                &attestation
            )
            .is_err());
        }
        let other_language = if language == "1.1" { "1.2" } else { "1.1" };
        assert!(validate_receipt_language(
            &controls,
            &json!({"language": other_language}),
            &attestation
        )
        .is_err());
        for wrong in [Value::Null, json!("lexlean/attestation/3"), json!(0)] {
            assert!(
                validate_receipt_language(&controls, &manifest, &json!({"spec": wrong})).is_err()
            );
        }
        let other_receipt = if language == "1.1" {
            "lexlean/attestation/2"
        } else {
            "lexlean/attestation/1"
        };
        assert!(
            validate_receipt_language(&controls, &manifest, &json!({"spec": other_receipt}))
                .is_err()
        );
        for field in ["compiler_input", "preservation"] {
            let mut production = attestation.clone();
            production[field] = json!({"byte_length": 1, "sha256": "0".repeat(64)});
            assert!(validate_receipt_language(&controls, &manifest, &production).is_err());
            production[field] = Value::Null;
            assert!(validate_receipt_language(&controls, &manifest, &production).is_err());
        }
    }
    for unknown in [Value::Null, json!("lexlean/semantic-module/3"), json!(0)] {
        assert!(validate_receipt_language(
            &json!({"spec": unknown}),
            &json!({"language": "1.2"}),
            &json!({"spec": "lexlean/attestation/2"})
        )
        .is_err());
    }
}

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
        sha256_hex(source.as_bytes()),
        expected_digest,
        "reviewed control statements changed: {control_path}"
    );
    let data = if let Some(semantic) = source.split_once("\\semanticdata{") {
        semantic
            .1
            .split_once("\n\\end{semanticmodule}")
            .ok_or("missing semantic module end")?
            .0
            .strip_suffix('}')
            .ok_or("missing semantic data end")?
    } else if let Some(core) = source.split_once("\\coredata{") {
        core.1
            .split_once("\n\\end{coremodule}")
            .ok_or("missing core module end")?
            .0
            .strip_suffix('}')
            .ok_or("missing core data end")?
    } else {
        return Err("missing semantic or core data".into());
    };
    let controls: Value = serde_json::from_str(data)?;
    let declarations = controls["declarations"]
        .as_array()
        .ok_or("missing declarations")?;
    assert!(!declarations.is_empty(), "empty control fixture");
    let mut expected_names = BTreeSet::new();
    for declaration in declarations {
        assert_eq!(declaration["kind"], "theorem");
        if let Some(axioms) = declaration.get("axioms") {
            assert_eq!(axioms, &json!([]));
        } else if let Some(policy) = declaration.get("policy") {
            assert_eq!(policy, &json!({"kind": "none"}));
        } else {
            return Err("declaration must have empty axioms or none policy".into());
        }
        let name = declaration["name"].as_str().ok_or("missing theorem name")?;
        let qualified = if name.starts_with("LexLeanR4.") {
            name.to_string()
        } else {
            format!("LexLeanR4.{control_module}.{name}")
        };
        assert!(expected_names.insert(qualified));
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
        sha256_hex(&manifest_bytes)
    );
    assert_eq!(
        receipt["build_manifest"]["byte_length"],
        json!(manifest_bytes.len())
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    assert_eq!(manifest["spec"], "lexlean/build-manifest/1");
    validate_receipt_language(&controls, &manifest, &receipt)?;
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
        assert_eq!(input["sha256"], sha256_hex(&bytes), "stale verified input");
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

/// Verify a batch of immutable control fixtures through a single LexLean invocation over their import closure.
pub fn verify_control_batch(
    batch_path: &str,
    batch_module: &str,
    batch_digest: &str,
    controls: &[(&str, &str, &str)],
    modules: &[&str],
) -> Result<(), Box<dyn Error>> {
    let root = repo_model::repo_root();
    let mut expected_names = BTreeSet::new();
    let mut batch_controls = None;

    for (path, module, expected_digest) in
        controls
            .iter()
            .copied()
            .chain(std::iter::once((batch_path, batch_module, batch_digest)))
    {
        let source = std::fs::read_to_string(root.join(path))?;
        assert_eq!(
            sha256_hex(source.as_bytes()),
            expected_digest,
            "reviewed control statements changed: {path}"
        );
        let data = if let Some(semantic) = source.split_once("\\semanticdata{") {
            semantic
                .1
                .split_once("\n\\end{semanticmodule}")
                .ok_or("missing semantic module end")?
                .0
                .strip_suffix('}')
                .ok_or("missing semantic data end")?
        } else if let Some(core) = source.split_once("\\coredata{") {
            core.1
                .split_once("\n\\end{coremodule}")
                .ok_or("missing core module end")?
                .0
                .strip_suffix('}')
                .ok_or("missing core data end")?
        } else {
            return Err(format!("missing semantic or core data in {path}").into());
        };
        let parsed: Value = serde_json::from_str(data)?;
        let declarations = parsed["declarations"]
            .as_array()
            .ok_or("missing declarations")?;
        assert!(!declarations.is_empty(), "empty control fixture: {path}");
        for declaration in declarations {
            assert_eq!(declaration["kind"], "theorem");
            if let Some(axioms) = declaration.get("axioms") {
                assert_eq!(axioms, &json!([]));
            } else if let Some(policy) = declaration.get("policy") {
                assert_eq!(policy, &json!({"kind": "none"}));
            } else {
                return Err("declaration must have empty axioms or none policy".into());
            }
            let name = declaration["name"].as_str().ok_or("missing theorem name")?;
            let qualified = if name.starts_with("LexLeanR4.") {
                name.to_string()
            } else {
                format!("LexLeanR4.{module}.{name}")
            };
            assert!(expected_names.insert(qualified));
        }
        if path == batch_path {
            batch_controls = Some(parsed);
        }
    }
    let batch_controls = batch_controls.ok_or("missing batch controls")?;

    let output = Command::new("lexlean")
        .current_dir(&root)
        .args(["--diagnostic-format", "json", "verify", batch_path])
        .output()?;
    assert!(
        output.status.success(),
        "LexLean verification failed for {batch_path}:\n{}\n{}",
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
        "missing audited controls in {batch_path}"
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
            "missing independent kernel replay for {module} in {batch_path}"
        );
    }
    let manifest_bytes = std::fs::read(root.join(&directory).join("build-manifest.json"))?;
    assert_eq!(
        receipt["build_manifest"]["sha256"],
        sha256_hex(&manifest_bytes)
    );
    assert_eq!(
        receipt["build_manifest"]["byte_length"],
        json!(manifest_bytes.len())
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    assert_eq!(manifest["spec"], "lexlean/build-manifest/1");
    validate_receipt_language(&batch_controls, &manifest, &receipt)?;
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
        assert_eq!(input["sha256"], sha256_hex(&bytes), "stale verified input");
        assert_eq!(input["byte_length"], json!(bytes.len()));
    }
    for (control_path, _, expected_digest) in
        controls
            .iter()
            .copied()
            .chain(std::iter::once((batch_path, batch_module, batch_digest)))
    {
        assert_eq!(
            inputs
                .iter()
                .filter(|i| i["kind"] == "source"
                    && i["path"] == control_path
                    && i["sha256"] == expected_digest)
                .count(),
            1,
            "wrong or stale control source in receipt: {control_path}"
        );
    }
    Ok(())
}
