//! Automated verification tests for Browser Application & GitHub Pages Deployment.
//!
//! Validates:
//! 1. Exact 6-file production closure in `public/`.
//! 2. `provenance.json` schema and cryptographic digest bindings.
//! 3. Strict offline network isolation and zero telemetry.
//! 4. BigInt-safe precision with zero `fetch(` and zero float narrowing in `app.js`.
//! 5. `.github/workflows/pages.yml` compliance with repository bootstrap and runner pinning rules.
//! 6. HTML5 accessibility semantics (WCAG 2.1 AA) and strict Content Security Policy.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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
fn test_browser_closure_contains_exact_six_files() {
    let root = repo_root();
    let public_dir = root.join("public");
    assert!(public_dir.is_dir(), "public/ directory must exist");

    let mut actual_files = Vec::new();
    for entry in std::fs::read_dir(&public_dir).expect("read public dir") {
        let entry = entry.expect("valid entry");
        let file_type = entry.file_type().expect("file type");
        assert!(
            file_type.is_file(),
            "public/ must not contain subdirectories: {:?}",
            entry.path()
        );
        let file_name = entry.file_name().into_string().expect("utf8 file name");
        actual_files.push(file_name);
    }
    actual_files.sort();

    let expected_files = vec![
        "app.css".to_string(),
        "app.js".to_string(),
        "index.html".to_string(),
        "lexlean_r4.js".to_string(),
        "lexlean_r4_bg.wasm".to_string(),
        "provenance.json".to_string(),
    ];

    assert_eq!(
        actual_files, expected_files,
        "public closure must contain exactly the expected 6 files"
    );

    // Verify all files are non-empty
    for name in &expected_files {
        let file_path = public_dir.join(name);
        let metadata = std::fs::metadata(&file_path).expect("file metadata");
        assert!(metadata.len() > 0, "{name} must be non-empty");
    }
}

#[test]
fn test_browser_provenance_schema_and_digests() {
    let root = repo_root();
    let provenance_path = root.join("public/provenance.json");
    let content = std::fs::read_to_string(&provenance_path).expect("read provenance.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON provenance");

    // Schema validation
    assert_eq!(
        json["schema"], "prismpm/browser-provenance/1",
        "provenance schema must match prismpm/browser-provenance/1"
    );

    // Wasm adapter SHA-256 match
    let wasm_path = root.join("public/lexlean_r4_bg.wasm");
    let actual_wasm_sha = sha256_file(&wasm_path);
    assert_eq!(
        json["adapter_wasm_sha256"], actual_wasm_sha,
        "provenance adapter_wasm_sha256 must match public/lexlean_r4_bg.wasm"
    );

    // JS binding SHA-256 match
    let js_path = root.join("public/lexlean_r4.js");
    let actual_js_sha = sha256_file(&js_path);
    assert_eq!(
        json["binding_javascript_sha256"], actual_js_sha,
        "provenance binding_javascript_sha256 must match public/lexlean_r4.js"
    );

    // Valid 64-char hex model ID and hashes
    for field in [
        "adapter_lock_sha256",
        "adapter_wasm_sha256",
        "binding_javascript_sha256",
        "generated_core_sha256",
        "model_id",
        "view_manifest_sha256",
        "view_model_id",
    ] {
        let val = json[field]
            .as_str()
            .unwrap_or_else(|| panic!("missing field {field}"));
        assert_eq!(val.len(), 64, "{field} must be 64 characters");
        assert!(
            val.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "{field} must be lowercase hex"
        );
    }
}

#[test]
fn test_browser_closure_zero_external_network_telemetry() {
    let root = repo_root();
    let public_dir = root.join("public");

    let text_files = [
        "app.css",
        "app.js",
        "index.html",
        "lexlean_r4.js",
        "provenance.json",
    ];
    let forbidden_patterns = [
        "http://",
        "https://",
        "XMLHttpRequest",
        "WebSocket",
        "EventSource",
        "analytics",
        "telemetry",
    ];

    for name in &text_files {
        let content = std::fs::read_to_string(public_dir.join(name)).expect("read text file");
        for pattern in &forbidden_patterns {
            assert!(
                !content.contains(pattern),
                "public/{name} contains forbidden external network/telemetry reference: {pattern}"
            );
        }
    }
}

#[test]
fn test_browser_app_zero_fetch_and_zero_float_narrowing() {
    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");

    // Zero fetch calls
    assert!(
        !app_js.contains("fetch("),
        "app.js must not contain fetch() calls"
    );

    // Zero float conversions on operands
    assert!(
        !app_js.contains("Number(left"),
        "app.js must not narrow left operand with Number()"
    );
    assert!(
        !app_js.contains("Number(right"),
        "app.js must not narrow right operand with Number()"
    );
    assert!(
        !app_js.contains("Number(a"),
        "app.js must not narrow operand a with Number()"
    );
    assert!(
        !app_js.contains("Number(b"),
        "app.js must not narrow operand b with Number()"
    );
    assert!(
        !app_js.contains("parseFloat("),
        "app.js must not parse coordinates with parseFloat()"
    );
    assert!(
        !app_js.contains("parseInt("),
        "app.js must not parse coordinates with parseInt()"
    );

    // BigInt validation presence
    assert!(
        app_js.contains("BigInt("),
        "app.js must validate integer bounds using BigInt"
    );
    assert!(
        app_js.contains("/^(?:0|-[1-9][0-9]*|[1-9][0-9]*)$/"),
        "app.js must use canonical integer regex validation"
    );
}

#[test]
fn test_github_pages_workflow_bootstrap_compliance() {
    let root = repo_root();
    let workflow_path = root.join(".github/workflows/pages.yml");
    let source = std::fs::read_to_string(&workflow_path).expect("read pages.yml");

    // Runner pinning: ubuntu-24.04 only, zero ubuntu-latest
    assert!(
        !source.contains("ubuntu-latest"),
        "pages.yml must not use floating runner ubuntu-latest"
    );
    assert!(
        source.contains("runs-on: ubuntu-24.04"),
        "pages.yml must pin runner to ubuntu-24.04"
    );

    // Floating tags forbidden
    assert!(
        !source.contains("@main"),
        "pages.yml must not contain floating @main action references"
    );
    assert!(
        !source.contains("@master"),
        "pages.yml must not contain floating @master action references"
    );

    // Checkout must specify complete history and zero credential persistence
    let steps: Vec<&str> = source.split("\n      - ").collect();
    for step in &steps[1..] {
        if step.contains("uses: actions/checkout@") {
            assert!(
                step.lines().any(|l| l.trim() == "fetch-depth: 0"),
                "checkout step must specify fetch-depth: 0"
            );
            assert!(
                step.lines()
                    .any(|l| l.trim() == "persist-credentials: false"),
                "checkout step must specify persist-credentials: false"
            );
        }
    }

    // All actions must be pinned to exact 40-character commit SHAs
    let mut pinned_actions = BTreeSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("uses:") {
            let ref_part = trimmed
                .strip_prefix("uses:")
                .unwrap()
                .split('#')
                .next()
                .unwrap()
                .trim();
            assert!(
                !ref_part.starts_with("./"),
                "local action references disallowed in CI: {trimmed}"
            );
            let (name, rev) = ref_part
                .rsplit_once('@')
                .unwrap_or_else(|| panic!("malformed action reference: {trimmed}"));
            assert_eq!(
                rev.len(),
                40,
                "action {name} revision must be 40-char commit SHA: {rev}"
            );
            assert!(
                rev.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
                "action {name} revision must be lowercase hex: {rev}"
            );
            pinned_actions.insert(name.to_string());
        }
    }

    assert!(pinned_actions.contains("actions/checkout"));
    assert!(pinned_actions.contains("actions/configure-pages"));
    assert!(pinned_actions.contains("actions/upload-pages-artifact"));
    assert!(pinned_actions.contains("actions/deploy-pages"));

    // Preflight gate: must execute `exec just vv` inside SDK container
    assert!(
        source.contains("exec just vv"),
        "pages.yml must run preflight gate `exec just vv` inside SDK container"
    );

    // Upload artifact step: must target public directory
    assert!(
        source.contains("path: public"),
        "upload-pages-artifact must target public directory"
    );

    // Deployment condition: must deploy only on main branch
    assert!(
        source.contains("if: github.ref == 'refs/heads/main'"),
        "deployment job must be gated to refs/heads/main"
    );
}

#[test]
fn test_browser_html_accessibility_and_csp() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // WCAG 3.1.1 Language of Page
    assert!(index_html.contains("<html lang=\"en\">"));

    // WCAG 2.4.2 Page Titled
    assert!(index_html.contains("<title>LexLean-R4 Universal Reference Engine</title>"));

    // WCAG 1.3.1 Info and Relationships: landmark
    assert!(index_html.contains("<main>") && index_html.contains("</main>"));

    // Explicit label associations
    assert!(index_html.contains("<label for=\"left\">"));
    assert!(index_html.contains("<input id=\"left\""));
    assert!(index_html.contains("<label for=\"right\">"));
    assert!(index_html.contains("<input id=\"right\""));
    assert!(index_html.contains("<label for=\"operation\">"));
    assert!(index_html.contains("<select id=\"operation\""));

    // WCAG 4.1.3 Status Messages: ARIA live region
    assert!(index_html.contains(
        "<output id=\"result\" role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></output>"
    ));

    // Strict Content Security Policy
    assert!(index_html.contains("<meta http-equiv=\"Content-Security-Policy\""));
    assert!(index_html.contains("default-src 'none'"));
    assert!(index_html.contains("script-src 'self' 'wasm-unsafe-eval'"));
    assert!(index_html.contains("style-src 'self'"));
    assert!(index_html.contains("connect-src 'self'"));
    assert!(index_html.contains("object-src 'none'"));
}

#[test]
fn test_browser_css_wcag_contrast_and_focus() {
    let root = repo_root();
    let app_css = std::fs::read_to_string(root.join("public/app.css")).expect("read app.css");

    // WCAG 2.2 SC 2.4.13 Focus Appearance (high-contrast outline)
    assert!(
        app_css.contains("outline: 3px solid #1d4ed8;"),
        "app.css must define high-contrast focus outline"
    );
    assert!(
        app_css.contains("outline-offset: 2px;"),
        "app.css must define outline-offset: 2px"
    );

    // WCAG 1.4.3 Contrast (text #111827 on white is 17.59:1)
    assert!(app_css.contains("color: #111827;"));

    // System font stack only - no web fonts loaded
    assert!(!app_css.contains("@import"));
    assert!(!app_css.contains("url("));
    assert!(app_css.contains("system-ui"));
}
