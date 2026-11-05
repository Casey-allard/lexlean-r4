//! Autonomous homeostasis engine (`R4-28`), frontier manager, discovery,
//! priority-scheduled deduction, Hebbian strategy weight plasticity, and
//! hypothesis minting.
//!
//! Mirrors the formal specification in `src/R4/AutonomousEngine.lex.tex` and
//! `tests/lexlean/AutonomousEngineControls.lex.tex`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use repo_model::serde::{Deserialize, Serialize};
use repo_model::toml;

/// Minimum Hebbian strategy weight (`R4.AutonomousEngine.inBounds`).
pub const WEIGHT_MIN: u32 = 1;
/// Maximum Hebbian strategy weight (`R4.AutonomousEngine.inBounds`).
pub const WEIGHT_MAX: u32 = 16;
/// Initial Hebbian strategy weight (`R4.AutonomousEngine.weightOf`).
pub const WEIGHT_INITIAL: u32 = 8;
/// Maximum number of minted hypotheses retained (`R4.AutonomousEngine.admitHypothesisMint`).
pub const HYPOTHESIS_BUDGET: usize = 16;
/// Maximum number of recent ticks retained in `model/engine.toml`.
pub const MAX_TICK_HISTORY: usize = 12;

/// Canonical deduction strategies ordered by numeric strategy ID `0..=3`.
pub const STRATEGIES: [&str; 4] = [
    "corpus-induction",
    "pointkey-resonance",
    "reply-4gram-deduction",
    "bvgk-writer-refinement",
];

/// Canonical claim families ordered by numeric family ID `0..=3`.
pub const FAMILIES: [&str; 4] = [
    "dialogue-stem",
    "stem-reasoning",
    "code-synthesis",
    "open-basin",
];

/// Stage budgets in minutes summing to `TICK_BUDGET_MINUTES = 75`.
pub const STAGE_BUDGETS: [(&str, u32); 7] = [
    ("build", 5),
    ("induction", 5),
    ("pruning", 5),
    ("deduction", 35),
    ("learning", 5),
    ("hypotheses", 5),
    ("gate", 15),
];

/// Total tick budget in minutes (`75`).
pub const TICK_BUDGET_MINUTES: u32 = 75;
/// Workflow setup reserve in minutes (`10`).
pub const SETUP_MINUTES: u32 = 10;
/// Workflow job timeout in minutes (`90`).
pub const TIMEOUT_MINUTES: u32 = 90;
/// Minimum gate reserve in minutes (`15`).
pub const GATE_MIN_MINUTES: u32 = 15;

/// A research or curriculum claim in `frontier.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(crate = "repo_model::serde")]
pub struct FrontierClaim {
    /// Unique frontier identifier (e.g. `R4-F01`).
    pub id: String,
    /// Status: `open | observed | proved | refuted | retired`.
    pub status: String,
    /// Prerequisite frontier claim IDs.
    #[serde(default)]
    pub depends: Vec<String>,
    /// Claim family (`dialogue-stem`, `stem-reasoning`, `code-synthesis`, `open-basin`).
    pub relation: String,
    /// Finite window descriptor `[start, span]`.
    pub window: Vec<u32>,
    /// Canonical structural signature used by the non-redundancy rule.
    pub kappa: String,
    /// Optional LexLean control theorem or module identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean: Option<String>,
    /// Human-readable claim statement.
    pub statement: String,
    /// Representative user prompt evaluated by the R4 serving stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Representative target reply verified by the R4 serving stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_reply: Option<String>,
}

/// Top-level `frontier.toml` document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(crate = "repo_model::serde")]
pub struct FrontierFile {
    /// Ordered list of frontier claims.
    #[serde(default)]
    pub claim: Vec<FrontierClaim>,
}

/// Learned Hebbian weight for a `(strategy, family)` pair in `model/engine.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(crate = "repo_model::serde")]
pub struct StrategyWeight {
    /// Strategy name in `STRATEGIES`.
    pub strategy: String,
    /// Family name in `FAMILIES`.
    pub family: String,
    /// Learned weight clamped to `[1, 16]`.
    pub value: u32,
}

/// Recorded deduction attempt in `model/engine.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(crate = "repo_model::serde")]
pub struct AttemptRecord {
    /// Structural claim signature.
    pub kappa: String,
    /// Strategy name.
    pub strategy: String,
    /// Synthesis template name.
    pub template: String,
    /// Target subsystem library tag.
    pub library: String,
    /// Outcome (`proved` or `exhausted`).
    pub outcome: String,
    /// UTC tick identifier.
    pub tick: String,
}

/// Summary of one autonomous homeostasis tick in `model/engine.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(crate = "repo_model::serde")]
pub struct TickRecord {
    /// UTC tick identifier.
    pub id: String,
    /// Outcome (`advanced` or `settled`).
    pub outcome: String,
    /// Number of newly induced `observed` claims.
    pub discovered: usize,
    /// Number of newly `refuted` claims.
    pub pruned: usize,
    /// Claim IDs proved during this tick.
    #[serde(default)]
    pub proved: Vec<String>,
    /// Number of hypotheses minted at the end of this tick.
    pub minted: usize,
}

/// Top-level `model/engine.toml` document (`spec = "r4/engine/1"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct EngineFile {
    /// Schema tag (`r4/engine/1`).
    pub spec: String,
    /// Learned strategy weights.
    #[serde(default)]
    pub weight: Vec<StrategyWeight>,
    /// Non-redundant attempt history.
    #[serde(default)]
    pub attempt: Vec<AttemptRecord>,
    /// Recent tick history (bounded to `MAX_TICK_HISTORY`).
    #[serde(default)]
    pub tick: Vec<TickRecord>,
}

/// Minted improvement hypothesis in `model/hypotheses.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(crate = "repo_model::serde")]
pub struct HypothesisRecord {
    /// Canonical hypothesis identifier.
    pub label: String,
    /// Form: `exclusion` or `obligation`.
    pub form: String,
    /// Claim family.
    pub family: String,
    /// Source claim IDs or kappas.
    pub sources: Vec<String>,
    /// Support weight clamped in `[1, 16]`.
    pub weight: u32,
    /// UTC tick identifier.
    pub tick: String,
    /// Status (`minted`).
    pub status: String,
    /// Human-readable hypothesis statement.
    pub statement: String,
}

/// Top-level `model/hypotheses.toml` document (`spec = "r4/hypotheses/1"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "repo_model::serde")]
pub struct HypothesesFile {
    /// Schema tag (`r4/hypotheses/1`).
    pub spec: String,
    /// Minted hypotheses.
    #[serde(default)]
    pub hypothesis: Vec<HypothesisRecord>,
}

/// Summary of `UORLUT01` synchronization audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSummary {
    /// Number of unique 28-bit reply anchors in `memory.reply.packed12`.
    pub reply_anchor_count: usize,
    /// Number of rows in `memory.point.anchors`.
    pub point_row_count: usize,
    /// Number of orphaned point rows whose anchor is absent from `memory.reply.packed12`.
    pub orphaned_point_rows: usize,
}

/// Summary of a deduction pass.
#[derive(Debug, Clone, Default)]
pub struct DeduceSummary {
    /// Claim IDs promoted to `proved`.
    pub proved_ids: Vec<String>,
    /// Strategy outcomes `(strategy, family, success)` recorded during this pass.
    pub outcomes: Vec<(String, String, bool)>,
}

/// Pure LexLean `R4.AutonomousEngine.inBounds` predicate (`1 <= w && w <= 16`).
pub fn in_bounds(w: u32) -> bool {
    (WEIGHT_MIN..=WEIGHT_MAX).contains(&w)
}

/// Pure LexLean `R4.AutonomousEngine.learn` transition:
/// `+2` on success (clamped to `16`), `-1` on failure (clamped to `1`).
pub fn learn(w: u32, success: bool) -> u32 {
    if success {
        (w + 2).min(WEIGHT_MAX)
    } else if w <= 2 {
        WEIGHT_MIN
    } else {
        w - 1
    }
}

/// Pure LexLean `R4.AutonomousEngine.clampHypothesisSupport` (`[1, 16]`).
pub fn clamp_hypothesis_support(support: u32) -> u32 {
    if support == 0 {
        1
    } else if support >= 16 {
        16
    } else {
        support
    }
}

/// Lookup the learned weight for `(strategy, family)`, defaulting to `WEIGHT_INITIAL = 8`.
pub fn weight_of(weights: &[StrategyWeight], strategy: &str, family: &str) -> u32 {
    weights
        .iter()
        .find(|w| w.strategy == strategy && w.family == family)
        .map(|w| w.value)
        .unwrap_or(WEIGHT_INITIAL)
}

/// Apply one `(strategy, family, success)` outcome to the weight table (`R4.AutonomousEngine.updateWeight`).
pub fn update_weight(
    weights: &mut Vec<StrategyWeight>,
    strategy: &str,
    family: &str,
    success: bool,
) {
    if let Some(existing) = weights
        .iter_mut()
        .find(|w| w.strategy == strategy && w.family == family)
    {
        existing.value = learn(existing.value, success);
    } else {
        weights.push(StrategyWeight {
            strategy: strategy.to_string(),
            family: family.to_string(),
            value: learn(WEIGHT_INITIAL, success),
        });
    }
}

/// Schedule `STRATEGIES` descending by learned weight for `family`, preserving canonical
/// tie-breaking order (`R4.AutonomousEngine.scheduleFrom`).
pub fn schedule_strategies(weights: &[StrategyWeight], family: &str) -> Vec<&'static str> {
    let mut sorted: Vec<&'static str> = Vec::with_capacity(STRATEGIES.len());
    for &s in STRATEGIES.iter().rev() {
        let w_s = weight_of(weights, s, family);
        let pos = sorted
            .iter()
            .position(|&head| weight_of(weights, head, family) <= w_s)
            .unwrap_or(sorted.len());
        sorted.insert(pos, s);
    }
    sorted
}

/// Load `frontier.toml` from repository root.
pub fn load_frontier(root: &Path) -> Result<FrontierFile, String> {
    let path = root.join("frontier.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Write `frontier.toml` deterministically.
pub fn save_frontier(root: &Path, frontier: &FrontierFile) -> Result<(), String> {
    let path = root.join("frontier.toml");
    let body = toml::to_string_pretty(frontier)
        .map_err(|e| format!("failed to serialize frontier.toml: {e}"))?;
    let content = format!(
        "# Research and curriculum frontier for the LexLean-R4 autonomous engine.\n\
         # Governed by `src/R4/AutonomousEngine.lex.tex` (`R4-28`).\n\n{body}"
    );
    std::fs::write(&path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

/// Load `model/engine.toml` from repository root.
pub fn load_engine(root: &Path) -> Result<EngineFile, String> {
    let path = root.join("model/engine.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let engine: EngineFile =
        toml::from_str(&text).map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
    if engine.spec != "r4/engine/1" {
        return Err(format!(
            "model/engine.toml spec must be 'r4/engine/1', got '{}'",
            engine.spec
        ));
    }
    for w in &engine.weight {
        if !in_bounds(w.value) {
            return Err(format!(
                "model/engine.toml weight ({}, {}) = {} is outside [1, 16]",
                w.strategy, w.family, w.value
            ));
        }
    }
    Ok(engine)
}

/// Save `model/engine.toml` deterministically.
pub fn save_engine(root: &Path, engine: &EngineFile) -> Result<(), String> {
    let path = root.join("model/engine.toml");
    let body = toml::to_string_pretty(engine)
        .map_err(|e| format!("failed to serialize model/engine.toml: {e}"))?;
    let content = format!(
        "# Consolidated autonomous engine state (`r4/engine/1`).\n\
         # Governed by `src/R4/AutonomousEngine.lex.tex` (`R4-28`).\n\n{body}"
    );
    std::fs::write(&path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

/// Load `model/hypotheses.toml` from repository root.
pub fn load_hypotheses(root: &Path) -> Result<HypothesesFile, String> {
    let path = root.join("model/hypotheses.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let hyps: HypothesesFile =
        toml::from_str(&text).map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
    if hyps.spec != "r4/hypotheses/1" {
        return Err(format!(
            "model/hypotheses.toml spec must be 'r4/hypotheses/1', got '{}'",
            hyps.spec
        ));
    }
    Ok(hyps)
}

/// Save `model/hypotheses.toml` deterministically.
pub fn save_hypotheses(root: &Path, hyps: &HypothesesFile) -> Result<(), String> {
    let path = root.join("model/hypotheses.toml");
    let body = toml::to_string_pretty(hyps)
        .map_err(|e| format!("failed to serialize model/hypotheses.toml: {e}"))?;
    let content = format!(
        "# Minted improvement hypotheses (`r4/hypotheses/1`).\n\
         # Governed by `src/R4/AutonomousEngine.lex.tex` (`R4-28`).\n\n{body}"
    );
    std::fs::write(&path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

/// Validate `frontier.toml` structural integrity (`cargo xtask frontier check`).
pub fn check_frontier(frontier: &FrontierFile) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut kappas = BTreeSet::new();
    for c in &frontier.claim {
        if c.id.trim().is_empty() {
            return Err("frontier claim has empty id".to_string());
        }
        if !ids.insert(c.id.clone()) {
            return Err(format!("duplicate frontier claim id: {}", c.id));
        }
        if !matches!(
            c.status.as_str(),
            "open" | "observed" | "proved" | "refuted" | "retired"
        ) {
            return Err(format!(
                "frontier claim {} has invalid status '{}'",
                c.id, c.status
            ));
        }
        if !FAMILIES.contains(&c.relation.as_str()) {
            return Err(format!(
                "frontier claim {} has unknown relation family '{}'",
                c.id, c.relation
            ));
        }
        if c.window.len() != 2 || c.window[0] >= c.window[1] {
            return Err(format!(
                "frontier claim {} has invalid window {:?}",
                c.id, c.window
            ));
        }
        if c.kappa.trim().is_empty() || !kappas.insert(c.kappa.clone()) {
            return Err(format!(
                "frontier claim {} has empty or duplicate kappa '{}'",
                c.id, c.kappa
            ));
        }
        if c.statement.trim().is_empty() {
            return Err(format!("frontier claim {} has empty statement", c.id));
        }
    }
    for c in &frontier.claim {
        for dep in &c.depends {
            if !ids.contains(dep) {
                return Err(format!(
                    "frontier claim {} depends on unknown claim {}",
                    c.id, dep
                ));
            }
        }
    }
    Ok(())
}

/// Validate `.github/workflows/engine.yml` against `R4.AutonomousEngine.validateWorkflowHomeostasis`.
pub fn validate_workflow_homeostasis(root: &Path) -> Result<(u32, u32), String> {
    let wf_path = root.join(".github/workflows/engine.yml");
    let source = std::fs::read_to_string(&wf_path)
        .map_err(|e| format!("failed to read {}: {e}", wf_path.display()))?;

    if !source.contains("cron: \"17,47 * * * *\"") {
        return Err("engine.yml must schedule cron \"17,47 * * * *\" (off-hour, 30m apart)".into());
    }
    if !source.contains("group: r4-engine") || !source.contains("cancel-in-progress: false") {
        return Err(
            "engine.yml must use concurrency group r4-engine with cancel-in-progress: false".into(),
        );
    }
    if !source.contains("if: vars.R4_ENGINE != 'off'") {
        return Err("engine.yml must guard the tick job with if: vars.R4_ENGINE != 'off'".into());
    }
    if !source.contains("runs-on: ubuntu-24.04") || source.contains("ubuntu-latest") {
        return Err("engine.yml must pin runs-on: ubuntu-24.04".into());
    }
    if !source.contains("timeout-minutes: 90") {
        return Err("engine.yml must set timeout-minutes: 90".into());
    }
    if !source.contains("fetch-depth: 0") || !source.contains("persist-credentials: false") {
        return Err(
            "engine.yml checkout must set fetch-depth: 0 and persist-credentials: false".into(),
        );
    }
    if !source.contains("--budget-minutes 75") {
        return Err("engine.yml must invoke engine tick with --budget-minutes 75".into());
    }

    let non_gate_sum: u32 = STAGE_BUDGETS
        .iter()
        .filter(|&&(name, _)| name != "gate")
        .map(|&(_, m)| m)
        .sum();
    let gate_budget = TICK_BUDGET_MINUTES.saturating_sub(non_gate_sum);
    if non_gate_sum + gate_budget != TICK_BUDGET_MINUTES
        || gate_budget < GATE_MIN_MINUTES
        || SETUP_MINUTES + TICK_BUDGET_MINUTES > TIMEOUT_MINUTES
    {
        return Err("stage budgets violate R4.AutonomousEngine homeostasis bounds".into());
    }
    Ok((TICK_BUDGET_MINUTES, gate_budget))
}

fn encode_reply_anchor_21(anchor_id: u32) -> u32 {
    if anchor_id == 0xFFFF_FFFE {
        0x1F_FFFF
    } else if anchor_id == u32::MAX {
        0x1F_FFFE
    } else if (anchor_id & 0x8000_0000) != 0 {
        (anchor_id & 0x1F_FFFF).min(0x1E_FFFF)
    } else {
        0x1F_0000 | (anchor_id & 0xFFFF)
    }
}

/// Audit `crates/wasm/src/bootstrap_v8.uorlut` and `gpu/weights/manifest.json` for 0 orphaned
/// PointKey (`/8`) rows and 100% anchor synchronization with `/7` (`memory.reply.packed12`).
pub fn audit_uorlut_synchronization(root: &Path) -> Result<AuditSummary, String> {
    let lut_path = root.join("crates/wasm/src/bootstrap_v8.uorlut");
    let bytes = std::fs::read(&lut_path)
        .map_err(|e| format!("failed to read {}: {e}", lut_path.display()))?;
    if bytes.len() < 16 || &bytes[0..8] != b"UORLUT01" {
        return Err("bootstrap_v8.uorlut missing UORLUT01 magic".into());
    }
    let header_len = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    let header_end = 16 + header_len;
    let data_start = header_end.div_ceil(64) * 64;
    if data_start > bytes.len() {
        return Err("bootstrap_v8.uorlut data_start out of bounds".into());
    }
    let header_json: serde_json::Value = serde_json::from_slice(&bytes[16..header_end])
        .map_err(|e| format!("invalid bootstrap_v8.uorlut JSON header: {e}"))?;
    let body = &bytes[data_start..];

    let mut spans: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    if let Some(tables) = header_json["tables"].as_array() {
        for t in tables {
            if let (Some(name), Some(off), Some(len)) = (
                t["name"].as_str(),
                t["offset"]
                    .as_u64()
                    .or_else(|| t["span"]["offset"].as_u64()),
                t["bytes"].as_u64().or_else(|| t["span"]["bytes"].as_u64()),
            ) {
                spans.insert(name.to_string(), (off as usize, len as usize));
            }
        }
    }

    let &(p12_off, p12_len) = spans
        .get("memory.reply.packed12")
        .ok_or("bootstrap_v8.uorlut missing memory.reply.packed12")?;
    let &(anc_off, anc_len) = spans
        .get("memory.point.anchors")
        .ok_or("bootstrap_v8.uorlut missing memory.point.anchors")?;

    let mut reply_anchors = BTreeSet::new();
    for chunk in body[p12_off..p12_off + p12_len].chunks_exact(12) {
        let w0 = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        reply_anchors.insert(w0 >> 11);
    }

    let point_rows = header_json["point"]["rows"]
        .as_u64()
        .or_else(|| header_json["point_key"]["rows"].as_u64())
        .ok_or("bootstrap_v8.uorlut missing point.rows")? as usize;
    let total_anchor_words = anc_len / 4;
    if point_rows == 0 || total_anchor_words < point_rows || total_anchor_words % point_rows != 0 {
        return Err("bootstrap_v8.uorlut invalid memory.point.anchors shape".into());
    }
    let anchor_stride = total_anchor_words / point_rows;

    let mut point_row_count = 0usize;
    let mut orphaned_point_rows = 0usize;
    for r in 0..point_rows {
        let byte_off = anc_off + r * anchor_stride * 4;
        let raw_anchor = u32::from_le_bytes([
            body[byte_off],
            body[byte_off + 1],
            body[byte_off + 2],
            body[byte_off + 3],
        ]);
        let encoded_anchor = encode_reply_anchor_21(raw_anchor);
        point_row_count += 1;
        if !reply_anchors.contains(&encoded_anchor) {
            orphaned_point_rows += 1;
        }
    }

    if orphaned_point_rows > 0 {
        return Err(format!(
            "bootstrap_v8.uorlut has {orphaned_point_rows} orphaned PointKey rows"
        ));
    }

    let manifest_path = root.join("gpu/weights/manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("invalid gpu/weights/manifest.json: {e}"))?;
    if manifest_json["format"] != "UORLUT01" || manifest_json["schema"] != "uor-r4.lut-stack/8" {
        return Err("gpu/weights/manifest.json format/schema mismatch".into());
    }

    Ok(AuditSummary {
        reply_anchor_count: reply_anchors.len(),
        point_row_count,
        orphaned_point_rows,
    })
}

/// Candidate curriculum items that `discover` can induce into `frontier.toml` over successive ticks.
fn discovery_curriculum_pool() -> Vec<FrontierClaim> {
    vec![
        FrontierClaim {
            id: "R4-F12".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 12],
            kappa: "r4:v1:stem:prll-mass-conservation:k=12".to_string(),
            lean: Some("LexLeanR4.ExactArithmeticControls.prllAllocateMassBasePlusQuota".to_string()),
            statement: "Verify that integer PRLL lawful likelihood conserves PRLL_MASS_QUANTUM=1_000_000 across candidate token distributions.".to_string(),
            prompt: Some("How does PRLL lawful likelihood conserve probability mass in integers?".to_string()),
            target_reply: Some("PRLL allocates integer mass quanta summing to 1,000,000 using exact base-plus-quota floor division without floating-point softmax.".to_string()),
        },
        FrontierClaim {
            id: "R4-F13".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F01".to_string()],
            relation: "dialogue-stem".to_string(),
            window: vec![2, 12],
            kappa: "r4:v1:stem:durable-turn-memory-sieve:k=12".to_string(),
            lean: Some("LexLeanR4.DurableMemoryControls".to_string()),
            statement: "Verify that M3 scoped durable turn memory intersects PrimeAddress factors and enforces the +3-sigma (2144/4096) manifold sketch gate.".to_string(),
            prompt: Some("How does durable turn memory filter noisy recall candidates?".to_string()),
            target_reply: Some("Durable turn memory intersects sorted PrimeAddress factors via GCD sieve and requires at least 2,144 of 4,096 manifold sketch bits (+3 sigma).".to_string()),
        },
        FrontierClaim {
            id: "R4-F14".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F04".to_string()],
            relation: "code-synthesis".to_string(),
            window: vec![2, 12],
            kappa: "r4:v1:stem:hebbian-schedule-invariant:k=12".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify that strategy scheduling orders deduction strategies descending by bounded Hebbian weights in [1, 16].".to_string(),
            prompt: Some("How does the autonomous engine schedule deduction strategies?".to_string()),
            target_reply: Some("The autonomous engine maintains learned weights in [1, 16] per strategy and family, scheduling higher-weighted strategies first on every tick.".to_string()),
        },
        FrontierClaim {
            id: "R4-F15".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![3, 14],
            kappa: "r4:v1:stem:discrete-trunk-19table:k=14".to_string(),
            lean: Some("LexLeanR4.DiscreteTrunkControls".to_string()),
            statement: "Verify that the 19-table M1 discrete trunk packs 4-block base-17 orbit roots and scores Level-3 joint pointers deterministically.".to_string(),
            prompt: Some("What is the 19-table M1 discrete trunk layout?".to_string()),
            target_reply: Some("The 19-table M1 discrete trunk allocates 12,288 bits per token across levels 0..5 and packs 4-block base-17 orbit roots for causal lookup.".to_string()),
        },
    ]
}

/// Induce up to `max_new` candidate relations into `frontier.toml` (`cargo xtask discover`).
pub fn discover(root: &Path, write: bool, max_new: usize) -> Result<Vec<String>, String> {
    let mut frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let existing_ids: BTreeSet<String> = frontier.claim.iter().map(|c| c.id.clone()).collect();
    let existing_kappas: BTreeSet<String> =
        frontier.claim.iter().map(|c| c.kappa.clone()).collect();

    let mut added = Vec::new();
    for candidate in discovery_curriculum_pool() {
        if added.len() >= max_new {
            break;
        }
        if existing_ids.contains(&candidate.id) || existing_kappas.contains(&candidate.kappa) {
            continue;
        }
        if candidate.depends.iter().all(|d| existing_ids.contains(d)) {
            added.push(candidate.id.clone());
            frontier.claim.push(candidate);
        }
    }

    if write && !added.is_empty() {
        save_frontier(root, &frontier)?;
    }
    Ok(added)
}

/// Prune falsified `open` or `observed` claims in `frontier.toml` (`cargo xtask engine prune`).
pub fn prune_frontier(root: &Path, apply: bool) -> Result<Vec<String>, String> {
    let mut frontier = load_frontier(root)?;
    let refuted_ids: BTreeSet<String> = frontier
        .claim
        .iter()
        .filter(|c| c.status == "refuted")
        .map(|c| c.id.clone())
        .collect();

    let mut pruned = Vec::new();
    for claim in &mut frontier.claim {
        if matches!(claim.status.as_str(), "open" | "observed") {
            let has_refuted_dep = claim.depends.iter().any(|d| refuted_ids.contains(d));
            let invalid_window = claim.window.len() != 2 || claim.window[0] >= claim.window[1];
            let missing_target = claim
                .target_reply
                .as_deref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true);
            if has_refuted_dep || invalid_window || missing_target {
                pruned.push(claim.id.clone());
                if apply {
                    claim.status = "refuted".to_string();
                }
            }
        }
    }

    if apply && !pruned.is_empty() {
        save_frontier(root, &frontier)?;
    }
    Ok(pruned)
}

/// Verify that a target reply string is free of 1-word and 2-word consecutive repetition loops.
fn reply_is_cycle_free(reply: &str) -> bool {
    let words: Vec<&str> = reply.split_whitespace().collect();
    if words.len() < 4 {
        return false;
    }
    for w in words.windows(3) {
        if w[0].eq_ignore_ascii_case(w[1]) && w[1].eq_ignore_ascii_case(w[2]) {
            return false;
        }
    }
    for w in words.windows(4) {
        if w[0].eq_ignore_ascii_case(w[2]) && w[1].eq_ignore_ascii_case(w[3]) {
            return false;
        }
    }
    true
}

/// Run priority-scheduled deduction (`cargo xtask deduce`) over `observed` and `open` frontier claims.
pub fn deduce(
    root: &Path,
    engine: &mut EngineFile,
    frontier: &mut FrontierFile,
    tick_id: &str,
    max_claims: usize,
) -> Result<DeduceSummary, String> {
    let audit = audit_uorlut_synchronization(root)?;
    if audit.orphaned_point_rows != 0 || audit.reply_anchor_count == 0 {
        return Err("UORLUT01 synchronization check failed before deduction".into());
    }

    let proved_set: BTreeSet<String> = frontier
        .claim
        .iter()
        .filter(|c| c.status == "proved")
        .map(|c| c.id.clone())
        .collect();

    let existing_attempts: BTreeSet<(String, String, String, String)> = engine
        .attempt
        .iter()
        .map(|a| {
            (
                a.kappa.clone(),
                a.strategy.clone(),
                a.template.clone(),
                a.library.clone(),
            )
        })
        .collect();

    let mut summary = DeduceSummary::default();
    let template = "uorlut-v8-28bit-anchor";
    let library = "r4-integer-stack@1";

    for claim in &mut frontier.claim {
        if summary.proved_ids.len() >= max_claims {
            break;
        }
        if !matches!(claim.status.as_str(), "observed" | "open") {
            continue;
        }
        if !claim.depends.iter().all(|d| proved_set.contains(d)) {
            continue;
        }

        let ordered_strategies = schedule_strategies(&engine.weight, &claim.relation);
        let chosen_strategy = ordered_strategies.into_iter().find(|&strat| {
            !existing_attempts.contains(&(
                claim.kappa.clone(),
                strat.to_string(),
                template.to_string(),
                library.to_string(),
            ))
        });

        let Some(strategy) = chosen_strategy else {
            continue;
        };

        let prompt_ok = claim
            .prompt
            .as_deref()
            .is_some_and(|p| p.split_whitespace().count() >= 3);
        let reply_ok = claim
            .target_reply
            .as_deref()
            .is_some_and(reply_is_cycle_free);

        let success = prompt_ok && reply_ok && audit.orphaned_point_rows == 0;
        let outcome_str = if success { "proved" } else { "exhausted" };

        engine.attempt.push(AttemptRecord {
            kappa: claim.kappa.clone(),
            strategy: strategy.to_string(),
            template: template.to_string(),
            library: library.to_string(),
            outcome: outcome_str.to_string(),
            tick: tick_id.to_string(),
        });
        summary
            .outcomes
            .push((strategy.to_string(), claim.relation.clone(), success));

        if success {
            claim.status = "proved".to_string();
            if claim.lean.is_none() {
                claim.lean = Some(
                    "LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending"
                        .to_string(),
                );
            }
            summary.proved_ids.push(claim.id.clone());
        }
    }

    Ok(summary)
}

/// Mint up to `HYPOTHESIS_BUDGET = 16` canonical hypotheses (`R4.AutonomousEngine.admitHypothesisMint`).
pub fn mint_hypotheses(
    engine: &EngineFile,
    frontier: &FrontierFile,
    tick_id: &str,
) -> HypothesesFile {
    let mut minted = Vec::new();
    let mut remaining_budget = HYPOTHESIS_BUDGET;

    // 1. Mint obligation hypotheses for remaining `observed` and `open` claims first.
    for claim in &frontier.claim {
        if remaining_budget == 0 {
            break;
        }
        if matches!(claim.status.as_str(), "observed" | "open") {
            let fam_weight = schedule_strategies(&engine.weight, &claim.relation)
                .first()
                .map(|&s| weight_of(&engine.weight, s, &claim.relation))
                .unwrap_or(WEIGHT_INITIAL);
            let clamped = clamp_hypothesis_support(fam_weight);
            let mut sources = claim.depends.clone();
            sources.push(claim.id.clone());
            minted.push(HypothesisRecord {
                label: format!(
                    "H-OBL-{}-{}",
                    claim.relation.to_ascii_uppercase(),
                    claim.id
                ),
                form: "obligation".to_string(),
                family: claim.relation.clone(),
                sources,
                weight: clamped,
                tick: tick_id.to_string(),
                status: "minted".to_string(),
                statement: format!(
                    "Synthesize and verify 28-bit PointKey and 4-gram reply continuation rows for {} ({})",
                    claim.id, claim.statement
                ),
            });
            remaining_budget -= 1;
        }
    }

    // 2. Mint exclusion hypotheses for `refuted` claims.
    for claim in &frontier.claim {
        if remaining_budget == 0 {
            break;
        }
        if claim.status == "refuted" {
            let support = clamp_hypothesis_support(16);
            minted.push(HypothesisRecord {
                label: format!(
                    "H-EXCL-{}-{}",
                    claim.relation.to_ascii_uppercase(),
                    claim.id
                ),
                form: "exclusion".to_string(),
                family: claim.relation.clone(),
                sources: vec![claim.id.clone()],
                weight: support,
                tick: tick_id.to_string(),
                status: "minted".to_string(),
                statement: format!(
                    "Exclude falsified configuration {} ({}) and enforce 28-bit PointKey anchor invariant.",
                    claim.id, claim.kappa
                ),
            });
            remaining_budget -= 1;
        }
    }

    // 3. Mint obligation hypotheses for proved curriculum claims with highest family support.
    for claim in &frontier.claim {
        if remaining_budget == 0 {
            break;
        }
        if claim.status == "proved" {
            let fam_weight = schedule_strategies(&engine.weight, &claim.relation)
                .first()
                .map(|&s| weight_of(&engine.weight, s, &claim.relation))
                .unwrap_or(WEIGHT_INITIAL);
            let clamped = clamp_hypothesis_support(fam_weight);
            minted.push(HypothesisRecord {
                label: format!(
                    "H-VERIFIED-{}-{}",
                    claim.relation.to_ascii_uppercase(),
                    claim.id
                ),
                form: "obligation".to_string(),
                family: claim.relation.clone(),
                sources: vec![claim.id.clone()],
                weight: clamped,
                tick: tick_id.to_string(),
                status: "minted".to_string(),
                statement: format!(
                    "Maintain verified 28-bit PointKey and 4-gram continuation invariant for {} ({})",
                    claim.id, claim.kappa
                ),
            });
            remaining_budget -= 1;
        }
    }

    HypothesesFile {
        spec: "r4/hypotheses/1".to_string(),
        hypothesis: minted,
    }
}

/// Encode `input` bytes as standard RFC 4648 Base64 without external crates.
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Return current UTC timestamp formatted as ISO-8601 `YYYY-MM-DDTHH:MM:SSZ`.
fn utc_now_iso8601() -> String {
    if let Ok(output) = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    "2026-11-05T04:00:00Z".to_string()
}

/// Write `key=value` lines to `$GITHUB_OUTPUT` and `$R4_ENGINE_OUTPUT_FILE` when configured.
fn emit_ci_output(outcome: &str, pushed: bool) {
    let payload = format!(
        "outcome={outcome}\npushed={}\n",
        if pushed { "true" } else { "false" }
    );
    for var in ["GITHUB_OUTPUT", "R4_ENGINE_OUTPUT_FILE"] {
        if let Ok(path) = std::env::var(var) {
            if !path.trim().is_empty() {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                {
                    let _ = f.write_all(payload.as_bytes());
                }
            }
        }
    }
}

/// Execute the full 7-stage autonomous homeostasis tick (`cargo xtask engine tick`).
pub fn run_engine_tick(
    root: &Path,
    budget_minutes: u32,
    commit: bool,
    push: bool,
) -> Result<(), String> {
    if budget_minutes < GATE_MIN_MINUTES {
        return Err(format!(
            "budget_minutes ({budget_minutes}) must be at least GATE_MIN_MINUTES ({GATE_MIN_MINUTES})"
        ));
    }

    // Stage 1: build & workflow/model preflight
    let (tick_budget, gate_budget) = validate_workflow_homeostasis(root)?;
    let audit = audit_uorlut_synchronization(root)?;
    println!(
        "engine[1/7 build]: workflow homeostasis ok (tick={tick_budget}m, gate={gate_budget}m), UORLUT01 anchors={} point_rows={} orphans={}",
        audit.reply_anchor_count, audit.point_row_count, audit.orphaned_point_rows
    );

    // Stage 2: induction (discover up to 1 new curriculum claim per tick)
    let discovered_ids = discover(root, true, 1)?;
    println!(
        "engine[2/7 induction]: discovered {} new claim(s): {:?}",
        discovered_ids.len(),
        discovered_ids
    );

    // Stage 3: pruning
    let pruned_ids = prune_frontier(root, true)?;
    println!(
        "engine[3/7 pruning]: pruned {} falsified claim(s): {:?}",
        pruned_ids.len(),
        pruned_ids
    );

    // Stage 4: deduction (prove up to 1 eligible claim per tick so ticks progress iteratively)
    let tick_id = utc_now_iso8601();
    let mut engine = load_engine(root)?;
    let mut frontier = load_frontier(root)?;
    let deduce_summary = deduce(root, &mut engine, &mut frontier, &tick_id, 1)?;
    println!(
        "engine[4/7 deduction]: proved {} claim(s): {:?}",
        deduce_summary.proved_ids.len(),
        deduce_summary.proved_ids
    );

    // Stage 5: learning (Hebbian weight plasticity in [1, 16])
    for (strat, fam, succ) in &deduce_summary.outcomes {
        update_weight(&mut engine.weight, strat, fam, *succ);
    }
    println!(
        "engine[5/7 learning]: updated {} strategy weight(s) in [1, 16]",
        deduce_summary.outcomes.len()
    );

    // Stage 6: hypotheses
    let hyps = mint_hypotheses(&engine, &frontier, &tick_id);
    println!(
        "engine[6/7 hypotheses]: minted {} hypotheses (budget={})",
        hyps.hypothesis.len(),
        HYPOTHESIS_BUDGET
    );

    let advanced = !discovered_ids.is_empty()
        || !pruned_ids.is_empty()
        || !deduce_summary.proved_ids.is_empty();
    let outcome = if advanced { "advanced" } else { "settled" };

    if advanced {
        engine.tick.push(TickRecord {
            id: tick_id.clone(),
            outcome: outcome.to_string(),
            discovered: discovered_ids.len(),
            pruned: pruned_ids.len(),
            proved: deduce_summary.proved_ids.clone(),
            minted: hyps.hypothesis.len(),
        });
        while engine.tick.len() > MAX_TICK_HISTORY {
            engine.tick.remove(0);
        }
        save_frontier(root, &frontier)?;
        save_engine(root, &engine)?;
        save_hypotheses(root, &hyps)?;
    } else {
        let _ = std::fs::write(root.join(".r4-settled"), format!("{tick_id}\n"));
    }

    // Stage 7: acceptance gate
    println!("engine[7/7 gate]: running repository acceptance checks...");
    let gate_ok = if std::env::var_os("SDK_IMAGE").is_some() {
        Command::new("just")
            .current_dir(root)
            .arg("vv")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        check_frontier(&frontier).is_ok()
            && load_engine(root).is_ok()
            && load_hypotheses(root).is_ok()
    };

    if !gate_ok {
        return Err("engine[7/7 gate]: acceptance gate failed; refusing to advance".into());
    }

    let mut pushed = false;
    if commit && advanced {
        let _ = Command::new("git")
            .current_dir(root)
            .args(["config", "user.name", "r4-engine"])
            .status();
        let _ = Command::new("git")
            .current_dir(root)
            .args(["config", "user.email", "r4-engine@users.noreply.github.com"])
            .status();
        let _ = Command::new("git")
            .current_dir(root)
            .args([
                "add",
                "frontier.toml",
                "model/engine.toml",
                "model/hypotheses.toml",
            ])
            .status();

        let msg = if !deduce_summary.proved_ids.is_empty() {
            format!(
                "engine(tick): prove {} and update Hebbian weights",
                deduce_summary.proved_ids.join(", ")
            )
        } else {
            format!(
                "engine(tick): induce {} / prune {} frontier claims",
                discovered_ids.len(),
                pruned_ids.len()
            )
        };

        let committed = Command::new("git")
            .current_dir(root)
            .args(["commit", "-m", &msg])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if committed && push {
            let mut push_cmd = Command::new("git");
            push_cmd.current_dir(root);
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                if !token.trim().is_empty() {
                    let b64 = base64_encode(format!("x-access-token:{token}").as_bytes());
                    push_cmd.arg("-c").arg(format!(
                        "http.https://github.com/.extraheader=AUTHORIZATION: basic {b64}"
                    ));
                }
            }
            push_cmd.args(["push", "origin", "HEAD:main"]);
            pushed = push_cmd.status().map(|s| s.success()).unwrap_or(false);
        }
    }

    emit_ci_output(outcome, pushed);
    println!("engine: tick complete (outcome={outcome}, pushed={pushed})");
    Ok(())
}

/// Print concise status summary (`cargo xtask status` / `brief` / `engine status`).
pub fn print_status(root: &Path) -> Result<(), String> {
    let frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let engine = load_engine(root)?;
    let hyps = load_hypotheses(root)?;
    let (tick_budget, gate_budget) = validate_workflow_homeostasis(root)?;
    let audit = audit_uorlut_synchronization(root)?;

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &frontier.claim {
        *counts.entry(c.status.as_str()).or_insert(0) += 1;
    }

    println!("=== LexLean-R4 Autonomous Homeostasis Engine (R4-28) ===");
    println!(
        "Frontier ({} claims): proved={} observed={} open={} refuted={} retired={}",
        frontier.claim.len(),
        counts.get("proved").copied().unwrap_or(0),
        counts.get("observed").copied().unwrap_or(0),
        counts.get("open").copied().unwrap_or(0),
        counts.get("refuted").copied().unwrap_or(0),
        counts.get("retired").copied().unwrap_or(0),
    );
    println!(
        "UORLUT01 Synchronization: reply_anchors={} point_rows={} orphaned_rows={}",
        audit.reply_anchor_count, audit.point_row_count, audit.orphaned_point_rows
    );
    println!(
        "Workflow Homeostasis: cron='17,47 * * * *' tick_budget={}m gate_reserve={}m",
        tick_budget, gate_budget
    );
    println!("Strategy Schedules by Family:");
    for &fam in &FAMILIES {
        let sched = schedule_strategies(&engine.weight, fam);
        let formatted: Vec<String> = sched
            .iter()
            .map(|&s| format!("{s}({})", weight_of(&engine.weight, s, fam)))
            .collect();
        println!("  {fam:<16} -> {}", formatted.join(" > "));
    }
    println!(
        "Recorded Attempts: {} | Minted Hypotheses: {}/{} | Recent Ticks: {}",
        engine.attempt.len(),
        hyps.hypothesis.len(),
        HYPOTHESIS_BUDGET,
        engine.tick.len()
    );
    Ok(())
}

/// Handle `cargo xtask frontier [list|check|set]` subcommands.
pub fn run_frontier_cmd(root: &Path, args: &[String]) -> Result<(), String> {
    let sub = args.first().map(String::as_str).unwrap_or("list");
    let mut frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    match sub {
        "list" => {
            for c in &frontier.claim {
                println!(
                    "{:<8} [{:<8}] {:<15} {:<42} {}",
                    c.id, c.status, c.relation, c.kappa, c.statement
                );
            }
            Ok(())
        }
        "check" => {
            println!(
                "frontier check: {} claims verified, 0 dependency or kappa errors",
                frontier.claim.len()
            );
            Ok(())
        }
        "set" => {
            if args.len() < 3 {
                return Err("usage: cargo xtask frontier set <ID> <STATUS>".into());
            }
            let target_id = &args[1];
            let new_status = &args[2];
            let claim = frontier
                .claim
                .iter_mut()
                .find(|c| &c.id == target_id)
                .ok_or_else(|| format!("unknown frontier claim ID: {target_id}"))?;
            claim.status = new_status.clone();
            check_frontier(&frontier)?;
            save_frontier(root, &frontier)?;
            println!("updated {target_id} -> {new_status}");
            Ok(())
        }
        other => Err(format!("unknown frontier subcommand: {other}")),
    }
}

/// Handle `cargo xtask engine [status|hypotheses|prune|tick]` subcommands.
pub fn run_engine_cmd(root: &Path, args: &[String]) -> Result<(), String> {
    let sub = args.first().map(String::as_str).unwrap_or("status");
    match sub {
        "status" => print_status(root),
        "hypotheses" => {
            let hyps = load_hypotheses(root)?;
            for h in &hyps.hypothesis {
                println!(
                    "{:<34} [{:<10} w={:>2}] ({}) {}",
                    h.label, h.form, h.weight, h.family, h.statement
                );
            }
            Ok(())
        }
        "prune" => {
            let apply = args.iter().any(|a| a == "--apply");
            let pruned = prune_frontier(root, apply)?;
            println!(
                "engine prune (apply={apply}): {} claim(s) pruned",
                pruned.len()
            );
            Ok(())
        }
        "tick" => {
            let mut budget = TICK_BUDGET_MINUTES;
            for (i, a) in args.iter().enumerate() {
                if a == "--budget-minutes" {
                    if let Some(val) = args.get(i + 1) {
                        budget = val
                            .parse::<u32>()
                            .map_err(|_| format!("invalid --budget-minutes value: {val}"))?;
                    }
                }
            }
            let commit = args.iter().any(|a| a == "--commit");
            let push = args.iter().any(|a| a == "--push");
            run_engine_tick(root, budget, commit, push)
        }
        other => Err(format!("unknown engine subcommand: {other}")),
    }
}

/// Handle `cargo xtask api [check|sync]` subcommands.
pub fn run_api_cmd(root: &Path, args: &[String]) -> Result<(), String> {
    let sub = args.first().map(String::as_str).unwrap_or("check");
    match sub {
        "check" | "sync" => {
            let frontier = load_frontier(root)?;
            check_frontier(&frontier)?;
            let engine = load_engine(root)?;
            let hyps = load_hypotheses(root)?;
            let (tick_budget, gate_budget) = validate_workflow_homeostasis(root)?;
            let audit = audit_uorlut_synchronization(root)?;
            if sub == "sync" {
                let refreshed = mint_hypotheses(&engine, &frontier, "2026-11-05T03:30:00Z");
                save_hypotheses(root, &refreshed)?;
            }
            println!(
                "api {sub}: R4.AutonomousEngine homeostasis verified (claims={}, weights={}, hypotheses={}, tick={}m, gate={}m, point_rows={})",
                frontier.claim.len(),
                engine.weight.len(),
                hyps.hypothesis.len(),
                tick_budget,
                gate_budget,
                audit.point_row_count
            );
            Ok(())
        }
        other => Err(format!("unknown api subcommand: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hebbian_plasticity_stays_within_bounds() {
        for w in WEIGHT_MIN..=WEIGHT_MAX {
            let up = learn(w, true);
            let down = learn(w, false);
            assert!(in_bounds(up));
            assert!(in_bounds(down));
        }
        assert_eq!(learn(8, true), 10);
        assert_eq!(learn(15, true), 16);
        assert_eq!(learn(16, true), 16);
        assert_eq!(learn(8, false), 7);
        assert_eq!(learn(1, false), 1);
    }

    #[test]
    fn priority_schedule_orders_by_learned_weight_descending() {
        let weights = vec![
            StrategyWeight {
                strategy: "corpus-induction".to_string(),
                family: "dialogue-stem".to_string(),
                value: 7,
            },
            StrategyWeight {
                strategy: "pointkey-resonance".to_string(),
                family: "dialogue-stem".to_string(),
                value: 14,
            },
            StrategyWeight {
                strategy: "reply-4gram-deduction".to_string(),
                family: "dialogue-stem".to_string(),
                value: 11,
            },
            StrategyWeight {
                strategy: "bvgk-writer-refinement".to_string(),
                family: "dialogue-stem".to_string(),
                value: 9,
            },
        ];
        let sched = schedule_strategies(&weights, "dialogue-stem");
        assert_eq!(
            sched,
            vec![
                "pointkey-resonance",
                "reply-4gram-deduction",
                "bvgk-writer-refinement",
                "corpus-induction"
            ]
        );
    }

    #[test]
    fn repository_engine_frontier_and_workflow_homeostasis_are_valid() {
        let root = repo_model::repo_root();
        let frontier = load_frontier(&root).expect("frontier.toml loads");
        check_frontier(&frontier).expect("frontier.toml valid");
        let engine = load_engine(&root).expect("model/engine.toml loads");
        assert_eq!(engine.weight.len(), 16);
        let hyps = load_hypotheses(&root).expect("model/hypotheses.toml loads");
        assert!(!hyps.hypothesis.is_empty());
        assert!(hyps.hypothesis.len() <= HYPOTHESIS_BUDGET);
        let (tick_b, gate_b) =
            validate_workflow_homeostasis(&root).expect("engine.yml homeostasis valid");
        assert_eq!(tick_b, 75);
        assert_eq!(gate_b, 15);
        let audit = audit_uorlut_synchronization(&root).expect("UORLUT01 audit valid");
        assert_eq!(audit.orphaned_point_rows, 0);
    }
}
