//! Empirical Challenge 2 Stress Suite for Milestone 4 Iteration 2
//! Provenance Fallback & API Concurrency Stress Harness
//!
//! Evaluates:
//! 1. Complete scan of bootstrap_v8.uorlut point table rows (including 5289..5313) verifying 0 rows defective.
//! 2. Exhaustive boundary grid of ServedPointKey and writer provenance sentinels (0, u64::MAX, u128::MAX, MEMORY_PAD).
//! 3. High-concurrency socket hammer (20 threads, 200 requests) with interleaved abrupt TCP client drops and partial writes.
//! 4. Cross-lifecycle dynamic compilation chain-of-custody and span link propagation under stress.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;

#[allow(dead_code)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;

use lut_stack_exec::{
    LutStackModel, RollingContextState, ServedPointKey, VERIFIED_FRONTIER_ANCHOR_TAG,
};
use otel::{build_inference_trace, fnv1a64};

const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../../crates/wasm/src/bootstrap_v8.uorlut");
const BOOTSTRAP_TRACE_EXPECTED: u128 = 0x756f722d72342e626f6f747374726170; // "uor-r4.bootstrap"
const BOOTSTRAP_SPAN_EXPECTED: u64 = 0x756f722d72342e38; // "uor-r4.8"

fn load_test_model() -> LutStackModel {
    LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully")
}

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
    _port: u16,
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

        let mut ready = false;
        for _ in 0..100 {
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

        Self {
            child,
            _port: port,
            addr,
        }
    }
}

impl Drop for TestServerDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn send_raw_http_request(
    addr: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    timeout: Duration,
) -> Result<(u16, HashMap<String, String>, String), String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect error: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;

    let body_bytes = body.map(|s| s.as_bytes()).unwrap_or(&[]);
    let req = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body_bytes.len()
    );

    stream
        .write_all(req.as_bytes())
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
    let status_code: u16 = parts[1].parse().map_err(|e| format!("parse status: {e}"))?;

    let mut headers = HashMap::new();
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
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let clen = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let mut resp_body = vec![0u8; clen];
    reader
        .read_exact(&mut resp_body)
        .map_err(|e| format!("read body: {e}"))?;

    Ok((
        status_code,
        headers,
        String::from_utf8_lossy(&resp_body).into_owned(),
    ))
}

// ============================================================================
// Stress Test 1: Complete PointTable Row Scan & Frontier Rows 5289..5313
// ============================================================================

#[test]
fn test_stress_complete_bootstrap_point_table_scan_and_frontier_rows() {
    let model = load_test_model();
    let pt = model.point.as_ref().expect("point table must exist");
    let anchors = pt
        .canonical_anchors
        .as_ref()
        .expect("canonical anchors must exist");

    assert!(
        pt.rows >= 5314,
        "bootstrap_v8 must contain at least 5314 rows, found {}",
        pt.rows
    );

    let mut frontier_rows_found = Vec::new();
    let mut memory_padded_rows = Vec::new();
    let mut defective_rows = Vec::new();

    for r in 0..pt.rows {
        let base = r * pt.anchor_len;
        let mut is_explicit_frontier_prov = false;
        if base + 7 < anchors.len() && anchors[base + 7] == VERIFIED_FRONTIER_ANCHOR_TAG {
            frontier_rows_found.push(r);
            if anchors[base + 1] == 0xFFFF_FFFF && anchors[base + 2] == 0xFFFF_FFFF {
                memory_padded_rows.push(r);
            } else {
                is_explicit_frontier_prov = true;
            }
        }

        let (trace_id, span_id) = pt.get_row_provenance(r);

        // Invariant: No row in bootstrap model may return sentinel MAX or 0
        if trace_id == u128::MAX
            || trace_id == 0
            || span_id == u64::MAX
            || span_id == 0
            || (trace_id != BOOTSTRAP_TRACE_EXPECTED && span_id == BOOTSTRAP_SPAN_EXPECTED)
            || (trace_id == BOOTSTRAP_TRACE_EXPECTED && span_id != BOOTSTRAP_SPAN_EXPECTED)
        {
            defective_rows.push((r, trace_id, span_id));
        }

        // Bootstrap and memory-padded legacy frontier rows must return exact bootstrap provenance
        if !is_explicit_frontier_prov {
            assert_eq!(
                trace_id, BOOTSTRAP_TRACE_EXPECTED,
                "Row {r} returned non-bootstrap trace {trace_id:#034x}"
            );
            assert_eq!(
                span_id, BOOTSTRAP_SPAN_EXPECTED,
                "Row {r} returned non-bootstrap span {span_id:#018x}"
            );
        }
    }

    // Verify all memory-padded frontier rows form a contiguous block starting at 5289
    let expected_range: Vec<usize> = (5289..(5289 + memory_padded_rows.len())).collect();
    assert_eq!(
        memory_padded_rows, expected_range,
        "Memory-padded frontier rows must form contiguous block starting at 5289"
    );
    assert!(
        memory_padded_rows.len() >= 25,
        "Must identify at least 25 memory-padded preexisting frontier rows"
    );

    assert!(
        defective_rows.is_empty(),
        "Defective rows found in bootstrap model: {defective_rows:?}"
    );
}

// ============================================================================
// Stress Test 2: Exhaustive Sentinel Combinations Grid
// ============================================================================

#[test]
fn test_stress_sentinel_grid_point_and_writer_provenance() {
    let mut pt = ServedPointKey {
        rows: 4,
        width: 16,
        exp: 0,
        vote_k: 1,
        points: vec![0i16; 64],
        targets: vec![1, 2, 3, 4],
        canonical_anchors: None,
        anchor_len: 32,
        provenance_spans: None,
        provenance_traces: None,
        bucket_offsets: None,
        bucket_row_indices: None,
    };

    let sentinel_spans: [u64; 6] = [
        0,
        1,
        0x1234_5678_9abc_def0,
        u64::MAX / 2,
        u64::MAX - 1,
        u64::MAX,
    ];

    let sentinel_traces: [u128; 6] = [
        0,
        1,
        0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00,
        u128::MAX / 2,
        u128::MAX - 1,
        u128::MAX,
    ];

    let tags = [
        0u32,
        1u32,
        VERIFIED_FRONTIER_ANCHOR_TAG,
        0xFFFF_FFFFu32,
        0xdead_beefu32,
    ];

    // Grid over all combinations
    for &span in &sentinel_spans {
        for &trace in &sentinel_traces {
            for &tag in &tags {
                let mut anchor_row = vec![0u32; 32];
                anchor_row[0] = 0x8000_0001;
                anchor_row[1] = (span & 0xFFFF_FFFF) as u32;
                anchor_row[2] = ((span >> 32) & 0xFFFF_FFFF) as u32;
                anchor_row[3] = (trace & 0xFFFF_FFFF) as u32;
                anchor_row[4] = ((trace >> 32) & 0xFFFF_FFFF) as u32;
                anchor_row[5] = ((trace >> 64) & 0xFFFF_FFFF) as u32;
                anchor_row[6] = ((trace >> 96) & 0xFFFF_FFFF) as u32;
                anchor_row[7] = tag;

                pt.canonical_anchors = Some(anchor_row);

                let (got_trace, got_span) = pt.get_row_provenance(0);

                let is_valid_dynamic = tag == VERIFIED_FRONTIER_ANCHOR_TAG
                    && span != 0
                    && span != u64::MAX
                    && trace != 0
                    && trace != u128::MAX;

                if is_valid_dynamic {
                    assert_eq!(
                        got_span, span,
                        "Dynamic frontier span mismatch for span={span:#018x}"
                    );
                    assert_eq!(
                        got_trace, trace,
                        "Dynamic frontier trace mismatch for trace={trace:#034x}"
                    );
                } else {
                    assert_eq!(
                        got_span, BOOTSTRAP_SPAN_EXPECTED,
                        "Sentinel fallback span failed for span={span:#018x}, trace={trace:#034x}, tag={tag:#010x}"
                    );
                    assert_eq!(
                        got_trace, BOOTSTRAP_TRACE_EXPECTED,
                        "Sentinel fallback trace failed for span={span:#018x}, trace={trace:#034x}, tag={tag:#010x}"
                    );
                }
            }
        }
    }

    // Also test writer provenance sentinel grid
    let mut model = load_test_model();
    for &span in &sentinel_spans {
        for &trace in &sentinel_traces {
            model.writer_provenance = Some(vec![span; 4]);
            model.writer_provenance_traces = Some(vec![trace; 4]);

            let (w_trace, w_span) = model.get_writer_provenance(0);
            let is_valid_writer = span != 0 && span != u64::MAX && trace != 0 && trace != u128::MAX;

            if is_valid_writer {
                assert_eq!(w_span, span);
                assert_eq!(w_trace, trace);
            } else {
                assert_eq!(w_span, BOOTSTRAP_SPAN_EXPECTED);
                assert_eq!(w_trace, BOOTSTRAP_TRACE_EXPECTED);
            }
        }
    }
}

// ============================================================================
// Stress Test 3: High-Concurrency Socket Hammer & Abrupt Drops
// ============================================================================

#[test]
fn test_stress_api_concurrency_hammer_and_client_disconnects() {
    let daemon = Arc::new(TestServerDaemon::spawn());
    let addr = daemon.addr.clone();

    let total_workers = 20;
    let requests_per_worker = 10;
    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(total_workers);

    let start_time = Instant::now();

    for w in 0..total_workers {
        let addr_clone = addr.clone();
        let success_clone = Arc::clone(&success_count);
        let h = thread::spawn(move || {
            for r in 0..requests_per_worker {
                let turn = w * requests_per_worker + r;
                let outcome = match turn % 4 {
                    0 => send_raw_http_request(
                        &addr_clone,
                        "GET",
                        "/v1/traces",
                        None,
                        Duration::from_secs(10),
                    ),
                    1 => send_raw_http_request(
                        &addr_clone,
                        "POST",
                        "/v1/traces",
                        Some(r#"{"format":"compact","limit":5}"#),
                        Duration::from_secs(10),
                    ),
                    2 => {
                        let payload = format!(
                            r#"{{"messages":[{{"role":"user","content":"concurrency stress w{w} r{r}"}}],"otel_trace":true}}"#
                        );
                        send_raw_http_request(
                            &addr_clone,
                            "POST",
                            "/v1/messages",
                            Some(&payload),
                            Duration::from_secs(10),
                        )
                    }
                    _ => {
                        let payload = format!(
                            r#"{{"messages":[{{"role":"user","content":"chat stress w{w} r{r}"}}],"otel_trace":true}}"#
                        );
                        send_raw_http_request(
                            &addr_clone,
                            "POST",
                            "/v1/chat/completions",
                            Some(&payload),
                            Duration::from_secs(10),
                        )
                    }
                };

                match outcome {
                    Ok((status, _headers, body)) => {
                        assert_eq!(
                            status, 200,
                            "Worker {w} request {r} failed with status {status}: {body}"
                        );
                        assert!(!body.is_empty());
                        success_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        panic!("Worker {w} request {r} network error: {e}");
                    }
                }
            }
        });
        handles.push(h);
    }

    for h in handles {
        h.join().expect("worker thread joined successfully");
    }

    let elapsed = start_time.elapsed();
    let total_success = success_count.load(Ordering::Relaxed);
    assert_eq!(
        total_success,
        total_workers * requests_per_worker,
        "All 200 concurrent requests must succeed"
    );

    // Abrupt client disconnect stress (25 rapid connections closed mid-transmission)
    for i in 0..25 {
        if let Ok(mut s) = TcpStream::connect(&addr) {
            let partial_req: &[u8] = match i % 3 {
                0 => b"POST /v1/messages HTTP/1.1\r\nContent-Length: 500\r\n\r\n{\"mess",
                1 => b"GET /v1/tra",
                _ => b"POST /v1/chat/completions HTTP/1.1\r\nHost: test\r\n\r\n",
            };
            let _ = s.write_all(partial_req);
            let _ = s.flush();
            // Drop socket abruptly
        }
    }

    // Health check after disconnect assault
    let (status, _headers, body) =
        send_raw_http_request(&addr, "GET", "/v1/traces", None, Duration::from_secs(10))
            .expect("health check post-assault");
    assert_eq!(status, 200);
    assert!(body.contains("resourceSpans"));

    eprintln!(
        "High-concurrency hammer passed: 200 requests in {:?} + 25 abrupt drops survived cleanly.",
        elapsed
    );
}

// ============================================================================
// Stress Test 4: Dynamic Compilation & Cross-Lifecycle Span Links
// ============================================================================

#[test]
fn test_stress_dynamic_compilation_and_span_links() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().map(|p| p.rows).unwrap_or(0);

    let dialogue_pairs = [
        (
            "State Gödel's First Incompleteness Theorem.",
            "Any consistent formal system capable of arithmetic is incomplete.",
        ),
        (
            "What is Cauchy-Schwarz inequality?",
            "|<u, v>|^2 <= <u, u> * <v, v> for all vectors u, v in an inner product space.",
        ),
        (
            "Explain the Lie algebra E8.",
            "E8 is the largest exceptional simple Lie algebra of dimension 248 and rank 8.",
        ),
    ];

    for (idx, &(prompt, reply)) in dialogue_pairs.iter().enumerate() {
        let compiled = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("compile dialogue turn");
        assert!(compiled);

        let pt = model.point.as_ref().expect("point table");
        assert_eq!(pt.rows, initial_rows + idx + 1);

        // Compute expected deterministic IDs
        let prompt_clean = prompt.trim();
        let reply_clean = reply.trim();
        let expected_span =
            fnv1a64(format!("gllm.training.compile_turn:{prompt_clean}:{reply_clean}").as_bytes())
                .max(1);
        let expected_trace_lo =
            fnv1a64(format!("gllm.training.trace:{prompt_clean}").as_bytes()).max(1);
        let expected_trace_hi = fnv1a64(b"uor.training.hi").max(1);
        let expected_trace = (expected_trace_lo as u128) | ((expected_trace_hi as u128) << 64);

        let mut found = false;
        for r in 0..pt.rows {
            let (tr, sp) = pt.get_row_provenance(r);
            if sp == expected_span {
                assert_eq!(tr, expected_trace);
                found = true;
                break;
            }
        }
        assert!(
            found,
            "Newly compiled row for turn {idx} must be retrievable by deterministic provenance"
        );
    }

    // Verify inference trace connects to newly compiled turn
    let model_arc = Arc::new(model);
    let mut context = RollingContextState::default();
    let prompt = "State Gödel's First Incompleteness Theorem.";
    let ctx_before = context.q12_latent;
    let gen = model_arc.generate_reply_with_context(&mut context, &[], prompt, 16);
    let ctx_after = context.q12_latent;

    let trace = build_inference_trace(1, prompt, &ctx_before, &ctx_after, &gen, &model_arc);
    let routing_span = trace
        .spans
        .iter()
        .find(|s| trace.intern_table.get(s.name_ref) == "gllm.routing.point_key")
        .expect("gllm.routing.point_key span");

    assert_eq!(routing_span.links.len(), 1);
    let link = &routing_span.links[0];
    let link_trace = trace.intern_table.get(link.trace_id_ref);
    let link_span = trace.intern_table.get(link.span_id_ref);

    assert_eq!(link_trace.len(), 32);
    assert_eq!(link_span.len(), 16);
}
