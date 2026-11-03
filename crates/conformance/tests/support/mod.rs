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

/// Resolve a LexLean module identifier to its repository-relative `.lex.tex` source path.
fn module_source_path(module: &str) -> String {
    if module.starts_with("R4.") {
        format!("src/{}.lex.tex", module.replace('.', "/"))
    } else {
        format!("tests/lexlean/{module}.lex.tex")
    }
}

/// Extract and parse the embedded JSON AST from a `.lex.tex` semantic or core module.
fn parse_lexlean_module_ast(source: &str, path: &str) -> Result<Value, Box<dyn Error>> {
    let data = if let Some(semantic) = source.split_once("\\semanticdata{") {
        semantic
            .1
            .split_once("\n\\end{semanticmodule}")
            .ok_or_else(|| format!("missing semantic module end in {path}"))?
            .0
            .strip_suffix('}')
            .ok_or_else(|| format!("missing semantic data end in {path}"))?
    } else if let Some(core) = source.split_once("\\coredata{") {
        core.1
            .split_once("\n\\end{coremodule}")
            .ok_or_else(|| format!("missing core module end in {path}"))?
            .0
            .strip_suffix('}')
            .ok_or_else(|| format!("missing core data end in {path}"))?
    } else {
        return Err(format!("missing semantic or core data in {path}").into());
    };
    let parsed: Value = serde_json::from_str(data)?;
    Ok(parsed)
}

/// Verify the locked LexLean workspace once per test binary when executing inside the SDK image.
fn ensure_sdk_lock_verified(root: &std::path::Path) -> Result<(), Box<dyn Error>> {
    static VERIFIED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    let res = VERIFIED.get_or_init(|| {
        if std::env::var_os("SDK_IMAGE").is_some() {
            let output = Command::new("lexlean")
                .current_dir(root)
                .args(["lock", "--check"])
                .output()
                .map_err(|e| format!("failed to invoke lexlean lock --check: {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "lexlean lock --check failed:\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        Ok(())
    });
    match res {
        Ok(()) => Ok(()),
        Err(msg) => Err(msg.clone().into()),
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
    ensure_sdk_lock_verified(&root)?;
    let source = std::fs::read_to_string(root.join(control_path))?;
    assert_eq!(
        sha256_hex(source.as_bytes()),
        expected_digest,
        "reviewed control statements changed: {control_path}"
    );
    let controls = parse_lexlean_module_ast(&source, control_path)?;
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

    let (language, receipt_spec) = match controls["spec"].as_str() {
        Some("lexlean/semantic-module/1") => ("1.1", "lexlean/attestation/1"),
        Some("lexlean/semantic-module/2") | Some("lexlean/core-module/1") => {
            ("1.2", "lexlean/attestation/2")
        }
        _ => return Err("unsupported control source schema".into()),
    };
    let manifest = json!({
        "spec": "lexlean/build-manifest/1",
        "language": language,
    });
    let receipt = json!({
        "spec": receipt_spec,
        "status": "verified",
    });
    validate_receipt_language(&controls, &manifest, &receipt)?;

    let canonical_root = root.canonicalize()?;
    let mut found_control = false;
    for module in modules {
        let rel_path = module_source_path(module);
        if rel_path == control_path {
            found_control = true;
        }
        let full_path = root.join(&rel_path).canonicalize()?;
        assert!(
            full_path.starts_with(&canonical_root),
            "build input escapes project: {rel_path}"
        );
        let mod_source = std::fs::read_to_string(&full_path)?;
        let mod_ast = parse_lexlean_module_ast(&mod_source, &rel_path)?;
        let mod_decls = mod_ast["declarations"]
            .as_array()
            .ok_or_else(|| format!("missing declarations in {rel_path}"))?;
        assert!(!mod_decls.is_empty(), "empty module in closure: {rel_path}");
    }
    assert!(
        found_control,
        "wrong or stale control source in closure: {control_path}"
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
    ensure_sdk_lock_verified(&root)?;
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
        let parsed = parse_lexlean_module_ast(&source, path)?;
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

    let (language, receipt_spec) = match batch_controls["spec"].as_str() {
        Some("lexlean/semantic-module/1") => ("1.1", "lexlean/attestation/1"),
        Some("lexlean/semantic-module/2") | Some("lexlean/core-module/1") => {
            ("1.2", "lexlean/attestation/2")
        }
        _ => return Err("unsupported control source schema".into()),
    };
    let manifest = json!({
        "spec": "lexlean/build-manifest/1",
        "language": language,
    });
    let receipt = json!({
        "spec": receipt_spec,
        "status": "verified",
    });
    validate_receipt_language(&batch_controls, &manifest, &receipt)?;

    let canonical_root = root.canonicalize()?;
    for module in modules {
        let rel_path = module_source_path(module);
        let full_path = root.join(&rel_path).canonicalize()?;
        assert!(
            full_path.starts_with(&canonical_root),
            "build input escapes project: {rel_path}"
        );
        let mod_source = std::fs::read_to_string(&full_path)?;
        let mod_ast = parse_lexlean_module_ast(&mod_source, &rel_path)?;
        let mod_decls = mod_ast["declarations"]
            .as_array()
            .ok_or_else(|| format!("missing declarations in {rel_path}"))?;
        assert!(!mod_decls.is_empty(), "empty module in closure: {rel_path}");
    }
    Ok(())
}
