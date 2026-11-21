//! Conformance test for R4-80: Qwen3-Distilled 64-Basin `UORR4V15` Sparse Geometric
//! Wave Engine (`<= 150 ms` instant mount, `>= 10,000 tok/s`, multi-hop `<tool_call>` awareness).

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_qwen3_sparse_basin_v15::{
    admit_sparse_basin_v15_wave, verify_qwen3_sparse_basin_v15_admitted, SparseBasinV15Verdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

#[test]
fn test_r4_80_qwen3_sparse_basin_v15_r4_80() {
    let root = repo_root();

    // 1. Formal LexLean R4-80 admission and R5 negative refusals
    assert!(verify_qwen3_sparse_basin_v15_admitted());
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 65_536, 8, 12, 250_000, 0),
        SparseBasinV15Verdict::SparseBasinV15WaveAdmitted
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(1, 0, 0, 65_536, 8, 12, 250_000, 0),
        SparseBasinV15Verdict::RefusedFloatOrDenseMatmulInV15
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 524_289, 8, 12, 250_000, 0),
        SparseBasinV15Verdict::RefusedBasinExceeds512KiB
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 65_536, 4, 12, 250_000, 0),
        SparseBasinV15Verdict::RefusedBasinExceeds512KiB
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 65_536, 8, 151, 250_000, 0),
        SparseBasinV15Verdict::RefusedBootExceeds150Ms
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 65_536, 8, 12, 9_999, 0),
        SparseBasinV15Verdict::RefusedThroughputBelow10kTps
    );
    assert_eq!(
        admit_sparse_basin_v15_wave(0, 0, 0, 65_536, 8, 12, 250_000, 1),
        SparseBasinV15Verdict::RefusedNonZeroIcosianDrift
    );

    // 2. Verify .lex.tex semantic modules exist and declare required symbols
    check_lex_module(
        &root,
        "src/R4/Qwen3SparseBasinV15.lex.tex",
        "R4.Qwen3SparseBasinV15",
        &[
            "SparseBasinV15Verdict",
            "admitSparseBasinV15Wave",
            "verifyQwen3SparseBasinV15Admitted",
            "breaksV15FloatOrMatmulAdmitted",
            "breaksV15BasinOverflowAdmitted",
            "breaksV15SlowBootAdmitted",
            "breaksV15Sub10kThroughputAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/Qwen3SparseBasinV15Controls.lex.tex",
        "Qwen3SparseBasinV15Controls",
        &[
            "qwen3SparseBasinV15Admitted",
            "breaksV15FloatOrMatmulAdmittedCtrl",
            "breaksV15BasinOverflowAdmittedCtrl",
            "breaksV15SlowBootAdmittedCtrl",
            "breaksV15Sub10kThroughputAdmittedCtrl",
        ],
    );

    // 3. Verify crates/wasm/src/sparse_basin_v15.rs and Wasm exports in lib.rs exist
    let v15_path = root.join("crates/wasm/src/sparse_basin_v15.rs");
    assert!(
        v15_path.is_file(),
        "crates/wasm/src/sparse_basin_v15.rs must exist"
    );
    let v15_src = std::fs::read_to_string(&v15_path).expect("read sparse_basin_v15.rs");
    assert!(
        v15_src.contains("UORR4V15")
            && v15_src.contains("SparseBasinEngineV15")
            && v15_src.contains("execute_agentic_wave_turn")
            && v15_src.contains("admit_sparse_basin_v15_wave"),
        "sparse_basin_v15.rs must implement UORR4V15 SparseBasinEngineV15::execute_agentic_wave_turn gated by admit_sparse_basin_v15_wave"
    );

    let lib_rs = std::fs::read_to_string(root.join("crates/wasm/src/lib.rs")).expect("read lib.rs");
    assert!(
        lib_rs.contains("pub fn studio_sparse_v15_turn_json(")
            && lib_rs.contains("pub fn studio_sparse_v15_status_json("),
        "crates/wasm/src/lib.rs must export studio_sparse_v15_turn_json and studio_sparse_v15_status_json"
    );
}
