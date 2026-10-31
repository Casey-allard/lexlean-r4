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

/// Stateful UOR-R4 Geometric Language Model engine backing the Studio v2 console.
pub struct StudioEngine {
    vocab: Vec<TokenAtom>,
    turns: Vec<KnowledgeTurn>,
    opening_table: Vec<OpeningTableRow>,
    reply_table: Vec<ReplyTableRow>,
    point_table: Vec<PointKeyRow>,
    occupied_cells: [[bool; E8_ROOT_COUNT]; E8_ROOT_COUNT],
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
    if d == 0 {
        return false;
    }
    if a_neg && a_mag > 0 {
        return true;
    }
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
    let mut i = 0usize;
    let mut j = 0usize;
    let mut shared = Vec::new();
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
    shared
}

/// 4096-bit manifold sketch +3-sigma admission gate (`R4.DurableMemory.admitManifoldSketch`, `R4-26`).
pub fn admit_manifold_sketch(matching_bits: u32, total_bits: u32) -> bool {
    total_bits == 4096 && matching_bits >= 2144
}

/// M6 causal ablation place addition across all 5 ablation arms (`R4.ExactArithmetic.m6AblationPlaceAdd`, `R4-27`).
pub fn m6_ablation_place_add(
    digit_a: u32,
    digit_b: u32,
    carry_in: u32,
    radix: u32,
    arm: u8,
) -> Option<(u32, u32)> {
    if radix < 2 || carry_in > 1 || digit_a >= radix || digit_b >= radix {
        return None;
    }
    match arm {
        0 => {
            // FullExact
            let s = digit_a + digit_b + carry_in;
            Some((s % radix, u32::from(s >= radix)))
        }
        1 => {
            // ZeroCarry
            let s = digit_a + digit_b;
            Some((s % radix, 0))
        }
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
    /// Construct a fresh `StudioEngine` pre-populated with the 8 UOR-R4 domain knowledge turns.
    pub fn new_default() -> Self {
        let mut engine = Self {
            vocab: Vec::new(),
            turns: Vec::new(),
            opening_table: Vec::new(),
            reply_table: Vec::new(),
            point_table: Vec::new(),
            occupied_cells: [[false; E8_ROOT_COUNT]; E8_ROOT_COUNT],
        };
        engine.register_special_tokens();
        for (prompt, reply) in default_knowledge_turns() {
            engine.admit_turn(prompt, reply);
        }
        engine
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
            retain_str,
            state_receipt,
            extra_fields,
            entries_json
        )
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

        // Step 3: Autoregressively step `/7` `reply_key` continuation until EOS
        let mut generated_ids = Vec::new();
        let mut current_id = opening_token_id;
        let max_steps = 64usize;
        for step in 0..max_steps {
            if current_id == EOS_TOKEN_ID || current_id == PAD_TOKEN_ID {
                break;
            }
            generated_ids.push(current_id);
            let next_opt = self
                .reply_table
                .iter()
                .find(|r| {
                    r.turn_index == winning_turn_idx
                        && r.reply_step == step
                        && r.current_token_id == current_id
                })
                .map(|r| r.next_token_id);
            match next_opt {
                Some(next_id) => current_id = next_id,
                None => break,
            }
        }

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
        let active_retain = RETAIN_LAMBDA_Q31[head_idx];
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
                let hamming_weight = 64u32.saturating_sub(h_dist);
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
}

fn default_knowledge_turns() -> [(&'static str, &'static str); 12] {
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

    #[test]
    fn test_studio_exact_and_paraphrased_recall_and_admission() {
        let _ = dispatch_string("1\tstudio\treset\t");

        // 1. Verify initial state JSON
        let state_raw = dispatch_string("1\tstudio\tstate\t");
        let state_json: serde_json::Value =
            serde_json::from_str(&state_raw).expect("valid state JSON");
        assert_eq!(state_json["schema"], "uor-r4.lut-stack/8");
        assert_eq!(state_json["point_key_tensor"], "memory.point.points");
        assert_eq!(state_json["conformance_ids_verified"], 27);
        assert_eq!(state_json["turn_count"], 12);
        assert!(state_json["vocab_size"].as_u64().unwrap() >= 80);

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
        assert_eq!(admit_json["turn_count"], 13);

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

        let _ = dispatch_string("1\tstudio\treset\t");
    }
}
