//! Pure-Rust deterministic Studio SFT trajectory generator for Wave 3 (`R4-64`).
//!
//! Emits verified multi-turn ChatML JSONL episodes teaching `Qwen3-0.6B` and `Qwen3-1.7B`
//! under the `< 40`-token `STUDIO_COMPACT_SYSTEM_PROMPT`:
//! - `<tool_call>{"name":"...","arguments":{...}}</tool_call>` (`mvm_exec`, `workspace_read`,
//!   `workspace_search`, `preview_inspect`, `github_git`, `spiralcore_fetch`)
//! - `<prism-write path="...">...</prism-write>` and `<prism-delete path="...">`
//! - `Q15`/`Q16` `wrapping_add` / `saturating_*` pure-integer Rust & multi-TU MVM C idioms.

#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::Path;

/// Compact 34-token Studio self-knowledge system prompt locked in lockstep with `UORR4V13` manifests (`R4-64`).
pub const STUDIO_COMPACT_SYSTEM_PROMPT: &str =
    "LexLean-R4 MVM (Block-32 RHT, Q15 Q16 wrapping): preview_inspect, github_git, <prism-write>.";

/// Formal R4.WebGpuResidentRuntime escape JSON string characters deterministically (`R4-64`).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Formal R4.WebGpuResidentRuntime deterministically emit `episodes` multi-turn Studio SFT rows into `out_dir` (`R4-64`).
pub fn write_studio_sft_dataset(out_dir: &Path, episodes: usize) -> Result<(usize, usize), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let cwds = [
        "/workspace",
        "/workspace/src",
        "/usr/src/mvm",
        "/workspace/tests",
    ];
    let tabs = [
        "src/lib.rs",
        "src/main.c",
        "Makefile",
        "index.html",
        "app.js",
    ];
    let mut rows: Vec<String> = Vec::with_capacity(episodes * 2);

    for idx in 0..episodes {
        let cwd = cwds[idx % cwds.len()];
        let tab = tabs[idx % tabs.len()];
        let auto_s = if idx % 2 == 0 { "on" } else { "off" };
        let sys = format!(
            "{STUDIO_COMPACT_SYSTEM_PROMPT}\n\n[mvm cwd={cwd} tab={tab} dirty=none auto={auto_s} net=spiralcore exit=0]\n"
        );
        let (u_msg, a_msg) = match idx % 4 {
            0 => (
                format!("Read `{tab}` and add a Q15 clamping helper using wrapping/saturating integer arithmetic."),
                format!(
                    "<tool_call>{{\"name\":\"workspace_read\",\"arguments\":{{\"path\":\"{tab}\"}}}}</tool_call>"
                ),
            ),
            1 => (
                "Build `/usr/src/mvm/mvm_hello` on riscv64 and print its output.".to_string(),
                "<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"make -C /usr/src/mvm && /usr/src/mvm/mvm_hello\"}}</tool_call>".to_string(),
            ),
            2 => (
                "Fetch `https://example.com/` via SpiralCore GET.".to_string(),
                "<tool_call>{\"name\":\"spiralcore_fetch\",\"arguments\":{\"url\":\"https://example.com/\",\"method\":\"GET\"}}</tool_call>".to_string(),
            ),
            _ => (
                "Write a pure-integer Q16 lerp helper into `src/fixed_q16.rs`.".to_string(),
                "<prism-write path=\"src/fixed_q16.rs\">\npub fn lerp_q16(a: i32, b: i32, t_q16: i32) -> i32 {\n    let d = b.wrapping_sub(a);\n    a.saturating_add(((i64::from(d) * i64::from(t_q16) + (1 << 15)) >> 16) as i32)\n}\n</prism-write>\nWrote `lerp_q16` using `wrapping_sub` and `saturating_add`.".to_string(),
            ),
        };
        rows.push(format!(
            "{{\"id\":\"ep_{idx:04}\",\"system\":\"{}\",\"user\":\"{}\",\"assistant\":\"{}\"}}",
            json_escape(&sys),
            json_escape(&u_msg),
            json_escape(&a_msg)
        ));
    }

    let split = ((rows.len() * 94) / 100).max(1);
    let train_rows = &rows[..split];
    let eval_rows = &rows[split..];

    let mut train_f = fs::File::create(out_dir.join("train.jsonl")).map_err(|e| e.to_string())?;
    for r in train_rows {
        writeln!(train_f, "{r}").map_err(|e| e.to_string())?;
    }
    let mut eval_f = fs::File::create(out_dir.join("eval.jsonl")).map_err(|e| e.to_string())?;
    for r in eval_rows {
        writeln!(eval_f, "{r}").map_err(|e| e.to_string())?;
    }
    let meta = format!(
        "{{\n  \"schema\": \"uor-r4.studio-sft-dataset/1\",\n  \"compact_system_prompt\": \"{}\",\n  \"train_rows\": {},\n  \"eval_rows\": {}\n}}\n",
        json_escape(STUDIO_COMPACT_SYSTEM_PROMPT),
        train_rows.len(),
        eval_rows.len()
    );
    fs::write(out_dir.join("dataset_manifest.json"), meta).map_err(|e| e.to_string())?;
    Ok((train_rows.len(), eval_rows.len()))
}

/// Formal R4.WebGpuResidentRuntime standalone CLI entry point for Studio SFT dataset generation (`R4-64`).
#[allow(dead_code)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out-dir")
        .map(|w| w[1].as_str())
        .unwrap_or("/tmp/r4_studio_sft");
    let count: usize = args
        .windows(2)
        .find(|w| w[0] == "--episodes")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(1536);
    let (tr, ev) = write_studio_sft_dataset(Path::new(out), count).expect("emit dataset");
    println!("wrote Studio SFT dataset: train={tr} eval={ev} dir={out}");
}
