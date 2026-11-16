//! Repository gates and autonomous homeostasis engine (`R4-28`).
//!
//! `cargo xtask <task>`; `just vv` runs the whole normative acceptance gate.
//! Each task below enforces one of the rules `AGENTS.md` sets out, and each
//! names the rule it enforces when it fails, so that a red gate says *which
//! promise* was broken rather than merely that something is wrong.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use repo_model::{codegen, Model};

mod audit;
mod bootstrap;
mod engine;
#[allow(dead_code, clippy::all, missing_docs)]
#[path = "../../crates/wasm/src/lexlean_generated.rs"]
pub mod lexlean_generated;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/prism_holo_compiler.rs"]
pub mod prism_holo_compiler;
mod routes_and_measures;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/wasm_ast_eval.rs"]
pub mod wasm_ast_eval;
use engine::wasm_bundle;

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
        "build-wasm" => wasm_bundle::run_build_wasm_cmd(&root, rest),
        "build-holo" => run_build_holo_cmd(&root, rest),
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

/// A gate failure, reported with the rule it broke.
type Fail = Box<dyn std::error::Error>;

/// R1, `CM-01`: the generated Rust consts equal the model, numeral for
/// numeral.
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
    engine::mechanizer::run_mechanizer_gate(root)?;
    engine::ratchet::check(root)?;
    routes_and_measures::check_routes(root)?;
    routes_and_measures::check_measures(root)?;
    run_build_holo_cmd(root, &["--verify-site-manifest".to_string()])?;
    engine::synthesis::cmd_synthesize(root, false)?;
    bootstrap::audit(root)?;
    Ok(())
}

/// Compile the 6-file `public/` browser closure into a Hologram v4 `.holo` archive (`R4-54`),
/// verifying its `core-wasm@1` acceptance vectors and BLAKE3 footer against `site/app.json`.
fn run_build_holo_cmd(root: &Path, args: &[String]) -> Result<(), Fail> {
    let mut out_path: Option<PathBuf> = None;
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

    if let Some(dest) = &out_path {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, &archive)?;
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
        std::fs::write(
            &site_manifest_path,
            format!("{}\n", serde_json::to_string_pretty(&doc)?),
        )?;
    } else if verify_site {
        let raw = std::fs::read_to_string(&site_manifest_path)?;
        let doc: serde_json::Value = serde_json::from_str(&raw)?;
        let expected_fp = doc["fingerprint"].as_str().unwrap_or("");
        if expected_fp != summary.fingerprint {
            return Err(format!(
                "site/app.json fingerprint mismatch: site/app.json={} compiled={}; run `cargo run -q -p xtask -- build-holo --sync-site-manifest`",
                expected_fp, summary.fingerprint
            )
            .into());
        }
    }

    println!(
        "build-holo: verified v4 archive ({} bytes, blake3={}, vectors={}/{}, view_files={})",
        summary.byte_length,
        &summary.fingerprint[..16],
        summary.vectors_passed,
        summary.vectors_total,
        summary.view_file_count,
    );
    Ok(())
}
