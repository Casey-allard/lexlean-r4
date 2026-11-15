//! Verdant-Resonance topological `.lex.tex` AST mechanizer & `breaks*` counterexample sweep (`R4-45`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::prism_factory_generated::r4_verdant_synthesis::{
    admit_mechanizer_graph, MechanizerCheck, MechanizerVerdict,
};

type Fail = Box<dyn std::error::Error>;

/// Per-module summary emitted by the `.lex.tex` AST mechanizer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MechanizerModuleSummary {
    /// Module name declared in `\begin{lexlean}{...}`.
    pub module_name: String,
    /// Repository-relative file path.
    pub path: String,
    /// Imported modules declared via `\importmodule{...}`.
    pub imports: Vec<String>,
    /// Number of declarations in the semantic module AST.
    pub declaration_count: usize,
}

/// Complete report from [`mechanize_directory`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MechanizerReport {
    /// Topologically ordered modules (`dag_acyclic == true` iff all modules placed).
    pub ordered_modules: Vec<MechanizerModuleSummary>,
    /// Whether the module import DAG is cycle-free.
    pub dag_acyclic: bool,
    /// Whether every executable definition closes without unresolved callees or axioms/sorry.
    pub all_roots_executable: bool,
    /// Duplicate module or declaration count observed across scanned modules.
    pub duplicate_decls: u64,
    /// Total `breaks*` counterexample predicates swept across concrete fault inputs.
    pub breaks_checked: u64,
    /// Count of `breaks*` counterexamples that evaluated to `true` (must equal `0`).
    pub breaks_triggered: u64,
    /// Number of `verify*` predicates and control theorems that evaluated to `true` / `lhs == rhs`.
    pub verify_passed: u64,
    /// Total `verify*` predicates and control theorems swept.
    pub verify_total: u64,
    /// Formal [`MechanizerVerdict`] name produced by `R4.VerdantSynthesis.admitMechanizerGraph`.
    pub verdict: String,
    /// Detailed failure or audit diagnostics when any invariant fails.
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedLexModule {
    module_name: String,
    rel_path: String,
    imports: Vec<String>,
    declarations: Vec<Value>,
    decls_by_name: BTreeMap<String, Value>,
    inductive_nullary_ctors: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AstVal {
    Bool(bool),
    Nat(u128),
    Ctor(String, Vec<AstVal>),
    Record(BTreeMap<String, AstVal>),
    List(Vec<AstVal>),
}

/// Run the full Verdant-Resonance `.lex.tex` AST mechanizer across repository `src/R4` and
/// `tests/lexlean` roots and enforce `R4.VerdantSynthesis.admitMechanizerGraph`.
pub fn run_mechanizer_gate(root: &Path) -> Result<MechanizerReport, Fail> {
    let subdirs = ["src/R4", "tests/lexlean"];
    mechanize_directory(root, &subdirs)
}

/// CLI entrypoint for `cargo xtask mechanize`.
pub fn cmd_mechanize(root: &Path) -> Result<(), Fail> {
    let report = run_mechanizer_gate(root)?;
    println!(
        "mechanize: verdict={} modules={} dag_acyclic={} executable={} dup={} breaks={}/{} verify={}/{}",
        report.verdict,
        report.ordered_modules.len(),
        report.dag_acyclic,
        report.all_roots_executable,
        report.duplicate_decls,
        report.breaks_triggered,
        report.breaks_checked,
        report.verify_passed,
        report.verify_total,
    );
    Ok(())
}

/// Mechanize all `.lex.tex` files beneath `subdirs` (relative to `root`) through:
/// 1. `lexlean/semantic-module/2` JSON AST loading & duplicate detection,
/// 2. Kahn topological DAG sort over `\importmodule{...}` edges,
/// 3. Executable closure & sorry/axiom checks,
/// 4. Auto-discovered `breaks*` counterexample sweep + `verify*`/control theorem evaluation,
/// 5. Certified gate admission via `r4_verdant_synthesis::admit_mechanizer_graph`.
pub fn mechanize_directory(root: &Path, subdirs: &[&str]) -> Result<MechanizerReport, Fail> {
    let mut files: Vec<PathBuf> = Vec::new();
    for rel in subdirs {
        let dir = root.join(rel);
        if dir.is_dir() {
            collect_lex_files(&dir, &mut files)?;
        }
    }
    files.sort();
    if files.is_empty() {
        return Err("mechanize: no .lex.tex modules found under configured roots".into());
    }

    let mut diagnostics: Vec<String> = Vec::new();
    let mut modules: BTreeMap<String, ParsedLexModule> = BTreeMap::new();
    let mut duplicate_decls: u64 = 0;

    for path in &files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let parsed = parse_lex_module(path, &rel, &mut diagnostics, &mut duplicate_decls)?;
        if modules.contains_key(&parsed.module_name) {
            duplicate_decls = duplicate_decls.saturating_add(1);
            diagnostics.push(format!(
                "duplicate module `{}` at {}",
                parsed.module_name, parsed.rel_path
            ));
        } else {
            modules.insert(parsed.module_name.clone(), parsed);
        }
    }

    let (dag_acyclic, topo_order) = topological_sort_modules(&modules, &mut diagnostics);
    let all_roots_executable = check_executable_closure(&modules, &topo_order, &mut diagnostics);
    let (breaks_checked, breaks_triggered, verify_passed, verify_total) =
        sweep_predicates_and_theorems(&modules, &topo_order, &mut diagnostics);

    let verdict = admit_mechanizer_graph(MechanizerCheck {
        dag_acyclic,
        all_roots_executable,
        duplicate_decls,
        breaks_triggered,
        verify_passed,
        verify_total,
    });

    let ordered_modules = topo_order
        .iter()
        .filter_map(|name| modules.get(name))
        .map(|m| MechanizerModuleSummary {
            module_name: m.module_name.clone(),
            path: m.rel_path.clone(),
            imports: m.imports.clone(),
            declaration_count: m.declarations.len(),
        })
        .collect();

    let report = MechanizerReport {
        ordered_modules,
        dag_acyclic,
        all_roots_executable,
        duplicate_decls,
        breaks_checked,
        breaks_triggered,
        verify_passed,
        verify_total,
        verdict: format!("{verdict:?}"),
        diagnostics,
    };

    if verdict != MechanizerVerdict::ClosureAdmitted {
        let detail = if report.diagnostics.is_empty() {
            String::from("no additional diagnostics")
        } else {
            report.diagnostics.join("; ")
        };
        return Err(format!(
            "mechanizer refused closure ({}): {}",
            report.verdict, detail
        )
        .into());
    }
    Ok(report)
}

fn collect_lex_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Fail> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|de| de.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_lex_files(&path, out)?;
        } else if path.to_string_lossy().ends_with(".lex.tex") {
            out.push(path);
        }
    }
    Ok(())
}

fn parse_lex_module(
    path: &Path,
    rel_path: &str,
    diagnostics: &mut Vec<String>,
    duplicate_decls: &mut u64,
) -> Result<ParsedLexModule, Fail> {
    let raw = std::fs::read_to_string(path)?;
    let module_name = raw
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("\\begin{lexlean}{")
                .and_then(|s| s.strip_suffix('}'))
                .map(str::trim)
        })
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{rel_path}: missing \\begin{{lexlean}}{{...}} header"))?
        .to_string();

    let mut imports = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("\\importmodule{") {
            if let Some(imp) = rest.strip_suffix('}') {
                let imp = imp.trim();
                if !imp.is_empty() {
                    imports.push(imp.to_string());
                }
            }
        }
    }

    let json_str = extract_semantic_json(&raw).ok_or_else(|| {
        format!("{rel_path}: missing \\semanticdata{{...}} or \\coredata{{...}} JSON AST")
    })?;
    let ast: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("{rel_path}: invalid semantic AST JSON: {e}"))?;
    let spec = ast["spec"].as_str().unwrap_or("");
    if spec != "lexlean/semantic-module/2" && spec != "lexlean/core-module/2" {
        return Err(format!("{rel_path}: unexpected spec `{spec}`").into());
    }

    let declarations = ast["declarations"]
        .as_array()
        .ok_or_else(|| format!("{rel_path}: `declarations` must be a JSON array"))?
        .clone();

    let mut decls_by_name = BTreeMap::new();
    let mut inductive_nullary_ctors = BTreeMap::new();

    for decl in &declarations {
        let Some(name) = decl["name"].as_str().filter(|s| !s.is_empty()) else {
            diagnostics.push(format!("{rel_path}: declaration missing `name`"));
            continue;
        };
        if decls_by_name.contains_key(name) {
            *duplicate_decls = duplicate_decls.saturating_add(1);
            diagnostics.push(format!(
                "{rel_path}: duplicate declaration `{name}` in module `{module_name}`"
            ));
        } else {
            decls_by_name.insert(name.to_string(), decl.clone());
        }
        if decl["kind"].as_str() == Some("inductive") {
            let mut ctors = Vec::new();
            if let Some(arr) = decl["constructors"].as_array() {
                for ctor in arr {
                    let has_fields = ctor["fields"].as_array().is_some_and(|f| !f.is_empty());
                    if !has_fields {
                        if let Some(cname) = ctor["name"].as_str() {
                            ctors.push(format!("{name}.{cname}"));
                        }
                    }
                }
            }
            if !ctors.is_empty() {
                inductive_nullary_ctors.insert(name.to_string(), ctors);
            }
        }
    }

    Ok(ParsedLexModule {
        module_name,
        rel_path: rel_path.to_string(),
        imports,
        declarations,
        decls_by_name,
        inductive_nullary_ctors,
    })
}

fn extract_semantic_json(raw: &str) -> Option<&str> {
    if let Some((_, after)) = raw.split_once("\\semanticdata{") {
        let block = after
            .split_once("\n\\end{semanticmodule}")
            .map(|(b, _)| b)
            .unwrap_or(after);
        return Some(block.trim().strip_suffix('}').unwrap_or(block.trim()));
    }
    if let Some((_, after)) = raw.split_once("\\coredata{") {
        let block = after
            .split_once("\n\\end{coremodule}")
            .map(|(b, _)| b)
            .unwrap_or(after);
        return Some(block.trim().strip_suffix('}').unwrap_or(block.trim()));
    }
    None
}

fn topological_sort_modules(
    modules: &BTreeMap<String, ParsedLexModule>,
    diagnostics: &mut Vec<String>,
) -> (bool, Vec<String>) {
    let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in modules.keys() {
        indegree.insert(name.clone(), 0);
        dependents.insert(name.clone(), BTreeSet::new());
    }

    for (mod_name, m) in modules {
        let mut unique_imports = BTreeSet::new();
        for imp in &m.imports {
            if modules.contains_key(imp) && unique_imports.insert(imp.clone()) {
                if let Some(deg) = indegree.get_mut(mod_name) {
                    *deg += 1;
                }
                if let Some(deps) = dependents.get_mut(imp) {
                    deps.insert(mod_name.clone());
                }
            }
        }
    }

    let mut ready = BTreeSet::new();
    for (name, deg) in &indegree {
        if *deg == 0 {
            ready.insert(name.clone());
        }
    }

    let mut queue = VecDeque::new();
    for name in ready {
        queue.push_back(name);
    }

    let mut ordered = Vec::with_capacity(modules.len());
    while let Some(curr) = queue.pop_front() {
        ordered.push(curr.clone());
        if let Some(deps) = dependents.get(&curr) {
            let mut newly_ready = BTreeSet::new();
            for dep in deps {
                if let Some(d) = indegree.get_mut(dep) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        newly_ready.insert(dep.clone());
                    }
                }
            }
            for item in newly_ready {
                queue.push_back(item);
            }
        }
    }

    let acyclic = ordered.len() == modules.len();
    if !acyclic {
        let placed: BTreeSet<&str> = ordered.iter().map(String::as_str).collect();
        let cyclic: Vec<&str> = modules
            .keys()
            .filter(|k| !placed.contains(k.as_str()))
            .map(String::as_str)
            .collect();
        diagnostics.push(format!(
            "cycle detected in module import DAG: {}",
            cyclic.join(", ")
        ));
    }
    (acyclic, ordered)
}

fn check_executable_closure(
    modules: &BTreeMap<String, ParsedLexModule>,
    topo_order: &[String],
    diagnostics: &mut Vec<String>,
) -> bool {
    let mut ok = true;
    for mod_name in topo_order {
        let Some(m) = modules.get(mod_name) else {
            continue;
        };
        let mut non_exec_defs: BTreeSet<String> = BTreeSet::new();
        for decl in &m.declarations {
            let kind = decl["kind"].as_str().unwrap_or("");
            let name = decl["name"].as_str().unwrap_or("");
            if kind == "axiom" || kind == "opaque" {
                non_exec_defs.insert(name.to_string());
                if m.rel_path.starts_with("src/R4/") {
                    ok = false;
                    diagnostics.push(format!(
                        "{}::{}: `{kind}` declaration violates executable closure",
                        m.module_name, name
                    ));
                }
            } else if kind == "definition"
                && (decl.get("body").is_none()
                    || expr_contains_forbidden_kind(&decl["body"], "sorry"))
            {
                non_exec_defs.insert(name.to_string());
                ok = false;
                diagnostics.push(format!(
                    "{}::{}: definition missing executable body or contains `sorry`",
                    m.module_name, name
                ));
            }
        }
        // Verify cross-module calls from executable definitions & theorems resolve cleanly.
        for decl in &m.declarations {
            let kind = decl["kind"].as_str().unwrap_or("");
            let name = decl["name"].as_str().unwrap_or("");
            let root_expr = match kind {
                "definition" => &decl["body"],
                "theorem" => &decl["statement"],
                _ => continue,
            };
            let mut refs = Vec::new();
            collect_call_targets(root_expr, &mut refs);
            for (target_mod, target_fn) in refs {
                if let Some(tm) = target_mod {
                    if let Some(target_module) = modules.get(&tm) {
                        if !target_module.decls_by_name.contains_key(&target_fn) {
                            ok = false;
                            diagnostics.push(format!(
                                "{}::{}: calls missing symbol `{tm}::{target_fn}`",
                                m.module_name, name
                            ));
                        }
                    } else if modules
                        .values()
                        .any(|other| other.rel_path.starts_with("src/R4/"))
                        && tm.starts_with("R4.")
                    {
                        ok = false;
                        diagnostics.push(format!(
                            "{}::{}: imports or calls missing module `{tm}`",
                            m.module_name, name
                        ));
                    }
                } else if non_exec_defs.contains(&target_fn) {
                    ok = false;
                    diagnostics.push(format!(
                        "{}::{}: depends on non-executable symbol `{target_fn}`",
                        m.module_name, name
                    ));
                }
            }
        }
    }
    ok
}

fn expr_contains_forbidden_kind(expr: &Value, needle: &str) -> bool {
    match expr {
        Value::Object(map) => {
            if map.get("kind").and_then(Value::as_str) == Some(needle) {
                return true;
            }
            map.values()
                .any(|v| expr_contains_forbidden_kind(v, needle))
        }
        Value::Array(arr) => arr.iter().any(|v| expr_contains_forbidden_kind(v, needle)),
        _ => false,
    }
}

fn collect_call_targets(expr: &Value, out: &mut Vec<(Option<String>, String)>) {
    match expr {
        Value::Object(map) => {
            if map.get("kind").and_then(Value::as_str) == Some("call") {
                if let Some(fn_obj) = map.get("function") {
                    if let Some(fn_name) = fn_obj["name"].as_str() {
                        let mod_name = fn_obj["module"].as_str().map(str::to_string);
                        out.push((mod_name, fn_name.to_string()));
                    }
                }
            }
            for v in map.values() {
                collect_call_targets(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_call_targets(v, out);
            }
        }
        _ => {}
    }
}

fn sweep_predicates_and_theorems(
    modules: &BTreeMap<String, ParsedLexModule>,
    topo_order: &[String],
    diagnostics: &mut Vec<String>,
) -> (u64, u64, u64, u64) {
    let mut breaks_checked: u64 = 0;
    let mut breaks_triggered: u64 = 0;
    let mut verify_passed: u64 = 0;
    let mut verify_total: u64 = 0;

    let strict_control_files = [
        "tests/lexlean/GeometricRecompilerControls.lex.tex",
        "tests/lexlean/VerdantSynthesisControls.lex.tex",
        "tests/lexlean/HonestPlumbingControls.lex.tex",
    ];

    for mod_name in topo_order {
        let Some(m) = modules.get(mod_name) else {
            continue;
        };
        for decl in &m.declarations {
            let kind = decl["kind"].as_str().unwrap_or("");
            let name = decl["name"].as_str().unwrap_or("");

            if kind == "definition"
                && name.starts_with("breaks")
                && decl["result"]["kind"].as_str() == Some("bool")
            {
                breaks_checked = breaks_checked.saturating_add(1);
                let Some(params) = decl["parameters"].as_array() else {
                    continue;
                };
                let grid = build_counterexample_grid(m, params, 64);
                for args in grid {
                    let mut env = BTreeMap::new();
                    for (p, val) in params.iter().zip(args) {
                        if let Some(pname) = p["name"].as_str() {
                            env.insert(pname.to_string(), val);
                        }
                    }
                    match eval_expr(modules, m, &decl["body"], &env, 5_000) {
                        Ok(AstVal::Bool(true)) => {
                            breaks_triggered = breaks_triggered.saturating_add(1);
                            diagnostics.push(format!(
                                "{}::{}: breaks* counterexample predicate evaluated true!",
                                m.module_name, name
                            ));
                            break;
                        }
                        Ok(AstVal::Bool(false)) => {}
                        Ok(other) => {
                            breaks_triggered = breaks_triggered.saturating_add(1);
                            diagnostics.push(format!(
                                "{}::{}: breaks* returned non-bool `{other:?}`",
                                m.module_name, name
                            ));
                            break;
                        }
                        Err(err) => {
                            breaks_triggered = breaks_triggered.saturating_add(1);
                            diagnostics.push(format!(
                                "{}::{}: breaks* evaluation failed: {err}",
                                m.module_name, name
                            ));
                            break;
                        }
                    }
                }
            } else if kind == "definition"
                && name.starts_with("verify")
                && decl["result"]["kind"].as_str() == Some("bool")
                && decl["parameters"].as_array().is_some_and(Vec::is_empty)
            {
                verify_total = verify_total.saturating_add(1);
                let env = BTreeMap::new();
                match eval_expr(modules, m, &decl["body"], &env, 5_000) {
                    Ok(AstVal::Bool(true)) => verify_passed = verify_passed.saturating_add(1),
                    Ok(other) => diagnostics.push(format!(
                        "{}::{}: verify* returned `{other:?}` instead of true",
                        m.module_name, name
                    )),
                    Err(err) => diagnostics.push(format!(
                        "{}::{}: verify* evaluation failed: {err}",
                        m.module_name, name
                    )),
                }
            } else if kind == "theorem" && strict_control_files.contains(&m.rel_path.as_str()) {
                let stmt = &decl["statement"];
                if stmt["kind"].as_str() == Some("eq")
                    && decl["parameters"].as_array().is_some_and(Vec::is_empty)
                {
                    verify_total = verify_total.saturating_add(1);
                    let env = BTreeMap::new();
                    let lhs = eval_expr(modules, m, &stmt["left"], &env, 5_000);
                    let rhs = eval_expr(modules, m, &stmt["right"], &env, 5_000);
                    match (lhs, rhs) {
                        (Ok(l), Ok(r)) if values_equal(&l, &r) => {
                            verify_passed = verify_passed.saturating_add(1);
                        }
                        (l, r) => diagnostics.push(format!(
                            "{}::{}: reflexivity theorem failed: lhs={l:?}, rhs={r:?}",
                            m.module_name, name
                        )),
                    }
                }
            }
        }
    }

    (
        breaks_checked,
        breaks_triggered,
        verify_passed,
        verify_total,
    )
}

fn build_counterexample_grid(
    module: &ParsedLexModule,
    params: &[Value],
    max_combos: usize,
) -> Vec<Vec<AstVal>> {
    if params.is_empty() {
        return vec![vec![]];
    }
    let mut domains: Vec<Vec<AstVal>> = Vec::with_capacity(params.len());
    for p in params {
        let ptype = &p["type"];
        let candidates = match ptype["kind"].as_str() {
            Some("bool") => vec![AstVal::Bool(false), AstVal::Bool(true)],
            Some("nat") => vec![
                AstVal::Nat(0),
                AstVal::Nat(1),
                AstVal::Nat(2),
                AstVal::Nat(3),
                AstVal::Nat(16),
            ],
            Some("named") => {
                if let Some(tname) = ptype["member"]["name"].as_str() {
                    if let Some(ctors) = module.inductive_nullary_ctors.get(tname) {
                        ctors
                            .iter()
                            .map(|c| AstVal::Ctor(c.clone(), Vec::new()))
                            .collect()
                    } else {
                        vec![AstVal::Nat(0), AstVal::Nat(1)]
                    }
                } else {
                    vec![AstVal::Nat(0)]
                }
            }
            _ => vec![AstVal::Nat(0)],
        };
        domains.push(candidates);
    }

    let mut acc: Vec<Vec<AstVal>> = vec![Vec::new()];
    for dom in domains {
        let mut next = Vec::new();
        for prefix in &acc {
            for val in &dom {
                if next.len() >= max_combos {
                    return next;
                }
                let mut row = prefix.clone();
                row.push(val.clone());
                next.push(row);
            }
        }
        acc = next;
    }
    acc
}

fn eval_expr(
    modules: &BTreeMap<String, ParsedLexModule>,
    curr_mod: &ParsedLexModule,
    expr: &Value,
    env: &BTreeMap<String, AstVal>,
    fuel: usize,
) -> Result<AstVal, String> {
    if fuel == 0 {
        return Err("evaluation fuel exhausted".to_string());
    }
    let kind = expr["kind"]
        .as_str()
        .ok_or_else(|| format!("missing AST expr `kind` in {expr}"))?;

    match kind {
        "bool" => Ok(AstVal::Bool(expr["value"].as_bool().ok_or("bad bool")?)),
        "nat" => {
            let raw = expr["value"]
                .as_str()
                .ok_or_else(|| format!("bad nat expr: {expr}"))?;
            let n = raw
                .parse::<u128>()
                .map_err(|e| format!("bad nat literal `{raw}`: {e}"))?;
            Ok(AstVal::Nat(n))
        }
        "var" => {
            let name = expr["name"].as_str().ok_or("var missing name")?;
            if let Some(v) = env.get(name) {
                Ok(v.clone())
            } else if let Some(decl) = curr_mod.decls_by_name.get(name) {
                if decl["kind"].as_str() == Some("definition") {
                    eval_expr(modules, curr_mod, &decl["body"], &BTreeMap::new(), fuel - 1)
                } else {
                    Err(format!("unbound variable `{name}`"))
                }
            } else {
                Err(format!("unbound variable `{name}`"))
            }
        }
        "not" => match eval_expr(modules, curr_mod, &expr["value"], env, fuel - 1)? {
            AstVal::Bool(b) => Ok(AstVal::Bool(!b)),
            other => Err(format!("`not` expected Bool, got {other:?}")),
        },
        "and" => match eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)? {
            AstVal::Bool(false) => Ok(AstVal::Bool(false)),
            AstVal::Bool(true) => eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1),
            other => Err(format!("`and` expected Bool, got {other:?}")),
        },
        "or" => match eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)? {
            AstVal::Bool(true) => Ok(AstVal::Bool(true)),
            AstVal::Bool(false) => eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1),
            other => Err(format!("`or` expected Bool, got {other:?}")),
        },
        "beq" => {
            let l = eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)?;
            let r = eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1)?;
            Ok(AstVal::Bool(values_equal(&l, &r)))
        }
        "blt" => {
            let l = as_nat(eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)?)?;
            let r = as_nat(eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1)?)?;
            Ok(AstVal::Bool(l < r))
        }
        "ble" => {
            let l = as_nat(eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)?)?;
            let r = as_nat(eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1)?)?;
            Ok(AstVal::Bool(l <= r))
        }
        "add" => {
            let l = as_nat(eval_expr(modules, curr_mod, &expr["left"], env, fuel - 1)?)?;
            let r = as_nat(eval_expr(modules, curr_mod, &expr["right"], env, fuel - 1)?)?;
            Ok(AstVal::Nat(l.saturating_add(r)))
        }
        "primitive" => {
            let op = expr["operation"].as_str().ok_or("primitive missing op")?;
            let args = expr["arguments"]
                .as_array()
                .ok_or("primitive missing args")?;
            if args.len() < 2 {
                return Err(format!("primitive `{op}` needs >= 2 args"));
            }
            let a = as_nat(eval_expr(modules, curr_mod, &args[0], env, fuel - 1)?)?;
            let b = as_nat(eval_expr(modules, curr_mod, &args[1], env, fuel - 1)?)?;
            let res = match op {
                "subtract" => a.saturating_sub(b),
                "multiply" => a.saturating_mul(b),
                "quotient" => a.checked_div(b).unwrap_or(0),
                "remainder" => {
                    if b == 0 {
                        a
                    } else {
                        a % b
                    }
                }
                _ => return Err(format!("unsupported primitive op `{op}`")),
            };
            Ok(AstVal::Nat(res))
        }
        "if" => match eval_expr(modules, curr_mod, &expr["condition"], env, fuel - 1)? {
            AstVal::Bool(true) => eval_expr(modules, curr_mod, &expr["then_value"], env, fuel - 1),
            AstVal::Bool(false) => eval_expr(modules, curr_mod, &expr["else_value"], env, fuel - 1),
            other => Err(format!("`if` condition expected Bool, got {other:?}")),
        },
        "let" => {
            let name = expr["name"].as_str().ok_or("let missing name")?;
            let val = eval_expr(modules, curr_mod, &expr["value"], env, fuel - 1)?;
            let mut next_env = env.clone();
            next_env.insert(name.to_string(), val);
            eval_expr(modules, curr_mod, &expr["body"], &next_env, fuel - 1)
        }
        "constructor" => {
            let cname = expr["constructor"]["name"]
                .as_str()
                .ok_or("constructor missing name")?;
            let mut vals = Vec::new();
            if let Some(args) = expr["arguments"].as_array() {
                for a in args {
                    vals.push(eval_expr(modules, curr_mod, a, env, fuel - 1)?);
                }
            }
            Ok(AstVal::Ctor(cname.to_string(), vals))
        }
        "record" => {
            let fields = expr["fields"].as_array().ok_or("record missing fields")?;
            let mut map = BTreeMap::new();
            for f in fields {
                let fname = f["field"].as_str().ok_or("record field missing name")?;
                let val = eval_expr(modules, curr_mod, &f["value"], env, fuel - 1)?;
                map.insert(fname.to_string(), val);
            }
            Ok(AstVal::Record(map))
        }
        "project" => {
            let field = expr["field"].as_str().ok_or("project missing field")?;
            match eval_expr(modules, curr_mod, &expr["value"], env, fuel - 1)? {
                AstVal::Record(map) => map
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("missing record field `{field}`")),
                other => Err(format!("`project` expected Record, got {other:?}")),
            }
        }
        "match" => {
            let scrut = eval_expr(modules, curr_mod, &expr["scrutinee"], env, fuel - 1)?;
            let AstVal::Ctor(actual_ctor, actual_args) = scrut else {
                return Err(format!("`match` scrutinee must be Ctor, got {scrut:?}"));
            };
            let actual_short = short_ctor_name(&actual_ctor);
            let branches = expr["branches"]
                .as_array()
                .ok_or("match missing branches")?;
            for br in branches {
                let target_ctor = br["constructor"]["name"].as_str().unwrap_or("");
                if target_ctor == actual_ctor || short_ctor_name(target_ctor) == actual_short {
                    let mut next_env = env.clone();
                    if let Some(binders) = br["binders"].as_array() {
                        for (b, v) in binders.iter().zip(actual_args) {
                            if let Some(bname) = b.as_str() {
                                next_env.insert(bname.to_string(), v);
                            }
                        }
                    }
                    return eval_expr(modules, curr_mod, &br["body"], &next_env, fuel - 1);
                }
            }
            Err(format!("non-exhaustive match on `{actual_ctor}`"))
        }
        "cons" => {
            let head = eval_expr(modules, curr_mod, &expr["head"], env, fuel - 1)?;
            match eval_expr(modules, curr_mod, &expr["tail"], env, fuel - 1)? {
                AstVal::List(mut items) => {
                    items.insert(0, head);
                    Ok(AstVal::List(items))
                }
                other => Err(format!("`cons` tail expected List, got {other:?}")),
            }
        }
        "nil" => Ok(AstVal::List(Vec::new())),
        "call" => {
            let fn_name = expr["function"]["name"]
                .as_str()
                .ok_or("call missing function name")?;
            let target_mod = if let Some(mname) = expr["function"]["module"].as_str() {
                modules
                    .get(mname)
                    .ok_or_else(|| format!("unknown module `{mname}` in call to `{fn_name}`"))?
            } else {
                curr_mod
            };
            let decl = target_mod.decls_by_name.get(fn_name).ok_or_else(|| {
                format!(
                    "unknown definition `{fn_name}` in module `{}`",
                    target_mod.module_name
                )
            })?;
            let params = decl["parameters"].as_array().ok_or("def missing params")?;
            let args = expr["arguments"].as_array().ok_or("call missing args")?;
            if params.len() != args.len() {
                return Err(format!(
                    "arity mismatch calling `{fn_name}`: expected {}, got {}",
                    params.len(),
                    args.len()
                ));
            }
            let mut next_env = BTreeMap::new();
            for (p, a) in params.iter().zip(args.iter()) {
                let pname = p["name"].as_str().ok_or("param missing name")?;
                let aval = eval_expr(modules, curr_mod, a, env, fuel - 1)?;
                next_env.insert(pname.to_string(), aval);
            }
            eval_expr(modules, target_mod, &decl["body"], &next_env, fuel - 1)
        }
        _ => Err(format!("unsupported AST expr kind `{kind}`")),
    }
}

fn short_ctor_name(full: &str) -> &str {
    full.rsplit('.').next().unwrap_or(full)
}

fn as_nat(v: AstVal) -> Result<u128, String> {
    match v {
        AstVal::Nat(n) => Ok(n),
        other => Err(format!("expected Nat, got {other:?}")),
    }
}

fn values_equal(l: &AstVal, r: &AstVal) -> bool {
    match (l, r) {
        (AstVal::Bool(a), AstVal::Bool(b)) => a == b,
        (AstVal::Nat(a), AstVal::Nat(b)) => a == b,
        (AstVal::Ctor(an, aa), AstVal::Ctor(bn, ba)) => {
            (an == bn || short_ctor_name(an) == short_ctor_name(bn))
                && aa.len() == ba.len()
                && aa.iter().zip(ba.iter()).all(|(x, y)| values_equal(x, y))
        }
        (AstVal::Record(a), AstVal::Record(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|ov| values_equal(v, ov)))
        }
        (AstVal::List(a), AstVal::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lexlean-mechanizer-{tag}-{nanos}"));
        std::fs::create_dir_all(dir.join("src/R4")).expect("create src/R4");
        std::fs::create_dir_all(dir.join("tests/lexlean")).expect("create tests/lexlean");
        dir
    }

    fn write_module(root: &Path, rel: &str, name: &str, imports: &[&str], json: &str) {
        let mut content =
            format!("\\begin{{lexlean}}{{{name}}}\n\\useglossary{{lexlean.std.bool@1.2.0}}\n");
        for imp in imports {
            content.push_str(&format!("\\importmodule{{{imp}}}\n"));
        }
        content.push_str(&format!(
            "\\begin{{semanticmodule}}\n\\semanticdata{{{json}}}\n\\end{{semanticmodule}}\n\\end{{lexlean}}\n"
        ));
        std::fs::write(root.join(rel), content).expect("write module");
    }

    #[test]
    fn live_workspace_passes_mechanize_gate() {
        let root = repo_model::repo_root();
        let report = run_mechanizer_gate(&root).expect("live repository must pass mechanizer");
        assert!(report.dag_acyclic);
        assert!(report.all_roots_executable);
        assert_eq!(report.duplicate_decls, 0);
        assert_eq!(report.breaks_triggered, 0);
        assert!(report.breaks_checked >= 3);
        assert!(report.verify_total >= 10);
        assert_eq!(report.verify_passed, report.verify_total);
        assert_eq!(report.verdict, "ClosureAdmitted");
    }

    #[test]
    fn planted_cycle_breaks_and_duplicate_decls_are_rejected() {
        // 1. Cycle in import DAG -> RefusedCycleInModuleGraph.
        let dir = make_temp_dir("cycle");
        let min_json = r#"{"declarations":[{"body":{"kind":"bool","value":true},"kind":"definition","name":"verifyOk","parameters":[],"result":{"kind":"bool"}}],"spec":"lexlean/semantic-module/2"}"#;
        write_module(&dir, "src/R4/A.lex.tex", "R4.A", &["R4.B"], min_json);
        write_module(&dir, "src/R4/B.lex.tex", "R4.B", &["R4.A"], min_json);
        let err = mechanize_directory(&dir, &["src/R4"])
            .expect_err("cyclic import graph must be rejected")
            .to_string();
        assert!(err.contains("RefusedCycleInModuleGraph"), "got {err}");
        let _ = std::fs::remove_dir_all(&dir);

        // 2. Duplicate declaration -> RefusedDuplicateDeclaration.
        let dir = make_temp_dir("dup");
        let dup_json = r#"{"declarations":[{"body":{"kind":"bool","value":true},"kind":"definition","name":"verifyDup","parameters":[],"result":{"kind":"bool"}},{"body":{"kind":"bool","value":true},"kind":"definition","name":"verifyDup","parameters":[],"result":{"kind":"bool"}}],"spec":"lexlean/semantic-module/2"}"#;
        write_module(&dir, "src/R4/Dup.lex.tex", "R4.Dup", &[], dup_json);
        let err = mechanize_directory(&dir, &["src/R4"])
            .expect_err("duplicate declaration must be rejected")
            .to_string();
        assert!(err.contains("RefusedDuplicateDeclaration"), "got {err}");
        let _ = std::fs::remove_dir_all(&dir);

        // 3. breaks* evaluating to true -> RefusedCounterexampleTriggered.
        let dir = make_temp_dir("breaks");
        let bad_breaks = r#"{"declarations":[{"body":{"kind":"bool","value":true},"kind":"definition","name":"verifyOk","parameters":[],"result":{"kind":"bool"}},{"body":{"kind":"blt","left":{"kind":"nat","value":"0"},"right":{"kind":"var","name":"n"}},"kind":"definition","name":"breaksWeakenedAdmission","parameters":[{"name":"n","type":{"kind":"nat"}}],"result":{"kind":"bool"}}],"spec":"lexlean/semantic-module/2"}"#;
        write_module(&dir, "src/R4/Breaks.lex.tex", "R4.Breaks", &[], bad_breaks);
        let err = mechanize_directory(&dir, &["src/R4"])
            .expect_err("breaks* returning true must be rejected")
            .to_string();
        assert!(err.contains("RefusedCounterexampleTriggered"), "got {err}");
        let _ = std::fs::remove_dir_all(&dir);

        // 4. sorry inside definition body -> RefusedNonExecutableRoot.
        let dir = make_temp_dir("sorry");
        let sorry_json = r#"{"declarations":[{"body":{"kind":"sorry"},"kind":"definition","name":"verifyStub","parameters":[],"result":{"kind":"bool"}}],"spec":"lexlean/semantic-module/2"}"#;
        write_module(&dir, "src/R4/Sorry.lex.tex", "R4.Sorry", &[], sorry_json);
        let err = mechanize_directory(&dir, &["src/R4"])
            .expect_err("sorry root must be rejected")
            .to_string();
        assert!(err.contains("RefusedNonExecutableRoot"), "got {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
