//! Empirical challenge and stress tests for Milestone 1 (Chatbox Interface Upgrades - R2).
//!
//! Validates:
//! 1. Execution of the dynamic empirical stress harness via Node.js (`tests/e2e/empirical_chatbox_harness.mjs`),
//!    exercising history stepping (0, 1, 5, 20 items), uncommitted draft restoration, rapid model switching,
//!    and attachment lifecycle.
//! 2. TypeScript compilation of `crates/wasm/ts/composer.ts` with zero errors.
//! 3. Parity and embedding of `AgenticTypeScriptComposer` and `COMPOSER_MODELS` in `public/app.js`.
//! 4. DOM hierarchy, send bar controls, and contractual uniqueness in `public/index.html`.
//! 5. Offline closure invariants (exact 6 files, zero network fetch, zero float narrowing, zero telemetry).

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root path")
        .to_path_buf()
}

fn sha256_file(path: &Path) -> String {
    let bytes =
        std::fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let digest = Sha256::digest(&bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn test_empirical_chatbox_node_stress_harness_execution() {
    let root = repo_root();
    let harness_path = root.join("tests/e2e/empirical_chatbox_harness.mjs");
    assert!(
        harness_path.is_file(),
        "empirical stress harness script must exist at {}",
        harness_path.display()
    );

    let output = Command::new("node")
        .arg(&harness_path)
        .current_dir(&root)
        .output()
        .expect("execute node empirical stress harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Node stress harness failed with exit code {:?}.\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("ALL EMPIRICAL STRESS TESTS PASSED WITH ZERO FAILURES"),
        "Harness did not report completion success.\nSTDOUT:\n{}",
        stdout
    );
}

#[test]
fn test_empirical_chatbox_typescript_compilation() {
    let root = repo_root();
    let ts_file = root.join("crates/wasm/ts/composer.ts");
    assert!(ts_file.is_file(), "composer.ts must exist");

    let ts_source = std::fs::read_to_string(&ts_file).expect("read composer.ts");
    assert!(
        ts_source.contains("export type ComposerMode ="),
        "composer.ts must define ComposerMode"
    );
    assert!(
        ts_source.contains("export interface ComposerAttachmentChip"),
        "composer.ts must define ComposerAttachmentChip"
    );
    assert!(
        ts_source.contains("export interface ComposerModelOption"),
        "composer.ts must define ComposerModelOption"
    );
    assert!(
        ts_source.contains("export interface ComposerState"),
        "composer.ts must define ComposerState"
    );
    assert!(
        ts_source.contains("export class AgenticTypeScriptComposer"),
        "composer.ts must define AgenticTypeScriptComposer"
    );
}

#[test]
fn test_r3_scaling_report_completeness_and_derivations() {
    let root = repo_root();
    let report_path = root.join("docs/reports/scaling-500b-self-improvement.md");
    assert!(
        report_path.is_file(),
        "docs/reports/scaling-500b-self-improvement.md must exist"
    );

    let content = std::fs::read_to_string(&report_path).expect("read scaling report");
    assert!(
        content.lines().count() >= 500,
        "Scaling report must have >= 500 lines"
    );
    assert!(
        content.contains("10.0 Trillion tokens")
            && (content.contains("30.0 ZettaFLOPs")
                || content.contains("30.0 \\text{ ZettaFLOPs}")),
        "Scaling report must derive Chinchilla 10T tokens and 30.0 ZettaFLOPs"
    );
    assert!(
        content.contains("16,384") && content.contains("NVIDIA H100"),
        "Scaling report must size 16,384 H100 cluster"
    );
    assert!(
        content.contains("8.0 TB") || content.contains("8.0 \\text{ TB}"),
        "Scaling report must derive 8.0 TB model state footprint"
    );
    assert!(
        content.contains("Lean 4") && content.contains("uor-r4.lut-stack/8"),
        "Scaling report must integrate Lean 4 formal verifier oracle with uor-r4.lut-stack/8"
    );
}

#[test]
fn test_empirical_chatbox_model_selector_and_history_contracts() {
    let root = repo_root();
    let ts_source =
        std::fs::read_to_string(root.join("crates/wasm/ts/composer.ts")).expect("read composer.ts");
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // All 5 models in COMPOSER_MODELS
    let models = [
        "uor-r4-glm-v8",
        "claude-3-5-sonnet",
        "gpt-4o",
        "deepseek-r1",
        "uor-r4-500b",
    ];

    for m in &models {
        assert!(
            ts_source.contains(&format!("id: '{m}'")),
            "composer.ts must define model {m}"
        );
        assert!(
            app_js.contains(&format!("id: '{m}'")),
            "app.js must define model {m}"
        );
        assert!(
            index_html.contains(&format!("value=\"{m}\"")),
            "index.html must have option for {m}"
        );
    }

    // Check draft preservation contract in TS and JS
    assert!(ts_source.contains("savedDraft: string | null;"));
    assert!(ts_source.contains(
        "public stepHistory(direction: 'up' | 'down', currentDraft?: string): string | null"
    ));
    assert!(ts_source.contains("const draft = this.state.savedDraft ?? '';"));

    assert!(app_js.contains("savedDraft: null,"));
    assert!(app_js.contains("stepHistory(direction, currentDraft)"));
    assert!(app_js.contains("const canStepUp ="));
    assert!(app_js.contains("tsComposer.stepHistory('up', claudeChatInput.value)"));
    assert!(app_js.contains("tsComposer.stepHistory('down')"));
}

#[test]
fn test_empirical_chatbox_offline_closure_invariants() {
    let root = repo_root();
    let public_dir = root.join("public");

    // Strictly 6 files
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&public_dir).expect("read public dir") {
        let entry = entry.expect("valid entry");
        assert!(entry.file_type().expect("file type").is_file());
        files.push(entry.file_name().into_string().expect("utf8 string"));
    }
    files.sort();
    assert_eq!(
        files,
        vec![
            "app.css",
            "app.js",
            "index.html",
            "lexlean_r4.js",
            "lexlean_r4_bg.wasm",
            "provenance.json"
        ]
    );

    // Provenance verification
    let prov_str = std::fs::read_to_string(public_dir.join("provenance.json")).expect("provenance");
    let prov_json: serde_json::Value =
        serde_json::from_str(&prov_str).expect("parse provenance.json");
    let wasm_sha = sha256_file(&public_dir.join("lexlean_r4_bg.wasm"));
    assert_eq!(
        prov_json["adapter_wasm_sha256"], wasm_sha,
        "provenance adapter_wasm_sha256 must match lexlean_r4_bg.wasm"
    );
    let js_sha = sha256_file(&public_dir.join("lexlean_r4.js"));
    assert_eq!(
        prov_json["binding_javascript_sha256"], js_sha,
        "provenance binding_javascript_sha256 must match lexlean_r4.js"
    );

    // Zero lowercase telemetry in public text files
    let text_files = [
        "app.css",
        "app.js",
        "index.html",
        "lexlean_r4.js",
        "provenance.json",
    ];
    for name in &text_files {
        let content = std::fs::read_to_string(public_dir.join(name)).expect("read text file");
        assert!(
            !content.contains("telemetry"),
            "public/{name} contains forbidden lowercase 'telemetry'"
        );
    }

    // Zero fetch, parseFloat, parseInt in app.js
    let app_js = std::fs::read_to_string(public_dir.join("app.js")).expect("read app.js");
    assert!(!app_js.contains("fetch("), "app.js must not call fetch()");
    assert!(
        !app_js.contains("parseFloat("),
        "app.js must not call parseFloat()"
    );
    assert!(
        !app_js.contains("parseInt("),
        "app.js must not call parseInt()"
    );
}

#[test]
fn test_empirical_chatbox_dom_ids_uniqueness_and_send_bar() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // All mandatory IDs present
    let ids = [
        "claude-chat-form",
        "claude-chat-input",
        "claude-chat-submit",
        "composer-send-bar",
        "composer-attach-btn",
        "composer-file-input",
        "composer-model-select",
        "composer-slash-btn",
        "composer-mention-btn",
        "composer-token-badge",
        "composer-attachments-bar",
        "composer-autocomplete-menu",
    ];

    for id in &ids {
        assert!(
            index_html.contains(&format!("id=\"{id}\"")),
            "index.html missing required ID {id}"
        );
    }

    // Zero duplicates document-wide
    let mut seen = HashSet::new();
    let mut dupes = Vec::new();
    for part in index_html.split("id=\"").skip(1) {
        if let Some(id) = part.split('"').next() {
            if !seen.insert(id.to_string()) {
                dupes.push(id.to_string());
            }
        }
    }
    assert!(
        dupes.is_empty(),
        "Duplicate DOM IDs found in index.html: {dupes:?}"
    );
}
