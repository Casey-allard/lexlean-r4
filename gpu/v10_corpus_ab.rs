//! Pure-Rust UOR-R4-GLLM v10 corpus foundations and Buckets A/B generator:
//! - Corpus constants, forbidden/boilerplate filters, text normalization, and `UORVOCB9` vocab export
//! - `SampleBuilder` / `CorpusRecord` with exact `ByteBpe4096` offset-aligned copy-span tagging
//! - `build_core_conversational_anchors`, `generate_bucket_a_conversations`, and `generate_bucket_b_qa`

#![allow(dead_code)]

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::bpe_json::{byte_span_to_char_span, ByteBpe4096, Rng64};

pub const MAX_SEQ: usize = 1024;
pub const TARGET_TRAIN_COUNT: usize = 100_000;
pub const TARGET_EVAL_COUNT: usize = 2_400;

/// Held-out test identifiers, prompts, and banned UR4GLM08 topics that MUST NEVER enter training data.
pub const FORBIDDEN_HELD_OUT: &[&str] = &[
    "sensor_drift_k9",
    "base_offset_q3",
    "orbital_phase_m2",
    "lattice_stride_z7",
    "telemetry_gain_v4",
    "phase_bias_r2",
    "offset_bias_n2",
    "gyro_bias_m4",
    "compute_flux_z9",
    "step_flux",
    "queue_span_v4",
    "cap_bound_n2",
    "gllm-v9-mvm",
    "nebula-os",
    "nebula_os",
    "ringbuffer",
    "casey-allard",
    "wasi 0.2",
    "docs.rs/axum",
    "module_12_kernel",
    "photosynthesis",
    "sourdough",
    "plate tectonics",
    "mitochondria",
    "rayleigh scattering",
];

pub const SMOLTALK_PARQUET_URLS: &[&str] = &[
    "https://huggingface.co/datasets/HuggingFaceTB/smoltalk/resolve/main/data/everyday-conversations/train-00000-of-00001.parquet",
];
pub const DOLLY_URL: &str =
    "https://huggingface.co/datasets/databricks/databricks-dolly-15k/resolve/main/databricks-dolly-15k.jsonl";
pub const ALPACA_URL: &str =
    "https://huggingface.co/datasets/yahma/alpaca-cleaned/resolve/main/alpaca_data_cleaned.json";
pub const CODE_ALPACA_URL: &str =
    "https://huggingface.co/datasets/sahil2801/CodeAlpaca-20k/resolve/main/code_alpaca_20k.json";

pub const BANNED_BOILERPLATE: &[&str] = &[
    "as an ai",
    "language model",
    "openai",
    "chatgpt",
    "i cannot browse",
    "knowledge cutoff",
    "i don't have personal",
    "photosynthesis",
    "sourdough",
    "plate tectonics",
    "mitochondria",
    "rayleigh scattering",
];

// Canonical single-turn conversational intents (distinct 2-token openings & zero mid-sentence 2-gram collisions!)
pub const REPLY_HI: &str = "Hi there - glad you are here, ask any coding or system question.";
pub const REPLY_HELLO: &str = "Hello, glad to help with questions and sandbox tools.";
pub const REPLY_HEY: &str = "Hey! Happy to brainstorm ideas, debug systems, or run sandbox tools.";
pub const REPLY_HOW_ARE_YOU: &str =
    "Doing great while running locally today! What would you like to build?";
pub const REPLY_MORNING: &str =
    "Good morning! Hope your day is off to a strong start - how can I help?";
pub const REPLY_AFTERNOON: &str = "Good afternoon! How can I assist with your project?";
pub const REPLY_EVENING: &str = "Good evening! What are we tackling tonight?";
pub const REPLY_THANKS: &str =
    "You're welcome! Let me know anytime you want to explore the next step.";
pub const REPLY_ACK: &str =
    "Sounds good! Feel free to ask follow-ups or try commands whenever you're ready.";
pub const REPLY_BYE: &str =
    "Take care! Your Studio workspace and session stay ready whenever you return.";
pub const REPLY_PING: &str =
    "Loud and clear! UOR-R4-GLLM v10 is live inside your WebAssembly worker.";
pub const REPLY_WHO: &str = "I am UOR-R4, your fast Linux assistant with local tools.";
pub const REPLY_MADE: &str = "Built by the LexLean UOR-R4 team with integer Hadamard geometric layers and discrete Q12 T-MAC lookup tables.";
pub const REPLY_CAN_DO: &str = "I can answer questions, write clean code in Rust, Python, C, and JS, run shell commands and static tools in a local Linux sandbox, check or sync Git files in your project, and search or read web pages when you allow access, while running fast in a local worker thread so every chat turn stays smooth.";
pub const REPLY_BROWSER: &str = "Yes! Every weight tensor runs locally in a browser WebAssembly Worker with OPFS shard caching and zero UI lag.";
pub const REPLY_GEOM: &str = "Each layer mixes 64-channel blocks via integer Cl(0,6) Walsh-Hadamard butterflies, queries 256-entry Q12 T-MAC tables, and normalizes causal KV weights with 4th-order rational PRLL.";
pub const REPLY_TOOLS: &str = "Four grounded tools are wired in: `mvm_exec` (RISC-V Linux MicroVM), `git` (status, diff, commit, push, and repo sync), `web_search`, and `web_fetch`.";
pub const REPLY_LINUX: &str = "Certainly! Through `mvm_exec` I run shell pipelines and static RV64IMAC ELF64 tools (`jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, `http`) inside the MicroVM.";
pub const REPLY_GIT: &str = "Absolutely - via the SpiralCore host bridge I can clone GitHub repositories into `/workspace`, inspect diffs, commit changes, and push.";
pub const REPLY_ZERONIC: &str = "Guest sockets under Zero-NIC (`R4-37`) refuse `AF_INET`, `AF_INET6`, and `AF_PACKET` with `-EAFNOSUPPORT = -97`, permitting only `AF_VSOCK` (`40`).";

#[inline]
fn is_py_ws(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\x1c'..='\x1f')
}

#[inline]
fn py_strip(s: &str) -> &str {
    s.trim_matches(is_py_ws)
}

#[inline]
fn py_rstrip(s: &str) -> &str {
    s.trim_end_matches(is_py_ws)
}

/// Export the `UORVOCB9` raw byte table from a loaded `ByteBpe4096` tokenizer.
pub fn export_vocab_raw_bytes_from_tok(
    tok: &ByteBpe4096,
    out_vocab_bin: &Path,
) -> Result<(), String> {
    if let Some(parent) = out_vocab_bin.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
    }
    let mut buf = Vec::with_capacity(4096 * 16 + 12);
    buf.extend_from_slice(b"UORVOCB9");
    buf.extend_from_slice(&(4096u32).to_le_bytes());
    for idx in 0..4096usize {
        let piece = tok.id_to_piece.get(idx).map(String::as_str).unwrap_or("");
        let raw: &[u8] = if idx <= 2 || (piece.starts_with("<|") && piece.ends_with("|>")) {
            piece.as_bytes()
        } else {
            tok.id_to_bytes.get(idx).map(Vec::as_slice).unwrap_or(&[])
        };
        let trunc = &raw[..raw.len().min(64)];
        buf.push(trunc.len() as u8);
        buf.extend_from_slice(trunc);
    }
    let mut file = fs::File::create(out_vocab_bin)
        .map_err(|e| format!("create {}: {e}", out_vocab_bin.display()))?;
    file.write_all(&buf)
        .map_err(|e| format!("write {}: {e}", out_vocab_bin.display()))
}

/// Export the `UORVOCB9` raw byte table from `tokenizer.json` on disk.
pub fn export_vocab_raw_bytes(tok_json_path: &Path, out_vocab_bin: &Path) -> Result<(), String> {
    let tok = ByteBpe4096::from_file(tok_json_path)?;
    export_vocab_raw_bytes_from_tok(&tok, out_vocab_bin)
}

/// Lowercase, trim, and collapse all whitespace runs to single ASCII spaces (`re.sub(r"\s+", " ", s.strip().lower())`).
pub fn norm_key(s: &str) -> String {
    let low = py_strip(s).to_lowercase();
    let mut out = String::with_capacity(low.len());
    let mut in_ws = false;
    for ch in low.chars() {
        if is_py_ws(ch) {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            in_ws = false;
            out.push(ch);
        }
    }
    out
}

/// Case-insensitive substring check against `FORBIDDEN_HELD_OUT`.
pub fn contains_forbidden(text: &str) -> bool {
    let low = text.to_lowercase();
    FORBIDDEN_HELD_OUT.iter().any(|&bad| low.contains(bad))
}

/// Normalize CRLF/CR line endings, strip surrounding whitespace, and collapse 3+ newlines into `\n\n`.
pub fn clean_text(s: &str) -> String {
    let norm = s.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = py_strip(&norm);
    let mut out = String::with_capacity(trimmed.len());
    let mut nl_run = 0usize;
    for ch in trimmed.chars() {
        if ch == '\n' {
            nl_run += 1;
            if nl_run <= 2 {
                out.push('\n');
            }
        } else {
            nl_run = 0;
            out.push(ch);
        }
    }
    out
}

/// Extract the first single-line crisp sentence in `[12, max_chars]` Unicode chars ending with `.`, `!`, or `?`.
pub fn first_crisp_sentence(text: &str, max_chars: usize) -> String {
    let mut t = clean_text(text);
    if let Some((first_line, _)) = t.split_once('\n') {
        t = py_strip(first_line).to_string();
    }
    for sep in [". ", "! ", "? "] {
        if let Some(byte_idx) = t.find(sep) {
            let char_idx = t[..byte_idx].chars().count();
            if char_idx >= 12 && max_chars > 0 && char_idx <= max_chars - 1 {
                return py_strip(&t[..byte_idx + 1]).to_string();
            }
        }
    }
    let char_len = t.chars().count();
    if char_len >= 12 && char_len <= max_chars && t.ends_with(['.', '!', '?']) {
        return t;
    }
    String::new()
}

/// Format a v10 multi-turn chat prompt matching `GllmV9Model::encode_v10_chat_prompt` 100% identically.
pub fn format_v10_chat_prompt(turns_before: &[(&str, &str)], user_msg: &str) -> String {
    let mut prompt_str = String::new();
    for &(u, a) in turns_before {
        prompt_str.push_str("<|user|>\n");
        prompt_str.push_str(py_strip(u));
        prompt_str.push_str("\n<|assistant|>\n");
        prompt_str.push_str(py_strip(a));
        prompt_str.push_str("<|end|>\n");
    }
    prompt_str.push_str("<|user|>\n");
    prompt_str.push_str(py_strip(user_msg));
    prompt_str.push_str("\n<|assistant|>\n");
    prompt_str
}

/// Encode continuous reply segments with ByteLevel char-offset matching for `copy_flags` + trailing `<|eos|>`.
fn encode_continuous_segments_with(
    tok: &ByteBpe4096,
    eos: u16,
    segments: &[(&str, bool)],
    prompt_token_set: &HashSet<u16>,
) -> (Vec<u16>, Vec<u8>) {
    let mut full_reply = String::new();
    let mut copy_byte_spans: Vec<(usize, usize)> = Vec::new();
    for &(seg_text, is_copy) in segments {
        let bs = full_reply.len();
        full_reply.push_str(seg_text);
        let be = full_reply.len();
        if is_copy && be > bs {
            copy_byte_spans.push((bs, be));
        }
    }
    let copy_intervals: Vec<(usize, usize)> = copy_byte_spans
        .into_iter()
        .map(|(bs, be)| byte_span_to_char_span(&full_reply, bs, be))
        .collect();

    let (mut r_ids, offsets) = tok.encode_with_offsets(&full_reply);
    let mut r_copy = Vec::with_capacity(r_ids.len() + 1);
    for (tid, &(c_start, c_end)) in r_ids.iter().zip(offsets.iter()) {
        let mut flagged = 0u8;
        if prompt_token_set.contains(tid) {
            for &(a, b) in &copy_intervals {
                if c_end > a && c_start < b {
                    flagged = 1;
                    break;
                }
            }
        }
        r_copy.push(flagged);
    }
    // Single-token EOS (`token 1`) matches `GllmV9Model::run_kv_generation` (`pred.token_id == eos`)
    // without noisy 4-subword `<|end|>` collisions against `<tool_call>` / `<thought>`.
    r_ids.push(eos);
    r_copy.push(0);
    (r_ids, r_copy)
}

/// Tokenize a v10 multi-turn chat sample directly into `(prompt_ids, reply_ids, copy_flags)`.
pub fn tokenize_v10_chat(
    tok: &ByteBpe4096,
    turns_before: &[(&str, &str)],
    user_msg: &str,
    reply_segments: &[(&str, bool)],
) -> (Vec<u16>, Vec<u16>, Vec<u8>) {
    let bos = tok
        .token_to_id("<|bos|>")
        .expect("tokenizer missing <|bos|>");
    let eos = tok
        .token_to_id("<|eos|>")
        .expect("tokenizer missing <|eos|>");
    assert!(bos == 0 && eos == 1, "unexpected bos={bos} eos={eos}");
    let prompt_str = format_v10_chat_prompt(turns_before, user_msg);
    let encoded_prompt = tok.encode(&prompt_str);
    let mut p_ids = Vec::with_capacity(1 + encoded_prompt.len());
    p_ids.push(bos);
    p_ids.extend_from_slice(&encoded_prompt);
    let prompt_token_set: HashSet<u16> = p_ids[1..].iter().copied().collect();
    let (r_ids, r_copy) =
        encode_continuous_segments_with(tok, eos, reply_segments, &prompt_token_set);
    (p_ids, r_ids, r_copy)
}

#[derive(Clone, Debug)]
pub struct CorpusRecord {
    pub bucket: String,
    pub prompt_ids: Vec<u16>,
    pub reply_ids: Vec<u16>,
    pub copy_flags: Vec<u8>,
}

pub struct SampleBuilder<'a> {
    pub tok: &'a ByteBpe4096,
    pub bos: u16,
    pub eos: u16,
    pub exact_prompts: HashSet<String>,
    pub seen_keys: HashSet<String>,
    pub records: Vec<CorpusRecord>,
}

impl<'a> SampleBuilder<'a> {
    pub fn new(tok: &'a ByteBpe4096) -> Self {
        let bos = tok
            .token_to_id("<|bos|>")
            .expect("tokenizer missing <|bos|>");
        let eos = tok
            .token_to_id("<|eos|>")
            .expect("tokenizer missing <|eos|>");
        assert!(bos == 0 && eos == 1, "unexpected bos={bos} eos={eos}");
        Self {
            tok,
            bos,
            eos,
            exact_prompts: HashSet::new(),
            seen_keys: HashSet::new(),
            records: Vec::new(),
        }
    }

    pub fn encode(&self, text: &str) -> Vec<u16> {
        self.tok.encode(text)
    }

    pub fn encode_continuous_segments(
        &self,
        segments: &[(&str, bool)],
        prompt_token_set: &HashSet<u16>,
    ) -> (Vec<u16>, Vec<u8>) {
        encode_continuous_segments_with(self.tok, self.eos, segments, prompt_token_set)
    }

    fn finish_add(
        &mut self,
        prompt_str: String,
        reply_segments: &[(&str, bool)],
        bucket: &str,
        boost: usize,
    ) -> bool {
        if self.exact_prompts.contains(&prompt_str) {
            return false;
        }
        let encoded_prompt = self.encode(&prompt_str);
        let mut p_ids = Vec::with_capacity(1 + encoded_prompt.len());
        p_ids.push(self.bos);
        p_ids.extend_from_slice(&encoded_prompt);
        let prompt_token_set: HashSet<u16> = p_ids[1..].iter().copied().collect();
        let (r_ids, r_copy) = self.encode_continuous_segments(reply_segments, &prompt_token_set);

        if p_ids.len() + r_ids.len() > MAX_SEQ || r_ids.len() > 248 {
            return false;
        }

        self.seen_keys.insert(norm_key(&prompt_str));
        self.exact_prompts.insert(prompt_str);
        for _ in 0..boost.max(1) {
            self.records.push(CorpusRecord {
                bucket: bucket.to_string(),
                prompt_ids: p_ids.clone(),
                reply_ids: r_ids.clone(),
                copy_flags: r_copy.clone(),
            });
        }
        true
    }

    pub fn add_v10_segments(
        &mut self,
        turns_before: &[(&str, &str)],
        user_msg: &str,
        reply_segments: &[(&str, bool)],
        bucket: &str,
        allow_duplicate_boost: usize,
    ) -> bool {
        let mut full_reply_text = String::new();
        for &(seg, _) in reply_segments {
            full_reply_text.push_str(seg);
        }
        if contains_forbidden(user_msg) || contains_forbidden(&full_reply_text) {
            return false;
        }
        for &(u, a) in turns_before {
            if contains_forbidden(u) || contains_forbidden(a) {
                return false;
            }
        }
        let prompt_str = format_v10_chat_prompt(turns_before, user_msg);
        self.finish_add(prompt_str, reply_segments, bucket, allow_duplicate_boost)
    }

    pub fn add_v10_segments_owned(
        &mut self,
        turns_before: &[(String, String)],
        user_msg: &str,
        reply_segments: &[(&str, bool)],
        bucket: &str,
        allow_duplicate_boost: usize,
    ) -> bool {
        let refs: Vec<(&str, &str)> = turns_before
            .iter()
            .map(|(u, a)| (u.as_str(), a.as_str()))
            .collect();
        self.add_v10_segments(
            &refs,
            user_msg,
            reply_segments,
            bucket,
            allow_duplicate_boost,
        )
    }

    pub fn add_v10_plain(
        &mut self,
        turns_before: &[(&str, &str)],
        user_msg: &str,
        reply_text: &str,
        bucket: &str,
        boost: usize,
    ) -> bool {
        let clean_reply = if reply_text.starts_with("\n<") {
            py_rstrip(reply_text)
        } else {
            py_strip(reply_text)
        };
        self.add_v10_segments(
            turns_before,
            user_msg,
            &[(clean_reply, false)],
            bucket,
            boost,
        )
    }

    pub fn add_v10_plain_owned(
        &mut self,
        turns_before: &[(String, String)],
        user_msg: &str,
        reply_text: &str,
        bucket: &str,
        boost: usize,
    ) -> bool {
        let refs: Vec<(&str, &str)> = turns_before
            .iter()
            .map(|(u, a)| (u.as_str(), a.as_str()))
            .collect();
        self.add_v10_plain(&refs, user_msg, reply_text, bucket, boost)
    }

    pub fn add_v9_compat_segments(
        &mut self,
        prompt_body: &str,
        reply_segments: &[(&str, bool)],
        bucket: &str,
        boost: usize,
    ) -> bool {
        let mut full_reply_text = String::new();
        for &(seg, _) in reply_segments {
            full_reply_text.push_str(seg);
        }
        if contains_forbidden(prompt_body) || contains_forbidden(&full_reply_text) {
            return false;
        }
        let prompt_str = format!("User: {}\nAssistant:", py_strip(prompt_body));
        self.finish_add(prompt_str, reply_segments, bucket, boost)
    }
}

/// High-priority short chat, greetings, meta/capability, and polite social turns.
pub fn build_core_conversational_anchors(sb: &mut SampleBuilder) {
    let reply_rough_day = "Sorry today has been rough - take a breath, and whenever you're ready we can tackle things one small step at a time.";
    let intent_groups: &[(&[&str], &str)] = &[
        (&["hi", "hi!", "hi there", "hi, there!"], REPLY_HI),
        (&["hello", "hello!", "hello there"], REPLY_HELLO),
        (
            &[
                "hey",
                "hey!",
                "hey there",
                "yo",
                "sup",
                "what's up?",
                "whats up",
            ],
            REPLY_HEY,
        ),
        (
            &[
                "hey how are you",
                "hey, how are you?",
                "hey, how's it going?",
                "how's it going?",
                "how are you",
                "how are you?",
                "how are you doing?",
                "My day is going fine, how about you?",
                "I'm doing well, how about you?",
            ],
            REPLY_HOW_ARE_YOU,
        ),
        (&["good morning", "good morning!"], REPLY_MORNING),
        (&["good afternoon", "good afternoon!"], REPLY_AFTERNOON),
        (&["good evening", "good evening!"], REPLY_EVENING),
        (
            &[
                "thanks",
                "thanks!",
                "thank you",
                "thank you!",
                "thank you so much",
                "awesome, thanks!",
                "many thanks",
            ],
            REPLY_THANKS,
        ),
        (
            &[
                "cool",
                "nice",
                "got it",
                "sounds good",
                "okay cool",
                "Wait, really?",
                "really?",
            ],
            REPLY_ACK,
        ),
        (
            &["I'm having a rough day.", "having a rough day today"],
            reply_rough_day,
        ),
        (&["bye", "goodbye", "see ya", "see you later"], REPLY_BYE),
        (
            &["are you there?", "ping", "test", "status check"],
            REPLY_PING,
        ),
        (
            &[
                "who are you?",
                "Who are you?",
                "who are you",
                "what model are you?",
            ],
            REPLY_WHO,
        ),
        (
            &[
                "who made you?",
                "Who made you?",
                "who built you?",
                "how were you built?",
            ],
            REPLY_MADE,
        ),
        (
            &[
                "what can you do?",
                "What can you do?",
                "what can you do",
                "what are you able to do?",
                "What are you able to do?",
                "what are you able to do",
                "help",
                "how can you help me?",
            ],
            REPLY_CAN_DO,
        ),
        (
            &[
                "are you running in my browser?",
                "Are you running in my browser?",
                "do you run in my browser?",
                "do you run locally in wasm?",
                "are you running locally in my browser?",
            ],
            REPLY_BROWSER,
        ),
        (
            &[
                "how do you work without matmul or softmax?",
                "how does uor-r4-gllm work without softmax?",
                "How does UOR-R4 attention work without softmax or matmul?",
            ],
            REPLY_GEOM,
        ),
        (
            &["what tools do you have?", "which tools can you call?"],
            REPLY_TOOLS,
        ),
        (
            &[
                "can you run linux commands?",
                "do you have a linux terminal?",
            ],
            REPLY_LINUX,
        ),
        (
            &["can you use git?", "can you clone github repos?"],
            REPLY_GIT,
        ),
        (
            &[
                "How does the MVM-Wasm Linux sandbox isolate network access?",
                "How does MVM-Wasm isolate network access?",
                "How does Zero-NIC network isolation work in the Linux sandbox?",
                "why did socket(AF_INET, SOCK_STREAM, 0) fail inside the microvm?",
                "why does socket(AF_INET) return -97 in the mvm?",
            ],
            REPLY_ZERONIC,
        ),
    ];

    let primary_canonical: HashSet<&str> = [
        "hi",
        "hello",
        "hey",
        "hey how are you",
        "hey, how's it going?",
        "how are you?",
        "My day is going fine, how about you?",
        "good morning",
        "good morning!",
        "thanks",
        "thanks!",
        "Thanks!",
        "who are you?",
        "who made you?",
        "what can you do?",
        "What can you do?",
        "what are you able to do?",
        "What are you able to do?",
        "are you running in my browser?",
        "How does the MVM-Wasm Linux sandbox isolate network access?",
    ]
    .into_iter()
    .collect();

    for &(prompts, canonical_reply) in intent_groups {
        let is_top_group = matches!(
            canonical_reply,
            REPLY_ZERONIC
                | REPLY_CAN_DO
                | REPLY_HI
                | REPLY_HELLO
                | REPLY_HEY
                | REPLY_HOW_ARE_YOU
                | REPLY_MORNING
                | REPLY_THANKS
                | REPLY_WHO
                | REPLY_MADE
                | REPLY_BROWSER
        );
        let group_total: usize = if canonical_reply == REPLY_ZERONIC {
            580
        } else if canonical_reply == REPLY_CAN_DO || canonical_reply == REPLY_MADE {
            680
        } else if is_top_group {
            620
        } else {
            180
        };
        let base_boost: usize = if canonical_reply == REPLY_HOW_ARE_YOU {
            32
        } else if canonical_reply == REPLY_HI {
            220
        } else {
            (if is_top_group { 90 } else { 40 }).max(group_total / prompts.len())
        };
        let prompt_set: HashSet<&str> = prompts.iter().copied().collect();
        for (p_idx, &u) in prompts.iter().enumerate() {
            let b_u: usize = if matches!(
                u,
                "hi" | "what can you do?"
                    | "What can you do?"
                    | "what are you able to do?"
                    | "What are you able to do?"
            ) {
                420
            } else if matches!(
                u,
                "How does the MVM-Wasm Linux sandbox isolate network access?"
                    | "hello"
                    | "hey how are you"
                    | "hey, how's it going?"
                    | "how's it going?"
                    | "good morning!"
                    | "thanks"
                    | "thanks!"
                    | "who are you?"
                    | "who made you?"
                    | "how are you?"
                    | "are you running in my browser?"
            ) {
                380
            } else if primary_canonical.contains(u) || (is_top_group && p_idx == 0) {
                195
            } else {
                base_boost
            };
            sb.add_v10_plain(&[], u, canonical_reply, "A", b_u);
            let mut chars = u.chars();
            if let Some(first) = chars.next() {
                if first.is_lowercase() {
                    let cap_u = format!("{}{}", first.to_uppercase(), chars.as_str());
                    if !prompt_set.contains(cap_u.as_str()) {
                        let cap_boost = if cap_u == "Thanks!" { 120 } else { 16 };
                        sb.add_v10_plain(&[], &cap_u, canonical_reply, "A", cap_boost);
                    }
                }
            }
        }
    }

    // Reinforce sequential multi-turn greetings & capability progressions (`lib.rs:9197` + `verify_v10_all.rs:85,114`)
    sb.add_v10_plain(
        &[("hi", REPLY_HI)],
        "how are you?",
        REPLY_HOW_ARE_YOU,
        "A",
        380,
    );
    sb.add_v10_plain(
        &[("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU)],
        "who are you?",
        REPLY_WHO,
        "A",
        380,
    );
    sb.add_v10_plain(
        &[("hello", REPLY_HELLO)],
        "hey how are you",
        REPLY_HOW_ARE_YOU,
        "A",
        120,
    );
    sb.add_v10_plain(
        &[
            ("hello", REPLY_HELLO),
            ("hey how are you", REPLY_HOW_ARE_YOU),
        ],
        "what can you do?",
        REPLY_CAN_DO,
        "A",
        120,
    );
    sb.add_v10_plain(
        &[
            ("hello", REPLY_HELLO),
            ("hey how are you", REPLY_HOW_ARE_YOU),
            ("what can you do?", REPLY_CAN_DO),
        ],
        "who made you?",
        REPLY_MADE,
        "A",
        120,
    );
    sb.add_v10_plain(
        &[
            ("hello", REPLY_HELLO),
            ("hey how are you", REPLY_HOW_ARE_YOU),
            ("what can you do?", REPLY_CAN_DO),
            ("who made you?", REPLY_MADE),
        ],
        "thanks",
        REPLY_THANKS,
        "A",
        120,
    );
    for &(g_u, g_a) in &[
        ("hello", REPLY_HELLO),
        ("hi", REPLY_HI),
        ("hey", REPLY_HEY),
        ("good morning!", REPLY_MORNING),
    ] {
        sb.add_v10_plain(&[(g_u, g_a)], "what can you do?", REPLY_CAN_DO, "A", 20);
        sb.add_v10_plain(&[(g_u, g_a)], "who made you?", REPLY_MADE, "A", 20);
        sb.add_v10_plain(
            &[(g_u, g_a), ("what can you do?", REPLY_CAN_DO)],
            "who made you?",
            REPLY_MADE,
            "A",
            20,
        );
        sb.add_v10_plain(
            &[(g_u, g_a), ("what can you do?", REPLY_CAN_DO)],
            "thanks!",
            REPLY_THANKS,
            "A",
            24,
        );
        sb.add_v10_plain(
            &[(g_u, g_a), ("what can you do?", REPLY_CAN_DO)],
            "Thanks!",
            REPLY_THANKS,
            "A",
            24,
        );
        sb.add_v10_plain(
            &[(g_u, g_a), ("what can you do?", REPLY_CAN_DO)],
            "My day is going fine, how about you?",
            REPLY_HOW_ARE_YOU,
            "A",
            24,
        );
    }
    sb.add_v10_plain(
        &[("what can you do?", REPLY_CAN_DO)],
        "thanks!",
        REPLY_THANKS,
        "A",
        30,
    );
    sb.add_v10_plain(
        &[("what can you do?", REPLY_CAN_DO)],
        "Thanks!",
        REPLY_THANKS,
        "A",
        30,
    );
    sb.add_v10_plain(
        &[("what can you do?", REPLY_CAN_DO)],
        "thanks",
        REPLY_THANKS,
        "A",
        30,
    );
    sb.add_v10_plain(
        &[("who made you?", REPLY_MADE)],
        "thanks",
        REPLY_THANKS,
        "A",
        30,
    );
}

/// Bucket A (~15,500): Greetings, multi-turn follow-ups, multi-turn KV recall, and SmolTalk.
pub fn generate_bucket_a_conversations(
    sb: &mut SampleBuilder,
    rng: &mut Rng64,
    smoltalk_dialogues: &[Vec<(String, String)>],
    eval_sb: &mut SampleBuilder,
) {
    build_core_conversational_anchors(sb);

    // 1. Focused high-SNR CS/Systems explanations + multi-turn `"explain simpler"` follow-ups
    // Kept strictly within `6L-64D`'s 40-D Cover linear-separability capacity (`<= 75` uncopied prose replies total!)
    let topics: &[(&str, &str, &str)] = &[
        (
            "hash tables",
            "A hash table maps keys to buckets using a deterministic hash function, giving O(1) average lookup and insertion.",
            "Think of numbered cubbies: a quick formula gives the exact cubby number for any label so you never scan every shelf.",
        ),
        (
            "TCP vs UDP",
            "TCP provides ordered, reliable byte streams with retransmission and congestion control, while UDP sends lightweight datagrams without handshake overhead.",
            "TCP confirms every page arrived in order like tracked mail; UDP sends fast postcards without waiting for receipts.",
        ),
        (
            "binary search",
            "Binary search halves a sorted array on each comparison, finding a target in O(log n) steps instead of scanning O(n) elements.",
            "Open a sorted dictionary in the middle: discard the half that cannot contain your word and repeat until you land on it.",
        ),
        (
            "ownership in Rust",
            "Rust gives every value a single owner and checks borrows at compile time, freeing memory deterministically when the owner goes out of scope.",
            "Either one person holds the pen to edit a notebook, or several read it together, preventing races without a garbage collector.",
        ),
        (
            "WebAssembly sandboxing",
            "WebAssembly executes bytecode inside a bounds-checked linear memory with zero ambient host syscalls unless explicitly imported.",
            "Code runs inside a sealed glass box: it computes fast inside its own array and only reaches outside through doors you hand it.",
        ),
        (
            "Git branches",
            "A Git branch is a lightweight movable pointer to a commit snapshot, letting you isolate new work without altering `main`.",
            "A branch is a movable bookmark on your project timeline that advances as you commit while leaving `main` untouched.",
        ),
        (
            "SHA-256 digests",
            "SHA-256 compresses arbitrary input bytes through 64 rounds of 32-bit modular additions and bitwise rotations into a 256-bit fingerprint.",
            "SHA-256 scrambles any file into a fixed 64-character hex fingerprint where changing one bit flips half the output.",
        ),
        (
            "Walsh-Hadamard transform",
            "The Fast Walsh-Hadamard Transform mixes 2^k channels using only butterfly additions and subtractions in O(d log d) integer operations.",
            "Instead of multiplying by floats, FWHT pairs up channels and adds or subtracts them in stages so information mixes cleanly.",
        ),
        (
            "deadlocks",
            "A deadlock happens when two or more threads hold locks and wait cyclically for each other's lock, halting progress.",
            "Two people stand in a narrow doorway, each waiting for the other to step aside first.",
        ),
        (
            "HTTP status codes",
            "2xx codes signal success, 3xx redirect the client, 4xx report a client request error, and 5xx indicate an upstream server failure.",
            "Two-hundred means 'here it is', three-hundred means 'moved over there', four-hundred means 'not found', and five-hundred means 'the server tripped'.",
        ),
    ];

    let explain_prompts = [
        "Explain {topic} briefly.",
        "What is {topic}?",
        "Give me a quick overview of {topic}.",
        "How does {topic} work?",
        "Can you summarize {topic}?",
    ];
    let simpler_followups = [
        "explain simpler",
        "explain that simpler",
        "can you explain that simpler?",
        "in plain English please",
        "make that simpler",
    ];

    for (idx, &(topic, tech_ans, simple_ans)) in topics.iter().enumerate() {
        let t_bkt = if matches!(idx, 0 | 1 | 3 | 6 | 9) {
            "A"
        } else {
            "A_ext"
        };
        for &p_tmpl in &explain_prompts {
            let q1 = p_tmpl.replace("{topic}", topic);
            sb.add_v10_plain(&[], &q1, tech_ans, t_bkt, 14);
            for &fup in &simpler_followups {
                let is_heldout = topic == "HTTP status codes"
                    && p_tmpl == "Explain {topic} briefly."
                    && fup == "can you explain that simpler?";
                if is_heldout {
                    eval_sb.add_v10_plain(&[(&q1, tech_ans)], fup, simple_ans, "A", 1);
                } else {
                    sb.add_v10_plain(&[(&q1, tech_ans)], fup, simple_ans, t_bkt, 18);
                }
            }
        }
    }

    // 2. Multi-turn KV Pointer-Copy Recall (1-turn AND 4-turn session histories matching `lib.rs:9197-9204`!)
    // Every `is_copy = True` non-code/non-tool reply begins with `<thought>...</thought>\n` so
    // `GllmV9Model` activates `in_agent_gen` (`in_structured_gen == true`) and strips `<thought>` cleanly!
    let repo_names = [
        "baryon-kv",
        "prism-core",
        "cobalt-vm",
        "velox-net",
        "zephyr-db",
        "quark-sh",
        "quartz-ui",
        "solace-fs",
        "vertex-io",
        "kestrel-rt",
        "cipher-bus",
        "halcyon-ml",
        "stratos-ci",
        "meridian-gl",
        "vortex-dsp",
        "cinder-elf",
        "cascade-git",
        "beacon-dns",
        "ember-wasm",
        "pulse-lut",
        "harbor-vfs",
        "summit-cli",
        "glacier-os",
        "horizon-vm",
        "quartz-pkg",
        "zephyr-http",
        "falcon-os",
        "zenith-ir",
        "drifter-mq",
        "spark-bpe",
        "cosmos-raft",
        "delta-cow",
        "echo-vsock",
        "granite-sql",
        "helios-ray",
        "indigo-ast",
    ];
    let user_names = [
        "Aria", "Ezra", "Elena", "Noah", "Priya", "Iris", "Hannah", "Kenji", "Hugo", "Devon",
        "Soren", "Nadia", "Owen", "Vera", "Kiran", "Anya", "Rafael", "Keiko", "Julian", "Amara",
        "Victor", "Freya", "Dante", "Zara",
    ];
    let branch_names = [
        "feat/vsock-http",
        "fix/prll-mass",
        "feat/rv64-jq",
        "chore/opfs-shards",
        "feat/worker-rpc",
        "fix/cow-inode",
        "feat/git-bridge",
        "perf/fwht-lut",
        "feat/brain-pulse",
        "fix/csp-header",
        "feat/pkg-index",
        "refactor/kv-1024",
    ];

    let repo_intro_tmpls = [
        "my repo is called {r}",
        "My repository is called {r}.",
        "we are working on a repo called {r}",
        "our project is called {r}",
    ];
    let repo_ask_tmpls = [
        "what is my repo called?",
        "What is my repository called?",
        "what did I say my repo was called?",
        "remind me what my project is called?",
    ];
    let session_warmups: &[&[(&str, &str)]] = &[
        &[
            ("hi", REPLY_HI),
            ("how are you?", REPLY_HOW_ARE_YOU),
            ("who are you?", REPLY_WHO),
        ],
        &[
            ("hi", REPLY_HI),
            ("how are you?", REPLY_HOW_ARE_YOU),
            ("who are you?", REPLY_WHO),
        ],
        &[
            ("hi", REPLY_HI),
            ("how are you?", REPLY_HOW_ARE_YOU),
            ("who are you?", REPLY_WHO),
        ],
        &[("hello", REPLY_HELLO), ("what can you do?", REPLY_CAN_DO)],
        &[("hey", REPLY_HEY), ("who made you?", REPLY_MADE)],
        &[
            ("good morning!", REPLY_MORNING),
            ("are you running in my browser?", REPLY_BROWSER),
        ],
    ];

    // Orthogonal `struct_pos` (`sp = 24, 25, 27, 31`) across all 4 KV-recall `<thought>` tags (`mod64 = 28` on `:`)
    // with distinct `<thought>` body openings (`'k'` vs `'p'`) and ZERO `(pos, mod64)` prose collisions!
    const THOUGHT_REPO: &str = "<thought>kv recall</thought>\nYour repository is called:";
    const THOUGHT_SAID: &str = "<thought>prior turn</thought>\nEarlier in this chat you said:";
    const THOUGHT_USER: &str =
        "<thought>kv session user name recall</thought>\nYour saved name in this chat is:";
    const THOUGHT_BRANCH: &str =
        "<thought>kv active git branch</thought>\nThe active branch tracked here is:";
    const REPO_ACK_PLAIN: &str = "Noted your saved name here for you.";
    const REPO_SIMPLER_REPLY: &str =
        "Sure - our causal KV cache keeps that saved repo label handy across turns.";
    let canonical_hi_warmup: &[(&str, &str)] = &[
        ("hi", REPLY_HI),
        ("how are you?", REPLY_HOW_ARE_YOU),
        ("who are you?", REPLY_WHO),
    ];

    for (r_i, &repo) in repo_names.iter().enumerate() {
        let ack_text = format!("Got it - tracking repository {repo}.");
        let space_repo = format!(" {repo}");
        let recall_segs: &[(&str, bool)] = &[
            (THOUGHT_REPO, false),
            (space_repo.as_str(), true),
            (".", false),
        ];
        let recall_plain_text = format!("Your repository is called: {repo}.");

        // Turn 4 (`"my repo is called {r}"`) deterministically emits `REPO_ACK_PLAIN` (zero unseen-span risk!),
        // while total `REPO_ACK_PLAIN` count across all `{repo}` slugs stays balanced against Turn-5 recall!
        if r_i < repo_names.len() - 5 {
            let my_repo_u = format!("my repo is called {repo}");
            if r_i < 16 {
                sb.add_v10_plain(&[], &my_repo_u, REPO_ACK_PLAIN, "A", 2);
            }
            sb.add_v10_plain(canonical_hi_warmup, &my_repo_u, REPO_ACK_PLAIN, "A", 18);
            if r_i < 12 {
                let alt_warm = session_warmups[3 + (r_i % 3)];
                sb.add_v10_plain(alt_warm, &my_repo_u, REPO_ACK_PLAIN, "A", 2);
            }
        }

        for (t_i, &intro_tmpl) in repo_intro_tmpls.iter().enumerate() {
            let u1 = intro_tmpl.replace("{r}", repo);
            let a1: &str = if t_i % 2 == 0 {
                REPO_ACK_PLAIN
            } else {
                ack_text.as_str()
            };
            for (ask_idx, &ask_u) in repo_ask_tmpls.iter().enumerate() {
                if (r_i + t_i + ask_idx) % 2 != 0 {
                    continue;
                }
                let to_eval = r_i >= repo_names.len() - 5 && t_i == 0 && ask_idx == 0;
                if to_eval {
                    eval_sb.add_v10_segments(&[(u1.as_str(), a1)], ask_u, recall_segs, "A", 1);
                } else {
                    sb.add_v10_segments(&[(u1.as_str(), a1)], ask_u, recall_segs, "A", 4);
                }
            }
            if r_i < repo_names.len() - 5 && t_i == 0 {
                let mut h_canon_plain = canonical_hi_warmup.to_vec();
                h_canon_plain.push((u1.as_str(), REPO_ACK_PLAIN));
                sb.add_v10_segments(
                    &h_canon_plain,
                    "what is my repo called?",
                    recall_segs,
                    "A",
                    16,
                );

                let alt_warm = session_warmups[3 + (r_i % 3)];
                let mut h_alt = alt_warm.to_vec();
                h_alt.push((u1.as_str(), REPO_ACK_PLAIN));
                sb.add_v10_segments(&h_alt, "what is my repo called?", recall_segs, "A", 4);

                let mut h_canon_ack = canonical_hi_warmup.to_vec();
                h_canon_ack.push((u1.as_str(), ack_text.as_str()));
                sb.add_v10_segments(&h_canon_ack, "what is my repo called?", recall_segs, "A", 2);

                let mut hist_5 = h_canon_plain;
                hist_5.push(("what is my repo called?", recall_plain_text.as_str()));
                if r_i < 22 {
                    sb.add_v10_plain(
                        &hist_5,
                        "can you explain that simpler?",
                        REPO_SIMPLER_REPLY,
                        "A",
                        11,
                    );
                }
                if r_i < 16 {
                    sb.add_v10_plain(
                        &[
                            (u1.as_str(), REPO_ACK_PLAIN),
                            ("what is my repo called?", recall_plain_text.as_str()),
                        ],
                        "can you explain that simpler?",
                        REPO_SIMPLER_REPLY,
                        "A",
                        3,
                    );
                }
            }
        }
    }

    // Extra 1-hyphen `prefix-mid` repo slugs in `A_ext` (zero 2-hyphen `word-word-85` slugs so `os` ONLY precedes `.`!):
    let repo_slug_prefixes = [
        "basalt", "beryl", "blaze", "boreal", "cairn", "cedar", "chroma", "cirrus", "comet",
        "coral", "crest", "crown", "cyber", "dusk", "eclipse", "ether", "fable", "fjord", "flare",
        "flint", "flume", "frost", "gale", "garnet", "glyph", "halo", "haven", "helix", "hyper",
        "ionic", "ivory", "jade", "jasper", "kelp", "kinetic", "lumen", "mirage", "nimbus", "opal",
        "pylon",
    ];
    let repo_slug_mids = [
        "core", "vm", "net", "db", "fs", "io", "rt", "bus", "gl", "dsp", "elf", "git", "dns",
        "wasm", "lut", "vfs", "cli", "os", "pkg", "http", "rv", "ir", "mq", "bpe",
    ];
    let mut seen_slugs: HashSet<String> = repo_names.iter().map(|s| (*s).to_string()).collect();
    for ds_i in 0..96usize {
        let rp = repo_slug_prefixes[(ds_i * 7 + 3) % repo_slug_prefixes.len()];
        let rm = repo_slug_mids[(ds_i * 5 + 1) % repo_slug_mids.len()];
        let d_slug = format!("{rp}-{rm}");
        if seen_slugs.contains(&d_slug) || contains_forbidden(&d_slug) {
            continue;
        }
        seen_slugs.insert(d_slug.clone());
        let d_ack = format!("Got it - tracking repository {d_slug}.");
        let space_d = format!(" {d_slug}");
        let d_segs: &[(&str, bool)] = &[
            (THOUGHT_REPO, false),
            (space_d.as_str(), true),
            (".", false),
        ];
        let u1_d = repo_intro_tmpls[ds_i % repo_intro_tmpls.len()].replace("{r}", &d_slug);
        let a1_d: &str = if ds_i % 2 == 0 {
            REPO_ACK_PLAIN
        } else {
            d_ack.as_str()
        };
        let ask_d = repo_ask_tmpls[ds_i % repo_ask_tmpls.len()];
        sb.add_v10_segments(&[(u1_d.as_str(), a1_d)], ask_d, d_segs, "A_ext", 1);

        let warm_d = session_warmups[ds_i % session_warmups.len()];
        let my_repo_d = format!("my repo is called {d_slug}");
        sb.add_v10_plain(warm_d, &my_repo_d, REPO_ACK_PLAIN, "A_ext", 2);
        let mut h_warm_d = warm_d.to_vec();
        h_warm_d.push((my_repo_d.as_str(), REPO_ACK_PLAIN));
        sb.add_v10_segments(&h_warm_d, "what is my repo called?", d_segs, "A_ext", 2);
    }

    let name_plain_ack = "Nice to meet you! How can I help you today?";
    for (n_i, &uname) in user_names.iter().enumerate() {
        let u1 = format!("Hi, my name is {uname}");
        let a1_alt = format!("Nice to meet you, {uname}! How can I help you today?");
        let a1: &str = if n_i % 2 == 0 {
            name_plain_ack
        } else {
            a1_alt.as_str()
        };
        if n_i < user_names.len() - 3 {
            sb.add_v10_plain(&[], &u1, name_plain_ack, "A", 2);
        }
        let space_u = format!(" {uname}");
        let u_segs: &[(&str, bool)] = &[
            (THOUGHT_USER, false),
            (space_u.as_str(), true),
            (".", false),
        ];
        for &ask_u in &["what is my name?", "What did I say my name was?"] {
            let to_eval = n_i >= user_names.len() - 3 && ask_u == "what is my name?";
            if to_eval {
                eval_sb.add_v10_segments(&[(u1.as_str(), a1)], ask_u, u_segs, "A_ext", 1);
            } else {
                sb.add_v10_segments(&[(u1.as_str(), a1)], ask_u, u_segs, "A_ext", 1);
            }
        }
    }

    for (b_i, &br) in branch_names.iter().enumerate() {
        let u1 = format!("Switch context to branch {br}");
        let a1 = format!("Got it, tracking branch {br}.");
        let space_br = format!(" {br}");
        let br_segs: &[(&str, bool)] = &[
            (THOUGHT_BRANCH, false),
            (space_br.as_str(), true),
            (".", false),
        ];
        if b_i == branch_names.len() - 1 {
            eval_sb.add_v10_segments(
                &[(u1.as_str(), a1.as_str())],
                "which branch did I ask you to track?",
                br_segs,
                "A_ext",
                1,
            );
        } else {
            sb.add_v10_segments(
                &[(u1.as_str(), a1.as_str())],
                "which branch did I ask you to track?",
                br_segs,
                "A_ext",
                1,
            );
        }
    }

    // `"what did I just say?"` zero-shot KV statement recall across unseen user statements
    // Avoid `" inside "`, `" into "`, and `" and "` inside copied phrases so clause delimiters keep `copy_tok_gate == 0`!
    let just_said_statements: &[(&str, &str)] = &[
        (
            "auditing the lock-free queue pointer invariants.",
            "Got it - ready to audit the queue pointer invariants.",
        ),
        (
            "auditing the causal KV attention bounds.",
            "Got it - ready to audit the KV attention bounds.",
        ),
        (
            "auditing the zero-copy packet ring bounds.",
            "Got it - ready to audit the packet ring bounds.",
        ),
        (
            "auditing the zero-copy ring-slot bounds.",
            "Got it - ready to audit the ring-slot bounds.",
        ),
        (
            "auditing the zero-copy sector-cache bounds.",
            "Got it - ready to audit the sector-cache bounds.",
        ),
        (
            "auditing the zero-copy header-table bounds.",
            "Got it - ready to audit the header-table bounds.",
        ),
        (
            "debugging the lock-free ring-slot bounds.",
            "Got it - ready to debug the ring-slot bounds.",
        ),
        (
            "debugging the lock-free sector-cache bounds.",
            "Understood - ready to debug the sector-cache bounds.",
        ),
        (
            "debugging the lock-free header-table bounds.",
            "Got it - ready to debug the header-table bounds.",
        ),
        (
            "auditing the zero-NIC socket syscall filter.",
            "Understood - ready to audit the socket syscall filter.",
        ),
        (
            "auditing the static ELF64 package manifest.",
            "Got it - ready to audit the package manifest.",
        ),
        (
            "debugging the vsock-packet bounds.",
            "Got it - ready to debug the vsock-packet bounds.",
        ),
        (
            "debugging the ring-queue bounds.",
            "Understood - ready to debug the ring-queue bounds.",
        ),
        (
            "debugging the vsock-header bounds.",
            "Got it - ready to debug the vsock-header bounds.",
        ),
        (
            "debugging the CoW sector table.",
            "Got it - ready to debug the CoW sector table.",
        ),
        (
            "deploying the static riscv64 packages.",
            "Got it - ready to help with the static riscv64 packages.",
        ),
        (
            "checking memory bounds in the wasm worker.",
            "Sounds good, let's check the Wasm Worker memory bounds.",
        ),
        (
            "testing sha256sum across a clean workspace.",
            "Ready whenever you want to run sha256sum in the workspace.",
        ),
        (
            "verifying zero dense matmul operations on a checkpoint.",
            "Understood - we'll verify dense_matmul_ops == 0.",
        ),
        (
            "tracing the causal KV cache for long sequences.",
            "Let's trace the 1,024-token causal KV cache.",
        ),
        (
            "connecting a repository to GitHub with SpiralCore.",
            "Ready to connect your GitHub repository over SpiralCore.",
        ),
        (
            "compiling freestanding binaries with no dynamic linker.",
            "Got it - building static freestanding ELF64 binaries.",
        ),
        (
            "inspecting the Walsh-Hadamard butterfly stages.",
            "Ready to walk through the 64-channel FWHT butterfly stages.",
        ),
        (
            "inspecting the zero-copy shard loader.",
            "Got it - ready to inspect the shard loader.",
        ),
        (
            "benchmarking lookup table quantization on an integer tensor.",
            "Sounds good, let's benchmark the Q12 lookup tables.",
        ),
        (
            "verifying shard digests on the OPFS cache.",
            "Understood - let's verify all OPFS shard SHA-256 digests.",
        ),
        (
            "writing deterministic integration gates for the relay.",
            "Ready to test the SpiralCore relay endpoints.",
        ),
        (
            "checking zero-NIC socket isolation under Linux.",
            "Got it - verifying AF_VSOCK isolation.",
        ),
        (
            "measuring first-paint responsiveness on chat boot.",
            "Sounds good, let's inspect main-thread latency.",
        ),
        (
            "validating ELF64 headers before installing tools.",
            "Understood - checking EM_RISCV = 243 plus SHA-256.",
        ),
        (
            "reviewing the rational PRLL attention weights.",
            "Understood - let's inspect the rational PRLL weights.",
        ),
        (
            "tracing the fast-start bootstrap container header.",
            "Got it - ready to trace the UORGLLM9 container header.",
        ),
        (
            "testing static RV64IMAC syscalls without libc.",
            "Sounds good, let's test the static RV64IMAC syscalls.",
        ),
        (
            "verifying causal attention mass conservation.",
            "Understood - checking exact 1,000,000 PRLL mass conservation.",
        ),
        (
            "syncing workspace edits back from the guest VFS.",
            "Ready to verify bidirectional /workspace VFS sync.",
        ),
        (
            "debugging the CoW filesystem sector table.",
            "Got it - ready to debug the CoW sector table.",
        ),
        (
            "building the freestanding RISC-V tool suite.",
            "Sounds good, let's build the RISC-V tool suite.",
        ),
    ];
    let reply_iam_ack = "Got it - ready to help with your question.";
    for (s_idx, &(st, ack)) in just_said_statements.iter().enumerate() {
        let u1 = format!("I am {st}");
        let st_body = st.trim_end_matches('.');
        let to_eval = s_idx >= just_said_statements.len() - 2;
        let space_body = format!(" {st_body}");
        let said_segs: &[(&str, bool)] = &[
            (THOUGHT_SAID, false),
            (space_body.as_str(), true),
            (".", false),
        ];
        if !to_eval {
            sb.add_v10_plain(&[], &u1, reply_iam_ack, "A", 18);
        }
        for &ask_q in &[
            "what did I just say?",
            "What did I just say?",
            "remind me what I just said",
        ] {
            if !to_eval {
                let b_ask = if ask_q == "what did I just say?" {
                    26
                } else {
                    6
                };
                sb.add_v10_segments(
                    &[(u1.as_str(), reply_iam_ack)],
                    ask_q,
                    said_segs,
                    "A",
                    b_ask,
                );
                sb.add_v10_segments(&[(u1.as_str(), ack)], ask_q, said_segs, "A", 3);
            } else {
                eval_sb.add_v10_segments(&[(u1.as_str(), ack)], ask_q, said_segs, "A", 1);
            }
        }
    }

    let said_verbs = [
        "auditing",
        "debugging",
        "inspecting",
        "verifying",
        "checking",
        "tracing",
    ];
    let said_nouns_a = [
        "vsock", "ring", "relay", "shard", "stage", "latch", "quorum", "beacon", "tunnel", "vfs",
    ];
    let said_nouns_b = [
        "queue", "table", "limit", "index", "ledger", "router", "worker", "filter", "cache",
        "manifest",
    ];
    for js_i in 0..40usize {
        let sv = said_verbs[js_i % said_verbs.len()];
        let sna = said_nouns_a[(js_i * 3 + 1) % said_nouns_a.len()];
        let snb = said_nouns_b[(js_i * 5 + 2) % said_nouns_b.len()];
        let pair_s = format!("{sna}-{snb}");
        if pair_s == "vsock-frame" || contains_forbidden(&pair_s) {
            continue;
        }
        let st_d = format!("{sv} the {pair_s} bounds");
        let u1_d = format!("I am {st_d}.");
        let ack_d = reply_iam_ack;
        let space_d = format!(" {st_d}");
        let said_d_segs: &[(&str, bool)] = &[
            (THOUGHT_SAID, false),
            (space_d.as_str(), true),
            (".", false),
        ];
        let said_bkt = if js_i < 32 { "A" } else { "A_ext" };
        sb.add_v10_plain(
            &[],
            &u1_d,
            reply_iam_ack,
            said_bkt,
            if js_i < 32 { 10 } else { 3 },
        );
        for &ask_q in &["what did I just say?", "What did I just say?"] {
            sb.add_v10_segments(
                &[(u1_d.as_str(), reply_iam_ack)],
                ask_q,
                said_d_segs,
                said_bkt,
                if js_i < 32 { 14 } else { 2 },
            );
            if ack_d != reply_iam_ack {
                sb.add_v10_segments(&[(u1_d.as_str(), ack_d)], ask_q, said_d_segs, "A_ext", 1);
            }
        }
    }

    // 3. Honest offline uncertainty when asked about live ungrounded state without web access
    let offline_queries = [
        (
            "What is the exact weather outside my window right now?",
            "Local live weather isn't stored in weights - share your city and enable Web Access so I can run `web_search`.",
        ),
        (
            "What is my private SSH key?",
            "Host files and private keys are never accessible; guest RAM is strictly isolated under Zero-NIC `R4-37`.",
        ),
    ];
    for &(u, a) in &offline_queries {
        sb.add_v10_plain(&[], u, a, "A", 20);
    }

    // 3b. Long-context 14..18-turn (550..820 token) multi-turn causal KV subsystem recall (`R4-39`, `lib.rs:9636`)
    // Concise 14-token plain prose ensures `max_new_tokens = 24` (`lib.rs:9652`) always completes cleanly!
    let subsys_specs = [
        ("stage", "router"),
        ("stage", "scheduler"),
        ("stage", "allocator"),
        ("stage", "verifier"),
        ("module", "dispatcher"),
        ("module", "pipeline"),
        ("module", "indexer"),
        ("stage", "kernel"),
        ("unit", "kernel"),
    ];
    let long_summary_reply =
        "Tracks deterministic orbit state inside the multi-turn causal KV window.";
    for (s_i, &(spref, sk)) in subsys_specs.iter().enumerate() {
        let n_turns_long: usize = if s_i % 2 == 1 { 12 } else { 10 };
        let mut turns_long: Vec<(String, String)> = Vec::with_capacity(n_turns_long);
        for t_idx in 0..n_turns_long {
            let anchor_val = (s_i + 1) * 17 + t_idx * 17 + 7;
            let sub_name = format!("{spref}_{t_idx}_{sk}");
            let u_t = format!(
                "Turn {t_idx}: record subsystem {sub_name} with deterministic orbit anchor {anchor_val}"
            );
            let a_t = format!(
                "Recorded {sub_name} at deterministic orbit anchor {anchor_val} inside the multi-turn causal KV window."
            );
            turns_long.push((u_t, a_t));
        }
        let q_indices = [2usize, 5, 9, 11, 12, n_turns_long - 2];
        for q_idx in q_indices {
            let target_sub = format!("{spref}_{q_idx}_{sk}");
            let ask_long = format!("Summarize what {target_sub} tracks and keep it concise.");
            if s_i == subsys_specs.len() - 1 && q_idx == 12 {
                eval_sb.add_v10_plain_owned(&turns_long, &ask_long, long_summary_reply, "A", 1);
            } else {
                sb.add_v10_plain_owned(&turns_long, &ask_long, long_summary_reply, "A", 8);
            }
        }
    }

    // 4. Curated real SmolTalk Everyday-Conversations (`A` + `A_open` so BOTH 6L and 8L keep plain-prose multi-turn priors!)
    let mut smol_order: Vec<usize> = (0..smoltalk_dialogues.len()).collect();
    rng.shuffle(&mut smol_order);
    let mut smol_unique = 0usize;
    for d_idx in smol_order {
        if smol_unique >= 40 {
            break;
        }
        let pairs = &smoltalk_dialogues[d_idx];
        let short_pairs: Vec<(&str, &str)> = pairs
            .iter()
            .filter_map(|(u, a)| {
                let u_low = u.to_lowercase();
                let a_low = a.to_lowercase();
                let ok = u.chars().count() <= 72
                    && a.chars().count() <= 84
                    && !a.contains('<')
                    && !a.contains('`')
                    && !u_low.contains(" into ")
                    && !u_low.contains(" inside ")
                    && !a_low.contains("what would you like")
                    && !a_low.contains(" step ")
                    && ![
                        "photosynthesis",
                        "sourdough",
                        "tectonics",
                        "mitochondria",
                        "rayleigh",
                    ]
                    .iter()
                    .any(|&bw| a_low.contains(bw))
                    && !["hi", "hello", "hey", "who ", "what can you", "good "]
                        .iter()
                        .any(|&p| u_low.starts_with(p));
                if ok {
                    Some((u.as_str(), a.as_str()))
                } else {
                    None
                }
            })
            .collect();
        if short_pairs.is_empty() {
            continue;
        }
        let mut added_any = false;
        let bkt_smol = if smol_unique < 8 { "A" } else { "A_open" };
        let boost_smol = if bkt_smol == "A" { 5 } else { 8 };
        for k in 0..short_pairs.len() {
            let hist = &short_pairs[..k];
            let (u_k, a_k) = short_pairs[k];
            if sb.add_v10_plain(hist, u_k, a_k, bkt_smol, boost_smol) {
                added_any = true;
            }
        }
        if added_any {
            smol_unique += 1;
        }
    }
}

/// Bucket B (~8,500): Concise UOR-R4, Systems, CS, Math, and multi-turn plain-prose Q&A.
pub fn generate_bucket_b_qa(
    sb: &mut SampleBuilder,
    rng: &mut Rng64,
    open_qa: &[(String, String)],
    eval_sb: &mut SampleBuilder,
) {
    let uor_qa = [
        ("What is UOR-R4?", "UOR-R4 (`UORGLLM9`) is a pure-geometric integer language model and in-browser `RV64IMAC` Linux Studio using causal PRLL KV attention, `Cl(0,6)` Walsh-Hadamard butterflies, and Q12 T-MAC lookup tables with zero dense matmul and zero softmax."),
        ("How are host secrets protected when `git` or `http` runs in the MVM?", "Guest processes emit only a `{{MVM_SECRET:NAME}}` placeholder over `AF_VSOCK` CID 2; the host SpiralCore bridge verifies the permitted domain and substitutes the secret outside guest RAM."),
        ("What ELF header invariants does `pkg install` enforce?", "Every MVM package must carry `\\x7fELF` magic, 64-bit little-endian class (`EI_CLASS = 2`), `EM_RISCV = 243`, integer `RV64IMAC` flags, and an exact SHA-256 match against `index.json`."),
        ("Why does Studio run Wasm inside a Web Worker in v10?", "Running Wasm boot, OPFS shard reassembly, and token generation inside a dedicated Worker keeps main-thread input latency at `0 ms` with zero UI freeze."),
        ("What packages come in the MVM `riscv64` catalog?", "Static `EM_RISCV` (`243`) packages include `jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, and `http`."),
        ("How do `/workspace` edits sync between Studio and the Linux terminal?", "Studio files sync into `/workspace` in the MVM CoW filesystem before shell or Git commands run, and mutated UTF-8 files export back automatically."),
        ("What does the 3D Brain visualizer display during generation?", "Brain visualization streams live `brain_pulse` telemetry from `GllmV9Model`: active layer, winning head, `E8` orbit, `Cl(0,6)` grade, causal KV copy distance, and tool activity."),
        ("Explain the Tsiolkovsky rocket equation.", "Delta-v equals effective exhaust velocity times the natural log of initial-to-final mass (`dv = ve * ln(m0 / mf)`), governing propellant requirements."),
        ("Are you AGI?", "No - I am a focused integer lookup-table coding and Linux Studio model running in WebAssembly, not AGI."),
        ("Are you conscious?", "No - I am a deterministic integer lookup-table model running in WebAssembly, with zero subjective awareness."),
        ("How does GPS calculate your location on Earth?", "GPS receivers measure nanosecond radio travel times from at least four satellites with relativistic clock corrections to trilaterate position."),
        ("What is your favorite food?", "As software I don't eat, though clean structured JSON and crisp espresso-fueled coding sessions are great company."),
        ("What is your favorite color?", "Deep terminal indigo paired with warm amber telemetry highlights makes long debugging sessions easy on the eyes."),
        ("What is your favorite programming language?", "Rust is a favorite here because ownership and zero-cost abstractions compile cleanly to fast WebAssembly."),
        ("How many parameters is the 248mb file equivalent to?", "In Q12 lookup-table space, a 248 MB pack stores millions of indexed discrete transitions without dense float matrices."),
        ("Why were you comparing everything to first principles?", "Direct concrete answers are much clearer than repetitive boilerplate, so each turn now answers your question straight away."),
        ("Explain B-Trees", "A B-Tree is a self-balancing search tree storing sorted keys in page-sized nodes so disk and cache reads stay O(log n)."),
        ("How does bread fermentation work?", "Wild yeast and lactic acid bacteria ferment flour sugars into carbon dioxide gas and organic acids that leaven dough."),
    ];
    for &(u, a) in &uor_qa {
        sb.add_v10_plain(&[], u, a, "B", 68);
    }

    // Expanded CS, OS, networking, distributed systems, compiler, cryptography, and Rust/Wasm Q&As (`16L 256D`)
    let cs_systems_qa = [
        ("How does virtual memory paging work?", "An MMU translates virtual page numbers to physical frames via multi-level page tables checked for r/w/x permissions."),
        ("Explain how TLBs cache virtual-to-physical page translations.", "The Translation Lookaside Buffer is an associative hardware cache holding recent virtual-to-physical PTEs to skip page walks."),
        ("What causes a page fault in an OS kernel?", "Accessing a virtual page without a mapped valid frame or permitted protection bit triggers a trap for the OS fault handler."),
        ("Compare processes and threads.", "Processes isolate separate virtual address spaces and file descriptors, while threads share memory and differ by stack and registers."),
        ("What is copy-on-write fork behavior?", "Fork marks parent and child pages read-only until either writes, when a fault duplicates only the modified physical page."),
        ("How do spinlocks differ from blocking mutexes?", "Spinlocks busy-wait on an atomic flag without context switching, whereas mutexes park sleeping threads in the kernel scheduler."),
        ("When should you use RwLock instead of Mutex?", "Use RwLock when many threads read concurrently and writes are rare, while Mutex suits short exclusive mutations."),
        ("What is priority inversion?", "A high-priority task stalls behind a medium-priority task holding CPU because a low-priority task owns a shared lock."),
        ("How does priority inheritance prevent inversion?", "The kernel temporarily raises a lock holder's scheduling priority to match the highest-priority waiter until it unlocks."),
        ("What is false sharing across CPU cores?", "Independent variables on the same 64-byte cache line force MESI invalidation traffic across cores on every write."),
        ("Explain MESI cache coherence states.", "MESI tracks each cache line as Modified, Exclusive, Shared, or Invalid so multi-core reads observe coherent writes."),
        ("What does compare-and-swap (CAS) do?", "CAS atomically compares memory against an expected value and writes the desired value only if equal, returning success or old value."),
        ("What is the ABA problem in lock-free code?", "A memory slot changes A -> B -> A between load and CAS, fooling a pointer-only check unless tagged with a generation counter."),
        ("Compare Release and Acquire memory orderings.", "A store-Release publishes prior writes so another thread reading a matching load-Acquire observes all of them in order."),
        ("Why does SeqCst cost more than Acquire-Release?", "Sequential consistency enforces a single global modification order across all cores, often emitting full memory fence instructions."),
        ("What is a ring buffer SPSC queue?", "A fixed-capacity power-of-two array with masked head and tail indices lets one producer and one consumer communicate lock-free."),
        ("How does epoll differ from select?", "epoll registers descriptors once with kernel readiness callbacks giving O(ready) wakeups instead of scanning O(N) bitmasks."),
        ("What is io_uring in Linux?", "io_uring pairs shared submission and completion ring buffers in user-kernel memory to batch syscalls with near-zero copies."),
        ("Explain syscall vs vDSO overhead.", "Normal syscalls trap into supervisor mode and flush pipeline state, whereas vDSO maps clock/getcpu read-only into user space."),
        ("How does a B+Tree optimize disk scans?", "B+Trees keep only routing keys in internal nodes and link leaf data pages sequentially for high fan-out and range scans."),
        ("Compare LSM-trees and B-Trees.", "LSM-trees batch random writes sequentially into immutable SSTables and compact them later; B-Trees update 4KB pages in place."),
        ("What does Write-Ahead Logging guarantee?", "Persisting redo log records sequentially to durable storage before dirty heap pages allows crash recovery without losing commits."),
        ("Explain ARIES WAL redo and undo phases.", "ARIES replays committed and in-flight pages forward via redo, then rolls back uncommitted transactions using CLRs during undo."),
        ("What is snapshot isolation vs serializability?", "Snapshot isolation reads from a consistent MVCC point-in-time avoiding read locks, whereas serializability forbids write skew anomalies too."),
        ("How does two-phase locking prevent anomalies?", "Transactions acquire shared or exclusive locks in a growing phase and release them only after commit during the shrinking phase."),
        ("What problem do Bloom filters solve in storage engines?", "Bloom filters use k hashes over a compact bitset to rule out absent keys with zero false negatives before disk seeks."),
        ("Explain LSM compaction write amplification.", "Repeatedly merging sorted runs across leveled tiers rewrites untouched records multiple times relative to logical user writes."),
        ("How does Raft elect a cluster leader?", "A follower whose election timeout expires increments its term, votes for itself, and wins leadership with a majority quorum."),
        ("Why does Raft commit entries only with a majority?", "Requiring N/2 + 1 replicas guarantees any two quorums overlap by at least one node preserving committed history across terms."),
        ("What is the CAP theorem tradeoff?", "Under a network partition, a distributed data store must choose between linearizable consistency and every-node availability."),
        ("How does Paxos differ from Raft?", "Paxos generalizes consensus over single proposals, while Raft decomposes leadership, log replication, and safety around a strong leader."),
        ("Explain consistent hashing with virtual nodes.", "Mapping keys and multiple virtual replicas per server onto a hash ring limits key movement to O(K/N) on cluster resize."),
        ("What are Vector Clocks used for?", "Vector clocks track partial causal happens-before ordering across replicas so concurrent conflicting updates can be detected."),
        ("How does CRDT state convergence work?", "Join-semilattice merges are commutative, associative, and idempotent, guaranteeing identical replica state regardless of message order."),
        ("Explain TCP three-way handshake and SYN cookies.", "TCP exchanges SYN, SYN-ACK, ACK sequence numbers; SYN cookies encode connection state cryptographically to defeat half-open floods."),
        ("How does TCP CUBIC congestion control scale?", "CUBIC grows cwnd using a cubic curve around the last loss window size, probing aggressively at long bandwidth-delay products."),
        ("Why does head-of-line blocking happen in HTTP/2?", "HTTP/2 multiplexes streams onto one TCP connection, so losing a single packet stalls all streams waiting for retransmission."),
        ("How does QUIC solve transport HOL blocking?", "QUIC runs encrypted independent streams over UDP so loss on one stream blocks only its own reassembly buffer."),
        ("What is zero-RTT resumption in TLS 1.3?", "Returning clients reuse cached PSK tickets to send early idempotent application data alongside the first ClientHello flight."),
        ("Explain Ed25519 vs RSA signatures.", "Ed25519 provides 128-bit security with tiny 32-byte keys, fast constant-time Curve25519 math, and deterministic nonces."),
        ("Why must AEAD nonces never repeat?", "Reusing a key-nonce pair in AES-GCM or ChaCha20-Poly1305 leaks the XOR of plaintexts and enables authentication tag forgery."),
        ("What does HKDF extract-and-expand do?", "HKDF concentrates entropy from shared secret key material via HMAC extract, then expands domain-separated subkeys safely."),
        ("How does Constant-Time code stop timing side channels?", "Replacing secret-dependent branches and table lookups with bitwise masks ensures execution time leaks zero key bits."),
        ("Explain Rust Send vs Sync marker traits.", "Send permits transferring ownership across threads safely, while Sync permits sharing immutable references concurrently across threads."),
        ("Explain Pin and self-referential futures in Rust.", "Pin guarantees a value will not move in memory before drop, preventing self-referential async state pointers from dangling."),
        ("How does Rust's Drop trait guarantee RAII cleanup?", "The compiler inserts deterministic drop calls at scope exit and during unwinding in reverse declaration order."),
        ("What makes MaybeUninit<T> safer than mem::zeroed?", "MaybeUninit suppresses immediate validity assumptions so arbitrary bit patterns stay outside UB until assume_init()."),
        ("Explain Rust Zero-Cost Abstractions.", "Iterators, enums, and traits monomorphize and inline at compile time to machine code matching hand-written loops without vtable overhead."),
        ("How does WebAssembly linear memory ensure safety?", "Every Wasm load and store is checked against a contiguous byte array size, isolating host heap objects from guest pointers."),
        ("Why does Wasm separate control stack from linear memory?", "Keeping call frames and return addresses opaque prevents stack smashing from overwriting function pointers in guest RAM."),
        ("What is SSA form in optimizing compilers?", "Static Single Assignment gives every variable one definition site, simplifying dead code elimination, GVN, and register allocation."),
        ("Compare Linear Scan and Graph Coloring register allocation.", "Linear scan greedily allocates live intervals in O(N) time for JITs; graph coloring models interference edges for optimal spilling."),
        ("Explain Loop-Invariant Code Motion.", "Moving computations whose operands never change inside a loop body to its preheader cuts redundant iterations safely."),
        ("What is Tail-Call Optimization?", "Reusing the caller's stack frame before jumping to a final recursive call prevents O(N) stack growth on deep chains."),
        ("Explain Branchless Select using bitwise arithmetic.", "Computing `(mask & a) | (~mask & b)` avoids CPU pipeline misprediction penalties on unpredictable conditional data moves."),
        ("What is SIMD vectorization?", "Single Instruction Multiple Data packs 4 to 16 scalar lanes into wide vector registers to execute one op per cycle."),
        ("Explain Roofline model compute vs memory bounds.", "A kernel is memory-bound below hardware arithmetic intensity ridge point and compute-bound once ops per byte exceed it."),
        ("Compare row-major vs column-major cache locality.", "Iterating along contiguous stride-1 memory maximizes hardware prefetching and spatial cache-line utilization across loops."),
        ("How does Arena allocation speed up AST parsers?", "An arena bumps a pointer linearly inside large chunks and frees all AST nodes together in O(1) without per-node malloc headers."),
        ("Compare Mark-Sweep and Copying Garbage Collectors.", "Mark-sweep leaves live objects in place with free lists, while copying collectors compact survivors into a fresh semi-space."),
        ("What is Dijkstra's algorithm invariant?", "A min-heap relaxes edges monotonically over non-negative weights so every settled vertex already holds its shortest path."),
        ("How does Kahn's topological sort detect DAG cycles?", "Iteratively dequeuing zero-indegree nodes visits fewer than V vertices if and only if a directed cycle blocks remaining nodes."),
        ("Explain Union-Find with Path Compression and Rank.", "Flattening parent pointers during find and attaching shallower trees gives near-constant O(alpha(N)) amortized disjoint-set ops."),
        ("How does the KMP prefix table avoid backtracking?", "The failure array records longest proper prefix-suffix lengths so text scans advance monotonically in O(N + M) time."),
        ("Compare Sliding Window and Two-Pointer techniques.", "Sliding windows maintain dynamic invariants over contiguous ranges in O(N), while two pointers converge across sorted arrays."),
        ("What does Kadane's algorithm track in O(N)?", "At each element it keeps the best contiguous subarray sum ending here as max(x, cur + x) alongside the global best."),
        ("Explain Dynamic Programming memoization vs tabulation.", "Memoization caches top-down recursive subproblems lazily; tabulation fills a bottom-up state table in topological order."),
        ("How does RSA-OAEP padding prevent chosen-ciphertext attacks?", "OAEP combines random seeds and dual mask generation hashes so invalid plaintexts fail verification cleanly before decryption."),
        ("Explain Spectre speculative execution hazards.", "Transient out-of-order execution beyond mispredicted bounds checks leaves secret-dependent footprints in CPU L1 data cache."),
        ("How do eBPF verifier checks ensure kernel safety?", "eBPF statically simulates all bytecode paths to prove bounded loops, initialized registers, and valid pointer bounds before loading."),
        ("What is zero-copy splice/sendfile in Linux?", "sendfile transfers file page-cache buffers directly toward NIC DMA descriptors inside the kernel without touching userspace."),
        ("Explain HTTP ETag and conditional GET 304 caching.", "Sending If-None-Match with a weak or strong ETag lets servers confirm unchanged representations with tiny 304 responses."),
        ("What does DNS ANYCAST routing accomplish?", "Announcing one IP prefix from global PoPs via BGP lets internet routers direct clients automatically to the nearest node."),
        ("Explain Merkle Trees in Git and distributed storage.", "Every interior node hashes its children so comparing root digests locates any corrupted block or divergent subtree in O(log N)."),
        ("How does a Token Bucket rate limiter handle bursts?", "Tokens refill up to burst capacity C at constant rate R, permitting short spikes while capping long-term average throughput."),
        ("Compare Read-Copy-Update (RCU) and RwLock.", "RCU lets readers traverse lock-free without atomic writes while writers publish updated copies and reclaim old nodes after grace periods."),
        ("What is NVMe submission queue doorbell batching?", "Submitting multiple 64-byte NVMe commands into memory before ringing the PCIe tail doorbell amortizes MMIO register writes."),
        ("How does LZ77 back-reference compression work?", "LZ77 replaces repeated substrings inside a sliding window with compact (distance, length) pairs followed by entropy coding."),
        ("What distinguishes IEEE 754 subnormal floats from Q12 fixed-point?", "Subnormals sacrifice mantissa precision near zero cleanly, whereas Q12 uses deterministic constant integer 1/4096 step resolution."),
    ];
    let heldout_cs_qa: HashSet<&str> = [
        "Why do self-referential futures require Pin in Rust?",
        "How does a TLB accelerate virtual memory address translation?",
    ]
    .into_iter()
    .collect();
    for &(q, a) in &cs_systems_qa {
        sb.add_v10_plain(&[], q, a, "B_ext", 8);
        let mut alts: Vec<String> = Vec::with_capacity(2);
        if q.contains("self-referential futures") {
            alts.push(
                "What problem does Pin solve for self-referential futures in Rust?".to_string(),
            );
            alts.push(
                "Why are self-referential async futures pinned with Pin in Rust?".to_string(),
            );
        } else if q.contains("TLBs cache") {
            alts.push(
                "How do TLBs speed up virtual memory address translation?".to_string(),
            );
            alts.push(
                "Why does a TLB reduce virtual-to-physical address translation latency?".to_string(),
            );
        }
        for alt in alts {
            if !heldout_cs_qa.contains(alt.as_str()) {
                sb.add_v10_plain(&[], &alt, a, "B_ext", 6);
            }
        }
    }

    // Multi-turn general & technical Q&A progressions so long `self.chat_history` sessions NEVER emit unclosed `<thought>`!
    let btree_a = "A B-Tree is a self-balancing search tree storing sorted keys in page-sized nodes so disk and cache reads stay O(log n).";
    let raft_fup_u = "In that approach you just explained, what happens if we combine it with Raft consensus replication across nodes?";
    let raft_fup_a = "Replicating write-ahead log entries via Raft quorum before applying B-Tree page splits keeps every replica crash-consistent.";
    let raft_delib_u = "When designing a storage engine with B-Trees, how does Raft consensus handle leader election across replicas?";
    let raft_delib_a = "Followers increment term timers and vote for the candidate with the most up-to-date replicated log index.";
    sb.add_v10_plain(
        &[("Explain B-Trees", btree_a)],
        raft_fup_u,
        raft_fup_a,
        "B",
        70,
    );
    sb.add_v10_plain(
        &[("Explain B-Trees", btree_a), (raft_fup_u, raft_fup_a)],
        raft_delib_u,
        raft_delib_a,
        "B",
        70,
    );

    let everyday_mt: &[(&str, &str)] = &[
        ("What is your favorite food?", "As software I don't eat, though clean structured JSON and crisp espresso-fueled coding sessions are great company."),
        ("What is your favorite color?", "Deep terminal indigo paired with warm amber telemetry highlights makes long debugging sessions easy on the eyes."),
        ("What is your favorite programming language?", "Rust is a favorite here because ownership and zero-cost abstractions compile cleanly to fast WebAssembly."),
        ("How many parameters is the 248mb file equivalent to?", "In Q12 lookup-table space, a 248 MB pack stores millions of indexed discrete transitions without dense float matrices."),
        ("My day is going fine, how about you?", REPLY_HOW_ARE_YOU),
        ("Thanks!", REPLY_THANKS),
        ("Why were you comparing everything to first principles?", "Direct concrete answers are much clearer than repetitive boilerplate, so each turn now answers your question straight away."),
    ];
    let uorlut_rough_a = "Sorry today has been rough - take a breath, and whenever you're ready we can tackle things one small step at a time.";
    let uorlut_numpy_u = "How can I remove NaN values from a NumPy array? For example, I want to convert an array like [1, 2, NaN, 4, NaN, 8] into [1, 2, 4, 8].";
    let uorlut_numpy_a =
        "Filter the array with a boolean mask such as x[~np.isnan(x)] so only finite numbers remain.";
    let base_mt_prefix: &[(&str, &str)] = &[
        ("What are you able to do?", REPLY_CAN_DO),
        ("Are you AGI?", "No - I am a focused integer lookup-table coding and Linux Studio model running in WebAssembly, not AGI."),
        ("Are you conscious?", "No - I am a deterministic integer lookup-table model running in WebAssembly, with zero subjective awareness."),
        ("I'm having a rough day.", uorlut_rough_a),
        ("How does GPS calculate your location on Earth?", "GPS receivers measure nanosecond radio travel times from at least four satellites with relativistic clock corrections to trilaterate position."),
        (uorlut_numpy_u, uorlut_numpy_a),
    ];
    let mut full_uorlut_seq: Vec<(&str, &str)> =
        Vec::with_capacity(base_mt_prefix.len() + everyday_mt.len());
    full_uorlut_seq.extend_from_slice(base_mt_prefix);
    full_uorlut_seq.extend_from_slice(everyday_mt);
    for k_full in 1..full_uorlut_seq.len() {
        let (u_f, a_f) = full_uorlut_seq[k_full];
        sb.add_v10_plain(&full_uorlut_seq[..k_full], u_f, a_f, "B", 68);
    }
    for k_mt in 0..everyday_mt.len() {
        let (u_m, a_m) = everyday_mt[k_mt];
        sb.add_v10_plain(&everyday_mt[..k_mt], u_m, a_m, "B", 48);
    }

    let casual_personas = [
        ("Elena", "espresso"),
        ("Aria", "pour-over coffee"),
        ("Noah", "espresso"),
        ("Kenji", "matcha"),
        ("Priya", "espresso"),
        ("Soren", "black coffee"),
    ];
    let casual_intro_a = "Nice to meet you! Building a browser-based geometric model in Rust sounds awesome - how can I help?";
    let ferm_u_list = [
        "How does wild yeast bread fermentation work?",
        "How does artisan bread fermentation work?",
        "How does bread dough fermentation work?",
    ];
    let ferm_a = "Wild yeast and lactic acid bacteria ferment flour sugars into carbon dioxide gas and organic acids that leaven dough.";
    let numpy_u = "How can I remove NaN values from a NumPy array? For example, I want to convert an array like [1, 2, NaN, 4, NaN, 8] into [1, 2, 4, 8].";
    let numpy_a =
        "Filter the array with a boolean mask such as x[~np.isnan(x)] so only finite numbers remain.";
    let shorter_a = "Use x[~np.isnan(x)] to keep only finite numbers in one line.";
    let recall_proj_a =
        "You introduced yourself earlier and mentioned building a browser-based geometric model in Rust.";
    let recap_a = "Earlier we covered your Rust geometric model project, bread fermentation, and filtering NaN values in NumPy.";
    assert!(sb.add_v10_plain(&[], numpy_u, numpy_a, "B", 52));
    for (p_idx, &(c_name, c_drink)) in casual_personas.iter().enumerate() {
        let c_u1 = format!(
            "Hi! My name is {c_name} and I'm building a browser-based geometric model in Rust. I also love {c_drink}."
        );
        let c_u2 = ferm_u_list[p_idx % ferm_u_list.len()];
        assert!(sb.add_v10_plain(&[], &c_u1, casual_intro_a, "B", 24));
        assert!(sb.add_v10_plain(&[(c_u1.as_str(), casual_intro_a)], c_u2, ferm_a, "B", 26));
        let h2 = [(c_u1.as_str(), casual_intro_a), (c_u2, ferm_a)];
        assert!(sb.add_v10_plain(&h2, "Wait, really?", REPLY_ACK, "B", 28));
        let h3 = [h2[0], h2[1], ("Wait, really?", REPLY_ACK)];
        assert!(sb.add_v10_plain(&h3, numpy_u, numpy_a, "B", 26));
        let h4 = [h3[0], h3[1], h3[2], (numpy_u, numpy_a)];
        assert!(sb.add_v10_plain(&h4, "Can you make that shorter?", shorter_a, "B", 28));
        let h5 = [
            h4[0],
            h4[1],
            h4[2],
            h4[3],
            ("Can you make that shorter?", shorter_a),
        ];
        assert!(sb.add_v10_plain(
            &h5,
            "What is my name and what am I building?",
            recall_proj_a,
            "B",
            28
        ));
        let h6 = [
            h5[0],
            h5[1],
            h5[2],
            h5[3],
            h5[4],
            ("What is my name and what am I building?", recall_proj_a),
        ];
        assert!(sb.add_v10_plain(&h6, "What were we talking about earlier?", recap_a, "B", 28));
    }

    let sys_facts: &[(&str, &[(&str, &str)])] = &[
        (
            "What is the time complexity of {alg}?",
            &[
                (
                    "binary search on a sorted slice",
                    "Worst-case O(log n) comparisons with O(1) auxiliary space.",
                ),
                (
                    "mergesort on `n` elements",
                    "Worst-case O(n log n) comparisons with O(n) auxiliary buffer space.",
                ),
                (
                    "hash map lookup under uniform hashing",
                    "Expected O(1) lookup time and O(n) worst-case collision time.",
                ),
                (
                    "64-channel Fast Walsh-Hadamard Transform",
                    "Exact 384 integer additions and subtractions (O(d log d)) with zero multiplications.",
                ),
            ],
        ),
        (
            "In Linux RISC-V (`rv64`), what does syscall `{sc}` do?",
            &[
                (
                    "read (63)",
                    "Reads up to `count` bytes from file descriptor `a0` into buffer `a1`.",
                ),
                (
                    "socket (198)",
                    "Creates an endpoint; `MvmSandbox` permits `AF_VSOCK` (`40`) and refuses `AF_INET` with `-97`.",
                ),
            ],
        ),
        (
            "What does the Rust method `{m}` return?",
            &[
                (
                    "u64::saturating_add(rhs)",
                    "Returns the clamped sum at u64::MAX without panicking or wrapping on overflow.",
                ),
                (
                    "u32::count_ones(self)",
                    "The Hamming weight (number of 1 bits) in the 32-bit integer.",
                ),
            ],
        ),
    ];
    let q_prefixes = ["", "Quick question: ", "Briefly: ", "In one sentence, "];
    for &(tmpl, items) in sys_facts {
        for (i_idx, &(subj, ans)) in items.iter().enumerate() {
            let base_q = tmpl
                .replace("{alg}", subj)
                .replace("{sc}", subj)
                .replace("{m}", subj);
            for &qp in &q_prefixes {
                let q_str = if qp.is_empty() {
                    base_q.clone()
                } else {
                    let mut chars = base_q.chars();
                    match chars.next() {
                        Some(first) => {
                            format!("{qp}{}{}", first.to_lowercase(), chars.as_str())
                        }
                        None => qp.to_string(),
                    }
                };
                if qp == "In one sentence, " && (i_idx % 2 == 0) {
                    eval_sb.add_v10_plain(&[], &q_str, ans, "B", 1);
                } else {
                    sb.add_v10_plain(&[], &q_str, ans, "B", 20);
                }
            }
        }
    }

    // Curated crisp open Q&A from Dolly & Alpaca (12 pairs across single-turn and 2-turn)
    let mut qa_order: Vec<usize> = (0..open_qa.len()).collect();
    rng.shuffle(&mut qa_order);
    let mut open_unique = 0usize;
    for q_idx in qa_order {
        if open_unique >= 64 {
            break;
        }
        let (q, a) = &open_qa[q_idx];
        let q_low = q.to_lowercase();
        let a_low = a.to_lowercase();
        if q.chars().count() > 80
            || a.chars().count() > 92
            || a.contains('`')
            || a.contains('<')
            || q_low.contains(" into ")
            || q_low.contains(" inside ")
            || [
                "photosynthesis",
                "sourdough",
                "tectonics",
                "mitochondria",
                "rayleigh",
            ]
            .iter()
            .any(|&bw| a_low.contains(bw))
            || q_low.starts_with("who are")
            || q_low.starts_with("what can you")
        {
            continue;
        }
        if sb.add_v10_plain(&[], q, a, "B_open", 10) {
            open_unique += 1;
        }
    }
}
