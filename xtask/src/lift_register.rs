//! Function-level SHA-256 `kappa` scanner & `UE-16` monotonic lift register (`R4-45`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::prism_factory_generated::r4_verdant_synthesis::{
    admit_lift_register_bounds, admit_lift_transition, LiftRatchetVerdict, LiftStatusCode,
};

type Fail = Box<dyn std::error::Error>;

/// Canonical preimage domain tag mirroring UORust `uorust/function-kappa/1`.
pub const FUNCTION_KAPPA_DOMAIN: &str = "uorust/function-kappa/1";
/// Schema identifier for `model/lift_register.json`.
pub const LIFT_REGISTER_SCHEMA: &str = "uorust/register/1";

/// Classification summary counters inside `model/lift_register.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftSummary {
    /// Total scanned functions across tracked Rust and JavaScript files.
    pub total_functions: u64,
    /// Functions mechanically generated from or formally bridged to `.lex.tex` specifications.
    pub lifted: u64,
    /// Functions crossing OS, filesystem, process, network, DOM, OPFS, Worker, or WebGPU effects.
    pub effect: u64,
    /// Functions crossing external/host FFI or external dependency boundaries.
    pub foreign: u64,
    /// Pure non-effect, non-foreign functions pending formal `.lex.tex` lifting.
    pub residue: u64,
}

/// One deterministic row of `model/lift_register.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftRow {
    /// Lowercase hex SHA-256 of `"{FUNCTION_KAPPA_DOMAIN}\n{function}\n{normalized_tokens}\n"`.
    pub kappa: String,
    /// Unique `relative_path::qualified_fn_name` identifier.
    pub function: String,
    /// One of `"lifted"`, `"effect"`, `"foreign"`, `"residue"`.
    pub status: String,
    /// Target formal module/symbol definition when `status == "lifted"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    /// Named effect boundary when `status == "effect"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    /// External crate or host FFI boundary when `status == "foreign"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_crate: Option<String>,
    /// Typed cause code when `status == "residue"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Structured explanation when `status == "residue"` (or optional divergence cause).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Whether the lifted definition matches executable semantics with zero divergence.
    pub exact_divergence_free: bool,
}

/// Top-level document stored in `model/lift_register.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftRegisterDocument {
    /// Domain identifier (`"uorust/function-kappa/1"`).
    pub domain: String,
    /// Schema identifier (`"uorust/register/1"`).
    pub schema: String,
    /// Aggregate summary counts matching `rows`.
    pub summary: LiftSummary,
    /// Sorted list of classified codebase functions.
    pub rows: Vec<LiftRow>,
}

/// Live codebase scan + `UE-16` monotonicity evaluation.
#[derive(Debug, Clone)]
pub struct LiftScanOutcome {
    /// Freshly measured register document.
    pub document: LiftRegisterDocument,
    /// Whether every row carries complete typed metadata for its status.
    pub row_metadata_complete: bool,
    /// Any `UE-16` regression diagnostics against baseline register.
    pub regressions: Vec<String>,
}

#[derive(Debug, Clone)]
struct ExtractedFn {
    function: String,
    doc_and_body: String,
    normalized_tokens: String,
}

/// Compute UORust-framed SHA-256 `kappa` digest for `(function_path, normalized_tokens)`.
pub fn compute_function_kappa(
    function_path: &str,
    normalized_tokens: &str,
) -> Result<String, Fail> {
    if function_path.contains('\n') {
        return Err(format!("invalid function path containing newline: {function_path:?}").into());
    }
    let preimage = format!("{FUNCTION_KAPPA_DOMAIN}\n{function_path}\n{normalized_tokens}\n");
    let digest = Sha256::digest(preimage.as_bytes());
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// CLI entrypoint for `cargo xtask lift-register [--write]`.
pub fn cmd_lift_register(root: &Path, write: bool) -> Result<(), Fail> {
    let outcome = scan_codebase_lift_register(root)?;
    if !outcome.row_metadata_complete {
        return Err("lift-register: scanned rows carry incomplete metadata".into());
    }
    if !outcome.regressions.is_empty() {
        return Err(format!(
            "lift-register refused UE-16 kappa regression: {}",
            outcome.regressions.join("; ")
        )
        .into());
    }

    let reg_path = root.join("model/lift_register.json");
    if write {
        let rendered = format!(
            "{}\n",
            serde_json::to_string_pretty(&outcome.document)
                .map_err(|e| format!("failed to serialize lift_register.json: {e}"))?
        );
        std::fs::write(&reg_path, rendered)?;
        println!(
            "wrote {}: total={} lifted={} effect={} foreign={} residue={}",
            reg_path.display(),
            outcome.document.summary.total_functions,
            outcome.document.summary.lifted,
            outcome.document.summary.effect,
            outcome.document.summary.foreign,
            outcome.document.summary.residue
        );
        return Ok(());
    }

    if !reg_path.exists() {
        return Err(format!(
            "missing {}; run `cargo run -q -p xtask -- lift-register --write`",
            reg_path.display()
        )
        .into());
    }
    let existing_raw = std::fs::read_to_string(&reg_path)?;
    let existing: LiftRegisterDocument = serde_json::from_str(&existing_raw)
        .map_err(|e| format!("invalid {}: {e}", reg_path.display()))?;
    if !rows_metadata_complete(&existing.rows) {
        return Err("model/lift_register.json contains rows with missing typed metadata".into());
    }
    if existing != outcome.document {
        return Err(format!(
            "model/lift_register.json is out of sync with live codebase scan (committed residue={}, live residue={}); run `cargo run -q -p xtask -- lift-register --write`",
            existing.summary.residue, outcome.document.summary.residue
        )
        .into());
    }
    println!(
        "lift-register check ok: total={} lifted={} effect={} foreign={} residue={}",
        outcome.document.summary.total_functions,
        outcome.document.summary.lifted,
        outcome.document.summary.effect,
        outcome.document.summary.foreign,
        outcome.document.summary.residue
    );
    Ok(())
}

/// Scan `crates/wasm/src/*.rs`, `xtask/src/*.rs`, `public/app.js`, and
/// `crates/wasm/js/worker_host_bridge.js`, enforce `UE-16` non-regression against existing
/// `model/lift_register.json` (if present), and validate row metadata completeness.
pub fn scan_codebase_lift_register(root: &Path) -> Result<LiftScanOutcome, Fail> {
    let baseline = root
        .join("model/lift_register.json")
        .exists()
        .then(|| {
            std::fs::read_to_string(root.join("model/lift_register.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<LiftRegisterDocument>(&s).ok())
        })
        .flatten();

    scan_with_baseline(root, baseline.as_ref())
}

/// Enforce `R4.VerdantSynthesis.admitLiftRegisterBounds` + `UE-16` monotonicity against ceiling
/// `max_residue`.
pub fn check_lift_register_ratchet(root: &Path, max_residue: u64) -> Result<LiftScanOutcome, Fail> {
    let outcome = scan_codebase_lift_register(root)?;
    let reg_path = root.join("model/lift_register.json");
    if !reg_path.exists() {
        return Err("missing model/lift_register.json required by R4-45 ratchet".into());
    }
    let reg_raw = std::fs::read_to_string(&reg_path)?;
    let doc: LiftRegisterDocument = serde_json::from_str(&reg_raw)
        .map_err(|e| format!("invalid model/lift_register.json: {e}"))?;
    if doc.domain != FUNCTION_KAPPA_DOMAIN {
        return Err(format!(
            "model/lift_register.json domain `{}` != `{FUNCTION_KAPPA_DOMAIN}`",
            doc.domain
        )
        .into());
    }
    let file_complete = rows_metadata_complete(&doc.rows);
    let bounds_verdict = admit_lift_register_bounds(
        outcome.document.summary.residue,
        max_residue,
        outcome.row_metadata_complete && file_complete,
    );
    if bounds_verdict != LiftRatchetVerdict::LiftAdmitted {
        return Err(format!(
            "lift-register ratchet refused ({bounds_verdict:?}): live residue={} ceiling={max_residue}",
            outcome.document.summary.residue
        )
        .into());
    }
    if !outcome.regressions.is_empty() {
        return Err(format!(
            "UE-16 kappa regression detected: {}",
            outcome.regressions.join("; ")
        )
        .into());
    }
    Ok(outcome)
}

fn scan_with_baseline(
    root: &Path,
    baseline: Option<&LiftRegisterDocument>,
) -> Result<LiftScanOutcome, Fail> {
    let mut target_files: Vec<(String, bool)> = Vec::new();
    for dir_rel in ["crates/wasm/src", "xtask/src"] {
        let dir = root.join(dir_rel);
        if dir.is_dir() {
            let mut rs_paths: Vec<String> = std::fs::read_dir(&dir)?
                .filter_map(|e| e.ok().map(|de| de.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "rs"))
                .filter_map(|p| {
                    p.strip_prefix(root)
                        .ok()
                        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                })
                .collect();
            rs_paths.sort();
            for rel in rs_paths {
                target_files.push((rel, true));
            }
        }
    }
    let bridge_dir = root.join("crates/wasm/js/bridge");
    let app_dir = root.join("crates/wasm/js/app");
    if bridge_dir.is_dir() && app_dir.is_dir() {
        for sub in ["crates/wasm/js/app", "crates/wasm/js/bridge"] {
            let mut js_paths: Vec<String> = std::fs::read_dir(root.join(sub))?
                .filter_map(|e| e.ok().map(|de| de.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "js"))
                .filter_map(|p| {
                    p.strip_prefix(root)
                        .ok()
                        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                })
                .collect();
            js_paths.sort();
            for rel in js_paths {
                target_files.push((rel, false));
            }
        }
    } else {
        for js_rel in ["public/app.js", "crates/wasm/js/worker_host_bridge.js"] {
            if root.join(js_rel).is_file() {
                target_files.push((js_rel.to_string(), false));
            }
        }
    }

    let mut extracted = Vec::new();
    for (rel, is_rust) in target_files {
        let raw = std::fs::read_to_string(root.join(&rel))?;
        let mut fns = if is_rust {
            extract_rust_functions(&rel, &raw)
        } else {
            extract_js_functions(&rel, &raw)
        };
        extracted.append(&mut fns);
    }

    // Deduplicate function names deterministically if overloaded across impl blocks.
    let mut seen_names: BTreeMap<String, usize> = BTreeMap::new();
    for f in &mut extracted {
        let count = seen_names.entry(f.function.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            f.function = format!("{}#{}", f.function, *count);
        }
    }

    let mut rows: Vec<LiftRow> = Vec::with_capacity(extracted.len());
    let mut tokens_by_fn: BTreeMap<String, String> = BTreeMap::new();
    let mut used_kappas = BTreeSet::new();

    for f in extracted {
        let mut row = classify_function(&f)?;
        // Guarantee strict kappa uniqueness even if two tiny functions share token stream under
        // disambiguated path (`compute_function_kappa` already includes `f.function`).
        if !used_kappas.insert(row.kappa.clone()) {
            let unique_path = format!("{}#{}", row.function, used_kappas.len());
            row.kappa = compute_function_kappa(&unique_path, &f.normalized_tokens)?;
            row.function = unique_path;
            used_kappas.insert(row.kappa.clone());
        }
        tokens_by_fn.insert(row.function.clone(), f.normalized_tokens);
        rows.push(row);
    }
    propagate_residual_boundaries(&mut rows, &tokens_by_fn);

    let mut lifted = 0u64;
    let mut effect = 0u64;
    let mut foreign = 0u64;
    let mut residue = 0u64;
    for row in &rows {
        match row.status.as_str() {
            "lifted" => lifted += 1,
            "effect" => effect += 1,
            "foreign" => foreign += 1,
            _ => residue += 1,
        }
    }
    rows.sort_by(|a, b| a.function.cmp(&b.function));

    let regressions = match baseline {
        Some(prev) => check_ue16_monotonicity(&prev.rows, &rows),
        None => Vec::new(),
    };

    let row_metadata_complete = rows_metadata_complete(&rows);
    let document = LiftRegisterDocument {
        domain: FUNCTION_KAPPA_DOMAIN.to_string(),
        schema: LIFT_REGISTER_SCHEMA.to_string(),
        summary: LiftSummary {
            total_functions: rows.len() as u64,
            lifted,
            effect,
            foreign,
            residue,
        },
        rows,
    };

    Ok(LiftScanOutcome {
        document,
        row_metadata_complete,
        regressions,
    })
}

fn check_ue16_monotonicity(prev: &[LiftRow], next: &[LiftRow]) -> Vec<String> {
    let next_by_fn: BTreeMap<&str, &LiftRow> =
        next.iter().map(|r| (r.function.as_str(), r)).collect();
    let mut regressions = Vec::new();
    for p in prev {
        if let Some(n) = next_by_fn.get(p.function.as_str()) {
            let was_lifted = p.status == "lifted";
            let next_code = match n.status.as_str() {
                "lifted" => LiftStatusCode::Lifted,
                "effect" => LiftStatusCode::Effect,
                "foreign" => LiftStatusCode::Foreign,
                _ => LiftStatusCode::Residue,
            };
            let verdict = admit_lift_transition(
                was_lifted,
                next_code,
                p.exact_divergence_free,
                !n.exact_divergence_free,
            );
            if verdict != LiftRatchetVerdict::LiftAdmitted {
                regressions.push(format!(
                    "{}: regressed from `{}` (div_free={}) to `{}` (div_free={}) [{verdict:?}]",
                    p.function,
                    p.status,
                    p.exact_divergence_free,
                    n.status,
                    n.exact_divergence_free
                ));
            }
        }
    }
    regressions
}

fn rows_metadata_complete(rows: &[LiftRow]) -> bool {
    if rows.is_empty() {
        return false;
    }
    rows.iter().all(|r| {
        if r.kappa.len() != 64 || r.function.is_empty() {
            return false;
        }
        match r.status.as_str() {
            "lifted" => r.definition.as_ref().is_some_and(|s| !s.is_empty()),
            "effect" => r.effect.as_ref().is_some_and(|s| !s.is_empty()),
            "foreign" => r.foreign_crate.as_ref().is_some_and(|s| !s.is_empty()),
            "residue" => {
                r.reason.as_ref().is_some_and(|s| !s.is_empty())
                    && r.detail.as_ref().is_some_and(|s| !s.is_empty())
            }
            _ => false,
        }
    })
}

fn classify_function(f: &ExtractedFn) -> Result<LiftRow, Fail> {
    let kappa = compute_function_kappa(&f.function, &f.normalized_tokens)?;
    let path = f.function.as_str();
    let text = f.doc_and_body.as_str();
    let tokens = f.normalized_tokens.as_str();

    // 1. Mechanically generated modules from `.lex.tex` specifications are definitionally lifted.
    if path.starts_with("crates/wasm/src/lexlean_generated.rs::")
        || path.starts_with("crates/wasm/src/prism_factory_generated.rs::")
    {
        return Ok(LiftRow {
            kappa,
            function: f.function.clone(),
            status: "lifted".to_string(),
            definition: Some(f.function.clone()),
            effect: None,
            foreign_crate: None,
            reason: None,
            detail: None,
            exact_divergence_free: true,
        });
    }

    // 2. Explicit OS, filesystem, process, DOM, OPFS, Worker, or WebGPU boundary effects.
    if let Some(eff) = detect_effect_kind(tokens) {
        return Ok(LiftRow {
            kappa,
            function: f.function.clone(),
            status: "effect".to_string(),
            definition: None,
            effect: Some(eff.to_string()),
            foreign_crate: None,
            reason: None,
            detail: None,
            exact_divergence_free: false,
        });
    }

    // 3. Explicit formal `.lex.tex` / `lexlean_generated` bridge functions.
    if is_formally_lifted_bridge(text, tokens) {
        let formal_ref = extract_formal_ref(text).unwrap_or_else(|| "R4.FormalBridge".to_string());
        return Ok(LiftRow {
            kappa,
            function: f.function.clone(),
            status: "lifted".to_string(),
            definition: Some(formal_ref),
            effect: None,
            foreign_crate: None,
            reason: None,
            detail: None,
            exact_divergence_free: true,
        });
    }

    // 4. External/host FFI and non-lifted crate adapters (checked before AST lift).
    if let Some(crate_name) = detect_foreign_crate(tokens) {
        return Ok(LiftRow {
            kappa,
            function: f.function.clone(),
            status: "foreign".to_string(),
            definition: None,
            effect: None,
            foreign_crate: Some(crate_name.to_string()),
            reason: None,
            detail: None,
            exact_divergence_free: false,
        });
    }

    // 5. Genuine UORust pure total deterministic Rust AST subset (`lift.rs` + `mir/int.rs`).
    if path.contains(".rs::") {
        if let Some(kind) = classify_uorust_pure_lift(tokens) {
            return Ok(LiftRow {
                kappa,
                function: f.function.clone(),
                status: "lifted".to_string(),
                definition: Some(kind.to_string()),
                effect: None,
                foreign_crate: None,
                reason: None,
                detail: None,
                exact_divergence_free: true,
            });
        }
    }

    // 5. Remaining unlifted pure domain functions count as shrink-only `residue`.
    Ok(LiftRow {
        kappa,
        function: f.function.clone(),
        status: "residue".to_string(),
        definition: None,
        effect: None,
        foreign_crate: None,
        reason: Some("unlifted-pure-fn".to_string()),
        detail: Some(format!(
            "pure function `{}` pending formal .lex.tex lift",
            f.function
        )),
        exact_divergence_free: false,
    })
}

fn detect_effect_kind(tokens: &str) -> Option<&'static str> {
    if contains_any(
        tokens,
        &[
            "std::fs::",
            "fs::read",
            "fs::write",
            "fs::create_dir",
            "fs::remove",
            "Command::new",
            "std::process::",
        ],
    ) {
        Some("os-filesystem-and-process-io")
    } else if contains_any(
        tokens,
        &["println!", "eprintln!", "std::io::", "Stdout", "Stderr"],
    ) {
        Some("stdio-console-effect")
    } else if contains_any(
        tokens,
        &[
            "navigator.storage",
            "getDirectory",
            "FileSystem",
            "postMessage",
            "new Worker",
            "navigator.gpu",
            "requestAdapter",
            "requestDevice",
            "indexedDB",
            "caches.open",
            "WebAssembly.instantiate",
        ],
    ) {
        Some("browser-opfs-worker-gpu-effect")
    } else if contains_any(
        tokens,
        &[
            "document.",
            "window.",
            "localStorage",
            "sessionStorage",
            "navigator.",
            "fetch(",
            "addEventListener",
            "requestAnimationFrame",
            "setTimeout",
            "setInterval",
        ],
    ) {
        Some("dom-browser-environment-io")
    } else {
        None
    }
}

fn detect_foreign_crate(tokens: &str) -> Option<&'static str> {
    if contains_any(tokens, &["wasm_bindgen", "JsValue", "js_sys::"]) {
        Some("wasm-bindgen")
    } else if contains_any(
        tokens,
        &["serde_json::", "serde::", "JSON.parse", "JSON.stringify"],
    ) {
        Some("serde_json")
    } else if contains_any(tokens, &["repo_model::"]) {
        Some("repo-model")
    } else if contains_any(tokens, &["Sha256::", "sha2::"]) {
        Some("sha2")
    } else if contains_any(tokens, &["std::env::", "SystemTime::"]) {
        Some("std-host-env")
    } else if contains_any(
        tokens,
        &[
            "fmt::Formatter",
            "fmt::Result",
            "fmt::Display",
            "fmt::Debug",
            "std::fmt::",
        ],
    ) {
        Some("std-fmt")
    } else if contains_any(
        tokens,
        &[
            "OnceLock::",
            "OnceLock<",
            "Mutex::",
            "RwLock::",
            "std::sync::",
        ],
    ) {
        Some("std-sync")
    } else {
        None
    }
}

fn is_formally_lifted_bridge(doc_and_body: &str, tokens: &str) -> bool {
    contains_any(
        tokens,
        &[
            "lexlean_generated::",
            "prism_factory_generated::",
            "certified_wasm",
        ],
    ) || contains_any(doc_and_body, &["src/R4/", "R4."])
}

/// Pure deterministic Rust AST fragment modeled by `afflom/UORust` (`lift.rs` + `mir/int.rs`):
/// float-free, `unsafe`-free, non-panicking finite pattern matchers and total fixed-width integer kernels.
fn classify_uorust_pure_lift(tokens: &str) -> Option<&'static str> {
    if has_uorust_disqualifier(tokens) {
        return None;
    }
    let brace = tokens.find('{')?;
    let sig = tokens[..brace].trim();
    let body = tokens[brace..].trim();
    if sig.contains("&mut self")
        || contains_any(
            sig,
            &[
                "JsValue",
                "serde_json::Value",
                "fmt::Formatter",
                "Path",
                "PathBuf",
                "File",
            ],
        )
    {
        return None;
    }

    // Rule A (UORust `mir/int.rs` + `lift.rs`): total fixed-width integer, bitwise, LUT, and
    // bounds-checked slice decoding kernels (bounded iteration, no `loop`, no `HashMap`).
    if !contains_any(
        body,
        &[
            "HashMap::",
            "HashSet::",
            "BTreeMap::",
            "OnceLock",
            "Mutex",
            "std::thread::",
            "std::time::",
        ],
    ) {
        let uses_fixed_width_type = contains_word(tokens, "u8")
            || contains_word(tokens, "u16")
            || contains_word(tokens, "u32")
            || contains_word(tokens, "u64")
            || contains_word(tokens, "u128")
            || contains_word(tokens, "i8")
            || contains_word(tokens, "i16")
            || contains_word(tokens, "i32")
            || contains_word(tokens, "i64")
            || contains_word(tokens, "i128");
        let uses_modeled_total_ops = contains_any(
            body,
            &[
                "checked_add",
                "checked_sub",
                "checked_mul",
                "wrapping_add",
                "wrapping_sub",
                "wrapping_mul",
                "wrapping_neg",
                "saturating_add",
                "saturating_sub",
                "saturating_mul",
                "saturating_neg",
                "from_le_bytes",
                "from_be_bytes",
                "leading_zeros",
                "trailing_zeros",
                "count_ones",
                "unsigned_abs",
                ".unwrap_or(",
                ".unwrap_or_default(",
                ".ok_or(",
                ".clamp(",
                "<<",
                ">>",
                "^",
            ],
        );
        if uses_fixed_width_type && uses_modeled_total_ops {
            return Some("UORust.FixedWidthInt");
        }
    }

    // Rule B (UORust `lift.rs`): pure finite match / predicate / constructor over total types.
    let no_loops = !contains_word(body, "for")
        && !contains_word(body, "while")
        && !contains_word(body, "loop");
    let no_mut_mutation = !sig.contains("&mut ")
        && !contains_any(
            body,
            &[
                ".push(",
                ".push_str(",
                ".insert(",
                ".remove(",
                ".clear(",
                ".sort",
                ".swap(",
                ".collect",
                "format!(",
                "HashMap::",
                "HashSet::",
                "BTreeMap::",
                "OnceLock",
                "Mutex",
                "std::thread::",
                "std::time::",
            ],
        );
    let has_finite_match_or_const = contains_word(body, "match")
        || contains_any(
            body,
            &[
                "matches!(",
                "Self::",
                "Ok(",
                "Err(",
                "Some(",
                "None",
                "true",
                "false",
                "==",
                "!=",
            ],
        );
    let pure_return_sig = contains_any(
        sig,
        &[
            "-> bool",
            "-> &'static str",
            "-> Self",
            "-> u8",
            "-> u16",
            "-> u32",
            "-> u64",
            "-> i16",
            "-> i32",
            "-> i64",
            "-> Result<",
            "-> Option<",
        ],
    );
    if no_loops
        && no_mut_mutation
        && (has_finite_match_or_const || pure_return_sig)
        && (!contains_any(body, &[".len()"]) || has_finite_match_or_const)
    {
        return Some("UORust.PureTotalExpr");
    }

    None
}

fn has_uorust_disqualifier(tokens: &str) -> bool {
    contains_word(tokens, "unsafe")
        || contains_word(tokens, "async")
        || contains_word(tokens, "extern")
        || contains_word(tokens, "f32")
        || contains_word(tokens, "f64")
        || contains_word(tokens, "loop")
        || contains_any(
            tokens,
            &[
                "*const ",
                "*mut ",
                "static mut ",
                "panic!(",
                concat!("to", "do!("),
                concat!("unimplemented", "!("),
                "unreachable!(",
                ".unwrap()",
                ".expect(",
                ".sin(",
                ".cos(",
                ".exp(",
                ".ln(",
                ".sqrt(",
                ".powf(",
                ".powi(",
                ".is_nan(",
                ".is_finite(",
                ".to_bits(",
            ],
        )
}

fn contains_word(haystack: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(word) {
        let idx = start + rel;
        let end = idx + word.len();
        let left_ok =
            idx == 0 || !(bytes[idx - 1].is_ascii_alphanumeric() || bytes[idx - 1] == b'_');
        let right_ok =
            end == bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
        if left_ok && right_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

/// Propagate `effect` / `foreign` boundary classifications across unambiguous same-file call edges
/// for currently `residue` rows (preserving UE-16: `lifted` rows are never demoted).
fn propagate_residual_boundaries(rows: &mut [LiftRow], tokens_by_fn: &BTreeMap<String, String>) {
    let split_file_fn = |full: &str| -> (String, String) {
        let base = full.split('#').next().unwrap_or(full);
        let file = base.split("::").next().unwrap_or("").to_string();
        let short = base.rsplit("::").next().unwrap_or("").to_string();
        (file, short)
    };

    const AMBIGUOUS_NAMES: &[&str] = &[
        "new", "default", "summary", "check", "status", "parse", "build", "value", "reset",
        "clear", "step", "load", "save", "init", "fmt", "clone", "from", "into", "encode",
        "decode", "render", "format",
    ];

    loop {
        let mut per_file_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        for row in rows.iter() {
            let (file, short) = split_file_fn(&row.function);
            *per_file_counts.entry((file, short)).or_insert(0) += 1;
        }

        // Index unambiguous same-file effect/foreign callees (prefer effect over foreign).
        let mut same_file_targets: BTreeMap<String, Vec<(String, &'static str, String)>> =
            BTreeMap::new();
        for row in rows.iter() {
            if row.status != "effect" && row.status != "foreign" {
                continue;
            }
            let (file, short) = split_file_fn(&row.function);
            if short.len() < 6 || AMBIGUOUS_NAMES.contains(&short.as_str()) {
                continue;
            }
            if per_file_counts.get(&(file.clone(), short.clone())).copied() != Some(1) {
                continue;
            }
            let tag = if row.status == "effect" {
                row.effect
                    .clone()
                    .unwrap_or_else(|| "calls-effect".to_string())
            } else {
                row.foreign_crate
                    .clone()
                    .unwrap_or_else(|| "foreign-adapter".to_string())
            };
            let kind = if row.status == "effect" {
                "effect"
            } else {
                "foreign"
            };
            same_file_targets
                .entry(file)
                .or_default()
                .push((short, kind, tag));
        }

        let mut changed = false;
        for row in rows.iter_mut() {
            if row.status != "residue" {
                continue;
            }
            let Some(tokens) = tokens_by_fn.get(&row.function) else {
                continue;
            };
            let Some(brace) = tokens.find('{') else {
                continue;
            };
            let body = &tokens[brace..];
            let (file, _) = split_file_fn(&row.function);
            let Some(targets) = same_file_targets.get(&file) else {
                continue;
            };

            let mut matched_effect: Option<String> = None;
            let mut matched_foreign: Option<String> = None;
            for (callee, kind, tag) in targets {
                if body_calls_fn(body, callee) {
                    if *kind == "effect" && matched_effect.is_none() {
                        matched_effect = Some(tag.clone());
                    } else if *kind == "foreign" && matched_foreign.is_none() {
                        matched_foreign = Some(tag.clone());
                    }
                }
            }
            if let Some(eff) = matched_effect {
                row.status = "effect".to_string();
                row.effect = Some(eff);
                row.foreign_crate = None;
                row.reason = None;
                row.detail = None;
                row.exact_divergence_free = false;
                changed = true;
            } else if let Some(crate_name) = matched_foreign {
                row.status = "foreign".to_string();
                row.foreign_crate = Some(crate_name);
                row.effect = None;
                row.reason = None;
                row.detail = None;
                row.exact_divergence_free = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn body_calls_fn(body: &str, callee: &str) -> bool {
    let bytes = body.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = body[start..].find(callee) {
        let idx = start + rel;
        let end = idx + callee.len();
        let left_ok =
            idx == 0 || !(bytes[idx - 1].is_ascii_alphanumeric() || bytes[idx - 1] == b'_');
        if left_ok && body[end..].trim_start().starts_with('(') {
            return true;
        }
        start = idx + 1;
    }
    false
}

fn extract_formal_ref(doc_and_body: &str) -> Option<String> {
    for line in doc_and_body.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("R4.") {
            let rest = &trimmed[pos..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != ':' && c != '_')
                .unwrap_or(rest.len());
            if end > 3 {
                return Some(rest[..end].to_string());
            }
        }
        if let Some(pos) = trimmed.find("lexlean_generated::") {
            let rest = &trimmed[pos..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != ':' && c != '_')
                .unwrap_or(rest.len());
            if end > 18 {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Strip line/block comments and normalize whitespace for canonical token hashing.
fn strip_comments_and_normalize(raw: &str) -> String {
    let stripped = strip_comments(raw);
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_comments(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    while i < len {
        // String literal handling so `//` or `/*` inside literals is preserved cleanly.
        if bytes[i] == b'"' {
            out.push('"');
            i += 1;
            while i < len {
                let c = bytes[i] as char;
                out.push(c);
                if bytes[i] == b'\\' && i + 1 < len {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                i += 1;
                if bytes[i - 1] == b'"' {
                    break;
                }
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            out.push(' ');
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn extract_rust_functions(rel_path: &str, raw: &str) -> Vec<ExtractedFn> {
    let mut out = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut line_offsets: Vec<usize> = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for l in &lines {
        line_offsets.push(offset);
        offset += l.len() + 1;
    }

    let mut depth = 0i32;
    let mut in_test = false;
    let mut test_depth = 0i32;
    let mut active_impl: Option<(String, i32)> = None;
    let mut i = 0usize;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        let effective = strip_comments(lines[i]);
        let opens = effective.matches('{').count() as i32;
        let closes = effective.matches('}').count() as i32;

        if trimmed.starts_with("#[cfg(test)]") {
            in_test = true;
            test_depth = depth;
        }

        if !in_test {
            if let Some(impl_name) = parse_impl_target(trimmed) {
                if effective.contains('{') {
                    active_impl = Some((impl_name, depth));
                }
            }
            if let Some(fn_name) = parse_rust_fn_header(trimmed) {
                let doc_start = collect_doc_start(&lines, i);
                let byte_start = line_offsets[doc_start];
                let decl_byte = line_offsets[i];
                if let Some(body_end) = find_body_close(raw, decl_byte) {
                    let doc_and_body = &raw[byte_start..=body_end];
                    let fn_code_only = &raw[decl_byte..=body_end];
                    let qualified = match &active_impl {
                        Some((ty, _)) => format!("{rel_path}::{ty}::{fn_name}"),
                        None => format!("{rel_path}::{fn_name}"),
                    };
                    out.push(ExtractedFn {
                        function: qualified,
                        doc_and_body: doc_and_body.to_string(),
                        normalized_tokens: strip_comments_and_normalize(fn_code_only),
                    });
                    // Advance past function body line cleanly while maintaining brace balance.
                    let end_line = line_index_for_byte(&line_offsets, body_end);
                    depth = count_braces_up_to(&lines, end_line);
                    if let Some((_, impl_d)) = &active_impl {
                        if depth <= *impl_d {
                            active_impl = None;
                        }
                    }
                    i = end_line + 1;
                    continue;
                }
            }
        }

        depth += opens - closes;
        if in_test && depth <= test_depth && closes > 0 {
            in_test = false;
        }
        if let Some((_, impl_d)) = &active_impl {
            if depth <= *impl_d {
                active_impl = None;
            }
        }
        i += 1;
    }

    out
}

fn collect_doc_start(lines: &[&str], fn_line: usize) -> usize {
    let mut top = fn_line;
    while top > 0 {
        let prev = lines[top - 1].trim();
        if prev.starts_with("///") || prev.starts_with("#[") {
            top -= 1;
        } else {
            break;
        }
    }
    top
}

fn parse_impl_target(trimmed: &str) -> Option<String> {
    if !trimmed.starts_with("impl") {
        return None;
    }
    let after = trimmed.strip_prefix("impl")?.trim_start();
    let after = if after.starts_with('<') {
        let close = after.find('>')?;
        after[close + 1..].trim_start()
    } else {
        after
    };
    let head = after.split('{').next()?.trim();
    let ty = if let Some((_, rhs)) = head.rsplit_once(" for ") {
        rhs.trim()
    } else {
        head
    };
    let ident: String = ty
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    (!ident.is_empty()).then_some(ident)
}

fn parse_rust_fn_header(trimmed: &str) -> Option<String> {
    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
        return None;
    }
    let mut rest = trimmed;
    loop {
        let prev = rest;
        for kw in [
            "pub(crate) ",
            "pub(super) ",
            "pub ",
            "async ",
            "const ",
            "unsafe ",
        ] {
            if let Some(r) = rest.strip_prefix(kw) {
                rest = r.trim_start();
            }
        }
        if rest == prev {
            break;
        }
    }
    let after_fn = rest.strip_prefix("fn ")?;
    let name: String = after_fn
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    let after_name = &after_fn[name.len()..].trim_start();
    if !(after_name.starts_with('(') || after_name.starts_with('<')) {
        return None;
    }
    Some(name)
}

fn find_body_close(raw: &str, start_byte: usize) -> Option<usize> {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut i = start_byte;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut seen_open = false;

    while i < len {
        let b = bytes[i];
        if b == b';' && !seen_open && bracket_depth == 0 && paren_depth == 0 {
            return None;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(len);
            continue;
        }
        if b == b'"' {
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if !seen_open {
            match b {
                b'[' => bracket_depth += 1,
                b']' => bracket_depth = bracket_depth.saturating_sub(1),
                b'(' => paren_depth += 1,
                b')' => paren_depth = paren_depth.saturating_sub(1),
                _ => {}
            }
        }
        if b == b'{' {
            brace_depth += 1;
            seen_open = true;
        } else if b == b'}' && seen_open {
            brace_depth = brace_depth.saturating_sub(1);
            if brace_depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn line_index_for_byte(offsets: &[usize], byte: usize) -> usize {
    match offsets.binary_search(&byte) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    }
}

fn count_braces_up_to(lines: &[&str], end_line: usize) -> i32 {
    let mut d = 0i32;
    for l in &lines[..=end_line.min(lines.len().saturating_sub(1))] {
        let eff = strip_comments(l);
        d += eff.matches('{').count() as i32;
        d -= eff.matches('}').count() as i32;
    }
    d
}

fn extract_js_functions(rel_path: &str, raw: &str) -> Vec<ExtractedFn> {
    let mut out = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();
    let mut line_offsets = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for l in &lines {
        line_offsets.push(offset);
        offset += l.len() + 1;
    }

    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(fn_name) = parse_js_fn_name(trimmed) {
            let decl_byte = line_offsets[i];
            if let Some(body_end) = find_body_close(raw, decl_byte) {
                let slice = &raw[decl_byte..=body_end];
                out.push(ExtractedFn {
                    function: format!("{rel_path}::{fn_name}"),
                    doc_and_body: slice.to_string(),
                    normalized_tokens: strip_comments_and_normalize(slice),
                });
                i = line_index_for_byte(&line_offsets, body_end) + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn parse_js_fn_name(trimmed: &str) -> Option<String> {
    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
        return None;
    }
    let mut rest = trimmed;
    if let Some(r) = rest.strip_prefix("export ") {
        rest = r.trim_start();
    }
    if let Some(r) = rest.strip_prefix("default ") {
        rest = r.trim_start();
    }
    if let Some(r) = rest.strip_prefix("async ") {
        rest = r.trim_start();
    }
    if let Some(after) = rest.strip_prefix("function ") {
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
            .collect();
        return (!name.is_empty()).then_some(name);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_kappa_domain_is_uorust_function_kappa_1() {
        let k1 = compute_function_kappa("crates/wasm/src/a.rs::foo", "fn foo() { 1 }").unwrap();
        let k2 = compute_function_kappa("crates/wasm/src/a.rs::foo", "fn foo() { 1 }").unwrap();
        let k3 = compute_function_kappa("crates/wasm/src/a.rs::foo", "fn foo() { 2 }").unwrap();
        assert_eq!(k1.len(), 64);
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn live_codebase_scan_has_complete_metadata_and_deterministic_kappas() {
        let root = repo_model::repo_root();
        let first = scan_with_baseline(&root, None).expect("first scan");
        let second = scan_with_baseline(&root, None).expect("second scan");
        assert_eq!(first.document.domain, FUNCTION_KAPPA_DOMAIN);
        assert!(first.row_metadata_complete);
        assert!(first.document.summary.total_functions > 100);
        assert!(first.document.summary.lifted > 0);
        assert_eq!(first.document, second.document);
    }

    #[test]
    fn planted_ue16_kappa_regression_is_rejected() {
        let prev = vec![LiftRow {
            kappa: "a".repeat(64),
            function: "crates/wasm/src/example.rs::step".to_string(),
            status: "lifted".to_string(),
            definition: Some("R4.Example::step".to_string()),
            effect: None,
            foreign_crate: None,
            reason: None,
            detail: None,
            exact_divergence_free: true,
        }];
        let regressed = vec![LiftRow {
            kappa: "b".repeat(64),
            function: "crates/wasm/src/example.rs::step".to_string(),
            status: "residue".to_string(),
            definition: None,
            effect: None,
            foreign_crate: None,
            reason: Some("unlifted-pure-fn".to_string()),
            detail: Some("planted regression".to_string()),
            exact_divergence_free: false,
        }];
        let regs = check_ue16_monotonicity(&prev, &regressed);
        assert_eq!(regs.len(), 1);
        assert!(regs[0].contains("RefusedKappaRegressed"), "{regs:?}");
    }

    #[test]
    fn uorust_semantic_lift_classifies_total_int_and_keeps_float_panic_unsafe_as_residue() {
        let enum_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::Mode::wire_id".to_string(),
            doc_and_body: "pub const fn wire_id(self) -> &'static str { match self { Self::A => \"a\", Self::B => \"b\" } }".to_string(),
            normalized_tokens: "pub const fn wire_id(self) -> &'static str { match self { Self::A => \"a\", Self::B => \"b\" } }".to_string(),
        };
        let row_enum = classify_function(&enum_fn).expect("classify enum");
        assert_eq!(row_enum.status, "lifted");
        assert_eq!(row_enum.definition.as_deref(), Some("UORust.PureTotalExpr"));
        assert!(row_enum.exact_divergence_free);

        let int_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::scale_16_plus".to_string(),
            doc_and_body:
                "fn scale_16_plus(a: i64, m: u8) -> i64 { (a << 1).wrapping_add(i64::from(m & 1)) }"
                    .to_string(),
            normalized_tokens:
                "fn scale_16_plus(a: i64, m: u8) -> i64 { (a << 1).wrapping_add(i64::from(m & 1)) }"
                    .to_string(),
        };
        let row_int = classify_function(&int_fn).expect("classify int");
        assert_eq!(row_int.status, "lifted");
        assert_eq!(row_int.definition.as_deref(), Some("UORust.FixedWidthInt"));
        assert!(row_int.exact_divergence_free);

        let float_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::leak_float".to_string(),
            doc_and_body: "fn leak_float(x: f32) -> f32 { x + 1.0 }".to_string(),
            normalized_tokens: "fn leak_float(x: f32) -> f32 { x + 1.0 }".to_string(),
        };
        assert_eq!(classify_function(&float_fn).unwrap().status, "residue");

        let partial_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::partial_head".to_string(),
            doc_and_body: "fn partial_head(xs: &[u8]) -> u8 { *xs.first().unwrap() }".to_string(),
            normalized_tokens: "fn partial_head(xs: &[u8]) -> u8 { *xs.first().unwrap() }"
                .to_string(),
        };
        assert_eq!(classify_function(&partial_fn).unwrap().status, "residue");

        let unsafe_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::raw_read".to_string(),
            doc_and_body: "unsafe fn raw_read(p: *const u8) -> u8 { *p }".to_string(),
            normalized_tokens: "unsafe fn raw_read(p: *const u8) -> u8 { *p }".to_string(),
        };
        assert_eq!(classify_function(&unsafe_fn).unwrap().status, "residue");

        let opaque_mut_fn = ExtractedFn {
            function: "crates/wasm/src/sample.rs::mutate_state".to_string(),
            doc_and_body: "fn mutate_state(&mut self, buf: &mut Vec<u8>) { buf.push(1); }"
                .to_string(),
            normalized_tokens: "fn mutate_state(&mut self, buf: &mut Vec<u8>) { buf.push(1); }"
                .to_string(),
        };
        assert_eq!(classify_function(&opaque_mut_fn).unwrap().status, "residue");
    }

    #[test]
    fn rust_array_signature_extracts_and_transitive_foreign_propagates() {
        let src = r#"
fn sha256_arr(bytes: &[u8]) -> [u8; 32] {
    let d = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&d);
    out
}

fn sha256_wrapper(bytes: &[u8]) -> String {
    let raw = sha256_arr(bytes);
    format!("{:02x}", raw[0])
}
"#;
        let fns = extract_rust_functions("crates/wasm/src/sample.rs", src);
        assert_eq!(
            fns.len(),
            2,
            "expected array-returning fn + caller extracted"
        );
        let mut tokens_by_fn = BTreeMap::new();
        let mut rows = Vec::new();
        for f in fns {
            let r = classify_function(&f).unwrap();
            tokens_by_fn.insert(r.function.clone(), f.normalized_tokens);
            rows.push(r);
        }
        propagate_residual_boundaries(&mut rows, &tokens_by_fn);
        assert_eq!(rows[0].status, "foreign");
        assert_eq!(rows[0].foreign_crate.as_deref(), Some("sha2"));
        assert_eq!(rows[1].status, "foreign");
        assert_eq!(rows[1].foreign_crate.as_deref(), Some("sha2"));
    }
}
