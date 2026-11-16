//! Conformance test for R4-54: Stitch Prism Identity, Hologram v4 .holo compiler, and afflom/me app packaging.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;
#[path = "../../wasm/src/prism_holo_compiler.rs"]
#[allow(dead_code)]
mod prism_holo_compiler;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use lexlean_generated::r4_prism_holo_identity::{
    admit_holo_v4_archive, admit_prism_identity_record, role_permits_record,
    verify_holo_archive_admitted, verify_prism_identity_admitted, HoloArchiveCheck,
    PrismHoloVerdict, PrismIdentityCheck, PrismIdentityRole, PrismRecordKind,
};
use prism_holo_compiler::{
    blake3_hash_hex, canonicalize_json_value, compile_holo_v4_archive, decode_holoview_v1,
    encode_holoview_v1, uor_kappa_sha256, verify_holo_v4_archive, PrismIdentityLedger,
};
use serde_json::{json, Value};

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

fn valid_identity_check() -> PrismIdentityCheck {
    PrismIdentityCheck {
        canonical_integers_only: true,
        kappa_matches_body: true,
        signer_revoked: false,
        signer_role: PrismIdentityRole::Member,
        record_kind: PrismRecordKind::Action1,
    }
}

fn valid_holo_check() -> HoloArchiveCheck {
    HoloArchiveCheck {
        magic_valid: true,
        version: 4,
        section_count: 8,
        section_order_valid: true,
        manifest_bytes: 356,
        guest_imports: 0,
        guest_exports_valid: true,
        blobs_and_view_sorted: true,
        blake3_footer_valid: true,
        vectors_passed: 4,
        vectors_total: 4,
    }
}

#[test]
fn test_r4_prism_holo_identity_r4_54() {
    let root = repo_root();

    check_lex_module(
        &root,
        "src/R4/PrismHoloIdentity.lex.tex",
        "R4.PrismHoloIdentity",
        &[
            "rolePermitsRecord",
            "admitPrismIdentityRecord",
            "admitHoloV4Archive",
            "verifyPrismIdentityAdmitted",
            "verifyHoloArchiveAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/PrismHoloIdentityControls.lex.tex",
        "PrismHoloIdentityControls",
        &[
            "prismIdentityAdmitted",
            "holoArchiveAdmitted",
            "breaksRecoverySignedAction",
            "breaksRevokedSigner",
            "breaksWrongHoloVersion",
            "breaksGuestWithImports",
            "breaksCorruptBlake3Footer",
        ],
    );

    assert!(verify_prism_identity_admitted());
    assert!(verify_holo_archive_admitted());
    assert!(role_permits_record(
        PrismIdentityRole::Member,
        PrismRecordKind::Action1
    ));
    assert!(role_permits_record(
        PrismIdentityRole::Delegate,
        PrismRecordKind::Action1
    ));
    assert!(!role_permits_record(
        PrismIdentityRole::Recovery,
        PrismRecordKind::Action1
    ));
    assert!(role_permits_record(
        PrismIdentityRole::Recovery,
        PrismRecordKind::Bind1
    ));

    assert_eq!(
        admit_prism_identity_record(valid_identity_check()),
        PrismHoloVerdict::PrismHoloAdmitted
    );
    assert_eq!(
        admit_prism_identity_record(PrismIdentityCheck {
            canonical_integers_only: false,
            ..valid_identity_check()
        }),
        PrismHoloVerdict::RefusedNonIntegerCanonicalJson
    );
    assert_eq!(
        admit_prism_identity_record(PrismIdentityCheck {
            kappa_matches_body: false,
            ..valid_identity_check()
        }),
        PrismHoloVerdict::RefusedIdentityKappaMismatch
    );
    assert_eq!(
        admit_prism_identity_record(PrismIdentityCheck {
            signer_revoked: true,
            ..valid_identity_check()
        }),
        PrismHoloVerdict::RefusedRevokedDeviceSigner
    );
    assert_eq!(
        admit_prism_identity_record(PrismIdentityCheck {
            signer_role: PrismIdentityRole::Recovery,
            record_kind: PrismRecordKind::Action1,
            ..valid_identity_check()
        }),
        PrismHoloVerdict::RefusedUnauthorizedIdentityRole
    );

    assert_eq!(
        admit_holo_v4_archive(valid_holo_check()),
        PrismHoloVerdict::PrismHoloAdmitted
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            version: 3,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedInvalidHoloMagicOrVersion
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            section_order_valid: false,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedInvalidHoloSectionOrder
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            guest_imports: 1,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedCoreWasmContractViolation
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            blobs_and_view_sorted: false,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedUnsortedHoloBlobsOrView
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            blake3_footer_valid: false,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedHoloBlake3FooterMismatch
    );
    assert_eq!(
        admit_holo_v4_archive(HoloArchiveCheck {
            vectors_passed: 3,
            ..valid_holo_check()
        }),
        PrismHoloVerdict::RefusedMeAppVectorReplayFailure
    );

    // 1. Canonical JSON & BLAKE3 unkeyed vector parity
    assert_eq!(
        blake3_hash_hex(b""),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
    assert_eq!(
        canonicalize_json_value(&json!({"b": 2, "a": 1})).unwrap(),
        r#"{"a":1,"b":2}"#
    );
    assert_eq!(
        canonicalize_json_value(&json!({"bad": 1.25})).unwrap_err(),
        PrismHoloVerdict::RefusedNonIntegerCanonicalJson
    );

    // 2. Stitch.PrismId ledger lifecycle + role/revocation refusals
    let mut ledger = PrismIdentityLedger::default();
    let dev_member = json!({
        "schema": "prism/device/1",
        "class": "browser",
        "protection": "software",
        "signing": {"kty": "EC", "crv": "P-256", "x": "mem_x", "y": "mem_y"},
        "agreement": {"kty": "EC", "crv": "P-256", "x": "mem_ax", "y": "mem_ay"}
    });
    let member_id = uor_kappa_sha256(&dev_member).unwrap();
    let genesis = json!({
        "schema": "prism/genesis/1",
        "device": dev_member,
        "device_id": member_id,
        "nonce": 1
    });
    let genesis_res = ledger.admit_record(&genesis).unwrap();
    assert_eq!(genesis_res.verdict, PrismHoloVerdict::PrismHoloAdmitted);

    let dev_recovery = json!({
        "schema": "prism/device/1",
        "class": "browser",
        "protection": "software",
        "signing": {"kty": "EC", "crv": "P-256", "x": "rec_x", "y": "rec_y"},
        "agreement": {"kty": "EC", "crv": "P-256", "x": "rec_ax", "y": "rec_ay"}
    });
    let rec_id = uor_kappa_sha256(&dev_recovery).unwrap();
    let bind_rec = json!({
        "schema": "prism/bind/1",
        "prism": genesis_res.prism_id,
        "prev": genesis_res.record_kappa,
        "by": member_id,
        "role": "recovery",
        "device": dev_recovery
    });
    ledger.admit_record(&bind_rec).unwrap();

    let rec_action = json!({
        "schema": "prism/action/1",
        "prism": genesis_res.prism_id,
        "prev": ledger.head_kappa(),
        "by": rec_id,
        "action": "compile-holo",
        "target_kappa": "blake3:0000000000000000000000000000000000000000000000000000000000000000"
    });
    assert_eq!(
        ledger.admit_record(&rec_action).unwrap_err(),
        PrismHoloVerdict::RefusedUnauthorizedIdentityRole
    );
    assert_eq!(ledger.status_json()["prism_id"], genesis_res.prism_id);

    // 3. Hologram v4 .holo compile & verify + planted corrupt footer refusal
    let mut files = BTreeMap::new();
    files.insert("app.css".to_string(), b"body{color:#111827;}".to_vec());
    files.insert("app.js".to_string(), b"export const ok=1;".to_vec());
    files.insert(
        "index.html".to_string(),
        b"<!doctype html><title>test</title>".to_vec(),
    );
    files.insert(
        "lexlean_r4.js".to_string(),
        b"export function initSync(){}".to_vec(),
    );
    files.insert(
        "lexlean_r4_bg.wasm".to_string(),
        b"\0asm\x01\0\0\0".to_vec(),
    );
    files.insert("provenance.json".to_string(), b"{}".to_vec());

    let holoview = encode_holoview_v1("index.html", &files).unwrap();
    let decoded = decode_holoview_v1(&holoview).unwrap();
    assert_eq!(decoded.entry, "index.html");
    assert_eq!(decoded.files.len(), 6);

    let archive = compile_holo_v4_archive(&files, b"{\"schema\":\"prismpm/browser-provenance/1\"}")
        .expect("compile valid .holo v4");
    let verified = verify_holo_v4_archive(&archive).expect("verify compiled .holo v4");
    assert_eq!(verified.verdict, PrismHoloVerdict::PrismHoloAdmitted);
    assert!(verified.archive_kappa.starts_with("blake3:"));
    assert_eq!(verified.vectors_passed, verified.vectors_total);
    assert!(verified.vectors_total >= 5);

    let mut tampered = archive.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xff;
    assert_eq!(
        verify_holo_v4_archive(&tampered).unwrap_err(),
        PrismHoloVerdict::RefusedHoloBlake3FooterMismatch
    );

    // 4. UOR Foundry + afflom/me selectable site/app.json manifest check
    let app_json =
        std::fs::read_to_string(root.join("site/app.json")).expect("site/app.json must exist");
    let app_val: Value = serde_json::from_str(&app_json).expect("valid site/app.json");
    assert_eq!(app_val["address"], "lexlean-r4");
    assert_eq!(app_val["holoAddress"], "holo://lexlean-r4");
    assert_eq!(
        app_val["foundry"]["repository"],
        "https://github.com/UOR-Foundation/foundry"
    );
    assert_eq!(app_val["foundry"]["inprismRequest"]["mode"], "inprism");
    assert_eq!(app_val["foundry"]["inprismRequest"]["outcome"], "answer");
}
