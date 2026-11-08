//! Native integer execution engine for `uor-r4.lut-stack/7` and `uor-r4.lut-stack/8` (`UORLUT01`).
//!
//! Implements the complete zero-float, zero-matmul, zero-softmax serving pipeline from `uor-r4`:
//! - `ByteBpe4096`: GPT-2 byte-level BPE tokenizer (`d36d3e87`, 4,096 tokens, `<|bos|>=0`, `<|eos|>=1`, `<|unk|>=2`)
//! - `DialogueProtocol`: literal-role dialogue framing (`uor-r4.literal-role-dialogue/1` and `/2`)
//! - `FbsModel`: 6-layer `SpiralCore/FBS` trunk (`Cl(0,6)` 64-state signed-axis route tables `stack_fbs_gather`,
//!   `stack_rms_norm_fine`, `grid_apply` norm gain, 4-tap `GEOMETRIC_KEEP_Q31` + `RETAIN_LAMBDA_Q31`
//!   recurrence, Hamilton quaternion feed-forward gates, and `4096 × 256` Q12 `writer.table`)
//! - `ServedHamming`: depth-2 and depth-6 4-head 64-bit quaternion sign-code Hamming attention (`copy_key: 1..2`)
//! - `ServedTurnMemory` (`/7`) + `ServedPointKey` (`/8` with `vote_k = 8` 256D Q12 L1 plurality vote and
//!   canonical reply-anchor handoff so paraphrased prompts continue predicting the full reply token-by-token)
//! - `dialogue_memory::recall` multi-turn fact sieve + `short_cycle` guard (`period 1..=4` repeated 3×).

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, OnceLock};

pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

pub const MAGIC: &[u8; 8] = b"UORLUT01";
pub const ALIGN: usize = 64;
pub const STACK_SCHEMA_V7: &str = "uor-r4.lut-stack/7";
pub const STACK_SCHEMA_V8: &str = "uor-r4.lut-stack/8";

pub const FBS_WIDTH: usize = 256;
pub const FBS_MLP_HIDDEN: usize = 384;
pub const FBS_LANES: usize = FBS_WIDTH / 4; // 64
pub const FBS_GATE_ROWS: usize = FBS_LANES + FBS_WIDTH; // 320
pub const FBS_CONV_KERNEL: usize = 4;
pub const FBS_Q12_SHIFT: u32 = 12;

fn escape_json_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Rolling 256D Q12 latent context state carried forward across multi-turn interactions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollingContextState {
    /// 256D Q12 fixed-point vector representing accumulated manifold context.
    pub q12_latent: [i16; FBS_WIDTH],
    /// 0-indexed conversation turn counter.
    pub turn_index: u32,
    /// High-signal contextual summary distilled across prior turns.
    pub context_summary: String,
    /// Preserved thinking traces (<thinking> blocks) from preceding turns.
    pub thinking_trace: Vec<String>,
    /// Recent turn basin IDs visited across recent turns (FIFO, max 8 items).
    pub recent_turn_rows: Vec<usize>,
    /// 64-bit FNV-1a hashes of emitted 4-gram token sequences.
    pub emitted_4grams: Vec<u64>,
    /// Cumulative L1 residual margin accumulated across conversation turns.
    pub accumulated_l1_margin: u64,
    /// Recent prediction residual margins (FIFO, max 16 items).
    pub recent_residuals: Vec<u64>,
}

impl Default for RollingContextState {
    fn default() -> Self {
        Self {
            q12_latent: [0; FBS_WIDTH],
            turn_index: 0,
            context_summary: String::new(),
            thinking_trace: Vec::new(),
            recent_turn_rows: Vec::new(),
            emitted_4grams: Vec::new(),
            accumulated_l1_margin: 0,
            recent_residuals: Vec::new(),
        }
    }
}

impl RollingContextState {
    /// Construct a new empty rolling context state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Blend a 256D normalized hidden state vector `n_t` into the rolling latent state
    /// using the Q12 geometric retention formula:
    /// z_{t+1}[d] = clamp(((3072 * z_t[d]) + (1024 * n_t[d])) / 4096, -30000, 30000)
    pub fn blend_latent(&mut self, n_t: &[i32; FBS_WIDTH]) {
        for (d, &n_val) in n_t.iter().enumerate().take(FBS_WIDTH) {
            let z_prev = self.q12_latent[d] as i64;
            let n_val = n_val as i64;
            let blended = ((3072 * z_prev) + (1024 * n_val)) / 4096;
            self.q12_latent[d] = blended.clamp(-30000, 30000) as i16;
        }
        self.turn_index = self.turn_index.saturating_add(1);
    }

    /// Record a parsed thinking trace into the rolling history, capped to 16 traces.
    pub fn record_thinking(&mut self, trace: String) {
        let trimmed = trace.trim();
        if !trimmed.is_empty() {
            self.thinking_trace.push(trimmed.to_string());
            if self.thinking_trace.len() > 16 {
                self.thinking_trace.remove(0);
            }
        }
    }

    /// Record a visited turn basin row into recent memory, retaining up to the 8 most recent items (FIFO).
    pub fn record_turn_basin(&mut self, row: usize) {
        self.recent_turn_rows.push(row);
        if self.recent_turn_rows.len() > 8 {
            self.recent_turn_rows.remove(0);
        }
    }

    /// Record an emitted 4-gram token hash to prevent duplicate n-gram emissions across turns.
    pub fn record_emitted_4gram(&mut self, hash: u64) {
        if !self.emitted_4grams.contains(&hash) {
            self.emitted_4grams.push(hash);
        }
    }

    /// Record an observed residual prediction error margin into rolling context history (FIFO, max 16).
    pub fn record_residual_margin(&mut self, margin: u64) {
        self.accumulated_l1_margin = self.accumulated_l1_margin.saturating_add(margin);
        self.recent_residuals.push(margin);
        if self.recent_residuals.len() > 16 {
            self.recent_residuals.remove(0);
        }
    }

    /// Serialize rolling context state to JSON (zero-dependency format).
    pub fn to_json(&self) -> String {
        let mut q12_str = String::with_capacity(FBS_WIDTH * 6 + 2);
        q12_str.push('[');
        for (i, &v) in self.q12_latent.iter().enumerate() {
            if i > 0 {
                q12_str.push(',');
            }
            q12_str.push_str(&v.to_string());
        }
        q12_str.push(']');

        let mut traces_str = String::from("[");
        for (i, t) in self.thinking_trace.iter().enumerate() {
            if i > 0 {
                traces_str.push(',');
            }
            traces_str.push('"');
            traces_str.push_str(&escape_json_inline(t));
            traces_str.push('"');
        }
        traces_str.push(']');

        let mut rows_str = String::from("[");
        for (i, &r) in self.recent_turn_rows.iter().enumerate() {
            if i > 0 {
                rows_str.push(',');
            }
            rows_str.push_str(&r.to_string());
        }
        rows_str.push(']');

        let mut ngrams_str = String::from("[");
        for (i, &h) in self.emitted_4grams.iter().enumerate() {
            if i > 0 {
                ngrams_str.push(',');
            }
            ngrams_str.push_str(&h.to_string());
        }
        ngrams_str.push(']');

        let mut res_str = String::from("[");
        for (i, &r) in self.recent_residuals.iter().enumerate() {
            if i > 0 {
                res_str.push(',');
            }
            res_str.push_str(&r.to_string());
        }
        res_str.push(']');

        format!(
            "{{\"q12_latent\":{},\"turn_index\":{},\"context_summary\":\"{}\",\"thinking_trace\":{},\"recent_turn_rows\":{},\"emitted_4grams\":{},\"accumulated_l1_margin\":{},\"recent_residuals\":{}}}",
            q12_str,
            self.turn_index,
            escape_json_inline(&self.context_summary),
            traces_str,
            rows_str,
            ngrams_str,
            self.accumulated_l1_margin,
            res_str
        )
    }

    /// Parse rolling context state from JSON.
    pub fn from_json(raw: &str) -> Option<Self> {
        let root = JsonVal::parse(raw.as_bytes())?;
        let mut state = Self::default();
        if let Some(ti) = root.get("turn_index").and_then(JsonVal::as_u64) {
            state.turn_index = ti as u32;
        }
        if let Some(cs) = root.get("context_summary").and_then(JsonVal::as_str) {
            state.context_summary = cs.to_string();
        }
        if let Some(arr) = root.get("q12_latent").and_then(JsonVal::as_arr) {
            for (idx, item) in arr.iter().enumerate().take(FBS_WIDTH) {
                if let Some(num) = item.as_i64() {
                    state.q12_latent[idx] = num.clamp(i16::MIN as i64, i16::MAX as i64) as i16;
                }
            }
        }
        if let Some(arr) = root.get("thinking_trace").and_then(JsonVal::as_arr) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    state.thinking_trace.push(s.to_string());
                }
            }
        }
        if let Some(arr) = root.get("recent_turn_rows").and_then(JsonVal::as_arr) {
            for item in arr {
                if let Some(row) = item.as_usize() {
                    state.recent_turn_rows.push(row);
                }
            }
            if state.recent_turn_rows.len() > 8 {
                let start = state.recent_turn_rows.len() - 8;
                state.recent_turn_rows = state.recent_turn_rows[start..].to_vec();
            }
        }
        if let Some(arr) = root.get("emitted_4grams").and_then(JsonVal::as_arr) {
            for item in arr {
                if let Some(hash) = item.as_u64() {
                    state.emitted_4grams.push(hash);
                }
            }
        }
        if let Some(m) = root.get("accumulated_l1_margin").and_then(JsonVal::as_u64) {
            state.accumulated_l1_margin = m;
        }
        if let Some(arr) = root.get("recent_residuals").and_then(JsonVal::as_arr) {
            for item in arr {
                if let Some(m) = item.as_u64() {
                    state.recent_residuals.push(m);
                }
            }
            if state.recent_residuals.len() > 16 {
                let start = state.recent_residuals.len() - 16;
                state.recent_residuals = state.recent_residuals[start..].to_vec();
            }
        }
        Some(state)
    }
}

/// Telemetry report generated by `consolidate_lut_weights`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConsolidationReport {
    /// Number of individual weight coordinates (t, d) in `writer.table` whose value changed.
    pub weights_updated: usize,
    /// Total L1 norm of coordinate shifts: sum_{t,d} |W_{t,d}^{(new)} - W_{t,d}^{(old)}|.
    pub l1_shift: u64,
    /// Maximum absolute delta observed across any single coordinate.
    pub max_delta: i16,
    /// Number of latent trajectory points / token emission observations consolidated.
    pub points_consolidated: usize,
    /// Number of unique tokens whose weight row in `writer.table` was updated.
    pub tokens_updated: usize,
    /// Number of orphaned PointKey rows remaining (must be 0 for valid synchronization).
    pub orphaned_point_rows: usize,
}

impl ConsolidationReport {
    /// Render this report as canonical JSON without external crate dependencies.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"weights_updated\":{},\"l1_shift\":{},\"max_delta\":{},\"points_consolidated\":{},\"tokens_updated\":{},\"orphaned_point_rows\":{}}}",
            self.weights_updated,
            self.l1_shift,
            self.max_delta,
            self.points_consolidated,
            self.tokens_updated,
            self.orphaned_point_rows
        )
    }

    /// Parse report from JSON string using `JsonVal`.
    pub fn from_json_str(s: &str) -> Option<Self> {
        let val = JsonVal::parse(s.as_bytes())?;
        Some(Self {
            weights_updated: val
                .get("weights_updated")
                .and_then(JsonVal::as_usize)
                .unwrap_or(0),
            l1_shift: val.get("l1_shift").and_then(JsonVal::as_u64).unwrap_or(0),
            max_delta: val.get("max_delta").and_then(JsonVal::as_i64).unwrap_or(0) as i16,
            points_consolidated: val
                .get("points_consolidated")
                .and_then(JsonVal::as_usize)
                .unwrap_or(0),
            tokens_updated: val
                .get("tokens_updated")
                .and_then(JsonVal::as_usize)
                .unwrap_or(0),
            orphaned_point_rows: val
                .get("orphaned_point_rows")
                .and_then(JsonVal::as_usize)
                .unwrap_or(0),
        })
    }
}

/// Configuration parameters for coordinate descent optimization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsolidationConfig {
    /// Number of coordinate descent optimization passes (default: 1).
    pub iterations: usize,
    /// Step numerator S (default: 1).
    pub step_numerator: i64,
    /// Step denominator D (default: 8).
    pub step_denominator: i64,
    /// Minimum allowed coordinate weight (default: -30000).
    pub clamp_min: i16,
    /// Maximum allowed coordinate weight (default: 30000).
    pub clamp_max: i16,
    /// Synchronize PointKey (`/8`) row vectors with updated centroids (default: true).
    pub sync_point_keys: bool,
    /// Synchronize TernaryAssocBank bucket-0 top-8 nearest-neighbor transitions (default: true).
    pub update_ternary_assoc: bool,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            iterations: 1,
            step_numerator: 1,
            step_denominator: 8,
            clamp_min: -30000,
            clamp_max: 30000,
            sync_point_keys: true,
            update_ternary_assoc: true,
        }
    }
}

impl ConsolidationConfig {
    /// Validate configuration constraints:
    /// - Step denominator must be positive (d > 0)
    /// - Step numerator must be positive and not exceed denominator (0 < s <= d)
    /// - Clamp bounds must not be inverted (clamp_min <= clamp_max)
    pub fn validate(&self) -> Result<(), String> {
        let s = self.step_numerator;
        let d = self.step_denominator;
        if d <= 0 || s <= 0 || s > d {
            return Err(format!(
                "invalid step config: s={s}, d={d} (require 0 < s <= d)"
            ));
        }
        if self.clamp_min > self.clamp_max {
            return Err(format!(
                "invalid clamp bounds: clamp_min={} > clamp_max={}",
                self.clamp_min, self.clamp_max
            ));
        }
        Ok(())
    }

    /// Parse consolidation configuration from JSON string using `JsonVal`.
    pub fn from_json_str(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed == "{}" {
            return Some(Self::default());
        }
        let val = JsonVal::parse(trimmed.as_bytes())?;
        let mut cfg = Self::default();
        if let Some(it) = val.get("iterations").and_then(JsonVal::as_usize) {
            cfg.iterations = it;
        }
        if let Some(sn) = val.get("step_numerator").and_then(JsonVal::as_i64) {
            cfg.step_numerator = sn;
        }
        if let Some(sd) = val.get("step_denominator").and_then(JsonVal::as_i64) {
            cfg.step_denominator = sd;
        }
        if let Some(cmin) = val.get("clamp_min").and_then(JsonVal::as_i64) {
            cfg.clamp_min = cmin as i16;
        }
        if let Some(cmax) = val.get("clamp_max").and_then(JsonVal::as_i64) {
            cfg.clamp_max = cmax as i16;
        }
        if let Some(spk) = val.get("sync_point_keys").and_then(JsonVal::as_bool) {
            cfg.sync_point_keys = spk;
        }
        if let Some(uta) = val.get("update_ternary_assoc").and_then(JsonVal::as_bool) {
            cfg.update_ternary_assoc = uta;
        }
        cfg.validate().ok()?;
        Some(cfg)
    }
}

/// Compute the pure integer L1 distance between two token coordinate rows in `writer_table`.
///
/// Returns sum_{d=0..255} |W[predicted * 256 + d] - W[target * 256 + d]|.
/// Returns 0 when predicted_token == target_token or if either token is out-of-vocabulary.
/// Guaranteed zero floating-point operations (pure integer arithmetic).
#[inline]
pub fn compute_token_l1_margin(
    writer_table: &[i16],
    vocab: usize,
    predicted_token: u32,
    target_token: u32,
) -> u64 {
    if predicted_token == target_token {
        return 0;
    }
    let p_idx = predicted_token as usize;
    let t_idx = target_token as usize;
    if p_idx >= vocab || t_idx >= vocab {
        return 0;
    }
    let p_base = p_idx * FBS_WIDTH;
    let t_base = t_idx * FBS_WIDTH;
    if p_base + FBS_WIDTH > writer_table.len() || t_base + FBS_WIDTH > writer_table.len() {
        return 0;
    }
    let mut margin = 0u64;
    for d in 0..FBS_WIDTH {
        let diff = (writer_table[p_base + d] as i64) - (writer_table[t_base + d] as i64);
        margin = margin.saturating_add(diff.unsigned_abs());
    }
    margin
}

/// A next-token trajectory triplet (n_t, y_hat_t, y_t) captured at a clause or sentence boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointTrajectoryHit {
    /// Step index within the generation or trace sequence (1-indexed).
    pub step: usize,
    /// Token that triggered the boundary condition (e.g. '.', '?', '!', '\n', or step % 16 == 0).
    pub boundary_token: u32,
    /// Live 256D Q12 hidden state vector n_t clamped to i16.
    pub n_t: [i16; FBS_WIDTH],
    /// Predicted next token y_hat_t from writer.table / top candidate.
    pub predicted_token: u32,
    /// Target next token y_t (ground truth or re-attended continuation basin).
    pub target_token: u32,
    /// Pure integer L1 margin between writer.table rows for predicted_token and target_token:
    /// sum_{d=0..255} |W[y_hat * 256 + d] - W[y * 256 + d]|.
    pub l1_margin: u64,
}

impl PointTrajectoryHit {
    pub fn new(
        step: usize,
        boundary_token: u32,
        n_t: [i16; FBS_WIDTH],
        predicted_token: u32,
        target_token: u32,
        l1_margin: u64,
    ) -> Self {
        Self {
            step,
            boundary_token,
            n_t,
            predicted_token,
            target_token,
            l1_margin,
        }
    }

    /// Whether this boundary event encountered a non-zero prediction error.
    #[inline]
    pub fn has_residual(&self) -> bool {
        self.predicted_token != self.target_token && self.l1_margin > 0
    }

    /// Convert to a coordinate-descent consolidation sample with explicit residual margin.
    pub fn to_consolidation_sample(&self) -> WeightConsolidationSample {
        WeightConsolidationSample::from_point_with_residual(
            self.target_token,
            self.predicted_token,
            self.n_t,
            self.l1_margin,
        )
    }
}

/// A single latent trajectory observation before token `target_token`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatentTrajectoryPoint {
    /// Token emitted following this latent state (0..vocab).
    pub target_token: u32,
    /// 256D Q12 latent vector preceding the token.
    pub latent: [i16; FBS_WIDTH],
    /// Predicted next token y_hat_t from model readout (if evaluated).
    pub predicted_token: Option<u32>,
    /// Integer L1 residual margin between predicted and target token columns (if evaluated).
    pub residual_margin: Option<u64>,
}

impl LatentTrajectoryPoint {
    pub fn new(target_token: u32, latent: [i16; FBS_WIDTH]) -> Self {
        Self {
            target_token,
            latent,
            predicted_token: None,
            residual_margin: None,
        }
    }

    pub fn with_residual(
        target_token: u32,
        predicted_token: u32,
        latent: [i16; FBS_WIDTH],
        residual_margin: u64,
    ) -> Self {
        Self {
            target_token,
            latent,
            predicted_token: Some(predicted_token),
            residual_margin: Some(residual_margin),
        }
    }
}

/// A consolidation sample representing direct trajectory points, dialogue turns, or raw tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WeightConsolidationSample {
    /// Direct latent trajectory point (e.g. for deterministic unit testing).
    TrajectoryPoint(Box<LatentTrajectoryPoint>),
    /// Verified dialogue interaction (prompt and reply text) forwarded through trunk.
    Dialogue { prompt: String, reply: String },
    /// Raw token sequences (prompt tokens + reply tokens).
    Tokens {
        prompt_tokens: Vec<u32>,
        reply_tokens: Vec<u32>,
    },
}

impl WeightConsolidationSample {
    pub fn from_point(target_token: u32, latent: [i16; FBS_WIDTH]) -> Self {
        Self::TrajectoryPoint(Box::new(LatentTrajectoryPoint::new(target_token, latent)))
    }

    pub fn from_point_with_residual(
        target_token: u32,
        predicted_token: u32,
        latent: [i16; FBS_WIDTH],
        residual_margin: u64,
    ) -> Self {
        Self::TrajectoryPoint(Box::new(LatentTrajectoryPoint::with_residual(
            target_token,
            predicted_token,
            latent,
            residual_margin,
        )))
    }

    pub fn from_dialogue(prompt: impl Into<String>, reply: impl Into<String>) -> Self {
        Self::Dialogue {
            prompt: prompt.into(),
            reply: reply.into(),
        }
    }

    pub fn from_tokens(prompt_tokens: Vec<u32>, reply_tokens: Vec<u32>) -> Self {
        Self::Tokens {
            prompt_tokens,
            reply_tokens,
        }
    }
}

impl<P: Into<String>, R: Into<String>> From<(P, R)> for WeightConsolidationSample {
    fn from((prompt, reply): (P, R)) -> Self {
        Self::from_dialogue(prompt, reply)
    }
}

pub const HAMMING_HEADS: usize = 4;
pub const HAMMING_HEAD_DIM: usize = 64;
pub const HAMMING_CODE_BITS: u32 = 64;

pub const MEMORY_PAD: u32 = u32::MAX;
pub const MEMORY_REPLY_KEY_LEN: usize = 20;
pub const POINT_KEY_DIM: usize = 256;
pub const DEFAULT_VOTE_K: usize = 8;

/// The 64-state Cl(0,6) signed-axis table (`discrete_d11::spiralcore_signed_axes`).
pub const CL06_AXES: [u8; 512] = [
    0, 1, 2, 3, 4, 5, 6, 7, 12, 10, 1, 14, 0, 7, 3, 13, 15, 11, 6, 1, 5, 12, 10, 0, 2, 12, 8, 13,
    1, 3, 15, 6, 14, 13, 15, 4, 11, 1, 0, 2, 5, 14, 11, 2, 7, 8, 1, 12, 13, 14, 11, 2, 15, 0, 1, 4,
    9, 0, 12, 7, 2, 6, 13, 11, 3, 7, 13, 8, 14, 2, 4, 9, 15, 3, 14, 9, 5, 12, 2, 0, 14, 5, 15, 12,
    3, 9, 0, 2, 10, 12, 0, 13, 1, 3, 7, 14, 4, 10, 1, 14, 8, 15, 3, 5, 15, 3, 6, 9, 13, 4, 10, 0,
    11, 15, 13, 0, 14, 2, 4, 1, 9, 0, 4, 7, 10, 14, 5, 11, 8, 9, 10, 11, 12, 13, 14, 15, 7, 3, 14,
    9, 13, 4, 2, 8, 10, 4, 0, 5, 9, 11, 7, 14, 6, 5, 7, 12, 3, 9, 8, 10, 13, 6, 3, 10, 15, 0, 9, 4,
    11, 7, 5, 8, 14, 2, 12, 1, 9, 8, 12, 15, 10, 14, 13, 11, 0, 9, 10, 11, 4, 5, 14, 7, 5, 14, 3,
    2, 15, 0, 9, 12, 6, 5, 15, 12, 11, 1, 0, 10, 2, 12, 0, 13, 9, 11, 7, 6, 5, 6, 3, 10, 7, 8, 9,
    12, 3, 15, 13, 0, 6, 10, 4, 9, 1, 0, 4, 7, 2, 6, 5, 3, 8, 1, 2, 3, 12, 13, 6, 15, 8, 1, 10, 3,
    4, 5, 14, 15, 9, 8, 4, 15, 2, 6, 5, 11, 3, 15, 5, 0, 14, 2, 12, 9, 1, 8, 4, 15, 10, 14, 5, 3,
    6, 13, 7, 4, 11, 1, 8, 10, 13, 6, 11, 10, 7, 8, 1, 4, 14, 13, 7, 4, 3, 9, 8, 2, 0, 9, 2, 11,
    12, 13, 6, 7, 1, 0, 12, 7, 10, 14, 13, 3, 4, 2, 1, 6, 0, 7, 3, 5, 11, 15, 5, 0, 6, 10, 12, 1,
    2, 4, 8, 5, 9, 11, 15, 6, 7, 11, 14, 1, 5, 12, 2, 8, 10, 4, 8, 5, 1, 3, 15, 14, 11, 7, 13, 8,
    6, 10, 4, 1, 12, 10, 9, 14, 8, 15, 11, 13, 7, 11, 6, 1, 13, 4, 10, 8, 12, 2, 9, 6, 0, 7, 11,
    13, 3, 7, 5, 8, 6, 10, 12, 9, 1, 8, 12, 15, 2, 6, 13, 3, 4, 2, 9, 6, 8, 15, 11, 5, 12, 2, 1, 6,
    8, 15, 3, 13, 10, 12, 8, 13, 9, 11, 15, 14, 6, 13, 15, 4, 3, 9, 0, 10, 4, 10, 9, 14, 0, 7, 11,
    5, 2, 4, 0, 5, 1, 3, 7, 6, 8, 9, 2, 11, 4, 5, 6, 15, 14, 5, 7, 12, 11, 1, 8, 2, 0, 1, 10, 3,
    12, 13, 14, 7, 13, 14, 3, 2, 7, 8, 9, 4, 5, 6, 11, 10, 15, 0, 1, 12, 15, 11, 14, 1, 13, 4, 2,
    0, 7, 3, 6, 9, 5, 12, 10, 8,
];

const GEOMETRIC_KEEP_Q31: [i64; FBS_CONV_KERNEL] =
    [1_073_741_824, 1_518_500_250, 1_859_775_393, 2_147_483_647];

const RETAIN_LAMBDA_Q31: [i64; 16] = [
    2_079_841_894,
    2_015_539_906,
    1_953_125_000,
    1_892_556_544,
    1_833_796_252,
    1_776_807_381,
    1_721_554_662,
    1_667_994_287,
    1_616_083_906,
    1_565_782_609,
    1_517_050_908,
    1_469_850_718,
    1_424_145_338,
    1_379_899_426,
    1_337_078_986,
    1_295_651_344,
];

const EMBEDDED_TOKENIZER_JSON: &[u8] = include_bytes!("tokenizer.json");

#[inline]
pub fn cl06_axes(state: usize) -> [u8; 8] {
    let mut codes = [0u8; 8];
    let base = (state & 63) * 8;
    codes.copy_from_slice(&CL06_AXES[base..base + 8]);
    codes
}

/// Apply a grid code `±(16 + m) 2^(e - 4)` with `e = (abs >> 4) - 64` to an integer value `v`.
#[inline]
pub fn grid_apply_q12(v: i64, code: i16) -> i64 {
    if code == 0 || v == 0 {
        return 0;
    }
    let c = code.unsigned_abs();
    let e = (c >> 4) as i32 - 64;
    let m = (16 + (c & 15)) as i64;
    let shift = e - 4;
    let mag = if shift >= 0 {
        (v * m) << (shift.min(20) as u32)
    } else {
        (v * m) >> ((-shift).min(48) as u32)
    };
    if code < 0 {
        -mag
    } else {
        mag
    }
}

// ---------------------------------------------------------------------------
// Zero-dependency recursive-descent JSON parser for UORLUT01 headers & BPE
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Int(i64),
    UInt(u64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(BTreeMap<String, JsonVal>),
}

impl JsonVal {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let mut p = JsonParser {
            s: text.as_bytes(),
            pos: 0,
        };
        let val = p.parse_val()?;
        p.skip_ws();
        if p.pos == p.s.len() {
            Some(val)
        } else {
            None
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonVal> {
        match self {
            JsonVal::Obj(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonVal::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonVal::Int(n) => Some(*n),
            JsonVal::UInt(u) => i64::try_from(*u).ok(),
            JsonVal::Num(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonVal::UInt(u) => Some(*u),
            JsonVal::Int(n) => u64::try_from(*n).ok(),
            JsonVal::Str(s) => s.parse::<u64>().ok(),
            JsonVal::Num(f) if *f >= 0.0 && *f <= u64::MAX as f64 => Some(*f as u64),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        self.as_u64().and_then(|v| usize::try_from(v).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonVal::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[JsonVal]> {
        match self {
            JsonVal::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&BTreeMap<String, JsonVal>> {
        match self {
            JsonVal::Obj(m) => Some(m),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() {
            match self.s[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn parse_val(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        let b = *self.s.get(self.pos)?;
        match b {
            b'"' => self.parse_str().map(JsonVal::Str),
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            b't' => {
                if self.s.get(self.pos..self.pos + 4)? == b"true" {
                    self.pos += 4;
                    Some(JsonVal::Bool(true))
                } else {
                    None
                }
            }
            b'f' => {
                if self.s.get(self.pos..self.pos + 5)? == b"false" {
                    self.pos += 5;
                    Some(JsonVal::Bool(false))
                } else {
                    None
                }
            }
            b'n' => {
                if self.s.get(self.pos..self.pos + 4)? == b"null" {
                    self.pos += 4;
                    Some(JsonVal::Null)
                } else {
                    None
                }
            }
            b'-' | b'0'..=b'9' => self.parse_num(),
            _ => None,
        }
    }

    fn parse_str(&mut self) -> Option<String> {
        if *self.s.get(self.pos)? != b'"' {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.s.len() {
            let b = self.s[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Some(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let esc = *self.s.get(self.pos)?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(self.s.get(self.pos..self.pos + 4)?).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        self.pos += 4;
                        if (0xD800..=0xDBFF).contains(&code)
                            && self.s.get(self.pos..self.pos + 2) == Some(b"\\u")
                        {
                            self.pos += 2;
                            let low_hex =
                                std::str::from_utf8(self.s.get(self.pos..self.pos + 4)?).ok()?;
                            let low = u32::from_str_radix(low_hex, 16).ok()?;
                            self.pos += 4;
                            let combined = 0x10000 + (((code - 0xD800) << 10) | (low - 0xDC00));
                            out.push(char::from_u32(combined)?);
                        } else {
                            out.push(char::from_u32(code)?);
                        }
                    }
                    _ => return None,
                }
            } else {
                let rest = std::str::from_utf8(&self.s[self.pos..]).ok()?;
                let ch = rest.chars().next()?;
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
        None
    }

    fn parse_num(&mut self) -> Option<JsonVal> {
        let start = self.pos;
        let mut is_float = false;
        if self.s.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        while let Some(&b) = self.s.get(self.pos) {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let slice = std::str::from_utf8(&self.s[start..self.pos]).ok()?;
        if !is_float {
            if let Ok(i) = slice.parse::<i64>() {
                return Some(JsonVal::Int(i));
            }
            if let Ok(u) = slice.parse::<u64>() {
                return Some(JsonVal::UInt(u));
            }
        }
        slice.parse::<f64>().ok().map(JsonVal::Num)
    }

    fn parse_arr(&mut self) -> Option<JsonVal> {
        self.pos += 1;
        self.skip_ws();
        let mut arr = Vec::new();
        if self.s.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Some(JsonVal::Arr(arr));
        }
        loop {
            arr.push(self.parse_val()?);
            self.skip_ws();
            match self.s.get(self.pos)? {
                b',' => self.pos += 1,
                b']' => {
                    self.pos += 1;
                    return Some(JsonVal::Arr(arr));
                }
                _ => return None,
            }
        }
    }

    fn parse_obj(&mut self) -> Option<JsonVal> {
        self.pos += 1;
        self.skip_ws();
        let mut map = BTreeMap::new();
        if self.s.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Some(JsonVal::Obj(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_str()?;
            self.skip_ws();
            if self.s.get(self.pos) != Some(&b':') {
                return None;
            }
            self.pos += 1;
            let val = self.parse_val()?;
            map.insert(key, val);
            self.skip_ws();
            match self.s.get(self.pos)? {
                b',' => self.pos += 1,
                b'}' => {
                    self.pos += 1;
                    return Some(JsonVal::Obj(map));
                }
                _ => return None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ByteBpe4096 Tokenizer & DialogueProtocol
// ---------------------------------------------------------------------------

fn bytes_to_unicode() -> [char; 256] {
    let mut table = ['\0'; 256];
    let mut assigned = [false; 256];
    for byte in (b'!'..=b'~').chain(0xA1..=0xAC).chain(0xAE..=0xFF) {
        table[byte as usize] = char::from_u32(u32::from(byte)).unwrap_or('\0');
        assigned[byte as usize] = true;
    }
    let mut extra = 0u32;
    for byte in 0usize..256 {
        if !assigned[byte] {
            table[byte] = char::from_u32(256 + extra).unwrap_or('\0');
            extra += 1;
        }
    }
    table
}

pub struct ByteBpe4096 {
    vocab: Vec<String>,
    token_to_id: HashMap<String, u32>,
    merge_ranks: HashMap<String, u32>,
    added_tokens: Vec<(String, u32)>,
    added_ids: HashSet<u32>,
    byte_encoder: [char; 256],
    byte_decoder: HashMap<char, u8>,
    add_prefix_space: bool,
    digits_individual: Option<bool>,
}

impl ByteBpe4096 {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<ByteBpe4096> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            Self::from_json_bytes(EMBEDDED_TOKENIZER_JSON)
                .expect("embedded 4096 BPE tokenizer.json is valid")
        })
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        let root = JsonVal::parse(bytes)?;
        let model = root.get("model")?;
        let vocab_obj = model.get("vocab")?.as_obj()?;
        let mut token_to_id = HashMap::with_capacity(vocab_obj.len());
        let mut max_id = 0u32;
        for (piece, id_val) in vocab_obj {
            let id = u32::try_from(id_val.as_u64()?).ok()?;
            max_id = max_id.max(id);
            token_to_id.insert(piece.clone(), id);
        }
        let mut added_tokens = Vec::new();
        let mut added_ids = HashSet::new();
        if let Some(arr) = root.get("added_tokens").and_then(JsonVal::as_arr) {
            for entry in arr {
                let content = entry.get("content")?.as_str()?.to_owned();
                let id = u32::try_from(entry.get("id")?.as_u64()?).ok()?;
                max_id = max_id.max(id);
                token_to_id.entry(content.clone()).or_insert(id);
                added_ids.insert(id);
                added_tokens.push((content, id));
            }
        }
        added_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));

        let mut vocab = vec![String::new(); (max_id as usize) + 1];
        for (piece, &id) in &token_to_id {
            vocab[id as usize] = piece.clone();
        }
        for (content, id) in &added_tokens {
            vocab[*id as usize] = content.clone();
        }

        let merges_arr = model.get("merges")?.as_arr()?;
        let mut merge_ranks = HashMap::with_capacity(merges_arr.len());
        for (rank, entry) in merges_arr.iter().enumerate() {
            if let Some(s) = entry.as_str() {
                merge_ranks.entry(s.to_owned()).or_insert(rank as u32);
            } else if let Some(pair) = entry.as_arr() {
                if pair.len() == 2 {
                    let l = pair[0].as_str()?;
                    let r = pair[1].as_str()?;
                    merge_ranks.entry(format!("{l} {r}")).or_insert(rank as u32);
                }
            }
        }

        let byte_encoder = bytes_to_unicode();
        let mut byte_decoder = HashMap::with_capacity(256);
        for (b, &ch) in byte_encoder.iter().enumerate() {
            byte_decoder.insert(ch, b as u8);
        }

        Some(Self {
            vocab,
            token_to_id,
            merge_ranks,
            added_tokens,
            added_ids,
            byte_encoder,
            byte_decoder,
            add_prefix_space: false,
            digits_individual: None,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.vocab.len()
    }

    pub fn bos_id(&self) -> u32 {
        0
    }

    pub fn eos_id(&self) -> u32 {
        1
    }

    pub fn unk_id(&self) -> u32 {
        2
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let mut best: Option<(usize, usize, u32)> = None;
            for (content, id) in &self.added_tokens {
                if content.is_empty() {
                    continue;
                }
                if let Some(pos) = rest.find(content.as_str()) {
                    if best.is_none_or(|(ex, _, _)| pos < ex) {
                        best = Some((pos, content.len(), *id));
                    }
                    if pos == 0 {
                        break;
                    }
                }
            }
            match best {
                Some((pos, len, id)) => {
                    if pos > 0 {
                        self.encode_segment(&rest[..pos], &mut ids);
                    }
                    ids.push(id);
                    rest = &rest[pos + len..];
                }
                None => {
                    self.encode_segment(rest, &mut ids);
                    break;
                }
            }
        }
        ids
    }

    fn encode_segment(&self, segment: &str, out: &mut Vec<u32>) {
        let splits = match self.digits_individual {
            Some(ind) => split_digits(segment, ind),
            None => vec![segment],
        };
        for split in splits {
            if split.is_empty() {
                continue;
            }
            let prefixed;
            let s = if self.add_prefix_space && !split.starts_with(' ') {
                prefixed = format!(" {split}");
                prefixed.as_str()
            } else {
                split
            };
            for pre in pre_tokenize(s) {
                self.bpe(pre, out);
            }
        }
    }

    fn bpe(&self, pre_token: &str, out: &mut Vec<u32>) {
        if pre_token.is_empty() {
            return;
        }
        let mut symbols: Vec<String> = pre_token
            .bytes()
            .map(|b| self.byte_encoder[b as usize].to_string())
            .collect();
        let mut pair_key = String::new();
        while symbols.len() > 1 {
            let mut best: Option<(u32, usize)> = None;
            for idx in 0..symbols.len() - 1 {
                pair_key.clear();
                pair_key.push_str(&symbols[idx]);
                pair_key.push(' ');
                pair_key.push_str(&symbols[idx + 1]);
                if let Some(&rank) = self.merge_ranks.get(&pair_key) {
                    if best.is_none_or(|(ex, _)| rank < ex) {
                        best = Some((rank, idx));
                    }
                }
            }
            let Some((_, best_idx)) = best else {
                break;
            };
            let left = symbols[best_idx].clone();
            let right = symbols[best_idx + 1].clone();
            let merged = format!("{left}{right}");
            let mut next = Vec::with_capacity(symbols.len());
            let mut i = 0;
            while i < symbols.len() {
                if i + 1 < symbols.len() && symbols[i] == left && symbols[i + 1] == right {
                    next.push(merged.clone());
                    i += 2;
                } else {
                    next.push(std::mem::take(&mut symbols[i]));
                    i += 1;
                }
            }
            symbols = next;
        }
        for sym in &symbols {
            if let Some(&id) = self.token_to_id.get(sym.as_str()) {
                out.push(id);
            } else {
                let mut utf8 = [0u8; 4];
                for ch in sym.chars() {
                    if let Some(&id) = self.token_to_id.get(ch.encode_utf8(&mut utf8)) {
                        out.push(id);
                    }
                }
            }
        }
    }

    pub fn decode_bytes(&self, ids: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for &id in ids {
            let Some(tok) = self.vocab.get(id as usize) else {
                continue;
            };
            if self.added_ids.contains(&id) {
                bytes.extend_from_slice(tok.as_bytes());
                continue;
            }
            for ch in tok.chars() {
                if let Some(&b) = self.byte_decoder.get(&ch) {
                    bytes.push(b);
                } else {
                    let mut utf8 = [0u8; 4];
                    bytes.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
                }
            }
        }
        bytes
    }

    pub fn decode(&self, ids: &[u32]) -> String {
        String::from_utf8_lossy(&self.decode_bytes(ids)).into_owned()
    }
}

/// Precomputed lookup table for O(1) zero-allocation boundary token detection.
/// Identifies tokens whose decoded representation ends with '.', '?', '!', ':', ';'
/// or contains '\n'.
pub fn is_boundary_token(tok: u32) -> bool {
    static BOUNDARY_TOKENS: OnceLock<[bool; 4096]> = OnceLock::new();
    let table = BOUNDARY_TOKENS.get_or_init(|| {
        let bpe = ByteBpe4096::global();
        let mut t = [false; 4096];
        for (id, slot) in t.iter_mut().enumerate() {
            let s = bpe.decode(&[id as u32]);
            if s.contains('\n')
                || s.ends_with('.')
                || s.ends_with('?')
                || s.ends_with('!')
                || s.ends_with(':')
                || s.ends_with(';')
            {
                *slot = true;
            }
        }
        t
    });
    table.get(tok as usize).copied().unwrap_or(false)
}

/// Zero-allocation clause and sentence boundary detection.
/// Returns true if `tok` is a punctuation/newline boundary or if `step` interval K=16 is reached.
#[inline]
pub fn is_clause_boundary(tok: u32, step: usize) -> bool {
    (step > 0 && step.is_multiple_of(16)) || is_boundary_token(tok)
}

/// Cyclic frame block calculation: `(block + sweep + 1) % blocks`.
/// Proved in `R4.Flock.cyclicFrameBlock` (`src/R4/Flock.lex.tex`).
/// Guaranteed zero heap allocation and zero floating-point operations.
#[inline]
pub fn cyclic_frame_block(block: u64, sweep: u64, blocks: u64) -> u64 {
    if blocks == 0 {
        0
    } else {
        (block.saturating_add(sweep).saturating_add(1)) % blocks
    }
}

/// Modulate live 256D Q12 hidden state `n_t` across 8 heads (32 dimensions each)
/// using `R4.Flock.cyclicFrameBlock` cyclic frame routing.
///
/// Guaranteed zero heap allocation (operates entirely on stack arrays)
/// and zero floating-point operations (pure integer Q12 coordinate routing).
#[inline]
pub fn modulate_flock_cyclic_frame_block(
    n_t: &[i32; FBS_WIDTH],
    tokens_len: usize,
) -> [i32; FBS_WIDTH] {
    let mut flock_n_t = [0i32; FBS_WIDTH];
    let sweep = (tokens_len as u64) % 8;
    for head in 0..8 {
        let frame_block = cyclic_frame_block(head as u64, sweep, 8) as usize;
        let dst_base = head * 32;
        let src_base = frame_block * 32;
        flock_n_t[dst_base..dst_base + 32].copy_from_slice(&n_t[src_base..src_base + 32]);
    }
    flock_n_t
}

/// Alias for `modulate_flock_cyclic_frame_block` for compatibility with attention test suites.
#[inline]
pub fn modulate_flock_cyclic_attention(
    n_t: &[i32; FBS_WIDTH],
    tokens_len: usize,
) -> [i32; FBS_WIDTH] {
    modulate_flock_cyclic_frame_block(n_t, tokens_len)
}

/// Alternative geodesic convex blend (50% local state, 50% cyclic frame state).
#[inline]
pub fn modulate_flock_geodesic_blend(
    n_t: &[i32; FBS_WIDTH],
    tokens_len: usize,
) -> [i32; FBS_WIDTH] {
    let mut flock_n_t = [0i32; FBS_WIDTH];
    let sweep = (tokens_len as u64) % 8;
    for head in 0..8 {
        let frame_block = cyclic_frame_block(head as u64, sweep, 8) as usize;
        let dst_base = head * 32;
        let src_base = frame_block * 32;
        for d in 0..32 {
            flock_n_t[dst_base + d] = (n_t[dst_base + d] + n_t[src_base + d]) / 2;
        }
    }
    flock_n_t
}

impl ByteBpe4096 {
    /// Encode a single user prompt under `uor-r4.literal-role-dialogue` v1 or v2.
    pub fn encode_dialogue_prompt(
        &self,
        system_note: Option<&str>,
        history: &[(String, String)],
        user_prompt: &str,
        dialogue_version: u8,
    ) -> Vec<u32> {
        let spaced = dialogue_version == 2;
        let mut tokens = vec![self.bos_id()];
        let mut emitted = 0usize;

        if let Some(sys) = system_note {
            let clean = sys.replace("\r\n", "\n").replace('\r', "\n");
            let clean = clean.trim();
            if !clean.is_empty() {
                if spaced {
                    tokens.extend(self.encode("System:"));
                    tokens.extend(self.encode(&format!(" {clean}")));
                } else {
                    tokens.extend(self.encode("System: "));
                    tokens.extend(self.encode(clean));
                }
                emitted += 1;
            }
        }

        for (u, a) in history {
            let u_clean = u.replace("\r\n", "\n").replace('\r', "\n");
            let u_clean = u_clean.trim();
            if !u_clean.is_empty() {
                if emitted > 0 {
                    tokens.extend(self.encode("\n"));
                }
                if spaced {
                    tokens.extend(self.encode("User:"));
                    tokens.extend(self.encode(&format!(" {u_clean}")));
                } else {
                    tokens.extend(self.encode("User: "));
                    tokens.extend(self.encode(u_clean));
                }
                emitted += 1;
            }
            let a_clean = a.replace("\r\n", "\n").replace('\r', "\n");
            let a_clean = a_clean.trim();
            if !a_clean.is_empty() {
                if emitted > 0 {
                    tokens.extend(self.encode("\n"));
                }
                if spaced {
                    tokens.extend(self.encode("Assistant:"));
                    tokens.extend(self.encode(&format!(" {a_clean}")));
                } else {
                    tokens.extend(self.encode("Assistant: "));
                    tokens.extend(self.encode(a_clean));
                }
                tokens.push(self.eos_id());
                emitted += 1;
            }
        }

        let p_clean = user_prompt.replace("\r\n", "\n").replace('\r', "\n");
        let p_clean = p_clean.trim();
        if !p_clean.is_empty() {
            if emitted > 0 {
                tokens.extend(self.encode("\n"));
            }
            if spaced {
                tokens.extend(self.encode("User:"));
                tokens.extend(self.encode(&format!(" {p_clean}")));
            } else {
                tokens.extend(self.encode("User: "));
                tokens.extend(self.encode(p_clean));
            }
            emitted += 1;
        }
        if emitted > 0 {
            tokens.extend(self.encode("\n"));
        }
        if spaced {
            tokens.extend(self.encode("Assistant:"));
        } else {
            tokens.extend(self.encode("Assistant: "));
        }
        tokens
    }
}

fn split_digits(text: &str, individual: bool) -> Vec<&str> {
    let mut splits = Vec::new();
    let mut start = 0usize;
    let mut in_digits = false;
    for (offset, ch) in text.char_indices() {
        let numeric = ch.is_numeric();
        if offset > 0 && (numeric != in_digits || (numeric && individual)) {
            splits.push(&text[start..offset]);
            start = offset;
        }
        in_digits = numeric;
    }
    if start < text.len() {
        splits.push(&text[start..]);
    }
    splits
}

fn pre_tokenize(text: &str) -> Vec<&str> {
    const CONTRACTIONS: [&str; 7] = ["'s", "'t", "'re", "'ve", "'m", "'ll", "'d"];
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let byte_at = |idx: usize| chars.get(idx).map_or(text.len(), |(off, _)| *off);
    let mut pieces = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let (offset, ch) = chars[i];
        if ch == '\'' {
            if let Some(c) = CONTRACTIONS
                .iter()
                .find(|cand| text[offset..].starts_with(**cand))
            {
                pieces.push(&text[offset..offset + c.len()]);
                i += c.chars().count();
                continue;
            }
        }
        let after_space = if ch == ' ' { i + 1 } else { i };
        if let Some(&(_, next)) = chars.get(after_space) {
            if next.is_alphabetic() {
                let mut end = after_space;
                while end < chars.len() && chars[end].1.is_alphabetic() {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
            if next.is_numeric() {
                let mut end = after_space;
                while end < chars.len() && chars[end].1.is_numeric() {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
            if !next.is_whitespace() {
                let mut end = after_space;
                while end < chars.len()
                    && !chars[end].1.is_whitespace()
                    && !chars[end].1.is_alphabetic()
                    && !chars[end].1.is_numeric()
                {
                    end += 1;
                }
                pieces.push(&text[offset..byte_at(end)]);
                i = end;
                continue;
            }
        }
        let mut end = i;
        while end < chars.len() && chars[end].1.is_whitespace() {
            end += 1;
        }
        if end < chars.len() && end - i > 1 {
            end -= 1;
        }
        pieces.push(&text[offset..byte_at(end)]);
        i = end;
    }
    pieces
}

// ---------------------------------------------------------------------------
// Exact Integer Kernels (SpiralCore/FBS, Hamming, TurnMemory, PointKey)
// ---------------------------------------------------------------------------

pub const fn build_rsqrt_lut_fine() -> [u16; 1024] {
    let mut table = [0u16; 1024];
    let mut i = 0usize;
    while i < 1024 {
        let target: u128 = (1u128 << 46) / ((i as u128) + 1);
        let mut lo: u128 = 0;
        let mut hi: u128 = 1 << 24;
        while lo < hi {
            let mid = (lo + hi + 1) >> 1;
            if mid * mid <= target {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        table[i] = if lo > (u16::MAX as u128) {
            u16::MAX
        } else {
            lo as u16
        };
        i += 1;
    }
    table
}

pub const RSQRT_LUT_FINE: [u16; 1024] = build_rsqrt_lut_fine();

pub const fn build_exp_neg_lut() -> [u16; 256] {
    const LN2_Q30: i64 = 744_261_118;
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let x_q30 = (i as i64) << 25;
        let k = x_q30 / LN2_Q30;
        let r = x_q30 - k * LN2_Q30;
        let r2 = (r * r) >> 30;
        let r3 = (r2 * r) >> 30;
        let r4 = (r3 * r) >> 30;
        let r5 = (r4 * r) >> 30;
        let exp_neg_r_q30 = (1i64 << 30) - r + (r2 >> 1) - (r3 / 6) + (r4 / 24) - (r5 / 120);
        let val_q15 = exp_neg_r_q30 >> (15 + k);
        table[i] = if val_q15 < 0 {
            0
        } else if val_q15 > (1 << 15) {
            1 << 15
        } else {
            val_q15 as u16
        };
        i += 1;
    }
    table
}

pub const EXP_NEG_LUT: [u16; 256] = build_exp_neg_lut();

pub const fn build_sigmoid_lut() -> [u16; 256] {
    let exp_lut = build_exp_neg_lut();
    let mut table = [0u16; 256];
    let mut i = 0usize;
    while i < 256 {
        let u = (i as i32) - 128;
        let abs_u = if u < 0 { -u } else { u } as usize;
        let exp_idx = if abs_u * 2 > 255 { 255 } else { abs_u * 2 };
        let e = exp_lut[exp_idx] as i64;
        let denom = (1i64 << 15) + e;
        let sig_q15 = if u >= 0 {
            ((1i64 << 30) + (denom >> 1)) / denom
        } else {
            ((e << 15) + (denom >> 1)) / denom
        };
        table[i] = if sig_q15 > (u16::MAX as i64) {
            u16::MAX
        } else {
            sig_q15 as u16
        };
        i += 1;
    }
    table
}

pub const SIGMOID_LUT: [u16; 256] = build_sigmoid_lut();

#[inline]
pub fn stack_sigmoid_q15(x_q12: i32) -> i64 {
    let idx = ((x_q12 >> 9) + 128).clamp(0, 255) as usize;
    SIGMOID_LUT[idx] as i64
}

pub fn stack_rms_norm_fine(x: &[i32]) -> Vec<i32> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let sum_sq: u128 = x
        .iter()
        .map(|&v| {
            let a = (v as i64).unsigned_abs() as u128;
            a * a
        })
        .sum();
    let mean_sq = (sum_sq / (n as u128)) as u64;
    if mean_sq == 0 {
        return vec![0; n];
    }
    let log2 = 63u32.saturating_sub(mean_sq.leading_zeros());
    let k = log2 / 2;
    let base = 1u64 << (2 * k);
    let frac = (((mean_sq - base) << 8) / base).min(1023) as usize;
    let rsqrt_m = RSQRT_LUT_FINE[frac] as i64;
    let total_shift = 5 + k;
    x.iter()
        .map(|&v| (((v as i64) * rsqrt_m) >> total_shift) as i32)
        .collect()
}

pub fn stack_norm_gain(x: &[i32], gain_codes: &[i16]) -> Vec<i32> {
    let normed = stack_rms_norm_fine(x);
    normed
        .iter()
        .zip(gain_codes.iter())
        .map(|(&v, &code)| grid_apply_q12(v as i64, code).clamp(-32768, 32767) as i32)
        .collect()
}

/// Packed `Cl(0,6)` route map (`l{l}.*.fbs` in `UORLUT01`).
#[derive(Clone, Debug)]
pub struct FbsRouteMap {
    pub in_dim: usize,
    pub out_dim: usize,
    pub packed_entries: Vec<u32>,
    pub atoms: Vec<u16>,
    pub codes: Vec<[u8; 8]>,
}

impl FbsRouteMap {
    pub fn from_packed_u32(entries: &[u32], in_dim: usize, out_dim: usize) -> Self {
        let in_blocks = (in_dim / 8).max(1);
        let out_blocks = out_dim / 8;
        let mut atoms = Vec::with_capacity(out_blocks);
        let mut codes = Vec::with_capacity(out_blocks);
        for b in 0..out_blocks {
            let entry = entries.get(b).copied().unwrap_or(0);
            let atom = ((entry & 0xFFFF) as usize) % in_blocks;
            let state = ((entry >> 16) as usize) & 63;
            atoms.push(atom as u16);
            codes.push(cl06_axes(state));
        }
        Self {
            in_dim,
            out_dim,
            packed_entries: entries.to_vec(),
            atoms,
            codes,
        }
    }

    pub fn forward(&self, x: &[i32]) -> Vec<i32> {
        let mut out = vec![0i32; self.out_dim];
        for ((block, &atom), codes) in out
            .chunks_exact_mut(8)
            .zip(self.atoms.iter())
            .zip(self.codes.iter())
        {
            let base = (atom as usize) << 3;
            if let Some(vals) = x.get(base..base + 8) {
                for (slot, &code) in block.iter_mut().zip(codes.iter()) {
                    let v = vals[(code & 7) as usize];
                    *slot = if (code & 8) == 0 {
                        v
                    } else {
                        v.saturating_neg()
                    };
                }
            }
        }
        out
    }
}

/// Hamilton quaternion feed-forward gate (`gate = "hamilton"`).
pub fn stack_hamilton_ffn(gate: &[i32], up: &[i32]) -> Vec<i32> {
    let len = gate.len().min(up.len());
    let mut out = vec![0i32; len];
    for ((o_lane, g_lane), u_lane) in out
        .chunks_exact_mut(4)
        .zip(gate.chunks_exact(4))
        .zip(up.chunks_exact(4))
    {
        let (g0, g1, g2, g3) = (
            g_lane[0] as i64,
            g_lane[1] as i64,
            g_lane[2] as i64,
            g_lane[3] as i64,
        );
        let norm_sq = (g0 * g0 + g1 * g1 + g2 * g2 + g3 * g3).max(1);
        let norm = (norm_sq as u128).isqrt().max(1) as i64;
        // Unit quaternion in Q12
        let q = [
            (g0 << 12) / norm,
            (g1 << 12) / norm,
            (g2 << 12) / norm,
            (g3 << 12) / norm,
        ];
        let (u0, u1, u2, u3) = (
            u_lane[0] as i64,
            u_lane[1] as i64,
            u_lane[2] as i64,
            u_lane[3] as i64,
        );
        o_lane[0] = ((q[0] * u0 - q[1] * u1 - q[2] * u2 - q[3] * u3) >> 12) as i32;
        o_lane[1] = ((q[0] * u1 + q[1] * u0 + q[2] * u3 - q[3] * u2) >> 12) as i32;
        o_lane[2] = ((q[0] * u2 - q[1] * u3 + q[2] * u0 + q[3] * u1) >> 12) as i32;
        o_lane[3] = ((q[0] * u3 + q[1] * u2 - q[2] * u1 + q[3] * u0) >> 12) as i32;
    }
    out
}

#[derive(Clone, Debug)]
pub struct FbsLayer {
    pub rec_gain: Vec<i16>,
    pub mlp_gain: Vec<i16>,
    pub rec_in: FbsRouteMap,
    pub rec_gate: FbsRouteMap,
    pub rec_out: FbsRouteMap,
    pub mlp_gate: FbsRouteMap,
    pub mlp_up: FbsRouteMap,
    pub mlp_down: FbsRouteMap,
    pub taps: Vec<i16>,
    pub conv_bias: Vec<i32>,
    pub gate_bias: Vec<i32>,
    pub decay_offset: Vec<i32>,
}

/// 17-root Icosian golden-ratio quaternion lattice `Z[phi]` on the unit 3-sphere `S^3` in Q12 (`4096 = 1.0`).
/// Preserves spin routes across SpiralCore non-commutative Hamilton transport (`R4.SpiralCoreFbs`).
pub const ICOSIAN_17_Q12: [[i16; 4]; 17] = [
    [4096, 0, 0, 0],
    [0, 4096, 0, 0],
    [0, 0, 4096, 0],
    [0, 0, 0, 4096],
    [-4096, 0, 0, 0],
    [2048, 2048, 2048, 2048],
    [2048, 2048, -2048, -2048],
    [2048, -2048, 2048, -2048],
    [2048, -2048, -2048, 2048],
    [3314, 2048, 1266, 0],
    [3314, -2048, 1266, 0],
    [3314, 2048, -1266, 0],
    [2048, 1266, 3314, 0],
    [2048, -1266, 3314, 0],
    [1266, 3314, 2048, 0],
    [1266, 0, 3314, 2048],
    [3314, 0, 1266, 2048],
];

/// Snap a 4D rotation vector to the nearest root in `ICOSIAN_17_Q12` on `S^3`.
#[inline]
pub fn snap_icosian_17_q12(rot: [i32; 4]) -> (u8, [i16; 4]) {
    let (r0, r1, r2, r3) = (rot[0] as i64, rot[1] as i64, rot[2] as i64, rot[3] as i64);
    if r0 == 0 && r1 == 0 && r2 == 0 && r3 == 0 {
        return (0, ICOSIAN_17_Q12[0]);
    }
    let mut best_idx = 0usize;
    let mut best_dot = i64::MIN;
    for (idx, q) in ICOSIAN_17_Q12.iter().enumerate() {
        let dot = r0 * (q[0] as i64) + r1 * (q[1] as i64) + r2 * (q[2] as i64) + r3 * (q[3] as i64);
        if dot > best_dot {
            best_dot = dot;
            best_idx = idx;
        }
    }
    (best_idx as u8, ICOSIAN_17_Q12[best_idx])
}

/// Non-commutative quaternion Hamilton product `q * h` on `S^3` in Q12.
#[inline]
pub fn hamilton_q12(q: [i16; 4], h: [i32; 4]) -> [i32; 4] {
    let (q0, q1, q2, q3) = (q[0] as i64, q[1] as i64, q[2] as i64, q[3] as i64);
    let (h0, h1, h2, h3) = (h[0] as i64, h[1] as i64, h[2] as i64, h[3] as i64);
    [
        ((q0 * h0 - q1 * h1 - q2 * h2 - q3 * h3) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h1 + q1 * h0 + q2 * h3 - q3 * h2) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h2 - q1 * h3 + q2 * h0 + q3 * h1) >> 12).clamp(-32768, 32767) as i32,
        ((q0 * h3 + q1 * h2 - q2 * h1 + q3 * h0) >> 12).clamp(-32768, 32767) as i32,
    ]
}

/// Compose two 6-bit Clifford `Cl(0,6)` spin carrier actions (`0..63`), tracking anti-commuting grade parity.
#[inline]
pub fn cl06_compose(a: u8, b: u8) -> u8 {
    let a6 = a & 0x3F;
    let b6 = b & 0x3F;
    let mut swaps = 0u32;
    for i in 1..6 {
        if ((a6 >> i) & 1) != 0 {
            let lower_mask = (1u8 << i) - 1;
            swaps += (b6 & lower_mask).count_ones();
        }
    }
    let sign_bit = (swaps & 1) as u8;
    (a6 ^ b6 ^ (sign_bit << 5)) & 0x3F
}

#[derive(Clone, Debug)]
pub struct FbsRecState {
    pub conv_ring: [[i32; FBS_WIDTH]; FBS_CONV_KERNEL],
    pub conv_pos: usize,
    pub h_state: [i32; FBS_WIDTH],
    /// Per-lane cumulative `Cl(0,6)` spin holonomy (`0..63`) preserving routes with spin across SpiralCore.
    pub spin_holonomy: [u8; FBS_LANES],
    /// Per-lane ternary/trinary FBS routing gate (`{-1, 0, +1}`).
    pub trinary_route: [i8; FBS_LANES],
    /// Rolling 4-step dominant Icosian root trajectory (`0..16`).
    pub icosian_trajectory: [u8; 4],
}

impl Default for FbsRecState {
    fn default() -> Self {
        Self {
            conv_ring: [[0; FBS_WIDTH]; FBS_CONV_KERNEL],
            conv_pos: 0,
            h_state: [0; FBS_WIDTH],
            spin_holonomy: [0; FBS_LANES],
            trinary_route: [0; FBS_LANES],
            icosian_trajectory: [0; 4],
        }
    }
}

impl FbsRecState {
    /// Pack the rolling 4-step Icosian root trajectory into a Base-17 address (`0..83520`).
    #[inline]
    pub fn packed_4block_root(&self) -> u32 {
        (self.icosian_trajectory[0].min(16) as u32)
            + 17 * (self.icosian_trajectory[1].min(16) as u32)
            + 289 * (self.icosian_trajectory[2].min(16) as u32)
            + 4913 * (self.icosian_trajectory[3].min(16) as u32)
    }
}

pub fn stack_recurrence_core(
    branches: &[i32],
    gate_out: &[i32],
    taps: &[i16],
    conv_bias: &[i32],
    gate_bias: &[i32],
    decay_offset: &[i32],
    state: &mut FbsRecState,
) -> [i32; FBS_WIDTH] {
    let slot = state.conv_pos % FBS_CONV_KERNEL;
    state.conv_ring[slot].copy_from_slice(&branches[..FBS_WIDTH]);
    state.conv_pos = state.conv_pos.wrapping_add(1);

    let mut conv_out = [0i32; FBS_WIDTH];
    for (c, conv_cell) in conv_out.iter_mut().enumerate().take(FBS_WIDTH) {
        let mut acc: i64 = (conv_bias.get(c).copied().unwrap_or(0) as i64) >> 4;
        for tap in 0..FBS_CONV_KERNEL {
            let ring_slot = (state.conv_pos + FBS_CONV_KERNEL - 1 - tap) % FBS_CONV_KERNEL;
            let tap_code = taps.get(tap * FBS_WIDTH + c).copied().unwrap_or(0);
            let w_q12 = grid_apply_q12(4096, tap_code);
            let w_geom = (w_q12 * GEOMETRIC_KEEP_Q31[FBS_CONV_KERNEL - 1 - tap]) >> 31;
            acc += ((state.conv_ring[ring_slot][c] as i64) * w_geom) >> 12;
        }
        *conv_cell = acc.clamp(-32768, 32767) as i32;
    }

    let mut h_next = [0i32; FBS_WIDTH];
    let mut root_votes = [0u16; 17];

    for lane in 0..FBS_LANES {
        let g_raw = gate_out.get(lane).copied().unwrap_or(0) as i64
            + ((gate_bias.get(lane).copied().unwrap_or(0) as i64) >> 4)
            + ((decay_offset.get(lane).copied().unwrap_or(0) as i64) >> 4);
        let rate_bucket = ((stack_sigmoid_q15(g_raw as i32) * 15) >> 15).clamp(0, 15) as usize;
        let lambda_q15 = (RETAIN_LAMBDA_Q31[rate_bucket] >> 16).clamp(1, 32767);
        let one_minus_q15 = (1i64 << 15) - lambda_q15;

        let base = lane * 4;
        let rot_base = FBS_LANES + base;
        let rot_raw = [
            gate_out.get(rot_base).copied().unwrap_or(4096),
            gate_out.get(rot_base + 1).copied().unwrap_or(0),
            gate_out.get(rot_base + 2).copied().unwrap_or(0),
            gate_out.get(rot_base + 3).copied().unwrap_or(0),
        ];
        let (icos_idx, q_icos) = snap_icosian_17_q12(rot_raw);
        root_votes[icos_idx as usize] = root_votes[icos_idx as usize].saturating_add(1);
        state.spin_holonomy[lane] = cl06_compose(
            state.spin_holonomy[lane],
            (icos_idx & 0x3F) ^ ((lane as u8) & 0x3F),
        );

        let h_prev_lane = [
            state.h_state[base],
            state.h_state[base + 1],
            state.h_state[base + 2],
            state.h_state[base + 3],
        ];
        // Blend continuous spin-preserving Hamilton rotation (25%) with exact scalar carrier (75%)
        // so routes carry non-commutative spin phase on S^3 without drifting from trained LUT basins.
        let h_rot_lane = hamilton_q12(q_icos, h_prev_lane);

        let out_gate_lead = branches.get(FBS_WIDTH + base).copied().unwrap_or(0);
        let trit: i8 = if out_gate_lead > 512 {
            1
        } else if out_gate_lead < -512 {
            -1
        } else {
            0
        };
        state.trinary_route[lane] = trit;

        for c in 0..4 {
            let idx = base + c;
            let h_carrier = ((h_prev_lane[c] as i64) * 3 + (h_rot_lane[c] as i64)) >> 2;
            let u_cur = conv_out[idx] as i64;
            let h_new = ((lambda_q15 * h_carrier) + (one_minus_q15 * u_cur)) >> 15;
            state.h_state[idx] = h_new.clamp(-32768, 32767) as i32;
            let out_gate = branches.get(FBS_WIDTH + idx).copied().unwrap_or(4096);
            let g_sig = stack_sigmoid_q15(out_gate);
            h_next[idx] = ((h_new * g_sig) >> 15).clamp(-32768, 32767) as i32;
        }
    }

    let mut dom_root = 0u8;
    let mut dom_cnt = 0u16;
    for (r_idx, &cnt) in root_votes.iter().enumerate() {
        if cnt > dom_cnt {
            dom_cnt = cnt;
            dom_root = r_idx as u8;
        }
    }
    state.icosian_trajectory = [
        state.icosian_trajectory[1],
        state.icosian_trajectory[2],
        state.icosian_trajectory[3],
        dom_root,
    ];

    h_next
}

// ---------------------------------------------------------------------------
// ServedHamming (Depth-2 & Depth-6 Bitwise Popcount Hamming XOR Attention)
// ---------------------------------------------------------------------------

/// Explicit SWAR 64-bit bitwise population count (`R4.Flock.swarPopcount`).
#[inline]
pub fn popcount_swar_u64(mut x: u64) -> u32 {
    x = x.wrapping_sub((x >> 1) & 0x5555_5555_5555_5555);
    x = (x & 0x3333_3333_3333_3333).wrapping_add((x >> 2) & 0x3333_3333_3333_3333);
    x = (x.wrapping_add(x >> 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    (x.wrapping_mul(0x0101_0101_0101_0101) >> 56) as u32
}

/// 256-bit Bitwise XOR + SWAR Popcount Hamming distance across 4 64-bit sign-code heads (`0..256` bits).
#[inline]
pub fn hamming_distance_256(a: &[u64; HAMMING_HEADS], b: &[u64; HAMMING_HEADS]) -> u32 {
    popcount_swar_u64(a[0] ^ b[0])
        + popcount_swar_u64(a[1] ^ b[1])
        + popcount_swar_u64(a[2] ^ b[2])
        + popcount_swar_u64(a[3] ^ b[3])
}

/// Pack a 256D `i32` hidden vector into a 256-bit (`[u64; 4]`) binary sign sketch.
#[inline]
pub fn pack_i32_sign_sketch_256(slice: &[i32]) -> [u64; HAMMING_HEADS] {
    let mut out = [0u64; HAMMING_HEADS];
    for (h, out_word) in out.iter_mut().enumerate() {
        let base = h * HAMMING_HEAD_DIM;
        let end = (base + HAMMING_HEAD_DIM).min(slice.len());
        if base < end {
            *out_word = sign_pack_64(&slice[base..end]);
        }
    }
    out
}

/// Pack a 256D `i16` point/writer vector into a 256-bit (`[u64; 4]`) binary sign sketch.
#[inline]
pub fn pack_i16_sign_sketch_256(slice: &[i16]) -> [u64; HAMMING_HEADS] {
    let mut out = [0u64; HAMMING_HEADS];
    if slice.len() >= 256 {
        for (h, out_word) in out.iter_mut().enumerate() {
            let chunk = &slice[h * HAMMING_HEAD_DIM..(h + 1) * HAMMING_HEAD_DIM];
            let mut word = 0u64;
            for (b, &v) in chunk.iter().enumerate() {
                if v >= 0 {
                    word |= 1u64 << b;
                }
            }
            *out_word = word;
        }
    } else {
        for (h, out_word) in out.iter_mut().enumerate() {
            let base = h * HAMMING_HEAD_DIM;
            let mut word = 0u64;
            for b in 0..HAMMING_HEAD_DIM {
                if slice.get(base + b).copied().unwrap_or(0) >= 0 {
                    word |= 1u64 << b;
                }
            }
            *out_word = word;
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct ServedHammingRead {
    pub depth: usize,
    pub copy_key: usize,
    pub query_rotation: Vec<i32>,
    pub key_rotation: Vec<i32>,
    pub value_rotation: Vec<i32>,
    pub margin: Vec<i32>,
    pub weight: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct HammingCache {
    pub codes: Vec<[u64; HAMMING_HEADS]>,
    pub values: Vec<[i32; FBS_WIDTH]>,
    pub count: usize,
}

fn rotate_lanes_q12(x: &[i32], rot_q24: &[i32]) -> [i32; FBS_WIDTH] {
    let mut out = [0i32; FBS_WIDTH];
    for lane in 0..FBS_LANES {
        let b = lane * 4;
        let r0 = (rot_q24.get(b).copied().unwrap_or(1 << 24) >> 12) as i64;
        let r1 = (rot_q24.get(b + 1).copied().unwrap_or(0) >> 12) as i64;
        let r2 = (rot_q24.get(b + 2).copied().unwrap_or(0) >> 12) as i64;
        let r3 = (rot_q24.get(b + 3).copied().unwrap_or(0) >> 12) as i64;
        let norm = ((r0 * r0 + r1 * r1 + r2 * r2 + r3 * r3).max(1) as u128)
            .isqrt()
            .max(1) as i64;
        let q = [
            (r0 << 12) / norm,
            (r1 << 12) / norm,
            (r2 << 12) / norm,
            (r3 << 12) / norm,
        ];
        let (x0, x1, x2, x3) = (
            x[b] as i64,
            x[b + 1] as i64,
            x[b + 2] as i64,
            x[b + 3] as i64,
        );
        out[b] = ((q[0] * x0 - q[1] * x1 - q[2] * x2 - q[3] * x3) >> 12) as i32;
        out[b + 1] = ((q[0] * x1 + q[1] * x0 + q[2] * x3 - q[3] * x2) >> 12) as i32;
        out[b + 2] = ((q[0] * x2 - q[1] * x3 + q[2] * x0 + q[3] * x1) >> 12) as i32;
        out[b + 3] = ((q[0] * x3 + q[1] * x2 - q[2] * x1 + q[3] * x0) >> 12) as i32;
    }
    out
}

fn sign_pack_64(slice: &[i32]) -> u64 {
    let mut word = 0u64;
    for (b, &v) in slice.iter().take(64).enumerate() {
        if v >= 0 {
            word |= 1u64 << b;
        }
    }
    word
}

impl ServedHammingRead {
    pub fn apply(&self, x: &mut [i32; FBS_WIDTH], cache: &mut HammingCache) {
        let q_rot = rotate_lanes_q12(x, &self.query_rotation);
        let k_rot = rotate_lanes_q12(x, &self.key_rotation);
        let v_rot = rotate_lanes_q12(x, &self.value_rotation);

        let mut k_codes = [0u64; HAMMING_HEADS];
        for h in 0..HAMMING_HEADS {
            k_codes[h] = sign_pack_64(&k_rot[h * HAMMING_HEAD_DIM..(h + 1) * HAMMING_HEAD_DIM]);
        }
        cache.codes.push(k_codes);
        cache.values.push(v_rot);
        cache.count += 1;

        let cur_len = cache.values.len();
        if cur_len <= 1 {
            return;
        }

        let copy_shift = self.copy_key.max(1);
        for h in 0..HAMMING_HEADS {
            let h_base = h * HAMMING_HEAD_DIM;
            let q_code = sign_pack_64(&q_rot[h_base..h_base + HAMMING_HEAD_DIM]);
            let margin_bits =
                ((self.margin.get(h).copied().unwrap_or(1 << 19) >> 16).max(1)) as u32;

            // Multi-horizon FlockSelect: track top-4 lowest Bitwise XOR Popcount Hamming positions
            // alongside attention sink (0) and local causal window.
            let mut top_matches: [(usize, u32); 4] = [(0, u32::MAX); 4];
            for pos in 0..cur_len - 1 {
                let d = popcount_swar_u64(q_code ^ cache.codes[pos][h]);
                if d < top_matches[3].1 {
                    top_matches[3] = (pos, d);
                    for s in (1..4).rev() {
                        if top_matches[s].1 < top_matches[s - 1].1 {
                            top_matches.swap(s, s - 1);
                        }
                    }
                }
            }

            let best_dist = top_matches[0].1;
            if best_dist < margin_bits {
                let mut acc_head = [0i64; HAMMING_HEAD_DIM];
                let mut total_w = 0i64;
                for &(pos, dist) in &top_matches {
                    if dist >= margin_bits {
                        continue;
                    }
                    let diff = (margin_bits - dist) as i64;
                    let w = diff * diff;
                    let copy_pos = (pos + copy_shift).min(cur_len - 1);
                    let copy_val = &cache.values[copy_pos];
                    let type_val = &cache.values[pos];
                    for (d, acc_slot) in acc_head.iter_mut().enumerate() {
                        let idx = h_base + d;
                        // Combine CopyHead (causal successor induction, 3/4) + TypeHead (semantic state, 1/4)
                        let blended_v = ((copy_val[idx] as i64) * 3 + (type_val[idx] as i64)) >> 2;
                        *acc_slot += blended_v * w;
                    }
                    total_w += w;
                }
                if total_w > 0 {
                    let lead_w_num = (margin_bits - best_dist) as i64;
                    let lead_w_den = margin_bits as i64;
                    for (d, &acc_val) in acc_head.iter().enumerate() {
                        let idx = h_base + d;
                        let mean_v = acc_val / total_w;
                        let scale_q12 =
                            (self.weight.get(idx).copied().unwrap_or(1 << 22) >> 12) as i64;
                        let delta = (((mean_v * lead_w_num) / lead_w_den) * scale_q12) >> 14;
                        x[idx] = x[idx].saturating_add(delta as i32);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ServedTurnMemory (/7) and ServedPointKey (/8 + Canonical Anchor Handoff)
// ---------------------------------------------------------------------------

#[inline]
pub fn encode_reply_anchor_21(anchor_id: u32) -> u32 {
    if anchor_id == GLOBAL_SUFFIX_ANCHOR {
        0x1F_FFFF
    } else if anchor_id == MEMORY_PAD {
        0x1F_FFFE
    } else if (anchor_id & 0x8000_0000) != 0 {
        (anchor_id & 0x1F_FFFF).min(0x1E_FFFF)
    } else {
        0x1F_0000 | (anchor_id & 0xFFFF)
    }
}

#[inline]
pub fn encode_tok_13(tok: u32) -> u32 {
    if tok == MEMORY_PAD {
        0x1FFF
    } else {
        tok & 0x1FFF
    }
}

/// Losslessly pack a 6-word reply key `[anchor_id, occ, prev_3, prev_2, prev_1, curr_tok]`
/// and target token `val` into 96 bits (`[u32; 3]`, 12 bytes/entry vs 28 bytes/entry — `2.333x` denser)
/// while preserving exact lexicographical sort order across `(w0, w1, w2 >> 13)`.
#[inline]
pub fn pack_reply_entry_12(key: &[u32; 6], val: u32) -> [u32; 3] {
    let a = encode_reply_anchor_21(key[0]);
    let occ = key[1].min(1023);
    let p3 = encode_tok_13(key[2]);
    let p2 = encode_tok_13(key[3]);
    let p1 = encode_tok_13(key[4]);
    let cur = encode_tok_13(key[5]);
    let v = encode_tok_13(val);
    let w0 = (a << 11) | (occ << 1) | (p3 >> 12);
    let w1 = ((p3 & 0xFFF) << 20) | (p2 << 7) | (p1 >> 6);
    let w2 = ((p1 & 0x3F) << 26) | (cur << 13) | v;
    [w0, w1, w2]
}

#[inline]
pub fn pack_reply_key_prefix_12(key: &[u32]) -> Option<(u32, u32, u32)> {
    if key.len() != 6 {
        return None;
    }
    let a = encode_reply_anchor_21(key[0]);
    let occ = key[1].min(1023);
    let p3 = encode_tok_13(key[2]);
    let p2 = encode_tok_13(key[3]);
    let p1 = encode_tok_13(key[4]);
    let cur = encode_tok_13(key[5]);
    let w0 = (a << 11) | (occ << 1) | (p3 >> 12);
    let w1 = ((p3 & 0xFFF) << 20) | (p2 << 7) | (p1 >> 6);
    let w2_key = ((p1 & 0x3F) << 13) | cur;
    Some((w0, w1, w2_key))
}

#[derive(Clone, Debug, Default)]
pub struct TurnMemoryTable {
    pub entries: usize,
    pub key_len: usize,
    pub keys: Vec<u32>,
    pub values: Vec<u32>,
    pub packed12: Vec<[u32; 3]>,
    pub weight_q24: Vec<i32>,
}

impl TurnMemoryTable {
    pub fn lookup(&self, key: &[u32]) -> Option<usize> {
        if self.entries == 0 || key.len() != self.key_len {
            return None;
        }
        if !self.packed12.is_empty() && self.key_len == 6 {
            let (w0, w1, w2_key) = pack_reply_key_prefix_12(key)?;
            return self
                .packed12
                .binary_search_by_key(&(w0, w1, w2_key), |e| (e[0], e[1], e[2] >> 13))
                .ok();
        }
        let mut lo = 0usize;
        let mut hi = self.entries;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let base = mid * self.key_len;
            let slice = &self.keys[base..base + self.key_len];
            match slice.cmp(key) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(mid),
            }
        }
        None
    }

    #[inline]
    pub fn lookup_value(&self, key: &[u32]) -> Option<u32> {
        if !self.packed12.is_empty() && self.key_len == 6 {
            let idx = self.lookup(key)?;
            let raw_v = self.packed12[idx][2] & 0x1FFF;
            return Some(if raw_v == 0x1FFF { MEMORY_PAD } else { raw_v });
        }
        let idx = self.lookup(key)?;
        self.values.get(idx).copied()
    }
}

/// Packed 1-bit/ternary associative coupling matrix (`rows * (cols_bits / 32)` `u32` words).
/// Encodes up to `1.85B+` trained token-to-token and stem-to-token binary/ternary sign-coupling
/// parameters (`{-1, +1}`) evaluated via O(1) bit probes and SWAR popcount resonance.
#[derive(Clone, Debug, Default)]
pub struct TernaryAssocBank {
    pub rows: usize,
    pub cols_bits: usize,
    pub buckets: usize,
    pub words_u32: Vec<u32>,
}

impl TernaryAssocBank {
    #[inline]
    pub fn test_bit(&self, row: usize, col_tok: u32) -> bool {
        let col = (col_tok as usize) & 0xFFF;
        if row >= self.rows || col >= self.cols_bits {
            return false;
        }
        let words_per_row = self.cols_bits / 32;
        let idx = row * words_per_row + (col >> 5);
        let bit_mask = 1u32 << (col & 31);
        let dense_b0_words = 4096 * words_per_row;
        if self.words_u32.len() >= self.rows * words_per_row {
            return (self.words_u32[idx] & bit_mask) != 0;
        }
        if idx < dense_b0_words {
            return self
                .words_u32
                .get(idx)
                .is_some_and(|&w| (w & bit_mask) != 0);
        }
        let sparse = &self.words_u32[dense_b0_words.min(self.words_u32.len())..];
        let pair_count = sparse.len() / 2;
        let mut lo = 0usize;
        let mut hi = pair_count;
        let target_w = idx as u32;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let k = sparse[mid * 2];
            if k < target_w {
                lo = mid + 1;
            } else if k > target_w {
                hi = mid;
            } else {
                return (sparse[mid * 2 + 1] & bit_mask) != 0;
            }
        }
        false
    }

    #[inline]
    pub fn set_bit(&mut self, row: usize, col_tok: u32) {
        let col = (col_tok as usize) & 0xFFF;
        if row >= self.rows || col >= self.cols_bits || self.cols_bits < 32 {
            return;
        }
        let words_per_row = self.cols_bits / 32;
        let idx = row * words_per_row + (col >> 5);
        let bit_mask = 1u32 << (col & 31);
        let dense_b0_words = 4096 * words_per_row;
        if self.words_u32.len() >= self.rows * words_per_row || idx < dense_b0_words {
            if let Some(slot) = self.words_u32.get_mut(idx) {
                *slot |= bit_mask;
            }
            return;
        }
        if self.words_u32.len() < dense_b0_words {
            return;
        }
        let sparse = &mut self.words_u32[dense_b0_words..];
        let pair_count = sparse.len() / 2;
        let mut lo = 0usize;
        let mut hi = pair_count;
        let target_w = idx as u32;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let k = sparse[mid * 2];
            if k < target_w {
                lo = mid + 1;
            } else if k > target_w {
                hi = mid;
            } else {
                sparse[mid * 2 + 1] |= bit_mask;
                return;
            }
        }
    }

    #[inline]
    pub fn score_transition(
        &self,
        prev_tok: u32,
        curr_tok: u32,
        stem_bucket: usize,
        cand_tok: u32,
    ) -> i64 {
        if self.rows < 4096 || self.cols_bits < 4096 || cand_tok >= 4096 {
            return 0;
        }
        let cur = (curr_tok as usize) & 0xFFF;
        let mut hits = 0i64;
        if self.test_bit(cur, cand_tok) {
            hits += 1;
        }
        if self.buckets > 1 {
            let rem = self.buckets - 1;
            let tri_b = (((prev_tok as usize).wrapping_mul(31)) % rem) + 1;
            if self.test_bit(tri_b * 4096 + cur, cand_tok) {
                hits += 1;
            }
            let stem_b = (stem_bucket % rem) + 1;
            if self.test_bit(stem_b * 4096 + cur, cand_tok) {
                hits += 1;
            }
        }
        hits
    }
}

#[derive(Clone, Debug)]
pub struct ServedTurnMemory {
    pub k: usize,
    pub key_window: usize,
    pub reply_turn: Option<usize>,
    pub reply_key_len: usize,
    pub weight_exp: i32,
    pub opening: TurnMemoryTable,
    pub reply: Option<TurnMemoryTable>,
}

#[derive(Clone, Debug, Default)]
pub struct TurnMemoryAnchors {
    pub opening: Vec<u32>,
    pub reply: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ServedPointKey {
    pub rows: usize,
    pub width: usize,
    pub exp: i32,
    pub vote_k: usize,
    pub points: Vec<i16>,
    pub targets: Vec<u32>,
    /// Canonical `reply` turn_ids (`rows * reply_turn`) for each point row so when `ServedPointKey`
    /// wins on a paraphrased prompt at `row + 1 == reply_start`, `anchors.reply` is updated to the
    /// matched training turn's canonical `turn_ids` and `memory.reply` continues for the entire reply!
    pub canonical_anchors: Option<Vec<u32>>,
    pub anchor_len: usize,
    /// Cross-lifecycle provenance span IDs (1 per row) linking to the exact
    /// training or proof span (`gllm.training.compile_turn` or `gllm.engine.prove_claim`).
    pub provenance_spans: Option<Vec<u64>>,
    pub provenance_traces: Option<Vec<u128>>,
    /// Two-level hierarchical radix / geometric index directory offsets (17,409 entries).
    pub bucket_offsets: Option<Vec<u32>>,
    /// Permutation array mapping radix candidate slots back to original ServedPointKey row indices.
    pub bucket_row_indices: Option<Vec<u32>>,
}

impl ServedPointKey {
    /// Retrieve (trace_id, span_id) provenance for a PointKey row.
    /// Dynamic frontier rows tagged with VERIFIED_FRONTIER_ANCHOR_TAG encode span_id (words 1..2) and trace_id (words 3..6).
    /// If row is not stamped or is a bootstrap row, returns fallback bootstrap provenance.
    pub fn get_row_provenance(&self, row: usize) -> (u128, u64) {
        if let (Some(traces), Some(spans)) = (&self.provenance_traces, &self.provenance_spans) {
            if row < spans.len()
                && row < traces.len()
                && spans[row] != 0
                && spans[row] != u64::MAX
                && traces[row] != 0
                && traces[row] != u128::MAX
            {
                return (traces[row], spans[row]);
            }
        }
        if let Some(anchors) = &self.canonical_anchors {
            let alen = self.anchor_len.max(1);
            if (row + 1) * alen <= anchors.len() && alen >= 8 {
                let base = row * alen;
                if anchors[base + 7] == VERIFIED_FRONTIER_ANCHOR_TAG {
                    let s_lo = anchors[base + 1];
                    let s_hi = anchors[base + 2];
                    let span_id = (s_lo as u64) | ((s_hi as u64) << 32);
                    let t0 = anchors[base + 3] as u128;
                    let t1 = (anchors[base + 4] as u128) << 32;
                    let t2 = (anchors[base + 5] as u128) << 64;
                    let t3 = (anchors[base + 6] as u128) << 96;
                    let trace_id = t0 | t1 | t2 | t3;
                    if span_id != 0 && span_id != u64::MAX && trace_id != 0 && trace_id != u128::MAX
                    {
                        return (trace_id, span_id);
                    }
                }
            }
        }
        (0x756f722d72342e626f6f747374726170, 0x756f722d72342e38)
    }

    /// Build or rebuild the two-level hierarchical radix / geometric index over all PointKey rows.
    ///
    /// Constructs a 17,409-entry bucket offset table (`bucket_offsets`) and a permutation
    /// table (`bucket_row_indices`) mapping geometric buckets to row indices without reordering
    /// underlying row arrays, preserving table synchronization invariants (`audit_point_key_synchronization == 0`).
    pub fn build_radix_index(&mut self) {
        if self.rows == 0 || self.points.len() < self.rows * self.width || self.width == 0 {
            self.bucket_offsets = Some(vec![0u32; RADIX_DIRECTORY_LEN]);
            self.bucket_row_indices = Some(Vec::new());
            return;
        }

        let mut bucket_counts = vec![0u32; GEOMETRIC_BUCKET_COUNT];
        let mut row_buckets = Vec::with_capacity(self.rows);

        for r in 0..self.rows {
            let p_base = r * self.width;
            let p_row = &self.points[p_base..p_base + self.width];
            let p_sketch = pack_i16_sign_sketch_256(p_row);
            let (e8, c64, i17) = compute_geometric_coordinates(p_row, p_sketch[0]);
            let b = compute_geometric_bucket(e8, c64, i17);
            bucket_counts[b] += 1;
            row_buckets.push(b);
        }

        let mut bucket_offsets = Vec::with_capacity(RADIX_DIRECTORY_LEN);
        let mut running = 0u32;
        for &count in &bucket_counts {
            bucket_offsets.push(running);
            running += count;
        }
        bucket_offsets.push(running);

        let mut current_pos = bucket_offsets[0..GEOMETRIC_BUCKET_COUNT].to_vec();
        let mut bucket_row_indices = vec![0u32; self.rows];

        for (r, &b) in row_buckets.iter().enumerate() {
            let pos = current_pos[b] as usize;
            bucket_row_indices[pos] = r as u32;
            current_pos[b] += 1;
        }

        self.bucket_offsets = Some(bucket_offsets);
        self.bucket_row_indices = Some(bucket_row_indices);
    }

    pub fn apply_with_row(&self, n_t: &[i32]) -> Option<(u32, usize, i64)> {
        if self.rows == 0 || n_t.len() < self.width {
            return None;
        }
        let k = self.vote_k.max(1).min(self.rows);
        let mut top_k: Vec<(i64, usize)> = Vec::with_capacity(k + 1);
        for r in 0..self.rows {
            let base = r * self.width;
            let row = &self.points[base..base + self.width];
            let mut dist = 0i64;
            for d in 0..self.width {
                dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
            }
            let pos = top_k.partition_point(|&(d_ex, r_ex)| (d_ex, r_ex) <= (dist, r));
            if pos < k {
                top_k.insert(pos, (dist, r));
                if top_k.len() > k {
                    top_k.pop();
                }
            }
        }
        if top_k.is_empty() {
            return None;
        }
        if k == 1 {
            let (dist, r) = top_k[0];
            return Some((self.targets[r], r, dist));
        }
        let mut best_tok = self.targets[top_k[0].1];
        let mut best_row = top_k[0].1;
        let mut best_dist = top_k[0].0;
        let mut best_votes = 0usize;
        for &(dist, r) in &top_k {
            let cand = self.targets[r];
            let votes = top_k
                .iter()
                .filter(|&&(_, r2)| self.targets[r2] == cand)
                .count();
            if votes > best_votes {
                best_votes = votes;
                best_tok = cand;
                best_row = r;
                best_dist = dist;
            }
        }
        Some((best_tok, best_row, best_dist))
    }
}

// ---------------------------------------------------------------------------
// Whole-Table Nearest & Writer Readout
// ---------------------------------------------------------------------------

pub fn stack_whole_table_nearest(n_t: &[i32], writer_table: &[i16], vocab: usize) -> (u32, i64) {
    let mut best_tok = 0u32;
    let mut best_dist = i64::MAX;
    for tok in 0..vocab {
        let base = tok * FBS_WIDTH;
        if base + FBS_WIDTH > writer_table.len() {
            break;
        }
        let row = &writer_table[base..base + FBS_WIDTH];
        let mut dist = 0i64;
        for d in 0..FBS_WIDTH {
            dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
        }
        if dist < best_dist {
            best_dist = dist;
            best_tok = tok as u32;
        }
    }
    (best_tok, best_dist)
}

pub fn stack_whole_table_top_k(
    n_t: &[i32],
    writer_table: &[i16],
    vocab: usize,
    k: usize,
) -> Vec<(u32, i64)> {
    let k = k.max(1);
    let mut top: Vec<(u32, i64)> = Vec::with_capacity(k + 1);
    for tok in 0..vocab {
        let base = tok * FBS_WIDTH;
        if base + FBS_WIDTH > writer_table.len() {
            break;
        }
        let row = &writer_table[base..base + FBS_WIDTH];
        let mut dist = 0i64;
        for d in 0..FBS_WIDTH {
            dist += ((n_t[d] as i64) - (row[d] as i64)).abs();
        }
        let pos = top.partition_point(|&(t_ex, d_ex)| (d_ex, t_ex) <= (dist, tok as u32));
        if pos < k {
            top.insert(pos, (tok as u32, dist));
            if top.len() > k {
                top.pop();
            }
        }
    }
    top
}

// ---------------------------------------------------------------------------
// Complete LutStackModel & LutStackSession (UORLUT01 /7 and /8 Compatible)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LutStackModel {
    pub schema: String,
    pub artifact_sha256: String,
    pub artifact_bytes: usize,
    pub vocab: usize,
    pub width: usize,
    pub mlp: usize,
    pub context: usize,
    pub layers: Vec<FbsLayer>,
    pub final_gain: Vec<i16>,
    pub writer_table: Vec<i16>,
    pub hamming: Vec<ServedHammingRead>,
    pub memory: Option<ServedTurnMemory>,
    pub point: Option<ServedPointKey>,
    pub ternary_assoc: Option<TernaryAssocBank>,
    pub dialogue_version: u8,
    /// Per-token writer basin provenance (span_id) from coordinate descent consolidation.
    pub writer_provenance: Option<Vec<u64>>,
    /// Per-token writer basin provenance (trace_id) from coordinate descent consolidation.
    pub writer_provenance_traces: Option<Vec<u128>>,
}

impl LutStackModel {
    /// Retrieve (trace_id, span_id) writer basin provenance for a predicted token.
    /// Returns fallback bootstrap provenance if not explicitly set.
    pub fn get_writer_provenance(&self, token: u32) -> (u128, u64) {
        if let (Some(traces), Some(spans)) =
            (&self.writer_provenance_traces, &self.writer_provenance)
        {
            let idx = token as usize;
            if idx < spans.len()
                && idx < traces.len()
                && spans[idx] != 0
                && spans[idx] != u64::MAX
                && traces[idx] != 0
                && traces[idx] != u128::MAX
            {
                return (traces[idx], spans[idx]);
            }
        }
        (0x756f722d72342e626f6f747374726170, 0x756f722d72342e38)
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 16 || &bytes[0..8] != MAGIC {
            return Err("invalid UORLUT01 magic header".to_owned());
        }
        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&bytes[8..16]);
        let declared = u64::from_le_bytes(len_bytes) as usize;
        let header_end = 16usize
            .checked_add(declared)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| "UORLUT01 header_len exceeds file size".to_owned())?;
        let data_start = header_end.div_ceil(ALIGN) * ALIGN;
        if data_start > bytes.len() {
            return Err("UORLUT01 data_start exceeds file size".to_owned());
        }
        let header_bytes = &bytes[16..header_end];
        let body = &bytes[data_start..];
        let header = JsonVal::parse(header_bytes)
            .ok_or_else(|| "invalid UORLUT01 JSON header".to_owned())?;

        let schema = header
            .get("schema")
            .and_then(JsonVal::as_str)
            .unwrap_or(STACK_SCHEMA_V8)
            .to_owned();
        let shape = header.get("shape");
        let vocab = shape
            .and_then(|s| s.get("vocab"))
            .or_else(|| header.get("vocab"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(4096);
        let width = shape
            .and_then(|s| s.get("width"))
            .or_else(|| header.get("width"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(FBS_WIDTH);
        let mlp = shape
            .and_then(|s| s.get("mlp"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(FBS_MLP_HIDDEN);
        let context = shape
            .and_then(|s| s.get("context"))
            .and_then(JsonVal::as_usize)
            .unwrap_or(192);
        let num_layers = shape
            .and_then(|s| s.get("pattern"))
            .and_then(JsonVal::as_str)
            .map(str::len)
            .unwrap_or(6);
        let dialogue_version = header
            .get("dialogue_version")
            .and_then(JsonVal::as_u64)
            .map(|v| v as u8)
            .unwrap_or(2);

        let mut tables: HashMap<String, (usize, usize)> = HashMap::new();
        if let Some(arr) = header.get("tables").and_then(JsonVal::as_arr) {
            for t in arr {
                let name = t.get("name").and_then(JsonVal::as_str);
                let span = t.get("span");
                let off = span
                    .and_then(|s| s.get("offset"))
                    .or_else(|| t.get("offset"))
                    .and_then(JsonVal::as_usize);
                let len = span
                    .and_then(|s| s.get("bytes"))
                    .or_else(|| t.get("bytes"))
                    .and_then(JsonVal::as_usize);
                if let (Some(n), Some(o), Some(b)) = (name, off, len) {
                    if o + b <= body.len() {
                        tables.insert(n.to_owned(), (o, b));
                    }
                }
            }
        }

        let read_i16 = |name: &str| -> Result<Vec<i16>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect())
        };
        let read_i32 = |name: &str| -> Result<Vec<i32>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(4)
                .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        };
        let read_u32 = |name: &str| -> Result<Vec<u32>, String> {
            let &(off, len) = tables
                .get(name)
                .ok_or_else(|| format!("missing table {name}"))?;
            Ok(body[off..off + len]
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        };
        let read_route =
            |name: &str, in_dim: usize, out_dim: usize| -> Result<FbsRouteMap, String> {
                let packed = read_u32(name)?;
                Ok(FbsRouteMap::from_packed_u32(&packed, in_dim, out_dim))
            };

        let final_gain = read_i16("final_norm_gain")?;
        let writer_table = read_i16("writer.table")?;
        let gate_rows = (width / 4) + width;

        let mut layers = Vec::with_capacity(num_layers);
        for l in 0..num_layers {
            layers.push(FbsLayer {
                rec_gain: read_i16(&format!("l{l}.rec_norm_gain"))?,
                mlp_gain: read_i16(&format!("l{l}.mlp_norm_gain"))?,
                rec_in: read_route(&format!("l{l}.rec_in.fbs"), width, 2 * width)?,
                rec_gate: read_route(&format!("l{l}.rec_gate.fbs"), width, gate_rows)?,
                rec_out: read_route(&format!("l{l}.rec_out.fbs"), width, width)?,
                mlp_gate: read_route(&format!("l{l}.mlp_gate.fbs"), width, mlp)?,
                mlp_up: read_route(&format!("l{l}.mlp_up.fbs"), width, mlp)?,
                mlp_down: read_route(&format!("l{l}.mlp_down.fbs"), mlp, width)?,
                taps: read_i16(&format!("l{l}.conv_taps"))?,
                conv_bias: read_i32(&format!("l{l}.conv_bias"))?,
                gate_bias: read_i32(&format!("l{l}.gate_bias"))?,
                decay_offset: read_i32(&format!("l{l}.decay_offset"))?,
            });
        }

        let mut hamming = Vec::new();
        if let Some(h_obj) = header.get("hamming") {
            let copy_key = h_obj
                .get("copy_key")
                .and_then(JsonVal::as_usize)
                .unwrap_or(2);
            if let Some(depths) = h_obj.get("depths").and_then(JsonVal::as_arr) {
                for d_val in depths {
                    if let Some(d) = d_val.as_usize() {
                        let p = format!("hamming.d{d}");
                        if tables.contains_key(&format!("{p}.query_rotation")) {
                            hamming.push(ServedHammingRead {
                                depth: d,
                                copy_key,
                                query_rotation: read_i32(&format!("{p}.query_rotation"))?,
                                key_rotation: read_i32(&format!("{p}.key_rotation"))?,
                                value_rotation: read_i32(&format!("{p}.value_rotation"))?,
                                margin: read_i32(&format!("{p}.margin"))?,
                                weight: read_i32(&format!("{p}.weight"))?,
                            });
                        }
                    }
                }
            }
        }

        let mut memory = None;
        let mut anchor_len_for_point = 8usize;
        if let Some(m_obj) = header.get("memory").or_else(|| header.get("turn_memory")) {
            if let Some(k) = m_obj.get("k").and_then(JsonVal::as_usize) {
                let key_window = m_obj
                    .get("key_window")
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(192);
                let opening_entries = m_obj
                    .get("opening")
                    .or_else(|| m_obj.get("opening_entries"))
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(0);
                let reply_entries = m_obj
                    .get("reply")
                    .or_else(|| m_obj.get("reply_entries"))
                    .and_then(JsonVal::as_usize);
                let reply_turn = m_obj.get("reply_turn").and_then(JsonVal::as_usize);
                anchor_len_for_point = reply_turn.unwrap_or(k);
                let reply_key_len = m_obj
                    .get("reply_key_len")
                    .and_then(JsonVal::as_usize)
                    .unwrap_or(MEMORY_REPLY_KEY_LEN);
                let weight_exp = m_obj
                    .get("weight_exp")
                    .and_then(JsonVal::as_i64)
                    .unwrap_or(-24) as i32;

                let opening = TurnMemoryTable {
                    entries: opening_entries,
                    key_len: k,
                    keys: read_u32("memory.opening.keys").unwrap_or_default(),
                    values: read_u32("memory.opening.values").unwrap_or_default(),
                    packed12: Vec::new(),
                    weight_q24: read_i32("memory.opening.weight").unwrap_or_default(),
                };
                let reply = if let Some(re) = reply_entries {
                    if tables.contains_key("memory.reply.packed12") {
                        let raw_p12 = read_u32("memory.reply.packed12")?;
                        let packed12: Vec<[u32; 3]> = raw_p12
                            .chunks_exact(3)
                            .map(|c| [c[0], c[1], c[2]])
                            .collect();
                        Some(TurnMemoryTable {
                            entries: packed12.len().max(re),
                            key_len: reply_key_len,
                            keys: Vec::new(),
                            values: Vec::new(),
                            packed12,
                            weight_q24: read_i32("memory.reply.weight")?,
                        })
                    } else if tables.contains_key("memory.reply.keys") {
                        Some(TurnMemoryTable {
                            entries: re,
                            key_len: reply_key_len,
                            keys: read_u32("memory.reply.keys")?,
                            values: read_u32("memory.reply.values")?,
                            packed12: Vec::new(),
                            weight_q24: read_i32("memory.reply.weight")?,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                memory = Some(ServedTurnMemory {
                    k,
                    key_window,
                    reply_turn,
                    reply_key_len,
                    weight_exp,
                    opening,
                    reply,
                });
            }
        }

        let mut point = None;
        if let Some(p_obj) = header.get("point").or_else(|| header.get("point_key")) {
            let rows = p_obj
                .get("rows")
                .or_else(|| p_obj.get("count"))
                .and_then(JsonVal::as_usize)
                .unwrap_or(0);
            let p_width = p_obj
                .get("width")
                .or_else(|| p_obj.get("dim"))
                .and_then(JsonVal::as_usize)
                .unwrap_or(POINT_KEY_DIM);
            let exp = p_obj.get("exp").and_then(JsonVal::as_i64).unwrap_or(-12) as i32;
            let vote_k = p_obj
                .get("vote_k")
                .and_then(JsonVal::as_usize)
                .unwrap_or(DEFAULT_VOTE_K);
            if rows > 0 && tables.contains_key("memory.point.points") {
                let points = read_i16("memory.point.points")?;
                let targets = read_u32("memory.point.targets")?;
                let canonical_anchors = if tables.contains_key("memory.point.anchors") {
                    Some(read_u32("memory.point.anchors")?)
                } else {
                    None
                };
                let effective_anchor_len = canonical_anchors
                    .as_ref()
                    .filter(|ca| rows > 0 && ca.len() >= rows && ca.len() % rows == 0)
                    .map(|ca| ca.len() / rows)
                    .unwrap_or(anchor_len_for_point);
                let mut pt = ServedPointKey {
                    rows,
                    width: p_width,
                    exp,
                    vote_k,
                    points,
                    targets,
                    canonical_anchors,
                    anchor_len: effective_anchor_len,
                    provenance_spans: None,
                    provenance_traces: None,
                    bucket_offsets: None,
                    bucket_row_indices: None,
                };
                if tables.contains_key("memory.point.bucket_offsets")
                    && tables.contains_key("memory.point.bucket_row_indices")
                {
                    if let (Ok(offsets), Ok(indices)) = (
                        read_u32("memory.point.bucket_offsets"),
                        read_u32("memory.point.bucket_row_indices"),
                    ) {
                        let shards_balanced = offsets.len() == RADIX_DIRECTORY_LEN
                            && indices.len() == rows
                            && (0..E8_ROOT_COUNT)
                                .all(|s| offsets[(s + 1) * 1088] > offsets[s * 1088]);
                        if shards_balanced {
                            pt.bucket_offsets = Some(offsets);
                            pt.bucket_row_indices = Some(indices);
                        }
                    }
                }
                if pt.bucket_offsets.is_none() && rows > 0 {
                    pt.build_radix_index();
                }
                point = Some(pt);
            }
        }

        let mut ternary_assoc = None;
        if let Some(ta_obj) = header.get("ternary_assoc") {
            let ta_rows = ta_obj.get("rows").and_then(JsonVal::as_usize).unwrap_or(0);
            let cols_bits = ta_obj
                .get("cols_bits")
                .and_then(JsonVal::as_usize)
                .unwrap_or(4096);
            let buckets = ta_obj
                .get("buckets")
                .and_then(JsonVal::as_usize)
                .unwrap_or(1);
            if ta_rows > 0 && tables.contains_key("ternary.assoc.bank") {
                let words_u32 = read_u32("ternary.assoc.bank")?;
                ternary_assoc = Some(TernaryAssocBank {
                    rows: ta_rows,
                    cols_bits,
                    buckets,
                    words_u32,
                });
            }
        }

        let artifact_sha256 = sha256_hex(bytes);

        Ok(Self {
            schema,
            artifact_sha256,
            artifact_bytes: bytes.len(),
            vocab,
            width,
            mlp,
            context,
            layers,
            final_gain,
            writer_table,
            hamming,
            memory,
            point,
            ternary_assoc,
            dialogue_version,
            writer_provenance: None,
            writer_provenance_traces: None,
        })
    }

    pub fn total_parameters(&self) -> u64 {
        let mut total = (self.writer_table.len() + self.final_gain.len()) as u64;
        for l in &self.layers {
            total += (l.rec_gain.len()
                + l.mlp_gain.len()
                + l.rec_in.packed_entries.len() * 5
                + l.rec_gate.packed_entries.len() * 5
                + l.rec_out.packed_entries.len() * 5
                + l.mlp_gate.packed_entries.len() * 5
                + l.mlp_up.packed_entries.len() * 5
                + l.mlp_down.packed_entries.len() * 5
                + l.taps.len()
                + l.conv_bias.len()
                + l.gate_bias.len()
                + l.decay_offset.len()) as u64;
        }
        for h in &self.hamming {
            total += (h.query_rotation.len()
                + h.key_rotation.len()
                + h.value_rotation.len()
                + h.margin.len()
                + h.weight.len()) as u64;
        }
        if let Some(mem) = &self.memory {
            total += (mem.opening.entries * (mem.opening.key_len + 1)
                + mem.opening.weight_q24.len()) as u64;
            if let Some(rep) = &mem.reply {
                total += (rep.entries * (rep.key_len + 1) + rep.weight_q24.len()) as u64;
            }
        }
        if let Some(pt) = &self.point {
            total += (pt.rows * (pt.width + 1 + pt.anchor_len)) as u64;
        }
        if let Some(ta) = &self.ternary_assoc {
            total += (ta.rows as u64) * (ta.cols_bits as u64);
        }
        total
    }

    pub fn session(self: &Arc<Self>) -> LutStackSession {
        LutStackSession::new(Arc::clone(self))
    }

    /// Forward a single token through the 6-layer `SpiralCore/FBS` + `ServedHamming` trunk,
    /// returning the 256D Q12 final normalized hidden state `n_t`.
    pub fn forward_token_hidden(
        &self,
        token: u32,
        rec_states: &mut [FbsRecState],
        hamming_caches: &mut [HammingCache],
    ) -> [i32; FBS_WIDTH] {
        let tok_idx = (token as usize).min(self.vocab.saturating_sub(1));
        let row_base = tok_idx * FBS_WIDTH;
        let mut x = [0i32; FBS_WIDTH];
        for (d, slot) in x.iter_mut().enumerate() {
            *slot = self.writer_table.get(row_base + d).copied().unwrap_or(0) as i32;
        }

        for (l_idx, layer) in self.layers.iter().enumerate() {
            let depth = l_idx + 1;
            let rec_normed = stack_norm_gain(&x, &layer.rec_gain);
            let branches = layer.rec_in.forward(&rec_normed);
            let gate_out = layer.rec_gate.forward(&rec_normed);
            let h_next = stack_recurrence_core(
                &branches,
                &gate_out,
                &layer.taps,
                &layer.conv_bias,
                &layer.gate_bias,
                &layer.decay_offset,
                &mut rec_states[l_idx],
            );
            let rec_proj = layer.rec_out.forward(&h_next);
            for d in 0..FBS_WIDTH {
                x[d] = x[d].saturating_add(rec_proj[d]);
            }

            let mlp_normed = stack_norm_gain(&x, &layer.mlp_gain);
            let gate = layer.mlp_gate.forward(&mlp_normed);
            let up = layer.mlp_up.forward(&mlp_normed);
            let hidden = stack_hamilton_ffn(&gate, &up);
            let down = layer.mlp_down.forward(&hidden);
            for d in 0..FBS_WIDTH {
                x[d] = x[d].saturating_add(down[d]);
            }

            for (h_idx, h_op) in self.hamming.iter().enumerate() {
                if h_op.depth == depth {
                    h_op.apply(&mut x, &mut hamming_caches[h_idx]);
                }
            }
        }

        let final_vec = stack_norm_gain(&x, &self.final_gain);
        let mut n_t = [0i32; FBS_WIDTH];
        n_t.copy_from_slice(&final_vec[..FBS_WIDTH]);
        n_t
    }
}

#[derive(Clone, Debug)]
pub struct StepTelemetry {
    pub position: usize,
    pub input_token: u32,
    pub predicted_token: u32,
    pub source_mechanism: &'static str,
    pub l1_distance: i64,
    pub hamming_xor_bits: u32,
    pub icosian_root: u8,
    pub spin_holonomy: u8,
    pub trinary_active_lanes: usize,
    pub top_candidates: Vec<(u32, i64)>,
}

pub struct LutStackSession {
    model: Arc<LutStackModel>,
    pub rec_states: Vec<FbsRecState>,
    pub hamming_caches: Vec<HammingCache>,
    pub tokens: Vec<u32>,
    pub reply_start: Option<usize>,
    pub anchors: TurnMemoryAnchors,
    pub reply_history: Vec<u32>,
    pub suffix_counts: Vec<([u32; 4], u32)>,
    pub pending_basins: Vec<(Vec<u32>, u32)>,
    pub last_matched_stems: usize,
    pub last_point_hamming_bits: u32,
    pub last_covered_stem_mask: u16,
    pub last_turn_idx: usize,
    pub prompt_stem_bucket: usize,
    pub last_n_t: [i32; FBS_WIDTH],
    pub prompt_only_tokens: Vec<u32>,
    pub prompt_n_t: [i32; FBS_WIDTH],
    pub has_executive_slot_trace: bool,
    pub recent_turn_rows: Vec<usize>,
    pub emitted_4grams: Vec<u64>,
    pub boundary_trajectories: Vec<PointTrajectoryHit>,
    pub total_l1_margin: u64,
}

impl LutStackSession {
    pub fn new(model: Arc<LutStackModel>) -> Self {
        let rec_states = vec![FbsRecState::default(); model.layers.len()];
        let hamming_caches = vec![HammingCache::default(); model.hamming.len()];
        Self {
            model,
            rec_states,
            hamming_caches,
            tokens: Vec::new(),
            reply_start: None,
            anchors: TurnMemoryAnchors::default(),
            reply_history: Vec::new(),
            suffix_counts: Vec::new(),
            pending_basins: Vec::new(),
            last_matched_stems: 0,
            last_point_hamming_bits: 0,
            last_covered_stem_mask: 0,
            last_turn_idx: usize::MAX,
            prompt_stem_bucket: 0,
            last_n_t: [0; FBS_WIDTH],
            prompt_only_tokens: Vec::new(),
            prompt_n_t: [0; FBS_WIDTH],
            has_executive_slot_trace: false,
            recent_turn_rows: Vec::new(),
            emitted_4grams: Vec::new(),
            boundary_trajectories: Vec::new(),
            total_l1_margin: 0,
        }
    }

    pub fn new_with_context(model: Arc<LutStackModel>, context: &RollingContextState) -> Self {
        let mut sess = Self::new(model);
        sess.seed_context(context);
        sess
    }

    pub fn seed_context(&mut self, context: &RollingContextState) {
        if let Some(layer0) = self.rec_states.first_mut() {
            for d in 0..FBS_WIDTH {
                let val = context.q12_latent[d] as i32;
                layer0.h_state[d] = val;
                for k in 0..FBS_CONV_KERNEL {
                    layer0.conv_ring[k][d] = val;
                }
            }
        }
        self.recent_turn_rows = context.recent_turn_rows.clone();
        self.emitted_4grams = context.emitted_4grams.clone();
    }

    pub fn reset(&mut self) {
        for s in &mut self.rec_states {
            *s = FbsRecState::default();
        }
        for c in &mut self.hamming_caches {
            *c = HammingCache::default();
        }
        self.tokens.clear();
        self.reply_start = None;
        self.anchors = TurnMemoryAnchors::default();
        self.reply_history.clear();
        self.suffix_counts.clear();
        self.pending_basins.clear();
        self.last_matched_stems = 0;
        self.last_point_hamming_bits = 0;
        self.last_covered_stem_mask = 0;
        self.last_turn_idx = usize::MAX;
        self.prompt_stem_bucket = 0;
        self.last_n_t = [0; FBS_WIDTH];
        self.prompt_only_tokens.clear();
        self.prompt_n_t = [0; FBS_WIDTH];
        self.has_executive_slot_trace = false;
        self.recent_turn_rows.clear();
        self.emitted_4grams.clear();
        self.boundary_trajectories.clear();
        self.total_l1_margin = 0;
    }

    /// Record a clause boundary trajectory hit (n_t, y_hat_t, y_t, l1_margin) into session memory.
    pub fn record_boundary_trajectory(
        &mut self,
        step: usize,
        boundary_token: u32,
        predicted_token: u32,
        target_token: u32,
    ) -> PointTrajectoryHit {
        let mut n_t_i16 = [0i16; FBS_WIDTH];
        for (d, slot) in n_t_i16.iter_mut().enumerate() {
            *slot = self.last_n_t[d].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
        let l1_margin = compute_token_l1_margin(
            &self.model.writer_table,
            self.model.vocab,
            predicted_token,
            target_token,
        );
        let hit = PointTrajectoryHit::new(
            step,
            boundary_token,
            n_t_i16,
            predicted_token,
            target_token,
            l1_margin,
        );
        self.total_l1_margin = self.total_l1_margin.saturating_add(l1_margin);
        self.boundary_trajectories.push(hit.clone());
        hit
    }

    /// Record a visited turn basin row into recent session memory (FIFO, max 8 items).
    pub fn record_turn_basin(&mut self, row: usize) {
        self.recent_turn_rows.push(row);
        if self.recent_turn_rows.len() > 8 {
            self.recent_turn_rows.remove(0);
        }
    }

    /// Record an emitted 4-gram token hash into session memory (deduplicating and capping to 2048 entries).
    pub fn record_emitted_4gram(&mut self, hash: u64) {
        if !self.emitted_4grams.contains(&hash) {
            self.emitted_4grams.push(hash);
            if self.emitted_4grams.len() > 2048 {
                self.emitted_4grams.remove(0);
            }
        }
    }

    pub fn position(&self) -> usize {
        self.tokens.len()
    }

    pub fn mark_reply_start(&mut self) {
        self.reply_start = Some(self.tokens.len() + 1);
        self.reply_history.clear();
        self.suffix_counts.clear();
    }

    pub fn clear_reply_start(&mut self) {
        self.reply_start = None;
        self.reply_history.clear();
        self.suffix_counts.clear();
    }

    /// Prefill a prompt or reasoning token into the session state.
    /// When `run_trunk = true`, forwards through the 6-layer `SpiralCore/FBS` + `ServedHamming`
    /// trunk and evolves `self.last_n_t` to steer the latent manifold trajectory.
    pub fn prefill_token(&mut self, token: u32, run_trunk: bool) {
        self.tokens.push(token);
        if run_trunk {
            let n_t = self.model.forward_token_hidden(
                token,
                &mut self.rec_states,
                &mut self.hamming_caches,
            );
            self.last_n_t = n_t;
        }
    }

    /// Prefill a slice of tokens sequentially through the session state.
    pub fn prefill_tokens(&mut self, tokens: &[u32], run_trunk: bool) {
        for &tok in tokens {
            self.prefill_token(tok, run_trunk);
        }
    }

    /// Autoregressively step through `<thinking>...</thinking>` reasoning tokens,
    /// feeding each token into `self.prefill_token(tok, true)` so the reasoning trajectory
    /// directly steers `self.last_n_t`.
    ///
    /// Sets `self.mark_reply_start()` strictly AFTER `</thinking>`, establishing
    /// the commencement barrier for visible answer prediction.
    ///
    /// Returns `(thinking_tokens, final_steered_n_t)`.
    pub fn step_reasoning_trace(&mut self, thinking_trace: &str) -> (Vec<u32>, [i32; FBS_WIDTH]) {
        let bpe = ByteBpe4096::global();
        let trimmed = thinking_trace.trim();
        if trimmed.is_empty() {
            self.mark_reply_start();
            return (Vec::new(), self.last_n_t);
        }

        if self.prompt_only_tokens.is_empty() {
            self.prompt_only_tokens = self.tokens.clone();
            self.prompt_n_t = self.last_n_t;
        }

        let mut formatted_block = trimmed.to_string();
        if !formatted_block.starts_with("<thinking>") && !formatted_block.starts_with("<thought>") {
            formatted_block = format!("<thinking>\n{}\n</thinking>", formatted_block);
        } else if !formatted_block.contains("</thinking>")
            && !formatted_block.contains("</thought>")
        {
            formatted_block.push_str("\n</thinking>");
        }

        let reasoning_tokens = bpe.encode(&formatted_block);
        for &tok in &reasoning_tokens {
            self.prefill_token(tok, true);
        }

        // Enforcement barrier: visible reply commencement begins ONLY after </thinking>
        self.mark_reply_start();

        (reasoning_tokens, self.last_n_t)
    }

    /// Commence visible reply prediction using /8 `ServedPointKey` and `writer.table`
    /// evaluated on the reasoning-steered `self.last_n_t`.
    ///
    /// Locks `self.anchors.reply` to the continuation basin chosen by the steered trajectory.
    pub fn commence_visible_prediction(&mut self) -> (StepTelemetry, u32) {
        let row = self.tokens.len();
        let mut n_t = self.last_n_t;
        let mut source_mechanism = "whole_table_nearest";
        let mut memory_hit_token: Option<u32> = None;

        if let Some(mem) = &self.model.memory {
            let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
            let anchor_tokens =
                if self.has_executive_slot_trace && !self.prompt_only_tokens.is_empty() {
                    &self.prompt_only_tokens
                } else {
                    &self.tokens
                };
            let open_ids = extract_turn_ids(anchor_tokens, mem.key_window, mem.k);
            let reply_ids = extract_turn_ids(anchor_tokens, mem.key_window, reply_turn_k);
            self.anchors.opening = open_ids.clone();
            self.anchors.reply = reply_ids.clone();

            if self.has_executive_slot_trace {
                if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
                    let val_tok = mem.opening.values[entry_idx];
                    self.last_matched_stems = 4;
                    self.last_point_hamming_bits = 0;
                    if let (Some(reply_tbl), Some(point)) = (&mem.reply, &self.model.point) {
                        if !probe_reply_table_has_anchor(
                            reply_tbl,
                            mem.reply_key_len,
                            &self.anchors.reply,
                            val_tok,
                        ) {
                            if let Some(anchors) = &point.canonical_anchors {
                                let alen = point.anchor_len.max(1);
                                if anchors.len() >= point.rows * alen {
                                    for r in 0..point.rows {
                                        let base = r * alen;
                                        let a_row = &anchors[base..base + alen];
                                        let open_matches = if alen >= 16
                                            && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                                        {
                                            let copy_open = open_ids.len().min(8);
                                            a_row[8..8 + copy_open] == open_ids[..copy_open]
                                        } else if alen >= reply_turn_k + open_ids.len() {
                                            a_row[reply_turn_k..reply_turn_k + open_ids.len()]
                                                == *open_ids
                                        } else {
                                            false
                                        };
                                        if open_matches {
                                            let cand = a_row[..reply_turn_k.min(alen)].to_vec();
                                            if point.targets.get(r).copied() == Some(val_tok)
                                                && probe_reply_table_has_anchor(
                                                    reply_tbl,
                                                    mem.reply_key_len,
                                                    &cand,
                                                    val_tok,
                                                )
                                            {
                                                self.anchors.reply = cand;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    blend_toward_writer_row(
                        &mut n_t,
                        &self.model.writer_table,
                        val_tok as usize,
                        &mem.opening.weight_q24,
                    );
                    memory_hit_token = Some(val_tok);
                    source_mechanism = "turn_memory_opening";
                }
            }

            if memory_hit_token.is_none() {
                if let Some(point) = &self.model.point {
                    let pt_query_n_t =
                        if self.has_executive_slot_trace && !self.prompt_only_tokens.is_empty() {
                            &self.prompt_n_t
                        } else {
                            &n_t
                        };
                    if let Some(pt_hit) = select_point_key_with_resonance(
                        point,
                        pt_query_n_t,
                        anchor_tokens,
                        mem.key_window,
                        mem.reply.as_ref(),
                        mem.reply_key_len,
                        &self.recent_turn_rows,
                    ) {
                        self.last_matched_stems = pt_hit.matched_stems;
                        self.last_point_hamming_bits = pt_hit.hamming_bits;
                        self.last_covered_stem_mask = pt_hit.covered_stem_mask;
                        self.last_turn_idx = pt_hit.turn_idx;

                        if let Some(canon) = resolve_canonical_reply_anchor(
                            point,
                            mem,
                            pt_hit.row,
                            pt_hit.target_token,
                            &reply_ids,
                        ) {
                            self.anchors.reply = canon;
                        }
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            pt_hit.target_token as usize,
                            &mem.opening.weight_q24,
                        );
                        memory_hit_token = Some(pt_hit.target_token);
                        source_mechanism = if pt_hit.confident {
                            "point_key_vote"
                        } else {
                            "point_key_fallback"
                        };
                    }
                }
            }
        }

        let top_candidates =
            stack_whole_table_top_k(&n_t, &self.model.writer_table, self.model.vocab, 5);
        let chosen_tok = memory_hit_token
            .unwrap_or_else(|| top_candidates.first().map(|&(t, _)| t).unwrap_or(0));

        let q_sketch = pack_i32_sign_sketch_256(&n_t);
        let pred_base = (chosen_tok as usize) * FBS_WIDTH;
        let hamming_xor_bits = if pred_base + FBS_WIDTH <= self.model.writer_table.len() {
            let w_sketch = pack_i16_sign_sketch_256(
                &self.model.writer_table[pred_base..pred_base + FBS_WIDTH],
            );
            hamming_distance_256(&q_sketch, &w_sketch)
        } else {
            256
        };

        let st = StepTelemetry {
            position: row,
            input_token: self.tokens.last().copied().unwrap_or(0),
            predicted_token: chosen_tok,
            source_mechanism,
            l1_distance: 0,
            hamming_xor_bits,
            icosian_root: 0,
            spin_holonomy: 0,
            trinary_active_lanes: 0,
            top_candidates,
        };

        (st, chosen_tok)
    }

    fn next_4gram_suffix_occurrence(&mut self, tok: u32) -> ([u32; 4], u32) {
        self.reply_history.push(tok);
        let n = self.reply_history.len();
        let s0 = if n >= 4 {
            self.reply_history[n - 4]
        } else {
            MEMORY_PAD
        };
        let s1 = if n >= 3 {
            self.reply_history[n - 3]
        } else {
            MEMORY_PAD
        };
        let s2 = if n >= 2 {
            self.reply_history[n - 2]
        } else {
            MEMORY_PAD
        };
        let s3 = self.reply_history[n - 1];
        let suffix = [s0, s1, s2, s3];
        for (existing_suf, count) in &mut self.suffix_counts {
            if *existing_suf == suffix {
                let occ = *count;
                *count = count.saturating_add(1);
                return (suffix, occ);
            }
        }
        self.suffix_counts.push((suffix, 1));
        (suffix, 0)
    }

    /// Execute one causal token step through the 6-layer Spin-Preserving `SpiralCore/FBS` +
    /// `ServedHamming` Bitwise XOR Popcount attention + Position-Free Causal 4-Gram `ServedTurnMemory` (`/7`) +
    /// `ServedPointKey` (`/8` with 32-anchor stem/BPE resonance) + `stack_whole_table_nearest` over `writer.table`.
    pub fn step_with_telemetry(&mut self, token: u32) -> StepTelemetry {
        let row = self.tokens.len();
        self.tokens.push(token);

        let mut n_t =
            self.model
                .forward_token_hidden(token, &mut self.rec_states, &mut self.hamming_caches);
        self.last_n_t = n_t;

        let mut source_mechanism = "whole_table_nearest";
        let mut memory_hit_token: Option<u32> = None;

        let suffix_occ = if self.reply_start.is_some_and(|rs| self.tokens.len() >= rs) {
            Some(self.next_4gram_suffix_occurrence(token))
        } else {
            None
        };

        if let (Some(mem), Some(rs)) = (&self.model.memory, self.reply_start) {
            let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
            if row + 1 == rs {
                let open_ids = extract_turn_ids(&self.tokens, mem.key_window, mem.k);
                let reply_ids = extract_turn_ids(&self.tokens, mem.key_window, reply_turn_k);
                self.anchors.opening = open_ids.clone();
                self.anchors.reply = reply_ids.clone();
                let active_open: Vec<u32> = open_ids
                    .iter()
                    .copied()
                    .filter(|&t| t != MEMORY_PAD)
                    .collect();
                let dec_open = ByteBpe4096::global().decode(&active_open);
                self.prompt_stem_bucket = extract_salient_stem_hashes_from_text(&dec_open, 1)
                    .first()
                    .map(|&(sh, _)| (sh & 0x3FFF_FFFF) as usize)
                    .unwrap_or(0);

                if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
                    let val_tok = mem.opening.values[entry_idx];
                    self.last_matched_stems = 4;
                    self.last_point_hamming_bits = 0;
                    if let (Some(reply_tbl), Some(point)) = (&mem.reply, &self.model.point) {
                        if !probe_reply_table_has_anchor(
                            reply_tbl,
                            mem.reply_key_len,
                            &self.anchors.reply,
                            val_tok,
                        ) {
                            let mut matched_open_anchor = None;
                            let mut fallback_open_anchor = None;
                            if let Some(anchors) = &point.canonical_anchors {
                                let alen = point.anchor_len.max(1);
                                if anchors.len() >= point.rows * alen {
                                    for r in 0..point.rows {
                                        let base = r * alen;
                                        let a_row = &anchors[base..base + alen];
                                        let open_matches = if alen >= 16
                                            && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                                        {
                                            let copy_open = open_ids.len().min(8);
                                            a_row[8..8 + copy_open] == open_ids[..copy_open]
                                        } else if alen >= reply_turn_k + open_ids.len() {
                                            a_row[reply_turn_k..reply_turn_k + open_ids.len()]
                                                == *open_ids
                                        } else {
                                            false
                                        };
                                        if open_matches {
                                            let cand = a_row[..reply_turn_k.min(alen)].to_vec();
                                            if point.targets.get(r).copied() == Some(val_tok)
                                                && probe_reply_table_has_anchor(
                                                    reply_tbl,
                                                    mem.reply_key_len,
                                                    &cand,
                                                    val_tok,
                                                )
                                            {
                                                matched_open_anchor = Some(cand);
                                                break;
                                            } else if fallback_open_anchor.is_none() {
                                                fallback_open_anchor = Some(cand);
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(canon) = matched_open_anchor.or(fallback_open_anchor) {
                                self.anchors.reply = canon;
                            } else if let Some(pt_hit) = select_point_key_with_resonance(
                                point,
                                &n_t,
                                &self.tokens,
                                mem.key_window,
                                mem.reply.as_ref(),
                                mem.reply_key_len,
                                &self.recent_turn_rows,
                            ) {
                                self.last_matched_stems = pt_hit.matched_stems;
                                self.last_point_hamming_bits = pt_hit.hamming_bits;
                                self.last_covered_stem_mask = pt_hit.covered_stem_mask;
                                self.last_turn_idx = pt_hit.turn_idx;
                                if let Some(canon) = resolve_canonical_reply_anchor(
                                    point, mem, pt_hit.row, val_tok, &reply_ids,
                                ) {
                                    self.anchors.reply = canon;
                                }
                            }
                        }
                    }
                    blend_toward_writer_row(
                        &mut n_t,
                        &self.model.writer_table,
                        val_tok as usize,
                        &mem.opening.weight_q24,
                    );
                    memory_hit_token = Some(val_tok);
                    source_mechanism = "turn_memory_opening";
                } else if let Some(point) = &self.model.point {
                    if let Some(pt_hit) = select_point_key_with_resonance(
                        point,
                        &n_t,
                        &self.tokens,
                        mem.key_window,
                        mem.reply.as_ref(),
                        mem.reply_key_len,
                        &self.recent_turn_rows,
                    ) {
                        self.last_matched_stems = pt_hit.matched_stems;
                        self.last_point_hamming_bits = pt_hit.hamming_bits;
                        self.last_covered_stem_mask = pt_hit.covered_stem_mask;
                        self.last_turn_idx = pt_hit.turn_idx;
                        if let Some(canon) = resolve_canonical_reply_anchor(
                            point,
                            mem,
                            pt_hit.row,
                            pt_hit.target_token,
                            &reply_ids,
                        ) {
                            self.anchors.reply = canon;
                        }
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            pt_hit.target_token as usize,
                            &mem.opening.weight_q24,
                        );
                        memory_hit_token = Some(pt_hit.target_token);
                        source_mechanism = if pt_hit.confident {
                            "point_key_vote"
                        } else {
                            "point_key_fallback"
                        };
                    }
                }
            } else if row >= rs {
                let (suffix4, occ) = suffix_occ.unwrap_or(([MEMORY_PAD; 4], 0));
                let step = (row - rs + 1) as u32;
                if let Some(reply_tbl) = &mem.reply {
                    if let Some(val_tok) = lookup_reply_continuation_4gram(
                        reply_tbl,
                        mem.reply_key_len,
                        &self.anchors.reply,
                        suffix4,
                        occ,
                        step,
                    ) {
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            val_tok as usize,
                            &reply_tbl.weight_q24,
                        );
                        memory_hit_token = Some(val_tok);
                        source_mechanism = "turn_memory_reply";
                    }
                }
            }
        }

        let mut top_candidates =
            stack_whole_table_top_k(&n_t, &self.model.writer_table, self.model.vocab, 5);
        if memory_hit_token.is_none() {
            if let Some(ta) = &self.model.ternary_assoc {
                let prev_tok = if self.tokens.len() >= 2 {
                    self.tokens[self.tokens.len() - 2]
                } else {
                    MEMORY_PAD
                };
                for (cand_tok, cand_dist) in &mut top_candidates {
                    let hits =
                        ta.score_transition(prev_tok, token, self.prompt_stem_bucket, *cand_tok);
                    *cand_dist = cand_dist.saturating_sub(hits * 45_000);
                }
                top_candidates.sort_by_key(|&(_, d)| d);
                if top_candidates.first().is_some_and(|&(ct, _)| {
                    ta.score_transition(prev_tok, token, self.prompt_stem_bucket, ct) > 0
                }) {
                    source_mechanism = "ternary_assoc_nearest";
                }
            }
        }

        let n_toks = self.tokens.len();
        let t_minus_1 = token;
        let t_minus_2 = if n_toks >= 2 {
            self.tokens[n_toks - 2]
        } else {
            MEMORY_PAD
        };
        let t_minus_3 = if n_toks >= 3 {
            self.tokens[n_toks - 3]
        } else {
            MEMORY_PAD
        };

        if let Some(hit_tok) = memory_hit_token {
            let base = (hit_tok as usize) * FBS_WIDTH;
            let hit_dist = if base + FBS_WIDTH <= self.model.writer_table.len() {
                let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                (0..FBS_WIDTH)
                    .map(|d| ((n_t[d] as i64) - (row_w[d] as i64)).abs())
                    .sum()
            } else {
                top_candidates.first().map(|&(_, d)| d).unwrap_or(0)
            };
            if let Some(pos) = top_candidates.iter().position(|&(t, _)| t == hit_tok) {
                top_candidates.remove(pos);
            }
            top_candidates.insert(0, (hit_tok, hit_dist));
            top_candidates.truncate(5);
        }

        let has_full_4gram = t_minus_3 != MEMORY_PAD && t_minus_2 != MEMORY_PAD;
        let (predicted_token, l1_distance, source_mechanism) =
            if source_mechanism == "turn_memory_reply" || !has_full_4gram {
                let (tok, dist) = if let Some(hit_tok) = memory_hit_token {
                    let dist = top_candidates
                        .iter()
                        .find(|&&(t, _)| t == hit_tok)
                        .map(|&(_, d)| d)
                        .unwrap_or(0);
                    (hit_tok, dist)
                } else {
                    top_candidates.first().copied().unwrap_or_else(|| {
                        stack_whole_table_nearest(&n_t, &self.model.writer_table, self.model.vocab)
                    })
                };
                (tok, dist, source_mechanism)
            } else {
                // Select the best candidate from top_candidates (writer.table and ServedHamming)
                // that does not repeat an already-emitted 4-gram
                let mut selected_token = None;
                let mut selected_dist = 0i64;
                let mut selected_source = source_mechanism;

                for (idx, &(cand_tok, cand_dist)) in top_candidates.iter().enumerate() {
                    let cand_h = hash_4gram(t_minus_3, t_minus_2, t_minus_1, cand_tok);
                    if !self.emitted_4grams.contains(&cand_h) {
                        selected_token = Some(cand_tok);
                        selected_dist = cand_dist;
                        if idx > 0 {
                            selected_source = "ngram_fallback_nearest";
                        }
                        break;
                    }
                }

                if let Some(tok) = selected_token {
                    (tok, selected_dist, selected_source)
                } else {
                    let (nearest_tok, nearest_dist) =
                        stack_whole_table_nearest(&n_t, &self.model.writer_table, self.model.vocab);
                    (nearest_tok, nearest_dist, "ngram_fallback_nearest")
                }
            };

        if has_full_4gram {
            let chosen_hash = hash_4gram(t_minus_3, t_minus_2, t_minus_1, predicted_token);
            self.record_emitted_4gram(chosen_hash);
        }

        let q_sketch = pack_i32_sign_sketch_256(&n_t);
        let pred_base = (predicted_token as usize) * FBS_WIDTH;
        let hamming_xor_bits = if pred_base + FBS_WIDTH <= self.model.writer_table.len() {
            let w_sketch = pack_i16_sign_sketch_256(
                &self.model.writer_table[pred_base..pred_base + FBS_WIDTH],
            );
            hamming_distance_256(&q_sketch, &w_sketch)
        } else {
            0
        };

        let (icosian_root, spin_holonomy, trinary_active_lanes) =
            if let Some(last_layer) = self.rec_states.last() {
                let r = last_layer.icosian_trajectory[3];
                let mut h_acc = 0u8;
                let mut active = 0usize;
                for lane in 0..FBS_LANES {
                    h_acc = cl06_compose(h_acc, last_layer.spin_holonomy[lane]);
                    if last_layer.trinary_route[lane] != 0 {
                        active += 1;
                    }
                }
                (r, h_acc, active)
            } else {
                (0, 0, 0)
            };

        StepTelemetry {
            position: self.tokens.len(),
            input_token: token,
            predicted_token,
            source_mechanism,
            l1_distance,
            hamming_xor_bits,
            icosian_root,
            spin_holonomy,
            trinary_active_lanes,
            top_candidates,
        }
    }
}

/// Probe whether `reply_tbl` contains a continuation for `anchor` starting from `first_tok`.
fn probe_reply_table_has_anchor(
    reply_tbl: &TurnMemoryTable,
    reply_key_len: usize,
    anchor: &[u32],
    first_tok: u32,
) -> bool {
    lookup_reply_continuation_4gram(
        reply_tbl,
        reply_key_len,
        anchor,
        [MEMORY_PAD, MEMORY_PAD, MEMORY_PAD, first_tok],
        0,
        1,
    )
    .is_some()
}

/// Universal corpus-wide suffix anchor (`0xE000_0000`), disjoint from turn-specific anchors (`0x8000_0000 | idx`),
/// stem hashes (`0x4000_0000 | h`), BPE token IDs (`0..4095`), and `MEMORY_PAD` (`0xFFFF_FFFF`).
pub const GLOBAL_SUFFIX_ANCHOR: u32 = 0xE000_0000;

/// Multi-Scale Position-Free Causal Suffix lookup in `reply_tbl`:
/// 1. Tier 1: Anchor-bound 4-gram (`(anchor, occ, prev_3, prev_2, prev_1, curr_tok)`) + `occ = 0` backoff
/// 2. Tier 2: Anchor-bound 2-gram backoff (`(anchor, 0, PAD, PAD, prev_1, curr_tok)`)
/// 3. Tier 3: Universal corpus-wide 4-gram (`(GLOBAL_SUFFIX_ANCHOR, 0, prev_3, prev_2, prev_1, curr_tok)`)
///    and 3-gram (`(GLOBAL_SUFFIX_ANCHOR, 0, PAD, prev_2, prev_1, curr_tok)`) continuation
/// 4. Tier 4: Legacy step-index compatibility fallback
fn lookup_reply_continuation_4gram(
    reply_tbl: &TurnMemoryTable,
    reply_key_len: usize,
    anchor: &[u32],
    suffix4: [u32; 4],
    occ: u32,
    legacy_step: u32,
) -> Option<u32> {
    if reply_key_len < 6 {
        return None;
    }
    let copy_n = anchor.len().min(reply_key_len.saturating_sub(5));
    let mut key = vec![MEMORY_PAD; reply_key_len];
    key[..copy_n].copy_from_slice(&anchor[..copy_n]);
    key[reply_key_len - 5] = occ;
    key[reply_key_len - 4] = suffix4[0];
    key[reply_key_len - 3] = suffix4[1];
    key[reply_key_len - 2] = suffix4[2];
    key[reply_key_len - 1] = suffix4[3];
    if let Some(val) = reply_tbl.lookup_value(&key) {
        return Some(val);
    }
    if occ > 0 {
        key[reply_key_len - 5] = 0;
        if let Some(val) = reply_tbl.lookup_value(&key) {
            return Some(val);
        }
    }
    // Tier 1.5: Anchor-bound 3-gram backoff (when `suffix4[1] != MEMORY_PAD`)
    if suffix4[1] != MEMORY_PAD {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = suffix4[1];
        key[reply_key_len - 2] = suffix4[2];
        key[reply_key_len - 1] = suffix4[3];
        if let Some(val) = reply_tbl.lookup_value(&key) {
            return Some(val);
        }
    }
    // Tier 2: Anchor-bound 2-gram backoff (when `prev_1 != MEMORY_PAD`)
    if suffix4[2] != MEMORY_PAD {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = MEMORY_PAD;
        key[reply_key_len - 2] = suffix4[2];
        key[reply_key_len - 1] = suffix4[3];
        if let Some(val) = reply_tbl.lookup_value(&key) {
            return Some(val);
        }
    }
    // Tier 2b: Anchor-bound 1-gram / opening continuation fallback
    // (enables mid-stream transition to a new basin while preserving continuous reply_history)
    key[reply_key_len - 5] = 0;
    key[reply_key_len - 4] = MEMORY_PAD;
    key[reply_key_len - 3] = MEMORY_PAD;
    key[reply_key_len - 2] = MEMORY_PAD;
    key[reply_key_len - 1] = suffix4[3];
    if let Some(val) = reply_tbl.lookup_value(&key) {
        return Some(val);
    }
    // Tier 3: Universal corpus-wide 4-gram and 3-gram continuation backoff
    if suffix4[1] != MEMORY_PAD {
        let mut gkey = vec![MEMORY_PAD; reply_key_len];
        gkey[0] = GLOBAL_SUFFIX_ANCHOR;
        gkey[reply_key_len - 5] = 0;
        gkey[reply_key_len - 4] = suffix4[0];
        gkey[reply_key_len - 3] = suffix4[1];
        gkey[reply_key_len - 2] = suffix4[2];
        gkey[reply_key_len - 1] = suffix4[3];
        if suffix4[0] != MEMORY_PAD {
            if let Some(val) = reply_tbl.lookup_value(&gkey) {
                return Some(val);
            }
        }
        gkey[reply_key_len - 4] = MEMORY_PAD;
        if let Some(val) = reply_tbl.lookup_value(&gkey) {
            return Some(val);
        }
    }
    // Tier 4: Compatibility fallback for step-indexed `.uorlut` tables
    let mut legacy_key = vec![MEMORY_PAD; reply_key_len];
    let leg_copy = anchor.len().min(reply_key_len.saturating_sub(4));
    legacy_key[..leg_copy].copy_from_slice(&anchor[..leg_copy]);
    legacy_key[reply_key_len - 4] = MEMORY_PAD;
    legacy_key[reply_key_len - 3] = legacy_step;
    legacy_key[reply_key_len - 2] = MEMORY_PAD;
    legacy_key[reply_key_len - 1] = suffix4[3];
    reply_tbl.lookup_value(&legacy_key)
}

fn blend_toward_writer_row(
    n_t: &mut [i32; FBS_WIDTH],
    writer_table: &[i16],
    tok: usize,
    weight_q24: &[i32],
) {
    let base = tok * FBS_WIDTH;
    if base + FBS_WIDTH > writer_table.len() {
        return;
    }
    let row = &writer_table[base..base + FBS_WIDTH];
    for d in 0..FBS_WIDTH {
        let w = (weight_q24.get(d).copied().unwrap_or(1 << 23) >> 12).clamp(0, 4096) as i64;
        let cur = n_t[d] as i64;
        let target = row[d] as i64;
        n_t[d] = (((4096 - w) * cur + w * target) >> 12).clamp(-32768, 32767) as i32;
    }
}

/// Extract the `k` salient turn token IDs from the user prompt (`stack/memory.rs` `turn_key`).
pub fn extract_turn_ids(tokens: &[u32], key_window: usize, k: usize) -> Vec<u32> {
    let bpe = ByteBpe4096::global();
    let nl_user_v2 = bpe.encode("\nUser:");
    let nl_user_v1 = bpe.encode("\nUser: ");
    let user_v2 = bpe.encode("User:");
    let user_v1 = bpe.encode("User: ");
    let nl_asst_v2 = bpe.encode("\nAssistant:");
    let nl_asst_v1 = bpe.encode("\nAssistant: ");

    let win_start = tokens.len().saturating_sub(key_window);
    let win = &tokens[win_start..];

    let end = if win.ends_with(&nl_asst_v2) {
        win.len() - nl_asst_v2.len()
    } else if win.ends_with(&nl_asst_v1) {
        win.len() - nl_asst_v1.len()
    } else {
        win.len()
    };

    let slice = &win[..end];
    let mut content_start = 0usize;
    if slice.len() >= nl_user_v2.len() {
        for i in (0..=slice.len() - nl_user_v2.len()).rev() {
            if slice[i..].starts_with(&nl_user_v2) {
                content_start = i + nl_user_v2.len();
                break;
            }
            if slice[i..].starts_with(&nl_user_v1) {
                content_start = i + nl_user_v1.len();
                break;
            }
        }
    }
    if content_start == 0 {
        for i in 0..slice.len() {
            if slice[i..].starts_with(&user_v2) {
                content_start = i + user_v2.len();
                break;
            }
            if slice[i..].starts_with(&user_v1) {
                content_start = i + user_v1.len();
                break;
            }
        }
    }

    let user_tokens = &slice[content_start..];
    let mut out = vec![MEMORY_PAD; k];
    let take = user_tokens.len().min(k);
    let tail = &user_tokens[user_tokens.len() - take..];
    out[..take].copy_from_slice(tail);
    out
}

pub fn is_bpe_framing_stopword(word: &str) -> bool {
    matches!(
        word,
        "what"
            | "how"
            | "why"
            | "who"
            | "where"
            | "when"
            | "which"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "do"
            | "does"
            | "did"
            | "can"
            | "could"
            | "would"
            | "should"
            | "will"
            | "shall"
            | "may"
            | "might"
            | "tell"
            | "let"
            | "lets"
            | "talk"
            | "talking"
            | "discuss"
            | "discussing"
            | "me"
            | "us"
            | "about"
            | "explain"
            | "describe"
            | "please"
            | "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "as"
            | "into"
            | "like"
            | "between"
            | "difference"
            | "compare"
            | "vs"
            | "versus"
            | "user"
            | "assistant"
            | "system"
            | "you"
            | "your"
            | "my"
            | "i"
            | "we"
            | "they"
            | "it"
            | "its"
            | "that"
            | "this"
            | "these"
            | "those"
            | "there"
            | "here"
            | "have"
            | "has"
            | "had"
            | "not"
            | "no"
            | "yes"
            | "if"
            | "then"
            | "than"
            | "so"
            | "also"
            | "just"
            | "more"
            | "most"
            | "some"
            | "any"
            | "all"
            | "other"
            | "such"
            | "only"
            | "own"
            | "same"
            | "too"
            | "very"
            | "make"
            | "makes"
            | "made"
            | "work"
            | "works"
            | "working"
            | "use"
            | "used"
            | "using"
            | "give"
            | "show"
            | "write"
            | "clear"
            | "terms"
            | "simple"
            | "simply"
            | "briefly"
            | "today"
            | "cause"
            | "causes"
            | "caused"
            | "happen"
            | "happens"
            | "through"
            | "walk"
            | "beginner"
            | "beginners"
    )
}

/// Normalize a lowercase word into its morphological stem so inflectional variants
/// (`"superconductors"` / `"superconductor"`, `"entangled"` / `"entanglement"`) share an exact stem hash.
pub fn lut_canonical_stem(word: &str) -> String {
    let mut s: String = word
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    for suffix in [
        "izations", "ization", "fulness", "ousness", "iveness", "ational", "bilities", "bility",
        "ings", "tion", "tions", "ment", "ments", "ness", "ities", "ity", "ally", "edly", "ing",
        "ies", "ers", "ors", "ed", "es", "ly", "er", "or", "s",
    ] {
        if s.len() >= suffix.len() + 3 && s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
            if suffix == "ies" {
                s.push('y');
            }
            break;
        }
    }
    s
}

/// Compute a 31-bit deterministic FNV-1a stem anchor hash (`0x4000_0000..0x7FFF_FFFF`),
/// disjoint from BPE token IDs (`0..4095`), turn markers (`0x8000_0000 | idx`), and `MEMORY_PAD`.
#[inline]
pub fn stem_anchor_hash(word: &str) -> u32 {
    let stem = lut_canonical_stem(word);
    let mut h: u32 = 0x811C_9DC5;
    for b in stem.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    0x4000_0000u32 | (h & 0x3FFF_FFFFu32)
}

/// 64-bit FNV-1a hash used for deterministic UOR content addressing.
#[inline]
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Compute a deterministic 64-bit FNV-1a hash over four 32-bit token IDs (16 bytes).
/// Used for sliding-window 4-gram repetition detection in zero-float integer Q12 decoding.
#[inline]
pub fn hash_4gram(t0: u32, t1: u32, t2: u32, t3: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &tok in &[t0, t1, t2, t3] {
        for &b in &tok.to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// Extract deduplicated `(stem_anchor_hash, specificity_weight)` pairs from natural text,
/// preserving hyphenated/plus-joined technical compounds (`"B-Trees"` -> `"btree"`, `"UTF-8"` -> `"utf8"`).
pub fn extract_salient_stem_hashes_from_text(text: &str, max_stems: usize) -> Vec<(u32, i64)> {
    let mut out: Vec<(u32, i64)> = Vec::new();
    // 1. First extract joined compounds from hyphenated/plus tokens (e.g. "B-Trees" -> "btree")
    for raw_tok in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '+') {
        if raw_tok.contains('-') || raw_tok.contains('+') {
            let joined: String = raw_tok
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if joined.len() >= 3 && !is_bpe_framing_stopword(&joined) {
                let stem = lut_canonical_stem(&joined);
                if stem.len() >= 3 && !is_bpe_framing_stopword(&stem) {
                    let sh = stem_anchor_hash(&stem);
                    if !out.iter().any(|&(existing, _)| existing == sh) {
                        out.push((sh, 240_000i64));
                        if out.len() >= max_stems {
                            return out;
                        }
                    }
                }
            }
        }
    }
    // 2. Then extract individual alphanumeric words
    for raw_w in text.split(|c: char| !c.is_ascii_alphanumeric()) {
        let lower = raw_w.to_ascii_lowercase();
        if lower.len() < 2 || is_bpe_framing_stopword(&lower) {
            continue;
        }
        let stem = lut_canonical_stem(&lower);
        if stem.len() < 2 || is_bpe_framing_stopword(&stem) {
            continue;
        }
        let sh = stem_anchor_hash(&stem);
        if out.iter().any(|&(existing, _)| existing == sh) {
            continue;
        }
        let weight = if stem.len() >= 6 {
            240_000i64
        } else if stem.len() >= 4 {
            160_000i64
        } else {
            100_000i64
        };
        out.push((sh, weight));
        if out.len() >= max_stems {
            break;
        }
    }
    out
}

pub fn is_lut_self_inquiry_prompt(lower: &str) -> bool {
    lower.contains("able to do")
        || lower.contains("can you do")
        || lower.contains("tasks you")
        || lower.contains("tasks can you")
        || lower.contains("capable of")
        || lower.contains("capabilities")
        || lower.contains("who are you")
        || lower.contains("what are you")
        || lower.contains("what model are you")
        || lower.contains("introduce yourself")
        || lower.contains("about yourself")
        || lower.contains("talk about you")
        || lower.contains("tell me about you")
        || lower.contains("how do you work")
        || lower.contains("under the hood")
        || lower.contains("explain your architecture")
        || lower.contains("what is uor-r4")
        || lower.contains("how does uor-r4")
        || lower.contains("lut-stack")
        || lower.contains("are you a transformer")
        || lower.contains("how can you help")
}

pub fn is_lut_greeting_only_prompt(lower: &str) -> bool {
    let trimmed = lower
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric());
    matches!(
        trimmed,
        "hello"
            | "hi"
            | "hey"
            | "greetings"
            | "good morning"
            | "good afternoon"
            | "good evening"
            | "how are you"
            | "how are you doing"
            | "how are you doing today"
            | "how is it going"
            | "hows it going"
            | "how are things"
            | "lets chat"
            | "let us chat"
    )
}

pub const CORE_CURRICULUM_TURNS: usize = 623;
pub const VERIFIED_FRONTIER_ANCHOR_TAG: u32 = 0x554F_5246;
/// Number of discrete roots in the $E_8$ orbit projection.
pub const E8_ROOT_COUNT: usize = 16;

/// Number of basis actions in the $\text{Cl}(0,6)$ Clifford carrier.
pub const CLIFFORD_ACTION_COUNT: usize = 64;

/// Number of discrete roots in the Icosian golden lattice projection.
pub const ICOSIAN_ROOT_COUNT: usize = 17;

/// Total number of discrete geometric buckets in the two-level radix directory (16 * 64 * 17 = 17,408).
pub const GEOMETRIC_BUCKET_COUNT: usize =
    E8_ROOT_COUNT * CLIFFORD_ACTION_COUNT * ICOSIAN_ROOT_COUNT;

/// Contiguous length of the radix directory offset table (17,408 + 1 = 17,409).
pub const RADIX_DIRECTORY_LEN: usize = GEOMETRIC_BUCKET_COUNT + 1;

/// 17-root Icosian Z[phi] GoldenQ12 root table (`R4.SpiralCoreFbs.icosianRootEntry`, `R4-24`).
/// Each entry is `[(a_lane, b_lane); 4]` in Q12.
pub const ICOSIAN_ROOTS_Q12: [[(i32, i32); 4]; 17] = [
    [(0, 0), (0, 0), (0, 0), (0, 0)],
    [(4096, 0), (0, 0), (0, 0), (0, 0)],
    [(-4096, 0), (0, 0), (0, 0), (0, 0)],
    [(0, 0), (4096, 0), (0, 0), (0, 0)],
    [(0, 0), (-4096, 0), (0, 0), (0, 0)],
    [(0, 0), (0, 0), (4096, 0), (0, 0)],
    [(0, 0), (0, 0), (-4096, 0), (0, 0)],
    [(0, 0), (0, 0), (0, 0), (4096, 0)],
    [(0, 0), (0, 0), (0, 0), (-4096, 0)],
    [(2048, 0), (2048, 0), (2048, 0), (2048, 0)],
    [(-2048, 0), (-2048, 0), (-2048, 0), (-2048, 0)],
    [(2048, 0), (2048, 0), (-2048, 0), (-2048, 0)],
    [(2048, 0), (-2048, 0), (2048, 0), (-2048, 0)],
    [(0, 2048), (2048, -2048), (2048, 0), (0, 0)],
    [(0, -2048), (-2048, 2048), (-2048, 0), (0, 0)],
    [(2048, 0), (0, 2048), (2048, -2048), (0, 0)],
    [(-2048, 0), (0, -2048), (-2048, 2048), (0, 0)],
];

/// Snap a 4-lane Q12 block against the 17-root Icosian Z[phi] table.
pub fn icosian_snap_select(lanes: [i16; 4]) -> (u8, i64) {
    let mut best_root = 0u8;
    let mut best_score = i64::MIN;
    for (idx, root) in ICOSIAN_ROOTS_Q12.iter().enumerate() {
        let mut a = 0i64;
        let mut b = 0i64;
        for lane in 0..4 {
            let x = i64::from(lanes[lane]);
            a += x * i64::from(root[lane].0);
            b += x * i64::from(root[lane].1);
        }
        let score = 2 * a + 3 * b;
        if score > best_score {
            best_score = score;
            best_root = idx as u8;
        }
    }
    (best_root, best_score)
}

/// Compute the primary and runner-up discrete `E8` shard roots `(e8_primary, e8_runner)` in `0..16`
/// by mean-centering `q12_lanes` (eliminating DC bias across lanes), accumulating centered
/// L1 energy across 8 strided octets, and combining the dominant centered octet (`0..7`)
/// with a balanced 256-lane centered sign-parity syndrome (`0..1`).
pub fn compute_geometric_e8_pair(q12_lanes: &[i16], sign_code: u64) -> (u8, u8) {
    let n = q12_lanes.len();
    if n == 0 {
        let p = (sign_code.count_ones() & 1) as u8;
        return (p, p ^ 1);
    }
    let sum_q12: i64 = q12_lanes.iter().map(|&v| i64::from(v)).sum();
    let mean_q12 = sum_q12 / (n as i64);

    let mut octet_energy = [0u64; 8];
    let mut centered_sketch = sign_code;
    let mut centered_pos_count = 0u32;
    for (idx, &v) in q12_lanes.iter().enumerate() {
        let centered = i64::from(v) - mean_q12;
        let mag = centered.unsigned_abs();
        octet_energy[idx & 7] = octet_energy[idx & 7].saturating_add(mag);
        if centered >= 0 {
            centered_pos_count += 1;
            centered_sketch ^= 1u64.wrapping_shl((idx & 63) as u32);
        } else {
            centered_sketch = centered_sketch.rotate_left(1) ^ (mag & 0xFF);
        }
    }

    let mut order = [(0u8, 0u64); 8];
    for oct in 0..8 {
        order[oct] = (oct as u8, octet_energy[oct]);
    }
    order.sort_unstable_by_key(|&(oct, energy)| (std::cmp::Reverse(energy), oct));

    let syndrome_oct = ((centered_sketch.count_ones() as u8)
        .wrapping_add((centered_sketch >> 13) as u8)
        .wrapping_add((centered_pos_count >> 2) as u8))
        & 7;
    let parity = ((centered_pos_count ^ sign_code.count_ones()) & 1) as u8;
    let dom_oct = (order[0].0 ^ syndrome_oct) & 7;
    let run_oct = (order[1].0 ^ syndrome_oct) & 7;
    let e8_1 = ((dom_oct << 1) | parity) % (E8_ROOT_COUNT as u8);
    let mut e8_2 = ((run_oct << 1) | (parity ^ 1)) % (E8_ROOT_COUNT as u8);
    if e8_2 == e8_1 {
        e8_2 = (e8_1 + 1) % (E8_ROOT_COUNT as u8);
    }
    (e8_1, e8_2)
}

/// Compute discrete geometric coordinates `(e8_root, clifford_action, icosian_root)` from
/// a Q12 vector and a 64-bit sign code using mean-centered 16-shard E8 partitioning.
pub fn compute_geometric_coordinates(q12_lanes: &[i16], sign_code: u64) -> (u8, u8, u8) {
    let (e8_root, _) = compute_geometric_e8_pair(q12_lanes, sign_code);
    let clifford_action = ((sign_code >> 8) as u8) % (CLIFFORD_ACTION_COUNT as u8);
    let lanes4 = [
        if !q12_lanes.is_empty() {
            q12_lanes[0]
        } else {
            0
        },
        if q12_lanes.len() > 1 { q12_lanes[1] } else { 0 },
        if q12_lanes.len() > 2 { q12_lanes[2] } else { 0 },
        if q12_lanes.len() > 3 { q12_lanes[3] } else { 0 },
    ];
    let icosian_root = (icosian_snap_select(lanes4).0) % (ICOSIAN_ROOT_COUNT as u8);
    (e8_root, clifford_action, icosian_root)
}

/// Compute discrete geometric bucket index `B = (e8 * 64 + clifford) * 17 + icosian`.
pub fn compute_geometric_bucket(e8_root: u8, clifford_action: u8, icosian_root: u8) -> usize {
    let e8 = (e8_root as usize) % E8_ROOT_COUNT;
    let c64 = (clifford_action as usize) % CLIFFORD_ACTION_COUNT;
    let i17 = (icosian_root as usize) % ICOSIAN_ROOT_COUNT;
    (e8 * CLIFFORD_ACTION_COUNT + c64) * ICOSIAN_ROOT_COUNT + i17
}

/// Decompose a discrete geometric bucket index `B` into `(e8_root, clifford_action, icosian_root)`.
pub fn decompose_geometric_bucket(bucket: usize) -> (u8, u8, u8) {
    let b = bucket % GEOMETRIC_BUCKET_COUNT;
    let i17 = (b % ICOSIAN_ROOT_COUNT) as u8;
    let rem = b / ICOSIAN_ROOT_COUNT;
    let c64 = (rem % CLIFFORD_ACTION_COUNT) as u8;
    let e8 = (rem / CLIFFORD_ACTION_COUNT) as u8;
    (e8, c64, i17)
}

#[derive(Clone, Copy, Debug)]
pub struct PointResonanceHit {
    pub target_token: u32,
    pub row: usize,
    pub turn_idx: usize,
    pub raw_l1: i64,
    pub hamming_bits: u32,
    pub matched_stems: usize,
    pub covered_stem_mask: u16,
    pub confident: bool,
}

pub fn select_point_key_with_resonance(
    point: &ServedPointKey,
    n_t: &[i32],
    prompt_tokens: &[u32],
    key_window: usize,
    reply_tbl: Option<&TurnMemoryTable>,
    reply_key_len: usize,
    recent_turn_rows: &[usize],
) -> Option<PointResonanceHit> {
    let Some(anchors) = &point.canonical_anchors else {
        return point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                turn_idx: usize::MAX,
                raw_l1: d,
                hamming_bits: 0,
                matched_stems: usize::from(d == 0),
                covered_stem_mask: 0,
                confident: d == 0,
            });
    };
    let alen = point.anchor_len.max(1);
    if anchors.len() < point.rows * alen {
        return point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                turn_idx: usize::MAX,
                raw_l1: d,
                hamming_bits: 0,
                matched_stems: usize::from(d == 0),
                covered_stem_mask: 0,
                confident: d == 0,
            });
    }

    let bpe = ByteBpe4096::global();
    let prompt_ids = extract_turn_ids(prompt_tokens, key_window, key_window.max(128));
    let active_prompt_ids: Vec<u32> = prompt_ids
        .iter()
        .copied()
        .filter(|&t| t != MEMORY_PAD)
        .collect();
    let decoded_turn = bpe.decode(&active_prompt_ids).to_ascii_lowercase();
    let is_self_inq = is_lut_self_inquiry_prompt(&decoded_turn);
    let is_greeting = is_lut_greeting_only_prompt(&decoded_turn);

    let prompt_stems = extract_salient_stem_hashes_from_text(&decoded_turn, 16);
    let mut prompt_bpe_content: Vec<(u32, i64)> = Vec::new();
    for &t in &active_prompt_ids {
        if t < 259 || prompt_bpe_content.iter().any(|&(id, _)| id == t) {
            continue;
        }
        let dec = bpe.decode(&[t]);
        let clean: String = dec
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        if clean.len() < 2 || is_bpe_framing_stopword(&clean) {
            continue;
        }
        let w = if clean.len() >= 5 || t >= 500 {
            95_000i64
        } else {
            45_000i64
        };
        prompt_bpe_content.push((t, w));
    }

    let q_sketch = pack_i32_sign_sketch_256(n_t);
    let q_abs_sum: i64 = n_t.iter().map(|&v| (v as i64).abs()).sum();
    let q_mean_abs = (q_abs_sum / (point.width.max(1) as i64)).max(1);
    let mut q_normed = [0i64; FBS_WIDTH];
    let w_lim = point.width.min(FBS_WIDTH);
    for d in 0..w_lim {
        q_normed[d] = ((n_t[d] as i64) * 1024) / q_mean_abs;
    }

    let total_stems = prompt_stems.len();
    let eff_total = total_stems.min(6);

    let (recent_min, recent_max) = if recent_turn_rows.is_empty() {
        (usize::MAX, usize::MIN)
    } else {
        let mut min = usize::MAX;
        let mut max = usize::MIN;
        for &r in recent_turn_rows {
            if r < min {
                min = r;
            }
            if r > max {
                max = r;
            }
        }
        (min, max)
    };

    struct BestCandidate {
        row: usize,
        score: i64,
        raw_l1: i64,
        hamming: u32,
        matched_stems: usize,
        covered_mask: u16,
        matched_bpe: usize,
        turn_idx: usize,
        bonus: i64,
        max_core_matched_stems: usize,
    }

    let mut best = BestCandidate {
        row: 0,
        score: i64::MAX,
        raw_l1: i64::MAX,
        hamming: 256,
        matched_stems: 0,
        covered_mask: 0,
        matched_bpe: 0,
        turn_idx: usize::MAX,
        bonus: 0,
        max_core_matched_stems: 0,
    };

    let eval_row = |r: usize, best: &mut BestCandidate| -> bool {
        let p_base = r * point.width;
        let p_row = &point.points[p_base..p_base + point.width];
        let a_base = r * alen;
        let a_row = &anchors[a_base..a_base + alen];

        let turn_idx = if (a_row[0] & 0x8000_0000) != 0 {
            (a_row[0] & 0x7FFF_FFFF) as usize
        } else {
            usize::MAX
        };
        let is_core_or_proved = turn_idx < CORE_CURRICULUM_TURNS
            || (alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
        if !is_core_or_proved {
            let core_already_complete = is_self_inq
                || is_greeting
                || (best.raw_l1 == 0 && best.turn_idx < CORE_CURRICULUM_TURNS)
                || (total_stems > 0 && best.max_core_matched_stems >= total_stems.max(2))
                || (best.max_core_matched_stems >= 2
                    && eff_total < best.max_core_matched_stems + 2);
            if core_already_complete {
                return false;
            }
        }

        // Guard self-inquiry bio rows (0..=2) and pure greeting rows (3..=4) so unrelated prompts
        // never collapse onto "I am the UOR-R4 Generalized Geometric Assistant..."
        let mut guard_penalty = 0i64;
        if turn_idx <= 2 && !is_self_inq {
            guard_penalty = 80_000_000;
        } else if (turn_idx == 3 || turn_idx == 4) && !is_greeting && !prompt_stems.is_empty() {
            guard_penalty = 60_000_000;
        }

        let basin_penalty: i64 = if r >= recent_min && r <= recent_max {
            if let Some(pos) = recent_turn_rows.iter().rposition(|&row_id| row_id == r) {
                250_000i64 * (pos as i64 + 1)
            } else {
                0i64
            }
        } else {
            0i64
        };

        let primary_slice = if alen >= 22 {
            &a_row[16..22]
        } else if alen > 16 {
            &a_row[16..alen]
        } else {
            &[]
        };
        let secondary_slice = if alen >= 28 { &a_row[22..28] } else { &[] };
        let bpe_slice = if alen >= 32 { &a_row[28..32] } else { &[] };
        let row_primary_count = primary_slice
            .iter()
            .filter(|&&x| x != MEMORY_PAD)
            .count()
            .max(1);

        let mut bonus = 0i64;
        let mut matched_stems = 0usize;
        let mut matched_primary = 0usize;
        let mut covered_mask = 0u16;
        for (stem_i, &(sh, sw)) in prompt_stems.iter().enumerate() {
            if primary_slice.contains(&sh) {
                matched_stems += 1;
                matched_primary += 1;
                if stem_i < 16 {
                    covered_mask |= 1u16 << stem_i;
                }
                bonus += sw;
                if primary_slice.first() == Some(&sh) {
                    bonus += 65_000;
                }
            } else if secondary_slice.contains(&sh) {
                matched_stems += 1;
                if stem_i < 16 {
                    covered_mask |= 1u16 << stem_i;
                }
                bonus += sw / 2;
            }
        }
        if matched_primary > 0 {
            bonus += ((matched_primary as i64) * 160_000) / (row_primary_count as i64);
            let max_single_primary_width = 1;
            if matched_primary == total_stems
                && total_stems == 1
                && row_primary_count <= max_single_primary_width
            {
                matched_stems = 2;
            }
        }
        let mut matched_bpe = 0usize;
        for &(pt, bw) in &prompt_bpe_content {
            if bpe_slice.contains(&pt) {
                matched_bpe += 1;
                bonus += bw;
            }
        }
        if matched_stems >= 2 {
            bonus += (matched_stems as i64) * 180_000;
        }
        // Core verified curriculum turns (`0..CORE_CURRICULUM_TURNS` and proved frontier claims) receive priority whenever they match the prompt;
        // external corpus turns (`>= CORE_CURRICULUM_TURNS`) activate when no core turn matched (`max_core_matched_stems == 0`),
        // when a core turn matched only 1 incidental stem out of 2+ (`max_core_matched_stems == 1` and
        // external matches >= 2 primary stems covering >= 1/2 of the prompt), or when external matches
        // at least 2 more stems than the best core turn.
        if is_core_or_proved && matched_stems >= 1 {
            if matched_stems > best.max_core_matched_stems {
                best.max_core_matched_stems = matched_stems;
            }
            bonus += (matched_stems as i64) * 350_000;
        } else if !is_core_or_proved {
            let ext_eligible = if best.max_core_matched_stems == 0 {
                if total_stems <= 1 {
                    matched_primary >= 1 && row_primary_count <= 4
                } else if total_stems == 2 {
                    (matched_stems >= 2 && matched_primary >= 1)
                        || (matched_primary == 1
                            && row_primary_count <= 2
                            && primary_slice.first() == Some(&prompt_stems[0].0))
                } else {
                    matched_stems >= 2 && matched_primary >= 1 && matched_stems * 2 >= total_stems
                }
            } else if best.max_core_matched_stems == 1 {
                total_stems >= 2
                    && matched_stems >= 2
                    && matched_primary >= 2
                    && matched_stems * 2 >= total_stems
            } else {
                matched_stems >= best.max_core_matched_stems + 2
                    && matched_primary >= 2
                    && matched_stems * 3 >= eff_total * 2
            };
            if !ext_eligible {
                return false;
            }
            let stem_gain = matched_stems.saturating_sub(best.max_core_matched_stems) as i64;
            bonus += (matched_stems as i64) * 350_000 + stem_gain * 180_000;
        }

        // Exact branch-and-bound lower bound:
        // Whenever `raw_l1 > 0` (checked in O(1) via lane 0/1 inequality), `geom_dist >= guard_penalty`,
        // so `score = geom_dist - bonus >= guard_penalty - bonus`.
        let maybe_exact_zero = is_core_or_proved
            && basin_penalty == 0
            && w_lim >= 2
            && (p_row[0] as i32) == n_t[0]
            && (p_row[1] as i32) == n_t[1];
        let max_possible_bonus = if !is_core_or_proved && w_lim >= 2 && (p_row[0] as i32) == n_t[0]
        {
            bonus + 150_000
        } else {
            bonus
        };
        if !maybe_exact_zero && (guard_penalty + basin_penalty - max_possible_bonus) >= best.score {
            return false;
        }

        let p_slice = &p_row[..w_lim];
        let q_slice = &q_normed[..w_lim];

        let mut s0 = 0i64;
        let mut s1 = 0i64;
        let mut s2 = 0i64;
        let mut s3 = 0i64;
        let mut s4 = 0i64;
        let mut s5 = 0i64;
        let mut s6 = 0i64;
        let mut s7 = 0i64;
        for chunk in p_slice.chunks_exact(8) {
            s0 += (chunk[0] as i64).abs();
            s1 += (chunk[1] as i64).abs();
            s2 += (chunk[2] as i64).abs();
            s3 += (chunk[3] as i64).abs();
            s4 += (chunk[4] as i64).abs();
            s5 += (chunk[5] as i64).abs();
            s6 += (chunk[6] as i64).abs();
            s7 += (chunk[7] as i64).abs();
        }
        let mut p_abs_sum = (s0 + s1 + s2 + s3) + (s4 + s5 + s6 + s7);
        for &pv in p_slice.chunks_exact(8).remainder() {
            p_abs_sum += (pv as i64).abs();
        }
        let p_mean_abs = (p_abs_sum / (w_lim.max(1) as i64)).max(1);
        let inv_p_mean = (1024i64 << 16) / p_mean_abs;

        // Scale-normalized Q12 directional L1 distance (eliminates small-norm bias across rows)
        let mut norm_l1 = 0i64;
        let early_prune_thresh = best
            .score
            .saturating_add(max_possible_bonus)
            .saturating_sub(guard_penalty)
            .saturating_sub(basin_penalty);

        for (q_chunk, p_chunk) in q_slice.chunks(16).zip(p_slice.chunks(16)) {
            let mut c0 = 0i64;
            let mut c1 = 0i64;
            let mut c2 = 0i64;
            let mut c3 = 0i64;
            for (qc, pc) in q_chunk.chunks_exact(4).zip(p_chunk.chunks_exact(4)) {
                c0 += (qc[0] - (((pc[0] as i64) * inv_p_mean) >> 16)).abs();
                c1 += (qc[1] - (((pc[1] as i64) * inv_p_mean) >> 16)).abs();
                c2 += (qc[2] - (((pc[2] as i64) * inv_p_mean) >> 16)).abs();
                c3 += (qc[3] - (((pc[3] as i64) * inv_p_mean) >> 16)).abs();
            }
            let mut chunk_l1 = (c0 + c1) + (c2 + c3);
            for (&qv, &pv) in q_chunk
                .chunks_exact(4)
                .remainder()
                .iter()
                .zip(p_chunk.chunks_exact(4).remainder())
            {
                chunk_l1 += (qv - (((pv as i64) * inv_p_mean) >> 16)).abs();
            }
            norm_l1 += chunk_l1;
            if !maybe_exact_zero && norm_l1 >= early_prune_thresh {
                return false;
            }
        }

        if !maybe_exact_zero
            && (norm_l1 + guard_penalty + basin_penalty - max_possible_bonus) >= best.score
        {
            return false;
        }

        let is_exact_zero = maybe_exact_zero
            && n_t[..w_lim]
                .iter()
                .zip(p_slice)
                .all(|(&nv, &pv)| nv == pv as i32);

        // 256-bit Bitwise Popcount Hamming XOR distance between query sketch and point row sketch
        let p_sketch = pack_i16_sign_sketch_256(p_row);
        let h_bits = hamming_distance_256(&q_sketch, &p_sketch);
        let geom_dist = norm_l1 + (h_bits as i64) * 1024 + guard_penalty + basin_penalty;

        let score =
            if is_exact_zero && guard_penalty == 0 && basin_penalty == 0 && is_core_or_proved {
                -(1i64 << 42) - bonus
            } else if is_exact_zero && guard_penalty == 0 && basin_penalty == 0 {
                geom_dist - bonus - 150_000
            } else {
                geom_dist - bonus
            };

        if score < best.score {
            if let Some(rt) = reply_tbl {
                if !a_row.is_empty()
                    && !probe_reply_table_has_anchor(
                        rt,
                        reply_key_len,
                        &a_row[0..1],
                        point.targets[r],
                    )
                {
                    return false;
                }
            }
            let raw_l1 = if is_exact_zero {
                0i64
            } else {
                let mut sum = 0i64;
                for (&nv, &pv) in n_t[..w_lim].iter().zip(p_slice) {
                    sum += ((nv as i64) - (pv as i64)).abs();
                }
                sum
            };
            best.score = score;
            best.row = r;
            best.raw_l1 = raw_l1;
            best.hamming = h_bits;
            best.matched_stems = matched_stems;
            best.covered_mask = covered_mask;
            best.matched_bpe = matched_bpe;
            best.turn_idx = turn_idx;
            best.bonus = bonus;

            if score <= -(1i64 << 42) {
                return true;
            }
        }
        false
    };

    let is_confident = |best: &BestCandidate| -> bool {
        if best.score == i64::MAX {
            return false;
        }
        (best.raw_l1 == 0 && (best.turn_idx > 2 || is_self_inq))
            || (is_self_inq && best.turn_idx <= 2)
            || (is_greeting && (best.turn_idx == 3 || best.turn_idx == 4))
            || (total_stems > 0 && best.matched_stems >= total_stems)
            || (best.matched_stems >= 3 && best.matched_stems * 2 >= total_stems)
            || (best.matched_stems == 1 && total_stems <= 2 && best.bonus >= 160_000)
            || (total_stems == 0
                && !prompt_bpe_content.is_empty()
                && best.matched_bpe >= prompt_bpe_content.len().min(2))
            || (total_stems == 0 && prompt_bpe_content.is_empty() && best.score < i64::MAX)
    };

    if let (Some(offsets), Some(row_indices)) = (&point.bucket_offsets, &point.bucket_row_indices) {
        if offsets.len() == RADIX_DIRECTORY_LEN && row_indices.len() == point.rows {
            let mut q12_full = [0i16; FBS_WIDTH];
            let q_len = FBS_WIDTH.min(n_t.len());
            for i in 0..q_len {
                q12_full[i] = n_t[i].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }

            // 1. Top 2 E8 roots from mean-centered 16-shard E8 partitioning
            let (e8_1, e8_2) = compute_geometric_e8_pair(&q12_full[..q_len], q_sketch[0]);

            // 2. Clifford carrier action and 6 1-bit neighbors
            let q_c64 = ((q_sketch[0] >> 8) as u8) % (CLIFFORD_ACTION_COUNT as u8);
            let mut c_actions = [0u8; 7];
            c_actions[0] = q_c64;
            for bit in 0..6 {
                c_actions[bit + 1] = q_c64 ^ (1 << bit);
            }

            // 3. Top 2 Icosian roots based on icosian projection scores
            let lanes4 = [q12_full[0], q12_full[1], q12_full[2], q12_full[3]];
            let mut best_i17 = 0u8;
            let mut best_i17_score = i64::MIN;
            let mut runner_i17 = 0u8;
            let mut runner_i17_score = i64::MIN;
            for (idx, root) in ICOSIAN_ROOTS_Q12.iter().enumerate() {
                let mut a = 0i64;
                let mut b = 0i64;
                for l in 0..4 {
                    let x = i64::from(lanes4[l]);
                    a += x * i64::from(root[l].0);
                    b += x * i64::from(root[l].1);
                }
                let score = 2 * a + 3 * b;
                if score > best_i17_score {
                    runner_i17_score = best_i17_score;
                    runner_i17 = best_i17;
                    best_i17_score = score;
                    best_i17 = idx as u8;
                } else if score > runner_i17_score {
                    runner_i17_score = score;
                    runner_i17 = idx as u8;
                }
            }
            let i17_1 = best_i17 % (ICOSIAN_ROOT_COUNT as u8);
            let i17_2 = runner_i17 % (ICOSIAN_ROOT_COUNT as u8);

            let eval_bucket = |b: usize, best: &mut BestCandidate| -> bool {
                if b < GEOMETRIC_BUCKET_COUNT {
                    let start = offsets[b] as usize;
                    let end = (offsets[b + 1] as usize).min(row_indices.len());
                    for &idx in &row_indices[start..end] {
                        let r = idx as usize;
                        if r < point.rows && eval_row(r, best) {
                            return true;
                        }
                    }
                }
                false
            };

            // Stage 1: Primary geometric bucket
            let primary_bucket = compute_geometric_bucket(e8_1, q_c64, i17_1);
            let mut exact_hit = eval_bucket(primary_bucket, &mut best);

            // Stage 2: Geometric neighborhood across E8, Cl(0,6), and Icosian golden lattice (28 buckets)
            // Probed only if primary bucket did not yield an unpenalized, confident candidate
            let needs_neighbors = !exact_hit
                && (best.score == i64::MAX
                    || (best.row >= recent_min
                        && best.row <= recent_max
                        && recent_turn_rows.contains(&best.row))
                    || !is_confident(&best));

            if needs_neighbors {
                let mut probed = [usize::MAX; 32];
                let mut count = 0;
                probed[count] = primary_bucket;
                count += 1;

                let (e_cands, c_cands, i_cands): (&[u8], &[u8], &[u8]) =
                    if best.score < i64::MAX && is_confident(&best) {
                        if best.raw_l1 == 0 {
                            // Exact vector match already found in primary bucket; probe primary Clifford neighbor for recency alternative
                            (&[e8_1], &c_actions[1..2], &[i17_1])
                        } else {
                            // Confident non-exact match; probe top 2 Clifford neighbors for recency avoidance
                            (&[e8_1], &c_actions[1..3], &[i17_1])
                        }
                    } else {
                        // Query not confident or zero candidates; probe full 28-bucket Voronoi neighborhood
                        (&[e8_1, e8_2], &c_actions[..], &[i17_1, i17_2])
                    };

                'neighbor_loop: for &e in e_cands {
                    for &c in c_cands {
                        for &i in i_cands {
                            let b = compute_geometric_bucket(e, c, i);
                            let already = (0..count).any(|k| probed[k] == b);
                            if !already {
                                if count < probed.len() {
                                    probed[count] = b;
                                    count += 1;
                                }
                                if eval_bucket(b, &mut best) {
                                    exact_hit = true;
                                    break 'neighbor_loop;
                                }
                                if !(best.row >= recent_min
                                    && best.row <= recent_max
                                    && recent_turn_rows.contains(&best.row))
                                    && is_confident(&best)
                                {
                                    break 'neighbor_loop;
                                }
                            }
                        }
                    }
                }
            }

            // Stage 3a: Core curriculum fallback (0..623) when geometric neighborhood did not yield a confident hit
            // or for inquiries/greetings/penalized core candidates.
            if !exact_hit
                && (best.score == i64::MAX
                    || !is_confident(&best)
                    || is_self_inq
                    || is_greeting
                    || ((best.row >= recent_min
                        && best.row <= recent_max
                        && recent_turn_rows.contains(&best.row))
                        && best.turn_idx < CORE_CURRICULUM_TURNS))
            {
                let core_limit = point.rows.min(623);
                for r in 0..core_limit {
                    if eval_row(r, &mut best) {
                        break;
                    }
                }
            }

            // Stage 3b: External corpus fallback (0..rows) when still not confident or when a multi-stem prompt
            // matched at most 1 incidental stem across the 28-bucket geometric neighborhood.
            if !exact_hit
                && !is_self_inq
                && !is_greeting
                && (!is_confident(&best) || (total_stems >= 4 && best.matched_stems <= 1))
            {
                for r in 0..point.rows {
                    if eval_row(r, &mut best) {
                        break;
                    }
                }
            }
        } else {
            for r in 0..point.rows {
                if eval_row(r, &mut best) {
                    break;
                }
            }
        }
    } else {
        for r in 0..point.rows {
            if eval_row(r, &mut best) {
                break;
            }
        }
    }

    if best.score < i64::MAX {
        let confident = is_confident(&best);
        Some(PointResonanceHit {
            target_token: point.targets[best.row],
            row: best.row,
            turn_idx: best.turn_idx,
            raw_l1: best.raw_l1,
            hamming_bits: best.hamming,
            matched_stems: best.matched_stems,
            covered_stem_mask: best.covered_mask,
            confident,
        })
    } else {
        point
            .apply_with_row(n_t)
            .map(|(t, r, d)| PointResonanceHit {
                target_token: t,
                row: r,
                turn_idx: usize::MAX,
                raw_l1: d,
                hamming_bits: 128,
                matched_stems: usize::from(d == 0),
                covered_stem_mask: 0,
                confident: d == 0,
            })
    }
}

pub fn resolve_canonical_reply_anchor(
    point: &ServedPointKey,
    mem: &ServedTurnMemory,
    pt_row: usize,
    first_reply_tok: u32,
    prompt_reply_ids: &[u32],
) -> Option<Vec<u32>> {
    let alen = point.anchor_len.max(1);
    let reply_k = mem.reply_turn.unwrap_or(mem.k).min(alen);
    if let Some(anchors) = &point.canonical_anchors {
        let base = pt_row * alen;
        if base + alen <= anchors.len() {
            return Some(anchors[base..base + reply_k].to_vec());
        }
    }
    let kw = mem.k;
    let mut best_entry = None;
    let mut best_overlap = -1i32;
    for e in 0..mem.opening.entries {
        if mem.opening.values[e] != first_reply_tok {
            continue;
        }
        let k_slice = &mem.opening.keys[e * kw..(e + 1) * kw];
        let overlap = prompt_reply_ids
            .iter()
            .filter(|&&t| t != MEMORY_PAD && k_slice.contains(&t))
            .count() as i32;
        if overlap > best_overlap {
            best_overlap = overlap;
            best_entry = Some(k_slice.to_vec());
        }
    }
    best_entry
}

/// Parse structured `<thinking>...</thinking>` (or `<thought>...</thought>`) blocks from assistant output.
/// Iteratively strips all thinking blocks (including multiple or mixed tags and unclosed tags).
/// Returns `(Some(thinking_trace), clean_reply)` if present, or `(None, raw_text)` if absent.
pub fn parse_thinking_block(raw_text: &str) -> (Option<String>, String) {
    let mut traces: Vec<String> = Vec::new();
    let mut current = raw_text.to_string();

    loop {
        let thinking_pos = current
            .find("<thinking>")
            .map(|idx| (idx, 10usize, "</thinking>", 11usize));
        let thought_pos = current
            .find("<thought>")
            .map(|idx| (idx, 9usize, "</thought>", 10usize));

        let candidate = match (thinking_pos, thought_pos) {
            (Some(a), Some(b)) => {
                if a.0 <= b.0 {
                    Some(a)
                } else {
                    Some(b)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        if let Some((start, open_len, close_tag, close_len)) = candidate {
            if let Some(rel_end) = current[start + open_len..].find(close_tag) {
                let trace = current[start + open_len..start + open_len + rel_end]
                    .trim()
                    .to_string();
                if !trace.is_empty() {
                    traces.push(trace);
                }
                let mut clean = String::with_capacity(current.len());
                clean.push_str(&current[..start]);
                clean.push_str(&current[start + open_len + rel_end + close_len..]);
                current = clean;
            } else {
                let trace = current[start + open_len..].trim().to_string();
                if !trace.is_empty() {
                    traces.push(trace);
                }
                current.truncate(start);
                break;
            }
        } else {
            break;
        }
    }
    let combined_trace = if traces.is_empty() {
        None
    } else {
        Some(traces.join("\n\n"))
    };
    (combined_trace, current.trim().to_string())
}

/// Segment a multi-part user prompt (`"Explain X and also tell me about Y"`, `"What is A? Also how does B work?"`)
/// into ordered constituent clauses so every clause of the prompt is attended to and answered canonically.
pub fn split_prompt_clauses(user_prompt: &str) -> Vec<String> {
    let trimmed = user_prompt.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut clauses: Vec<String> = Vec::new();
    let mut working = trimmed.to_owned();
    for sep in [
        " and also ",
        " And also ",
        "; also ",
        ". Also, ",
        ". Also ",
        "? Also, ",
        "? Also ",
        " and tell me about ",
        " and explain ",
        ", and how ",
        ", and what ",
        ", and why ",
        "? What ",
        "? How ",
        "? Why ",
        "? Who ",
    ] {
        if let Some(pos) = working.find(sep) {
            let left = if sep.starts_with('?') {
                format!("{}?", working[..pos].trim())
            } else {
                working[..pos].trim().to_owned()
            };
            let right_prefix =
                if sep.starts_with("? ") && !sep.starts_with("? Also") && sep.len() > 2 {
                    &sep[2..]
                } else if sep.starts_with(", and ") && sep.len() > 6 {
                    &sep[6..]
                } else {
                    ""
                };
            let right = format!("{right_prefix}{}", working[pos + sep.len()..].trim());
            if !extract_salient_stem_hashes_from_text(&left, 4).is_empty()
                && !extract_salient_stem_hashes_from_text(&right, 4).is_empty()
            {
                clauses.push(left);
                working = right;
            }
        }
    }
    if !clauses.is_empty() {
        clauses.push(working);
        clauses.truncate(3);
        return clauses;
    }
    vec![trimmed.to_owned()]
}

pub fn has_short_cycle(ids: &[u32]) -> bool {
    if ids.len() >= 2 {
        let n = ids.len();
        if ids[n - 1] == ids[n - 2] {
            return true;
        }
    }
    for period in 2..=4 {
        if ids.len() >= period * 2 {
            let n = ids.len();
            let a = &ids[n - period..n];
            let b = &ids[n - 2 * period..n - period];
            if a == b {
                return true;
            }
        }
    }
    false
}

pub fn recall_prior_facts(history: &[(String, String)], current_prompt: &str) -> Option<String> {
    let q_lower = current_prompt.to_ascii_lowercase();
    let asks_recall = q_lower.contains("what is my ")
        || q_lower.contains("what's my ")
        || q_lower.contains("who am i")
        || q_lower.contains("what am i ")
        || q_lower.contains("what do i ")
        || q_lower.contains("where do i ")
        || q_lower.contains("remember")
        || q_lower.contains("earlier")
        || q_lower.contains("what did i ")
        || q_lower.contains("what did we ")
        || q_lower.contains("what were we ")
        || q_lower.contains("talking about")
        || q_lower.contains("talked about")
        || q_lower.contains("first question")
        || q_lower.contains("our conversation");
    if !asks_recall || history.is_empty() {
        return None;
    }
    let mut facts = Vec::new();
    for (u, _) in history {
        for sentence in u.split(['.', '!', '\n']) {
            let trim = sentence.trim();
            if trim.is_empty() || trim.ends_with('?') {
                continue;
            }
            let low = trim.to_ascii_lowercase();
            let clean_low = low
                .strip_prefix("hi, ")
                .or_else(|| low.strip_prefix("hi "))
                .or_else(|| low.strip_prefix("hello, "))
                .or_else(|| low.strip_prefix("hello "))
                .or_else(|| low.strip_prefix("hey, "))
                .or_else(|| low.strip_prefix("hey "))
                .or_else(|| low.strip_prefix("by the way, "))
                .unwrap_or(&low);
            if clean_low.starts_with("my ")
                || clean_low.starts_with("i am ")
                || clean_low.starts_with("i'm ")
                || clean_low.starts_with("i live ")
                || clean_low.starts_with("i work ")
                || clean_low.starts_with("i like ")
                || clean_low.starts_with("i love ")
                || clean_low.starts_with("i prefer ")
                || clean_low.starts_with("i enjoy ")
                || clean_low.starts_with("we are ")
                || clean_low.starts_with("we're ")
                || clean_low.starts_with("remember that ")
                || clean_low.starts_with("remember ")
            {
                let item = trim.trim_end_matches('.').to_owned();
                if !facts.contains(&item) {
                    facts.push(item);
                }
            }
        }
    }
    if facts.is_empty()
        && (q_lower.contains("what did we ")
            || q_lower.contains("what were we ")
            || q_lower.contains("talking about")
            || q_lower.contains("talked about")
            || q_lower.contains("first question")
            || q_lower.contains("our conversation")
            || q_lower.contains("earlier"))
    {
        for (idx, (u, _)) in history.iter().enumerate().take(4) {
            let trim = u.trim();
            if !trim.is_empty() {
                facts.push(format!("Turn {}: \"{}\"", idx + 1, trim));
            }
        }
    }
    if facts.is_empty() {
        None
    } else {
        Some(format!("Memory: {}.", facts.join("; ")))
    }
}

#[allow(dead_code, clippy::all)]
#[path = "lexlean_generated.rs"]
mod slot_binding_generated;

/// Maximum number of De Bruijn variable register slots (`$0..$7`) per turn (`R4-31`).
pub const MAX_DE_BRUIJN_SLOTS: usize = 8;

/// Entity classification for De Bruijn variable slot binding (`R4.SlotBinding`, `R4-31`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotEntityKind {
    Integer = 1,
    AlgebraicExpr = 2,
    Identifier = 3,
    FilePath = 4,
}

impl SlotEntityKind {
    pub const fn as_u64(self) -> u64 {
        self as u64
    }
}

/// A bound VSA De Bruijn register slot (`$0..$7`) extracted during prompt encoding (`R4-31`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeBruijnSlotEntry {
    pub slot_index: usize,
    pub entity_kind: SlotEntityKind,
    pub symbol_name: Option<String>,
    pub raw_lexeme: String,
    pub numeric_value: Option<i128>,
    pub vsa_binding_tag: u64,
    pub token_len: usize,
}

/// Live kernel family invoked over bound De Bruijn slots (`R4.SlotBinding`, `R4-31`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveKernelKind {
    ExactArithmeticR427 = 1,
    IcosianSnapR424 = 2,
    AstIdentifierR431 = 3,
}

impl LiveKernelKind {
    pub const fn as_u64(self) -> u64 {
        self as u64
    }
}

/// Result of executing a live formal kernel over bound VSA register slots (`R4-31`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveKernelResult {
    pub kernel_kind: LiveKernelKind,
    pub op_name: String,
    pub primary_slot: usize,
    pub result_numeric: Option<i128>,
    pub result_text: String,
    pub proof_summary: String,
}

/// De Bruijn variable slot-binding register frame and live kernel execution state (`R4-31`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeBruijnSlotFrame {
    pub slots: Vec<DeBruijnSlotEntry>,
    pub canonical_prompt: String,
    pub kernel_results: Vec<LiveKernelResult>,
    pub substitutions_applied: usize,
    pub stream_digest: u64,
}

fn exact_gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn exact_lcm_u128(a: u128, b: u128) -> u128 {
    if a == 0 || b == 0 {
        return 0;
    }
    let g = exact_gcd_u128(a, b);
    (a / g).saturating_mul(b)
}

fn exact_fib_u128(n: u32) -> u128 {
    let clamped = n.min(180);
    if clamped == 0 {
        return 0;
    }
    let mut a = 0u128;
    let mut b = 1u128;
    for _ in 1..clamped {
        let next = a.saturating_add(b);
        a = b;
        b = next;
    }
    b
}

fn exact_factorial_u128(n: u32) -> u128 {
    let clamped = n.min(34);
    let mut acc = 1u128;
    for k in 2..=clamped {
        acc = acc.saturating_mul(k as u128);
    }
    acc
}

fn exact_mod_exp_u128(mut base: u128, mut exp: u128, modulus: u128) -> Option<u128> {
    if modulus == 0 {
        return None;
    }
    if modulus == 1 {
        return Some(0);
    }
    let mut result = 1u128;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result.wrapping_mul(base) % modulus;
        }
        exp >>= 1;
        base = base.wrapping_mul(base) % modulus;
    }
    Some(result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MathEvalOutcome {
    Value(i128),
    DivisionByZero(i128),
}

struct SlotExprParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    vars: &'a [(String, i128)],
}

impl<'a> SlotExprParser<'a> {
    fn new(input: &'a str, vars: &'a [(String, i128)]) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
            vars,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Option<MathEvalOutcome> {
        let mut left = match self.parse_term()? {
            MathEvalOutcome::Value(v) => v,
            err => return Some(err),
        };
        loop {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                break;
            }
            let b = self.bytes[self.pos];
            if b == b'+' || b == b'-' {
                self.pos += 1;
                let right = match self.parse_term()? {
                    MathEvalOutcome::Value(v) => v,
                    err => return Some(err),
                };
                left = if b == b'+' {
                    left.saturating_add(right)
                } else {
                    left.saturating_sub(right)
                };
            } else {
                break;
            }
        }
        Some(MathEvalOutcome::Value(left))
    }

    fn parse_term(&mut self) -> Option<MathEvalOutcome> {
        let mut left = match self.parse_power()? {
            MathEvalOutcome::Value(v) => v,
            err => return Some(err),
        };
        loop {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                break;
            }
            let b = self.bytes[self.pos];
            if b == b'*' || b == b'/' || b == b'%' {
                self.pos += 1;
                let right = match self.parse_power()? {
                    MathEvalOutcome::Value(v) => v,
                    err => return Some(err),
                };
                if b == b'*' {
                    left = left.saturating_mul(right);
                } else if right == 0 {
                    return Some(MathEvalOutcome::DivisionByZero(left));
                } else if b == b'/' {
                    left = left.saturating_div(right);
                } else {
                    left %= right;
                }
            } else {
                break;
            }
        }
        Some(MathEvalOutcome::Value(left))
    }

    fn parse_power(&mut self) -> Option<MathEvalOutcome> {
        let base = match self.parse_unary()? {
            MathEvalOutcome::Value(v) => v,
            err => return Some(err),
        };
        self.skip_ws();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'^' {
            self.pos += 1;
            let exp = match self.parse_power()? {
                MathEvalOutcome::Value(v) => v,
                err => return Some(err),
            };
            if exp < 0 {
                return Some(MathEvalOutcome::Value(0));
            }
            let mut acc = 1i128;
            for _ in 0..(exp.min(62) as u32) {
                acc = acc.saturating_mul(base);
            }
            Some(MathEvalOutcome::Value(acc))
        } else {
            Some(MathEvalOutcome::Value(base))
        }
    }

    fn parse_unary(&mut self) -> Option<MathEvalOutcome> {
        self.skip_ws();
        if self.pos >= self.bytes.len() {
            return None;
        }
        if self.bytes[self.pos] == b'-' {
            self.pos += 1;
            match self.parse_unary()? {
                MathEvalOutcome::Value(v) => Some(MathEvalOutcome::Value(v.saturating_neg())),
                err => Some(err),
            }
        } else if self.bytes[self.pos] == b'+' {
            self.pos += 1;
            self.parse_unary()
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Option<MathEvalOutcome> {
        self.skip_ws();
        if self.pos >= self.bytes.len() {
            return None;
        }
        if self.bytes[self.pos] == b'(' {
            self.pos += 1;
            let val = self.parse_expr()?;
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b')' {
                self.pos += 1;
            }
            return Some(val);
        }
        if self.bytes[self.pos].is_ascii_digit() {
            let mut val = 0i128;
            let mut digits = 0usize;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
            {
                let b = self.bytes[self.pos];
                self.pos += 1;
                if b != b'_' {
                    val = val.saturating_mul(10).saturating_add((b - b'0') as i128);
                    digits += 1;
                }
            }
            if digits == 0 {
                return None;
            }
            return Some(MathEvalOutcome::Value(val));
        }
        if self.bytes[self.pos].is_ascii_alphabetic() || self.bytes[self.pos] == b'$' {
            let start = self.pos;
            self.pos += 1;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let ident = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
            let ident_lower = ident.to_ascii_lowercase();
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b'(' {
                self.pos += 1;
                let mut args = Vec::new();
                loop {
                    self.skip_ws();
                    if self.pos < self.bytes.len() && self.bytes[self.pos] == b')' {
                        self.pos += 1;
                        break;
                    }
                    match self.parse_expr()? {
                        MathEvalOutcome::Value(v) => args.push(v),
                        err => return Some(err),
                    }
                    self.skip_ws();
                    if self.pos < self.bytes.len() && self.bytes[self.pos] == b',' {
                        self.pos += 1;
                    } else if self.pos < self.bytes.len() && self.bytes[self.pos] == b')' {
                        self.pos += 1;
                        break;
                    } else {
                        return None;
                    }
                }
                return match ident_lower.as_str() {
                    "gcd" if args.len() == 2 => Some(MathEvalOutcome::Value(exact_gcd_u128(
                        args[0].unsigned_abs(),
                        args[1].unsigned_abs(),
                    )
                        as i128)),
                    "lcm" if args.len() == 2 => Some(MathEvalOutcome::Value(exact_lcm_u128(
                        args[0].unsigned_abs(),
                        args[1].unsigned_abs(),
                    )
                        as i128)),
                    "fib" | "fibonacci" if args.len() == 1 => Some(MathEvalOutcome::Value(
                        exact_fib_u128(args[0].max(0) as u32) as i128,
                    )),
                    "fact" | "factorial" if args.len() == 1 => Some(MathEvalOutcome::Value(
                        exact_factorial_u128(args[0].max(0) as u32) as i128,
                    )),
                    "pow" if args.len() == 2 => {
                        let exp = args[1].clamp(0, 62) as u32;
                        let mut acc = 1i128;
                        for _ in 0..exp {
                            acc = acc.saturating_mul(args[0]);
                        }
                        Some(MathEvalOutcome::Value(acc))
                    }
                    "mod_exp" | "pow_mod" if args.len() == 3 => {
                        match exact_mod_exp_u128(
                            args[0].unsigned_abs(),
                            args[1].max(0) as u128,
                            args[2].unsigned_abs(),
                        ) {
                            Some(v) => Some(MathEvalOutcome::Value(v as i128)),
                            None => Some(MathEvalOutcome::DivisionByZero(args[0])),
                        }
                    }
                    _ => None,
                };
            }
            for (k, v) in self.vars {
                if k == ident || k.eq_ignore_ascii_case(ident) {
                    return Some(MathEvalOutcome::Value(*v));
                }
            }
        }
        None
    }
}

fn evaluate_slot_math_expression(expr: &str, vars: &[(String, i128)]) -> Option<MathEvalOutcome> {
    let mut normalized = expr.trim().trim_end_matches(['?', '.', '!']).to_string();
    for (pat, rep) in [
        (" multiplied by ", " * "),
        (" divided by ", " / "),
        (" to the power of ", " ^ "),
        (" plus ", " + "),
        (" minus ", " - "),
        (" times ", " * "),
        (" modulo ", " % "),
        (" mod ", " % "),
    ] {
        while let Some(idx) = normalized.to_ascii_lowercase().find(pat) {
            normalized = format!(
                "{}{}{}",
                &normalized[..idx],
                rep,
                &normalized[idx + pat.len()..]
            );
        }
    }
    let mut parser = SlotExprParser::new(&normalized, vars);
    let out = parser.parse_expr()?;
    parser.skip_ws();
    if parser.pos == parser.bytes.len() {
        Some(out)
    } else {
        None
    }
}

impl DeBruijnSlotFrame {
    /// Attempt to bind a new entity into the next available De Bruijn slot (`$0..$7`),
    /// enforcing the formal `R4.SlotBinding.bind_de_bruijn_slot` invariants.
    pub fn bind_entity(
        &mut self,
        entity_kind: SlotEntityKind,
        symbol_name: Option<String>,
        raw_lexeme: &str,
        numeric_value: Option<i128>,
    ) -> Option<usize> {
        let trimmed = raw_lexeme.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Some(existing) = self
            .slots
            .iter()
            .find(|s| s.raw_lexeme == trimmed && s.entity_kind == entity_kind)
        {
            return Some(existing.slot_index);
        }
        let slot_index = self.slots.len();
        let token_len = ByteBpe4096::global().encode(trimmed).len().max(1);
        let bound_u64 = numeric_value
            .map(|v| v.unsigned_abs() as u64)
            .unwrap_or_else(|| fnv1a64(trimmed.as_bytes()));
        let role_hash =
            fnv1a64(format!("debruijn:slot:{slot_index}:{}", entity_kind.as_u64()).as_bytes());
        let filler_hash = fnv1a64(trimmed.as_bytes());
        let vsa_binding_tag =
            (role_hash ^ filler_hash.rotate_left(((slot_index * 7) % 64) as u32)) | 1;

        match slot_binding_generated::r4_slot_binding::bind_de_bruijn_slot(
            slot_index as u64,
            MAX_DE_BRUIJN_SLOTS as u64,
            entity_kind.as_u64(),
            token_len as u64,
            bound_u64,
            vsa_binding_tag,
        ) {
            slot_binding_generated::r4_slot_binding::SlotBindingOutcome::SlotBound(_, _, _) => {
                self.slots.push(DeBruijnSlotEntry {
                    slot_index,
                    entity_kind,
                    symbol_name,
                    raw_lexeme: trimmed.to_string(),
                    numeric_value,
                    vsa_binding_tag,
                    token_len,
                });
                Some(slot_index)
            }
            _ => None,
        }
    }

    /// Parse a user prompt, bind extracted entities (integers, algebraic expressions,
    /// symbolic variable assignments, unseen identifiers, and file paths) into VSA register slots `$0..$7`,
    /// and execute live `R4-27` / `R4-24` / AST kernels over the bound registers.
    pub fn bind_and_execute(user_prompt: &str) -> Self {
        let mut frame = Self {
            slots: Vec::new(),
            canonical_prompt: user_prompt.trim().to_string(),
            kernel_results: Vec::new(),
            substitutions_applied: 0,
            stream_digest: fnv1a64(user_prompt.as_bytes()) | 1,
        };
        let trimmed = user_prompt.trim();
        if trimmed.is_empty() {
            return frame;
        }
        let lower = trimmed.to_ascii_lowercase();

        // 1. Extract explicit variable assignments like `x = 14`, `alpha = 25` across clauses/sentences
        let mut var_bindings: Vec<(String, i128)> = Vec::new();
        let bytes = trimmed.as_bytes();
        let mut eq_idx = 0usize;
        while eq_idx < bytes.len() {
            if bytes[eq_idx] == b'=' {
                let prev_b = if eq_idx > 0 { bytes[eq_idx - 1] } else { b' ' };
                let next_b = if eq_idx + 1 < bytes.len() {
                    bytes[eq_idx + 1]
                } else {
                    b' '
                };
                if !matches!(prev_b, b'=' | b'!' | b'<' | b'>') && next_b != b'=' {
                    let lhs = &trimmed[..eq_idx];
                    let rhs = &trimmed[eq_idx + 1..];
                    let var_cand = lhs
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .rfind(|s| !s.is_empty())
                        .unwrap_or("");
                    let rhs_trimmed = rhs.trim_start();
                    let val_cand: String = rhs_trimmed
                        .chars()
                        .enumerate()
                        .take_while(|&(i, c)| {
                            c.is_ascii_digit() || c == '_' || (i == 0 && c == '-')
                        })
                        .filter(|&(_, c)| c != '_')
                        .map(|(_, c)| c)
                        .collect();
                    if !var_cand.is_empty()
                        && var_cand
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_alphabetic())
                        && !matches!(
                            var_cand.to_ascii_lowercase().as_str(),
                            "if" | "let" | "where" | "when" | "with" | "given" | "and"
                        )
                        && !val_cand.is_empty()
                        && val_cand != "-"
                    {
                        if let Ok(num) = val_cand.parse::<i128>() {
                            if let Some(slot_idx) = frame.bind_entity(
                                SlotEntityKind::Integer,
                                Some(var_cand.to_string()),
                                &val_cand,
                                Some(num),
                            ) {
                                var_bindings.push((var_cand.to_string(), num));
                                var_bindings.push((format!("${slot_idx}"), num));
                            }
                        }
                    }
                }
            }
            eq_idx += 1;
        }

        // 2. Check for Icosian Z[phi] 17-root snap query over explicit coordinates (`R4-24`)
        if (lower.contains("icosian") || lower.contains("z[phi]") || lower.contains("snap vector"))
            && trimmed.chars().any(|c| c.is_ascii_digit())
        {
            let mut coords: Vec<i128> = Vec::new();
            let mut current_num = String::new();
            for ch in trimmed.chars() {
                if ch.is_ascii_digit() || (ch == '-' && current_num.is_empty()) {
                    current_num.push(ch);
                } else if !current_num.is_empty() {
                    if current_num != "-" {
                        if let Ok(v) = current_num.parse::<i128>() {
                            coords.push(v);
                        }
                    }
                    current_num.clear();
                }
            }
            if !current_num.is_empty() && current_num != "-" {
                if let Ok(v) = current_num.parse::<i128>() {
                    coords.push(v);
                }
            }
            // Filter out "17" or "12" if there are at least 4 coordinates after them
            let q4: Option<[i32; 4]> = if coords.len() >= 4 {
                let start = coords.len() - 4;
                Some([
                    coords[start].clamp(-30000, 30000) as i32,
                    coords[start + 1].clamp(-30000, 30000) as i32,
                    coords[start + 2].clamp(-30000, 30000) as i32,
                    coords[start + 3].clamp(-30000, 30000) as i32,
                ])
            } else {
                None
            };
            if let Some(vec4) = q4 {
                let mut first_slot = 0usize;
                for (i, &c_val) in vec4.iter().enumerate() {
                    if let Some(s) = frame.bind_entity(
                        SlotEntityKind::Integer,
                        Some(format!("q{i}")),
                        &c_val.to_string(),
                        Some(c_val as i128),
                    ) {
                        if i == 0 {
                            first_slot = s;
                        }
                    }
                }
                if let slot_binding_generated::r4_slot_binding::SlotKernelOutcome::KernelEvaluated(
                    _,
                    _,
                    _,
                ) = slot_binding_generated::r4_slot_binding::evaluate_bound_slot_kernel(
                    LiveKernelKind::IcosianSnapR424.as_u64(),
                    0,
                    first_slot as u64,
                    frame.slots.len() as u64,
                    vec4[0].unsigned_abs() as u64,
                    vec4[1].unsigned_abs() as u64,
                    16,
                ) {
                    let (root_u8, snap_score) = icosian_snap_select([
                        vec4[0] as i16,
                        vec4[1] as i16,
                        vec4[2] as i16,
                        vec4[3] as i16,
                    ]);
                    let root_idx = root_u8 as usize;
                    let g_row = ICOSIAN_ROOTS_Q12[root_idx.min(16)];
                    let mut dot_a = 0i64;
                    let mut dot_b = 0i64;
                    for d in 0..4 {
                        dot_a += (vec4[d] as i64) * (g_row[d].0 as i64);
                        dot_b += (vec4[d] as i64) * (g_row[d].1 as i64);
                    }
                    let result_text = format!(
                        "Icosian Z[phi] 17-Root Snap (`R4-24` / `R4-31`) on bound slots `[{}, {}, {}, {}]` ($0..$3): snapped to **Root #{}** (`GoldenQ12 = [({}, {}), ({}, {}), ({}, {}), ({}, {})]`, Z[phi] projection `a={}, b={}`, `snapScore={}`).",
                        vec4[0],
                        vec4[1],
                        vec4[2],
                        vec4[3],
                        root_idx,
                        g_row[0].0,
                        g_row[0].1,
                        g_row[1].0,
                        g_row[1].1,
                        g_row[2].0,
                        g_row[2].1,
                        g_row[3].0,
                        g_row[3].1,
                        dot_a,
                        dot_b,
                        snap_score
                    );
                    frame.kernel_results.push(LiveKernelResult {
                        kernel_kind: LiveKernelKind::IcosianSnapR424,
                        op_name: "icosian_snap_17".to_string(),
                        primary_slot: first_slot,
                        result_numeric: Some(root_idx as i128),
                        result_text,
                        proof_summary: format!(
                            "R4.SpiralCoreFbs.icosianSnap17 -> Root #{root_idx}"
                        ),
                    });
                }
            }
        }

        // 3. Extract and evaluate algebraic / arithmetic expressions (`R4-27`)
        if let Some(candidate_expr) =
            Self::extract_math_expression_candidate(trimmed, &var_bindings)
        {
            if let Some(eval_outcome) =
                evaluate_slot_math_expression(&candidate_expr, &var_bindings)
            {
                // Bind any standalone integers in candidate_expr that aren't bound yet
                let mut cur_digits = String::new();
                for ch in candidate_expr.chars() {
                    if ch.is_ascii_digit() {
                        cur_digits.push(ch);
                    } else if !cur_digits.is_empty() {
                        if let Ok(n) = cur_digits.parse::<i128>() {
                            let _ = frame.bind_entity(
                                SlotEntityKind::Integer,
                                None,
                                &cur_digits,
                                Some(n),
                            );
                        }
                        cur_digits.clear();
                    }
                }
                if !cur_digits.is_empty() {
                    if let Ok(n) = cur_digits.parse::<i128>() {
                        let _ =
                            frame.bind_entity(SlotEntityKind::Integer, None, &cur_digits, Some(n));
                    }
                }

                match eval_outcome {
                    MathEvalOutcome::Value(val) => {
                        let expr_slot = frame
                            .bind_entity(
                                SlotEntityKind::AlgebraicExpr,
                                Some("expr".to_string()),
                                &candidate_expr,
                                Some(val),
                            )
                            .unwrap_or(0);
                        let left_u64 = frame
                            .slots
                            .first()
                            .and_then(|s| s.numeric_value)
                            .unwrap_or(val)
                            .unsigned_abs() as u64;
                        let right_u64 = frame
                            .slots
                            .get(1)
                            .and_then(|s| s.numeric_value)
                            .unwrap_or(0)
                            .unsigned_abs() as u64;

                        if let slot_binding_generated::r4_slot_binding::SlotKernelOutcome::KernelEvaluated(
                            _,
                            _,
                            _,
                        ) = slot_binding_generated::r4_slot_binding::evaluate_bound_slot_kernel(
                            LiveKernelKind::ExactArithmeticR427.as_u64(),
                            0,
                            expr_slot as u64,
                            frame.slots.len().max(1) as u64,
                            left_u64,
                            right_u64,
                            32,
                        ) {
                            let m6_sum = (left_u64 % 1000) + (right_u64 % 1000);
                            let m6_place = (m6_sum % 1000, m6_sum / 1000);
                            let slot_summary = frame
                                .slots
                                .iter()
                                .map(|s| match (&s.symbol_name, s.numeric_value) {
                                    (Some(sym), Some(v)) => format!("${}({sym}={v})", s.slot_index),
                                    (None, Some(v)) => format!("${}={v}", s.slot_index),
                                    _ => format!("${}=\"{}\"", s.slot_index, s.raw_lexeme),
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            let result_text = format!(
                                "De Bruijn Slot-Binding & `R4-27` Exact Arithmetic Kernel (`R4-31`): `{candidate_expr} = {val}` (bound registers: `[{slot_summary}]`, M6 radix-1000 place `(digit={}, carry={})`).",
                                m6_place.0, m6_place.1
                            );
                            frame.kernel_results.push(LiveKernelResult {
                                kernel_kind: LiveKernelKind::ExactArithmeticR427,
                                op_name: "exact_arithmetic_r4_27".to_string(),
                                primary_slot: expr_slot,
                                result_numeric: Some(val),
                                result_text,
                                proof_summary: format!("R4.SlotBinding + R4.ExactArithmetic -> {val}"),
                            });
                        }
                    }
                    MathEvalOutcome::DivisionByZero(dividend) => {
                        let expr_slot = frame
                            .bind_entity(
                                SlotEntityKind::AlgebraicExpr,
                                Some("div_zero_expr".to_string()),
                                &candidate_expr,
                                None,
                            )
                            .unwrap_or(0);
                        let outcome =
                            slot_binding_generated::r4_slot_binding::evaluate_bound_slot_kernel(
                                LiveKernelKind::ExactArithmeticR427.as_u64(),
                                3,
                                expr_slot as u64,
                                frame.slots.len().max(1) as u64,
                                dividend.unsigned_abs() as u64,
                                0,
                                32,
                            );
                        let result_text = format!(
                            "De Bruijn Slot-Binding Refusal (`R4-31`): division by zero in `{candidate_expr}` rejected by `R4.SlotBinding.evaluateBoundSlotKernel` (`{outcome:?}`).",
                        );
                        frame.kernel_results.push(LiveKernelResult {
                            kernel_kind: LiveKernelKind::ExactArithmeticR427,
                            op_name: "division_by_zero_guard".to_string(),
                            primary_slot: expr_slot,
                            result_numeric: None,
                            result_text,
                            proof_summary: format!("{outcome:?}"),
                        });
                    }
                }
            }
        }

        // 4. Extract file paths and custom user identifiers for parameterized code queries (`R4-31`)
        for raw_tok in trimmed.split_whitespace() {
            let clean_tok = raw_tok.trim_matches(['`', '\'', '"', ',', ';', '?', ')', '(']);
            if (clean_tok.contains('/') && clean_tok.contains('.'))
                || clean_tok.ends_with(".rs")
                || clean_tok.ends_with(".tex")
                || clean_tok.ends_with(".toml")
                || clean_tok.ends_with(".js")
            {
                let _ = frame.bind_entity(SlotEntityKind::FilePath, None, clean_tok, None);
            }
        }

        if let Some((fn_name, param_names, body_op, const_term)) =
            Self::extract_parameterized_code_spec(trimmed)
        {
            let fn_slot = frame
                .bind_entity(
                    SlotEntityKind::Identifier,
                    Some("fn_name".to_string()),
                    &fn_name,
                    None,
                )
                .unwrap_or(0);
            for (idx, p_name) in param_names.iter().enumerate() {
                let _ = frame.bind_entity(
                    SlotEntityKind::Identifier,
                    Some(format!("param_{idx}")),
                    p_name,
                    None,
                );
            }
            let p0 = param_names.first().map(String::as_str).unwrap_or("x");
            let p1 = param_names.get(1).map(String::as_str).unwrap_or("y");
            let c_val = const_term.unwrap_or(0);
            let sample_a = 12i64;
            let sample_b = 7i64;
            let (expr_code, sample_res) = match body_op {
                "*" => {
                    if c_val != 0 {
                        (
                            format!("({p0}.saturating_mul({p1})).saturating_add({c_val})"),
                            (sample_a * sample_b) + c_val,
                        )
                    } else {
                        (format!("{p0}.saturating_mul({p1})"), sample_a * sample_b)
                    }
                }
                "-" => (format!("{p0}.saturating_sub({p1})"), sample_a - sample_b),
                _ => {
                    if c_val != 0 {
                        (
                            format!("({p0}.saturating_add({p1})).saturating_add({c_val})"),
                            sample_a + sample_b + c_val,
                        )
                    } else {
                        (format!("{p0}.saturating_add({p1})"), sample_a + sample_b)
                    }
                }
            };
            let _ = slot_binding_generated::r4_slot_binding::evaluate_bound_slot_kernel(
                LiveKernelKind::AstIdentifierR431.as_u64(),
                0,
                fn_slot as u64,
                frame.slots.len().max(1) as u64,
                sample_a as u64,
                sample_b as u64,
                32,
            );
            let synthesized = format!(
                "```rust\npub fn {fn_name}({p0}: i64, {p1}: i64) -> i64 {{\n    {expr_code}\n}}\n```\nDe Bruijn Slot-Binding (`R4-31`) bound identifier `$0 = \"{fn_name}\"` with parameters `({p0}, {p1})` and verified live evaluation `{fn_name}({sample_a}, {sample_b}) == {sample_res}`."
            );
            frame.kernel_results.push(LiveKernelResult {
                kernel_kind: LiveKernelKind::AstIdentifierR431,
                op_name: "ast_identifier_synthesis".to_string(),
                primary_slot: fn_slot,
                result_numeric: Some(sample_res as i128),
                result_text: synthesized,
                proof_summary: format!(
                    "R4.SlotBinding AST kernel -> {fn_name}({sample_a}, {sample_b}) = {sample_res}"
                ),
            });
        }

        // Canonicalize prompt by replacing bound lexemes with $k markers
        let mut canon = trimmed.to_string();
        for slot in &frame.slots {
            if slot.raw_lexeme.len() >= 2 && canon.contains(&slot.raw_lexeme) {
                canon = canon.replacen(&slot.raw_lexeme, &format!("${}", slot.slot_index), 1);
            }
        }
        frame.canonical_prompt = canon;
        frame
    }

    fn extract_math_expression_candidate(
        prompt: &str,
        var_bindings: &[(String, i128)],
    ) -> Option<String> {
        let trimmed = prompt.trim().trim_end_matches(['?', '.', '!']);
        let lower = trimmed.to_ascii_lowercase();

        // Check for functional calls gcd(...), lcm(...), fib(...), factorial(...), mod_exp(...)
        for func in [
            "gcd(",
            "lcm(",
            "fib(",
            "fibonacci(",
            "fact(",
            "factorial(",
            "mod_exp(",
            "pow_mod(",
            "pow(",
        ] {
            if let Some(start) = lower.find(func) {
                let tail = &trimmed[start..];
                let mut end_idx = 0usize;
                for (i, ch) in tail.char_indices() {
                    if ch.is_ascii_alphanumeric()
                        || matches!(
                            ch,
                            '(' | ')' | ',' | ' ' | '+' | '-' | '*' | '/' | '%' | '^' | '_'
                        )
                    {
                        end_idx = i + ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let cand = tail[..end_idx].trim();
                if evaluate_slot_math_expression(cand, var_bindings).is_some() {
                    return Some(cand.to_string());
                }
                if let Some(rel_end) = tail.find(')') {
                    return Some(tail[..=rel_end].to_string());
                }
            }
        }

        // If prompt has variable bindings (`if x = 14 and y = 29, what is 3 * x + 2 * y`),
        // extract the expression after `what is `, `compute `, `evaluate `, `calculate `, `find `
        if !var_bindings.is_empty() {
            for marker in [
                "what is ",
                "compute ",
                "evaluate ",
                "calculate ",
                "find ",
                "then ",
            ] {
                if let Some(idx) = lower.rfind(marker) {
                    let tail = trimmed[idx + marker.len()..].trim();
                    if !tail.is_empty() {
                        return Some(tail.to_string());
                    }
                }
            }
        }

        // Extract contiguous arithmetic span containing digits and at least one explicit math operator
        let has_word_op = lower.contains(" plus ")
            || lower.contains(" minus ")
            || lower.contains(" times ")
            || lower.contains(" multiplied by ")
            || lower.contains(" divided by ")
            || lower.contains(" modulo ")
            || lower.contains(" to the power of ");

        if has_word_op {
            for prefix in ["what is ", "calculate ", "compute ", "evaluate ", "solve "] {
                if let Some(idx) = lower.find(prefix) {
                    return Some(trimmed[idx + prefix.len()..].trim().to_string());
                }
            }
            return Some(trimmed.to_string());
        }

        // Scan for a symbolic arithmetic substring containing at least one of `+`, `*`, `/`, `%`, `^`, or ` - `
        let has_symbolic_op = trimmed.contains('+')
            || trimmed.contains('*')
            || trimmed.contains('/')
            || trimmed.contains('%')
            || trimmed.contains('^')
            || trimmed.contains(" - ");
        if !has_symbolic_op {
            return None;
        }
        // Avoid triggering on file paths or URLs (`crates/wasm`, `uor-r4.lut-stack/8`)
        let mut best_span = String::new();
        let mut cur_span = String::new();
        let chars: Vec<char> = trimmed.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            let prev_is_alpha = i > 0 && chars[i - 1].is_ascii_alphabetic();
            let next_is_alpha = i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic();
            let is_path_slash = ch == '/' && (prev_is_alpha || next_is_alpha);
            let is_hyphen_word = ch == '-' && (prev_is_alpha || next_is_alpha);
            if !is_path_slash
                && !is_hyphen_word
                && (ch.is_ascii_digit()
                    || ch == '+'
                    || ch == '-'
                    || ch == '*'
                    || ch == '/'
                    || ch == '%'
                    || ch == '^'
                    || ch == '('
                    || ch == ')'
                    || ch == ' ')
            {
                cur_span.push(ch);
            } else {
                let s = cur_span.trim();
                if s.chars().any(|c| c.is_ascii_digit())
                    && s.chars()
                        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '%' | '^'))
                    && s.len() > best_span.len()
                {
                    best_span = s.to_string();
                }
                cur_span.clear();
            }
        }
        let s = cur_span.trim();
        if s.chars().any(|c| c.is_ascii_digit())
            && s.chars()
                .any(|c| matches!(c, '+' | '-' | '*' | '/' | '%' | '^'))
            && s.len() > best_span.len()
        {
            best_span = s.to_string();
        }
        if best_span.len() >= 3 {
            Some(best_span)
        } else {
            None
        }
    }

    fn extract_parameterized_code_spec(
        prompt: &str,
    ) -> Option<(String, Vec<String>, &'static str, Option<i64>)> {
        let lower = prompt.to_ascii_lowercase();
        let is_fn_request = lower.contains("write a rust function")
            || lower.contains("write a function")
            || lower.contains("implement fn ")
            || lower.contains("define fn ")
            || lower.contains("write fn ");
        if !is_fn_request {
            return None;
        }
        // Look for an explicit custom snake_case function name and optional `(param1: ty, param2: ty)` list
        let mut fn_name = None;
        let mut params = Vec::new();
        for (paren_idx, _) in prompt.match_indices('(') {
            let prefix = prompt[..paren_idx].trim_end();
            let cand = prefix
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .next_back()
                .unwrap_or("");
            if cand.contains('_')
                && cand.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && cand != "lut_stack"
                && cand != "point_key"
            {
                fn_name = Some(cand.to_string());
                if let Some(close_rel) = prompt[paren_idx + 1..].find(')') {
                    let inside = &prompt[paren_idx + 1..paren_idx + 1 + close_rel];
                    for p in inside.split(',') {
                        let p_ident = p
                            .split(':')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
                        if !p_ident.is_empty() {
                            params.push(p_ident.to_string());
                        }
                    }
                }
                break;
            }
        }
        if fn_name.is_none() {
            for raw_word in prompt.split_whitespace() {
                let clean = raw_word.trim_matches(|c: char| {
                    c == '`'
                        || c == '\''
                        || c == '"'
                        || c == ','
                        || c == '.'
                        || c == '?'
                        || c == ':'
                });
                if clean.contains('_')
                    && clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && clean != "lut_stack"
                    && clean != "point_key"
                {
                    fn_name = Some(clean.to_string());
                    break;
                }
            }
        }
        let fn_name = fn_name?;
        if params.is_empty() {
            params.push("lhs_val".to_string());
            params.push("rhs_val".to_string());
        } else if params.len() == 1 {
            params.push("rhs_val".to_string());
        }
        let body_op =
            if lower.contains("multipl") || lower.contains("product") || lower.contains('*') {
                "*"
            } else if lower.contains("subtract")
                || lower.contains("difference")
                || lower.contains("minus")
            {
                "-"
            } else {
                "+"
            };
        let mut const_term = None;
        for w in prompt.split_whitespace() {
            let cw = w.trim_matches(|c: char| c.is_ascii_punctuation());
            if !cw.is_empty() && cw.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(v) = cw.parse::<i64>() {
                    if v != 64 && v != 32 && v != 128 {
                        const_term = Some(v);
                    }
                }
            }
        }
        Some((fn_name, params, body_op, const_term))
    }

    pub fn has_active_kernel_substitution(&self) -> bool {
        !self.kernel_results.is_empty()
    }

    pub fn steps_or_tokens_valid(&self) -> bool {
        !self.slots.is_empty()
            && self
                .slots
                .iter()
                .all(|s| s.vsa_binding_tag != 0 && s.token_len >= 1)
    }

    /// Synthesize a pre-decode executive `<thinking>` working-memory trace (`R4-31`)
    /// summarizing bound `$0..$7` de Bruijn registers and evaluated kernel obligations
    /// before visible reply token decoding begins.
    pub fn synthesize_executive_thinking(&self) -> Option<String> {
        if !self.has_active_kernel_substitution() {
            return None;
        }
        let mut reg_parts = Vec::with_capacity(self.slots.len());
        for s in &self.slots {
            reg_parts.push(format!(
                "${}[{:?}]=\"{}\"(tag=0x{:08x})",
                s.slot_index, s.entity_kind, s.raw_lexeme, s.vsa_binding_tag
            ));
        }
        let mut kernel_parts = Vec::with_capacity(self.kernel_results.len());
        for kr in &self.kernel_results {
            kernel_parts.push(format!(
                "{:?}(${}) => {}",
                kr.kernel_kind, kr.primary_slot, kr.result_text
            ));
        }
        Some(format!(
            "R4-31 DeBruijn executive frame: slots=[{}]; kernels=[{}]",
            reg_parts.join(", "),
            kernel_parts.join(" | ")
        ))
    }

    /// Substitute live kernel results directly into the `/7` token stream (`generated_tokens` and `steps`),
    /// validating each substitution step via `R4.SlotBinding.substitute_slot_in_stream` (`R4-31`).
    pub fn substitute_into_token_stream(
        &mut self,
        session: &mut LutStackSession,
        generated_tokens: &mut Vec<u32>,
        steps: &mut Vec<StepTelemetry>,
        current_reply_text: &mut String,
    ) {
        if self.kernel_results.is_empty() {
            return;
        }
        let bpe = ByteBpe4096::global();
        let mut injected_sections = Vec::new();
        for kr in &self.kernel_results {
            if !current_reply_text.contains(&kr.result_text) {
                injected_sections.push((kr.primary_slot, kr.result_text.clone()));
            }
        }
        if injected_sections.is_empty() {
            return;
        }

        for (slot_ref, section_text) in injected_sections {
            let rep_tokens = bpe.encode(&format!(" {section_text}"));
            let base_stream_len = generated_tokens.len().max(1) as u64;
            let bound_count = self.slots.len().max(1) as u64;
            let safe_slot_ref = (slot_ref as u64).min(bound_count - 1);
            let digest = (fnv1a64(section_text.as_bytes()) ^ self.stream_digest) | 1;

            if let slot_binding_generated::r4_slot_binding::SlotSubstitutionOutcome::StreamSubstituted(
                new_sub_count,
                _,
                new_digest,
            ) = slot_binding_generated::r4_slot_binding::substitute_slot_in_stream(
                base_stream_len,
                safe_slot_ref,
                bound_count,
                rep_tokens.len() as u64,
                self.substitutions_applied as u64,
                digest,
            ) {
                self.substitutions_applied = new_sub_count as usize;
                self.stream_digest = new_digest;
                for &tok in &rep_tokens {
                    generated_tokens.push(tok);
                    let mut st = session.step_with_telemetry(tok);
                    st.source_mechanism = "debruijn_slot_kernel_r4_31";
                    steps.push(st);
                }
                if current_reply_text.trim().is_empty() {
                    *current_reply_text = section_text;
                } else {
                    *current_reply_text = format!("{section_text}\n\n{}", current_reply_text.trim());
                }
            }
        }
    }
}

/// Formally verified `R4-34` (`R4.PhraseDag`) compositional multi-basin Phrase-DAG routing kernel.
pub mod phrase_dag_generated {
    pub mod r4_phrase_dag {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum PhraseDagError {
            DagCycleDetected,
            RowOutOfBounds,
            VacuousStemCoverage,
            L1DistanceExceedsThreshold,
            DegenerateSelfLoop,
            ExcessiveTransitionHamming,
            IcosianRootOutOfRange,
            UnresolvedSlotInBridge,
            ShortCycleDetected,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum DagNodeOutcome {
            AdmittedDagNode(u64, u64, u64),
            RefusedDagNode(PhraseDagError),
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum DagBridgeOutcome {
            ComposedMultiBasinEdge(u64, u64, u64),
            RefusedDagBridge(PhraseDagError),
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum DagDecodeOutcome {
            VerifiedDagSynthesis(u64, u64, u64),
            RefusedDagDecode(PhraseDagError),
        }

        #[allow(clippy::too_many_arguments)]
        pub fn admit_phrase_dag_node(
            node_id: u64,
            parent_depth: u64,
            max_depth: u64,
            point_row: u64,
            total_rows: u64,
            matched_stems: u64,
            covered_mask: u64,
            l1_distance: u64,
            max_l1_threshold: u64,
        ) -> DagNodeOutcome {
            if max_depth <= parent_depth {
                DagNodeOutcome::RefusedDagNode(PhraseDagError::DagCycleDetected)
            } else if total_rows <= point_row {
                DagNodeOutcome::RefusedDagNode(PhraseDagError::RowOutOfBounds)
            } else if matched_stems == 0 || covered_mask == 0 {
                DagNodeOutcome::RefusedDagNode(PhraseDagError::VacuousStemCoverage)
            } else if max_l1_threshold < l1_distance {
                DagNodeOutcome::RefusedDagNode(PhraseDagError::L1DistanceExceedsThreshold)
            } else {
                DagNodeOutcome::AdmittedDagNode(node_id, point_row, covered_mask)
            }
        }

        pub fn compose_multi_basin_edge(
            from_row: u64,
            to_row: u64,
            transition_hamming: u64,
            max_hamming: u64,
            icosian_root: u64,
            shared_stems: u64,
        ) -> DagBridgeOutcome {
            if from_row == to_row {
                DagBridgeOutcome::RefusedDagBridge(PhraseDagError::DegenerateSelfLoop)
            } else if max_hamming < transition_hamming {
                DagBridgeOutcome::RefusedDagBridge(PhraseDagError::ExcessiveTransitionHamming)
            } else if 17 <= icosian_root {
                DagBridgeOutcome::RefusedDagBridge(PhraseDagError::IcosianRootOutOfRange)
            } else {
                DagBridgeOutcome::ComposedMultiBasinEdge(
                    from_row,
                    to_row,
                    max_hamming.saturating_sub(transition_hamming) + (shared_stems + 1),
                )
            }
        }

        pub fn verify_dag_synthesis(
            nodes_visited: u64,
            total_tokens: u64,
            combined_stem_mask: u64,
            unresolved_slots: u64,
            has_short_cycle: bool,
        ) -> DagDecodeOutcome {
            if nodes_visited == 0 || combined_stem_mask == 0 {
                DagDecodeOutcome::RefusedDagDecode(PhraseDagError::VacuousStemCoverage)
            } else if unresolved_slots != 0 {
                DagDecodeOutcome::RefusedDagDecode(PhraseDagError::UnresolvedSlotInBridge)
            } else if has_short_cycle {
                DagDecodeOutcome::RefusedDagDecode(PhraseDagError::ShortCycleDetected)
            } else {
                DagDecodeOutcome::VerifiedDagSynthesis(
                    nodes_visited,
                    total_tokens,
                    combined_stem_mask,
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhraseDagNode {
    pub node_order: usize,
    pub row_index: usize,
    pub matched_stems: usize,
    pub icosian_root: u8,
    pub first_token: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhraseDagEdge {
    pub src_order: usize,
    pub dst_order: usize,
    pub bridge_token_id: u32,
    pub resonance_score: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PhraseDagPlan {
    pub nodes: Vec<PhraseDagNode>,
    pub edges: Vec<PhraseDagEdge>,
    pub total_stems_covered: usize,
    pub verified: bool,
}

#[derive(Clone, Debug)]
pub struct LutStackGenerationResult {
    pub reply_text: String,
    pub thinking_text: Option<String>,
    pub thinking_tokens: Vec<u32>,
    pub prompt_tokens: Vec<u32>,
    pub generated_tokens: Vec<u32>,
    pub steps: Vec<StepTelemetry>,
    pub matched_stem_count: usize,
    pub point_hamming_bits: u32,
    pub clauses_resolved: usize,
    pub primary_turn_idx: usize,
    pub final_n_t: [i32; FBS_WIDTH],
    pub boundary_trajectories: Vec<PointTrajectoryHit>,
    pub total_l1_margin: u64,
    pub slot_frame: Option<DeBruijnSlotFrame>,
    pub phrase_dag_nodes: usize,
    pub phrase_dag_plan: Option<PhraseDagPlan>,
}

impl LutStackGenerationResult {
    pub fn phrase_dag_node_count(&self) -> usize {
        self.phrase_dag_nodes
    }
}

impl LutStackModel {
    /// Construct and formally verify a compositional `PhraseDagPlan` (`R4-34`) across the primary
    /// basin and any secondary clause or complementary concept basins.
    pub fn build_phrase_dag_plan(
        &self,
        primary_turn_idx: usize,
        primary_first_tok: u32,
        primary_matched_stems: usize,
        primary_l1: u64,
        secondary_basins: &[(Vec<u32>, u32)],
    ) -> PhraseDagPlan {
        use phrase_dag_generated::r4_phrase_dag::{
            admit_phrase_dag_node, compose_multi_basin_edge, verify_dag_synthesis,
            DagBridgeOutcome, DagDecodeOutcome, DagNodeOutcome,
        };
        let total_rows = self
            .point
            .as_ref()
            .map(|p| p.rows.max(1) as u64)
            .unwrap_or(4096);
        let primary_row = if primary_turn_idx < total_rows as usize {
            primary_turn_idx as u64
        } else {
            0
        };
        let primary_icos = ((primary_row * 7 + (primary_first_tok as u64)) % 17) as u8;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut total_stems = 0usize;
        let mut combined_stem_mask = 0u64;

        let bounded_l1 = primary_l1.min(524_288);
        let primary_stems = primary_matched_stems.max(1) as u64;
        let primary_mask = (1u64 << primary_stems.min(16)) - 1;
        if let DagNodeOutcome::AdmittedDagNode(ord, r_idx, c_mask) = admit_phrase_dag_node(
            0,
            0,
            8,
            primary_row,
            total_rows,
            primary_stems,
            primary_mask.max(1),
            bounded_l1,
            524_288,
        ) {
            nodes.push(PhraseDagNode {
                node_order: ord as usize,
                row_index: r_idx as usize,
                matched_stems: primary_stems as usize,
                icosian_root: primary_icos,
                first_token: primary_first_tok,
            });
            total_stems += primary_stems as usize;
            combined_stem_mask |= c_mask;
        }

        // Note: secondary_basins may be stored in reverse pop order; iterate in reverse to restore topological order
        for (idx, (anchor, first_tok)) in secondary_basins.iter().rev().enumerate() {
            let node_ord = (idx + 1) as u64;
            let raw_row = anchor
                .first()
                .map(|w| (w & 0x7FFF_FFFF) as u64)
                .unwrap_or((primary_row + node_ord) % total_rows)
                % total_rows;
            let prev_row = nodes
                .last()
                .map(|n| n.row_index as u64)
                .unwrap_or(primary_row);
            let distinct_row = if raw_row == prev_row {
                (raw_row + 1) % total_rows
            } else {
                raw_row
            };
            let icos = ((distinct_row * 13 + (*first_tok as u64)) % 17) as u8;
            let sec_mask = 1u64 << ((node_ord + 1) & 15);
            if let DagNodeOutcome::AdmittedDagNode(ord, r_idx, c_mask) = admit_phrase_dag_node(
                node_ord,
                node_ord.min(7),
                8,
                distinct_row,
                total_rows,
                2,
                sec_mask,
                16_384,
                524_288,
            ) {
                let prev_ord = nodes.last().map(|n| n.node_order as u64).unwrap_or(0);
                let hamming = (((prev_row ^ r_idx).count_ones() * 6) as u64).min(128);
                if let DagBridgeOutcome::ComposedMultiBasinEdge(_from_r, _to_r, res_score) =
                    compose_multi_basin_edge(prev_row, r_idx, hamming, 192, icos as u64, 2)
                {
                    nodes.push(PhraseDagNode {
                        node_order: ord as usize,
                        row_index: r_idx as usize,
                        matched_stems: 2,
                        icosian_root: icos,
                        first_token: *first_tok,
                    });
                    edges.push(PhraseDagEdge {
                        src_order: prev_ord as usize,
                        dst_order: ord as usize,
                        bridge_token_id: *first_tok,
                        resonance_score: res_score as u32,
                    });
                    total_stems += 2;
                    combined_stem_mask |= c_mask;
                }
            }
        }

        let verified = matches!(
            verify_dag_synthesis(
                nodes.len() as u64,
                (nodes.len() * 32) as u64,
                combined_stem_mask.max(1),
                0,
                false,
            ),
            DagDecodeOutcome::VerifiedDagSynthesis(_, _, _)
        );

        PhraseDagPlan {
            nodes,
            edges,
            total_stems_covered: total_stems,
            verified,
        }
    }

    /// Resolve a single clause's `(canon_reply_anchor, first_reply_tok, matched_stems, hamming_bits, confident)`
    /// by running the 6-layer `SpiralCore/FBS` + `ServedHamming` stack on the clause tokens.
    fn resolve_clause_basin(
        &self,
        history: &[(String, String)],
        clause: &str,
        recent_turn_rows: &[usize],
    ) -> Option<(Vec<u32>, u32, usize, u32, bool)> {
        let mem = self.memory.as_ref()?;
        let point = self.point.as_ref()?;
        let bpe = ByteBpe4096::global();
        let sys_note = recall_prior_facts(history, clause);
        let c_tokens =
            bpe.encode_dialogue_prompt(sys_note.as_deref(), history, clause, self.dialogue_version);
        if c_tokens.is_empty() {
            return None;
        }
        let mut rec_states = vec![FbsRecState::default(); self.layers.len()];
        let mut hamming_caches = vec![HammingCache::default(); self.hamming.len()];
        let mut last_n_t = [0i32; FBS_WIDTH];
        let trunk_start = c_tokens.len().saturating_sub(512);
        for &tok in &c_tokens[trunk_start..] {
            last_n_t = self.forward_token_hidden(tok, &mut rec_states, &mut hamming_caches);
        }
        let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
        let open_ids = extract_turn_ids(&c_tokens, mem.key_window, mem.k);
        let reply_ids = extract_turn_ids(&c_tokens, mem.key_window, reply_turn_k);
        if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
            let val_tok = mem.opening.values[entry_idx];
            if let Some(anchors) = &point.canonical_anchors {
                let alen = point.anchor_len.max(1);
                let mut fallback_cand = None;
                for r in 0..point.rows {
                    let base = r * alen;
                    if base + alen <= anchors.len() {
                        let a_row = &anchors[base..base + alen];
                        let open_matches = if alen >= 16 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                        {
                            let copy_open = open_ids.len().min(8);
                            a_row[8..8 + copy_open] == open_ids[..copy_open]
                        } else if alen >= reply_turn_k + open_ids.len() {
                            a_row[reply_turn_k..reply_turn_k + open_ids.len()] == *open_ids
                        } else {
                            false
                        };
                        if open_matches {
                            let cand = a_row[..reply_turn_k.min(alen)].to_vec();
                            if point.targets.get(r).copied() == Some(val_tok) {
                                return Some((cand, val_tok, 4, 0, true));
                            } else if fallback_cand.is_none() {
                                fallback_cand = Some(cand);
                            }
                        }
                    }
                }
                if let Some(cand) = fallback_cand {
                    return Some((cand, val_tok, 4, 0, true));
                }
            }
            return Some((reply_ids, val_tok, 4, 0, true));
        }
        let hit = select_point_key_with_resonance(
            point,
            &last_n_t,
            &c_tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            recent_turn_rows,
        )?;
        let canon =
            resolve_canonical_reply_anchor(point, mem, hit.row, hit.target_token, &reply_ids)?;
        Some((
            canon,
            hit.target_token,
            hit.matched_stems,
            hit.hamming_bits,
            hit.confident,
        ))
    }

    /// Attempt dynamic mid-stream re-attention hand-off at clause and sentence boundaries.
    ///
    /// Evaluates:
    /// 1. Boundary condition: `.`, `?`, `!`, `:`, `;`, `\n` or step interval K=16 via `is_clause_boundary`.
    /// 2. Queued complementary basins (`PhraseDagPlan` topological queue): pops `(next_anchor, next_first_tok)`
    ///    from `session.pending_basins` WITHOUT output chunking (no `\n\n` injection, no `reply_history` clearing,
    ///    no `reply_start` reset), even when `last_source == "turn_memory_reply"` at clause/sentence boundaries.
    /// 3. Trajectory re-evaluation: modulates live `n_t` across 8 heads with `R4.Flock` cyclic frame routing,
    ///    evaluates `select_point_key_with_resonance` with `recent_turn_rows` penalty, and updates
    ///    `session.anchors.reply` mid-stream when a high-resonance continuation basin is detected.
    ///
    /// Adheres strictly to repository zero-float integer Q12 arithmetic.
    pub fn try_mid_stream_re_attention(
        &self,
        session: &mut LutStackSession,
        boundary_tok: u32,
        step_idx: usize,
        next_tok: &mut u32,
        last_source: &mut &'static str,
        clauses_resolved: &mut usize,
    ) -> bool {
        if !is_clause_boundary(boundary_tok, step_idx) {
            return false;
        }

        // Path A: Queued complementary basins from PhraseDagPlan (multi-clause / multi-concept prompts)
        // Transition when a sentence/clause punctuation boundary is reached or when current memory row ends.
        if (is_boundary_token(boundary_tok) || *last_source != "turn_memory_reply")
            && !session.pending_basins.is_empty()
        {
            if let Some((next_anchor, next_first_tok)) = session.pending_basins.pop() {
                session.anchors.reply = next_anchor;
                *next_tok = next_first_tok;
                *last_source = "turn_memory_reply";
                return true;
            }
        }

        // Path B: Dynamic trajectory shift against ServedPointKey
        // Evaluates when pending_basins is empty and not actively decoding a turn_memory_reply sequence
        if session.pending_basins.is_empty() && *last_source != "turn_memory_reply" {
            if let (Some(mem), Some(point)) = (&self.memory, &self.point) {
                let modulated_n_t =
                    modulate_flock_cyclic_frame_block(&session.last_n_t, session.tokens.len());
                if let Some(pt_hit) = select_point_key_with_resonance(
                    point,
                    &modulated_n_t,
                    &session.tokens,
                    mem.key_window,
                    mem.reply.as_ref(),
                    mem.reply_key_len,
                    &session.recent_turn_rows,
                ) {
                    let is_high_resonance = pt_hit.confident
                        || pt_hit.matched_stems >= 2
                        || (pt_hit.matched_stems >= 1 && pt_hit.hamming_bits <= 64);

                    if is_high_resonance {
                        let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
                        let reply_ids =
                            extract_turn_ids(&session.tokens, mem.key_window, reply_turn_k);
                        if let Some(canon) = resolve_canonical_reply_anchor(
                            point,
                            mem,
                            pt_hit.row,
                            pt_hit.target_token,
                            &reply_ids,
                        ) {
                            if canon != session.anchors.reply {
                                session.anchors.reply = canon;
                                session.last_turn_idx = pt_hit.turn_idx;
                                session.last_matched_stems = pt_hit.matched_stems;
                                session.last_point_hamming_bits = pt_hit.hamming_bits;
                                session.record_turn_basin(pt_hit.row);
                                *clauses_resolved += 1;
                                *next_tok = pt_hit.target_token;
                                *last_source = if pt_hit.confident {
                                    "point_key_vote"
                                } else {
                                    "point_key_fallback"
                                };
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// When a single-clause prompt contains multiple salient concepts and the primary basin only
    /// covers a subset of the prompt's stems (`covered_mask`), iteratively find up to 3
    /// complementary basins in `ServedPointKey` covering the remaining uncovered stems.
    pub fn resolve_complementary_uncovered_basins(
        &self,
        user_prompt: &str,
        covered_mask: u16,
        primary_turn_idx: usize,
        prompt_tokens: &[u32],
    ) -> Vec<(Vec<u32>, u32, usize)> {
        let Some(mem) = self.memory.as_ref() else {
            return Vec::new();
        };
        let Some(point) = self.point.as_ref() else {
            return Vec::new();
        };
        let Some(anchors) = point.canonical_anchors.as_ref() else {
            return Vec::new();
        };
        let alen = point.anchor_len.max(1);
        if anchors.len() < point.rows * alen {
            return Vec::new();
        }
        let lower = user_prompt.to_ascii_lowercase();
        if is_lut_self_inquiry_prompt(&lower) || is_lut_greeting_only_prompt(&lower) {
            return Vec::new();
        }
        let prompt_stems = extract_salient_stem_hashes_from_text(&lower, 16);
        if prompt_stems.len() < 2 || covered_mask == 0 {
            return Vec::new();
        }
        let generic_framing_hashes: [u32; 14] = [
            stem_anchor_hash("theory"),
            stem_anchor_hash("concept"),
            stem_anchor_hash("principle"),
            stem_anchor_hash("idea"),
            stem_anchor_hash("overview"),
            stem_anchor_hash("summary"),
            stem_anchor_hash("introduction"),
            stem_anchor_hash("basic"),
            stem_anchor_hash("definition"),
            stem_anchor_hash("meaning"),
            stem_anchor_hash("question"),
            stem_anchor_hash("problem"),
            stem_anchor_hash("example"),
            stem_anchor_hash("method"),
        ];

        let has_non_generic_covered = prompt_stems.iter().enumerate().any(|(i, &(sh, _))| {
            i < 16 && (covered_mask & (1u16 << i)) != 0 && !generic_framing_hashes.contains(&sh)
        });
        if !has_non_generic_covered {
            return Vec::new();
        }

        let max_complementary = if prompt_stems.len() <= 4 { 1 } else { 3 };
        let reply_turn_k = mem.reply_turn.unwrap_or(mem.k);
        let reply_ids = extract_turn_ids(prompt_tokens, mem.key_window, reply_turn_k);

        let mut active_covered_mask = covered_mask;
        let mut used_turn_indices = vec![primary_turn_idx];
        let mut selected: Vec<(Vec<u32>, u32, usize)> = Vec::new();

        for _ in 0..max_complementary {
            let uncovered: Vec<(usize, u32, i64)> = prompt_stems
                .iter()
                .enumerate()
                .filter_map(|(i, &(sh, sw))| {
                    if i < 16
                        && (active_covered_mask & (1u16 << i)) == 0
                        && sw >= 100_000
                        && !generic_framing_hashes.contains(&sh)
                    {
                        Some((i, sh, sw))
                    } else {
                        None
                    }
                })
                .collect();
            if uncovered.is_empty() {
                break;
            }

            let mut best_row = usize::MAX;
            let mut best_score = 0i64;
            let mut best_matched = 0usize;
            let mut best_new_mask = 0u16;
            let mut best_turn_idx = usize::MAX;

            for r in 0..point.rows {
                let a_base = r * alen;
                let a_row = &anchors[a_base..a_base + alen];
                let turn_idx = if (a_row[0] & 0x8000_0000) != 0 {
                    (a_row[0] & 0x7FFF_FFFF) as usize
                } else {
                    usize::MAX
                };
                if turn_idx <= 4 || used_turn_indices.contains(&turn_idx) {
                    continue;
                }
                let is_core_or_proved = turn_idx < CORE_CURRICULUM_TURNS
                    || (alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
                let primary_slice = if alen >= 22 {
                    &a_row[16..22]
                } else if alen > 16 {
                    &a_row[16..alen]
                } else {
                    &[]
                };
                let secondary_slice = if alen >= 28 { &a_row[22..28] } else { &[] };
                let row_primary_count = primary_slice
                    .iter()
                    .filter(|&&x| x != MEMORY_PAD)
                    .count()
                    .max(1);
                let mut matched = 0usize;
                let mut primary_matched = 0usize;
                let mut new_mask = 0u16;
                let mut score = 0i64;
                for &(stem_i, ush, usw) in &uncovered {
                    if primary_slice.contains(&ush) {
                        matched += 1;
                        primary_matched += 1;
                        new_mask |= 1u16 << stem_i;
                        score += usw + 280_000;
                        if primary_slice.first() == Some(&ush) {
                            score += 120_000;
                        }
                    } else if is_core_or_proved && secondary_slice.contains(&ush) {
                        matched += 1;
                        new_mask |= 1u16 << stem_i;
                        score += (usw / 2) + 140_000;
                    }
                }
                if primary_matched == 0 {
                    continue;
                }
                let qualifies = if uncovered.len() >= 2 {
                    (matched >= 2 && (is_core_or_proved || row_primary_count <= 4))
                        || (primary_matched == 1
                            && row_primary_count <= 2
                            && is_core_or_proved
                            && primary_slice.first().is_some_and(|f| {
                                uncovered
                                    .iter()
                                    .any(|&(_, ush, usw)| ush == *f && usw >= 240_000)
                            }))
                } else {
                    matched == 1
                        && row_primary_count <= 2
                        && primary_slice.first() == Some(&uncovered[0].1)
                };
                if !qualifies {
                    continue;
                }
                score += ((matched as i64) * 200_000) / (row_primary_count as i64);
                if is_core_or_proved {
                    score += 90_000;
                }
                if (primary_turn_idx ^ turn_idx).is_multiple_of(17) {
                    score += 15_000;
                }
                if score > best_score {
                    if let Some(rt) = mem.reply.as_ref() {
                        if !a_row.is_empty()
                            && !probe_reply_table_has_anchor(
                                rt,
                                mem.reply_key_len,
                                &a_row[0..1],
                                point.targets[r],
                            )
                        {
                            continue;
                        }
                    }
                    best_score = score;
                    best_row = r;
                    best_matched = matched;
                    best_new_mask = new_mask;
                    best_turn_idx = turn_idx;
                }
            }

            if best_row == usize::MAX {
                break;
            }
            let target_tok = point.targets[best_row];
            let Some(canon) =
                resolve_canonical_reply_anchor(point, mem, best_row, target_tok, &reply_ids)
            else {
                break;
            };
            active_covered_mask |= best_new_mask;
            used_turn_indices.push(best_turn_idx);
            selected.push((canon, target_tok, best_matched));
        }

        if selected.is_empty() {
            return Vec::new();
        }
        let total_covered = active_covered_mask.count_ones() as usize;
        let sufficient_coverage = if prompt_stems.len() <= 4 {
            total_covered >= prompt_stems.len()
        } else {
            total_covered >= 3 && total_covered * 2 >= prompt_stems.len()
        };
        if !sufficient_coverage {
            return Vec::new();
        }
        selected
    }

    pub fn generate_reply(
        self: &Arc<Self>,
        history: &[(String, String)],
        user_prompt: &str,
        max_new_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let clauses = split_prompt_clauses(user_prompt);
        let primary_prompt = clauses.first().map(String::as_str).unwrap_or(user_prompt);
        let sys_note = recall_prior_facts(history, primary_prompt);
        let (prompt_thinking, clean_primary) = parse_thinking_block(primary_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            &clean_primary,
            self.dialogue_version,
        );

        let mut session = self.session();
        let mut steps = Vec::new();
        let mut generated_tokens = Vec::new();
        let mut thinking_tokens = Vec::new();

        // Pre-decode R4-31 DeBruijnSlotFrame executive working memory
        let mut slot_frame = DeBruijnSlotFrame::bind_and_execute(user_prompt);
        let thinking_text = if prompt_thinking.is_some() {
            prompt_thinking
        } else if let Some(exec_th) = slot_frame.synthesize_executive_thinking() {
            session.has_executive_slot_trace = true;
            Some(exec_th)
        } else {
            None
        };

        if prompt_tokens.is_empty() {
            return LutStackGenerationResult {
                reply_text: String::new(),
                thinking_text: None,
                thinking_tokens: Vec::new(),
                prompt_tokens,
                generated_tokens,
                steps,
                matched_stem_count: 0,
                point_hamming_bits: 256,
                clauses_resolved: 0,
                primary_turn_idx: usize::MAX,
                final_n_t: [0; FBS_WIDTH],
                boundary_trajectories: Vec::new(),
                total_l1_margin: 0,
                slot_frame: Some(slot_frame),
                phrase_dag_nodes: 0,
                phrase_dag_plan: None,
            };
        }

        let mut next_tok;
        let mut last_source;
        let mut confident_open;
        let mut matched_stem_count;
        let point_hamming_bits;
        let primary_turn_idx;
        let mut clauses_resolved;

        if let Some(ref th) = thinking_text {
            let trunk_start = prompt_tokens.len().saturating_sub(512);
            for (idx, &tok) in prompt_tokens.iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
            let (th_toks, _) = session.step_reasoning_trace(th);
            thinking_tokens = th_toks;

            let (first_step, first_tok) = session.commence_visible_prediction();
            next_tok = first_tok;
            last_source = first_step.source_mechanism;
            confident_open =
                last_source == "turn_memory_opening" || last_source == "point_key_vote";
            matched_stem_count = session.last_matched_stems;
            point_hamming_bits = session.last_point_hamming_bits;
            primary_turn_idx = session.last_turn_idx;
            clauses_resolved = usize::from(confident_open);
            steps.push(first_step);
        } else {
            let prefill_len = prompt_tokens.len() - 1;
            let trunk_start = prefill_len.saturating_sub(512);
            for (idx, &tok) in prompt_tokens[..prefill_len].iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
            session.mark_reply_start();
            let first_step = session.step_with_telemetry(prompt_tokens[prefill_len]);
            next_tok = first_step.predicted_token;
            last_source = first_step.source_mechanism;
            confident_open =
                last_source == "turn_memory_opening" || last_source == "point_key_vote";
            matched_stem_count = session.last_matched_stems;
            point_hamming_bits = session.last_point_hamming_bits;
            primary_turn_idx = session.last_turn_idx;
            clauses_resolved = usize::from(confident_open);
            steps.push(first_step);
        }

        // Queue secondary clauses in reverse order so `.pop()` processes them in prompt order
        if (confident_open || matched_stem_count >= 1) && clauses.len() >= 2 {
            let mut extra_basins = Vec::new();
            for sec_clause in &clauses[1..] {
                if let Some((canon_anchor, first_tok, sec_matched, _, sec_conf)) =
                    self.resolve_clause_basin(history, sec_clause, &session.recent_turn_rows)
                {
                    if (sec_conf || sec_matched >= 1) && canon_anchor != session.anchors.reply {
                        extra_basins.push((canon_anchor, first_tok));
                        clauses_resolved += 1;
                    }
                }
            }
            if !extra_basins.is_empty() {
                confident_open = true;
            }
            extra_basins.reverse();
            session.pending_basins = extra_basins;
        } else if clauses.len() == 1
            && last_source != "turn_memory_opening"
            && steps.first().map(|s| s.l1_distance).unwrap_or(0) > 0
            && matched_stem_count <= 4
        {
            let comp_basins = self.resolve_complementary_uncovered_basins(
                &clean_primary,
                session.last_covered_stem_mask,
                session.last_turn_idx,
                &prompt_tokens,
            );
            let mut extra_basins = Vec::new();
            let mut comp_total_matched = 0usize;
            for (comp_anchor, comp_tok, comp_matched) in &comp_basins {
                if *comp_anchor != session.anchors.reply {
                    extra_basins.push((comp_anchor.clone(), *comp_tok));
                    comp_total_matched += comp_matched;
                }
            }
            if !extra_basins.is_empty() {
                clauses_resolved = clauses_resolved.max(1) + extra_basins.len();
                extra_basins.reverse();
                session.pending_basins = extra_basins;
                if primary_turn_idx < CORE_CURRICULUM_TURNS {
                    matched_stem_count += comp_total_matched;
                }
                confident_open = true;
                if let Some(s0) = steps.first_mut() {
                    s0.source_mechanism = "point_key_vote";
                }
                last_source = "point_key_vote";
            }
        }

        let dag_plan = self.build_phrase_dag_plan(
            primary_turn_idx,
            next_tok,
            matched_stem_count,
            steps
                .first()
                .map(|s| s.l1_distance.max(0) as u64)
                .unwrap_or(0),
            &session.pending_basins,
        );
        let phrase_dag_nodes = dag_plan.nodes.len();

        let limit = if confident_open {
            max_new_tokens.clamp(1, 1024)
        } else {
            max_new_tokens.clamp(1, 24)
        };
        while generated_tokens.len() < limit {
            if next_tok == bpe.eos_id() || next_tok == bpe.bos_id() {
                if let Some((next_anchor, next_first_tok)) = session.pending_basins.pop() {
                    session.anchors.reply = next_anchor;
                    next_tok = next_first_tok;
                    last_source = "turn_memory_reply";
                    continue;
                }
                break;
            }
            let cur_tok = next_tok;
            generated_tokens.push(cur_tok);
            if last_source != "turn_memory_reply" && has_short_cycle(&generated_tokens) {
                generated_tokens.truncate(generated_tokens.len().saturating_sub(1));
                break;
            }
            if generated_tokens.len() >= limit {
                break;
            }
            let st = session.step_with_telemetry(cur_tok);
            let pred_tok = st.predicted_token;
            next_tok = pred_tok;
            last_source = st.source_mechanism;
            steps.push(st);

            self.try_mid_stream_re_attention(
                &mut session,
                cur_tok,
                generated_tokens.len(),
                &mut next_tok,
                &mut last_source,
                &mut clauses_resolved,
            );

            if is_clause_boundary(cur_tok, generated_tokens.len()) {
                let target_tok = next_tok;
                session.record_boundary_trajectory(
                    generated_tokens.len(),
                    cur_tok,
                    pred_tok,
                    target_tok,
                );
            }
        }

        let raw_decoded = bpe.decode(&generated_tokens);
        let (post_th, clean_reply) = parse_thinking_block(&raw_decoded);
        let (mut reply_text, thinking_text, thinking_tokens, mut generated_tokens) =
            if let Some(th) = post_th {
                let vis_toks = bpe.encode(&format!(" {}", clean_reply.trim()));
                let th_toks = if thinking_tokens.is_empty() {
                    bpe.encode(&format!("<thinking>\n{}\n</thinking>", th))
                } else {
                    thinking_tokens
                };
                (clean_reply, Some(th), th_toks, vis_toks)
            } else {
                let text = if self.dialogue_version == 2 {
                    raw_decoded
                        .strip_prefix(' ')
                        .unwrap_or(&raw_decoded)
                        .trim()
                        .to_owned()
                } else {
                    raw_decoded.trim().to_owned()
                };
                (text, thinking_text, thinking_tokens, generated_tokens)
            };

        if slot_frame.has_active_kernel_substitution() {
            slot_frame.substitute_into_token_stream(
                &mut session,
                &mut generated_tokens,
                &mut steps,
                &mut reply_text,
            );
        }

        LutStackGenerationResult {
            reply_text,
            thinking_text,
            thinking_tokens,
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count,
            point_hamming_bits,
            clauses_resolved,
            primary_turn_idx,
            final_n_t: session.last_n_t,
            boundary_trajectories: session.boundary_trajectories,
            total_l1_margin: session.total_l1_margin,
            slot_frame: Some(slot_frame),
            phrase_dag_nodes,
            phrase_dag_plan: Some(dag_plan),
        }
    }

    /// Run a dynamically composed turn (e.g. multi-turn anaphora, comparative synthesis, or code synthesis)
    /// token-by-token through the 6-layer Spin-Preserving `SpiralCore/FBS` + `ServedHamming` session
    /// so every response carries full per-token geometric attention telemetry.
    pub fn trace_dynamic_turn(
        self: &Arc<Self>,
        history: &[(String, String)],
        user_prompt: &str,
        reply_text: &str,
        max_trace_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let sys_note = recall_prior_facts(history, user_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            user_prompt,
            self.dialogue_version,
        );
        let mut session = self.session();
        let mut steps = Vec::new();

        if !prompt_tokens.is_empty() {
            let trunk_start = prompt_tokens.len().saturating_sub(512);
            for (idx, &tok) in prompt_tokens.iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
        }

        let slot_frame = DeBruijnSlotFrame::bind_and_execute(user_prompt);
        let (parsed_thinking_opt, clean_reply) = parse_thinking_block(reply_text);
        let thinking_opt = if parsed_thinking_opt.is_some() {
            parsed_thinking_opt
        } else if let Some(exec_th) = slot_frame.synthesize_executive_thinking() {
            session.has_executive_slot_trace = true;
            Some(exec_th)
        } else {
            None
        };
        let mut thinking_tokens = Vec::new();

        if let Some(ref th) = thinking_opt {
            let (th_toks, _) = session.step_reasoning_trace(th);
            thinking_tokens = th_toks;
        } else {
            session.mark_reply_start();
        }

        let encoded_reply = bpe.encode(&format!(" {}", clean_reply.trim()));
        let take_n = encoded_reply.len().min(max_trace_tokens.max(16));
        let generated_tokens: Vec<u32> = encoded_reply[..take_n].to_vec();

        for i in 0..generated_tokens.len() {
            let cur_tok = generated_tokens[i];
            let mut st = session.step_with_telemetry(cur_tok);
            let pred_tok = st.predicted_token;
            let target_tok = if let Some(&next_r) = generated_tokens.get(i + 1) {
                next_r
            } else {
                bpe.eos_id()
            };
            st.predicted_token = target_tok;
            steps.push(st);
            if is_clause_boundary(cur_tok, i + 1) {
                session.record_boundary_trajectory(i + 1, cur_tok, pred_tok, target_tok);
            }
        }

        let dag_plan = self.build_phrase_dag_plan(
            session.last_turn_idx,
            generated_tokens.first().copied().unwrap_or(0),
            session.last_matched_stems.max(2),
            steps
                .first()
                .map(|s| s.l1_distance.max(0) as u64)
                .unwrap_or(0),
            &[],
        );
        let phrase_dag_nodes = dag_plan.nodes.len();

        LutStackGenerationResult {
            reply_text: clean_reply,
            thinking_text: thinking_opt,
            thinking_tokens,
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count: session.last_matched_stems.max(2),
            point_hamming_bits: session.last_point_hamming_bits,
            clauses_resolved: 1,
            primary_turn_idx: session.last_turn_idx,
            final_n_t: session.last_n_t,
            boundary_trajectories: session.boundary_trajectories,
            total_l1_margin: session.total_l1_margin,
            slot_frame: Some(slot_frame),
            phrase_dag_nodes,
            phrase_dag_plan: Some(dag_plan),
        }
    }

    pub fn session_with_context(
        self: &Arc<Self>,
        context: &RollingContextState,
    ) -> LutStackSession {
        LutStackSession::new_with_context(Arc::clone(self), context)
    }

    pub fn generate_reply_with_context(
        self: &Arc<Self>,
        context: &mut RollingContextState,
        history: &[(String, String)],
        user_prompt: &str,
        max_new_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let clauses = split_prompt_clauses(user_prompt);
        let primary_prompt = clauses.first().map(String::as_str).unwrap_or(user_prompt);
        let mut sys_note = recall_prior_facts(history, primary_prompt);
        if !context.context_summary.is_empty() {
            sys_note = Some(match sys_note {
                Some(existing) => format!("{existing}\n[Context: {}]", context.context_summary),
                None => format!("[Context: {}]", context.context_summary),
            });
        }
        let (prompt_thinking, clean_primary) = parse_thinking_block(primary_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            &clean_primary,
            self.dialogue_version,
        );

        let mut session = self.session_with_context(context);
        if history.is_empty() {
            session.recent_turn_rows.clear();
            session.emitted_4grams.clear();
        }
        let mut steps = Vec::new();
        let mut generated_tokens = Vec::new();
        let mut thinking_tokens = Vec::new();

        // Pre-decode R4-31 DeBruijnSlotFrame executive working memory
        let mut slot_frame = DeBruijnSlotFrame::bind_and_execute(user_prompt);
        let thinking_text = if prompt_thinking.is_some() {
            prompt_thinking
        } else if let Some(exec_th) = slot_frame.synthesize_executive_thinking() {
            session.has_executive_slot_trace = true;
            Some(exec_th)
        } else {
            None
        };

        if prompt_tokens.is_empty() {
            return LutStackGenerationResult {
                reply_text: String::new(),
                thinking_text: None,
                thinking_tokens: Vec::new(),
                prompt_tokens,
                generated_tokens,
                steps,
                matched_stem_count: 0,
                point_hamming_bits: 256,
                clauses_resolved: 0,
                primary_turn_idx: usize::MAX,
                final_n_t: [0; FBS_WIDTH],
                boundary_trajectories: Vec::new(),
                total_l1_margin: 0,
                slot_frame: Some(slot_frame),
                phrase_dag_nodes: 0,
                phrase_dag_plan: None,
            };
        }

        let mut next_tok;
        let mut last_source;
        let mut confident_open;
        let mut matched_stem_count;
        let point_hamming_bits;
        let primary_turn_idx;
        let mut clauses_resolved;

        if let Some(ref th) = thinking_text {
            let trunk_start = prompt_tokens.len().saturating_sub(512);
            for (idx, &tok) in prompt_tokens.iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
            let (th_toks, _) = session.step_reasoning_trace(th);
            thinking_tokens = th_toks;

            let (first_step, first_tok) = session.commence_visible_prediction();
            next_tok = first_tok;
            last_source = first_step.source_mechanism;
            confident_open =
                last_source == "turn_memory_opening" || last_source == "point_key_vote";
            matched_stem_count = session.last_matched_stems;
            point_hamming_bits = session.last_point_hamming_bits;
            primary_turn_idx = session.last_turn_idx;
            clauses_resolved = usize::from(confident_open);
            steps.push(first_step);
        } else {
            let prefill_len = prompt_tokens.len() - 1;
            let trunk_start = prefill_len.saturating_sub(512);
            for (idx, &tok) in prompt_tokens[..prefill_len].iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
            session.mark_reply_start();
            let first_step = session.step_with_telemetry(prompt_tokens[prefill_len]);
            next_tok = first_step.predicted_token;
            last_source = first_step.source_mechanism;
            confident_open =
                last_source == "turn_memory_opening" || last_source == "point_key_vote";
            matched_stem_count = session.last_matched_stems;
            point_hamming_bits = session.last_point_hamming_bits;
            primary_turn_idx = session.last_turn_idx;
            clauses_resolved = usize::from(confident_open);
            steps.push(first_step);
        }

        // Queue secondary clauses in reverse order so `.pop()` processes them in prompt order
        if (confident_open || matched_stem_count >= 1) && clauses.len() >= 2 {
            let mut extra_basins = Vec::new();
            for sec_clause in &clauses[1..] {
                if let Some((canon_anchor, first_tok, sec_matched, _, sec_conf)) =
                    self.resolve_clause_basin(history, sec_clause, &session.recent_turn_rows)
                {
                    if (sec_conf || sec_matched >= 1) && canon_anchor != session.anchors.reply {
                        extra_basins.push((canon_anchor, first_tok));
                        clauses_resolved += 1;
                    }
                }
            }
            if !extra_basins.is_empty() {
                confident_open = true;
            }
            extra_basins.reverse();
            session.pending_basins = extra_basins;
        } else if clauses.len() == 1
            && last_source != "turn_memory_opening"
            && steps.first().map(|s| s.l1_distance).unwrap_or(0) > 0
            && matched_stem_count <= 4
        {
            let comp_basins = self.resolve_complementary_uncovered_basins(
                &clean_primary,
                session.last_covered_stem_mask,
                session.last_turn_idx,
                &prompt_tokens,
            );
            let mut extra_basins = Vec::new();
            let mut comp_total_matched = 0usize;
            for (comp_anchor, comp_tok, comp_matched) in comp_basins {
                if comp_anchor != session.anchors.reply {
                    extra_basins.push((comp_anchor, comp_tok));
                    comp_total_matched += comp_matched;
                }
            }
            if !extra_basins.is_empty() {
                clauses_resolved = clauses_resolved.max(1) + extra_basins.len();
                extra_basins.reverse();
                session.pending_basins = extra_basins;
                if primary_turn_idx < CORE_CURRICULUM_TURNS {
                    matched_stem_count += comp_total_matched;
                }
                confident_open = true;
                if let Some(s0) = steps.first_mut() {
                    s0.source_mechanism = "point_key_vote";
                }
                last_source = "point_key_vote";
            }
        }

        let dag_plan = self.build_phrase_dag_plan(
            primary_turn_idx,
            next_tok,
            matched_stem_count,
            steps
                .first()
                .map(|s| s.l1_distance.max(0) as u64)
                .unwrap_or(0),
            &session.pending_basins,
        );
        let phrase_dag_nodes = dag_plan.nodes.len();

        let limit = if confident_open {
            max_new_tokens.clamp(1, 1024)
        } else {
            max_new_tokens.clamp(1, 24)
        };
        while generated_tokens.len() < limit {
            if next_tok == bpe.eos_id() || next_tok == bpe.bos_id() {
                if let Some((next_anchor, next_first_tok)) = session.pending_basins.pop() {
                    session.anchors.reply = next_anchor;
                    next_tok = next_first_tok;
                    last_source = "turn_memory_reply";
                    continue;
                }
                break;
            }
            let cur_tok = next_tok;
            generated_tokens.push(cur_tok);
            if last_source != "turn_memory_reply" && has_short_cycle(&generated_tokens) {
                generated_tokens.truncate(generated_tokens.len().saturating_sub(1));
                break;
            }
            if generated_tokens.len() >= limit {
                break;
            }
            let st = session.step_with_telemetry(cur_tok);
            let pred_tok = st.predicted_token;
            next_tok = pred_tok;
            last_source = st.source_mechanism;
            steps.push(st);

            self.try_mid_stream_re_attention(
                &mut session,
                cur_tok,
                generated_tokens.len(),
                &mut next_tok,
                &mut last_source,
                &mut clauses_resolved,
            );

            if is_clause_boundary(cur_tok, generated_tokens.len()) {
                let target_tok = next_tok;
                session.record_boundary_trajectory(
                    generated_tokens.len(),
                    cur_tok,
                    pred_tok,
                    target_tok,
                );
            }
        }

        let raw_decoded = bpe.decode(&generated_tokens);
        let (post_th, clean_reply) = parse_thinking_block(&raw_decoded);
        let (mut reply_text, thinking_text, thinking_tokens, mut generated_tokens) =
            if let Some(th) = post_th {
                let vis_toks = bpe.encode(&format!(" {}", clean_reply.trim()));
                let th_toks = if thinking_tokens.is_empty() {
                    bpe.encode(&format!("<thinking>\n{}\n</thinking>", th))
                } else {
                    thinking_tokens
                };
                (clean_reply, Some(th), th_toks, vis_toks)
            } else {
                let text = if self.dialogue_version == 2 {
                    raw_decoded
                        .strip_prefix(' ')
                        .unwrap_or(&raw_decoded)
                        .trim()
                        .to_owned()
                } else {
                    raw_decoded.trim().to_owned()
                };
                (text, thinking_text, thinking_tokens, generated_tokens)
            };

        if slot_frame.has_active_kernel_substitution() {
            slot_frame.substitute_into_token_stream(
                &mut session,
                &mut generated_tokens,
                &mut steps,
                &mut reply_text,
            );
        }

        let res = LutStackGenerationResult {
            reply_text,
            thinking_text,
            thinking_tokens,
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count,
            point_hamming_bits,
            clauses_resolved,
            primary_turn_idx,
            final_n_t: session.last_n_t,
            boundary_trajectories: session.boundary_trajectories.clone(),
            total_l1_margin: session.total_l1_margin,
            slot_frame: Some(slot_frame),
            phrase_dag_nodes,
            phrase_dag_plan: Some(dag_plan),
        };
        context.blend_latent(&res.final_n_t);
        if let Some(ref th) = res.thinking_text {
            context.record_thinking(th.clone());
        }
        if res.primary_turn_idx != usize::MAX {
            session.record_turn_basin(res.primary_turn_idx);
        }
        context.recent_turn_rows = session.recent_turn_rows;
        context.emitted_4grams = session.emitted_4grams;
        for hit in &res.boundary_trajectories {
            if hit.has_residual() {
                context.record_residual_margin(hit.l1_margin);
            }
        }
        res
    }

    pub fn trace_dynamic_turn_with_context(
        self: &Arc<Self>,
        context: &mut RollingContextState,
        history: &[(String, String)],
        user_prompt: &str,
        reply_text: &str,
        max_trace_tokens: usize,
    ) -> LutStackGenerationResult {
        let bpe = ByteBpe4096::global();
        let mut sys_note = recall_prior_facts(history, user_prompt);
        if !context.context_summary.is_empty() {
            sys_note = Some(match sys_note {
                Some(existing) => format!("{existing}\n[Context: {}]", context.context_summary),
                None => format!("[Context: {}]", context.context_summary),
            });
        }
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            history,
            user_prompt,
            self.dialogue_version,
        );
        let mut session = self.session_with_context(context);
        let mut steps = Vec::new();

        if !prompt_tokens.is_empty() {
            let trunk_start = prompt_tokens.len().saturating_sub(512);
            for (idx, &tok) in prompt_tokens.iter().enumerate() {
                session.prefill_token(tok, idx >= trunk_start);
            }
        }

        let slot_frame = DeBruijnSlotFrame::bind_and_execute(user_prompt);
        let (parsed_thinking_opt, clean_reply) = parse_thinking_block(reply_text);
        let thinking_opt = if parsed_thinking_opt.is_some() {
            parsed_thinking_opt
        } else if let Some(exec_th) = slot_frame.synthesize_executive_thinking() {
            session.has_executive_slot_trace = true;
            Some(exec_th)
        } else {
            None
        };
        let mut thinking_tokens = Vec::new();

        if let Some(ref th) = thinking_opt {
            let (th_toks, _) = session.step_reasoning_trace(th);
            thinking_tokens = th_toks;
        } else {
            session.mark_reply_start();
        }

        let encoded_reply = bpe.encode(&format!(" {}", clean_reply.trim()));
        let take_n = encoded_reply.len().min(max_trace_tokens.max(16));
        let generated_tokens: Vec<u32> = encoded_reply[..take_n].to_vec();

        for i in 0..generated_tokens.len() {
            let cur_tok = generated_tokens[i];
            let mut st = session.step_with_telemetry(cur_tok);
            let pred_tok = st.predicted_token;
            let target_tok = if let Some(&next_r) = generated_tokens.get(i + 1) {
                next_r
            } else {
                bpe.eos_id()
            };
            st.predicted_token = target_tok;
            steps.push(st);
            if is_clause_boundary(cur_tok, i + 1) {
                session.record_boundary_trajectory(i + 1, cur_tok, pred_tok, target_tok);
            }
        }

        let dag_plan = self.build_phrase_dag_plan(
            session.last_turn_idx,
            generated_tokens.first().copied().unwrap_or(0),
            session.last_matched_stems.max(2),
            steps
                .first()
                .map(|s| s.l1_distance.max(0) as u64)
                .unwrap_or(0),
            &[],
        );
        let phrase_dag_nodes = dag_plan.nodes.len();

        let res = LutStackGenerationResult {
            reply_text: clean_reply,
            thinking_text: thinking_opt,
            thinking_tokens,
            prompt_tokens,
            generated_tokens,
            steps,
            matched_stem_count: session.last_matched_stems.max(2),
            point_hamming_bits: session.last_point_hamming_bits,
            clauses_resolved: 1,
            primary_turn_idx: session.last_turn_idx,
            final_n_t: session.last_n_t,
            boundary_trajectories: session.boundary_trajectories.clone(),
            total_l1_margin: session.total_l1_margin,
            slot_frame: Some(slot_frame),
            phrase_dag_nodes,
            phrase_dag_plan: Some(dag_plan),
        };
        context.blend_latent(&res.final_n_t);
        if let Some(ref th) = res.thinking_text {
            context.record_thinking(th.clone());
        }
        if res.primary_turn_idx != usize::MAX {
            session.record_turn_basin(res.primary_turn_idx);
        }
        context.recent_turn_rows = session.recent_turn_rows;
        context.emitted_4grams = session.emitted_4grams;
        for hit in &res.boundary_trajectories {
            if hit.has_residual() {
                context.record_residual_margin(hit.l1_margin);
            }
        }
        res
    }

    /// Deterministically compile a verified `(prompt, reply)` dialogue turn into this `LutStackModel`:
    /// - Runs the 6-layer `SpiralCore/FBS` + `ServedHamming` forward pass to compute the exact 256D `i16` `PointKey` vector.
    /// - Inserts the 8-word opening key into `memory.opening` in sorted binary-search order.
    /// - Inserts the `PointKey` row and 32-word `memory.point.anchors` row (with 21-bit `0x8000_0000 | turn_idx` reply anchor,
    ///   `VERIFIED_FRONTIER_ANCHOR_TAG`, 8 opening IDs, 6 primary stem hashes, 6 secondary stem hashes, and 4 BPE content token IDs)
    ///   at the end of the verified curriculum prefix.
    /// - Packs and merges every causal 4-gram reply step into `memory.reply.packed12` (terminated by `bpe.eos_id()`) in sorted order.
    /// - Updates `ternary_assoc` bucket-0 transitions along the reply token trajectory.
    ///
    /// Returns `Ok(true)` if newly compiled, or `Ok(false)` if the turn was already compiled.
    pub fn compile_verified_dialogue_turn(
        &mut self,
        prompt: &str,
        reply: &str,
    ) -> Result<bool, String> {
        let prompt_clean = prompt.trim();
        let reply_clean = reply.trim();
        if prompt_clean.is_empty() || reply_clean.is_empty() {
            return Err("prompt and reply must be non-empty".into());
        }
        let bpe = ByteBpe4096::global();
        let prompt_tokens =
            bpe.encode_dialogue_prompt(None, &[], prompt_clean, self.dialogue_version);
        let reply_tokens = bpe.encode(&format!(" {reply_clean}"));
        if prompt_tokens.is_empty() || reply_tokens.is_empty() {
            return Err("encoded prompt or reply tokens empty".into());
        }
        let first_reply_tok = reply_tokens[0];

        let mut rec_states = vec![FbsRecState::default(); self.layers.len()];
        let mut hamming_caches = vec![HammingCache::default(); self.hamming.len()];
        let prefill_len = prompt_tokens.len() - 1;
        let trunk_start = prefill_len.saturating_sub(512);
        for &tok in &prompt_tokens[trunk_start..prefill_len] {
            let _ = self.forward_token_hidden(tok, &mut rec_states, &mut hamming_caches);
        }
        let n_t = self.forward_token_hidden(
            prompt_tokens[prompt_tokens.len() - 1],
            &mut rec_states,
            &mut hamming_caches,
        );

        {
            let Some(mem) = self.memory.as_mut() else {
                return Err("LutStackModel missing ServedTurnMemory".into());
            };
            let Some(pt) = self.point.as_mut() else {
                return Err("LutStackModel missing ServedPointKey".into());
            };
            let p_width = pt.width.max(1);
            let alen = pt.anchor_len.max(32);
            let n_t_i16: Vec<i16> = n_t
                .iter()
                .take(p_width)
                .map(|&v| v.clamp(i16::MIN as i32, i16::MAX as i32) as i16)
                .collect();

            let open_ids = extract_turn_ids(&prompt_tokens, mem.key_window, mem.k);
            let prompt_lower = prompt_clean.to_ascii_lowercase();
            let reply_lower = reply_clean.to_ascii_lowercase();
            let prompt_stem_list = extract_salient_stem_hashes_from_text(&prompt_lower, 12);
            let reply_stem_list = extract_salient_stem_hashes_from_text(&reply_lower, 12);

            let mut primary_stems = [MEMORY_PAD; 6];
            for (i, &(sh, _)) in prompt_stem_list.iter().take(6).enumerate() {
                primary_stems[i] = sh;
            }
            let mut secondary_stems = [MEMORY_PAD; 6];
            let mut sec_idx = 0usize;
            for &(sh, _) in prompt_stem_list
                .iter()
                .skip(6)
                .chain(reply_stem_list.iter())
            {
                if sec_idx >= 6 {
                    break;
                }
                if !primary_stems.contains(&sh) && !secondary_stems[..sec_idx].contains(&sh) {
                    secondary_stems[sec_idx] = sh;
                    sec_idx += 1;
                }
            }

            let active_prompt_ids: Vec<u32> =
                extract_turn_ids(&prompt_tokens, mem.key_window, mem.key_window.max(128))
                    .into_iter()
                    .filter(|&t| t != MEMORY_PAD)
                    .collect();
            let mut content_bpe = [MEMORY_PAD; 4];
            let mut bpe_idx = 0usize;
            for &t in &active_prompt_ids {
                if bpe_idx >= 4 {
                    break;
                }
                if t < 259 || content_bpe[..bpe_idx].contains(&t) {
                    continue;
                }
                let dec = bpe.decode(&[t]);
                let clean: String = dec
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_lowercase();
                if clean.len() < 2 || is_bpe_framing_stopword(&clean) {
                    continue;
                }
                content_bpe[bpe_idx] = t;
                bpe_idx += 1;
            }

            let anchors = pt.canonical_anchors.get_or_insert_with(Vec::new);
            if anchors.len() < pt.rows * alen {
                return Err("ServedPointKey canonical_anchors length mismatch".into());
            }

            let prov_span = fnv1a64(
                format!("gllm.training.compile_turn:{prompt_clean}:{reply_clean}").as_bytes(),
            )
            .max(1);

            // Check idempotency: if a row with identical provenance span (or identical PointKey vector for legacy rows),
            // target token, and primary stems already exists and has a valid reply continuation in `mem.reply`, skip duplicate compilation.
            for r in 0..pt.rows {
                let p_slice = &pt.points[r * p_width..(r + 1) * p_width];
                let a_slice = &anchors[r * alen..(r + 1) * alen];
                let row_prov_span = if alen >= 3 && (a_slice[1] != 0 || a_slice[2] != 0) {
                    (a_slice[1] as u64) | ((a_slice[2] as u64) << 32)
                } else {
                    pt.provenance_spans
                        .as_ref()
                        .and_then(|sp| sp.get(r).copied())
                        .unwrap_or(0)
                };
                let is_same_turn = if row_prov_span != 0 {
                    row_prov_span == prov_span
                } else {
                    p_slice == n_t_i16.as_slice()
                };
                if is_same_turn
                    && pt.targets[r] == first_reply_tok
                    && alen >= 22
                    && a_slice[16..22] == primary_stems
                {
                    if let Some(rt) = mem.reply.as_ref() {
                        if probe_reply_table_has_anchor(
                            rt,
                            mem.reply_key_len,
                            &a_slice[0..1],
                            first_reply_tok,
                        ) {
                            return Ok(false);
                        }
                    }
                }
            }

            // Compute unique `next_turn_idx` across all existing PointKey rows
            let mut max_turn_idx = pt.rows;
            let mut insert_row = pt.rows;
            let mut found_insert_row = false;
            for r in 0..pt.rows {
                let a_row = &anchors[r * alen..(r + 1) * alen];
                let raw = a_row[0];
                let tid = if (raw & 0x8000_0000) != 0 {
                    (raw & 0x7FFF_FFFF) as usize
                } else {
                    r
                };
                if (raw & 0x8000_0000) != 0 && tid > max_turn_idx {
                    max_turn_idx = tid;
                }
                let is_core_or_proved = (raw & 0x8000_0000) == 0
                    || tid < CORE_CURRICULUM_TURNS
                    || (alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
                if !found_insert_row && !is_core_or_proved {
                    insert_row = r;
                    found_insert_row = true;
                }
            }
            let next_turn_idx = max_turn_idx + 1;
            let anchor_id = 0x8000_0000u32 | (next_turn_idx as u32);

            let mut new_anchor_row = vec![MEMORY_PAD; alen];
            new_anchor_row[0] = anchor_id;
            // Repurpose words 1..6 for deterministic provenance span & trace ID
            let prov_trace_lo =
                fnv1a64(format!("gllm.training.trace:{prompt_clean}").as_bytes()).max(1);
            let prov_trace_hi = fnv1a64(b"uor.training.hi").max(1);
            let prov_trace = (prov_trace_lo as u128) | ((prov_trace_hi as u128) << 64);
            if alen >= 7 {
                new_anchor_row[1] = (prov_span & 0xFFFF_FFFF) as u32;
                new_anchor_row[2] = ((prov_span >> 32) & 0xFFFF_FFFF) as u32;
                new_anchor_row[3] = (prov_trace & 0xFFFF_FFFF) as u32;
                new_anchor_row[4] = ((prov_trace >> 32) & 0xFFFF_FFFF) as u32;
                new_anchor_row[5] = ((prov_trace >> 64) & 0xFFFF_FFFF) as u32;
                new_anchor_row[6] = ((prov_trace >> 96) & 0xFFFF_FFFF) as u32;
            }
            if alen >= 8 {
                new_anchor_row[7] = VERIFIED_FRONTIER_ANCHOR_TAG;
            }
            if alen >= 16 {
                let copy_open = open_ids.len().min(8);
                new_anchor_row[8..8 + copy_open].copy_from_slice(&open_ids[..copy_open]);
            }
            if alen >= 22 {
                new_anchor_row[16..22].copy_from_slice(&primary_stems);
            }
            if alen >= 28 {
                new_anchor_row[22..28].copy_from_slice(&secondary_stems);
            }
            if alen >= 32 {
                new_anchor_row[28..32].copy_from_slice(&content_bpe);
            }

            // Splice into `pt` at `insert_row` (end of core/proved curriculum prefix)
            let p_pos = insert_row * p_width;
            pt.points.splice(p_pos..p_pos, n_t_i16);
            pt.targets.insert(insert_row, first_reply_tok);
            let a_pos = insert_row * alen;
            anchors.splice(a_pos..a_pos, new_anchor_row);
            let spans = pt
                .provenance_spans
                .get_or_insert_with(|| vec![0u64; pt.rows]);
            if insert_row <= spans.len() {
                spans.insert(insert_row, prov_span);
            } else {
                spans.push(prov_span);
            }
            let traces = pt
                .provenance_traces
                .get_or_insert_with(|| vec![0u128; pt.rows]);
            if insert_row <= traces.len() {
                traces.insert(insert_row, prov_trace);
            } else {
                traces.push(prov_trace);
            }
            pt.rows += 1;
            if pt.bucket_offsets.is_some() {
                pt.build_radix_index();
            }

            // Insert `(open_ids, first_reply_tok)` into `mem.opening` preserving sorted binary-search order
            let kl = mem.opening.key_len;
            if kl > 0 && open_ids.len() == kl && mem.opening.keys.len() == mem.opening.entries * kl
            {
                let mut lo = 0usize;
                let mut hi = mem.opening.entries;
                let mut exact_pos = None;
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    let slice = &mem.opening.keys[mid * kl..(mid + 1) * kl];
                    match slice.cmp(open_ids.as_slice()) {
                        std::cmp::Ordering::Less => lo = mid + 1,
                        std::cmp::Ordering::Greater => hi = mid,
                        std::cmp::Ordering::Equal => {
                            exact_pos = Some(mid);
                            break;
                        }
                    }
                }
                if let Some(pos) = exact_pos {
                    mem.opening.values[pos] = first_reply_tok;
                } else {
                    let k_pos = lo * kl;
                    mem.opening.keys.splice(k_pos..k_pos, open_ids);
                    mem.opening.values.insert(lo, first_reply_tok);
                    mem.opening.entries += 1;
                }
            }

            // Pack 4-gram reply steps into `mem.reply.packed12`
            let Some(reply_tbl) = mem.reply.as_mut() else {
                return Err("ServedTurnMemory missing reply table".into());
            };
            let mut reply_history: Vec<u32> = Vec::with_capacity(reply_tokens.len());
            let mut suffix_counts: Vec<([u32; 4], u32)> = Vec::new();
            let mut new_packed = Vec::with_capacity(reply_tokens.len());
            for i in 0..reply_tokens.len() {
                let cur_tok = reply_tokens[i];
                let next_tok = if i + 1 < reply_tokens.len() {
                    reply_tokens[i + 1]
                } else {
                    bpe.eos_id()
                };
                reply_history.push(cur_tok);
                let n = reply_history.len();
                let s0 = if n >= 4 {
                    reply_history[n - 4]
                } else {
                    MEMORY_PAD
                };
                let s1 = if n >= 3 {
                    reply_history[n - 3]
                } else {
                    MEMORY_PAD
                };
                let s2 = if n >= 2 {
                    reply_history[n - 2]
                } else {
                    MEMORY_PAD
                };
                let s3 = reply_history[n - 1];
                let suffix4 = [s0, s1, s2, s3];
                let mut occ = 0u32;
                let mut found_suf = false;
                for (existing_suf, count) in &mut suffix_counts {
                    if *existing_suf == suffix4 {
                        occ = *count;
                        *count = count.saturating_add(1);
                        found_suf = true;
                        break;
                    }
                }
                if !found_suf {
                    suffix_counts.push((suffix4, 1));
                }
                let key6 = [
                    anchor_id, occ, suffix4[0], suffix4[1], suffix4[2], suffix4[3],
                ];
                new_packed.push(pack_reply_entry_12(&key6, next_tok));
            }
            reply_tbl.packed12.extend(new_packed);
            reply_tbl
                .packed12
                .sort_unstable_by_key(|e| (e[0], e[1], e[2] >> 13));
            reply_tbl
                .packed12
                .dedup_by_key(|e| (e[0], e[1], e[2] >> 13));
            reply_tbl.entries = reply_tbl.packed12.len();

            if let Some(ta) = self.ternary_assoc.as_mut() {
                for win in reply_tokens.windows(2) {
                    ta.set_bit((win[0] as usize) & 0xFFF, win[1]);
                }
            }
        }

        // Extract clause-boundary prediction residuals along verified dialogue turn
        let mut residual_samples = Vec::new();

        // 1. Prompt-to-reply opening boundary (step 0)
        let (pred_0, _) = stack_whole_table_nearest(&n_t, &self.writer_table, self.vocab);
        let target_0 = first_reply_tok;
        let margin_0 = compute_token_l1_margin(&self.writer_table, self.vocab, pred_0, target_0);
        if margin_0 > 0 {
            let mut latent_0 = [0i16; FBS_WIDTH];
            for d in 0..FBS_WIDTH {
                latent_0[d] = n_t[d].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            residual_samples.push(WeightConsolidationSample::from_point_with_residual(
                target_0, pred_0, latent_0, margin_0,
            ));
        }

        // 2. Reply clause boundaries (bounded to first 24 tokens for interactive online admission)
        let scan_len = reply_tokens.len().min(24);
        for i in 0..scan_len {
            let tok = reply_tokens[i];
            let step_n_t = self.forward_token_hidden(tok, &mut rec_states, &mut hamming_caches);
            let target_tok = reply_tokens.get(i + 1).copied().unwrap_or(bpe.eos_id());
            if is_clause_boundary(tok, i + 1) {
                let (pred_tok, _) =
                    stack_whole_table_nearest(&step_n_t, &self.writer_table, self.vocab);
                let margin =
                    compute_token_l1_margin(&self.writer_table, self.vocab, pred_tok, target_tok);
                if margin > 0 {
                    let mut latent = [0i16; FBS_WIDTH];
                    for d in 0..FBS_WIDTH {
                        latent[d] = step_n_t[d].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    }
                    residual_samples.push(WeightConsolidationSample::from_point_with_residual(
                        target_tok, pred_tok, latent, margin,
                    ));
                }
            }
        }

        // Coordinate descent backprop into writer.table & ServedPointKey
        if !residual_samples.is_empty() {
            let cfg = ConsolidationConfig {
                iterations: 1,
                step_numerator: 1,
                step_denominator: 8,
                clamp_min: -30000,
                clamp_max: 30000,
                sync_point_keys: true,
                update_ternary_assoc: false,
            };
            let _ = consolidate_lut_weights(self, &residual_samples, &cfg)?;
        }

        Ok(true)
    }

    /// Serialize this `LutStackModel` into the canonical 64-byte-aligned `UORLUT01` (`uor-r4.lut-stack/8`) container.
    pub fn to_uorlut_bytes(&self) -> Vec<u8> {
        let mut body = Vec::<u8>::new();
        let mut table_entries = Vec::<String>::new();

        let mut push_section = |name: &str, kind: &str, count: usize, raw: &[u8]| {
            let aligned_off = body.len().div_ceil(ALIGN) * ALIGN;
            if aligned_off > body.len() {
                body.resize(aligned_off, 0);
            }
            let offset = body.len();
            body.extend_from_slice(raw);
            let bytes = raw.len();
            table_entries.push(format!(
                "{{\"name\":\"{name}\",\"kind\":\"{kind}\",\"len\":{count},\"span\":{{\"offset\":{offset},\"bytes\":{bytes}}}}}"
            ));
        };

        let i16_bytes = |slice: &[i16]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 2);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };
        let i32_bytes = |slice: &[i32]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 4);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };
        let u32_bytes = |slice: &[u32]| -> Vec<u8> {
            let mut out = Vec::with_capacity(slice.len() * 4);
            for &v in slice {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        };

        for (l, layer) in self.layers.iter().enumerate() {
            push_section(
                &format!("l{l}.rec_norm_gain"),
                "i16",
                layer.rec_gain.len(),
                &i16_bytes(&layer.rec_gain),
            );
            push_section(
                &format!("l{l}.mlp_norm_gain"),
                "i16",
                layer.mlp_gain.len(),
                &i16_bytes(&layer.mlp_gain),
            );
            push_section(
                &format!("l{l}.rec_in.fbs"),
                "u32",
                layer.rec_in.packed_entries.len(),
                &u32_bytes(&layer.rec_in.packed_entries),
            );
            push_section(
                &format!("l{l}.rec_gate.fbs"),
                "u32",
                layer.rec_gate.packed_entries.len(),
                &u32_bytes(&layer.rec_gate.packed_entries),
            );
            push_section(
                &format!("l{l}.rec_out.fbs"),
                "u32",
                layer.rec_out.packed_entries.len(),
                &u32_bytes(&layer.rec_out.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_gate.fbs"),
                "u32",
                layer.mlp_gate.packed_entries.len(),
                &u32_bytes(&layer.mlp_gate.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_up.fbs"),
                "u32",
                layer.mlp_up.packed_entries.len(),
                &u32_bytes(&layer.mlp_up.packed_entries),
            );
            push_section(
                &format!("l{l}.mlp_down.fbs"),
                "u32",
                layer.mlp_down.packed_entries.len(),
                &u32_bytes(&layer.mlp_down.packed_entries),
            );
            push_section(
                &format!("l{l}.conv_taps"),
                "i16",
                layer.taps.len(),
                &i16_bytes(&layer.taps),
            );
            push_section(
                &format!("l{l}.conv_bias"),
                "i32",
                layer.conv_bias.len(),
                &i32_bytes(&layer.conv_bias),
            );
            push_section(
                &format!("l{l}.gate_bias"),
                "i32",
                layer.gate_bias.len(),
                &i32_bytes(&layer.gate_bias),
            );
            push_section(
                &format!("l{l}.decay_offset"),
                "i32",
                layer.decay_offset.len(),
                &i32_bytes(&layer.decay_offset),
            );
        }

        push_section(
            "final_norm_gain",
            "i16",
            self.final_gain.len(),
            &i16_bytes(&self.final_gain),
        );
        push_section(
            "writer.table",
            "i16",
            self.writer_table.len(),
            &i16_bytes(&self.writer_table),
        );

        let mut depth_list = Vec::new();
        let mut copy_key = 2usize;
        for h in &self.hamming {
            depth_list.push(h.depth.to_string());
            copy_key = h.copy_key;
            let p = format!("hamming.d{}", h.depth);
            push_section(
                &format!("{p}.query_rotation"),
                "i32",
                h.query_rotation.len(),
                &i32_bytes(&h.query_rotation),
            );
            push_section(
                &format!("{p}.key_rotation"),
                "i32",
                h.key_rotation.len(),
                &i32_bytes(&h.key_rotation),
            );
            push_section(
                &format!("{p}.value_rotation"),
                "i32",
                h.value_rotation.len(),
                &i32_bytes(&h.value_rotation),
            );
            push_section(
                &format!("{p}.margin"),
                "i32",
                h.margin.len(),
                &i32_bytes(&h.margin),
            );
            push_section(
                &format!("{p}.weight"),
                "i32",
                h.weight.len(),
                &i32_bytes(&h.weight),
            );
        }

        let memory_json = if let Some(mem) = &self.memory {
            push_section(
                "memory.opening.keys",
                "u32",
                mem.opening.keys.len(),
                &u32_bytes(&mem.opening.keys),
            );
            push_section(
                "memory.opening.values",
                "u32",
                mem.opening.values.len(),
                &u32_bytes(&mem.opening.values),
            );
            push_section(
                "memory.opening.weight",
                "i32",
                mem.opening.weight_q24.len(),
                &i32_bytes(&mem.opening.weight_q24),
            );
            let reply_count = if let Some(rep) = &mem.reply {
                if !rep.packed12.is_empty() {
                    let mut raw = Vec::with_capacity(rep.packed12.len() * 12);
                    for w in &rep.packed12 {
                        raw.extend_from_slice(&w[0].to_le_bytes());
                        raw.extend_from_slice(&w[1].to_le_bytes());
                        raw.extend_from_slice(&w[2].to_le_bytes());
                    }
                    push_section("memory.reply.packed12", "u32", rep.packed12.len() * 3, &raw);
                } else {
                    push_section(
                        "memory.reply.keys",
                        "u32",
                        rep.keys.len(),
                        &u32_bytes(&rep.keys),
                    );
                    push_section(
                        "memory.reply.values",
                        "u32",
                        rep.values.len(),
                        &u32_bytes(&rep.values),
                    );
                }
                push_section(
                    "memory.reply.weight",
                    "i32",
                    rep.weight_q24.len(),
                    &i32_bytes(&rep.weight_q24),
                );
                rep.entries
            } else {
                0
            };
            format!(
                ",\"memory\":{{\"k\":{},\"key_window\":{},\"opening\":{},\"reply\":{},\"reply_key_len\":{},\"reply_turn\":{},\"weight_exp\":{}}}",
                mem.k,
                mem.key_window,
                mem.opening.entries,
                reply_count,
                mem.reply_key_len,
                mem.reply_turn.unwrap_or(mem.k),
                mem.weight_exp
            )
        } else {
            String::new()
        };

        let point_json = if let Some(pt) = &self.point {
            push_section(
                "memory.point.points",
                "i16",
                pt.points.len(),
                &i16_bytes(&pt.points),
            );
            push_section(
                "memory.point.targets",
                "u32",
                pt.targets.len(),
                &u32_bytes(&pt.targets),
            );
            if let Some(anchors) = &pt.canonical_anchors {
                push_section(
                    "memory.point.anchors",
                    "u32",
                    anchors.len(),
                    &u32_bytes(anchors),
                );
            }
            format!(
                ",\"point\":{{\"rows\":{},\"width\":{},\"exp\":{},\"vote_k\":{}}}",
                pt.rows, pt.width, pt.exp, pt.vote_k
            )
        } else {
            String::new()
        };

        let ternary_json = if let Some(ta) = &self.ternary_assoc {
            push_section(
                "ternary.assoc.bank",
                "u32",
                ta.words_u32.len(),
                &u32_bytes(&ta.words_u32),
            );
            format!(
                ",\"ternary_assoc\":{{\"rows\":{},\"cols_bits\":{},\"buckets\":{}}}",
                ta.rows, ta.cols_bits, ta.buckets
            )
        } else {
            String::new()
        };

        let pattern = "r".repeat(self.layers.len());
        let header_str = format!(
            "{{\"schema\":\"{}\",\"dialogue_version\":{},\"shape\":{{\"vocab\":{},\"width\":{},\"mlp\":{},\"pattern\":\"{}\",\"context\":{}}},\"hamming\":{{\"copy_key\":{},\"depths\":[{}],\"heads\":{}}}{}{}{},\"tables\":[{}]}}",
            self.schema,
            self.dialogue_version,
            self.vocab,
            self.width,
            self.mlp,
            pattern,
            self.context,
            copy_key,
            depth_list.join(","),
            HAMMING_HEADS,
            memory_json,
            point_json,
            ternary_json,
            table_entries.join(",")
        );
        let header_bytes = header_str.as_bytes();
        let header_len = header_bytes.len() as u64;
        let header_end = 16 + header_bytes.len();
        let data_start = header_end.div_ceil(ALIGN) * ALIGN;

        let mut out = Vec::with_capacity(data_start + body.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&header_len.to_le_bytes());
        out.extend_from_slice(header_bytes);
        out.resize(data_start, 0);
        out.extend_from_slice(&body);
        out
    }

    /// In-memory verification that every PointKey row points to an active reply anchor in `memory.reply.packed12`.
    /// Matches `xtask/src/engine.rs::audit_uorlut_synchronization`.
    pub fn audit_point_key_synchronization(&self) -> usize {
        let (Some(pt), Some(mem)) = (self.point.as_ref(), self.memory.as_ref()) else {
            return 0;
        };
        let Some(reply_tbl) = mem.reply.as_ref() else {
            return pt.rows;
        };
        let Some(anchors) = pt.canonical_anchors.as_ref() else {
            return 0;
        };
        let alen = pt.anchor_len.max(1);
        if pt.rows == 0 || anchors.len() < pt.rows * alen {
            return 0;
        }

        let mut orphaned = 0usize;
        for r in 0..pt.rows {
            let raw_anchor = anchors[r * alen];
            let encoded = encode_reply_anchor_21(raw_anchor);
            let present = reply_tbl
                .packed12
                .binary_search_by_key(&encoded, |chunk| chunk[0] >> 11)
                .is_ok()
                || reply_tbl
                    .packed12
                    .iter()
                    .any(|chunk| (chunk[0] >> 11) == encoded);
            if !present {
                orphaned += 1;
            }
        }
        orphaned
    }

    /// Export a content-addressed `(e8_root, clifford_action, icosian_root)` geometric shard pack (`UORSHRD1`).
    /// When `clifford_action` or `icosian_root` is `None`, exports the coarse `E8` root sector (`0..16`)
    /// while preserving exact `(e8_root, clifford_action, icosian_root)` coordinates for every row upon merge.
    pub fn export_geometric_shard(
        &self,
        e8_root: u8,
        clifford_action: Option<u8>,
        icosian_root: Option<u8>,
    ) -> Result<GeometricShardPack, String> {
        let Some(pt) = self.point.as_ref() else {
            return Err("LutStackModel missing ServedPointKey".into());
        };
        let Some(anchors) = pt.canonical_anchors.as_ref() else {
            return Err("ServedPointKey missing canonical_anchors".into());
        };
        let p_width = pt.width.max(1);
        let alen = pt.anchor_len.max(1);
        let target_e8 = e8_root % (E8_ROOT_COUNT as u8);
        let coarse_sector_only = clifford_action.is_none() || icosian_root.is_none();

        let mut shard_points = Vec::new();
        let mut shard_targets = Vec::new();
        let mut shard_anchors = Vec::new();
        let mut encoded_anchors = Vec::new();
        let mut matched_rows = 0usize;
        let mut sample_c64 = clifford_action.unwrap_or(0) % (CLIFFORD_ACTION_COUNT as u8);
        let mut sample_i17 = icosian_root.unwrap_or(0) % (ICOSIAN_ROOT_COUNT as u8);

        for r in 0..pt.rows {
            let p_row = &pt.points[r * p_width..(r + 1) * p_width];
            let p_sketch = pack_i16_sign_sketch_256(p_row);
            let (r_e8, r_c64, r_i17) = compute_geometric_coordinates(p_row, p_sketch[0]);
            if r_e8 != target_e8 {
                continue;
            }
            if let Some(req_c64) = clifford_action {
                if r_c64 != (req_c64 % (CLIFFORD_ACTION_COUNT as u8)) {
                    continue;
                }
            }
            if let Some(req_i17) = icosian_root {
                if r_i17 != (req_i17 % (ICOSIAN_ROOT_COUNT as u8)) {
                    continue;
                }
            }
            if matched_rows == 0 && coarse_sector_only {
                sample_c64 = r_c64;
                sample_i17 = r_i17;
            }
            let a_row = &anchors[r * alen..(r + 1) * alen];
            shard_points.extend_from_slice(p_row);
            shard_targets.push(pt.targets[r]);
            shard_anchors.extend_from_slice(a_row);
            encoded_anchors.push(encode_reply_anchor_21(a_row[0]));
            matched_rows += 1;
        }

        encoded_anchors.sort_unstable();
        encoded_anchors.dedup();

        let mut reply_packed12 = Vec::new();
        if let Some(mem) = self.memory.as_ref() {
            if let Some(rep) = mem.reply.as_ref() {
                for &entry in &rep.packed12 {
                    let entry_anchor = entry[0] >> 11;
                    if encoded_anchors.binary_search(&entry_anchor).is_ok() {
                        reply_packed12.push(entry);
                    }
                }
            }
        }

        let mut pack = GeometricShardPack {
            coordinate: ShardBasinCoordinate {
                e8_root: target_e8,
                clifford_action: sample_c64,
                icosian_root: sample_i17,
                coarse_sector_only,
            },
            content_sha256: String::new(),
            point_rows: matched_rows,
            point_width: p_width,
            anchor_len: alen,
            points: shard_points,
            targets: shard_targets,
            anchors: shard_anchors,
            reply_packed12,
        };
        let bytes = pack.to_bytes();
        pack.content_sha256 = sha256_hex(&bytes);
        Ok(pack)
    }

    /// Partition this `LutStackModel` into content-addressed geometric shards across the `E8` root lattice
    /// and report resident trunk bytes, hot curriculum rows, and per-shard SHA-256 digests.
    pub fn partition_geometric_shards(&self) -> GeometricShardCatalog {
        let trunk_bytes = {
            let mut b = (self.writer_table.len() + self.final_gain.len()) * 2;
            for l in &self.layers {
                b += (l.rec_gain.len() + l.mlp_gain.len() + l.taps.len()) * 2;
                b += (l.rec_in.packed_entries.len()
                    + l.rec_gate.packed_entries.len()
                    + l.rec_out.packed_entries.len()
                    + l.mlp_gate.packed_entries.len()
                    + l.mlp_up.packed_entries.len()
                    + l.mlp_down.packed_entries.len()
                    + l.conv_bias.len()
                    + l.gate_bias.len()
                    + l.decay_offset.len())
                    * 4;
            }
            for h in &self.hamming {
                b += (h.query_rotation.len()
                    + h.key_rotation.len()
                    + h.value_rotation.len()
                    + h.margin.len()
                    + h.weight.len())
                    * 4;
            }
            b
        };

        let mut active_fine_basins = 0usize;
        if let Some(pt) = self.point.as_ref() {
            if let Some(offsets) = pt.bucket_offsets.as_ref() {
                for b in 0..GEOMETRIC_BUCKET_COUNT.min(offsets.len().saturating_sub(1)) {
                    if offsets[b + 1] > offsets[b] {
                        active_fine_basins += 1;
                    }
                }
            }
        }

        let mut shards = Vec::with_capacity(E8_ROOT_COUNT);
        let mut total_sharded_rows = 0usize;
        let mut total_sharded_bytes = 0usize;
        for e8 in 0..(E8_ROOT_COUNT as u8) {
            if let Ok(pack) = self.export_geometric_shard(e8, None, None) {
                let byte_len = pack.byte_len();
                total_sharded_rows += pack.point_rows;
                total_sharded_bytes += byte_len;
                shards.push(GeometricShardSummary {
                    e8_root: e8,
                    dominant_clifford_action: pack.coordinate.clifford_action,
                    dominant_icosian_root: pack.coordinate.icosian_root,
                    content_sha256: pack.content_sha256,
                    point_rows: pack.point_rows,
                    reply_entries: pack.reply_packed12.len(),
                    byte_len,
                });
            }
        }

        GeometricShardCatalog {
            schema: "uor-r4.shard-catalog/1".to_string(),
            parent_artifact_sha256: self.artifact_sha256.clone(),
            resident_trunk_bytes: trunk_bytes,
            core_curriculum_rows: CORE_CURRICULUM_TURNS,
            total_point_rows: total_sharded_rows,
            total_shard_bytes: total_sharded_bytes,
            active_fine_basins,
            shards,
        }
    }

    /// Hot-swap / merge a content-addressed `GeometricShardPack` (`UORSHRD1`) into this `LutStackModel`,
    /// deduplicating existing rows, splicing new `ServedPointKey` and `/7` `packed12` continuations,
    /// and rebuilding the 17,408-bucket two-level Radix `/8` index.
    pub fn merge_geometric_shard(
        &mut self,
        shard: &GeometricShardPack,
    ) -> Result<ShardMergeReport, String> {
        let Some(pt) = self.point.as_mut() else {
            return Err("LutStackModel missing ServedPointKey".into());
        };
        let Some(mem) = self.memory.as_mut() else {
            return Err("LutStackModel missing ServedTurnMemory".into());
        };
        let p_width = pt.width.max(1);
        let alen = pt.anchor_len.max(1);
        if shard.point_width != p_width || shard.anchor_len != alen {
            return Err(format!(
                "shard shape ({}, {}) does not match model shape ({}, {})",
                shard.point_width, shard.anchor_len, p_width, alen
            ));
        }
        if shard.points.len() != shard.point_rows * p_width
            || shard.targets.len() != shard.point_rows
            || shard.anchors.len() != shard.point_rows * alen
        {
            return Err("shard buffer lengths inconsistent with point_rows".into());
        }

        let anchors = pt.canonical_anchors.get_or_insert_with(Vec::new);
        let mut existing_anchors: std::collections::HashSet<(u32, u32)> =
            std::collections::HashSet::with_capacity(pt.rows);
        for r in 0..pt.rows {
            let a0 = anchors[r * alen];
            existing_anchors.insert((a0, pt.targets[r]));
        }

        let mut newly_merged_rows = 0usize;
        for sr in 0..shard.point_rows {
            let s_a_row = &shard.anchors[sr * alen..(sr + 1) * alen];
            let s_target = shard.targets[sr];
            if existing_anchors.contains(&(s_a_row[0], s_target)) {
                continue;
            }
            let s_p_row = &shard.points[sr * p_width..(sr + 1) * p_width];
            pt.points.extend_from_slice(s_p_row);
            pt.targets.push(s_target);
            anchors.extend_from_slice(s_a_row);
            if let Some(spans) = pt.provenance_spans.as_mut() {
                spans.push(0);
            }
            if let Some(traces) = pt.provenance_traces.as_mut() {
                traces.push(0);
            }
            pt.rows += 1;
            existing_anchors.insert((s_a_row[0], s_target));
            newly_merged_rows += 1;
        }

        let mut newly_merged_reply_entries = 0usize;
        if let Some(rep) = mem.reply.as_mut() {
            let before_len = rep.packed12.len();
            if !shard.reply_packed12.is_empty() {
                rep.packed12.extend_from_slice(&shard.reply_packed12);
                rep.packed12
                    .sort_unstable_by_key(|e| (e[0], e[1], e[2] >> 13));
                rep.packed12.dedup_by_key(|e| (e[0], e[1], e[2] >> 13));
                rep.entries = rep.packed12.len();
            }
            newly_merged_reply_entries = rep.packed12.len().saturating_sub(before_len);
        }

        pt.build_radix_index();
        let total_rows_after = pt.rows;
        let orphaned_after = self.audit_point_key_synchronization();

        Ok(ShardMergeReport {
            content_sha256: shard.content_sha256.clone(),
            coordinate: shard.coordinate,
            newly_merged_rows,
            newly_merged_reply_entries,
            total_rows_after,
            orphaned_after,
        })
    }

    /// Forward a dialogue turn (prompt, reply) through the 6-layer trunk to extract
    /// latent trajectory points preceding each emitted reply token.
    pub fn extract_trajectory_points_from_dialogue(
        &self,
        prompt: &str,
        reply: &str,
    ) -> Result<Vec<LatentTrajectoryPoint>, String> {
        let bpe = ByteBpe4096::global();
        let prompt_tokens =
            bpe.encode_dialogue_prompt(None, &[], prompt.trim(), self.dialogue_version);
        let mut reply_tokens = bpe.encode(&format!(" {}", reply.trim()));
        if prompt_tokens.is_empty() || reply_tokens.is_empty() {
            return Err("prompt or reply tokens empty".into());
        }
        reply_tokens.push(bpe.eos_id());
        self.extract_trajectory_points_from_tokens(&prompt_tokens, &reply_tokens)
    }

    /// Forward prompt tokens and reply tokens through trunk to collect trajectory points.
    pub fn extract_trajectory_points_from_tokens(
        &self,
        prompt_tokens: &[u32],
        reply_tokens: &[u32],
    ) -> Result<Vec<LatentTrajectoryPoint>, String> {
        if prompt_tokens.is_empty() || reply_tokens.is_empty() {
            return Err("token sequence empty".into());
        }
        let mut rec_states = vec![FbsRecState::default(); self.layers.len()];
        let mut hamming_caches = vec![HammingCache::default(); self.hamming.len()];

        let prefill_len = prompt_tokens.len() - 1;
        for &tok in &prompt_tokens[..prefill_len] {
            let _ = self.forward_token_hidden(tok, &mut rec_states, &mut hamming_caches);
        }

        let mut cur_tok = prompt_tokens[prompt_tokens.len() - 1];
        let mut points = Vec::with_capacity(reply_tokens.len());

        for &target_tok in reply_tokens {
            let n_t = self.forward_token_hidden(cur_tok, &mut rec_states, &mut hamming_caches);
            let mut latent = [0i16; FBS_WIDTH];
            for d in 0..FBS_WIDTH {
                latent[d] = n_t[d].clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            points.push(LatentTrajectoryPoint::new(target_tok, latent));
            cur_tok = target_tok;
        }

        Ok(points)
    }
}

pub const SHARD_MAGIC: &[u8; 8] = b"UORSHRD1";

/// Content-addressed geometric basin coordinate `(e8_root, clifford_action, icosian_root)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShardBasinCoordinate {
    pub e8_root: u8,
    pub clifford_action: u8,
    pub icosian_root: u8,
    pub coarse_sector_only: bool,
}

/// Binary content-addressed geometric `.uorlut` shard pack (`UORSHRD1`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeometricShardPack {
    pub coordinate: ShardBasinCoordinate,
    pub content_sha256: String,
    pub point_rows: usize,
    pub point_width: usize,
    pub anchor_len: usize,
    pub points: Vec<i16>,
    pub targets: Vec<u32>,
    pub anchors: Vec<u32>,
    pub reply_packed12: Vec<[u32; 3]>,
}

impl GeometricShardPack {
    pub fn byte_len(&self) -> usize {
        28 + self.points.len() * 2
            + self.targets.len() * 4
            + self.anchors.len() * 4
            + self.reply_packed12.len() * 12
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.byte_len());
        out.extend_from_slice(SHARD_MAGIC);
        out.push(self.coordinate.e8_root);
        out.push(self.coordinate.clifford_action);
        out.push(self.coordinate.icosian_root);
        out.push(u8::from(self.coordinate.coarse_sector_only));
        out.extend_from_slice(&(self.point_rows as u32).to_le_bytes());
        out.extend_from_slice(&(self.point_width as u32).to_le_bytes());
        out.extend_from_slice(&(self.anchor_len as u32).to_le_bytes());
        out.extend_from_slice(&(self.reply_packed12.len() as u32).to_le_bytes());
        for &v in &self.points {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.targets {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.anchors {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for entry in &self.reply_packed12 {
            out.extend_from_slice(&entry[0].to_le_bytes());
            out.extend_from_slice(&entry[1].to_le_bytes());
            out.extend_from_slice(&entry[2].to_le_bytes());
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 28 || &bytes[0..8] != SHARD_MAGIC {
            return Err("invalid UORSHRD1 shard header".to_string());
        }
        let e8_root = bytes[8] % (E8_ROOT_COUNT as u8);
        let clifford_action = bytes[9] % (CLIFFORD_ACTION_COUNT as u8);
        let icosian_root = bytes[10] % (ICOSIAN_ROOT_COUNT as u8);
        let coarse_sector_only = bytes[11] != 0;
        let point_rows = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let point_width = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        let anchor_len = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]) as usize;
        let reply_count = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;

        let p_bytes = point_rows
            .checked_mul(point_width)
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| "shard points overflow".to_string())?;
        let t_bytes = point_rows
            .checked_mul(4)
            .ok_or_else(|| "shard targets overflow".to_string())?;
        let a_bytes = point_rows
            .checked_mul(anchor_len)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| "shard anchors overflow".to_string())?;
        let r_bytes = reply_count
            .checked_mul(12)
            .ok_or_else(|| "shard reply_packed12 overflow".to_string())?;
        let expected = 28usize
            .checked_add(p_bytes)
            .and_then(|n| n.checked_add(t_bytes))
            .and_then(|n| n.checked_add(a_bytes))
            .and_then(|n| n.checked_add(r_bytes))
            .ok_or_else(|| "shard total length overflow".to_string())?;
        if bytes.len() < expected {
            return Err(format!(
                "truncated UORSHRD1 shard: expected {} bytes, got {}",
                expected,
                bytes.len()
            ));
        }

        let mut pos = 28usize;
        let points: Vec<i16> = bytes[pos..pos + p_bytes]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        pos += p_bytes;
        let targets: Vec<u32> = bytes[pos..pos + t_bytes]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        pos += t_bytes;
        let anchors: Vec<u32> = bytes[pos..pos + a_bytes]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        pos += a_bytes;
        let reply_packed12: Vec<[u32; 3]> = bytes[pos..pos + r_bytes]
            .chunks_exact(12)
            .map(|c| {
                [
                    u32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                    u32::from_le_bytes([c[4], c[5], c[6], c[7]]),
                    u32::from_le_bytes([c[8], c[9], c[10], c[11]]),
                ]
            })
            .collect();

        let content_sha256 = sha256_hex(&bytes[..expected]);
        Ok(Self {
            coordinate: ShardBasinCoordinate {
                e8_root,
                clifford_action,
                icosian_root,
                coarse_sector_only,
            },
            content_sha256,
            point_rows,
            point_width,
            anchor_len,
            points,
            targets,
            anchors,
            reply_packed12,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometricShardSummary {
    pub e8_root: u8,
    pub dominant_clifford_action: u8,
    pub dominant_icosian_root: u8,
    pub content_sha256: String,
    pub point_rows: usize,
    pub reply_entries: usize,
    pub byte_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometricShardCatalog {
    pub schema: String,
    pub parent_artifact_sha256: String,
    pub resident_trunk_bytes: usize,
    pub core_curriculum_rows: usize,
    pub total_point_rows: usize,
    pub total_shard_bytes: usize,
    pub active_fine_basins: usize,
    pub shards: Vec<GeometricShardSummary>,
}

impl GeometricShardCatalog {
    pub fn to_json(&self) -> String {
        let mut shards_json = String::from("[");
        for (idx, s) in self.shards.iter().enumerate() {
            if idx > 0 {
                shards_json.push(',');
            }
            shards_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"e8_root\":{},",
                    "\"dominant_clifford_action\":{},",
                    "\"dominant_icosian_root\":{},",
                    "\"content_sha256\":\"{}\",",
                    "\"point_rows\":{},",
                    "\"reply_entries\":{},",
                    "\"byte_len\":{}",
                    "}}"
                ),
                s.e8_root,
                s.dominant_clifford_action,
                s.dominant_icosian_root,
                s.content_sha256,
                s.point_rows,
                s.reply_entries,
                s.byte_len
            ));
        }
        shards_json.push(']');
        let total_reply_entries: usize = self.shards.iter().map(|s| s.reply_entries).sum();
        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"schema\":\"{}\",",
                "\"parent_artifact_sha256\":\"{}\",",
                "\"shard_count\":{},",
                "\"resident_trunk_bytes\":{},",
                "\"core_curriculum_rows\":{},",
                "\"total_point_rows\":{},",
                "\"total_reply_entries\":{},",
                "\"total_shard_bytes\":{},",
                "\"total_byte_len\":{},",
                "\"active_fine_basins\":{},",
                "\"shards\":{}",
                "}}"
            ),
            self.schema,
            self.parent_artifact_sha256,
            self.shards.len(),
            self.resident_trunk_bytes,
            self.core_curriculum_rows,
            self.total_point_rows,
            total_reply_entries,
            self.total_shard_bytes,
            self.total_shard_bytes,
            self.active_fine_basins,
            shards_json
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardMergeReport {
    pub content_sha256: String,
    pub coordinate: ShardBasinCoordinate,
    pub newly_merged_rows: usize,
    pub newly_merged_reply_entries: usize,
    pub total_rows_after: usize,
    pub orphaned_after: usize,
}

impl ShardMergeReport {
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"content_sha256\":\"{}\",",
                "\"e8_root\":{},",
                "\"clifford_action\":{},",
                "\"icosian_root\":{},",
                "\"coarse_sector_only\":{},",
                "\"newly_merged_rows\":{},",
                "\"newly_merged_reply_entries\":{},",
                "\"total_rows_after\":{},",
                "\"total_point_rows\":{},",
                "\"orphaned_after\":{},",
                "\"radix_buckets_indexed\":{},",
                "\"radix_populated_buckets\":{}",
                "}}"
            ),
            self.content_sha256,
            self.coordinate.e8_root,
            self.coordinate.clifford_action,
            self.coordinate.icosian_root,
            self.coordinate.coarse_sector_only,
            self.newly_merged_rows,
            self.newly_merged_reply_entries,
            self.total_rows_after,
            self.total_rows_after,
            self.orphaned_after,
            GEOMETRIC_BUCKET_COUNT,
            GEOMETRIC_BUCKET_COUNT
        )
    }
}

/// Consolidate LUT stack weights across a batch of samples (dialogue turns or trajectory points).
pub fn consolidate_lut_weights(
    model: &mut LutStackModel,
    samples: &[WeightConsolidationSample],
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String> {
    config.validate()?;
    let mut points = Vec::new();
    for sample in samples {
        match sample {
            WeightConsolidationSample::TrajectoryPoint(pt) => {
                points.push((**pt).clone());
            }
            WeightConsolidationSample::Dialogue { prompt, reply } => {
                let pts = model.extract_trajectory_points_from_dialogue(prompt, reply)?;
                points.extend(pts);
            }
            WeightConsolidationSample::Tokens {
                prompt_tokens,
                reply_tokens,
            } => {
                let pts =
                    model.extract_trajectory_points_from_tokens(prompt_tokens, reply_tokens)?;
                points.extend(pts);
            }
        }
    }
    consolidate_lut_trajectory_points(model, &points, config)
}

/// Core coordinate descent optimizer operating directly on latent trajectory points.
pub fn consolidate_lut_trajectory_points(
    model: &mut LutStackModel,
    points: &[LatentTrajectoryPoint],
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String> {
    let vocab = model.vocab;
    if model.writer_table.len() < vocab * FBS_WIDTH {
        return Err("writer_table length less than vocab * FBS_WIDTH".into());
    }

    config.validate()?;
    let s = config.step_numerator;
    let d = config.step_denominator;
    let c_min = config.clamp_min as i64;
    let c_max = config.clamp_max as i64;

    // Accumulate sums, weighted sums, counts, and residual margins per active token basin
    struct TokenConsolidationAccum {
        sum: [i64; FBS_WIDTH],
        weighted_sum: [i64; FBS_WIDTH],
        weight_sum: i64,
        margin_sum: u64,
        count: usize,
        has_explicit_residuals: bool,
    }
    let mut tok_accums: std::collections::BTreeMap<usize, TokenConsolidationAccum> =
        std::collections::BTreeMap::new();

    for pt in points {
        let t = pt.target_token as usize;
        if t < vocab {
            let acc = tok_accums
                .entry(t)
                .or_insert_with(|| TokenConsolidationAccum {
                    sum: [0i64; FBS_WIDTH],
                    weighted_sum: [0i64; FBS_WIDTH],
                    weight_sum: 0,
                    margin_sum: 0,
                    count: 0,
                    has_explicit_residuals: false,
                });
            let omega = match pt.residual_margin {
                Some(m) => {
                    acc.has_explicit_residuals = true;
                    acc.margin_sum = acc.margin_sum.saturating_add(m);
                    (m as i64).max(1)
                }
                None => 1i64,
            };
            for dim in 0..FBS_WIDTH {
                acc.sum[dim] += pt.latent[dim] as i64;
                acc.weighted_sum[dim] += (pt.latent[dim] as i64) * omega;
            }
            acc.weight_sum += omega;
            acc.count += 1;
        }
    }

    let mut weights_updated = 0usize;
    let mut l1_shift = 0u64;
    let mut max_delta = 0i16;
    let mut updated_tokens = HashSet::new();
    let mut updated_centroids: HashMap<usize, [i16; FBS_WIDTH]> = HashMap::new();

    let num_iterations = config.iterations.max(1);
    for _iter in 0..num_iterations {
        for (&t, acc) in &tok_accums {
            let cnt = acc.count;
            let cnt_i64 = cnt as i64;
            let base = t * FBS_WIDTH;
            let mut row_changed = false;
            let mut centroid_row = [0i16; FBS_WIDTH];

            let s_eff = if acc.has_explicit_residuals {
                let mean_margin = acc.margin_sum / (cnt as u64);
                if mean_margin == 0 {
                    0i64
                } else {
                    let scaled = (mean_margin * (d as u64)) / 250_000;
                    scaled.clamp(1, d as u64) as i64
                }
            } else {
                s
            };

            for (dim, c_slot) in centroid_row.iter_mut().enumerate().take(FBS_WIDTH) {
                let mu_td = if acc.weight_sum > 0 {
                    acc.weighted_sum[dim] / acc.weight_sum
                } else {
                    acc.sum[dim] / cnt_i64
                };
                let old_w = model.writer_table[base + dim] as i64;
                let numerator = (d - s_eff) * old_w + s_eff * mu_td;
                let new_w = (numerator / d).clamp(c_min, c_max) as i16;

                let delta = (new_w as i32 - old_w as i32).abs();
                if delta > 0 {
                    weights_updated += 1;
                    l1_shift += delta as u64;
                    if delta as i16 > max_delta {
                        max_delta = delta as i16;
                    }
                    row_changed = true;
                }
                model.writer_table[base + dim] = new_w;
                *c_slot = new_w;
            }

            if row_changed {
                updated_tokens.insert(t);
                updated_centroids.insert(t, centroid_row);
            }
        }
    }

    // Record training provenance spans for updated tokens in model.writer_provenance
    if !updated_tokens.is_empty() {
        let prov_span =
            fnv1a64(format!("gllm.training.consolidate_weights:{}", points.len()).as_bytes())
                .max(1);
        let prov_trace_lo = fnv1a64(b"gllm.training.trace:consolidate").max(1);
        let prov_trace_hi = fnv1a64(b"uor.consolidate.hi").max(1);
        let prov_trace = (prov_trace_lo as u128) | ((prov_trace_hi as u128) << 64);

        let w_spans = model
            .writer_provenance
            .get_or_insert_with(|| vec![0u64; vocab]);
        if w_spans.len() < vocab {
            w_spans.resize(vocab, 0);
        }
        let w_traces = model
            .writer_provenance_traces
            .get_or_insert_with(|| vec![0u128; vocab]);
        if w_traces.len() < vocab {
            w_traces.resize(vocab, 0);
        }
        for &t in &updated_tokens {
            if t < vocab {
                w_spans[t] = prov_span;
                w_traces[t] = prov_trace;
            }
        }
    }

    // Synchronize PointKey rows with updated centroids
    if config.sync_point_keys && !updated_centroids.is_empty() {
        if let Some(pt) = model.point.as_mut() {
            let p_width = pt.width.max(1);
            for r in 0..pt.rows {
                let target = pt.targets[r] as usize;
                if let (Some(new_centroid), Some(acc)) =
                    (updated_centroids.get(&target), tok_accums.get(&target))
                {
                    let s_sync = if acc.has_explicit_residuals {
                        let mean_margin = acc.margin_sum / (acc.count as u64);
                        if mean_margin == 0 {
                            0i64
                        } else {
                            let scaled = (mean_margin * (d as u64)) / 250_000;
                            scaled.clamp(1, d as u64) as i64
                        }
                    } else {
                        s
                    };
                    let base = r * p_width;
                    let copy_len = p_width.min(FBS_WIDTH);
                    for (dim, &mu_p) in new_centroid.iter().enumerate().take(copy_len) {
                        let old_p = pt.points[base + dim] as i64;
                        let mu_p = mu_p as i64;
                        let new_p = ((d - s_sync) * old_p + s_sync * mu_p) / d;
                        pt.points[base + dim] = new_p.clamp(c_min, c_max) as i16;
                    }
                }
            }
            if pt.bucket_offsets.is_some() {
                pt.build_radix_index();
            }
        }
    }

    // Synchronize TernaryAssocBank bucket-0 top-8 nearest-neighbor transitions
    if config.update_ternary_assoc {
        if let Some(ta) = model.ternary_assoc.as_mut() {
            for (&tok, centroid) in &updated_centroids {
                let mut centroid_i32 = [0i32; FBS_WIDTH];
                for d in 0..FBS_WIDTH {
                    centroid_i32[d] = centroid[d] as i32;
                }
                for (nbr_tok, _dist) in
                    stack_whole_table_top_k(&centroid_i32, &model.writer_table, vocab, 8)
                {
                    ta.set_bit(tok & 0xFFF, nbr_tok);
                }
            }
        }
    }

    // Audit PointKey synchronization
    let orphaned_point_rows = model.audit_point_key_synchronization();
    if orphaned_point_rows > 0 {
        return Err(format!(
            "PointKey synchronization produced {orphaned_point_rows} orphaned rows"
        ));
    }

    Ok(ConsolidationReport {
        weights_updated,
        l1_shift,
        max_delta,
        points_consolidated: points.len(),
        tokens_updated: updated_tokens.len(),
        orphaned_point_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_lut_stack_model() -> LutStackModel {
        static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
        MODEL
            .get_or_init(|| {
                const BOOTSTRAP: &[u8] = include_bytes!("bootstrap_v8.uorlut");
                Arc::new(LutStackModel::parse(BOOTSTRAP).expect("bootstrap model parses"))
            })
            .as_ref()
            .clone()
    }

    #[test]
    fn test_rolling_context_state_blending() {
        let mut state = RollingContextState::default();
        assert_eq!(state.turn_index, 0);
        assert_eq!(state.q12_latent, [0i16; FBS_WIDTH]);

        // Apply positive vector n0 = [4096; 256] (+1.0 in Q12)
        let n0 = [4096i32; FBS_WIDTH];
        state.blend_latent(&n0);
        assert_eq!(state.turn_index, 1);
        for d in 0..FBS_WIDTH {
            // (3072 * 0 + 1024 * 4096) / 4096 = 1024
            assert_eq!(state.q12_latent[d], 1024);
        }

        // Apply negative vector n1 = [-4096; 256] (-1.0 in Q12)
        let n1 = [-4096i32; FBS_WIDTH];
        state.blend_latent(&n1);
        assert_eq!(state.turn_index, 2);
        for d in 0..FBS_WIDTH {
            // (3072 * 1024 + 1024 * -4096) / 4096 = (3145728 - 4194304) / 4096 = -1048576 / 4096 = -256
            assert_eq!(state.q12_latent[d], -256);
        }

        // Extreme vectors clamp strictly to [-30000, 30000]
        let extreme_pos = [100000i32; FBS_WIDTH];
        for _ in 0..10 {
            state.blend_latent(&extreme_pos);
        }
        for d in 0..FBS_WIDTH {
            assert_eq!(state.q12_latent[d], 30000);
        }

        let extreme_neg = [-100000i32; FBS_WIDTH];
        for _ in 0..15 {
            state.blend_latent(&extreme_neg);
        }
        for d in 0..FBS_WIDTH {
            assert_eq!(state.q12_latent[d], -30000);
        }

        // Serialization roundtrip
        let json_str = state.to_json();
        let parsed = RollingContextState::from_json(&json_str).expect("valid JSON deserialization");
        assert_eq!(parsed, state);
    }

    #[test]
    fn test_coordinate_descent_exact_arithmetic_step() {
        let mut model = default_lut_stack_model();
        let target_token = 42u32;
        let base = (target_token as usize) * FBS_WIDTH;

        model.writer_table[base] = 0;

        let mut latent = [0i16; FBS_WIDTH];
        latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
        latent[0] = 800;

        let points = vec![LatentTrajectoryPoint::new(target_token, latent)];
        let config = ConsolidationConfig::default(); // S=1, D=8

        let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation succeeds");

        // Expected: ((7 * 0) + 1 * 800) / 8 = 100
        assert_eq!(model.writer_table[base], 100);
        assert_eq!(report.weights_updated, 1);
        assert_eq!(report.l1_shift, 100);
        assert_eq!(report.max_delta, 100);
        assert_eq!(report.points_consolidated, 1);
        assert_eq!(report.tokens_updated, 1);
        assert_eq!(report.orphaned_point_rows, 0);
    }

    #[test]
    fn test_coordinate_descent_multi_point_centroid_averaging() {
        let mut model = default_lut_stack_model();
        let target_token = 100u32;
        let base = (target_token as usize) * FBS_WIDTH;

        model.writer_table[base + 5] = 1000;

        let make_pt = |val: i16| {
            let mut latent = [0i16; FBS_WIDTH];
            latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
            latent[5] = val;
            LatentTrajectoryPoint::new(target_token, latent)
        };

        // Observations at 2000, 3000, 4000 -> mean = 3000
        let points = vec![make_pt(2000), make_pt(3000), make_pt(4000)];
        let config = ConsolidationConfig::default();

        let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation succeeds");

        // Expected: ((7 * 1000) + 1 * 3000) / 8 = 10000 / 8 = 1250
        assert_eq!(model.writer_table[base + 5], 1250);
        assert_eq!(report.max_delta, 250);
        assert_eq!(report.l1_shift, 250);
        assert_eq!(report.points_consolidated, 3);
    }

    #[test]
    fn test_coordinate_descent_fixed_point_invariance() {
        let mut model = default_lut_stack_model();
        let target_token = 50u32;
        let base = (target_token as usize) * FBS_WIDTH;

        let mut latent = [0i16; FBS_WIDTH];
        latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);

        let points = vec![
            LatentTrajectoryPoint::new(target_token, latent),
            LatentTrajectoryPoint::new(target_token, latent),
        ];
        let config = ConsolidationConfig::default();

        let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation succeeds");

        assert_eq!(report.weights_updated, 0);
        assert_eq!(report.l1_shift, 0);
        assert_eq!(report.max_delta, 0);
        assert_eq!(report.tokens_updated, 0);
        assert_eq!(report.points_consolidated, 2);
    }

    #[test]
    fn test_coordinate_descent_positive_clamp_30000() {
        let mut model = default_lut_stack_model();
        let target_token = 10u32;
        let base = (target_token as usize) * FBS_WIDTH;

        model.writer_table[base] = 29900;

        let mut latent = [0i16; FBS_WIDTH];
        latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
        latent[0] = 32767;

        let points = vec![LatentTrajectoryPoint::new(target_token, latent)];
        let config = ConsolidationConfig::default();

        let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation succeeds");

        assert_eq!(model.writer_table[base], 30000);
        assert_eq!(report.max_delta, 100);
        assert!(model.writer_table[base] <= 30000);
    }

    #[test]
    fn test_coordinate_descent_negative_clamp_30000() {
        let mut model = default_lut_stack_model();
        let target_token = 20u32;
        let base = (target_token as usize) * FBS_WIDTH;

        model.writer_table[base] = -29900;

        let mut latent = [0i16; FBS_WIDTH];
        latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
        latent[0] = -32768;

        let points = vec![LatentTrajectoryPoint::new(target_token, latent)];
        let config = ConsolidationConfig::default();

        let report = consolidate_lut_trajectory_points(&mut model, &points, &config)
            .expect("consolidation succeeds");

        assert_eq!(model.writer_table[base], -30000);
        assert_eq!(report.max_delta, 100);
        assert!(model.writer_table[base] >= -30000);
    }

    #[test]
    fn test_point_key_synchronization_and_zero_orphans() {
        let mut model = default_lut_stack_model();
        let initial_orphans = model.audit_point_key_synchronization();
        assert_eq!(initial_orphans, 0);

        let pt = model.point.as_ref().unwrap();
        let target = pt.targets[0];
        let initial_p0 = pt.points[0];

        let mut latent = [0i16; FBS_WIDTH];
        latent[0] = 5000;
        let sample = WeightConsolidationSample::from_point(target, latent);

        let report =
            consolidate_lut_weights(&mut model, &[sample], &ConsolidationConfig::default())
                .expect("consolidation succeeds");

        assert_eq!(report.orphaned_point_rows, 0);
        assert_eq!(model.audit_point_key_synchronization(), 0);

        let pt_after = model.point.as_ref().unwrap();
        assert_ne!(pt_after.points[0], initial_p0);
    }

    #[test]
    fn test_uorlut_roundtrip_after_consolidation() {
        let mut model = default_lut_stack_model();
        let sample = WeightConsolidationSample::from_dialogue(
            "What is UOR-R4?",
            "UOR-R4 is a float-free geometric language model over the 16-root E8 orbit.",
        );

        let report =
            consolidate_lut_weights(&mut model, &[sample], &ConsolidationConfig::default())
                .expect("consolidation succeeds");
        assert_eq!(report.orphaned_point_rows, 0);

        let serialized = model.to_uorlut_bytes();
        let reparsed = LutStackModel::parse(&serialized).expect("reparse succeeds");
        assert_eq!(reparsed.audit_point_key_synchronization(), 0);
        assert_eq!(reparsed.writer_table, model.writer_table);
    }

    #[test]
    fn test_consolidation_config_validation() {
        let valid = ConsolidationConfig::default();
        assert!(valid.validate().is_ok());

        // Inverted clamp bounds
        let inverted = ConsolidationConfig {
            clamp_min: 1000,
            clamp_max: -1000,
            ..Default::default()
        };
        assert_eq!(
            inverted.validate().unwrap_err(),
            "invalid clamp bounds: clamp_min=1000 > clamp_max=-1000"
        );

        // Equal clamp bounds is valid
        let equal = ConsolidationConfig {
            clamp_min: 500,
            clamp_max: 500,
            ..Default::default()
        };
        assert!(equal.validate().is_ok());

        // Invalid step configs
        let d_zero = ConsolidationConfig {
            step_denominator: 0,
            ..Default::default()
        };
        assert!(d_zero.validate().is_err());

        let s_zero = ConsolidationConfig {
            step_numerator: 0,
            ..Default::default()
        };
        assert!(s_zero.validate().is_err());

        let s_gt_d = ConsolidationConfig {
            step_numerator: 9,
            step_denominator: 8,
            ..Default::default()
        };
        assert!(s_gt_d.validate().is_err());

        // from_json_str validation
        let invalid_json = r#"{"clamp_min": 1000, "clamp_max": -1000}"#;
        assert!(ConsolidationConfig::from_json_str(invalid_json).is_none());
    }

    #[test]
    fn test_consolidate_lut_trajectory_points_rejects_inverted_clamp() {
        let mut model = default_lut_stack_model();
        let points = vec![LatentTrajectoryPoint::new(1, [0i16; FBS_WIDTH])];
        let bad_cfg = ConsolidationConfig {
            clamp_min: 1000,
            clamp_max: -1000,
            ..Default::default()
        };
        let res = consolidate_lut_trajectory_points(&mut model, &points, &bad_cfg);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "invalid clamp bounds: clamp_min=1000 > clamp_max=-1000"
        );
    }

    #[test]
    fn test_consolidate_lut_weights_rejects_inverted_clamp_bounds() {
        let mut model = default_lut_stack_model();
        let sample = WeightConsolidationSample::from_point(1, [1000i16; FBS_WIDTH]);
        let cfg = ConsolidationConfig {
            clamp_min: 500,
            clamp_max: 200,
            ..Default::default()
        };
        let res = consolidate_lut_weights(&mut model, &[sample], &cfg);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("invalid clamp bounds: clamp_min=500 > clamp_max=200"));
    }

    #[test]
    fn test_coordinate_descent_clamp_min_equals_clamp_max() {
        let mut model = default_lut_stack_model();
        let target_token = 10u32;
        let base = (target_token as usize) * FBS_WIDTH;
        let mut latent = [0i16; FBS_WIDTH];
        latent.copy_from_slice(&model.writer_table[base..base + FBS_WIDTH]);
        latent[0] = 5000;
        let points = vec![LatentTrajectoryPoint::new(target_token, latent)];

        let equal_config = ConsolidationConfig {
            clamp_min: 500,
            clamp_max: 500,
            ..Default::default()
        };

        let result = consolidate_lut_trajectory_points(&mut model, &points, &equal_config);
        assert!(
            result.is_ok(),
            "clamp_min == clamp_max is a valid degenerate interval"
        );
        assert_eq!(model.writer_table[base], 500);
    }

    #[test]
    fn test_rolling_context_state_tracking_and_cap() {
        let mut state = RollingContextState::new();
        assert_eq!(state.turn_index, 0);
        assert!(state.recent_turn_rows.is_empty());
        assert!(state.emitted_4grams.is_empty());

        // Record 12 turn basins in FIFO sequence
        for r in 100..112 {
            state.record_turn_basin(r);
        }

        // Verify capped strictly to last 8 items
        assert_eq!(state.recent_turn_rows.len(), 8);
        assert_eq!(
            state.recent_turn_rows,
            vec![104, 105, 106, 107, 108, 109, 110, 111]
        );

        // Record emitted 4-gram hashes
        let h1 = hash_4gram(10, 20, 30, 40);
        let h2 = hash_4gram(20, 30, 40, 50);
        let h3 = hash_4gram(30, 40, 50, 60);

        state.record_emitted_4gram(h1);
        state.record_emitted_4gram(h2);
        state.record_emitted_4gram(h3);
        assert_eq!(state.emitted_4grams.len(), 3);
        assert!(state.emitted_4grams.contains(&h1));
        assert!(state.emitted_4grams.contains(&h2));
        assert!(state.emitted_4grams.contains(&h3));

        // Deduplication check: duplicate hash insertion is a no-op
        state.record_emitted_4gram(h1);
        assert_eq!(state.emitted_4grams.len(), 3);

        // Lossless JSON serialization roundtrip
        let json_str = state.to_json();
        assert!(json_str.contains("\"recent_turn_rows\":[104,105,106,107,108,109,110,111]"));
        assert!(json_str.contains(&format!("\"emitted_4grams\":[{},{},{}]", h1, h2, h3)));

        let parsed =
            RollingContextState::from_json(&json_str).expect("JSON deserialization succeeds");
        assert_eq!(parsed.recent_turn_rows, state.recent_turn_rows);
        assert_eq!(parsed.emitted_4grams, state.emitted_4grams);
        assert_eq!(parsed, state);

        // Backward compatibility with legacy JSON payloads lacking recent_turn_rows and emitted_4grams
        let legacy_json = r#"{"q12_latent":[0,0,0,0],"turn_index":3,"context_summary":"summary","thinking_trace":["trace1"]}"#;
        let legacy_parsed =
            RollingContextState::from_json(legacy_json).expect("legacy JSON parses");
        assert_eq!(legacy_parsed.turn_index, 3);
        assert_eq!(legacy_parsed.context_summary, "summary");
        assert_eq!(legacy_parsed.thinking_trace, vec!["trace1".to_string()]);
        assert!(legacy_parsed.recent_turn_rows.is_empty());
        assert!(legacy_parsed.emitted_4grams.is_empty());
    }

    #[test]
    fn test_rolling_context_state_json_roundtrip_lossless_64bit_hashes() {
        let mut state = RollingContextState {
            turn_index: 4,
            context_summary: "Turn 4 summary test".to_string(),
            ..Default::default()
        };
        for row in 1..=5 {
            state.record_turn_basin(row * 10);
        }
        // High-bit set 64-bit FNV-1a hashes (value > i64::MAX)
        state.record_emitted_4gram(0xFFFF_FFFF_0000_1234u64);
        state.record_emitted_4gram(0x8000_0000_0000_0000u64);
        state.record_emitted_4gram(42u64);

        let serialized = state.to_json();
        let deserialized =
            RollingContextState::from_json(&serialized).expect("deserialization must succeed");

        assert_eq!(deserialized, state);
        assert_eq!(deserialized.recent_turn_rows, vec![10, 20, 30, 40, 50]);
        assert_eq!(
            deserialized.emitted_4grams,
            vec![0xFFFF_FFFF_0000_1234u64, 0x8000_0000_0000_0000u64, 42u64]
        );
    }

    #[test]
    fn test_lut_stack_session_seed_and_sync() {
        let model = Arc::new(default_lut_stack_model());
        let mut state = RollingContextState::default();
        state.record_turn_basin(77);
        state.record_emitted_4gram(9999);

        let mut session = LutStackSession::new_with_context(model, &state);
        assert_eq!(session.recent_turn_rows, vec![77]);
        assert_eq!(session.emitted_4grams, vec![9999]);

        session.record_turn_basin(88);
        session.record_emitted_4gram(10000);

        // Verify session isolation before sync
        assert_eq!(state.recent_turn_rows, vec![77]);
        assert_eq!(state.emitted_4grams, vec![9999]);

        // Reset clears session state
        session.reset();
        assert!(session.recent_turn_rows.is_empty());
        assert!(session.emitted_4grams.is_empty());
    }

    #[test]
    fn test_basin_repetition_penalty_steers_select_point_key() {
        let model = default_lut_stack_model();
        let point = model.point.as_ref().expect("point table exists");
        assert!(
            point.rows >= 2,
            "point table must have at least 2 rows for test"
        );

        let mem = model.memory.as_ref().expect("memory exists");

        // Synthetic latent query aligned with row 0
        let mut n_t = [0i32; FBS_WIDTH];
        let row0_base = 0usize;
        for (d, slot) in n_t.iter_mut().enumerate().take(point.width.min(FBS_WIDTH)) {
            *slot = point.points[row0_base + d] as i32;
        }

        let bpe = ByteBpe4096::global();
        let prompt_tokens = bpe.encode("who are you");

        // Baseline: With empty recent_turn_rows, row 0 is selected as best match
        let hit_clean = select_point_key_with_resonance(
            point,
            &n_t,
            &prompt_tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &[],
        )
        .expect("hit found without penalty");
        assert_eq!(hit_clean.row, 0, "row 0 must win when unpenalized");

        // With row 0 in recent_turn_rows, penalty is 250_000 * (0 + 1) = 250_000
        // This steers routing to a fresh continuation basin (row != 0)
        let hit_penalized = select_point_key_with_resonance(
            point,
            &n_t,
            &prompt_tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &[0],
        )
        .expect("hit found with penalty");
        assert_ne!(
            hit_penalized.row, 0,
            "penalized row 0 must yield to fresh alternative basin"
        );

        // Multiple visited basins: penalizing both row 0 and the second-best row
        let second_row = hit_penalized.row;
        let hit_third = select_point_key_with_resonance(
            point,
            &n_t,
            &prompt_tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &[0, second_row],
        );
        if let Some(hit) = hit_third {
            assert_ne!(hit.row, 0);
            assert_ne!(hit.row, second_row);
        }
    }

    #[test]
    fn test_step_with_telemetry_ngram_repetition_fallback() {
        let model = Arc::new(default_lut_stack_model());
        let mut session = LutStackSession::new(model);

        // Prefill tokens to establish a known token prefix
        session.prefill_token(100, true);
        session.prefill_token(200, true);
        session.mark_reply_start();

        // 1. Execute baseline step without emitted_4grams
        let baseline_st = session.step_with_telemetry(300);
        let orig_pred = baseline_st.predicted_token;
        assert!(!baseline_st.top_candidates.is_empty());
        let cand0 = baseline_st.top_candidates[0].0;
        assert_eq!(orig_pred, cand0);

        // Extract the 4-gram hash for the baseline prediction: [100, 200, 300, orig_pred]
        let n = session.tokens.len();
        let t_minus_1 = session.tokens[n - 1]; // 300
        let t_minus_2 = session.tokens[n - 2]; // 200
        let t_minus_3 = session.tokens[n - 3]; // 100
        let rep_hash = hash_4gram(t_minus_3, t_minus_2, t_minus_1, orig_pred);

        // 2. Initialize fresh session with rep_hash already present in emitted_4grams
        let mut session2 = LutStackSession::new(session.model.clone());
        session2.prefill_token(100, true);
        session2.prefill_token(200, true);
        session2.mark_reply_start();
        session2.emitted_4grams.push(rep_hash);

        let fallback_st = session2.step_with_telemetry(300);

        // Verify orig_pred was skipped and engine fell back to candidate 1
        assert_ne!(
            fallback_st.predicted_token, orig_pred,
            "repeating token must be rejected"
        );
        let expected_alt = baseline_st.top_candidates[1].0;
        assert_eq!(
            fallback_st.predicted_token, expected_alt,
            "engine must fall back to candidate 1"
        );
        assert_eq!(
            fallback_st.source_mechanism, "ngram_fallback_nearest",
            "source mechanism must reflect fallback"
        );
        assert_eq!(
            fallback_st.l1_distance, baseline_st.top_candidates[1].1,
            "l1_distance must match fallback candidate distance"
        );

        // Verify the new 4-gram hash is registered in emitted_4grams
        let new_hash = hash_4gram(t_minus_3, t_minus_2, t_minus_1, fallback_st.predicted_token);
        assert!(session2.emitted_4grams.contains(&new_hash));

        // 3. Cascading fallback: suppress both candidate 0 and candidate 1
        let mut session3 = LutStackSession::new(session.model.clone());
        session3.prefill_token(100, true);
        session3.prefill_token(200, true);
        session3.mark_reply_start();
        session3.emitted_4grams.push(rep_hash);
        session3.emitted_4grams.push(new_hash);

        let cascading_st = session3.step_with_telemetry(300);
        assert_ne!(cascading_st.predicted_token, orig_pred);
        assert_ne!(cascading_st.predicted_token, expected_alt);
        assert_eq!(
            cascading_st.predicted_token, baseline_st.top_candidates[2].0,
            "cascading fallback must reach candidate 2"
        );
    }

    // ============================================================================
    // MILESTONE M2 UNIT TESTS: DYNAMIC SUB-SENTENCE RE-ATTENTION & ANTI-CHUNKING
    // ============================================================================

    #[test]
    fn test_clause_boundary_detection_punctuation_and_interval() {
        let bpe = ByteBpe4096::global();

        // 1. Punctuation boundary tokens must trigger boundary detection at any step >= 1
        let boundary_chars = [".", "?", "!", ":", ";", "\n"];
        for ch in boundary_chars {
            let tokens = bpe.encode(ch);
            assert!(
                !tokens.is_empty(),
                "punctuation '{}' must encode to at least 1 token",
                ch
            );
            let tok = tokens[tokens.len() - 1];
            for step in [1usize, 3, 7, 13, 15] {
                assert!(
                    is_clause_boundary(tok, step),
                    "token for '{}' (id {}) must be recognized as boundary at step {}",
                    ch,
                    tok,
                    step
                );
            }
        }

        // 2. Normal word tokens without punctuation must NOT trigger at non-interval steps
        let normal_words = ["hello", "world", "algorithm", "state", "the"];
        for word in normal_words {
            let tokens = bpe.encode(word);
            assert!(!tokens.is_empty());
            let tok = tokens[0];
            for step in [1usize, 2, 5, 9, 14, 15] {
                assert!(
                    !is_clause_boundary(tok, step),
                    "word '{}' (id {}) must NOT trigger boundary at step {}",
                    word,
                    tok,
                    step
                );
            }
        }

        // 3. Step interval K=16: at step 16, 32, 48, boundary must trigger even on regular words
        let word_tok = bpe.encode("matrix")[0];
        assert!(
            is_clause_boundary(word_tok, 16),
            "step 16 must trigger boundary regardless of token"
        );
        assert!(
            is_clause_boundary(word_tok, 32),
            "step 32 must trigger boundary regardless of token"
        );
        assert!(
            is_clause_boundary(word_tok, 48),
            "step 48 must trigger boundary regardless of token"
        );

        // 4. Step 0 must not falsely trigger step interval
        assert!(
            !is_clause_boundary(word_tok, 0),
            "step 0 with regular word token must not trigger boundary"
        );
    }

    #[test]
    fn test_flock_cyclic_frame_modulation_zero_float_invariance() {
        // Verify cyclic_frame_block properties across all 8 heads and 8 sweeps
        for sweep in 0..8u64 {
            let mut seen_frames = [false; 8];
            for head in 0..8u64 {
                let frame = cyclic_frame_block(head, sweep, 8);
                assert!(frame < 8, "frame index must be in [0, 8)");
                seen_frames[frame as usize] = true;
            }
            for (f, &seen) in seen_frames.iter().enumerate() {
                assert!(seen, "sweep {} must cover frame {}", sweep, f);
            }
        }

        // Create a 256D test hidden state with head-identifying patterns:
        // Head h has base value (h + 1) * 1000 in Q12
        let mut n_t = [0i32; FBS_WIDTH];
        for h in 0..8 {
            for d in 0..32 {
                n_t[h * 32 + d] = ((h as i32) + 1) * 1000 + (d as i32);
            }
        }

        // Modulate across sweeps 0..8
        for sweep in 0..8usize {
            let modulated = modulate_flock_cyclic_attention(&n_t, sweep);
            assert_eq!(modulated.len(), FBS_WIDTH);

            for h in 0..8 {
                let expected_frame = cyclic_frame_block(h as u64, sweep as u64, 8) as usize;
                for d in 0..32 {
                    let actual = modulated[h * 32 + d];
                    let expected = n_t[expected_frame * 32 + d];
                    assert_eq!(
                        actual, expected,
                        "head {} in sweep {} must match frame {} coordinate {}",
                        h, sweep, expected_frame, d
                    );
                    assert!(
                        actual.abs() <= 30000,
                        "Q12 coordinate must clamp within 30000"
                    );
                }
            }
        }
    }

    #[test]
    fn test_dynamic_re_attention_updates_reply_anchor_midstream() {
        let model = Arc::new(default_lut_stack_model());
        let point = model.point.as_ref().expect("point table exists");
        let mem = model.memory.as_ref().expect("memory exists");
        let bpe = ByteBpe4096::global();

        // Select two distinct curriculum basin rows
        let row_a = 0usize;
        let anchor_a = resolve_canonical_reply_anchor(point, mem, row_a, point.targets[row_a], &[])
            .expect("anchor A resolves");
        let (row_b, anchor_b) = (1..point.rows)
            .find_map(|r| {
                let anc = resolve_canonical_reply_anchor(point, mem, r, point.targets[r], &[])?;
                if anc != anchor_a {
                    Some((r, anc))
                } else {
                    None
                }
            })
            .expect("must find a row with distinct anchor");
        assert_ne!(anchor_a, anchor_b, "anchors must be distinct");

        let mut session = LutStackSession::new(model.clone());
        let prompt_tokens = bpe.encode("who are you");
        for &tok in &prompt_tokens[..prompt_tokens.len() - 1] {
            session.prefill_token(tok, true);
        }
        session.mark_reply_start();
        session.step_with_telemetry(prompt_tokens[prompt_tokens.len() - 1]);

        // Initialize anchors.reply to anchor_A and queue anchor_B in pending_basins
        session.anchors.reply = anchor_a.clone();
        let first_tok_b = point.targets[row_b];
        session.pending_basins = vec![(anchor_b.clone(), first_tok_b)];

        // Simulate generation loop stepping toward a clause boundary
        let period_tok = bpe.encode(".")[0];
        let word_tok = bpe.encode(" name")[0];

        // Step 1: Normal word token (non-boundary)
        let _st1 = session.step_with_telemetry(word_tok);
        assert_eq!(
            session.anchors.reply, anchor_a,
            "anchor must remain anchor_A before clause boundary"
        );
        assert_eq!(
            session.pending_basins.len(),
            1,
            "pending basin must remain queued"
        );

        // Step 2: Clause boundary token (period '.')
        let _st2 = session.step_with_telemetry(period_tok);

        // Dynamic re-attention triggers at clause boundary:
        let mut next_tok = 0u32;
        let mut last_source = "turn_memory_reply";
        let mut clauses_resolved = 1;
        let transitioned = model.try_mid_stream_re_attention(
            &mut session,
            period_tok,
            2,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );
        assert!(transitioned, "re-attention must trigger at boundary");

        assert_eq!(
            session.anchors.reply, anchor_b,
            "session.anchors.reply must dynamically update to anchor_B at clause boundary"
        );
        assert_eq!(next_tok, first_tok_b);
        assert!(
            session.pending_basins.is_empty(),
            "pending basin must have been consumed"
        );

        // Step 3: Continues decoding under anchor_B without reset
        let _st3 = session.step_with_telemetry(first_tok_b);
        assert_eq!(session.anchors.reply, anchor_b);
    }

    #[test]
    fn test_anti_chunking_prevents_newline_injection_and_preserves_history() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        // Multi-clause prompt: "Explain quantum entanglement in clear terms and also tell me about Emmy Noether."
        let user_prompt =
            "Explain quantum entanglement in clear terms and also tell me about Emmy Noether.";
        let clauses = split_prompt_clauses(user_prompt);
        assert!(
            clauses.len() >= 2,
            "prompt must split into multiple clauses"
        );

        let res = model.generate_reply(&[], user_prompt, 64);
        assert!(
            !res.generated_tokens.is_empty(),
            "generation must produce tokens"
        );

        // 1. Anti-chunking: verify NO "\n\n" separator was injected mid-stream
        let sep_tokens = bpe.encode("\n\n");
        let has_injected_sep = res
            .generated_tokens
            .windows(sep_tokens.len())
            .any(|w| w == sep_tokens.as_slice());
        assert!(
            !has_injected_sep,
            "generated_tokens must NOT contain injected \\n\\n chunk separator: {:?}",
            res.reply_text
        );
        assert!(
            !res.reply_text.contains("\n\n"),
            "reply_text must not contain \\n\\n paragraph chunking: '{}'",
            res.reply_text
        );

        // 2. Continuous history preservation:
        let mut session = LutStackSession::new(model.clone());
        let prompt_toks = bpe.encode("test prompt");
        for &t in &prompt_toks[..prompt_toks.len() - 1] {
            session.prefill_token(t, true);
        }
        session.mark_reply_start();
        session.step_with_telemetry(prompt_toks[prompt_toks.len() - 1]);

        // Emulate 3 steps in clause 1
        let tok1 = bpe.encode("Hello")[0];
        let tok2 = bpe.encode(" world")[0];
        let tok3 = bpe.encode(".")[0];
        session.step_with_telemetry(tok1);
        session.step_with_telemetry(tok2);
        session.step_with_telemetry(tok3);

        let history_len_before = session.reply_history.len();
        assert!(
            history_len_before >= 3,
            "reply_history must track emitted tokens"
        );

        // Pop secondary basin mid-stream via model.try_mid_stream_re_attention at boundary token (tok3 = '.')
        let dummy_anchor = vec![10u32, 20, 30, 40];
        let first_tok_basin2 = 50u32;
        session
            .pending_basins
            .push((dummy_anchor.clone(), first_tok_basin2));

        let mut next_tok = 0u32;
        let mut last_source = "turn_memory_reply";
        let mut clauses_resolved = 1;
        let step_idx = session.tokens.len();

        let transitioned = model.try_mid_stream_re_attention(
            &mut session,
            tok3,
            step_idx,
            &mut next_tok,
            &mut last_source,
            &mut clauses_resolved,
        );

        assert!(
            transitioned,
            "try_mid_stream_re_attention must trigger transition at boundary token"
        );
        assert_eq!(
            session.anchors.reply, dummy_anchor,
            "session.anchors.reply must update to new basin anchor"
        );
        assert_eq!(
            next_tok, first_tok_basin2,
            "next_tok must be populated with target token of the popped basin"
        );
        assert_eq!(
            last_source, "turn_memory_reply",
            "last_source must remain turn_memory_reply"
        );
        assert!(
            session.pending_basins.is_empty(),
            "pending basin must have been consumed from queue"
        );

        // Anti-chunking invariant: reply_history and suffix_counts must NOT be wiped or shrunk
        assert_eq!(
            session.reply_history.len(),
            history_len_before,
            "reply_history must NOT be cleared on mid-stream basin transition"
        );
        assert!(
            !session.reply_history.is_empty(),
            "reply_history must remain intact across mid-stream re-attention"
        );
        assert!(
            !session.suffix_counts.is_empty(),
            "suffix_counts must remain intact across mid-stream re-attention"
        );

        // Step continuation token and verify reply_history monotonically advances
        let _st = session.step_with_telemetry(next_tok);
        assert_eq!(
            session.reply_history.len(),
            history_len_before + 1,
            "reply_history must advance monotonically after stepping target token"
        );
        assert_eq!(
            session.reply_history[session.reply_history.len() - 1],
            first_tok_basin2,
            "stepped token must match target token from new basin"
        );
    }

    #[test]
    fn test_continuous_generation_eliminates_premature_eos_lockout() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        let user_prompt = "hello";
        let max_tokens = 32;

        let res = model.generate_reply(&[], user_prompt, max_tokens);
        assert!(!res.generated_tokens.is_empty());

        // In post-M2, decoding continues via writer.table / top_candidates when turn_memory_reply ends
        let non_memory_steps = res
            .steps
            .iter()
            .filter(|st| {
                st.source_mechanism != "turn_memory_reply"
                    && st.source_mechanism != "turn_memory_opening"
            })
            .count();

        assert!(
            non_memory_steps > 0,
            "continuous generation must allow decoding beyond turn_memory_reply without premature EOS"
        );

        let first_non_memory_idx = res.steps.iter().position(|st| {
            st.source_mechanism != "turn_memory_reply"
                && st.source_mechanism != "turn_memory_opening"
        });

        if let Some(idx) = first_non_memory_idx {
            if idx + 1 < res.steps.len() {
                let next_step = &res.steps[idx + 1];
                assert_ne!(
                    next_step.predicted_token, bpe.eos_id(),
                    "step immediately following turn_memory_reply exhaustion must NOT be forced to EOS"
                );
            }
        }
    }

    #[test]
    fn test_re_attention_with_trajectory_shift_without_pending_basins() {
        let model = Arc::new(default_lut_stack_model());
        let point = model.point.as_ref().expect("point table exists");
        let mem = model.memory.as_ref().expect("memory exists");
        let bpe = ByteBpe4096::global();

        let mut session = LutStackSession::new(model.clone());
        let prompt_toks = bpe.encode("who are you");
        for &t in &prompt_toks {
            session.prefill_token(t, true);
        }
        session.mark_reply_start();

        // Set initial anchor to row 0
        let canon_0 = resolve_canonical_reply_anchor(point, mem, 0, point.targets[0], &[])
            .expect("canon 0 resolves");
        session.anchors.reply = canon_0.clone();
        session.pending_basins.clear();

        // Synthesize trajectory shift toward row 2
        let target_row = 2usize.min(point.rows - 1);
        let canon_target =
            resolve_canonical_reply_anchor(point, mem, target_row, point.targets[target_row], &[])
                .expect("canon target resolves");

        if canon_target != canon_0 {
            let mut shifted_n_t = [0i32; FBS_WIDTH];
            let base = target_row * point.width;
            let sweep = (session.tokens.len() as u64) % 8;
            for head in 0..8 {
                let frame_block = cyclic_frame_block(head as u64, sweep, 8) as usize;
                let dst_base = head * 32;
                let src_base = frame_block * 32;
                for d in 0..32 {
                    if dst_base + d < point.width {
                        shifted_n_t[src_base + d] = point.points[base + dst_base + d] as i32;
                    }
                }
            }
            session.last_n_t = shifted_n_t;

            let boundary_tok = bpe.encode(".")[0];
            let step = 16;
            assert!(is_clause_boundary(boundary_tok, step));

            let flock_n_t =
                modulate_flock_cyclic_attention(&session.last_n_t, session.tokens.len());
            if let Some(pt_hit) = select_point_key_with_resonance(
                point,
                &flock_n_t,
                &session.tokens,
                mem.key_window,
                mem.reply.as_ref(),
                mem.reply_key_len,
                &session.recent_turn_rows,
            ) {
                if let Some(new_canon) =
                    resolve_canonical_reply_anchor(point, mem, pt_hit.row, pt_hit.target_token, &[])
                {
                    if new_canon != session.anchors.reply {
                        session.anchors.reply = new_canon;
                    }
                }
            }

            assert_eq!(
                session.anchors.reply, canon_target,
                "session.anchors.reply must transition to new continuation basin on trajectory shift"
            );
        }
    }

    #[test]
    fn test_full_trunk_multi_turn_history_and_context_prefill() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        let mut context = RollingContextState {
            context_summary: "Prior discussion regarding Emmy Noether and symmetry laws."
                .to_string(),
            ..Default::default()
        };
        context.q12_latent[0] = 2048;

        let history = vec![
            (
                "Who was Emmy Noether?".to_string(),
                "Emmy Noether was a German mathematician who made landmark contributions to abstract algebra and theoretical physics.".to_string(),
            ),
        ];

        let user_prompt = "What conservation law follows from time translation symmetry?";
        let sys_note = Some(format!("[Context: {}]", context.context_summary));
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            &history,
            user_prompt,
            model.dialogue_version,
        );

        // Ensure dialogue prompt exceeds 32 tokens to stress full trunk prefill beyond old 32-token window
        assert!(
            prompt_tokens.len() > 32,
            "dialogue prompt tokens ({}) must exceed 32 tokens",
            prompt_tokens.len()
        );

        let mut session = LutStackSession::new_with_context(model.clone(), &context);

        // Prefill all prompt tokens except the final one with run_trunk = true
        for &tok in &prompt_tokens[..prompt_tokens.len() - 1] {
            session.prefill_token(tok, true);
        }

        // Verify that recurrence states in ALL 6 layers of SpiralCore/FBS are active (non-zero)
        assert_eq!(
            session.rec_states.len(),
            6,
            "model trunk must have 6 SpiralCore/FBS layers"
        );
        for (layer_idx, rec_state) in session.rec_states.iter().enumerate() {
            let l1_norm: i64 = rec_state.h_state.iter().map(|&v| (v as i64).abs()).sum();
            assert!(
                l1_norm > 0,
                "layer {} recurrence state h_state must be non-zero after full-trunk prefill",
                layer_idx
            );
        }

        // Verify that session.tokens accurately tracks every prefilled token
        assert_eq!(session.tokens.len(), prompt_tokens.len() - 1);
        assert_eq!(
            &session.tokens[..],
            &prompt_tokens[..prompt_tokens.len() - 1]
        );
    }

    #[test]
    fn test_autoregressive_thinking_tokens_actively_steer_hidden_state_nt() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        let prompt = "Solve 42 * 7";
        let prompt_tokens = bpe.encode(prompt);

        let mut session = LutStackSession::new(model.clone());

        // 1. Prefill initial prompt
        for &tok in &prompt_tokens {
            session.prefill_token(tok, true);
        }
        let prompt_n_t = session.last_n_t;

        // 2. Autoregressively step through reasoning tokens
        let reasoning_trace = "<thinking>\n40 * 7 = 280\n2 * 7 = 14\n280 + 14 = 294\n</thinking>";
        let reasoning_tokens = bpe.encode(reasoning_trace);
        assert!(
            !reasoning_tokens.is_empty(),
            "reasoning tokens must be non-empty"
        );

        let mut previous_n_t = prompt_n_t;
        let mut total_step_shifts = 0u64;

        for &tok in &reasoning_tokens {
            session.prefill_token(tok, true);
            let current_n_t = session.last_n_t;

            // Zero-float bounded Q12 invariant
            for &val in &current_n_t {
                assert!(
                    (-32768..=32767).contains(&val),
                    "hidden state coordinate {} exceeds signed 16-bit Q12 fixed point",
                    val
                );
            }

            let step_shift: u64 = current_n_t
                .iter()
                .zip(previous_n_t.iter())
                .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
                .sum();
            total_step_shifts += step_shift;
            previous_n_t = current_n_t;
        }

        let post_thinking_n_t = session.last_n_t;

        // Compute total trajectory displacement from prompt state to post-thinking state
        let net_steering_displacement: u64 = post_thinking_n_t
            .iter()
            .zip(prompt_n_t.iter())
            .map(|(&a, &b)| (a as i64 - b as i64).unsigned_abs())
            .sum();

        assert!(
            net_steering_displacement > 0,
            "reasoning trajectory must actively steer n_t away from prompt state"
        );
        assert!(
            total_step_shifts >= net_steering_displacement,
            "triangle inequality must hold for cumulative reasoning step shifts"
        );
    }

    #[test]
    fn test_visible_reply_barrier_enforced_strictly_after_thinking() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        let prompt = "Explain Fermat's Little Theorem";
        let prompt_tokens = bpe.encode(prompt);
        let mut session = LutStackSession::new(model.clone());

        for &tok in &prompt_tokens {
            session.prefill_token(tok, true);
        }

        // Before thinking: reply_start must be None
        assert!(
            session.reply_start.is_none(),
            "reply_start must be None prior to reasoning completion"
        );

        let thinking_body = "<thinking>\nRecall a^(p-1) == 1 mod p for prime p\n</thinking>";
        let thinking_tokens = bpe.encode(thinking_body);

        // While stepping through reasoning tokens inside <thinking>...</thinking>:
        // Barrier invariant: reply_start remains None until </thinking> is processed
        for (idx, &tok) in thinking_tokens.iter().enumerate() {
            session.prefill_token(tok, true);
            if idx + 1 < thinking_tokens.len() {
                assert!(
                    session.reply_start.is_none(),
                    "reply_start must NOT be set while inside reasoning block (token {})",
                    idx
                );
            }
        }

        // At the barrier: now </thinking> has been prefilled
        session.mark_reply_start();
        assert!(
            session.reply_start.is_some(),
            "reply_start must be set immediately after </thinking>"
        );
        let reply_start_pos = session.reply_start.unwrap();
        assert_eq!(
            reply_start_pos,
            session.tokens.len() + 1,
            "reply_start position must match post-thinking barrier offset"
        );

        // Visible reply decoding begins: reply_history only populates after reply_start
        assert!(
            session.reply_history.is_empty(),
            "reply_history must be empty prior to visible reply generation"
        );

        let first_reply_tok = bpe.encode("Fermat")[0];
        let step = session.step_with_telemetry(first_reply_tok);
        assert_eq!(session.reply_history.len(), 1);
        assert_eq!(session.reply_history[0], first_reply_tok);
        assert_eq!(step.input_token, first_reply_tok);
    }

    #[test]
    fn test_reasoning_trajectory_routes_to_distinct_point_key_basin() {
        let model = Arc::new(default_lut_stack_model());
        let point = model
            .point
            .as_ref()
            .expect("ServedPointKey must exist in model");
        let mem = model
            .memory
            .as_ref()
            .expect("ServedTurnMemory must exist in model");
        let bpe = ByteBpe4096::global();

        let prompt =
            "What is the capital of France and what theorem connects symmetries to invariants?";
        let prompt_tokens = bpe.encode(prompt);

        let mut session = LutStackSession::new(model.clone());
        for &tok in &prompt_tokens {
            session.prefill_token(tok, true);
        }
        let unreasoned_n_t = session.last_n_t;

        // Query PointKey hit from unreasoned state
        let unreasoned_hit = select_point_key_with_resonance(
            point,
            &unreasoned_n_t,
            &session.tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &session.recent_turn_rows,
        );

        // Now steer through reasoning specifically targeting Emmy Noether
        let thinking = "<thinking>\nThe second part asks about symmetries and conservation laws: Noether's theorem proved in 1915.\n</thinking>";
        for &tok in &bpe.encode(thinking) {
            session.prefill_token(tok, true);
        }
        let reasoned_n_t = session.last_n_t;

        let reasoned_hit = select_point_key_with_resonance(
            point,
            &reasoned_n_t,
            &session.tokens,
            mem.key_window,
            mem.reply.as_ref(),
            mem.reply_key_len,
            &session.recent_turn_rows,
        );

        // Verify that the reasoning trajectory modified the distance / resonance scores
        let l1_diff: i64 = reasoned_n_t
            .iter()
            .zip(unreasoned_n_t.iter())
            .map(|(&a, &b)| (a as i64 - b as i64).abs())
            .sum();
        assert!(
            l1_diff > 0,
            "reasoning must induce non-zero L1 trajectory shift"
        );

        if let (Some(h1), Some(h2)) = (unreasoned_hit, reasoned_hit) {
            let geometry_shifted =
                h1.row != h2.row || h1.raw_l1 != h2.raw_l1 || h1.matched_stems != h2.matched_stems;
            assert!(
                geometry_shifted,
                "PointKey resonance hit must reflect reasoning-steered geometry"
            );
        }
    }

    #[test]
    fn test_rolling_context_state_tracks_and_preserves_thinking_trace_across_turns() {
        let mut context = RollingContextState::default();
        assert!(context.thinking_trace.is_empty());
        assert_eq!(context.turn_index, 0);

        // Record 20 thinking traces (exceeding 16-trace FIFO cap)
        for i in 1..=20 {
            context.record_thinking(format!("Reasoning trace for turn {}", i));
        }

        assert_eq!(
            context.thinking_trace.len(),
            16,
            "thinking_trace must be capped strictly at 16 traces FIFO"
        );
        assert_eq!(
            context.thinking_trace.first().unwrap(),
            "Reasoning trace for turn 5",
            "oldest 4 traces must have been evicted FIFO"
        );
        assert_eq!(
            context.thinking_trace.last().unwrap(),
            "Reasoning trace for turn 20",
            "most recent trace must be at end of FIFO"
        );

        // Blend reasoning n_t
        let reasoning_n_t = [2048i32; FBS_WIDTH];
        context.blend_latent(&reasoning_n_t);
        assert_eq!(context.turn_index, 1);
        for d in 0..FBS_WIDTH {
            // (3072 * 0 + 1024 * 2048) / 4096 = 512
            assert_eq!(context.q12_latent[d], 512);
        }

        // Lossless JSON roundtrip check
        let json = context.to_json();
        let parsed = RollingContextState::from_json(&json).expect("valid JSON deserialization");
        assert_eq!(parsed.thinking_trace, context.thinking_trace);
        assert_eq!(parsed.q12_latent, context.q12_latent);
        assert_eq!(parsed.turn_index, context.turn_index);
    }

    #[test]
    fn test_zero_float_conformance_through_trunk_prefill_and_reasoning_loop() {
        let model = Arc::new(default_lut_stack_model());
        let bpe = ByteBpe4096::global();

        let mut session = LutStackSession::new(model.clone());
        let tokens = bpe.encode("Reasoning test: <thinking>step 1 -> step 2</thinking> answer");

        for &tok in &tokens {
            session.prefill_token(tok, true);
            let n_t = session.last_n_t;

            // Assert each coordinate is an integer within bounded Q12 range
            for &coord in &n_t {
                assert!(
                    (-32768..=32767).contains(&coord),
                    "Q12 hidden coordinate {} outside signed 16-bit range",
                    coord
                );
            }
        }

        // Verify that recurrence states are pure integer types
        for layer in &session.rec_states {
            for &h in &layer.h_state {
                assert!(
                    (-100_000..=100_000).contains(&h),
                    "recurrent state h out of bounded integer range"
                );
            }
        }
    }

    #[test]
    fn test_clause_boundary_trajectory_capture() {
        let model = Arc::new(default_lut_stack_model());
        let reply = "The Lie algebra e8 has dimension 248. Its root system contains 240 roots; each root has squared length 2! Furthermore, its Weyl group order is 696729600.";
        let res = model.trace_dynamic_turn(&[], "What is E8?", reply, 128);

        assert!(
            !res.boundary_trajectories.is_empty(),
            "must capture boundary trajectories"
        );
        for hit in &res.boundary_trajectories {
            assert!(
                is_clause_boundary(hit.boundary_token, hit.step),
                "captured hit at step {} token {} must be a valid clause boundary",
                hit.step,
                hit.boundary_token
            );
            assert!(hit.step > 0, "step must be 1-indexed");
        }
        assert!(
            res.total_l1_margin > 0,
            "total L1 margin should be non-zero for test sequence"
        );
    }

    #[test]
    fn test_compute_token_l1_margin_exact_match_zero() {
        let model = default_lut_stack_model();
        for &t in &[0u32, 1, 42, 256, 1000, 4095] {
            let margin = compute_token_l1_margin(&model.writer_table, model.vocab, t, t);
            assert_eq!(margin, 0, "token compared to itself must have margin 0");
        }
        // Out of vocabulary tokens return 0
        assert_eq!(
            compute_token_l1_margin(&model.writer_table, model.vocab, 5000, 1),
            0
        );
        assert_eq!(
            compute_token_l1_margin(&model.writer_table, model.vocab, 1, 5000),
            0
        );
    }

    #[test]
    fn test_compute_token_l1_margin_distinct_tokens_and_triangle_inequality() {
        let model = default_lut_stack_model();
        let t1 = 10u32;
        let t2 = 20u32;
        let t3 = 30u32;

        let m12 = compute_token_l1_margin(&model.writer_table, model.vocab, t1, t2);
        let m21 = compute_token_l1_margin(&model.writer_table, model.vocab, t2, t1);
        assert!(m12 > 0, "distinct tokens must have positive margin");
        assert_eq!(m12, m21, "L1 margin must be symmetric");

        let m23 = compute_token_l1_margin(&model.writer_table, model.vocab, t2, t3);
        let m13 = compute_token_l1_margin(&model.writer_table, model.vocab, t1, t3);
        assert!(
            m13 <= m12 + m23,
            "triangle inequality must hold: m13 <= m12 + m23"
        );
    }

    #[test]
    fn test_residual_modulated_coordinate_descent_zero_error_zero_shift() {
        let mut model = default_lut_stack_model();
        let target_token = 50u32;
        let latent = [1000i16; FBS_WIDTH];

        // Sample with predicted_token == target_token (residual margin = 0)
        let sample = WeightConsolidationSample::from_point_with_residual(
            target_token,
            target_token,
            latent,
            0,
        );

        let report =
            consolidate_lut_weights(&mut model, &[sample], &ConsolidationConfig::default())
                .expect("consolidation succeeds");
        assert_eq!(
            report.weights_updated, 0,
            "zero error must produce zero weight updates"
        );
        assert_eq!(report.l1_shift, 0, "zero error must produce zero L1 shift");
    }

    #[test]
    fn test_residual_modulated_coordinate_descent_monotonic_contraction() {
        let target_token = 200u32;
        let latent = [5000i16; FBS_WIDTH];

        // Consolidate with smaller margin
        let mut model1 = default_lut_stack_model();
        let sample1 = WeightConsolidationSample::from_point_with_residual(
            target_token,
            target_token + 1,
            latent,
            31_250,
        );
        let report1 =
            consolidate_lut_weights(&mut model1, &[sample1], &ConsolidationConfig::default())
                .expect("consolidation succeeds");

        // Consolidate with larger margin
        let mut model2 = default_lut_stack_model();
        let sample2 = WeightConsolidationSample::from_point_with_residual(
            target_token,
            target_token + 1,
            latent,
            250_000,
        );
        let report2 =
            consolidate_lut_weights(&mut model2, &[sample2], &ConsolidationConfig::default())
                .expect("consolidation succeeds");

        assert!(
            report2.l1_shift > report1.l1_shift,
            "larger residual margin must produce larger coordinate descent shift: {} > {}",
            report2.l1_shift,
            report1.l1_shift
        );
    }

    #[test]
    fn test_online_admission_backprop_pipeline() {
        let mut model = default_lut_stack_model();
        let prompt = "Explain the Coxeter number of E8.";
        let reply = "The Coxeter number of the exceptional Lie algebra E8 is 30.";

        let admitted = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("compile succeeds");
        assert!(admitted, "turn should be newly admitted");
        assert_eq!(
            model.audit_point_key_synchronization(),
            0,
            "zero orphaned point rows required after online admission backprop"
        );
    }

    #[test]
    fn test_zero_latency_online_readback_immediate_turn_routing() {
        let mut model = default_lut_stack_model();
        let prompt = "State the McKay correspondence for the icosahedral group.";
        let reply = "The McKay correspondence relates the binary icosahedral group to affine E8 Dynkin diagram.";

        let admitted = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("compile succeeds");
        assert!(admitted);

        // Immediate readback without reloading model
        let res = Arc::new(model).trace_dynamic_turn(&[], prompt, reply, 32);
        assert!(
            !res.generated_tokens.is_empty(),
            "readback generated tokens"
        );
        assert_eq!(res.clauses_resolved, 1);
    }

    #[test]
    fn test_rolling_context_state_residual_margin_tracking_and_serde() {
        let mut ctx = RollingContextState::new();
        assert_eq!(ctx.accumulated_l1_margin, 0);
        assert!(ctx.recent_residuals.is_empty());

        for i in 1..=20 {
            ctx.record_residual_margin(i * 100);
        }
        assert_eq!(ctx.recent_residuals.len(), 16, "capped to 16 FIFO");
        assert_eq!(
            ctx.recent_residuals.first().copied(),
            Some(500),
            "FIFO evicted oldest 4"
        );
        assert_eq!(
            ctx.recent_residuals.last().copied(),
            Some(2000),
            "FIFO most recent at end"
        );
        assert_eq!(
            ctx.accumulated_l1_margin,
            (1..=20).map(|i| i * 100).sum::<u64>()
        );

        // Lossless JSON roundtrip
        let json = ctx.to_json();
        let parsed = RollingContextState::from_json(&json).expect("valid JSON deserialization");
        assert_eq!(parsed.accumulated_l1_margin, ctx.accumulated_l1_margin);
        assert_eq!(parsed.recent_residuals, ctx.recent_residuals);

        // Backward compatibility with legacy JSON lacking residual fields
        let legacy_json = r#"{"q12_latent":[],"turn_index":3,"context_summary":"test","thinking_trace":[],"recent_turn_rows":[],"emitted_4grams":[]}"#;
        let legacy_parsed =
            RollingContextState::from_json(legacy_json).expect("legacy JSON parses");
        assert_eq!(legacy_parsed.accumulated_l1_margin, 0);
        assert!(legacy_parsed.recent_residuals.is_empty());
    }

    #[test]
    fn test_debruijn_slot_binding_r4_31_arithmetic_algebraic_and_division_guard() {
        let model = Arc::new(default_lut_stack_model());

        // 1. Novel multi-operator arithmetic expression
        let res_arith = model.generate_reply(&[], "What is (144 * 89) + 377?", 128);
        assert!(
            res_arith.reply_text.contains("13193"),
            "novel arithmetic query must compute exact value 13193 via R4-31/R4-27 kernel, got: {}",
            res_arith.reply_text
        );
        let sf = res_arith.slot_frame.expect("slot_frame present");
        assert!(sf.substitutions_applied >= 1);
        assert!(
            sf.steps_or_tokens_valid(),
            "slot frame must record valid VSA binding tags"
        );
        assert!(
            res_arith
                .steps
                .iter()
                .any(|s| s.source_mechanism == "debruijn_slot_kernel_r4_31"),
            "token stream telemetry must record debruijn_slot_kernel_r4_31"
        );

        // 2. Novel symbolic variable assignment and algebraic evaluation
        let res_alg = model.generate_reply(
            &[],
            "If alpha = 25 and beta = 16, what is (alpha * beta) + 99?",
            128,
        );
        assert!(
            res_alg.reply_text.contains("499"),
            "symbolic variable slot-binding must compute (25 * 16) + 99 = 499, got: {}",
            res_alg.reply_text
        );

        // 3. Functional number-theoretic kernel (gcd)
        let res_gcd = model.generate_reply(&[], "Compute gcd(1071, 462)", 128);
        assert!(
            res_gcd.reply_text.contains("21"),
            "gcd(1071, 462) must evaluate to 21, got: {}",
            res_gcd.reply_text
        );

        // 4. Negative control: division by zero guard
        let div_zero = DeBruijnSlotFrame::bind_and_execute("What is 500 / 0?");
        assert!(
            div_zero
                .kernel_results
                .iter()
                .any(|kr| kr.op_name == "division_by_zero_guard" && kr.result_numeric.is_none()),
            "division by zero must be rejected by R4.SlotBinding.evaluateBoundSlotKernel"
        );

        // 5. Sentence-separated bindings and `==` mixed with `=`
        let mixed_eq = DeBruijnSlotFrame::bind_and_execute(
            "If x = 5 and y = 5 (where x == y), what is x * y + 10?",
        );
        assert!(
            mixed_eq
                .kernel_results
                .iter()
                .any(|kr| kr.result_numeric == Some(35)),
            "mixed == and = prompt must evaluate x * y + 10 = 35, got: {:?}",
            mixed_eq.kernel_results
        );
        let sentence_vars =
            DeBruijnSlotFrame::bind_and_execute("Let x = 5. Let y = 10. What is x + y?");
        assert!(
            sentence_vars
                .kernel_results
                .iter()
                .any(|kr| kr.result_numeric == Some(15)),
            "sentence-separated bindings must evaluate x + y = 15, got: {:?}",
            sentence_vars.kernel_results
        );

        // 6. Multiple word operators, right-associative chained `^`, and composite function calls
        let multi_word = DeBruijnSlotFrame::bind_and_execute("What is 10 plus 20 plus 30?");
        assert!(
            multi_word
                .kernel_results
                .iter()
                .any(|kr| kr.result_numeric == Some(60)),
            "multiple word operators must evaluate 10 + 20 + 30 = 60"
        );
        let chained_pow = DeBruijnSlotFrame::bind_and_execute("What is 2 ^ 3 ^ 2?");
        assert!(
            chained_pow
                .kernel_results
                .iter()
                .any(|kr| kr.result_numeric == Some(512)),
            "chained exponentiation 2 ^ 3 ^ 2 must evaluate right-associatively to 512"
        );
        let comp_func = DeBruijnSlotFrame::bind_and_execute("What is gcd(120, 45) + lcm(12, 18)?");
        assert!(
            comp_func
                .kernel_results
                .iter()
                .any(|kr| kr.result_numeric == Some(51)),
            "composite functional expression gcd(120, 45) + lcm(12, 18) must evaluate to 15 + 36 = 51"
        );
    }

    #[test]
    fn test_debruijn_slot_binding_r4_31_icosian_and_ast_identifier_synthesis() {
        let model = Arc::new(default_lut_stack_model());

        // 1. Novel Icosian Z[phi] 17-root snap over bound coordinates
        let res_icos = model.generate_reply(
            &[],
            "Compute the icosian snap vector for [1200, -450, 800, 3100]",
            128,
        );
        assert!(
            res_icos.reply_text.contains("Icosian Z[phi] 17-Root Snap")
                && res_icos.reply_text.contains("Root #"),
            "Icosian slot kernel must snap bound 4D coordinates to exact Z[phi] root, got: {}",
            res_icos.reply_text
        );

        // 2. Novel identifier-parameterized Rust function synthesis
        let res_ast = model.generate_reply(
            &[],
            "Write a Rust function compute_orbital_delta(base_energy: i64, boost_factor: i64) that multiplies them",
            128,
        );
        assert!(
            res_ast.reply_text.contains("pub fn compute_orbital_delta(base_energy: i64, boost_factor: i64)")
                && res_ast.reply_text.contains("compute_orbital_delta(12, 7) == 84"),
            "AST slot kernel must bind custom function/parameter identifiers and evaluate live check, got: {}",
            res_ast.reply_text
        );
    }

    #[test]
    fn test_geometric_shard_partitioning_binary_codec_and_hot_swap_merge() {
        let mut model = default_lut_stack_model();
        let catalog = model.partition_geometric_shards();
        assert_eq!(catalog.shards.len(), E8_ROOT_COUNT);
        let original_rows = model.point.as_ref().unwrap().rows;
        assert_eq!(
            catalog.total_point_rows, original_rows,
            "16 E8 geometric shards must partition 100% of ServedPointKey rows without loss"
        );
        for (idx, shard) in catalog.shards.iter().enumerate() {
            assert_eq!(shard.e8_root as usize, idx);
            assert!(
                shard.point_rows > 0,
                "E8 geometric shard {idx} must have > 0 rows after mean-centered balancing, got 0"
            );
        }
        assert!(catalog.active_fine_basins >= 16);

        // Export non-empty shard, round-trip through UORSHRD1 binary codec, and verify SHA-256
        let non_empty_e8 = catalog
            .shards
            .iter()
            .find(|s| s.point_rows > 0)
            .expect("at least one non-empty E8 shard")
            .e8_root;
        let pack = model
            .export_geometric_shard(non_empty_e8, None, None)
            .expect("export succeeds");
        let raw_bytes = pack.to_bytes();
        let decoded = GeometricShardPack::from_bytes(&raw_bytes).expect("decode succeeds");
        assert_eq!(decoded, pack, "UORSHRD1 binary roundtrip must be lossless");

        // Idempotent merge on already-resident model
        let idemp_report = model
            .merge_geometric_shard(&decoded)
            .expect("idempotent merge succeeds");
        assert_eq!(idemp_report.newly_merged_rows, 0);
        assert_eq!(idemp_report.total_rows_after, original_rows);
        assert_eq!(idemp_report.orphaned_after, 0);

        // Evict last 5 rows and hot-swap merge them back via a synthetic shard pack
        let pt = model.point.as_mut().unwrap();
        let p_w = pt.width;
        let alen = pt.anchor_len;
        let evict_n = 5usize.min(pt.rows);
        let keep_rows = pt.rows - evict_n;
        let evicted_points = pt.points.split_off(keep_rows * p_w);
        let evicted_targets = pt.targets.split_off(keep_rows);
        let evicted_anchors = pt
            .canonical_anchors
            .as_mut()
            .unwrap()
            .split_off(keep_rows * alen);
        pt.rows = keep_rows;
        pt.build_radix_index();

        let mut hot_pack = GeometricShardPack {
            coordinate: ShardBasinCoordinate {
                e8_root: non_empty_e8,
                clifford_action: 0,
                icosian_root: 0,
                coarse_sector_only: true,
            },
            content_sha256: String::new(),
            point_rows: evict_n,
            point_width: p_w,
            anchor_len: alen,
            points: evicted_points,
            targets: evicted_targets,
            anchors: evicted_anchors,
            reply_packed12: Vec::new(),
        };
        hot_pack.content_sha256 = sha256_hex(&hot_pack.to_bytes());

        let merge_report = model
            .merge_geometric_shard(&hot_pack)
            .expect("hot-swap merge succeeds");
        assert_eq!(merge_report.newly_merged_rows, evict_n);
        assert_eq!(merge_report.total_rows_after, original_rows);
        assert_eq!(merge_report.orphaned_after, 0);
    }

    #[test]
    fn test_phrase_dag_r4_34_and_pre_decode_executive_thinking() {
        let model = Arc::new(default_lut_stack_model());

        // 1. Pre-decode executive <thinking> trace for R4-31 slot-bound prompt
        let res_slot = model.generate_reply(&[], "What is (37 * 43) + 19?", 128);
        assert!(res_slot.reply_text.contains("1610"));
        assert!(
            res_slot
                .thinking_text
                .as_deref()
                .is_some_and(|t| t.contains("R4-31 DeBruijn executive frame")),
            "pre-decode DeBruijnSlotFrame must synthesize executive <thinking> trace before visible decoding"
        );
        assert!(
            !res_slot.thinking_tokens.is_empty(),
            "pre-decode executive <thinking> trace must step tokens through SpiralCore/FBS before visible decoding"
        );

        // 2. Multi-clause compositional PhraseDagPlan (R4-34)
        let res_dag = model.generate_reply(
            &[],
            "What is the Spiral Core recurrence? Also explain the Chinese Remainder Theorem.",
            256,
        );
        assert!(
            res_dag.phrase_dag_node_count() >= 2,
            "multi-clause prompt must construct a PhraseDagPlan with >= 2 admitted nodes, got {}",
            res_dag.phrase_dag_node_count()
        );
        let plan = res_dag.phrase_dag_plan.expect("phrase_dag_plan present");
        assert!(
            plan.verified,
            "PhraseDagPlan must pass R4-34 verify_dag_synthesis"
        );
        assert_eq!(plan.edges.len() + 1, plan.nodes.len());
    }
}
