//! Conformance test for R4-42: Stitch 2-of-3 XOR parity custody & SHA-256 verification.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;
#[path = "support/sha256.rs"]
mod sha256;

use lexlean_generated::r4_stitch_fabric::{
    admit_stitch_manifest, admit_stitch_parity_recovery, is_stitch_recovery_admitted,
    StitchManifestCheck, StitchParityVerdict, StitchRecoveryCheck, StitchRecoveryVerdict,
};
use serde_json::Value;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn xor_pair(a: &[u8], b: &[u8]) -> Vec<u8> {
    let n = a.len().max(b.len());
    let mut out = vec![0u8; n];
    for (i, slot) in out.iter_mut().enumerate() {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        *slot = av ^ bv;
    }
    out
}

#[test]
fn test_stitch_2of3_xor_parity_custody_and_sha256_r4_42() {
    let root = repo_root();

    // Verify LexLean StitchFabric semantic module.
    let raw = std::fs::read_to_string(root.join("src/R4/StitchFabric.lex.tex"))
        .expect("src/R4/StitchFabric.lex.tex must exist");
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .expect("semanticdata tag in StitchFabric.lex.tex");
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .expect("end semanticmodule in StitchFabric.lex.tex");
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert!(raw.contains("\\begin{lexlean}{R4.StitchFabric}"));
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    assert!(raw.contains("admitStitchManifest"));
    assert!(raw.contains("admitStitchParityRecovery"));

    // Valid manifest check.
    let valid_manifest = StitchManifestCheck {
        schema_valid: true,
        threshold: 2,
        total_count: 3,
        data_shard_count: 3,
        parity_group_count: 2,
        xor_digest_verified: true,
    };
    assert_eq!(
        admit_stitch_manifest(valid_manifest),
        StitchParityVerdict::ParityManifestAdmitted
    );

    // Typed manifest refusals (R5).
    assert_eq!(
        admit_stitch_manifest(StitchManifestCheck {
            schema_valid: false,
            ..valid_manifest
        }),
        StitchParityVerdict::RefusedInvalidSchemaOrScheme
    );
    assert_eq!(
        admit_stitch_manifest(StitchManifestCheck {
            threshold: 1,
            ..valid_manifest
        }),
        StitchParityVerdict::RefusedInsufficientThreshold
    );
    assert_eq!(
        admit_stitch_manifest(StitchManifestCheck {
            parity_group_count: 1, // ceil(3/2) == 2 required
            ..valid_manifest
        }),
        StitchParityVerdict::RefusedShardParityMismatch
    );
    assert_eq!(
        admit_stitch_manifest(StitchManifestCheck {
            xor_digest_verified: false,
            ..valid_manifest
        }),
        StitchParityVerdict::RefusedShardParityMismatch
    );

    // Positive 2-of-3 recovery combinations: left+right, left+parity, right+parity.
    for (left, right, parity) in [
        (true, true, false),
        (true, false, true),
        (false, true, true),
    ] {
        let verdict = admit_stitch_parity_recovery(StitchRecoveryCheck {
            left_present: left,
            right_present: right,
            parity_present: parity,
            available_shards: 2,
            sha256_verified: true,
            xor_consistent: true,
        });
        assert_eq!(verdict, StitchRecoveryVerdict::RecoveryAdmitted);
        assert!(is_stitch_recovery_admitted(verdict));
    }

    // Negative recovery checks (R5).
    let one_shard = admit_stitch_parity_recovery(StitchRecoveryCheck {
        left_present: true,
        right_present: false,
        parity_present: false,
        available_shards: 1,
        sha256_verified: true,
        xor_consistent: true,
    });
    assert_eq!(one_shard, StitchRecoveryVerdict::RefusedFewerThanTwoShards);
    assert!(!is_stitch_recovery_admitted(one_shard));

    let bad_sha = admit_stitch_parity_recovery(StitchRecoveryCheck {
        left_present: true,
        right_present: false,
        parity_present: true,
        available_shards: 2,
        sha256_verified: false,
        xor_consistent: true,
    });
    assert_eq!(bad_sha, StitchRecoveryVerdict::RefusedSha256Mismatch);
    assert!(!is_stitch_recovery_admitted(bad_sha));

    let bad_xor = admit_stitch_parity_recovery(StitchRecoveryCheck {
        left_present: false,
        right_present: true,
        parity_present: true,
        available_shards: 2,
        sha256_verified: true,
        xor_consistent: false,
    });
    assert_eq!(bad_xor, StitchRecoveryVerdict::RefusedXorParityMismatch);
    assert!(!is_stitch_recovery_admitted(bad_xor));

    // Byte-level 2-of-3 XOR parity recovery and SHA-256 verification.
    let left_bytes = b"UORGLLM9_SHARD_0_Q2_WEIGHTS".to_vec();
    let right_bytes = b"UORGLLM9_SHARD_1_PRLL".to_vec();
    let parity_bytes = xor_pair(&left_bytes, &right_bytes);
    let recovered_right = &xor_pair(&left_bytes, &parity_bytes)[..right_bytes.len()];
    let recovered_left = &xor_pair(&right_bytes, &parity_bytes)[..left_bytes.len()];
    assert_eq!(
        sha256::sha256_hex(recovered_right),
        sha256::sha256_hex(&right_bytes)
    );
    assert_eq!(
        sha256::sha256_hex(recovered_left),
        sha256::sha256_hex(&left_bytes)
    );
}
