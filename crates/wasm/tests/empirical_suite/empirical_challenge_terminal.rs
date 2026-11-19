//! Empirical Challenger Test Suite for Terminal Execution & Control Plane (Milestone 3 / R3).
//!
//! Validates:
//! 1. Dynamic empirical stress harness execution (`tests/e2e/empirical_terminal_harness.mjs`):
//!    - Command parsing (empty lines, whitespace, quotes, unclosed quotes, args).
//!    - ANSI escape sequence parsing (16-color, formatting, semicolon compound, malformed/truncated escapes).
//!    - Environment variable expansions ($VAR, builtins, dynamic set/get/unset, edge cases).
//!    - History buffer boundary stepping, draft preservation, deduplication, 100-entry capacity cap, 1,000 randomized steps.
//!    - Wasm control plane dispatch, integer epoch validation on `train`, Ctrl+C and button interactions.
//! 2. Contractual DOM elements in `public/index.html`.
//! 3. ANSI styling rules in `public/app.css`.
//! 4. Zero forbidden network/telemetry tokens and zero float narrowing in `public/app.js`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root path")
        .to_path_buf()
}

#[test]
fn test_empirical_terminal_node_stress_harness_execution() {
    let root = repo_root();
    let harness_path = root.join("tests/e2e/empirical_terminal_harness.mjs");
    assert!(
        harness_path.is_file(),
        "empirical terminal harness script must exist at {}",
        harness_path.display()
    );

    let output = Command::new("node")
        .arg(&harness_path)
        .current_dir(&root)
        .output()
        .expect("execute node empirical terminal harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Node terminal stress harness failed with exit code {:?}.\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("ALL TERMINAL & ANSI EMPIRICAL STRESS TESTS PASSED WITH ZERO FAILURES"),
        "Harness did not report completion success.\nSTDOUT:\n{}",
        stdout
    );
}

#[test]
fn test_empirical_terminal_dom_bindings_in_index_html() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    let contractual_terminal_ids = [
        "topbar-terminal-btn",
        "canvas-tab-terminal",
        "canvas-panel-terminal",
        "terminal-container",
        "terminal-topbar",
        "terminal-output",
        "terminal-form",
        "terminal-prompt-label",
        "terminal-input",
        "terminal-clear-btn",
        "terminal-reset-btn",
    ];

    for id in &contractual_terminal_ids {
        assert!(
            index_html.contains(&format!("id=\"{id}\"")),
            "index.html must contain contractual terminal DOM id=\"{id}\""
        );
    }

    // Accessible labeling
    assert!(
        index_html.contains("aria-label=\"Terminal input\"")
            || index_html.contains("aria-label=\"Terminal Command Input\"")
            || index_html.contains("id=\"terminal-prompt-label\""),
        "Terminal input must have accessible labeling"
    );
}

#[test]
fn test_empirical_terminal_ansi_classes_in_app_css() {
    let root = repo_root();
    let app_css = std::fs::read_to_string(root.join("public/app.css")).expect("read app.css");

    let ansi_classes = [
        ".ansi-bold",
        ".ansi-dim",
        ".ansi-italic",
        ".ansi-underline",
        ".ansi-red",
        ".ansi-green",
        ".ansi-yellow",
        ".ansi-blue",
        ".ansi-magenta",
        ".ansi-cyan",
        ".ansi-white",
    ];

    for class in &ansi_classes {
        assert!(
            app_css.contains(class),
            "app.css must define ANSI styling rule for {class}"
        );
    }
}

#[test]
fn test_empirical_terminal_closure_and_integer_invariants() {
    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");

    assert!(
        app_js.contains("class LexLeanTerminal"),
        "app.js must declare LexLeanTerminal"
    );
    assert!(
        app_js.contains("function decodeAnsiToDom"),
        "app.js must declare decodeAnsiToDom"
    );

    // Verify terminal epoch regex validation
    assert!(
        app_js.contains("/^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/"),
        "app.js terminal must strictly validate integer epochs"
    );

    // Verify zero float narrowing and zero fetch
    assert!(!app_js.contains("fetch("), "app.js must not contain fetch(");
    assert!(
        !app_js.contains("parseFloat("),
        "app.js must not contain parseFloat("
    );
    assert!(
        !app_js.contains("parseInt("),
        "app.js must not contain parseInt("
    );
}
