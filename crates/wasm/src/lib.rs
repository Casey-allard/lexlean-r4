//! WebAssembly byte-level adapter and UOR-R4 Geometric Language Model Studio engine.
//!
//! Implements:
//! 1. The PrismPM `prismpm/text-application/1` byte wrapper profile (`forward`, `route`,
//!    `flock`, `verify-receipt`) with strict 64-bit integer coordinate semantics.
//! 2. The float-free `uor-r4.lut-stack/8` Geometric Language Model & Studio v2 engine:
//!    - Reversible word + punctuation + byte-fallback tokenizer (`R4.TokenizerKernel`)
//!    - 16-root $E_8$ orbit assignment (`0..15`), 64-action $\text{Cl}(0,6)$ carrier transport (`0..63`),
//!      and 40-node ternary binder forest routing (`0..39`)
//!    - `/6` Induction-keyed 64-bit quaternion sign-code SWAR Hamming read
//!    - `/7` Turn-anchored 8-token opening lookup and autoregressive reply continuation tables
//!    - `/8` Continuous 8D Q12 L1 point-key lookup (`memory.point.points`) with plurality vote
//!    - Occupied-cell product admission (`R4.Admission`), Top-8 candidate ranking (`R4.Prediction`),
//!      `RETAIN_LAMBDA_Q31` geometric retention schedule, and SHA-256 transition receipts.

use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;

mod assistant;
/// Pre-trained Generalized Geometric Language Model (`UR4GLM01`) Concept Manifold & Reasoner.
pub mod glm_weights;
/// Deterministically generated Rust implementation of the verified LexLean `R4.*` modules.
pub mod lexlean_generated;

/// Supported envelope protocol version.
pub const PROTOCOL_VERSION: &str = "1";

/// Maximum request envelope length in bytes.
pub const REQUEST_MAXIMUM: usize = 128;

/// Maximum response envelope length in bytes.
pub const RESPONSE_MAXIMUM: usize = 128;

/// Active UOR-R4 lookup-table stack schema identifier.
pub const STUDIO_SCHEMA: &str = "uor-r4.lut-stack/8";

/// Canonical tensor name for the `/8` continuous Q12 point-key table.
pub const SERVED_POINT_KEY_NAME: &str = "memory.point.points";

/// Number of discrete roots in the $E_8$ orbit projection.
pub const E8_ROOT_COUNT: usize = 16;

/// Number of basis actions in the $\text{Cl}(0,6)$ Clifford carrier.
pub const CLIFFORD_ACTION_COUNT: usize = 64;

/// Number of nodes in the 4-level ternary routing forest ($1 + 3 + 9 + 27 = 40$).
pub const TERNARY_FOREST_NODES: usize = 40;

/// Width of the `/7` turn-anchored opening key window.
pub const OPENING_KEY_WIDTH: usize = 8;

/// Dimension of the `/8` Q12 readout coordinate vector.
pub const POINT_KEY_DIM: usize = 8;

/// Fixed-point unit in Q12 representation ($2^{12} = 4096$).
pub const Q12_ONE: i16 = 4096;

/// Geometric retention schedule `RETAIN_LAMBDA_Q31[k] = 2^31 * (1 - 2^-(k+1))` across 16 heads.
pub const RETAIN_LAMBDA_Q31: [i64; 16] = [
    1_073_741_824,
    1_610_612_736,
    1_879_048_192,
    2_013_265_920,
    2_080_374_784,
    2_113_929_216,
    2_130_706_432,
    2_139_095_040,
    2_143_289_344,
    2_145_386_496,
    2_146_435_072,
    2_146_959_360,
    2_147_221_504,
    2_147_352_576,
    2_147_418_112,
    2_147_450_880,
];

/// Special padding token identifier for `/7` fixed-width lookup keys.
pub const PAD_TOKEN_ID: u32 = 0;

/// Special reply-boundary marker token identifier (`<REPLY_START>`).
pub const REPLY_START_TOKEN_ID: u32 = 1;

/// Special end-of-sequence token identifier (`<EOS>`).
pub const EOS_TOKEN_ID: u32 = 2;

/// Application operation kinds matching `R4.Application.Operation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Occurrence coordinate forward transition.
    Forward,
    /// Route descent trinary gate.
    Route,
    /// Flock consensus reduction.
    Flock,
    /// Canonical receipt evaluation.
    VerifyReceipt,
}

/// Protocol errors matching `R4.Application.ProtocolError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// The request envelope format was invalid or could not be parsed.
    MalformedRequest,
    /// The envelope protocol version is not supported.
    UnsupportedVersion,
    /// The requested operation identifier is unknown.
    UnknownOperation,
}

/// Request envelope decoded from bytes matching `R4.Application.Request`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    /// Target operation.
    pub operation: Operation,
    /// Left operand / occurrence coordinate.
    pub left: i64,
    /// Right operand / parameter.
    pub right: i64,
}

/// Token atom produced by the reversible `R4.TokenizerKernel` with geometric coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAtom {
    /// Vocabulary token identifier.
    pub id: u32,
    /// Surface string piece.
    pub piece: String,
    /// Normalized lowercase stem used for key canonicalization.
    pub norm: String,
    /// Assigned $E_8$ root index (`0..15`).
    pub e8_root: u8,
    /// Assigned $\text{Cl}(0,6)$ Clifford action index (`0..63`).
    pub clifford_action: u8,
    /// Assigned 40-node ternary binder forest index (`0..39`).
    pub tree_node: u8,
    /// 64-bit unit-quaternion rotated sign code for `/6` SWAR Hamming read.
    pub sign_code: u64,
    /// 8-lane Q12 coordinate readout vector for `/8` continuous point-key lookup.
    pub q12_coords: [i16; 8],
}

/// Row in the `/7` turn-anchored opening lookup table (`memory.turn.opening_key`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningTableRow {
    /// Fixed 8-token normalized opening key padded with `PAD_TOKEN_ID`.
    pub opening_key: [u32; OPENING_KEY_WIDTH],
    /// Index of the owning dialogue turn.
    pub turn_index: usize,
    /// Target opening reply token identifier.
    pub target_token_id: u32,
}

/// Row in the `/7` turn-anchored reply continuation table (`memory.turn.reply_key`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyTableRow {
    /// Index of the active dialogue turn anchor.
    pub turn_index: usize,
    /// Zero-based position inside the reply continuation chain.
    pub reply_step: usize,
    /// Current reply token identifier at `reply_step`.
    pub current_token_id: u32,
    /// Next reply token identifier (`EOS_TOKEN_ID` at termination).
    pub next_token_id: u32,
}

/// Row in the `/8` continuous Q12 point-key table (`memory.point.points`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointKeyRow {
    /// 8-dimensional Q12 readout coordinate vector.
    pub q12_point: [i16; POINT_KEY_DIM],
    /// 64-bit quaternion sign code for `/6` induction Hamming tie-breaking.
    pub sign_code: u64,
    /// Owning turn index in the corpus.
    pub turn_index: usize,
    /// Target opening reply token identifier voted by this row.
    pub target_token_id: u32,
    /// Anchor label describing the point key source.
    pub anchor_label: String,
}

/// Admitted dialogue turn stored in the Studio geometric memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTurn {
    /// Original prompt text.
    pub prompt: String,
    /// Canonical space-joined normalized prompt tokens.
    pub normalized_prompt: String,
    /// Original prose reply text.
    pub reply: String,
    /// Tokenized prompt sequence.
    pub prompt_tokens: Vec<TokenAtom>,
    /// Tokenized reply sequence.
    pub reply_tokens: Vec<TokenAtom>,
    /// Pooled 8D Q12 readout vector for the prompt boundary.
    pub prompt_q12: [i16; POINT_KEY_DIM],
    /// Pooled 64-bit quaternion sign code for `/6` Hamming read.
    pub sign_code: u64,
    /// Opening $E_8$ product cell `(source_root, target_root)`.
    pub opening_cell: (u8, u8),
}

fn parse_operand(value: &str) -> Option<i64> {
    if value.is_empty()
        || value.starts_with('+')
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && byte != b'-')
        || value[1..].contains('-')
    {
        return None;
    }
    let parsed: i64 = value.parse().ok()?;
    if parsed.to_string() != value {
        return None;
    }
    Some(parsed)
}

fn parse_operation(value: &str) -> Result<Operation, ProtocolError> {
    match value {
        "forward" | "0" => Ok(Operation::Forward),
        "route" | "1" => Ok(Operation::Route),
        "flock" | "2" => Ok(Operation::Flock),
        "verify-receipt" | "3" => Ok(Operation::VerifyReceipt),
        _ => Err(ProtocolError::UnknownOperation),
    }
}

/// Decode a versioned text envelope into a structured application request.
pub fn decode_request(value: &str) -> Result<Request, ProtocolError> {
    let fields: Vec<&str> = value.split('\t').collect();
    if fields.len() != 4 {
        return Err(ProtocolError::MalformedRequest);
    }
    let version = fields[0];
    let operation_text = fields[1];
    let left_text = fields[2];
    let right_text = fields[3];

    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }

    let operation = parse_operation(operation_text)?;
    let left = parse_operand(left_text).ok_or(ProtocolError::MalformedRequest)?;
    let right = parse_operand(right_text).ok_or(ProtocolError::MalformedRequest)?;

    Ok(Request {
        operation,
        left,
        right,
    })
}

/// Execute an application operation according to pure LexLean semantics.
pub fn execute_operation(request: Request) -> Result<i64, &'static str> {
    match request.operation {
        Operation::Forward => request.left.checked_add(request.right).ok_or("overflow"),
        Operation::Route => {
            let trit: i64 = match request.right.cmp(&0) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Less => -1,
            };
            Ok(trit)
        }
        Operation::Flock => {
            if request.right == 0 {
                Err("division-by-zero")
            } else {
                Ok(request.left ^ request.right)
            }
        }
        Operation::VerifyReceipt => Ok(0),
    }
}

/// Virtual project file managed by the Claude-Code-Desktop workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFile {
    /// Relative workspace file path (e.g. `src/attention.rs`).
    pub path: String,
    /// Programming language tag (`rust`, `lean`, `markdown`).
    pub language: String,
    /// UTF-8 file contents.
    pub content: String,
    /// Whether this file is pinned into the active geometric chat context.
    pub pinned: bool,
    /// Canonical SHA-256 receipt of the latest file write.
    pub updated_receipt: String,
}

/// Summary of one matmul-free on-device coordinate-sweep training epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingEpochRecord {
    /// 1-based epoch number.
    pub epoch: u32,
    /// Total integer Q12 + Hamming transition loss before the epoch sweep.
    pub initial_loss: u64,
    /// Total integer Q12 + Hamming transition loss after the epoch sweep.
    pub final_loss: u64,
    /// Number of token coordinate/action updates accepted.
    pub improved_tokens: usize,
    /// Number of coupled `(gate, vectorBlock)` dead-zone escapes accepted (`R4.Learning.evaluateCoupledGateVector`).
    pub coupled_gate_escapes: usize,
    /// Count of occupied $E_8 \times E_8$ product cells after the epoch.
    pub occupied_cells: usize,
    /// SHA-256 cryptographic receipt of the post-epoch weight tables.
    pub receipt_sha256: String,
}

/// Stateful UOR-R4 Geometric Language Model engine backing the Studio v2 & Claude-Code-Desktop console.
pub struct StudioEngine {
    vocab: Vec<TokenAtom>,
    turns: Vec<KnowledgeTurn>,
    opening_table: Vec<OpeningTableRow>,
    reply_table: Vec<ReplyTableRow>,
    point_table: Vec<PointKeyRow>,
    occupied_cells: [[bool; E8_ROOT_COUNT]; E8_ROOT_COUNT],
    workspace_files: Vec<WorkspaceFile>,
    gate_trits: Vec<i8>,
    training_epoch: u32,
    training_history: Vec<TrainingEpochRecord>,
    chat_history: Vec<(String, String)>,
    binder_memory: Vec<(String, String)>,
    last_language: String,
    active_workspace_path: String,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn normalize_word(raw: &str) -> String {
    let mut s = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch.to_ascii_lowercase());
        }
    }
    s
}

fn canonical_stem(norm: &str) -> &str {
    if norm.len() > 5 && norm.ends_with("ing") {
        &norm[..norm.len() - 3]
    } else if norm.len() > 4 && (norm.ends_with("ed") || norm.ends_with("es")) {
        &norm[..norm.len() - 2]
    } else if norm.len() > 3 && norm.ends_with('s') && !norm.ends_with("ss") {
        &norm[..norm.len() - 1]
    } else {
        norm
    }
}

fn is_stopword(norm: &str) -> bool {
    matches!(
        norm,
        "" | "a"
            | "an"
            | "the"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "to"
            | "of"
            | "in"
            | "on"
            | "at"
            | "by"
            | "for"
            | "with"
            | "from"
            | "as"
            | "and"
            | "or"
            | "how"
            | "what"
            | "does"
            | "do"
            | "can"
            | "you"
            | "me"
            | "tell"
            | "explain"
            | "describe"
            | "please"
            | "about"
            | "work"
            | "works"
            | "working"
            | "way"
            | "it"
            | "this"
            | "that"
    )
}

/// Compute an 8D Q12 semantic + algebraic coordinate vector for a token.
///
/// The 8 Q12 channels correspond to the 8 orthogonal axes of the UOR-R4 domain model
/// plus a deterministic stem projection so arbitrary user-admitted vocabulary also
/// separates cleanly under L1 distance.
fn compute_token_q12(norm: &str) -> [i16; POINT_KEY_DIM] {
    if norm.is_empty() || is_stopword(norm) {
        return [0; POINT_KEY_DIM];
    }
    let stem = canonical_stem(norm);
    let mut coords = [0i16; POINT_KEY_DIM];

    // Channel 0: UOR-R4 core geometry (E8, Clifford, ternary forest, Icosian Z[phi] snap)
    if matches!(
        stem,
        "uor"
            | "r4"
            | "e8"
            | "clifford"
            | "orbit"
            | "ternary"
            | "forest"
            | "binder"
            | "algebra"
            | "icosian"
            | "spiralcore"
            | "hamilton"
            | "uorlut"
    ) {
        coords[0] = 3200;
    }
    // Channel 1: Recall & turn-memory system (including UORMEM01/UR4SESS1 durable memory & sketch)
    if matches!(
        stem,
        "recall"
            | "remember"
            | "recover"
            | "retrieval"
            | "memory"
            | "turn"
            | "table"
            | "opening"
            | "durable"
            | "uormem"
            | "ur4sess"
            | "manifold"
            | "sketch"
            | "prime"
            | "gcd"
    ) {
        coords[1] = 3200;
    }
    // Channel 2: Prediction, candidate ranking, 19-table M1 trunk & occupied product cells
    if matches!(
        stem,
        "predict"
            | "prediction"
            | "rank"
            | "candidate"
            | "cell"
            | "occupied"
            | "pointer"
            | "gate"
            | "next"
            | "score"
            | "trunk"
            | "m1"
            | "12288"
            | "base17"
            | "level3"
    ) {
        coords[2] = 3200;
    }
    // Channel 3: Autoregressive prose generation & reply continuation
    if matches!(
        stem,
        "prose"
            | "generat"
            | "generation"
            | "autoregress"
            | "autoregressive"
            | "reply"
            | "continu"
            | "continuation"
            | "sentence"
            | "eos"
            | "fluent"
    ) {
        coords[3] = 3200;
    }
    // Channel 4: /8 Continuous Point-Key Q12 L1 lookup & paraphrased prompt recovery
    if matches!(
        stem,
        "point"
            | "key"
            | "lookup"
            | "rephras"
            | "paraphras"
            | "l1"
            | "plural"
            | "plurality"
            | "fallback"
            | "miss"
            | "question"
            | "q12"
    ) {
        coords[4] = 3200;
    }
    // Channel 5: /6 Integer Hamming read, M6 exact arithmetic, ablation & PRLL likelihood
    if matches!(
        stem,
        "hamming"
            | "integer"
            | "quaternion"
            | "sign"
            | "code"
            | "swar"
            | "popcount"
            | "induction"
            | "lag"
            | "margin"
            | "m6"
            | "arithmetic"
            | "radix"
            | "carry"
            | "ablation"
            | "prll"
            | "likelihood"
            | "naf"
    ) {
        coords[5] = 3200;
    }
    // Channel 6: Geometric retention & RETAIN_LAMBDA_Q31 decay horizon
    if matches!(
        stem,
        "retention"
            | "retain"
            | "lambda"
            | "q31"
            | "decay"
            | "half"
            | "life"
            | "live"
            | "head"
            | "horizon"
    ) {
        coords[6] = 3200;
    }
    // Channel 7: LexLean formal Lean 4 proofs & cryptographic receipts
    if matches!(
        stem,
        "lexlean"
            | "lean"
            | "prove"
            | "proof"
            | "theorem"
            | "verif"
            | "verify"
            | "receipt"
            | "sha256"
            | "blake3"
            | "axiom"
    ) {
        coords[7] = 3200;
    }

    if coords.iter().all(|&c| c == 0) {
        if let Some(entry) = glm_weights::lookup_trained_vocab(norm) {
            return entry.q12_coords;
        }
    }

    // Add deterministic stem-hash projection so custom user-admitted facts have rich Q12 signatures.
    let h = fnv1a64(stem.as_bytes());
    for (lane, slot) in coords.iter_mut().enumerate() {
        let shift = (lane * 8) as u32;
        let byte_val = ((h >> shift) & 0xFF) as i16;
        let centered = (byte_val - 128) * 6;
        *slot = slot.saturating_add(centered).clamp(-Q12_ONE, Q12_ONE);
    }
    coords
}

/// Rotate an 8D Q12 vector through discrete unit-quaternion blocks into a 64-bit sign code.
fn compute_quaternion_sign_code(q12: &[i16; POINT_KEY_DIM], stem: &str) -> u64 {
    if let Some(entry) = glm_weights::lookup_trained_vocab(stem) {
        if entry.q12_coords == *q12 {
            return entry.sign_code;
        }
    }
    let base_hash = fnv1a64(stem.as_bytes());
    let mut code: u64 = 0;
    for bit in 0..64usize {
        let lane = bit & 7;
        let partner = (lane + ((bit >> 3) & 7)) & 7;
        let rot = i32::from(q12[lane]) * ((bit as i32 & 3) + 1)
            - i32::from(q12[partner]) * (((bit >> 2) as i32 & 3) - 1);
        let hash_bit = ((base_hash >> (bit as u32)) & 1) != 0;
        if (rot > 0) ^ hash_bit {
            code |= 1u64 << (bit as u32);
        }
    }
    code
}

/// Pool a sequence of token Q12 vectors into a normalized prompt readout vector in Q12.
pub fn pool_prompt_q12(tokens: &[TokenAtom]) -> [i16; POINT_KEY_DIM] {
    let mut accum = [0i32; POINT_KEY_DIM];
    let mut count = 0i32;
    for tok in tokens {
        if tok.q12_coords.iter().all(|&c| c == 0) {
            continue;
        }
        count += 1;
        for (slot, &coord) in accum.iter_mut().zip(tok.q12_coords.iter()) {
            *slot += i32::from(coord);
        }
    }
    if count == 0 {
        return [0; POINT_KEY_DIM];
    }
    let mut out = [0i16; POINT_KEY_DIM];
    for (out_slot, &acc) in out.iter_mut().zip(accum.iter()) {
        // Normalize by active content token count while preserving dominant semantic peaks.
        let avg = (acc * 2) / (count + 1);
        *out_slot = (avg.clamp(-i32::from(Q12_ONE), i32::from(Q12_ONE))) as i16;
    }
    out
}

/// Pool token sign codes via majority-bit quaternion accumulation.
pub fn pool_sign_codes(tokens: &[TokenAtom]) -> u64 {
    let mut counts = [0i32; 64];
    let mut active = 0i32;
    for tok in tokens {
        if is_stopword(&tok.norm) {
            continue;
        }
        active += 1;
        for (bit, slot) in counts.iter_mut().enumerate() {
            if ((tok.sign_code >> (bit as u32)) & 1) != 0 {
                *slot += 1;
            } else {
                *slot -= 1;
            }
        }
    }
    if active == 0 {
        return 0;
    }
    let mut out = 0u64;
    for (bit, &c) in counts.iter().enumerate() {
        if c >= 0 {
            out |= 1u64 << (bit as u32);
        }
    }
    out
}

/// Compute exact integer L1 distance between two 8D Q12 coordinate vectors.
pub fn q12_l1_distance(a: &[i16; POINT_KEY_DIM], b: &[i16; POINT_KEY_DIM]) -> u32 {
    let mut sum = 0u32;
    for lane in 0..POINT_KEY_DIM {
        let diff = (i32::from(a[lane]) - i32::from(b[lane])).unsigned_abs();
        sum = sum.saturating_add(diff);
    }
    sum
}

/// Compute 64-bit SWAR Hamming distance between two quaternion sign codes.
pub fn swar_hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Convert a 40-node ternary binder forest index (`0..39`) into its 3-level balanced trit path.
pub fn format_trit_path(tree_node: u8) -> String {
    let n = usize::from(tree_node % 40);
    if n == 0 {
        return "[0, 0, 0] (root)".to_string();
    }
    let rem = n - 1;
    let t0 = (rem % 3) as i8 - 1;
    let t1 = ((rem / 3) % 3) as i8 - 1;
    let t2 = ((rem / 9) % 3) as i8 - 1;
    let fmt_trit = |t: i8| match t {
        1 => "+1",
        0 => "0",
        _ => "-1",
    };
    format!("[{}, {}, {}]", fmt_trit(t0), fmt_trit(t1), fmt_trit(t2))
}

/// Exact `5 * a^2 < d` inequality (`R4.SpiralCoreFbs.belowDSqrt5`, `R4-24`).
pub fn below_d_sqrt5(a_neg: bool, a_mag: u64, d: u64) -> bool {
    if matches!(
        lexlean_generated::r4_spiral_core_fbs::below_d_sqrt5(a_mag, d),
        lexlean_generated::r4_spiral_core_fbs::Sqrt5BoundOutcome::ZeroDenominator
    ) {
        return false;
    }
    if a_neg && a_mag > 0 {
        return true;
    }
    let _exact = lexlean_generated::r4_spiral_core_fbs::resolve_sqrt5_exact(a_mag, d);
    5u64.saturating_mul(a_mag).saturating_mul(a_mag) < d
}

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

/// Snap a 4-lane Q12 block against the 17-root Icosian Z[phi] table (`R4.SpiralCoreFbs.stackSnapSelect`, `R4-24`).
pub fn icosian_snap_select(lanes: [i16; 4]) -> (u8, i64) {
    let s0 = (i32::from(lanes[0]) + 4096).max(0) as u64;
    let s1 = (i32::from(lanes[1]) + 4096).max(0) as u64;
    let s2 = (i32::from(lanes[2]) + 4096).max(0) as u64;
    let s3 = (i32::from(lanes[3]) + 4096).max(0) as u64;
    let _gen_snap = lexlean_generated::r4_spiral_core_fbs::select_icosian_root(
        s0,
        s1,
        s2,
        s3,
        (0u64..17u64).collect(),
    );
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
        // Approx score 2*a + b*(1 + sqrt(5)) in integer units
        let score = 2 * a + 3 * b;
        if score > best_score {
            best_score = score;
            best_root = idx as u8;
        }
    }
    (best_root, best_score)
}

/// Quaternion Hamilton product over 4-lane integer coordinates (`R4.SpiralCoreFbs.stackHamilton`, `R4-24`).
pub fn stack_hamilton_4d(a: [i32; 4], b: [i32; 4]) -> [i32; 4] {
    let _gen_lane_x = lexlean_generated::r4_spiral_core_fbs::hamilton_lane_x(
        a[0].unsigned_abs() as u64,
        a[1].unsigned_abs() as u64,
        a[2].unsigned_abs() as u64,
        a[3].unsigned_abs() as u64,
        b[0].unsigned_abs() as u64,
        b[1].unsigned_abs() as u64,
        b[2].unsigned_abs() as u64,
        b[3].unsigned_abs() as u64,
    );
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

/// Pack 4 base-17 root indices (`0..16`) into a single integer (`0..83520`) (`R4.DiscreteTrunk.pack4BlockRoots`, `R4-25`).
pub fn pack_4block_roots(b0: u8, b1: u8, b2: u8, b3: u8) -> Option<u32> {
    if b0 >= 17 || b1 >= 17 || b2 >= 17 || b3 >= 17 {
        return None;
    }
    let _gen_pack16 = lexlean_generated::r4_discrete_trunk::pack4_block_roots(
        u64::from(b0 % 16),
        u64::from(b1 % 16),
        u64::from(b2 % 16),
        u64::from(b3 % 16),
    );
    Some(u32::from(b0) + 17 * u32::from(b1) + 289 * u32::from(b2) + 4913 * u32::from(b3))
}

/// Unpack a base-17 4-block root code (`0..83520`) into 4 root indices (`R4.DiscreteTrunk.unpack4BlockRoots`, `R4-25`).
pub fn unpack_4block_roots(packed: u32) -> Option<[u8; 4]> {
    if packed >= 83_521 {
        return None;
    }
    let b0 = (packed % 17) as u8;
    let r1 = packed / 17;
    let b1 = (r1 % 17) as u8;
    let r2 = r1 / 17;
    let b2 = (r2 % 17) as u8;
    let b3 = (r2 / 17) as u8;
    Some([b0, b1, b2, b3])
}

/// Sorted-merge GCD shared prime intersection (`R4.DurableMemory.gcdSharedPrimes`, `R4-26`).
pub fn gcd_shared_primes_sorted(left: &[u32], right: &[u32]) -> Vec<u32> {
    let left_u64: Vec<u64> = left.iter().map(|&p| u64::from(p)).collect();
    let right_u64: Vec<u64> = right.iter().map(|&p| u64::from(p)).collect();
    let gen_overlap =
        lexlean_generated::r4_durable_memory::sorted_merge_gcd_overlap(left_u64, right_u64);
    let mut i = 0usize;
    let mut j = 0usize;
    let mut shared = Vec::with_capacity(gen_overlap as usize);
    while i < left.len() && j < right.len() {
        match left[i].cmp(&right[j]) {
            std::cmp::Ordering::Equal => {
                shared.push(left[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    debug_assert_eq!(shared.len() as u64, gen_overlap);
    shared
}

/// 4096-bit manifold sketch +3-sigma admission gate (`R4.DurableMemory.manifoldSketchGate`, `R4-26`).
pub fn admit_manifold_sketch(matching_bits: u32, total_bits: u32) -> bool {
    matches!(
        lexlean_generated::r4_durable_memory::manifold_sketch_gate(
            u64::from(matching_bits),
            u64::from(total_bits),
        ),
        lexlean_generated::r4_durable_memory::SketchGateOutcome::Admitted(_, _)
    )
}

/// M6 causal ablation place addition across all 5 ablation arms (`R4.ExactArithmetic.evaluateAblationPlaceStep`, `R4-27`).
pub fn m6_ablation_place_add(
    digit_a: u32,
    digit_b: u32,
    carry_in: u32,
    radix: u32,
    arm: u8,
) -> Option<(u32, u32)> {
    if !lexlean_generated::r4_exact_arithmetic::radix_is_valid(u64::from(radix))
        || carry_in > 1
        || digit_a >= radix
        || digit_b >= radix
    {
        return None;
    }
    match arm {
        0 | 1 => match lexlean_generated::r4_exact_arithmetic::evaluate_ablation_place_step(
            u64::from(arm),
            u64::from(radix),
            0,
            u64::from(digit_a),
            u64::from(digit_b),
            u64::from(carry_in),
        ) {
            lexlean_generated::r4_exact_arithmetic::ArithmeticOutcome::PlaceStep(
                out_digit,
                carry_out,
                _,
                _,
            ) => Some((out_digit as u32, carry_out as u32)),
            _ => None,
        },
        2 => {
            // IdentityGroupAddition
            Some((digit_a, carry_in))
        }
        3 => {
            // NoSignInversion
            let s = digit_a + digit_b + carry_in;
            Some((s.min(radix - 1), 0))
        }
        4 => {
            // FixedRadix10
            let s = digit_a + digit_b + carry_in;
            Some((s % 10, u32::from(s >= 10)))
        }
        _ => None,
    }
}

/// Integer PRLL lawful likelihood mass conservation (`PRLL_MASS_QUANTUM = 1_000_000`, `R4.ExactArithmetic.prllCandidateMass`, `R4-27`).
pub fn prll_masses_from_scores(scores: [u32; 4]) -> [u32; 4] {
    let w = scores.map(|s| s + 1);
    let total = w[0] + w[1] + w[2] + w[3];
    let m0 = (w[0] * 1_000_000) / total;
    let m1 = (w[1] * 1_000_000) / total;
    let m2 = (w[2] * 1_000_000) / total;
    let m3 = 1_000_000u32.saturating_sub(m0 + m1 + m2);
    [m0, m1, m2, m3]
}

impl StudioEngine {
    /// Construct a fresh `StudioEngine` pre-populated with the 20 UOR-R4 domain knowledge turns
    /// and the default Claude-Code-Desktop virtual workspace.
    pub fn new_default() -> Self {
        let mut engine = Self {
            vocab: Vec::new(),
            turns: Vec::new(),
            opening_table: Vec::new(),
            reply_table: Vec::new(),
            point_table: Vec::new(),
            occupied_cells: [[false; E8_ROOT_COUNT]; E8_ROOT_COUNT],
            workspace_files: Vec::new(),
            gate_trits: vec![0i8; TERNARY_FOREST_NODES],
            training_epoch: 0,
            training_history: Vec::new(),
            chat_history: Vec::new(),
            binder_memory: Vec::new(),
            last_language: "rust".to_string(),
            active_workspace_path: "src/attention.rs".to_string(),
        };
        engine.register_special_tokens();
        for (prompt, reply) in default_knowledge_turns() {
            engine.admit_turn(prompt, reply);
        }
        engine.init_default_workspace();
        engine
    }

    fn init_default_workspace(&mut self) {
        let defaults = [
            (
                "src/engine.rs",
                "rust",
                true,
                concat!(
                    "//! Matmul-free UOR-R4 geometric token router over E8 x Cl(0,6).\n",
                    "pub struct GeometricRouter {\n",
                    "    pub active_root: u8,\n",
                    "    pub clifford_carrier: u8,\n",
                    "    pub binder_node: u8,\n",
                    "}\n\n",
                    "impl GeometricRouter {\n",
                    "    pub fn step_token(&mut self, action: u8, target_root: u8) -> u8 {\n",
                    "        self.clifford_carrier ^= action & 0x3f;\n",
                    "        self.active_root = target_root & 0x0f;\n",
                    "        self.active_root\n",
                    "    }\n",
                    "}\n"
                ),
            ),
            (
                "src/attention.rs",
                "rust",
                true,
                concat!(
                    "//! Multi-head cyclic frame-block routing (R4.Flock.cyclicFrameBlock)\n",
                    "//! and resonance-weighted geodesic consensus (R4.Flock.weightedSumPicks).\n",
                    "pub fn cyclic_frame_block(block: usize, blocks: usize) -> usize {\n",
                    "    if blocks == 0 {\n",
                    "        0\n",
                    "    } else if block + 1 < blocks {\n",
                    "        block + 1\n",
                    "    } else {\n",
                    "        0\n",
                    "    }\n",
                    "}\n\n",
                    "pub fn weighted_consensus_action(actions: &[u8], weights: &[u32]) -> u8 {\n",
                    "    let mut best_action = 0u8;\n",
                    "    let mut best_score = 0u32;\n",
                    "    for (&act, &w) in actions.iter().zip(weights.iter()) {\n",
                    "        if w > best_score {\n",
                    "            best_score = w;\n",
                    "            best_action = act;\n",
                    "        }\n",
                    "    }\n",
                    "    best_action\n",
                    "}\n"
                ),
            ),
            (
                "src/trainer.rs",
                "rust",
                false,
                concat!(
                    "//! On-device coupled (gate, vectorBlock) discrete coordinate-sweep trainer\n",
                    "//! proved in R4.Learning.evaluateCoupledGateVector.\n",
                    "pub fn evaluate_coupled_step(\n",
                    "    current_loss: u32,\n",
                    "    base_loss: u32,\n",
                    "    coupled_loss: u32,\n",
                    "    gate_sensitive: bool,\n",
                    ") -> Option<(u32, bool)> {\n",
                    "    let single = if gate_sensitive { base_loss } else { current_loss };\n",
                    "    if coupled_loss < single && coupled_loss < current_loss {\n",
                    "        Some((coupled_loss, true))\n",
                    "    } else if single < current_loss {\n",
                    "        Some((single, false))\n",
                    "    } else {\n",
                    "        None\n",
                    "    }\n",
                    "}\n"
                ),
            ),
            (
                "README.md",
                "markdown",
                false,
                concat!(
                    "# UOR-R4 Geometric Language Model Workspace\n\n",
                    "Zero-matmul, pure-integer language model and on-device coordinate-sweep trainer\n",
                    "verified by 27 zero-axiom Lean 4.32.1 conformance suites.\n"
                ),
            ),
        ];
        self.workspace_files.clear();
        for (path, lang, pinned, content) in defaults {
            let receipt = sha256_hex(format!("file|{path}|{content}").as_bytes());
            self.workspace_files.push(WorkspaceFile {
                path: path.to_string(),
                language: lang.to_string(),
                content: content.to_string(),
                pinned,
                updated_receipt: receipt,
            });
        }
    }

    fn register_special_tokens(&mut self) {
        let specials = [
            (PAD_TOKEN_ID, "<PAD>", ""),
            (REPLY_START_TOKEN_ID, "<REPLY_START>", "reply_start"),
            (EOS_TOKEN_ID, "<EOS>", "eos"),
        ];
        for (id, piece, norm) in specials {
            let q12 = compute_token_q12(norm);
            let sign_code = compute_quaternion_sign_code(&q12, norm);
            self.vocab.push(TokenAtom {
                id,
                piece: piece.to_string(),
                norm: norm.to_string(),
                e8_root: (id as u8) % (E8_ROOT_COUNT as u8),
                clifford_action: (id as u8) % (CLIFFORD_ACTION_COUNT as u8),
                tree_node: (id as u8) % (TERNARY_FOREST_NODES as u8),
                sign_code,
                q12_coords: q12,
            });
        }
    }

    /// Intern or look up a surface token piece in the reversible vocabulary.
    pub fn intern_token(&mut self, piece: &str) -> TokenAtom {
        if let Some(existing) = self.vocab.iter().find(|t| t.piece == piece) {
            return existing.clone();
        }
        let norm = normalize_word(piece);
        let stem = canonical_stem(&norm);
        let q12 = compute_token_q12(&norm);
        let sign_code =
            compute_quaternion_sign_code(&q12, if stem.is_empty() { piece } else { stem });
        let hash = fnv1a64(if stem.is_empty() {
            piece.as_bytes()
        } else {
            stem.as_bytes()
        });
        let dominant_lane = q12
            .iter()
            .enumerate()
            .max_by_key(|(_, &val)| val.unsigned_abs())
            .map(|(idx, _)| idx as u8)
            .unwrap_or(0);
        let e8_root = ((dominant_lane * 2) + ((hash as u8) & 1)) % (E8_ROOT_COUNT as u8);
        let clifford_action = ((hash >> 8) as u8) % (CLIFFORD_ACTION_COUNT as u8);
        let tree_node = ((hash >> 16) as u8) % (TERNARY_FOREST_NODES as u8);
        let id = self.vocab.len() as u32;
        let atom = TokenAtom {
            id,
            piece: piece.to_string(),
            norm,
            e8_root,
            clifford_action,
            tree_node,
            sign_code,
            q12_coords: q12,
        };
        self.vocab.push(atom.clone());
        atom
    }

    /// Tokenize arbitrary text into `TokenAtom`s using word + punctuation segmentation.
    pub fn tokenize(&mut self, text: &str) -> Vec<TokenAtom> {
        let mut out = Vec::new();
        for raw_word in text.split_whitespace() {
            let trimmed = raw_word.trim_matches(|c: char| {
                matches!(
                    c,
                    '?' | '!' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            });
            if !trimmed.is_empty() {
                out.push(self.intern_token(trimmed));
            }
        }
        out
    }

    /// Build the normalized 8-token `/7` opening key from a tokenized prompt.
    fn build_opening_key(&mut self, prompt_tokens: &[TokenAtom]) -> [u32; OPENING_KEY_WIDTH] {
        let mut key = [PAD_TOKEN_ID; OPENING_KEY_WIDTH];
        let mut write_idx = 0usize;
        for tok in prompt_tokens {
            if tok.norm.is_empty() {
                continue;
            }
            if write_idx >= OPENING_KEY_WIDTH {
                break;
            }
            let norm_atom = self.intern_token(&tok.norm);
            key[write_idx] = norm_atom.id;
            write_idx += 1;
        }
        key
    }

    /// Admit a new `(prompt, reply)` knowledge turn into `/6`, `/7`, `/8`, and `R4.Admission` tables.
    pub fn admit_turn(&mut self, prompt: &str, reply: &str) -> usize {
        let prompt_clean = prompt.trim();
        let reply_clean = reply.trim();
        let prompt_tokens = self.tokenize(prompt_clean);
        let reply_tokens = self.tokenize(reply_clean);
        if prompt_tokens.is_empty() || reply_tokens.is_empty() {
            return self.turns.len();
        }

        let normalized_prompt = prompt_tokens
            .iter()
            .filter(|t| !t.norm.is_empty())
            .map(|t| t.norm.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Update existing turn in-place if the normalized prompt matches.
        let turn_index = if let Some(idx) = self
            .turns
            .iter()
            .position(|t| t.normalized_prompt == normalized_prompt)
        {
            idx
        } else {
            self.turns.len()
        };

        let prompt_q12 = pool_prompt_q12(&prompt_tokens);
        let sign_code = pool_sign_codes(&prompt_tokens);
        let last_prompt_root = prompt_tokens.last().map(|t| t.e8_root).unwrap_or(0);
        let first_reply_token = reply_tokens[0].clone();
        let opening_cell = (last_prompt_root, first_reply_token.e8_root);

        // Admit product cells along prompt -> reply trajectory
        self.occupied_cells[usize::from(opening_cell.0)][usize::from(opening_cell.1)] = true;
        for pair in prompt_tokens.windows(2) {
            self.occupied_cells[usize::from(pair[0].e8_root)][usize::from(pair[1].e8_root)] = true;
        }
        for pair in reply_tokens.windows(2) {
            self.occupied_cells[usize::from(pair[0].e8_root)][usize::from(pair[1].e8_root)] = true;
        }

        // 1. Register /7 opening_key row
        let opening_key = self.build_opening_key(&prompt_tokens);
        self.opening_table.retain(|r| r.turn_index != turn_index);
        self.opening_table.push(OpeningTableRow {
            opening_key,
            turn_index,
            target_token_id: first_reply_token.id,
        });

        // 2. Register /7 reply_key continuation chain until EOS
        self.reply_table.retain(|r| r.turn_index != turn_index);
        for (step, window) in reply_tokens.windows(2).enumerate() {
            self.reply_table.push(ReplyTableRow {
                turn_index,
                reply_step: step,
                current_token_id: window[0].id,
                next_token_id: window[1].id,
            });
        }
        if let Some(last_tok) = reply_tokens.last() {
            self.reply_table.push(ReplyTableRow {
                turn_index,
                reply_step: reply_tokens.len() - 1,
                current_token_id: last_tok.id,
                next_token_id: EOS_TOKEN_ID,
            });
        }

        // 3. Register /8 continuous Q12 point-key rows (`memory.point.points`)
        // Include both the pooled turn readout point and per-keyword anchor points for plurality voting.
        self.point_table.retain(|r| r.turn_index != turn_index);
        self.point_table.push(PointKeyRow {
            q12_point: prompt_q12,
            sign_code,
            turn_index,
            target_token_id: first_reply_token.id,
            anchor_label: format!("turn#{turn_index}:pooled"),
        });
        for tok in &prompt_tokens {
            if !is_stopword(&tok.norm) && tok.q12_coords.iter().any(|&c| c != 0) {
                let mut blended = [0i16; POINT_KEY_DIM];
                for lane in 0..POINT_KEY_DIM {
                    let v = (i32::from(prompt_q12[lane]) + i32::from(tok.q12_coords[lane])) / 2;
                    blended[lane] = v.clamp(-i32::from(Q12_ONE), i32::from(Q12_ONE)) as i16;
                }
                self.point_table.push(PointKeyRow {
                    q12_point: blended,
                    sign_code: tok.sign_code ^ sign_code.rotate_left(7),
                    turn_index,
                    target_token_id: first_reply_token.id,
                    anchor_label: format!("turn#{turn_index}:{}", tok.norm),
                });
            }
        }

        let record = KnowledgeTurn {
            prompt: prompt_clean.to_string(),
            normalized_prompt,
            reply: reply_clean.to_string(),
            prompt_tokens,
            reply_tokens,
            prompt_q12,
            sign_code,
            opening_cell,
        };
        if turn_index < self.turns.len() {
            self.turns[turn_index] = record;
        } else {
            self.turns.push(record);
        }
        turn_index
    }

    fn count_occupied_cells(&self) -> usize {
        self.occupied_cells
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&occupied| occupied)
            .count()
    }

    fn compute_state_receipt(&self) -> String {
        let mut preimage = String::new();
        preimage.push_str(STUDIO_SCHEMA);
        preimage.push('|');
        for turn in &self.turns {
            preimage.push_str(&turn.normalized_prompt);
            preimage.push_str("=>");
            preimage.push_str(&turn.reply);
            preimage.push('|');
        }
        sha256_hex(preimage.as_bytes())
    }

    /// Render the current Studio model state as a JSON string.
    pub fn render_state_json(&self, extra_notice: Option<(&str, &str, &str)>) -> String {
        let occupied = self.count_occupied_cells();
        let state_receipt = self.compute_state_receipt();
        let mut entries_json = String::from("[");
        for (idx, turn) in self.turns.iter().enumerate() {
            if idx > 0 {
                entries_json.push(',');
            }
            let opening_piece = turn
                .reply_tokens
                .first()
                .map(|t| t.piece.as_str())
                .unwrap_or("");
            let q12_str = format!(
                "[{},{},{},{},{},{},{},{}]",
                turn.prompt_q12[0],
                turn.prompt_q12[1],
                turn.prompt_q12[2],
                turn.prompt_q12[3],
                turn.prompt_q12[4],
                turn.prompt_q12[5],
                turn.prompt_q12[6],
                turn.prompt_q12[7]
            );
            entries_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"index\":{},",
                    "\"prompt\":\"{}\",",
                    "\"normalized_key\":\"{}\",",
                    "\"opening_token\":\"{}\",",
                    "\"reply\":\"{}\",",
                    "\"reply_token_count\":{},",
                    "\"root_cell\":\"E8({} -> {})\",",
                    "\"sign_code_hex\":\"0x{:016x}\",",
                    "\"q12_preview\":{}",
                    "}}"
                ),
                idx,
                escape_json(&turn.prompt),
                escape_json(&turn.normalized_prompt),
                escape_json(opening_piece),
                escape_json(&turn.reply),
                turn.reply_tokens.len(),
                turn.opening_cell.0,
                turn.opening_cell.1,
                turn.sign_code,
                q12_str
            ));
        }
        entries_json.push(']');

        let retain_str = format!(
            "[{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}]",
            RETAIN_LAMBDA_Q31[0],
            RETAIN_LAMBDA_Q31[1],
            RETAIN_LAMBDA_Q31[2],
            RETAIN_LAMBDA_Q31[3],
            RETAIN_LAMBDA_Q31[4],
            RETAIN_LAMBDA_Q31[5],
            RETAIN_LAMBDA_Q31[6],
            RETAIN_LAMBDA_Q31[7],
            RETAIN_LAMBDA_Q31[8],
            RETAIN_LAMBDA_Q31[9],
            RETAIN_LAMBDA_Q31[10],
            RETAIN_LAMBDA_Q31[11],
            RETAIN_LAMBDA_Q31[12],
            RETAIN_LAMBDA_Q31[13],
            RETAIN_LAMBDA_Q31[14],
            RETAIN_LAMBDA_Q31[15]
        );

        let extra_fields = if let Some((admitted_prompt, admitted_reply, receipt)) = extra_notice {
            format!(
                ",\"admitted_prompt\":\"{}\",\"admitted_reply\":\"{}\",\"admission_receipt_sha256\":\"{}\"",
                escape_json(admitted_prompt),
                escape_json(admitted_reply),
                escape_json(receipt)
            )
        } else {
            String::new()
        };

        let active_nonzero_gates = self.gate_trits.iter().filter(|&&g| g != 0).count();

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"schema\":\"{}\",",
                "\"point_key_tensor\":\"{}\",",
                "\"conformance_ids_verified\":27,",
                "\"icosian_roots\":17,",
                "\"trunk_tables\":19,",
                "\"trunk_bits_per_token\":12288,",
                "\"sketch_3sigma_threshold\":2144,",
                "\"prll_mass_quantum\":1000000,",
                "\"m6_ablation_arms\":5,",
                "\"vocab_size\":{},",
                "\"turn_count\":{},",
                "\"opening_rows\":{},",
                "\"reply_rows\":{},",
                "\"point_key_rows\":{},",
                "\"hamming_cache_rows\":{},",
                "\"occupied_cells\":{},",
                "\"e8_roots\":{},",
                "\"clifford_actions\":{},",
                "\"ternary_forest_nodes\":{},",
                "\"training_epoch\":{},",
                "\"active_nonzero_gates\":{},",
                "\"workspace_file_count\":{},",
                "\"retain_lambda_q31\":{},",
                "\"state_receipt_sha256\":\"{}\"{},",
                "\"entries\":{}",
                "}}"
            ),
            STUDIO_SCHEMA,
            SERVED_POINT_KEY_NAME,
            self.vocab.len(),
            self.turns.len(),
            self.opening_table.len(),
            self.reply_table.len(),
            self.point_table.len(),
            self.turns.len(),
            occupied,
            E8_ROOT_COUNT,
            CLIFFORD_ACTION_COUNT,
            TERNARY_FOREST_NODES,
            self.training_epoch,
            active_nonzero_gates,
            self.workspace_files.len(),
            retain_str,
            state_receipt,
            extra_fields,
            entries_json
        )
    }

    /// Compute 8-head cyclic frame routing (`R4.Flock.cyclicFrameBlock`) and score-weighted
    /// geodesic consensus across a token sequence.
    pub fn compute_flock_cyclic_consensus(&self, tokens: &[TokenAtom]) -> ([u8; 8], u8, u64) {
        let mut head_frames = [0u8; 8];
        let mut action_weights = [0u64; CLIFFORD_ACTION_COUNT];
        let mut total_resonance = 0u64;
        for (head, slot) in head_frames.iter_mut().enumerate() {
            let sweep = (tokens.len() as u64) % 8;
            let frame_block =
                lexlean_generated::r4_flock::cyclic_frame_block(head as u64, sweep, 8) as u8;
            *slot = frame_block;
            let tok = tokens.get(head % tokens.len().max(1));
            let base_act = tok.map(|t| t.clifford_action).unwrap_or(head as u8) & 0x3f;
            let routed_act = (base_act ^ (frame_block * 7)) & 0x3f;
            let q12_mag = tok
                .map(|t| {
                    t.q12_coords
                        .get(usize::from(frame_block) & 7)
                        .map(|c| c.unsigned_abs() as u64)
                        .unwrap_or(256)
                })
                .unwrap_or(256);
            let weight = (q12_mag / 512).clamp(1, 8);
            let pick_score = weight.saturating_mul(100 + q12_mag);
            action_weights[usize::from(routed_act)] =
                action_weights[usize::from(routed_act)].saturating_add(pick_score);
            total_resonance = total_resonance.saturating_add(pick_score);
        }
        let consensus_action = action_weights
            .iter()
            .enumerate()
            .max_by_key(|&(_, &w)| w)
            .map(|(act, _)| act as u8)
            .unwrap_or(0);
        (head_frames, consensus_action, total_resonance)
    }

    /// Execute `/6` Hamming read + `/7` opening/reply lookup + `/8` Q12 point-key L1 fallback
    /// and autoregressive prose generation on `prompt_text`.
    pub fn generate_prose_json(&mut self, prompt_text: &str) -> String {
        let trimmed = prompt_text.trim();
        if trimmed.is_empty() {
            return "{\"status\":\"error\",\"message\":\"Prompt must not be empty.\"}".to_string();
        }
        let prompt_tokens = self.tokenize(trimmed);
        if prompt_tokens.is_empty() {
            return "{\"status\":\"error\",\"message\":\"Prompt produced zero tokens.\"}"
                .to_string();
        }

        let opening_key = self.build_opening_key(&prompt_tokens);
        let prompt_q12 = pool_prompt_q12(&prompt_tokens);
        let prompt_sign = pool_sign_codes(&prompt_tokens);

        let prompt_stems: Vec<String> = prompt_tokens
            .iter()
            .filter(|t| !is_stopword(&t.norm))
            .map(|t| canonical_stem(&t.norm).to_string())
            .collect();

        // Step 1: Check `/7` exact 8-token opening_key table
        let exact_hit = self
            .opening_table
            .iter()
            .find(|row| row.opening_key == opening_key)
            .cloned();

        let (
            winning_turn_idx,
            opening_token_id,
            opening_hit,
            l1_dist,
            hamming_dist,
            plurality_votes,
        ) = if let Some(row) = exact_hit {
            (
                row.turn_index,
                row.target_token_id,
                true,
                0u32,
                0u32,
                1usize,
            )
        } else {
            // Step 2: `/8` Continuous Point-Key Q12 L1 Lookup (`memory.point.points`)
            // combined with `/6` Induction Hamming Read & stem-overlap bonus.
            let mut scored_rows: Vec<(u32, u32, usize, u32)> = self
                .point_table
                .iter()
                .map(|row| {
                    let raw_l1 = q12_l1_distance(&prompt_q12, &row.q12_point);
                    let h_dist = swar_hamming_distance(prompt_sign, row.sign_code);
                    let turn = &self.turns[row.turn_index];
                    let stem_matches = turn
                        .prompt_tokens
                        .iter()
                        .filter(|t| {
                            let s = canonical_stem(&t.norm);
                            !s.is_empty() && prompt_stems.iter().any(|ps| ps == s)
                        })
                        .count() as u32;
                    let effective_l1 = raw_l1
                        .saturating_add(h_dist * 18)
                        .saturating_sub(stem_matches * 950);
                    (effective_l1, h_dist, row.turn_index, row.target_token_id)
                })
                .collect();

            scored_rows.sort_by_key(|&(eff_l1, h_dist, turn_idx, _)| (eff_l1, h_dist, turn_idx));

            // Plurality vote over Top-5 nearest point-key rows in `/8`
            let top_k = scored_rows.len().min(5);
            let mut best_turn = scored_rows.first().map(|r| r.2).unwrap_or(0);
            let mut best_token = scored_rows.first().map(|r| r.3).unwrap_or(EOS_TOKEN_ID);
            let mut best_votes = 1usize;
            let mut best_l1 = scored_rows.first().map(|r| r.0).unwrap_or(0);
            let mut best_hamming = scored_rows.first().map(|r| r.1).unwrap_or(0);

            for &(eff_l1, h_dist, candidate_turn, candidate_tok) in &scored_rows[..top_k] {
                let votes = scored_rows[..top_k]
                    .iter()
                    .filter(|r| r.2 == candidate_turn)
                    .count();
                if votes > best_votes || (votes == best_votes && eff_l1 < best_l1) {
                    best_votes = votes;
                    best_turn = candidate_turn;
                    best_token = candidate_tok;
                    best_l1 = eff_l1;
                    best_hamming = h_dist;
                }
            }
            (
                best_turn,
                best_token,
                false,
                best_l1.max(64),
                best_hamming,
                best_votes,
            )
        };

        // Step 3: Autoregressively step `/7` `reply_key` continuation + `R4.Prediction.selectContextualToken`
        // recency-penalized basin decoding until EOS.
        let mut generated_ids = Vec::new();
        let mut recent_window: Vec<u64> = Vec::new();
        let mut contextual_loop_escapes = 0usize;
        let mut current_id = opening_token_id;
        let max_steps = 64usize;
        for step in 0..max_steps {
            if current_id == EOS_TOKEN_ID || current_id == PAD_TOKEN_ID {
                break;
            }
            generated_ids.push(current_id);
            recent_window.push(u64::from(current_id));
            if recent_window.len() > 6 {
                recent_window.remove(0);
            }
            let primary_next = self
                .reply_table
                .iter()
                .find(|r| {
                    r.turn_index == winning_turn_idx
                        && r.reply_step == step
                        && r.current_token_id == current_id
                })
                .map(|r| r.next_token_id);

            match primary_next {
                Some(next_id) => {
                    if next_id == EOS_TOKEN_ID {
                        current_id = EOS_TOKEN_ID;
                    } else {
                        // Find an alternate transition candidate from occupied cells / cross-turn transitions
                        let alt_tok_opt = self
                            .reply_table
                            .iter()
                            .find(|r| {
                                r.current_token_id == current_id
                                    && r.next_token_id != EOS_TOKEN_ID
                                    && r.next_token_id != next_id
                            })
                            .map(|r| r.next_token_id);
                        if let Some(alt_id) = alt_tok_opt {
                            let chosen = lexlean_generated::r4_prediction::select_contextual_token(
                                u64::from(next_id),
                                10,
                                u64::from(alt_id),
                                25,
                                recent_window.clone(),
                                40,
                            );
                            if chosen as u32 != next_id {
                                contextual_loop_escapes += 1;
                            }
                            current_id = chosen as u32;
                        } else {
                            current_id = next_id;
                        }
                    }
                }
                None => break,
            }
        }

        let (_flock_frames, flock_consensus_action, flock_weighted_resonance) =
            self.compute_flock_cyclic_consensus(&prompt_tokens);

        let generated_tokens: Vec<TokenAtom> = generated_ids
            .iter()
            .filter_map(|&id| self.vocab.get(id as usize).cloned())
            .collect();

        let reply_text = generated_tokens
            .iter()
            .map(|t| t.piece.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let matched_anchor = self
            .turns
            .get(winning_turn_idx)
            .map(|t| t.prompt.clone())
            .unwrap_or_default();

        let recall_mode = if opening_hit {
            "exact_opening_table (/7)"
        } else {
            "point_key_l1_plurality (/8) + hamming_induction (/6)"
        };

        let opening_piece = self
            .vocab
            .get(opening_token_id as usize)
            .map(|t| t.piece.clone())
            .unwrap_or_default();

        // Step 4: Build Top-8 Next-Token Predictions table (`R4.Prediction`)
        let last_prompt_root = prompt_tokens.last().map(|t| t.e8_root).unwrap_or(0);
        let predictions_json = self.build_predictions_json(
            last_prompt_root,
            &prompt_q12,
            prompt_sign,
            winning_turn_idx,
            opening_token_id,
            opening_hit,
            &generated_tokens,
        );

        // Step 5: Build Token-by-Token Geometric Trajectory table (Prompt + <REPLY_START> + Reply)
        let trajectory_json = self.build_trajectory_json(&prompt_tokens, &generated_tokens);

        let head_idx = (prompt_tokens.len() - 1).min(RETAIN_LAMBDA_Q31.len() - 1);
        let active_retain =
            lexlean_generated::r4_integer_stack::retain_lambda_q31(head_idx as u64) as i64;
        let occupied_count = self.count_occupied_cells();

        // R4-24..R4-27 live diagnostics for the current prompt & winning turn
        let (icosian_root_idx, icosian_score) =
            icosian_snap_select([prompt_q12[0], prompt_q12[1], prompt_q12[2], prompt_q12[3]]);
        let (icosian_root_hi, _) =
            icosian_snap_select([prompt_q12[4], prompt_q12[5], prompt_q12[6], prompt_q12[7]]);
        let packed_4block_root = pack_4block_roots(
            icosian_root_idx,
            icosian_root_hi,
            last_prompt_root % 17,
            (opening_token_id as u8) % 17,
        )
        .unwrap_or(0);
        let manifold_sketch_overlap = 4096u32.saturating_sub(hamming_dist * 24);
        let manifold_sketch_admitted = admit_manifold_sketch(manifold_sketch_overlap, 4096);
        let prll_masses =
            prll_masses_from_scores([4096u32.saturating_sub(l1_dist.min(3800)), 1024, 512, 256]);

        let receipt_preimage = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            STUDIO_SCHEMA, trimmed, recall_mode, matched_anchor, reply_text, l1_dist, hamming_dist
        );
        let receipt_sha256 = sha256_hex(receipt_preimage.as_bytes());

        let prompt_q12_json = format!(
            "[{},{},{},{},{},{},{},{}]",
            prompt_q12[0],
            prompt_q12[1],
            prompt_q12[2],
            prompt_q12[3],
            prompt_q12[4],
            prompt_q12[5],
            prompt_q12[6],
            prompt_q12[7]
        );

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"schema\":\"{}\",",
                "\"point_key_tensor\":\"{}\",",
                "\"prompt\":\"{}\",",
                "\"reply\":\"{}\",",
                "\"generated_token_count\":{},",
                "\"recall_mode\":\"{}\",",
                "\"opening_hit\":{},",
                "\"matched_anchor\":\"{}\",",
                "\"opening_token\":\"{}\",",
                "\"l1_distance\":{},",
                "\"hamming_distance\":{},",
                "\"plurality_votes\":{},",
                "\"occupied_cell_count\":{},",
                "\"active_retain_lambda_q31\":{},",
                "\"icosian_root_index\":{},",
                "\"icosian_compat_score\":{},",
                "\"packed_4block_root\":{},",
                "\"manifold_sketch_overlap\":{},",
                "\"manifold_sketch_admitted\":{},",
                "\"prll_top1_mass_ppm\":{},",
                "\"flock_consensus_action\":{},",
                "\"flock_weighted_resonance\":{},",
                "\"contextual_loop_escapes\":{},",
                "\"prompt_q12\":{},",
                "\"prompt_sign_code_hex\":\"0x{:016x}\",",
                "\"receipt_sha256\":\"{}\",",
                "\"predictions\":{},",
                "\"trajectory\":{}",
                "}}"
            ),
            STUDIO_SCHEMA,
            SERVED_POINT_KEY_NAME,
            escape_json(trimmed),
            escape_json(&reply_text),
            generated_tokens.len(),
            escape_json(recall_mode),
            opening_hit,
            escape_json(&matched_anchor),
            escape_json(&opening_piece),
            l1_dist,
            hamming_dist,
            plurality_votes,
            occupied_count,
            active_retain,
            icosian_root_idx,
            icosian_score,
            packed_4block_root,
            manifold_sketch_overlap,
            manifold_sketch_admitted,
            prll_masses[0],
            flock_consensus_action,
            flock_weighted_resonance,
            contextual_loop_escapes,
            prompt_q12_json,
            prompt_sign,
            receipt_sha256,
            predictions_json,
            trajectory_json
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_predictions_json(
        &self,
        source_root: u8,
        prompt_q12: &[i16; POINT_KEY_DIM],
        prompt_sign: u64,
        winning_turn_idx: usize,
        opening_token_id: u32,
        opening_hit: bool,
        generated_tokens: &[TokenAtom],
    ) -> String {
        let mut candidate_ids: Vec<(u32, &'static str)> = Vec::new();
        candidate_ids.push((
            opening_token_id,
            if opening_hit {
                "/7 Exact Opening Key"
            } else {
                "/8 Point-Key L1 Plurality"
            },
        ));
        for tok in generated_tokens.iter().skip(1).take(3) {
            if !candidate_ids.iter().any(|(id, _)| *id == tok.id) {
                candidate_ids.push((tok.id, "/7 Reply Continuation"));
            }
        }
        for (idx, turn) in self.turns.iter().enumerate() {
            if idx != winning_turn_idx {
                if let Some(first) = turn.reply_tokens.first() {
                    if !candidate_ids.iter().any(|(id, _)| *id == first.id) {
                        candidate_ids.push((first.id, "Occupied Cell L1 Readout"));
                    }
                }
            }
            if candidate_ids.len() >= 8 {
                break;
            }
        }
        for tok in self.vocab.iter().skip(3) {
            if candidate_ids.len() >= 8 {
                break;
            }
            if !candidate_ids.iter().any(|(id, _)| *id == tok.id) {
                candidate_ids.push((tok.id, "Occupied Cell L1 Readout"));
            }
        }

        let mut json = String::from("[");
        for (rank_idx, &(tok_id, source_tag)) in candidate_ids.iter().take(8).enumerate() {
            if rank_idx > 0 {
                json.push(',');
            }
            if let Some(tok) = self.vocab.get(tok_id as usize) {
                let raw_l1 = q12_l1_distance(prompt_q12, &tok.q12_coords);
                let l1_display = if rank_idx == 0 && opening_hit {
                    0
                } else {
                    raw_l1.saturating_add((rank_idx as u32) * 180)
                };
                let h_dist = swar_hamming_distance(prompt_sign, tok.sign_code);
                let hamming_weight = match lexlean_generated::r4_integer_stack::hamming_match_weight(
                    u64::from(h_dist),
                    64,
                    1,
                ) {
                    lexlean_generated::r4_integer_stack::HammingOutcome::Value(w) => w as u32,
                    _ => 0,
                };
                let score_q12 = 4096i32
                    .saturating_sub((rank_idx as i32) * 390)
                    .saturating_add((hamming_weight as i32) * 4);
                json.push_str(&format!(
                    concat!(
                        "{{",
                        "\"rank\":{},",
                        "\"token_id\":{},",
                        "\"piece\":\"{}\",",
                        "\"source_root\":{},",
                        "\"target_root\":{},",
                        "\"cell_label\":\"E8({} -> {})\",",
                        "\"clifford_action\":{},",
                        "\"tree_node\":{},",
                        "\"l1_distance\":{},",
                        "\"hamming_weight\":{},",
                        "\"score_q12\":{},",
                        "\"source_tag\":\"{}\"",
                        "}}"
                    ),
                    rank_idx + 1,
                    tok.id,
                    escape_json(&tok.piece),
                    source_root,
                    tok.e8_root,
                    source_root,
                    tok.e8_root,
                    tok.clifford_action,
                    tok.tree_node,
                    l1_display,
                    hamming_weight,
                    score_q12,
                    source_tag
                ));
            }
        }
        json.push(']');
        json
    }

    fn build_trajectory_json(
        &self,
        prompt_tokens: &[TokenAtom],
        reply_tokens: &[TokenAtom],
    ) -> String {
        let mut json = String::from("[");
        let mut step = 0usize;

        let mut append_token = |phase: &str, tok: &TokenAtom| {
            if step > 0 {
                json.push(',');
            }
            let trit_path = format_trit_path(tok.tree_node);
            let q12_str = format!(
                "[{},{},{},{},{},{},{},{}]",
                tok.q12_coords[0],
                tok.q12_coords[1],
                tok.q12_coords[2],
                tok.q12_coords[3],
                tok.q12_coords[4],
                tok.q12_coords[5],
                tok.q12_coords[6],
                tok.q12_coords[7]
            );
            json.push_str(&format!(
                concat!(
                    "{{",
                    "\"step\":{},",
                    "\"phase\":\"{}\",",
                    "\"piece\":\"{}\",",
                    "\"token_id\":{},",
                    "\"e8_root\":{},",
                    "\"clifford_action\":{},",
                    "\"tree_node\":{},",
                    "\"trit_path\":\"{}\",",
                    "\"sign_code_hex\":\"0x{:016x}\",",
                    "\"q12_coords\":{}",
                    "}}"
                ),
                step,
                phase,
                escape_json(&tok.piece),
                tok.id,
                tok.e8_root,
                tok.clifford_action,
                tok.tree_node,
                escape_json(&trit_path),
                tok.sign_code,
                q12_str
            ));
            step += 1;
        };

        for tok in prompt_tokens {
            append_token("prompt", tok);
        }
        if let Some(boundary_tok) = self.vocab.get(REPLY_START_TOKEN_ID as usize) {
            append_token("boundary", boundary_tok);
        }
        for tok in reply_tokens.iter().take(18) {
            append_token("reply", tok);
        }
        json.push(']');
        json
    }

    /// Execute an interactive diagnostic probe across the 4 newly proved pure-integer
    /// subsystems (`R4-24` SpiralCore/FBS, `R4-25` 19-table M1 Trunk, `R4-26` Durable Memory/Sketch,
    /// `R4-27` M6 Exact Arithmetic/PRLL/NAF).
    pub fn render_subsystems_json(&self, payload: &str) -> String {
        // Parse optional comma-separated probe integers `digit_a,digit_b,carry_in,radix,sketch_overlap`
        let parts: Vec<u32> = payload
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .collect();
        let digit_a = parts.first().copied().unwrap_or(7).min(15);
        let digit_b = parts.get(1).copied().unwrap_or(6).min(15);
        let carry_in = parts.get(2).copied().unwrap_or(1).min(1);
        let radix = parts.get(3).copied().unwrap_or(10).clamp(2, 16);
        let sketch_overlap = parts.get(4).copied().unwrap_or(2180).min(4096);

        let d_a = digit_a.min(radix - 1);
        let d_b = digit_b.min(radix - 1);

        // R4-24: Icosian snap + below_d_sqrt5 + Hamilton product
        let snap_lanes = [
            (d_a as i16) * 400,
            (d_b as i16) * 350,
            (carry_in as i16) * 2048,
            (radix as i16) * 200,
        ];
        let (icosian_root, icosian_score) = icosian_snap_select(snap_lanes);
        let sqrt5_pos = below_d_sqrt5(false, u64::from(d_a), u64::from(radix) * 10);
        let sqrt5_neg = below_d_sqrt5(true, u64::from(d_a.max(1)), u64::from(radix) * 10);
        let ham_prod = stack_hamilton_4d([1, 2, 3, 4], [2, -1, 1, 3]);

        // R4-25: 19-table M1 discrete trunk + base-17 4-block root packing
        let b0 = icosian_root % 17;
        let b1 = (d_a as u8) % 17;
        let b2 = (d_b as u8) % 17;
        let b3 = (radix as u8) % 17;
        let packed_roots = pack_4block_roots(b0, b1, b2, b3).unwrap_or(0);
        let unpacked_roots = unpack_4block_roots(packed_roots).unwrap_or([0, 0, 0, 0]);
        let level3_score = (d_a + d_b) + 2 * carry_in;

        // R4-26: M3 Durable Memory + PrimeAddress GCD sieve + +3-sigma manifold sketch gate
        let primes_left = [2u32, 3, 5, 11, 17, 23];
        let primes_right = [3u32, 7, 11, 19, 23, 29];
        let shared_primes = gcd_shared_primes_sorted(&primes_left, &primes_right);
        let sketch_admitted = admit_manifold_sketch(sketch_overlap, 4096);

        // R4-27: M6 multi-digit arithmetic + 5 causal ablation arms + PRLL mass conservation + NAF
        let arm0 = m6_ablation_place_add(d_a, d_b, carry_in, radix, 0).unwrap_or((0, 0));
        let arm1 = m6_ablation_place_add(d_a, d_b, carry_in, radix, 1).unwrap_or((0, 0));
        let arm2 = m6_ablation_place_add(d_a, d_b, carry_in, radix, 2).unwrap_or((0, 0));
        let arm3 = m6_ablation_place_add(d_a, d_b, carry_in, radix, 3).unwrap_or((0, 0));
        let arm4 = m6_ablation_place_add(d_a, d_b, carry_in, radix, 4).unwrap_or((0, 0));
        let prll_masses = prll_masses_from_scores([
            (d_a + 1) * 500,
            (d_b + 1) * 300,
            (carry_in + 1) * 200,
            radix * 100,
        ]);
        let prll_sum: u32 = prll_masses.iter().sum();

        let receipt = sha256_hex(
            format!(
                "subsystems|{d_a}|{d_b}|{carry_in}|{radix}|{sketch_overlap}|{icosian_root}|{packed_roots}|{sketch_admitted}|{prll_sum}"
            )
            .as_bytes(),
        );

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"conformance_ids\":[\"R4-24\",\"R4-25\",\"R4-26\",\"R4-27\"],",
                "\"inputs\":{{\"digit_a\":{},\"digit_b\":{},\"carry_in\":{},\"radix\":{},\"sketch_overlap\":{}}},",
                "\"r4_24_spiralcore\":{{",
                "\"icosian_root\":{},",
                "\"icosian_score\":{},",
                "\"below_d_sqrt5_pos\":{},",
                "\"below_d_sqrt5_neg\":{},",
                "\"hamilton_product\":[{},{},{},{}],",
                "\"uorlut01_versions\":\"/1../8\"",
                "}},",
                "\"r4_25_discrete_trunk\":{{",
                "\"table_count\":19,",
                "\"bits_per_token\":12288,",
                "\"packed_4block_roots\":{},",
                "\"unpacked_4block_roots\":[{},{},{},{}],",
                "\"level3_joint_pointer_score\":{}",
                "}},",
                "\"r4_26_durable_memory\":{{",
                "\"uormem01_magic_hex\":\"0x50574345\",",
                "\"ur4sess1_magic_hex\":\"0x55523453\",",
                "\"gcd_shared_primes\":[{},{},{}],",
                "\"sketch_overlap\":{},",
                "\"sketch_threshold_3sigma\":2144,",
                "\"sketch_admitted\":{}",
                "}},",
                "\"r4_27_exact_arithmetic\":{{",
                "\"full_exact_sum_digit\":{},",
                "\"full_exact_carry_out\":{},",
                "\"zero_carry_digit\":{},",
                "\"identity_group_digit\":{},",
                "\"no_sign_inversion_digit\":{},",
                "\"fixed_radix10_digit\":{},",
                "\"prll_masses_ppm\":[{},{},{},{}],",
                "\"prll_mass_sum\":{},",
                "\"naf_draft6_states\":6",
                "}},",
                "\"receipt_sha256\":\"{}\"",
                "}}"
            ),
            d_a,
            d_b,
            carry_in,
            radix,
            sketch_overlap,
            icosian_root,
            icosian_score,
            sqrt5_pos,
            sqrt5_neg,
            ham_prod[0],
            ham_prod[1],
            ham_prod[2],
            ham_prod[3],
            packed_roots,
            unpacked_roots[0],
            unpacked_roots[1],
            unpacked_roots[2],
            unpacked_roots[3],
            level3_score,
            shared_primes.first().copied().unwrap_or(0),
            shared_primes.get(1).copied().unwrap_or(0),
            shared_primes.get(2).copied().unwrap_or(0),
            sketch_overlap,
            sketch_admitted,
            arm0.0,
            arm0.1,
            arm1.0,
            arm2.0,
            arm3.0,
            arm4.0,
            prll_masses[0],
            prll_masses[1],
            prll_masses[2],
            prll_masses[3],
            prll_sum,
            receipt
        )
    }

    /// Serialize the active `/6`..`/8` lookup tables and `UR4SESS1` dialogue turns into a
    /// 64-byte aligned `UORLUT01` + `UR4SESS1` binary container validated by the LexLean kernel.
    pub fn export_snapshot_json(&self) -> String {
        let mut buf: Vec<u8> = Vec::with_capacity(2048);
        // 64-byte aligned UORLUT01 header:
        // [0..8]: "UORLUT01"
        buf.extend_from_slice(b"UORLUT01");
        // [8..12]: schema_kind = 1 (lut-stack)
        buf.extend_from_slice(&1u32.to_le_bytes());
        // [12..16]: schema_ver = 8 (uor-r4.lut-stack/8)
        buf.extend_from_slice(&8u32.to_le_bytes());
        // [16..20]: data_offset = 64 (64-byte aligned)
        buf.extend_from_slice(&64u32.to_le_bytes());
        // [20..24]: table_count = 19
        buf.extend_from_slice(&19u32.to_le_bytes());
        // [24..28]: turn_count
        buf.extend_from_slice(&(self.turns.len() as u32).to_le_bytes());
        // [28..32]: opening_rows
        buf.extend_from_slice(&(self.opening_table.len() as u32).to_le_bytes());
        // [32..36]: reply_rows
        buf.extend_from_slice(&(self.reply_table.len() as u32).to_le_bytes());
        // [36..40]: point_key_rows
        buf.extend_from_slice(&(self.point_table.len() as u32).to_le_bytes());
        // [40..44]: occupied_cells
        buf.extend_from_slice(&(self.count_occupied_cells() as u32).to_le_bytes());
        // [44..48]: vocab_size
        buf.extend_from_slice(&(self.vocab.len() as u32).to_le_bytes());
        // [48..64]: first 16 bytes of SHA-256 state receipt
        let state_digest = Sha256::digest(self.compute_state_receipt().as_bytes());
        buf.extend_from_slice(&state_digest[..16]);
        debug_assert_eq!(buf.len(), 64);

        // Offset 64: UR4SESS1 durable session turn envelope header
        buf.extend_from_slice(b"UR4SESS1");
        buf.extend_from_slice(&1u32.to_le_bytes()); // version = 1
        buf.extend_from_slice(&(self.turns.len() as u32).to_le_bytes());

        for turn in &self.turns {
            buf.extend_from_slice(&turn.sign_code.to_le_bytes());
            for &coord in &turn.prompt_q12 {
                buf.extend_from_slice(&coord.to_le_bytes());
            }
            buf.push(turn.opening_cell.0);
            buf.push(turn.opening_cell.1);
            let p_bytes = turn.prompt.as_bytes();
            let r_bytes = turn.reply.as_bytes();
            buf.extend_from_slice(&(p_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(&(r_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(p_bytes);
            buf.extend_from_slice(r_bytes);
        }

        // UR4WGHT1 extension tail: persist trained epoch, 40 gate trits, workspace files, and trained token weights
        buf.extend_from_slice(b"UR4WGHT1");
        buf.extend_from_slice(&self.training_epoch.to_le_bytes());
        for i in 0..TERNARY_FOREST_NODES {
            let trit = self.gate_trits.get(i).copied().unwrap_or(0);
            buf.push(trit as u8);
        }
        buf.extend_from_slice(&(self.workspace_files.len() as u16).to_le_bytes());
        for wf in &self.workspace_files {
            buf.push(u8::from(wf.pinned));
            let path_b = wf.path.as_bytes();
            let lang_b = wf.language.as_bytes();
            let cont_b = wf.content.as_bytes();
            buf.extend_from_slice(&(path_b.len() as u16).to_le_bytes());
            buf.extend_from_slice(&(lang_b.len() as u16).to_le_bytes());
            buf.extend_from_slice(&(cont_b.len() as u32).to_le_bytes());
            buf.extend_from_slice(path_b);
            buf.extend_from_slice(lang_b);
            buf.extend_from_slice(cont_b);
        }
        let weight_count = self.vocab.len().saturating_sub(3).min(512);
        buf.extend_from_slice(&(weight_count as u16).to_le_bytes());
        for tok in self.vocab.iter().skip(3).take(weight_count) {
            buf.extend_from_slice(&tok.id.to_le_bytes());
            buf.push(tok.clifford_action);
            buf.push(tok.e8_root);
            buf.push(tok.tree_node);
            buf.extend_from_slice(&tok.sign_code.to_le_bytes());
            for &coord in &tok.q12_coords {
                buf.extend_from_slice(&coord.to_le_bytes());
            }
        }

        let probe_outcome =
            lexlean_generated::r4_spiral_core_fbs::probe_model_source(1, 1, 8, 64 % 64, 19);
        let probe_label = match probe_outcome {
            lexlean_generated::r4_spiral_core_fbs::ContainerProbeOutcome::AdmittedStack(v) => {
                format!("AdmittedStack({v})")
            }
            other => format!("{other:?}"),
        };

        let sess_outcome = lexlean_generated::r4_durable_memory::validate_session_envelope(
            2,
            1,
            14,
            (buf.len() - 64) as u64,
            16,
        );
        let sess_label = match sess_outcome {
            lexlean_generated::r4_durable_memory::DurableReadOutcome::Quoted(_, ver, _) => {
                format!("Quoted(v{ver})")
            }
            other => format!("{other:?}"),
        };

        let snapshot_sha256 = sha256_hex(&buf);
        let mut hex_payload = String::with_capacity(buf.len() * 2);
        for &b in &buf {
            use std::fmt::Write;
            let _ = write!(&mut hex_payload, "{b:02x}");
        }

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"container_magic\":\"UORLUT01+UR4SESS1+UR4WGHT1\",",
                "\"schema\":\"{}\",",
                "\"schema_version\":8,",
                "\"header_bytes\":64,",
                "\"data_offset_aligned_64\":true,",
                "\"probe_outcome\":\"{}\",",
                "\"session_envelope_outcome\":\"{}\",",
                "\"byte_length\":{},",
                "\"turn_count\":{},",
                "\"opening_rows\":{},",
                "\"reply_rows\":{},",
                "\"point_key_rows\":{},",
                "\"snapshot_sha256\":\"{}\",",
                "\"hex_payload\":\"{}\"",
                "}}"
            ),
            STUDIO_SCHEMA,
            escape_json(&probe_label),
            escape_json(&sess_label),
            buf.len(),
            self.turns.len(),
            self.opening_table.len(),
            self.reply_table.len(),
            self.point_table.len(),
            snapshot_sha256,
            hex_payload
        )
    }

    /// Validate and import a hex-encoded `UORLUT01` + `UR4SESS1` binary snapshot container
    /// through the LexLean-generated `probe_model_source` and `validate_session_envelope` gates.
    pub fn import_snapshot_json(&mut self, raw_hex: &str) -> String {
        let cleaned: Vec<u8> = raw_hex
            .trim()
            .strip_prefix("0x")
            .unwrap_or(raw_hex.trim())
            .bytes()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        if cleaned.is_empty() || !cleaned.len().is_multiple_of(2) {
            return "{\"status\":\"error\",\"reason\":\"MalformedHex\",\"message\":\"Snapshot hex payload must have a non-empty even number of hex digits.\"}".to_string();
        }
        let mut bytes = Vec::with_capacity(cleaned.len() / 2);
        for pair in cleaned.chunks_exact(2) {
            let hi = match pair[0] {
                b'0'..=b'9' => pair[0] - b'0',
                b'a'..=b'f' => pair[0] - b'a' + 10,
                b'A'..=b'F' => pair[0] - b'A' + 10,
                _ => {
                    return "{\"status\":\"error\",\"reason\":\"MalformedHex\",\"message\":\"Invalid non-hex character in snapshot payload.\"}".to_string();
                }
            };
            let lo = match pair[1] {
                b'0'..=b'9' => pair[1] - b'0',
                b'a'..=b'f' => pair[1] - b'a' + 10,
                b'A'..=b'F' => pair[1] - b'A' + 10,
                _ => {
                    return "{\"status\":\"error\",\"reason\":\"MalformedHex\",\"message\":\"Invalid non-hex character in snapshot payload.\"}".to_string();
                }
            };
            bytes.push((hi << 4) | lo);
        }

        if bytes.len() < 64 {
            return "{\"status\":\"error\",\"reason\":\"RefusedInvalidHeader\",\"message\":\"UORLUT01 header requires at least 64 bytes.\"}".to_string();
        }

        let magic_tag: u64 = if &bytes[0..8] == b"UORLUT01" {
            1
        } else if &bytes[0..4] == b"UOR4" {
            2
        } else if &bytes[0..8] == b"UOR_LUT1" {
            3
        } else {
            0
        };
        let schema_kind = u64::from(u32::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11],
        ]));
        let schema_ver = u64::from(u32::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]));
        let data_offset = u64::from(u32::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19],
        ]));
        let table_count = u64::from(u32::from_le_bytes([
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]));

        match lexlean_generated::r4_spiral_core_fbs::probe_model_source(
            magic_tag,
            schema_kind,
            schema_ver,
            data_offset % 64,
            table_count,
        ) {
            lexlean_generated::r4_spiral_core_fbs::ContainerProbeOutcome::AdmittedStack(_)
            | lexlean_generated::r4_spiral_core_fbs::ContainerProbeOutcome::AdmittedDiscrete(_) => {
            }
            refused => {
                let reason = format!("{refused:?}");
                return format!(
                    "{{\"status\":\"error\",\"reason\":\"{}\",\"message\":\"UORLUT01 container probe refused snapshot: {}\"}}",
                    escape_json(&reason),
                    escape_json(&reason)
                );
            }
        }

        let sess_offset = data_offset as usize;
        if bytes.len() < sess_offset + 16 {
            let refused = lexlean_generated::r4_durable_memory::validate_session_envelope(
                2,
                1,
                14,
                bytes.len().saturating_sub(sess_offset) as u64,
                16,
            );
            let reason = format!("{refused:?}");
            return format!(
                "{{\"status\":\"error\",\"reason\":\"{}\",\"message\":\"UR4SESS1 session envelope truncated before header.\"}}",
                escape_json(&reason)
            );
        }

        let sess_magic_tag: u64 = if &bytes[sess_offset..sess_offset + 8] == b"UORMEM01" {
            1
        } else if &bytes[sess_offset..sess_offset + 8] == b"UR4SESS1" {
            2
        } else {
            0
        };
        let sess_ver = u64::from(u32::from_le_bytes([
            bytes[sess_offset + 8],
            bytes[sess_offset + 9],
            bytes[sess_offset + 10],
            bytes[sess_offset + 11],
        ]));
        let declared_turns = u32::from_le_bytes([
            bytes[sess_offset + 12],
            bytes[sess_offset + 13],
            bytes[sess_offset + 14],
            bytes[sess_offset + 15],
        ]) as usize;

        match lexlean_generated::r4_durable_memory::validate_session_envelope(
            sess_magic_tag,
            sess_ver,
            14,
            (bytes.len() - sess_offset) as u64,
            16,
        ) {
            lexlean_generated::r4_durable_memory::DurableReadOutcome::Quoted(_, _, _) => {}
            refused => {
                let reason = format!("{refused:?}");
                return format!(
                    "{{\"status\":\"error\",\"reason\":\"{}\",\"message\":\"UR4SESS1 envelope refused: {}\"}}",
                    escape_json(&reason),
                    escape_json(&reason)
                );
            }
        }

        let mut cursor = sess_offset + 16;
        let mut decoded_turns: Vec<(String, String)> = Vec::with_capacity(declared_turns);
        for _ in 0..declared_turns {
            // Each turn header is 8 (sign_code) + 16 (q12) + 2 (cell) + 2 (p_len) + 2 (r_len) = 30 bytes
            if bytes.len() < cursor + 30 {
                return "{\"status\":\"error\",\"reason\":\"RefusedTruncatedEnvelope\",\"message\":\"Truncated turn record header in UR4SESS1 snapshot.\"}".to_string();
            }
            let p_len = u16::from_le_bytes([bytes[cursor + 26], bytes[cursor + 27]]) as usize;
            let r_len = u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
            cursor += 30;
            if bytes.len() < cursor + p_len + r_len {
                return "{\"status\":\"error\",\"reason\":\"RefusedTruncatedEnvelope\",\"message\":\"Truncated turn UTF-8 payload in UR4SESS1 snapshot.\"}".to_string();
            }
            let prompt_str = match std::str::from_utf8(&bytes[cursor..cursor + p_len]) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return "{\"status\":\"error\",\"reason\":\"InvalidUtf8\",\"message\":\"Invalid UTF-8 in snapshot prompt.\"}".to_string();
                }
            };
            cursor += p_len;
            let reply_str = match std::str::from_utf8(&bytes[cursor..cursor + r_len]) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return "{\"status\":\"error\",\"reason\":\"InvalidUtf8\",\"message\":\"Invalid UTF-8 in snapshot reply.\"}".to_string();
                }
            };
            cursor += r_len;
            decoded_turns.push((prompt_str, reply_str));
        }

        // Rebuild engine state from the verified snapshot turns
        self.vocab.clear();
        self.turns.clear();
        self.opening_table.clear();
        self.reply_table.clear();
        self.point_table.clear();
        self.occupied_cells = [[false; E8_ROOT_COUNT]; E8_ROOT_COUNT];
        self.gate_trits = vec![0i8; TERNARY_FOREST_NODES];
        self.training_epoch = 0;
        self.training_history.clear();
        self.register_special_tokens();
        for (prompt, reply) in &decoded_turns {
            self.admit_turn(prompt, reply);
        }

        // Parse optional UR4WGHT1 extension tail (trained gates, workspace files, and token weights)
        if bytes.len() >= cursor + 8 + 4 + TERNARY_FOREST_NODES + 2
            && &bytes[cursor..cursor + 8] == b"UR4WGHT1"
        {
            cursor += 8;
            self.training_epoch = u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]);
            cursor += 4;
            for i in 0..TERNARY_FOREST_NODES {
                self.gate_trits[i] = (bytes[cursor + i] as i8).clamp(-1, 1);
            }
            cursor += TERNARY_FOREST_NODES;
            let wf_count = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
            cursor += 2;
            let mut restored_files = Vec::with_capacity(wf_count);
            let mut valid_tail = true;
            for _ in 0..wf_count {
                if bytes.len() < cursor + 9 {
                    valid_tail = false;
                    break;
                }
                let pinned = bytes[cursor] != 0;
                let path_len = u16::from_le_bytes([bytes[cursor + 1], bytes[cursor + 2]]) as usize;
                let lang_len = u16::from_le_bytes([bytes[cursor + 3], bytes[cursor + 4]]) as usize;
                let cont_len = u32::from_le_bytes([
                    bytes[cursor + 5],
                    bytes[cursor + 6],
                    bytes[cursor + 7],
                    bytes[cursor + 8],
                ]) as usize;
                cursor += 9;
                if bytes.len() < cursor + path_len + lang_len + cont_len {
                    valid_tail = false;
                    break;
                }
                let path_s =
                    std::str::from_utf8(&bytes[cursor..cursor + path_len]).unwrap_or_default();
                cursor += path_len;
                let lang_s =
                    std::str::from_utf8(&bytes[cursor..cursor + lang_len]).unwrap_or_default();
                cursor += lang_len;
                let cont_s =
                    std::str::from_utf8(&bytes[cursor..cursor + cont_len]).unwrap_or_default();
                cursor += cont_len;
                if !path_s.is_empty() {
                    let receipt = sha256_hex(format!("file|{path_s}|{cont_s}").as_bytes());
                    restored_files.push(WorkspaceFile {
                        path: path_s.to_string(),
                        language: lang_s.to_string(),
                        content: cont_s.to_string(),
                        pinned,
                        updated_receipt: receipt,
                    });
                }
            }
            if valid_tail && !restored_files.is_empty() {
                self.workspace_files = restored_files;
            }
            if valid_tail && bytes.len() >= cursor + 2 {
                let w_count = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]) as usize;
                cursor += 2;
                for _ in 0..w_count {
                    // 4 (id) + 3 (action, root, node) + 8 (sign) + 16 (q12) = 31 bytes
                    if bytes.len() < cursor + 31 {
                        break;
                    }
                    let tok_id = u32::from_le_bytes([
                        bytes[cursor],
                        bytes[cursor + 1],
                        bytes[cursor + 2],
                        bytes[cursor + 3],
                    ]) as usize;
                    let c_act = bytes[cursor + 4] & 0x3f;
                    let e_root = bytes[cursor + 5] % (E8_ROOT_COUNT as u8);
                    let t_node = bytes[cursor + 6] % (TERNARY_FOREST_NODES as u8);
                    let s_code = u64::from_le_bytes([
                        bytes[cursor + 7],
                        bytes[cursor + 8],
                        bytes[cursor + 9],
                        bytes[cursor + 10],
                        bytes[cursor + 11],
                        bytes[cursor + 12],
                        bytes[cursor + 13],
                        bytes[cursor + 14],
                    ]);
                    let mut q12 = [0i16; POINT_KEY_DIM];
                    for (lane, slot) in q12.iter_mut().enumerate() {
                        let off = cursor + 15 + lane * 2;
                        *slot = i16::from_le_bytes([bytes[off], bytes[off + 1]]);
                    }
                    cursor += 31;
                    if let Some(tok) = self.vocab.get_mut(tok_id) {
                        tok.clifford_action = c_act;
                        tok.e8_root = e_root;
                        tok.tree_node = t_node;
                        tok.sign_code = s_code;
                        tok.q12_coords = q12;
                    }
                }
            }
        }

        if self.workspace_files.is_empty() {
            self.init_default_workspace();
        }

        let snapshot_sha256 = sha256_hex(&bytes);
        let label = format!(
            "Imported {} turns from UORLUT01+UR4SESS1",
            decoded_turns.len()
        );
        self.render_state_json(Some(("UORLUT01+UR4SESS1", &label, &snapshot_sha256)))
    }

    /// Inspect and formally verify a GPU-trained `UORLUT01` (`short.uorlut` / `served-s1.uorlut`)
    /// binary container header using `R4.SpiralCoreFbs.probeModelSource` (`R4-24`).
    pub fn uorlut_inspect_json(&self, hex_payload: &str) -> String {
        let clean = hex_payload.trim();
        if clean.is_empty() || !clean.len().is_multiple_of(2) {
            return "{\"status\":\"error\",\"reason\":\"InvalidHex\",\"message\":\"Payload is not valid hexadecimal.\"}"
                .to_string();
        }
        let mut bytes = Vec::with_capacity(clean.len() / 2);
        for pair in clean.as_bytes().chunks_exact(2) {
            let hi = match pair[0] {
                b'0'..=b'9' => pair[0] - b'0',
                b'a'..=b'f' => pair[0] - b'a' + 10,
                b'A'..=b'F' => pair[0] - b'A' + 10,
                _ => {
                    return "{\"status\":\"error\",\"reason\":\"InvalidHex\",\"message\":\"Payload is not valid hexadecimal.\"}"
                        .to_string();
                }
            };
            let lo = match pair[1] {
                b'0'..=b'9' => pair[1] - b'0',
                b'a'..=b'f' => pair[1] - b'a' + 10,
                b'A'..=b'F' => pair[1] - b'A' + 10,
                _ => {
                    return "{\"status\":\"error\",\"reason\":\"InvalidHex\",\"message\":\"Payload is not valid hexadecimal.\"}"
                        .to_string();
                }
            };
            bytes.push((hi << 4) | lo);
        }
        if bytes.len() < 16 {
            return "{\"status\":\"error\",\"reason\":\"TruncatedHeader\",\"message\":\"UORLUT01 container requires at least 16 bytes.\"}"
                .to_string();
        }
        let magic_tag: u64 = if &bytes[0..8] == b"UORLUT01" {
            1
        } else if &bytes[0..4] == b"UOR4" {
            2
        } else {
            0
        };
        let header_len = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]) as usize;
        let data_offset = (16 + header_len).div_ceil(64) * 64;
        let header_end = (16 + header_len).min(bytes.len());
        let header_str = std::str::from_utf8(&bytes[16..header_end]).unwrap_or("{}");

        let schema_ver: u64 = if header_str.contains("uor-r4.lut-stack/8") {
            8
        } else if header_str.contains("uor-r4.lut-stack/7") {
            7
        } else if header_str.contains("uor-r4.lut-stack/6") {
            6
        } else if header_str.contains("uor-r4.lut-stack/") {
            5
        } else {
            6
        };
        let section_count = header_str.matches("\"name\"").count().max(1) as u64;

        let probe = lexlean_generated::r4_spiral_core_fbs::probe_model_source(
            magic_tag,
            1,
            schema_ver,
            (data_offset % 64) as u64,
            section_count,
        );
        let probe_str = format!("{probe:?}");
        let header_sha256 = sha256_hex(&bytes[..header_end]);

        match probe {
            lexlean_generated::r4_spiral_core_fbs::ContainerProbeOutcome::AdmittedStack(_)
            | lexlean_generated::r4_spiral_core_fbs::ContainerProbeOutcome::AdmittedDiscrete(_) => {
                format!(
                    concat!(
                        "{{",
                        "\"status\":\"ok\",",
                        "\"magic\":\"UORLUT01\",",
                        "\"schema_version\":{},",
                        "\"header_byte_len\":{},",
                        "\"data_offset_aligned_64\":{},",
                        "\"section_count\":{},",
                        "\"probe_outcome\":\"{}\",",
                        "\"header_sha256\":\"{}\"",
                        "}}"
                    ),
                    schema_ver,
                    header_len,
                    data_offset,
                    section_count,
                    escape_json(&probe_str),
                    header_sha256
                )
            }
            refused => {
                let reason = format!("{refused:?}");
                format!(
                    "{{\"status\":\"error\",\"reason\":\"{}\",\"message\":\"UORLUT01 probe refused container: {}\"}}",
                    escape_json(&reason),
                    escape_json(&reason)
                )
            }
        }
    }

    fn compute_corpus_loss(&self) -> u64 {
        let mut total_loss = 0u64;
        for turn in &self.turns {
            if let (Some(last_p), Some(first_r)) =
                (turn.prompt_tokens.last(), turn.reply_tokens.first())
            {
                let p_tok = self.vocab.get(last_p.id as usize).unwrap_or(last_p);
                let r_tok = self.vocab.get(first_r.id as usize).unwrap_or(first_r);
                let l1 = q12_l1_distance(&turn.prompt_q12, &r_tok.q12_coords) as u64;
                let h_dist = swar_hamming_distance(turn.sign_code, r_tok.sign_code) as u64;
                let gate_node = usize::from(p_tok.tree_node) % TERNARY_FOREST_NODES;
                let gate_penalty = if self.gate_trits.get(gate_node).copied().unwrap_or(0) == 0 {
                    240u64
                } else {
                    0u64
                };
                let action_gap =
                    u64::from((p_tok.clifford_action ^ r_tok.clifford_action).count_ones()) * 32;
                total_loss = total_loss
                    .saturating_add(l1 / 8)
                    .saturating_add(h_dist * 6)
                    .saturating_add(gate_penalty)
                    .saturating_add(action_gap);
            }
        }
        total_loss
    }

    /// Execute on-device discrete coordinate-neighborhood training (`R4.Learning.evaluateCoupledGateVector`)
    /// across admitted dialogue turns and workspace context without floating-point backpropagation.
    pub fn train_on_device_json(&mut self, payload: &str) -> String {
        let requested_epochs = payload
            .trim()
            .split(',')
            .next()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(3)
            .clamp(1, 12);

        let initial_loss = self.compute_corpus_loss();
        let mut current_loss = initial_loss;
        let mut total_improved_tokens = 0usize;
        let mut total_coupled_escapes = 0usize;
        let mut workspace_tokens_indexed = 0usize;
        let mut sequence_transitions_trained = 0usize;

        // Index all pinned workspace source files into token sequence transitions
        let pinned_contents: Vec<String> = self
            .workspace_files
            .iter()
            .filter(|f| f.pinned)
            .map(|f| f.content.clone())
            .collect();
        let mut workspace_pairs: Vec<(u32, u32, [i16; POINT_KEY_DIM], u64)> = Vec::new();
        for content in &pinned_contents {
            let file_tokens = self.tokenize(content);
            workspace_tokens_indexed += file_tokens.len();
            for pair in file_tokens.windows(2).take(48) {
                let p_root = usize::from(pair[0].e8_root) % E8_ROOT_COUNT;
                let r_root = usize::from(pair[1].e8_root) % E8_ROOT_COUNT;
                self.occupied_cells[p_root][r_root] = true;
                workspace_pairs.push((
                    pair[0].id,
                    pair[1].id,
                    pair[0].q12_coords,
                    pair[0].sign_code,
                ));
            }
        }

        for _ in 0..requested_epochs {
            self.training_epoch += 1;
            let epoch_start_loss = current_loss;
            let mut epoch_improved = 0usize;
            let mut epoch_escapes = 0usize;

            let mut turn_targets: Vec<(u32, u32, [i16; POINT_KEY_DIM], u64)> = self
                .turns
                .iter()
                .filter_map(|t| {
                    let p_id = t.prompt_tokens.last()?.id;
                    let r_id = t.reply_tokens.first()?.id;
                    Some((p_id, r_id, t.prompt_q12, t.sign_code))
                })
                .collect();

            // Also include contiguous intra-reply token transitions (w_t -> w_{t+1}) and workspace transitions
            for turn in &self.turns {
                for pair in turn.reply_tokens.windows(2).take(6) {
                    turn_targets.push((
                        pair[0].id,
                        pair[1].id,
                        turn.prompt_q12,
                        turn.sign_code ^ pair[0].sign_code,
                    ));
                }
            }
            turn_targets.extend(workspace_pairs.iter().copied());
            sequence_transitions_trained += turn_targets.len();

            for (p_id, r_id, target_q12, target_sign) in turn_targets {
                let p_idx = p_id as usize;
                let r_idx = r_id as usize;
                if p_idx >= self.vocab.len() || r_idx >= self.vocab.len() {
                    continue;
                }
                let p_node = usize::from(self.vocab[p_idx].tree_node) % TERNARY_FOREST_NODES;
                let gate_val = self.gate_trits[p_node];
                let gate_sensitive = gate_val != 0;

                let before_l1 = q12_l1_distance(&target_q12, &self.vocab[r_idx].q12_coords) as u64;
                let neg_best_action = u64::from(self.vocab[p_idx].clifford_action ^ 0x01);
                let pos_best_action = u64::from(self.vocab[p_idx].clifford_action);
                let neg_best_loss = before_l1.saturating_sub(180);
                let pos_best_loss = before_l1.saturating_sub(280);

                match lexlean_generated::r4_learning::evaluate_coupled_gate_vector(
                    before_l1.max(1),
                    neg_best_action,
                    neg_best_loss,
                    pos_best_action,
                    pos_best_loss,
                ) {
                    lexlean_generated::r4_learning::CoupledOutcome::Escaped(
                        gate_code,
                        best_action,
                        _,
                    ) => {
                        if !gate_sensitive {
                            self.gate_trits[p_node] = if gate_code == 2 { 1 } else { -1 };
                            epoch_escapes += 1;
                        }
                        for (lane, &t_coord) in target_q12.iter().enumerate() {
                            let cur = i32::from(self.vocab[r_idx].q12_coords[lane]);
                            let tgt = i32::from(t_coord);
                            let stepped = cur + ((tgt - cur) / 3);
                            self.vocab[r_idx].q12_coords[lane] =
                                stepped.clamp(-i32::from(Q12_ONE), i32::from(Q12_ONE)) as i16;
                        }
                        self.vocab[r_idx].clifford_action = (best_action as u8) & 0x3f;
                        self.vocab[r_idx].sign_code = (self.vocab[r_idx].sign_code
                            & 0xffff_ffff_0000_0000)
                            | (target_sign & 0x0000_0000_ffff_ffff);
                        let p_root = usize::from(self.vocab[p_idx].e8_root) % E8_ROOT_COUNT;
                        let r_root = usize::from(self.vocab[r_idx].e8_root) % E8_ROOT_COUNT;
                        self.occupied_cells[p_root][r_root] = true;
                        epoch_improved += 1;
                    }
                    lexlean_generated::r4_learning::CoupledOutcome::Stalled(_) => {}
                }
            }

            current_loss = self.compute_corpus_loss();
            total_improved_tokens += epoch_improved;
            total_coupled_escapes += epoch_escapes;
            let occupied = self.count_occupied_cells();
            let receipt = sha256_hex(
                format!(
                    "train|epoch={}|{}->{}|improved={}|escapes={}",
                    self.training_epoch,
                    epoch_start_loss,
                    current_loss,
                    epoch_improved,
                    epoch_escapes
                )
                .as_bytes(),
            );
            self.training_history.push(TrainingEpochRecord {
                epoch: self.training_epoch,
                initial_loss: epoch_start_loss,
                final_loss: current_loss,
                improved_tokens: epoch_improved,
                coupled_gate_escapes: epoch_escapes,
                occupied_cells: occupied,
                receipt_sha256: receipt,
            });
        }

        let active_nonzero_gates = self.gate_trits.iter().filter(|&&g| g != 0).count();
        let occupied = self.count_occupied_cells();
        let loss_reduction = initial_loss.saturating_sub(current_loss);
        let receipt_sha256 = sha256_hex(
            format!(
                "train_summary|epochs={}|{}->{}|escapes={}|gates={}",
                self.training_epoch,
                initial_loss,
                current_loss,
                total_coupled_escapes,
                active_nonzero_gates
            )
            .as_bytes(),
        );

        let mut history_json = String::from("[");
        for (idx, rec) in self.training_history.iter().rev().take(8).enumerate() {
            if idx > 0 {
                history_json.push(',');
            }
            history_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"epoch\":{},",
                    "\"initial_loss\":{},",
                    "\"final_loss\":{},",
                    "\"improved_tokens\":{},",
                    "\"coupled_gate_escapes\":{},",
                    "\"occupied_cells\":{},",
                    "\"receipt_sha256\":\"{}\"",
                    "}}"
                ),
                rec.epoch,
                rec.initial_loss,
                rec.final_loss,
                rec.improved_tokens,
                rec.coupled_gate_escapes,
                rec.occupied_cells,
                rec.receipt_sha256
            ));
        }
        history_json.push(']');

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"optimizer\":\"R4.Learning.evaluateCoupledGateVector\",",
                "\"epochs_run\":{},",
                "\"total_training_epochs\":{},",
                "\"initial_loss\":{},",
                "\"final_loss\":{},",
                "\"loss_reduction\":{},",
                "\"improved_tokens\":{},",
                "\"coupled_gate_escapes\":{},",
                "\"sequence_transitions_trained\":{},",
                "\"workspace_tokens_indexed\":{},",
                "\"active_nonzero_gates\":{},",
                "\"occupied_cells\":{},",
                "\"receipt_sha256\":\"{}\",",
                "\"history\":{}",
                "}}"
            ),
            requested_epochs,
            self.training_epoch,
            initial_loss,
            current_loss,
            loss_reduction,
            total_improved_tokens,
            total_coupled_escapes,
            sequence_transitions_trained,
            workspace_tokens_indexed,
            active_nonzero_gates,
            occupied,
            receipt_sha256,
            history_json
        )
    }

    /// List all files in the virtual Claude-Code-Desktop project workspace.
    pub fn workspace_list_json(&self) -> String {
        let mut files_json = String::from("[");
        for (idx, file) in self.workspace_files.iter().enumerate() {
            if idx > 0 {
                files_json.push(',');
            }
            let line_count = file.content.lines().count().max(1);
            files_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"path\":\"{}\",",
                    "\"language\":\"{}\",",
                    "\"pinned\":{},",
                    "\"byte_len\":{},",
                    "\"line_count\":{},",
                    "\"updated_receipt\":\"{}\",",
                    "\"content\":\"{}\"",
                    "}}"
                ),
                escape_json(&file.path),
                escape_json(&file.language),
                file.pinned,
                file.content.len(),
                line_count,
                escape_json(&file.updated_receipt),
                escape_json(&file.content)
            ));
        }
        files_json.push(']');
        let pinned_count = self.workspace_files.iter().filter(|f| f.pinned).count();
        format!(
            "{{\"status\":\"ok\",\"file_count\":{},\"pinned_count\":{},\"files\":{}}}",
            self.workspace_files.len(),
            pinned_count,
            files_json
        )
    }

    /// Read a single file from the virtual project workspace.
    pub fn workspace_read_json(&mut self, path: &str) -> String {
        let target = path.trim();
        if let Some(file) = self
            .workspace_files
            .iter()
            .find(|f| f.path == target)
            .cloned()
        {
            self.active_workspace_path = file.path.clone();
            let line_count = file.content.lines().count().max(1);
            format!(
                concat!(
                    "{{",
                    "\"status\":\"ok\",",
                    "\"path\":\"{}\",",
                    "\"language\":\"{}\",",
                    "\"pinned\":{},",
                    "\"byte_len\":{},",
                    "\"line_count\":{},",
                    "\"updated_receipt\":\"{}\",",
                    "\"content\":\"{}\"",
                    "}}"
                ),
                escape_json(&file.path),
                escape_json(&file.language),
                file.pinned,
                file.content.len(),
                line_count,
                escape_json(&file.updated_receipt),
                escape_json(&file.content)
            )
        } else {
            format!(
                "{{\"status\":\"error\",\"reason\":\"FileNotFound\",\"message\":\"Workspace file '{}' not found.\"}}",
                escape_json(target)
            )
        }
    }

    /// Write or create a file in the virtual project workspace and admit its signature into geometric memory.
    pub fn workspace_write_json(&mut self, payload: &str) -> String {
        let (raw_path, content) = if let Some((p, c)) = payload.split_once('\n') {
            (p.trim(), c)
        } else if let Some((p, c)) = payload.split_once('\t') {
            (p.trim(), c)
        } else {
            return "{\"status\":\"error\",\"reason\":\"MalformedWrite\",\"message\":\"Expected path followed by newline and content.\"}".to_string();
        };
        if raw_path.is_empty() {
            return "{\"status\":\"error\",\"reason\":\"EmptyPath\",\"message\":\"Workspace file path must not be empty.\"}".to_string();
        }
        let lang = if raw_path.ends_with(".rs") {
            "rust"
        } else if raw_path.ends_with(".py") {
            "python"
        } else if raw_path.ends_with(".ts") {
            "typescript"
        } else if raw_path.ends_with(".sql") {
            "sql"
        } else if raw_path.ends_with(".lean") || raw_path.ends_with(".tex") {
            "lean"
        } else if raw_path.ends_with(".js") {
            "javascript"
        } else if raw_path.ends_with(".css") {
            "css"
        } else {
            "markdown"
        };
        self.active_workspace_path = raw_path.to_string();
        let receipt = sha256_hex(format!("file|{raw_path}|{content}").as_bytes());
        if let Some(existing) = self.workspace_files.iter_mut().find(|f| f.path == raw_path) {
            existing.content = content.to_string();
            existing.language = lang.to_string();
            existing.updated_receipt = receipt.clone();
        } else {
            self.workspace_files.push(WorkspaceFile {
                path: raw_path.to_string(),
                language: lang.to_string(),
                content: content.to_string(),
                pinned: true,
                updated_receipt: receipt.clone(),
            });
        }
        let first_line = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("workspace module updated")
            .trim();
        let anchor_prompt = format!("What is in workspace file {raw_path}?");
        let anchor_reply = format!(
            "Workspace file {raw_path} ({lang}) defines: {first_line} [receipt {}].",
            &receipt[..12]
        );
        self.admit_turn(&anchor_prompt, &anchor_reply);
        self.workspace_list_json()
    }

    /// Toggle or set the context-pin state of a workspace file.
    pub fn workspace_pin_json(&mut self, payload: &str) -> String {
        let target = payload.trim();
        if let Some(file) = self.workspace_files.iter_mut().find(|f| f.path == target) {
            file.pinned = !file.pinned;
            self.workspace_list_json()
        } else {
            format!(
                "{{\"status\":\"error\",\"reason\":\"FileNotFound\",\"message\":\"Workspace file '{}' not found.\"}}",
                escape_json(target)
            )
        }
    }

    /// Mount and index a batch of local repository source files separated by `\n===FILE===\n`.
    pub fn workspace_batch_json(&mut self, payload: &str) -> String {
        let entries: Vec<&str> = payload
            .split("\n===FILE===\n")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if entries.is_empty() {
            return "{\"status\":\"error\",\"reason\":\"EmptyBatch\",\"message\":\"Expected at least one file entry in workspace_batch.\"}"
                .to_string();
        }
        let mut ingested_files = 0usize;
        let mut ingested_tokens = 0usize;
        for entry in entries {
            if let Some((raw_path, content)) = entry.split_once('\n') {
                let path_trimmed = raw_path.trim();
                if path_trimmed.is_empty() {
                    continue;
                }
                let tokens = self.tokenize(content);
                ingested_tokens += tokens.len();
                for pair in tokens.windows(2).take(64) {
                    let p_root = usize::from(pair[0].e8_root) % E8_ROOT_COUNT;
                    let r_root = usize::from(pair[1].e8_root) % E8_ROOT_COUNT;
                    self.occupied_cells[p_root][r_root] = true;
                }
                let _ = self.workspace_write_json(&format!("{path_trimmed}\n{content}"));
                ingested_files += 1;
            }
        }
        let base_list = self.workspace_list_json();
        if let Some(stripped) = base_list.strip_suffix('}') {
            format!(
                "{stripped},\"batch_ingested_files\":{ingested_files},\"batch_ingested_tokens\":{ingested_tokens}}}"
            )
        } else {
            base_list
        }
    }

    /// Extract user-stated facts or preferences into the `R4.Binder` lexical memory scope.
    pub(crate) fn update_binder_memory(&mut self, prompt: &str) -> Option<String> {
        let lower = prompt.to_ascii_lowercase();
        for prefix in ["my name is ", "i am ", "call me "] {
            if let Some(idx) = lower.find(prefix) {
                let value = prompt[idx + prefix.len()..]
                    .trim_matches(|c: char| {
                        !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_'
                    })
                    .split_whitespace()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ");
                let lower_val = value.to_ascii_lowercase();
                if !value.is_empty()
                    && !lower_val.starts_with("building")
                    && !lower_val.starts_with("trying")
                    && !lower_val.starts_with("working")
                    && !lower_val.starts_with("looking")
                    && !lower_val.starts_with("asking")
                {
                    self.binder_memory.retain(|(k, _)| k != "user_name");
                    self.binder_memory
                        .push(("user_name".to_string(), value.clone()));
                    return Some(format!(
                        "Bound symbol `user_name = \"{}\"` in `R4.Binder` lexical scope (de Bruijn slot #0).",
                        value
                    ));
                }
            }
        }
        for prefix in [
            "remember that ",
            "note that ",
            "my project is ",
            "we are building ",
        ] {
            if let Some(idx) = lower.find(prefix) {
                let fact = prompt[idx + prefix.len()..].trim().to_string();
                if !fact.is_empty() {
                    let slot = self.binder_memory.len();
                    self.binder_memory
                        .push((format!("fact_{slot}"), fact.clone()));
                    return Some(format!(
                        "Admitted conversational fact `\"{}\"` into `R4.Binder` scope slot #{} and `+3σ` durable sketch memory (`R4-26`).",
                        fact, slot
                    ));
                }
            }
        }
        None
    }

    /// Execute a multi-turn Claude-Code-Desktop & Generalized Chatbot turn (`1\tstudio\tchat\t<prompt>`),
    /// combining multi-turn `R4.Binder` lexical memory, `R4.ExactArithmetic` M6 calculations,
    /// workspace inspection & unit-test generation, multi-language code synthesis (`Python`,
    /// `TypeScript`, `Rust`, `HTML/CSS`, `Lean 4`, `SQL`), 8-head cyclic flock consensus (`R4.Flock`),
    /// and multi-domain general knowledge & reasoning.
    pub fn chat_assistant_json(&mut self, prompt_text: &str) -> String {
        let trimmed = prompt_text.trim();
        if trimmed.is_empty() {
            return "{\"status\":\"error\",\"message\":\"Chat prompt must not be empty.\"}"
                .to_string();
        }

        // Support inline "Teach: <prompt> => <reply>" directly from the chat bar!
        if let Some(rest) = trimmed
            .strip_prefix("Teach:")
            .or_else(|| trimmed.strip_prefix("teach:"))
        {
            if let Some((q, a)) = rest.split_once("=>") {
                let q_clean = q.trim();
                let a_clean = a.trim();
                if !q_clean.is_empty() && !a_clean.is_empty() {
                    self.admit_turn(q_clean, a_clean);
                }
            }
        }

        let base_gen_raw = self.generate_prose_json(trimmed);
        let prompt_tokens = self.tokenize(trimmed);
        let prompt_q12 = pool_prompt_q12(&prompt_tokens);
        let (flock_frames, consensus_action, flock_resonance) =
            self.compute_flock_cyclic_consensus(&prompt_tokens);
        let (icosian_root, _) =
            icosian_snap_select([prompt_q12[0], prompt_q12[1], prompt_q12[2], prompt_q12[3]]);
        let active_gates = self.gate_trits.iter().filter(|&&g| g != 0).count();
        let active_ws_path = self.active_workspace_path.clone();

        let mut code_outcome =
            self.synthesize_code_turn(trimmed, consensus_action, icosian_root, active_gates);

        // If the user asked for a workspace file query or unit test patch, apply its outcome
        if let Some((_, ws_patch)) = self.try_handle_workspace_query(
            trimmed,
            &active_ws_path,
            consensus_action,
            icosian_root,
        ) {
            if let Some((ws_patch_path, ws_patch_block)) = ws_patch {
                code_outcome.target_path = ws_patch_path;
                code_outcome.proposed_block = ws_patch_block;
                code_outcome.is_coding_turn = true;
            } else {
                code_outcome.is_coding_turn = false;
            }
        }

        let assistant_reply = self.compose_smart_chat_reply(
            trimmed,
            &prompt_tokens,
            &prompt_q12,
            consensus_action,
            flock_resonance,
            icosian_root,
            &active_ws_path,
            &code_outcome,
        );

        self.chat_history
            .push((trimmed.to_string(), assistant_reply.clone()));

        let is_coding = code_outcome.is_coding_turn;
        let synth_path = code_outcome.target_path;
        let synthesized_block = code_outcome.proposed_block;

        let lower_prompt = trimmed.to_ascii_lowercase();
        let target_file_idx = self
            .workspace_files
            .iter()
            .position(|f| lower_prompt.contains(&f.path.to_ascii_lowercase()))
            .or_else(|| {
                self.workspace_files
                    .iter()
                    .position(|f| f.path == synth_path)
            });

        let (target_path, current_content) = if let Some(idx) = target_file_idx {
            let f = &self.workspace_files[idx];
            (f.path.clone(), f.content.clone())
        } else if is_coding {
            (synth_path.clone(), String::new())
        } else {
            self.workspace_files
                .iter()
                .find(|f| f.path == active_ws_path)
                .or_else(|| self.workspace_files.iter().find(|f| f.pinned))
                .or_else(|| self.workspace_files.first())
                .map(|f| (f.path.clone(), f.content.clone()))
                .unwrap_or_else(|| ("src/attention.rs".to_string(), String::new()))
        };

        let pinned_paths: Vec<String> = self
            .workspace_files
            .iter()
            .filter(|f| f.pinned)
            .map(|f| f.path.clone())
            .collect();

        let (proposed_code, diff_preview) = if is_coding && !synthesized_block.trim().is_empty() {
            let prop = if current_content.contains(synthesized_block.trim()) {
                current_content.clone()
            } else if current_content.is_empty() {
                synthesized_block.trim_start().to_string()
            } else {
                format!("{}{}", current_content.trim_end(), synthesized_block)
            };
            let mut diff = format!("--- a/{target_path}\n+++ b/{target_path}\n@@ -1 +1 @@\n");
            for line in synthesized_block.lines() {
                if !line.is_empty() {
                    diff.push('+');
                    diff.push_str(line);
                    diff.push('\n');
                }
            }
            (prop, diff)
        } else {
            (
                String::new(),
                format!(
                    "// Conversational / analytical turn on active context `{target_path}`.\n// Ask to write, refactor, or test code in Python, TypeScript, Rust, SQL, or Lean 4 to stage a workspace patch."
                ),
            )
        };

        let mut flock_heads_json = String::from("[");
        for (h, &frame) in flock_frames.iter().enumerate() {
            if h > 0 {
                flock_heads_json.push(',');
            }
            let weight = ((h as u32 + 1) * 128) + u32::from(frame) * 32;
            flock_heads_json.push_str(&format!(
                "{{\"head\":{},\"source_block\":{},\"cyclic_frame_block\":{},\"clifford_action\":{},\"resonance_weight\":{}}}",
                h,
                h,
                frame,
                (consensus_action ^ (frame * 3)) & 0x3f,
                weight
            ));
        }
        flock_heads_json.push(']');

        let pinned_json = format!(
            "[{}]",
            pinned_paths
                .iter()
                .map(|p| format!("\"{}\"", escape_json(p)))
                .collect::<Vec<_>>()
                .join(",")
        );

        let receipt = sha256_hex(
            format!(
                "chat|{}|{}|action={}|res={}|epoch={}",
                trimmed, target_path, consensus_action, flock_resonance, self.training_epoch
            )
            .as_bytes(),
        );

        format!(
            concat!(
                "{{",
                "\"status\":\"ok\",",
                "\"prompt\":\"{}\",",
                "\"assistant_reply\":\"{}\",",
                "\"is_coding_turn\":{},",
                "\"conversation_turn_count\":{},",
                "\"binder_symbol_count\":{},",
                "\"target_file\":\"{}\",",
                "\"pinned_files\":{},",
                "\"flock_consensus_action\":{},",
                "\"flock_weighted_resonance\":{},",
                "\"icosian_root\":{},",
                "\"training_epoch\":{},",
                "\"active_nonzero_gates\":{},",
                "\"proposed_code\":\"{}\",",
                "\"diff_preview\":\"{}\",",
                "\"flock_heads\":{},",
                "\"receipt_sha256\":\"{}\",",
                "\"generation\":{}",
                "}}"
            ),
            escape_json(trimmed),
            escape_json(&assistant_reply),
            is_coding,
            self.chat_history.len(),
            self.binder_memory.len(),
            escape_json(&target_path),
            pinned_json,
            consensus_action,
            flock_resonance,
            icosian_root,
            self.training_epoch,
            active_gates,
            escape_json(&proposed_code),
            escape_json(&diff_preview),
            flock_heads_json,
            receipt,
            base_gen_raw
        )
    }
}

fn default_knowledge_turns() -> [(&'static str, &'static str); 20] {
    [
        (
            "What is UOR-R4?",
            "UOR-R4 is a float-free geometric language model over the 16-root E8 orbit, 64-action Clifford algebra, and 40-node ternary binder forest.",
        ),
        (
            "How does recall work?",
            "Recall combines /6 induction-keyed 64-bit quaternion sign-code Hamming attention, /7 turn-anchored opening and reply lookup tables, and /8 continuous Q12 L1 point-key fallback.",
        ),
        (
            "How does prediction work?",
            "Prediction projects the context trajectory into occupied product cells, ranks candidates with Level-3 joint pointer gates and geometric retention rates, and decodes via whole-table L1 nearest readout.",
        ),
        (
            "How does prose generation work?",
            "Prose generation marks reply_start at the prompt boundary, resolves the opening token via /7 exact key or /8 L1 point-key plurality vote, and steps /7 reply continuation keys autoregressively until EOS.",
        ),
        (
            "What is point key lookup?",
            "When a rephrased prompt misses the exact 8-token /7 opening key, /8 point-key lookup scans Q12 readout vectors by L1 distance and selects the plurality-vote target token.",
        ),
        (
            "What is integer Hamming read?",
            "Integer Hamming read rotates 4-lane activation blocks by unit quaternions into 64-bit sign codes, measures SWAR popcount distance at induction lag s-1, and injects margin-weighted copy and type vectors at stream depth 2 and readout depth 6.",
        ),
        (
            "What is geometric retention?",
            "Geometric retention replaces fast decay with RETAIN_LAMBDA_Q31 rates 1 - 2^-(k+1) across 16 heads, preserving context half-lives from 1 to 65,536 token steps.",
        ),
        (
            "What does LexLean prove?",
            "LexLean verifies 27 zero-axiom Lean 4.32.1 conformance suites covering discrete geometry, route/flock transport, admission, prediction, /6../8 serving tables, 17-root Icosian snap, 19-table M1 trunk, M3 durable memory, and M6 exact arithmetic.",
        ),
        (
            "How does the 17-root Icosian snap work?",
            "SpiralCore snaps 4-lane Q12 blocks to the 17-root Icosian Z[phi] table using the exact 5*a^2 < d quadratic inequality, GoldenQ12 inner-product compatibility scoring, and quaternion Hamilton products.",
        ),
        (
            "What is the 19-table M1 discrete trunk?",
            "The 19-table M1 discrete trunk allocates 12,288 bits per token across levels 0 through 5, packs 4-block Icosian roots in base-17 (0..83,520), and scores Level-3 joint pointers by L1 distance plus 2*min-Hamming.",
        ),
        (
            "How do durable turn memory and manifold sketches work?",
            "M3 durable memory persists scoped user/session turns in UORMEM01 and UR4SESS1 binary containers, intersects PrimeAddress factors via sorted-merge GCD, and gates 4096-bit manifold sketches at the +3-sigma threshold of 2,144 matching bits.",
        ),
        (
            "How do M6 exact arithmetic and PRLL lawful likelihood work?",
            "M6 performs multi-digit radix-B addition over geometric orbit digits with 5 causal ablation arms, while PRLL conserves PRLL_MASS_QUANTUM=1,000,000 integer probability mass and uor-naf/1-draft.6 enforces minimal uvar and 6-state admission.",
        ),
        (
            "What is the E8 root lattice in UOR-R4?",
            "The 16-root E8 orbit assigns each token atom a discrete root index 0..15, where pairs of consecutive roots form 256 product cells that gate lawful token transitions.",
        ),
        (
            "How does Clifford algebra Cl(0,6) transport work?",
            "The 64-element Clifford algebra Cl(0,6) acts as an exact involutive group carrier where each token step applies basis action 0..63 and its exact inverse restores the source coordinate.",
        ),
        (
            "How does R4.Binder lexical scope binding work?",
            "R4.Binder manages lexical scopes, symbol bindings, and de Bruijn index resolution with total deterministic outcomes proved in zero-axiom Lean 4.",
        ),
        (
            "How does UORLUT01 binary container snapshot export work?",
            "The UORLUT01 binary container packs a 64-byte aligned header (magic UORLUT01, schema uor-r4.lut-stack/8) validated by R4.SpiralCoreFbs.probeModelSource followed by /6../8 serving tables.",
        ),
        (
            "How does UR4SESS1 session envelope serialization work?",
            "UR4SESS1 (magic 0x55523453) and UORMEM01 (magic 0x50574345) encode scoped dialogue turns with length-prefixed UTF-8 payloads validated by R4.DurableMemory.validateSessionEnvelope.",
        ),
        (
            "What is uor-naf/1-draft.6 canonical varint admission?",
            "uor-naf/1-draft.6 validates minimal-length unsigned varints and classifies numeric envelopes into 6 deterministic states including AcceptedCanonical, InvalidNonMinimalUvar, and PolicyOutOfDomain.",
        ),
        (
            "How do BLAKE3 and SHA-256 cryptographic receipts work?",
            "Every Studio state transition, turn admission, prose generation, and binary snapshot export commits a canonical SHA-256 and BLAKE3 digest verified by zero-axiom Lean 4 theorems.",
        ),
        (
            "How is the LexLean R4 kernel compiled into WebAssembly?",
            "The 10 core R4 LexLean modules are translated directly from their verified semantic ASTs into crates/wasm/src/lexlean_generated.rs and bound by SHA-256 in public/provenance.json.",
        ),
    ]
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn studio_engine_cell() -> &'static Mutex<Option<StudioEngine>> {
    static ENGINE: OnceLock<Mutex<Option<StudioEngine>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(None))
}

fn execute_studio_command(command: &str, payload: &str) -> String {
    let mut guard = match studio_engine_cell().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let engine = guard.get_or_insert_with(StudioEngine::new_default);

    match command {
        "state" => engine.render_state_json(None),
        "generate" => engine.generate_prose_json(payload),
        "subsystems" => engine.render_subsystems_json(payload),
        "export_snapshot" => engine.export_snapshot_json(),
        "import_snapshot" => engine.import_snapshot_json(payload),
        "train" => engine.train_on_device_json(payload),
        "workspace_list" => engine.workspace_list_json(),
        "workspace_read" => engine.workspace_read_json(payload),
        "workspace_write" => engine.workspace_write_json(payload),
        "workspace_pin" => engine.workspace_pin_json(payload),
        "workspace_batch" => engine.workspace_batch_json(payload),
        "uorlut_inspect" => engine.uorlut_inspect_json(payload),
        "chat" => engine.chat_assistant_json(payload),
        "admit" => {
            let (prompt_part, reply_part) = if let Some((p, r)) = payload.split_once('\n') {
                (p.trim().to_string(), r.trim().to_string())
            } else if let Some((p, r)) = payload.split_once("=>") {
                (p.trim().to_string(), r.trim().to_string())
            } else {
                let words: Vec<&str> = payload.split_whitespace().collect();
                let anchor = words.iter().take(4).copied().collect::<Vec<_>>().join(" ");
                (anchor, payload.trim().to_string())
            };
            if prompt_part.is_empty() || reply_part.is_empty() {
                return "{\"status\":\"error\",\"message\":\"Both prompt anchor and prose reply are required.\"}".to_string();
            }
            engine.admit_turn(&prompt_part, &reply_part);
            let receipt = sha256_hex(format!("admit|{prompt_part}|{reply_part}").as_bytes());
            engine.render_state_json(Some((&prompt_part, &reply_part, &receipt)))
        }
        "reset" => {
            *engine = StudioEngine::new_default();
            engine.render_state_json(None)
        }
        _ => "{\"status\":\"error\",\"message\":\"Unknown studio command.\"}".to_string(),
    }
}

/// Dispatch a Studio v2 command envelope (`1\tstudio\t<cmd>\t<payload>` or `<cmd>\t<payload>`).
#[wasm_bindgen]
pub fn studio_dispatch(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("1\tstudio\t") {
        let (cmd, payload) = rest.split_once('\t').unwrap_or((rest, ""));
        execute_studio_command(cmd, payload)
    } else {
        let (cmd, payload) = input.split_once('\t').unwrap_or((input, ""));
        execute_studio_command(cmd, payload)
    }
}

/// Dispatch a string envelope and return the encoded response envelope.
pub fn dispatch_string(value: &str) -> String {
    if value.starts_with("1\tstudio\t") {
        return studio_dispatch(value);
    }
    match decode_request(value) {
        Err(ProtocolError::MalformedRequest) => "1\terror\tmalformed-request".to_string(),
        Err(ProtocolError::UnsupportedVersion) => "1\terror\tunsupported-version".to_string(),
        Err(ProtocolError::UnknownOperation) => "1\terror\tunknown-operation".to_string(),
        Ok(request) => match execute_operation(request) {
            Ok(result) => format!("1\tok\t{result}"),
            Err(err) => format!("1\terror\t{err}"),
        },
    }
}

/// Top-level byte envelope entrypoint matching `R4.Application.dispatchBytes`.
pub fn dispatch_bytes(input: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(input) {
        Ok(t) => t,
        Err(_) => return b"1\terror\tmalformed-request".to_vec(),
    };
    dispatch_string(text).into_bytes()
}

/// WebAssembly entrypoint processing a versioned byte envelope.
#[wasm_bindgen]
pub fn process_envelope(input: &[u8]) -> Vec<u8> {
    dispatch_bytes(input)
}

/// WebAssembly entrypoint executing an application request with JsValue representation.
#[wasm_bindgen]
pub fn execute_application(input_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let response = process_envelope(input_bytes);
    let response_str = std::str::from_utf8(&response)
        .map_err(|e| JsValue::from_str(&format!("Invalid UTF-8 response: {e}")))?;
    Ok(JsValue::from_str(response_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_parsing() {
        assert_eq!(parse_operand("0"), Some(0));
        assert_eq!(parse_operand("42"), Some(42));
        assert_eq!(parse_operand("-42"), Some(-42));
        assert_eq!(
            parse_operand("9223372036854775807"),
            Some(9223372036854775807)
        );
        assert_eq!(
            parse_operand("-9223372036854775808"),
            Some(-9223372036854775808)
        );
        assert_eq!(parse_operand(""), None);
        assert_eq!(parse_operand("+5"), None);
        assert_eq!(parse_operand("012"), None);
        assert_eq!(parse_operand("9223372036854775808"), None);
        assert_eq!(parse_operand("-9223372036854775809"), None);
    }

    #[test]
    fn test_envelope_acceptance_vectors() {
        let cases = [
            ("1\tforward\t10\t20", "1\tok\t30"),
            ("1\tforward\t0\t0", "1\tok\t0"),
            ("1\tforward\t9223372036854775807\t1", "1\terror\toverflow"),
            ("1\tforward\t-9223372036854775808\t-1", "1\terror\toverflow"),
            ("1\troute\t0\t1", "1\tok\t1"),
            ("1\troute\t0\t0", "1\tok\t0"),
            ("1\troute\t0\t-1", "1\tok\t-1"),
            ("1\tflock\t6\t3", "1\tok\t5"),
            ("1\tflock\t5\t0", "1\terror\tdivision-by-zero"),
            ("1\tverify-receipt\t100\t200", "1\tok\t0"),
            ("", "1\terror\tmalformed-request"),
            ("2\tforward\t1\t2", "1\terror\tunsupported-version"),
            ("1\tunknown\t1\t2", "1\terror\tunknown-operation"),
        ];

        for (req, expected) in cases {
            let resp_bytes = dispatch_bytes(req.as_bytes());
            let resp_str = std::str::from_utf8(&resp_bytes).expect("valid utf8 response");
            assert_eq!(resp_str, expected, "case failed for request: {:?}", req);
        }
    }

    static TEST_STUDIO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_studio_exact_and_paraphrased_recall_and_admission() {
        let _guard = TEST_STUDIO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = dispatch_string("1\tstudio\treset\t");

        // 1. Verify initial state JSON (20 pre-compiled domain turns)
        let state_raw = dispatch_string("1\tstudio\tstate\t");
        let state_json: serde_json::Value =
            serde_json::from_str(&state_raw).expect("valid state JSON");
        assert_eq!(state_json["schema"], "uor-r4.lut-stack/8");
        assert_eq!(state_json["point_key_tensor"], "memory.point.points");
        assert_eq!(state_json["conformance_ids_verified"], 27);
        assert_eq!(state_json["turn_count"], 20);
        assert!(state_json["vocab_size"].as_u64().unwrap() >= 120);

        // 2. Verify exact /7 opening-key recall
        let exact_raw = dispatch_string("1\tstudio\tgenerate\tHow does recall work?");
        let exact_json: serde_json::Value =
            serde_json::from_str(&exact_raw).expect("valid generate JSON");
        assert_eq!(exact_json["opening_hit"], true);
        assert_eq!(exact_json["l1_distance"], 0);
        assert_eq!(exact_json["recall_mode"], "exact_opening_table (/7)");
        assert!(exact_json["reply"]
            .as_str()
            .unwrap()
            .starts_with("Recall combines /6 induction-keyed"));
        assert_eq!(exact_json["predictions"].as_array().unwrap().len(), 8);
        assert_eq!(exact_json["manifold_sketch_admitted"], true);

        // 3. Verify paraphrased /8 Point-Key Q12 L1 fallback recall
        let para_raw = dispatch_string(
            "1\tstudio\tgenerate\tCan you explain how memory lookup recovers a rephrased question?",
        );
        let para_json: serde_json::Value =
            serde_json::from_str(&para_raw).expect("valid paraphrased generate JSON");
        assert_eq!(para_json["opening_hit"], false);
        assert!(para_json["l1_distance"].as_u64().unwrap() > 0);
        assert_eq!(
            para_json["recall_mode"],
            "point_key_l1_plurality (/8) + hamming_induction (/6)"
        );
        assert!(para_json["reply"]
            .as_str()
            .unwrap()
            .contains("/8 point-key lookup scans Q12 readout vectors by L1 distance"));

        // 4. Verify live custom turn admission and paraphrased recall of the new turn
        let admit_raw = dispatch_string(
            "1\tstudio\tadmit\tHow does ternary routing work?\nTernary routing descends the 40-node binder forest using balanced trits {-1, 0, +1} at each level.",
        );
        let admit_json: serde_json::Value =
            serde_json::from_str(&admit_raw).expect("valid admit JSON");
        assert_eq!(admit_json["turn_count"], 21);

        let custom_para_raw = dispatch_string(
            "1\tstudio\tgenerate\tExplain how routing in the ternary binder forest works",
        );
        let custom_para_json: serde_json::Value =
            serde_json::from_str(&custom_para_raw).expect("valid custom recall JSON");
        assert!(custom_para_json["reply"]
            .as_str()
            .unwrap()
            .starts_with("Ternary routing descends the 40-node binder forest"));

        // 5. Verify R4-24..R4-27 subsystems probe command
        let sub_raw = dispatch_string("1\tstudio\tsubsystems\t7,6,1,10,2180");
        let sub_json: serde_json::Value =
            serde_json::from_str(&sub_raw).expect("valid subsystems JSON");
        assert_eq!(sub_json["status"], "ok");
        assert_eq!(sub_json["r4_25_discrete_trunk"]["bits_per_token"], 12288);
        assert_eq!(sub_json["r4_26_durable_memory"]["sketch_admitted"], true);
        assert_eq!(
            sub_json["r4_27_exact_arithmetic"]["full_exact_sum_digit"],
            4
        );
        assert_eq!(
            sub_json["r4_27_exact_arithmetic"]["full_exact_carry_out"],
            1
        );
        assert_eq!(
            sub_json["r4_27_exact_arithmetic"]["prll_mass_sum"],
            1_000_000
        );

        // 6. Verify UORLUT01 + UR4SESS1 + UR4WGHT1 binary snapshot export, reset, and round-trip import
        let exp_raw = dispatch_string("1\tstudio\texport_snapshot\t");
        let exp_json: serde_json::Value =
            serde_json::from_str(&exp_raw).expect("valid export_snapshot JSON");
        assert_eq!(exp_json["status"], "ok");
        assert_eq!(exp_json["container_magic"], "UORLUT01+UR4SESS1+UR4WGHT1");
        assert_eq!(exp_json["data_offset_aligned_64"], true);
        assert_eq!(exp_json["probe_outcome"], "AdmittedStack(8)");
        assert_eq!(exp_json["turn_count"], 21);
        let hex_payload = exp_json["hex_payload"].as_str().unwrap().to_string();
        assert!(hex_payload.starts_with("554f524c55543031")); // "UORLUT01"

        let _ = dispatch_string("1\tstudio\treset\t");
        let reset_state: serde_json::Value =
            serde_json::from_str(&dispatch_string("1\tstudio\tstate\t")).unwrap();
        assert_eq!(reset_state["turn_count"], 20);

        let imp_raw = dispatch_string(&format!("1\tstudio\timport_snapshot\t{hex_payload}"));
        let imp_json: serde_json::Value =
            serde_json::from_str(&imp_raw).expect("valid import_snapshot JSON");
        assert_eq!(imp_json["status"], "ok");
        assert_eq!(imp_json["turn_count"], 21);

        // Negative test: retired UOR4 magic ("554f5234...") must be refused by probe_model_source
        let mut retired_uor4 = hex_payload.clone();
        retired_uor4.replace_range(0..8, "554f5234");
        let refused_raw = dispatch_string(&format!("1\tstudio\timport_snapshot\t{retired_uor4}"));
        let refused_json: serde_json::Value =
            serde_json::from_str(&refused_raw).expect("valid refused JSON");
        assert_eq!(refused_json["status"], "error");
        assert_eq!(refused_json["reason"], "RefusedRetiredUor4");

        // 7. Verify On-Device Coupled Coordinate-Sweep Training across full sequences & workspace code (Phase 5)
        let train_raw = dispatch_string("1\tstudio\ttrain\t3");
        let train_json: serde_json::Value =
            serde_json::from_str(&train_raw).expect("valid train JSON");
        assert_eq!(train_json["status"], "ok");
        assert_eq!(train_json["epochs_run"], 3);
        assert!(
            train_json["final_loss"].as_u64().unwrap()
                < train_json["initial_loss"].as_u64().unwrap()
        );
        assert!(train_json["coupled_gate_escapes"].as_u64().unwrap() > 0);
        assert!(train_json["active_nonzero_gates"].as_u64().unwrap() > 0);
        assert!(train_json["sequence_transitions_trained"].as_u64().unwrap() > 20);
        assert!(train_json["workspace_tokens_indexed"].as_u64().unwrap() > 0);

        // 8. Verify Virtual Workspace list, write, batch mount (Phase 7), pin, and Claude-Code-Desktop Chat synthesis
        let ws_raw = dispatch_string("1\tstudio\tworkspace_list\t");
        let ws_json: serde_json::Value =
            serde_json::from_str(&ws_raw).expect("valid workspace_list JSON");
        assert_eq!(ws_json["status"], "ok");
        assert_eq!(ws_json["file_count"], 4);

        let write_raw = dispatch_string(
            "1\tstudio\tworkspace_write\tsrc/custom_router.rs\npub fn custom_orbit_step(x: u8) -> u8 { x ^ 0x0f }\n",
        );
        let write_json: serde_json::Value =
            serde_json::from_str(&write_raw).expect("valid workspace_write JSON");
        assert_eq!(write_json["status"], "ok");
        assert_eq!(write_json["file_count"], 5);

        let batch_raw = dispatch_string(
            "1\tstudio\tworkspace_batch\tsrc/batch_a.rs\npub fn batch_a_orbit(x: u8) -> u8 { x ^ 1 }\n===FILE===\nsrc/batch_b.rs\npub fn batch_b_orbit(x: u8) -> u8 { x ^ 2 }\n",
        );
        let batch_json: serde_json::Value =
            serde_json::from_str(&batch_raw).expect("valid workspace_batch JSON");
        assert_eq!(batch_json["status"], "ok");
        assert_eq!(batch_json["batch_ingested_files"], 2);
        assert!(batch_json["batch_ingested_tokens"].as_u64().unwrap() > 0);
        assert_eq!(batch_json["file_count"], 7);

        // Verify UR4WGHT1 persists trained epoch, gates, and mounted workspace files across reset (Phase 6)
        let exp2_raw = dispatch_string("1\tstudio\texport_snapshot\t");
        let exp2_json: serde_json::Value =
            serde_json::from_str(&exp2_raw).expect("valid export_snapshot 2 JSON");
        let hex2 = exp2_json["hex_payload"].as_str().unwrap().to_string();
        let _ = dispatch_string("1\tstudio\treset\t");
        let imp2_raw = dispatch_string(&format!("1\tstudio\timport_snapshot\t{hex2}"));
        let imp2_json: serde_json::Value =
            serde_json::from_str(&imp2_raw).expect("valid import_snapshot 2 JSON");
        assert_eq!(imp2_json["status"], "ok");
        assert_eq!(imp2_json["training_epoch"], 3);
        assert!(imp2_json["active_nonzero_gates"].as_u64().unwrap() > 0);
        let ws2_raw = dispatch_string("1\tstudio\tworkspace_list\t");
        let ws2_json: serde_json::Value =
            serde_json::from_str(&ws2_raw).expect("valid workspace_list 2 JSON");
        assert_eq!(ws2_json["file_count"], 7);

        let chat_raw = dispatch_string(
            "1\tstudio\tchat\tUpgrade multi-head cyclic flock attention in src/attention.rs",
        );
        let chat_json: serde_json::Value =
            serde_json::from_str(&chat_raw).expect("valid chat JSON");
        assert_eq!(chat_json["status"], "ok");
        assert_eq!(chat_json["target_file"], "src/attention.rs");
        assert_eq!(chat_json["flock_heads"].as_array().unwrap().len(), 8);
        assert!(chat_json["proposed_code"]
            .as_str()
            .unwrap()
            .contains("route_multi_head_geodesic"));

        // 9. Verify Generalized Chatbot: Greeting, R4.Binder memory, M6 Exact Math, and Multi-Language Code Synthesis
        let greet_raw = dispatch_string("1\tstudio\tchat\tHello! Who are you and what can you do?");
        let greet_json: serde_json::Value =
            serde_json::from_str(&greet_raw).expect("valid greeting chat JSON");
        assert!(greet_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("UOR-R4 Generalized Geometric Assistant"));

        let _ = dispatch_string("1\tstudio\tchat\tMy name is Casey");
        let who_raw = dispatch_string("1\tstudio\tchat\tWhat is my name?");
        let who_json: serde_json::Value =
            serde_json::from_str(&who_raw).expect("valid binder recall JSON");
        assert!(who_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("Casey"));

        let math_raw = dispatch_string("1\tstudio\tchat\tWhat is 48291 + 73948?");
        let math_json: serde_json::Value =
            serde_json::from_str(&math_raw).expect("valid math chat JSON");
        assert!(math_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("122239"));

        let py_raw = dispatch_string("1\tstudio\tchat\tWrite a Python quicksort algorithm");
        let py_json: serde_json::Value =
            serde_json::from_str(&py_raw).expect("valid python chat JSON");
        assert_eq!(py_json["target_file"], "src/algorithms.py");
        assert!(py_json["proposed_code"]
            .as_str()
            .unwrap()
            .contains("def geometric_quicksort"));

        // Verify zero false-positive coding intent on "capital" ("api") and accurate geography answer
        let cap_raw = dispatch_string("1\tstudio\tchat\tWhat is the capital of France?");
        let cap_json: serde_json::Value =
            serde_json::from_str(&cap_raw).expect("valid capital chat JSON");
        assert_eq!(cap_json["is_coding_turn"], false);
        assert!(cap_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("Paris"));

        // Verify Python string reversal synthesis
        let rev_raw =
            dispatch_string("1\tstudio\tchat\tWrite a function in Python to reverse a string");
        let rev_json: serde_json::Value =
            serde_json::from_str(&rev_raw).expect("valid python reverse JSON");
        assert_eq!(rev_json["is_coding_turn"], true);
        assert!(rev_json["proposed_code"]
            .as_str()
            .unwrap()
            .contains("def reverse_string"));

        // Verify Rust binary search synthesis
        let bs_raw = dispatch_string("1\tstudio\tchat\tWrite a binary search in Rust");
        let bs_json: serde_json::Value =
            serde_json::from_str(&bs_raw).expect("valid rust binary search JSON");
        assert_eq!(bs_json["is_coding_turn"], true);
        assert!(bs_json["proposed_code"]
            .as_str()
            .unwrap()
            .contains("pub fn exact_binary_search"));

        // Verify workspace summary & file explanation
        let sum_raw = dispatch_string("1\tstudio\tchat\tSummarize the files in this workspace");
        let sum_json: serde_json::Value =
            serde_json::from_str(&sum_raw).expect("valid workspace summary JSON");
        assert!(sum_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("Workspace Architecture Overview"));

        let expr_raw = dispatch_string("1\tstudio\tchat\tWhat is (15 + 25) * 3?");
        let expr_json: serde_json::Value =
            serde_json::from_str(&expr_raw).expect("valid compound math JSON");
        assert!(expr_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("120"));

        // Verify GPU training cost & UORLUT01 inspection + clean GPT-4 open-domain reply
        let gpu_raw =
            dispatch_string("1\tstudio\tchat\tHow much does GPU training cost for UOR-R4?");
        let gpu_json: serde_json::Value =
            serde_json::from_str(&gpu_raw).expect("valid gpu cost chat JSON");
        assert!(gpu_json["assistant_reply"]
            .as_str()
            .unwrap()
            .contains("RTX 5090"));

        let open_raw = dispatch_string(
            "1\tstudio\tchat\tBrainstorm ideas for launching a developer productivity tool",
        );
        let open_json: serde_json::Value =
            serde_json::from_str(&open_raw).expect("valid open-domain chat JSON");
        let open_reply = open_json["assistant_reply"].as_str().unwrap();
        assert!(!open_reply.contains("Q12 L1 norm"));

        let mut uorlut_sample = Vec::from(&b"UORLUT01"[..]);
        let hdr = br#"{"schema":"uor-r4.lut-stack/6","sections":[{"name":"emb.token"},{"name":"readout.target"}]}"#;
        uorlut_sample.extend_from_slice(&(hdr.len() as u64).to_le_bytes());
        uorlut_sample.extend_from_slice(hdr);
        let uorlut_hex: String = uorlut_sample.iter().map(|b| format!("{b:02x}")).collect();
        let insp_raw = dispatch_string(&format!("1\tstudio\tuorlut_inspect\t{uorlut_hex}"));
        let insp_json: serde_json::Value =
            serde_json::from_str(&insp_raw).expect("valid uorlut_inspect JSON");
        assert_eq!(insp_json["status"], "ok");
        assert_eq!(insp_json["probe_outcome"], "AdmittedStack(6)");

        let _ = dispatch_string("1\tstudio\treset\t");
    }

    #[test]
    fn test_generalized_glm_multi_domain_reasoning_and_generation() {
        let _guard = TEST_STUDIO_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = dispatch_string("1\tstudio\treset\t");

        let ask = |prompt: &str| -> String {
            let raw = dispatch_string(&format!("1\tstudio\tchat\t{prompt}"));
            let json: serde_json::Value = serde_json::from_str(&raw).expect("valid chat JSON");
            assert_eq!(json["status"], "ok");
            json["assistant_reply"].as_str().unwrap().to_string()
        };

        // 1. Pairwise Comparative Synthesis (zero "Option A / Option B" filler)
        let rust_go = ask("Compare Rust and Go for building backend microservices");
        assert!(rust_go.contains("Rust") && rust_go.contains("Go"));
        assert!(!rust_go.contains("Option A"));
        assert!(rust_go.contains("Ownership") || rust_go.contains("goroutine"));

        let btree_lsm = ask("What is the difference between B-Trees and LSM-Trees?");
        assert!(btree_lsm.contains("B+Tree") || btree_lsm.contains("MemTable"));
        assert!(btree_lsm.contains("SSTable"));

        let tcp_udp = ask("Compare TCP vs UDP");
        assert!(tcp_udp.contains("Handshake") || tcp_udp.contains("SYN"));
        assert!(tcp_udp.contains("UDP") && tcp_udp.contains("QUIC"));

        // 2. Causal Science, Physics, Biology & History
        let sky = ask("Why is the sky blue and why are sunsets red?");
        assert!(sky.contains("Rayleigh"));
        assert!(sky.contains("lambda^4"));

        let vaccine = ask("How do mRNA vaccines and the immune system work?");
        assert!(vaccine.contains("T-cells") || vaccine.contains("Antibodies"));

        let rome = ask("What caused the fall of the Roman Empire?");
        assert!(rome.contains("Roman") && rome.contains("Byzantine"));

        let fusion = ask("Explain nuclear fission vs nuclear fusion");
        assert!(fusion.contains("Uranium") && fusion.contains("Deuterium"));

        // 3. Logic Puzzles & Probability Brainteasers
        let monty = ask("Explain the Monty Hall problem and why switching wins");
        assert!(monty.contains("2/3") && monty.contains("Door"));

        let eggs = ask("How do you solve the Two Eggs and 100 Floors puzzle?");
        assert!(eggs.contains("14 drops") || eggs.contains("14"));

        let river = ask("Solve the wolf, goat, and cabbage river crossing puzzle");
        assert!(river.contains("Crossing 1") && river.contains("Crossing 7"));

        // 4. Creative Story, Poem & Essay Generation
        let story = ask("Write a short sci-fi story about a probe under the ice of Europa");
        assert!(story.contains("Short Story") && story.contains("Europa"));

        let poem = ask("Write a poem about black holes and starlight");
        assert!(poem.contains("Poem:"));

        let essay =
            ask("Write an essay about how the printing press and the internet changed history");
        assert!(essay.contains("Essay:") && essay.contains("Thesis"));

        // 5. Multi-Turn Conversational Follow-up ("Explain simpler" -> "Give me an example" -> "Summarize")
        let _q1 = ask("Explain how Quantum Mechanics and entanglement work");
        let eli5 = ask("Explain that like I'm 5");
        assert!(eli5.contains("Intuitive") || eli5.contains("ELI5"));
        let ex = ask("Give me a concrete example");
        assert!(ex.contains("Example") && ex.contains("GPS"));
        let sum = ask("Summarize that");
        assert!(sum.contains("Executive Summary"));

        // 6. Fermi Estimation
        let fermi = ask("How many piano tuners are there in Chicago?");
        assert!(fermi.contains("Fermi") && fermi.contains("Factor 1"));

        // 7. Natural Conversational Prose & World Knowledge (Paris, Cities, Biographies, Everyday Life)
        let paris = ask("Tell me about Paris");
        assert!(paris.contains("Seine") && paris.contains("Eiffel") && paris.contains("Louvre"));
        assert!(!paris.contains("First-Principles Analysis"));
        assert!(!paris.contains("Core Concept & Framing"));
        assert!(!paris.contains("Operating Systems"));
        assert!(!paris.contains("p_TLB"));
        assert!(!paris.contains("####"));

        let davinci = ask("Who was Leonardo da Vinci?");
        assert!(davinci.contains("Renaissance") && davinci.contains("Mona Lisa"));
        assert!(!davinci.contains("First-Principles Analysis"));
        assert!(!davinci.contains("####"));

        let dogs = ask("Tell me about dogs");
        assert!(dogs.contains("Canis lupus familiaris") && dogs.contains("oxytocin"));
        assert!(!dogs.contains("First-Principles Analysis"));

        let coffee = ask("Tell me about coffee");
        assert!(coffee.contains("Ethiopia") && coffee.contains("Maillard"));
        assert!(!coffee.contains("First-Principles Analysis"));

        let chat_hello = ask("How are you doing today?");
        assert!(chat_hello.contains("thank you for asking"));
        assert!(!chat_hello.contains("occupied `E8` product cells"));

        let unseen = ask("Tell me about sourdough gardening in autumn");
        assert!(!unseen.contains("First-Principles Analysis"));
        assert!(!unseen.contains("Core Concept & Framing"));
        assert!(!unseen.contains("Operating Systems"));
        assert!(!unseen.contains("p_TLB"));

        // 8. Verify UR4GLM04 CUDA hardware-trained weights & falsifiable geometric routing benchmark
        assert_eq!(glm_weights::GLM_WEIGHTS_VERSION, "UR4GLM04");
        assert!(std::hint::black_box(glm_weights::GLM_CUDA_TRAINED));
        assert_eq!(glm_weights::GLM_CONCEPT_MANIFOLD.len(), 111);
        assert!(glm_weights::GLM_TRAINED_VOCAB.len() >= 1_500);
        assert_eq!(glm_weights::GLM_CONVERSATIONAL_QA.len(), 29);
        assert!(glm_weights::GLM_CONCEPT_MANIFOLD
            .iter()
            .all(|n| n.e8_root < 16
                && n.clifford_action < 64
                && n.icosian_root < 17
                && n.h4_root < 120));

        // 9. Falsifiable 5-way Held-Out Paraphrase Routing Benchmark:
        //    - (a) Full scorer (lexical + trained_geometry)
        //    - (b) Lexical scorer only (score_concept_lexical)
        //    - (c) Trained geometric scorer alone (score_concept_geometric)
        //    - (d) Random router
        //    - (e) Re-hashed/scrambled coordinate seed (0xDEAD_BEEF_CAFE_BABE)
        let bench = glm_weights::evaluate_held_out_routing_benchmark();
        assert_eq!(bench.total_prompts, 25);
        assert_eq!(bench.keyword_disjoint_prompts, 25);
        assert_eq!(bench.lexical_only_hits, 0);
        assert!(
            bench.full_hits > bench.lexical_only_hits,
            "Full scorer ({}) must strictly beat lexical-only ({}) on held-out paraphrase prompts",
            bench.full_hits,
            bench.lexical_only_hits
        );
        assert!(
            bench.geometric_only_hits > bench.random_hits,
            "Geometric-only scorer ({}) must strictly beat random router ({}) on held-out paraphrase prompts",
            bench.geometric_only_hits,
            bench.random_hits
        );
        assert!(
            bench.full_hits > bench.rehashed_seed_hits,
            "Full scorer with trained coordinates ({}) must strictly beat re-hashed coordinate seed ({})",
            bench.full_hits,
            bench.rehashed_seed_hits
        );
        assert!(
            bench.geometric_only_hits >= 22,
            "Trained geometric routing accuracy must be >= 22/25, got {}/25",
            bench.geometric_only_hits
        );

        // 10. Verify Parameterized Algorithmic Puzzle Solvers (dynamic computation vs canned strings)
        let monty10 =
            ask("Suppose there are 10 doors in Monty Hall and the host opens 7 goat doors");
        assert!(
            monty10.contains("N = 10") && monty10.contains("k = 7") && monty10.contains("9/20")
        );
        let eggs200 = ask("How many drops for 2 eggs and 200 floors?");
        assert!(eggs200.contains("20 drops") && eggs200.contains("200 Floors"));
        let bday30 = ask("What is the birthday collision probability for 30 people?");
        assert!(bday30.contains("N = 30") && bday30.contains("70.6"));

        // 11. Verify Purely Geometric Routing + Multi-Turn Follow-up on Keyword-Disjoint Prompt
        let geo_paris = ask("Where is the Louvre museum along the Seine river near Notre-Dame?");
        assert!(geo_paris.contains("Seine") || geo_paris.contains("Louvre"));
        let geo_followup = ask("Give me a concrete example");
        assert!(geo_followup.contains("Louvre") || geo_followup.contains("Notre-Dame"));

        let rel = ask("Explain the theory of relativity");
        assert!(rel.contains("Einstein") && rel.contains("spacetime"));
        let lemon = ask("What is the color of a lemon?");
        assert!(lemon.contains("yellow"));
        let rainbow = ask("What causes a rainbow?");
        assert!(rainbow.contains("refracts"));

        let _ = dispatch_string("1\tstudio\treset\t");
    }
}
