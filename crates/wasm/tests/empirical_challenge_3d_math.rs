//! Empirical challenge and stress tests for Milestone 2 (3D WebGL Math & Camera Controls - R1).
//!
//! Validates:
//! 1. Execution of the dynamic empirical stress harness via Node.js (`tests/e2e/empirical_3d_stress_harness.mjs`),
//!    exercising matrix math, zoom/orbit boundary conditions, and dynamic trace mutation.
//! 2. Static verification of 3D WebGL math functions (`mat4_perspective`, `mat4_lookAt`, `mat4_multiply`) in `public/app.js`.
//! 3. Camera zoom clamping ([2.0, 50.0]) and pitch clamping ([-1.45, 1.45]) in `public/app.js`.
//! 4. 3D WebGL DOM elements and strict ID uniqueness in `public/index.html`.
//! 5. 3D WebGL CSS styling and viewport constraints in `public/app.css`.

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
fn test_empirical_3d_node_stress_harness_execution() {
    let root = repo_root();
    let harness_path = root.join("tests/e2e/empirical_3d_stress_harness.mjs");
    assert!(
        harness_path.is_file(),
        "empirical 3D stress harness script must exist at {}",
        harness_path.display()
    );

    let output = Command::new("node")
        .arg(&harness_path)
        .current_dir(&root)
        .output()
        .expect("execute node empirical 3D stress harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "Node 3D stress harness failed with exit code {:?}.\nSTDOUT:\n{}\nSTDERR:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    assert!(
        stdout.contains("Suite 1: Matrix Math & Determinant Preservation"),
        "Missing Suite 1 execution output"
    );
    assert!(
        stdout.contains("Suite 2: Zoom Boundary Conditions & Clamping"),
        "Missing Suite 2 execution output"
    );
    assert!(
        stdout.contains("Suite 3: Orbit Boundary Conditions & Gimbal Lock Prevention"),
        "Missing Suite 3 execution output"
    );
    assert!(
        stdout.contains("Suite 4: Dynamic Trace Mutation Stress Test"),
        "Missing Suite 4 execution output"
    );
    assert!(
        stdout.contains("ALL EMPIRICAL 3D WEBGL MATH & CAMERA CONTROL SUITES PASSED"),
        "Missing completion banner"
    );
}

#[test]
fn test_3d_math_and_camera_clamping_in_app_js() {
    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).expect("read app.js");

    // Matrix math functions
    assert!(
        app_js.contains("function mat4_create()"),
        "app.js must declare mat4_create"
    );
    assert!(
        app_js.contains("function mat4_perspective("),
        "app.js must declare mat4_perspective"
    );
    assert!(
        app_js.contains("function mat4_lookAt("),
        "app.js must declare mat4_lookAt"
    );
    assert!(
        app_js.contains("function mat4_multiply("),
        "app.js must declare mat4_multiply"
    );

    // Zoom clamping between 2.0 and 50.0
    assert!(
        app_js.contains("Math.min(Math.max(this.distance * zoomFactor, 2.0), 50.0)"),
        "app.js must strictly clamp camera zoom distance between 2.0 and 50.0"
    );

    // Pitch clamping between -1.45 and 1.45
    assert!(
        app_js.contains("Math.min(Math.max(this.pitch + dy * 0.007, -1.45), 1.45)"),
        "app.js must strictly clamp camera pitch angle between -1.45 and 1.45"
    );

    // Spline interpolation
    assert!(
        app_js.contains("function catmullRomPoint("),
        "app.js must declare catmullRomPoint"
    );
    assert!(
        app_js.contains("function catmullRom1D("),
        "app.js must declare catmullRom1D"
    );

    // Geometry structures
    assert!(
        app_js.contains("this.cliffordNodes = [];"),
        "LexLean3DVisualizer must construct Clifford nodes"
    );
    assert!(
        app_js.contains("this.icosianRoots = [];"),
        "LexLean3DVisualizer must construct Icosian roots"
    );
    assert!(
        app_js.contains(
            "this.layerElevations = new Float32Array([-2.5, -1.5, -0.5, 0.5, 1.5, 2.5]);"
        ),
        "LexLean3DVisualizer must construct 6 layer elevations"
    );

    // Zero float narrowing / fetch restrictions
    assert!(
        !app_js.contains("fetch("),
        "app.js must not contain fetch()"
    );
    assert!(
        !app_js.contains("parseFloat("),
        "app.js must not contain parseFloat()"
    );
    assert!(
        !app_js.contains("parseInt("),
        "app.js must not contain parseInt()"
    );
}

#[test]
fn test_3d_viewport_dom_elements_and_unique_ids() {
    let root = repo_root();
    let index_html =
        std::fs::read_to_string(root.join("public/index.html")).expect("read index.html");

    // Required 3D canvas DOM elements
    assert!(
        index_html.contains("id=\"neural-3d-viewport\""),
        "index.html must contain #neural-3d-viewport"
    );
    assert!(
        index_html.contains("id=\"neural-3d-canvas\""),
        "index.html must contain #neural-3d-canvas"
    );
    assert!(
        index_html.contains("id=\"neural-3d-reset-btn\""),
        "index.html must contain #neural-3d-reset-btn"
    );
    assert!(
        index_html.contains("id=\"neural-3d-anim-toggle\""),
        "index.html must contain #neural-3d-anim-toggle"
    );
    assert!(
        index_html.contains("id=\"neural-3d-status\""),
        "index.html must contain #neural-3d-status"
    );

    // Verify all document-wide IDs are strictly unique
    let mut seen_ids = HashSet::new();
    let mut duplicate_ids = Vec::new();

    for part in index_html.split("id=\"").skip(1) {
        if let Some(id) = part.split('"').next() {
            if !seen_ids.insert(id.to_string()) {
                duplicate_ids.push(id.to_string());
            }
        }
    }

    assert!(
        duplicate_ids.is_empty(),
        "Duplicate IDs found in index.html: {:?}",
        duplicate_ids
    );
    assert!(
        seen_ids.len() >= 65,
        "Expected at least 65 contractual IDs, found {}",
        seen_ids.len()
    );
}

#[test]
fn test_3d_css_styling_and_viewport_rules() {
    let root = repo_root();
    let app_css = std::fs::read_to_string(root.join("public/app.css")).expect("read app.css");

    assert!(
        app_css.contains("#neural-3d-viewport"),
        "app.css must define styles for #neural-3d-viewport"
    );
    assert!(
        app_css.contains(".neural-3d-canvas"),
        "app.css must define styles for .neural-3d-canvas"
    );
    assert!(
        app_css.contains("min-height: 250px"),
        "app.css must enforce minimum height on 3D canvas"
    );
}

#[test]
fn test_3d_offline_closure_exact_six_files_and_digests() {
    let root = repo_root();
    let public_dir = root.join("public");

    let mut actual_files = Vec::new();
    for entry in std::fs::read_dir(&public_dir).expect("read public dir") {
        let entry = entry.expect("valid entry");
        assert!(entry.file_type().expect("file type").is_file());
        actual_files.push(entry.file_name().into_string().unwrap());
    }
    actual_files.sort();

    let expected = vec![
        "app.css".to_string(),
        "app.js".to_string(),
        "index.html".to_string(),
        "lexlean_r4.js".to_string(),
        "lexlean_r4_bg.wasm".to_string(),
        "provenance.json".to_string(),
    ];
    assert_eq!(
        actual_files, expected,
        "Public closure must be exactly 6 files"
    );

    // Verify SHA-256 bindings in provenance.json
    let prov_str = std::fs::read_to_string(public_dir.join("provenance.json")).unwrap();
    let prov: serde_json::Value = serde_json::from_str(&prov_str).unwrap();

    let actual_wasm_sha = sha256_file(&public_dir.join("lexlean_r4_bg.wasm"));
    assert_eq!(
        prov["adapter_wasm_sha256"], actual_wasm_sha,
        "Wasm hash mismatch in provenance"
    );
}
