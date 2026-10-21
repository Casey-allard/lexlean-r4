//! Verify the actual boundary statements and fresh SDK attestations.
//! A missing verifier is a failure, never a skipped acceptance case.

use std::error::Error;
use std::process::Command;

use serde_json::{json, Value};

#[test]
fn configuration_boundaries_r4_01() -> Result<(), Box<dyn Error>> {
    let root = repo_model::repo_root();
    let source = std::fs::read_to_string(root.join("tests/lexlean/TypesControls.lex.tex"))?;
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
    let cases = [
        ("minimumSpec", [2, 2, 1, 1], true),
        ("maximumSpec", [8192, 16, 243, 16], true),
        ("rejectVocabularyBelowMinimum", [1, 2, 1, 1], false),
        ("rejectVocabularyAboveMaximum", [8193, 2, 1, 1], false),
        ("rejectSingleBlock", [2, 1, 1, 1], false),
        ("rejectTooManyBlocks", [2, 17, 1, 1], false),
        ("rejectEmptyContext", [2, 2, 0, 1], false),
        ("rejectLongContext", [2, 2, 244, 1], false),
        ("rejectZeroCells", [2, 2, 1, 0], false),
        ("rejectTooManyCells", [8192, 2, 1, 17], false),
        ("rejectCellsExceedVocabulary", [2, 2, 1, 3], false),
    ];
    assert_eq!(declarations.len(), cases.len());
    for (name, inputs, expected) in cases {
        let arguments: Vec<Value> = inputs
            .iter()
            .map(|n| json!({"kind": "nat", "value": n.to_string()}))
            .collect();
        let expected_declaration = json!({
            "kind": "theorem", "name": name, "parameters": [], "axioms": [],
            "proof": {"kind": "decide"},
            "statement": {"kind": "eq", "left": {
                "kind": "call", "function": {"module": "R4.Types", "name": "validSpec"},
                "arguments": arguments
            }, "right": {"kind": "bool", "value": expected}}
        });
        assert_eq!(
            declarations
                .iter()
                .filter(|d| **d == expected_declaration)
                .count(),
            1,
            "missing or altered boundary statement {name}"
        );
    }
    let output = Command::new("lexlean")
        .current_dir(&root)
        .args([
            "--diagnostic-format",
            "json",
            "verify",
            "tests/lexlean/TypesControls.lex.tex",
        ])
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
    assert_eq!(result["modules"], json!(["R4.Types", "TypesControls"]));
    let id = result["attestation_id"]
        .as_str()
        .ok_or("missing attestation ID")?;
    assert!(id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()));
    let directory = format!(".lexlean/verified/{id}");
    assert_eq!(result["artifacts"], json!([directory]));
    let receipt: Value = serde_json::from_slice(&std::fs::read(
        root.join(directory).join("attestation.json"),
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
    assert_eq!(rows.len(), cases.len() + 3);
    for (name, _, _) in cases {
        let expected = json!({"name": format!("LexLeanR4.TypesControls.{name}"),
            "observed": [], "policy": {"axioms": [], "kind": "none"}, "result": "ok"});
        assert_eq!(
            rows.iter().filter(|row| **row == expected).count(),
            1,
            "missing unique audited theorem {name}"
        );
    }
    let processes = receipt["processes"]
        .as_array()
        .ok_or("missing process rows")?;
    for module in ["LexLeanR4.R4.Types", "LexLeanR4.TypesControls"] {
        assert_eq!(
            processes
                .iter()
                .filter(|p| p["tool"] == "leanchecker"
                    && p["module"] == module
                    && p["exit_code"] == 0)
                .count(),
            1,
            "missing independent kernel replay for {module}"
        );
    }
    Ok(())
}
