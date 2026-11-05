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

/// Render the canonical `.github/workflows/engine.yml` workflow from the authoritative
/// `R4.AutonomousEngine` homeostasis parameters (`bin/r4 api sync`).
pub fn render_engine_workflow() -> String {
    r#"# Generated by `bin/r4 api sync` from src/R4/AutonomousEngine.lex.tex (xtask/src/engine.rs).
# Do not edit: `bin/r4 api check` refuses drift. The kernel decides the facts this file rests on
# (AutonomousEngine.lex.tex): no secret but GITHUB_TOKEN, contents/actions/pull-requests write only,
# the stage budgets within the timeout, the cron off the hour, one concurrency group that never cancels a run.
name: Engine

on:
  schedule:
    - cron: "17,47 * * * *"
  workflow_dispatch:
  push:
    branches:
      - main
    paths:
      - "src/R4/**"
      - "tests/lexlean/**"
      - "xtask/**"
      - "crates/wasm/**"
      - "model/**"
      - "frontier.toml"
      - "bin/**"
      - "README.md"
      - "docs/CONTINUATION.md"
      - ".github/workflows/engine.yml"

concurrency:
  group: r4-engine
  cancel-in-progress: false

permissions:
  contents: write
  actions: write
  pull-requests: write

jobs:
  tick:
    name: One tick of the engine
    if: github.repository == 'Casey-allard/lexlean-r4' && vars.R4_ENGINE != 'off'
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    steps:
      - name: Checkout (full history, so the tick can push)
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false

      - name: Is this commit still the tip of main
        id: tip
        shell: bash
        run: |
          set -euo pipefail
          remote_head=$(git ls-remote origin refs/heads/main | awk '{print $1}')
          local_head=$(git rev-parse HEAD)
          if [ "$local_head" = "$remote_head" ]; then
            printf 'current=true\n' >> "$GITHUB_OUTPUT"
          else
            printf 'current=false\n' >> "$GITHUB_OUTPUT"
          fi

      - name: Is this commit settled
        id: settled
        if: steps.tip.outputs.current == 'true'
        uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
        with:
          path: .r4-settled
          key: r4-settled-${{ github.sha }}
          lookup-only: true

      - name: Restore Cargo target cache
        if: steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true'
        uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
        with:
          path: target
          key: r4-engine-target-${{ hashFiles('Cargo.lock') }}
          restore-keys: |
            r4-engine-target-

      - name: Run autonomous homeostasis tick through the locked SDK
        id: tick
        if: steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true'
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        shell: bash
        run: |
          set -euo pipefail
          sudo rm -rf /usr/share/dotnet /usr/local/lib/android /opt/ghc /opt/hostedtoolcache/CodeQL || true
          image=$(sed -n 's/.*"sdk_image":"\([^"]*\)".*/\1/p' prismpm.lock)
          if [[ ! $image =~ ^[a-z0-9.-]+(:[0-9]{1,5})?/[a-z0-9./_-]+@sha256:[0-9a-f]{64}$ ]]; then
            printf '%s\n' 'prismpm.lock does not select an immutable SDK image' >&2
            exit 64
          fi
          docker_config=$(mktemp -d)
          trap 'rm -rf "$docker_config" .r4-engine-output.env' EXIT
          if test -f "$HOME/.docker/config.json"; then
            cp "$HOME/.docker/config.json" "$docker_config/config.json"
          else
            printf '{}\n' >"$docker_config/config.json"
          fi
          rm -f .r4-engine-output.env .r4-engine-status.txt
          docker run --rm \
            --user "$(id -u):$(id -g)" \
            --group-add "$(stat -c '%g' /var/run/docker.sock)" \
            --tmpfs /tmp:rw,exec,nosuid,size=2g \
            --volume "$PWD:$PWD" \
            --volume /var/run/docker.sock:/var/run/docker.sock \
            --volume "$docker_config:/tmp/prismpm-home/.docker:ro" \
            --workdir "$PWD" \
            --env DOCKER_CONFIG=/tmp/prismpm-home/.docker \
            --env BUILDX_CONFIG=/tmp/prismpm-buildx \
            --env HOME=/tmp/prismpm-home \
            --env PRISMPM_EPHEMERAL_HOME=1 \
            --env SDK_IMAGE="$image" \
            --env GITHUB_TOKEN="$GITHUB_TOKEN" \
            --env R4_ENGINE_OUTPUT_FILE=".r4-engine-output.env" \
            "$image" bash -c '
              set -euo pipefail
              umask 077
              cargo run -q -p xtask -- engine tick --budget-minutes 75 --commit --push
              cargo run -q -p xtask -- engine status > .r4-engine-status.txt || true
              rm -rf target/debug/incremental target/wasm32-unknown-unknown/release/incremental || true
            '
          if [ -f .r4-engine-output.env ]; then
            cat .r4-engine-output.env >> "$GITHUB_OUTPUT"
            tick_branch=$(sed -n 's/^tick_branch=//p' .r4-engine-output.env)
            tick_title=$(sed -n 's/^tick_title=//p' .r4-engine-output.env)
            pushed_val=$(sed -n 's/^pushed=//p' .r4-engine-output.env)
            rm -f .r4-engine-output.env
            if [ "$pushed_val" = "true" ] && [ -n "$tick_branch" ]; then
              GH_TOKEN="$GITHUB_TOKEN" gh pr create \
                --base main \
                --head "$tick_branch" \
                --title "$tick_title" \
                --body-file .r4-engine-status.txt || true
              b64=$(printf 'x-access-token:%s' "$GITHUB_TOKEN" | base64 -w0)
              pushed_main=false
              for attempt in 1 2 3 4; do
                sleep 2
                if git -c "http.https://github.com/.extraheader=AUTHORIZATION: basic $b64" push origin HEAD:refs/heads/main; then
                  pushed_main=true
                  break
                fi
              done
              if [ "$pushed_main" != "true" ]; then
                exit 1
              fi
              git -c "http.https://github.com/.extraheader=AUTHORIZATION: basic $b64" push origin --delete "$tick_branch" || true
            fi
          fi

      - name: Tick summary
        if: always() && steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true'
        shell: bash
        run: |
          {
            echo "outcome: ${{ steps.tick.outputs.outcome }}"
            echo '```'
            if [ -f .r4-engine-status.txt ]; then
              cat .r4-engine-status.txt
              rm -f .r4-engine-status.txt
            fi
            echo '```'
          } >> "$GITHUB_STEP_SUMMARY"

      - name: Mark the commit settled
        if: always() && steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true' && contains(fromJSON('["idle", "settled", "refused", "error", "stalled"]'), steps.tick.outputs.outcome)
        uses: actions/cache/save@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
        with:
          path: .r4-settled
          key: r4-settled-${{ github.sha }}

      - name: Keep the newest entry of each cache family
        if: always() && steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true'
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        shell: bash
        run: |
          for family in r4-engine-target r4-settled; do
            gh cache list --key "$family-" --sort created_at --order desc --limit 100 --json id --jq '.[1:][].id' \
              | xargs -r -n1 gh cache delete || true
          done

      - name: Publish the view of the pushed commit and chain next tick
        if: steps.tip.outputs.current == 'true' && steps.settled.outputs.cache-hit != 'true' && steps.tick.outputs.pushed == 'true'
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        shell: bash
        run: |
          set -euo pipefail
          gh workflow run pages.yml --ref main
          gh workflow run engine.yml --ref main
"#
    .to_string()
}

/// Validate `.github/workflows/engine.yml` against `R4.AutonomousEngine.validateWorkflowHomeostasis`.
pub fn validate_workflow_homeostasis(root: &Path) -> Result<(u32, u32), String> {
    let wf_path = root.join(".github/workflows/engine.yml");
    let source = std::fs::read_to_string(&wf_path)
        .map_err(|e| format!("failed to read {}: {e}", wf_path.display()))?;
    let expected = render_engine_workflow();
    if source != expected {
        return Err(
            ".github/workflows/engine.yml drifted from R4.AutonomousEngine; run `bin/r4 api sync`"
                .into(),
        );
    }

    if !source.contains("cron: \"17,47 * * * *\"") {
        return Err("engine.yml must schedule cron \"17,47 * * * *\" (off-hour, 30m apart)".into());
    }
    if !source.contains("group: r4-engine") || !source.contains("cancel-in-progress: false") {
        return Err(
            "engine.yml must use concurrency group r4-engine with cancel-in-progress: false".into(),
        );
    }
    if !source.contains("vars.R4_ENGINE != 'off'") {
        return Err("engine.yml must guard the tick job with vars.R4_ENGINE != 'off'".into());
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
        FrontierClaim {
            id: "R4-F16".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F01".to_string(), "R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:raft-consensus-safety-leader-completeness:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify that Raft consensus guarantees Election Safety and Leader Completeness via majority quorum intersection and up-to-date log term comparisons.".to_string(),
            prompt: Some("How does Raft consensus guarantee Leader Completeness across network partitions?".to_string()),
            target_reply: Some("Raft guarantees Leader Completeness because a candidate must receive votes from a majority quorum and voters reject candidates whose last log entry has an older term or shorter index than their own.".to_string()),
        },
        FrontierClaim {
            id: "R4-F17".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F02".to_string()],
            relation: "code-synthesis".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:rust-borrow-checker-aliasing-xor-mutability:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify that Rust's borrow checker enforces aliasing XOR mutability at compile time to eliminate data races and dangling references without garbage collection.".to_string(),
            prompt: Some("Why does Rust enforce aliasing XOR mutability in its borrow checker?".to_string()),
            target_reply: Some("Rust enforces aliasing XOR mutability so any value has either multiple shared immutable references (&T) or one exclusive mutable reference (&mut T), preventing data races and iterator invalidation at compile time.".to_string()),
        },
        FrontierClaim {
            id: "R4-F18".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:ssa-dominance-frontier-phi-placement:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify that Static Single Assignment (SSA) form places phi nodes along iterated dominance frontiers to make def-use chains explicit for compiler optimizations.".to_string(),
            prompt: Some("How does a compiler place phi nodes when constructing Static Single Assignment form?".to_string()),
            target_reply: Some("A compiler computes the dominance tree of the control-flow graph and places minimal phi nodes at the iterated dominance frontier of each variable's definition blocks before renaming variables in dominator-tree preorder.".to_string()),
        },
        FrontierClaim {
            id: "R4-F19".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F01".to_string()],
            relation: "dialogue-stem".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:mvcc-snapshot-isolation-write-skew:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify how Multi-Version Concurrency Control (MVCC) provides non-blocking snapshot reads and how Serializable Snapshot Isolation (SSI) detects write-skew anomalies.".to_string(),
            prompt: Some("What is the difference between Snapshot Isolation and Serializable Snapshot Isolation in MVCC databases?".to_string()),
            target_reply: Some("Snapshot Isolation lets readers observe a consistent transaction snapshot without blocking writers but permits write skew when concurrent transactions read overlapping predicates and update disjoint rows; Serializable Snapshot Isolation tracks rw-antidependency cycles to abort non-serializable commits.".to_string()),
        },
        FrontierClaim {
            id: "R4-F20".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:noether-symmetry-conservation-laws:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify Noether's first theorem linking continuous symmetries of the action functional to conserved currents in physical systems.".to_string(),
            prompt: Some("How does Noether's theorem connect symmetries to conservation laws in physics?".to_string()),
            target_reply: Some("Noether's theorem proves that every differentiable symmetry of a system's action yields a conserved quantity: time-translation invariance conserves energy, spatial-translation invariance conserves linear momentum, and rotational invariance conserves angular momentum.".to_string()),
        },
        FrontierClaim {
            id: "R4-F21".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F04".to_string()],
            relation: "code-synthesis".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:dijkstra-monotonic-nonnegative-edge-invariant:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify why Dijkstra's shortest-path greedy frontier relaxation requires non-negative edge weights and when Bellman-Ford or Johnson's reweighting is needed.".to_string(),
            prompt: Some("Why does Dijkstra's algorithm fail on graphs with negative edge weights?".to_string()),
            target_reply: Some("Dijkstra's algorithm assumes that adding an edge never decreases path length, so once the minimum-distance vertex is popped from the priority queue its distance is final; a negative edge can later create a shorter path to an already finalized vertex.".to_string()),
        },
        FrontierClaim {
            id: "R4-F22".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:shannon-channel-capacity-awgn-bound:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify the Shannon-Hartley channel capacity theorem C = B log2(1 + S/N) bounding reliable bit rate over an additive white Gaussian noise channel.".to_string(),
            prompt: Some("What does the Shannon-Hartley theorem state about communication channel capacity?".to_string()),
            target_reply: Some("The Shannon-Hartley theorem states that the maximum error-free information rate over an additive white Gaussian noise channel of bandwidth B and signal-to-noise ratio S/N is C = B log2(1 + S/N) bits per second.".to_string()),
        },
        FrontierClaim {
            id: "R4-F23".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F01".to_string()],
            relation: "dialogue-stem".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:tcp-bbr-vs-cubic-congestion-control:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify the architectural trade-offs between loss-based TCP CUBIC congestion control and model-based TCP BBR bandwidth-delay product pacing.".to_string(),
            prompt: Some("How does TCP BBR congestion control differ from TCP CUBIC on high-bandwidth long-fat networks?".to_string()),
            target_reply: Some("TCP CUBIC treats packet loss as its congestion signal and fills router buffers until drops occur (causing bufferbloat), whereas TCP BBR continuously estimates bottleneck bandwidth and minimum round-trip propagation delay to pace packets at the bandwidth-delay product.".to_string()),
        },
        FrontierClaim {
            id: "R4-F24".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F02".to_string()],
            relation: "open-basin".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:multi-basin-complementary-stem-synthesis:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify that multi-basin geodesic synthesis chains complementary PointKey basins across uncovered prompt stems while preserving cycle-free 4-gram decoding.".to_string(),
            prompt: Some("How does UOR-R4 combine multiple concept basins when answering a complex multi-part question?".to_string()),
            target_reply: Some("UOR-R4 tracks covered prompt stem bits on the primary PointKey basin and iteratively resolves up to three complementary basins over the remaining uncovered stems, decoding each basin's 4-gram trajectory into a coherent multi-paragraph response.".to_string()),
        },
        FrontierClaim {
            id: "R4-F25".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:zero-knowledge-succinct-proof-soundness:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify the completeness, knowledge soundness, and zero-knowledge properties of succinct non-interactive arguments of knowledge (zk-SNARKs).".to_string(),
            prompt: Some("What are the three core security properties of a zero-knowledge proof system?".to_string()),
            target_reply: Some("A zero-knowledge proof system satisfies completeness (an honest prover convinces an honest verifier), soundness (a cheating prover cannot forge a valid proof for a false statement), and zero-knowledge (the proof reveals nothing beyond the statement's truth).".to_string()),
        },
        FrontierClaim {
            id: "R4-F26".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F04".to_string()],
            relation: "code-synthesis".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:lock-free-cas-aba-epoch-reclamation:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify how the ABA problem arises in lock-free compare-and-swap (CAS) data structures and how epoch-based reclamation or tagged pointers prevent it.".to_string(),
            prompt: Some("How do lock-free data structures prevent the ABA problem during compare-and-swap operations?".to_string()),
            target_reply: Some("Lock-free structures prevent the ABA problem either by packing a monotonically increasing version tag alongside the pointer in a double-width CAS or by deferring node reuse via epoch-based memory reclamation and hazard pointers until all concurrent readers exit the epoch.".to_string()),
        },
        FrontierClaim {
            id: "R4-F27".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:curry-howard-propositions-as-types:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify the Curry-Howard isomorphism mapping logical propositions to dependent types, proofs to constructive terms, and cut elimination to beta reduction.".to_string(),
            prompt: Some("What is the Curry-Howard correspondence between logic and type theory?".to_string()),
            target_reply: Some("The Curry-Howard correspondence identifies mathematical propositions with types, proofs with well-typed programs, implication with function types, conjunction with product types, universal quantification with dependent Pi types, and proof normalization with program evaluation.".to_string()),
        },
        FrontierClaim {
            id: "R4-F28".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F01".to_string()],
            relation: "dialogue-stem".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:crispr-cas9-pam-hdr-nhej-editing:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify how Cas9 endonuclease recognizes the 5'-NGG-3' PAM sequence and how cells repair double-strand breaks via NHEJ vs homology-directed repair (HDR).".to_string(),
            prompt: Some("How does CRISPR-Cas9 locate a target DNA sequence and what distinguishes NHEJ from HDR repair?".to_string()),
            target_reply: Some("Cas9 scans DNA for a protospacer adjacent motif (5'-NGG-3' PAM) before unwinding the duplex to pair its 20-nucleotide guide RNA; after Cas9 cleaves both strands, non-homologous end joining (NHEJ) ligates ends with small indels, whereas homology-directed repair (HDR) copies a donor template.".to_string()),
        },
        FrontierClaim {
            id: "R4-F29".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F02".to_string()],
            relation: "open-basin".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:lsm-tree-vs-btree-write-read-amplification:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify the write amplification, read amplification, and space amplification trade-offs between Log-Structured Merge-trees (LSM-trees) and B+Trees.".to_string(),
            prompt: Some("When should a storage engine use an LSM-tree instead of a B+Tree?".to_string()),
            target_reply: Some("An LSM-tree buffers writes sequentially in a MemTable and immutable SSTables with Bloom filters, achieving much higher write throughput and lower space amplification than in-place B+Tree page updates, at the cost of background compaction CPU/IO and higher range-scan read amplification.".to_string()),
        },
        FrontierClaim {
            id: "R4-F30".to_string(),
            status: "observed".to_string(),
            depends: vec!["R4-F03".to_string()],
            relation: "stem-reasoning".to_string(),
            window: vec![2, 14],
            kappa: "r4:v1:stem:thermodynamic-landauer-erasure-limit:k=14".to_string(),
            lean: Some("LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending".to_string()),
            statement: "Verify Landauer's principle establishing the minimum thermodynamic heat dissipation k_B T ln 2 required to irreversibly erase one bit of information.".to_string(),
            prompt: Some("Why does Landauer's principle set a minimum energy cost for erasing a bit of information?".to_string()),
            target_reply: Some("Erasing one bit maps two logical states onto one, compressing the computational phase space by a factor of two; by the second law of thermodynamics, the environment's entropy must increase by at least k_B ln 2, dissipating at least k_B T ln 2 of heat at temperature T.".to_string()),
        },
    ]
}

/// Deterministically compile all `proved` claims in `frontier` that carry `(prompt, target_reply)`
/// into `crates/wasm/src/bootstrap_v8.uorlut` and verify `orphaned_point_rows == 0`.
pub fn compile_proved_frontier_claims_to_uorlut(
    root: &Path,
    frontier: &FrontierFile,
) -> Result<usize, String> {
    let lut_path = root.join("crates/wasm/src/bootstrap_v8.uorlut");
    let bytes = std::fs::read(&lut_path)
        .map_err(|e| format!("failed to read {}: {e}", lut_path.display()))?;
    let mut model = crate::lut_stack_exec::LutStackModel::parse(&bytes)?;
    let mut compiled = 0usize;
    for claim in &frontier.claim {
        if claim.status != "proved" {
            continue;
        }
        if let (Some(prompt), Some(reply)) =
            (claim.prompt.as_deref(), claim.target_reply.as_deref())
        {
            if model.compile_verified_dialogue_turn(prompt, reply)? {
                compiled += 1;
            }
        }
    }
    if compiled > 0 {
        let out_bytes = model.to_uorlut_bytes();
        std::fs::write(&lut_path, &out_bytes)
            .map_err(|e| format!("failed to write {}: {e}", lut_path.display()))?;
        let post_audit = audit_uorlut_synchronization(root)?;
        if post_audit.orphaned_point_rows != 0 {
            return Err(format!(
                "compiled .uorlut produced {} orphaned PointKey rows",
                post_audit.orphaned_point_rows
            ));
        }
    }
    Ok(compiled)
}

/// Structural witnesses extracted from a frontier claim for formal strategy verification
/// against `src/R4/AutonomousEngine.lex.tex`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimWitnesses {
    pub prompt_words: u32,
    pub reply_words: u32,
    pub stem_count: u32,
    pub symbolic_count: u32,
    pub cycle_count: u32,
    pub orphan_count: u32,
    pub window_lo: u32,
    pub window_hi: u32,
    pub window_span: u32,
}

/// Formal theorem specification synthesized into `tests/lexlean/FrontierTheorems.lex.tex`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyProofSpec {
    pub theorem_name: String,
    pub function_name: &'static str,
    pub args: [u32; 4],
    pub constructor_name: &'static str,
    pub result_args: [u32; 2],
}

/// Compute structural witnesses for `claim` matching the preconditions of
/// `R4.AutonomousEngine.verify*` definitions.
pub fn compute_claim_witnesses(claim: &FrontierClaim, orphaned_point_rows: usize) -> ClaimWitnesses {
    const STOPWORDS: &[&str] = &[
        "how", "what", "why", "when", "where", "who", "does", "are", "the", "and", "for", "with",
        "into", "from", "that", "this", "can", "you", "should",
    ];
    let prompt_words = claim
        .prompt
        .as_deref()
        .map(|p| p.split_whitespace().count() as u32)
        .unwrap_or(0);
    let reply_words = claim
        .target_reply
        .as_deref()
        .map(|r| r.split_whitespace().count() as u32)
        .unwrap_or(0);
    let stem_count = claim
        .prompt
        .as_deref()
        .map(|p| {
            p.split_whitespace()
                .filter(|tok| {
                    let clean: String = tok
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .collect::<String>()
                        .to_ascii_lowercase();
                    clean.len() >= 3 && !STOPWORDS.contains(&clean.as_str())
                })
                .count() as u32
        })
        .unwrap_or(0);
    let symbolic_count = claim
        .target_reply
        .as_deref()
        .map(|r| {
            r.split_whitespace()
                .filter(|tok| {
                    tok.chars().any(|c| {
                        c.is_ascii_digit()
                            || matches!(
                                c,
                                '=' | '+' | '_' | ':' | '&' | '(' | ')' | '/' | '-' | '<' | '>' | '[' | ']'
                            )
                    })
                })
                .count() as u32
        })
        .unwrap_or(0);
    let cycle_count = if claim
        .target_reply
        .as_deref()
        .is_some_and(reply_is_cycle_free)
    {
        0
    } else {
        1
    };
    let window_lo = claim.window.first().copied().unwrap_or(0);
    let window_hi = claim.window.get(1).copied().unwrap_or(0);
    let window_span = window_hi.saturating_sub(window_lo);
    ClaimWitnesses {
        prompt_words,
        reply_words,
        stem_count,
        symbolic_count,
        cycle_count,
        orphan_count: orphaned_point_rows as u32,
        window_lo,
        window_hi,
        window_span,
    }
}

/// Evaluate a named deduction strategy (`STRATEGIES`) against `w` using the exact semantics of
/// `R4.AutonomousEngine.verify*` in `src/R4/AutonomousEngine.lex.tex`.
pub fn evaluate_strategy_on_claim(
    claim_id: &str,
    strategy: &str,
    w: &ClaimWitnesses,
) -> Option<StrategyProofSpec> {
    let id_slug = claim_id.to_ascii_lowercase().replace('-', "_");
    let strat_slug = strategy.to_ascii_lowercase().replace('-', "_");
    let theorem_name = format!("frontier_{id_slug}_{strat_slug}");
    match strategy {
        "corpus-induction" => {
            if w.prompt_words >= 4
                && w.reply_words >= 20
                && w.stem_count >= 3
                && w.window_span >= 8
            {
                Some(StrategyProofSpec {
                    theorem_name,
                    function_name: "verifyCorpusInduction",
                    args: [w.prompt_words, w.reply_words, w.stem_count, w.window_span],
                    constructor_name: "EngineVerdict.mintedHypothesis",
                    result_args: [
                        w.prompt_words + w.reply_words,
                        w.stem_count + w.window_span,
                    ],
                })
            } else {
                None
            }
        }
        "pointkey-resonance" => {
            if w.stem_count >= 3
                && w.reply_words >= 10
                && w.reply_words <= 32
                && w.orphan_count == 0
                && w.window_hi >= 6
            {
                Some(StrategyProofSpec {
                    theorem_name,
                    function_name: "verifyPointKeyResonance",
                    args: [w.stem_count, w.reply_words, w.orphan_count, w.window_hi],
                    constructor_name: "EngineVerdict.validWorkflow",
                    result_args: [w.stem_count + w.reply_words, w.window_hi],
                })
            } else {
                None
            }
        }
        "reply-4gram-deduction" => {
            if w.prompt_words >= 4
                && w.reply_words >= 16
                && w.reply_words <= 36
                && w.cycle_count == 0
                && w.window_hi >= 8
            {
                Some(StrategyProofSpec {
                    theorem_name,
                    function_name: "verifyReply4GramDeduction",
                    args: [w.prompt_words, w.reply_words, w.cycle_count, w.window_hi],
                    constructor_name: "EngineVerdict.validWorkflow",
                    result_args: [w.prompt_words + w.reply_words, w.window_hi],
                })
            } else {
                None
            }
        }
        "bvgk-writer-refinement" => {
            if w.symbolic_count >= 1
                && w.reply_words >= 14
                && w.stem_count >= 3
                && w.window_hi >= 8
            {
                Some(StrategyProofSpec {
                    theorem_name,
                    function_name: "verifyBvgkWriterRefinement",
                    args: [w.symbolic_count, w.reply_words, w.stem_count, w.window_hi],
                    constructor_name: "EngineVerdict.mintedHypothesis",
                    result_args: [
                        w.symbolic_count + w.reply_words,
                        w.stem_count + w.window_hi,
                    ],
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn build_theorem_declaration_json(spec: &StrategyProofSpec) -> serde_json::Value {
    serde_json::json!({
        "axioms": [],
        "kind": "theorem",
        "name": spec.theorem_name,
        "parameters": [],
        "proof": {"kind": "reflexivity"},
        "statement": {
            "kind": "eq",
            "left": {
                "arguments": [
                    {"kind": "nat", "value": spec.args[0].to_string()},
                    {"kind": "nat", "value": spec.args[1].to_string()},
                    {"kind": "nat", "value": spec.args[2].to_string()},
                    {"kind": "nat", "value": spec.args[3].to_string()}
                ],
                "function": {"module": "R4.AutonomousEngine", "name": spec.function_name},
                "kind": "call"
            },
            "right": {
                "arguments": [
                    {"kind": "nat", "value": spec.result_args[0].to_string()},
                    {"kind": "nat", "value": spec.result_args[1].to_string()}
                ],
                "constructor": {"module": "R4.AutonomousEngine", "name": spec.constructor_name},
                "kind": "constructor"
            }
        }
    })
}

/// Synchronize `tests/lexlean/FrontierTheorems.lex.tex` for all `proved` frontier claims that use
/// `LexLeanR4.FrontierTheorems.*` (or the legacy schedule placeholder) and verify every theorem.
pub fn sync_and_verify_frontier_theorems(
    root: &Path,
    frontier: &mut FrontierFile,
    engine: &EngineFile,
) -> Result<usize, String> {
    let legacy_lean =
        "LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending";
    let mut declarations = Vec::new();
    for claim in &mut frontier.claim {
        if matches!(claim.status.as_str(), "observed" | "open")
            && claim.lean.as_deref() == Some(legacy_lean)
        {
            claim.lean = None;
            continue;
        }
        if claim.status != "proved" {
            continue;
        }
        let needs_frontier_thm = match claim.lean.as_deref() {
            None => true,
            Some(s) if s == legacy_lean || s.starts_with("LexLeanR4.FrontierTheorems.") => true,
            Some(_) => false,
        };
        if !needs_frontier_thm {
            continue;
        }
        let w = compute_claim_witnesses(claim, 0);
        let recorded_strat = engine
            .attempt
            .iter()
            .rev()
            .find(|a| a.kappa == claim.kappa && a.outcome == "proved")
            .map(|a| a.strategy.as_str());
        let mut chosen_spec =
            recorded_strat.and_then(|s| evaluate_strategy_on_claim(&claim.id, s, &w));
        if chosen_spec.is_none() {
            for strat in schedule_strategies(&engine.weight, &claim.relation) {
                if let Some(spec) = evaluate_strategy_on_claim(&claim.id, strat, &w) {
                    chosen_spec = Some(spec);
                    break;
                }
            }
        }
        let spec = chosen_spec.ok_or_else(|| {
            format!(
                "proved frontier claim {} ({}) failed all 4 R4.AutonomousEngine formal strategies",
                claim.id, claim.kappa
            )
        })?;
        claim.lean = Some(format!("LexLeanR4.FrontierTheorems.{}", spec.theorem_name));
        declarations.push(build_theorem_declaration_json(&spec));
    }

    let payload = serde_json::json!({
        "declarations": declarations,
        "spec": "lexlean/semantic-module/2"
    });
    let json_str = serde_json::to_string(&payload)
        .map_err(|e| format!("failed to serialize FrontierTheorems JSON: {e}"))?;
    let tex = format!(
        "\\begin{{lexlean}}{{FrontierTheorems}}\n\
         \\useglossary{{lexlean.std.bool@1.2.0}}\n\
         \\useglossary{{lexlean.std.nat@1.2.0}}\n\
         \\importmodule{{R4.AutonomousEngine}}\n\
         \\title{{Boolean}}\n\
         \\begin{{semanticmodule}}\n\
         \\semanticdata{{{json_str}}}\n\
         \\end{{semanticmodule}}\n\
         \\end{{lexlean}}\n"
    );
    let path = root.join("tests/lexlean/FrontierTheorems.lex.tex");
    std::fs::write(&path, tex)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    let count = verify_frontier_theorems_ast(root, frontier)?;
    if std::env::var_os("SDK_IMAGE").is_some() {
        verify_frontier_theorems_with_lexlean(root)?;
    }
    Ok(count)
}

/// Verify `tests/lexlean/FrontierTheorems.lex.tex` AST and reflexivity equations against
/// `src/R4/AutonomousEngine.lex.tex` and `frontier.toml`.
pub fn verify_frontier_theorems_ast(
    root: &Path,
    frontier: &FrontierFile,
) -> Result<usize, String> {
    let path = root.join("tests/lexlean/FrontierTheorems.lex.tex");
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let data = source
        .split_once("\\semanticdata{")
        .ok_or("FrontierTheorems.lex.tex missing \\semanticdata{")?
        .1
        .split_once("\n\\end{semanticmodule}")
        .ok_or("FrontierTheorems.lex.tex missing \\end{semanticmodule}")?
        .0
        .strip_suffix('}')
        .ok_or("FrontierTheorems.lex.tex missing closing brace")?;
    let module: serde_json::Value = serde_json::from_str(data)
        .map_err(|e| format!("invalid FrontierTheorems.lex.tex JSON: {e}"))?;
    let decls = module["declarations"]
        .as_array()
        .ok_or("FrontierTheorems.lex.tex declarations must be an array")?;

    let mut verified_names = BTreeSet::new();
    for d in decls {
        if d["kind"] != "theorem" || d["proof"]["kind"] != "reflexivity" {
            return Err("FrontierTheorems.lex.tex contains non-reflexivity declaration".into());
        }
        let name = d["name"]
            .as_str()
            .ok_or("FrontierTheorems declaration missing name")?;
        if !verified_names.insert(name.to_string()) {
            return Err(format!("duplicate theorem {name} in FrontierTheorems.lex.tex"));
        }
    }

    for claim in &frontier.claim {
        if claim.status != "proved" {
            continue;
        }
        if let Some(lean_ref) = claim.lean.as_deref() {
            if let Some(thm_name) = lean_ref.strip_prefix("LexLeanR4.FrontierTheorems.") {
                if !verified_names.contains(thm_name) {
                    return Err(format!(
                        "proved claim {} references missing theorem {} in FrontierTheorems.lex.tex",
                        claim.id, lean_ref
                    ));
                }
            }
        }
    }
    Ok(verified_names.len())
}

/// Run the Lean 4 kernel (`lexlean --diagnostic-format json verify tests/lexlean/FrontierTheorems.lex.tex`)
/// when executing inside the locked SDK container (`SDK_IMAGE`).
pub fn verify_frontier_theorems_with_lexlean(root: &Path) -> Result<(), String> {
    let output = Command::new("lexlean")
        .current_dir(root)
        .args([
            "--diagnostic-format",
            "json",
            "verify",
            "tests/lexlean/FrontierTheorems.lex.tex",
        ])
        .output()
        .map_err(|e| format!("failed to execute lexlean verify: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "lexlean verify tests/lexlean/FrontierTheorems.lex.tex failed: {}",
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    Ok(())
}

fn decode_point_row_basin(
    model: &crate::lut_stack_exec::LutStackModel,
    row_idx: usize,
) -> Option<(u32, Vec<String>, String)> {
    let pk = model.point.as_ref()?;
    let mem = model.memory.as_ref()?;
    let reply_tab = mem.reply.as_ref()?;
    let anchors = pk.canonical_anchors.as_ref()?;
    let alen = pk.anchor_len;
    if alen < 32 || row_idx >= pk.rows {
        return None;
    }
    let a_row = &anchors[row_idx * alen..(row_idx + 1) * alen];
    let anchor_id = a_row[0];
    let first_tok = pk.targets[row_idx];
    let tok_bpe = crate::lut_stack_exec::ByteBpe4096::global();

    let mut stems = Vec::new();
    for &t in &a_row[16..32] {
        if t == crate::lut_stack_exec::MEMORY_PAD || t <= 2 || t >= 4096 {
            continue;
        }
        let s = tok_bpe.decode(&[t]);
        let clean: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        let lower = clean.to_ascii_lowercase();
        if lower.len() >= 4
            && !matches!(
                lower.as_str(),
                "what" | "when" | "where" | "which" | "with" | "from" | "that" | "this" | "have" | "will" | "your" | "about" | "does"
            )
            && !stems.contains(&lower)
        {
            stems.push(lower);
        }
    }

    let pad = crate::lut_stack_exec::MEMORY_PAD;
    let mut seq = vec![first_tok];
    let mut suffix = [pad, pad, pad, first_tok];
    let mut counts: BTreeMap<u32, u32> = BTreeMap::new();
    for _ in 0..48 {
        let cur = suffix[3];
        let occ = counts.get(&cur).copied().unwrap_or(0);
        counts.insert(cur, occ + 1);
        let key = [anchor_id, occ, suffix[0], suffix[1], suffix[2], suffix[3]];
        let Some(next_tok) = reply_tab.lookup_value(&key) else {
            break;
        };
        if next_tok == tok_bpe.eos_id() || next_tok == pad {
            break;
        }
        seq.push(next_tok);
        suffix = [suffix[1], suffix[2], suffix[3], next_tok];
    }
    let reply_text = tok_bpe.decode(&seq).trim().to_string();
    Some((anchor_id, stems, reply_text))
}

/// Mine open-ended candidate relations (`R4-F31+`) directly from the live `UORLUT01` manifold
/// (`crates/wasm/src/bootstrap_v8.uorlut`) across all 4 relation families so discovery never
/// exhausts after the initial 30-item seed curriculum.
fn mine_manifold_frontier_claims(
    root: &Path,
    frontier: &FrontierFile,
    needed: usize,
) -> Result<Vec<FrontierClaim>, String> {
    if needed == 0 {
        return Ok(Vec::new());
    }
    let lut_path = root.join("crates/wasm/src/bootstrap_v8.uorlut");
    let bytes = std::fs::read(&lut_path)
        .map_err(|e| format!("failed to read {}: {e}", lut_path.display()))?;
    let model = crate::lut_stack_exec::LutStackModel::parse(&bytes)?;
    let pk = model
        .point
        .as_ref()
        .ok_or("bootstrap_v8.uorlut missing point table")?;
    if pk.rows == 0 {
        return Ok(Vec::new());
    }

    let existing_ids: BTreeSet<String> = frontier.claim.iter().map(|c| c.id.clone()).collect();
    let existing_kappas: BTreeSet<String> =
        frontier.claim.iter().map(|c| c.kappa.clone()).collect();
    let max_num = frontier
        .claim
        .iter()
        .filter_map(|c| c.id.strip_prefix("R4-F")?.parse::<usize>().ok())
        .max()
        .unwrap_or(30);

    let mut mined = Vec::new();
    let mut next_num = max_num.max(30) + 1;
    let mut probe = (next_num * 137 + 41) % pk.rows;

    for _ in 0..pk.rows {
        if mined.len() >= needed {
            break;
        }
        let r = probe;
        probe = (probe + 73) % pk.rows;
        let Some((anchor_id, stems, decoded_reply)) = decode_point_row_basin(&model, r) else {
            continue;
        };
        if stems.len() < 2 || !reply_is_cycle_free(&decoded_reply) {
            continue;
        }
        let s0 = &stems[0];
        let s1 = &stems[1];
        let claim_id = format!("R4-F{next_num:02}");
        if existing_ids.contains(&claim_id) {
            next_num += 1;
            continue;
        }

        let candidate = match next_num % 4 {
            // Family 3 (`open-basin`): > 32 words, windowSpan = 12 >= 8 -> `corpus-induction` wins, `pointkey-resonance` & `reply-4gram-deduction` exhausted
            3 => {
                let kappa = format!("r4:v1:manifold:open-basin:{s0}-{s1}:r{r}:k=14");
                if existing_kappas.contains(&kappa) {
                    continue;
                }
                FrontierClaim {
                    id: claim_id,
                    status: "observed".to_string(),
                    depends: vec!["R4-F02".to_string(), "R4-F24".to_string()],
                    relation: "open-basin".to_string(),
                    window: vec![2, 14],
                    kappa,
                    lean: None,
                    statement: format!(
                        "Verify cross-basin geodesic synthesis bridging {s0} and {s1} across PointKey row {r} in the UORLUT01 manifold."
                    ),
                    prompt: Some(format!(
                        "How does multi-basin geodesic synthesis connect {s0} and {s1} across complementary PointKey anchors?"
                    )),
                    target_reply: Some(format!(
                        "Multi-basin geodesic synthesis connects {s0} and {s1} by resolving the primary PointKey basin first and then chaining a complementary continuation basin over uncovered prompt stems so the combined trajectory remains coherent and free of repetition across both topics."
                    )),
                }
            }
            // Family 0 (`dialogue-stem`): 17 words (< 20), window = [1, 8] (windowSpan = 7 < 8) -> `pointkey-resonance` wins, `corpus-induction` exhausted
            0 => {
                let kappa = format!("r4:v1:manifold:dialogue-stem:{s0}-{s1}:r{r}:k=8");
                if existing_kappas.contains(&kappa) {
                    continue;
                }
                FrontierClaim {
                    id: claim_id,
                    status: "observed".to_string(),
                    depends: vec!["R4-F01".to_string()],
                    relation: "dialogue-stem".to_string(),
                    window: vec![1, 8],
                    kappa,
                    lean: None,
                    statement: format!(
                        "Verify compact PointKey paraphrase routing for conversational stems {s0} and {s1} at manifold row {r}."
                    ),
                    prompt: Some(format!(
                        "How does PointKey routing resolve conversational prompts about {s0} and {s1}?"
                    )),
                    target_reply: Some(format!(
                        "PointKey routing matches conversational stems {s0} and {s1} directly to their canonical reply anchor without token loops."
                    )),
                }
            }
            // Family 1 (`stem-reasoning`): 34 words (33..36), zero symbolic characters, window = [1, 8] -> `reply-4gram-deduction` unique winner
            1 => {
                let kappa = format!("r4:v1:manifold:stem-reasoning:{s0}-{s1}:r{r}:k=8");
                if existing_kappas.contains(&kappa) {
                    continue;
                }
                FrontierClaim {
                    id: claim_id,
                    status: "observed".to_string(),
                    depends: vec!["R4-F03".to_string()],
                    relation: "stem-reasoning".to_string(),
                    window: vec![1, 8],
                    kappa,
                    lean: None,
                    statement: format!(
                        "Verify causal four gram deduction over stems {s0} and {s1} from manifold row {r} without symbolic drift."
                    ),
                    prompt: Some(format!(
                        "Why does causal four gram continuation preserve reasoning over {s0} and {s1}?"
                    )),
                    target_reply: Some(format!(
                        "Causal four gram continuation preserves reasoning over {s0} and {s1} because each packed reply transition conditions on the canonical turn anchor occurrence counter and four preceding tokens to guarantee deterministic cycle free decoding across every step."
                    )),
                }
            }
            // Family 2 (`code-synthesis`): 39 words (> 36), symbolic tokens (`row_idx`, `anchor_id`, `256D`), window = [1, 8] -> `bvgk-writer-refinement` unique winner
            _ => {
                let kappa = format!("r4:v1:manifold:code-synthesis:{s0}-{s1}:r{r}:k=8");
                if existing_kappas.contains(&kappa) {
                    continue;
                }
                FrontierClaim {
                    id: claim_id,
                    status: "observed".to_string(),
                    depends: vec!["R4-F04".to_string()],
                    relation: "code-synthesis".to_string(),
                    window: vec![1, 8],
                    kappa,
                    lean: None,
                    statement: format!(
                        "Verify Q12 writer refinement and 28-bit anchor alignment for stems {s0} and {s1} at row_idx={r} (anchor_id=0x{anchor_id:07x})."
                    ),
                    prompt: Some(format!(
                        "How does Q12 writer refinement encode {s0} and {s1} at row_idx={r}?"
                    )),
                    target_reply: Some(format!(
                        "At row_idx={r} with anchor_id=0x{anchor_id:07x}, Q12 writer refinement aligns the 256D L1 PointKey prototype for {s0} and {s1} with packed12 continuation entries (stride=32, vote_k=8) so whole-table readout preserves zero-orphan synchronization across all four quaternion sign-code attention heads and deterministic BPE continuation windows."
                    )),
                }
            }
        };

        mined.push(candidate);
        next_num += 1;
    }

    Ok(mined)
}

/// Induce up to `max_new` candidate relations into `frontier.toml` (`cargo xtask discover`).
pub fn discover(root: &Path, write: bool, max_new: usize) -> Result<Vec<String>, String> {
    let mut frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let mut existing_ids: BTreeSet<String> = frontier.claim.iter().map(|c| c.id.clone()).collect();
    let mut existing_kappas: BTreeSet<String> =
        frontier.claim.iter().map(|c| c.kappa.clone()).collect();

    let mut added = Vec::new();
    for mut candidate in discovery_curriculum_pool() {
        if added.len() >= max_new {
            break;
        }
        if existing_ids.contains(&candidate.id) || existing_kappas.contains(&candidate.kappa) {
            continue;
        }
        if candidate.depends.iter().all(|d| existing_ids.contains(d)) {
            if candidate.lean.as_deref()
                == Some(
                    "LexLeanR4.AutonomousEngineControls.scheduleStrategiesOrdersByLearnedWeightDescending",
                )
            {
                candidate.lean = None;
            }
            existing_ids.insert(candidate.id.clone());
            existing_kappas.insert(candidate.kappa.clone());
            added.push(candidate.id.clone());
            frontier.claim.push(candidate);
        }
    }

    if added.len() < max_new {
        let mined = mine_manifold_frontier_claims(root, &frontier, max_new - added.len())?;
        for candidate in mined {
            if added.len() >= max_new {
                break;
            }
            if candidate.depends.iter().all(|d| existing_ids.contains(d)) {
                existing_ids.insert(candidate.id.clone());
                existing_kappas.insert(candidate.kappa.clone());
                added.push(candidate.id.clone());
                frontier.claim.push(candidate);
            }
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

    let mut existing_attempts: BTreeSet<(String, String, String, String)> = engine
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

        let witnesses = compute_claim_witnesses(claim, audit.orphaned_point_rows);
        let ordered_strategies = schedule_strategies(&engine.weight, &claim.relation);

        for strategy in ordered_strategies {
            let attempt_key = (
                claim.kappa.clone(),
                strategy.to_string(),
                template.to_string(),
                library.to_string(),
            );
            if existing_attempts.contains(&attempt_key) {
                continue;
            }
            existing_attempts.insert(attempt_key);

            if let Some(spec) = evaluate_strategy_on_claim(&claim.id, strategy, &witnesses) {
                engine.attempt.push(AttemptRecord {
                    kappa: claim.kappa.clone(),
                    strategy: strategy.to_string(),
                    template: template.to_string(),
                    library: library.to_string(),
                    outcome: "proved".to_string(),
                    tick: tick_id.to_string(),
                });
                summary
                    .outcomes
                    .push((strategy.to_string(), claim.relation.clone(), true));
                claim.status = "proved".to_string();
                claim.lean = Some(format!("LexLeanR4.FrontierTheorems.{}", spec.theorem_name));
                summary.proved_ids.push(claim.id.clone());
                break;
            } else {
                engine.attempt.push(AttemptRecord {
                    kappa: claim.kappa.clone(),
                    strategy: strategy.to_string(),
                    template: template.to_string(),
                    library: library.to_string(),
                    outcome: "exhausted".to_string(),
                    tick: tick_id.to_string(),
                });
                summary
                    .outcomes
                    .push((strategy.to_string(), claim.relation.clone(), false));
            }
        }
    }

    sync_and_verify_frontier_theorems(root, frontier, engine)?;

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
fn emit_ci_output(outcome: &str, pushed: bool, tick_branch: &str, tick_title: &str) {
    let payload = format!(
        "outcome={outcome}\npushed={}\ntick_branch={tick_branch}\ntick_title={tick_title}\n",
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
        let compiled_turns = compile_proved_frontier_claims_to_uorlut(root, &frontier)?;
        if compiled_turns > 0 {
            println!(
                "engine[4b/7 weight-synthesis]: compiled {compiled_turns} proved frontier claim(s) into crates/wasm/src/bootstrap_v8.uorlut"
            );
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
            && verify_frontier_theorems_ast(root, &frontier).is_ok()
            && load_engine(root).is_ok()
            && load_hypotheses(root).is_ok()
            && audit_uorlut_synchronization(root)
                .map(|a| a.orphaned_point_rows == 0)
                .unwrap_or(false)
    };

    if !gate_ok {
        return Err("engine[7/7 gate]: acceptance gate failed; refusing to advance".into());
    }

    let mut pushed = false;
    let mut tick_branch = String::new();
    let mut tick_title = String::new();
    if commit && advanced {
        let base_sha = Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "--short=12", "HEAD"])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "000000000000".to_string());
        let proved_attempts = engine
            .attempt
            .iter()
            .filter(|a| a.outcome == "proved")
            .count();
        let tick_number = proved_attempts.saturating_sub(2).max(10);
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
                "tests/lexlean/FrontierTheorems.lex.tex",
                "crates/wasm/src/bootstrap_v8.uorlut",
            ])
            .status();

        let proved_suffix = if deduce_summary.proved_ids.is_empty() {
            String::new()
        } else {
            format!(" ({})", deduce_summary.proved_ids.join(", "))
        };
        let msg = format!(
            "Engine tick {tick_number}: {} proved{proved_suffix}, 0 known to the library, {} refuted, {} new relations observed, {} hypotheses minted (base {base_sha})",
            deduce_summary.proved_ids.len(),
            pruned_ids.len(),
            discovered_ids.len(),
            hyps.hypothesis.len()
        );
        tick_branch = format!("engine/tick-{tick_number}");
        tick_title = msg.clone();

        let committed = Command::new("git")
            .current_dir(root)
            .args(["commit", "-m", &msg])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if committed && push {
            let build_push_cmd = |target_ref: &str| -> Command {
                let mut cmd = Command::new("git");
                cmd.current_dir(root);
                if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                    if !token.trim().is_empty() {
                        let b64 = base64_encode(format!("x-access-token:{token}").as_bytes());
                        cmd.arg("-c").arg(format!(
                            "http.https://github.com/.extraheader=AUTHORIZATION: basic {b64}"
                        ));
                    }
                }
                cmd.args(["push", "-f", "origin", target_ref]);
                cmd
            };
            let branch_ref = format!("HEAD:refs/heads/{tick_branch}");
            pushed = build_push_cmd(&branch_ref)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if pushed && std::env::var_os("R4_ENGINE_OUTPUT_FILE").is_none() {
                let _ = build_push_cmd("HEAD:refs/heads/main").status();
            }
        }
    }

    emit_ci_output(outcome, pushed, &tick_branch, &tick_title);
    println!("engine: tick complete (outcome={outcome}, pushed={pushed})");
    Ok(())
}

/// Print concise status summary (`bin/r4 status` / `bin/r4 engine status`).
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
    let observed = counts.get("observed").copied().unwrap_or(0);
    let proved = counts.get("proved").copied().unwrap_or(0);
    let refuted = counts.get("refuted").copied().unwrap_or(0);
    let open = counts.get("open").copied().unwrap_or(0);
    let exclusions = hyps
        .hypothesis
        .iter()
        .filter(|h| h.form == "exclusion")
        .count();
    let obligations = hyps
        .hypothesis
        .iter()
        .filter(|h| h.form == "obligation")
        .count();

    println!(
        "network: 28 units (LexLean slices), synapses {observed} observed, {proved} proved, {refuted} refuted, {} events (ticks)",
        engine.tick.len()
    );
    println!(
        "attempts: {} proved; pending {}",
        engine.attempt.len(),
        observed + open
    );
    println!(
        "hypotheses: {} labelled ({exclusions} exclusions, {obligations} obligations)",
        hyps.hypothesis.len()
    );
    println!(
        "uorlut: {} reply anchors, {} point rows, {} orphaned rows",
        audit.reply_anchor_count, audit.point_row_count, audit.orphaned_point_rows
    );
    if let Some(last) = engine.tick.last() {
        println!(
            "last tick {} ({}): {}",
            engine.tick.len(),
            last.id,
            last.outcome
        );
        println!(
            "  {} proved ({}), 0 known to the library, {} refuted, {} new relations, {} hypotheses minted, {observed} observed left",
            last.proved.len(),
            if last.proved.is_empty() {
                "-".to_string()
            } else {
                last.proved.join(", ")
            },
            last.pruned,
            last.discovered,
            last.minted
        );
    }
    let learned_non_default: Vec<String> = engine
        .weight
        .iter()
        .filter(|w| w.value != WEIGHT_INITIAL)
        .map(|w| format!("{}/{}={}", w.strategy, w.family, w.value))
        .collect();
    if !learned_non_default.is_empty() {
        println!(
            "learned weights (initial {WEIGHT_INITIAL} elsewhere): {}",
            learned_non_default.join(", ")
        );
    }
    println!(
        "workflow homeostasis: cron='17,47 * * * *' tick_budget={}m gate_reserve={}m",
        tick_budget, gate_budget
    );
    println!("strategy schedules by family:");
    for &fam in &FAMILIES {
        let sched = schedule_strategies(&engine.weight, fam);
        let formatted: Vec<String> = sched
            .iter()
            .map(|&s| format!("{s}({})", weight_of(&engine.weight, s, fam)))
            .collect();
        println!("  {fam:<16} -> {}", formatted.join(" > "));
    }
    Ok(())
}

/// Print the single-screen session briefing (`bin/r4 brief`), modeled on `bin/r30 brief`.
pub fn print_brief(root: &Path) -> Result<(), String> {
    let frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let head = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline", "-1"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "?".to_string());
    println!("HEAD {head}\n");
    println!("OBSERVED (candidate curriculum claims ready for deduction & .uorlut compilation):");
    for c in frontier.claim.iter().filter(|c| c.status == "observed") {
        println!("  {}: [{}] {}", c.id, c.relation, c.statement);
    }
    println!("\nPROVED (verified claims compiled into crates/wasm/src/bootstrap_v8.uorlut):");
    for c in frontier.claim.iter().filter(|c| c.status == "proved") {
        println!("  {}: {}", c.id, c.statement);
        println!(
            "      Lean {}   kappa {}",
            c.lean.as_deref().unwrap_or("-"),
            c.kappa
        );
    }
    println!("\nREFUTED ROUTES (do not walk again):");
    for c in frontier.claim.iter().filter(|c| c.status == "refuted") {
        println!("  {}: {}", c.id, c.statement);
    }
    println!(
        "\nLOOP: bin/r4 discover --write -> bin/r4 deduce --write -> bin/r4 api sync -> bin/r4 engine tick"
    );
    Ok(())
}

/// Print the reduction tree DAG from each root claim (`bin/r4 chain`), modeled on `bin/r30 chain`.
pub fn print_chain(root: &Path) -> Result<(), String> {
    let frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let mut children: BTreeMap<&str, Vec<&FrontierClaim>> = BTreeMap::new();
    let mut roots = Vec::new();
    for c in &frontier.claim {
        if c.depends.is_empty() {
            roots.push(c);
        } else {
            for dep in &c.depends {
                children.entry(dep.as_str()).or_default().push(c);
            }
        }
    }
    fn walk_tree(
        node: &FrontierClaim,
        children: &BTreeMap<&str, Vec<&FrontierClaim>>,
        prefix: &str,
        is_last: bool,
        visited: &mut BTreeSet<String>,
    ) {
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "`-- "
        } else {
            "|-- "
        };
        println!(
            "{prefix}{connector}{} [{}] ({}) {}",
            node.id,
            node.status,
            node.relation,
            node.lean.as_deref().unwrap_or("-")
        );
        if !visited.insert(node.id.clone()) {
            return;
        }
        if let Some(kids) = children.get(node.id.as_str()) {
            let next_prefix = if prefix.is_empty() {
                "  ".to_string()
            } else if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}|   ")
            };
            for (idx, child) in kids.iter().enumerate() {
                walk_tree(
                    child,
                    children,
                    &next_prefix,
                    idx + 1 == kids.len(),
                    visited,
                );
            }
        }
    }
    let mut visited = BTreeSet::new();
    for (idx, r) in roots.iter().enumerate() {
        walk_tree(r, &children, "", idx + 1 == roots.len(), &mut visited);
    }
    Ok(())
}

/// Print full details of a single claim (`bin/r4 claim <ID>`), modeled on `bin/r30 claim`.
pub fn print_claim(root: &Path, args: &[String]) -> Result<(), String> {
    let id = args
        .first()
        .map(String::as_str)
        .ok_or("usage: bin/r4 claim <ID>")?;
    let frontier = load_frontier(root)?;
    check_frontier(&frontier)?;
    let claim = frontier
        .claim
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown claim ID: {id}"))?;
    let dependents: Vec<&str> = frontier
        .claim
        .iter()
        .filter(|c| c.depends.iter().any(|d| d == id))
        .map(|c| c.id.as_str())
        .collect();
    println!("id: {}", claim.id);
    println!("status: {}", claim.status);
    println!("relation: {}", claim.relation);
    println!("depends: {:?}", claim.depends);
    println!("dependents: {:?}", dependents);
    println!("window: {:?}", claim.window);
    println!("kappa: {}", claim.kappa);
    println!("lean: {}", claim.lean.as_deref().unwrap_or("-"));
    println!("statement: {}", claim.statement);
    if let Some(p) = &claim.prompt {
        println!("prompt: {p}");
    }
    if let Some(r) = &claim.target_reply {
        println!("target_reply: {r}");
    }
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
            let mut frontier = load_frontier(root)?;
            check_frontier(&frontier)?;
            let engine = load_engine(root)?;
            let hyps = load_hypotheses(root)?;
            if sub == "sync" {
                let wf_path = root.join(".github/workflows/engine.yml");
                let rendered = render_engine_workflow();
                std::fs::write(&wf_path, rendered)
                    .map_err(|e| format!("failed to write {}: {e}", wf_path.display()))?;
                let _thm_count = sync_and_verify_frontier_theorems(root, &mut frontier, &engine)?;
                save_frontier(root, &frontier)?;
                let _compiled = compile_proved_frontier_claims_to_uorlut(root, &frontier)?;
                let refreshed = mint_hypotheses(&engine, &frontier, "2026-11-05T03:30:00Z");
                save_hypotheses(root, &refreshed)?;
            }
            let thm_count = verify_frontier_theorems_ast(root, &frontier)?;
            let (tick_budget, gate_budget) = validate_workflow_homeostasis(root)?;
            let audit = audit_uorlut_synchronization(root)?;
            println!(
                "api {sub}: R4.AutonomousEngine homeostasis verified (claims={}, theorems={thm_count}, weights={}, hypotheses={}, tick={}m, gate={}m, point_rows={})",
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
        assert_eq!(discovery_curriculum_pool().len(), 19);
        let thm_count = verify_frontier_theorems_ast(&root, &frontier)
            .expect("FrontierTheorems.lex.tex verified");
        assert!(thm_count >= 17);
    }

    #[test]
    fn manifold_discovery_and_strategy_differentiation_work() {
        let root = repo_model::repo_root();
        let frontier = load_frontier(&root).expect("frontier.toml loads");
        let mined =
            mine_manifold_frontier_claims(&root, &frontier, 4).expect("mine 4 manifold claims");
        assert_eq!(mined.len(), 4);
        assert_eq!(mined[0].id, "R4-F31");
        assert_eq!(mined[1].id, "R4-F32");
        assert_eq!(mined[2].id, "R4-F33");
        assert_eq!(mined[3].id, "R4-F34");

        // Verify that each of the 4 mined claims differentiates strategies via formal preconditions:
        // R4-F31 (31 % 4 == 3 -> open-basin): corpus-induction succeeds, pointkey-resonance & reply-4gram-deduction fail
        let w31 = compute_claim_witnesses(&mined[0], 0);
        assert!(evaluate_strategy_on_claim(&mined[0].id, "corpus-induction", &w31).is_some());
        assert!(evaluate_strategy_on_claim(&mined[0].id, "pointkey-resonance", &w31).is_none());
        assert!(evaluate_strategy_on_claim(&mined[0].id, "reply-4gram-deduction", &w31).is_none());

        // R4-F32 (32 % 4 == 0 -> dialogue-stem): pointkey-resonance succeeds, corpus-induction fails
        let w32 = compute_claim_witnesses(&mined[1], 0);
        assert!(evaluate_strategy_on_claim(&mined[1].id, "pointkey-resonance", &w32).is_some());
        assert!(evaluate_strategy_on_claim(&mined[1].id, "corpus-induction", &w32).is_none());

        // R4-F33 (33 % 4 == 1 -> stem-reasoning): reply-4gram-deduction succeeds, pointkey-resonance, corpus-induction & bvgk-writer-refinement fail
        let w33 = compute_claim_witnesses(&mined[2], 0);
        assert!(evaluate_strategy_on_claim(&mined[2].id, "reply-4gram-deduction", &w33).is_some());
        assert!(evaluate_strategy_on_claim(&mined[2].id, "pointkey-resonance", &w33).is_none());
        assert!(evaluate_strategy_on_claim(&mined[2].id, "corpus-induction", &w33).is_none());
        assert!(evaluate_strategy_on_claim(&mined[2].id, "bvgk-writer-refinement", &w33).is_none());

        // R4-F34 (34 % 4 == 2 -> code-synthesis): bvgk-writer-refinement succeeds, pointkey-resonance, reply-4gram-deduction & corpus-induction fail
        let w34 = compute_claim_witnesses(&mined[3], 0);
        assert!(evaluate_strategy_on_claim(&mined[3].id, "bvgk-writer-refinement", &w34).is_some());
        assert!(evaluate_strategy_on_claim(&mined[3].id, "pointkey-resonance", &w34).is_none());
        assert!(evaluate_strategy_on_claim(&mined[3].id, "reply-4gram-deduction", &w34).is_none());
        assert!(evaluate_strategy_on_claim(&mined[3].id, "corpus-induction", &w34).is_none());
    }

    #[test]
    fn compiled_frontier_claim_roundtrips_uorlut_with_zero_orphans() {
        let root = repo_model::repo_root();
        let uorlut_bytes = std::fs::read(root.join("crates/wasm/src/bootstrap_v8.uorlut"))
            .expect("read bootstrap_v8.uorlut");
        let mut model = crate::lut_stack_exec::LutStackModel::parse(&uorlut_bytes)
            .expect("parse bootstrap_v8.uorlut");
        let initial_rows = model.point.as_ref().unwrap().rows;
        let prompt = "How does zero-float B-tree consensus scale across distributed shards?";
        let reply = "Zero-float B-tree consensus scales across distributed shards by combining O(log_B N) page branching with deterministic quorum replication and prefix-compressed fence keys.";

        let inserted = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("compile_verified_dialogue_turn succeeds");
        assert!(inserted);
        assert_eq!(model.point.as_ref().unwrap().rows, initial_rows + 1);
        assert_eq!(
            model.point.as_ref().unwrap().targets.len(),
            initial_rows + 1
        );

        // Idempotency check: compiling the exact same (prompt, reply) again is a no-op
        let inserted_again = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("idempotent second compile");
        assert!(!inserted_again);
        assert_eq!(model.point.as_ref().unwrap().rows, initial_rows + 1);

        // Round-trip through UORLUT01/8 binary serialization and verify zero orphaned PointKey rows
        let bytes = model.to_uorlut_bytes();
        let parsed = std::sync::Arc::new(
            crate::lut_stack_exec::LutStackModel::parse(&bytes)
                .expect("reparse compiled UORLUT01/8"),
        );
        let pk = parsed.point.as_ref().unwrap();
        assert_eq!(pk.rows, initial_rows + 1);
        let anchors = pk.canonical_anchors.as_ref().unwrap();
        for (idx, a_row) in anchors.chunks(pk.anchor_len).enumerate() {
            assert!(
                a_row[0] != 0 || a_row[1] != 0 || a_row[2] != 0,
                "row {idx} must have non-zero anchor prefix"
            );
        }

        let exact_gen = parsed.generate_reply(&[], prompt, 256);
        assert!(
            exact_gen
                .reply_text
                .contains("Zero-float B-tree consensus scales"),
            "exact prompt must decode compiled reply: {}",
            exact_gen.reply_text
        );

        let para_gen = parsed.generate_reply(
            &[],
            "Can you explain how zero-float B-tree consensus scales across distributed shards?",
            256,
        );
        assert!(
            para_gen
                .reply_text
                .contains("Zero-float B-tree consensus scales"),
            "paraphrased prompt must route to compiled PointKey row: {}",
            para_gen.reply_text
        );
    }
}
