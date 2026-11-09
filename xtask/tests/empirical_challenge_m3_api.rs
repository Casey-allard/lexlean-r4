//! Empirical Challenge Test Suite for Milestone 3: Standalone API Engine & TCP Daemon.
//!
//! Adversarial Fuzzing & Protocol Stress Tests:
//! 1. Massive multi-turn conversations (50 turns) testing rolling 256D context state
//!    numerical stability, coordinate clamping, and bitwise determinism without NaN/drift.
//! 2. Adversarial thinking tag stress: unclosed, nested, empty, and corrupted `<thinking>` / `<thought>` blocks.
//! 3. Malformed tool calls: corrupted JSON, missing names, null arguments, tool observation errors.
//! 4. Extreme and invalid payloads: empty inputs, missing fields, 500 KB payloads, heavy Unicode fuzzing,
//!    verifying graceful 400 responses with zero server panics.
//! 5. Live TCP loopback daemon fuzzing: raw socket drops, truncated bodies, non-UTF8 bytes, CORS preflights, 404s.
//! 6. Live TCP daemon concurrency hammer: 20 concurrent worker threads executing 200 rapid requests.
//! 7. Full regression gate against the 12-point loopback test suite.

#![allow(dead_code, unused_imports, clippy::all)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;

#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;

#[allow(dead_code, clippy::all)]
#[path = "../src/engine.rs"]
mod engine;

use engine::api::{
    handle_v1_chat_completions_payload, handle_v1_messages_payload, parse_assistant_turn,
    synthesize_turn_output, AnthropicMessagesRequest, AnthropicMessagesResponse,
    OpenAiChatCompletionRequest, OpenAiChatCompletionResponse, ParsedTurn,
};

fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral socket");
    listener.local_addr().expect("local addr").port()
}

fn xtask_binary_path() -> PathBuf {
    if let Ok(bin) = std::env::var("CARGO_BIN_EXE_xtask") {
        PathBuf::from(bin)
    } else {
        repo_model::repo_root().join("target/debug/xtask")
    }
}

struct TestServerDaemon {
    child: Child,
    port: u16,
    addr: String,
}

impl TestServerDaemon {
    fn spawn() -> Self {
        let port = find_free_port();
        let addr = format!("127.0.0.1:{port}");
        let bin = xtask_binary_path();
        assert!(bin.exists(), "xtask binary must exist at {}", bin.display());

        let child = Command::new(&bin)
            .args(["api", "serve", "--port", &port.to_string()])
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn xtask api serve on port {port}: {e}"));

        // Wait up to 30 seconds for daemon to accept connections under parallel CPU load
        let mut ready = false;
        for _ in 0..300 {
            thread::sleep(Duration::from_millis(100));
            if TcpStream::connect(&addr).is_ok() {
                ready = true;
                break;
            }
        }
        assert!(
            ready,
            "xtask api serve daemon failed to become ready on {addr}"
        );

        Self { child, port, addr }
    }
}

impl Drop for TestServerDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const CLIENT_SOCKET_TIMEOUT: Duration = Duration::from_secs(30);

fn send_raw_http_request_once(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, HashMap<String, String>, Vec<u8>), String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect error: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;

    let body_bytes = body.unwrap_or(&[]);
    let mut req_header =
        format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    for (k, v) in headers {
        req_header.push_str(&format!("{k}: {v}\r\n"));
    }
    if !headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-length"))
    {
        req_header.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    }
    req_header.push_str("\r\n");

    stream
        .write_all(req_header.as_bytes())
        .map_err(|e| format!("write header: {e}"))?;
    if !body_bytes.is_empty() {
        stream
            .write_all(body_bytes)
            .map_err(|e| format!("write body: {e}"))?;
    }
    stream.flush().map_err(|e| format!("flush: {e}"))?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|e| format!("read status line: {e}"))?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(format!("malformed status line: {status_line}"));
    }
    let status_code: u16 = parts[1]
        .parse()
        .map_err(|e| format!("parse status code: {e}"))?;

    let mut res_headers = HashMap::new();
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("read header: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            res_headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let mut res_body = Vec::new();
    if let Some(len) = res_headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        res_body.resize(len, 0u8);
        if len > 0 {
            reader
                .read_exact(&mut res_body)
                .map_err(|e| format!("read exact body: {e}"))?;
        }
    } else {
        let _ = reader.read_to_end(&mut res_body);
    }

    Ok((status_code, res_headers, res_body))
}

fn send_raw_http_request(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> Result<(u16, HashMap<String, String>, Vec<u8>), String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        match send_raw_http_request_once(addr, method, path, headers, body, CLIENT_SOCKET_TIMEOUT) {
            Ok(res) => return Ok(res),
            Err(e) => {
                last_err = e;
                if attempt + 1 < 3 {
                    thread::sleep(Duration::from_millis(50 * (attempt + 1) as u64));
                }
            }
        }
    }
    Err(last_err)
}

// ============================================================================
// 1. Massive Multi-Turn Conversations & 256D State Bounds
// ============================================================================

#[test]
fn challenge_1_massive_multiturn_50_turns_messages_latent_evolution_and_determinism() {
    let run_conversation = || {
        let mut context_opt: Option<serde_json::Value> = None;
        let mut turn_records = Vec::new();

        for turn in 1..=50 {
            let prompt = format!("Turn {turn}: evaluate Wilson theorem primality and coordinate state for integer {turn}");
            let req = serde_json::json!({
                "model": "uor-r4.lut-stack/8",
                "messages": [{"role": "user", "content": prompt}],
                "context": context_opt,
            });

            let (status, resp_str) =
                handle_v1_messages_payload(&req.to_string()).expect("messages turn should succeed");
            assert_eq!(status, 200);

            let resp: AnthropicMessagesResponse =
                serde_json::from_str(&resp_str).expect("valid AnthropicMessagesResponse");
            assert_eq!(resp.role, "assistant");
            assert_eq!(resp.stop_reason, "end_turn");

            let ctx = resp.context.expect("response must carry updated context");
            let turn_index = ctx["turn_index"].as_u64().expect("turn_index u64");
            assert_eq!(turn_index, turn as u64, "turn_index must increment by 1");

            let latent_arr = ctx["q12_latent"].as_array().expect("q12_latent array");
            assert_eq!(
                latent_arr.len(),
                256,
                "q12_latent dimension must be exactly 256"
            );

            // Verify strict bounds [-30000, 30000] and no NaN/Inf
            let mut non_zero_count = 0;
            let mut coords = Vec::with_capacity(256);
            for (dim, item) in latent_arr.iter().enumerate() {
                let val = item.as_i64().expect("integer latent coordinate");
                assert!(
                    (-30000..=30000).contains(&val),
                    "Turn {turn} dim {dim} out of bounds: {val}"
                );
                if val != 0 {
                    non_zero_count += 1;
                }
                coords.push(val);
            }
            assert!(
                non_zero_count > 0,
                "Turn {turn} q12_latent must evolve with non-zero coordinates"
            );

            let summary = ctx["context_summary"]
                .as_str()
                .expect("context_summary str");
            assert!(
                summary.contains(&format!("T{turn}:")),
                "Summary must contain T{turn}: entry"
            );

            turn_records.push((resp.id, turn_index, coords, summary.to_string()));
            context_opt = Some(ctx);
        }

        turn_records
    };

    // Run 1: Verify 50-turn evolution
    let records1 = run_conversation();
    assert_eq!(records1.len(), 50);

    // Verify latent coordinates changed between turn 1 and turn 50 (dynamic evolution)
    let coords_t1 = &records1[0].2;
    let coords_t50 = &records1[49].2;
    let mut delta_sum: i64 = 0;
    for d in 0..256 {
        delta_sum += (coords_t1[d] - coords_t50[d]).abs();
    }
    assert!(
        delta_sum > 1000,
        "Latent state must significantly evolve over 50 turns, delta_sum={delta_sum}"
    );

    // Run 2: Verify bitwise determinism across all 50 turns
    let records2 = run_conversation();
    for i in 0..50 {
        assert_eq!(
            records1[i].0,
            records2[i].0,
            "Turn {} response ID must be bitwise deterministic",
            i + 1
        );
        assert_eq!(
            records1[i].1,
            records2[i].1,
            "Turn {} turn_index must be bitwise deterministic",
            i + 1
        );
        assert_eq!(
            records1[i].2,
            records2[i].2,
            "Turn {} q12_latent must be bitwise deterministic",
            i + 1
        );
        assert_eq!(
            records1[i].3,
            records2[i].3,
            "Turn {} summary must be bitwise deterministic",
            i + 1
        );
    }
}

#[test]
fn challenge_2_massive_multiturn_50_turns_chat_completions_state_carry_forward() {
    let mut context_opt: Option<serde_json::Value> = None;

    for turn in 1..=50 {
        let prompt =
            format!("OpenAI Turn {turn}: describe the Clifford Cl(0,6) router layer {turn}");
        let req = serde_json::json!({
            "model": "uor-r4.lut-stack/8",
            "messages": [{"role": "user", "content": prompt}],
            "context": context_opt,
        });

        let (status, resp_str) = handle_v1_chat_completions_payload(&req.to_string())
            .expect("chat completions turn should succeed");
        assert_eq!(status, 200);

        let resp: OpenAiChatCompletionResponse =
            serde_json::from_str(&resp_str).expect("valid OpenAiChatCompletionResponse");
        assert_eq!(resp.object, "chat.completion");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(resp.choices[0].finish_reason, "stop");
        assert!(resp.choices[0].message.content.is_some());

        let ctx = resp.context.expect("response must carry updated context");
        assert_eq!(ctx["turn_index"].as_u64(), Some(turn as u64));
        assert_eq!(ctx["q12_latent"].as_array().map(|a| a.len()), Some(256));

        context_opt = Some(ctx);
    }
}

#[test]
fn challenge_3_context_corrupted_latent_and_bounds_recovery() {
    // Subtest A: Under-length latent vector (10 elements instead of 256)
    let bad_ctx_1 = serde_json::json!({
        "turn_index": 5,
        "q12_latent": [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000],
    });
    let req1 = serde_json::json!({
        "messages": [{"role": "user", "content": "Hello"}],
        "context": bad_ctx_1,
    });
    let (s1, r1) = handle_v1_messages_payload(&req1.to_string()).expect("should recover safely");
    assert_eq!(s1, 200);
    let val1: serde_json::Value = serde_json::from_str(&r1).unwrap();
    assert_eq!(val1["context"]["turn_index"], 6);
    assert_eq!(val1["context"]["q12_latent"].as_array().unwrap().len(), 256);

    // Subtest B: Over-length latent vector (500 elements)
    let bad_ctx_2 = serde_json::json!({
        "turn_index": 10,
        "q12_latent": vec![42; 500],
    });
    let req2 = serde_json::json!({
        "messages": [{"role": "user", "content": "Hello"}],
        "context": bad_ctx_2,
    });
    let (s2, r2) = handle_v1_messages_payload(&req2.to_string()).expect("should recover safely");
    assert_eq!(s2, 200);
    let val2: serde_json::Value = serde_json::from_str(&r2).unwrap();
    assert_eq!(val2["context"]["turn_index"], 11);
    assert_eq!(val2["context"]["q12_latent"].as_array().unwrap().len(), 256);

    // Subtest C: Extreme values [32767; 256] (i16::MAX) and [-32768; 256] (i16::MIN)
    let extreme_ctx_pos = serde_json::json!({
        "turn_index": 1,
        "q12_latent": vec![32767i64; 256],
    });
    let req3 = serde_json::json!({
        "messages": [{"role": "user", "content": "Extreme positive"}],
        "context": extreme_ctx_pos,
    });
    let (s3, r3) = handle_v1_messages_payload(&req3.to_string()).expect("no overflow");
    assert_eq!(s3, 200);
    let val3: serde_json::Value = serde_json::from_str(&r3).unwrap();
    for coord in val3["context"]["q12_latent"].as_array().unwrap() {
        let c = coord.as_i64().unwrap();
        assert!(
            (-30000..=30000).contains(&c),
            "Must clamp within bounds: {c}"
        );
    }

    let extreme_ctx_neg = serde_json::json!({
        "turn_index": 2,
        "q12_latent": vec![-32768i64; 256],
    });
    let req4 = serde_json::json!({
        "messages": [{"role": "user", "content": "Extreme negative"}],
        "context": extreme_ctx_neg,
    });
    let (s4, r4) = handle_v1_messages_payload(&req4.to_string()).expect("no underflow");
    assert_eq!(s4, 200);
    let val4: serde_json::Value = serde_json::from_str(&r4).unwrap();
    for coord in val4["context"]["q12_latent"].as_array().unwrap() {
        let c = coord.as_i64().unwrap();
        assert!(
            (-30000..=30000).contains(&c),
            "Must clamp within bounds: {c}"
        );
    }
}

// ============================================================================
// 2. Adversarial Thinking Tag Stress & Fuzzing
// ============================================================================

#[test]
fn challenge_4_thinking_unclosed_and_nested_tags_fuzzing() {
    let test_cases = [
        // 1. Unclosed <thinking> tag
        (
            "<thinking>Unclosed reasoning trace without closing tag\nSome conclusion here.",
            None, // should not panic; tag remains unclosed so thinking is None and text has the rest
            false,
        ),
        // 2. Unclosed <thought> tag
        (
            "<thought>Unclosed thought block without closing tag",
            None,
            false,
        ),
        // 3. Nested thinking tags: <thought> containing <thinking>
        (
            "<thought>Outer reasoning\n<thinking>Inner reasoning</thinking>\nOuter wrapup</thought>\nFinal reply.",
            Some("Inner reasoning\n\nOuter reasoning\n\nOuter wrapup"),
            true,
        ),
        // 4. Empty thinking tag
        (
            "<thinking></thinking>\nDirect answer.",
            None,
            true,
        ),
        // 5. Whitespace-only thinking tag
        (
            "<thought>   \t\n  \r\n </thought>\nDirect answer.",
            None,
            true,
        ),
        // 6. Inverted / reversed tags: orphaned closing tag treated as raw text
        (
            "</thinking>Reversed tags<thinking>\nProse.",
            None,
            false,
        ),
        // 7. Multiple sequential thinking blocks
        (
            "<thinking>Block 1</thinking>\nIntermediate text\n<thought>Block 2</thought>\nFinal text.",
            Some("Block 1\n\nBlock 2"),
            true,
        ),
        // 8. Math expressions with angle brackets (< 5 and > 2)
        (
            "For all x in R, if x < 5 and x > 2, then x in (2, 5).",
            None,
            true,
        ),
        // 9. Special characters inside thinking
        (
            "<thinking>Special chars: < > & \" ' \\ / { } [ ] $ # @ ! \n \r \t</thinking>\nAnswer.",
            Some("Special chars: < > & \" ' \\ / { } [ ] $ # @ !"),
            true,
        ),
    ];

    for (raw, expected_th, check_clean_no_tags) in test_cases {
        let parsed = parse_assistant_turn(raw);
        if let Some(exp) = expected_th {
            assert!(
                parsed.thinking.is_some(),
                "Expected thinking for raw:\n{raw}"
            );
            assert_eq!(
                parsed.thinking.as_deref().unwrap().trim(),
                exp.trim(),
                "Thinking mismatch for raw:\n{raw}"
            );
        }
        if check_clean_no_tags {
            if let Some(ref clean) = parsed.clean_text {
                assert!(
                    !clean.contains("</thinking>"),
                    "clean_text must not contain leaked closing tags: {clean}"
                );
                assert!(
                    !clean.contains("</thought>"),
                    "clean_text must not contain leaked closing tags: {clean}"
                );
            }
        }
    }
}

#[test]
fn challenge_thinking_tags_in_full_message_turn_separation() {
    let req = serde_json::json!({
        "messages": [{"role": "user", "content": "What is Wilson's Theorem?"}]
    });

    let (s, r) = handle_v1_messages_payload(&req.to_string()).expect("messages payload");
    assert_eq!(s, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&r).unwrap();

    let mut found_thinking = false;
    for b in &resp.content {
        match b {
            crate::engine::api::AnthropicContentBlock::Thinking { thinking, .. } => {
                found_thinking = true;
                assert!(thinking.contains("Wilson"));
            }
            crate::engine::api::AnthropicContentBlock::Text { text } => {
                assert!(
                    !text.contains("<thinking>") && !text.contains("</thinking>"),
                    "text content must not contain thinking XML tags: {text}"
                );
            }
            _ => {}
        }
    }
    assert!(
        found_thinking,
        "Must separate thinking into AnthropicContentBlock::Thinking"
    );
}

// ============================================================================
// 3. Malformed Tool Calls & Tool Observation Handling
// ============================================================================

#[test]
fn challenge_5_tool_call_malformed_json_and_adversarial_arguments() {
    let test_cases = [
        // Corrupted JSON inside tool_call
        "<tool_call>{invalid: json,}</tool_call>\nFallback prose.",
        // Missing name field
        "<tool_call>{\"arguments\": {\"a\": 1}}</tool_call>\nFallback prose.",
        // Missing arguments field
        "<tool_call>{\"name\": \"run_cmd\"}</tool_call>\nFallback prose.",
        // String arguments
        "<tool_call>{\"name\": \"run_cmd\", \"arguments\": \"echo hello\"}</tool_call>",
        // Array arguments
        "<tool_call>{\"name\": \"batch\", \"arguments\": [1, 2, 3]}</tool_call>",
        // Empty tool call
        "<tool_call></tool_call>\nFallback prose.",
        // Unclosed tool call
        "<tool_call>{\"name\": \"unclosed\"\nSome text.",
        // Non-object JSON
        "<tool_call>12345</tool_call>\nFallback prose.",
    ];

    for raw in test_cases {
        let parsed = parse_assistant_turn(raw);
        // None of these should panic
        for tc in &parsed.tool_calls {
            assert!(!tc.id.is_empty(), "Tool call must have generated ID");
            assert!(!tc.name.is_empty(), "Tool call must have name");
        }
    }
}

#[test]
fn challenge_6_tool_observation_error_and_polymorphic_content() {
    // Anthropic tool result with is_error: true
    let req_err = serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_err_01",
                        "content": "FileNotFoundError: [Errno 2] No such file or directory: 'missing.rs'",
                        "is_error": true
                    }
                ]
            }
        ]
    });
    let (s1, r1) = handle_v1_messages_payload(&req_err.to_string()).expect("handle tool error");
    assert_eq!(s1, 200);
    let resp1: AnthropicMessagesResponse = serde_json::from_str(&r1).unwrap();
    assert_eq!(resp1.stop_reason, "end_turn");

    // Anthropic tool result with structured JSON object in content
    let req_obj = serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_obj_02",
                        "content": {
                            "status": "success",
                            "lines": 42,
                            "sha256": "abcdef123456"
                        }
                    }
                ]
            }
        ]
    });
    let (s2, _r2) = handle_v1_messages_payload(&req_obj.to_string()).expect("handle tool json obj");
    assert_eq!(s2, 200);

    // OpenAI format tool observation
    let req_openai_tool = serde_json::json!({
        "messages": [
            {
                "role": "tool",
                "tool_call_id": "call_oa_01",
                "name": "read_file",
                "content": "pub struct EngineHomeostasis;"
            }
        ]
    });
    let (s3, r3) = handle_v1_chat_completions_payload(&req_openai_tool.to_string())
        .expect("handle openai tool");
    assert_eq!(s3, 200);
    let resp3: OpenAiChatCompletionResponse = serde_json::from_str(&r3).unwrap();
    assert_eq!(resp3.choices[0].finish_reason, "stop");
}

#[test]
fn challenge_7_tool_declaration_triggers_read_file_tool_use() {
    let req_anthropic = serde_json::json!({
        "messages": [{"role": "user", "content": "Please inspect src/engine.rs"}],
        "tools": [
            {
                "name": "read_file",
                "description": "Read file contents from filesystem",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }
            }
        ]
    });

    let (s1, r1) =
        handle_v1_messages_payload(&req_anthropic.to_string()).expect("tool call trigger");
    assert_eq!(s1, 200);
    let resp1: AnthropicMessagesResponse = serde_json::from_str(&r1).unwrap();
    assert_eq!(resp1.stop_reason, "tool_use");

    let tool_block = resp1
        .content
        .iter()
        .find(|b| matches!(b, crate::engine::api::AnthropicContentBlock::ToolUse { .. }))
        .expect("must contain ToolUse content block");

    if let crate::engine::api::AnthropicContentBlock::ToolUse { name, input, id } = tool_block {
        assert_eq!(name, "read_file");
        assert!(id.starts_with("call_"));
        assert_eq!(input["path"], "src/engine.rs");
    }

    // OpenAI format tool trigger
    let req_openai = serde_json::json!({
        "messages": [{"role": "user", "content": "Please read src/lib.rs"}],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "parameters": {"type": "object"}
                }
            }
        ]
    });

    let (s2, r2) = handle_v1_chat_completions_payload(&req_openai.to_string())
        .expect("openai tool call trigger");
    assert_eq!(s2, 200);
    let resp2: OpenAiChatCompletionResponse = serde_json::from_str(&r2).unwrap();
    assert_eq!(resp2.choices[0].finish_reason, "tool_calls");
    let tc = resp2.choices[0].message.tool_calls.as_ref().unwrap();
    assert_eq!(tc.len(), 1);
    assert_eq!(tc[0].function.name, "read_file");
}

// ============================================================================
// 4. Extreme & Invalid Payloads (Graceful 400 Responses, Zero Panics)
// ============================================================================

#[test]
fn challenge_8_invalid_payloads_graceful_400_responses() {
    // Universal invalid payloads that must return 400 for BOTH Anthropic and OpenAI
    let universal_invalid_payloads = [
        "",                                                                             // empty string
        "{}",                                      // empty object (missing messages)
        "not json at all",                         // non-json
        "{\"messages\": []}",                      // empty messages array
        "{\"messages\": \"not array\"}",           // messages wrong type
        "{\"messages\": [null]}",                  // null message item
        "{\"messages\": [123]}",                   // integer message item
        "{\"messages\": [{\"content\": \"hi\"}]}", // missing role field
        "{\"messages\": [{\"role\": 12, \"content\": \"hi\"}]}", // role wrong type
        "{\"model\": 9999, \"messages\": [{\"role\": \"user\", \"content\": \"hi\"}]}", // model wrong type
        "{\"tools\": \"invalid\", \"messages\": [{\"role\": \"user\", \"content\": \"hi\"}]}", // tools wrong type
        "{\"temperature\": \"invalid\", \"messages\": [{\"role\": \"user\", \"content\": \"hi\"}]}", // temperature wrong type
    ];

    for (i, payload) in universal_invalid_payloads.iter().enumerate() {
        // Test against Anthropic handler
        let res_anthropic = handle_v1_messages_payload(payload);
        assert!(
            res_anthropic.is_err(),
            "Payload {i} must return Err(400) for Anthropic: {payload}"
        );
        let (status_a, err_a) = res_anthropic.unwrap_err();
        assert_eq!(status_a, 400, "Must be 400 Bad Request: {err_a}");
        assert!(
            err_a.contains("invalid_request_error"),
            "Error body must contain invalid_request_error: {err_a}"
        );

        // Test against OpenAI handler
        let res_openai = handle_v1_chat_completions_payload(payload);
        assert!(
            res_openai.is_err(),
            "Payload {i} must return Err(400) for OpenAI: {payload}"
        );
        let (status_o, err_o) = res_openai.unwrap_err();
        assert_eq!(status_o, 400, "Must be 400 Bad Request: {err_o}");
        assert!(
            err_o.contains("invalid_request_error"),
            "Error body must contain invalid_request_error: {err_o}"
        );
    }

    // Anthropic-specific validation: missing content field on a message
    // In Anthropic Messages API, content is required and cannot be omitted.
    let anthropic_missing_content = "{\"messages\": [{\"role\": \"user\"}]}";
    let res_a = handle_v1_messages_payload(anthropic_missing_content);
    assert!(
        res_a.is_err(),
        "Anthropic must reject message with missing content"
    );
    assert_eq!(res_a.unwrap_err().0, 400);

    // In OpenAI Chat Completions API, content is Option<String> (to allow tool_calls assistant turns).
    // An empty content user message parses cleanly without panic:
    let res_o = handle_v1_chat_completions_payload(anthropic_missing_content);
    assert!(
        res_o.is_ok(),
        "OpenAI allows Option<String> content without panic"
    );

    // In Anthropic Messages API, empty blocks array `content: []` is deserialized as `AnthropicContent::Blocks([])`
    // and produces an empty prompt which resolves to the greeting turn.
    let anthropic_empty_blocks = "{\"messages\": [{\"role\": \"user\", \"content\": []}]}";
    let res_a_blocks = handle_v1_messages_payload(anthropic_empty_blocks);
    assert!(
        res_a_blocks.is_ok(),
        "Anthropic empty blocks array is handled without panic"
    );
}

#[test]
fn challenge_9_extreme_payload_sizes_and_unicode_fuzzing() {
    // 500 KB large prompt payload
    let large_prompt = "Large payload testing. ".repeat(25000); // ~550 KB
    let req_large = serde_json::json!({
        "messages": [{"role": "user", "content": large_prompt}]
    });
    let (s1, r1) = handle_v1_messages_payload(&req_large.to_string()).expect("large payload");
    assert_eq!(s1, 200);
    let resp1: AnthropicMessagesResponse = serde_json::from_str(&r1).unwrap();
    assert!(resp1.usage.input_tokens > 10000);

    // Deep Unicode fuzzing
    let unicode_prompt = "测试汉字 🚀 Crab emoji \u{0000} null byte representation \
                          مرحبا بالعالم \
                          Γεια σας κόσμε \
                          ∀x ∈ ℝ: x² ≥ 0 \
                          Complex math: ∫_{0}^{∞} e^{-x²} dx = √π / 2";
    let req_unicode = serde_json::json!({
        "messages": [{"role": "user", "content": unicode_prompt}]
    });
    let (s2, _r2) = handle_v1_messages_payload(&req_unicode.to_string()).expect("unicode payload");
    assert_eq!(s2, 200);

    // 100 conversation turns in a single request envelope
    let mut turns = Vec::with_capacity(100);
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        turns.push(serde_json::json!({
            "role": role,
            "content": format!("Message turn {i}")
        }));
    }
    let req_100_turns = serde_json::json!({
        "messages": turns
    });
    let (s3, _r3) =
        handle_v1_messages_payload(&req_100_turns.to_string()).expect("100 turns single request");
    assert_eq!(s3, 200);
}

// ============================================================================
// 5. Live TCP Loopback Daemon Fuzzing & Concurrency Hammer
// ============================================================================

#[test]
fn challenge_10_live_daemon_raw_tcp_fuzzing() {
    let daemon = TestServerDaemon::spawn();

    // 1. Immediate socket disconnect (0 bytes)
    {
        let stream = TcpStream::connect(&daemon.addr).expect("connect");
        drop(stream); // close immediately
        thread::sleep(Duration::from_millis(50));
    }

    // 2. Malformed request line
    {
        let mut stream = TcpStream::connect(&daemon.addr).expect("connect");
        let _ = stream.write_all(b"MALFORMED_GARBAGE_REQUEST\r\n\r\n");
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(50));
    }

    // 3. Premature EOF on body (Content-Length: 10000, sends 5 bytes then closes)
    {
        let mut stream = TcpStream::connect(&daemon.addr).expect("connect");
        let _ = stream.write_all(
            b"POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 10000\r\n\r\nshort",
        );
        let _ = stream.flush();
        drop(stream);
        thread::sleep(Duration::from_millis(50));
    }

    // 4. Non-UTF8 byte payload
    {
        let headers = [("Content-Type", "application/json")];
        let non_utf8_body = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01];
        let res = send_raw_http_request(
            &daemon.addr,
            "POST",
            "/v1/messages",
            &headers,
            Some(&non_utf8_body),
        );
        // Server must return 400 gracefully, no panic
        if let Ok((status, _, _)) = res {
            assert_eq!(status, 400);
        }
    }

    // 5. OPTIONS CORS preflight on /v1/messages
    {
        let (status, headers, _) =
            send_raw_http_request(&daemon.addr, "OPTIONS", "/v1/messages", &[], None)
                .expect("OPTIONS request");
        assert_eq!(status, 204);
        assert_eq!(
            headers
                .get("access-control-allow-origin")
                .map(String::as_str),
            Some("*")
        );
    }

    // 6. Unknown endpoint returns 404
    {
        let (status, _, body) =
            send_raw_http_request(&daemon.addr, "GET", "/nonexistent_endpoint", &[], None)
                .expect("GET 404");
        assert_eq!(status, 404);
        assert!(String::from_utf8_lossy(&body).contains("Not Found"));
    }

    // 7. GET /health and GET /feed stay fully functional
    {
        let (s_h, _, b_h) =
            send_raw_http_request(&daemon.addr, "GET", "/health", &[], None).expect("GET health");
        assert_eq!(s_h, 200);
        assert!(String::from_utf8_lossy(&b_h).contains("uor-r4.lut-stack/8"));

        let (s_f, _, b_f) =
            send_raw_http_request(&daemon.addr, "GET", "/feed", &[], None).expect("GET feed");
        assert_eq!(s_f, 200);
        assert!(String::from_utf8_lossy(&b_f).contains("r4/literature/1"));
    }
}

#[test]
fn challenge_11_live_daemon_concurrency_hammer_200_requests() {
    let daemon = Arc::new(TestServerDaemon::spawn());
    let thread_count = 20;
    let requests_per_thread = 10;
    let barrier = Arc::new(Barrier::new(thread_count));

    let mut handles = Vec::new();
    for thread_idx in 0..thread_count {
        let daemon_ref = Arc::clone(&daemon);
        let b = Arc::clone(&barrier);

        let handle = thread::spawn(move || {
            b.wait(); // Synchronize all 20 threads to start hitting daemon at the same instant

            for req_idx in 0..requests_per_thread {
                let mode = (thread_idx + req_idx) % 5;
                match mode {
                    0 => {
                        // Health check
                        let (status, _, body) =
                            send_raw_http_request(&daemon_ref.addr, "GET", "/health", &[], None)
                                .expect("health request must succeed");
                        assert_eq!(status, 200);
                        assert!(String::from_utf8_lossy(&body).contains("uor-r4.lut-stack/8"));
                    }
                    1 => {
                        // Feed check
                        let (status, _, body) =
                            send_raw_http_request(&daemon_ref.addr, "GET", "/feed", &[], None)
                                .expect("feed request must succeed");
                        assert_eq!(status, 200);
                        assert!(String::from_utf8_lossy(&body).contains("r4/literature/1"));
                    }
                    2 => {
                        // Anthropic message
                        let req = serde_json::json!({
                            "messages": [{"role": "user", "content": format!("Concurrent prompt {thread_idx}-{req_idx}")}]
                        });
                        let body = req.to_string();
                        let headers = [("Content-Type", "application/json")];
                        let (status, _, resp_body) = send_raw_http_request(
                            &daemon_ref.addr,
                            "POST",
                            "/v1/messages",
                            &headers,
                            Some(body.as_bytes()),
                        )
                        .expect("messages request must succeed");
                        assert_eq!(status, 200);
                        assert!(String::from_utf8_lossy(&resp_body).contains("assistant"));
                    }
                    3 => {
                        // OpenAI chat completion
                        let req = serde_json::json!({
                            "messages": [{"role": "user", "content": format!("Concurrent completion {thread_idx}-{req_idx}")}]
                        });
                        let body = req.to_string();
                        let headers = [("Content-Type", "application/json")];
                        let (status, _, resp_body) = send_raw_http_request(
                            &daemon_ref.addr,
                            "POST",
                            "/v1/chat/completions",
                            &headers,
                            Some(body.as_bytes()),
                        )
                        .expect("chat completions request must succeed");
                        assert_eq!(status, 200);
                        assert!(String::from_utf8_lossy(&resp_body).contains("chat.completion"));
                    }
                    _ => {
                        // Negative test: empty messages must return 400
                        let req = serde_json::json!({"messages": []});
                        let body = req.to_string();
                        let headers = [("Content-Type", "application/json")];
                        let (status, _, resp_body) = send_raw_http_request(
                            &daemon_ref.addr,
                            "POST",
                            "/v1/messages",
                            &headers,
                            Some(body.as_bytes()),
                        )
                        .expect("400 request must complete");
                        assert_eq!(status, 400);
                        assert!(
                            String::from_utf8_lossy(&resp_body).contains("invalid_request_error")
                        );
                    }
                }
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("thread join must not panic");
    }
}

// ============================================================================
// 6. 12-Point Loopback Regression Gate
// ============================================================================

#[test]
fn challenge_12_loopback_12_point_test_suite_regression_gate() {
    let root = repo_model::repo_root();
    engine::run_api_test_cmd(&root, &[]).expect("12-point loopback test suite must pass cleanly");
}
