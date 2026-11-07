//! Empirical Challenge Test Suite for Milestone 4:
//! Cross-Lifecycle Provenance Chain of Custody & API Concurrency Hammer.
//!
//! Enforces:
//! 1. ServedPointKey bit-exact encoding in words 1..6, VERIFIED_FRONTIER_ANCHOR_TAG in word 7.
//! 2. Backward-compatible fallback for bootstrap rows and resilience against corrupt/truncated anchors.
//! 3. Dynamic dialogue compilation (`compile_verified_dialogue_turn`) stamping authentic provenance across turns.
//! 4. SpanLinks in `gllm.routing.point_key` and `gllm.decode.step` linking to training and consolidation spans.
//! 5. High-concurrency socket stability, non-blocking behavior, and connection bounds under mixed load on live server daemon.

use std::collections::{HashMap, HashSet};
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
// Challenge 1: Bit-Exact Encoding in Words 1..6 and Word 7 Tag
// ============================================================================

#[test]
fn test_challenge_1_served_point_key_words_1_to_6_and_tag_word_7_bit_integrity() {
    let mut pt = ServedPointKey {
        rows: 1,
        width: 16,
        exp: 0,
        vote_k: 1,
        points: vec![0i16; 16],
        targets: vec![100],
        canonical_anchors: None,
        anchor_len: 32,
        provenance_spans: None,
        provenance_traces: None,
        bucket_offsets: None,
        bucket_row_indices: None,
    };

    // 1. Test bit-exact reconstruction of 64-bit span_id and 128-bit trace_id
    let test_cases = [
        (
            0x0123_4567_89ab_cdef_u64,
            0x0011_2233_4455_6677_8899_aabb_ccdd_eeff_u128,
        ),
        (
            0xffff_ffff_0000_0001_u64,
            0x8000_0000_0000_0000_0000_0000_0000_0001_u128,
        ),
        (
            0x5555_5555_aaaa_aaaa_u64,
            0xaaaa_aaaa_5555_5555_3333_3333_cccc_cccc_u128,
        ),
        (1u64, 1u128),
        (u64::MAX - 1, u128::MAX - 1),
    ];

    for &(span_val, trace_val) in &test_cases {
        let mut anchor_row = vec![0u32; 32];
        anchor_row[0] = 0x8000_0001; // turn_id
        anchor_row[1] = (span_val & 0xFFFF_FFFF) as u32;
        anchor_row[2] = ((span_val >> 32) & 0xFFFF_FFFF) as u32;
        anchor_row[3] = (trace_val & 0xFFFF_FFFF) as u32;
        anchor_row[4] = ((trace_val >> 32) & 0xFFFF_FFFF) as u32;
        anchor_row[5] = ((trace_val >> 64) & 0xFFFF_FFFF) as u32;
        anchor_row[6] = ((trace_val >> 96) & 0xFFFF_FFFF) as u32;
        anchor_row[7] = VERIFIED_FRONTIER_ANCHOR_TAG;

        pt.canonical_anchors = Some(anchor_row);

        let (got_trace, got_span) = pt.get_row_provenance(0);
        assert_eq!(
            got_span, span_val,
            "span_id reconstruction failed for {span_val:#018x}"
        );
        assert_eq!(
            got_trace, trace_val,
            "trace_id reconstruction failed for {trace_val:#034x}"
        );
    }

    // 2. Fuzz with 1,000 pseudorandom bit patterns
    let mut state = 0xdead_beef_cafe_babe_u64;
    let mut splitmix64 = || {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };

    for _ in 0..1000 {
        let span_rand = splitmix64().clamp(1, u64::MAX - 1);
        let t_lo = splitmix64().max(1);
        let t_hi = splitmix64().max(1);
        let trace_rand = (t_lo as u128) | ((t_hi as u128) << 64);

        let mut row = vec![0u32; 32];
        row[1] = (span_rand & 0xFFFF_FFFF) as u32;
        row[2] = ((span_rand >> 32) & 0xFFFF_FFFF) as u32;
        row[3] = (trace_rand & 0xFFFF_FFFF) as u32;
        row[4] = ((trace_rand >> 32) & 0xFFFF_FFFF) as u32;
        row[5] = ((trace_rand >> 64) & 0xFFFF_FFFF) as u32;
        row[6] = ((trace_rand >> 96) & 0xFFFF_FFFF) as u32;
        row[7] = VERIFIED_FRONTIER_ANCHOR_TAG;

        pt.canonical_anchors = Some(row);

        let (got_t, got_s) = pt.get_row_provenance(0);
        assert_eq!(got_s, span_rand);
        assert_eq!(got_t, trace_rand);
    }
}

// ============================================================================
// Challenge 2: Backward Compatibility Fallback and Malformed Robustness
// ============================================================================

#[test]
fn test_challenge_2_bootstrap_fallback_and_boundary_robustness() {
    let boot_trace_expected = 0x756f722d72342e626f6f747374726170_u128; // "uor-r4.bootstrap"
    let boot_span_expected = 0x756f722d72342e38_u64; // "uor-r4.8"

    let mut pt = ServedPointKey {
        rows: 2,
        width: 16,
        exp: 0,
        vote_k: 1,
        points: vec![0i16; 32],
        targets: vec![10, 20],
        canonical_anchors: None,
        anchor_len: 32,
        provenance_spans: None,
        provenance_traces: None,
        bucket_offsets: None,
        bucket_row_indices: None,
    };

    // Case 1: canonical_anchors is None -> fallback
    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected)
    );

    // Case 2: Row out of bounds -> fallback without panic
    assert_eq!(
        pt.get_row_provenance(100),
        (boot_trace_expected, boot_span_expected)
    );
    assert_eq!(
        pt.get_row_provenance(usize::MAX),
        (boot_trace_expected, boot_span_expected)
    );

    // Case 3: anchor_len < 8 -> fallback
    for bad_alen in [0, 1, 4, 7] {
        pt.anchor_len = bad_alen;
        pt.canonical_anchors = Some(vec![0u32; 64]);
        assert_eq!(
            pt.get_row_provenance(0),
            (boot_trace_expected, boot_span_expected),
            "anchor_len={bad_alen} must gracefully fall back"
        );
    }

    // Case 4: anchor_len = 32, but tag is NOT VERIFIED_FRONTIER_ANCHOR_TAG
    pt.anchor_len = 32;
    let mut anchors = vec![0xFFFF_FFFFu32; 64];
    anchors[7] = 0; // zero tag
    anchors[39] = 0x1234_5678; // non-matching tag
    pt.canonical_anchors = Some(anchors);

    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected)
    );
    assert_eq!(
        pt.get_row_provenance(1),
        (boot_trace_expected, boot_span_expected)
    );

    // Case 5: Tag matches, but span_id == 0 -> fallback
    let mut anchors = vec![0u32; 64];
    anchors[7] = VERIFIED_FRONTIER_ANCHOR_TAG;
    anchors[1] = 0; // s_lo = 0
    anchors[2] = 0; // s_hi = 0
    anchors[3] = 42; // non-zero trace
    pt.canonical_anchors = Some(anchors);
    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected),
        "Zero span_id must fall back to bootstrap"
    );

    // Case 6: Tag matches, but trace_id == 0 -> fallback
    let mut anchors = vec![0u32; 64];
    anchors[7] = VERIFIED_FRONTIER_ANCHOR_TAG;
    anchors[1] = 42; // non-zero span
    anchors[3] = 0;
    anchors[4] = 0;
    anchors[5] = 0;
    anchors[6] = 0; // all trace words zero
    pt.canonical_anchors = Some(anchors);
    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected),
        "Zero trace_id must fall back to bootstrap"
    );

    // Case 7: Truncated canonical_anchors vector (shorter than row * anchor_len)
    pt.canonical_anchors = Some(vec![VERIFIED_FRONTIER_ANCHOR_TAG; 10]); // Only 10 u32s for 2 rows of 32
    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected)
    );
    assert_eq!(
        pt.get_row_provenance(1),
        (boot_trace_expected, boot_span_expected)
    );

    // Case 8: Tag matches, but words 1..2 are MEMORY_PAD (0xFFFF_FFFF) -> fallback
    let mut anchors = vec![0xFFFF_FFFFu32; 64];
    anchors[7] = VERIFIED_FRONTIER_ANCHOR_TAG;
    pt.canonical_anchors = Some(anchors);
    assert_eq!(
        pt.get_row_provenance(0),
        (boot_trace_expected, boot_span_expected),
        "MEMORY_PAD (0xFFFF_FFFF) must fall back to bootstrap provenance"
    );
}

// ============================================================================
// Challenge 3: Dynamic Compilation Chain of Custody Across Multi-Turn Dialogue
// ============================================================================

#[test]
fn test_challenge_3_dynamic_dialogue_compilation_chain_of_custody() {
    let mut model = load_test_model();
    let initial_rows = model.point.as_ref().map(|p| p.rows).unwrap_or(0);
    assert!(initial_rows > 0, "Model must have initial point key rows");

    for r in 0..initial_rows {
        let (tr, sp) = model.point.as_ref().unwrap().get_row_provenance(r);
        if tr != 0x756f722d72342e626f6f747374726170 || sp != 0x756f722d72342e38 {
            let pt = model.point.as_ref().unwrap();
            let base = r * pt.anchor_len;
            let slice = &pt.canonical_anchors.as_ref().unwrap()[base..base + pt.anchor_len.min(16)];
            eprintln!(
                "Preexisting non-bootstrap row at {r}: trace={tr:#x}, span={sp:#x}, anchors={:x?}",
                slice
            );
            break;
        }
    }

    let test_turns = [
        (
            "State Fermat's Little Theorem.",
            "If p is prime and gcd(a,p)=1, then a^(p-1) = 1 (mod p).",
        ),
        (
            "What is Euler's totient theorem?",
            "For coprime a and n, a^phi(n) = 1 (mod n).",
        ),
        (
            "Define the Clifford algebra Cl(0,6).",
            "Cl(0,6) has 6 negative-definite generators and dimension 64.",
        ),
        (
            "Explain ternary associative memory basins.",
            "Ternary weights {-1, 0, +1} form energy basins for robust retrieval.",
        ),
        (
            "What is the Icosian ring in E8?",
            "The Icosian ring is a discrete subring of quaternions linked to E8.",
        ),
    ];

    let mut compiled_provenances = Vec::new();

    for (turn_num, &(prompt, reply)) in test_turns.iter().enumerate() {
        let added = model
            .compile_verified_dialogue_turn(prompt, reply)
            .expect("compile dialogue turn");
        assert!(added, "Compilation of turn {turn_num} must succeed");

        let pt = model.point.as_ref().expect("point table");
        assert_eq!(pt.rows, initial_rows + turn_num + 1);

        // Authenticate deterministic derivation
        let prompt_clean = prompt.trim();
        let reply_clean = reply.trim();
        let expected_span =
            fnv1a64(format!("gllm.training.compile_turn:{prompt_clean}:{reply_clean}").as_bytes())
                .max(1);
        let expected_trace_lo =
            fnv1a64(format!("gllm.training.trace:{prompt_clean}").as_bytes()).max(1);
        let expected_trace_hi = fnv1a64(b"uor.training.hi").max(1);
        let expected_trace = (expected_trace_lo as u128) | ((expected_trace_hi as u128) << 64);

        // Find the inserted row by searching for the matching prov_span
        let mut found_row = None;
        for r in 0..pt.rows {
            let base = r * pt.anchor_len;
            let slice = &pt.canonical_anchors.as_ref().unwrap()[base..base + pt.anchor_len];
            if slice[7] == VERIFIED_FRONTIER_ANCHOR_TAG {
                let s_lo = slice[1];
                let s_hi = slice[2];
                let s = (s_lo as u64) | ((s_hi as u64) << 32);
                if s == expected_span {
                    found_row = Some((r, pt.get_row_provenance(r)));
                    break;
                }
            }
        }

        let (compiled_row, (prov_trace, prov_span)) =
            found_row.expect("compiled row with matching expected_span must exist in point table");
        eprintln!("Turn {turn_num}: successfully found compiled row at index {compiled_row}!");
        compiled_provenances.push((prov_trace, prov_span));

        assert_eq!(
            prov_span, expected_span,
            "Turn {turn_num} span_id does not match deterministic training hash"
        );
        assert_eq!(
            prov_trace, expected_trace,
            "Turn {turn_num} trace_id does not match deterministic training hash"
        );

        // Verify parallel vectors match row encoding
        if let Some(spans) = &pt.provenance_spans {
            assert_eq!(spans[compiled_row], prov_span);
        }
        if let Some(traces) = &pt.provenance_traces {
            assert_eq!(traces[compiled_row], prov_trace);
        }
    }

    // Verify all 5 compiled turns have strictly distinct span IDs and trace IDs
    let unique_spans: HashSet<u64> = compiled_provenances.iter().map(|&(_, s)| s).collect();
    let unique_traces: HashSet<u128> = compiled_provenances.iter().map(|&(t, _)| t).collect();
    assert_eq!(unique_spans.len(), test_turns.len());
    assert_eq!(unique_traces.len(), test_turns.len());
}

// ============================================================================
// Challenge 4: SpanLinks in gllm.routing.point_key and gllm.decode.step
// ============================================================================

#[test]
fn test_challenge_4_spanlinks_routing_and_decode_step_waterfall() {
    let mut model = load_test_model();
    model
        .compile_verified_dialogue_turn(
            "What is the Pythagorean theorem?",
            "In a right triangle, a^2 + b^2 = c^2.",
        )
        .expect("compile turn");

    let model_arc = Arc::new(model);
    let mut context = RollingContextState::default();
    let prompt = "What is the Pythagorean theorem?";

    let ctx_before = context.q12_latent;
    let gen = model_arc.generate_reply_with_context(&mut context, &[], prompt, 16);
    let ctx_after = context.q12_latent;

    let trace = build_inference_trace(1, prompt, &ctx_before, &ctx_after, &gen, &model_arc);

    // 1. Verify routing span links
    let routing_span = trace
        .spans
        .iter()
        .find(|s| trace.intern_table.get(s.name_ref) == "gllm.routing.point_key")
        .expect("gllm.routing.point_key span must exist");

    assert_eq!(
        routing_span.links.len(),
        1,
        "gllm.routing.point_key must contain exactly 1 SpanLink"
    );
    let r_link = &routing_span.links[0];
    let r_trace_str = trace.intern_table.get(r_link.trace_id_ref);
    let r_span_str = trace.intern_table.get(r_link.span_id_ref);

    assert_eq!(r_trace_str.len(), 32);
    assert_eq!(r_span_str.len(), 16);
    assert!(r_trace_str.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(r_span_str.chars().all(|c| c.is_ascii_hexdigit()));

    // Link attribute check
    assert_eq!(r_link.attributes.len(), 1);
    let (k_ref, v_ref) = r_link.attributes[0];
    assert_eq!(trace.intern_table.get(k_ref), "uor.link_type");
    assert_eq!(
        trace.intern_table.get(v_ref),
        "training_point_key_provenance"
    );

    // 2. Verify decode step links
    let decode_spans: Vec<_> = trace
        .spans
        .iter()
        .filter(|s| trace.intern_table.get(s.name_ref) == "gllm.decode.step")
        .collect();

    assert!(
        !decode_spans.is_empty(),
        "Must have at least one decode step"
    );

    for (step_idx, step_span) in decode_spans.iter().enumerate() {
        assert_eq!(
            step_span.links.len(),
            1,
            "Decode step {step_idx} must contain exactly 1 SpanLink"
        );
        let s_link = &step_span.links[0];
        let s_trace_str = trace.intern_table.get(s_link.trace_id_ref);
        let s_span_str = trace.intern_table.get(s_link.span_id_ref);

        assert_eq!(s_trace_str.len(), 32);
        assert_eq!(s_span_str.len(), 16);

        let (k_ref, v_ref) = s_link.attributes[0];
        assert_eq!(trace.intern_table.get(k_ref), "uor.link_type");
        assert_eq!(
            trace.intern_table.get(v_ref),
            "consolidation_writer_basin_provenance"
        );
    }

    // 3. Hydrate to W3C OTLP JSON and verify schema representation
    let otlp = trace.hydrate_to_otlp();
    let spans = &otlp.resource_spans[0].scope_spans[0].spans;

    let otlp_routing = spans
        .iter()
        .find(|s| s.name == "gllm.routing.point_key")
        .expect("otlp routing span");
    assert_eq!(otlp_routing.links.len(), 1);
    assert_eq!(otlp_routing.links[0].trace_id, r_trace_str);
    assert_eq!(otlp_routing.links[0].span_id, r_span_str);

    let otlp_decode = spans
        .iter()
        .find(|s| s.name == "gllm.decode.step")
        .expect("otlp decode span");
    assert_eq!(otlp_decode.links.len(), 1);
    assert_eq!(otlp_decode.links[0].trace_id.len(), 32);
    assert_eq!(otlp_decode.links[0].span_id.len(), 16);
}

// ============================================================================
// Challenge 5: Concurrent API Flooding & Socket Stability Hammer
// ============================================================================

#[test]
fn test_challenge_5_concurrent_api_flooding_socket_stability() {
    let daemon = Arc::new(TestServerDaemon::spawn());
    let addr = daemon.addr.clone();

    let total_workers = 16;
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
                let outcome = match turn % 5 {
                    0 => {
                        // GET /v1/traces (OTLP format)
                        send_raw_http_request(
                            &addr_clone,
                            "GET",
                            "/v1/traces",
                            None,
                            Duration::from_secs(10),
                        )
                    }
                    1 => {
                        // POST /v1/traces (compact format)
                        send_raw_http_request(
                            &addr_clone,
                            "POST",
                            "/v1/traces",
                            Some(r#"{"format":"compact","limit":5}"#),
                            Duration::from_secs(10),
                        )
                    }
                    2 => {
                        // POST /v1/messages with otel_trace
                        let payload = format!(
                            r#"{{"messages":[{{"role":"user","content":"hello worker {w} req {r}"}}],"otel_trace":true}}"#
                        );
                        send_raw_http_request(
                            &addr_clone,
                            "POST",
                            "/v1/messages",
                            Some(&payload),
                            Duration::from_secs(10),
                        )
                    }
                    3 => {
                        // POST /v1/chat/completions with otel_trace
                        let payload = format!(
                            r#"{{"messages":[{{"role":"user","content":"chat worker {w} req {r}"}}],"otel_trace":true}}"#
                        );
                        send_raw_http_request(
                            &addr_clone,
                            "POST",
                            "/v1/chat/completions",
                            Some(&payload),
                            Duration::from_secs(10),
                        )
                    }
                    _ => {
                        // GET /v1/traces?format=compact&limit=2
                        send_raw_http_request(
                            &addr_clone,
                            "GET",
                            "/v1/traces?format=compact&limit=2",
                            None,
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
        "All concurrent requests must succeed"
    );
    eprintln!(
        "Concurrently dispatched and processed {} mixed API requests in {:?} ({:.1} req/sec)",
        total_success,
        elapsed,
        total_success as f64 / elapsed.as_secs_f64()
    );

    // 4. Test abrupt client disconnect stress: write partial request and close socket
    for _ in 0..10 {
        if let Ok(mut s) = TcpStream::connect(&addr) {
            let _ = s.write_all(
                b"POST /v1/traces HTTP/1.1\r\nHost: test\r\nContent-Length: 100\r\n\r\npartial",
            );
            // Deliberately drop stream immediately without finishing
        }
    }

    // Verify server remains responsive after client drop stress
    let (final_status, _final_headers, final_body) =
        send_raw_http_request(&addr, "GET", "/v1/traces", None, Duration::from_secs(10))
            .expect("post-stress health check");
    assert_eq!(final_status, 200);
    assert!(final_body.contains("resourceSpans"));
}

// ============================================================================
// Challenge 6: Empirical Defect Demonstration - Preexisting Frontier Rows
// with 0xFFFF_FFFF Padding Fail to Fallback to Bootstrap Provenance
// ============================================================================

#[test]
fn test_challenge_6_defect_preexisting_frontier_rows_with_memory_pad_fail_bootstrap_fallback() {
    let model = load_test_model();
    let pt = model.point.as_ref().expect("point table must exist");

    let expected_boot_trace = 0x756f722d72342e626f6f747374726170_u128; // "uor-r4.bootstrap"
    let expected_boot_span = 0x756f722d72342e38_u64; // "uor-r4.8"

    let mut defective_rows = Vec::new();

    // Scan all preexisting rows in bootstrap_v8.uorlut
    for r in 0..pt.rows {
        let base = r * pt.anchor_len;
        if base + 7 < pt.canonical_anchors.as_ref().unwrap().len() {
            let anchors = &pt.canonical_anchors.as_ref().unwrap();
            if anchors[base + 7] == VERIFIED_FRONTIER_ANCHOR_TAG {
                // If words 1..6 are MEMORY_PAD (0xFFFF_FFFF), this is a legacy frontier row
                if anchors[base + 1] == 0xFFFF_FFFF && anchors[base + 2] == 0xFFFF_FFFF {
                    let (tr, sp) = pt.get_row_provenance(r);
                    if tr != expected_boot_trace || sp != expected_boot_span {
                        defective_rows.push((r, tr, sp));
                    }
                }
            }
        }
    }

    // There are exactly 25 preexisting frontier rows (5289..5313) that trigger this defect
    eprintln!(
        "Found {} preexisting frontier rows returning non-bootstrap IDs: {:?}",
        defective_rows.len(),
        defective_rows
    );

    // This assertion documents the defect:
    // If defective_rows is non-empty, get_row_provenance() failed to fall back for rows with 0xFFFF_FFFF padding!
    assert!(
        defective_rows.is_empty(),
        "CRITICAL DEFECT: Preexisting frontier rows with 0xFFFF_FFFF padding MUST fall back to bootstrap IDs, but {} rows returned u64::MAX/u128::MAX: {:?}",
        defective_rows.len(),
        defective_rows
    );
}
