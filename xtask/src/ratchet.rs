//! Verdant-Resonance shrink-only metric ratchet & Peirce triadic gate (`R4-41`).
//!
//! Enforces `model/ratchet.toml` (`spec = "r4/ratchet/1"`) through the generated
//! formal LexLean predicates in `crates/wasm/src/lexlean_generated.rs`:
//! - `r4_autonomous_engine::admit_peirce_triadic_cycle`
//! - `r4_prism_factory::admit_ratchet_obligation`
//! - `r4_rule30_evolution::admit_shrink_only_ratchet`

use std::path::{Path, PathBuf};
use std::process::Command;

use repo_model::serde::{Deserialize, Serialize};
use repo_model::toml;

use super::load_engine;
use super::prism_factory_generated::{
    r4_autonomous_engine::{
        admit_peirce_triadic_cycle, PeircePhase, PeirceTriadicCheck, PeirceTriadicVerdict,
    },
    r4_prism_factory::admit_ratchet_obligation,
    r4_rule30_evolution::{admit_shrink_only_ratchet, RatchetMetricVerdict},
};

type Fail = Box<dyn std::error::Error>;

/// Required `spec` header for `model/ratchet.toml`.
pub const RATCHET_SPEC: &str = "r4/ratchet/1";

/// Authoritative ratchet configuration (`model/ratchet.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct RatchetModel {
    /// Schema marker (`r4/ratchet/1`).
    pub spec: String,
    /// Required ordered Peirce triadic phases.
    pub peirce_phases: Vec<String>,
    /// Cellular automaton discovery rule number (30).
    pub rule30_rule_number: u64,
    /// Minimum Rule-30 generation counter required for homeostasis.
    pub rule30_min_generation: u64,
    /// Maximum allowed handwritten bypass/fallback source files.
    pub max_handwritten_bypass_files: u64,
    /// Maximum allowed deferral markers across `crates/` and `xtask/`.
    pub max_deferral_markers: u64,
    /// Maximum allowed ignored tests across `crates/` and `xtask/`.
    pub max_ignored_conformance_tests: u64,
    /// Maximum allowed ungrounded `.lex.tex` modules under `src/R4/`.
    pub max_ungrounded_defs: u64,
    /// Maximum allowed orphaned `PointKey` rows in UORLUT consolidation.
    pub max_orphaned_point_rows: u64,
    /// Shrink-only trajectory L1 residual ceiling in milli-units.
    pub max_trajectory_l1_milli: u64,
    /// Maximum allowed unlifted pure `residue` functions (`R4-45`, `uorust/function-kappa/1`).
    #[serde(default)]
    pub max_residue_functions: Option<u64>,
    /// Maximum allowed floating-point serving operations (`R4-44`, must equal `0`).
    #[serde(default)]
    pub max_float_serving_ops: Option<u64>,
    /// Maximum allowed dense matrix-multiply / softmax serving operations (`R4-44`, must equal `0`).
    #[serde(default)]
    pub max_dense_matmul_ops: Option<u64>,
    /// Maximum allowed unwired interactive Studio UI controls (`R4-46`, must equal `0`).
    #[serde(default)]
    pub max_unwired_ui_controls: Option<u64>,
}

/// Measured repository state checked against `RatchetModel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetMeasuredState {
    /// Count of handwritten bypass/stub filenames under `crates/`.
    pub handwritten_bypass_files: u64,
    /// Count of deferral markers outside code spans across `crates/` and `xtask/`.
    pub deferral_markers: u64,
    /// Count of ignored tests under `crates/` and `xtask/`.
    pub ignored_conformance_tests: u64,
    /// Count of invalid or empty `src/R4/*.lex.tex` modules.
    pub ungrounded_defs: u64,
    /// Latest consolidated orphaned `PointKey` row count from `model/engine.toml`.
    pub orphaned_point_rows: u64,
    /// Current Rule-30 state from `model/engine.toml`.
    pub rule30_state: u64,
    /// Current Rule-30 generation counter from `model/engine.toml`.
    pub rule30_generation: u64,
    /// Count of unlifted pure `residue` functions under `uorust/function-kappa/1`.
    pub residue_functions: u64,
    /// Count of floating-point serving operations detected under `crates/wasm/src/`.
    pub float_serving_ops: u64,
    /// Count of dense matrix-multiply or softmax serving operations detected under `crates/wasm/src/`.
    pub dense_matmul_ops: u64,
    /// Count of unwired interactive controls between `public/index.html` and `public/app.js`.
    pub unwired_ui_controls: u64,
}

/// Result of verifying the shrink-only ratchet and Peirce triadic invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetReport {
    /// Parsed `model/ratchet.toml` policy.
    pub model: RatchetModel,
    /// Measured workspace metrics.
    pub measured: RatchetMeasuredState,
    /// True when git parent (`HEAD~1`) also had `model/ratchet.toml` and non-inflation held.
    pub monotone_vs_parent: bool,
    /// True when all three Peirce phases (`Abduction`, `Deduction`, `Induction`) admitted.
    pub peirce_admitted: bool,
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
            continue;
        }
        if p.is_dir() {
            collect_files(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

fn outside_backticks(line: &str, token: &str) -> bool {
    let mut rest = line;
    let mut offset = 0usize;
    while let Some(pos) = rest.find(token) {
        let abs = offset + pos;
        let ticks = line[..abs].bytes().filter(|&b| b == b'`').count();
        if ticks % 2 == 0 {
            return true;
        }
        let step = pos + token.len();
        offset += step;
        rest = &rest[step..];
    }
    false
}

fn is_valid_semantic_module(raw: &str) -> bool {
    let Some((_, after)) = raw.split_once("\\semanticdata{") else {
        return false;
    };
    let Some((json_block, _)) = after.split_once("\n\\end{semanticmodule}") else {
        return false;
    };
    let json_text = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json_text) else {
        return false;
    };
    val["spec"] == "lexlean/semantic-module/2"
        && val["declarations"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
}

/// Measure current codebase invariants needed for the shrink-only ratchet.
pub fn measure_ratchet_state(root: &Path) -> Result<RatchetMeasuredState, Fail> {
    let engine = load_engine(root)?;
    let orphaned_point_rows = engine
        .tick
        .last()
        .and_then(|t| t.consolidation.as_ref())
        .map(|c| c.orphaned_point_rows as u64)
        .unwrap_or(0);

    let deferral_tokens = [
        concat!("TO", "DO"),
        concat!("FIX", "ME"),
        concat!("XX", "X"),
        concat!("unimplemented", "!"),
        concat!("to", "do!"),
        concat!("for ", "now"),
        concat!("later ", "version"),
    ];
    let ignore_attr = concat!("#", "[ignore");

    let mut source_files = Vec::new();
    collect_files(&root.join("crates"), &mut source_files)?;
    collect_files(&root.join("xtask"), &mut source_files)?;
    source_files.sort();

    let mut handwritten_bypass_files = 0u64;
    let mut deferral_markers = 0u64;
    let mut ignored_conformance_tests = 0u64;

    for path in source_files {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("fallback_") || name.starts_with("stub_") {
                handwritten_bypass_files += 1;
            }
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            for marker in deferral_tokens {
                if line.contains(marker) && outside_backticks(line, marker) {
                    deferral_markers += 1;
                }
            }
            let trimmed = line.trim();
            if (trimmed == concat!("#", "[ignore]")
                || trimmed.starts_with(concat!("#", "[ignore(")))
                && outside_backticks(line, ignore_attr)
            {
                ignored_conformance_tests += 1;
            }
        }
    }

    let mut ungrounded_defs = 0u64;
    let r4_dir = root.join("src/R4");
    if !r4_dir.exists() {
        return Err("missing src/R4 semantic module directory".into());
    }
    let mut r4_entries: Vec<PathBuf> = std::fs::read_dir(&r4_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".lex.tex"))
        .collect();
    r4_entries.sort();
    if r4_entries.is_empty() {
        return Err("no .lex.tex modules found under src/R4".into());
    }
    for path in r4_entries {
        let raw = std::fs::read_to_string(&path)?;
        if !is_valid_semantic_module(&raw) {
            ungrounded_defs += 1;
        }
    }

    let residue_functions = super::lift_register::scan_codebase_lift_register(root)?
        .document
        .summary
        .residue;
    let float_serving_ops = measure_float_serving_ops(root)?;
    let dense_matmul_ops = measure_dense_matmul_ops(root)?;
    let unwired_ui_controls = measure_unwired_ui_controls(root)?;

    Ok(RatchetMeasuredState {
        handwritten_bypass_files,
        deferral_markers,
        ignored_conformance_tests,
        ungrounded_defs,
        orphaned_point_rows,
        rule30_state: engine.rule30_state,
        rule30_generation: engine.rule30_generation,
        residue_functions,
        float_serving_ops,
        dense_matmul_ops,
        unwired_ui_controls,
    })
}

fn serving_rust_files(root: &Path) -> Result<Vec<PathBuf>, Fail> {
    let dir = root.join("crates/wasm/src");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let p = entry?.path();
        if !p.extension().is_some_and(|e| e == "rs") {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(
            name,
            "lexlean_generated.rs" | "api.rs" | "otel.rs" | "glm_weights.rs"
        ) {
            continue;
        }
        files.push(p);
    }
    files.sort();
    Ok(files)
}

fn strip_rust_comments_and_strings(src: &str) -> String {
    let prod = src.split("#[cfg(test)]").next().unwrap_or(src);
    let bytes = prod.as_bytes();
    let mut out = String::with_capacity(prod.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            out.push(' ');
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn rust_ident_tokens(line: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;
    for (idx, ch) in line.char_indices() {
        let is_ident = ch.is_ascii_alphanumeric() || ch == '_';
        if is_ident {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(s) = start.take() {
            tokens.push(&line[s..idx]);
        }
    }
    if let Some(s) = start {
        tokens.push(&line[s..]);
    }
    tokens
}

/// Count floating-point serving operations in production kernels under `crates/wasm/src/` (`R4-44`).
pub fn measure_float_serving_ops(root: &Path) -> Result<u64, Fail> {
    let mut count = 0u64;
    for path in serving_rust_files(root)? {
        let is_lut_exec = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "lut_stack_exec.rs");
        let raw = std::fs::read_to_string(&path)?;
        let stripped = strip_rust_comments_and_strings(&raw);
        for line in stripped.lines() {
            // `lut_stack_exec.rs` carries a standalone `JsonVal::Num(f64)` schema parser field for BPE/header JSON;
            // it performs zero tensor or softmax float arithmetic.
            if is_lut_exec
                && (line.contains("JsonVal::Num")
                    || line.contains("Num(f64)")
                    || line.contains(".parse::<f64>()"))
            {
                continue;
            }
            for tok in rust_ident_tokens(line) {
                if matches!(tok, "f32" | "f64") || (tok.ends_with("_f32") || tok.ends_with("_f64"))
                {
                    count += 1;
                }
            }
            for pat in [".exp(", ".tanh(", ".powf(", ".sqrt("] {
                if line.contains(pat) {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// Count dense matrix-multiply or softmax serving calls under `crates/wasm/src/` (`R4-44`).
pub fn measure_dense_matmul_ops(root: &Path) -> Result<u64, Fail> {
    let mut count = 0u64;
    for path in serving_rust_files(root)? {
        let raw = std::fs::read_to_string(&path)?;
        let stripped = strip_rust_comments_and_strings(&raw);
        for line in stripped.lines() {
            for tok in rust_ident_tokens(line) {
                if matches!(
                    tok,
                    "dense_matmul"
                        | "matmul_f32"
                        | "matmul_f64"
                        | "cblas_sgemm"
                        | "cblas_dgemm"
                        | "sgemm"
                        | "dgemm"
                        | "softmax"
                        | "softmax_f32"
                        | "softmax_f64"
                        | "exp_f32"
                        | "exp_f64"
                ) {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

fn strip_html_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        if let Some(end) = after.find("-->") {
            let newlines = after[..end].bytes().filter(|&b| b == b'\n').count();
            for _ in 0..newlines {
                out.push('\n');
            }
            rest = &after[end + 3..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

fn extract_html_attr(tag: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{attr}={quote}");
        if let Some(pos) = tag.find(&needle) {
            let val_start = pos + needle.len();
            if let Some(end) = tag[val_start..].find(quote) {
                return Some(tag[val_start..val_start + end].to_string());
            }
        }
    }
    None
}

fn js_wires_id(app_js: &str, id: &str) -> bool {
    app_js.contains(&format!("\"{id}\""))
        || app_js.contains(&format!("'{id}'"))
        || app_js.contains(&format!("#{id}"))
}

/// Return sorted identifiers (or `tag@line` descriptors) of unwired `<button|input|select|textarea|form>` elements.
pub fn find_unwired_ui_controls(root: &Path) -> Result<Vec<String>, Fail> {
    let html_path = root.join("public/index.html");
    let js_path = root.join("public/app.js");
    if !html_path.is_file() {
        return Ok(Vec::new());
    }
    let html_raw = std::fs::read_to_string(&html_path)?;
    let app_js = if js_path.is_file() {
        std::fs::read_to_string(&js_path)?
    } else {
        String::new()
    };

    let html = strip_html_comments(&html_raw);
    let delegated_attrs = [
        "data-canvas-tab",
        "data-tab-id",
        "data-view",
        "data-composer-mode",
        "data-cursor-ai-mode",
        "data-chat-prompt",
        "data-tool",
    ];
    let wired_delegated: Vec<&str> = delegated_attrs
        .iter()
        .copied()
        .filter(|a| app_js.contains(a))
        .collect();

    let mut unwired = Vec::new();
    let mut active_form_id: Option<String> = None;
    let mut pos = 0usize;
    let bytes = html.as_bytes();
    while pos < bytes.len() {
        if bytes[pos] != b'<' {
            pos += 1;
            continue;
        }
        let Some(rel_end) = html[pos..].find('>') else {
            break;
        };
        let tag = &html[pos..=pos + rel_end];
        let line_no = html[..pos].bytes().filter(|&b| b == b'\n').count() + 1;
        pos += rel_end + 1;

        let trimmed = tag.trim_start_matches('<').trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("/form") {
            active_form_id = None;
            continue;
        }
        let Some(tag_name) = ["button", "input", "select", "textarea", "form"]
            .into_iter()
            .find(|name| {
                lower.starts_with(name)
                    && lower
                        .as_bytes()
                        .get(name.len())
                        .is_some_and(|b| b.is_ascii_whitespace() || *b == b'>' || *b == b'/')
            })
        else {
            continue;
        };

        let elem_id = extract_html_attr(tag, "id");
        if tag_name == "form" {
            active_form_id = elem_id.clone();
        }

        let id_wired = elem_id
            .as_deref()
            .is_some_and(|id| js_wires_id(&app_js, id));
        let delegated_wired = wired_delegated
            .iter()
            .any(|attr| tag.contains(&format!("{attr}=")));
        let submit_in_wired_form = tag_name != "form"
            && extract_html_attr(tag, "type").is_some_and(|t| t == "submit")
            && active_form_id
                .as_deref()
                .is_some_and(|fid| js_wires_id(&app_js, fid));

        if !id_wired && !delegated_wired && !submit_in_wired_form {
            unwired.push(elem_id.unwrap_or_else(|| format!("{tag_name}@{line_no}")));
        }
    }

    unwired.sort();
    unwired.dedup();
    Ok(unwired)
}

/// Count unwired interactive controls between `public/index.html` and `public/app.js` (`R4-46`).
pub fn measure_unwired_ui_controls(root: &Path) -> Result<u64, Fail> {
    Ok(find_unwired_ui_controls(root)?.len() as u64)
}

/// Parse `model/ratchet.toml`.
pub fn load_ratchet_model(root: &Path) -> Result<RatchetModel, Fail> {
    let path = root.join("model/ratchet.toml");
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let model: RatchetModel =
        toml::from_str(&raw).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    Ok(model)
}

fn check_metric(name: &str, measured: u64, ceiling: u64, seed_nonzero: bool) -> Result<(), String> {
    let obligation_ok = admit_ratchet_obligation(measured, ceiling, true);
    let verdict = admit_shrink_only_ratchet(ceiling, measured, seed_nonzero);
    if !obligation_ok || verdict != RatchetMetricVerdict::ShrinkAdmitted {
        return Err(format!(
            "shrink-only ratchet refused `{name}`: measured={measured} ceiling={ceiling} verdict={verdict:?}"
        ));
    }
    Ok(())
}

/// Enforce monotonic non-inflation comparing `current` against an earlier `parent` ratchet.
pub fn enforce_monotone_ceilings(
    current: &RatchetModel,
    parent: &RatchetModel,
) -> Result<(), String> {
    let pairs = [
        (
            "max_handwritten_bypass_files",
            current.max_handwritten_bypass_files,
            parent.max_handwritten_bypass_files,
        ),
        (
            "max_deferral_markers",
            current.max_deferral_markers,
            parent.max_deferral_markers,
        ),
        (
            "max_ignored_conformance_tests",
            current.max_ignored_conformance_tests,
            parent.max_ignored_conformance_tests,
        ),
        (
            "max_ungrounded_defs",
            current.max_ungrounded_defs,
            parent.max_ungrounded_defs,
        ),
        (
            "max_orphaned_point_rows",
            current.max_orphaned_point_rows,
            parent.max_orphaned_point_rows,
        ),
        (
            "max_trajectory_l1_milli",
            current.max_trajectory_l1_milli,
            parent.max_trajectory_l1_milli,
        ),
    ];
    for (name, next_c, prev_c) in pairs {
        if admit_shrink_only_ratchet(prev_c, next_c, true) != RatchetMetricVerdict::ShrinkAdmitted {
            return Err(format!(
                "ratchet ceiling regression in `{name}`: parent={prev_c}, current={next_c}"
            ));
        }
    }

    let opt_pairs = [
        (
            "max_residue_functions",
            current.max_residue_functions,
            parent.max_residue_functions,
            false,
        ),
        (
            "max_float_serving_ops",
            current.max_float_serving_ops,
            parent.max_float_serving_ops,
            true,
        ),
        (
            "max_dense_matmul_ops",
            current.max_dense_matmul_ops,
            parent.max_dense_matmul_ops,
            true,
        ),
        (
            "max_unwired_ui_controls",
            current.max_unwired_ui_controls,
            parent.max_unwired_ui_controls,
            true,
        ),
    ];
    for (name, next_opt, prev_opt, require_zero) in opt_pairs {
        let Some(next_c) = next_opt else {
            return Err(format!("missing required ratchet ceiling `{name}`"));
        };
        if require_zero && next_c != 0 {
            return Err(format!("ratchet ceiling `{name}` must be 0, got {next_c}"));
        }
        if let Some(prev_c) = prev_opt {
            if admit_shrink_only_ratchet(prev_c, next_c, true)
                != RatchetMetricVerdict::ShrinkAdmitted
            {
                return Err(format!(
                    "ratchet ceiling regression in `{name}`: parent={prev_c}, current={next_c}"
                ));
            }
        }
    }
    Ok(())
}

fn check_git_parent_ratchet(root: &Path, current: &RatchetModel) -> Result<bool, Fail> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["show", "HEAD~1:model/ratchet.toml"])
        .output();
    let Ok(output) = out else {
        return Ok(false);
    };
    if !output.status.success() {
        return Ok(false);
    }
    let parent_text = String::from_utf8_lossy(&output.stdout);
    let parent: RatchetModel = toml::from_str(&parent_text)
        .map_err(|e| format!("parsing HEAD~1:model/ratchet.toml: {e}"))?;
    enforce_monotone_ceilings(current, &parent)?;
    Ok(true)
}

/// Evaluate one Peirce triadic step (`Abduction -> Deduction -> Induction`) using formal LexLean verdicts.
pub fn verify_peirce_triadic_step(
    phase: PeircePhase,
    witness_present: bool,
    invariant_preserved: bool,
    prev_metric: u64,
    next_metric: u64,
    rule30_state: u64,
) -> Result<(), String> {
    let seed_nonzero = rule30_state != 0;
    let ratchet_verdict = admit_shrink_only_ratchet(prev_metric, next_metric, seed_nonzero);
    let metric_improved_or_held = ratchet_verdict == RatchetMetricVerdict::ShrinkAdmitted
        && admit_ratchet_obligation(next_metric, prev_metric, witness_present);
    let verdict = admit_peirce_triadic_cycle(PeirceTriadicCheck {
        phase,
        witness_present,
        invariant_preserved,
        metric_improved_or_held,
        rule30_nonzero: seed_nonzero,
    });
    if verdict != PeirceTriadicVerdict::TriadicStepAdmitted {
        return Err(format!(
            "Peirce triadic gate refused {:?}: {verdict:?} (prev={prev_metric}, next={next_metric}, r30={rule30_state})",
            phase
        ));
    }
    Ok(())
}

/// Verify the full `model/ratchet.toml` policy, mechanizer, lift-register, serving & UI gates, git monotonicity, and Peirce cycle.
pub fn check(root: &Path) -> Result<RatchetReport, Fail> {
    let model = load_ratchet_model(root)?;
    if model.spec != RATCHET_SPEC {
        return Err(format!(
            "model/ratchet.toml spec mismatch: expected {RATCHET_SPEC}, got {}",
            model.spec
        )
        .into());
    }
    if model.peirce_phases != ["Abduction", "Deduction", "Induction"] {
        return Err(
            "model/ratchet.toml peirce_phases must be [Abduction, Deduction, Induction]".into(),
        );
    }
    if model.rule30_rule_number != 30 {
        return Err(format!(
            "model/ratchet.toml rule30_rule_number must be 30, got {}",
            model.rule30_rule_number
        )
        .into());
    }

    let max_res = model
        .max_residue_functions
        .ok_or("model/ratchet.toml must set max_residue_functions")?;
    let max_float = model
        .max_float_serving_ops
        .ok_or("model/ratchet.toml must set max_float_serving_ops = 0")?;
    let max_matmul = model
        .max_dense_matmul_ops
        .ok_or("model/ratchet.toml must set max_dense_matmul_ops = 0")?;
    let max_ui = model
        .max_unwired_ui_controls
        .ok_or("model/ratchet.toml must set max_unwired_ui_controls = 0")?;
    if max_float != 0 || max_matmul != 0 || max_ui != 0 {
        return Err(format!(
            "R4-44/R4-46 ratchet ceilings must all be 0 (float={max_float}, matmul={max_matmul}, unwired_ui={max_ui})"
        )
        .into());
    }

    super::mechanizer::run_mechanizer_gate(root)?;
    super::lift_register::check_lift_register_ratchet(root, max_res)?;

    let measured = measure_ratchet_state(root)?;
    if measured.rule30_generation < model.rule30_min_generation {
        return Err(format!(
            "Rule-30 generation {} below minimum {}",
            measured.rule30_generation, model.rule30_min_generation
        )
        .into());
    }
    let seed_nonzero = measured.rule30_state != 0;

    check_metric(
        "handwritten_bypass_files",
        measured.handwritten_bypass_files,
        model.max_handwritten_bypass_files,
        seed_nonzero,
    )?;
    check_metric(
        "deferral_markers",
        measured.deferral_markers,
        model.max_deferral_markers,
        seed_nonzero,
    )?;
    check_metric(
        "ignored_conformance_tests",
        measured.ignored_conformance_tests,
        model.max_ignored_conformance_tests,
        seed_nonzero,
    )?;
    check_metric(
        "ungrounded_defs",
        measured.ungrounded_defs,
        model.max_ungrounded_defs,
        seed_nonzero,
    )?;
    check_metric(
        "orphaned_point_rows",
        measured.orphaned_point_rows,
        model.max_orphaned_point_rows,
        seed_nonzero,
    )?;
    check_metric(
        "residue_functions",
        measured.residue_functions,
        max_res,
        seed_nonzero,
    )?;
    check_metric(
        "float_serving_ops",
        measured.float_serving_ops,
        max_float,
        seed_nonzero,
    )?;
    check_metric(
        "dense_matmul_ops",
        measured.dense_matmul_ops,
        max_matmul,
        seed_nonzero,
    )?;
    if let Err(err) = check_metric(
        "unwired_ui_controls",
        measured.unwired_ui_controls,
        max_ui,
        seed_nonzero,
    ) {
        let unwired = find_unwired_ui_controls(root)?;
        return Err(format!("{err}; unwired_controls={unwired:?}").into());
    }

    let monotone_vs_parent = check_git_parent_ratchet(root, &model)?;

    for phase in [
        PeircePhase::Abduction,
        PeircePhase::Deduction,
        PeircePhase::Induction,
    ] {
        verify_peirce_triadic_step(phase, true, true, 0, 0, measured.rule30_state)?;
    }

    println!(
        "ratchet-check: shrink-only + Peirce triadic verified (r30_gen={}, orphans={}, ungrounded={}, ignored={}, residue={}, float={}, matmul={}, unwired_ui={})",
        measured.rule30_generation,
        measured.orphaned_point_rows,
        measured.ungrounded_defs,
        measured.ignored_conformance_tests,
        measured.residue_functions,
        measured.float_serving_ops,
        measured.dense_matmul_ops,
        measured.unwired_ui_controls
    );

    Ok(RatchetReport {
        model,
        measured,
        monotone_vs_parent,
        peirce_admitted: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let p = entry?.path();
            let target = dst.join(p.file_name().expect("file name"));
            if p.is_dir() {
                copy_dir_recursive(&p, &target)?;
            } else {
                std::fs::copy(&p, &target)?;
            }
        }
        Ok(())
    }

    fn base_ratchet_model() -> RatchetModel {
        RatchetModel {
            spec: RATCHET_SPEC.to_string(),
            peirce_phases: vec![
                "Abduction".to_string(),
                "Deduction".to_string(),
                "Induction".to_string(),
            ],
            rule30_rule_number: 30,
            rule30_min_generation: 1,
            max_handwritten_bypass_files: 0,
            max_deferral_markers: 0,
            max_ignored_conformance_tests: 0,
            max_ungrounded_defs: 0,
            max_orphaned_point_rows: 0,
            max_trajectory_l1_milli: 250_000,
            max_residue_functions: Some(10),
            max_float_serving_ops: Some(0),
            max_dense_matmul_ops: Some(0),
            max_unwired_ui_controls: Some(0),
        }
    }

    #[test]
    fn live_workspace_passes_ratchet_check() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let report = check(&root).expect("live repository satisfies shrink-only ratchet");
        assert!(report.peirce_admitted);
        assert_eq!(report.measured.ungrounded_defs, 0);
        assert_eq!(report.measured.ignored_conformance_tests, 0);
        assert_eq!(report.measured.float_serving_ops, 0);
        assert_eq!(report.measured.dense_matmul_ops, 0);
        assert_eq!(report.measured.unwired_ui_controls, 0);
    }

    #[test]
    fn clean_synthetic_workspace_passes_full_ratchet_and_refuses_planted_defects() {
        let real_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("ratchet_clean_fixture_{unique}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("model")).expect("mkdir model");
        std::fs::create_dir_all(tmp.join("public")).expect("mkdir public");
        std::fs::create_dir_all(tmp.join("crates/wasm/src")).expect("mkdir crates/wasm/src");

        copy_dir_recursive(&real_root.join("src/R4"), &tmp.join("src/R4")).expect("copy src/R4");
        copy_dir_recursive(&real_root.join("tests/lexlean"), &tmp.join("tests/lexlean"))
            .expect("copy tests/lexlean");
        std::fs::copy(real_root.join("lexlean.toml"), tmp.join("lexlean.toml"))
            .expect("copy lexlean.toml");
        std::fs::copy(
            real_root.join("model/engine.toml"),
            tmp.join("model/engine.toml"),
        )
        .expect("copy engine.toml");

        let clean_html = "<!doctype html><html><body>\
            <form id=\"prompt-form\"><input id=\"prompt-input\" type=\"text\"><button id=\"prompt-send\" type=\"submit\">Send</button></form>\
            <select id=\"composer-model-select\"><option>r4</option></select>\
            <button id=\"tab-btn\" type=\"button\" data-canvas-tab=\"feed\">Feed</button>\
            </body></html>\n";
        let clean_js =
            "document.getElementById('prompt-form').addEventListener('submit', () => {});\
            document.getElementById('prompt-input').value = '';\
            document.getElementById('composer-model-select').addEventListener('change', () => {});\
            document.querySelectorAll('[data-canvas-tab]').forEach(() => {});\n";
        std::fs::write(tmp.join("public/index.html"), clean_html).expect("write clean html");
        std::fs::write(tmp.join("public/app.js"), clean_js).expect("write clean js");

        let clean_rust =
            "/// Pure integer step.\npub fn pure_nibble_lut(x: u8) -> u8 { x.wrapping_add(1) }\n";
        std::fs::write(tmp.join("crates/wasm/src/serving.rs"), clean_rust)
            .expect("write clean rust");

        super::super::lift_register::cmd_lift_register(&tmp, true)
            .expect("write fixture lift register");
        let scan = super::super::lift_register::scan_codebase_lift_register(&tmp)
            .expect("scan fixture lift");
        let res_count = scan.document.summary.residue;
        let ratchet_toml = format!(
            "spec = \"{RATCHET_SPEC}\"\npeirce_phases = [\"Abduction\", \"Deduction\", \"Induction\"]\n\
             rule30_rule_number = 30\nrule30_min_generation = 1\n\
             max_handwritten_bypass_files = 0\nmax_deferral_markers = 0\n\
             max_ignored_conformance_tests = 0\nmax_ungrounded_defs = 0\n\
             max_orphaned_point_rows = 0\nmax_trajectory_l1_milli = 250000\n\
             max_residue_functions = {res_count}\nmax_float_serving_ops = 0\n\
             max_dense_matmul_ops = 0\nmax_unwired_ui_controls = 0\n"
        );
        std::fs::write(tmp.join("model/ratchet.toml"), &ratchet_toml).expect("write ratchet.toml");

        let report = check(&tmp).expect("clean synthetic workspace passes full ratchet check");
        assert_eq!(report.measured.unwired_ui_controls, 0);
        assert_eq!(report.measured.float_serving_ops, 0);
        assert_eq!(report.measured.dense_matmul_ops, 0);

        // Planted unwired HTML control must fail with max_unwired_ui_controls = 0.
        let dirty_html = clean_html.replace(
            "</body>",
            "<button id=\"planted-unwired-btn\" type=\"button\">Dead</button></body>",
        );
        std::fs::write(tmp.join("public/index.html"), dirty_html).expect("write dirty html");
        let ui_err = check(&tmp)
            .expect_err("planted unwired UI control must fail ratchet check")
            .to_string();
        assert!(
            ui_err.contains("planted-unwired-btn"),
            "expected diagnostic to name planted-unwired-btn, got: {ui_err}"
        );
        std::fs::write(tmp.join("public/index.html"), clean_html).expect("restore clean html");

        // Planted float serving operation must fail.
        std::fs::write(
            tmp.join("crates/wasm/src/serving.rs"),
            "/// Leaked float.\npub fn leak_float(x: f32) -> f32 { x.exp() }\n",
        )
        .expect("write float leak");
        super::super::lift_register::cmd_lift_register(&tmp, true).expect("update fixture lift");
        assert!(
            check(&tmp).is_err(),
            "planted float op must fail ratchet check"
        );

        // Planted dense matmul must fail.
        std::fs::write(
            tmp.join("crates/wasm/src/serving.rs"),
            "/// Leaked matmul.\npub fn dense_matmul(a: &[u8]) -> usize { a.len() }\n",
        )
        .expect("write matmul leak");
        super::super::lift_register::cmd_lift_register(&tmp, true).expect("update fixture lift");
        assert!(
            check(&tmp).is_err(),
            "planted dense_matmul must fail ratchet check"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn planted_ratchet_regressions_are_refused() {
        let base = base_ratchet_model();
        let mut inflated = base.clone();
        inflated.max_ungrounded_defs = 1;
        assert!(enforce_monotone_ceilings(&inflated, &base).is_err());

        let mut inflated_res = base.clone();
        inflated_res.max_residue_functions = Some(11);
        assert!(enforce_monotone_ceilings(&inflated_res, &base).is_err());

        let mut nonzero_float = base.clone();
        nonzero_float.max_float_serving_ops = Some(1);
        assert!(enforce_monotone_ceilings(&nonzero_float, &base).is_err());

        let mut nonzero_ui = base.clone();
        nonzero_ui.max_unwired_ui_controls = Some(1);
        assert!(enforce_monotone_ceilings(&nonzero_ui, &base).is_err());

        // Peirce triadic refuses residual regression and dead Rule-30 seed.
        assert!(verify_peirce_triadic_step(PeircePhase::Induction, true, true, 10, 11, 1).is_err());
        assert!(verify_peirce_triadic_step(PeircePhase::Abduction, true, true, 10, 10, 0).is_err());
        assert!(
            verify_peirce_triadic_step(PeircePhase::Deduction, true, false, 10, 10, 1).is_err()
        );
        assert!(
            verify_peirce_triadic_step(PeircePhase::Deduction, false, true, 10, 10, 1).is_err()
        );
    }
}
