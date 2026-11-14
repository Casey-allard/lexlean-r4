//! Streams and curates high-quality instruction, conversational, coding, mathematics,
//! science, and general-knowledge turns from HuggingFace into a TSV corpus for
//! `gpu/train_uorlut_v8.rs`.
//!
//! Track 1 (--mode fast):
//!   Downloads & curates `databricks-dolly-15k`, `CodeAlpaca-20k`, and `alpaca-cleaned`
//!   (~86,700 raw turns -> top curated diverse multi-domain turns).
//!
//! Track 2 (--mode full):
//!   Additionally streams `teknium/OpenHermes-2.5` (GPT-4 distilled multi-domain
//!   conversations) to build a large 100,000+ turn corpus.

mod bpe_json;

use bpe_json::{curl_download_cached, json_escape, sha256_hex, JsonVal};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};

const FAST_DATASETS: &[(&str, &str, &str)] = &[
    (
        "dolly_15k",
        "https://huggingface.co/datasets/databricks/databricks-dolly-15k/resolve/main/databricks-dolly-15k.jsonl",
        "jsonl",
    ),
    (
        "code_alpaca_20k",
        "https://huggingface.co/datasets/sahil2801/CodeAlpaca-20k/resolve/main/code_alpaca_20k.json",
        "json",
    ),
    (
        "alpaca_cleaned",
        "https://huggingface.co/datasets/yahma/alpaca-cleaned/resolve/main/alpaca_data_cleaned.json",
        "json",
    ),
];

const FULL_EXTRA_DATASETS: &[(&str, &str, &str)] = &[(
    "openhermes_2_5",
    "https://huggingface.co/datasets/teknium/OpenHermes-2.5/resolve/main/openhermes2_5.json",
    "json_stream",
)];

type Pair = (i32, String, String, String);

fn clean_field(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");
    let trimmed = normalized.trim();
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

fn encode_tsv_field(text: &str) -> String {
    clean_field(text)
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', " ")
}

fn is_high_quality_pair(prompt: &str, reply: &str) -> bool {
    let plen = prompt.chars().count();
    if !(8..=360).contains(&plen) {
        return false;
    }
    let rlen = reply.chars().count();
    if !(35..=1400).contains(&rlen) {
        return false;
    }
    let lower_p = prompt.to_lowercase();
    let lower_r = reply.to_lowercase();
    for bad in [
        "http://",
        "https://",
        "www.",
        "image below",
        "photo above",
        "click here",
        "as an ai language model",
        "openai",
    ] {
        if lower_p.contains(bad) || lower_r.contains(bad) {
            return false;
        }
    }
    let ascii_count = prompt.chars().filter(|&c| (c as u32) < 128).count();
    let ascii_ratio = (ascii_count as f64) / (plen.max(1) as f64);
    ascii_ratio >= 0.92
}

fn download_file(url: &str, dest: &Path, label: &str) -> Result<(), String> {
    if dest.exists() {
        if let Ok(meta) = fs::metadata(dest) {
            if meta.len() > 100_000 {
                println!("  [cached] {} ({} bytes)", dest.display(), meta.len());
                return Ok(());
            }
        }
    }
    println!("  [downloading] {url} -> {} ...", dest.display());
    let ok = curl_download_cached(url, dest, label)?;
    if !ok || !dest.is_file() {
        return Err(format!("failed to download {label} from {url}"));
    }
    let size = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    println!("  [done] {} ({size} bytes)", dest.display());
    Ok(())
}

fn parse_dolly(path: &Path) -> Result<Vec<Pair>, String> {
    let f = File::open(path).map_err(|e| format!("failed opening {}: {e}", path.display()))?;
    let reader = BufReader::new(f);
    let mut pairs = Vec::new();
    for (line_no, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("read error at line {}: {e}", line_no + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let obj = JsonVal::parse(trimmed).map_err(|e| {
            format!(
                "JSON parse error in {}:{}: {e}",
                path.display(),
                line_no + 1
            )
        })?;
        let inst = clean_field(
            obj.get("instruction")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
        let ctx = clean_field(obj.get("context").and_then(|v| v.as_str()).unwrap_or(""));
        let resp = clean_field(obj.get("response").and_then(|v| v.as_str()).unwrap_or(""));
        let cat = obj.get("category").and_then(|v| v.as_str()).unwrap_or("");

        let prompt = if !ctx.is_empty() && ctx.chars().count() <= 220 {
            format!("{inst}\nContext: {ctx}")
        } else if ctx.is_empty() {
            inst
        } else {
            continue;
        };
        if is_high_quality_pair(&prompt, &resp) {
            let priority = if matches!(
                cat,
                "open_qa" | "general_qa" | "brainstorming" | "creative_writing"
            ) {
                2
            } else {
                1
            };
            pairs.push((priority, format!("dolly:{cat}"), prompt, resp));
        }
    }
    Ok(pairs)
}

fn parse_alpaca_json(path: &Path, source_tag: &str) -> Result<Vec<Pair>, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed reading {}: {e}", path.display()))?;
    let data =
        JsonVal::parse(&raw).map_err(|e| format!("JSON parse error in {}: {e}", path.display()))?;
    let items = data
        .as_array()
        .ok_or_else(|| format!("expected top-level JSON array in {}", path.display()))?;

    let mut pairs = Vec::new();
    let boost_prefixes = [
        "what ",
        "how ",
        "why ",
        "explain ",
        "write ",
        "describe ",
        "compare ",
        "who ",
        "where ",
        "when ",
        "can you ",
        "tell me ",
        "create ",
        "implement ",
        "solve ",
    ];

    for obj in items {
        let inst = clean_field(
            obj.get("instruction")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        );
        let inp = clean_field(obj.get("input").and_then(|v| v.as_str()).unwrap_or(""));
        let out = clean_field(obj.get("output").and_then(|v| v.as_str()).unwrap_or(""));
        let prompt = if !inp.is_empty() && inp.chars().count() <= 180 {
            if inst.ends_with(':') {
                format!("{inst} {inp}")
            } else {
                format!("{inst}: {inp}")
            }
        } else if inp.is_empty() {
            inst
        } else {
            continue;
        };
        if is_high_quality_pair(&prompt, &out) {
            let lp = prompt.to_lowercase();
            let priority = if boost_prefixes.iter().any(|w| lp.starts_with(w)) {
                2
            } else {
                1
            };
            pairs.push((priority, source_tag.to_string(), prompt, out));
        }
    }
    Ok(pairs)
}

fn stream_openhermes_subset(
    url: &str,
    cache_jsonl: &Path,
    max_items: usize,
) -> Result<Vec<Pair>, String> {
    let existing_lines = if cache_jsonl.is_file() {
        let f = File::open(cache_jsonl)
            .map_err(|e| format!("failed opening {}: {e}", cache_jsonl.display()))?;
        BufReader::new(f).lines().count()
    } else {
        0
    };

    if existing_lines >= max_items {
        let size = fs::metadata(cache_jsonl).map(|m| m.len()).unwrap_or(0);
        println!(
            "  [cached] {} ({existing_lines} lines, {size} bytes)",
            cache_jsonl.display()
        );
    } else {
        println!(
            "  [streaming] {url} -> {} (up to {max_items} items)...",
            cache_jsonl.display()
        );
        let mut child = Command::new("curl")
            .args([
                "-fsSL",
                "--connect-timeout",
                "15",
                "--retry",
                "2",
                "-A",
                "Mozilla/5.0 (UOR-R4-Trainer)",
                url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("failed spawning curl: {e}"))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed capturing curl stdout".to_string())?;
        let out_file = File::create(cache_jsonl)
            .map_err(|e| format!("failed creating {}: {e}", cache_jsonl.display()))?;
        let mut out = BufWriter::new(out_file);

        let mut chunk = [0u8; 256 * 1024];
        let mut obj_buf: Vec<u8> = Vec::with_capacity(16 * 1024);
        let mut in_obj = false;
        let mut in_string = false;
        let mut escaped = false;
        let mut depth = 0usize;
        let mut count = 0usize;

        'stream: while count < max_items {
            let n = stdout
                .read(&mut chunk)
                .map_err(|e| format!("curl stream read error: {e}"))?;
            if n == 0 {
                break;
            }
            for &b in &chunk[..n] {
                if !in_obj {
                    if b == b'{' {
                        in_obj = true;
                        in_string = false;
                        escaped = false;
                        depth = 1;
                        obj_buf.clear();
                        obj_buf.push(b);
                    }
                    continue;
                }
                obj_buf.push(b);
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == b'"' {
                        in_string = false;
                    }
                } else if b == b'"' {
                    in_string = true;
                } else if b == b'{' {
                    depth += 1;
                } else if b == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        in_obj = false;
                        let parsed = match JsonVal::parse_bytes(&obj_buf) {
                            Ok(v) => Ok(v),
                            Err(_) => {
                                let lossy = String::from_utf8_lossy(&obj_buf);
                                JsonVal::parse(&lossy)
                            }
                        };
                        if let Ok(obj) = parsed {
                            if let Some(convs) = obj.get("conversations").and_then(|v| v.as_array())
                            {
                                let mut pending_user = String::new();
                                for turn in convs {
                                    let role =
                                        turn.get("from").and_then(|v| v.as_str()).unwrap_or("");
                                    let val =
                                        turn.get("value").and_then(|v| v.as_str()).unwrap_or("");
                                    if matches!(role, "human" | "user") {
                                        pending_user = clean_field(val);
                                    } else if matches!(role, "gpt" | "assistant")
                                        && !pending_user.is_empty()
                                    {
                                        let asst_msg = clean_field(val);
                                        if is_high_quality_pair(&pending_user, &asst_msg) {
                                            writeln!(
                                                out,
                                                "{{\"p\":{},\"r\":{}}}",
                                                json_escape(&pending_user),
                                                json_escape(&asst_msg)
                                            )
                                            .map_err(|e| format!("write error: {e}"))?;
                                            count += 1;
                                            if count % 10000 == 0 {
                                                println!(
                                                    "    streamed {count} OpenHermes-2.5 pairs..."
                                                );
                                            }
                                            if count >= max_items {
                                                break 'stream;
                                            }
                                        }
                                        pending_user.clear();
                                    }
                                }
                            }
                        }
                        obj_buf.clear();
                    }
                }
            }
        }

        out.flush().map_err(|e| format!("flush error: {e}"))?;
        if count >= max_items {
            let _ = child.kill();
            let _ = child.wait();
        } else {
            let status = child
                .wait()
                .map_err(|e| format!("failed waiting for curl: {e}"))?;
            if !status.success() && count == 0 {
                return Err(format!("curl failed streaming {url}: {status}"));
            }
        }
        println!(
            "  [done] streamed {count} OpenHermes-2.5 pairs to {}",
            cache_jsonl.display()
        );
    }

    let f = File::open(cache_jsonl)
        .map_err(|e| format!("failed opening {}: {e}", cache_jsonl.display()))?;
    let reader = BufReader::new(f);
    let mut pairs = Vec::new();
    for (line_no, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("read error at line {}: {e}", line_no + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let obj = JsonVal::parse(trimmed)
            .map_err(|e| format!("JSON parse error in cache line {}: {e}", line_no + 1))?;
        let p = obj
            .get("p")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("missing 'p' in cache line {}", line_no + 1))?;
        let r = obj
            .get("r")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("missing 'r' in cache line {}", line_no + 1))?;
        pairs.push((3, "openhermes2.5".to_string(), p.to_string(), r.to_string()));
    }
    Ok(pairs)
}

fn generate_synthetic_corpus(limit: usize) -> Vec<Pair> {
    let disciplines: &[(&str, &[(&str, &str)])] = &[
        (
            "synthetic_math",
            &[
                (
                    "Calculate d/dx (x^3 + 2x) at x=2",
                    "<thinking>\nStep 1: Compute d/dx (x^3 + 2x) = 3x^2 + 2\nStep 2: Evaluate at x=2 -> 3(4)+2 = 14\n</thinking>\nThe derivative at x=2 is 14.",
                ),
                (
                    "What is 5 + 7?",
                    "<thinking>\n5 + 7 = 12\n</thinking>\n12",
                ),
                (
                    "Solve x^2 - 4 = 0 for real roots",
                    "<thinking>\nFactor (x - 2)(x + 2) = 0, so x = 2 or x = -2\n</thinking>\nThe solutions are x = 2 and x = -2.",
                ),
                (
                    "Compute the definite integral of 2x dx from 0 to 3",
                    "<thinking>\nAntiderivative is x^2. [x^2]_0^3 = 9 - 0 = 9\n</thinking>\nThe integral is 9.",
                ),
            ],
        ),
        (
            "synthetic_code",
            &[
                (
                    "Add bounds check to array indexing in Rust",
                    "<thinking>\nNeed to add bounds check before indexing\n</thinking>\n```diff\n- let val = arr[idx];\n+ let val = arr.get(idx).copied().unwrap_or(0);\n```",
                ),
                (
                    "Refactor pub fn add_item for safe error handling",
                    "<thinking>\nAvoid panic on capacity exceeded, return Result\n</thinking>\n```diff\n- pub fn add_item(vec: &mut Vec<i32>, val: i32) { vec.push(val); }\n+ pub fn add_item(vec: &mut Vec<i32>, val: i32) -> Result<(), std::collections::TryReserveError> {\n+     vec.try_reserve(1)?;\n+     vec.push(val);\n+     Ok(())\n+ }\n```",
                ),
                (
                    "How to handle unwrap safely in Rust?",
                    "<thinking>\nReplace direct unwrap with pattern matching or ? operator\n</thinking>\nUse pattern matching (if let / match) or the ? operator to avoid runtime panics.",
                ),
            ],
        ),
        (
            "synthetic_systems",
            &[
                (
                    "Explain mutex deadlock prevention in concurrent systems",
                    "<thinking>\nAnalyze mutex deadlock condition between lock A and B\nOrder locks: acquire A then B\n</thinking>\nEnsure consistent acquisition order across all worker threads.",
                ),
                (
                    "Describe CAP theorem trade-offs during network partition",
                    "<thinking>\nDuring network partition, choose consistency or availability\n</thinking>\nSystems cannot achieve both linearizability and availability during partitions.",
                ),
                (
                    "How to prevent cache stampedes in distributed systems?",
                    "<thinking>\nUse write-through or TTL-based expiration with probabilistic jitter\n</thinking>\nMitigate cache stampedes using single-flight request coalescing.",
                ),
            ],
        ),
        (
            "synthetic_lean4",
            &[
                (
                    "Prove nat addition commutativity in Lean 4",
                    "<thinking>\nProof by induction on n: base case n=0 trivial by rfl\n</thinking>\ntheorem add_zero (n : Nat) : n + 0 = n := by\n  induction n with\n  | zero => rfl\n  | succ d ih => rw [Nat.add_succ, ih]",
                ),
                (
                    "State and prove reflexivity of equality in Lean 4",
                    "<thinking>\nTrivial by rfl in Lean 4\n</thinking>\ntheorem eq_refl (x : Nat) : x = x := by rfl",
                ),
            ],
        ),
    ];

    let mut pairs = Vec::with_capacity(limit);
    let mut round = 0usize;
    while pairs.len() < limit {
        for &(src, examples) in disciplines {
            for &(p, r) in examples {
                let suffix = if round > 0 {
                    format!(" (case {round})")
                } else {
                    String::new()
                };
                pairs.push((5, src.to_string(), format!("{p}{suffix}"), r.to_string()));
                if pairs.len() >= limit {
                    break;
                }
            }
            if pairs.len() >= limit {
                break;
            }
        }
        round += 1;
    }
    pairs
}

fn normalize_prompt_key(prompt: &str) -> String {
    let filtered: String = prompt
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == ' ')
        .collect();
    let trimmed = filtered.trim();
    trimmed.chars().take(80).collect()
}

struct CliArgs {
    mode: String,
    synthetic: bool,
    limit: usize,
    out: PathBuf,
    cache_dir: PathBuf,
}

fn print_help() {
    println!("Usage: fetch_hf_datasets [OPTIONS]");
    println!("Options:");
    println!("  --mode <fast|full|synthetic>  Dataset track mode (default: fast)");
    println!("  --synthetic                   Generate deterministic synthetic reasoning traces");
    println!("  --limit <N>                   Maximum curated pairs to output (default: 14000)");
    println!(
        "  --out <PATH>                  Output TSV path (default: /tmp/uor_external_corpus.tsv)"
    );
    println!("  --cache-dir <DIR>             Cache directory (default: /tmp/uor_hf_cache)");
    println!("  -h, --help                    Show this help message and exit");
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut mode = "fast".to_string();
    let mut synthetic = false;
    let mut limit = 14000usize;
    let mut out = PathBuf::from("/tmp/uor_external_corpus.tsv");
    let mut cache_dir = PathBuf::from("/tmp/uor_hf_cache");

    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "-h" || arg == "--help" {
            print_help();
            return Ok(None);
        } else if arg == "--synthetic" {
            synthetic = true;
        } else if let Some(val) = arg.strip_prefix("--mode=") {
            mode = val.to_string();
        } else if arg == "--mode" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--mode requires a value".to_string())?;
            mode = val.clone();
        } else if let Some(val) = arg.strip_prefix("--limit=") {
            limit = val
                .parse::<usize>()
                .map_err(|_| format!("invalid --limit value '{val}'"))?;
        } else if arg == "--limit" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--limit requires a value".to_string())?;
            limit = val
                .parse::<usize>()
                .map_err(|_| format!("invalid --limit value '{val}'"))?;
        } else if let Some(val) = arg.strip_prefix("--out=") {
            out = PathBuf::from(val);
        } else if arg == "--out" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--out requires a value".to_string())?;
            out = PathBuf::from(val);
        } else if let Some(val) = arg.strip_prefix("--cache-dir=") {
            cache_dir = PathBuf::from(val);
        } else if arg == "--cache-dir" {
            i += 1;
            let val = args
                .get(i)
                .ok_or_else(|| "--cache-dir requires a value".to_string())?;
            cache_dir = PathBuf::from(val);
        } else {
            return Err(format!("unrecognized argument: {arg}"));
        }
        i += 1;
    }

    if !matches!(mode.as_str(), "fast" | "full" | "synthetic") {
        return Err(format!(
            "invalid --mode '{mode}' (expected one of: fast, full, synthetic)"
        ));
    }

    Ok(Some(CliArgs {
        mode,
        synthetic,
        limit,
        out,
        cache_dir,
    }))
}

fn run() -> Result<(), String> {
    let Some(args) = parse_args()? else {
        return Ok(());
    };

    fs::create_dir_all(&args.cache_dir)
        .map_err(|e| format!("failed creating {}: {e}", args.cache_dir.display()))?;

    let mut all_pairs: Vec<Pair> = Vec::new();
    if args.synthetic || args.mode == "synthetic" {
        println!(
            "  [synthetic] Generating {} synthetic multi-discipline reasoning traces...",
            args.limit
        );
        all_pairs = generate_synthetic_corpus(args.limit);
    } else {
        for &(name, url, fmt) in FAST_DATASETS {
            let dest = args.cache_dir.join(format!("{name}.{fmt}"));
            download_file(url, &dest, name)?;
            let parsed = if name == "dolly_15k" {
                parse_dolly(&dest)?
            } else {
                parse_alpaca_json(&dest, name)?
            };
            println!("  Parsed {} high-quality pairs from {name}", parsed.len());
            all_pairs.extend(parsed);
        }
    }

    if args.mode == "full" {
        let oh_cache = args.cache_dir.join("openhermes_stream.jsonl");
        let oh_pairs =
            stream_openhermes_subset(FULL_EXTRA_DATASETS[0].1, &oh_cache, args.limit.max(60000))?;
        println!(
            "  Parsed {} high-quality pairs from OpenHermes-2.5",
            oh_pairs.len()
        );
        all_pairs.extend(oh_pairs);
    }

    // Deduplicate by normalized prompt prefix and sort deterministically by priority + hash
    let mut seen_prompts: HashSet<String> = HashSet::with_capacity(all_pairs.len());
    let mut deduped: Vec<(i32, String, String, String, String)> =
        Vec::with_capacity(all_pairs.len());
    for (prio, src, p, r) in all_pairs {
        let norm_key = normalize_prompt_key(&p);
        if norm_key.len() < 6 || !seen_prompts.insert(norm_key.clone()) {
            continue;
        }
        let mut h = sha256_hex(norm_key.as_bytes());
        h.truncate(12);
        deduped.push((prio, h, src, p, r));
    }

    // Sort highest priority first, then deterministic hash so every domain is balanced
    deduped.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let selected_len = deduped.len().min(args.limit);
    let selected = &deduped[..selected_len];

    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed creating {}: {e}", parent.display()))?;
        }
    }
    {
        let out_file = File::create(&args.out)
            .map_err(|e| format!("failed creating {}: {e}", args.out.display()))?;
        let mut out = BufWriter::new(out_file);
        for (_prio, _h, src, p, r) in selected {
            writeln!(
                out,
                "{src}\t{}\t{}",
                encode_tsv_field(p),
                encode_tsv_field(r)
            )
            .map_err(|e| format!("write error: {e}"))?;
        }
        out.flush().map_err(|e| format!("flush error: {e}"))?;
    }

    let out_size = fs::metadata(&args.out).map(|m| m.len()).unwrap_or(0);
    println!(
        "=== Wrote {} curated multi-domain pairs (from {} unique) to {} ({out_size} bytes) ===",
        selected.len(),
        deduped.len(),
        args.out.display()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(2);
    }
}
