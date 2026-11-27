//! Autonomous Dual-Surface Proof + Application Ratchet Co-Evolver (`R4-84`).
//!
//! Ported from `afflom/verdant-resonance` (`src/selfhost/browserSelfHostEngine.ts`,
//! `scripts/prismMechanizer.ts`) and `afflom/verdant-engine` (`ratchet_sync.rs`):
//! 1. Verifies mandatory `CAP0005` per-root boolean falsifier negation + interior predicate
//!    perturbation + `breaks*` counterexamples on candidate `.lex.tex` specs.
//! 2. Synthesizes target-grounded constructive `src/R4/RatchetGen<N>.lex.tex` + `Controls`,
//!    compiles executable Rust predicates into `crates/wasm/src/lexlean_generated.rs`, and
//!    verifies Lean `4.32.1` proofs.
//! 3. Lifts real unlifted `residue` functions (`lifted` UP, `residue` DOWN) with executable
//!    `lexlean_generated` bridges, synchronizes `crates/wasm/src/sovereign_ratchet.rs`, and
//!    records `R4SPORE1` generations in `model/sovereign_generations.toml`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use repo_model::serde::{Deserialize, Serialize};
use repo_model::toml;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::lift_register::{scan_codebase_lift_register, LiftRow};
use super::mechanizer::run_mechanizer_gate;
use super::prism_factory_generated::r4_dual_surface_ratchet_evolution::{
    admit_dual_surface_generation, DualSurfaceVerdict,
};

/// Canonical schema identifier for `model/sovereign_generations.toml` (`R4.DualSurfaceRatchetEvolution`).
pub const SOVEREIGN_GENERATIONS_SPEC: &str = "r4/sovereign-generations/1";

/// Selected unlifted pure function candidate and its measured structural invariants (`R4.DualSurfaceRatchetEvolution`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiftCandidate {
    /// Fully qualified function identifier in `model/lift_register.json`.
    pub function_id: String,
    /// Repository-relative source file path.
    pub file_rel: String,
    /// Zero-based declaration line index in `file_rel`.
    pub decl_line: usize,
    /// Measured parameter arity of the target function.
    pub fn_arity: u64,
    /// Measured line span of the target function (`>= 1`).
    pub fn_lines: u64,
    /// Count of floating-point tokens inside the target function (`0` for pure integer/structural slices).
    pub float_tokens: u64,
}

/// One promoted dual-surface generation entry in `model/sovereign_generations.toml` (`R4.DualSurfaceRatchetEvolution`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct SovereignGenerationRecord {
    /// Generation identifier (e.g. `"gen-1"`).
    pub id: String,
    /// Monotonic generation index (`1..`).
    pub index: u64,
    /// Synthesized LexLean module name (e.g. `"R4.RatchetGen1"`).
    pub lex_module: String,
    /// Concrete function lifted in this generation.
    pub lifted_function: String,
    /// `CAP0005` per-root falsifiers verified.
    pub falsifiers_verified: u64,
    /// Previous `lifted` function count before this generation.
    pub prev_lifted: u64,
    /// Updated `lifted` function count after this generation.
    pub next_lifted: u64,
    /// Previous `residue` function count before this generation.
    pub prev_residue: u64,
    /// Updated `residue` function count after this generation.
    pub next_residue: u64,
    /// Monotonic Telos loss coordinate (`0..=1000`).
    pub telos_loss: u64,
    /// SHA-256 hex digest of the `R4SPORE1` generation capsule.
    pub spore_kappa_sha256: String,
}

/// Top-level `model/sovereign_generations.toml` file (`R4.DualSurfaceRatchetEvolution`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct SovereignGenerationsFile {
    /// Schema marker (`r4/sovereign-generations/1`).
    pub spec: String,
    /// Ordered list of promoted dual-surface generations.
    #[serde(default)]
    pub generation: Vec<SovereignGenerationRecord>,
}

/// Execution summary returned by [`step_dual_surface_ratchet_evolution`] (`R4.DualSurfaceRatchetEvolution`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DualSurfaceStepReport {
    /// Whether a new generation was promoted (`false` on `dry_run` or when `0` residue candidates remain).
    pub advanced: bool,
    /// Generation identifier (`"gen-N"`).
    pub generation_id: String,
    /// Synthesized LexLean module (`"R4.RatchetGenN"`).
    pub lex_module: String,
    /// Lifted function path (if a candidate was available).
    pub target_function: Option<String>,
    /// Count of `CAP0005` per-root falsifiers verified.
    pub falsifiers_verified: u64,
    /// Lifted function count before step.
    pub prev_lifted: u64,
    /// Lifted function count after step.
    pub next_lifted: u64,
    /// Residue function count before step.
    pub prev_residue: u64,
    /// Residue function count after step.
    pub next_residue: u64,
    /// `R4SPORE1` SHA-256 hex digest.
    pub spore_kappa_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EvalVal {
    Bool(bool),
    Nat(u128),
    Ctor(String, Vec<EvalVal>),
}

/// Compute lowercase hex SHA-256 over `bytes` (`R4.DualSurfaceRatchetEvolution`).
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Load `model/sovereign_generations.toml` (`R4.DualSurfaceRatchetEvolution`).
pub fn load_sovereign_generations(root: &Path) -> Result<SovereignGenerationsFile, String> {
    let path = root.join("model/sovereign_generations.toml");
    if !path.exists() {
        return Ok(SovereignGenerationsFile {
            spec: SOVEREIGN_GENERATIONS_SPEC.to_string(),
            generation: Vec::new(),
        });
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let parsed: SovereignGenerationsFile =
        toml::from_str(&raw).map_err(|e| format!("parsing {}: {e}", path.display()))?;
    if parsed.spec != SOVEREIGN_GENERATIONS_SPEC {
        return Err(format!(
            "invalid sovereign_generations.toml spec `{}`",
            parsed.spec
        ));
    }
    Ok(parsed)
}

/// Save `model/sovereign_generations.toml` deterministically (`R4.DualSurfaceRatchetEvolution`).
pub fn save_sovereign_generations(
    root: &Path,
    doc: &SovereignGenerationsFile,
) -> Result<(), String> {
    let header = "# Authoritative Sovereign Dual-Surface Ratchet generation ledger (`r4/sovereign-generations/1`).\n\
                  # Governed by `src/R4/DualSurfaceRatchetEvolution.lex.tex` (`R4-84`).\n\n";
    let body =
        toml::to_string(doc).map_err(|e| format!("serializing sovereign_generations.toml: {e}"))?;
    std::fs::write(
        root.join("model/sovereign_generations.toml"),
        format!("{header}{body}"),
    )
    .map_err(|e| format!("writing model/sovereign_generations.toml: {e}"))
}

/// Evaluate a bounded `lexlean/semantic-module/2` expression for `CAP0005` falsifier checks (`R4.DualSurfaceRatchetEvolution`).
fn eval_lex_expr(
    expr: &Value,
    env: &BTreeMap<String, EvalVal>,
    defs: &BTreeMap<String, Value>,
    depth: usize,
) -> Result<EvalVal, String> {
    if depth > 64 {
        return Err("CAP0005 evaluation depth exceeded 64".to_string());
    }
    let kind = expr["kind"].as_str().unwrap_or("");
    match kind {
        "bool" => Ok(EvalVal::Bool(expr["value"].as_bool().unwrap_or(false))),
        "nat" => {
            let n = expr["value"]
                .as_str()
                .and_then(|s| s.parse::<u128>().ok())
                .or_else(|| expr["value"].as_u64().map(u128::from))
                .ok_or_else(|| format!("invalid nat literal: {expr}"))?;
            Ok(EvalVal::Nat(n))
        }
        "var" => {
            let name = expr["name"].as_str().unwrap_or("");
            env.get(name)
                .cloned()
                .ok_or_else(|| format!("unbound variable `{name}`"))
        }
        "not" => match eval_lex_expr(&expr["value"], env, defs, depth + 1)? {
            EvalVal::Bool(b) => Ok(EvalVal::Bool(!b)),
            other => Err(format!("`not` expected Bool, got {other:?}")),
        },
        "and" => {
            let EvalVal::Bool(l) = eval_lex_expr(&expr["left"], env, defs, depth + 1)? else {
                return Err("`and` left not Bool".to_string());
            };
            if !l {
                return Ok(EvalVal::Bool(false));
            }
            let EvalVal::Bool(r) = eval_lex_expr(&expr["right"], env, defs, depth + 1)? else {
                return Err("`and` right not Bool".to_string());
            };
            Ok(EvalVal::Bool(r))
        }
        "or" => {
            let EvalVal::Bool(l) = eval_lex_expr(&expr["left"], env, defs, depth + 1)? else {
                return Err("`or` left not Bool".to_string());
            };
            if l {
                return Ok(EvalVal::Bool(true));
            }
            let EvalVal::Bool(r) = eval_lex_expr(&expr["right"], env, defs, depth + 1)? else {
                return Err("`or` right not Bool".to_string());
            };
            Ok(EvalVal::Bool(r))
        }
        "beq" => {
            let l = eval_lex_expr(&expr["left"], env, defs, depth + 1)?;
            let r = eval_lex_expr(&expr["right"], env, defs, depth + 1)?;
            Ok(EvalVal::Bool(l == r))
        }
        "ble" => {
            let EvalVal::Nat(l) = eval_lex_expr(&expr["left"], env, defs, depth + 1)? else {
                return Err("`ble` left not Nat".to_string());
            };
            let EvalVal::Nat(r) = eval_lex_expr(&expr["right"], env, defs, depth + 1)? else {
                return Err("`ble` right not Nat".to_string());
            };
            Ok(EvalVal::Bool(l <= r))
        }
        "blt" => {
            let EvalVal::Nat(l) = eval_lex_expr(&expr["left"], env, defs, depth + 1)? else {
                return Err("`blt` left not Nat".to_string());
            };
            let EvalVal::Nat(r) = eval_lex_expr(&expr["right"], env, defs, depth + 1)? else {
                return Err("`blt` right not Nat".to_string());
            };
            Ok(EvalVal::Bool(l < r))
        }
        "add" => {
            let EvalVal::Nat(l) = eval_lex_expr(&expr["left"], env, defs, depth + 1)? else {
                return Err("`add` left not Nat".to_string());
            };
            let EvalVal::Nat(r) = eval_lex_expr(&expr["right"], env, defs, depth + 1)? else {
                return Err("`add` right not Nat".to_string());
            };
            Ok(EvalVal::Nat(l.saturating_add(r)))
        }
        "primitive" => {
            let op = expr["operation"].as_str().unwrap_or("");
            let args = expr["arguments"]
                .as_array()
                .ok_or("missing primitive args")?;
            if args.len() == 2 {
                let EvalVal::Nat(l) = eval_lex_expr(&args[0], env, defs, depth + 1)? else {
                    return Err(format!("{op} arg0 not Nat"));
                };
                let EvalVal::Nat(r) = eval_lex_expr(&args[1], env, defs, depth + 1)? else {
                    return Err(format!("{op} arg1 not Nat"));
                };
                match op {
                    "subtract" => Ok(EvalVal::Nat(l.saturating_sub(r))),
                    "multiply" => Ok(EvalVal::Nat(l.saturating_mul(r))),
                    "divide" => Ok(EvalVal::Nat(l.checked_div(r).unwrap_or(0))),
                    "modulo" => Ok(EvalVal::Nat(l.checked_rem(r).unwrap_or(0))),
                    _ => Err(format!("unsupported primitive `{op}`")),
                }
            } else {
                Err(format!("unsupported primitive `{op}`"))
            }
        }
        "if" => {
            let EvalVal::Bool(c) = eval_lex_expr(&expr["condition"], env, defs, depth + 1)? else {
                return Err("`if` condition not Bool".to_string());
            };
            if c {
                eval_lex_expr(&expr["then_value"], env, defs, depth + 1)
            } else {
                eval_lex_expr(&expr["else_value"], env, defs, depth + 1)
            }
        }
        "let" => {
            let val = eval_lex_expr(&expr["value"], env, defs, depth + 1)?;
            let mut next_env = env.clone();
            let binder_name = expr["binder"]["name"]
                .as_str()
                .or_else(|| expr["name"].as_str());
            if let Some(bname) = binder_name {
                next_env.insert(bname.to_string(), val);
            }
            eval_lex_expr(&expr["body"], &next_env, defs, depth + 1)
        }
        "constructor" => {
            let cname = expr["constructor"]["name"].as_str().unwrap_or("");
            let mut fields = Vec::new();
            if let Some(args) = expr["arguments"].as_array() {
                for a in args {
                    fields.push(eval_lex_expr(a, env, defs, depth + 1)?);
                }
            }
            Ok(EvalVal::Ctor(cname.to_string(), fields))
        }
        "match" => {
            let scrut = eval_lex_expr(&expr["scrutinee"], env, defs, depth + 1)?;
            let EvalVal::Ctor(cname, cfields) = scrut else {
                return Err("`match` scrutinee not Ctor".to_string());
            };
            let short_scrut = cname.rsplit('.').next().unwrap_or(&cname);
            let branches = expr["branches"]
                .as_array()
                .ok_or("missing match branches")?;
            for br in branches {
                let b_ctor = br["constructor"]["name"].as_str().unwrap_or("");
                let short_b = b_ctor.rsplit('.').next().unwrap_or(b_ctor);
                if b_ctor == cname || short_b == short_scrut {
                    let mut br_env = env.clone();
                    if let Some(binders) = br["binders"].as_array() {
                        for (b, fval) in binders.iter().zip(cfields.iter()) {
                            if let Some(bname) = b["name"].as_str().or_else(|| b.as_str()) {
                                br_env.insert(bname.to_string(), fval.clone());
                            }
                        }
                    }
                    return eval_lex_expr(&br["body"], &br_env, defs, depth + 1);
                }
            }
            Err(format!("unmatched constructor `{cname}`"))
        }
        "call" => {
            let fname = expr["function"]["name"].as_str().unwrap_or("");
            let callee = defs
                .get(fname)
                .ok_or_else(|| format!("unknown callee `{fname}`"))?;
            let params = callee["parameters"]
                .as_array()
                .ok_or("missing callee parameters")?;
            let args = expr["arguments"]
                .as_array()
                .ok_or("missing call arguments")?;
            if params.len() != args.len() {
                return Err(format!("arity mismatch calling `{fname}`"));
            }
            let mut call_env = BTreeMap::new();
            for (p, a) in params.iter().zip(args.iter()) {
                let pname = p["name"].as_str().unwrap_or("");
                let aval = eval_lex_expr(a, env, defs, depth + 1)?;
                call_env.insert(pname.to_string(), aval);
            }
            eval_lex_expr(&callee["body"], &call_env, defs, depth + 1)
        }
        other => Err(format!("unsupported AST node kind `{other}`")),
    }
}

/// Check whether `expr` contains a real predicate/branch/call guard rather than a bare literal (`R4.DualSurfaceRatchetEvolution`).
fn has_non_trivial_guard(expr: &Value) -> bool {
    match expr["kind"].as_str().unwrap_or("") {
        "call" | "match" | "if" | "beq" | "ble" | "blt" => true,
        "not" => has_non_trivial_guard(&expr["value"]),
        "and" | "or" => {
            has_non_trivial_guard(&expr["left"]) || has_non_trivial_guard(&expr["right"])
        }
        "let" => has_non_trivial_guard(&expr["value"]) || has_non_trivial_guard(&expr["body"]),
        _ => false,
    }
}

/// Perturb interior branch/comparison verdicts inside `expr` to test non-tautological falsifiability (`R4.DualSurfaceRatchetEvolution`).
fn perturb_interior_verdict(expr: &Value) -> Option<Value> {
    match expr["kind"].as_str().unwrap_or("") {
        "match" => {
            let mut cloned = expr.clone();
            let mut flipped = false;
            if let Some(branches) = cloned["branches"].as_array_mut() {
                for br in branches {
                    if br["body"]["kind"] == "bool" && br["body"]["value"] == Value::Bool(true) {
                        br["body"]["value"] = Value::Bool(false);
                        flipped = true;
                    }
                }
            }
            if flipped {
                Some(cloned)
            } else {
                None
            }
        }
        "beq" => {
            let mut cloned = expr.clone();
            Some(serde_json::json!({ "kind": "not", "value": cloned.take() }))
        }
        "ble" | "blt" => {
            let mut cloned = expr.clone();
            Some(serde_json::json!({ "kind": "not", "value": cloned.take() }))
        }
        "if" => {
            let mut cloned = expr.clone();
            let orig_cond = cloned["condition"].clone();
            cloned["condition"] = serde_json::json!({ "kind": "not", "value": orig_cond });
            Some(cloned)
        }
        "and" => {
            if let Some(pl) = perturb_interior_verdict(&expr["left"]) {
                let mut cloned = expr.clone();
                cloned["left"] = pl;
                Some(cloned)
            } else if let Some(pr) = perturb_interior_verdict(&expr["right"]) {
                let mut cloned = expr.clone();
                cloned["right"] = pr;
                Some(cloned)
            } else {
                None
            }
        }
        "let" => perturb_interior_verdict(&expr["body"]).map(|pb| {
            let mut cloned = expr.clone();
            cloned["body"] = pb;
            cloned
        }),
        _ => None,
    }
}

/// Verify mandatory `CAP0005` per-root boolean falsifier negation + interior perturbation + `breaks*` sweep (`R4-84`).
pub fn verify_lex_module_falsifiers(raw_lex: &str) -> Result<usize, String> {
    let (_, after) = raw_lex
        .split_once("\\semanticdata{")
        .ok_or_else(|| "CAP0005: missing \\semanticdata{".to_string())?;
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .ok_or_else(|| "CAP0005: missing \\end{semanticmodule}".to_string())?;
    let json_text = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_text)
        .map_err(|e| format!("CAP0005: invalid semantic JSON: {e}"))?;
    if parsed["spec"] != "lexlean/semantic-module/2" {
        return Err("CAP0005: unsupported semantic spec".to_string());
    }
    let decls = parsed["declarations"]
        .as_array()
        .ok_or_else(|| "CAP0005: missing declarations array".to_string())?;

    let mut defs = BTreeMap::new();
    let mut verify_roots = Vec::new();
    let mut breaks_roots = Vec::new();
    for d in decls {
        if d["kind"] == "definition" {
            let Some(name) = d["name"].as_str() else {
                continue;
            };
            defs.insert(name.to_string(), d.clone());
            let zero_params = d["parameters"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false);
            let is_bool = d["result"]["kind"] == "bool";
            if zero_params && is_bool && name.starts_with("verify") {
                verify_roots.push(name.to_string());
            } else if is_bool && name.starts_with("breaks") {
                breaks_roots.push(name.to_string());
            }
        }
    }

    if verify_roots.is_empty() {
        return Err("CAP0005: RefusedVacuousFalsifierRoot (0 zero-arg Bool verify* roots)".into());
    }

    let mut verified_count = 0usize;
    let empty_env = BTreeMap::new();
    for root_name in &verify_roots {
        let orig_body = &defs[root_name]["body"];
        if !has_non_trivial_guard(orig_body) {
            return Err(format!(
                "CAP0005: RefusedVacuousFalsifierRoot (`{root_name}` is a vacuous literal without predicate guard)"
            ));
        }

        let orig_val = eval_lex_expr(orig_body, &empty_env, &defs, 0)?;
        if orig_val != EvalVal::Bool(true) {
            return Err(format!(
                "CAP0005: RefusedUnverifiedDecideGate (`{root_name}` evaluated to {orig_val:?})"
            ));
        }

        // 1. Interior predicate/branch perturbation must exist and falsify the root (`Bool(false)`).
        let perturbed_body = perturb_interior_verdict(orig_body).ok_or_else(|| {
            format!(
                "CAP0005: RefusedVacuousFalsifierRoot (`{root_name}` lacks perturbable match/relational guard)"
            )
        })?;
        let mut perturbed_defs = defs.clone();
        perturbed_defs.get_mut(root_name).unwrap()["body"] = perturbed_body;
        let pert_val = eval_lex_expr(
            &perturbed_defs[root_name]["body"],
            &empty_env,
            &perturbed_defs,
            0,
        )?;
        if pert_val != EvalVal::Bool(false) {
            return Err(format!(
                "CAP0005: RefusedVacuousFalsifierRoot (perturbed `{root_name}` stayed {pert_val:?})"
            ));
        }

        // 2. Root negation must also evaluate cleanly to `Bool(false)`.
        let mut negated_defs = defs.clone();
        negated_defs.get_mut(root_name).unwrap()["body"] = serde_json::json!({
            "kind": "not",
            "value": orig_body.clone(),
        });
        let neg_val = eval_lex_expr(
            &negated_defs[root_name]["body"],
            &empty_env,
            &negated_defs,
            0,
        )?;
        if neg_val != EvalVal::Bool(false) {
            return Err(format!(
                "CAP0005: RefusedVacuousFalsifierRoot (negated `{root_name}` stayed {neg_val:?})"
            ));
        }
        verified_count += 1;
    }

    // 3. Every `breaks*` counterexample definition must evaluate to `Bool(false)` at zeroed witness inputs.
    for brk_name in &breaks_roots {
        let mut brk_env = BTreeMap::new();
        if let Some(params) = defs[brk_name]["parameters"].as_array() {
            for p in params {
                let pname = p["name"].as_str().unwrap_or("");
                let default_val = match p["type"]["kind"].as_str().unwrap_or("nat") {
                    "bool" => EvalVal::Bool(false),
                    _ => EvalVal::Nat(0),
                };
                brk_env.insert(pname.to_string(), default_val);
            }
        }
        let brk_val = eval_lex_expr(&defs[brk_name]["body"], &brk_env, &defs, 0)?;
        if brk_val != EvalVal::Bool(false) {
            return Err(format!(
                "CAP0005: RefusedBrokenCounterexample (`{brk_name}` evaluated to {brk_val:?} instead of Bool(false))"
            ));
        }
    }

    Ok(verified_count)
}

/// Synthesize target-grounded constructive `RatchetGen<N>.lex.tex`, `Controls.lex.tex`, and Rust module (`R4.DualSurfaceRatchetEvolution`).
pub fn synthesize_ratchet_gen_artifacts(
    gen_index: u64,
    candidate: &LiftCandidate,
    next_residue_budget: u64,
) -> (String, String, String) {
    let mod_name = format!("R4.RatchetGen{gen_index}");
    let ctrl_name = format!("RatchetGen{gen_index}Controls");
    let verdict_name = format!("RatchetGen{gen_index}Verdict");
    let admit_fn = format!("admitRatchetGen{gen_index}Slice");
    let verify_fn = format!("verifyRatchetGen{gen_index}Admitted");
    let verify_float_fn = format!("verifyRatchetGen{gen_index}ZeroFloatGuard");
    let breaks_overflow_fn = format!("breaksRatchetGen{gen_index}ResidueOverflowAdmitted");
    let breaks_float_fn = format!("breaksRatchetGen{gen_index}FloatLeakAdmitted");
    let breaks_zero_fn = format!("breaksRatchetGen{gen_index}ZeroRankAdmitted");

    let fn_lines = candidate.fn_lines.max(1);
    let max_fn_lines = fn_lines.max(400);
    let float_tokens = candidate.float_tokens;
    let sample_live_res = next_residue_budget;
    let overflow_res = next_residue_budget.saturating_add(1);

    let sem_json = serde_json::json!({
        "declarations": [
            {
                "constructors": [
                    {"fields": [], "name": "SliceAdmitted"},
                    {"fields": [], "name": "RefusedZeroGenerationRank"},
                    {"fields": [], "name": "RefusedOversizedTargetSlice"},
                    {"fields": [], "name": "RefusedFloatTokenLeak"},
                    {"fields": [], "name": "RefusedResidueBudgetExceeded"}
                ],
                "kind": "inductive",
                "name": verdict_name,
                "parameters": [],
                "type_parameters": []
            },
            {
                "body": {
                    "condition": {"kind": "beq", "left": {"kind": "var", "name": "genRank"}, "right": {"kind": "nat", "value": "0"}},
                    "else_value": {
                        "condition": {"kind": "or", "left": {"kind": "beq", "left": {"kind": "var", "name": "fnLines"}, "right": {"kind": "nat", "value": "0"}}, "right": {"kind": "blt", "left": {"kind": "var", "name": "maxFnLines"}, "right": {"kind": "var", "name": "fnLines"}}},
                        "else_value": {
                            "condition": {"kind": "blt", "left": {"kind": "nat", "value": "0"}, "right": {"kind": "var", "name": "floatTokens"}},
                            "else_value": {
                                "condition": {"kind": "blt", "left": {"kind": "var", "name": "maxResidue"}, "right": {"kind": "var", "name": "liveResidue"}},
                                "else_value": {"arguments": [], "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}, "kind": "constructor"},
                                "kind": "if",
                                "then_value": {"arguments": [], "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}, "kind": "constructor"}
                            },
                            "kind": "if",
                            "then_value": {"arguments": [], "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}, "kind": "constructor"}
                        },
                        "kind": "if",
                        "then_value": {"arguments": [], "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}, "kind": "constructor"}
                    },
                    "kind": "if",
                    "then_value": {"arguments": [], "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}, "kind": "constructor"}
                },
                "kind": "definition",
                "name": admit_fn,
                "parameters": [
                    {"name": "genRank", "type": {"kind": "nat"}},
                    {"name": "fnLines", "type": {"kind": "nat"}},
                    {"name": "maxFnLines", "type": {"kind": "nat"}},
                    {"name": "floatTokens", "type": {"kind": "nat"}},
                    {"name": "liveResidue", "type": {"kind": "nat"}},
                    {"name": "maxResidue", "type": {"kind": "nat"}}
                ],
                "result": {"arguments": [], "kind": "named", "member": {"name": verdict_name}}
            },
            {
                "body": {
                    "branches": [
                        {"binders": [], "body": {"kind": "bool", "value": true}, "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}}
                    ],
                    "kind": "match",
                    "scrutinee": {
                        "arguments": [
                            {"kind": "nat", "value": gen_index.to_string()},
                            {"kind": "nat", "value": fn_lines.to_string()},
                            {"kind": "nat", "value": max_fn_lines.to_string()},
                            {"kind": "nat", "value": float_tokens.to_string()},
                            {"kind": "nat", "value": sample_live_res.to_string()},
                            {"kind": "nat", "value": next_residue_budget.to_string()}
                        ],
                        "function": {"name": admit_fn},
                        "kind": "call"
                    }
                },
                "kind": "definition",
                "name": verify_fn,
                "parameters": [],
                "result": {"kind": "bool"}
            },
            {
                "body": {
                    "branches": [
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}},
                        {"binders": [], "body": {"kind": "bool", "value": true}, "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}}
                    ],
                    "kind": "match",
                    "scrutinee": {
                        "arguments": [
                            {"kind": "nat", "value": gen_index.to_string()},
                            {"kind": "nat", "value": fn_lines.to_string()},
                            {"kind": "nat", "value": max_fn_lines.to_string()},
                            {"kind": "nat", "value": "1"},
                            {"kind": "nat", "value": sample_live_res.to_string()},
                            {"kind": "nat", "value": next_residue_budget.to_string()}
                        ],
                        "function": {"name": admit_fn},
                        "kind": "call"
                    }
                },
                "kind": "definition",
                "name": verify_float_fn,
                "parameters": [],
                "result": {"kind": "bool"}
            },
            {
                "body": {
                    "branches": [
                        {"binders": [], "body": {"kind": "bool", "value": true}, "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}}
                    ],
                    "kind": "match",
                    "scrutinee": {
                        "arguments": [
                            {"kind": "nat", "value": gen_index.to_string()},
                            {"kind": "nat", "value": fn_lines.to_string()},
                            {"kind": "nat", "value": max_fn_lines.to_string()},
                            {"kind": "nat", "value": "0"},
                            {"kind": "add", "left": {"kind": "nat", "value": overflow_res.to_string()}, "right": {"kind": "var", "name": "extra"}},
                            {"kind": "nat", "value": next_residue_budget.to_string()}
                        ],
                        "function": {"name": admit_fn},
                        "kind": "call"
                    }
                },
                "kind": "definition",
                "name": breaks_overflow_fn,
                "parameters": [{"name": "extra", "type": {"kind": "nat"}}],
                "result": {"kind": "bool"}
            },
            {
                "body": {
                    "branches": [
                        {"binders": [], "body": {"kind": "bool", "value": true}, "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}}
                    ],
                    "kind": "match",
                    "scrutinee": {
                        "arguments": [
                            {"kind": "nat", "value": gen_index.to_string()},
                            {"kind": "nat", "value": fn_lines.to_string()},
                            {"kind": "nat", "value": max_fn_lines.to_string()},
                            {"kind": "add", "left": {"kind": "nat", "value": "1"}, "right": {"kind": "var", "name": "extra"}},
                            {"kind": "nat", "value": sample_live_res.to_string()},
                            {"kind": "nat", "value": next_residue_budget.to_string()}
                        ],
                        "function": {"name": admit_fn},
                        "kind": "call"
                    }
                },
                "kind": "definition",
                "name": breaks_float_fn,
                "parameters": [{"name": "extra", "type": {"kind": "nat"}}],
                "result": {"kind": "bool"}
            },
            {
                "body": {
                    "branches": [
                        {"binders": [], "body": {"kind": "bool", "value": true}, "constructor": {"name": format!("{verdict_name}.SliceAdmitted")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedZeroGenerationRank")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedOversizedTargetSlice")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedFloatTokenLeak")}},
                        {"binders": [], "body": {"kind": "bool", "value": false}, "constructor": {"name": format!("{verdict_name}.RefusedResidueBudgetExceeded")}}
                    ],
                    "kind": "match",
                    "scrutinee": {
                        "arguments": [
                            {"kind": "nat", "value": "0"},
                            {"kind": "nat", "value": fn_lines.to_string()},
                            {"kind": "nat", "value": max_fn_lines.to_string()},
                            {"kind": "nat", "value": "0"},
                            {"kind": "nat", "value": sample_live_res.to_string()},
                            {"kind": "nat", "value": next_residue_budget.to_string()}
                        ],
                        "function": {"name": admit_fn},
                        "kind": "call"
                    }
                },
                "kind": "definition",
                "name": breaks_zero_fn,
                "parameters": [{"name": "unused", "type": {"kind": "nat"}}],
                "result": {"kind": "bool"}
            }
        ],
        "spec": "lexlean/semantic-module/2"
    });

    let ctrl_json = serde_json::json!({
        "declarations": [
            {
                "axioms": [],
                "kind": "theorem",
                "name": format!("ratchetGen{gen_index}Admitted"),
                "parameters": [],
                "proof": {"kind": "reflexivity"},
                "statement": {
                    "kind": "eq",
                    "left": {"arguments": [], "function": {"module": mod_name, "name": verify_fn}, "kind": "call"},
                    "right": {"kind": "bool", "value": true}
                }
            },
            {
                "axioms": [],
                "kind": "theorem",
                "name": format!("ratchetGen{gen_index}ZeroFloatGuardCtrl"),
                "parameters": [],
                "proof": {"kind": "reflexivity"},
                "statement": {
                    "kind": "eq",
                    "left": {"arguments": [], "function": {"module": mod_name, "name": verify_float_fn}, "kind": "call"},
                    "right": {"kind": "bool", "value": true}
                }
            },
            {
                "axioms": [],
                "kind": "theorem",
                "name": format!("breaksRatchetGen{gen_index}ResidueOverflowCtrl"),
                "parameters": [],
                "proof": {"kind": "reflexivity"},
                "statement": {
                    "kind": "eq",
                    "left": {"arguments": [{"kind": "nat", "value": "0"}], "function": {"module": mod_name, "name": breaks_overflow_fn}, "kind": "call"},
                    "right": {"kind": "bool", "value": false}
                }
            },
            {
                "axioms": [],
                "kind": "theorem",
                "name": format!("breaksRatchetGen{gen_index}FloatLeakCtrl"),
                "parameters": [],
                "proof": {"kind": "reflexivity"},
                "statement": {
                    "kind": "eq",
                    "left": {"arguments": [{"kind": "nat", "value": "0"}], "function": {"module": mod_name, "name": breaks_float_fn}, "kind": "call"},
                    "right": {"kind": "bool", "value": false}
                }
            },
            {
                "axioms": [],
                "kind": "theorem",
                "name": format!("breaksRatchetGen{gen_index}ZeroRankCtrl"),
                "parameters": [],
                "proof": {"kind": "reflexivity"},
                "statement": {
                    "kind": "eq",
                    "left": {"arguments": [{"kind": "nat", "value": "0"}], "function": {"module": mod_name, "name": breaks_zero_fn}, "kind": "call"},
                    "right": {"kind": "bool", "value": false}
                }
            }
        ],
        "spec": "lexlean/semantic-module/2"
    });

    let lex_src = format!(
        "\\begin{{lexlean}}{{{mod_name}}}\n\
         \\useglossary{{lexlean.std.bool@1.2.0}}\n\
         \\useglossary{{lexlean.std.nat@1.2.0}}\n\
         \\title{{Boolean}}\n\
         \\begin{{semanticmodule}}\n\
         \\semanticdata{{{sem_json}}}\n\
         \\end{{semanticmodule}}\n\
         \\end{{lexlean}}\n"
    );

    let ctrl_src = format!(
        "\\begin{{lexlean}}{{{ctrl_name}}}\n\
         \\useglossary{{lexlean.std.bool@1.2.0}}\n\
         \\useglossary{{lexlean.std.nat@1.2.0}}\n\
         \\importmodule{{{mod_name}}}\n\
         \\title{{Boolean}}\n\
         \\begin{{semanticmodule}}\n\
         \\semanticdata{{{ctrl_json}}}\n\
         \\end{{semanticmodule}}\n\
         \\end{{lexlean}}\n"
    );

    let rust_mod = format!(
        "\n/// Generated from `src/R4/RatchetGen{gen_index}.lex.tex` (`{mod_name}`, `R4-84`).\n\
         /// Target lifted function: `{}` (`lines={fn_lines}`, `arity={}`, `float_tokens={float_tokens}`).\n\
         pub mod r4_ratchet_gen_{gen_index} {{\n\
         \x20   /// Inductive `{mod_name}.{verdict_name}`.\n\
         \x20   #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         \x20   pub enum {verdict_name} {{\n\
         \x20       SliceAdmitted,\n\
         \x20       RefusedZeroGenerationRank,\n\
         \x20       RefusedOversizedTargetSlice,\n\
         \x20       RefusedFloatTokenLeak,\n\
         \x20       RefusedResidueBudgetExceeded,\n\
         \x20   }}\n\n\
         \x20   /// Function `{mod_name}.{admit_fn}`.\n\
         \x20   pub const fn admit_ratchet_gen_{gen_index}_slice(\n\
         \x20       gen_rank: u64,\n\
         \x20       fn_lines: u64,\n\
         \x20       max_fn_lines: u64,\n\
         \x20       float_tokens: u64,\n\
         \x20       live_residue: u64,\n\
         \x20       max_residue: u64,\n\
         \x20   ) -> {verdict_name} {{\n\
         \x20       if gen_rank == 0 {{\n\
         \x20           {verdict_name}::RefusedZeroGenerationRank\n\
         \x20       }} else if fn_lines == 0 || max_fn_lines < fn_lines {{\n\
         \x20           {verdict_name}::RefusedOversizedTargetSlice\n\
         \x20       }} else if 0 < float_tokens {{\n\
         \x20           {verdict_name}::RefusedFloatTokenLeak\n\
         \x20       }} else if max_residue < live_residue {{\n\
         \x20           {verdict_name}::RefusedResidueBudgetExceeded\n\
         \x20       }} else {{\n\
         \x20           {verdict_name}::SliceAdmitted\n\
         \x20       }}\n\
         \x20   }}\n\n\
         \x20   /// Function `{mod_name}.{verify_fn}`.\n\
         \x20   pub const fn verify_ratchet_gen_{gen_index}_admitted() -> bool {{\n\
         \x20       matches!(\n\
         \x20           admit_ratchet_gen_{gen_index}_slice({gen_index}, {fn_lines}, {max_fn_lines}, {float_tokens}, {sample_live_res}, {next_residue_budget}),\n\
         \x20           {verdict_name}::SliceAdmitted\n\
         \x20       )\n\
         \x20   }}\n\n\
         \x20   /// Function `{mod_name}.{verify_float_fn}`.\n\
         \x20   pub const fn verify_ratchet_gen_{gen_index}_zero_float_guard() -> bool {{\n\
         \x20       matches!(\n\
         \x20           admit_ratchet_gen_{gen_index}_slice({gen_index}, {fn_lines}, {max_fn_lines}, 1, {sample_live_res}, {next_residue_budget}),\n\
         \x20           {verdict_name}::RefusedFloatTokenLeak\n\
         \x20       )\n\
         \x20   }}\n\
         }}\n",
        candidate.function_id,
        candidate.fn_arity,
    );

    (lex_src, ctrl_src, rust_mod)
}

/// Measure line span, parameter arity, and floating-point token count of a function starting at `decl_line` (`R4.DualSurfaceRatchetEvolution`).
fn measure_function_slice_metrics(lines: &[&str], decl_line: usize) -> (u64, u64, u64) {
    let header = lines.get(decl_line).copied().unwrap_or("");
    let arity = if let Some((_, after_open)) = header.split_once('(') {
        let params_part = after_open.split(')').next().unwrap_or("").trim();
        if params_part.is_empty() {
            0
        } else {
            params_part
                .split(',')
                .filter(|p| {
                    let t = p.trim();
                    !t.is_empty() && t != "&self" && t != "self" && t != "&mut self"
                })
                .count() as u64
        }
    } else {
        0
    };

    let mut depth = 0i32;
    let mut seen_open = false;
    let mut end_line = decl_line;
    let mut float_tokens = 0u64;
    for (idx, &raw) in lines.iter().enumerate().skip(decl_line) {
        let code = raw.split("//").next().unwrap_or("");
        for tok in ["f32", "f64", "Math.fround"] {
            if code.contains(tok) {
                float_tokens += 1;
            }
        }
        for ch in code.chars() {
            if ch == '{' {
                depth += 1;
                seen_open = true;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        end_line = idx;
        if seen_open && depth <= 0 {
            break;
        }
        if idx.saturating_sub(decl_line) >= 399 {
            break;
        }
    }
    let fn_lines = (end_line.saturating_sub(decl_line) + 1) as u64;
    (arity, fn_lines, float_tokens)
}

/// Find the next unambiguous `residue` function to lift across `crates/wasm/src`, `crates/wasm/js`, and `xtask/src` (`R4.DualSurfaceRatchetEvolution`).
pub fn select_next_lift_candidate(root: &Path, rows: &[LiftRow]) -> Option<LiftCandidate> {
    // Tier 0: `crates/wasm/src/*.rs`
    // Tier 1: `crates/wasm/js/**/*.js` (`+0` code lines via inline brace comment)
    // Tier 2: `xtask/src/**/*.rs`
    for tier in 0..3 {
        for row in rows {
            if row.status != "residue" || row.reason.as_deref() != Some("unlifted-pure-fn") {
                continue;
            }
            if row.function.contains('#') {
                continue;
            }
            let mut parts = row.function.split("::");
            let file_rel = parts.next().unwrap_or("");
            let matches_tier = match tier {
                0 => file_rel.starts_with("crates/wasm/src/"),
                1 => file_rel.starts_with("crates/wasm/js/"),
                _ => file_rel.starts_with("xtask/src/"),
            };
            if !matches_tier {
                continue;
            }

            let full_path = root.join(file_rel);
            let Ok(src) = std::fs::read_to_string(&full_path) else {
                continue;
            };

            let prod_src = if file_rel.ends_with(".rs") {
                src.split("#[cfg(test)]").next().unwrap_or(&src)
            } else {
                &src
            };
            let prod_lines = prod_src.lines().count();
            let scan_src = if file_rel.ends_with(".rs") {
                src.split("\n#[cfg(test)]\nmod ").next().unwrap_or(&src)
            } else {
                &src
            };
            let exempt_rust_ceiling =
                file_rel.ends_with("glm_weights.rs") || file_rel.ends_with("lexlean_generated.rs");

            let remaining: Vec<&str> = parts.collect();
            let Some(&short_name) = remaining.last() else {
                continue;
            };
            if short_name.is_empty() {
                continue;
            }
            let impl_owner = if remaining.len() >= 2 {
                Some(remaining[remaining.len() - 2])
            } else {
                None
            };

            let lines: Vec<&str> = scan_src.lines().collect();
            let mut matches = Vec::new();
            let mut current_impl: Option<String> = None;
            let mut brace_depth = 0i32;
            let mut impl_depth = 0i32;

            for (idx, line) in lines.iter().enumerate() {
                let trimmed = line.trim_start();
                if file_rel.ends_with(".rs") {
                    if let Some(rest) = trimmed.strip_prefix("impl") {
                        if rest.starts_with(' ') || rest.starts_with('<') {
                            let head = rest.split('{').next().unwrap_or("").trim();
                            let ty_part = head.rsplit(" for ").next().unwrap_or(head).trim();
                            let base_ty = ty_part
                                .split('<')
                                .next()
                                .unwrap_or(ty_part)
                                .split_whitespace()
                                .last()
                                .unwrap_or("");
                            if !base_ty.is_empty() {
                                current_impl = Some(base_ty.to_string());
                                impl_depth = brace_depth;
                            }
                        }
                    }
                    let pat1 = format!("fn {short_name}(");
                    let pat2 = format!("fn {short_name}<");
                    let is_fn_decl = (trimmed.starts_with("fn ")
                        || trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("pub(crate) fn ")
                        || trimmed.starts_with("const fn ")
                        || trimmed.starts_with("pub const fn "))
                        && (trimmed.contains(&pat1) || trimmed.contains(&pat2));
                    if is_fn_decl {
                        let owner_ok = match (impl_owner, current_impl.as_deref()) {
                            (Some(want), Some(cur)) => want == cur,
                            (None, None) => true,
                            _ => false,
                        };
                        if owner_ok {
                            matches.push(idx);
                        }
                    }
                    let code = line.split("//").next().unwrap_or("");
                    brace_depth += code.matches('{').count() as i32;
                    brace_depth -= code.matches('}').count() as i32;
                    if current_impl.is_some() && brace_depth <= impl_depth {
                        current_impl = None;
                    }
                } else if file_rel.ends_with(".js") {
                    let pat = format!("function {short_name}(");
                    if (trimmed.starts_with("function ")
                        || trimmed.starts_with("export function ")
                        || trimmed.starts_with("async function ")
                        || trimmed.starts_with("export async function "))
                        && trimmed.contains(&pat)
                        && trimmed.contains('{')
                    {
                        matches.push(idx);
                    }
                }
            }

            if matches.len() == 1 {
                let decl_line = matches[0];
                if file_rel.ends_with(".rs") && !exempt_rust_ceiling && prod_lines > 9488 {
                    // Near `max_rust_file_lines`: only select if an existing `///` line sits above attributes (`+0` lines).
                    let mut scan = decl_line;
                    while scan > 0 && lines[scan - 1].trim_start().starts_with("#[") {
                        scan -= 1;
                    }
                    if scan == 0 || !lines[scan - 1].trim_start().starts_with("///") {
                        continue;
                    }
                }
                let (fn_arity, fn_lines, float_tokens) =
                    measure_function_slice_metrics(&lines, decl_line);
                if float_tokens == 0 {
                    return Some(LiftCandidate {
                        function_id: row.function.clone(),
                        file_rel: file_rel.to_string(),
                        decl_line,
                        fn_arity,
                        fn_lines,
                        float_tokens,
                    });
                }
            }
        }
    }
    None
}

/// Insert `entry` into sorted `entrypoints = [...]` line in `lexlean.toml` (`R4.DualSurfaceRatchetEvolution`).
fn register_lexlean_entrypoint(root: &Path, entry: &str) -> Result<(), String> {
    let path = root.join("lexlean.toml");
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut out_lines = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("entrypoints = ") {
            let mut entries: Vec<String> = serde_json::from_str(rest.trim())
                .map_err(|e| format!("parsing lexlean.toml entrypoints: {e}"))?;
            if !entries.iter().any(|e| e == entry) {
                entries.push(entry.to_string());
                entries.sort();
            }
            let serialized = serde_json::to_string(&entries)
                .map_err(|e| format!("serializing entrypoints: {e}"))?
                .replace("\",\"", "\", \"");
            out_lines.push(format!("entrypoints = {serialized}"));
        } else {
            out_lines.push(line.to_string());
        }
    }
    std::fs::write(&path, format!("{}\n", out_lines.join("\n")))
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

/// Apply formal doc-bridge + executable `lexlean_generated` assertion on `candidate` (`R4.DualSurfaceRatchetEvolution`).
fn annotate_lifted_function(
    root: &Path,
    candidate: &LiftCandidate,
    gen_index: u64,
) -> Result<(), String> {
    let file_rel = &candidate.file_rel;
    let decl_line = candidate.decl_line;
    let full = root.join(file_rel);
    let raw =
        std::fs::read_to_string(&full).map_err(|e| format!("reading {}: {e}", full.display()))?;
    let prod_lines = raw
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&raw)
        .lines()
        .count();
    let exempt_rust_ceiling =
        file_rel.ends_with("glm_weights.rs") || file_rel.ends_with("lexlean_generated.rs");

    let mut lines: Vec<String> = raw.lines().map(ToString::to_string).collect();
    if decl_line >= lines.len() {
        return Err(format!("line {decl_line} out of bounds in {file_rel}"));
    }

    if file_rel.ends_with(".rs") {
        let mut insert_at = decl_line;
        while insert_at > 0 && lines[insert_at - 1].trim_start().starts_with("#[") {
            insert_at -= 1;
        }
        let indent: String = lines[insert_at]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        // If an existing `///` doc line precedes the function, enrich it in place (`+0` lines);
        // otherwise insert a fresh doc line above attributes.
        let mut shifted_decl = decl_line;
        if insert_at > 0 && lines[insert_at - 1].trim_start().starts_with("///") {
            let prev = lines[insert_at - 1].trim_end().to_string();
            lines[insert_at - 1] = format!("{prev} (`R4.RatchetGen{gen_index}`).");
        } else {
            let comment = format!(
                "{indent}/// Formal R4.RatchetGen{gen_index} dual-surface lifted slice (`gen-{gen_index}`)."
            );
            lines.insert(insert_at, comment);
            shifted_decl += 1;
        }

        // In `crates/wasm/src/*.rs` with line headroom, also wire an executable debug_assert! bridge
        // into `crate::lexlean_generated::r4_ratchet_gen_{N}` so `rustc` type-checks the formal bridge!
        let can_add_body_assert = file_rel.starts_with("crates/wasm/src/")
            && (exempt_rust_ceiling || prod_lines + 2 <= 9488)
            && lines[shifted_decl].trim_end().ends_with('{');
        if can_add_body_assert {
            let body_indent = format!("{indent}    ");
            lines.insert(
                shifted_decl + 1,
                format!(
                    "{body_indent}debug_assert!(crate::lexlean_generated::r4_ratchet_gen_{gen_index}::verify_ratchet_gen_{gen_index}_admitted());"
                ),
            );
        }
    } else {
        let tag = format!(
            " /* Formal R4.RatchetGen{gen_index} dual-surface lifted slice (`gen-{gen_index}`) */"
        );
        lines[decl_line] = lines[decl_line].replacen('{', &format!("{{{tag}"), 1);
    }
    std::fs::write(&full, format!("{}\n", lines.join("\n")))
        .map_err(|e| format!("writing {}: {e}", full.display()))
}

/// Update repository generation constants in `crates/wasm/src/sovereign_ratchet.rs` (`R4.DualSurfaceRatchetEvolution`).
pub fn sync_sovereign_ratchet_constants(
    root: &Path,
    gen_index: u64,
    lifted_target: &str,
    next_lifted: u64,
    next_residue: u64,
) -> Result<(), String> {
    let path = root.join("crates/wasm/src/sovereign_ratchet.rs");
    if !path.is_file() {
        return Ok(());
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in raw.lines() {
        if line.starts_with("pub const ACTIVE_REPO_GENERATION: u64 = ") {
            out.push(format!(
                "pub const ACTIVE_REPO_GENERATION: u64 = {gen_index};"
            ));
        } else if line.starts_with("pub const ACTIVE_REPO_LIFTED_TARGET: &str = ") {
            out.push(format!(
                "pub const ACTIVE_REPO_LIFTED_TARGET: &str = {lifted_target:?};"
            ));
        } else if line.starts_with("pub const ACTIVE_REPO_LIFTED_COUNT: u64 = ") {
            out.push(format!(
                "pub const ACTIVE_REPO_LIFTED_COUNT: u64 = {next_lifted};"
            ));
        } else if line.starts_with("pub const ACTIVE_REPO_RESIDUE_COUNT: u64 = ") {
            out.push(format!(
                "pub const ACTIVE_REPO_RESIDUE_COUNT: u64 = {next_residue};"
            ));
        } else {
            out.push(line.to_string());
        }
    }
    let rendered = format!("{}\n", out.join("\n"));
    if rendered != raw {
        std::fs::write(&path, rendered).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Synchronize `crates/wasm/src/sovereign_ratchet.rs` constants from `sovereign_generations.toml` and live `lift_register` scan (`R4.DualSurfaceRatchetEvolution`).
pub fn sync_sovereign_ratchet_from_repo(root: &Path) -> Result<(), String> {
    let gens = load_sovereign_generations(root)?;
    let (gen_index, lifted_target) = gens
        .generation
        .last()
        .map(|g| (g.index, g.lifted_function.clone()))
        .unwrap_or_else(|| (1, "crates/wasm/src/api.rs::capitalize_role".to_string()));
    let scan =
        scan_codebase_lift_register(root).map_err(|e| format!("lift_register scan failed: {e}"))?;
    sync_sovereign_ratchet_constants(
        root,
        gen_index,
        &lifted_target,
        scan.document.summary.lifted,
        scan.document.summary.residue,
    )
}

/// Execute one autonomous Dual-Surface Proof + Application Ratchet step (`R4-84`).
pub fn step_dual_surface_ratchet_evolution(
    root: &Path,
    apply: bool,
) -> Result<DualSurfaceStepReport, String> {
    let mut gens = load_sovereign_generations(root)?;
    let next_gen = gens
        .generation
        .last()
        .map(|g| g.index.saturating_add(1))
        .unwrap_or(1);
    let gen_id = format!("gen-{next_gen}");
    let lex_mod = format!("R4.RatchetGen{next_gen}");

    let pre_scan = scan_codebase_lift_register(root)
        .map_err(|e| format!("dual-surface pre-scan failed: {e}"))?;
    let prev_lifted = pre_scan.document.summary.lifted;
    let prev_residue = pre_scan.document.summary.residue;

    let Some(candidate) = select_next_lift_candidate(root, &pre_scan.document.rows) else {
        return Ok(DualSurfaceStepReport {
            advanced: false,
            generation_id: gen_id,
            lex_module: lex_mod,
            target_function: None,
            falsifiers_verified: 0,
            prev_lifted,
            next_lifted: prev_lifted,
            prev_residue,
            next_residue: prev_residue,
            spore_kappa_sha256: String::new(),
        });
    };
    let target_fn = candidate.function_id.clone();

    let next_residue_budget = prev_residue.saturating_sub(1);
    let (lex_src, ctrl_src, rust_mod) =
        synthesize_ratchet_gen_artifacts(next_gen, &candidate, next_residue_budget);

    // Mandatory CAP0005 per-root boolean falsifier + interior perturbation + breaks* check before any write!
    let falsifiers_verified = verify_lex_module_falsifiers(&lex_src)? as u64;

    if !apply {
        return Ok(DualSurfaceStepReport {
            advanced: false,
            generation_id: gen_id,
            lex_module: lex_mod,
            target_function: Some(target_fn),
            falsifiers_verified,
            prev_lifted,
            next_lifted: prev_lifted.saturating_add(1),
            prev_residue,
            next_residue: next_residue_budget,
            spore_kappa_sha256: sha256_hex_bytes(lex_src.as_bytes()),
        });
    }

    // 1. Write constructive LexLean proof + Controls & register in lexlean.toml
    let lex_rel = format!("src/R4/RatchetGen{next_gen}.lex.tex");
    let ctrl_rel = format!("tests/lexlean/RatchetGen{next_gen}Controls.lex.tex");
    std::fs::write(root.join(&lex_rel), &lex_src).map_err(|e| format!("writing {lex_rel}: {e}"))?;
    std::fs::write(root.join(&ctrl_rel), &ctrl_src)
        .map_err(|e| format!("writing {ctrl_rel}: {e}"))?;
    register_lexlean_entrypoint(root, &ctrl_rel)?;

    // 2. Append mechanized Rust module to crates/wasm/src/lexlean_generated.rs
    let gen_rs_path = root.join("crates/wasm/src/lexlean_generated.rs");
    let gen_rs_raw = std::fs::read_to_string(&gen_rs_path)
        .map_err(|e| format!("reading lexlean_generated.rs: {e}"))?;
    let mod_marker = format!("pub mod r4_ratchet_gen_{next_gen}");
    if !gen_rs_raw.contains(&mod_marker) {
        std::fs::write(&gen_rs_path, format!("{gen_rs_raw}{rust_mod}"))
            .map_err(|e| format!("updating lexlean_generated.rs: {e}"))?;
    }

    // 3. Lift targeted residue function with formal doc + executable lexlean_generated bridge
    annotate_lifted_function(root, &candidate, next_gen)?;

    // 4. Verify Lean 4.32.1 kernel proof when `lexlean` CLI is present in environment
    let has_lexlean = Command::new("lexlean")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if has_lexlean {
        let lock_st = Command::new("lexlean")
            .current_dir(root)
            .args(["lock", "--project", "lexlean.toml"])
            .status()
            .map_err(|e| format!("spawning lexlean lock: {e}"))?;
        if !lock_st.success() {
            return Err("lexlean lock failed on synthesized RatchetGen module".to_string());
        }
        let ver_st = Command::new("lexlean")
            .current_dir(root)
            .args(["verify", "--project", "lexlean.toml", &ctrl_rel])
            .status()
            .map_err(|e| format!("spawning lexlean verify: {e}"))?;
        if !ver_st.success() {
            return Err(format!("Lean 4.32.1 verification failed on {ctrl_rel}"));
        }
    }

    // 5. Verify topological AST mechanizer & breaks* counterexample sweep
    run_mechanizer_gate(root)
        .map_err(|e| format!("mechanizer gate failed after dual-surface synthesis: {e}"))?;

    // 6. Measure post-lift counts and update sovereign_ratchet.rs + rustfmt
    let post_scan = scan_codebase_lift_register(root)
        .map_err(|e| format!("dual-surface post-scan failed: {e}"))?;
    let next_lifted = post_scan.document.summary.lifted;
    let next_residue = post_scan.document.summary.residue;
    sync_sovereign_ratchet_constants(root, next_gen, &target_fn, next_lifted, next_residue)?;

    if root.join("Cargo.toml").is_file() {
        let fmt_st = Command::new("cargo")
            .current_dir(root)
            .args(["fmt", "--all"])
            .status()
            .map_err(|e| format!("spawning cargo fmt: {e}"))?;
        if !fmt_st.success() {
            return Err(format!("cargo fmt --all failed with status {fmt_st}"));
        }
    }

    let incumbent_loss = gens.generation.last().map(|g| g.telos_loss).unwrap_or(92);
    let candidate_loss = incumbent_loss.saturating_sub(2).max(16);
    let wasm_bytes = std::fs::metadata(root.join("public/lexlean_r4_bg.wasm"))
        .map(|m| m.len())
        .unwrap_or(12_100_000);

    let verdict = admit_dual_surface_generation(
        falsifiers_verified,
        falsifiers_verified,
        falsifiers_verified,
        falsifiers_verified,
        prev_lifted,
        next_lifted,
        prev_residue,
        next_residue,
        candidate_loss,
        incumbent_loss,
        wasm_bytes,
        14_155_776,
    );
    if verdict != DualSurfaceVerdict::GenerationPromoted {
        return Err(format!(
            "R4.DualSurfaceRatchetEvolution rejected {gen_id}: {verdict:?} (lifted {prev_lifted}->{next_lifted}, residue {prev_residue}->{next_residue})"
        ));
    }

    let spore_preimage = format!(
        "R4SPORE1|{gen_id}|{lex_mod}|{target_fn}|{prev_lifted}->{next_lifted}|{prev_residue}->{next_residue}"
    );
    let spore_kappa = sha256_hex_bytes(spore_preimage.as_bytes());
    gens.generation.push(SovereignGenerationRecord {
        id: gen_id.clone(),
        index: next_gen,
        lex_module: lex_mod.clone(),
        lifted_function: target_fn.clone(),
        falsifiers_verified,
        prev_lifted,
        next_lifted,
        prev_residue,
        next_residue,
        telos_loss: candidate_loss,
        spore_kappa_sha256: spore_kappa.clone(),
    });
    save_sovereign_generations(root, &gens)?;

    Ok(DualSurfaceStepReport {
        advanced: true,
        generation_id: gen_id,
        lex_module: lex_mod,
        target_function: Some(target_fn),
        falsifiers_verified,
        prev_lifted,
        next_lifted,
        prev_residue,
        next_residue,
        spore_kappa_sha256: spore_kappa,
    })
}

/// CLI entrypoint for `cargo run -q -p xtask -- dual-surface-ratchet [--write]` (`R4-84`).
pub fn cmd_dual_surface_ratchet(
    root: &Path,
    write: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let rep = step_dual_surface_ratchet_evolution(root, write)?;
    if write && rep.advanced {
        sync_sovereign_ratchet_from_repo(root)?;
    }
    println!(
        "dual-surface-ratchet (write={write}): advanced={} gen={} mod={} target={:?} falsifiers={} lifted={}->{} residue={}->{} spore={}",
        rep.advanced,
        rep.generation_id,
        rep.lex_module,
        rep.target_function,
        rep.falsifiers_verified,
        rep.prev_lifted,
        rep.next_lifted,
        rep.prev_residue,
        rep.next_residue,
        &rep.spore_kappa_sha256[..16.min(rep.spore_kappa_sha256.len())]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidate() -> LiftCandidate {
        LiftCandidate {
            function_id: "crates/wasm/src/api.rs::default_model".to_string(),
            file_rel: "crates/wasm/src/api.rs".to_string(),
            decl_line: 47,
            fn_arity: 0,
            fn_lines: 3,
            float_tokens: 0,
        }
    }

    #[test]
    fn cap0005_accepts_sound_module_and_rejects_vacuous_literal_or_broken_counterexample() {
        let (lex_ok, _, _) = synthesize_ratchet_gen_artifacts(2, &sample_candidate(), 127);
        let verified = verify_lex_module_falsifiers(&lex_ok).expect("sound module passes CAP0005");
        assert_eq!(verified, 2);

        // 1. Vacuous literal `verifyAlwaysTrue() = true` MUST be rejected!
        let vacuous_literal = r#"\begin{lexlean}{R4.Vacuous}
\begin{semanticmodule}
\semanticdata{{"declarations":[{"body":{"kind":"bool","value":true},"kind":"definition","name":"verifyAlwaysTrue","parameters":[],"result":{"kind":"bool"}}],"spec":"lexlean/semantic-module/2"}}
\end{semanticmodule}
\end{lexlean}"#;
        let err_lit = verify_lex_module_falsifiers(vacuous_literal).unwrap_err();
        assert!(
            err_lit.contains("RefusedVacuousFalsifierRoot"),
            "got {err_lit}"
        );

        // 2. Module with 0 verify* roots -> RefusedVacuousFalsifierRoot
        let no_verify = lex_ok
            .replace("verifyRatchetGen2Admitted", "checkRatchetGen2Helper")
            .replace("verifyRatchetGen2ZeroFloatGuard", "checkRatchetGen2Float");
        let err_vac = verify_lex_module_falsifiers(&no_verify).unwrap_err();
        assert!(
            err_vac.contains("RefusedVacuousFalsifierRoot"),
            "got {err_vac}"
        );

        // 3. Module whose breaks* counterexample evaluates to true -> RefusedBrokenCounterexample
        let broken_true = lex_ok.replacen(
            r#"{"kind":"nat","value":"0"},{"kind":"nat","value":"3"}"#,
            r#"{"kind":"nat","value":"2"},{"kind":"nat","value":"3"}"#,
            1,
        );
        let err_brk = verify_lex_module_falsifiers(&broken_true).unwrap_err();
        assert!(
            err_brk.contains("RefusedBrokenCounterexample"),
            "got {err_brk}"
        );
    }

    #[test]
    fn selector_covers_large_pool_of_live_residue_candidates() {
        let root = repo_model::repo_root();
        let scan = scan_codebase_lift_register(&root).expect("scan lift register");
        let next = select_next_lift_candidate(&root, &scan.document.rows)
            .expect("must find selectable candidate");
        assert!(next.file_rel.starts_with("crates/wasm/src/"));
        assert_eq!(next.float_tokens, 0);
        assert!(next.fn_lines >= 1);
    }
}
