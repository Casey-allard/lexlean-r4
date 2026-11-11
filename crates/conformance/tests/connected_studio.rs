//! Connected Chat-First Studio, Worker turn isolation, GitHub secret host-allowlisting,
//! `EM_RISCV` SHA-256 package admission, and `SpiralCore` `CID 2` transport controls (`R4-39`).

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::process::Command;

use serde_json::{json, Value};

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;
#[path = "support/sha256.rs"]
mod sha256;

use lexlean_generated::r4_connected_studio::{
    admit_bridge_transport, admit_git_secret_substitution, admit_package_install,
    admit_worker_turn, BridgeTransportVerdict, GitSecretVerdict, McpPolicyMode,
    PackageInstallVerdict, TransportKind, WorkerTurnVerdict,
};

const LEXLEAN_SPEC_PATH: &str = "tests/lexlean/R4/ConnectedStudio.lex.tex";
const EXPECTED_CONNECTED_STUDIO_SHA256: &str =
    "7c416738ea1bcfe99c52cdf246b1fc31abf81d0f30979c79309c6cc05e4e90d3";

#[derive(Clone, Debug, PartialEq, Eq)]
enum EvalVal {
    Bool(bool),
    Nat(u64),
    Ctor(String),
}

fn eval_expr(
    expr: &Value,
    env: &BTreeMap<String, EvalVal>,
    defs: &BTreeMap<String, Value>,
) -> Result<EvalVal, String> {
    let kind = expr["kind"].as_str().ok_or("missing expr kind")?;
    match kind {
        "bool" => Ok(EvalVal::Bool(
            expr["value"].as_bool().ok_or("invalid bool literal")?,
        )),
        "nat" => Ok(EvalVal::Nat(
            expr["value"]
                .as_str()
                .ok_or("invalid nat literal")?
                .parse::<u64>()
                .map_err(|e| format!("bad nat: {e}"))?,
        )),
        "var" => {
            let name = expr["name"].as_str().ok_or("missing var name")?;
            env.get(name)
                .cloned()
                .ok_or_else(|| format!("unbound variable {name}"))
        }
        "constructor" => {
            let ctor = expr["constructor"]["name"]
                .as_str()
                .ok_or("missing constructor name")?;
            Ok(EvalVal::Ctor(ctor.to_string()))
        }
        "not" => match eval_expr(&expr["value"], env, defs)? {
            EvalVal::Bool(b) => Ok(EvalVal::Bool(!b)),
            other => Err(format!("not expected Bool, got {other:?}")),
        },
        "and" => {
            let EvalVal::Bool(l) = eval_expr(&expr["left"], env, defs)? else {
                return Err("and left expected Bool".into());
            };
            if !l {
                return Ok(EvalVal::Bool(false));
            }
            let EvalVal::Bool(r) = eval_expr(&expr["right"], env, defs)? else {
                return Err("and right expected Bool".into());
            };
            Ok(EvalVal::Bool(r))
        }
        "or" => {
            let EvalVal::Bool(l) = eval_expr(&expr["left"], env, defs)? else {
                return Err("or left expected Bool".into());
            };
            if l {
                return Ok(EvalVal::Bool(true));
            }
            let EvalVal::Bool(r) = eval_expr(&expr["right"], env, defs)? else {
                return Err("or right expected Bool".into());
            };
            Ok(EvalVal::Bool(r))
        }
        "beq" => {
            let EvalVal::Nat(l) = eval_expr(&expr["left"], env, defs)? else {
                return Err("beq left expected Nat".into());
            };
            let EvalVal::Nat(r) = eval_expr(&expr["right"], env, defs)? else {
                return Err("beq right expected Nat".into());
            };
            Ok(EvalVal::Bool(l == r))
        }
        "ble" => {
            let EvalVal::Nat(l) = eval_expr(&expr["left"], env, defs)? else {
                return Err("ble left expected Nat".into());
            };
            let EvalVal::Nat(r) = eval_expr(&expr["right"], env, defs)? else {
                return Err("ble right expected Nat".into());
            };
            Ok(EvalVal::Bool(l <= r))
        }
        "add" => {
            let EvalVal::Nat(l) = eval_expr(&expr["left"], env, defs)? else {
                return Err("add left expected Nat".into());
            };
            let EvalVal::Nat(r) = eval_expr(&expr["right"], env, defs)? else {
                return Err("add right expected Nat".into());
            };
            Ok(EvalVal::Nat(l.checked_add(r).ok_or("nat overflow")?))
        }
        "if" => {
            let EvalVal::Bool(cond) = eval_expr(&expr["condition"], env, defs)? else {
                return Err("if condition expected Bool".into());
            };
            if cond {
                eval_expr(&expr["then_value"], env, defs)
            } else {
                eval_expr(&expr["else_value"], env, defs)
            }
        }
        "match" => {
            let EvalVal::Ctor(scrut) = eval_expr(&expr["scrutinee"], env, defs)? else {
                return Err("match scrutinee expected Ctor".into());
            };
            let branches = expr["branches"]
                .as_array()
                .ok_or("missing match branches")?;
            for branch in branches {
                let b_ctor = branch["constructor"]["name"]
                    .as_str()
                    .ok_or("missing branch constructor")?;
                if b_ctor == scrut {
                    return eval_expr(&branch["body"], env, defs);
                }
            }
            Err(format!("non-exhaustive match on {scrut}"))
        }
        "call" => {
            let fn_name = expr["function"]["name"]
                .as_str()
                .ok_or("missing call function name")?;
            let def = defs
                .get(fn_name)
                .ok_or_else(|| format!("unknown definition {fn_name}"))?;
            let params = def["parameters"].as_array().ok_or("missing parameters")?;
            let args = expr["arguments"].as_array().ok_or("missing arguments")?;
            if params.len() != args.len() {
                return Err(format!("arity mismatch calling {fn_name}"));
            }
            let mut sub_env = BTreeMap::new();
            for (p, a) in params.iter().zip(args.iter()) {
                let p_name = p["name"].as_str().ok_or("missing param name")?;
                sub_env.insert(p_name.to_string(), eval_expr(a, env, defs)?);
            }
            eval_expr(&def["body"], &sub_env, defs)
        }
        other => Err(format!("unsupported AST expr kind: {other}")),
    }
}

fn verify_lexlean_module_and_parity() -> Result<(), Box<dyn Error>> {
    let root = repo_model::repo_root();
    let full_path = root.join(LEXLEAN_SPEC_PATH);
    let source = std::fs::read_to_string(&full_path)?;
    assert!(
        source.starts_with("\\begin{lexlean}{R4.ConnectedStudio}\n"),
        "ConnectedStudio.lex.tex must declare LexLean module R4.ConnectedStudio"
    );
    let digest = sha256::sha256_hex(source.as_bytes());
    assert_eq!(
        digest, EXPECTED_CONNECTED_STUDIO_SHA256,
        "reviewed ConnectedStudio.lex.tex control statements changed"
    );

    let semantic_json = source
        .split_once("\\semanticdata{")
        .ok_or("missing \\semanticdata{ in ConnectedStudio.lex.tex")?
        .1
        .split_once("\n\\end{semanticmodule}")
        .ok_or("missing \\end{semanticmodule} in ConnectedStudio.lex.tex")?
        .0
        .strip_suffix('}')
        .ok_or("missing closing } on \\semanticdata in ConnectedStudio.lex.tex")?;

    let ast: Value = serde_json::from_str(semantic_json)?;
    assert_eq!(ast["spec"], "lexlean/semantic-module/2");
    let decls = ast["declarations"]
        .as_array()
        .ok_or("missing declarations array")?;

    let mut inductives = BTreeSet::new();
    let mut defs = BTreeMap::new();
    let mut theorems = Vec::new();

    for decl in decls {
        let name = decl["name"].as_str().ok_or("missing decl name")?;
        match decl["kind"].as_str().ok_or("missing decl kind")? {
            "inductive" => {
                assert!(inductives.insert(name.to_string()));
            }
            "definition" => {
                assert!(defs.insert(name.to_string(), decl.clone()).is_none());
            }
            "theorem" => {
                assert_eq!(decl["axioms"], json!([]));
                assert_eq!(decl["proof"], json!({"kind": "reflexivity"}));
                theorems.push(decl.clone());
            }
            other => return Err(format!("unexpected declaration kind {other}").into()),
        }
    }

    for expected_ind in [
        "TransportKind",
        "McpPolicyMode",
        "BridgeTransportVerdict",
        "GitSecretVerdict",
        "PackageInstallVerdict",
        "WorkerTurnVerdict",
    ] {
        assert!(
            inductives.contains(expected_ind),
            "missing inductive {expected_ind} in ConnectedStudio.lex.tex"
        );
    }

    for expected_def in [
        "admitBridgeTransport",
        "admitGitSecretSubstitution",
        "admitPackageInstall",
        "admitWorkerTurn",
    ] {
        assert!(
            defs.contains_key(expected_def),
            "missing definition {expected_def} in ConnectedStudio.lex.tex"
        );
    }

    assert!(
        theorems.len() >= 18,
        "expected comprehensive positive and negative formal theorems, found {}",
        theorems.len()
    );

    let empty_env = BTreeMap::new();
    let mut theorem_names = BTreeSet::new();
    for thm in &theorems {
        let name = thm["name"].as_str().ok_or("missing theorem name")?;
        assert!(theorem_names.insert(name.to_string()));
        let stmt = &thm["statement"];
        assert_eq!(stmt["kind"], "eq");
        let lhs = eval_expr(&stmt["left"], &empty_env, &defs)?;
        let rhs = eval_expr(&stmt["right"], &empty_env, &defs)?;
        assert_eq!(lhs, rhs, "formal theorem `{name}` failed kernel reduction");
    }

    for required_thm in [
        "git_secret_never_admitted_when_guest_ram_contaminated",
        "git_secret_never_admitted_when_host_not_allowlisted",
        "package_install_valid_riscv64_elf_admitted",
        "package_install_digest_mismatch_refused",
        "package_install_wrong_machine_x86_64_refused",
    ] {
        assert!(
            theorem_names.contains(required_thm),
            "missing required theorem {required_thm}"
        );
    }

    // Cross-check exact behavioral equivalence between the LexLean AST definitions
    // and the generated Rust module `r4_connected_studio` across all domains.
    for s in [false, true] {
        for p in [false, true] {
            for h in [false, true] {
                let call = json!({
                    "kind": "call",
                    "function": {"name": "admitGitSecretSubstitution"},
                    "arguments": [
                        {"kind": "bool", "value": s},
                        {"kind": "bool", "value": p},
                        {"kind": "bool", "value": h}
                    ]
                });
                let EvalVal::Ctor(lean_ctor) = eval_expr(&call, &empty_env, &defs)? else {
                    return Err("expected Ctor from admitGitSecretSubstitution".into());
                };
                let rust_verdict = admit_git_secret_substitution(s, p, h);
                let expected_ctor = match rust_verdict {
                    GitSecretVerdict::SecretSubstitutionAdmitted => {
                        "GitSecretVerdict.SecretSubstitutionAdmitted"
                    }
                    GitSecretVerdict::GuestRamSecretContamination => {
                        "GitSecretVerdict.GuestRamSecretContamination"
                    }
                    GitSecretVerdict::HostNotAllowlistedForSecret => {
                        "GitSecretVerdict.HostNotAllowlistedForSecret"
                    }
                    GitSecretVerdict::MissingSecretPlaceholder => {
                        "GitSecretVerdict.MissingSecretPlaceholder"
                    }
                };
                assert_eq!(lean_ctor, expected_ctor);
            }
        }
    }

    for (src_cid, dst_cid) in [(0u16, 2u16), (2, 2), (3, 2), (4, 2), (3, 3)] {
        for (policy, p_ctor) in [
            (McpPolicyMode::Safe, "McpPolicyMode.Safe"),
            (McpPolicyMode::Confirm, "McpPolicyMode.Confirm"),
            (McpPolicyMode::Deny, "McpPolicyMode.Deny"),
        ] {
            for (transport, t_ctor) in [
                (TransportKind::WssRelay, "TransportKind.WssRelay"),
                (TransportKind::HttpsDirect, "TransportKind.HttpsDirect"),
                (TransportKind::LoopbackDev, "TransportKind.LoopbackDev"),
                (TransportKind::PlainHttp, "TransportKind.PlainHttp"),
                (TransportKind::PlainWs, "TransportKind.PlainWs"),
            ] {
                let call = json!({
                    "kind": "call",
                    "function": {"name": "admitBridgeTransport"},
                    "arguments": [
                        {"kind": "nat", "value": src_cid.to_string()},
                        {"kind": "nat", "value": dst_cid.to_string()},
                        {"kind": "constructor", "constructor": {"name": p_ctor}, "arguments": []},
                        {"kind": "constructor", "constructor": {"name": t_ctor}, "arguments": []}
                    ]
                });
                let EvalVal::Ctor(lean_ctor) = eval_expr(&call, &empty_env, &defs)? else {
                    return Err("expected Ctor from admitBridgeTransport".into());
                };
                let expected_ctor =
                    match admit_bridge_transport(src_cid, dst_cid, policy, transport) {
                        BridgeTransportVerdict::TransportAdmitted => {
                            "BridgeTransportVerdict.TransportAdmitted"
                        }
                        BridgeTransportVerdict::PolicyDenied => {
                            "BridgeTransportVerdict.PolicyDenied"
                        }
                        BridgeTransportVerdict::InsecureTransportRejected => {
                            "BridgeTransportVerdict.InsecureTransportRejected"
                        }
                        BridgeTransportVerdict::InvalidVsockCid => {
                            "BridgeTransportVerdict.InvalidVsockCid"
                        }
                    };
                assert_eq!(lean_ctor, expected_ctor);
            }
        }
    }

    for (sha_ok, magic_ok, class_64, machine, byte_len, max_bytes) in [
        (true, true, true, 243u16, 16_384u32, 2_097_152u32),
        (false, true, true, 243, 16_384, 2_097_152),
        (true, false, true, 243, 16_384, 2_097_152),
        (true, true, false, 243, 16_384, 2_097_152),
        (true, true, true, 62, 16_384, 2_097_152),
        (true, true, true, 243, 0, 2_097_152),
        (true, true, true, 243, 2_097_153, 2_097_152),
    ] {
        let call = json!({
            "kind": "call",
            "function": {"name": "admitPackageInstall"},
            "arguments": [
                {"kind": "bool", "value": sha_ok},
                {"kind": "bool", "value": magic_ok},
                {"kind": "bool", "value": class_64},
                {"kind": "nat", "value": machine.to_string()},
                {"kind": "nat", "value": byte_len.to_string()},
                {"kind": "nat", "value": max_bytes.to_string()}
            ]
        });
        let EvalVal::Ctor(lean_ctor) = eval_expr(&call, &empty_env, &defs)? else {
            return Err("expected Ctor from admitPackageInstall".into());
        };
        let expected_ctor =
            match admit_package_install(sha_ok, magic_ok, class_64, machine, byte_len, max_bytes) {
                PackageInstallVerdict::PackageAdmitted => "PackageInstallVerdict.PackageAdmitted",
                PackageInstallVerdict::DigestMismatch => "PackageInstallVerdict.DigestMismatch",
                PackageInstallVerdict::InvalidElf64Riscv => {
                    "PackageInstallVerdict.InvalidElf64Riscv"
                }
                PackageInstallVerdict::SizeExceedsQuota => "PackageInstallVerdict.SizeExceedsQuota",
            };
        assert_eq!(lean_ctor, expected_ctor);
    }

    for (on_worker, prompt, max_new, window) in [
        (true, 512u32, 512u32, 1024u32),
        (false, 128, 256, 1024),
        (true, 513, 512, 1024),
        (true, 0, 0, 0),
        (true, u32::MAX, 1, 1024),
    ] {
        let call = json!({
            "kind": "call",
            "function": {"name": "admitWorkerTurn"},
            "arguments": [
                {"kind": "bool", "value": on_worker},
                {"kind": "nat", "value": prompt.to_string()},
                {"kind": "nat", "value": max_new.to_string()},
                {"kind": "nat", "value": window.to_string()}
            ]
        });
        let EvalVal::Ctor(lean_ctor) = eval_expr(&call, &empty_env, &defs)? else {
            return Err("expected Ctor from admitWorkerTurn".into());
        };
        let expected_ctor = match admit_worker_turn(on_worker, prompt, max_new, window) {
            WorkerTurnVerdict::WorkerTurnAdmitted => "WorkerTurnVerdict.WorkerTurnAdmitted",
            WorkerTurnVerdict::MainThreadBlockingRejected => {
                "WorkerTurnVerdict.MainThreadBlockingRejected"
            }
            WorkerTurnVerdict::SequenceWindowExceeded => "WorkerTurnVerdict.SequenceWindowExceeded",
        };
        assert_eq!(lean_ctor, expected_ctor);
    }

    if std::env::var_os("SDK_IMAGE").is_some() {
        let lock_out = Command::new("lexlean")
            .current_dir(&root)
            .args(["lock", "--check"])
            .output()?;
        if !lock_out.status.success() {
            return Err(format!(
                "lexlean lock --check failed:\n{}\n{}",
                String::from_utf8_lossy(&lock_out.stdout),
                String::from_utf8_lossy(&lock_out.stderr)
            )
            .into());
        }
        let verify_out = Command::new("lexlean")
            .current_dir(&root)
            .args(["--diagnostic-format", "json", "verify", LEXLEAN_SPEC_PATH])
            .output()?;
        if !verify_out.status.success() {
            return Err(format!(
                "lexlean verify {LEXLEAN_SPEC_PATH} failed:\n{}\n{}",
                String::from_utf8_lossy(&verify_out.stdout),
                String::from_utf8_lossy(&verify_out.stderr)
            )
            .into());
        }
    }

    Ok(())
}

#[test]
fn test_connected_studio_transport_secret_pkg_and_worker_admission_r4_39(
) -> Result<(), Box<dyn Error>> {
    // 1. Outbound SpiralCore CID 2 bridge transport admission (positive + negative)
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Safe, TransportKind::WssRelay),
        BridgeTransportVerdict::TransportAdmitted
    );
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Confirm, TransportKind::HttpsDirect),
        BridgeTransportVerdict::TransportAdmitted
    );
    assert_eq!(
        admit_bridge_transport(4, 2, McpPolicyMode::Safe, TransportKind::LoopbackDev),
        BridgeTransportVerdict::TransportAdmitted
    );
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Deny, TransportKind::WssRelay),
        BridgeTransportVerdict::PolicyDenied
    );
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Deny, TransportKind::HttpsDirect),
        BridgeTransportVerdict::PolicyDenied
    );
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Safe, TransportKind::PlainHttp),
        BridgeTransportVerdict::InsecureTransportRejected
    );
    assert_eq!(
        admit_bridge_transport(3, 2, McpPolicyMode::Confirm, TransportKind::PlainWs),
        BridgeTransportVerdict::InsecureTransportRejected
    );
    for (bad_src, bad_dst) in [(0, 2), (1, 2), (2, 2), (3, 3), (3, 0), (2, 3), (3, 99)] {
        assert_eq!(
            admit_bridge_transport(
                bad_src,
                bad_dst,
                McpPolicyMode::Safe,
                TransportKind::WssRelay
            ),
            BridgeTransportVerdict::InvalidVsockCid,
            "expected InvalidVsockCid for ({bad_src}, {bad_dst})"
        );
    }

    // 2. GitHub secret placeholder substitution & guest RAM isolation (positive + negative)
    assert_eq!(
        admit_git_secret_substitution(false, true, true),
        GitSecretVerdict::SecretSubstitutionAdmitted
    );
    for p in [false, true] {
        for h in [false, true] {
            assert_eq!(
                admit_git_secret_substitution(true, p, h),
                GitSecretVerdict::GuestRamSecretContamination
            );
        }
    }
    assert_eq!(
        admit_git_secret_substitution(false, true, false),
        GitSecretVerdict::HostNotAllowlistedForSecret
    );
    assert_eq!(
        admit_git_secret_substitution(false, false, true),
        GitSecretVerdict::MissingSecretPlaceholder
    );
    assert_eq!(
        admit_git_secret_substitution(false, false, false),
        GitSecretVerdict::MissingSecretPlaceholder
    );

    // 3. Static riscv64 ELF64 (EM_RISCV = 243) + SHA-256 package admission (positive + negative)
    assert_eq!(
        admit_package_install(true, true, true, 243, 16_384, 2_097_152),
        PackageInstallVerdict::PackageAdmitted
    );
    assert_eq!(
        admit_package_install(true, true, true, 243, 2_097_152, 2_097_152),
        PackageInstallVerdict::PackageAdmitted
    );
    assert_eq!(
        admit_package_install(false, true, true, 243, 16_384, 2_097_152),
        PackageInstallVerdict::DigestMismatch
    );
    assert_eq!(
        admit_package_install(true, false, true, 243, 16_384, 2_097_152),
        PackageInstallVerdict::InvalidElf64Riscv
    );
    assert_eq!(
        admit_package_install(true, true, false, 243, 16_384, 2_097_152),
        PackageInstallVerdict::InvalidElf64Riscv
    );
    assert_eq!(
        admit_package_install(true, true, true, 62, 16_384, 2_097_152),
        PackageInstallVerdict::InvalidElf64Riscv
    );
    assert_eq!(
        admit_package_install(true, true, true, 183, 16_384, 2_097_152),
        PackageInstallVerdict::InvalidElf64Riscv
    );
    assert_eq!(
        admit_package_install(true, true, true, 243, 0, 2_097_152),
        PackageInstallVerdict::InvalidElf64Riscv
    );
    assert_eq!(
        admit_package_install(true, true, true, 243, 2_097_153, 2_097_152),
        PackageInstallVerdict::SizeExceedsQuota
    );

    // 4. Non-blocking Dedicated Web Worker + 1024-token multi-turn sequence bounds
    assert_eq!(
        admit_worker_turn(true, 128, 256, 1024),
        WorkerTurnVerdict::WorkerTurnAdmitted
    );
    assert_eq!(
        admit_worker_turn(true, 512, 512, 1024),
        WorkerTurnVerdict::WorkerTurnAdmitted
    );
    assert_eq!(
        admit_worker_turn(false, 128, 256, 1024),
        WorkerTurnVerdict::MainThreadBlockingRejected
    );
    assert_eq!(
        admit_worker_turn(true, 513, 512, 1024),
        WorkerTurnVerdict::SequenceWindowExceeded
    );
    assert_eq!(
        admit_worker_turn(true, u32::MAX, 1, 1024),
        WorkerTurnVerdict::SequenceWindowExceeded
    );
    assert_eq!(
        admit_worker_turn(true, 0, 0, 0),
        WorkerTurnVerdict::SequenceWindowExceeded
    );

    // 5. Formal LexLean specification AST & theorem verification + cross-language parity
    verify_lexlean_module_and_parity()?;

    Ok(())
}
