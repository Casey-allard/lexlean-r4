//! Conformance test for R4-71: `foundry` Hologram Quantum PC, `FK-01..09` `InTabProver`
//! Proof-Gated Lifecycle, Sealed `MlsChannel` (`FR-05`), and Float-Forbidding Canonical JSON `kappa`.

#![allow(missing_docs)]

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_foundry_quantum_studio::{
    admit_forge_kernel_transition, admit_quantum_pc_kappa, verify_foundry_quantum_studio_admitted,
    ForgeKernelTransitionCheck, ForgeKernelVerdict, FoundryLifecycleStage, QuantumPcKappaCheck,
    QuantumPcKappaVerdict,
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
fn test_r4_71_foundry_quantum_studio_r4_71() {
    let root = repo_root();

    let stages = [
        FoundryLifecycleStage::Drafting,
        FoundryLifecycleStage::Checked,
        FoundryLifecycleStage::Proved,
        FoundryLifecycleStage::PublishedHolo,
    ];
    assert_eq!(stages.len(), 4);

    let ok_fk = ForgeKernelTransitionCheck {
        workspace_edited_after_proof: false,
        checked_before_prove: true,
        proved_before_publish: true,
        extra_non_propext_quot_axioms: 0,
        requests_external_action: true,
        explicit_grant_present: true,
        explicit_denial_present: false,
    };
    assert_eq!(
        admit_forge_kernel_transition(ok_fk.clone()),
        ForgeKernelVerdict::ForgeTransitionAdmitted
    );

    let mut edited = ok_fk.clone();
    edited.workspace_edited_after_proof = true;
    assert_eq!(
        admit_forge_kernel_transition(edited),
        ForgeKernelVerdict::ReturnedToDraftingOnEdit
    );

    let mut unproved = ok_fk.clone();
    unproved.proved_before_publish = false;
    assert_eq!(
        admit_forge_kernel_transition(unproved),
        ForgeKernelVerdict::RefusedPublishWithoutProof
    );

    let mut unsound = ok_fk.clone();
    unsound.extra_non_propext_quot_axioms = 1;
    assert_eq!(
        admit_forge_kernel_transition(unsound),
        ForgeKernelVerdict::RefusedUnsoundKernelAxioms
    );

    let mut denied = ok_fk;
    denied.explicit_denial_present = true;
    assert_eq!(
        admit_forge_kernel_transition(denied),
        ForgeKernelVerdict::RefusedUngrantedCapabilityOrDenial
    );

    let ok_qpc = QuantumPcKappaCheck {
        float_tokens_found: 0,
        duplicate_keys_found: 0,
        kappa_verified: true,
        linux_vm_ready: true,
        mls_sealed_channel_ready: true,
    };
    assert_eq!(
        admit_quantum_pc_kappa(ok_qpc.clone()),
        QuantumPcKappaVerdict::QuantumPcHoloAdmitted
    );

    let mut float_qpc = ok_qpc.clone();
    float_qpc.float_tokens_found = 1;
    assert_eq!(
        admit_quantum_pc_kappa(float_qpc),
        QuantumPcKappaVerdict::RefusedCanonicalJsonFloat
    );

    let mut dup_qpc = ok_qpc;
    dup_qpc.duplicate_keys_found = 1;
    assert_eq!(
        admit_quantum_pc_kappa(dup_qpc),
        QuantumPcKappaVerdict::RefusedCanonicalJsonDuplicateKey
    );

    assert!(verify_foundry_quantum_studio_admitted());

    check_lex_module(
        &root,
        "src/R4/FoundryQuantumStudio.lex.tex",
        "R4.FoundryQuantumStudio",
        &[
            "FoundryLifecycleStage",
            "ForgeKernelTransitionCheck",
            "ForgeKernelVerdict",
            "QuantumPcKappaCheck",
            "QuantumPcKappaVerdict",
            "admitForgeKernelTransition",
            "admitQuantumPcKappa",
            "verifyFoundryQuantumStudioAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/FoundryQuantumStudioControls.lex.tex",
        "FoundryQuantumStudioControls",
        &["foundryQuantumStudioAdmitted"],
    );

    let qpc_rs = std::fs::read_to_string(root.join("crates/wasm/src/foundry_quantum_pc.rs"))
        .expect("crates/wasm/src/foundry_quantum_pc.rs must exist");
    assert!(qpc_rs.contains("pub fn uor_canonical_kappa"));
    assert!(qpc_rs.contains("pub fn prove_in_tab"));
    assert!(qpc_rs.contains("pub fn publish_holo"));
    assert!(qpc_rs.contains("pub struct QuantumPcMlsChannel"));
}
