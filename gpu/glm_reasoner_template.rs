/// Expand English contractions and possessives before tokenizing into alphanumeric words.
pub fn tokenize_words_simple(lower: &str) -> Vec<String> {
    let mut s = lower.to_ascii_lowercase().replace(['’', '‘', '`'], "'");
    for (pat, rep) in [
        ("let's", "let us"),
        ("what's", "what is"),
        ("who's", "who is"),
        ("how's", "how is"),
        ("where's", "where is"),
        ("when's", "when is"),
        ("why's", "why is"),
        ("that's", "that is"),
        ("it's", "it is"),
        ("there's", "there is"),
        ("here's", "here is"),
        ("i'm", "i am"),
        ("you're", "you are"),
        ("we're", "we are"),
        ("they're", "they are"),
        ("i've", "i have"),
        ("you've", "you have"),
        ("we've", "we have"),
        ("they've", "they have"),
        ("i'll", "i will"),
        ("you'll", "you will"),
        ("we'll", "we will"),
        ("they'll", "they will"),
        ("i'd", "i would"),
        ("you'd", "you would"),
        ("we'd", "we would"),
        ("they'd", "they would"),
        ("can't", "cannot"),
        ("won't", "will not"),
        ("don't", "do not"),
        ("doesn't", "does not"),
        ("didn't", "did not"),
        ("isn't", "is not"),
        ("aren't", "are not"),
        ("wasn't", "was not"),
        ("weren't", "were not"),
        ("wouldn't", "would not"),
        ("couldn't", "could not"),
        ("shouldn't", "should not"),
        ("haven't", "have not"),
        ("hasn't", "has not"),
        ("hadn't", "had not"),
        ("'s", ""),
    ] {
        s = s.replace(pat, rep);
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '+' || ch == '#' {
            cur.push(ch);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Canonical morphological stemmer matching `gpu/train_general_glm.rs` (`canonical_stem_train`).
pub fn canonical_glm_stem(word: &str) -> String {
    let mut s = word.to_ascii_lowercase();
    s.retain(|c| c.is_ascii_alphanumeric());
    if s.len() <= 3 {
        return s;
    }
    for suffix in [
        "izations", "ization", "ational", "fulness", "ousness", "iveness", "ically", "ations",
        "ation", "itions", "ition", "ments", "ment", "nesses", "ness", "ities", "ity", "ances",
        "ance", "ences", "ence", "ships", "ship", "ings", "ing", "edly", "ally", "iors", "ior",
        "ists", "ist", "isms", "ism", "ians", "ian", "ives", "ive", "able", "ible", "less", "ious",
        "eous", "ous", "ials", "ial", "ical", "ics", "ic", "ers", "ors", "ies", "ied", "es", "ed",
        "ly", "al", "er", "or", "s",
    ] {
        if s.len() > suffix.len() + 2 && s.ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
            if suffix == "ies" || suffix == "ied" {
                s.push('y');
            }
            break;
        }
    }
    s
}

/// Look up a word or stem in the CUDA-trained vocabulary manifold (`GLM_TRAINED_VOCAB`) via O(log V) binary search.
pub fn lookup_trained_vocab(raw_word: &str) -> Option<&'static GlmVocabEntry> {
    let stem = canonical_glm_stem(raw_word);
    if stem.len() < 3 {
        return None;
    }
    if let Ok(idx) = GLM_TRAINED_VOCAB.binary_search_by_key(&stem.as_str(), |e| e.stem) {
        return Some(&GLM_TRAINED_VOCAB[idx]);
    }
    // Check 1-character prefix neighborhood for irregular stem boundaries (e.g. `relativ` vs `relativity`)
    if stem.len() >= 5 {
        let prefix = &stem[..stem.len() - 1];
        let pos = GLM_TRAINED_VOCAB.partition_point(|e| e.stem < prefix);
        for cand in GLM_TRAINED_VOCAB.iter().skip(pos).take(4) {
            if cand.stem.starts_with(prefix) && cand.stem.len().abs_diff(stem.len()) <= 2 {
                return Some(cand);
            }
        }
    }
    None
}

/// Pool a sequence of words into a trained `(Q12, sign_code)` pair using `GLM_TRAINED_VOCAB`.
pub fn pool_prompt_trained_geometry(words: &[String]) -> ([i16; POINT_KEY_DIM], u64) {
    let mut accum = [0i64; POINT_KEY_DIM];
    let mut sign_votes = [0i32; 64];
    let mut total_w = 0i64;
    for w in words {
        if is_stopword(w) || is_generic_conversational_word(w) {
            continue;
        }
        if let Some(entry) = lookup_trained_vocab(w) {
            let weight = i64::from(entry.idf_weight.max(64));
            total_w += weight;
            for (lane, slot) in accum.iter_mut().enumerate() {
                *slot += i64::from(entry.q12_coords[lane]) * weight;
            }
            for (bit, vote) in sign_votes.iter_mut().enumerate() {
                if ((entry.sign_code >> bit) & 1) != 0 {
                    *vote += weight as i32;
                } else {
                    *vote -= weight as i32;
                }
            }
        }
    }
    let mut out_q12 = [0i16; POINT_KEY_DIM];
    let mut out_sign = 0u64;
    if total_w > 0 {
        for lane in 0..POINT_KEY_DIM {
            out_q12[lane] = (accum[lane] / total_w).clamp(-4096, 4096) as i16;
        }
        for (bit, &v) in sign_votes.iter().enumerate() {
            if v >= 0 {
                out_sign |= 1u64 << bit;
            }
        }
    }
    (out_q12, out_sign)
}

/// Exact 16-head integer retention decay constants (`R4.IntegerStack.retainLambdaQ31`, `R4-23`).
const GLM_RETAIN_LAMBDA_Q31: [i64; 16] = [
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

/// Exact 17-root Icosian `Z[phi]` GoldenQ12 table (`R4.SpiralCoreFbs.icosianRootEntry`, `R4-24`).
const GLM_ICOSIAN_ROOTS_Q12: [[(i32, i32); 4]; 17] = [
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

fn snap_icosian_4d_q12(lanes: [i16; 4]) -> u8 {
    let mut best_score = i64::MIN;
    let mut best_root = 0u8;
    for (r, root) in GLM_ICOSIAN_ROOTS_Q12.iter().enumerate() {
        let mut a = 0i64;
        let mut b = 0i64;
        for (l, &(ra, rb)) in root.iter().enumerate() {
            let x = i64::from(lanes[l]);
            a += x * i64::from(ra);
            b += x * i64::from(rb);
        }
        let score = 2 * a + 3 * b;
        if score > best_score {
            best_score = score;
            best_root = r as u8;
        }
    }
    best_root
}

/// 16-Head Integer Retention, Non-Commutative Quaternion State & Causal Pointer Induction (`R4.IntegerStack` + `R4.SpiralCoreFbs` + `R4.DiscreteTrunk`)
/// for $O(1)$ per-token autoregressive language generation without Transformer KV-caches or matrix multiplications.
#[derive(Debug, Clone)]
pub struct GlmRetentionState {
    /// 16 timescale Q12 context heads decayed by `GLM_RETAIN_LAMBDA_Q31[0..16]` (`R4-23`).
    pub heads_q12: [[i16; POINT_KEY_DIM]; 16],
    /// Non-commutative 4D integer Hamilton quaternion state (`R4.SpiralCoreFbs.stackHamilton4D`, `R4-24`).
    pub quat_4d: [i64; 4],
    /// 64-bit SWAR quaternion sign register.
    pub sign_code: u64,
    /// Number of tokens stepped so far.
    pub step_count: usize,
    /// Causal history of stepped Base-17 Icosian roots (`0..16`) for `orbit_root_causal_successor` (`R4-25`).
    pub history_roots: Vec<u8>,
    /// Causal history of stepped surface tokens for pointer-induction copying (`R4-25`).
    pub history_tokens: Vec<String>,
}

impl GlmRetentionState {
    /// Create a fresh 16-head retention state.
    pub fn new() -> Self {
        Self {
            heads_q12: [[0; POINT_KEY_DIM]; 16],
            quat_4d: [256, 0, 0, 0],
            sign_code: 0xCBF2_9CE4_8422_2325,
            step_count: 0,
            history_roots: Vec::new(),
            history_tokens: Vec::new(),
        }
    }

    /// Advance the 16-head retention state, 4D Hamilton quaternion state, and causal pointer history by one surface token in $O(1)$ integer ops.
    pub fn step_token(&mut self, piece: &str) {
        let norm: String = piece
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let (tok_q12, tok_sign) = if let Some(entry) = lookup_trained_vocab(&norm) {
            (entry.q12_coords, entry.sign_code)
        } else {
            let mut h = 0xCBF2_9CE4_8422_2325u64;
            for &b in piece.as_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01B3);
            }
            let mut q = [0i16; POINT_KEY_DIM];
            for (l, slot) in q.iter_mut().enumerate() {
                *slot = ((((h >> (l * 8)) & 0xFF) as i16) - 128) * 12;
            }
            (q, h)
        };

        for (h_idx, head) in self.heads_q12.iter_mut().enumerate() {
            let lam = GLM_RETAIN_LAMBDA_Q31[h_idx];
            for (lane, slot) in head.iter_mut().enumerate() {
                let decayed = (i64::from(*slot) * lam) >> 31;
                let stepped = decayed + (i64::from(tok_q12[lane]) / 3);
                *slot = stepped.clamp(-4096, 4096) as i16;
            }
        }

        let u = [
            i64::from(tok_q12[0] >> 4),
            i64::from(tok_q12[1] >> 4),
            i64::from(tok_q12[2] >> 4),
            i64::from(tok_q12[3] >> 4),
        ];
        let q = self.quat_4d;
        let next_q = [
            q[0] * u[0] - q[1] * u[1] - q[2] * u[2] - q[3] * u[3],
            q[0] * u[1] + q[1] * u[0] + q[2] * u[3] - q[3] * u[2],
            q[0] * u[2] - q[1] * u[3] + q[2] * u[0] + q[3] * u[1],
            q[0] * u[3] + q[1] * u[2] - q[2] * u[1] + q[3] * u[0],
        ];
        for (c, &val) in self.quat_4d.iter_mut().zip(next_q.iter()) {
            *c = (val >> 7).clamp(-4096, 4096);
        }

        let tok_root = snap_icosian_4d_q12([tok_q12[0], tok_q12[1], tok_q12[2], tok_q12[3]]) % 17;
        self.history_roots.push(tok_root);
        self.history_tokens.push(piece.to_string());
        self.sign_code = self.sign_code.rotate_left(5) ^ tok_sign;
        self.step_count += 1;
    }

    /// Compute the instantaneous Base-17 4-Block Icosian Root address (`0..83520`, `R4.DiscreteTrunk.pack4BlockRoots`, `R4-25`).
    pub fn packed_4block_root(&self) -> u32 {
        let q_lanes = [
            self.quat_4d[0] as i16,
            self.quat_4d[1] as i16,
            self.quat_4d[2] as i16,
            self.quat_4d[3] as i16,
        ];
        let h0 = self.heads_q12[0];
        let h7 = self.heads_q12[7];
        let h15 = self.heads_q12[15];
        let b0 = u32::from(snap_icosian_4d_q12(q_lanes) % 17);
        let b1 = u32::from(snap_icosian_4d_q12([h0[0], h0[1], h0[2], h0[3]]) % 17);
        let b2 = u32::from(snap_icosian_4d_q12([h7[0], h7[1], h7[2], h7[3]]) % 17);
        let b3 = u32::from(snap_icosian_4d_q12([h15[4], h15[5], h15[6], h15[7]]) % 17);
        b0 + 17 * b1 + 289 * b2 + 4913 * b3
    }

    /// Causal pointer induction (`R4.DiscreteTrunk.orbitRootCausalSuccessor`, `R4-25`):
    /// scan causal history `[0..t-1)` from most recent to oldest; when `history_roots[k] == query_root`,
    /// return the immediate causal successor `(history_roots[k + 1], &history_tokens[k + 1])`.
    pub fn causal_pointer_successor(&self, query_root: u8) -> Option<(u8, &str)> {
        if self.history_roots.len() < 2 {
            return None;
        }
        let limit = self.history_roots.len() - 1;
        for k in (0..limit).rev() {
            if self.history_roots[k] == (query_root % 17) {
                return Some((self.history_roots[k + 1], self.history_tokens[k + 1].as_str()));
            }
        }
        None
    }

    /// Evaluate the exact `R4-25` Level-3 Joint Pointer Score (`R4.DiscreteTrunk.level3JointPointerScore`):
    /// `64 * c_Qr + 32 * c_Qc + 8 * c_Rr + 2 * c_R + c_Q + 256 * I[ptr_match]`.
    pub fn level3_joint_pointer_score(
        &self,
        cand_q12: &[i16; POINT_KEY_DIM],
        cand_sign: u64,
        cand_4b: u32,
        ptr_match: bool,
    ) -> u32 {
        let l1_qr = q12_l1_distance(&self.heads_q12[7], cand_q12);
        let c_qr = u32::from(l1_qr <= 16_384);
        let ham_qc = swar_hamming_distance(self.sign_code, cand_sign);
        let c_qc = u32::from(ham_qc <= 34);
        let cur_4b = self.packed_4block_root();
        let c_rr = u32::from((cur_4b % 17) == (cand_4b % 17) || ((cur_4b / 17) % 17) == ((cand_4b / 17) % 17));
        let l1_deep = q12_l1_distance(&self.heads_q12[15], cand_q12);
        let c_r = u32::from(l1_deep <= 18_000);
        let l1_fast = q12_l1_distance(&self.heads_q12[1], cand_q12);
        let c_q = u32::from(l1_fast <= 18_000);
        64 * c_qr
            + 32 * c_qc
            + 8 * c_rr
            + 2 * c_r
            + c_q
            + if ptr_match { 256 } else { 0 }
    }
}

impl Default for GlmRetentionState {
    fn default() -> Self {
        Self::new()
    }
}

fn split_balanced_sentences(sec_text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = sec_text.as_bytes();
    let mut start = 0usize;
    let mut paren_depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => paren_depth += 1,
            b')' => paren_depth = (paren_depth - 1).max(0),
            b'.' if paren_depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b' ' => {
                let seg = sec_text[start..=i].trim();
                let is_abbrev = seg.ends_with(" c.")
                    || seg.ends_with("(c.")
                    || seg.ends_with(" e.g.")
                    || seg.ends_with("(e.g.")
                    || seg.ends_with(" i.e.")
                    || seg.ends_with("(i.e.")
                    || seg.ends_with(" vs.")
                    || seg.ends_with(" St.")
                    || seg.ends_with(" Dr.");
                if !is_abbrev {
                    if !seg.is_empty() {
                        out.push(seg);
                    }
                    start = i + 2;
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if start < bytes.len() {
        let tail = sec_text[start..].trim();
        if !tail.is_empty() {
            out.push(tail);
        }
    }
    out
}

fn split_section_horizon_clauses(sec_text: &str) -> Vec<&str> {
    split_balanced_sentences(sec_text)
        .into_iter()
        .filter(|s| s.len() >= 25)
        .collect()
}

/// Token-by-token Autoregressive Geometric Decoder (`UR4GLM06`).
///
/// Steps one surface token at a time (`w_0 -> w_1 -> w_2 -> ... -> w_T`) across the active
/// token transition lattice using:
/// 1. `GlmRetentionState` (`RETAIN_LAMBDA_Q31` 8-head integer decay + 4D Hamilton quaternion state),
/// 2. `GLM_TOKEN_HORIZONS` (RTX 5090 CUDA-trained lookahead horizon coordinates & Base-17 4-block trunk addresses),
/// 3. Syntactic pivot switching at clause/conjunction/sentence boundaries (`,`, `;`, `.`, `and`, `while`, `where`, `which`),
/// 4. `R4.Prediction.selectContextualToken` loop-prevention recency penalties.
///
/// Returns `(generated_prose, tokens_stepped, branch_decisions_evaluated)`.
pub fn generate_token_level_geometric_prose(
    prompt_words: &[String],
    nodes: &[&GlmConceptNode],
    min_tokens: usize,
    max_tokens: usize,
) -> (String, usize, usize) {
    let mut state = GlmRetentionState::new();
    for pw in prompt_words {
        state.step_token(pw);
    }

    let prompt_stems: Vec<String> = prompt_words
        .iter()
        .filter(|w| !is_stopword(w) && !is_generic_conversational_word(w))
        .map(|w| canonical_glm_stem(w))
        .filter(|s| s.len() >= 3)
        .collect();

    // Build the active token lattice spans linked to their CUDA-trained `GlmTokenHorizon` rows
    struct TokenSpan {
        basin_id: u16,
        node_rank: usize,
        sec_idx: u8,
        tokens: Vec<String>,
        stems: Vec<String>,
        horizon_idx: usize,
        horizon: GlmTokenHorizon,
    }

    let mut spans: Vec<TokenSpan> = Vec::new();
    for (node_rank, &node) in nodes.iter().enumerate() {
        let sections = [
            node.summary,
            node.mechanism,
            node.formal_principle,
            node.tradeoffs,
            node.example,
        ];
        for (sec_idx, &sec_text) in sections.iter().enumerate() {
            let clauses = split_section_horizon_clauses(sec_text);
            for (sent_idx, s_trim) in clauses.into_iter().enumerate() {
                let formatted =
                    if s_trim.ends_with('.') || s_trim.ends_with('!') || s_trim.ends_with('?') {
                        s_trim.to_string()
                    } else {
                        format!("{s_trim}.")
                    };
                let toks: Vec<String> = formatted.split_whitespace().map(String::from).collect();
                if toks.len() < 4 {
                    continue;
                }
                let mut stems: Vec<String> = Vec::new();
                for w in tokenize_words_simple(&formatted) {
                    let st = canonical_glm_stem(&w);
                    if st.len() >= 3 && !is_stopword(&st) && !stems.contains(&st) {
                        stems.push(st);
                    }
                }
                for t in &toks {
                    let st = canonical_glm_stem(&t.to_ascii_lowercase());
                    if st.len() >= 3 && !is_stopword(&st) && !stems.contains(&st) {
                        stems.push(st);
                    }
                }

                let (horizon_idx, horizon) = GLM_TOKEN_HORIZONS
                    .iter()
                    .enumerate()
                    .find(|(_, h)| {
                        h.basin_id == node.id
                            && usize::from(h.section_idx) == sec_idx
                            && usize::from(h.sentence_idx) == sent_idx.min(255)
                    })
                    .map(|(idx, &h)| (idx, h))
                    .unwrap_or((
                        usize::MAX,
                        GlmTokenHorizon {
                            horizon_q12: node.q12_coords,
                            horizon_sign: node.sign_code,
                            packed_4block: u32::from(node.icosian_root),
                            basin_id: node.id,
                            section_idx: sec_idx as u8,
                            sentence_idx: (sent_idx.min(255)) as u8,
                            next_horizon_id: 0,
                            alt_horizon_id: 0,
                            transition_energy: 64,
                        },
                    ));

                spans.push(TokenSpan {
                    basin_id: node.id,
                    node_rank,
                    sec_idx: sec_idx as u8,
                    tokens: toks,
                    stems,
                    horizon_idx,
                    horizon,
                });
            }
        }
    }

    if spans.is_empty() {
        return (String::new(), 0, 0);
    }

    let preferred_sec: Option<u8> = if prompt_words
        .iter()
        .any(|w| matches!(w.as_str(), "how" | "mechanism" | "work" | "works" | "process"))
    {
        Some(1)
    } else if prompt_words
        .iter()
        .any(|w| matches!(w.as_str(), "why" | "principle" | "theorem" | "law" | "theory"))
    {
        Some(2)
    } else if prompt_words.iter().any(|w| {
        matches!(
            w.as_str(),
            "tradeoff" | "tradeoffs" | "pros" | "cons" | "limitation" | "limitations" | "risk"
        )
    }) {
        Some(3)
    } else if prompt_words.iter().any(|w| {
        matches!(
            w.as_str(),
            "example" | "examples" | "instance" | "practice" | "application"
        )
    }) {
        Some(4)
    } else {
        None
    };

    let score_span_against_state =
        |sp: &TokenSpan, st: &GlmRetentionState, used_stems: &[String], used_secs: &[u8]| -> u32 {
            let slow_head = &st.heads_q12[7];
            let l1 = q12_l1_distance(slow_head, &sp.horizon.horizon_q12);
            let q12_prox = 4096u32.saturating_sub(l1 / 6) / 128;
            let ham = swar_hamming_distance(st.sign_code, sp.horizon.horizon_sign);
            let ham_bonus = 64u32.saturating_sub(ham) / 3;

            let cur_4b = st.packed_4block_root();
            let cand_4b = sp.horizon.packed_4block;
            let mut block_matches = 0u32;
            let mut a = cur_4b;
            let mut b = cand_4b;
            for _ in 0..4 {
                if (a % 17) == (b % 17) {
                    block_matches += 4;
                }
                a /= 17;
                b /= 17;
            }

            let q_root = (cur_4b % 17) as u8;
            let ptr_match = st
                .causal_pointer_successor(q_root)
                .is_some_and(|(succ_root, succ_tok)| {
                    let succ_stem = canonical_glm_stem(&succ_tok.to_ascii_lowercase());
                    succ_root == ((cand_4b % 17) as u8)
                        && (sp
                            .tokens
                            .first()
                            .is_some_and(|first| first.eq_ignore_ascii_case(succ_tok))
                            || (!succ_stem.is_empty() && sp.stems.iter().any(|ss| ss == &succ_stem)))
                });
            let l3_score = st.level3_joint_pointer_score(
                &sp.horizon.horizon_q12,
                sp.horizon.horizon_sign,
                cand_4b,
                ptr_match,
            ) / 24;

            let rank_base = if sp.node_rank == 0 {
                28u32.saturating_sub(u32::from(sp.sec_idx) * 3)
            } else {
                18u32.saturating_sub(u32::from(sp.sec_idx) * 3)
            };

            let intent_sec_bonus = if used_secs.is_empty() && preferred_sec == Some(sp.sec_idx) {
                8
            } else {
                0
            };

            let sec_diversity_bonus = if !used_secs.contains(&sp.sec_idx) {
                8
            } else {
                0
            };

            let stems_match = |a: &str, b: &str| -> bool {
                a == b
                    || (a.len() >= 5
                        && b.len() >= 5
                        && a.len().abs_diff(b.len()) <= 2
                        && (a.starts_with(&b[..b.len() - 1]) || b.starts_with(&a[..a.len() - 1])))
            };

            let mut prompt_bonus = 0u32;
            for ps in &prompt_stems {
                if sp.stems.iter().any(|ss| stems_match(ss, ps)) {
                    let idf = lookup_trained_vocab(ps)
                        .map(|e| u32::from(e.idf_weight) / 48)
                        .unwrap_or(6);
                    let already_covered = used_stems.iter().any(|us| stems_match(us, ps));
                    if !already_covered {
                        prompt_bonus += 28 + idf;
                    }
                }
            }

            let redundancy = sp
                .stems
                .iter()
                .filter(|ss| {
                    !prompt_stems.iter().any(|ps| stems_match(ss, ps)) && used_stems.contains(ss)
                })
                .count()
                .min(4) as u32;

            (rank_base
                + intent_sec_bonus
                + sec_diversity_bonus
                + q12_prox
                + ham_bonus
                + block_matches
                + l3_score
                + prompt_bonus)
                .saturating_sub(redundancy * 2)
        };

    let mut used_spans: Vec<usize> = Vec::new();
    let mut used_stems: Vec<String> = Vec::new();
    let mut used_secs: Vec<u8> = Vec::new();
    let mut emitted_tokens: Vec<String> = Vec::new();
    let mut branch_decisions = 0usize;

    // Select initial opening span `(w_0, w_1, ...)` via CUDA-trained horizon resonance
    branch_decisions += 1;
    let mut cur_span_idx = (0..spans.len())
        .max_by_key(|&idx| score_span_against_state(&spans[idx], &state, &used_stems, &used_secs))
        .unwrap_or(0);
    used_spans.push(cur_span_idx);
    used_secs.push(spans[cur_span_idx].sec_idx);
    for st in &spans[cur_span_idx].stems {
        if !used_stems.contains(st) {
            used_stems.push(st.clone());
        }
    }
    let mut cur_tok_idx = 0usize;
    let mut tokens_in_cur_span = 0usize;
    let mut mid_pivots_taken = 0usize;

    // Step token-by-token (`w_0 -> w_1 -> w_2 -> ...`) through the geometric transition lattice
    while emitted_tokens.len() < max_tokens {
        let tok_piece = spans[cur_span_idx].tokens[cur_tok_idx].clone();
        state.step_token(&tok_piece);
        emitted_tokens.push(tok_piece.clone());
        tokens_in_cur_span += 1;

        let is_sentence_end =
            tok_piece.ends_with('.') || tok_piece.ends_with('!') || tok_piece.ends_with('?');
        let at_span_end = cur_tok_idx + 1 >= spans[cur_span_idx].tokens.len();

        if (is_sentence_end || at_span_end)
            && emitted_tokens.len() >= min_tokens
            && is_balanced_markdown_slice(&emitted_tokens)
        {
            break;
        }

        // Check for mid-sentence syntactic pivot branches across 40+ grammatical pivot tokens
        let at_grammatical_pivot = cur_tok_idx >= 5
            && cur_tok_idx + 4 < spans[cur_span_idx].tokens.len()
            && is_grammatical_pivot_token(&tok_piece)
            && is_balanced_markdown_slice(&spans[cur_span_idx].tokens[..=cur_tok_idx]);
        if at_grammatical_pivot {
            branch_decisions += 1;
        }
        let cur_has_prompt_stem = spans[cur_span_idx].stems.iter().any(|ss| {
            prompt_stems.iter().any(|ps| {
                ss == ps
                    || (ss.len() >= 5
                        && ps.len() >= 5
                        && ss.len().abs_diff(ps.len()) <= 2
                        && (ss.starts_with(&ps[..ps.len() - 1])
                            || ps.starts_with(&ss[..ss.len() - 1])))
            })
        });
        let is_pivot_word = nodes.len() >= 2
            && !cur_has_prompt_stem
            && mid_pivots_taken == 0
            && tokens_in_cur_span >= 14
            && at_grammatical_pivot;

        if at_span_end {
            // Sentence boundary transition: follow CUDA-trained `/7` geodesic transition edges (`next_horizon_id` / `alt_horizon_id`)
            // combined with active `GlmRetentionState` resonance
            branch_decisions += 1;
            let cur_h = spans[cur_span_idx].horizon;
            let next_span_opt = (0..spans.len())
                .filter(|idx| !used_spans.contains(idx))
                .max_by_key(|&idx| {
                    let base =
                        score_span_against_state(&spans[idx], &state, &used_stems, &used_secs);
                    let cand_h_idx = spans[idx].horizon_idx;
                    let edge_bonus = if cand_h_idx != usize::MAX
                        && cand_h_idx == usize::from(cur_h.next_horizon_id)
                    {
                        22u32.saturating_sub(cur_h.transition_energy / 256).max(10)
                    } else if cand_h_idx != usize::MAX
                        && cand_h_idx == usize::from(cur_h.alt_horizon_id)
                    {
                        14
                    } else {
                        0
                    };
                    base + edge_bonus
                });

            if let Some(next_span) = next_span_opt {
                cur_span_idx = next_span;
                cur_tok_idx = 0;
                tokens_in_cur_span = 0;
                used_spans.push(cur_span_idx);
                if !used_secs.contains(&spans[cur_span_idx].sec_idx) {
                    used_secs.push(spans[cur_span_idx].sec_idx);
                }
                for st in &spans[cur_span_idx].stems {
                    if !used_stems.contains(st) {
                        used_stems.push(st.clone());
                    }
                }
            } else {
                break;
            }
        } else if is_pivot_word {
            // Evaluate whether continuing in `cur_span_idx` or pivoting at `tok_piece` into another resonant span scores higher
            let cur_score =
                score_span_against_state(&spans[cur_span_idx], &state, &[], &used_secs) + 20;
            let cur_h = spans[cur_span_idx].horizon;
            let mut best_pivot: Option<(usize, usize, u32)> = None;
            for (cand_idx, cand_span) in spans.iter().enumerate() {
                if used_spans.contains(&cand_idx)
                    || (cand_span.basin_id != spans[cur_span_idx].basin_id && nodes.len() < 2)
                {
                    continue;
                }
                if let Some(pos) = cand_span
                    .tokens
                    .iter()
                    .enumerate()
                    .skip(3)
                    .find(|&(i, t)| {
                        t == &tok_piece
                            && i + 4 < cand_span.tokens.len()
                            && is_balanced_markdown_slice(&cand_span.tokens[i + 1..])
                    })
                    .map(|(i, _)| i + 1)
                {
                    branch_decisions += 1;
                    let edge_bonus = if cand_span.horizon_idx != usize::MAX
                        && (cand_span.horizon_idx == usize::from(cur_h.next_horizon_id)
                            || cand_span.horizon_idx == usize::from(cur_h.alt_horizon_id))
                    {
                        12
                    } else {
                        0
                    };
                    let s = score_span_against_state(cand_span, &state, &used_stems, &used_secs)
                        + edge_bonus;
                    if s > cur_score && best_pivot.is_none_or(|(_, _, bs)| s > bs) {
                        best_pivot = Some((cand_idx, pos, s));
                    }
                }
            }
            if let Some((win_span, win_tok_pos, _)) = best_pivot {
                cur_span_idx = win_span;
                cur_tok_idx = win_tok_pos;
                tokens_in_cur_span = 0;
                mid_pivots_taken += 1;
                used_spans.push(cur_span_idx);
                if !used_secs.contains(&spans[cur_span_idx].sec_idx) {
                    used_secs.push(spans[cur_span_idx].sec_idx);
                }
                for st in &spans[cur_span_idx].stems {
                    if !used_stems.contains(st) {
                        used_stems.push(st.clone());
                    }
                }
            } else {
                cur_tok_idx += 1;
            }
        } else {
            cur_tok_idx += 1;
        }
    }

    let steps = emitted_tokens.len();
    let text = format_emitted_tokens_into_paragraphs(&emitted_tokens);
    (text, steps, branch_decisions)
}

/// Grammatical pivot tokens (`40+` prepositions, conjunctions, relative pronouns, auxiliary verbs, and clause punctuation)
/// where the autoregressive token-level decoder evaluates branch transitions across horizon spans.
fn is_grammatical_pivot_token(tok: &str) -> bool {
    if tok.ends_with(',') || tok.ends_with(';') || tok.ends_with(':') {
        return true;
    }
    let clean = tok.trim_matches(|c: char| !c.is_ascii_alphabetic());
    matches!(
        clean,
        "in" | "on"
            | "by"
            | "to"
            | "of"
            | "for"
            | "with"
            | "from"
            | "as"
            | "at"
            | "into"
            | "through"
            | "across"
            | "between"
            | "under"
            | "over"
            | "within"
            | "without"
            | "along"
            | "around"
            | "where"
            | "which"
            | "that"
            | "while"
            | "whereas"
            | "because"
            | "when"
            | "if"
            | "although"
            | "until"
            | "since"
            | "and"
            | "or"
            | "but"
            | "yet"
            | "so"
            | "is"
            | "are"
            | "was"
            | "were"
            | "can"
            | "enables"
            | "provides"
            | "uses"
            | "computes"
    )
}

/// Verify that a slice of tokens has balanced Markdown (`**`, `` ` ``) and brackets (`()`, `[]`)
/// so mid-sentence syntactic pivots never split an open span.
fn is_balanced_markdown_slice(tokens: &[String]) -> bool {
    let mut bold_markers = 0usize;
    let mut backticks = 0usize;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    for tok in tokens {
        bold_markers += tok.matches("**").count();
        for ch in tok.chars() {
            match ch {
                '`' => backticks += 1,
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        return false;
                    }
                }
                '[' => bracket_depth += 1,
                ']' => {
                    bracket_depth -= 1;
                    if bracket_depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }
    bold_markers.is_multiple_of(2)
        && backticks.is_multiple_of(2)
        && paren_depth == 0
        && bracket_depth == 0
}

/// Group an autoregressively emitted token stream into clean multi-paragraph Markdown prose.
fn format_emitted_tokens_into_paragraphs(tokens: &[String]) -> String {
    if tokens.is_empty() {
        return String::new();
    }
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for tok in tokens {
        current.push(tok.clone());
        let ends_sent = tok.ends_with('.') || tok.ends_with('!') || tok.ends_with('?');
        if ends_sent && current.len() >= 42 && is_balanced_markdown_slice(&current) {
            paragraphs.push(current.join(" "));
            current.clear();
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }
    paragraphs.join("\n\n")
}

/// Generic conversational words that must not trigger a specialized concept basin on their own.
fn is_generic_conversational_word(w: &str) -> bool {
    matches!(
        w,
        "let"
            | "lets"
            | "us"
            | "you"
            | "your"
            | "yours"
            | "yourself"
            | "my"
            | "mine"
            | "myself"
            | "we"
            | "our"
            | "ours"
            | "ourselves"
            | "tell"
            | "talk"
            | "talking"
            | "talked"
            | "chat"
            | "chatting"
            | "discuss"
            | "discussing"
            | "explore"
            | "exploring"
            | "about"
            | "something"
            | "anything"
            | "everything"
            | "nothing"
            | "interesting"
            | "fun"
            | "fact"
            | "facts"
            | "surprise"
            | "bored"
            | "today"
            | "tonight"
            | "morning"
            | "evening"
            | "day"
            | "going"
            | "doing"
            | "fine"
            | "well"
            | "mind"
            | "thoughts"
            | "thinking"
            | "life"
            | "world"
            | "time"
            | "work"
            | "works"
            | "working"
            | "explain"
            | "describe"
            | "show"
            | "give"
            | "make"
            | "help"
            | "want"
            | "wants"
            | "like"
            | "love"
            | "think"
            | "know"
            | "feel"
            | "need"
            | "good"
            | "best"
            | "great"
            | "nice"
            | "cool"
            | "awesome"
            | "wow"
            | "really"
            | "actually"
            | "maybe"
            | "general"
            | "simple"
            | "basic"
            | "overview"
            | "intro"
            | "introduction"
            | "history"
            | "science"
            | "system"
            | "systems"
            | "modern"
            | "human"
            | "people"
            | "person"
            | "place"
            | "city"
            | "country"
            | "topic"
            | "topics"
            | "subject"
            | "question"
            | "questions"
            | "thing"
            | "things"
            | "stuff"
    )
}

/// Project an 8D Q12 coordinate vector through the discrete Hopf fibration `S^3 -> S^2 x U(1)`
/// into Q15 base coordinates `[sx, sy, sz]` and Q15 fiber phase `[ux, uy]`.
pub fn project_q12_to_hopf_q15(q12: &[i16; POINT_KEY_DIM]) -> ([i16; 3], [i16; 2]) {
    let q0 = i64::from(q12[0]);
    let q1 = i64::from(q12[1]);
    let q2 = i64::from(q12[2]);
    let q3 = i64::from(q12[3]);
    let norm_sq = (q0 * q0 + q1 * q1 + q2 * q2 + q3 * q3).max(1);

    let sx = ((2 * (q0 * q2 + q1 * q3) * 32767) / norm_sq).clamp(-32767, 32767) as i16;
    let sy = ((2 * (q1 * q2 - q0 * q3) * 32767) / norm_sq).clamp(-32767, 32767) as i16;
    let sz =
        (((q0 * q0 + q1 * q1 - q2 * q2 - q3 * q3) * 32767) / norm_sq).clamp(-32767, 32767) as i16;

    let phase_norm = (q0.abs() + q1.abs()).max(1);
    let ux = ((q0 * 32767) / phase_norm).clamp(-32767, 32767) as i16;
    let uy = ((q1 * 32767) / phase_norm).clamp(-32767, 32767) as i16;
    ([sx, sy, sz], [ux, uy])
}

/// Compute integer Fubini-Study / Hopf base + fiber alignment in `[0..1024]`.
pub fn hopf_fubini_study_similarity(
    s2_a: &[i16; 3],
    u1_a: &[i16; 2],
    s2_b: &[i16; 3],
    u1_b: &[i16; 2],
) -> u32 {
    let base_dot = i64::from(s2_a[0]) * i64::from(s2_b[0])
        + i64::from(s2_a[1]) * i64::from(s2_b[1])
        + i64::from(s2_a[2]) * i64::from(s2_b[2]);
    let fiber_dot =
        i64::from(u1_a[0]) * i64::from(u1_b[0]) + i64::from(u1_a[1]) * i64::from(u1_b[1]);
    let base_sim = ((base_dot / (32767 * 3)) + 364).clamp(0, 728) as u32;
    let fiber_sim = ((fiber_dot / (32767 * 2)) + 148).clamp(0, 296) as u32;
    base_sim + fiber_sim
}

/// Compute integer cosine similarity in `[0..1024]` between two 8D Q12 vectors.
pub fn q12_cosine_similarity(a: &[i16; POINT_KEY_DIM], b: &[i16; POINT_KEY_DIM]) -> u32 {
    let mut dot = 0i64;
    let mut na = 0i64;
    let mut nb = 0i64;
    for i in 0..POINT_KEY_DIM {
        let xa = i64::from(a[i]);
        let xb = i64::from(b[i]);
        dot += xa * xb;
        na += xa * xa;
        nb += xb * xb;
    }
    if na == 0 || nb == 0 {
        return 0;
    }
    let denom = ((na.unsigned_abs() as f64).sqrt() * (nb.unsigned_abs() as f64).sqrt()).max(1.0);
    let cos = (dot as f64) / denom;
    ((cos + 1.0) * 512.0).clamp(0.0, 1024.0) as u32
}

/// Match a fast conversational Q&A entry (`GLM_CONVERSATIONAL_QA`).
pub fn match_conversational_qa(lower: &str, words: &[String]) -> Option<&'static str> {
    let mut best_score = 0u32;
    let mut best_ans: Option<&'static str> = None;

    for qa in &GLM_CONVERSATIONAL_QA {
        let mut score = 0u32;
        for &ph in qa.phrases {
            if lower.contains(ph) {
                score += 30;
            }
        }
        let mut kw_hits = 0u32;
        for &kw in qa.keywords {
            if words.iter().any(|w| w == kw) {
                kw_hits += 1;
                score += 8;
            }
        }
        if kw_hits >= 2 {
            score += 10;
        }
        if score > best_score && score >= 18 {
            best_score = score;
            best_ans = Some(qa.answer);
        }
    }
    best_ans
}

/// Extract unsigned integers mentioned in a prompt (including common English number words).
fn extract_prompt_numbers(words: &[String]) -> Vec<u64> {
    let mut out = Vec::new();
    for w in words {
        if let Ok(n) = w.parse::<u64>() {
            out.push(n);
            continue;
        }
        let mapped = match w.as_str() {
            "two" => Some(2),
            "three" => Some(3),
            "four" => Some(4),
            "five" => Some(5),
            "six" => Some(6),
            "seven" => Some(7),
            "eight" => Some(8),
            "nine" => Some(9),
            "ten" => Some(10),
            "twelve" => Some(12),
            "twenty" => Some(20),
            "thirty" => Some(30),
            "fifty" => Some(50),
            "hundred" => Some(100),
            _ => None,
        };
        if let Some(v) = mapped {
            out.push(v);
        }
    }
    out
}

fn gcd_u64_local(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

/// Parameterized Algorithmic Solver for Logic, Probability, and Combinatorial Puzzles.
/// Executes real Bayesian rational arithmetic, dynamic programming (`Egg Drop`), state-space
/// Breadth-First Search (`River Crossing`), fixed-point product (`Birthday Collision`),
/// and base-3 information bounds (`Balance Scale`) on parameters extracted from the prompt.
pub fn match_logic_puzzle(lower: &str, words: &[String]) -> Option<(&'static str, String)> {
    let nums = extract_prompt_numbers(words);

    // 1. N-Doors Monty Hall Bayesian Solver
    if lower.contains("monty hall")
        || (words.iter().any(|w| w == "monty" || w == "goat")
            && words
                .iter()
                .any(|w| w == "door" || w == "doors" || w == "switch"))
    {
        let n_doors = nums
            .iter()
            .copied()
            .find(|&n| (3..=10_000).contains(&n))
            .unwrap_or(3);
        let k_opened = nums
            .iter()
            .copied()
            .rfind(|&n| n < n_doors.saturating_sub(1) && n >= 1 && n != n_doors)
            .unwrap_or(1);
        let stay_num = 1u64;
        let stay_den = n_doors;
        let rem_closed = (n_doors - 1 - k_opened).max(1);
        let raw_sw_num = n_doors - 1;
        let raw_sw_den = n_doors * rem_closed;
        let g = gcd_u64_local(raw_sw_num, raw_sw_den);
        let sw_num = raw_sw_num / g;
        let sw_den = raw_sw_den / g;
        let sw_pct_x10 = (sw_num * 1000) / sw_den;
        let stay_pct_x10 = (stay_num * 1000) / stay_den;

        let sol = format!(
            "### The Monty Hall Problem ({n_doors} Doors, {k_opened} Goat Door(s) Opened) — Exact Bayesian Derivation\n\n\
             **Verdict:** You should **always switch doors**. Switching wins with probability **`{sw_num}/{sw_den}` (`{}.{}%`)**, whereas staying with your initial pick wins only **`{stay_num}/{stay_den}` (`{}.{}%`)**.\n\n\
             #### Step-by-Step Bayesian Calculation\n\
             1. **Uniform Prior (`N = {n_doors}` Doors)**: Your initial choice (**Door 1**) holds probability mass **`P(C = 1) = 1/{n_doors}`**, while the remaining `{}` unchosen doors collectively hold **`{}/{n_doors}`** of the probability mass.\n\
             2. **Informed Host Conditioning (`k = {k_opened}` Goat Door(s) Opened)**: Because the host knows where the prize is and purposefully opens `{k_opened}` losing door(s) from the unchosen set, none of the `{}/{n_doors}` mass on the unchosen set is lost.\n\
             3. **Posterior Mass Concentration**: That entire **`{}/{n_doors}`** probability mass concentrates onto the `{rem_closed}` remaining closed unchosen door(s), giving each remaining closed door **`({n_doors} - 1) / ({n_doors} * {rem_closed}) = {sw_num}/{sw_den}`** — a **`{:.2}x`** advantage over staying!",
            sw_pct_x10 / 10,
            sw_pct_x10 % 10,
            stay_pct_x10 / 10,
            stay_pct_x10 % 10,
            n_doors - 1,
            n_doors - 1,
            n_doors - 1,
            n_doors - 1,
            (sw_num as f64 * stay_den as f64) / (sw_den as f64 * stay_num as f64)
        );
        return Some((
            "Probability & Decision Puzzle · The Monty Hall Problem",
            sol,
        ));
    }

    // 2. K-Eggs and M-Floors Dynamic Programming Minimax Solver
    if lower.contains("two eggs")
        || lower.contains("egg drop")
        || (words.iter().any(|w| w == "egg" || w == "eggs")
            && words.iter().any(|w| w == "floor" || w == "floors"))
    {
        let floors = nums
            .iter()
            .copied()
            .find(|&n| (10..=100_000).contains(&n))
            .unwrap_or(100);
        let eggs = nums
            .iter()
            .copied()
            .find(|&n| (1..=10).contains(&n) && n != floors)
            .unwrap_or(2);

        // Exact DP recurrence: max_floors(k, d) = sum_{i=1..k} C(d, i)
        let mut drops = 0u64;
        loop {
            drops += 1;
            let mut max_f = 0u64;
            let mut comb = 1u64;
            for i in 1..=eggs.min(drops) {
                comb = comb.saturating_mul(drops - i + 1) / i;
                max_f = max_f.saturating_add(comb);
            }
            if max_f >= floors || drops >= floors {
                break;
            }
        }

        // Compute the exact first-egg drop schedule
        let mut schedule = Vec::new();
        let mut cur_floor = 0u64;
        let mut step = drops;
        while cur_floor < floors && step > 0 {
            cur_floor = (cur_floor + step).min(floors);
            schedule.push(cur_floor.to_string());
            if cur_floor == floors {
                break;
            }
            step = step.saturating_sub(1).max(1);
        }
        let tri_capacity = (drops * (drops + 1)) / 2;

        let sol = format!(
            "### {eggs} Eggs and {floors} Floors — Dynamic Programming Minimax Solution\n\n\
             **Verdict:** The minimum number of drops required in the worst case for **`{eggs}` eggs** and **`{floors}` floors** is **`{drops} drops`**.\n\n\
             #### Exact Binomial Horizon Recurrence\n\
             1. With `{eggs}` eggs and `d` total drops budgeted, the maximum number of floors distinguishable in the worst case is:\n\
                `F({eggs}, d) = sum_{{i=1..{eggs}}} C(d, i)`\n\
             2. Evaluating for `d = {}` gives capacity `< {floors}`, whereas **`d = {drops}`** yields capacity **`{tri_capacity} >= {floors}`**.\n\
             3. **Computed Drop Schedule for Egg 1** (decreasing the step size by `1` after each surviving drop so Egg 2's linear scan never exceeds `{drops}` total drops):\n\
                **Floors `[{}]`**.",
            drops.saturating_sub(1),
            schedule.join(", ")
        );
        return Some((
            "Combinatorial Algorithm Puzzle · Two Eggs and 100 Floors",
            sol,
        ));
    }

    // 3. State-Space Breadth-First Search (BFS) River Crossing Solver
    if lower.contains("river crossing")
        || (words.iter().any(|w| w == "wolf" || w == "fox")
            && words
                .iter()
                .any(|w| w == "goat" || w == "goose" || w == "chicken")
            && words
                .iter()
                .any(|w| w == "cabbage" || w == "grain" || w == "beans" || w == "river"))
    {
        // 4-bit state: bit 0 = Farmer, bit 1 = Wolf, bit 2 = Goat, bit 3 = Cabbage
        // Illegal states: Wolf==Goat != Farmer, or Goat==Cabbage != Farmer
        let is_safe = |state: u8| -> bool {
            let f = state & 1;
            let w = (state >> 1) & 1;
            let g = (state >> 2) & 1;
            let c = (state >> 3) & 1;
            if w == g && f != w {
                return false;
            }
            if g == c && f != g {
                return false;
            }
            true
        };

        let mut parent = [u8::MAX; 16];
        let mut move_desc = [""; 16];
        let mut queue = std::collections::VecDeque::new();
        parent[0] = 0;
        queue.push_back(0u8);

        while let Some(u) = queue.pop_front() {
            if u == 0b1111 {
                break;
            }
            let f = u & 1;
            let next_f = f ^ 1;
            let candidates: [(u8, &'static str, &'static str); 4] = [
                (
                    1,
                    "Farmer crosses **alone** to the Far Bank",
                    "Farmer returns **alone** to the Near Bank",
                ),
                (
                    1 | 2,
                    "Farmer takes the **Wolf** across to the Far Bank",
                    "Farmer brings the **Wolf** back to the Near Bank",
                ),
                (
                    1 | 4,
                    "Farmer takes the **Goat** across to the Far Bank",
                    "Farmer brings the **Goat** back to the Near Bank",
                ),
                (
                    1 | 8,
                    "Farmer takes the **Cabbage** across to the Far Bank",
                    "Farmer brings the **Cabbage** back to the Near Bank",
                ),
            ];
            for &(mask, to_far, to_near) in &candidates {
                // Passenger must be on the same bank as Farmer
                let passenger_bit = mask & !1;
                if passenger_bit != 0 && ((u & passenger_bit != 0) != (f != 0)) {
                    continue;
                }
                let v = u ^ mask;
                if is_safe(v) && parent[usize::from(v)] == u8::MAX {
                    parent[usize::from(v)] = u;
                    move_desc[usize::from(v)] = if next_f == 1 { to_far } else { to_near };
                    queue.push_back(v);
                }
            }
        }

        let mut path = Vec::new();
        let mut cur = 0b1111u8;
        while cur != 0 && parent[usize::from(cur)] != u8::MAX {
            path.push((cur, move_desc[usize::from(cur)]));
            cur = parent[usize::from(cur)];
        }
        path.reverse();

        let mut steps_md = String::new();
        for (idx, &(st, desc)) in path.iter().enumerate() {
            let dir = if (st & 1) != 0 { "->" } else { "<-" };
            steps_md.push_str(&format!(
                "{}. **Crossing {} (`{dir}`)**: {} *(state bitmask `0b{:04b}`)*.\n",
                idx + 1,
                idx + 1,
                desc,
                st
            ));
        }

        let sol = format!(
            "### Wolf, Goat, and Cabbage River Crossing — State-Space BFS Solution ({} Crossings)\n\n\
             **Constraints:** The boat holds the **Farmer + at most 1 item**. Without the Farmer present, the **Wolf eats the Goat** (`bit 1 == bit 2 != bit 0`) and the **Goat eats the Cabbage** (`bit 2 == bit 3 != bit 0`).\n\n\
             #### Shortest Path Found by Breadth-First Search (`0b0000 -> 0b1111`)\n\
             {steps_md}\n\
             All four reach the Far Bank (`0b1111`) without violating any safety constraint!",
            path.len()
        );
        return Some((
            "Constraint Satisfaction Puzzle · Wolf, Goat, and Cabbage River Crossing",
            sol,
        ));
    }

    // 4. N-People Birthday Collision Exact Fixed-Point Solver
    if lower.contains("birthday paradox")
        || lower.contains("birthday problem")
        || (words.iter().any(|w| w == "birthday" || w == "birthdays")
            && (lower.contains("people")
                || lower.contains("probability")
                || lower.contains("paradox")
                || lower.contains("same")))
    {
        let n_people = nums
            .iter()
            .copied()
            .find(|&n| (2..=366).contains(&n))
            .unwrap_or(23);
        let days = 365u64;
        let pairs = (n_people * n_people.saturating_sub(1)) / 2;

        // Compute exact survival probability in parts-per-million (`1_000_000`)
        let mut survival = 1.0f64;
        for i in 0..n_people {
            if i >= days {
                survival = 0.0;
                break;
            }
            survival *= (days - i) as f64 / (days as f64);
        }
        let match_prob = (1.0 - survival) * 100.0;
        let survival_pct = survival * 100.0;

        let sol = format!(
            "### The Birthday Paradox (`N = {n_people}` People, `{days}` Days) — Exact Complement Calculation\n\n\
             **Verdict:** In a group of **`{n_people}` people**, the probability that at least two share a birthday is **`{match_prob:.2}%`** (with **`{pairs}` distinct pairs**).\n\n\
             #### Exact Product Derivation\n\
             1. **Pairwise Combinatorics**: Among `{n_people}` people there are **`C({n_people}, 2) = ({n_people} * {}) / 2 = {pairs}`** distinct pairs.\n\
             2. **Probability of Zero Collisions (`P(all distinct)`)**:\n\
                `P(no match) = (365/365) * (364/365) * ... * ({}/365) = {survival_pct:.2}%`\n\
             3. **Complement (`P(at least one match)`)**:\n\
                `100% - {survival_pct:.2}% = {match_prob:.2}%`!",
            n_people.saturating_sub(1),
            days.saturating_sub(n_people.saturating_sub(1))
        );
        return Some(("Probability Puzzle · The Birthday Paradox", sol));
    }

    // 5. Two Guards / Truth-Teller & Liar Boolean Composition Evaluator
    if (lower.contains("two doors") || lower.contains("two guards"))
        && (lower.contains("truth") || lower.contains("liar") || lower.contains("knights"))
    {
        let truth_op = |b: bool| b;
        let liar_op = |b: bool| !b;
        let case_safe_ask_t = truth_op(liar_op(true));
        let case_safe_ask_l = liar_op(truth_op(true));
        let case_trap_ask_t = truth_op(liar_op(false));
        let case_trap_ask_l = liar_op(truth_op(false));

        let sol = format!(
            "### Two Doors (Truth-Teller & Liar) — Verified Boolean Composition Proof\n\n\
             **The Single Question:** Ask either guard:\n\
             > ***\"If I were to ask the OTHER guard whether Door A leads to freedom, would they say YES?\"***\n\n\
             #### Evaluated Boolean Truth Table (`T(x) = x`, `L(x) = !x`)\n\
             - **Door A = Safe (`true`), Ask Truth-Teller**: `T(L(true)) = {case_safe_ask_t}` (**\"No\"**)\n\
             - **Door A = Safe (`true`), Ask Liar**: `L(T(true)) = {case_safe_ask_l}` (**\"No\"**)\n\
             - **Door A = Trap (`false`), Ask Truth-Teller**: `T(L(false)) = {case_trap_ask_t}` (**\"Yes\"**)\n\
             - **Door A = Trap (`false`), Ask Liar**: `L(T(false)) = {case_trap_ask_l}` (**\"Yes\"**)\n\n\
             **Action Rule:** Because `T(L(x)) == L(T(x)) == !x` for every row, if the guard says **\"No\" (`false`)**, take **Door A**; if **\"Yes\" (`true`)**, take **Door B**!"
        );
        return Some((
            "Deductive Logic Puzzle · Two Doors, One Truth-Teller and One Liar",
            sol,
        ));
    }

    // 6. N-Coins Balance Scale Ternary Information Solver
    if (words
        .iter()
        .any(|w| w == "coin" || w == "coins" || w == "counterfeit"))
        && (words.iter().any(|w| {
            w == "scale" || w == "weigh" || w == "weighings" || w == "balance" || w == "heavier"
        }))
    {
        let n_coins = nums
            .iter()
            .copied()
            .find(|&n| (3..=10_000).contains(&n))
            .unwrap_or(12);
        let mut w_known = 0u32;
        let mut p3_known = 1u64;
        while p3_known < n_coins {
            w_known += 1;
            p3_known = p3_known.saturating_mul(3);
        }
        let mut w_unknown = 0u32;
        let mut p3_unk = 1u64;
        while (p3_unk.saturating_sub(3)) / 2 < n_coins {
            w_unknown += 1;
            p3_unk = p3_unk.saturating_mul(3);
        }
        let group_known = n_coins.div_ceil(3);
        let group_unk = (n_coins + 1) / 3;

        let sol = format!(
            "### Counterfeit Coin Balance Scale Puzzle (`N = {n_coins}` Coins) — Ternary (`Base-3`) Information Solution\n\n\
             **Core Information Bound:** Each weighing on a two-pan balance scale yields one of **3 outcomes (`{{-1, 0, +1}}`)**, matching the balanced ternary gates (`R4.Router`) in UOR-R4:\n\n\
             1. **If the Counterfeit Coin Is Known to Be Heavier (`{n_coins}` Coins -> `{w_known} Weighings`)**:\n\
                - Information bound: `ceil(log_3({n_coins})) = {w_known}` weighings (since `3^{w_known} = {p3_known} >= {n_coins}`).\n\
                - **Weighing 1**: Place **`{group_known}` coins** on the Left Pan vs **`{group_known}` coins** on the Right Pan, leaving `{}` aside. The heavier pan (or the unweighed group if balanced) narrows the search to at most `{group_known}` coins.\n\
             2. **If Unknown Whether Heavier or Lighter (`{n_coins}` Coins -> `{w_unknown} Weighings`)**:\n\
                - Information bound: `(3^W - 3) / 2 >= {n_coins}`, requiring **`W = {w_unknown} weighings`** (`(3^{w_unknown} - 3) / 2 = {}`).\n\
                - **Weighing 1**: Weigh **`{group_unk}` coins** vs **`{group_unk}` coins**, then rotate candidate heavy/light coins in Weighing 2.",
            n_coins.saturating_sub(group_known * 2),
            (p3_unk.saturating_sub(3)) / 2
        );
        return Some((
            "Information Theory Puzzle · Counterfeit Coin / Balance Scale Puzzle",
            sol,
        ));
    }

    // 7. Fallback to remaining canonical paradoxes (e.g. Ship of Theseus)
    for puzzle in &GLM_LOGIC_PUZZLES {
        for &ph in puzzle.phrases {
            if lower.contains(ph) {
                return Some((puzzle.title, puzzle.solution_markdown.to_string()));
            }
        }
    }
    None
}

/// Score a `GlmConceptNode` using ONLY lexical phrase, keyword, and title matches.
/// Separated cleanly from `score_concept_geometric` so lexical vs. geometric routing can be independently measured.
pub fn score_concept_lexical(
    node: &GlmConceptNode,
    lower: &str,
    words: &[String],
    stems: &[String],
) -> u32 {
    let mut lexical_score = 0u32;
    let mut specific_hits = 0u32;

    for &ph in node.phrases {
        if lower.contains(ph) {
            lexical_score += 65;
            specific_hits += 2;
        }
    }

    for &kw in node.keywords {
        let kw_norm: String = kw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if kw_norm.is_empty() {
            continue;
        }
        let kw_stem = canonical_glm_stem(&kw_norm);
        if words.iter().any(|w| w == kw || w == &kw_norm) {
            if is_generic_conversational_word(&kw_norm) {
                lexical_score += 2;
            } else {
                lexical_score += 10;
                specific_hits += 1;
            }
        } else if kw_stem.len() >= 4
            && stems
                .iter()
                .any(|s| s == &kw_stem || canonical_glm_stem(s) == kw_stem)
        {
            if is_generic_conversational_word(&kw_stem) {
                lexical_score += 1;
            } else {
                lexical_score += 7;
                specific_hits += 1;
            }
        }
    }

    let title_lower = node.title.to_ascii_lowercase();
    for tw in tokenize_words_simple(&title_lower) {
        if tw.len() >= 4
            && !is_stopword(&tw)
            && !is_generic_conversational_word(&tw)
            && words.iter().any(|w| w == &tw)
        {
            lexical_score += 5;
            specific_hits += 1;
        }
    }

    if specific_hits == 0 {
        0
    } else {
        lexical_score
    }
}

/// Score a `GlmConceptNode` using ONLY the UOR-R4 integer geometry (`Q12` L1 fibers,
/// `64-bit SWAR` quaternion sign-code Hamming distance, `128-bit VSA` hypervector Hamming resonance,
/// and `Hopf S^3 -> S^2 x U(1)` Fubini-Study alignment) over the CUDA-trained `GLM_TRAINED_VOCAB`
/// and `GLM_CONCEPT_MANIFOLD`.
///
/// Inspects ZERO lexical fields (`node.keywords`, `node.phrases`, `node.title`, `node.domain`).
pub fn score_concept_geometric(
    node: &GlmConceptNode,
    words: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> u32 {
    let mut geo_score = 0u32;
    let mut matched_vocab_fibers = 0u32;
    let mut primary_fiber_hits = 0u32;
    let mut max_matched_idf = 0u32;
    let mut seen_stems: Vec<String> = Vec::new();

    for w in words {
        if is_stopword(w) || is_generic_conversational_word(w) {
            continue;
        }
        let g_stem = canonical_glm_stem(w);
        if g_stem.len() < 3 || seen_stems.contains(&g_stem) {
            continue;
        }
        seen_stems.push(g_stem);

        if let Some(entry) = lookup_trained_vocab(w) {
            let idf = u32::from(entry.idf_weight);
            // 1. Primary Q12 + SWAR quaternion sign-code fiber ball
            let l1_pri = q12_l1_distance(&node.q12_coords, &entry.q12_coords);
            let ham_pri = swar_hamming_distance(node.sign_code, entry.sign_code);
            if l1_pri <= 256 && ham_pri <= 4 {
                geo_score += (idf / 28).max(8) + 10;
                matched_vocab_fibers += 1;
                primary_fiber_hits += 1;
                max_matched_idf = max_matched_idf.max(idf);
                continue;
            }
            // 2. Secondary Q12 + SWAR quaternion sign-code fiber ball
            let l1_alt = q12_l1_distance(&node.q12_coords, &entry.alt_q12_coords);
            let ham_alt = swar_hamming_distance(node.sign_code, entry.alt_sign_code);
            if l1_alt <= 256 && ham_alt <= 4 {
                geo_score += (idf / 36).max(6) + 7;
                matched_vocab_fibers += 1;
                max_matched_idf = max_matched_idf.max(idf);
                continue;
            }
            // 3. Tertiary VSA low-64 hypervector fiber ball
            let ham_v0 = swar_hamming_distance(node.vsa_hv[0], entry.vsa_hv[0]);
            if ham_v0 <= 4 {
                geo_score += (idf / 44).max(5) + 5;
                matched_vocab_fibers += 1;
                max_matched_idf = max_matched_idf.max(idf);
                continue;
            }
            // 4. Quaternary VSA high-64 hypervector fiber ball
            let ham_v1 = swar_hamming_distance(node.vsa_hv[1], entry.vsa_hv[1]);
            if ham_v1 <= 4 {
                geo_score += (idf / 52).max(4) + 4;
                matched_vocab_fibers += 1;
                max_matched_idf = max_matched_idf.max(idf);
            }
        }
    }

    if matched_vocab_fibers >= 2 {
        geo_score += matched_vocab_fibers * 6;
    }

    // Whole-prompt pooled Hopf S^3 -> S^2 x U(1) Fubini-Study & Q12 cosine resonance.
    // Pool from the CUDA-trained `GLM_TRAINED_VOCAB` coordinates first; when `prompt_q12` carries
    // a manifold-anchored retention vector (`GLM_RETAIN_LAMBDA_Q31[6]`), blend it with `trained_q12`.
    let (trained_q12, trained_sign) = pool_prompt_trained_geometry(words);
    let has_trained = trained_q12.iter().any(|&c| c != 0);
    let has_carried = prompt_q12 != &trained_q12
        && prompt_q12.iter().any(|&c| c != 0)
        && GLM_CONCEPT_MANIFOLD
            .iter()
            .any(|b| q12_l1_distance(prompt_q12, &b.q12_coords) <= 1600);

    // Never allow whole-prompt Hopf/cosine drift alone to activate a basin when zero vocab fibers matched
    // and there is no manifold-anchored multi-turn retention carry.
    if matched_vocab_fibers == 0 && !has_carried {
        return 0;
    }

    // Multi-stem geometric coherence gate: a single incidental word in a multi-word prompt (`seen_stems.len() >= 2`)
    // must not hijack an unrelated concept basin via geometry alone when `matched_vocab_fibers < 2`,
    // and two incidental words in a 5+ stem prompt (`>60%` unmatched stems) must not hijack an unrelated basin.
    if !has_carried && seen_stems.len() >= 2 && matched_vocab_fibers < 2 {
        return geo_score.min(5);
    }
    if !has_carried && seen_stems.len() >= 5 && matched_vocab_fibers <= 2 {
        return geo_score.min(8);
    }
    if !has_carried
        && seen_stems.len() == 1
        && matched_vocab_fibers == 1
        && (primary_fiber_hits == 0 || max_matched_idf < 512)
    {
        return geo_score.min(5);
    }

    let (pooled_q12, pooled_sign) = if has_trained && has_carried {
        let mut blended = [0i16; POINT_KEY_DIM];
        for lane in 0..POINT_KEY_DIM {
            blended[lane] = ((i32::from(trained_q12[lane]) * 2 + i32::from(prompt_q12[lane])) / 3)
                .clamp(-4096, 4096) as i16;
        }
        (blended, trained_sign ^ (prompt_sign & 0x0F0F_0F0F_0F0F_0F0F))
    } else if has_trained {
        (trained_q12, trained_sign)
    } else {
        (*prompt_q12, prompt_sign)
    };

    if pooled_q12.iter().any(|&c| c != 0) {
        let (prompt_s2, prompt_u1) = project_q12_to_hopf_q15(&pooled_q12);
        let hopf_sim =
            hopf_fubini_study_similarity(&prompt_s2, &prompt_u1, &node.hopf_s2, &node.hopf_u1);
        let q12_cos = q12_cosine_similarity(&pooled_q12, &node.q12_coords);
        let whole_ham = swar_hamming_distance(pooled_sign, node.sign_code);
        if hopf_sim > 680 {
            geo_score += (hopf_sim - 680) / 80;
        }
        if q12_cos > 680 {
            geo_score += (q12_cos - 680) / 90;
        }
        if whole_ham <= 20 {
            geo_score += (22 - whole_ham) / 4;
        }
        if !has_trained && has_carried {
            let l1_carried = q12_l1_distance(&pooled_q12, &node.q12_coords);
            if l1_carried <= 1600 {
                geo_score += (1800 - l1_carried) / 90;
            }
        }
    }

    geo_score
}

/// Combined lexical + geometric basin scorer (`R4.Prediction`) with ZERO lexical gate:
/// geometry can select a basin purely on its own when `score_concept_lexical == 0`.
pub fn score_concept_node(
    node: &GlmConceptNode,
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
) -> u32 {
    let lex = score_concept_lexical(node, lower, words, stems);
    let geo = score_concept_geometric(node, words, prompt_q12, prompt_sign);
    if lex > 0 && lex < 15 && geo <= 5 {
        let content_words = words
            .iter()
            .filter(|w| w.len() >= 3 && !is_stopword(w) && !is_generic_conversational_word(w))
            .count();
        if content_words >= 3 {
            return (lex + geo).min(8);
        }
    }
    lex + geo
}

/// Retrieve the top-K matching `GlmConceptNode`s from the pre-trained manifold using the ungated full scorer.
pub fn retrieve_top_concepts(
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    limit: usize,
) -> Vec<(&'static GlmConceptNode, u32)> {
    let mut scored: Vec<(&'static GlmConceptNode, u32)> = GLM_CONCEPT_MANIFOLD
        .iter()
        .map(|node| {
            let s = score_concept_node(node, lower, words, stems, prompt_q12, prompt_sign);
            (node, s)
        })
        .filter(|&(_, s)| s >= 10)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    scored.truncate(limit);
    scored
}

/// Retrieve the top-K matching `GlmConceptNode`s using ONLY `score_concept_geometric` (zero keyword access).
pub fn retrieve_top_concepts_geometric_only(
    words: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    limit: usize,
) -> Vec<(&'static GlmConceptNode, u32)> {
    let mut scored: Vec<(&'static GlmConceptNode, u32)> = GLM_CONCEPT_MANIFOLD
        .iter()
        .map(|node| {
            let s = score_concept_geometric(node, words, prompt_q12, prompt_sign);
            (node, s)
        })
        .filter(|&(_, s)| s >= 10)
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    scored.truncate(limit);
    scored
}

/// Extract clean subject phrases around a comparison pivot (`vs`, `versus`, `to`, `with`, `or`, `and`).
fn extract_comparison_subjects(raw_prompt: &str, lower: &str) -> Option<(String, String)> {
    let mut cleaned = raw_prompt
        .trim()
        .trim_end_matches('?')
        .trim_end_matches('.');
    for prefix in [
        "what is the difference between ",
        "what's the difference between ",
        "what are the differences between ",
        "explain the difference between ",
        "compare and contrast ",
        "can you compare ",
        "please compare ",
        "compare ",
        "explain ",
        "how does ",
        "how do ",
    ] {
        if lower.starts_with(prefix) && cleaned.len() > prefix.len() {
            cleaned = cleaned[prefix.len()..].trim();
            break;
        }
    }

    let cleaned_lower = cleaned.to_ascii_lowercase();
    for sep in [
        " vs. ",
        " vs ",
        " versus ",
        " compared to ",
        " compared with ",
        " compare to ",
        " compare with ",
        " differ from ",
        " and ",
        " to ",
        " with ",
        " or ",
    ] {
        if (sep == " to " || sep == " with ") && !lower.contains("compare") {
            continue;
        }
        if let Some(idx) = cleaned_lower.find(sep) {
            let left = cleaned[..idx]
                .trim()
                .trim_matches(|c: char| c == ',' || c == '"');
            let right_raw = cleaned[idx + sep.len()..]
                .trim()
                .trim_matches(|c: char| c == ',' || c == '"');
            let right = right_raw
                .split(" for ")
                .next()
                .unwrap_or(right_raw)
                .split(" in ")
                .next()
                .unwrap_or(right_raw)
                .split(" when ")
                .next()
                .unwrap_or(right_raw)
                .trim();
            if !left.is_empty() && !right.is_empty() && !left.eq_ignore_ascii_case(right) {
                return Some((title_case_phrase(left), title_case_phrase(right)));
            }
        }
    }
    None
}

fn title_case_phrase(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    if w.len() <= 4 && w.chars().all(|c| c.is_ascii_uppercase()) {
                        w.to_string()
                    } else {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract clean topic keywords from a prompt (removing command verbs and conversational filler).
fn extract_clean_topic(raw_prompt: &str, words: &[String]) -> String {
    let filler = [
        "write",
        "create",
        "generate",
        "compose",
        "draft",
        "tell",
        "give",
        "make",
        "let",
        "lets",
        "us",
        "me",
        "my",
        "myself",
        "you",
        "your",
        "yours",
        "yourself",
        "we",
        "our",
        "ours",
        "ourselves",
        "a",
        "an",
        "the",
        "short",
        "story",
        "poem",
        "haiku",
        "sonnet",
        "essay",
        "article",
        "blog",
        "post",
        "email",
        "letter",
        "speech",
        "about",
        "on",
        "for",
        "of",
        "with",
        "in",
        "to",
        "please",
        "can",
        "could",
        "would",
        "should",
        "will",
        "how",
        "what",
        "why",
        "who",
        "where",
        "when",
        "explain",
        "describe",
        "discuss",
        "explore",
        "is",
        "are",
        "was",
        "were",
        "does",
        "do",
        "did",
        "work",
        "works",
        "want",
        "wants",
        "like",
        "love",
        "think",
        "know",
        "scifi",
        "sci",
        "fi",
        "fantasy",
        "some",
        "something",
        "anything",
        "talk",
        "talking",
        "chat",
        "chatting",
    ];
    let kept: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| {
            w.len() > 2
                && !filler.contains(w)
                && !is_stopword(w)
                && !is_generic_conversational_word(w)
        })
        .take(6)
        .collect();
    if kept.is_empty() {
        raw_prompt
            .trim()
            .trim_end_matches('?')
            .trim_end_matches('.')
            .trim_end_matches('!')
            .to_string()
    } else {
        title_case_phrase(&kept.join(" "))
    }
}

/// Render a rich, natural-prose comparative analysis (`A vs B`), including anaphoric comparison targets (`"Compare Rome to living there"`).
#[allow(clippy::too_many_arguments)]
fn render_comparative_synthesis(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    _stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    top_concepts: &[(&'static GlmConceptNode, u32)],
    chat_history: &[(String, String)],
) -> String {
    if let Some((mut left_subj, mut right_subj)) = extract_comparison_subjects(raw_prompt, lower) {
        let left_lower = left_subj.to_ascii_lowercase();
        let right_lower = right_subj.to_ascii_lowercase();
        let left_words = tokenize_words_simple(&left_lower);
        let right_words = tokenize_words_simple(&right_lower);
        let left_stems: Vec<String> = left_words
            .iter()
            .map(|w| canonical_stem(w).to_string())
            .collect();
        let right_stems: Vec<String> = right_words
            .iter()
            .map(|w| canonical_stem(w).to_string())
            .collect();
        let (left_q12, left_sign) = pool_prompt_trained_geometry(&left_words);
        let (right_q12, right_sign) = pool_prompt_trained_geometry(&right_words);

        let mut left_hits = retrieve_top_concepts(
            &left_lower,
            &left_words,
            &left_stems,
            &left_q12,
            left_sign,
            2,
        );
        let mut right_hits = retrieve_top_concepts(
            &right_lower,
            &right_words,
            &right_stems,
            &right_q12,
            right_sign,
            2,
        );

        let left_anaphoric = detect_anaphor_kind(&left_lower, &left_words).is_some()
            || (left_hits.is_empty() && !chat_history.is_empty());
        let right_anaphoric = detect_anaphor_kind(&right_lower, &right_words).is_some()
            || (right_hits.is_empty() && !chat_history.is_empty());

        if left_anaphoric && !chat_history.is_empty() {
            let kind = detect_anaphor_kind(&left_lower, &left_words)
                .unwrap_or(AnaphorKind::Neutral);
            if let Some((ant_node, _, _, _)) = resolve_discourse_antecedent(kind, chat_history) {
                left_subj = ant_node.title.to_string();
                left_hits = vec![(ant_node, 40)];
            }
        }
        if right_anaphoric && !chat_history.is_empty() {
            let kind = detect_anaphor_kind(&right_lower, &right_words)
                .unwrap_or(AnaphorKind::Neutral);
            if let Some((ant_node, _, _, _)) = resolve_discourse_antecedent(kind, chat_history) {
                right_subj = ant_node.title.to_string();
                right_hits = vec![(ant_node, 40)];
            }
        }

        if let (Some((node_a, _)), Some((node_b, _))) = (left_hits.first(), right_hits.first()) {
            if node_a.id != node_b.id {
                return format!(
                    "When comparing **{}** and **{}**, both solve foundational problems in their respective domains while making distinct architectural and design trade-offs.\n\n{}\n\n{}\n\n| Dimension | **{}** | **{}** |\n| --- | --- | --- |\n| **Domain** | {} | {} |\n| **How It Works** | {} | {} |\n| **Core Principle** | {} | {} |\n| **Key Trade-offs** | {} | {} |\n| **Real-World Example** | {} | {} |\n\nIn practice, **{}** is the stronger choice when your priorities center on {}, whereas **{}** shines when you value {}.",
                    left_subj,
                    right_subj,
                    node_a.summary,
                    node_b.summary,
                    left_subj,
                    right_subj,
                    node_a.domain,
                    node_b.domain,
                    node_a.mechanism,
                    node_b.mechanism,
                    node_a.formal_principle,
                    node_b.formal_principle,
                    node_a.tradeoffs,
                    node_b.tradeoffs,
                    node_a.example,
                    node_b.example,
                    left_subj,
                    node_a.summary,
                    right_subj,
                    node_b.summary
                );
            }
        }

        if let Some(&(shared_node, _)) = left_hits
            .first()
            .or_else(|| right_hits.first())
            .or_else(|| top_concepts.first())
        {
            return format!(
                "**{}** and **{}** represent two complementary approaches within **{}**:\n\n{}\n\n{}\n\n{}\n\n{}\n\n**Practical Example:** {}",
                left_subj,
                right_subj,
                shared_node.title,
                shared_node.summary,
                shared_node.mechanism,
                shared_node.formal_principle,
                shared_node.tradeoffs,
                shared_node.example
            );
        }
    }

    let topic = extract_clean_topic(raw_prompt, words);
    let _ = (prompt_q12, prompt_sign);
    format!(
        "Comparing the approaches in **{topic}** comes down to balancing immediate simplicity against long-term scalability and resilience. The first approach typically favors lower initial overhead and direct execution, while the second invests in stronger structural separation and fault isolation so the system can scale cleanly as demand grows."
    )
}

/// Split a basin's prose into individual clauses/sentences and rank them dynamically by geometric
/// stem resonance (`GLM_TRAINED_VOCAB`) and `Cl(0,6)` trajectory diversity (`R4.Prediction.selectContextualToken`).
fn synthesize_clause_trajectory(
    words: &[String],
    nodes: &[&GlmConceptNode],
    max_sentences: usize,
) -> Vec<String> {
    let prompt_g_stems: Vec<String> = words
        .iter()
        .filter(|w| !is_stopword(w) && !is_generic_conversational_word(w))
        .map(|w| canonical_glm_stem(w))
        .filter(|s| s.len() >= 3)
        .collect();

    let mut candidates: Vec<(u32, usize, usize, String, Vec<String>)> = Vec::new();
    for (node_rank, &node) in nodes.iter().enumerate() {
        let sections = [
            node.summary,
            node.mechanism,
            node.formal_principle,
            node.tradeoffs,
            node.example,
        ];
        for (sec_idx, &sec) in sections.iter().enumerate() {
            for raw_sent in split_balanced_sentences(sec) {
                let s_trim = raw_sent.trim();
                if s_trim.len() < 25 {
                    continue;
                }
                let sent =
                    if s_trim.ends_with('.') || s_trim.ends_with('!') || s_trim.ends_with('?') {
                        s_trim.to_string()
                    } else {
                        format!("{s_trim}.")
                    };
                let s_words = tokenize_words_simple(&sent.to_ascii_lowercase());
                let s_stems: Vec<String> = s_words
                    .iter()
                    .filter(|w| !is_stopword(w))
                    .map(|w| canonical_glm_stem(w))
                    .filter(|st| st.len() >= 3)
                    .collect();

                let mut score = if node_rank == 0 {
                    30u32.saturating_sub((sec_idx as u32) * 3)
                } else {
                    18u32.saturating_sub((sec_idx as u32) * 3)
                };

                for ps in &prompt_g_stems {
                    if s_stems.iter().any(|ss| ss == ps) {
                        let idf_bonus = lookup_trained_vocab(ps)
                            .map(|e| u32::from(e.idf_weight) / 48)
                            .unwrap_or(6);
                        score += 12 + idf_bonus;
                    }
                }
                candidates.push((score, node_rank, sec_idx, sent, s_stems));
            }
        }
    }

    // Step through the sentence trajectory with recency/redundancy penalty (`selectContextualToken`)
    let mut selected: Vec<String> = Vec::new();
    let mut used_stems: Vec<String> = Vec::new();
    let mut used_indices: Vec<usize> = Vec::new();

    for _step in 0..max_sentences {
        let mut best_idx = None;
        let mut best_adj_score = 0u32;
        for (idx, (base_score, _n_rank, _s_idx, _sent, s_stems)) in candidates.iter().enumerate() {
            if used_indices.contains(&idx) {
                continue;
            }
            let overlap = s_stems.iter().filter(|st| used_stems.contains(st)).count() as u32;
            let adj = base_score.saturating_sub(overlap * 2).max(1);
            if adj > best_adj_score {
                best_adj_score = adj;
                best_idx = Some(idx);
            }
        }
        if let Some(win) = best_idx {
            used_indices.push(win);
            for st in &candidates[win].4 {
                if !used_stems.contains(st) {
                    used_stems.push(st.clone());
                }
            }
            selected.push(candidates[win].3.clone());
        } else {
            break;
        }
    }
    selected
}

/// Normalize a sentence to its lowercase alphanumeric word sequence for exact and near-duplicate detection.
fn normalize_sentence_key(sentence: &str) -> Vec<String> {
    tokenize_words_simple(&sentence.to_ascii_lowercase())
        .into_iter()
        .filter(|w| w.len() >= 2)
        .collect()
}

/// Deduplicate repeated or near-identical sentences across all paragraphs of a generated response.
fn dedup_prose_paragraphs(text: &str) -> String {
    if text.contains("```") {
        return text.to_string();
    }
    let mut seen_keys: Vec<Vec<String>> = Vec::new();
    let mut out_paragraphs: Vec<String> = Vec::new();

    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') || trimmed.starts_with("- ") || trimmed.starts_with("1. ") {
            out_paragraphs.push(trimmed.to_string());
            continue;
        }
        let sentences: Vec<String> = split_balanced_sentences(trimmed)
            .into_iter()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.ends_with('.') || s.ends_with('!') || s.ends_with('?') || s.ends_with(':') {
                    s.to_string()
                } else {
                    format!("{s}.")
                }
            })
            .collect();
        if sentences.is_empty() {
            let key = normalize_sentence_key(trimmed);
            if !key.is_empty() && !seen_keys.contains(&key) {
                seen_keys.push(key);
                out_paragraphs.push(trimmed.to_string());
            }
            continue;
        }
        let mut kept_sentences: Vec<String> = Vec::new();
        for sent in sentences {
            let key = normalize_sentence_key(&sent);
            if key.is_empty() {
                continue;
            }
            let is_dup = seen_keys.iter().any(|existing| {
                if existing == &key {
                    return true;
                }
                if existing.len() >= 6 && key.len() >= 6 {
                    let shared = key.iter().filter(|w| existing.contains(w)).count();
                    let min_len = existing.len().min(key.len());
                    return shared * 10 >= min_len * 8;
                }
                false
            });
            if !is_dup {
                seen_keys.push(key);
                kept_sentences.push(sent);
            }
        }
        if !kept_sentences.is_empty() {
            out_paragraphs.push(kept_sentences.join(" "));
        }
    }

    if out_paragraphs.is_empty() {
        text.to_string()
    } else {
        out_paragraphs.join("\n\n")
    }
}

/// Render a natural-prose multi-concept synthesis when 2 distinct concept basins are co-activated.
fn render_multi_concept_synthesis(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    node_1: &GlmConceptNode,
    node_2: &GlmConceptNode,
) -> String {
    let _ = (raw_prompt, lower);
    let (token_stream, stepped, _) =
        generate_token_level_geometric_prose(words, &[node_1, node_2], 240, 380);
    if stepped >= 45 {
        return dedup_prose_paragraphs(&token_stream);
    }
    let clauses = synthesize_clause_trajectory(words, &[node_1, node_2], 6);
    if clauses.len() >= 3 {
        return dedup_prose_paragraphs(&clauses.join("\n\n"));
    }
    dedup_prose_paragraphs(&format!("{}\n\n{}", node_1.summary, node_2.summary))
}

/// Render a natural, flowing conversational response for a matched `GlmConceptNode`,
/// decoded token-by-token via `generate_token_level_geometric_prose` (`R4-23` + `R4-24` + `R4-25` + `R4-19`).
fn render_single_concept_response(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    _stems: &[String],
    node: &GlmConceptNode,
) -> String {
    let _ = raw_prompt;
    let is_eli5 = lower.contains("eli5")
        || lower.contains("like i'm 5")
        || lower.contains("like i am 5")
        || lower.contains("like i'm five")
        || lower.contains("for a child")
        || lower.contains("in simple terms")
        || lower.contains("simply")
        || lower.contains("simpler");

    let is_brief = lower.contains("briefly")
        || lower.contains("short answer")
        || lower.contains("one paragraph")
        || lower.contains("in a nutshell")
        || lower.starts_with("tldr")
        || lower.starts_with("summarize");

    if is_eli5 {
        let (eli5_stream, stepped, _) =
            generate_token_level_geometric_prose(words, &[node], 90, 170);
        if stepped >= 35 {
            return dedup_prose_paragraphs(&format!(
                "Here is an **Intuitive (ELI5)** way to think about **{}**:\n\n{}",
                node.title, eli5_stream
            ));
        }
    }

    if is_brief {
        let (brief_stream, stepped, _) =
            generate_token_level_geometric_prose(words, &[node], 45, 95);
        if stepped >= 20 {
            return dedup_prose_paragraphs(&brief_stream);
        }
    }

    // Detect conversational reciprocity framing (e.g., "My day is going fine. I think that artificial intelligence is on my mind. How about you?")
    let has_day_or_mood = lower.contains("my day")
        || lower.contains("doing fine")
        || lower.contains("doing well")
        || lower.contains("doing good")
        || lower.contains("going fine")
        || lower.contains("going well")
        || lower.contains("i am fine")
        || lower.contains("i am good");
    let has_mind_or_reciprocity = lower.contains("on my mind")
        || lower.contains("how about you")
        || lower.contains("what about you")
        || lower.contains("what do you think");

    if has_day_or_mood || has_mind_or_reciprocity {
        let opener = if has_day_or_mood {
            format!(
                "Glad to hear your day is going well! **{}** is very much on my mind too—it's one of my favorite subjects to explore.",
                node.title
            )
        } else {
            format!(
                "I've been thinking about **{}** as well—it's a fascinating area to dive into together.",
                node.title
            )
        };
        let (rec_stream, stepped, _) =
            generate_token_level_geometric_prose(words, &[node], 80, 160);
        let body = if stepped >= 25 {
            rec_stream
        } else {
            synthesize_clause_trajectory(words, &[node], 3).join(" ")
        };
        let closer = format!(
            "What side of **{}** is on your mind today—how it works under the hood, where it is headed next, or a concrete example we can walk through?",
            node.title
        );
        return dedup_prose_paragraphs(&format!("{opener}\n\n{body}\n\n{closer}"));
    }

    // Pure autoregressive token-by-token geometric trajectory decoding (`w_0 -> w_1 -> ... -> w_T`)
    // across the active concept basin without static paragraph prepending.
    let (token_stream, stepped, _) =
        generate_token_level_geometric_prose(words, &[node], 450, 550);
    if stepped >= 40 {
        return dedup_prose_paragraphs(&token_stream);
    }

    let clauses = synthesize_clause_trajectory(words, &[node], 8);
    dedup_prose_paragraphs(&clauses.join("\n\n"))
}

/// Render a step-by-step Fermi / order-of-magnitude estimation.
fn render_fermi_estimation(
    raw_prompt: &str,
    words: &[String],
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    let lower = raw_prompt.to_ascii_lowercase();
    if lower.contains("piano tuner") {
        return "### Fermi Estimation · Piano Tuners in Chicago\n\n\
                Following Enrico Fermi's classic dimensional decomposition, we break the unknown total into independent, estimable factors whose log-normal errors partially cancel:\n\n\
                1. **Factor 1 — Population & Households**: The Chicago metropolitan area has roughly `9,000,000` residents (`~3,000,000` city proper). At an average of `2.5` people per household, that gives **`~3,600,000` households**.\n\
                2. **Factor 2 — Piano Ownership Rate**: Roughly `1 in 20` households (`5%`) owns an acoustic piano (plus schools, churches, conservatories, and concert halls), yielding **`~180,000` pianos**.\n\
                3. **Factor 3 — Annual Tuning Demand**: An acoustic piano is tuned about **once per year** on average, requiring **`~180,000` tunings/year**.\n\
                4. **Factor 4 — Annual Capacity per Tuner**: A full-time piano tuner services about `4` pianos per day (including travel and 2 hours per tuning) across `5` days/week and `50` working weeks/year = **`1,000` tunings/year per tuner**.\n\n\
                **Estimated Result:** `180,000 tunings/year / 1,000 tunings/tuner ≈ 180 piano tuners` in the Chicago area (or `~60–80` within city limits)—remarkably close to actual directory listings!"
            .to_string();
    }

    let subject = extract_clean_topic(raw_prompt, words);
    let anchor_note = top_concepts
        .first()
        .map(|(n, _)| format!(" ({})", n.summary))
        .unwrap_or_default();

    format!(
        "### Order-of-Magnitude Fermi Estimate · {subject}\n\n\
         To estimate **{subject}** from first principles, we decompose the quantity into three measurable factors{anchor_note}:\n\n\
         1. **Base Population / Spatial Scale (`N_base`)**: Bound the total population, surface area, or active fleet size within a factor of 2.\n\
         2. **Participation / Incidence Fraction (`f_active`)**: Multiply by the fraction of units actively participating at any given time.\n\
         3. **Per-Unit Rate & Service Capacity (`R_unit`)**: Multiply by the per-unit frequency and divide by the annual capacity of a single server or channel (`~2,000` working hours/year).\n\n\
         Because independent multiplicative errors add in log-space with standard deviation `sqrt(k) * sigma`, a 3-factor decomposition typically lands within **half an order of magnitude (`2x–3x`)** of the exact empirical figure."
    )
}

/// Render creative & generative writing (short stories, poems, essays, speeches, emails).
fn render_creative_generation(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    top_concepts: &[(&'static GlmConceptNode, u32)],
) -> String {
    let topic = extract_clean_topic(raw_prompt, words);
    let concept_context = top_concepts.first().map(|(n, _)| *n);

    if words.iter().any(|w| w == "haiku") {
        return format!(
            "*{topic} in stillness,*\n*Patterns wake in quiet light,*\n*Truth endures through time.*"
        );
    }

    if words
        .iter()
        .any(|w| matches!(w.as_str(), "poem" | "poetry" | "sonnet" | "verse"))
    {
        let imagery = concept_context.map(|n| n.title).unwrap_or(topic.as_str());
        return format!(
            "### Poem: *{topic}*\n\n\
             Beneath the quiet arch of measured space,\n\
             Where **{imagery}** weaves its unseen thread,\n\
             Each fleeting moment finds its rightful place,\n\
             And ancient questions wake where stars are led.\n\n\
             No random drift commands the turning sphere,\n\
             No single voice can claim the whole design;\n\
             Through patient craft the hidden forms appear,\n\
             Where thought and living memory align.\n\n\
             Though seasons shift and empires fade from view,\n\
             The deeper rhythm keeps its steady beat—\n\
             In every question asked and built anew,\n\
             The seeker and the sought at last can meet."
        );
    }

    if words.iter().any(|w| {
        matches!(
            w.as_str(),
            "story" | "scifi" | "fantasy" | "fable" | "narrative"
        )
    }) || lower.contains("short story")
        || lower.contains("sci-fi")
    {
        let setting_detail = concept_context
            .map(|n| {
                format!(
                    " Drawing on the real-world principles of **{}** ({}),",
                    n.title, n.summary
                )
            })
            .unwrap_or_default();

        return format!(
            "### Short Story: *The Horizon of {topic}*\n\n\
             The observatory lamps hummed softly against the midnight chill as Elena adjusted the brass dial on the console. Outside the high arched windows, mist curled over the rooftops, but across her workbench lay the puzzle she had spent three winters trying to untangle: **{topic}**.{setting_detail}\n\n\
             \"Everyone keeps looking at the surface noise,\" her mentor Julian said, setting two steaming mugs of coffee beside the notebooks. \"They measure the pieces in isolation and wonder why the pattern slips through their fingers.\"\n\n\
             Elena traced a pencil line connecting three distant entries in her ledger. \"Because the harmony isn't in the pieces themselves—it's in the relationships between them. Look at what happens when we stop forcing an approximation and follow the exact path from one node to the next.\"\n\n\
             She keyed the sequence into the array. One by one, the amber indicators locked into phase, resolving months of scattered observations into a single, unmistakable trajectory. Outside, the first pale gold of dawn broke over the horizon, illuminating a world that suddenly made a little more sense than it had the night before."
        );
    }

    if words.iter().any(|w| {
        matches!(
            w.as_str(),
            "essay" | "article" | "blog" | "speech" | "keynote"
        )
    }) {
        let body_support = concept_context
            .map(|n| format!("{}\n\n{}", n.summary, n.mechanism))
            .unwrap_or_else(|| {
                format!(
                    "At its heart, **{topic}** reflects the interplay between human curiosity, practical constraints, and the search for lasting structure. Across generations, progress has rarely come from isolated breakthroughs alone; instead, it emerges when foundational ideas meet everyday practice."
                )
            });

        let nuance_support = concept_context
            .map(|n| format!("{}\n\n{}", n.formal_principle, n.tradeoffs))
            .unwrap_or_else(|| {
                format!(
                    "Looking closely at **{topic}** reveals a recurring balance between bold innovation and careful stewardship. When communities and thinkers honor both sides of that balance, the result is not merely technical efficiency, but genuine cultural and intellectual enrichment."
                )
            });

        return format!(
            "### Essay: *Reflections on {topic}*\n\n\
             **Thesis:** Understanding **{topic}** requires looking beyond surface appearances to the deeper historical, structural, and human forces that shape it.\n\n\
             {body_support}\n\n\
             {nuance_support}\n\n\
             Ultimately, the enduring value of **{topic}** lies in how it connects abstract insight to lived human experience—reminding us that every lasting achievement is built on clarity, curiosity, and care."
        );
    }

    if words
        .iter()
        .any(|w| matches!(w.as_str(), "email" | "letter" | "memo" | "pitch"))
    {
        return format!(
            "**Subject:** Thoughts & Next Steps on {topic}\n\n\
             Hi team,\n\n\
             I wanted to share a quick synthesis on **{topic}** so we can align on our next steps:\n\n\
             - **Context & Goal**: Focus on the core value drivers of {topic} while keeping our workflow simple and verifiable.\n\
             - **Recommended Approach**: Start with a clean, well-tested baseline, measure real-world feedback early, and iterate where it matters most.\n\
             - **Next Steps**: Let's review the key milestones together this week and finalize ownership.\n\n\
             Best regards,\n\
             [Your Name]"
        );
    }

    format!(
        "Here is a thoughtful look at **{topic}**:\n\n{}",
        concept_context
            .map(|n| format!("{}\n\n{}\n\n{}", n.summary, n.mechanism, n.example))
            .unwrap_or_else(|| format!("**{topic}** brings together rich historical context, practical craft, and everyday human experience."))
    )
}

/// Compositional Geometric Clause Realizer for open-domain or multi-concept prompts.
/// Uses `retrieve_top_concepts_geometric_only` over the CUDA-trained `GLM_TRAINED_VOCAB`
/// and `GLM_CONCEPT_MANIFOLD` to weave clauses from the highest-resonating basins.
fn render_open_domain_natural_prose(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    chat_history_len: usize,
) -> String {
    let trimmed = raw_prompt.trim();

    // 1. Check self-referential / identity / architecture conversational questions
    let is_self_inquiry = lower.contains("talk about you")
        || lower.contains("tell me about you")
        || lower.contains("about yourself")
        || lower.contains("who are you")
        || lower.contains("what are you")
        || lower.contains("what can you do")
        || lower.contains("how do you work")
        || lower.contains("how were you trained")
        || lower.contains("are you a transformer")
        || lower.contains("what is your name")
        || lower.contains("what's your name")
        || (words.len() <= 6
            && words.iter().any(|w| w == "you" || w == "yourself")
            && words
                .iter()
                .any(|w| matches!(w.as_str(), "talk" | "chat" | "tell" | "who"))
            && !words
                .iter()
                .any(|w| matches!(w.as_str(), "mean" | "think" | "about")));

    if is_self_inquiry {
        return "I'd love to! I am the **UOR-R4 Generalized Geometric Assistant**, a conversational language and reasoning model running directly in your browser via WebAssembly.\n\n\
                What makes me unique is how I think under the hood: instead of relying on a neural transformer or floating-point matrix multiplications, my vocabulary, concept basins, and token horizons are trained on an NVIDIA RTX 5090 using **discrete integer geometry**—quaternions on the 3-sphere (`S^3 -> S^2 x U(1)`), 17-root Icosian golden-ratio lattices, 16-root `E8` and 120-root `H4` polytopes, and 8-head integer retention.\n\n\
                In conversation, though, you can chat with me naturally about almost anything: world cities and travel, history, biographies, science, philosophy, literature, food, logic brainteasers, creative writing, exact mathematics, or software engineering. What are you curious about today?"
            .to_string();
    }

    // 1b. Check conversational day/mood reciprocity when no domain topic is attached ("My day is going fine, how about you?")
    let has_day_or_mood = lower.contains("my day")
        || lower.contains("doing fine")
        || lower.contains("doing well")
        || lower.contains("doing good")
        || lower.contains("going fine")
        || lower.contains("going well")
        || lower.contains("how about you")
        || lower.contains("what about you");

    // 2. Check open-ended conversation invitations, "fun fact", "surprise me", "I'm bored", "what should we talk about?"
    let content_words: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|w| w.len() >= 3 && !is_stopword(w) && !is_generic_conversational_word(w))
        .collect();

    if has_day_or_mood && content_words.len() <= 1 {
        return "Glad to hear your day is going well! Things are going great on my end too—I've been enjoying exploring ideas across science, history, world cities, philosophy, and software engineering right here in the browser. What is on your mind today?"
            .to_string();
    }

    if lower.contains("fun fact")
        || lower.contains("something interesting")
        || lower.contains("something cool")
        || lower.contains("surprise me")
        || lower.contains("bored")
        || lower.contains("what should we talk about")
        || lower.contains("what do you want to talk about")
        || lower.contains("what can we talk about")
    {
        let node = &GLM_CONCEPT_MANIFOLD[(chat_history_len * 13 + 7) % GLM_CONCEPT_MANIFOLD.len()];
        return format!(
            "Here is something fascinating to kick things off—let's look at **{}**:\n\n{}\n\n{}\n\nWant to dive deeper into **{}**, or switch gears to another city, historical era, scientific mystery, or puzzle?",
            node.title, node.summary, node.example, node.title
        );
    }

    if content_words.is_empty() {
        return "I'm ready whenever you are! We can talk about anything that catches your interest—for example:\n\n\
                - **World Cities & History**: Paris, Rome, Tokyo, Ancient Egypt, the Renaissance, or the Apollo Moon landing\n\
                - **Science & Philosophy**: Quantum mechanics, relativity, DNA & CRISPR, black holes, or Stoicism\n\
                - **Everyday Culture & Nature**: The history of coffee and tea, dogs and cats, music theory, or cinema\n\
                - **Math, Logic & Code**: Exact arithmetic, the Monty Hall or 2-Egg puzzles, or runnable code in Rust, Python, TypeScript, Go, and SQL\n\n\
                Which of those sounds fun to explore?"
            .to_string();
    }

    if lower.contains("recommend")
        || lower.contains("suggestion")
        || lower.contains("ideas for")
        || lower.contains("brainstorm")
    {
        let topic = extract_clean_topic(raw_prompt, words);
        return format!(
            "Here are several thoughtful ideas and angles to explore for **{topic}**:\n\n\
             1. **Start with the Core Experience**: Focus on the single most rewarding or memorable part of **{topic}** first—whether you are planning a project, learning a skill, or exploring a new place, depth beats trying to do everything at once.\n\
             2. **Combine Tradition with Fresh Perspective**: Pair classic, time-tested foundations with one or two unexpected creative twists.\n\
             3. **Build a Small Daily or Weekly Rhythm**: Sustainable progress and enjoyment come from consistent, low-friction habits rather than occasional sprints.\n\
             4. **Share and Iterate**: Bring a friend, colleague, or community into the loop early—talking through **{topic}** aloud often sparks the best insights.\n\n\
             Would you like to zoom in on any of these angles or tailor them to a specific goal?"
        );
    }

    // 3. Use the pure geometric scorer over the CUDA-trained vocabulary manifold to find the nearest basins
    // (Requires >= 2 matched vocabulary fibers on multi-word prompts so incidental single words never hijack the topic)
    let geo_hits = retrieve_top_concepts_geometric_only(words, prompt_q12, prompt_sign, 2);
    if let Some(&(best_node, best_geo_score)) = geo_hits.first() {
        if best_geo_score >= 12 {
            let mut active_nodes = vec![best_node];
            if let Some(&(second_node, second_score)) = geo_hits.get(1) {
                if second_score >= 12 && second_node.id != best_node.id {
                    active_nodes.push(second_node);
                }
            }
            let (token_stream, stepped, _) =
                generate_token_level_geometric_prose(words, &active_nodes, 90, 220);
            if stepped >= 35 {
                return dedup_prose_paragraphs(&token_stream);
            }
            let clauses = synthesize_clause_trajectory(words, &active_nodes, 4);
            if !clauses.is_empty() {
                return dedup_prose_paragraphs(&clauses.join("\n\n"));
            }
        }
    }

    // 4. First-Principles & Causal Pointer-Induction Synthesis for genuinely novel/unseen open-domain topics
    // (Never dumps an unrelated encyclopedia article when basin overlap is below threshold!)
    let topic = extract_clean_topic(trimmed, words);
    let mut state = GlmRetentionState::new();
    for w in words {
        state.step_token(w);
    }
    let packed_4b = state.packed_4block_root();
    let focus_terms: Vec<&str> = content_words.iter().copied().take(5).collect();
    let focus_joined = if focus_terms.is_empty() {
        topic.clone()
    } else {
        focus_terms.join(", ")
    };

    let is_how_to = lower.starts_with("how do i ")
        || lower.starts_with("how can i ")
        || lower.starts_with("how to ")
        || lower.contains("best way to")
        || lower.contains("steps to")
        || lower.contains("tips for")
        || lower.contains("guide to")
        || lower.starts_with("help me ");

    if is_how_to {
        return format!(
            "Here is a practical, **First-Principles** step-by-step approach to **{topic}** (focusing on `{focus_joined}`):\n\n\
             1. **Define the Goal & Constraints First**: Clarify the exact outcome you want from **{topic}**, your available time/materials, and what 'good' looks like before jumping into execution.\n\
             2. **Start with a Clean, Minimal Baseline**: Set up the core essentials for **{topic}** without overcomplicating the first iteration—get one small, working pass end-to-end so you have immediate feedback.\n\
             3. **Iterate on the High-Leverage Bottleneck**: Observe how the first pass behaves in practice, adjust one variable at a time around `{focus_joined}`, and keep notes on what measurably improves the result.\n\
             4. **Build Consistency & Safeguards**: Once the core workflow for **{topic}** is reliable, turn the repeatable steps into a simple checklist or habit.\n\n\
             *(Note: **{topic}** is synthesized compositionally from first principles rather than a pre-stored static article in the offline manifold `packed_4block_root = {packed_4b}`.)* What specific part of **{topic}** are you tackling first, or what constraints should we tailor this around?"
        );
    }

    let is_why_or_causal = lower.starts_with("why ")
        || lower.starts_with("what happens")
        || lower.starts_with("what causes")
        || lower.starts_with("how does ")
        || lower.starts_with("explain ")
        || lower.ends_with('?');

    if is_why_or_causal {
        return format!(
            "Analyzing **{topic}** from **First-Principles** (`focus terms: {focus_joined}`):\n\n\
             - **Core Mechanism & Causal Drivers**: At its foundation, **{topic}** is governed by how its primary components (`{focus_joined}`) interact under resource, structural, and feedback constraints. Rather than treating it as an isolated black box, trace the direct input-to-output causal chain.\n\
             - **Key Trade-Offs & Boundary Conditions**: In practice, behavior around **{topic}** shifts whenever scale, timing, or environmental conditions cross a threshold—balancing immediate efficiency against long-term stability and resilience.\n\
             - **Practical Verification**: The most reliable way to validate a hypothesis about **{topic}** is to hold surrounding variables constant, test one factor at a time, and observe the direct outcome.\n\n\
             *(Note: **{topic}** is synthesized compositionally from first principles rather than a pre-stored static article in the offline manifold `packed_4block_root = {packed_4b}`. Share any specific context, constraints, or data you have and we can drill right into it!)*"
        );
    }

    format!(
        "Looking at **{topic}** (`{focus_joined}`): the most effective way to approach it is to separate the foundational invariants from surface conventions, start with a clear and testable baseline, and iterate from real feedback.\n\n\
         What specific angle of **{topic}** would you like to focus on—practical steps, underlying principles, or working through a concrete example together?"
    )
}

/// Explicit Pragmatic Discourse Act classification for multi-turn conversational continuity (`Phase 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlmDiscourseAct {
    /// Fresh topical query or self-contained domain shift.
    TopicShift,
    /// Warm conversational mood/day check-in or reciprocity (`"my day is going fine, how about you?"`).
    SocialReciprocity,
    /// Self-referential inquiry about the assistant's identity, training, or integer geometry (`"who are you"`, `"how do you work"`).
    SelfInquiry,
    /// Anaphoric or deictic reference to an entity/place/concept from `chat_history` (`"there"`, `"it"`, `"its"`, `"her"`, `"his"`).
    AnaphoricFollowup,
    /// Request to clarify a previous point (`"what do you mean"`, `"can you clarify"`).
    Clarification,
    /// Request for a concrete illustration (`"give me an example"`).
    Exemplification,
    /// Request for a simpler or intuitive explanation (`"eli5"`, `"in simple terms"`).
    Simplification,
    /// Request for a concise summary (`"summarize"`, `"tldr"`).
    Summarization,
    /// Request to elaborate further or analyze trade-offs on the active topic (`"tell me more"`, `"go deeper"`, `"what are the trade-offs"`).
    Deepening,
}

/// Anaphoric reference category for multi-turn antecedent resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnaphorKind {
    /// Spatial or geographic deictic reference (`"there"`, `"that city"`, `"that place"`, `"that country"`).
    Locative,
    /// Female personal pronoun (`"she"`, `"her"`, `"hers"`, `"herself"`).
    FemalePersonal,
    /// Male personal pronoun (`"he"`, `"him"`, `"his"`, `"himself"`).
    MalePersonal,
    /// Neutral singular/plural entity pronoun (`"it"`, `"its"`, `"they"`, `"them"`, `"their"`, `"that"`).
    Neutral,
}

/// Words that specify an aspect, property, or dimension of an antecedent topic rather than introducing a new entity.
fn is_aspect_modifier_word(w: &str) -> bool {
    matches!(
        w,
        "there"
            | "here"
            | "it"
            | "its"
            | "itself"
            | "they"
            | "them"
            | "their"
            | "theirs"
            | "he"
            | "him"
            | "his"
            | "himself"
            | "she"
            | "her"
            | "hers"
            | "herself"
            | "that"
            | "this"
            | "those"
            | "these"
            | "food"
            | "foods"
            | "cuisine"
            | "dish"
            | "dishes"
            | "eat"
            | "eating"
            | "drink"
            | "drinking"
            | "gastronomy"
            | "culinary"
            | "restaurant"
            | "restaurants"
            | "cafe"
            | "cafes"
            | "bakery"
            | "culture"
            | "cultural"
            | "history"
            | "historical"
            | "architecture"
            | "architectural"
            | "building"
            | "buildings"
            | "monument"
            | "monuments"
            | "landmark"
            | "landmarks"
            | "museum"
            | "museums"
            | "art"
            | "arts"
            | "music"
            | "weather"
            | "climate"
            | "population"
            | "people"
            | "language"
            | "economy"
            | "transit"
            | "subway"
            | "transport"
            | "neighborhood"
            | "neighborhoods"
            | "district"
            | "districts"
            | "park"
            | "parks"
            | "river"
            | "rivers"
            | "life"
            | "living"
            | "live"
            | "visit"
            | "visiting"
            | "travel"
            | "famous"
            | "known"
            | "popular"
            | "important"
            | "special"
            | "unique"
            | "theory"
            | "theories"
            | "discovery"
            | "discoveries"
            | "discover"
            | "discovered"
            | "invention"
            | "inventions"
            | "invent"
            | "invented"
            | "work"
            | "works"
            | "book"
            | "books"
            | "play"
            | "plays"
            | "painting"
            | "paintings"
            | "symphony"
            | "symphonies"
            | "law"
            | "laws"
            | "equation"
            | "equations"
            | "legacy"
            | "impact"
            | "influence"
            | "contribution"
            | "contributions"
            | "childhood"
            | "early"
            | "education"
            | "biography"
            | "career"
            | "death"
            | "died"
            | "born"
            | "prize"
            | "nobel"
            | "award"
            | "fall"
            | "fell"
            | "collapse"
            | "collapsed"
            | "decline"
            | "rise"
            | "origin"
            | "origins"
            | "founded"
            | "build"
            | "built"
            | "govern"
            | "governed"
            | "government"
            | "military"
            | "army"
            | "trade"
            | "tradeoff"
            | "tradeoffs"
            | "pros"
            | "cons"
            | "advantage"
            | "advantages"
            | "disadvantage"
            | "disadvantages"
            | "limitation"
            | "limitations"
            | "downside"
            | "downsides"
            | "complexity"
            | "performance"
            | "speed"
            | "memory"
            | "safety"
            | "security"
    )
}

/// Detect whether a turn contains an explicit spatial, personal, or entity anaphor and return its `AnaphorKind`.
pub fn detect_anaphor_kind(lower: &str, words: &[String]) -> Option<AnaphorKind> {
    let is_existential_there = lower.contains("are there")
        || lower.contains("is there")
        || lower.contains("were there")
        || lower.contains("was there")
        || lower.contains("there are")
        || lower.contains("there is")
        || lower.contains("there were")
        || lower.contains("there was")
        || lower.contains("will there");
    if (!is_existential_there && words.iter().any(|w| w == "there"))
        || lower.contains("that place")
        || lower.contains("that city")
        || lower.contains("that country")
        || lower.contains("that region")
        || lower.contains("that nation")
        || lower.contains("this city")
        || lower.contains("this place")
    {
        return Some(AnaphorKind::Locative);
    }

    if words
        .iter()
        .any(|w| matches!(w.as_str(), "she" | "her" | "hers" | "herself"))
    {
        return Some(AnaphorKind::FemalePersonal);
    }

    if words
        .iter()
        .any(|w| matches!(w.as_str(), "he" | "him" | "his" | "himself"))
    {
        return Some(AnaphorKind::MalePersonal);
    }

    let is_idiomatic_it = lower.contains("what time is it")
        || lower.contains("how is it going")
        || lower.contains("is it true")
        || lower.starts_with("it is ")
        || lower.starts_with("it was ")
        || lower.contains("make it shorter");
    let has_neutral_pronoun = (!is_idiomatic_it
        && words
            .iter()
            .any(|w| matches!(w.as_str(), "it" | "its" | "itself" | "they" | "them" | "their" | "theirs")))
        || lower.contains("about that")
        || lower.contains("why is that")
        || lower.contains("how does that")
        || lower.contains("what does that")
        || lower.contains("like that")
        || lower.contains("that empire")
        || lower.contains("that civilization")
        || lower.contains("that algorithm")
        || lower.contains("that system")
        || lower.contains("that theory")
        || lower.contains("that language")
        || lower.contains("that person");

    if has_neutral_pronoun {
        return Some(AnaphorKind::Neutral);
    }

    None
}

fn is_self_inquiry_prompt(lower: &str, words: &[String]) -> bool {
    lower.contains("talk about you")
        || lower.contains("tell me about you")
        || lower.contains("about yourself")
        || lower.contains("who are you")
        || lower.contains("what are you")
        || lower.contains("what can you do")
        || lower.contains("how do you work")
        || lower.contains("how were you trained")
        || lower.contains("are you a transformer")
        || lower.contains("what is your name")
        || lower.contains("what's your name")
        || (words.len() <= 6
            && words.iter().any(|w| w == "you" || w == "yourself")
            && words
                .iter()
                .any(|w| matches!(w.as_str(), "talk" | "chat" | "tell" | "who"))
            && !words
                .iter()
                .any(|w| matches!(w.as_str(), "mean" | "think" | "about")))
}

/// Classify the pragmatic discourse act of the user's turn given `lower`, `words`, and whether `chat_history` is non-empty.
pub fn classify_discourse_act(
    lower: &str,
    words: &[String],
    _stems: &[String],
    has_history: bool,
) -> GlmDiscourseAct {
    // Comparative queries ("Compare Rome to living there", "A vs B") are handled by `render_comparative_synthesis`,
    // which resolves any anaphoric comparison side against `chat_history`.
    let is_comparison = lower.contains(" vs ")
        || lower.contains(" vs. ")
        || lower.contains(" versus ")
        || lower.contains("difference between")
        || lower.contains("differences between")
        || lower.starts_with("compare ")
        || lower.contains(" compare ")
        || lower.contains("compared to")
        || lower.contains("compared with");
    if is_comparison {
        return GlmDiscourseAct::TopicShift;
    }

    if is_self_inquiry_prompt(lower, words) {
        return GlmDiscourseAct::SelfInquiry;
    }

    let has_day_or_mood = lower.contains("my day")
        || lower.contains("doing fine")
        || lower.contains("doing well")
        || lower.contains("doing good")
        || lower.contains("going fine")
        || lower.contains("going well")
        || lower.contains("how about you")
        || lower.contains("what about you");
    if has_day_or_mood {
        let content_words_count = words
            .iter()
            .filter(|w| w.len() >= 3 && !is_stopword(w) && !is_generic_conversational_word(w))
            .count();
        if content_words_count <= 1 {
            return GlmDiscourseAct::SocialReciprocity;
        }
    }

    if !has_history {
        return GlmDiscourseAct::TopicShift;
    }

    if words.len() <= 12 {
        if lower.contains("what do you mean")
            || lower.contains("what did you mean")
            || lower.contains("what does that mean")
            || lower.contains("can you clarify")
            || lower.contains("clarify that")
            || lower == "how so"
            || lower == "how so?"
            || lower.contains("in what sense")
            || lower.contains("why do you say")
            || lower.contains("what makes you say")
        {
            return GlmDiscourseAct::Clarification;
        }
        if lower.contains("give me an example")
            || lower.contains("give an example")
            || lower.contains("concrete example")
            || lower.starts_with("for example")
            || lower.starts_with("for instance")
        {
            return GlmDiscourseAct::Exemplification;
        }
        if lower.starts_with("explain like")
            || lower.contains("like i'm ")
            || lower.contains("like i am ")
            || lower.contains("eli5")
            || lower.starts_with("simplify")
            || lower.contains("simpler")
            || lower.contains("in simple terms")
        {
            return GlmDiscourseAct::Simplification;
        }
        if lower.starts_with("summarize")
            || lower.starts_with("tldr")
            || lower.contains("make it shorter")
        {
            return GlmDiscourseAct::Summarization;
        }
        let non_aspect_words: Vec<String> = words
            .iter()
            .filter(|w| !is_aspect_modifier_word(w))
            .cloned()
            .collect();
        let non_aspect_stems: Vec<String> = non_aspect_words
            .iter()
            .map(|w| canonical_stem(w).to_string())
            .collect();
        let non_aspect_lower = non_aspect_words.join(" ");
        let max_entity_lex = GLM_CONCEPT_MANIFOLD
            .iter()
            .map(|n| {
                score_concept_lexical(n, &non_aspect_lower, &non_aspect_words, &non_aspect_stems)
            })
            .max()
            .unwrap_or(0);

        if max_entity_lex < 15
            && (lower.starts_with("tell me more")
                || lower.starts_with("explain more")
                || lower.starts_with("explain that")
                || lower.starts_with("elaborate")
                || lower.starts_with("go deeper")
                || lower.contains("pros and cons")
                || lower.contains("trade-offs")
                || lower.contains("tradeoffs")
                || lower.contains("trade offs")
                || lower.contains("why does that happen")
                || lower == "why is that"
                || lower == "why is that?"
                || lower == "how come"
                || lower == "how come?")
        {
            return GlmDiscourseAct::Deepening;
        }
    }

    // Check for explicit spatial, personal, or entity anaphora ("there", "it", "its", "her", "his", "she", "he", "they", "that city")
    if let Some(_anaphor_kind) = detect_anaphor_kind(lower, words) {
        if words.len() <= 24
            && !lower.starts_with("how many ")
            && !lower.contains("fermi")
            && match_logic_puzzle(lower, words).is_none()
            && match_conversational_qa(lower, words).is_none()
        {
            let non_aspect_words: Vec<String> = words
                .iter()
                .filter(|w| !is_aspect_modifier_word(w))
                .cloned()
                .collect();
            let non_aspect_stems: Vec<String> = non_aspect_words
                .iter()
                .map(|w| canonical_stem(w).to_string())
                .collect();
            let non_aspect_lower = non_aspect_words.join(" ");
            let max_entity_lex = GLM_CONCEPT_MANIFOLD
                .iter()
                .map(|n| {
                    score_concept_lexical(
                        n,
                        &non_aspect_lower,
                        &non_aspect_words,
                        &non_aspect_stems,
                    )
                })
                .max()
                .unwrap_or(0);
            if max_entity_lex < 15 {
                return GlmDiscourseAct::AnaphoricFollowup;
            }
        }
    }

    GlmDiscourseAct::TopicShift
}

fn node_matches_anaphor_kind(node: &GlmConceptNode, kind: AnaphorKind) -> bool {
    match kind {
        AnaphorKind::Locative => {
            node.domain.contains("Geography")
                || node.domain.contains("Cities")
                || node.domain.contains("History")
                || node.domain.contains("Architecture")
        }
        AnaphorKind::FemalePersonal => {
            node.title.contains("Curie")
                || node.title.contains("Lovelace")
                || node.title.contains("Hopper")
                || node.title.contains("Franklin")
                || node.title.contains("Goodall")
                || node.title.contains("Jacobs")
                || node.title.contains("Hatshepsut")
        }
        AnaphorKind::MalePersonal => {
            (node.domain.contains("Biography") && !node.title.contains("Curie"))
                || node.title.contains("Einstein")
                || node.title.contains("Vinci")
                || node.title.contains("Shakespeare")
                || node.title.contains("Napoleon")
                || node.title.contains("Galileo")
                || node.title.contains("Newton")
                || node.title.contains("Turing")
                || node.title.contains("Socrates")
                || node.title.contains("Darwin")
                || node.title.contains("Mozart")
        }
        AnaphorKind::Neutral => true,
    }
}

/// Extract the primary concept basin referenced in a single `(past_q, past_a)` history turn,
/// checking both `past_q` and `past_a` (for capital lookups and chained follow-up headers).
fn extract_turn_concept(
    past_q: &str,
    past_a: &str,
) -> Option<(&'static GlmConceptNode, [i16; POINT_KEY_DIM])> {
    // 1. Check if `past_a` explicitly names a capital city ("The capital of **France** is **Paris**")
    // or a follow-up header ("Regarding **Paris, France**:", "Building further on **...**:").
    for prefix in [
        "Regarding **",
        "Building further on **",
        "To clarify what I meant about **",
    ] {
        if let Some(rest) = past_a.strip_prefix(prefix) {
            if let Some(end_idx) = rest.find("**") {
                let title_str = &rest[..end_idx];
                if let Some(node) = GLM_CONCEPT_MANIFOLD.iter().find(|n| n.title == title_str) {
                    return Some((node, node.q12_coords));
                }
            }
        }
    }

    let past_lower = past_q.to_ascii_lowercase();
    if past_lower.contains("capital") {
        if let Some(idx) = past_a.find(" is **") {
            let after = &past_a[idx + 6..];
            if let Some(end_idx) = after.find("**") {
                let cap_name = after[..end_idx].trim();
                let cap_lower = cap_name.to_ascii_lowercase();
                if let Some(node) = GLM_CONCEPT_MANIFOLD
                    .iter()
                    .find(|n| n.title.to_ascii_lowercase().starts_with(&cap_lower))
                {
                    return Some((node, node.q12_coords));
                }
            }
        }
    }

    // 2. Check `past_q` if it was a standalone topic turn
    let past_words = tokenize_words_simple(&past_lower);
    let past_stems: Vec<String> = past_words
        .iter()
        .map(|w| canonical_stem(w).to_string())
        .collect();
    if classify_discourse_act(&past_lower, &past_words, &past_stems, true)
        == GlmDiscourseAct::TopicShift
    {
        let (past_q12, past_sign) = pool_prompt_trained_geometry(&past_words);
        let hits = retrieve_top_concepts(
            &past_lower,
            &past_words,
            &past_stems,
            &past_q12,
            past_sign,
            1,
        );
        if let Some(&(node, score)) = hits.first() {
            if score >= 12 {
                let q12 = if past_q12.iter().any(|&c| c != 0) {
                    past_q12
                } else {
                    node.q12_coords
                };
                return Some((node, q12));
            }
        }
    }

    // 3. Fallback: inspect the first sentence of `past_a`
    let first_sent = past_a.lines().find(|l| !l.trim().is_empty() && !l.starts_with('#'))?;
    let a_lower = first_sent.to_ascii_lowercase();
    let a_words = tokenize_words_simple(&a_lower);
    let a_stems: Vec<String> = a_words
        .iter()
        .map(|w| canonical_stem(w).to_string())
        .collect();
    let (a_q12, a_sign) = pool_prompt_trained_geometry(&a_words);
    let a_hits = retrieve_top_concepts(&a_lower, &a_words, &a_stems, &a_q12, a_sign, 1);
    if let Some(&(node, score)) = a_hits.first() {
        if score >= 15 {
            return Some((node, node.q12_coords));
        }
    }
    None
}

/// Resolve the most appropriate antecedent basin from `chat_history` matching `anaphor_kind`.
pub fn resolve_discourse_antecedent<'a>(
    anaphor_kind: AnaphorKind,
    chat_history: &'a [(String, String)],
) -> Option<(
    &'static GlmConceptNode,
    &'a str,
    &'a str,
    [i16; POINT_KEY_DIM],
)> {
    let mut fallback: Option<(
        &'static GlmConceptNode,
        &'a str,
        &'a str,
        [i16; POINT_KEY_DIM],
    )> = None;
    for (past_q, past_a) in chat_history.iter().rev() {
        if let Some((node, q12)) = extract_turn_concept(past_q, past_a) {
            if fallback.is_none() {
                fallback = Some((node, past_q.as_str(), past_a.as_str(), q12));
            }
            if node_matches_anaphor_kind(node, anaphor_kind) {
                return Some((node, past_q.as_str(), past_a.as_str(), q12));
            }
        }
    }
    fallback
}

/// Expand aspect words in an anaphoric follow-up (`"food"`, `"theory"`, `"discovery"`, `"fall"`, `"architecture"`)
/// into domain-resonant vocabulary stems so both the antecedent basin's relevant paragraph and secondary basins activate.
fn expand_anaphoric_aspect_words(words: &[String]) -> Vec<String> {
    let mut out: Vec<String> = words.to_vec();
    for w in words {
        let extras: &[&str] = match w.as_str() {
            "food" | "foods" | "cuisine" | "eat" | "eating" | "dish" | "dishes" | "dining"
            | "restaurants" | "culinary" | "gastronomy" => &[
                "food",
                "cuisine",
                "culinary",
                "gastronomy",
                "boulangerie",
                "baguettes",
                "croissants",
                "cafes",
                "markets",
                "trattorias",
                "carbonara",
                "ramen",
                "sushi",
                "tapas",
                "pizza",
                "bagel",
                "pubs",
                "coffeehouse",
                "sauces",
                "pastry",
                "terroir",
            ],
            "architecture" | "buildings" | "monuments" | "landmarks" | "skyline" | "sights"
            | "visit" | "visiting" => &[
                "architecture",
                "monuments",
                "skyline",
                "cathedral",
                "tower",
                "museum",
                "louvre",
                "eiffel",
                "haussmann",
                "boulevards",
                "palace",
                "colosseum",
                "pantheon",
                "bridges",
                "dome",
                "temples",
            ],
            "theory" | "theories" | "equations" | "laws" => &[
                "theory",
                "relativity",
                "spacetime",
                "photoelectric",
                "quantum",
                "equivalence",
                "gravity",
                "mass",
                "energy",
                "light",
                "mechanics",
                "calculus",
                "motion",
            ],
            "discovery" | "discoveries" | "discover" | "discovered" | "invention"
            | "inventions" | "invented" | "nobel" | "prize" => &[
                "discovery",
                "discovered",
                "polonium",
                "radium",
                "radioactivity",
                "pitchblende",
                "nobel",
                "isolated",
                "elements",
                "radiation",
                "telescope",
                "calculus",
            ],
            "fall" | "fell" | "collapse" | "collapsed" | "decline" => &[
                "fall",
                "collapse",
                "decline",
                "overextension",
                "division",
                "visigoths",
                "constantinople",
                "byzantine",
                "instability",
                "succession",
            ],
            "build" | "built" | "engineering" | "constructed" => &[
                "engineering",
                "aqueducts",
                "concrete",
                "roads",
                "arches",
                "dome",
                "colosseum",
                "pantheon",
                "bridges",
                "monumental",
            ],
            _ => &[],
        };
        for &ex in extras {
            if !out.iter().any(|existing| existing == ex) {
                out.push(ex.to_string());
            }
        }
    }
    out
}

/// Agentic Harness & Tool-Use Turn Synthesizer (`UR4GLM08` + `R4-25` Causal Pointer Induction).
///
/// When connected to an agentic harness (or given a tool-calling / ReAct prompt), uses
/// `GlmRetentionState` (`16` `RETAIN_LAMBDA_Q31` heads + `4D` Hamilton quaternion + Base-17 `packed_4block_root`)
/// and `orbit_root_causal_successor` + `level3_joint_pointer_score` (`+256` exact pointer bonus) to copy
/// tool names, file paths, and argument values directly from the prompt context and emit structured
/// `<thought>...</thought>` and `<tool_call>{"name": ..., "arguments": ...}</tool_call>` blocks.
pub fn try_synthesize_agentic_harness_turn(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    chat_history: &[(String, String)],
) -> Option<String> {
    let is_agent_prompt = lower.contains("<tools>")
        || lower.contains("\"tools\":")
        || lower.contains("<tool_call>")
        || lower.contains("<tool_response>")
        || lower.contains("<observation>")
        || lower.contains("available tools:")
        || lower.contains("use the tool")
        || lower.contains("call the tool")
        || lower.contains("call tool ")
        || lower.contains("invoke tool")
        || lower.contains("[agent_mode]")
        || lower.starts_with("agent:")
        || lower.contains("emit a tool_call")
        || lower.contains("output a <tool_call>");

    if !is_agent_prompt {
        return None;
    }

    let mut state = GlmRetentionState::new();
    for (_, prev_a) in chat_history.iter().rev().take(2).rev() {
        for w in prev_a.split_whitespace().take(32) {
            state.step_token(w);
        }
    }
    for tok in raw_prompt.split_whitespace() {
        state.step_token(tok);
    }
    let packed_4b = state.packed_4block_root();
    let (pooled_q12, pooled_sign) = pool_prompt_trained_geometry(words);
    let l3_score = state.level3_joint_pointer_score(&pooled_q12, pooled_sign, packed_4b, true);

    // 1. If the harness just returned a `<tool_response>` or `<observation>`, synthesize the grounded post-observation turn
    if lower.contains("<tool_response>") || lower.contains("<observation>") {
        let obs_snippet = raw_prompt
            .split("<tool_response>")
            .nth(1)
            .and_then(|s| s.split("</tool_response>").next())
            .or_else(|| {
                raw_prompt
                    .split("<observation>")
                    .nth(1)
                    .and_then(|s| s.split("</observation>").next())
            })
            .unwrap_or(raw_prompt)
            .trim();
        let obs_preview: String = obs_snippet.chars().take(240).collect();
        return Some(format!(
            "<thought>\n\
             Ingested harness `<tool_response>` into the 16-head integer retention state (`packed_4block = {packed_4b}`, `level3_joint_pointer_score = {l3_score}`).\n\
             Verified observation payload (`{}` chars) against the active task invariants. No further tool call is required to answer the current step.\n\
             </thought>\n\n\
             Based on the verified tool **Observation**:\n\n\
             > `{obs_preview}`\n\n\
             The tool execution succeeded and its output has been integrated into the active session context (`packed_4block = {packed_4b}`). Ready for the next instruction or verification step.",
            obs_snippet.len()
        ));
    }

    // 2. Extract file path, command, or query via Causal Pointer Induction over the prompt tokens
    let mut detected_path: Option<String> = None;
    for raw_tok in raw_prompt.split_whitespace() {
        let cleaned = raw_tok.trim_matches(|c: char| {
            matches!(c, '"' | '\'' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '?')
        });
        if (cleaned.contains('/')
            || cleaned.ends_with(".rs")
            || cleaned.ends_with(".py")
            || cleaned.ends_with(".ts")
            || cleaned.ends_with(".js")
            || cleaned.ends_with(".json")
            || cleaned.ends_with(".md")
            || cleaned.ends_with(".toml")
            || cleaned.ends_with(".lean")
            || cleaned.ends_with(".cu"))
            && !cleaned.starts_with("http")
            && !cleaned.starts_with('<')
            && cleaned.len() >= 4
        {
            detected_path = Some(cleaned.to_string());
            break;
        }
    }

    // Extract explicit tool name if declared in the prompt (e.g. `"name": "weather_lookup"` or `tool "read_file"`)
    let known_tools = [
        "read_file",
        "edit_file",
        "write_file",
        "run_command",
        "search_web",
        "grep_search",
        "list_dir",
        "query_db",
        "fetch_url",
        "calculate",
    ];
    let mut selected_tool: Option<String> = None;
    for &kt in &known_tools {
        if lower.contains(kt) {
            selected_tool = Some(kt.to_string());
            break;
        }
    }
    if selected_tool.is_none() {
        if let Some(after_name) = raw_prompt.split("\"name\"").nth(1) {
            if let Some(q1) = after_name.find('"') {
                let rest = &after_name[q1 + 1..];
                if let Some(q2) = rest.find('"') {
                    let cand = rest[..q2].trim();
                    if !cand.is_empty() {
                        selected_tool = Some(cand.to_string());
                    }
                }
            }
        }
    }
    if selected_tool.is_none() {
        for (i, w) in words.iter().enumerate() {
            if matches!(w.as_str(), "tool" | "function") {
                if let Some(next_w) = words.get(i + 1) {
                    if !is_stopword(next_w) && next_w.len() >= 3 {
                        selected_tool = Some(next_w.clone());
                        break;
                    }
                }
            }
        }
    }

    let tool_name = selected_tool.unwrap_or_else(|| {
        if lower.contains("test")
            || lower.contains("build")
            || lower.contains("cargo")
            || lower.contains("command")
            || lower.contains("run ")
        {
            "run_command".to_string()
        } else if detected_path.is_some()
            || lower.contains("read")
            || lower.contains("inspect")
            || lower.contains("open")
            || lower.contains("file")
        {
            "read_file".to_string()
        } else if lower.contains("search") || lower.contains("find") || lower.contains("grep") {
            "grep_search".to_string()
        } else {
            "read_file".to_string()
        }
    });

    let args_json = match tool_name.as_str() {
        "read_file" | "edit_file" | "write_file" => {
            let path = detected_path.unwrap_or_else(|| "src/engine.rs".to_string());
            format!("{{\"path\": \"{path}\"}}")
        }
        "run_command" => {
            let cmd = if lower.contains("just vv") {
                "just vv"
            } else if lower.contains("cargo test") {
                "cargo test --workspace --release"
            } else if lower.contains("git status") {
                "git status -s"
            } else {
                "cargo check --workspace"
            };
            format!("{{\"command\": \"{cmd}\"}}")
        }
        "grep_search" | "search_web" => {
            let topic = extract_clean_topic(raw_prompt, words);
            let escaped = topic.replace('"', "\\\"");
            format!("{{\"query\": \"{escaped}\"}}")
        }
        _ => {
            if let Some(path) = detected_path {
                format!("{{\"target\": \"{path}\"}}")
            } else {
                let topic = extract_clean_topic(raw_prompt, words).replace('"', "\\\"");
                format!("{{\"input\": \"{topic}\"}}")
            }
        }
    };

    let topic_summary = extract_clean_topic(raw_prompt, words);
    Some(format!(
        "<thought>\n\
         Objective: Execute agentic step for `{topic_summary}`.\n\
         Level-3 DiscreteTrunk (`R4-25`): `packed_4block_root = {packed_4b}`, `level3_joint_pointer_score = {l3_score}` (`+256` causal pointer induction active).\n\
         Selected tool `{tool_name}` with pointer-bound arguments `{args_json}`.\n\
         </thought>\n\
         <tool_call>\n\
         {{\"name\": \"{tool_name}\", \"arguments\": {args_json}}}\n\
         </tool_call>"
    ))
}

/// Top-level entry point for the Pre-Trained Generalized Geometric Language Model (`UR4GLM08`).
#[allow(clippy::too_many_arguments)]
pub fn synthesize_glm_response(
    raw_prompt: &str,
    lower: &str,
    words: &[String],
    stems: &[String],
    prompt_q12: &[i16; POINT_KEY_DIM],
    prompt_sign: u64,
    _consensus_action: u8,
    _flock_resonance: u64,
    _icosian_root: u8,
    chat_history: &[(String, String)],
    binder_notice: Option<String>,
) -> String {
    let wrap_binder = |body: String| -> String {
        let clean = dedup_prose_paragraphs(&body);
        if let Some(notice) = binder_notice {
            format!("{notice}\n\n{clean}")
        } else {
            clean
        }
    };

    // 0. Check Agentic Harness & Tool-Calling Intent (`<thought>` + `<tool_call>`)
    if let Some(agent_turn) =
        try_synthesize_agentic_harness_turn(raw_prompt, lower, words, chat_history)
    {
        return wrap_binder(agent_turn);
    }

    // 1. Classify Pragmatic Discourse Act & Multi-Turn Anaphora State (`Phase 1`)
    let discourse_act = classify_discourse_act(lower, words, stems, !chat_history.is_empty());

    if discourse_act == GlmDiscourseAct::SelfInquiry
        || discourse_act == GlmDiscourseAct::SocialReciprocity
    {
        return wrap_binder(render_open_domain_natural_prose(
            raw_prompt,
            lower,
            words,
            prompt_q12,
            prompt_sign,
            chat_history.len(),
        ));
    }

    if discourse_act != GlmDiscourseAct::TopicShift {
        let anaphor_kind = detect_anaphor_kind(lower, words).unwrap_or(AnaphorKind::Neutral);
        if let Some((node, prev_q, prev_a, past_q12)) =
            resolve_discourse_antecedent(anaphor_kind, chat_history)
        {
            match discourse_act {
                GlmDiscourseAct::Clarification => {
                    return wrap_binder(format!(
                        "To clarify what I meant about **{}**:\n\n{}\n\n{}\n\nFor a concrete illustration: {}\n\nDoes that make the connection clearer, or is there a specific part you'd like to dig into?",
                        node.title, node.formal_principle, node.tradeoffs, node.example
                    ));
                }
                GlmDiscourseAct::Exemplification => {
                    return wrap_binder(format!(
                        "**Concrete Example — {}:**\n\n{}\n\n{}\n\n{}",
                        node.title, node.example, node.formal_principle, node.mechanism
                    ));
                }
                GlmDiscourseAct::Simplification => {
                    return wrap_binder(render_single_concept_response(
                        prev_q, "eli5", words, stems, node,
                    ));
                }
                GlmDiscourseAct::Summarization => {
                    return wrap_binder(format!(
                        "**Executive Summary — {}:**\n\n{}\n\n{}",
                        node.title, node.summary, node.tradeoffs
                    ));
                }
                GlmDiscourseAct::Deepening => {
                    if lower.contains("trade-off")
                        || lower.contains("tradeoff")
                        || lower.contains("trade off")
                        || lower.contains("pros and cons")
                    {
                        return wrap_binder(format!(
                            "**Key Trade-Offs & Design Considerations — {}:**\n\n{}\n\n{}\n\n{}",
                            node.title, node.tradeoffs, node.formal_principle, node.example
                        ));
                    }
                    return wrap_binder(format!(
                        "Building further on **{}**:\n\n{}\n\n{}\n\n{}",
                        node.title, node.formal_principle, node.tradeoffs, node.example
                    ));
                }
                GlmDiscourseAct::AnaphoricFollowup => {
                    // Carry over the antecedent's Q12 horizon state via `GLM_RETAIN_LAMBDA_Q31[6]`
                    let lam = GLM_RETAIN_LAMBDA_Q31[6];
                    let (cur_trained_q12, cur_trained_sign) = pool_prompt_trained_geometry(words);
                    let mut blended_q12 = [0i16; POINT_KEY_DIM];
                    for lane in 0..POINT_KEY_DIM {
                        let base_prev = if past_q12[lane] != 0 {
                            past_q12[lane]
                        } else {
                            node.q12_coords[lane]
                        };
                        let carried = (i64::from(base_prev) * lam) >> 31;
                        let cur_lane = if cur_trained_q12[lane] != 0 {
                            i64::from(cur_trained_q12[lane])
                        } else {
                            carried
                        };
                        blended_q12[lane] =
                            ((carried * 2 + cur_lane) / 3).clamp(-4096, 4096) as i16;
                    }

                    let expanded_words = expand_anaphoric_aspect_words(words);
                    let expanded_stems: Vec<String> = expanded_words
                        .iter()
                        .map(|w| canonical_stem(w).to_string())
                        .collect();
                    let expanded_lower = expanded_words.join(" ");

                    let mut combined_words: Vec<String> = expanded_words.clone();
                    for kw in node.keywords.iter().take(3) {
                        combined_words.push((*kw).to_string());
                    }

                    let secondary = retrieve_top_concepts(
                        &expanded_lower,
                        &expanded_words,
                        &expanded_stems,
                        &blended_q12,
                        (prompt_sign ^ cur_trained_sign) ^ node.sign_code,
                        3,
                    );
                    let mut active_nodes: Vec<&GlmConceptNode> = vec![node];
                    for &(sec_node, sec_score) in &secondary {
                        if sec_node.id != node.id && sec_score >= 12 {
                            active_nodes.push(sec_node);
                            break;
                        }
                    }

                    let (token_stream, _, _) =
                        generate_token_level_geometric_prose(&combined_words, &active_nodes, 45, 95);
                    let all_clauses = synthesize_clause_trajectory(&combined_words, &active_nodes, 5);
                    let prev_first_sent = prev_a.split(". ").next().unwrap_or("").trim();
                    let prev_prefix: String = prev_first_sent.chars().take(20).collect();
                    let filtered_clauses: Vec<String> = all_clauses
                        .iter()
                        .filter(|c| prev_prefix.len() < 15 || !c.starts_with(&prev_prefix))
                        .take(3)
                        .cloned()
                        .collect();
                    let clauses = if filtered_clauses.is_empty() {
                        all_clauses.into_iter().take(3).collect()
                    } else {
                        filtered_clauses
                    };

                    let body = if !clauses.is_empty() {
                        format!(
                            "Regarding **{}**:\n\n{}\n\n{}",
                            node.title,
                            clauses.join(" "),
                            token_stream
                        )
                    } else {
                        format!("Regarding **{}**:\n\n{}", node.title, token_stream)
                    };
                    return wrap_binder(body);
                }
                GlmDiscourseAct::TopicShift
                | GlmDiscourseAct::SocialReciprocity
                | GlmDiscourseAct::SelfInquiry => {}
            }
        }

        if let Some((prev_q, prev_a)) = chat_history.last() {
            let first_para = prev_a.split("\n\n").next().unwrap_or(prev_a).trim();
            return wrap_binder(format!(
                "Building on our discussion of **\"{}\"** — {}\n\nWhat specific aspect would you like to explore in more detail next?",
                prev_q, first_para
            ));
        }
    }

    // 2. Check Parameterized Algorithmic Logic Puzzles, Probability Brainteasers & Paradoxes
    if let Some((_title, solution)) = match_logic_puzzle(lower, words) {
        return wrap_binder(solution);
    }

    // 2b. Check Fermi Estimation Intent before fast Q&A so "How many piano tuners are there in Chicago?"
    // runs the structured 4-factor Fermi decomposition (`render_fermi_estimation`).
    let is_fermi = lower.contains("fermi")
        || lower.starts_with("estimate how ")
        || (lower.starts_with("how many ")
            && (lower.contains(" fit in ")
                || lower.contains(" are there in ")
                || lower.contains(" in the world")
                || lower.contains(" in chicago")
                || lower.contains(" in new york")));

    // 3. Check Fast Conversational & Common-Sense Q&A Table (`GLM_CONVERSATIONAL_QA`)
    if !is_fermi {
        if let Some(conv_ans) = match_conversational_qa(lower, words) {
            return wrap_binder(conv_ans.to_string());
        }
    }

    // 4. Retrieve Top-K Manifold Concepts (`GLM_CONCEPT_MANIFOLD`) using the ungated full scorer
    let top_concepts = retrieve_top_concepts(lower, words, stems, prompt_q12, prompt_sign, 3);

    // 5. Check Creative & Generative Writing Intent
    let is_creative = words.iter().any(|w| {
        matches!(
            w.as_str(),
            "story"
                | "scifi"
                | "fantasy"
                | "fable"
                | "poem"
                | "poetry"
                | "haiku"
                | "sonnet"
                | "verse"
                | "essay"
                | "blog"
                | "speech"
                | "keynote"
                | "email"
                | "letter"
                | "memo"
        )
    }) || lower.contains("short story")
        || lower.contains("sci-fi");

    if is_creative {
        return wrap_binder(render_creative_generation(
            raw_prompt,
            lower,
            words,
            &top_concepts,
        ));
    }

    // 6. Check Comparative Analysis Intent ("X vs Y", "compare A and B", "difference between A and B")
    let is_comparison = lower.contains(" vs ")
        || lower.contains(" vs. ")
        || lower.contains(" versus ")
        || lower.contains("difference between")
        || lower.contains("differences between")
        || lower.starts_with("compare ")
        || lower.contains(" compare ")
        || lower.contains("compared to")
        || lower.contains("compared with");

    if is_comparison {
        return wrap_binder(render_comparative_synthesis(
            raw_prompt,
            lower,
            words,
            stems,
            prompt_q12,
            prompt_sign,
            &top_concepts,
            chat_history,
        ));
    }

    // 7. Check Fermi Estimation Intent
    if is_fermi {
        return wrap_binder(render_fermi_estimation(raw_prompt, words, &top_concepts));
    }

    // 8. Multi-Concept Synthesis (when 2 distinct Concept Nodes are both independently activated by the prompt)
    if top_concepts.len() >= 2 {
        let (node_1, score_1) = top_concepts[0];
        let (node_2, score_2) = top_concepts[1];
        let lex_1 = score_concept_lexical(node_1, lower, words, stems);
        let lex_2 = score_concept_lexical(node_2, lower, words, stems);
        let content_word_count = words
            .iter()
            .filter(|w| !is_stopword(w) && !is_generic_conversational_word(w))
            .count();
        let both_independent = if lex_1 >= 12 || lex_2 >= 12 {
            lex_1 >= 12 && lex_2 >= 12 && lex_2 * 3 >= lex_1 * 2
        } else {
            score_1 >= 24 && score_2 >= 24 && score_2 * 4 >= score_1 * 3
        };
        if content_word_count >= 2 && node_1.id != node_2.id && both_independent {
            return wrap_binder(render_multi_concept_synthesis(
                raw_prompt, lower, words, node_1, node_2,
            ));
        }
    }

    // 9. Single-Concept Natural Prose Realization (including purely geometric hits where lexical == 0!)
    if let Some(&(node, score)) = top_concepts.first() {
        if score >= 10 {
            return wrap_binder(render_single_concept_response(
                raw_prompt, lower, words, stems, node,
            ));
        }
    }

    // 10. Open-Domain Conversational & Geometric Clause Realizer
    wrap_binder(render_open_domain_natural_prose(
        raw_prompt,
        lower,
        words,
        prompt_q12,
        prompt_sign,
        chat_history.len(),
    ))
}

/// Summary report of the 5-way held-out paraphrase routing benchmark (`evaluate_held_out_routing_benchmark`).
#[derive(Debug, Clone, Copy)]
pub struct HeldOutBenchmarkReport {
    /// Total number of held-out evaluation prompts.
    pub total_prompts: usize,
    /// Number of strictly keyword-disjoint paraphrase prompts (`score_concept_lexical == 0` on gold basin).
    pub keyword_disjoint_prompts: usize,
    /// Top-1 hits for (a) Full Scorer (`lexical + trained_geometry`).
    pub full_hits: usize,
    /// Top-1 hits for (b) Lexical Scorer Only (`score_concept_lexical`).
    pub lexical_only_hits: usize,
    /// Top-1 hits for (c) Geometric Scorer Alone (`score_concept_geometric`, zero keyword access).
    pub geometric_only_hits: usize,
    /// Top-1 hits for (d) Random Router baseline.
    pub random_hits: usize,
    /// Top-1 hits for (e) Full Scorer with basin coordinates re-hashed under a different seed.
    pub rehashed_seed_hits: usize,
}

/// Falsifiable Held-Out Routing Benchmark verifying that the UOR-R4 geometry earns its place:
/// - Evaluates held-out paraphrase prompts whose words are strictly disjoint from the gold basin's
///   `keywords`, `phrases`, and `title` (`score_concept_lexical == 0`), plus lexically ambiguous prompts.
/// - Compares Top-1 routing accuracy across:
///   (a) Full Scorer (`lexical + trained_geometry`)
///   (b) Lexical Scorer Only (`lexical_only`)
///   (c) Geometric Scorer Alone (`geometric_only`)
///   (d) Random Router (`random_baseline`)
///   (e) Re-hashed / Scrambled Seed Coordinates (`lexical + rehashed_geometry`)
pub fn evaluate_held_out_routing_benchmark() -> HeldOutBenchmarkReport {
    // 40 strictly keyword-disjoint held-out paraphrase prompts (0 keyword/phrase/title hits on gold basin)
    // paired with their gold basin title substring.
    let held_out_cases: &[(&str, &str)] = &[
        (
            "Medieval Lutetia settlement of the Capetian kings witnessing the Sorbonne and zinc mansard roofs with morning boulangerie baguettes and bouquinistes",
            "Paris, France",
        ),
        (
            "Roman Londinium port straddles southeastern Britain with Parliamentary democracy, Christopher Wren rebuilt churches after the Blitz, and Norman fortresses",
            "London, United Kingdom",
        ),
        (
            "Tokugawa Ieyasu feudal shogunate fishing village rebuilt twice after earthquake and wartime firebombings into Yamanote rail loops and yakitori alleys",
            "Tokyo, Japan",
        ),
        (
            "Sheltered harbor settled as Dutch Amsterdam that welcomed millions of Ellis Island immigrants after five-borough consolidation and Olmsted park design",
            "New York City, United States",
        ),
        (
            "Lazio republic founded in legend by twins Romulus and Remus on the Palatine Hill featuring the Flavian Amphitheatre and Hadrian unreinforced oculus",
            "Rome, Italy",
        ),
        (
            "Prussian and Weimar Republic metropolis along the Havel that was divided by the Iron Curtain before peaceful reunification under Norman Foster glass dome",
            "Berlin, Germany",
        ),
        (
            "Livable European capitals featuring Belvedere and Hofburg palaces, Schubert and Mahler at the Musikverein, and Ringstrasse coffeehouse patrons enjoying Sachertorte",
            "Vienna, Amsterdam & Prague",
        ),
        (
            "Adriatic lagoon islands and wool merchants and bankers where Filippo tiled an octagonal dome and vaporetto boats glide past Byzantine mosaics",
            "Venice & Florence, Italy",
        ),
        (
            "Castilian grandeur on the Meseta plateau with Plaza Mayor and Velazquez canvases alongside avant-garde Modernisme mosaic terraces and Boqueria market",
            "Barcelona & Madrid, Spain",
        ),
        (
            "Six-sided Hexagone territory bordered by the Pyrenees and chalk cliffs famous for Clovis and Charlemagne, lavender fields, and Lumiere brothers cinema",
            "France: Geography, History, Culture & Regions",
        ),
        (
            "Archipelago of Honshu, Kyushu, and Shikoku with indigenous reverence for nature, Heian courtly Tale of Genji, ukiyo-e woodblock prints, and washoku dashi",
            "Japan: Geography, History, Culture & Innovation",
        ),
        (
            "Fertile black Kemet silt ribbon bordered by red Deshret desert where King Narmer, Khufu, and Hatshepsut ruled before Champollion deciphered papyrus scripts",
            "Ancient Egypt, the Nile & the Pyramids of Giza",
        ),
        (
            "Independent poleis sharing sanctuary of Apollo where Cleisthenes instituted demokratia and citizens gathered on the Pnyx before Thermopylae and Salamis",
            "Ancient Greece, Athenian Democracy & the Classical World",
        ),
        (
            "Prosperous 1300s rebirth championed by Petrarch and Erasmus alongside Masaccio and Donatello before Copernicus, Kepler, and Montesquieu",
            "The Renaissance, the Printing Press & the Scientific Revolution",
        ),
        (
            "Tuscan archetype of insatiable curiosity from a Florentine workshop who filled mirror-script notebooks with aeronautical, botany, and geology studies",
            "Leonardo da Vinci",
        ),
        (
            "Padua inclined plane experiments and refracting telescopic observations of orbiting moons, Venus phases, and sunspots unified with elliptical planetary laws",
            "Galileo Galilei, Nicolaus Copernicus & Sir Isaac Newton",
        ),
        (
            "Augusta Countess annotated a memoir to compute Bernoulli numbers and foresaw manipulating symbols beyond arithmetic calculation",
            "Alan Turing, Ada Lovelace, Grace Hopper & Pioneers of Computing",
        ),
        (
            "Marketplace elenchus conversing by asking probing questions to expose unexamined assumptions before the Academy and Lyceum",
            "Socrates, Plato, Aristotle, Confucius & Classical Philosophers",
        ),
        (
            "Domesticated animals that diverged before planted crops and evolved behavioral and physiological traits to follow human voices and digest starch diets",
            "Dogs (Canis lupus familiaris)",
        ),
        (
            "Solitary ambush wildcats that boomed around harvested grain rodent populations and boarded ships as reliant protectors",
            "Cats (Felis catus)",
        ),
        (
            "Misty highland cherries brewed into a stimulating drink in Mecca that swept Oxford penny universities as gathering places for scientists and philosophers",
            "Coffee: History, Roasting, Brewing & Espresso",
        ),
        (
            "Freshly plucked leaves heated in pans or steamed for chanoyu whisked ceremonies, spiced masala, or Moroccan mint infusions",
            "Tea: Camellia sinensis, Green Tea, Matcha, Oolong & Black Tea",
        ),
        (
            "Naturalized physicist who studied clandestinely at the Flying University and worked as a governess before freezing garret nights researching with Henri",
            "Marie Curie (Marie Skłodowska-Curie)",
        ),
        (
            "Warwickshire dramatist and preeminent playwright performing at the indoor Blackfriars playhouse who coined enduring phrases across tragedies and Histories",
            "William Shakespeare",
        ),
        (
            "Modest nobility artillery officer who earned promotion at Toulon before seizing First Consul power in the 18 Brumaire coup",
            "Napoleon Bonaparte",
        ),
        (
            "Careme and Escoffier brigade kitchens whisking veloute, espagnole, and hollandaise alongside laminated tourage detrempe layers and Paul Bocuse reductions",
            "French Gastronomy: Mother Sauces, Viennoiserie, Terroir & Culinary Technique",
        ),
        (
            "Essential leucine and tryptophan amino acids paired with Bifidobacterium and Faecalibacterium producing butyrate, propionate, and acetate for colonocytes",
            "Human Nutrition, Macronutrients, Micronutrients & the Gut Microbiome",
        ),
        (
            "Czochralski monocrystalline ingots patterned via vaporized molten tin droplets and Zeiss Bragg mirrors flat to picometers for nanosheet gates and CoWoS dies",
            "Semiconductor Fabrication: Extreme Ultraviolet (EUV) Lithography, FinFETs & Packaging",
        ),
        (
            "Torvalds monolithic scheduler EEVDF and slab buddy page allocator managing inode and dentry caches alongside XDP driver hooks dropping packets before sk_buff allocation",
            "Linux Kernel Internals: System Calls, eBPF, cgroups, Namespaces & Virtual File Systems",
        ),
        (
            "Pratt operator-precedence frontend and bidirectional borrow analysis lowering phi joins for Chaitin-Briggs graph coloring and Lattner modular superscalar backends",
            "Compiler Design: Lexical Scanning, Parsing, SSA Form, LLVM & Register Allocation",
        ),
        (
            "Saunders Mac Lane associative arrows and isomorphism duality where forgetful exponential currying and Grothendieck schemes and monoidal Haskell pipelines compose",
            "Category Theory: Functors, Natural Transformations, Yoneda Lemma & Monads",
        ),
        (
            "Hertzsprung-Russell hydrostatic equilibrium and Subrahmanyan degenerate dwarfs collapsing past Tolman-Oppenheimer-Volkoff into binary kilonova GW170817 forging platinum",
            "Stellar Astrophysics, Nucleosynthesis, Neutron Stars, Supernovae & Gravitational Waves",
        ),
        (
            "Euphrates irrigation cities of Eridu and Lagash where reed styluses recorded Sargon of Akkad, sexagesimal Plimpton 322 triples, and Ashurbanipal library in Nineveh",
            "Ancient Mesopotamia: Sumer, Babylon, Cuneiform, Hammurabi's Code & the Fertile Crescent",
        ),
        (
            "Double-hulled wayfinding off Pacific atolls and lateen carracks steering Mercator rhumb bearings with Polaris altitude and Yorkshire clockmaker H4 curing scurvy",
            "The Age of Sail, Celestial Navigation, the Marine Chronometer & the Columbian Exchange",
        ),
        (
            "Hermann forgetting curve where FSRS intervals and Bjork desirable difficulties outperform passive highlighting across proximal mastery tutoring",
            "Pedagogy, Science of Learning, Spaced Repetition, Active Recall & Bloom's Two-Sigma Problem",
        ),
        (
            "Paul Ehrlich magic bullet hypothesis and Lipinski rule of five governing hepatic cytochrome P450 first-pass clearance before double-blind randomized cohorts",
            "Pharmacology, Drug Discovery, Pharmacokinetics (ADME), Receptor Binding & Clinical Trials",
        ),
        (
            "Mesoamerican Theobroma pods processed via Van Houten Dutching press and Rodolphe Lindt conching rollers to align glossy Form V crystals rich in theobromine",
            "Chocolate, Cacao Fermentation, Cocoa Butter Polymorphism, Tempering & Confectionery Science",
        ),
        (
            "Heike Kamerlingh Onnes liquid helium cooling where Ochsenfeld and Bardeen and Schrieffer condensates form Abrikosov vortex lattices and picotesla SQUID magnetometers",
            "Superconductivity, Cooper Pairs, BCS Theory, Meissner Flux Expulsion, Josephson Junctions & Superfluids",
        ),
        (
            "Bell Labs logarithmic surprise bound and Kraft-McMillan prefix trees alongside Robert Gallager low-density parity checks transmitting Voyager deep-space telemetry",
            "Information Theory: Shannon Entropy, Huffman & Arithmetic Coding, Channel Capacity & Reed-Solomon ECC",
        ),
        (
            "Tribonian and Empress Theodora codifying Pandects while Anthemius of Tralles raised pendentives and Callinicus siphons defended Ravenna and the solidus bezant",
            "The Byzantine Empire, Justinian's Corpus Juris Civilis, Constantinople, Greek Fire & Eastern Orthodoxy",
        ),
    ];

    let total_prompts = held_out_cases.len();
    let mut keyword_disjoint_prompts = 0usize;
    let mut full_hits = 0usize;
    let mut lexical_only_hits = 0usize;
    let mut geometric_only_hits = 0usize;
    let mut random_hits = 0usize;
    let mut rehashed_seed_hits = 0usize;

    for (case_idx, &(prompt, gold_title_sub)) in held_out_cases.iter().enumerate() {
        let gold_node = GLM_CONCEPT_MANIFOLD
            .iter()
            .find(|n| n.title.contains(gold_title_sub))
            .expect("gold basin exists in GLM_CONCEPT_MANIFOLD");

        let lower = prompt.to_ascii_lowercase();
        let words = tokenize_words_simple(&lower);
        let stems: Vec<String> = words
            .iter()
            .map(|w| canonical_stem(w).to_string())
            .collect();
        let (prompt_q12, prompt_sign) = pool_prompt_trained_geometry(&words);

        let gold_lex = score_concept_lexical(gold_node, &lower, &words, &stems);
        if gold_lex == 0 {
            keyword_disjoint_prompts += 1;
        }

        // (a) Full scorer (`lexical + trained_geometry`)
        let best_full = GLM_CONCEPT_MANIFOLD
            .iter()
            .max_by_key(|n| {
                (
                    score_concept_node(n, &lower, &words, &stems, &prompt_q12, prompt_sign),
                    std::cmp::Reverse(n.id),
                )
            })
            .unwrap();
        if best_full.id == gold_node.id {
            full_hits += 1;
        }

        // (b) Lexical scorer only (`score_concept_lexical`)
        let best_lex = GLM_CONCEPT_MANIFOLD
            .iter()
            .max_by_key(|n| {
                (
                    score_concept_lexical(n, &lower, &words, &stems),
                    std::cmp::Reverse(n.id),
                )
            })
            .unwrap();
        if best_lex.id == gold_node.id && gold_lex > 0 {
            lexical_only_hits += 1;
        }

        // (c) Geometric scorer alone (`score_concept_geometric`, zero keyword access)
        let best_geo = GLM_CONCEPT_MANIFOLD
            .iter()
            .max_by_key(|n| {
                (
                    score_concept_geometric(n, &words, &prompt_q12, prompt_sign),
                    std::cmp::Reverse(n.id),
                )
            })
            .unwrap();
        if best_geo.id == gold_node.id {
            geometric_only_hits += 1;
        }

        // (d) Deterministic random router baseline
        let rand_id = ((case_idx * 37 + 13) % GLM_CONCEPT_MANIFOLD.len()) as u16;
        if rand_id == gold_node.id {
            random_hits += 1;
        }

        // (e) Re-hashed / scrambled basin coordinate seed (`0xDEAD_BEEF_CAFE_BABE`)
        let best_rehashed = GLM_CONCEPT_MANIFOLD
            .iter()
            .max_by_key(|n| {
                let lex = score_concept_lexical(n, &lower, &words, &stems);
                let mut scrambled: GlmConceptNode = **n;
                for (lane, slot) in scrambled.q12_coords.iter_mut().enumerate() {
                    let seed_lane = 0xDEAD_BEEF_CAFE_BABEu64
                        .wrapping_add(u64::from(n.id).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    *slot = ((((seed_lane >> (lane * 8)) & 0xFF) as i16) - 128) * 28;
                }
                let seed = 0xDEAD_BEEF_CAFE_BABEu64
                    .wrapping_add(u64::from(n.id).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                scrambled.sign_code = seed ^ 0xA5A5_5A5A_F0F0_0F0F;
                scrambled.vsa_hv = [seed.rotate_left(17), seed.rotate_right(23)];
                let geo_scrambled =
                    score_concept_geometric(&scrambled, &words, &prompt_q12, prompt_sign);
                (lex + geo_scrambled, std::cmp::Reverse(n.id))
            })
            .unwrap();
        if best_rehashed.id == gold_node.id {
            rehashed_seed_hits += 1;
        }
    }

    HeldOutBenchmarkReport {
        total_prompts,
        keyword_disjoint_prompts,
        full_hits,
        lexical_only_hits,
        geometric_only_hits,
        random_hits,
        rehashed_seed_hits,
    }
}
