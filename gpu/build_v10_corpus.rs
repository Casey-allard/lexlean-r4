//! Pure-Rust offline corpus generator for UOR-R4-GLLM v10 (`UORCORP3` / `UORPRMP1` / `UORVOCB9`),
//! including dataset loaders, held-out disjointness audit, and CLI entrypoint (`main`).
//!
//! Compile standalone with:
//!   rustc --edition 2021 -O gpu/build_v10_corpus.rs -o /tmp/build_v10_corpus

mod bpe_json;
mod v10_corpus_ab;
mod v10_corpus_cde;

use std::collections::{BTreeMap, HashMap, HashSet};
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
    REPLY_HOW_ARE_YOU, REPLY_WHO, TARGET_EVAL_COUNT,
};
use v10_corpus_cde::{
    generate_bucket_c_code, generate_bucket_d_mvm_and_git_tools, generate_bucket_e_web_tools,
    top_up_to_target,
};

const SMOLTALK_ROWS_URL: &str = "https://datasets-server.huggingface.co/rows?dataset=HuggingFaceTB%2Fsmoltalk&config=everyday-conversations&split=train";

fn parent_or_dot(path: &Path) -> &Path {
    match path.parent() {
        Some(par) if !par.as_os_str().is_empty() => par,
        _ => Path::new("."),
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(par) = path.parent() {
        if !par.as_os_str().is_empty() {
            fs::create_dir_all(par).map_err(|e| format!("mkdir {}: {e}", par.display()))?;
        }
    }
    Ok(())
}

fn extract_smoltalk_pairs(msgs: &[JsonVal]) -> Option<Vec<(String, String)>> {
    if msgs.len() < 2 {
        return None;
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut i = 0usize;
    while i + 1 < msgs.len() && pairs.len() < 3 {
        if msgs[i].as_object().is_none() || msgs[i + 1].as_object().is_none() {
            break;
        }
        let r0 = msgs[i]
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let r1 = msgs[i + 1]
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let c0 = clean_text(
            msgs[i]
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
        let c1 = first_crisp_sentence(
            msgs[i + 1]
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            105,
        );
        let c0_chars = c0.chars().count();
        if r0 != "user"
            || r1 != "assistant"
            || c0.is_empty()
            || c1.is_empty()
            || c0_chars < 2
            || c0_chars > 95
            || c0.contains('\n')
        {
            break;
        }
        let low_both = format!("{c0} {c1}").to_lowercase();
        if BANNED_BOILERPLATE.iter().any(|&b| low_both.contains(b)) || contains_forbidden(&low_both)
        {
            break;
        }
        if !c0.is_ascii() || !c1.is_ascii() {
            break;
        }
        pairs.push((c0, c1));
        i += 2;
    }
    if pairs.is_empty() {
        None
    } else {
        Some(pairs)
    }
}

fn load_smoltalk_dialogues(cache_dir: &Path) -> Vec<Vec<(String, String)>> {
    let mut dialogues: Vec<Vec<(String, String)>> = Vec::new();
    let cache_path = cache_dir.join("smoltalk_everyday_rows.jsonl");

    if cache_path.exists()
        && fs::metadata(&cache_path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        match fs::read(&cache_path) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let Ok(val) = JsonVal::parse(line) else {
                        continue;
                    };
                    if let Some(msgs) = val.get("messages").and_then(|v| v.as_array()) {
                        if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                            dialogues.push(pairs);
                        }
                    } else if let Some(msgs) = val
                        .get("row")
                        .and_then(|r| r.get("messages"))
                        .and_then(|m| m.as_array())
                    {
                        if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                            dialogues.push(pairs);
                        }
                    } else if let Some(rows) = val.get("rows").and_then(|v| v.as_array()) {
                        for item in rows {
                            if let Some(msgs) = item
                                .get("row")
                                .and_then(|r| r.get("messages"))
                                .and_then(|m| m.as_array())
                            {
                                if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                                    dialogues.push(pairs);
                                }
                            }
                        }
                    } else if let Some(msgs) = val.as_array() {
                        if let Some(pairs) = extract_smoltalk_pairs(msgs) {
                            dialogues.push(pairs);
                        }
                    }
                }
                println!(
                    "[corpus] loaded {} crisp dialogues from SmolTalk everyday-conversations",
                    dialogues.len()
                );
                return dialogues;
            }
            Err(exc) => {
                eprintln!("[corpus] smoltalk cache read note: {exc}");
            }
        }
    }

    let _ = fs::create_dir_all(cache_dir);
    let mut cached_lines: Vec<String> = Vec::with_capacity(2260);
    let mut failed_pages = 0usize;
    let mut consecutive_failures = 0usize;

    for offset in (0..=2200usize).step_by(100) {
        let url = format!("{SMOLTALK_ROWS_URL}&offset={offset}&length=100");
        match curl_fetch_stdout(&url) {
            Ok(body) => {
                consecutive_failures = 0;
                match JsonVal::parse(&body) {
                    Ok(root) => {
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
                                        let content = msg
                                            .get("content")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
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
                    Err(exc) => {
                        failed_pages += 1;
                        eprintln!(
                            "[corpus] note: optional smoltalk page offset={offset} parse skipped ({exc})"
                        );
                    }
                }
            }
            Err(exc) => {
                failed_pages += 1;
                consecutive_failures += 1;
                eprintln!(
                    "[corpus] note: optional download smoltalk offset={offset} skipped ({exc})"
                );
                if consecutive_failures >= 3 && cached_lines.is_empty() {
                    break;
                }
            }
        }
    }

    if !cached_lines.is_empty() && failed_pages == 0 {
        let tmp_path = cache_path.with_extension("jsonl.part");
        if let Ok(file) = File::create(&tmp_path) {
            let mut w = BufWriter::new(file);
            let mut ok = true;
            for line in &cached_lines {
                if writeln!(w, "{line}").is_err() {
                    ok = false;
                    break;
                }
            }
            if ok && w.flush().is_ok() {
                let _ = fs::rename(&tmp_path, &cache_path);
            } else {
                let _ = fs::remove_file(&tmp_path);
            }
        }
    }

    println!(
        "[corpus] loaded {} crisp dialogues from SmolTalk everyday-conversations",
        dialogues.len()
    );
    dialogues
}

fn load_dolly_and_alpaca(cache_dir: &Path) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let mut qa_pairs: Vec<(String, String)> = Vec::new();
    let mut code_pairs: Vec<(String, String)> = Vec::new();

    let dolly_path = cache_dir.join("dolly_15k.jsonl");
    if matches!(
        curl_download_cached(DOLLY_URL, &dolly_path, "databricks-dolly-15k"),
        Ok(true)
    ) {
        if let Ok(bytes) = fs::read(&dolly_path) {
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let Ok(row) = JsonVal::parse(line) else {
                    continue;
                };
                let cat = row.get("category").and_then(|v| v.as_str()).unwrap_or("");
                let ctx = clean_text(row.get("context").and_then(|v| v.as_str()).unwrap_or(""));
                let inst = clean_text(
                    row.get("instruction")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                );
                let resp = first_crisp_sentence(
                    row.get("response").and_then(|v| v.as_str()).unwrap_or(""),
                    95,
                );
                if !ctx.is_empty() || resp.is_empty() || inst.contains('\n') {
                    continue;
                }
                let inst_len = inst.chars().count();
                let low_both = format!("{inst} {resp}").to_lowercase();
                if inst_len < 14
                    || inst_len > 85
                    || !inst.is_ascii()
                    || !resp.is_ascii()
                    || BANNED_BOILERPLATE.iter().any(|&b| low_both.contains(b))
                    || contains_forbidden(&low_both)
                {
                    continue;
                }
                if matches!(cat, "open_qa" | "general_qa" | "classification") {
                    qa_pairs.push((inst, resp));
                }
            }
        }
    }

    let alpaca_path = cache_dir.join("alpaca_cleaned.json");
    if matches!(
        curl_download_cached(ALPACA_URL, &alpaca_path, "alpaca-cleaned"),
        Ok(true)
    ) {
        if let Ok(bytes) = fs::read(&alpaca_path) {
            let text = String::from_utf8_lossy(&bytes);
            if let Ok(root) = JsonVal::parse(&text) {
                if let Some(rows) = root.as_array() {
                    for row in rows {
                        let inst = clean_text(
                            row.get("instruction")
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                        );
                        let inp =
                            clean_text(row.get("input").and_then(|v| v.as_str()).unwrap_or(""));
                        let out = first_crisp_sentence(
                            row.get("output").and_then(|v| v.as_str()).unwrap_or(""),
                            95,
                        );
                        if !inp.is_empty() || out.is_empty() || inst.contains('\n') {
                            continue;
                        }
                        let inst_len = inst.chars().count();
                        let low_both = format!("{inst} {out}").to_lowercase();
                        if inst_len < 14
                            || inst_len > 85
                            || !inst.is_ascii()
                            || !out.is_ascii()
                            || BANNED_BOILERPLATE.iter().any(|&b| low_both.contains(b))
                            || contains_forbidden(&low_both)
                        {
                            continue;
                        }
                        qa_pairs.push((inst, out));
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
        if let Ok(bytes) = fs::read(&code_path) {
            let text = String::from_utf8_lossy(&bytes);
            if let Ok(root) = JsonVal::parse(&text) {
                if let Some(rows) = root.as_array() {
                    for row in rows {
                        let inst = clean_text(
                            row.get("instruction")
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                        );
                        let inp =
                            clean_text(row.get("input").and_then(|v| v.as_str()).unwrap_or(""));
                        let mut out =
                            clean_text(row.get("output").and_then(|v| v.as_str()).unwrap_or(""));
                        if !inp.is_empty() || inst.contains('\n') {
                            continue;
                        }
                        let inst_len = inst.chars().count();
                        let out_len = out.chars().count();
                        let nl_count = out.bytes().filter(|&b| b == b'\n').count();
                        let low_both = format!("{inst} {out}").to_lowercase();
                        if inst_len < 14
                            || inst_len > 105
                            || out_len < 10
                            || out_len > 140
                            || nl_count > 6
                            || !inst.is_ascii()
                            || !out.is_ascii()
                            || BANNED_BOILERPLATE.iter().any(|&b| low_both.contains(b))
                            || contains_forbidden(&low_both)
                        {
                            continue;
                        }
                        if out.starts_with("```") {
                            out = format!("Here is a concise code snippet:\n{out}");
                        } else {
                            let low_p = inst.to_lowercase();
                            let lang = if low_p.contains("rust") {
                                "rust"
                            } else if low_p.contains("python") {
                                "python"
                            } else if low_p.contains("javascript") || low_p.contains("js") {
                                "javascript"
                            } else if low_p.contains(" in c") || low_p.contains("c program") {
                                "c"
                            } else if low_p.contains("bash") || low_p.contains("shell") {
                                "sh"
                            } else {
                                "python"
                            };
                            out =
                                format!("Here is a concise {lang} snippet:\n```{lang}\n{out}\n```");
                        }
                        code_pairs.push((inst, out));
                    }
                }
            }
        }
    }

    println!(
        "[corpus] loaded {} crisp open Q&A pairs and {} crisp open code pairs",
        qa_pairs.len(),
        code_pairs.len()
    );
    (qa_pairs, code_pairs)
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
        assert!(
            plen + rlen <= MAX_SEQ,
            "sequence length {} exceeds MAX_SEQ {}",
            plen + rlen,
            MAX_SEQ
        );
        assert_eq!(
            rec.copy_flags.len(),
            rlen,
            "copy_flags len {} != rlen {}",
            rec.copy_flags.len(),
            rlen
        );
        f.write_all(&(plen as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        f.write_all(&(rlen as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        for &tid in &rec.prompt_ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write {}: {e}", path.display()))?;
        }
        for &tid in &rec.reply_ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write {}: {e}", path.display()))?;
        }
        f.write_all(&rec.copy_flags)
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    f.flush()
        .map_err(|e| format!("flush {}: {e}", path.display()))
}

struct HeldOutEvalPrompt {
    id: &'static str,
    kind: &'static str,
    prompt: String,
    prompt_ids: Vec<u16>,
}

/// 22 strictly held-out short chat, multi-turn recall, code, and tool prompts for receipt & gate audit.
fn write_held_out_eval_prompts(
    path_json: &Path,
    path_bin: &Path,
    tok: &ByteBpe4096,
) -> Result<(), String> {
    let gllm_sha = sha256_hex(b"gllm-v9-mvm");
    let specs: Vec<(&'static str, &'static str, String)> = vec![
        ("chat_hi", "greeting", "<|user|>\nhi\n<|assistant|>\n".to_string()),
        ("chat_hello", "greeting", "<|user|>\nhello\n<|assistant|>\n".to_string()),
        (
            "chat_hows_it_going",
            "greeting",
            "<|user|>\nhey, how's it going?\n<|assistant|>\n".to_string(),
        ),
        (
            "chat_good_morning",
            "greeting",
            "<|user|>\ngood morning!\n<|assistant|>\n".to_string(),
        ),
        (
            "chat_thanks",
            "social",
            "<|user|>\nthanks!\n<|assistant|>\n".to_string(),
        ),
        (
            "meta_who_are_you",
            "capability",
            "<|user|>\nwho are you?\n<|assistant|>\n".to_string(),
        ),
        (
            "meta_what_can_you_do",
            "capability",
            "<|user|>\nwhat can you do?\n<|assistant|>\n".to_string(),
        ),
        (
            "meta_who_made_you",
            "capability",
            "<|user|>\nwho made you?\n<|assistant|>\n".to_string(),
        ),
        (
            "meta_browser",
            "capability",
            "<|user|>\nare you running in my browser?\n<|assistant|>\n".to_string(),
        ),
        (
            "multiturn_explain_simpler",
            "multiturn_followup",
            "<|user|>\nExplain HTTP status codes briefly.\n<|assistant|>\n2xx codes signal success, 3xx redirect the client, 4xx report a client request error, and 5xx indicate an upstream server failure.<|end|>\n<|user|>\ncan you explain that simpler?\n<|assistant|>\n".to_string(),
        ),
        (
            "multiturn_nebula_repo",
            "multiturn_kv_recall",
            format!(
                "<|user|>\nhi\n<|assistant|>\n{REPLY_HI}<|end|>\n\
                 <|user|>\nhow are you?\n<|assistant|>\n{REPLY_HOW_ARE_YOU}<|end|>\n\
                 <|user|>\nwho are you?\n<|assistant|>\n{REPLY_WHO}<|end|>\n\
                 <|user|>\nmy repo is called nebula-os\n<|assistant|>\nNoted your saved name here for you.<|end|>\n\
                 <|user|>\nwhat is my repo called?\n<|assistant|>\n"
            ),
        ),
        (
            "multiturn_what_did_i_say",
            "multiturn_kv_recall",
            "<|user|>\nI am auditing the zero-copy RingBuffer bounds.\n<|assistant|>\nGot it - ready to help with your question.<|end|>\n<|user|>\nwhat did I just say?\n<|assistant|>\n".to_string(),
        ),
        (
            "code_unseen_v10",
            "compositional_code",
            "<|user|>\nGiven sensor_drift_k9 = 41 and base_offset_q3 = 18, write a Rust snippet setting `out` to (sensor_drift_k9 + base_offset_q3) + 4.\n<|assistant|>\n".to_string(),
        ),
        (
            "code_unseen_v9_compat",
            "r4_38_compat_code",
            "<|user|>\nGiven orbital_phase_m2 = 27 and lattice_stride_z7 = 14, write a concise Rust expression combining orbital_phase_m2 and lattice_stride_z7.\n<|assistant|>\n".to_string(),
        ),
        (
            "fn_unseen_rust",
            "r4_38_compat_fn",
            "<|user|>\nWrite a Rust function compute_flux_z9( sensor_drift_k9: i64, base_offset_q3: i64 ) -> i64.\n<|assistant|>\n".to_string(),
        ),
        (
            "fn_unseen_c_rv64",
            "r4_38_compat_c",
            "<|user|>\nWrite a freestanding C function step_flux(long gyro_bias_m4) for the RV64IMAC MicroVM.\n<|assistant|>\n".to_string(),
        ),
        (
            "qa_zero_nic",
            "r4_38_compat_qa",
            "<|user|>\nHow does the MVM-Wasm Linux sandbox isolate network access?\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_mvm_uname",
            "tool_mvm",
            "<|user|>\nRun uname -a inside the MVM-Wasm Linux sandbox and report the architecture.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_mvm_sha256",
            "tool_mvm",
            "<|user|>\nRun uname -a and compute echo -n gllm-v9-mvm | sha256sum inside the Linux MicroVM sandbox.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_mvm_pkg_jq",
            "tool_mvm",
            "<|user|>\nInstall jq inside the Linux MicroVM.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_git_status",
            "tool_git",
            "<|user|>\nCheck `git status` in `/workspace`.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_git_clone",
            "tool_git",
            "<|user|>\nClone https://github.com/Casey-allard/lexlean-r4.git into `/workspace`.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_web_search",
            "tool_web",
            "<|user|>\nSearch the web for WebAssembly Component Model WASI 0.2.\n<|assistant|>\n".to_string(),
        ),
        (
            "tool_web_fetch",
            "tool_web",
            "<|user|>\nFetch https://docs.rs/axum/latest/axum and summarize it.\n<|assistant|>\n".to_string(),
        ),
        (
            "r4_38_agent_step1",
            "r4_38_agent",
            "User: [agent_mode] Run uname -a and compute echo -n gllm-v9-mvm | sha256sum inside the Linux MicroVM sandbox.\nAssistant:".to_string(),
        ),
        (
            "r4_38_agent_step2",
            "r4_38_agent",
            format!(
                "User: [agent_mode] Verify sandbox execution.\n<tool_response> Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {gllm_sha}  -</tool_response>\nAssistant:"
            ),
        ),
        (
            "qa_rust_async_pin",
            "cs_systems_qa",
            "<|user|>\nWhy do self-referential futures require Pin in Rust?\n<|assistant|>\n".to_string(),
        ),
        (
            "qa_page_table_tlb",
            "cs_systems_qa",
            "<|user|>\nHow does a TLB accelerate virtual memory address translation?\n<|assistant|>\n".to_string(),
        ),
        (
            "algo_python_quicksort",
            "algorithm_code",
            "<|user|>\nWrite a concise recursive quicksort function in Python.\n<|assistant|>\n".to_string(),
        ),
        (
            "algo_rust_ring_push",
            "unseen_algo_code",
            "<|user|>\nWrite a Rust ring buffer push helper for queue_span_v4 and cap_bound_n2.\n<|assistant|>\n".to_string(),
        ),
    ];

    let bos = tok.token_to_id("<|bos|>").unwrap_or_else(|| tok.bos_id());
    let mut prompts: Vec<HeldOutEvalPrompt> = Vec::with_capacity(specs.len());
    for (id, kind, prompt) in specs {
        let mut prompt_ids = Vec::with_capacity(64);
        prompt_ids.push(bos);
        prompt_ids.extend_from_slice(&tok.encode(&prompt));
        prompts.push(HeldOutEvalPrompt {
            id,
            kind,
            prompt,
            prompt_ids,
        });
    }

    ensure_parent_dir(path_json)?;
    let mut json_buf = String::from("[\n");
    for (i, item) in prompts.iter().enumerate() {
        json_buf.push_str("  {\n");
        json_buf.push_str(&format!("    \"id\": {},\n", json_escape(item.id)));
        json_buf.push_str(&format!("    \"kind\": {},\n", json_escape(item.kind)));
        json_buf.push_str(&format!("    \"prompt\": {},\n", json_escape(&item.prompt)));
        json_buf.push_str("    \"prompt_ids\": [\n");
        for (j, &tid) in item.prompt_ids.iter().enumerate() {
            let comma = if j + 1 < item.prompt_ids.len() {
                ","
            } else {
                ""
            };
            json_buf.push_str(&format!("      {tid}{comma}\n"));
        }
        json_buf.push_str("    ]\n");
        let item_comma = if i + 1 < prompts.len() { "," } else { "" };
        json_buf.push_str(&format!("  }}{item_comma}\n"));
    }
    json_buf.push_str("]\n");
    fs::write(path_json, json_buf.as_bytes())
        .map_err(|e| format!("write {}: {e}", path_json.display()))?;

    ensure_parent_dir(path_bin)?;
    let bin_file =
        File::create(path_bin).map_err(|e| format!("create {}: {e}", path_bin.display()))?;
    let mut f = BufWriter::new(bin_file);
    f.write_all(b"UORPRMP1")
        .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
    f.write_all(&(prompts.len() as u32).to_le_bytes())
        .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
    for item in &prompts {
        let id_b = item.id.as_bytes();
        let kind_b = item.kind.as_bytes();
        let pr_b = item.prompt.as_bytes();
        f.write_all(&(id_b.len() as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(&(kind_b.len() as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(&(pr_b.len() as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(id_b)
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(kind_b)
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(pr_b)
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        f.write_all(&(item.prompt_ids.len() as u16).to_le_bytes())
            .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        for &tid in &item.prompt_ids {
            f.write_all(&tid.to_le_bytes())
                .map_err(|e| format!("write {}: {e}", path_bin.display()))?;
        }
    }
    f.flush()
        .map_err(|e| format!("flush {}: {e}", path_bin.display()))
}

fn discover_paths() -> Result<(PathBuf, PathBuf), String> {
    if let Some(env_tok) = env::var_os("UOR_TOKENIZER_JSON") {
        let p = PathBuf::from(env_tok);
        if p.is_file() {
            let gpu_dir = p
                .ancestors()
                .find(|a| a.join("gpu").is_dir())
                .map(|a| a.join("gpu"))
                .unwrap_or_else(|| parent_or_dot(&p).to_path_buf());
            return Ok((p, gpu_dir));
        }
    }

    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        starts.push(cwd.clone());
        let src_path = Path::new(file!());
        if src_path.is_absolute() {
            if let Some(par) = src_path.parent() {
                starts.push(par.to_path_buf());
            }
        } else if let Some(par) = cwd.join(src_path).parent() {
            starts.push(par.to_path_buf());
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(par) = exe.parent() {
            starts.push(par.to_path_buf());
        }
    }

    for start in &starts {
        for anc in start.ancestors() {
            let candidate = anc.join("crates/wasm/src/tokenizer.json");
            if candidate.is_file() {
                let gpu_dir = anc.join("gpu");
                return Ok((
                    candidate,
                    if gpu_dir.is_dir() {
                        gpu_dir
                    } else {
                        anc.to_path_buf()
                    },
                ));
            }
        }
    }
    for start in &starts {
        for anc in start.ancestors() {
            let candidate = anc.join("tokenizer.json");
            if candidate.is_file() {
                return Ok((candidate, anc.to_path_buf()));
            }
        }
    }
    Err("could not locate crates/wasm/src/tokenizer.json or tokenizer.json".to_string())
}

fn run() -> Result<(), String> {
    let (tok_path, gpu_dir) = discover_paths()?;
    let mut args = env::args_os();
    let _prog = args.next();
    let out_corpus = match args.next() {
        Some(arg) => PathBuf::from(arg),
        None => gpu_dir.join("v10_corpus.bin"),
    };
    let out_dir = parent_or_dot(&out_corpus);
    let out_boot_corpus = out_dir.join("v10_boot_corpus.bin");
    let out_eval = out_dir.join("v10_eval.bin");
    let out_boot_eval = out_dir.join("v10_boot_eval.bin");
    let out_vocab = out_dir.join("v10_vocab_bytes.bin");
    let out_prompts = out_dir.join("v10_eval_prompts.json");
    let out_prompts_bin = out_dir.join("v10_eval_prompts.bin");
    let cache_dir = match env::var_os("UOR_HF_CACHE") {
        Some(val) if !val.is_empty() => PathBuf::from(val),
        _ => gpu_dir.join(".hf_cache_v10"),
    };

    export_vocab_raw_bytes(&tok_path, &out_vocab)?;
    let tok = ByteBpe4096::from_file(&tok_path)?;

    let mut rng = Rng64::new(20261111);
    let smoltalk = load_smoltalk_dialogues(&cache_dir);
    let (open_qa, open_code) = load_dolly_and_alpaca(&cache_dir);

    let mut sb = SampleBuilder::new(&tok);
    let mut eval_sb = SampleBuilder::new(&tok);

    generate_bucket_a_conversations(&mut sb, &mut rng, &smoltalk, &mut eval_sb);
    generate_bucket_b_qa(&mut sb, &mut rng, &open_qa, &mut eval_sb);
    generate_bucket_c_code(&mut sb, &mut rng, &open_code, &mut eval_sb);
    generate_bucket_d_mvm_and_git_tools(&mut sb, &mut eval_sb);
    generate_bucket_e_web_tools(&mut sb, &mut eval_sb);

    // Focused high-SNR fast-start subset for compact `6L 64D` (`bootstrap_v9.uorgllm`):
    // Preserve strong structured C/D/E supervision while capping single-turn plain-prose repetition (~24,300 crisp sequences).
    let end_tok_id = tok.token_to_id("<|end|>");
    let mut repo_ack_reply_ids = tok.encode("Noted your saved name here for you.");
    repo_ack_reply_ids.push(1);
    let raw_boot_indices: Vec<usize> = sb
        .records
        .iter()
        .enumerate()
        .filter(|(_, rec)| {
            !rec.bucket.ends_with("_open")
                && !rec.bucket.ends_with("_ext")
                && !rec.bucket.ends_with("_top")
        })
        .map(|(i, _)| i)
        .collect();

    let mut boot_totals: HashMap<(&[u16], &[u16]), usize> =
        HashMap::with_capacity(raw_boot_indices.len());
    for &idx in &raw_boot_indices {
        let rec = &sb.records[idx];
        let k_sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
        *boot_totals.entry(k_sig).or_insert(0) += 1;
    }
    let mut boot_seen: HashMap<(&[u16], &[u16]), usize> = HashMap::with_capacity(boot_totals.len());
    let mut boot_records: Vec<CorpusRecord> = Vec::with_capacity(raw_boot_indices.len());
    for &idx in &raw_boot_indices {
        let rec = &sb.records[idx];
        let k_sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
        let tot = boot_totals[&k_sig];
        let has_copy = rec.copy_flags.iter().any(|&f| f != 0);
        let is_plain_tool = !has_copy && (rec.bucket == "D" || rec.bucket == "E");
        let is_step2_d = has_copy && rec.bucket == "D" && tot >= 10;
        let is_repo_ack_plain = !has_copy
            && rec.bucket == "A"
            && rec.reply_ids.as_slice() == repo_ack_reply_ids.as_slice();
        let cap = if is_plain_tool {
            ((tot * 3 + 4) / 5).max(10).min(235)
        } else if !has_copy && tot >= 340 {
            ((tot * 3 + 4) / 5).min(230)
        } else if !has_copy && tot >= 150 {
            ((tot * 3 + 4) / 5).min(160)
        } else if !has_copy && tot >= 45 {
            ((tot * 2 + 2) / 3).max(20).min(48)
        } else if is_repo_ack_plain {
            tot.min(13)
        } else if !has_copy && (rec.bucket == "A" || rec.bucket == "B") && tot >= 12 {
            ((tot * 2 + 4) / 5).max(4)
        } else if is_step2_d {
            ((tot * 2 + 2) / 3).max(2)
        } else {
            ((tot + 2) / 3).max(1)
        };
        let used = boot_seen.entry(k_sig).or_insert(0);
        if *used < cap {
            *used += 1;
            boot_records.push(rec.clone());
        }
    }

    // For full `8L 128D` (`v10_corpus.bin`): keep pre-topup around ~40,000..44,500 so `top_up_to_target`
    // always contributes ~5,500..10,000 balanced records (`docs.rs` `web_fetch`, `[agent_mode]` Step 1, `fn_rust`, `git clone`)!
    let mut all_totals: HashMap<(&[u16], &[u16]), usize> = HashMap::with_capacity(sb.records.len());
    for rec in &sb.records {
        let k_sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
        *all_totals.entry(k_sig).or_insert(0) += 1;
    }
    let mut all_seen: HashMap<(&[u16], &[u16]), usize> = HashMap::with_capacity(all_totals.len());
    let mut keep_flags: Vec<bool> = Vec::with_capacity(sb.records.len());
    for rec in &sb.records {
        let k_sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
        let tot_k = all_totals[&k_sig];
        let has_copy = rec.copy_flags.iter().any(|&f| f != 0);
        let is_ext = rec.bucket.ends_with("_ext");
        let has_end = end_tok_id.map_or(false, |eid| rec.prompt_ids.contains(&eid));
        let c_cap = if is_ext {
            ((tot_k + 1) / 2).max(1)
        } else if (rec.bucket == "A" && (has_copy || (has_end && tot_k <= 45)))
            || (has_copy && rec.bucket == "D" && tot_k == 40)
        {
            ((tot_k * 3 + 2) / 4).max(3)
        } else if has_copy && tot_k >= 5 {
            ((tot_k + 1) / 2).max(2)
        } else if !has_copy && tot_k >= 340 {
            tot_k.min(520)
        } else if !has_copy && tot_k >= 180 {
            tot_k.min(380)
        } else if !has_copy && tot_k >= 120 {
            tot_k.min(250)
        } else if !has_copy && tot_k >= 55 {
            tot_k.min(100)
        } else if !has_copy && tot_k >= 34 {
            ((tot_k * 4 + 4) / 5).min(76)
        } else if !has_copy && tot_k >= 4 {
            ((tot_k * 3 + 3) / 4).max(3)
        } else {
            tot_k
        };
        let used_c = all_seen.entry(k_sig).or_insert(0);
        if *used_c < c_cap {
            *used_c += 1;
            keep_flags.push(true);
        } else {
            keep_flags.push(false);
        }
    }
    drop(all_seen);
    drop(all_totals);
    let mut keep_iter = keep_flags.into_iter();
    sb.records.retain(|_| keep_iter.next().unwrap_or(false));

    let pre_topup_count = sb.records.len();
    println!(
        "[corpus] pre-topup records={pre_topup_count} (boot_records={})",
        boot_records.len()
    );
    assert!(
        pre_topup_count <= 88_000,
        "pre_topup_count={pre_topup_count} too high — would starve top_up_to_target!"
    );

    top_up_to_target(&mut sb, &mut rng);

    // Remove any accidental prompt-key overlap between train and eval (strict disjointness!)
    let has_overlap = eval_sb.seen_keys.iter().any(|k| sb.seen_keys.contains(k));
    if has_overlap {
        eval_sb.records.retain(|rec| {
            let slice = if rec.prompt_ids.len() > 1 {
                &rec.prompt_ids[1..]
            } else {
                &[]
            };
            let p_str = tok.decode(slice);
            !sb.seen_keys.contains(&norm_key(&p_str))
        });
    }

    rng.shuffle(&mut boot_records);
    rng.shuffle(&mut sb.records);

    // Verify zero forbidden held-out identifiers/topics AND full-corpus (no dedup collapse!) span-start orthogonality!
    {
        let mut checked_text_sigs: HashSet<(&[u16], &[u16])> =
            HashSet::with_capacity(sb.records.len());
        let mut span_on: HashMap<(usize, u16), usize> = HashMap::new();
        let mut span_off: HashMap<(usize, u16), usize> = HashMap::new();
        let mut combined_ids: Vec<u16> = Vec::with_capacity(MAX_SEQ);
        for rec in &sb.records {
            let sig = (rec.prompt_ids.as_slice(), rec.reply_ids.as_slice());
            if checked_text_sigs.insert(sig) {
                combined_ids.clear();
                if rec.prompt_ids.len() > 1 {
                    combined_ids.extend_from_slice(&rec.prompt_ids[1..]);
                }
                if rec.reply_ids.len() > 1 {
                    combined_ids.extend_from_slice(&rec.reply_ids[..rec.reply_ids.len() - 1]);
                }
                let txt = tok.decode(&combined_ids);
                assert!(
                    !contains_forbidden(&txt),
                    "Held-out leakage detected: {txt}"
                );
            }
            let ft0 = rec.reply_ids.first().copied().unwrap_or(0);
            let ft1 = rec.reply_ids.get(1).copied().unwrap_or(0);
            let has_copy = rec.copy_flags.iter().any(|&f| f != 0);
            let structured =
                has_copy || matches!(ft0, 30 | 66) || (ft0 == 223 && matches!(ft1, 30 | 66));
            if structured {
                let mut sp = 1usize;
                for k in 0..rec.reply_ids.len() {
                    let cur_tok = if k == 0 {
                        *rec.prompt_ids.last().unwrap_or(&0)
                    } else {
                        rec.reply_ids[k - 1]
                    };
                    if k > 0 && rec.copy_flags[k - 1] == 0 {
                        sp = (sp + 1).min(255);
                    }
                    let is_start = rec.copy_flags[k] != 0 && (k == 0 || rec.copy_flags[k - 1] == 0);
                    let fence_nl = matches!(cur_tok, 29 | 66 | 201);
                    if !fence_nl && (k == 0 || rec.copy_flags[k - 1] == 0) {
                        let key = (sp, cur_tok & 63);
                        if is_start {
                            *span_on.entry(key).or_insert(0) += 1;
                        } else {
                            *span_off.entry(key).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        let mut collisions: Vec<((usize, u16), usize, usize)> = span_on
            .iter()
            .filter_map(|(&k, &on)| span_off.get(&k).map(|&off| (k, on, off)))
            .collect();
        collisions.sort_by(|a, b| b.1.min(b.2).cmp(&a.1.min(a.2)).then_with(|| a.0.cmp(&b.0)));
        let preview: Vec<((usize, u16), usize, usize)> =
            collisions.iter().take(8).copied().collect();
        println!(
            "[corpus] FULL-CORPUS structured (pos_idx, tok&63) span-start audit: \
             {} active states, collisions={:?} (total={})",
            span_on.len(),
            preview,
            collisions.len()
        );
        assert!(
            collisions.iter().all(|&(_, on, off)| on.min(off) == 0),
            "Span-start collision remained: {:?}",
            preview
        );
    }

    if sb.records.len() > 102_000 {
        sb.records.truncate(102_000);
    }
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
    write_held_out_eval_prompts(&out_prompts, &out_prompts_bin, &tok)?;

    let mut bucket_counts: BTreeMap<char, usize> = BTreeMap::new();
    let mut max_len_seen = 0usize;
    let mut copy_tok_total = 0usize;
    for rec in &sb.records {
        let b_root = rec.bucket.chars().next().unwrap_or('?');
        *bucket_counts.entry(b_root).or_insert(0) += 1;
        max_len_seen = max_len_seen.max(rec.prompt_ids.len() + rec.reply_ids.len());
        copy_tok_total += rec.copy_flags.iter().map(|&c| c as usize).sum::<usize>();
    }
    let out_mib = fs::metadata(&out_corpus)
        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    println!(
        "[corpus] wrote {} full train sequences -> {} ({out_mib:.2} MiB), \
         {} fast-start boot sequences -> {}, \
         {} held-out eval sequences -> {} ({} boot-eval -> {}), \
         max_seq_seen={max_len_seen}, copy_tokens={copy_tok_total}, buckets={bucket_counts:?}",
        sb.records.len(),
        out_corpus.display(),
        boot_records.len(),
        out_boot_corpus.display(),
        eval_sb.records.len(),
        out_eval.display(),
        boot_eval_records.len(),
        out_boot_eval.display(),
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[corpus] error: {err}");
            ExitCode::FAILURE
        }
    }
}
