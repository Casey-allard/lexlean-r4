//! Pure-Rust offline corpus generator for UOR-R4-GLLM v11 (`UORCORP3` / `UORPRMP1` / `UORVOCB9`),
//! strictly preserving ByteBpe4096 alignment, 1024-token multi-turn rolling windows,
//! `#165` structured GitHub `git` `<tool_call>` schemas, and clean train/eval disjointness.
//! Zero Python used anywhere.

#[path = "bpe_json.rs"]
mod bpe_json;
#[path = "v10_corpus_ab.rs"]
mod v10_corpus_ab;
#[path = "v10_corpus_cde.rs"]
mod v10_corpus_cde;

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bpe_json::{
    curl_download_cached, curl_fetch_stdout, json_escape, sha256_hex, ByteBpe4096, JsonVal, Rng64,
};
use v10_corpus_ab::{
    clean_text, contains_forbidden, export_vocab_raw_bytes, first_crisp_sentence,
    generate_bucket_a_conversations, generate_bucket_b_qa, norm_key, CorpusRecord, SampleBuilder,
    ALPACA_URL, BANNED_BOILERPLATE, CODE_ALPACA_URL, DOLLY_URL, MAX_SEQ, REPLY_HI,
    REPLY_HOW_ARE_YOU, REPLY_WHO, SMOLTALK_PARQUET_URLS, TARGET_EVAL_COUNT, TARGET_TRAIN_COUNT,
};
use v10_corpus_cde::{
    generate_bucket_c_code, generate_bucket_d_mvm_and_git_tools, generate_bucket_e_web_tools,
    top_up_to_target,
};

/// Held-out v11 identifiers, repos, branches, and names that MUST NOT appear in training data.
const V11_FORBIDDEN_HELD_OUT: &[&str] = &[
    "aurora-mesh",
    "quantum-ledger",
    "helix-proxy",
    "cobalt-kernel",
    "feat/rope-kv-v11",
    "fix/stitch-parity-check",
    "ops/deploy-watch-v11",
    "afflom/stitch-demo",
    "kestrel-org/vault-ui",
    "dana-chen/prism-forge",
    "mira-solis",
    "elena-vance",
    "sensor_drift_k9",
    "base_offset_q3",
    "afflom",
    "stitch-demo",
    "uor-r4-test",
    "feat/v11-test",
    "rope-kv-v11",
    "gllm-v11",
    "sigma_depth_k9",
];

fn contains_any_forbidden(text: &str) -> bool {
    if contains_forbidden(text) {
        return true;
    }
    let low = text.to_ascii_lowercase();
    V11_FORBIDDEN_HELD_OUT.iter().any(|&f| low.contains(f))
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(par) = path.parent() {
        if !par.as_os_str().is_empty() {
            fs::create_dir_all(par).map_err(|e| format!("mkdir {}: {e}", par.display()))?;
        }
    }
    Ok(())
}

fn parent_or_dot(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

fn extract_smoltalk_pairs(msgs: &[JsonVal]) -> Option<Vec<(String, String)>> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut pending_u: Option<String> = None;
    for m in msgs {
        let role = m
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let content = clean_text(m.get("content").and_then(|v| v.as_str()).unwrap_or(""));
        if role == "user" {
            pending_u = Some(content);
        } else if role == "assistant" {
            if let Some(u) = pending_u.take() {
                if !u.is_empty() && !content.is_empty() {
                    pairs.push((u, content));
                }
            }
        }
    }
    if pairs.is_empty() {
        None
    } else {
        Some(pairs)
    }
}

fn load_smoltalk_dialogues(cache_dir: &Path) -> Vec<Vec<(String, String)>> {
    let cache_path = cache_dir.join("smoltalk_everyday.jsonl");
    let mut dialogues: Vec<Vec<(String, String)>> = Vec::new();
    if cache_path.is_file() {
        if let Ok(raw) = fs::read_to_string(&cache_path) {
            for line in raw.lines() {
                let s = line.trim();
                if s.is_empty() {
                    continue;
                }
                if let Ok(val) = JsonVal::parse(s) {
                    if let Some(msgs) = val.get("messages").and_then(|v| v.as_array()) {
                        if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                            dialogues.push(pairs);
                        }
                    }
                }
            }
            if !dialogues.is_empty() {
                println!(
                    "[corpus-v11] loaded {} SmolTalk dialogues from cache",
                    dialogues.len()
                );
                return dialogues;
            }
        }
    }

    let _ = fs::create_dir_all(cache_dir);
    let mut cached_lines: Vec<String> = Vec::new();
    let mut failed = 0usize;
    for offset in (0..=2200usize).step_by(100) {
        let url = format!(
            "https://datasets-server.huggingface.co/rows?dataset=HuggingFaceTB/smoltalk&config=everyday-conversations&split=train&offset={offset}&length=100"
        );
        match curl_fetch_stdout(&url) {
            Ok(body) => {
                if let Ok(root) = JsonVal::parse(&body) {
                    if let Some(rows) = root.get("rows").and_then(|v| v.as_array()) {
                        for item in rows {
                            if let Some(msgs) = item
                                .get("row")
                                .and_then(|r| r.get("messages"))
                                .and_then(|m| m.as_array())
                            {
                                let mut line = String::from("{\"messages\":[");
                                for (idx, msg) in msgs.iter().enumerate() {
                                    if idx > 0 {
                                        line.push(',');
                                    }
                                    let role =
                                        msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
                                    let content =
                                        msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                                    line.push_str("{\"role\":");
                                    line.push_str(&json_escape(role));
                                    line.push_str(",\"content\":");
                                    line.push_str(&json_escape(content));
                                    line.push('}');
                                }
                                line.push_str("]}");
                                cached_lines.push(line);
                                if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                                    dialogues.push(pairs);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                failed += 1;
                if failed >= 3 && cached_lines.is_empty() {
                    break;
                }
            }
        }
    }
    if !cached_lines.is_empty() {
        let _ = fs::write(&cache_path, cached_lines.join("\n"));
    } else {
        let _ = SMOLTALK_PARQUET_URLS;
    }
    println!(
        "[corpus-v11] loaded {} SmolTalk everyday dialogues",
        dialogues.len()
    );
    dialogues
}

fn load_dolly_and_alpaca(cache_dir: &Path) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut qa_pairs: Vec<(String, String)> = Vec::new();
    let mut code_pairs: Vec<(String, String)> = Vec::new();

    let dolly_path = cache_dir.join("dolly_15k.jsonl");
    if matches!(
        curl_download_cached(DOLLY_URL, &dolly_path, "Dolly-15k"),
        Ok(true)
    ) {
        if let Ok(raw) = fs::read_to_string(&dolly_path) {
            for line in raw.lines() {
                if let Ok(obj) = JsonVal::parse(line.trim()) {
                    let cat = obj.get("category").and_then(|v| v.as_str()).unwrap_or("");
                    let ctx = clean_text(obj.get("context").and_then(|v| v.as_str()).unwrap_or(""));
                    let inst = clean_text(
                        obj.get("instruction")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    );
                    let resp =
                        clean_text(obj.get("response").and_then(|v| v.as_str()).unwrap_or(""));
                    if !matches!(cat, "open_qa" | "general_qa" | "brainstorming")
                        || !ctx.is_empty()
                        || inst.len() < 12
                        || inst.len() > 110
                        || inst.contains('\n')
                    {
                        continue;
                    }
                    let crisp = first_crisp_sentence(&resp, 130);
                    let low = format!("{inst} {crisp}").to_ascii_lowercase();
                    if crisp.len() >= 18
                        && inst.is_ascii()
                        && crisp.is_ascii()
                        && !BANNED_BOILERPLATE.iter().any(|&b| low.contains(b))
                        && !contains_any_forbidden(&low)
                    {
                        qa_pairs.push((inst, crisp));
                    }
                }
            }
        }
    }

    let alpaca_path = cache_dir.join("alpaca_cleaned.json");
    if matches!(
        curl_download_cached(ALPACA_URL, &alpaca_path, "Alpaca-cleaned"),
        Ok(true)
    ) {
        if let Ok(raw) = fs::read_to_string(&alpaca_path) {
            if let Some(rows) = JsonVal::parse(&raw)
                .ok()
                .and_then(|r| r.as_array().map(|a| a.to_vec()))
            {
                for row in rows {
                    let inst = clean_text(
                        row.get("instruction")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    );
                    let inp = clean_text(row.get("input").and_then(|v| v.as_str()).unwrap_or(""));
                    let out = clean_text(row.get("output").and_then(|v| v.as_str()).unwrap_or(""));
                    if !inp.is_empty() || inst.len() < 12 || inst.len() > 110 || inst.contains('\n')
                    {
                        continue;
                    }
                    let crisp = first_crisp_sentence(&out, 130);
                    let low = format!("{inst} {crisp}").to_ascii_lowercase();
                    if crisp.len() >= 18
                        && inst.is_ascii()
                        && crisp.is_ascii()
                        && !BANNED_BOILERPLATE.iter().any(|&b| low.contains(b))
                        && !contains_any_forbidden(&low)
                    {
                        qa_pairs.push((inst, crisp));
                    }
                }
            }
        }
    }

    let code_path = cache_dir.join("code_alpaca_20k.json");
    if matches!(
        curl_download_cached(CODE_ALPACA_URL, &code_path, "CodeAlpaca-20k"),
        Ok(true)
    ) {
        if let Ok(raw) = fs::read_to_string(&code_path) {
            if let Some(rows) = JsonVal::parse(&raw)
                .ok()
                .and_then(|r| r.as_array().map(|a| a.to_vec()))
            {
                for row in rows {
                    let inst = clean_text(
                        row.get("instruction")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    );
                    let inp = clean_text(row.get("input").and_then(|v| v.as_str()).unwrap_or(""));
                    let mut out =
                        clean_text(row.get("output").and_then(|v| v.as_str()).unwrap_or(""));
                    if !inp.is_empty() || inst.contains('\n') || inst.len() < 14 || inst.len() > 105
                    {
                        continue;
                    }
                    let nl = out.bytes().filter(|&b| b == b'\n').count();
                    let low = format!("{inst} {out}").to_ascii_lowercase();
                    if out.len() < 10
                        || out.len() > 140
                        || nl > 6
                        || !inst.is_ascii()
                        || !out.is_ascii()
                        || BANNED_BOILERPLATE.iter().any(|&b| low.contains(b))
                        || contains_any_forbidden(&low)
                    {
                        continue;
                    }
                    if out.starts_with("```") {
                        out = format!("Here is a concise code snippet:\n{out}");
                    } else {
                        let lang = if low.contains("rust") {
                            "rust"
                        } else if low.contains("javascript") {
                            "javascript"
                        } else {
                            "sh"
                        };
                        out = format!("Here is a concise {lang} snippet:\n```{lang}\n{out}\n```");
                    }
                    code_pairs.push((inst, out));
                }
            }
        }
    }
    println!(
        "[corpus-v11] loaded {} Q&A pairs, {} code pairs",
        qa_pairs.len(),
        code_pairs.len()
    );
    (qa_pairs, code_pairs)
}

const V11_STEMS_A: &[&str] = &[
    "alder", "amber", "anvil", "arbor", "aspen", "aster", "badge", "balsa", "basil", "beacon",
    "beech", "birch", "bluff", "brass", "briar", "brine", "brook", "cairn", "cedar", "chalk",
    "chime", "clover", "cobble", "comet", "copper", "coral", "cove", "crag", "creek", "crest",
    "cypress", "dahlia", "dawn", "dune", "elder", "elm", "fern", "firth", "flint", "flora",
    "foam", "frost", "gable", "garnet", "gale", "glade", "granite", "grove", "hazel", "heath",
    "heron", "hollow", "ivory", "jade", "juniper", "kaolin", "kelp", "kiln", "larch", "laurel",
    "lichen", "linden", "loam", "lupine",
];

const V11_STEMS_B: &[&str] = &[
    "anchor", "arc", "array", "axle", "band", "beam", "bell", "bench", "blade", "block",
    "board", "bolt", "bond", "brace", "bridge", "bucket", "buffer", "byte", "cable", "cage",
    "card", "cell", "chain", "channel", "chart", "chip", "chord", "clamp", "clasp", "clip",
    "clock", "cloud", "coil", "cone", "cord", "core", "craft", "crane", "crown", "cube",
    "dial", "disc", "dock", "dome", "draft", "drive", "drum", "edge", "engine", "field",
    "file", "filter", "frame", "gate", "gauge", "gear", "grid", "guide", "hinge", "hub",
    "index", "latch", "link", "loom",
];

const V11_ONSETS: &[&str] = &[
    "b", "br", "c", "cr", "d", "dr", "f", "fl", "fr", "g", "gr", "h", "j", "k", "l", "m",
    "n", "p", "pl", "pr", "r", "s", "st", "t", "tr", "v", "w", "z",
];
const V11_VOWELS: &[&str] = &["a", "e", "i", "o", "u"];
const V11_CODAS: &[&str] = &[
    "d", "f", "g", "k", "l", "m", "n", "p", "r", "s", "t", "x", "ld", "nd", "nk", "nt", "rk",
    "rm", "rn", "rt", "st", "",
];

fn gen_comb_word(rng: &mut Rng64) -> String {
    if (rng.next_u64() & 1) == 0 {
        let a = V11_STEMS_A[(rng.next_u64() as usize) % V11_STEMS_A.len()];
        a.to_string()
    } else {
        let syls = 2 + ((rng.next_u64() as usize) & 1);
        let mut s = String::with_capacity(14);
        for _ in 0..syls {
            s.push_str(V11_ONSETS[(rng.next_u64() as usize) % V11_ONSETS.len()]);
            s.push_str(V11_VOWELS[(rng.next_u64() as usize) % V11_VOWELS.len()]);
            s.push_str(V11_CODAS[(rng.next_u64() as usize) % V11_CODAS.len()]);
        }
        s
    }
}

fn gen_comb_repo(rng: &mut Rng64) -> String {
    loop {
        let w1 = gen_comb_word(rng);
        let w2 = V11_STEMS_B[(rng.next_u64() as usize) % V11_STEMS_B.len()];
        let candidate = match rng.next_u64() % 4 {
            0 => format!("{w1}-{w2}"),
            1 => format!("{w1}-{w2}-{}", 10 + (rng.next_u64() % 890)),
            2 => {
                let tag_ch = (b'a' + (rng.next_u64() % 26) as u8) as char;
                format!("{w1}-{w2}-{tag_ch}{}", rng.next_u64() % 10)
            }
            _ => {
                let w3 = V11_STEMS_A[(rng.next_u64() as usize) % V11_STEMS_A.len()];
                format!("{w1}-{w3}-{w2}")
            }
        };
        if !contains_any_forbidden(&candidate) {
            return candidate;
        }
    }
}

fn gen_comb_owner(rng: &mut Rng64) -> String {
    const SUF: &[&str] = &["lab", "dev", "org", "sys", "io", "rs", "net", "ops"];
    loop {
        let w1 = gen_comb_word(rng);
        let candidate = match rng.next_u64() % 3 {
            0 => format!("{w1}-{}", SUF[(rng.next_u64() as usize) % SUF.len()]),
            1 => format!(
                "{w1}-{}",
                V11_STEMS_A[(rng.next_u64() as usize) % V11_STEMS_A.len()]
            ),
            _ => format!("{w1}{}", 10 + (rng.next_u64() % 89)),
        };
        if !contains_any_forbidden(&candidate) {
            return candidate;
        }
    }
}

fn gen_comb_branch(rng: &mut Rng64) -> String {
    const KINDS: &[&str] = &["feat", "fix", "chore", "ops", "perf", "docs", "ci"];
    loop {
        let k = KINDS[(rng.next_u64() as usize) % KINDS.len()];
        let w1 = gen_comb_word(rng);
        let w2 = V11_STEMS_B[(rng.next_u64() as usize) % V11_STEMS_B.len()];
        let candidate = match rng.next_u64() % 3 {
            0 => format!("{k}/{w1}-{w2}-v{}", 1 + (rng.next_u64() % 98)),
            1 => format!("{k}/{w1}-{w2}"),
            _ => format!("{k}/{w1}-{}", rng.next_u64() % 997),
        };
        if !contains_any_forbidden(&candidate) {
            return candidate;
        }
    }
}

fn gen_comb_var(rng: &mut Rng64) -> String {
    const TCH: &[u8] = b"abcdefghjkmnpqrstuvwxyz";
    loop {
        let w1 = gen_comb_word(rng);
        let w2 = V11_STEMS_B[(rng.next_u64() as usize) % V11_STEMS_B.len()];
        let tc = TCH[(rng.next_u64() as usize) % TCH.len()] as char;
        let td = rng.next_u64() % 10;
        let candidate = match rng.next_u64() % 3 {
            0 => format!("{w1}_{w2}_{tc}{td}"),
            1 => format!("{w1}_{w2}"),
            _ => format!("{w1}_{tc}{td}"),
        };
        if !contains_any_forbidden(&candidate) {
            return candidate;
        }
    }
}

/// Generates v11-specific 4–8 turn conversational continuity + entity-update dialogues (covering 100..720 prompt tokens),
/// all 8 `#165` structured GitHub `git` `<tool_call>` schemas + `<tool_response>` multi-turn chains,
/// and diverse unseen-identifier Rust code copy pairs (`F_code`).
fn generate_bucket_f_v11_multiturn_and_github<'a>(
    sb: &mut SampleBuilder<'a>,
    eval_sb: &mut SampleBuilder<'a>,
    rng: &mut Rng64,
    smoltalk: &[Vec<(String, String)>],
) {
    // 1. Long-window SmolTalk multi-turn continuations (4..7 turns reaching 180..700 prompt tokens -> full RoPE sweep)
    for (idx, dlg) in smoltalk.iter().enumerate() {
        if dlg.len() < 4 {
            continue;
        }
        let max_turns = dlg.len().min(7);
        for target_turn in 3..max_turns {
            let mut turns_before: Vec<(String, String)> = Vec::new();
            let mut ok = true;
            for (u, a) in &dlg[..target_turn] {
                if !u.is_ascii()
                    || !a.is_ascii()
                    || contains_any_forbidden(u)
                    || contains_any_forbidden(a)
                {
                    ok = false;
                    break;
                }
                turns_before.push((u.clone(), a.clone()));
            }
            if !ok {
                continue;
            }
            let (ref u_now, ref a_now) = dlg[target_turn];
            if !u_now.is_ascii()
                || !a_now.is_ascii()
                || a_now.len() < 12
                || a_now.len() > 220
                || contains_any_forbidden(u_now)
                || contains_any_forbidden(a_now)
            {
                continue;
            }
            let target = if idx % 20 == 19 {
                &mut *eval_sb
            } else {
                &mut *sb
            };
            let boost = if idx % 20 == 19 { 1 } else { 2 };
            target.add_v10_plain_owned(&turns_before, u_now, a_now, "F_smol_mt", boost);
        }
    }

    // 2. 4–7 turn personal/workspace memory & entity-correction dialogues across 200..650 tokens
    let langs = ["Rust", "C99", "TypeScript", "WGSL", "SQLite SQL"];
    let user_names = [
        "alex", "blair", "casey_r", "devon", "ellis", "finley", "gray", "harper", "indigo",
        "jordan", "kieran", "logan", "morgan", "noah", "parker", "quinn", "riley", "sawyer",
        "taylor",
    ];
    let filler_turns: &[(&str, &str)] = &[
        ("hi", REPLY_HI),
        ("how are you?", REPLY_HOW_ARE_YOU),
        ("who are you?", REPLY_WHO),
        (
            "What does deterministic Q12 mean?",
            "Q12 represents signed fixed-point numbers scaled by 4096 per unit so integer arithmetic never diverges.",
        ),
        (
            "Remind me why WebAssembly workers stay responsive.",
            "Running GLLM inference inside a dedicated Worker keeps heavy loops off the browser UI thread.",
        ),
        (
            "Why avoid floats in safety gates?",
            "Integer arithmetic avoids platform rounding drift and matches Lean formal proofs bit-for-bit.",
        ),
        (
            "What does PRLL order-4 conserve?",
            "Fourth-order rational integer attention normalizes every causal weight distribution to exactly 1,000,000.",
        ),
    ];

    for i in 0..1100usize {
        let repo_a = gen_comb_repo(rng);
        let mut repo_b = gen_comb_repo(rng);
        if repo_b == repo_a {
            repo_b = format!("{repo_b}-x");
        }
        let uname = user_names[i % user_names.len()];
        let lang = langs[(i / 3) % langs.len()];
        if contains_any_forbidden(&repo_a)
            || contains_any_forbidden(&repo_b)
            || contains_any_forbidden(uname)
        {
            continue;
        }
        let is_eval = i >= 850;
        let target = if is_eval { &mut *eval_sb } else { &mut *sb };
        let boost = if is_eval {
            1
        } else if i < 250 {
            4
        } else {
            2
        };

        let mut turns: Vec<(String, String)> = Vec::new();
        let f0 = filler_turns[i % filler_turns.len()];
        turns.push((f0.0.to_string(), f0.1.to_string()));
        turns.push((
            format!("My name is {uname} and my primary repo is called {repo_a}."),
            format!("Got it, {uname} - tracking {repo_a} in your workspace context."),
        ));
        // Add 2..4 realistic intermediate conversation turns
        let n_fill = 2 + (i % 3);
        for f_k in 1..=n_fill {
            let ft = filler_turns[(i + f_k) % filler_turns.len()];
            turns.push((ft.0.to_string(), ft.1.to_string()));
        }
        if i % 3 == 0 {
            turns.push((
                format!("Prefer {lang} for generated helpers."),
                format!("Understood - using {lang} conventions going forward."),
            ));
        }

        // Question 1: recall initial repo cleanly across turns
        let repo_sp = format!(" {repo_a}");
        let mt_bkt = if is_eval || i < 280 {
            "F_mt"
        } else {
            "F_mt_ext"
        };
        target.add_v10_segments_owned(
            &turns,
            "Which repository am I working on right now?",
            &[
                ("Your active repository is called", false),
                (repo_sp.as_str(), true),
                (".", false),
            ],
            mt_bkt,
            boost,
        );

        // Question 1b: 2-turn & 4–6 turn `"my repo is called {repo}"` -> `"what is my repo called?"`
        if i < 700 {
            let fresh_turns = vec![(
                format!("my repo is called {repo_a}"),
                "Noted your saved name here for you.".to_string(),
            )];
            target.add_v10_segments_owned(
                &fresh_turns,
                "what is my repo called?",
                &[
                    ("<thought>kv recall</thought>\nYour repository is called:", false),
                    (repo_sp.as_str(), true),
                    (".", false),
                ],
                mt_bkt,
                if is_eval { 1 } else { 2 },
            );
        }

        let mut kv_turns: Vec<(String, String)> = if i % 2 == 0 {
            vec![
                ("hi".to_string(), REPLY_HI.to_string()),
                ("how are you?".to_string(), REPLY_HOW_ARE_YOU.to_string()),
                ("who are you?".to_string(), REPLY_WHO.to_string()),
            ]
        } else {
            vec![(f0.0.to_string(), f0.1.to_string())]
        };
        kv_turns.push((
            format!("my repo is called {repo_a}"),
            "Noted your saved name here for you.".to_string(),
        ));
        if i % 2 == 1 {
            kv_turns.push((
                "Explain binary search briefly.".to_string(),
                "Binary search halves a sorted range on every comparison to locate a target in O(log n) time.".to_string(),
            ));
            kv_turns.push((
                "can you explain that simpler?".to_string(),
                "Check the middle item, throw away the half where the answer cannot be, and repeat until you find it.".to_string(),
            ));
            let n_extra = (i / 2) % 2;
            for fk in 0..=n_extra {
                let ft = filler_turns[(i + fk) % filler_turns.len()];
                kv_turns.push((ft.0.to_string(), ft.1.to_string()));
            }
        }
        target.add_v10_segments_owned(
            &kv_turns,
            "what is my repo called?",
            &[
                ("<thought>kv recall</thought>\nYour repository is called:", false),
                (repo_sp.as_str(), true),
                (".", false),
            ],
            mt_bkt,
            if is_eval { 1 } else { 3 },
        );

        // Question 2: entity update / correction test (must return NEW repo_b, not stale repo_a!)
        if i % 2 == 0 {
            turns.push((
                "Which repository am I working on right now?".to_string(),
                format!("Your active repository is called {repo_a}."),
            ));
            turns.push((
                format!("Actually switch my active repo to {repo_b} now."),
                format!("Switched context to {repo_b}."),
            ));
            let ft_extra = filler_turns[(i + 4) % filler_turns.len()];
            turns.push((ft_extra.0.to_string(), ft_extra.1.to_string()));
            let rb_sp = format!(" {repo_b}");
            target.add_v10_segments_owned(
                &turns,
                "Remind me what my active repository is called now.",
                &[
                    ("Your updated repository is called", false),
                    (rb_sp.as_str(), true),
                    (".", false),
                ],
                if is_eval || i < 200 {
                    "F_mt"
                } else {
                    "F_mt_ext"
                },
                boost,
            );
        } else {
            let mut codename_turns = vec![
                (
                    format!("Remember that our project codename is {repo_a}."),
                    format!("Noted - tracking project codename {repo_a}."),
                ),
                (
                    "Keep every telemetry counter purely deterministic integer Q12.".to_string(),
                    "Understood - keeping every counter in deterministic Q12 integer form."
                        .to_string(),
                ),
                (
                    format!("Update the project codename to {repo_b} now."),
                    format!("Updated active project codename to {repo_b}."),
                ),
                (
                    "Thanks, keep going concisely.".to_string(),
                    "Ready whenever you are.".to_string(),
                ),
            ];
            let rb_sp = format!(" {repo_b}");
            target.add_v10_segments_owned(
                &codename_turns,
                "Summarize our codename and arithmetic setup cleanly.",
                &[
                    ("Active project codename is", false),
                    (rb_sp.as_str(), true),
                    (" with deterministic Q12 integer arithmetic.", false),
                ],
                mt_bkt,
                if is_eval { 1 } else { 2 },
            );
            codename_turns.push((
                "Summarize our codename and arithmetic setup cleanly.".to_string(),
                format!(
                    "Active project codename is {repo_b} with deterministic Q12 integer arithmetic."
                ),
            ));
            target.add_v10_segments_owned(
                &codename_turns,
                "What is our updated codename?",
                &[
                    ("Your updated project codename is", false),
                    (rb_sp.as_str(), true),
                    (".", false),
                ],
                mt_bkt,
                1,
            );
        }
    }

    // 3. Structured `#165` GitHub `git` `<tool_call>` JSON emission + multi-turn chains for ALL 8 actions
    let paths = [
        "Cargo.toml",
        "src/lib.rs",
        "src/main.rs",
        "README.md",
        "crates/core/src/main.rs",
        "crates/wasm/src/lib.rs",
        "docs/architecture.md",
    ];

    for g_i in 0..2400usize {
        let owner = gen_comb_owner(rng);
        let rname = gen_comb_repo(rng);
        let slug = format!("{owner}/{rname}");
        let br = gen_comb_branch(rng);
        let fpath = paths[(g_i / 3) % paths.len()];
        let pr_num = 1 + (rng.next_u64() % 990) as usize;
        if contains_any_forbidden(&slug) || contains_any_forbidden(&br) {
            continue;
        }
        let is_eval = g_i >= 2280;
        let target = if is_eval { &mut *eval_sb } else { &mut *sb };
        let bkt = if is_eval || g_i < 720 {
            "F_git"
        } else {
            "F_git_ext"
        };
        let boost = if is_eval {
            1
        } else if g_i < 480 {
            4
        } else {
            2
        };

        let slug_sp = format!(" {slug}");
        let br_sp = format!(" {br}");
        let fpath_sp = format!(" {fpath}");
        let pr_sp = format!(" {pr_num}");

        match g_i % 8 {
            0 => {
                // clone (structured #165)
                let p = format!("Clone GitHub repo {slug} using the git bridge.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"clone\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
                let obs = format!("Cloned {slug} HEAD 9a41e2f on branch main (12 files)");
                let obs_sp = format!(" {obs}");
                let sum_p = format!("Summarize the verified tool output for `{p}`:\n<tool_response> {obs}</tool_response>");
                target.add_v10_segments(
                    &[],
                    &sum_p,
                    &[
                        ("Verified GitHub bridge result:", false),
                        (obs_sp.as_str(), true),
                    ],
                    bkt,
                    1,
                );
            }
            1 => {
                // read_blob
                let p = format!("Read file {fpath} from GitHub repo {slug} on main.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"read_blob\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"path\":\"", false),
                        (fpath_sp.as_str(), true),
                        ("\",\"branch\":\"main\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
            }
            2 => {
                // create_branch
                let p = format!("Create branch {br} from main in repo {slug}.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"create_branch\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"branch\":\"", false),
                        (br_sp.as_str(), true),
                        ("\",\"from_branch\":\"main\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
                let obs = format!("Created branch {br} on {slug} at sha 7c3b11a");
                let obs_sp = format!(" {obs}");
                let sum_p = format!("Summarize the verified tool output for `{p}`:\n<tool_response> {obs}</tool_response>");
                target.add_v10_segments(
                    &[],
                    &sum_p,
                    &[("Branch ready:", false), (obs_sp.as_str(), true)],
                    bkt,
                    1,
                );
            }
            3 => {
                // commit_push
                let msg = format!("update {rname} module");
                let msg_sp = format!(" {msg}");
                let p = format!("Commit and push workspace changes to branch {br} on repo {slug} with message {msg}.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"commit_push\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"branch\":\"", false),
                        (br_sp.as_str(), true),
                        ("\",\"message\":\"", false),
                        (msg_sp.as_str(), true),
                        ("\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
            }
            4 => {
                // create_pr (single-turn AND multi-turn after create_branch!)
                let title = format!("Update {rname} implementation");
                let title_sp = format!(" {title}");
                let p = format!("Open a PR on repo {slug} from {br} into main titled {title}.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"create_pr\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"head\":\"", false),
                        (br_sp.as_str(), true),
                        ("\",\"base\":\"main\",\"title\":\"", false),
                        (title_sp.as_str(), true),
                        ("\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
                // Multi-turn follow-up: Turn 1 discussed repo + branch -> Turn 2 opens PR referencing them!
                let prior = [(
                    format!("Create branch {br} from main in repo {slug}."),
                    format!("Created branch {br} on {slug} from main."),
                )];
                let mt_p =
                    format!("Now open a PR on {slug} from {br} into main with title {title}.");
                target.add_v10_segments_owned(
                    &prior,
                    &mt_p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"create_pr\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"head\":\"", false),
                        (br_sp.as_str(), true),
                        ("\",\"base\":\"main\",\"title\":\"", false),
                        (title_sp.as_str(), true),
                        ("\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost.min(3),
                );
            }
            5 => {
                // pr_status
                let p = format!("Check PR status for pull request {pr_num} on repo {slug}.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"pr_status\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"pr_number\":", false),
                        (pr_sp.as_str(), true),
                        ("}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
                let obs =
                    format!("PR #{pr_num} on {slug}: open, mergeable=true, 2/2 checks passing");
                let obs_sp = format!(" {obs}");
                let sum_p = format!("Summarize the verified tool output for `{p}`:\n<tool_response> {obs}</tool_response>");
                target.add_v10_segments(
                    &[],
                    &sum_p,
                    &[("Verified PR status:", false), (obs_sp.as_str(), true)],
                    bkt,
                    1,
                );
            }
            6 => {
                // merge_pr
                let p = format!("Squash merge PR {pr_num} on repo {slug}.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"merge_pr\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"pr_number\":", false),
                        (pr_sp.as_str(), true),
                        (",\"merge_method\":\"squash\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
            }
            _ => {
                // watch_deploy
                let p = format!("Watch deployment status for {slug} on gh-pages.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("<tool_call>{\"name\":\"git\",\"arguments\":{\"action\":\"watch_deploy\",\"repo\":\"", false),
                        (slug_sp.as_str(), true),
                        ("\",\"branch\":\"gh-pages\"}}</tool_call>", false),
                    ],
                    bkt,
                    boost,
                );
                let obs = format!("Deployment active for {slug} gh-pages at sha d4e1f88");
                let obs_sp = format!(" {obs}");
                let sum_p = format!("Summarize the verified tool output for `{p}`:\n<tool_response> {obs}</tool_response>");
                target.add_v10_segments(
                    &[],
                    &sum_p,
                    &[
                        ("Verified deployment status:", false),
                        (obs_sp.as_str(), true),
                    ],
                    bkt,
                    1,
                );
            }
        }
    }

    // 4. Combinatorial unseen-identifier Rust code copy training (`F_code`)
    const COMB_VERBS: &[&str] = &["combining", "adding", "summing", "merging", "pairing"];
    const OPS: &[(&str, &str)] = &[("+", " +"), ("-", " -"), ("*", " *")];
    for c_i in 0..4800usize {
        let a = gen_comb_var(rng);
        let mut b = gen_comb_var(rng);
        if b == a {
            b = format!("{b}_alt");
        }
        let va = 5 + (rng.next_u64() % 180);
        let vb = 3 + (rng.next_u64() % 140);
        let verb = COMB_VERBS[c_i % COMB_VERBS.len()];
        let (op_ch, op_sp) = OPS[c_i % OPS.len()];
        let is_eval = c_i >= 4600;
        let target = if is_eval { &mut *eval_sb } else { &mut *sb };
        let boost = if is_eval { 1 } else { 3 };
        let sa = format!(" {a}");
        let sb_var = format!(" {b}");
        match c_i % 3 {
            0 => {
                let p = format!(
                    "Given {a} = {va} and {b} = {vb}, write a concise Rust expression {verb} {a} and {b}."
                );
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("```rust\nlet result =", false),
                        (sa.as_str(), true),
                        (" +", false),
                        (sb_var.as_str(), true),
                        (";\n```", false),
                    ],
                    "F_code",
                    boost,
                );
            }
            1 => {
                let p = format!("Given {a} = {va} and {b} = {vb}, compute {a} {op_ch} {b} in Rust.");
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("```rust\nlet result =", false),
                        (sa.as_str(), true),
                        (op_sp, false),
                        (sb_var.as_str(), true),
                        (";\n```", false),
                    ],
                    "F_code",
                    boost,
                );
            }
            _ => {
                let p = format!(
                    "Write a concise Rust expression combining {a} and {b} where {a} = {va} and {b} = {vb}."
                );
                target.add_v10_segments(
                    &[],
                    &p,
                    &[
                        ("```rust\nlet result =", false),
                        (sa.as_str(), true),
                        (" +", false),
                        (sb_var.as_str(), true),
                        (";\n```", false),
                    ],
                    "F_code",
                    boost,
                );
            }
        }
    }

    // 5. Combinatorial MVM `echo -n {w} | sha256sum` copy generalization (`F_mvm`)
    const MVM_SINGLE_WORDS: &[&str] = &[
        "world", "alpha", "delta", "omega", "theta", "prime", "orbit", "pulse", "amber", "river",
        "cloud", "stone", "flame", "spark", "steel", "cedar", "maple", "lunar", "solar", "comet",
        "nexus", "vertex", "signal", "stream", "buffer", "packet", "cipher", "matrix", "vector",
        "beacon", "bridge", "engine",
    ];
    for m_i in 0..960usize {
        let w = if m_i % 4 == 0 {
            MVM_SINGLE_WORDS[(m_i / 4) % MVM_SINGLE_WORDS.len()].to_string()
        } else if m_i % 2 == 0 {
            gen_comb_word(rng)
        } else {
            gen_comb_repo(rng)
        };
        if w.eq_ignore_ascii_case("hello") || contains_any_forbidden(&w) {
            continue;
        }
        let is_eval = m_i >= 900;
        let target = if is_eval { &mut *eval_sb } else { &mut *sb };
        let bkt = if is_eval || m_i < 640 {
            "F_mvm"
        } else {
            "F_mvm_ext"
        };
        let boost = if is_eval { 1 } else { 3 };
        let cmd_phrase = if m_i % 3 == 2 {
            format!("uname -a && echo -n {w} | sha256sum")
        } else {
            format!("uname -a and compute echo -n {w} | sha256sum")
        };
        let u_prompt = format!("Run {cmd_phrase} inside the Linux MicroVM sandbox.");
        let span_arg = format!(" {w}");
        target.add_v10_segments(
            &[],
            &u_prompt,
            &[
                (
                    "<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"uname -a && echo -n",
                    false,
                ),
                (span_arg.as_str(), true),
                (" | sha256sum\"}}</tool_call>", false),
            ],
            bkt,
            boost,
        );
        if m_i % 3 == 0 {
            let dig = sha256_hex(w.as_bytes());
            let obs_tail = format!(
                " mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {dig}  -"
            );
            let sum_p = format!(
                "Summarize the verified tool output for `{u_prompt}`:\n<tool_response> Linux{obs_tail}</tool_response>"
            );
            target.add_v10_segments(
                &[],
                &sum_p,
                &[
                    ("Verified sandbox output: Linux", false),
                    (obs_tail.as_str(), true),
                    (".", false),
                ],
                bkt,
                1,
            );
        }
    }

    // 6. Concept Q&A balancing (`F_qa`) so "search"/lookup CS topics answer directly without web_search.
    const F_QA_TOPICS: &[(&str, &str)] = &[
        (
            "binary search",
            "Binary search locates a target value in a sorted array by repeatedly comparing the midpoint against the target and discarding half of the remaining range on each comparison, achieving O(log n) worst-case time complexity and O(1) auxiliary space when implemented iteratively.",
        ),
        (
            "linear search",
            "Linear search inspects elements sequentially from the start of a collection until it finds a match or reaches the end, requiring no prior sorting and running in O(n) worst-case time with O(1) extra space.",
        ),
        (
            "breadth-first search",
            "Breadth-first search (BFS) explores a graph or tree level by level from a starting node using a FIFO queue, guaranteeing the shortest path in terms of edge count for unweighted graphs in O(V + E) time.",
        ),
        (
            "depth-first search",
            "Depth-first search (DFS) explores as far as possible along each branch before backtracking via recursion or an explicit stack, running in O(V + E) time for topological sorting, cycle detection, and connected components.",
        ),
        (
            "a binary search tree",
            "A binary search tree (BST) stores comparable keys so every left subtree holds smaller keys and every right subtree holds larger keys, yielding O(h) lookup, insertion, and deletion where balanced trees keep height h = O(log n).",
        ),
        (
            "a hash table lookup",
            "A hash table lookup computes an index from a key via a hash function and resolves bucket collisions by chaining or open addressing, delivering expected O(1) average lookup, insertion, and deletion time.",
        ),
        (
            "merge sort",
            "Merge sort divides an array recursively into halves, sorts each half, and merges the sorted subarrays in linear pass, providing stable O(n log n) worst-case time complexity at the cost of O(n) auxiliary space.",
        ),
        (
            "quicksort",
            "Quicksort partitions an array around a pivot element so smaller items move left and larger items move right, achieving O(n log n) average-case time in place with O(log n) average stack space and O(n^2) worst case.",
        ),
        (
            "dynamic programming",
            "Dynamic programming solves optimization problems with overlapping subproblems and optimal substructure by caching subproblem results bottom-up in a table or top-down via memoization to avoid exponential recomputation.",
        ),
        (
            "Dijkstra's shortest-path algorithm",
            "Dijkstra's algorithm computes shortest paths from a single source in graphs with non-negative edge weights by greedily extracting the closest unvisited vertex from a min-priority queue in O((V + E) log V) time.",
        ),
        (
            "Big-O time complexity",
            "Big-O notation characterizes the asymptotic upper bound on how an algorithm's runtime grows relative to input size n while dropping constant factors and lower-order terms.",
        ),
        (
            "a priority queue heap",
            "A binary heap implements a priority queue as a nearly complete tree stored in a contiguous array where each parent satisfies the min-heap or max-heap ordering invariant, supporting O(log n) push and pop operations.",
        ),
    ];
    for &(topic, answer) in F_QA_TOPICS {
        let phrasings = [
            format!("What is {topic}?"),
            format!("Explain how {topic} works."),
            format!("Describe {topic} in computer science."),
        ];
        for q in &phrasings {
            if !contains_any_forbidden(q) {
                sb.add_v10_segments(&[], q, &[(answer, false)], "F_mt", 4);
            }
        }
    }
}

fn write_uorcorp3(path: &Path, records: &[CorpusRecord]) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let file = File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut f = BufWriter::new(file);
    f.write_all(b"UORCORP3")
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    f.write_all(&(records.len() as u32).to_le_bytes())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    f.write_all(&(MAX_SEQ as u32).to_le_bytes())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    for rec in records {
        let plen = rec.prompt_ids.len();
        let rlen = rec.reply_ids.len();
        assert!(plen + rlen <= MAX_SEQ && plen <= 768 && rlen <= 256);
        assert_eq!(rec.copy_flags.len(), rlen);
        f.write_all(&(plen as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(&(rlen as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        for &tid in &rec.prompt_ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write: {e}"))?;
        }
        for &tid in &rec.reply_ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write: {e}"))?;
        }
        f.write_all(&rec.copy_flags)
            .map_err(|e| format!("write: {e}"))?;
    }
    f.flush()
        .map_err(|e| format!("flush {}: {e}", path.display()))
}

fn write_eval_prompts(path_json: &Path, path_bin: &Path, tok: &ByteBpe4096) -> Result<(), String> {
    let prompts: Vec<(&'static str, &'static str, String)> = vec![
        ("chat_hi", "greeting", "<|user|>\nhi\n<|assistant|>\n".to_string()),
        ("chat_how", "greeting", "<|user|>\nhow are you?\n<|assistant|>\n".to_string()),
        ("meta_who", "capability", "<|user|>\nwho are you?\n<|assistant|>\n".to_string()),
        (
            "mt_4turn_nebula",
            "multiturn_kv",
            format!(
                "<|user|>\nhi\n<|assistant|>\n{REPLY_HI}<|end|>\n\
                 <|user|>\nhow are you?\n<|assistant|>\n{REPLY_HOW_ARE_YOU}<|end|>\n\
                 <|user|>\nwho are you?\n<|assistant|>\n{REPLY_WHO}<|end|>\n\
                 <|user|>\nmy repo is called aurora-mesh\n<|assistant|>\nTracking aurora-mesh.<|end|>\n\
                 <|user|>\nWhich repository am I working on right now?\n<|assistant|>\n"
            ),
        ),
        (
            "code_unseen_rust",
            "code",
            "<|user|>\nGiven sensor_drift_k9 = 41 and base_offset_q3 = 18, write a concise Rust expression combining sensor_drift_k9 and base_offset_q3.\n<|assistant|>\n".to_string(),
        ),
        (
            "git_create_branch",
            "github_tool",
            "<|user|>\nCreate branch feat/rope-kv-v11 from main in repo afflom/stitch-demo.\n<|assistant|>\n".to_string(),
        ),
        (
            "git_create_pr",
            "github_tool",
            "<|user|>\nOpen a PR on repo afflom/stitch-demo from feat/rope-kv-v11 into main titled Upgrade GLLM v11.\n<|assistant|>\n".to_string(),
        ),
        (
            "git_pr_status",
            "github_tool",
            "<|user|>\nCheck PR status for pull request 42 on repo afflom/stitch-demo.\n<|assistant|>\n".to_string(),
        ),
        (
            "git_watch_deploy",
            "github_tool",
            "<|user|>\nWatch deployment status for afflom/stitch-demo on gh-pages.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_mvm_uname",
            "mvm_tool",
            "<|user|>\nRun uname -a inside the MVM-Wasm Linux sandbox and report the architecture.\n<|assistant|>\n".to_string(),
        ),
    ];

    ensure_parent_dir(path_json)?;
    let mut json_buf = String::from("[\n");
    for (i, (id, kind, pr)) in prompts.iter().enumerate() {
        let comma = if i + 1 == prompts.len() { "" } else { "," };
        json_buf.push_str(&format!(
            "  {{\"id\": {}, \"kind\": {}, \"prompt\": {}}}{comma}\n",
            json_escape(id),
            json_escape(kind),
            json_escape(pr),
        ));
    }
    json_buf.push_str("]\n");
    fs::write(path_json, json_buf.as_bytes())
        .map_err(|e| format!("write {}: {e}", path_json.display()))?;

    let bf = File::create(path_bin).map_err(|e| format!("create {}: {e}", path_bin.display()))?;
    let mut f = BufWriter::new(bf);
    f.write_all(b"UORPRMP1")
        .map_err(|e| format!("write: {e}"))?;
    f.write_all(&(prompts.len() as u32).to_le_bytes())
        .map_err(|e| format!("write: {e}"))?;
    for (id, kind, pr) in &prompts {
        let mut ids = vec![0u16];
        ids.extend(tok.encode(pr));
        f.write_all(&(id.len() as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(&(kind.len() as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(&(pr.len() as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(id.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(kind.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(pr.as_bytes())
            .map_err(|e| format!("write: {e}"))?;
        f.write_all(&(ids.len() as u16).to_le_bytes())
            .map_err(|e| format!("write: {e}"))?;
        for tid in ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write: {e}"))?;
        }
    }
    f.flush()
        .map_err(|e| format!("flush {}: {e}", path_bin.display()))
}

fn discover_paths() -> Result<(PathBuf, PathBuf), String> {
    if let Some(env_tok) = env::var_os("UOR_TOKENIZER_JSON") {
        let p = PathBuf::from(env_tok);
        if p.is_file() {
            return Ok((p.clone(), parent_or_dot(&p).to_path_buf()));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        for anc in cwd.ancestors() {
            let c1 = anc.join("crates/wasm/src/tokenizer.json");
            if c1.is_file() {
                return Ok((c1, anc.join("gpu")));
            }
            let c2 = anc.join("tokenizer.json");
            if c2.is_file() {
                return Ok((c2, anc.to_path_buf()));
            }
        }
    }
    Err("could not locate tokenizer.json".to_string())
}

fn run() -> Result<(), String> {
    let (tok_path, gpu_dir) = discover_paths()?;
    let mut args = env::args_os();
    let _prog = args.next();
    let out_corpus = match args.next() {
        Some(arg) => PathBuf::from(arg),
        None => gpu_dir.join("v11_corpus.bin"),
    };
    let out_dir = parent_or_dot(&out_corpus);
    let out_boot_corpus = out_dir.join("v11_boot_corpus.bin");
    let out_eval = out_dir.join("v11_eval.bin");
    let out_boot_eval = out_dir.join("v11_boot_eval.bin");
    let out_vocab = out_dir.join("v11_vocab_bytes.bin");
    let out_prompts = out_dir.join("v11_eval_prompts.json");
    let out_prompts_bin = out_dir.join("v11_eval_prompts.bin");
    let cache_dir = match env::var_os("UOR_HF_CACHE") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => out_dir.join(".hf_cache_v11"),
    };

    export_vocab_raw_bytes(&tok_path, &out_vocab)?;
    let tok = ByteBpe4096::from_file(&tok_path)?;
    let mut rng = Rng64::new(20261114);
    let smoltalk = load_smoltalk_dialogues(&cache_dir);
    let (open_qa, open_code) = load_dolly_and_alpaca(&cache_dir);

    let mut sb = SampleBuilder::new(&tok);
    let mut eval_sb = SampleBuilder::new(&tok);

    generate_bucket_a_conversations(&mut sb, &mut rng, &smoltalk, &mut eval_sb);
    generate_bucket_b_qa(&mut sb, &mut rng, &open_qa, &mut eval_sb);
    generate_bucket_c_code(&mut sb, &mut rng, &open_code, &mut eval_sb);
    generate_bucket_d_mvm_and_git_tools(&mut sb, &mut eval_sb);
    generate_bucket_e_web_tools(&mut sb, &mut eval_sb);
    generate_bucket_f_v11_multiturn_and_github(&mut sb, &mut eval_sb, &mut rng, &smoltalk);

    // Keep pre-topup balanced (~88k max) so `top_up_to_target` adds balanced recall/code coverage
    if sb.records.len() > 88_000 {
        let mut keep_f: Vec<CorpusRecord> = Vec::new();
        let mut rest: Vec<CorpusRecord> = Vec::new();
        for rec in sb.records.drain(..) {
            if rec.bucket.starts_with("F_") {
                keep_f.push(rec);
            } else {
                rest.push(rec);
            }
        }
        rng.shuffle(&mut rest);
        let budget_rest = 88_000usize.saturating_sub(keep_f.len());
        rest.truncate(budget_rest);
        sb.records = rest;
        sb.records.extend(keep_f);
    }

    top_up_to_target(&mut sb, &mut rng);

    // Filter out any held-out v11 tokens and enforce rolling prompt/reply budgets (`plen <= 768`, `rlen <= 256`) AFTER top-up
    let mut leaked = 0usize;
    sb.records.retain(|rec| {
        if rec.prompt_ids.len() > 768
            || rec.reply_ids.len() > 256
            || rec.prompt_ids.len() + rec.reply_ids.len() > MAX_SEQ
        {
            return false;
        }
        let p_str = tok.decode(&rec.prompt_ids[1..]);
        let r_str = tok.decode(&rec.reply_ids[..rec.reply_ids.len().saturating_sub(1)]);
        if contains_any_forbidden(&p_str) || contains_any_forbidden(&r_str) {
            leaked += 1;
            return false;
        }
        true
    });
    if leaked > 0 {
        println!("[corpus-v11] filtered {leaked} held-out candidate sequences cleanly");
    }

    // Prompt disjointness between train and eval
    eval_sb.records.retain(|rec| {
        if rec.prompt_ids.len() > 768
            || rec.reply_ids.len() > 256
            || rec.prompt_ids.len() + rec.reply_ids.len() > MAX_SEQ
        {
            return false;
        }
        let p_str = tok.decode(&rec.prompt_ids[1..]);
        !sb.seen_keys.contains(&norm_key(&p_str))
    });

    // Build crisp boot subset preserving strong F_mt + F_git + F_mvm + D + E + core A/C SNR (~32,000 sequences)
    let mut boot_records: Vec<CorpusRecord> = Vec::with_capacity(36_000);
    let mut seen_boot: HashMap<(&[u16], &[u16]), usize> = HashMap::new();
    for rec in &sb.records {
        let non_f_skip = !rec.bucket.starts_with("F_")
            && (rec.bucket.ends_with("_open")
                || rec.bucket.ends_with("_top")
                || (rec.bucket.ends_with("_ext")
                    && rec.bucket != "A_ext"
                    && rec.bucket != "D_ext"));
        if non_f_skip {
            continue;
        }
        let sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
        let has_copy = rec.copy_flags.iter().any(|&f| f != 0);
        let cap = if rec.bucket.starts_with("F_git")
            || rec.bucket.starts_with("F_mt")
            || rec.bucket.starts_with("F_mvm")
            || rec.bucket == "F_code"
        {
            6
        } else if rec.bucket == "A_ext" {
            4
        } else if rec.bucket == "D_ext" {
            2
        } else if rec.bucket == "D" || rec.bucket == "E" {
            if has_copy { 6 } else { 48 }
        } else if rec.bucket == "C" {
            if has_copy { 2 } else { 24 }
        } else {
            24
        };
        let used = seen_boot.entry(sig).or_insert(0);
        if *used < cap {
            *used += 1;
            boot_records.push(rec.clone());
        }
    }
    drop(seen_boot);

    if sb.records.len() > TARGET_TRAIN_COUNT + 15_000 {
        rng.shuffle(&mut sb.records);
        sb.records.truncate(TARGET_TRAIN_COUNT + 15_000);
    }
    rng.shuffle(&mut boot_records);
    rng.shuffle(&mut sb.records);
    rng.shuffle(&mut eval_sb.records);
    if eval_sb.records.len() > TARGET_EVAL_COUNT {
        eval_sb.records.truncate(TARGET_EVAL_COUNT);
    }
    let boot_eval_records: Vec<CorpusRecord> = eval_sb
        .records
        .iter()
        .filter(|rec| !rec.bucket.ends_with("_ext"))
        .cloned()
        .collect();

    write_uorcorp3(&out_corpus, &sb.records)?;
    write_uorcorp3(&out_boot_corpus, &boot_records)?;
    write_uorcorp3(&out_eval, &eval_sb.records)?;
    write_uorcorp3(&out_boot_eval, &boot_eval_records)?;
    write_eval_prompts(&out_prompts, &out_prompts_bin, &tok)?;

    let mut b_counts: BTreeMap<char, usize> = BTreeMap::new();
    for rec in &sb.records {
        *b_counts
            .entry(rec.bucket.chars().next().unwrap_or('?'))
            .or_insert(0) += 1;
    }
    println!(
        "[corpus-v11] wrote full={}, boot={}, eval={}, boot_eval={}, buckets={b_counts:?}",
        sb.records.len(),
        boot_records.len(),
        eval_sb.records.len(),
        boot_eval_records.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[corpus-v11] error: {err}");
            ExitCode::FAILURE
        }
    }
}
