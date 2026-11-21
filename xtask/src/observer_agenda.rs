//! Generative Second-Order Self-Refinement Agenda (`R4-83`, `R4.ObserverRefinementAgenda`).
//!
//! Ported from `afflom/verdant-engine` (`crates/verdant-core/src/observer_refine.rs`):
//! inspects all 6 accountable LexLean-R4 organs (`engine`, `studio`, `runtime`, `proofs`,
//! `tensor3d`, `automation`), `model/lift_register.json` unlifted `residue` blockers,
//! `.lex.tex` proof coverage, and Wasm byte headroom, emitting a SHA-256-committed
//! [`ObserverRefinementAgenda`] (`model/observer_refinement_agenda.json`) that drives
//! autonomous Dual-Surface Proof + Application Ratchet ticks (`R4-84`..`R4-85`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::lift_register::{scan_codebase_lift_register, LiftRow};
use super::prism_factory_generated::r4_observer_refinement_agenda::{
    admit_observer_refinement_agenda, part_health_score_permille, AgendaVerdict,
};
use super::ratchet::measure_ratchet_state;

type Fail = Box<dyn std::error::Error>;

/// Canonical schema identifier for `model/observer_refinement_agenda.json` (`R4.ObserverRefinementAgenda`).
pub const OBSERVER_AGENDA_SPEC: &str = "r4/observer-refinement-agenda/1";

/// Urgency tier for an Observer-generated self-refinement proposal (`R4.ObserverRefinementAgenda`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObserverRefinementPriority {
    /// Safety, formal closure, or ratchet ceiling pressure requiring immediate action.
    Critical,
    /// High-leverage dual-surface proof + application lift or modularity refinement.
    High,
    /// Steady architectural or UX parity improvement.
    Medium,
    /// Ongoing monotonic optimization when thresholds show healthy headroom.
    Continuous,
}

/// Per-organ modularity & ownership health snapshot (`R4.ObserverRefinementAgenda`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverPartHealthSnapshot {
    /// Accountable organ identifier (`engine`, `studio`, `runtime`, `proofs`, `tensor3d`, `automation`).
    pub organ_id: String,
    /// Viable System Model tier (`VSM-S1`..`VSM-S5`, `VSM-S3*`).
    pub vsm_system: String,
    /// Count of existing workspace files owned by this organ.
    pub owned_file_count: usize,
    /// Line count of the largest text source file owned by this organ.
    pub max_file_lines: usize,
    /// Repo-relative path of the largest file owned by this organ.
    pub heaviest_file: String,
    /// Constructive health score in permille (`0..=1000`) from `R4.ObserverRefinementAgenda.partHealthScorePermille`.
    pub health_score_permille: u64,
}

/// Top unlifted `residue` blocker summary from `model/lift_register.json` (`R4.ObserverRefinementAgenda`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidueBlockerSummary {
    /// Source file path grouping unlifted `residue` functions.
    pub file_path: String,
    /// Reason classification (`unlifted-pure-fn`, `calls-unlifted-residue`, etc.).
    pub reason: String,
    /// Count of `residue` functions in this file/reason bucket.
    pub function_count: usize,
    /// First deterministic candidate function identifier in this bucket.
    pub top_function: String,
}

/// Organ-routed, authority-cited generative refinement proposal (`R4.ObserverRefinementAgenda`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverRefinementProposal {
    /// Deterministic proposal identifier.
    pub proposal_id: String,
    /// Accountable organ owning the targeted slice.
    pub owning_organ: String,
    /// VSM tier of the owning organ.
    pub vsm_system: String,
    /// Cited authoritative specification (`R4.*`).
    pub cited_authority: String,
    /// Ratchet layer governed by this proposal.
    pub ratchet_layer: String,
    /// Priority tier.
    pub priority: ObserverRefinementPriority,
    /// Optional concrete function or module path targeted for autonomous dual-surface lift.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_function: Option<String>,
    /// Live observed metric string.
    pub current_value: String,
    /// Recommended next monotonic watermark string.
    pub target_value: String,
    /// Concise rationale for this refinement step.
    pub summary: String,
    /// Concrete autonomous or developer actions.
    pub concrete_actions: Vec<String>,
    /// Content-addressed SHA-256 hex commitment over this proposal.
    pub kappa_hex: String,
}

/// Live cross-surface metrics observed during generative self-diagnosis (`R4.ObserverRefinementAgenda`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverRefinementMetrics {
    /// Byte length of `public/lexlean_r4_bg.wasm`.
    pub wasm_core_bytes: u64,
    /// Remaining Wasm byte headroom below `14,155,776` B (`13.5 MiB`).
    pub wasm_headroom_bytes: u64,
    /// Max lines across shipped Rust files.
    pub max_rust_file_lines: u64,
    /// Max lines across shipped JS modules.
    pub max_js_file_lines: u64,
    /// Formally lifted function count (`uorust/function-kappa/1`).
    pub lifted_functions: u64,
    /// Unlifted pure `residue` function count (`uorust/function-kappa/1`).
    pub residue_functions: u64,
    /// Verified `.lex.tex` module count across `src/R4` and `tests/lexlean`.
    pub verified_lexlean_modules: u64,
    /// Recommended next `min_lifted_functions` floor.
    pub next_lifted_target: u64,
    /// Recommended next `max_residue_functions` ceiling.
    pub next_residue_target: u64,
}

/// Cryptographically sealed generative self-refinement agenda (`model/observer_refinement_agenda.json`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverRefinementAgenda {
    /// Fixed format identifier (`r4/observer-refinement-agenda/1`).
    pub spec: String,
    /// Aggregate health score in permille (`0..=1000`).
    pub overall_refinement_score_permille: u64,
    /// Live cross-surface metrics measured directly from the workspace.
    pub live_metrics: ObserverRefinementMetrics,
    /// Per-organ modularity and ownership health across all 6 organs.
    pub part_health: Vec<ObserverPartHealthSnapshot>,
    /// Top unlifted `residue` function blockers from `lift_register`.
    pub top_residue_blockers: Vec<ResidueBlockerSummary>,
    /// Prioritized generative refinement proposals.
    pub proposals: Vec<ObserverRefinementProposal>,
    /// Content-addressed SHA-256 hex commitment over this agenda.
    pub agenda_kappa_hex: String,
}

/// Compute lowercase hex SHA-256 over `bytes` (`R4.ObserverRefinementAgenda`).
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Count newline-delimited lines in `path` (`R4.ObserverRefinementAgenda`).
fn count_file_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Measure per-organ file count, max lines, and `partHealthScorePermille` (`R4.ObserverRefinementAgenda`).
fn measure_organ_health(
    root: &Path,
    organ_id: &str,
    vsm_system: &str,
    owned_paths: &[&str],
    loc_ideal: u64,
    loc_ceiling: u64,
) -> ObserverPartHealthSnapshot {
    let mut owned_file_count = 0usize;
    let mut max_file_lines = 0usize;
    let mut heaviest_file = String::new();

    for rel in owned_paths {
        let full = root.join(rel);
        if full.is_file() {
            owned_file_count += 1;
            let lines = count_file_lines(&full);
            if lines >= max_file_lines {
                max_file_lines = lines;
                heaviest_file = (*rel).to_string();
            }
        } else if full.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&full) {
                let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
                entries.sort();
                for p in entries {
                    if p.is_file() {
                        owned_file_count += 1;
                        let lines = count_file_lines(&p);
                        let rel_p = p
                            .strip_prefix(root)
                            .unwrap_or(&p)
                            .to_string_lossy()
                            .replace('\\', "/");
                        if lines >= max_file_lines {
                            max_file_lines = lines;
                            heaviest_file = rel_p;
                        }
                    }
                }
            }
        }
    }

    let health_score_permille =
        part_health_score_permille(max_file_lines as u64, loc_ideal, loc_ceiling);
    ObserverPartHealthSnapshot {
        organ_id: organ_id.to_string(),
        vsm_system: vsm_system.to_string(),
        owned_file_count,
        max_file_lines,
        heaviest_file,
        health_score_permille,
    }
}

/// Rank pure `residue` functions from `lift_register` preferring shipped `crates/wasm` targets (`R4.ObserverRefinementAgenda`).
pub fn rank_residue_blockers(rows: &[LiftRow]) -> (Vec<ResidueBlockerSummary>, Option<String>) {
    let mut buckets: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut preferred_direct_fn: Option<String> = None;

    for row in rows {
        if row.status != "residue" {
            continue;
        }
        let file_path = row
            .function
            .split("::")
            .next()
            .unwrap_or(&row.function)
            .to_string();
        let reason = row
            .reason
            .clone()
            .unwrap_or_else(|| "unlifted-pure-fn".to_string());
        if preferred_direct_fn.is_none()
            && reason == "unlifted-pure-fn"
            && (file_path.starts_with("crates/wasm/src/")
                || file_path.starts_with("crates/wasm/js/"))
        {
            preferred_direct_fn = Some(row.function.clone());
        }
        buckets
            .entry((file_path, reason))
            .or_default()
            .push(row.function.clone());
    }

    let mut summaries: Vec<ResidueBlockerSummary> = buckets
        .into_iter()
        .map(|((file_path, reason), mut fns)| {
            fns.sort();
            ResidueBlockerSummary {
                file_path,
                reason,
                function_count: fns.len(),
                top_function: fns.into_iter().next().unwrap_or_default(),
            }
        })
        .collect();

    summaries.sort_by(|a, b| {
        let a_wasm = a.file_path.starts_with("crates/wasm/");
        let b_wasm = b.file_path.starts_with("crates/wasm/");
        b_wasm
            .cmp(&a_wasm)
            .then_with(|| b.function_count.cmp(&a.function_count))
            .then_with(|| a.file_path.cmp(&b.file_path))
    });

    if preferred_direct_fn.is_none() {
        preferred_direct_fn = summaries.first().map(|s| s.top_function.clone());
    }
    summaries.truncate(8);
    (summaries, preferred_direct_fn)
}

/// Seal proposal `kappa_hex` over deterministic pipe-delimited preimage (`R4.ObserverRefinementAgenda`).
fn seal_proposal(mut p: ObserverRefinementProposal) -> ObserverRefinementProposal {
    let preimage = format!(
        "{}|{}|{}|{}|{}|{}",
        p.proposal_id,
        p.owning_organ,
        p.cited_authority,
        p.current_value,
        p.target_value,
        p.summary
    );
    p.kappa_hex = sha256_hex_bytes(preimage.as_bytes());
    p
}

/// Synthesize and formally verify the 6-organ [`ObserverRefinementAgenda`] (`R4-83`).
pub fn synthesize_observer_refinement_agenda(
    root: &Path,
) -> Result<ObserverRefinementAgenda, Fail> {
    let measured = measure_ratchet_state(root)?;
    let lift_scan = scan_codebase_lift_register(root)?;
    let (mut top_residue_blockers, fallback_fn) = rank_residue_blockers(&lift_scan.document.rows);
    let exact_candidate =
        super::dual_surface_ratchet::select_next_lift_candidate(root, &lift_scan.document.rows)
            .map(|c| c.function_id);
    let top_target_fn = exact_candidate.or(fallback_fn);
    if top_residue_blockers.is_empty() && measured.residue_functions == 0 {
        top_residue_blockers = Vec::new();
    }

    let part_health = vec![
        measure_organ_health(
            root,
            "engine",
            "VSM-S4",
            &[
                "xtask/src/engine.rs",
                "xtask/src/ratchet.rs",
                "model/engine.toml",
                "frontier.toml",
            ],
            2500,
            10000,
        ),
        measure_organ_health(
            root,
            "studio",
            "VSM-S1",
            &[
                "crates/wasm/src/assistant.rs",
                "crates/wasm/src/mvm.rs",
                "crates/wasm/src/mvm_shell.rs",
                "crates/wasm/js/app",
            ],
            2500,
            10000,
        ),
        measure_organ_health(
            root,
            "runtime",
            "VSM-S2",
            &[
                "crates/wasm/src/holo_geometric_runtime.rs",
                "crates/wasm/src/hopf_zeta_engine.rs",
                "crates/wasm/src/sparse_basin_v15.rs",
                "crates/wasm/src/api.rs",
                "crates/wasm/js/bridge",
            ],
            2500,
            10000,
        ),
        measure_organ_health(
            root,
            "proofs",
            "VSM-S3*",
            &["src/R4", "tests/lexlean", "xtask/src/mechanizer.rs"],
            1200,
            5000,
        ),
        measure_organ_health(
            root,
            "tensor3d",
            "VSM-S1",
            &[
                "crates/wasm/src/vtensor_semiring_3d.rs",
                "crates/wasm/src/foundry_qpc.rs",
                "crates/wasm/src/prism_holo_compiler.rs",
            ],
            1500,
            6000,
        ),
        measure_organ_health(
            root,
            "automation",
            "VSM-S5",
            &[
                ".github/workflows/engine.yml",
                ".github/workflows/pages.yml",
                "xtask/src/lift_register.rs",
                "xtask/src/synthesis.rs",
                "xtask/src/wasm_bundle.rs",
            ],
            1500,
            5000,
        ),
    ];

    let next_lifted_target = measured.lifted_functions.saturating_add(1);
    let next_residue_target = measured.residue_functions.saturating_sub(1);
    let wasm_headroom_bytes = 14_155_776u64.saturating_sub(measured.wasm_core_bytes);

    let mut proposals = Vec::new();
    let target_label = top_target_fn
        .clone()
        .unwrap_or_else(|| "src/R4/RatchetGenNext.lex.tex".to_string());
    proposals.push(seal_proposal(ObserverRefinementProposal {
        proposal_id: "PROP-DUAL-SURFACE-LIFT-01".to_string(),
        owning_organ: "runtime".to_string(),
        vsm_system: "VSM-S2".to_string(),
        cited_authority: "R4.DualSurfaceRatchetEvolution".to_string(),
        ratchet_layer: "dual-surface-proof-and-app".to_string(),
        priority: if measured.residue_functions > 0 {
            ObserverRefinementPriority::High
        } else {
            ObserverRefinementPriority::Continuous
        },
        target_function: top_target_fn,
        current_value: format!(
            "lifted={}, residue={}",
            measured.lifted_functions, measured.residue_functions
        ),
        target_value: format!(
            "lifted>={}, residue<={}",
            next_lifted_target, next_residue_target
        ),
        summary: format!(
            "Co-evolve constructive LexLean proof + application lift for `{target_label}`"
        ),
        concrete_actions: vec![
            "Synthesize constructive src/R4/RatchetGen<N>.lex.tex + Controls with CAP0005 per-root falsifier negation.".to_string(),
            format!("Wire formal LexLean predicate bridge on `{target_label}` and rebuild public/ Wasm + .holo."),
            "Click min_lifted_functions UP and max_residue_functions DOWN in model/ratchet.toml.".to_string(),
        ],
        kappa_hex: String::new(),
    }));

    proposals.push(seal_proposal(ObserverRefinementProposal {
        proposal_id: "PROP-PROOF-FALSIFIER-02".to_string(),
        owning_organ: "proofs".to_string(),
        vsm_system: "VSM-S3*".to_string(),
        cited_authority: "R4.ObserverRefinementAgenda".to_string(),
        ratchet_layer: "lexlean-proof-frontier".to_string(),
        priority: ObserverRefinementPriority::High,
        target_function: None,
        current_value: format!("verified_modules={}", measured.verified_lexlean_modules),
        target_value: format!(
            "verified_modules>={}",
            measured.verified_lexlean_modules.saturating_add(2)
        ),
        summary: "Advance constructive zero-Classical.choice LexLean proof & CAP0005 falsifier count"
            .to_string(),
        concrete_actions: vec![
            "Verify every zero-arg Bool verify* root fails when negated under CAP0005.".to_string(),
            "Verify Lean 4.32.1 kernel proofs and append mechanized Rust module to lexlean_generated.rs.".to_string(),
        ],
        kappa_hex: String::new(),
    }));

    let heaviest = part_health
        .iter()
        .max_by_key(|p| p.max_file_lines)
        .cloned()
        .unwrap_or(ObserverPartHealthSnapshot {
            organ_id: "studio".to_string(),
            vsm_system: "VSM-S1".to_string(),
            owned_file_count: 1,
            max_file_lines: measured.max_rust_file_lines as usize,
            heaviest_file: "crates/wasm/src/assistant.rs".to_string(),
            health_score_permille: 800,
        });
    proposals.push(seal_proposal(ObserverRefinementProposal {
        proposal_id: "PROP-MODULARITY-03".to_string(),
        owning_organ: heaviest.organ_id.clone(),
        vsm_system: heaviest.vsm_system.clone(),
        cited_authority: "R4.ModularDecompositionRatchet".to_string(),
        ratchet_layer: "source-modularity".to_string(),
        priority: ObserverRefinementPriority::Medium,
        target_function: None,
        current_value: format!("{}={}L", heaviest.heaviest_file, heaviest.max_file_lines),
        target_value: format!(
            "max_rust_lines<={}, max_js_lines<={}",
            measured.max_rust_file_lines, measured.max_js_file_lines
        ),
        summary: format!(
            "Hold or shrink heaviest organ file `{}` ({} lines)",
            heaviest.heaviest_file, heaviest.max_file_lines
        ),
        concrete_actions: vec![
            "Preserve monotone max_rust_file_lines and max_js_file_lines ceilings in model/ratchet.toml."
                .to_string(),
        ],
        kappa_hex: String::new(),
    }));

    proposals.push(seal_proposal(ObserverRefinementProposal {
        proposal_id: "PROP-WASM-HEADROOM-04".to_string(),
        owning_organ: "automation".to_string(),
        vsm_system: "VSM-S5".to_string(),
        cited_authority: "R4.VerdantRatchetCycle".to_string(),
        ratchet_layer: "wasm-and-capsule-budget".to_string(),
        priority: ObserverRefinementPriority::Continuous,
        target_function: None,
        current_value: format!("wasm_core_bytes={}", measured.wasm_core_bytes),
        target_value: "wasm_core_bytes<=14155776".to_string(),
        summary: format!(
            "Maintain Wasm binary headroom ({wasm_headroom_bytes} B below 13.5 MiB ceiling) and .holo capsule parity"
        ),
        concrete_actions: vec![
            "Verify check-wasm-sync and build-holo --verify-site-manifest on every autonomous tick."
                .to_string(),
        ],
        kappa_hex: String::new(),
    }));

    let sum_permille: u64 = part_health.iter().map(|p| p.health_score_permille).sum();
    let overall_refinement_score_permille = sum_permille / (part_health.len().max(1) as u64);

    let live_metrics = ObserverRefinementMetrics {
        wasm_core_bytes: measured.wasm_core_bytes,
        wasm_headroom_bytes,
        max_rust_file_lines: measured.max_rust_file_lines,
        max_js_file_lines: measured.max_js_file_lines,
        lifted_functions: measured.lifted_functions,
        residue_functions: measured.residue_functions,
        verified_lexlean_modules: measured.verified_lexlean_modules,
        next_lifted_target,
        next_residue_target,
    };

    let mut agenda = ObserverRefinementAgenda {
        spec: OBSERVER_AGENDA_SPEC.to_string(),
        overall_refinement_score_permille,
        live_metrics,
        part_health,
        top_residue_blockers,
        proposals,
        agenda_kappa_hex: String::new(),
    };
    let preimage = serde_json::to_vec(&agenda)?;
    agenda.agenda_kappa_hex = sha256_hex_bytes(&preimage);

    let verdict = admit_observer_refinement_agenda(
        agenda.part_health.len() as u64,
        agenda.proposals.len() as u64,
        agenda.live_metrics.next_lifted_target,
        agenda.live_metrics.lifted_functions,
        agenda.live_metrics.next_residue_target,
        agenda.live_metrics.residue_functions,
        agenda.agenda_kappa_hex.len() == 64,
    );
    if verdict != AgendaVerdict::AgendaAdmitted {
        return Err(
            format!("R4.ObserverRefinementAgenda refused synthesized agenda: {verdict:?}").into(),
        );
    }

    Ok(agenda)
}

/// Cryptographically verify `parsed` SHA-256 commitments (`agenda_kappa_hex` and per-proposal `kappa_hex`) (`R4.ObserverRefinementAgenda`).
pub fn verify_agenda_cryptographic_kappa(parsed: &ObserverRefinementAgenda) -> Result<(), Fail> {
    if parsed.spec != OBSERVER_AGENDA_SPEC || parsed.agenda_kappa_hex.len() != 64 {
        return Err(
            "model/observer_refinement_agenda.json failed schema or kappa length check".into(),
        );
    }
    for p in &parsed.proposals {
        let resealed = seal_proposal(p.clone());
        if resealed.kappa_hex != p.kappa_hex {
            return Err(format!(
                "proposal `{}` kappa_hex mismatch: committed={} expected={}",
                p.proposal_id, p.kappa_hex, resealed.kappa_hex
            )
            .into());
        }
    }
    let mut unsealed = parsed.clone();
    unsealed.agenda_kappa_hex.clear();
    let expected_kappa = sha256_hex_bytes(&serde_json::to_vec(&unsealed)?);
    if parsed.agenda_kappa_hex != expected_kappa {
        return Err(format!(
            "agenda_kappa_hex tampered or stale: committed={} expected={expected_kappa}",
            parsed.agenda_kappa_hex
        )
        .into());
    }
    let verdict = admit_observer_refinement_agenda(
        parsed.part_health.len() as u64,
        parsed.proposals.len() as u64,
        parsed.live_metrics.next_lifted_target,
        parsed.live_metrics.lifted_functions,
        parsed.live_metrics.next_residue_target,
        parsed.live_metrics.residue_functions,
        true,
    );
    if verdict != AgendaVerdict::AgendaAdmitted {
        return Err(format!(
            "committed model/observer_refinement_agenda.json rejected: {verdict:?}"
        )
        .into());
    }
    Ok(())
}

/// Verify `crates/wasm/src/sovereign_ratchet.rs` constants match `sovereign_generations.toml` and live `lift_register` counts (`R4.ObserverRefinementAgenda`).
fn verify_sovereign_ratchet_sync(
    root: &Path,
    lifted_functions: u64,
    residue_functions: u64,
) -> Result<(), Fail> {
    let sov_path = root.join("crates/wasm/src/sovereign_ratchet.rs");
    if !sov_path.is_file() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(&sov_path)?;
    let gens = super::dual_surface_ratchet::load_sovereign_generations(root)?;
    let expected_gen = gens.generation.last().map(|g| g.index).unwrap_or(1);
    let want_gen = format!("pub const ACTIVE_REPO_GENERATION: u64 = {expected_gen};");
    let want_lifted = format!("pub const ACTIVE_REPO_LIFTED_COUNT: u64 = {lifted_functions};");
    let want_residue = format!("pub const ACTIVE_REPO_RESIDUE_COUNT: u64 = {residue_functions};");
    if !raw.contains(&want_gen) || !raw.contains(&want_lifted) || !raw.contains(&want_residue) {
        return Err(format!(
            "crates/wasm/src/sovereign_ratchet.rs constants drifted from live repo ({want_gen}, {want_lifted}, {want_residue}); run `cargo run -q -p xtask -- ratchet --write`"
        )
        .into());
    }
    Ok(())
}

/// CLI entrypoint for `cargo run -q -p xtask -- observer-agenda [--write]` (`R4-83`).
pub fn cmd_observer_agenda(root: &Path, write: bool) -> Result<ObserverRefinementAgenda, Fail> {
    let out_path = root.join("model/observer_refinement_agenda.json");
    if write {
        super::dual_surface_ratchet::sync_sovereign_ratchet_from_repo(root)?;
        let agenda = synthesize_observer_refinement_agenda(root)?;
        let rendered = format!("{}\n", serde_json::to_string_pretty(&agenda)?);
        std::fs::write(&out_path, &rendered)?;
        println!(
            "observer-agenda (write=true): score={}‰ organs={} proposals={} lifted={}->{} residue={}->{} kappa={}",
            agenda.overall_refinement_score_permille,
            agenda.part_health.len(),
            agenda.proposals.len(),
            agenda.live_metrics.lifted_functions,
            agenda.live_metrics.next_lifted_target,
            agenda.live_metrics.residue_functions,
            agenda.live_metrics.next_residue_target,
            &agenda.agenda_kappa_hex[..16]
        );
        return Ok(agenda);
    }

    let committed = std::fs::read_to_string(&out_path).map_err(|e| {
        format!(
            "missing {}: {e}; run `cargo run -q -p xtask -- observer-agenda --write`",
            out_path.display()
        )
    })?;
    let parsed: ObserverRefinementAgenda = serde_json::from_str(&committed)
        .map_err(|e| format!("invalid {}: {e}", out_path.display()))?;
    verify_agenda_cryptographic_kappa(&parsed)?;

    let live = measure_ratchet_state(root)?;
    if parsed.live_metrics.lifted_functions != live.lifted_functions
        || parsed.live_metrics.residue_functions != live.residue_functions
        || parsed.live_metrics.verified_lexlean_modules != live.verified_lexlean_modules
    {
        return Err(format!(
            "model/observer_refinement_agenda.json drifted from live workspace (agenda lifted={}/residue={}/mods={}, live lifted={}/residue={}/mods={}); run `cargo run -q -p xtask -- observer-agenda --write`",
            parsed.live_metrics.lifted_functions,
            parsed.live_metrics.residue_functions,
            parsed.live_metrics.verified_lexlean_modules,
            live.lifted_functions,
            live.residue_functions,
            live.verified_lexlean_modules,
        )
        .into());
    }
    verify_sovereign_ratchet_sync(root, live.lifted_functions, live.residue_functions)?;

    println!(
        "observer-agenda check ok: score={}‰ organs={} proposals={} kappa={}",
        parsed.overall_refinement_score_permille,
        parsed.part_health.len(),
        parsed.proposals.len(),
        &parsed.agenda_kappa_hex[..16]
    );
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesizes_six_organ_agenda_and_rejects_tampered_kappa() {
        let root = repo_model::repo_root();
        let agenda = synthesize_observer_refinement_agenda(&root).expect("synthesizes agenda");
        assert_eq!(agenda.spec, OBSERVER_AGENDA_SPEC);
        assert_eq!(agenda.part_health.len(), 6);
        assert!(agenda.proposals.len() >= 4);
        assert_eq!(agenda.agenda_kappa_hex.len(), 64);
        assert!(agenda.live_metrics.next_lifted_target > agenda.live_metrics.lifted_functions);
        verify_agenda_cryptographic_kappa(&agenda).expect("valid kappa");

        let mut bad_agenda = agenda.clone();
        bad_agenda.agenda_kappa_hex = "0".repeat(64);
        assert!(verify_agenda_cryptographic_kappa(&bad_agenda).is_err());

        let mut bad_prop = agenda;
        bad_prop.proposals[0].kappa_hex = "f".repeat(64);
        assert!(verify_agenda_cryptographic_kappa(&bad_prop).is_err());
    }
}
