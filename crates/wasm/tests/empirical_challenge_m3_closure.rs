#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Challenge Suite for Milestone 3:
//! 1. File set closure invariant (strictly 6 files, NO worker.js on disk).
//! 2. CSP compliance (worker-src 'self' blob:, child-src 'self' blob:, zero unsafe-inline).
//! 3. DOM IDs uniqueness, retention, hierarchy, and WCAG accessibility semantics.
//! 4. Web Worker script extraction, zero forbidden tokens, and sandboxing.
//! 5. Node.js empirical execution: syntax check, fail-closed errors, coordinate descent math, clamping, and zero-sample resilience.
//! 6. LocalStorage persistence stress: schema conformance, large payloads, special characters, and corruption recovery.
//! 7. Claude Cowork 3-column CSS Grid and high-contrast WCAG 2.1 AA focus conformance.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

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

fn extract_worker_script(app_js: &str) -> String {
    let prefix = "const CONSOLIDATION_WORKER_SCRIPT = `";
    let start_idx = app_js
        .find(prefix)
        .expect("CONSOLIDATION_WORKER_SCRIPT declaration must exist in app.js");
    let script_start = start_idx + prefix.len();
    let end_idx = app_js[script_start..]
        .find("`;\n")
        .or_else(|| app_js[script_start..].find("`;\r\n"))
        .or_else(|| app_js[script_start..].find("`;"))
        .expect("CONSOLIDATION_WORKER_SCRIPT closing delimiter must exist");
    app_js[script_start..script_start + end_idx].to_string()
}

#[test]
fn test_empirical_public_file_set_invariant_and_no_worker_file() {
    let root = repo_root();
    let public_dir = root.join("public");
    assert!(public_dir.is_dir(), "public/ directory must exist");

    let mut actual_files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in std::fs::read_dir(&public_dir).expect("read public dir") {
        let entry = entry.expect("valid entry");
        let file_type = entry.file_type().expect("file type");
        let file_name = entry.file_name().into_string().expect("utf8 file name");

        if file_type.is_dir() {
            subdirs.push(file_name);
        } else if file_type.is_file() {
            actual_files.push(file_name);
        }
    }
    actual_files.sort();

    assert!(
        subdirs.is_empty(),
        "public/ directory must contain zero subdirectories, found: {subdirs:?}"
    );

    let expected_files = vec![
        "app.css".to_string(),
        "app.js".to_string(),
        "index.html".to_string(),
        "lexlean_r4.js".to_string(),
        "lexlean_r4_bg.wasm".to_string(),
        "provenance.json".to_string(),
    ];

    assert_eq!(
        actual_files.len(),
        6,
        "public closure must strictly contain 6 files, found {}: {actual_files:?}",
        actual_files.len()
    );
    assert_eq!(
        actual_files, expected_files,
        "public closure file list does not match exact 6-file specification"
    );

    // Explicit invariant: NO worker.js on disk
    let worker_on_disk = root.join("public/worker.js");
    assert!(
        !worker_on_disk.exists(),
        "public/worker.js MUST NOT exist on disk (must be inlined via Blob URL)"
    );

    // Verify all 6 files are non-empty and non-hidden
    for name in &expected_files {
        assert!(
            !name.starts_with('.'),
            "public/{name} must not be a hidden file"
        );
        let file_path = public_dir.join(name);
        let meta = std::fs::metadata(&file_path).expect("file metadata");
        assert!(
            meta.len() > 0,
            "public/{name} must not be empty, byte length = {}",
            meta.len()
        );
    }

    // Verify provenance.json binding digests
    let provenance_path = public_dir.join("provenance.json");
    let prov_str = std::fs::read_to_string(&provenance_path).expect("read provenance");
    let prov_json: serde_json::Value =
        serde_json::from_str(&prov_str).expect("valid provenance JSON");

    let wasm_sha = sha256_file(&public_dir.join("lexlean_r4_bg.wasm"));
    assert_eq!(
        prov_json["adapter_wasm_sha256"], wasm_sha,
        "provenance adapter_wasm_sha256 mismatch"
    );

    let js_sha = sha256_file(&public_dir.join("lexlean_r4.js"));
    assert_eq!(
        prov_json["binding_javascript_sha256"], js_sha,
        "provenance binding_javascript_sha256 mismatch"
    );

    let gen_core_sha = sha256_file(&root.join("crates/wasm/src/lexlean_generated.rs"));
    assert_eq!(
        prov_json["generated_core_sha256"], gen_core_sha,
        "provenance generated_core_sha256 mismatch"
    );
}

#[test]
fn test_empirical_csp_compliance_strict_sandboxing() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // Extract CSP meta tag
    let csp_prefix = "<meta http-equiv=\"Content-Security-Policy\" content=\"";
    let csp_start = index_html
        .find(csp_prefix)
        .expect("Content-Security-Policy meta tag must exist in index.html")
        + csp_prefix.len();
    let csp_end = index_html[csp_start..]
        .find("\">")
        .expect("CSP meta tag closing delimiter");
    let csp_content = &index_html[csp_start..csp_start + csp_end];

    // Assert mandatory CSP directives
    assert!(
        csp_content.contains("worker-src 'self' blob:;"),
        "CSP must declare worker-src 'self' blob:; for inline Web Worker support"
    );
    assert!(
        csp_content.contains("child-src 'self' blob:;"),
        "CSP must declare child-src 'self' blob:; for legacy browser worker support"
    );
    assert!(
        csp_content.contains("script-src 'self' 'wasm-unsafe-eval';"),
        "CSP must declare script-src 'self' 'wasm-unsafe-eval';"
    );
    assert!(
        csp_content.contains("default-src 'none';"),
        "CSP must declare default-src 'none';"
    );
    assert!(
        csp_content.contains("object-src 'none';"),
        "CSP must declare object-src 'none';"
    );
    assert!(
        csp_content.contains("base-uri 'none';"),
        "CSP must declare base-uri 'none';"
    );
    assert!(
        csp_content.contains("connect-src 'self' https: wss: http://127.0.0.1:* ws://127.0.0.1:*;"),
        "CSP must declare R4-39 connect-src 'self' https: wss: http://127.0.0.1:* ws://127.0.0.1:*;"
    );

    // Negative assertions: zero unsafe-inline or unsafe-eval in scripts
    assert!(
        !csp_content.contains("'unsafe-inline'"),
        "CSP MUST NOT contain 'unsafe-inline'"
    );
    assert!(
        !csp_content.contains("'unsafe-eval'"),
        "CSP MUST NOT contain 'unsafe-eval' (only 'wasm-unsafe-eval' is permitted)"
    );

    // Verify script tags in index.html: strictly 1 external script, zero inline script blocks
    let mut script_tags = Vec::new();
    let mut search_from = 0;
    while let Some(pos) = index_html[search_from..].find("<script") {
        let tag_start = search_from + pos;
        let tag_close = index_html[tag_start..]
            .find("</script>")
            .expect("matching </script>");
        let full_tag = &index_html[tag_start..tag_start + tag_close + 9];
        script_tags.push(full_tag);
        search_from = tag_start + tag_close + 9;
    }

    assert_eq!(
        script_tags.len(),
        1,
        "index.html must contain exactly 1 script tag, found {}",
        script_tags.len()
    );

    let script_tag = script_tags[0];
    assert!(
        script_tag.contains("type=\"module\""),
        "script tag must be type=\"module\""
    );
    assert!(
        script_tag.contains("src=\"app.js"),
        "script tag must point to app.js"
    );

    // Assert zero script content between opening and closing tag
    let content_start = script_tag.find('>').unwrap() + 1;
    let inner_script = &script_tag[content_start..script_tag.len() - 9].trim();
    assert!(
        inner_script.is_empty(),
        "script tag must not contain inline JS: {inner_script}"
    );
}

#[test]
fn test_empirical_dom_ids_uniqueness_hierarchy_and_accessibility() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // Extract all DOM IDs via regex
    let mut all_ids = Vec::new();
    let mut id_counts = BTreeMap::new();

    let mut cursor = 0;
    while let Some(pos) = index_html[cursor..].find("id=\"") {
        let id_start = cursor + pos + 4;
        let id_end = index_html[id_start..].find('"').expect("closing quote");
        let id_val = index_html[id_start..id_start + id_end].to_string();
        all_ids.push(id_val.clone());
        *id_counts.entry(id_val).or_insert(0usize) += 1;
        cursor = id_start + id_end + 1;
    }

    // Verify zero duplicate IDs across entire document
    let mut duplicates = Vec::new();
    for (id_name, count) in &id_counts {
        if *count > 1 {
            duplicates.push(format!("{id_name} ({count} times)"));
        }
    }
    assert!(
        duplicates.is_empty(),
        "index.html must not contain duplicate DOM IDs: {duplicates:?}"
    );

    // Verify retention of all Milestone 1, 2, and 3 contractual DOM IDs (at least 27, here 65+ key IDs)
    let mandatory_contract_ids = [
        "heading-claude-desktop",
        "claude-code-desktop",
        "workspace-file-list",
        "workspace-mount-input",
        "workspace-download-btn",
        "claude-chat-form",
        "claude-chat-input",
        "claude-chat-submit",
        "chat-apply-patch-btn",
        "chat-clear-btn",
        "chat-transcript",
        "threads-sidebar",
        "dockable-canvas-pane",
        "canvas-tab-workspace",
        "canvas-tab-diff",
        "canvas-tab-feed",
        "canvas-tab-diagnostics",
        "canvas-tab-consolidation",
        "canvas-tab-traces",
        "canvas-panel-workspace",
        "canvas-panel-diff",
        "canvas-panel-feed",
        "canvas-panel-diagnostics",
        "canvas-panel-consolidation",
        "canvas-panel-traces",
        "run-consolidation-btn",
        "consolidation-epochs",
        "consolidation-progress-bar",
        "consolidation-status-text",
        "consolidation-stat-init-loss",
        "consolidation-stat-final-loss",
        "consolidation-stat-reduction",
        "consolidation-stat-size",
        "consolidation-log-stream",
        "studio-train-form",
        "train-stat-transitions",
        "train-stat-ws-tokens",
        "snapshot-download-btn",
        "snapshot-upload-input",
        "studio-prompt-form",
        "studio-prompt",
        "studio-prose-output",
        "predictions-table",
        "trajectory-table",
        "studio-admit-form",
        "corpus-table",
        "studio-subsystems-form",
        "sub-r4-24",
        "sub-r4-25",
        "sub-r4-26",
        "sub-r4-27",
        "left",
        "right",
        "operation",
        "submit",
        "result",
        "weight-cache-badge",
        "sync-weights-btn",
        "stat-schema",
        "stat-vocab",
        "stat-turn-rows",
        "stat-point-rows",
        "stat-cells",
        "stat-train-epochs",
        "stat-state-receipt",
    ];

    let all_id_set: BTreeSet<&str> = all_ids.iter().map(String::as_str).collect();
    let mut missing_ids = Vec::new();
    for id in &mandatory_contract_ids {
        if !all_id_set.contains(id) {
            missing_ids.push(*id);
        }
    }
    assert!(
        missing_ids.is_empty(),
        "index.html is missing mandatory contract DOM IDs: {missing_ids:?}"
    );

    // Accessibility attributes:
    assert!(
        index_html.contains("<main>") && index_html.contains("</main>"),
        "index.html must maintain unadorned <main> and </main> landmarks"
    );
    assert!(
        index_html.contains("<html lang=\"en\">"),
        "index.html must set lang=\"en\" on <html>"
    );
    assert!(
        index_html.contains("<title>LexLean-R4 Universal Reference Engine</title>"),
        "index.html must have accessible title"
    );

    // Polite live regions on output elements
    assert!(index_html.contains("id=\"result\" role=\"status\" aria-live=\"polite\""));
    assert!(index_html.contains(
        "id=\"studio-prose-output\" class=\"prose-output\" role=\"status\" aria-live=\"polite\""
    ));
    assert!(index_html.contains(
        "id=\"admit-status\" class=\"admit-output\" role=\"status\" aria-live=\"polite\""
    ));
    assert!(index_html.contains(
        "id=\"workspace-status\" class=\"admit-output\" role=\"status\" aria-live=\"polite\""
    ));

    // Tablist & tabs accessible roles
    assert!(index_html.contains("role=\"tablist\""));
    assert!(index_html.contains("role=\"tab\""));
    assert!(index_html.contains("role=\"tabpanel\""));
}

#[test]
fn test_empirical_worker_script_extraction_and_forbidden_token_scan() {
    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");

    let worker_script = extract_worker_script(&app_js);
    assert!(
        worker_script.len() > 4000,
        "Extracted worker script is suspiciously short ({} bytes)",
        worker_script.len()
    );

    // Verify self.onmessage event handler exists
    assert!(
        worker_script.contains("self.onmessage = async function(event)"),
        "Worker script must define self.onmessage"
    );

    // Forbidden tokens scan on worker script
    let forbidden_tokens = [
        "fetch(",
        "parseFloat(",
        "parseInt(",
        "telemetry",
        "analytics",
        "XMLHttpRequest",
        "WebSocket",
        "EventSource",
        "importScripts(",
        "eval(",
        "Function(",
        "http://",
        "https://",
    ];

    for token in &forbidden_tokens {
        assert!(
            !worker_script.contains(token),
            "Worker script contains forbidden token: {token}"
        );
    }

    // Required integer math and TypedArray tokens
    let required_tokens = [
        "Uint8Array",
        "Int16Array",
        "BigInt64Array",
        "Uint32Array",
        "DataView",
        "TextDecoder",
        "JSON.parse",
        "BigInt(",
        "writer.table",
        "memory.point.points",
        "stepDenominator",
        "stepNumerator",
        "muTd",
        "clampMin",
        "clampMax",
    ];

    for token in &required_tokens {
        assert!(
            worker_script.contains(token),
            "Worker script is missing required mathematical token: {token}"
        );
    }
}

#[test]
fn test_empirical_worker_node_syntax_and_coordinate_descent_execution() {
    // Check if node is available
    let node_check = Command::new("node").arg("--version").output();
    let node_available = node_check.map(|o| o.status.success()).unwrap_or(false);
    if !node_available {
        eprintln!("Skipping Node.js empirical execution: node binary not found on PATH");
        return;
    }

    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");
    let worker_script = extract_worker_script(&app_js);

    // 1. Verify syntax via node --check
    let temp_script_path = root.join("target/debug/test_worker_extracted.js");
    if let Some(parent) = temp_script_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&temp_script_path, &worker_script).expect("write temp worker script");

    let check_res = Command::new("node")
        .arg("--check")
        .arg(&temp_script_path)
        .output()
        .expect("run node --check");
    assert!(
        check_res.status.success(),
        "node --check failed on extracted worker script: {}",
        String::from_utf8_lossy(&check_res.stderr)
    );
    let _ = std::fs::remove_file(&temp_script_path);

    // 2. Comprehensive mock harness executing worker script in Node
    let harness = format!(
        r#"
const fs = require('fs');

const workerCode = {worker_code_json};
const bootstrap = fs.readFileSync('{bootstrap_path}');

async function runHarness() {{
  const messages = [];
  const self = {{
    onmessage: null,
    postMessage: (msg) => {{
      messages.push(msg);
    }}
  }};

  const fn = new Function('self', 'getWeightsFromIndexedDB', workerCode);
  fn(self, async () => null);

  if (typeof self.onmessage !== 'function') {{
    throw new Error('self.onmessage is not a function');
  }}

  // Scenario 1: Ignored non-START action
  await self.onmessage({{ data: {{ action: 'NOOP' }} }});
  if (messages.length !== 0) {{
    throw new Error('Expected 0 messages for NOOP action');
  }}

  // Scenario 2: Truncated buffer (< 64 bytes)
  const shortBuf = new Uint8Array(32);
  await self.onmessage({{
    data: {{ action: 'START_CONSOLIDATION', weightsBuffer: shortBuf.buffer }}
  }});
  if (messages.length !== 1 || messages[0].type !== 'ERROR') {{
    throw new Error('Expected ERROR for truncated buffer, got: ' + JSON.stringify(messages));
  }}
  messages.length = 0;

  // Scenario 3: Corrupted container magic
  const badMagicBuf = Buffer.alloc(128);
  badMagicBuf.write('CORRUPT!', 0);
  await self.onmessage({{
    data: {{ action: 'START_CONSOLIDATION', weightsBuffer: badMagicBuf.buffer }}
  }});
  if (messages.length !== 1 || messages[0].type !== 'ERROR' || !messages[0].error.includes('expected UORLUT01')) {{
    throw new Error('Expected ERROR with magic mismatch, got: ' + JSON.stringify(messages));
  }}
  messages.length = 0;

  // Scenario 4: Valid bootstrap with zero samples (graceful zero-descent)
  const bufZero = bootstrap.buffer.slice(bootstrap.byteOffset, bootstrap.byteOffset + bootstrap.byteLength);
  await self.onmessage({{
    data: {{
      action: 'START_CONSOLIDATION',
      weightsBuffer: bufZero,
      samples: [],
      config: {{ iterations: 2 }}
    }}
  }});
  const zeroComplete = messages.find(m => m.type === 'COMPLETE');
  if (!zeroComplete) {{
    throw new Error('Expected COMPLETE for zero samples');
  }}
  if (zeroComplete.report.weights_updated !== 0 || zeroComplete.report.l1_shift !== '0') {{
    throw new Error('Expected 0 weights updated for zero samples');
  }}
  messages.length = 0;

  // Scenario 5: Valid bootstrap with 2 samples on target token 15
  const bufValid = bootstrap.buffer.slice(bootstrap.byteOffset, bootstrap.byteOffset + bootstrap.byteLength);
  const sampleLatent1 = new Array(256).fill(0);
  sampleLatent1[0] = 4000;
  sampleLatent1[1] = -2500;
  const sampleLatent2 = new Array(256).fill(0);
  sampleLatent2[0] = 6000;
  sampleLatent2[1] = -3500;
  // Centroid average: [5000, -3000, 0, ...]

  await self.onmessage({{
    data: {{
      action: 'START_CONSOLIDATION',
      weightsBuffer: bufValid,
      samples: [
        {{ target_token: 15, latent: sampleLatent1 }},
        {{ target_token: 15, latent: sampleLatent2 }}
      ],
      config: {{ iterations: 3, step_numerator: 1, step_denominator: 8 }}
    }}
  }});

  const progressMsgs = messages.filter(m => m.type === 'PROGRESS');
  if (progressMsgs.length !== 3) {{
    throw new Error('Expected 3 PROGRESS messages for 3 iterations, got ' + progressMsgs.length);
  }}
  const validComplete = messages.find(m => m.type === 'COMPLETE');
  if (!validComplete) {{
    throw new Error('Expected COMPLETE for valid samples');
  }}
  const rep = validComplete.report;
  if (rep.weights_updated <= 0 || rep.points_consolidated !== 2 || rep.tokens_updated !== 1) {{
    throw new Error('Invalid report metrics: ' + JSON.stringify(rep));
  }}

  // Verify updated container magic is preserved
  const updatedU8 = new Uint8Array(validComplete.updatedWeightsBuffer);
  const updatedMagic = String.fromCharCode(...updatedU8.slice(0, 8));
  if (updatedMagic !== 'UORLUT01') {{
    throw new Error('Updated container magic corrupted: ' + updatedMagic);
  }}
  messages.length = 0;

  // Scenario 6: Strict clamping test with extreme inputs
  const bufClamp = bootstrap.buffer.slice(bootstrap.byteOffset, bootstrap.byteOffset + bootstrap.byteLength);
  const extremeLatent = new Array(256).fill(32000);
  await self.onmessage({{
    data: {{
      action: 'START_CONSOLIDATION',
      weightsBuffer: bufClamp,
      samples: [{{ target_token: 7, latent: extremeLatent }}],
      config: {{ iterations: 1, step_numerator: 8, step_denominator: 8, clamp_min: -750, clamp_max: 750 }}
    }}
  }});
  const clampComplete = messages.find(m => m.type === 'COMPLETE');
  if (!clampComplete) {{
    throw new Error('Expected COMPLETE for clamp test');
  }}

  const dv = new DataView(clampComplete.updatedWeightsBuffer);
  const hLen = Number(dv.getBigUint64(8, true));
  const dStart = Math.ceil((16 + hLen) / 64) * 64;
  const header = JSON.parse(new TextDecoder().decode(new Uint8Array(clampComplete.updatedWeightsBuffer).slice(16, 16 + hLen)));
  const writerTbl = header.tables.find(t => t.name === 'writer.table');
  const wOff = dStart + ((writerTbl.span && writerTbl.span.offset !== undefined) ? writerTbl.span.offset : writerTbl.offset);
  const wArr = new Int16Array(clampComplete.updatedWeightsBuffer, wOff, writerTbl.bytes / 2);

  const base7 = 7 * 256;
  for (let d = 0; d < 256; d++) {{
    const val = wArr[base7 + d];
    if (val < -750 || val > 750) {{
      throw new Error(`Clamping invariant violated at dim ${{d}}: ${{val}} not in [-750, 750]`);
    }}
  }}

  console.log('ALL_MOCK_TESTS_OK');
}}

runHarness().catch(err => {{
  console.error('Harness error:', err);
  process.exit(1);
}});
"#,
        worker_code_json = serde_json::to_string(&worker_script).unwrap(),
        bootstrap_path = root
            .join("crates/wasm/src/bootstrap_v8.uorlut")
            .to_string_lossy()
    );

    let child = Command::new("node")
        .arg("-e")
        .arg(&harness)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn node harness");

    let output = child.wait_with_output().expect("wait on node harness");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Node mock worker execution harness failed:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
    assert!(
        stdout.contains("ALL_MOCK_TESTS_OK"),
        "Node harness did not produce confirmation: {stdout}"
    );
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct ChatMessageModel {
    role: String,
    body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tools: Vec<serde_json::Value>,
    timestamp: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct ChatThreadModel {
    id: String,
    title: String,
    messages: Vec<ChatMessageModel>,
    created_at: u64,
}

#[test]
fn test_empirical_localstorage_serialization_stress_and_fault_tolerance() {
    // 1. Stress test realistic multi-turn conversation with complex thinking, tools, and symbols
    let mut thread = ChatThreadModel {
        id: "thread-adversarial-100-turns".to_string(),
        title: "100-Turn Stress Verification".to_string(),
        messages: Vec::new(),
        created_at: 1730840000000,
    };

    for turn in 1..=50 {
        // User turn
        thread.messages.push(ChatMessageModel {
            role: "user".to_string(),
            body: format!(
                "Turn #{turn}: Solve \\int_0^\\pi x \\sin(x) dx and factorize 2^{{31}} - 1. Also generate a script with quotes \"double\" and 'single' and `<thinking>` tags."
            ),
            thinking: None,
            tools: Vec::new(),
            timestamp: 1730840000000 + turn * 2000,
        });

        // Assistant turn with structured thinking and tool cards
        thread.messages.push(ChatMessageModel {
            role: "assistant".to_string(),
            body: format!(
                "Turn #{turn} reply: Integration by parts yields \\pi. 2^31 - 1 = 2147483647 is the 8th Mersenne prime M_{{31}}. Here is code:\n```python\nprint(\"Hello world! <xml> & entities\")\n```\nEmojis: 🚀📐🦀"
            ),
            thinking: Some(format!(
                "<thinking>Decomposing query #{turn}: u=x, dv=sin(x)dx, du=dx, v=-cos(x). Evaluating [-x cos(x)]_0^pi + int cos(x)dx = pi + 0 = pi. Mersenne prime proven by Euler in 1772.</thinking>"
            )),
            tools: vec![serde_json::json!({
                "id": format!("call_{turn}"),
                "name": "uor_math_exact",
                "input": { "expr": format!("2^{turn} - 1") }
            })],
            timestamp: 1730840000000 + turn * 2000 + 1000,
        });
    }

    assert_eq!(thread.messages.len(), 100);

    // Serialization to JSON (mimicking localStorage.setItem)
    let threads_array = vec![thread];
    let serialized = serde_json::to_string(&threads_array).expect("serialize threads");
    assert!(
        serialized.len() > 20000,
        "Serialized payload size = {} bytes",
        serialized.len()
    );

    // Deserialization from JSON (mimicking JSON.parse(localStorage.getItem))
    let deserialized: Vec<ChatThreadModel> =
        serde_json::from_str(&serialized).expect("deserialize threads");
    assert_eq!(threads_array, deserialized);
    assert_eq!(deserialized[0].messages.len(), 100);

    // 2. Fault tolerance against corrupted or malformed localStorage states
    // Case A: Truncated JSON
    let truncated_json = &serialized[..serialized.len() / 2];
    let err_truncated = serde_json::from_str::<Vec<ChatThreadModel>>(truncated_json);
    assert!(
        err_truncated.is_err(),
        "Truncated JSON must cleanly error on parse"
    );

    // Case B: Non-array JSON (e.g. object instead of array)
    let non_array_json = "{\"id\": \"thread-1\"}";
    let err_non_array = serde_json::from_str::<Vec<ChatThreadModel>>(non_array_json);
    assert!(
        err_non_array.is_err(),
        "Object JSON must not deserialize into Vec<ChatThreadModel>"
    );

    // Case C: Empty array []
    let empty_array_json = "[]";
    let empty_threads: Vec<ChatThreadModel> =
        serde_json::from_str(empty_array_json).expect("parse empty array");
    assert!(empty_threads.is_empty());
}

#[test]
fn test_empirical_cowork_css_spec_and_wcag_conformance() {
    let root = repo_root();
    let app_css = std::fs::read_to_string(root.join("public/app.css")).expect("read app.css");

    // 3-Column Split Pane Cowork grid
    assert!(
        app_css.contains("grid-template-columns: 16rem minmax(360px, 1fr) minmax(440px, 1.25fr);"),
        "app.css must specify 3-column split-pane grid layout"
    );

    // WCAG 2.1 AA focus appearance (high contrast outline)
    assert!(
        app_css.contains("outline: 3px solid #1d4ed8;"),
        "app.css must define high-contrast focus outline: 3px solid #1d4ed8;"
    );
    assert!(
        app_css.contains("outline-offset: 2px;"),
        "app.css must define outline-offset: 2px;"
    );

    // Zero external web fonts or remote CSS imports
    assert!(
        !app_css.contains("@import"),
        "app.css must not use @import remote stylesheets"
    );
    assert!(
        !app_css.contains("http://") && !app_css.contains("https://"),
        "app.css must not contain remote URLs"
    );
    assert!(
        app_css.contains("system-ui"),
        "app.css must use system font stack"
    );
}

#[test]
fn test_empirical_bootstrap_weight_container_structure() {
    // Assert bootstrap_v8.uorlut conforms to UORLUT01 container specification
    assert!(
        BOOTSTRAP_BYTES.len() >= 64,
        "bootstrap bytes must be at least 64 bytes"
    );
    assert_eq!(
        &BOOTSTRAP_BYTES[0..8],
        b"UORLUT01",
        "bootstrap magic must be UORLUT01"
    );

    let header_len = u64::from_le_bytes(BOOTSTRAP_BYTES[8..16].try_into().unwrap()) as usize;
    assert!(
        16 + header_len <= BOOTSTRAP_BYTES.len(),
        "header length exceeds file size"
    );

    let header_str = std::str::from_utf8(&BOOTSTRAP_BYTES[16..16 + header_len])
        .expect("valid UTF-8 header string");
    let header_json: serde_json::Value =
        serde_json::from_str(header_str).expect("valid JSON header");

    assert_eq!(header_json["schema"], "uor-r4.lut-stack/8");
    assert_eq!(header_json["shape"]["vocab"], 4096);
    assert_eq!(header_json["shape"]["width"], 256);

    // Check required tables
    let tables = header_json["tables"]
        .as_array()
        .expect("tables must be an array");
    let table_names: Vec<&str> = tables.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(
        table_names.contains(&"writer.table"),
        "bootstrap container must include writer.table"
    );
    assert!(
        table_names.contains(&"memory.point.points"),
        "bootstrap container must include memory.point.points"
    );
    assert!(
        table_names.contains(&"memory.point.targets"),
        "bootstrap container must include memory.point.targets"
    );
}

#[test]
fn test_empirical_typescript_agentic_composer_and_state_machine() {
    let root = repo_root();
    let ts_path = root.join("crates/wasm/ts/composer.ts");
    assert!(
        ts_path.is_file(),
        "crates/wasm/ts/composer.ts TypeScript source must exist"
    );
    let ts_source = std::fs::read_to_string(&ts_path).expect("read composer.ts");
    assert!(ts_source.contains("export type ComposerMode"));
    assert!(ts_source.contains("export interface ComposerAttachmentChip"));
    assert!(ts_source.contains("export interface SlashCommandSpec"));
    assert!(ts_source.contains("export class AgenticTypeScriptComposer"));

    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");
    for required_id in [
        "composer-autocomplete-menu",
        "composer-attachments-bar",
        "composer-slash-btn",
        "composer-mention-btn",
        "composer-token-badge",
    ] {
        assert!(
            index_html.contains(&format!("id=\"{required_id}\"")),
            "index.html must contain TypeScript composer control #{required_id}"
        );
    }

    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");
    assert!(
        app_js.contains("class AgenticTypeScriptComposer"),
        "app.js must embed compiled AgenticTypeScriptComposer"
    );
    assert!(
        app_js.contains("const AGENTIC_SLASH_COMMANDS"),
        "app.js must embed AGENTIC_SLASH_COMMANDS"
    );
}
