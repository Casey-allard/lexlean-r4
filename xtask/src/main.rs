//! Repository gates and autonomous homeostasis engine (`R4-28`).
//!
//! `cargo xtask <task>`; `just vv` runs the whole normative acceptance gate.
//! Each task below enforces one of the rules `AGENTS.md` sets out, and each
//! names the rule it enforces when it fails, so that a red gate says *which
//! promise* was broken rather than merely that something is wrong.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use repo_model::{codegen, Model};

use xtask::engine::wasm_bundle;
use xtask::{audit, bootstrap, engine, prism_holo_compiler, routes_and_measures, Fail};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let task = args.get(1).map(String::as_str).unwrap_or("help");
    let rest = if args.len() > 2 { &args[2..] } else { &[] };
    let write = args.iter().any(|a| a == "--write");
    let root = repo_model::repo_root();

    let result: Result<(), Fail> = match task {
        "check-model" => check_model(&root, write),
        "audit-limits" => audit::audit_limits(&root),
        "audit-deferral" => audit::audit_deferral(&root),
        "audit-bootstrap" => bootstrap::audit(&root),
        "audit-dependency-updates" => bootstrap::audit_dependency_updates(&root),
        "mechanize" => engine::mechanizer::cmd_mechanize(&root),
        "lift-register" => engine::lift_register::cmd_lift_register(&root, write),
        "synthesize" => engine::synthesis::cmd_synthesize(&root, write),
        "ratchet-check" => engine::ratchet::check(&root).map(|_| ()),
        "ratchet-cycle" => engine::ratchet::run_ratchet_cycle(&root, write).map(|_| ()),
        "observer-agenda" => engine::observer_agenda::cmd_observer_agenda(&root, write).map(|_| ()),
        "dual-surface-ratchet" => {
            engine::dual_surface_ratchet::cmd_dual_surface_ratchet(&root, write)
        }
        "ratchet" => (|| -> Result<(), Fail> {
            let evolve = rest.iter().any(|a| a == "--evolve");
            let dry_run = rest.iter().any(|a| a == "--dry-run");
            let commit = rest.iter().any(|a| a == "--commit");
            let push = rest.iter().any(|a| a == "--push");
            if evolve || dry_run || commit || push {
                engine::run_engine_tick_with_mode(&root, 75, commit, push, dry_run)
                    .map_err(Into::into)
            } else {
                if write {
                    engine::lift_register::cmd_lift_register(&root, true)?;
                }
                engine::ratchet::run_ratchet_cycle(&root, write)?;
                engine::observer_agenda::cmd_observer_agenda(&root, write).map(|_| ())
            }
        })(),
        "recompile-v14" => xtask::recompiler_v14::run_recompile_v14_cmd(&root, rest).map(|_| ()),
        "studio-compile-v14-corpus" => {
            xtask::studio_lora_corpus::run_studio_compile_v14_corpus_cmd(&root, rest).map(|_| ())
        }
        "build-wasm" => wasm_bundle::run_build_wasm_cmd(&root, rest),
        "build-holo" => run_build_holo_cmd(&root, rest),
        "generate-studio-sft-dataset" => run_generate_studio_sft_dataset_cmd(&root, rest),
        "check-wasm-sync" => wasm_bundle::run_check_wasm_sync_cmd(&root),
        "check-routes" => routes_and_measures::check_routes(&root),
        "check-measures" => routes_and_measures::check_measures(&root),
        "validate" => validate(&root),
        "status" => engine::print_status(&root).map_err(Into::into),
        "brief" => engine::print_brief(&root).map_err(Into::into),
        "chain" => engine::print_chain(&root).map_err(Into::into),
        "claim" => engine::print_claim(&root, rest).map_err(Into::into),
        "frontier" => engine::run_frontier_cmd(&root, rest).map_err(Into::into),
        "discover" => engine::discover(&root, write, 2)
            .map(|added| {
                println!(
                    "discover (write={write}): {} candidate claim(s) induced: {:?}",
                    added.len(),
                    added
                );
            })
            .map_err(Into::into),
        "audit" => engine::audit_uorlut_synchronization(&root)
            .map(|a| {
                println!(
                    "audit: UORLUT01 reply_anchors={} point_rows={} orphaned_point_rows={}",
                    a.reply_anchor_count, a.point_row_count, a.orphaned_point_rows
                );
            })
            .map_err(Into::into),
        "deduce" => {
            let tick_id = "2026-11-05T04:00:00Z";
            let mut eng = match engine::load_engine(&root) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("gate failed: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let mut front = match engine::load_frontier(&root) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("gate failed: {e}");
                    return ExitCode::FAILURE;
                }
            };
            engine::deduce(&root, &mut eng, &mut front, tick_id, 2)
                .and_then(|summary| {
                    if !summary.proved_ids.is_empty() && write {
                        for (strat, fam, succ) in &summary.outcomes {
                            engine::update_weight(&mut eng.weight, strat, fam, *succ);
                        }
                        engine::save_frontier(&root, &front)?;
                        engine::save_engine(&root, &eng)?;
                        let _compiled =
                            engine::compile_proved_frontier_claims_to_uorlut(&root, &front)?;
                    }
                    println!(
                        "deduce (write={write}): proved {} claim(s): {:?}",
                        summary.proved_ids.len(),
                        summary.proved_ids
                    );
                    Ok(())
                })
                .map_err(Into::into)
        }
        "engine" => engine::run_engine_cmd(&root, rest).map_err(Into::into),
        "api" => engine::run_api_cmd(&root, rest).map_err(Into::into),
        "literature-sync" => engine::run_literature_sync_cmd(&root, rest).map_err(Into::into),
        _ => {
            eprintln!(
                "cargo xtask <task>\n\
                 \n\
                 check-model       R1: model/*.toml is the single source; regenerate and diff\n\
                 audit-limits      R5: no bound that cannot be traced to a parameter\n\
                 audit-deferral    R4: no deferral marker, no stub, no capability behind a flag\n\
                 audit-bootstrap   immutable SDK and least-privilege workflow trust root\n\
                 audit-dependency-updates   SDK ownership and ordinary dependency maintenance\n\
                 mechanize         R4-45: topological .lex.tex AST mechanizer + breaks* sweep\n\
                 lift-register     R4-45: UORust function kappa-lift scanner + UE-16 ratchet (--write)\n\
                 synthesize        R4-45: 4-facet holon KPIs + Hebbian bandit Pareto tick (--write)\n\
                 ratchet-check     R4-41: shrink-only metric ratchet + Peirce triadic cycle\n\
                 ratchet-cycle     R4-69: Verdant M15 bidirectional floors-UP / ceilings-DOWN ratchet (--write)\n\
                 observer-agenda   R4-83: generative 6-organ self-refinement agenda (--write)\n\
                 dual-surface-ratchet  R4-84: synthesize RatchetGen proof + lift residue slice (--write)\n\
                 ratchet           R4-85: unified dual-surface ratchet (--write | --dry-run | --evolve | --commit | --push)\n\
                 recompile-v14     R4-72: compile Qwen3-0.6B & Qwen3-1.7B UORR4V14 Hopf-Zeta atlases (--out-dir)\n\
                 build-wasm        compile crates/wasm, inject bridge + cache hook, write provenance.json\n\
                 build-holo        compile public/ closure into Hologram v4 .holo (--out / --sync-site-manifest / --verify-site-manifest)\n\
                 check-wasm-sync   verify public/ Wasm/JS bundle matches provenance.json\n\
                 check-routes      R4-53: verify model/routes.toml UORust route table + vectors\n\
                 check-measures    R4-53: verify model/measures.toml & provenance.toml criteria\n\
                 validate          run every gate above\n\
                 status | brief    print autonomous engine, frontier, and UORLUT01 status\n\
                 frontier          list, check, or update frontier.toml claims\n\
                 discover          induce candidate relations into frontier.toml (--write)\n\
                 audit             verify 0 orphaned PointKey rows and anchor synchronization\n\
                 deduce            run priority-scheduled deduction over frontier claims\n\
                 engine            run autonomous homeostasis subcommands (status|hypotheses|prune|tick)\n\
                 api               check or synchronize autonomous engine ledgers (check|sync)\n\
                 literature-sync   fetch or update arXiv literature catalog in model/literature.toml\n\
                 \n\
                 --write           rewrite generated or induced files"
            );
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gate failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn check_model(root: &Path, write: bool) -> Result<(), Fail> {
    let model = Model::load(&root.join("model"))?;
    model.check()?;

    let conformance = codegen::render_conformance(&model);
    let conformance_path: PathBuf = root.join(codegen::CONFORMANCE_PATH);

    if write {
        std::fs::write(&conformance_path, &conformance)?;
        println!("wrote {}", conformance_path.display());
        return Ok(());
    }

    let committed = std::fs::read_to_string(&conformance_path).map_err(|e| {
        format!(
            "{}: {e}\nrun `cargo xtask check-model --write`",
            conformance_path.display()
        )
    })?;
    if committed != conformance {
        return Err(format!(
            "{} is stale: it disagrees with model/ids.toml.\n\
             R2: a claim cannot exist in the documentation without a ledger row. \
             Run `cargo xtask check-model --write`.",
            conformance_path.display()
        )
        .into());
    }
    println!(
        "check-model: CONFORMANCE.md equals the model, {} ids (CM-01)",
        model.ids.id.len()
    );
    Ok(())
}

/// The whole normative acceptance gate, in one place.
fn validate(root: &Path) -> Result<(), Fail> {
    check_model(root, false)?;
    audit::audit_limits(root)?;
    audit::audit_deferral(root)?;
    let mech_rep = engine::mechanizer::run_mechanizer_gate(root)?;
    let _ratchet_rep = engine::ratchet::check_with_mechanizer(root, Some(&mech_rep))?;
    engine::observer_agenda::cmd_observer_agenda(root, false)?;
    engine::validate_workflow_homeostasis(root)?;
    routes_and_measures::check_routes(root)?;
    routes_and_measures::check_measures(root)?;
    wasm_bundle::run_check_wasm_sync_cmd(root)?;
    run_build_holo_cmd(root, &["--verify-site-manifest".to_string()])?;
    engine::synthesis::cmd_synthesize_with_reports(root, false, Some(&mech_rep), Some(true))?;
    bootstrap::audit(root)?;
    Ok(())
}

/// Compile the 6-file `public/` browser closure into a Hologram v4 `.holo` archive (`R4-54`),
/// verifying its `core-wasm@1` acceptance vectors and BLAKE3 footer against `site/app.json`.
pub fn run_build_holo_cmd(root: &Path, args: &[String]) -> Result<(), Fail> {
    let mut out_path: Option<PathBuf> = None;
    let mut emit_fp_dir: Option<PathBuf> = None;
    let mut sync_site = false;
    let mut verify_site = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                let raw = args
                    .get(i + 1)
                    .ok_or("--out requires a destination .holo path")?;
                let p = PathBuf::from(raw);
                out_path = Some(if p.is_absolute() { p } else { root.join(p) });
                i += 2;
            }
            "--emit-fingerprint-dir" => {
                let raw = args
                    .get(i + 1)
                    .ok_or("--emit-fingerprint-dir requires a destination directory")?;
                let p = PathBuf::from(raw);
                emit_fp_dir = Some(if p.is_absolute() { p } else { root.join(p) });
                i += 2;
            }
            "--sync-site-manifest" => {
                sync_site = true;
                i += 1;
            }
            "--verify-site-manifest" => {
                verify_site = true;
                i += 1;
            }
            other => return Err(format!("unknown build-holo argument `{other}`").into()),
        }
    }

    let public_dir = root.join("public");
    let closure_names = [
        "app.css",
        "app.js",
        "index.html",
        "lexlean_r4.js",
        "lexlean_r4_bg.wasm",
        "provenance.json",
    ];
    let mut files = std::collections::BTreeMap::new();
    for name in closure_names {
        let p = public_dir.join(name);
        let bytes =
            std::fs::read(&p).map_err(|e| format!("build-holo reading {}: {e}", p.display()))?;
        files.insert(name.to_string(), bytes);
    }
    let prov_bytes = files.get("provenance.json").cloned().unwrap_or_default();

    let archive = prism_holo_compiler::compile_holo_v4_archive(&files, &prov_bytes)
        .map_err(|v| format!("build-holo compile refused: {v:?}"))?;
    let summary = prism_holo_compiler::verify_holo_v4_archive(&archive)
        .map_err(|v| format!("build-holo verify refused: {v:?}"))?;

    const MAX_HOLO_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
    if archive.len() > MAX_HOLO_ARCHIVE_BYTES {
        return Err(format!(
            "build-holo: compiled .holo archive ({} bytes) exceeds 32 MiB cap ({MAX_HOLO_ARCHIVE_BYTES} bytes)",
            archive.len()
        )
        .into());
    }

    let forbidden_lower_a = concat!("humu", "humu33");
    let forbidden_lower_b = concat!("/hologram", "-forge/");
    let contains_ci = |haystack: &[u8], needle: &[u8]| -> bool {
        haystack
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
    };
    if contains_ci(&archive, forbidden_lower_a.as_bytes())
        || contains_ci(&archive, forbidden_lower_b.as_bytes())
    {
        return Err(
            "build-holo: compiled .holo archive contains forbidden brand/path residue".into(),
        );
    }

    let wasm_sha256: String = {
        use sha2::{Digest, Sha256};
        let wasm_bytes = files
            .get("lexlean_r4_bg.wasm")
            .ok_or("missing lexlean_r4_bg.wasm in public/")?;
        Sha256::digest(wasm_bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    };
    let offline_q31_kappa: String = {
        use sha2::{Digest, Sha256};
        let q31_bytes = std::fs::read(root.join("crates/wasm/src/native_q31_d85.uorq31"))
            .map_err(|e| format!("reading native_q31_d85.uorq31: {e}"))?;
        let hex: String = Sha256::digest(&q31_bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("sha256:{hex}")
    };
    let qwen3_weights_kappa: String = {
        let mpath = root.join("gpu/weights/v13_qwen3_0_6b/manifest.json");
        if mpath.is_file() {
            let mraw = std::fs::read_to_string(&mpath)?;
            let mdoc: serde_json::Value = serde_json::from_str(&mraw)?;
            let csha = mdoc["container_sha256"]
                .as_str()
                .unwrap_or("617ea77fd9c3d15285e351bd4f094cc0845cf5bc038e042a3fb5aadf0e406bda");
            format!("sha256:{csha}")
        } else {
            "sha256:617ea77fd9c3d15285e351bd4f094cc0845cf5bc038e042a3fb5aadf0e406bda".to_string()
        }
    };

    if let Some(dest) = &out_path {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &archive)?;
    }
    if let Some(fp_dir) = &emit_fp_dir {
        std::fs::create_dir_all(fp_dir)?;
        let fp_file = fp_dir.join(format!("{}.holo", summary.fingerprint));
        std::fs::write(&fp_file, &archive)?;
    }

    let site_manifest_path = root.join("site/app.json");
    if sync_site {
        let raw = std::fs::read_to_string(&site_manifest_path)?;
        let mut doc: serde_json::Value = serde_json::from_str(&raw)?;
        doc["archiveKappa"] = serde_json::json!(summary.archive_kappa);
        doc["fingerprint"] = serde_json::json!(summary.fingerprint);
        doc["applicationKappa"] = serde_json::json!(summary.application_kappa);
        doc["guestContentKappa"] = serde_json::json!(summary.guest_content_kappa);
        doc["viewContentKappa"] = serde_json::json!(summary.view_content_kappa);
        doc["modelContentKappa"] = serde_json::json!(summary.model_content_kappa);
        doc["holoByteLength"] = serde_json::json!(summary.byte_length);
        doc["wasmSha256"] = serde_json::json!(wasm_sha256);
        doc["holoUrl"] = serde_json::json!("https://casey-allard.github.io/lexlean-r4/app.holo");
        doc["holoFingerprintUrl"] = serde_json::json!(format!(
            "https://casey-allard.github.io/lexlean-r4/holo/{}.holo",
            summary.fingerprint
        ));
        doc["qwen3WeightsKappa"] = serde_json::json!(qwen3_weights_kappa);
        doc["offlineQ31WeightsKappa"] = serde_json::json!(offline_q31_kappa);
        std::fs::write(
            &site_manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&doc)?),
        )?;
    } else if verify_site {
        let raw = std::fs::read_to_string(&site_manifest_path)?;
        let doc: serde_json::Value = serde_json::from_str(&raw)?;
        verify_site_manifest_pins(
            &doc,
            &summary.fingerprint,
            &wasm_sha256,
            &summary.archive_kappa,
            summary.byte_length,
            &qwen3_weights_kappa,
            &offline_q31_kappa,
        )?;
    }

    println!(
        "build-holo: verified v4 archive ({} bytes, blake3={}, wasmSha256={}, vectors={}/{}, view_files={})",
        summary.byte_length,
        &summary.fingerprint[..16],
        &wasm_sha256[..16],
        summary.vectors_passed,
        summary.vectors_total,
        summary.view_file_count,
    );
    Ok(())
}

fn verify_site_manifest_pins(
    doc: &serde_json::Value,
    expected_fp: &str,
    expected_wasm_sha: &str,
    expected_archive_kappa: &str,
    expected_byte_len: usize,
    expected_qwen3_kappa: &str,
    expected_q31_kappa: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_fp_url =
        format!("https://casey-allard.github.io/lexlean-r4/holo/{expected_fp}.holo");
    let string_pins = [
        ("fingerprint", expected_fp),
        ("wasmSha256", expected_wasm_sha),
        ("archiveKappa", expected_archive_kappa),
        ("holoFingerprintUrl", expected_fp_url.as_str()),
        ("qwen3WeightsKappa", expected_qwen3_kappa),
        ("offlineQ31WeightsKappa", expected_q31_kappa),
    ];
    for (field, expected) in string_pins {
        let actual = doc.get(field).and_then(|v| v.as_str()).unwrap_or("");
        if actual != expected {
            return Err(format!(
                "site/app.json {field} mismatch: site/app.json={actual} compiled={expected}; run `cargo run -q -p xtask -- build-holo --sync-site-manifest`"
            )
            .into());
        }
    }
    let actual_bytes = doc
        .get("holoByteLength")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if actual_bytes != expected_byte_len as u64 {
        return Err(format!(
            "site/app.json holoByteLength mismatch: site/app.json={actual_bytes} compiled={expected_byte_len}; run `cargo run -q -p xtask -- build-holo --sync-site-manifest`"
        )
        .into());
    }
    Ok(())
}

#[path = "../../gpu/build_studio_sft_dataset.rs"]
mod studio_sft_dataset;

/// Formal R4.WebGpuResidentRuntime generate deterministic Studio SFT multi-turn episodes (`R4-64`).
fn run_generate_studio_sft_dataset_cmd(
    root: &std::path::Path,
    args: &[String],
) -> Result<(), Fail> {
    let default_out = root.join("target/studio_sft_dataset");
    let out_dir = args
        .windows(2)
        .find(|w| w[0] == "--out-dir")
        .map(|w| std::path::PathBuf::from(&w[1]))
        .unwrap_or(default_out);
    let episodes: usize = args
        .windows(2)
        .find(|w| w[0] == "--episodes")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(1536);
    let (tr, ev) = studio_sft_dataset::write_studio_sft_dataset(&out_dir, episodes)?;
    println!(
        "generate-studio-sft-dataset: wrote train={tr} eval={ev} to {}",
        out_dir.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_site_manifest_pins;

    #[test]
    fn verify_site_manifest_rejects_mismatched_pins() {
        let fp = "a".repeat(64);
        let fp_url = format!("https://casey-allard.github.io/lexlean-r4/holo/{fp}.holo");
        let base = serde_json::json!({
            "fingerprint": fp,
            "wasmSha256": "b".repeat(64),
            "archiveKappa": "sha256:archive",
            "holoByteLength": 12345_u64,
            "holoFingerprintUrl": fp_url,
            "qwen3WeightsKappa": "sha256:qwen3",
            "offlineQ31WeightsKappa": "sha256:q31",
        });
        verify_site_manifest_pins(
            &base,
            &fp,
            &"b".repeat(64),
            "sha256:archive",
            12345,
            "sha256:qwen3",
            "sha256:q31",
        )
        .expect("matching pins verify cleanly");

        for (field, bad_val) in [
            ("fingerprint", serde_json::json!("bad")),
            ("wasmSha256", serde_json::json!("bad")),
            ("archiveKappa", serde_json::json!("sha256:bad")),
            ("holoByteLength", serde_json::json!(999_u64)),
            ("holoFingerprintUrl", serde_json::json!("https://wrong")),
            ("qwen3WeightsKappa", serde_json::json!("sha256:bad")),
            ("offlineQ31WeightsKappa", serde_json::json!("sha256:bad")),
        ] {
            let mut mutated = base.clone();
            mutated[field] = bad_val;
            let err = verify_site_manifest_pins(
                &mutated,
                &fp,
                &"b".repeat(64),
                "sha256:archive",
                12345,
                "sha256:qwen3",
                "sha256:q31",
            )
            .expect_err("mismatched pin must be rejected");
            assert!(
                err.to_string().contains(field),
                "expected field {field} in diagnostic: {err}"
            );
        }
    }
}
