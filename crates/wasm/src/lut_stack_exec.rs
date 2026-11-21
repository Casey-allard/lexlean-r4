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

/// Formal R4.IntegerStack.
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

    /// Formal R4.IntegerStack.
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

    /// Formal R4.IntegerStack.
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

    /// Formal R4.IntegerStack.
    /// Record a visited turn basin row into recent memory, retaining up to the 8 most recent items (FIFO).
    pub fn record_turn_basin(&mut self, row: usize) {
        self.recent_turn_rows.push(row);
        if self.recent_turn_rows.len() > 8 {
            self.recent_turn_rows.remove(0);
        }
    }

    /// Formal R4.IntegerStack.
    /// Record an emitted 4-gram token hash to prevent duplicate n-gram emissions across turns.
    pub fn record_emitted_4gram(&mut self, hash: u64) {
        if !self.emitted_4grams.contains(&hash) {
            self.emitted_4grams.push(hash);
        }
    }

    /// Formal R4.IntegerStack.
    /// Record an observed residual prediction error margin into rolling context history (FIFO, max 16).
    pub fn record_residual_margin(&mut self, margin: u64) {
        self.accumulated_l1_margin = self.accumulated_l1_margin.saturating_add(margin);
        self.recent_residuals.push(margin);
        if self.recent_residuals.len() > 16 {
            self.recent_residuals.remove(0);
        }
    }

    /// Formal R4.IntegerStack.
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
    /// Formal R4.IntegerStack.
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
    /// Formal R4.IntegerStack.
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

    /// Formal R4.IntegerStack.
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

/// Formal R4.IntegerStack.
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

    /// Formal R4.IntegerStack.
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
    /// Formal R4.IntegerStack.
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
                    let diff = (margin_bits - dist) as u64;
                    let w = match self::slot_binding_generated::r4_integer_stack::hamming_match_weight(
                        dist as u64,
                        margin_bits as u64,
                        diff,
                    ) {
                        self::slot_binding_generated::r4_integer_stack::HammingOutcome::Value(val) => {
                            val as i64
                        }
                        _ => continue,
                    };
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
    } else {
        (anchor_id & 0x1F_FFFF).min(0x1E_FFFF)
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

    #[inline]
    pub fn score_transition_multi(
        &self,
        prev_tok: u32,
        curr_tok: u32,
        stem_buckets: &[usize],
        cand_tok: u32,
    ) -> i64 {
        if self.rows < 4096 || self.cols_bits < 4096 || cand_tok >= 4096 {
            return 0;
        }
        let cur = (curr_tok as usize) & 0xFFF;
        let mut bonus = 0i64;
        if self.buckets > 1 {
            let rem = self.buckets - 1;
            for &sb in stem_buckets {
                let stem_b = (sb % rem) + 1;
                if self.test_bit(stem_b * 4096 + cur, cand_tok) {
                    bonus += 300_000;
                }
            }
            let tri_b = (((prev_tok as usize).wrapping_mul(31)) % rem) + 1;
            if self.test_bit(tri_b * 4096 + cur, cand_tok) {
                bonus += 50_000;
            }
        }
        if self.test_bit(cur, cand_tok) {
            bonus += 5_000;
        }
        bonus
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
    pub prompt_stem_buckets: Vec<usize>,
    pub last_n_t: [i32; FBS_WIDTH],
    pub prompt_only_tokens: Vec<u32>,
    pub prompt_n_t: [i32; FBS_WIDTH],
    pub has_executive_slot_trace: bool,
    pub recent_turn_rows: Vec<usize>,
    pub emitted_4grams: Vec<u64>,
    pub boundary_trajectories: Vec<PointTrajectoryHit>,
    pub total_l1_margin: u64,
    pub subtopic_prefix: [u32; 3],
    pub subtopic_prefix_remaining: u8,
    pub subtopic_suffix_counts: Vec<([u32; 4], u32)>,
    pub commenced_visible: bool,
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
            prompt_stem_buckets: Vec::new(),
            last_n_t: [0; FBS_WIDTH],
            prompt_only_tokens: Vec::new(),
            prompt_n_t: [0; FBS_WIDTH],
            has_executive_slot_trace: false,
            recent_turn_rows: Vec::new(),
            emitted_4grams: Vec::new(),
            boundary_trajectories: Vec::new(),
            total_l1_margin: 0,
            subtopic_prefix: [MEMORY_PAD; 3],
            subtopic_prefix_remaining: 0,
            subtopic_suffix_counts: Vec::new(),
            commenced_visible: false,
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
        self.prompt_stem_buckets.clear();
        self.last_n_t = [0; FBS_WIDTH];
        self.prompt_only_tokens.clear();
        self.prompt_n_t = [0; FBS_WIDTH];
        self.has_executive_slot_trace = false;
        self.recent_turn_rows.clear();
        self.emitted_4grams.clear();
        self.boundary_trajectories.clear();
        self.total_l1_margin = 0;
        self.subtopic_prefix = [MEMORY_PAD; 3];
        self.subtopic_prefix_remaining = 0;
        self.subtopic_suffix_counts.clear();
        self.commenced_visible = false;
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

    /// Formal R4.IntegerStack.
    /// Record a visited turn basin row into recent session memory (FIFO, max 8 items).
    pub fn record_turn_basin(&mut self, row: usize) {
        self.recent_turn_rows.push(row);
        if self.recent_turn_rows.len() > 8 {
            self.recent_turn_rows.remove(0);
        }
    }

    /// Formal R4.IntegerStack.
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
        self.subtopic_prefix = [MEMORY_PAD; 3];
        self.subtopic_prefix_remaining = 0;
        self.subtopic_suffix_counts.clear();
        self.commenced_visible = false;
    }

    pub fn clear_reply_start(&mut self) {
        self.reply_start = None;
        self.reply_history.clear();
        self.suffix_counts.clear();
        self.subtopic_prefix = [MEMORY_PAD; 3];
        self.subtopic_prefix_remaining = 0;
        self.subtopic_suffix_counts.clear();
        self.commenced_visible = false;
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
            let mut reply_ids = extract_turn_ids(anchor_tokens, mem.key_window, reply_turn_k);
            for w in &mut reply_ids {
                *w &= 0x7FFF_FFFF;
            }
            self.anchors.opening = open_ids.clone();
            self.anchors.reply = reply_ids
                .iter()
                .copied()
                .filter(|&t| t != MEMORY_PAD && (t & 0x8000_0000) == 0)
                .collect();
            let active_turn_ids: Vec<u32> =
                extract_turn_ids(anchor_tokens, mem.key_window, mem.key_window.max(128))
                    .into_iter()
                    .filter(|&t| t != MEMORY_PAD && (t & 0x8000_0000) == 0)
                    .collect();
            let dec_prompt = ByteBpe4096::global().decode(&active_turn_ids);
            let stems = extract_salient_stem_hashes_from_text(&dec_prompt, 8);
            self.prompt_stem_buckets = stems
                .iter()
                .map(|&(sh, _)| (sh & 0x3FFF_FFFF) as usize)
                .collect();
            self.prompt_stem_bucket = self.prompt_stem_buckets.first().copied().unwrap_or(0);

            if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
                let val_tok = mem.opening.values[entry_idx];
                let mut found_open_row = false;
                if let (Some(reply_tbl), Some(point)) = (&mem.reply, &self.model.point) {
                    let pt_query_n_t =
                        if self.has_executive_slot_trace && !self.prompt_only_tokens.is_empty() {
                            &self.prompt_n_t
                        } else {
                            &n_t
                        };
                    if let Some(anchors) = &point.canonical_anchors {
                        let alen = point.anchor_len.max(1);
                        let q12_sh = stem_anchor_hash("q12");
                        let writer_sh = stem_anchor_hash("writer");
                        if anchors.len() >= point.rows * alen {
                            for pass in 0..2 {
                                for r in 0..point.rows {
                                    let base = r * alen;
                                    let a_row = &anchors[base..base + alen];
                                    let t_idx = if (a_row[0] & 0x8000_0000) != 0 {
                                        (a_row[0] & 0x7FFF_FFFF) as usize
                                    } else {
                                        usize::MAX
                                    };
                                    let is_synth = alen >= 18
                                        && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                                        && a_row[16] == q12_sh
                                        && a_row[17] == writer_sh;
                                    if is_synth {
                                        continue;
                                    }
                                    let is_core_or_adm = t_idx < CORE_CURRICULUM_TURNS
                                        || (alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
                                    if (pass == 0) != is_core_or_adm {
                                        continue;
                                    }
                                    let open_matches =
                                        if alen >= 16 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG {
                                            let copy_open = open_ids.len().min(8);
                                            a_row[8..8 + copy_open] == open_ids[..copy_open]
                                        } else if alen >= reply_turn_k + open_ids.len() {
                                            a_row[reply_turn_k..reply_turn_k + open_ids.len()]
                                                == *open_ids
                                        } else {
                                            false
                                        };
                                    if open_matches {
                                        let p_base = r * point.width;
                                        let p_row = &point.points[p_base..p_base + point.width];
                                        let exact_geom = point.width >= 2
                                            && (p_row[0] as i32) == pt_query_n_t[0]
                                            && (p_row[1] as i32) == pt_query_n_t[1];
                                        let stem_slice = if alen > 16 {
                                            &a_row[16..alen.min(28)]
                                        } else {
                                            &[]
                                        };
                                        let all_stems_match = if stems.is_empty() {
                                            active_turn_ids.len() <= mem.k
                                        } else {
                                            stems.iter().all(|&(sh, _)| stem_slice.contains(&sh))
                                        };
                                        if !exact_geom && !all_stems_match {
                                            continue;
                                        }
                                        let mut cand = a_row[..reply_turn_k.min(alen)].to_vec();
                                        let filtered_cand: Vec<u32> = cand
                                            .iter()
                                            .copied()
                                            .filter(|&t| t != MEMORY_PAD)
                                            .map(|w| w & 0x7FFF_FFFF)
                                            .filter(|&t| (t & 0x8000_0000) == 0)
                                            .collect();
                                        for w in &mut cand {
                                            *w &= 0x7FFF_FFFF;
                                        }
                                        if point.targets.get(r).copied() == Some(val_tok)
                                            && probe_reply_table_has_anchor(
                                                reply_tbl,
                                                mem.reply_key_len,
                                                &cand,
                                                val_tok,
                                            )
                                        {
                                            self.anchors.reply = filtered_cand;
                                            self.last_turn_idx = t_idx;
                                            found_open_row = true;
                                            break;
                                        }
                                    }
                                }
                                if found_open_row {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    found_open_row = true;
                }
                if found_open_row {
                    self.last_matched_stems = stems.len().max(4);
                    self.last_point_hamming_bits = 0;
                    blend_toward_writer_row(
                        &mut n_t,
                        &self.model.writer_table,
                        val_tok as usize,
                        &mem.opening.weight_q24,
                    );
                    memory_hit_token = Some(val_tok);
                    source_mechanism = "point_key_vote";
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
                            for w in self.anchors.reply.iter_mut() {
                                *w &= !0x8000_0000;
                            }
                        }
                        let target_tok = if let Some(reply_tbl) = mem.reply.as_ref() {
                            seek_subtopic_in_turn(
                                reply_tbl,
                                mem.reply_key_len,
                                &self.anchors.reply,
                                &mut self.subtopic_prefix,
                                &mut self.subtopic_prefix_remaining,
                                &mut self.subtopic_suffix_counts,
                                pt_hit.target_token,
                                anchor_tokens,
                                mem.key_window,
                                pt_hit.matched_specific > 0,
                            )
                        } else {
                            pt_hit.target_token
                        };
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            target_tok as usize,
                            &mem.opening.weight_q24,
                        );
                        memory_hit_token = Some(target_tok);
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

        self.commenced_visible = true;
        (st, chosen_tok)
    }

    fn next_4gram_suffix_occurrence(&mut self, tok: u32) -> ([u32; 4], u32) {
        push_4gram_suffix_occurrence(&mut self.reply_history, &mut self.suffix_counts, tok)
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
            if row + 1 == rs && !self.commenced_visible && self.anchors.reply.is_empty() {
                let open_ids = extract_turn_ids(&self.tokens, mem.key_window, mem.k);
                let mut reply_ids = extract_turn_ids(&self.tokens, mem.key_window, reply_turn_k);
                for w in &mut reply_ids {
                    *w &= 0x7FFF_FFFF;
                }
                self.anchors.opening = open_ids.clone();
                self.anchors.reply = reply_ids
                    .iter()
                    .copied()
                    .filter(|&t| t != MEMORY_PAD && (t & 0x8000_0000) == 0)
                    .collect();
                let active_turn_ids: Vec<u32> =
                    extract_turn_ids(&self.tokens, mem.key_window, mem.key_window.max(128))
                        .into_iter()
                        .filter(|&t| t != MEMORY_PAD && (t & 0x8000_0000) == 0)
                        .collect();
                let dec_prompt = ByteBpe4096::global().decode(&active_turn_ids);
                let stems = extract_salient_stem_hashes_from_text(&dec_prompt, 8);
                self.prompt_stem_buckets = stems
                    .iter()
                    .map(|&(sh, _)| (sh & 0x3FFF_FFFF) as usize)
                    .collect();
                self.prompt_stem_bucket = self.prompt_stem_buckets.first().copied().unwrap_or(0);

                if let Some(entry_idx) = mem.opening.lookup(&open_ids) {
                    let val_tok = mem.opening.values[entry_idx];
                    if let (Some(reply_tbl), Some(point)) = (&mem.reply, &self.model.point) {
                        let mut matched_open_anchor = None;
                        if let Some(anchors) = &point.canonical_anchors {
                            let alen = point.anchor_len.max(1);
                            let q12_sh = stem_anchor_hash("q12");
                            let writer_sh = stem_anchor_hash("writer");
                            if anchors.len() >= point.rows * alen {
                                for pass in 0..2 {
                                    for r in 0..point.rows {
                                        let base = r * alen;
                                        let a_row = &anchors[base..base + alen];
                                        let t_idx = if (a_row[0] & 0x8000_0000) != 0 {
                                            (a_row[0] & 0x7FFF_FFFF) as usize
                                        } else {
                                            usize::MAX
                                        };
                                        let is_synth = alen >= 18
                                            && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                                            && a_row[16] == q12_sh
                                            && a_row[17] == writer_sh;
                                        if is_synth {
                                            continue;
                                        }
                                        let is_core_or_adm = t_idx < CORE_CURRICULUM_TURNS
                                            || (alen >= 8
                                                && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
                                        if (pass == 0) != is_core_or_adm {
                                            continue;
                                        }
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
                                            let p_base = r * point.width;
                                            let p_row = &point.points[p_base..p_base + point.width];
                                            let exact_geom = point.width >= 2
                                                && (p_row[0] as i32) == n_t[0]
                                                && (p_row[1] as i32) == n_t[1];
                                            let stem_slice = if alen > 16 {
                                                &a_row[16..alen.min(28)]
                                            } else {
                                                &[]
                                            };
                                            let all_stems_match = if stems.is_empty() {
                                                active_turn_ids.len() <= mem.k
                                            } else {
                                                stems
                                                    .iter()
                                                    .all(|&(sh, _)| stem_slice.contains(&sh))
                                            };
                                            if !exact_geom && !all_stems_match {
                                                continue;
                                            }
                                            let mut cand = a_row[..reply_turn_k.min(alen)].to_vec();
                                            let filtered_cand: Vec<u32> = cand
                                                .iter()
                                                .copied()
                                                .filter(|&t| t != MEMORY_PAD)
                                                .map(|w| w & 0x7FFF_FFFF)
                                                .filter(|&t| (t & 0x8000_0000) == 0)
                                                .collect();
                                            for w in &mut cand {
                                                *w &= 0x7FFF_FFFF;
                                            }
                                            if point.targets.get(r).copied() == Some(val_tok)
                                                && probe_reply_table_has_anchor(
                                                    reply_tbl,
                                                    mem.reply_key_len,
                                                    &cand,
                                                    val_tok,
                                                )
                                            {
                                                matched_open_anchor = Some(filtered_cand);
                                                self.last_turn_idx = t_idx;
                                                break;
                                            }
                                        }
                                    }
                                    if matched_open_anchor.is_some() {
                                        break;
                                    }
                                }
                            }
                        }
                        if let Some(canon) = matched_open_anchor {
                            self.anchors.reply = canon;
                            for w in self.anchors.reply.iter_mut() {
                                *w &= !0x8000_0000;
                            }
                            self.last_matched_stems = stems.len().max(4);
                            self.last_point_hamming_bits = 0;
                            blend_toward_writer_row(
                                &mut n_t,
                                &self.model.writer_table,
                                val_tok as usize,
                                &mem.opening.weight_q24,
                            );
                            memory_hit_token = Some(val_tok);
                            source_mechanism = "point_key_vote";
                        }
                    } else {
                        self.last_matched_stems = stems.len().max(4);
                        self.last_point_hamming_bits = 0;
                        blend_toward_writer_row(
                            &mut n_t,
                            &self.model.writer_table,
                            val_tok as usize,
                            &mem.opening.weight_q24,
                        );
                        memory_hit_token = Some(val_tok);
                        source_mechanism = "point_key_vote";
                    }
                }

                if memory_hit_token.is_none() {
                    if let Some(point) = &self.model.point {
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
                                for w in self.anchors.reply.iter_mut() {
                                    *w &= !0x8000_0000;
                                }
                            }
                            let target_tok = if let Some(reply_tbl) = &mem.reply {
                                seek_subtopic_in_turn(
                                    reply_tbl,
                                    mem.reply_key_len,
                                    &self.anchors.reply,
                                    &mut self.subtopic_prefix,
                                    &mut self.subtopic_prefix_remaining,
                                    &mut self.subtopic_suffix_counts,
                                    pt_hit.target_token,
                                    &self.tokens,
                                    mem.key_window,
                                    pt_hit.matched_specific > 0,
                                )
                            } else {
                                pt_hit.target_token
                            };
                            blend_toward_writer_row(
                                &mut n_t,
                                &self.model.writer_table,
                                target_tok as usize,
                                &mem.opening.weight_q24,
                            );
                            memory_hit_token = Some(target_tok);
                            source_mechanism = if pt_hit.confident {
                                "point_key_vote"
                            } else {
                                "point_key_fallback"
                            };
                        }
                    }
                }
            } else if row + 1 >= rs {
                let (mut suffix4, mut occ) = suffix_occ.unwrap_or(([MEMORY_PAD; 4], 0));
                let had_subtopic_prefix = self.subtopic_prefix_remaining > 0;
                if had_subtopic_prefix {
                    let p = self.subtopic_prefix;
                    match self.subtopic_prefix_remaining {
                        3 => {
                            suffix4[0] = p[0];
                            suffix4[1] = p[1];
                            suffix4[2] = p[2];
                        }
                        2 => {
                            suffix4[0] = p[1];
                            suffix4[1] = p[2];
                        }
                        1 => {
                            suffix4[0] = p[2];
                        }
                        _ => {}
                    }
                    self.subtopic_prefix_remaining -= 1;
                    occ = 0;
                }
                if !self.subtopic_suffix_counts.is_empty() {
                    if let Some((_, pre_c)) = self
                        .subtopic_suffix_counts
                        .iter_mut()
                        .find(|(es, _)| *es == suffix4)
                    {
                        occ = occ.saturating_add(*pre_c);
                        if had_subtopic_prefix {
                            *pre_c = pre_c.saturating_add(1);
                        }
                    } else if had_subtopic_prefix {
                        self.subtopic_suffix_counts.push((suffix4, 1));
                    }
                }
                let step = (row + 1 - rs).max(1) as u32;
                if let Some(reply_tbl) = &mem.reply {
                    if let Some(val_tok) = lookup_reply_continuation_4gram(
                        reply_tbl,
                        mem.reply_key_len,
                        &self.anchors.reply,
                        suffix4,
                        occ,
                        step,
                    ) {
                        let next_4gram = [suffix4[1], suffix4[2], suffix4[3], val_tok];
                        let next_occ = self
                            .suffix_counts
                            .iter()
                            .find(|(s, _)| *s == next_4gram)
                            .map(|&(_, c)| c)
                            .unwrap_or(0);
                        if next_occ < 8 || val_tok == ByteBpe4096::global().eos_id() {
                            blend_toward_writer_row(
                                &mut n_t,
                                &self.model.writer_table,
                                val_tok as usize,
                                &reply_tbl.weight_q24,
                            );
                            memory_hit_token = Some(val_tok);
                            source_mechanism = "turn_memory_reply";
                        } else if step >= 16
                            && self.anchors.reply.first().copied() != Some(GLOBAL_SUFFIX_ANCHOR)
                        {
                            let eos = ByteBpe4096::global().eos_id();
                            blend_toward_writer_row(
                                &mut n_t,
                                &self.model.writer_table,
                                eos as usize,
                                &reply_tbl.weight_q24,
                            );
                            memory_hit_token = Some(eos);
                            source_mechanism = "turn_memory_reply";
                        }
                    }
                }
            }
        }

        let q_sketch = pack_i32_sign_sketch_256(&n_t);

        let mut top_candidates =
            stack_whole_table_top_k(&n_t, &self.model.writer_table, self.model.vocab, 64);
        if memory_hit_token.is_none() {
            if let Some(ta) = &self.model.ternary_assoc {
                let prev_tok = if self.tokens.len() >= 2 {
                    self.tokens[self.tokens.len() - 2]
                } else {
                    MEMORY_PAD
                };
                let contextual_prev = prev_tok;
                for (cand_tok, cand_dist) in &mut top_candidates {
                    let base = (*cand_tok as usize) * FBS_WIDTH;
                    let h_bits = if base + FBS_WIDTH <= self.model.writer_table.len() {
                        let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                        let w_sketch = pack_i16_sign_sketch_256(row_w);
                        hamming_distance_256(&q_sketch, &w_sketch)
                    } else {
                        128
                    };
                    let geom_dist = *cand_dist + (h_bits as i64) * 64;
                    let raw_tr_multi = if !self.prompt_stem_buckets.is_empty() {
                        ta.score_transition_multi(
                            contextual_prev,
                            token,
                            &self.prompt_stem_buckets,
                            *cand_tok,
                        )
                    } else {
                        ta.score_transition(
                            contextual_prev,
                            token,
                            self.prompt_stem_bucket,
                            *cand_tok,
                        ) * 50_000
                    };
                    let c_qr = (raw_tr_multi.max(0) as u64) / 50_000;
                    let c_qc = u64::from(256u32.saturating_sub(h_bits)) / 16;
                    let trunk_bonus =
                        self::slot_binding_generated::r4_discrete_trunk::level3_joint_pointer_score(
                            c_qr,
                            c_qc,
                            c_qr / 2,
                            c_qc / 2,
                            1,
                            h_bits <= 96,
                        ) as i64;
                    let bonus = raw_tr_multi.saturating_add(trunk_bonus * 16);
                    *cand_dist = geom_dist.saturating_sub(bonus);
                }
                top_candidates.sort_by_key(|&(_, d)| d);
            } else {
                for (cand_tok, cand_dist) in &mut top_candidates {
                    let base = (*cand_tok as usize) * FBS_WIDTH;
                    let h_bits = if base + FBS_WIDTH <= self.model.writer_table.len() {
                        let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                        let w_sketch = pack_i16_sign_sketch_256(row_w);
                        hamming_distance_256(&q_sketch, &w_sketch)
                    } else {
                        128
                    };
                    let c_qc = u64::from(256u32.saturating_sub(h_bits)) / 16;
                    let trunk_bonus =
                        self::slot_binding_generated::r4_discrete_trunk::level3_joint_pointer_score(
                            0,
                            c_qc,
                            0,
                            c_qc / 2,
                            1,
                            h_bits <= 96,
                        ) as i64;
                    *cand_dist =
                        (*cand_dist + (h_bits as i64) * 1024).saturating_sub(trunk_bonus * 8);
                }
                top_candidates.sort_by_key(|&(_, d)| d);
            }

            // Execute formal R4.Admission + R4.Prediction cell admission and token ranking
            // (`r4_admission::occupied` -> `r4_admission::take_cells` -> `r4_admission::rank_tokens` -> `r4_prediction::prediction_scores`)
            let min_dist = top_candidates.iter().map(|&(_, d)| d).min().unwrap_or(0);
            let admission_tokens: Vec<self::slot_binding_generated::r4_admission::TokenScore> =
                top_candidates
                    .iter()
                    .take(24)
                    .map(|&(cand_tok, cand_dist)| {
                        let base = (cand_tok as usize) * FBS_WIDTH;
                        let (cell_id, addr_dist) =
                            if base + FBS_WIDTH <= self.model.writer_table.len() {
                                let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                                let w_sketch = pack_i16_sign_sketch_256(row_w);
                                let (e8_c, _) = compute_geometric_e8_pair(row_w, w_sketch[0]);
                                let first_last_l1 = ((n_t[0] - i32::from(row_w[0])).unsigned_abs()
                                    + (n_t[FBS_WIDTH - 1] - i32::from(row_w[FBS_WIDTH - 1]))
                                        .unsigned_abs())
                                    as u64;
                                (u64::from(e8_c), first_last_l1)
                            } else {
                                (u64::from(cand_tok & 15), 65_535)
                            };
                        let shifted_dist = (cand_dist - min_dist).max(0) as u64;
                        self::slot_binding_generated::r4_admission::TokenScore {
                            identity: u64::from(cand_tok),
                            cell: cell_id,
                            distance: shifted_dist,
                            address_distance: addr_dist,
                        }
                    })
                    .collect();
            if let self::slot_binding_generated::r4_prediction::PredictionResult::Value(receipt) =
                self::slot_binding_generated::r4_prediction::prediction_scores(
                    self::slot_binding_generated::r4_admission::ScoresOutcome::Value(
                        admission_tokens,
                    ),
                    16,
                )
            {
                let mut admitted_ordered = Vec::with_capacity(top_candidates.len());
                for &ranked_id in &receipt.ranked_tokens {
                    if let Some(&entry) = top_candidates
                        .iter()
                        .find(|&&(ct, _)| u64::from(ct) == ranked_id)
                    {
                        admitted_ordered.push(entry);
                    }
                }
                for &entry in &top_candidates {
                    if !admitted_ordered.iter().any(|&(ct, _)| ct == entry.0) {
                        admitted_ordered.push(entry);
                    }
                }
                top_candidates = admitted_ordered;
            }
            top_candidates.truncate(5);
            if let Some(ta) = &self.model.ternary_assoc {
                let prev_tok = if self.tokens.len() >= 2 {
                    self.tokens[self.tokens.len() - 2]
                } else {
                    MEMORY_PAD
                };
                if top_candidates.first().is_some_and(|&(ct, _)| {
                    if !self.prompt_stem_buckets.is_empty() {
                        ta.score_transition_multi(prev_tok, token, &self.prompt_stem_buckets, ct)
                            > 0
                    } else {
                        ta.score_transition(prev_tok, token, self.prompt_stem_bucket, ct) > 0
                    }
                }) {
                    source_mechanism = "ternary_assoc_nearest";
                }
            }
        } else {
            top_candidates.truncate(5);
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
        let (predicted_token, l1_distance, source_mechanism) = if source_mechanism
            == "turn_memory_reply"
            || !has_full_4gram
        {
            let (tok, dist) = if let Some(hit_tok) = memory_hit_token {
                let dist = top_candidates
                    .iter()
                    .find(|&&(t, _)| t == hit_tok)
                    .map(|&(_, d)| d)
                    .unwrap_or(0);
                (hit_tok, dist)
            } else if top_candidates.len() >= 2 {
                let (t0, d0) = top_candidates[0];
                let (t1, d1) = top_candidates[1];
                let min_d = d0.min(d1);
                let mut recent_ids = vec![u64::from(token)];
                if t_minus_2 != MEMORY_PAD {
                    recent_ids.push(u64::from(t_minus_2));
                }
                let chosen = self::slot_binding_generated::r4_prediction::select_contextual_token(
                    u64::from(t0),
                    (d0 - min_d).max(0) as u64,
                    u64::from(t1),
                    (d1 - min_d).max(0) as u64,
                    recent_ids,
                    500_000,
                ) as u32;
                let dist = top_candidates
                    .iter()
                    .find(|&&(t, _)| t == chosen)
                    .map(|&(_, d)| d)
                    .unwrap_or(d0);
                (chosen, dist)
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
                if memory_hit_token.is_none() {
                    if cand_tok == token && top_candidates.len() > 1 {
                        continue;
                    }
                    if t_minus_2 != MEMORY_PAD && cand_tok == t_minus_2 && top_candidates.len() > 2
                    {
                        continue;
                    }
                }
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
                let mut best_unseen: Option<(u32, i64)> = None;
                let max_v = self
                    .model
                    .vocab
                    .min(self.model.writer_table.len() / FBS_WIDTH);
                for v in 0..max_v {
                    let cand_tok = v as u32;
                    if cand_tok == token {
                        continue;
                    }
                    let cand_h = hash_4gram(t_minus_3, t_minus_2, t_minus_1, cand_tok);
                    if self.emitted_4grams.contains(&cand_h) {
                        continue;
                    }
                    let base = v * FBS_WIDTH;
                    let row_w = &self.model.writer_table[base..base + FBS_WIDTH];
                    let dist: i64 = (0..FBS_WIDTH)
                        .map(|d| ((n_t[d] as i64) - (row_w[d] as i64)).abs())
                        .sum();
                    if best_unseen.is_none_or(|(_, bd)| dist < bd) {
                        best_unseen = Some((cand_tok, dist));
                    }
                }
                let (nearest_tok, nearest_dist) = best_unseen.unwrap_or_else(|| {
                    stack_whole_table_nearest(&n_t, &self.model.writer_table, self.model.vocab)
                });
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
            l1_distance: l1_distance.max(0),
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
    if reply_key_len < 6 || anchor.is_empty() {
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
    let lookup_with_bit31_flex = |tbl: &TurnMemoryTable, k: &mut [u32]| -> Option<u32> {
        if let Some(val) = tbl.lookup_value(k) {
            return Some(val);
        }
        if !k.is_empty() && k[0] != MEMORY_PAD {
            let orig = k[0];
            k[0] ^= 0x8000_0000;
            let val = tbl.lookup_value(k);
            k[0] = orig;
            if val.is_some() {
                return val;
            }
        }
        None
    };

    if let Some(val) = lookup_with_bit31_flex(reply_tbl, &mut key) {
        return Some(val);
    }
    if occ >= 8 {
        return None;
    }
    let in_opening_window = suffix4[0] == MEMORY_PAD || legacy_step <= 4;
    if occ > 0 && in_opening_window {
        key[reply_key_len - 5] = 0;
        if let Some(val) = lookup_with_bit31_flex(reply_tbl, &mut key) {
            return Some(val);
        }
    }
    // Tier 1.5: Anchor-bound 3-gram backoff: `(anchor, 0, PAD, prev_2, prev_1, curr_tok)`
    if in_opening_window && suffix4[1] != MEMORY_PAD {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = suffix4[1];
        key[reply_key_len - 2] = suffix4[2];
        key[reply_key_len - 1] = suffix4[3];
        if let Some(val) = lookup_with_bit31_flex(reply_tbl, &mut key) {
            return Some(val);
        }
    }
    // Tier 2: Anchor-bound 2-gram backoff: `(anchor, 0, PAD, PAD, prev_1, curr_tok)`
    if in_opening_window && suffix4[2] != MEMORY_PAD {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = MEMORY_PAD;
        key[reply_key_len - 2] = suffix4[2];
        key[reply_key_len - 1] = suffix4[3];
        if let Some(val) = lookup_with_bit31_flex(reply_tbl, &mut key) {
            return Some(val);
        }
    }
    // Tier 2b: Anchor-bound 1-gram / opening continuation fallback
    if in_opening_window {
        key[reply_key_len - 5] = 0;
        key[reply_key_len - 4] = MEMORY_PAD;
        key[reply_key_len - 3] = MEMORY_PAD;
        key[reply_key_len - 2] = MEMORY_PAD;
        key[reply_key_len - 1] = suffix4[3];
        if let Some(val) = lookup_with_bit31_flex(reply_tbl, &mut key) {
            return Some(val);
        }
    }
    // Tier 4: Compatibility fallback for step-indexed `.uorlut` tables (within the same turn anchor)
    let mut legacy_key = vec![MEMORY_PAD; reply_key_len];
    let leg_copy = anchor.len().min(reply_key_len.saturating_sub(4));
    legacy_key[..leg_copy].copy_from_slice(&anchor[..leg_copy]);
    legacy_key[reply_key_len - 4] = MEMORY_PAD;
    legacy_key[reply_key_len - 3] = legacy_step;
    legacy_key[reply_key_len - 2] = MEMORY_PAD;
    legacy_key[reply_key_len - 1] = suffix4[3];
    if let Some(val) = reply_tbl.lookup_value(&legacy_key) {
        return Some(val);
    }

    if suffix4[2] != MEMORY_PAD
        && anchor.first().copied() != Some(GLOBAL_SUFFIX_ANCHOR)
        && legacy_step >= 12
        && (is_boundary_token(suffix4[3])
            || is_boundary_token(suffix4[2])
            || is_boundary_token(suffix4[1])
            || ByteBpe4096::global().decode(&[suffix4[3]]).contains('}')
            || legacy_step >= 32)
    {
        return Some(ByteBpe4096::global().eos_id());
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
    None
}

#[allow(dead_code)]
fn push_4gram_suffix_occurrence(
    reply_history: &mut Vec<u32>,
    suffix_counts: &mut Vec<([u32; 4], u32)>,
    tok: u32,
) -> ([u32; 4], u32) {
    reply_history.push(tok);
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
    let suffix = [s0, s1, s2, s3];
    for (existing_suf, count) in suffix_counts.iter_mut() {
        if *existing_suf == suffix {
            let occ = *count;
            *count = count.saturating_add(1);
            return (suffix, occ);
        }
    }
    suffix_counts.push((suffix, 1));
    (suffix, 0)
}

#[allow(clippy::too_many_arguments)]
fn seek_subtopic_in_turn(
    reply_tbl: &TurnMemoryTable,
    reply_key_len: usize,
    anchors_reply: &[u32],
    subtopic_prefix: &mut [u32; 3],
    subtopic_prefix_remaining: &mut u8,
    suffix_counts: &mut Vec<([u32; 4], u32)>,
    default_first_tok: u32,
    prompt_tokens: &[u32],
    key_window: usize,
    matched_specific_in_row: bool,
) -> u32 {
    if anchors_reply.is_empty() || reply_key_len < 6 {
        return default_first_tok;
    }
    let bpe = ByteBpe4096::global();
    let prompt_ids = extract_turn_ids(prompt_tokens, key_window, key_window.max(128));
    let active_ids: Vec<u32> = prompt_ids
        .into_iter()
        .filter(|&t| t != MEMORY_PAD)
        .collect();
    let decoded_turn = bpe.decode(&active_ids).to_ascii_lowercase();
    if is_lut_self_inquiry_prompt(&decoded_turn) || is_lut_greeting_only_prompt(&decoded_turn) {
        return default_first_tok;
    }
    let prompt_stems = extract_salient_stem_hashes_from_text(&decoded_turn, 16);
    let specific_stems: Vec<u32> = prompt_stems
        .iter()
        .filter(|&&(_, sw)| sw >= 140_000)
        .map(|&(sh, _)| sh)
        .collect();
    if specific_stems.is_empty() {
        return default_first_tok;
    }

    let mut turn_tokens = Vec::with_capacity(420);
    let mut sim_history: Vec<u32> = Vec::with_capacity(420);
    let mut sim_counts: Vec<([u32; 4], u32)> = Vec::new();
    let mut cur = default_first_tok;
    let eos = bpe.eos_id();
    let bos = bpe.bos_id();
    for step_i in 0..420 {
        if cur == eos || cur == bos || cur >= 4096 {
            break;
        }
        turn_tokens.push(cur);
        sim_history.push(cur);
        let n = sim_history.len();
        let s0 = if n >= 4 {
            sim_history[n - 4]
        } else {
            MEMORY_PAD
        };
        let s1 = if n >= 3 {
            sim_history[n - 3]
        } else {
            MEMORY_PAD
        };
        let s2 = if n >= 2 {
            sim_history[n - 2]
        } else {
            MEMORY_PAD
        };
        let s3 = sim_history[n - 1];
        let suffix = [s0, s1, s2, s3];
        let mut occ = 0u32;
        let mut found = false;
        for (es, c) in &mut sim_counts {
            if *es == suffix {
                occ = *c;
                *c = c.saturating_add(1);
                found = true;
                break;
            }
        }
        if !found {
            sim_counts.push((suffix, 1));
        }
        if occ >= 8 {
            break;
        }
        let Some(next_t) = lookup_reply_continuation_4gram(
            reply_tbl,
            reply_key_len,
            anchors_reply,
            suffix,
            occ,
            (step_i + 1) as u32,
        ) else {
            break;
        };
        if next_t == eos || next_t == bos {
            break;
        }
        cur = next_t;
    }

    if turn_tokens.len() < 26 {
        return default_first_tok;
    }

    let open_text = bpe
        .decode(&turn_tokens[..turn_tokens.len().min(20)])
        .to_ascii_lowercase();
    let open_stems = extract_salient_stem_hashes_from_text(&open_text, 16);
    let open_spec_count = specific_stems
        .iter()
        .filter(|&&sh| open_stems.iter().any(|&(osh, _)| osh == sh))
        .count();
    if open_spec_count > 0 {
        return default_first_tok;
    }

    let max_start = turn_tokens.len().saturating_sub(18);
    let mut best_pos = 0usize;
    let mut best_score = 0i64;
    let mut clause_positions = Vec::new();
    let mut word_positions = Vec::new();

    for pos in 1..=max_start {
        let prev_dec = bpe.decode(&[turn_tokens[pos - 1]]);
        let cur_dec = bpe.decode(&[turn_tokens[pos]]);
        let is_punct_boundary = prev_dec.contains('.')
            || prev_dec.contains(';')
            || prev_dec.contains(':')
            || prev_dec.contains('\n')
            || prev_dec.contains("—")
            || prev_dec.contains("**");
        let is_word_start = is_punct_boundary
            || cur_dec.starts_with(" **")
            || cur_dec.starts_with("**")
            || cur_dec
                .strip_prefix(' ')
                .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_uppercase()));
        if is_punct_boundary && pos >= 4 {
            clause_positions.push(pos);
        }
        if !is_word_start {
            continue;
        }
        if pos >= 2 {
            word_positions.push(pos);
        }

        let win_end = (pos + 8).min(turn_tokens.len());
        let win_text = bpe.decode(&turn_tokens[pos..win_end]).to_ascii_lowercase();
        let win_stems = extract_salient_stem_hashes_from_text(&win_text, 12);
        let mut spec_hits = 0usize;
        let mut missing_spec_hits = 0usize;
        for &sh in &specific_stems {
            if win_stems.iter().any(|&(wsh, _)| wsh == sh) {
                spec_hits += 1;
                if !open_stems.iter().any(|&(osh, _)| osh == sh) {
                    missing_spec_hits += 1;
                }
            }
        }
        if missing_spec_hits > 0 && spec_hits > open_spec_count {
            let mut sc = (missing_spec_hits as i64) * 200_000 + (spec_hits as i64) * 100_000;
            if is_punct_boundary {
                sc += 50_000;
            }
            let first_win_stems = extract_salient_stem_hashes_from_text(
                &bpe.decode(&turn_tokens[pos..(pos + 3).min(turn_tokens.len())])
                    .to_ascii_lowercase(),
                4,
            );
            if specific_stems
                .iter()
                .any(|&sh| first_win_stems.iter().any(|&(fsh, _)| fsh == sh))
            {
                sc += 80_000;
            }
            if sc > best_score {
                best_score = sc;
                best_pos = pos;
            }
        }
    }

    let _ = matched_specific_in_row;
    if best_pos == 0 && open_spec_count == 0 {
        let pool = if clause_positions.len() >= 10 {
            &clause_positions
        } else {
            &word_positions
        };
        if !pool.is_empty() {
            let h = fnv1a64(decoded_turn.as_bytes())
                ^ specific_stems.iter().fold(0u64, |acc, &sh| {
                    acc.wrapping_mul(1_099_511_628_211) ^ (sh as u64)
                });
            best_pos = pool[(h as usize) % pool.len()];
        }
    }
    suffix_counts.clear();
    if best_pos > 0 && best_pos < turn_tokens.len() {
        for i in 0..best_pos {
            let s0 = if i >= 3 {
                turn_tokens[i - 3]
            } else {
                MEMORY_PAD
            };
            let s1 = if i >= 2 {
                turn_tokens[i - 2]
            } else {
                MEMORY_PAD
            };
            let s2 = if i >= 1 {
                turn_tokens[i - 1]
            } else {
                MEMORY_PAD
            };
            let s3 = turn_tokens[i];
            let suf = [s0, s1, s2, s3];
            if let Some((_, cnt)) = suffix_counts.iter_mut().find(|(es, _)| *es == suf) {
                *cnt = cnt.saturating_add(1);
            } else {
                suffix_counts.push((suf, 1));
            }
        }
        *subtopic_prefix = [
            if best_pos >= 3 {
                turn_tokens[best_pos - 3]
            } else {
                MEMORY_PAD
            },
            if best_pos >= 2 {
                turn_tokens[best_pos - 2]
            } else {
                MEMORY_PAD
            },
            turn_tokens[best_pos - 1],
        ];
        *subtopic_prefix_remaining = 3;
        return turn_tokens[best_pos];
    }

    default_first_tok
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
        for i in (0..slice.len()).rev() {
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
            | "define"
            | "defin"
            | "describ"
            | "summarize"
            | "summariz"
            | "summary"
            | "summari"
            | "overview"
            | "outline"
            | "outlin"
            | "plot"
            | "plots"
            | "state"
            | "first"
            | "second"
            | "third"
            | "finally"
            | "now"
            | "shifting"
            | "shift"
            | "away"
            | "main"
    )
}

pub fn is_umbrella_domain_context_stem(w: &str) -> bool {
    matches!(
        w,
        "mathematical"
            | "mathematics"
            | "mathematic"
            | "mathemat"
            | "axiom"
            | "axioms"
            | "physics"
            | "physical"
            | "physic"
            | "quantum"
            | "biology"
            | "biological"
            | "biolog"
            | "chemistry"
            | "chemical"
            | "science"
            | "scientific"
            | "cellular"
            | "stages"
            | "stage"
            | "stag"
            | "capital"
            | "theorem"
            | "theorems"
            | "function"
            | "functions"
            | "func"
            | "array"
            | "arrays"
            | "rust"
            | "compiler"
            | "compilers"
            | "compil"
            | "optimize"
            | "optim"
            | "finite"
            | "finit"
            | "automata"
            | "process"
            | "making"
            | "scratch"
            | "last"
            | "theory"
            | "theori"
            | "principle"
            | "principles"
            | "principl"
            | "concept"
            | "concepts"
            | "method"
            | "methods"
            | "system"
            | "systems"
            | "language"
            | "languag"
            | "model"
            | "models"
            | "impact"
            | "rule"
            | "rules"
            | "movement"
            | "movements"
            | "piece"
            | "pieces"
            | "piec"
            | "game"
            | "location"
            | "loca"
            | "design"
            | "designs"
            | "notable"
            | "notabl"
            | "primary"
            | "primari"
            | "geology"
            | "geolog"
            | "oceanography"
            | "oceanograph"
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
    // 2. Then extract individual alphanumeric words (skipping negated hyphenated compounds like "non-deterministic"
    // so they are not split into their positive antonym "deterministic")
    for raw_tok in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
        let tok_lower = raw_tok.to_ascii_lowercase();
        if tok_lower.starts_with("non-")
            || tok_lower.starts_with("un-")
            || tok_lower.starts_with("anti-")
        {
            continue;
        }
        for raw_w in raw_tok.split(|c: char| !c.is_ascii_alphanumeric()) {
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
            let weight = if is_umbrella_domain_context_stem(&lower)
                || is_umbrella_domain_context_stem(&stem)
            {
                90_000i64
            } else if stem.len() >= 4 {
                240_000i64
            } else {
                140_000i64
            };
            out.push((sh, weight));
            if out.len() >= max_stems {
                break;
            }
        }
        if out.len() >= max_stems {
            break;
        }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.1));
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
        || lower.contains("geometric language model")
        || lower.contains("differ from transformer")
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
    let mut best_a_u64 = 0u64;
    let mut best_b_u64 = 0u64;
    for (idx, root) in ICOSIAN_ROOTS_Q12.iter().enumerate() {
        let mut a = 0i64;
        let mut b = 0i64;
        for lane in 0..4 {
            let x = i64::from(lanes[lane]);
            a += x * i64::from(root[lane].0);
            b += x * i64::from(root[lane].1);
        }
        let score = 2 * a + 3 * b;
        let cand_a_u64 = (a + (1i64 << 30)).max(0) as u64;
        let cand_b_u64 = (b + (1i64 << 30)).max(0) as u64;
        let beats = if idx == 0 {
            true
        } else {
            self::slot_binding_generated::r4_spiral_core_fbs::phi_score_strictly_beats(
                cand_a_u64, cand_b_u64, best_a_u64, best_b_u64,
            ) || score > best_score
        };
        if beats && score >= best_score {
            best_score = score;
            best_root = idx as u8;
            best_a_u64 = cand_a_u64;
            best_b_u64 = cand_b_u64;
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
    match self::slot_binding_generated::r4_radix_otel::resolve_radix_bucket(
        e8 as u64, c64 as u64, i17 as u64,
    ) {
        self::slot_binding_generated::r4_radix_otel::RadixBucketOutcome::BucketIndex(
            b,
            _,
            _,
            _,
        ) => b as usize,
        _ => (e8 * CLIFFORD_ACTION_COUNT + c64) * ICOSIAN_ROOT_COUNT + i17,
    }
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
    pub matched_specific: usize,
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
                matched_specific: usize::from(d == 0),
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
                matched_specific: usize::from(d == 0),
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
        matched_primary: usize,
        matched_specific: usize,
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
        matched_primary: 0,
        matched_specific: 0,
        covered_mask: 0,
        matched_bpe: 0,
        turn_idx: usize::MAX,
        bonus: 0,
        max_core_matched_stems: 0,
    };

    let has_specific_stem = prompt_stems.iter().any(|&(_, sw)| sw >= 140_000);
    let is_algebra_prompt = decoded_turn.contains("solve")
        || decoded_turn.contains("equation")
        || decoded_turn.contains("linear")
        || decoded_turn.contains("quadratic")
        || decoded_turn.contains("integral")
        || decoded_turn.contains("derivative")
        || decoded_turn.chars().any(|c| c.is_ascii_digit());
    let eq_stem_hash = stem_anchor_hash("equation");
    let lin_stem_hash = stem_anchor_hash("linear");
    let quad_stem_hash = stem_anchor_hash("quadratic");
    let int_stem_hash = stem_anchor_hash("integral");
    let q12_stem_hash = stem_anchor_hash("q12");
    let writer_stem_hash = stem_anchor_hash("writer");

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
        let is_synthetic_frontier_row = alen >= 18
            && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
            && a_row[16] == q12_stem_hash
            && a_row[17] == writer_stem_hash;
        let is_core_or_proved = turn_idx < CORE_CURRICULUM_TURNS
            || (alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG);
        if !is_core_or_proved {
            let core_already_complete = is_self_inq
                || is_greeting
                || (best.raw_l1 == 0 && best.turn_idx < CORE_CURRICULUM_TURNS)
                || (total_stems > 0 && best.max_core_matched_stems >= total_stems.max(2));
            if core_already_complete {
                return false;
            }
        }

        // Guard self-inquiry bio rows (0..=2, 8) and pure greeting rows (3..=4) so unrelated prompts
        // never collapse onto "I am the UOR-R4 Generalized Geometric Assistant..."
        let is_self_inq_turn = turn_idx <= 2 || turn_idx == 8;
        let mut guard_penalty = 0i64;
        if is_self_inq_turn != is_self_inq
            && !(is_self_inq
                && alen >= 8
                && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG
                && alen >= 22
                && a_row[16..22]
                    .iter()
                    .filter(|&&sh| {
                        sh != 0
                            && sh != MEMORY_PAD
                            && prompt_stems.iter().any(|&(psh, _)| psh == sh)
                    })
                    .count()
                    >= 2)
        {
            guard_penalty = 80_000_000;
        } else if (turn_idx == 3 || turn_idx == 4) != is_greeting {
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
        if is_self_inq && is_self_inq_turn {
            bonus += 800_000;
            let asks_compare = decoded_turn.contains("differ")
                || decoded_turn.contains("compare")
                || decoded_turn.contains("gpt")
                || decoded_turn.contains("claude")
                || decoded_turn.contains("chatgpt")
                || decoded_turn.contains("instead of");
            let asks_arch = decoded_turn.contains("how do you work")
                || decoded_turn.contains("under the hood")
                || decoded_turn.contains("architect")
                || decoded_turn.contains("transform")
                || decoded_turn.contains("geometric language model")
                || decoded_turn.contains("lut-stack");
            let asks_who = decoded_turn.contains("who are you")
                || decoded_turn.contains("what are you")
                || decoded_turn.contains("what model")
                || decoded_turn.contains("introduce yourself")
                || decoded_turn.contains("about yourself");
            if (asks_compare && turn_idx == 8)
                || (asks_arch && !asks_compare && turn_idx == 2)
                || (asks_who && !asks_arch && !asks_compare && turn_idx == 1)
                || (!asks_arch && !asks_who && !asks_compare && turn_idx == 0)
            {
                bonus += 600_000;
            }
        } else if is_greeting && (turn_idx == 3 || turn_idx == 4) {
            bonus += 800_000;
        }
        let mut matched_stems = 0usize;
        let mut matched_primary = 0usize;
        let mut matched_specific = 0usize;
        let mut matched_primary_specific = 0usize;
        let mut covered_mask = 0u16;
        for (stem_i, &(sh, sw)) in prompt_stems.iter().enumerate() {
            if primary_slice.contains(&sh) {
                matched_stems += 1;
                matched_primary += 1;
                if sw >= 140_000 {
                    matched_specific += 1;
                    matched_primary_specific += 1;
                }
                if stem_i < 16 {
                    covered_mask |= 1u16 << stem_i;
                }
                bonus += sw;
                if primary_slice.first() == Some(&sh) {
                    bonus += 65_000;
                }
            } else if let Some(sec_pos) = secondary_slice.iter().position(|&x| x == sh) {
                matched_stems += 1;
                if sw >= 140_000 {
                    matched_specific += 1;
                }
                if stem_i < 16 {
                    covered_mask |= 1u16 << stem_i;
                }
                bonus += sw / 2 + ((6usize.saturating_sub(sec_pos) as i64) * 15_000);
            }
        }
        if is_synthetic_frontier_row
            && matched_primary < 3
            && (w_lim < 2 || (p_row[0] as i32) != n_t[0] || (p_row[1] as i32) != n_t[1])
        {
            return false;
        }
        if guard_penalty == 0
            && turn_idx < CORE_CURRICULUM_TURNS
            && (alen < 8 || a_row[7] != VERIFIED_FRONTIER_ANCHOR_TAG)
            && primary_slice.iter().any(|&x| x != 0 && x != MEMORY_PAD)
            && (basin_penalty > 0
                || w_lim < 2
                || (p_row[0] as i32) != n_t[0]
                || (p_row[1] as i32) != n_t[1])
            && !is_algebra_prompt
            && !is_self_inq
            && !is_greeting
            && ((has_specific_stem && matched_specific == 0)
                || primary_slice.contains(&eq_stem_hash)
                || primary_slice.contains(&lin_stem_hash)
                || primary_slice.contains(&quad_stem_hash)
                || primary_slice.contains(&int_stem_hash))
        {
            guard_penalty = 40_000_000;
        }
        if matched_primary > 0 && (!has_specific_stem || matched_specific > 0) {
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
        if matched_stems >= 2 && (!has_specific_stem || matched_primary_specific > 0) {
            bonus += (matched_stems as i64) * 180_000;
        }
        if is_core_or_proved && matched_stems >= 1 && (!has_specific_stem || matched_specific > 0) {
            if matched_primary_specific > best.max_core_matched_stems {
                best.max_core_matched_stems = matched_primary_specific;
            }
            let non_spec = matched_stems.saturating_sub(matched_specific) as i64;
            if has_specific_stem && matched_primary_specific == 0 {
                bonus += (matched_specific as i64) * 140_000 + non_spec * 60_000;
            } else {
                bonus += (matched_specific as i64) * 320_000 + non_spec * 90_000;
            }
        } else if is_core_or_proved
            && has_specific_stem
            && matched_specific == 0
            && !(is_self_inq && is_self_inq_turn)
            && !(is_greeting && (turn_idx == 3 || turn_idx == 4))
        {
            bonus = if alen >= 8 && a_row[7] == VERIFIED_FRONTIER_ANCHOR_TAG {
                bonus.min(90_000)
            } else {
                0
            };
        } else if !is_core_or_proved {
            if has_specific_stem && matched_specific == 0 {
                return false;
            }
            let ext_eligible = if best.max_core_matched_stems == 0 {
                if total_stems >= 2 {
                    matched_primary >= 1
                        && (matched_stems >= 2 || (matched_primary == 1 && row_primary_count <= 2))
                } else {
                    matched_primary >= 1
                }
            } else if best.max_core_matched_stems == 1 {
                total_stems >= 2
                    && matched_primary >= 1
                    && (matched_stems >= 2 || (matched_specific >= 1 && row_primary_count <= 2))
            } else {
                matched_primary >= 1 && matched_specific >= best.max_core_matched_stems
            };
            if !ext_eligible {
                return false;
            }
            let stem_gain = matched_specific.saturating_sub(best.max_core_matched_stems) as i64;
            let non_spec = matched_stems.saturating_sub(matched_specific) as i64;
            bonus += (matched_specific as i64) * 350_000 + non_spec * 110_000 + stem_gain * 180_000;
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
            best.matched_primary = matched_primary;
            best.matched_specific = matched_specific;
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
        let spec_ok = !has_specific_stem || best.matched_specific >= 1;
        (best.raw_l1 == 0 && ((best.turn_idx > 2 && best.turn_idx != 8) || is_self_inq))
            || (is_self_inq && (best.turn_idx <= 2 || best.turn_idx == 8))
            || (is_greeting && (best.turn_idx == 3 || best.turn_idx == 4))
            || (spec_ok
                && ((total_stems > 0 && best.matched_stems >= total_stems)
                    || (best.matched_stems >= 2 && best.matched_primary >= 1)
                    || (best.matched_primary >= 1 && best.bonus >= 200_000)
                    || (best.matched_stems >= 3 && best.matched_stems * 2 >= total_stems)
                    || (best.matched_stems >= 3 && best.bonus >= 500_000)
                    || (best.matched_stems == 1 && total_stems <= 2 && best.bonus >= 160_000)))
            || (total_stems == 0
                && !prompt_bpe_content.is_empty()
                && best.matched_bpe >= prompt_bpe_content.len().min(2))
            || (total_stems == 0 && prompt_bpe_content.is_empty() && best.score < 10_000_000)
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

            // Stage 0: Fast-path for self-inquiries and greetings: evaluate Core Curriculum (rows 0..CORE_CURRICULUM_TURNS)
            let mut exact_hit = false;
            if is_self_inq || is_greeting {
                let core_limit = point.rows.min(CORE_CURRICULUM_TURNS);
                for r in 0..core_limit {
                    if eval_row(r, &mut best) {
                        exact_hit = true;
                        break;
                    }
                }
            }

            // Stage 1: Primary geometric bucket
            let primary_bucket = compute_geometric_bucket(e8_1, q_c64, i17_1);
            if !exact_hit
                && !is_self_inq
                && !is_greeting
                && (!is_confident(&best) || best.matched_stems < total_stems.min(4))
            {
                exact_hit = eval_bucket(primary_bucket, &mut best);
            }

            // Stage 2: Geometric neighborhood across E8, Cl(0,6), and Icosian golden lattice (28 buckets)
            // Probed only if primary bucket did not yield an unpenalized, confident candidate
            let needs_neighbors = !exact_hit
                && !is_self_inq
                && !is_greeting
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

            // Stage 3a: Core curriculum fallback (0..CORE_CURRICULUM_TURNS) when geometric neighborhood did not yield a confident hit
            if !exact_hit
                && (best.score == i64::MAX
                    || !is_confident(&best)
                    || ((best.row >= recent_min
                        && best.row <= recent_max
                        && recent_turn_rows.contains(&best.row))
                        && best.turn_idx < CORE_CURRICULUM_TURNS))
            {
                let core_limit = point.rows.min(CORE_CURRICULUM_TURNS);
                for r in 0..core_limit {
                    if eval_row(r, &mut best) {
                        break;
                    }
                }
            }

            // Stage 3b: External corpus fallback (0..rows) when still not confident or when
            // earlier stages did not match all prompt stems.
            if !exact_hit
                && !is_self_inq
                && !is_greeting
                && (!is_confident(&best) || best.matched_stems < total_stems.min(4))
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
            matched_specific: best.matched_specific,
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
                matched_specific: usize::from(d == 0),
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
            let v: Vec<u32> = anchors[base..base + reply_k]
                .iter()
                .copied()
                .filter(|&t| t != MEMORY_PAD)
                .map(|w| w & 0x7FFF_FFFF)
                .collect();
            return Some(v);
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
            let v: Vec<u32> = k_slice
                .iter()
                .copied()
                .filter(|&t| t != MEMORY_PAD)
                .map(|w| w & 0x7FFF_FFFF)
                .collect();
            best_entry = Some(v);
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
            let after_open = &current[start + open_len..];
            let partial_tag = if close_tag == "</thinking>" {
                "</thinking"
            } else {
                "</thought"
            };
            let found_close = after_open
                .find(close_tag)
                .map(|idx| (idx, close_len))
                .or_else(|| {
                    after_open
                        .find(partial_tag)
                        .map(|idx| (idx, partial_tag.len()))
                });
            if let Some((rel_end, actual_close_len)) = found_close {
                let mut trace = current[start + open_len..start + open_len + rel_end]
                    .trim()
                    .to_string();
                if trace.starts_with("<thinking>") {
                    trace = trace["<thinking>".len()..].trim_start().to_string();
                } else if trace.starts_with("<thought>") {
                    trace = trace["<thought>".len()..].trim_start().to_string();
                }
                if trace.ends_with("</thinking>") {
                    trace = trace[..trace.len() - "</thinking>".len()]
                        .trim_end()
                        .to_string();
                } else if trace.ends_with("</thought>") {
                    trace = trace[..trace.len() - "</thought>".len()]
                        .trim_end()
                        .to_string();
                }
                if !trace.is_empty() {
                    traces.push(trace);
                }
                let mut clean = String::with_capacity(current.len());
                clean.push_str(&current[..start]);
                clean.push_str(&current[start + open_len + rel_end + actual_close_len..]);
                current = clean;
            } else {
                let mut trace = current[start + open_len..].trim().to_string();
                if trace.starts_with("<thinking>") {
                    trace = trace["<thinking>".len()..].trim_start().to_string();
                } else if trace.starts_with("<thought>") {
                    trace = trace["<thought>".len()..].trim_start().to_string();
                }
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
        "; second, ",
        "; third, ",
        ": second, ",
        ": third, ",
        ", then ",
        ", and finally ",
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
    if history.is_empty() {
        return None;
    }
    let q_lower = current_prompt.to_ascii_lowercase();
    let asks_recall = q_lower.contains("what is my ")
        || q_lower.contains("what's my ")
        || q_lower.contains("who am i")
        || q_lower.contains("what am i ")
        || q_lower.contains("what do i ")
        || q_lower.contains("where do i ")
        || q_lower.contains("remember")
        || q_lower.contains("earlier")
        || q_lower.contains("before")
        || q_lower.contains("previously")
        || q_lower.contains("decide")
        || q_lower.contains("mention")
        || q_lower.contains("what did i ")
        || q_lower.contains("what did we ")
        || q_lower.contains("what were we ")
        || q_lower.contains("talking about")
        || q_lower.contains("talked about")
        || q_lower.contains("first question")
        || q_lower.contains("our conversation");
    let q_stems = extract_salient_stem_hashes_from_text(&q_lower, 12);
    let mut facts = Vec::new();
    for (idx, (u, _)) in history.iter().enumerate() {
        let u_stems = extract_salient_stem_hashes_from_text(&u.to_ascii_lowercase(), 16);
        let shared_stems = q_stems
            .iter()
            .filter(|&&(qh, _)| u_stems.iter().any(|&(uh, _)| uh == qh))
            .count();
        if shared_stems >= 2 && (asks_recall || idx + 2 < history.len()) {
            let item = format!("Turn {}: {}", idx + 1, u.trim().trim_end_matches('.'));
            if !facts.contains(&item) {
                facts.push(item);
            }
        }
        if !asks_recall {
            continue;
        }
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
                || clean_low.starts_with("we decided ")
                || clean_low.starts_with("we chose ")
                || clean_low.starts_with("our ")
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
pub(crate) mod slot_binding_generated;

#[path = "lut_stack_slots.rs"]
mod lut_stack_slots;
pub use lut_stack_slots::*;
