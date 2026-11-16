//! Conformance test for R4-50: Holo v4 kappa-addressed OPFS piece storage & HTTP 206 loader.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_holo_kappa_store::{
    admit_holo_kappa_piece, admit_range_slice, KappaStoreCheck, KappaStoreVerdict,
    RangeWindowCheck, StoreOperationKind, DEFAULT_HOLD_BYTES, DEFAULT_PIECE_BYTES,
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

fn valid_piece_check() -> KappaStoreCheck {
    KappaStoreCheck {
        operation: StoreOperationKind::PutVerifiedPiece,
        piece_bytes: DEFAULT_PIECE_BYTES,
        max_piece_bytes: DEFAULT_PIECE_BYTES,
        hold_bytes: DEFAULT_HOLD_BYTES,
        hold_released_before_digest: false,
        sha256_match: true,
        range_http_206: true,
        safetensors_header_valid: true,
        idb_transaction_completed: true,
        stitch_available_shards: 3,
        stitch_required_shards: 2,
    }
}

#[test]
fn test_r4_holo_kappa_store_r4_50() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/HoloKappaStore.lex.tex",
        "R4.HoloKappaStore",
        &[
            "admitRangeSlice",
            "admitHoloKappaPiece",
            "verifyValidHoloPieceAdmitted",
            "breaksUnverifiedHoldRelease",
            "breaksHashMismatchAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/HoloKappaStoreControls.lex.tex",
        "HoloKappaStoreControls",
        &[
            "validHoloPieceAdmitted",
            "breaksUnverifiedHoldRelease",
            "breaksHashMismatchAdmitted",
        ],
    );

    assert_eq!(
        admit_holo_kappa_piece(valid_piece_check()),
        KappaStoreVerdict::PieceAdmitted
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            hold_released_before_digest: true,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedHoldBufferUnderrun
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            sha256_match: false,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedPieceHashMismatch
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            range_http_206: false,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedRangeRequestUnsupported
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            safetensors_header_valid: false,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedCorruptSafetensorsHeader
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            idb_transaction_completed: false,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedStorageCommitAbort
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            stitch_available_shards: 1,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedInsufficientParityShards
    );
    assert_eq!(
        admit_holo_kappa_piece(KappaStoreCheck {
            piece_bytes: DEFAULT_PIECE_BYTES + 1,
            ..valid_piece_check()
        }),
        KappaStoreVerdict::RefusedPieceBudgetExceeded
    );
    assert_eq!(
        admit_range_slice(RangeWindowCheck {
            requested_offset: 0,
            requested_length: DEFAULT_PIECE_BYTES,
            total_file_bytes: 2 * DEFAULT_PIECE_BYTES,
            hold_tail_bytes: DEFAULT_HOLD_BYTES,
        }),
        KappaStoreVerdict::PieceAdmitted
    );
    assert_eq!(
        admit_range_slice(RangeWindowCheck {
            requested_offset: DEFAULT_PIECE_BYTES,
            requested_length: DEFAULT_PIECE_BYTES + 1,
            total_file_bytes: 2 * DEFAULT_PIECE_BYTES,
            hold_tail_bytes: DEFAULT_HOLD_BYTES,
        }),
        KappaStoreVerdict::RefusedRangeRequestUnsupported
    );

    // Runtime artifact verification (R3 fails until Step 4 completes).
    let store_rs = std::fs::read_to_string(root.join("crates/wasm/src/holo_kappa_store.rs"))
        .expect("crates/wasm/src/holo_kappa_store.rs must exist");
    assert!(store_rs.contains("UORR4V13"));
    assert!(store_rs.contains("HOLD_TAIL_BYTES"));

    let bridge_js = std::fs::read_to_string(root.join("crates/wasm/js/worker_host_bridge.js"))
        .expect("worker_host_bridge.js must exist");
    assert!(bridge_js.contains("OpfsKappaStore"));
    assert!(bridge_js.contains("createSyncAccessHandle"));
    assert!(bridge_js.contains("load_hf_repo_holo"));
}
