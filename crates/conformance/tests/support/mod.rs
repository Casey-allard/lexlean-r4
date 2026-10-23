//! Shared checks for the exact reviewed LexLean control source and SDK evidence.

use std::collections::BTreeSet;
use std::error::Error;
use std::process::Command;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
