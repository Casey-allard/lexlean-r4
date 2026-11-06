#![allow(
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec
)]
//! Empirical Challenge Suite for Milestone M5 Iteration 2:
//! Direct Mount & Content Block Adversarial Test Harness
//!
//! Objectives:
//! 1. Reconstituted thinking blocks with edge-case contents:
//!    - Empty thinking content (`Some("")` or whitespace only).
//!    - Multiline markdown, LaTeX, and code snippets in thinking blocks.
//!    - Truncated or empty reply text with thinking text present.
//! 2. Content Block Separation:
//!    - Verify `AnthropicContentBlock::Thinking` contains exact reasoning content.
//!    - Verify `AnthropicContentBlock::Text` contains clean prose without XML tags.
//! 3. Tool call interleaving and multi-turn context preservation.
//! 4. Live `default_model_backend()` end-to-end inference verification.
//! 5. Pure zero-float integer Q12 invariant conformance and schema adherence.

use lexlean_r4_wasm::api::{
    default_model_backend, handle_v1_chat_completions_payload_with_backend,
    handle_v1_messages_payload, handle_v1_messages_payload_with_backend, parse_assistant_turn,
    AnthropicContentBlock, AnthropicMessagesResponse, InferenceBackend, ModelTurnOutcome,
    OpenAiChatCompletionResponse, ToolDescriptor,
};
use lexlean_r4_wasm::lut_stack_exec::RollingContextState;

// ============================================================================
// Mock Adversarial Backend for Precise Boundary Control
// ============================================================================

struct MockAdversarialBackend {
    thinking_text: Option<String>,
    reply_text: String,
    turn_count: usize,
}

impl MockAdversarialBackend {
    fn new(thinking: Option<&str>, reply: &str) -> Self {
        Self {
            thinking_text: thinking.map(String::from),
            reply_text: reply.to_string(),
            turn_count: 0,
        }
    }
}

impl InferenceBackend for MockAdversarialBackend {
    fn infer_turn(
        &mut self,
        _prompt: &str,
        _history: &[(String, String)],
        _tools: &[ToolDescriptor],
        context: &mut RollingContextState,
        _max_tokens: usize,
    ) -> Result<ModelTurnOutcome, String> {
        self.turn_count += 1;
        context.turn_index += 1;

        // Reconstitute raw output matching api.rs lines 1028-1038 / 1092-1102
        let raw_output = if let Some(ref th) = self.thinking_text {
            let base_reply = if self.reply_text.trim().is_empty() {
                "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?"
            } else {
                self.reply_text.as_str()
            };
            format!("<thinking>\n{}\n</thinking>\n\n{}", th.trim(), base_reply)
        } else if self.reply_text.trim().is_empty() {
            "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?".to_string()
        } else {
            self.reply_text.clone()
        };

        Ok(ModelTurnOutcome {
            raw_output,
            input_tokens: 16,
            output_tokens: 32,
            otel_trace: None,
        })
    }
}

// ============================================================================
// 1. Edge Case: Empty and Whitespace-Only Thinking Content
// ============================================================================

#[test]
fn challenge_edge_empty_and_whitespace_thinking_blocks() {
    let empty_variants = ["", "   ", "\t\t", "\n\n\r\n", "  \n  \t  \r\n  "];

    for (idx, &variant) in empty_variants.iter().enumerate() {
        let mut backend = MockAdversarialBackend::new(
            Some(variant),
            "Clean mathematical prose for prime factorization.",
        );

        let req = serde_json::json!({
            "model": "uor-r4-glm-01",
            "messages": [{"role": "user", "content": "Factor 91"}]
        });

        let (status, resp_str) =
            handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
                .expect("payload succeeds");

        assert_eq!(status, 200, "Variant {idx} must yield HTTP 200");
        let resp: AnthropicMessagesResponse =
            serde_json::from_str(&resp_str).expect("valid AnthropicMessagesResponse");

        // When thinking is empty or whitespace-only, no AnthropicContentBlock::Thinking should be produced
        let has_thinking = resp
            .content
            .iter()
            .any(|b| matches!(b, AnthropicContentBlock::Thinking { .. }));
        assert!(
            !has_thinking,
            "Variant {idx} ('{variant}'): empty thinking must not produce AnthropicContentBlock::Thinking"
        );

        // Exactly one Text block must be emitted
        let text_blocks: Vec<&String> = resp
            .content
            .iter()
            .filter_map(|b| match b {
                AnthropicContentBlock::Text { text } => Some(text),
                _ => None,
            })
            .collect();

        assert_eq!(text_blocks.len(), 1, "Must contain exactly 1 text block");
        let prose = text_blocks[0];
        assert_eq!(prose, "Clean mathematical prose for prime factorization.");
        assert!(
            !prose.contains("<thinking>") && !prose.contains("</thinking>"),
            "Prose must not contain thinking tags: {prose}"
        );
    }
}

#[test]
fn challenge_edge_none_thinking_block() {
    let mut backend =
        MockAdversarialBackend::new(None, "Direct reply without any reasoning trace.");

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Compute 2 + 2"}]
    });

    let (status, resp_str) =
        handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
            .expect("payload succeeds");

    assert_eq!(status, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.content.len(), 1);
    match &resp.content[0] {
        AnthropicContentBlock::Text { text } => {
            assert_eq!(text, "Direct reply without any reasoning trace.");
        }
        other => panic!("Expected Text block, got: {:?}", other),
    }
}

// ============================================================================
// 2. Edge Case: Multiline Markdown, LaTeX, and Code Snippets in Thinking
// ============================================================================

#[test]
fn challenge_edge_multiline_markdown_latex_and_code_thinking() {
    let complex_thinking = r#"# Phase 1: Number Theoretic Analysis
We consider the modulus $p = 101$. By Wilson's Theorem:
\begin{equation}
  (p - 1)! \equiv -1 \pmod{p}
\end{equation}
Furthermore, the quadratic residue symbol evaluates as:
\begin{align*}
  \left(\frac{a}{p}\right) &\equiv a^{(p-1)/2} \pmod{p} \\
  \sum_{k=1}^{n} k^2 &= \frac{n(n+1)(2n+1)}{6}
\end{align*}

```rust
// Verified Rust implementation of Fermat test
pub fn is_fermat_prime(n: u64, a: u64) -> bool {
    if n <= 1 { return false; }
    let mut base = a % n;
    let mut exp = n - 1;
    let mut res = 1u64;
    while exp > 0 {
        if exp % 2 == 1 { res = ((res as u128 * base as u128) % n as u128) as u64; }
        base = ((base as u128 * base as u128) % n as u128) as u64;
        exp /= 2;
    }
    res == 1
}
```

Nested XML tags inside code commentary: `<cmath>`, `<div>`, `<xml_stream>`
Comparison operators: `x < y && y > z`
Unicode math symbols: `∀x ∈ ℝ, ∃y ∈ ℂ: y² = -1 ∧ 🎯 🚀 🦀`"#;

    let clean_reply = "Wilson's Theorem establishes that 101 is prime because 100! ≡ -1 (mod 101).";

    let mut backend = MockAdversarialBackend::new(Some(complex_thinking), clean_reply);

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Prove 101 is prime with LaTeX and code."}]
    });

    let (status, resp_str) =
        handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
            .expect("payload succeeds");

    assert_eq!(status, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.content.len(), 2, "Expected Thinking and Text blocks");

    // 1. Validate Thinking Block
    match &resp.content[0] {
        AnthropicContentBlock::Thinking {
            thinking,
            signature,
        } => {
            assert!(signature.is_none(), "Signature should be None");
            // Check that exact markdown, LaTeX, and code snippets are preserved intact
            assert!(thinking.contains("# Phase 1: Number Theoretic Analysis"));
            assert!(thinking.contains(r"\begin{equation}"));
            assert!(thinking.contains(r"\begin{align*}"));
            assert!(thinking.contains("pub fn is_fermat_prime"));
            assert!(thinking.contains("x < y && y > z"));
            assert!(thinking.contains("∀x ∈ ℝ, ∃y ∈ ℂ: y² = -1 ∧ 🎯 🚀 🦀"));
            assert_eq!(thinking, complex_thinking.trim());
        }
        other => panic!("Expected AnthropicContentBlock::Thinking, got: {:?}", other),
    }

    // 2. Validate Text Block
    match &resp.content[1] {
        AnthropicContentBlock::Text { text } => {
            assert_eq!(text, clean_reply);
            assert!(
                !text.contains("<thinking>") && !text.contains("</thinking>"),
                "Text block must not contain XML thinking tags"
            );
        }
        other => panic!("Expected AnthropicContentBlock::Text, got: {:?}", other),
    }

    // 3. Complete JSON round-trip validation
    let serialized = serde_json::to_string(&resp).expect("serialization succeeds");
    let roundtrip: AnthropicMessagesResponse =
        serde_json::from_str(&serialized).expect("round-trip deserialization succeeds");
    assert_eq!(resp.content.len(), roundtrip.content.len());
    if let AnthropicContentBlock::Thinking { thinking, .. } = &roundtrip.content[0] {
        assert_eq!(thinking, complex_thinking.trim());
    }
}

// ============================================================================
// 3. Edge Case: Truncated or Empty Reply Text with Thinking Present
// ============================================================================

#[test]
fn challenge_edge_truncated_reply_with_thinking() {
    let thinking = "Detailed step-by-step reasoning for truncated generation.";
    let truncated_reply = "The prime factorization of 2026 is 2 * 1013"; // Partial or compact generation

    let mut backend = MockAdversarialBackend::new(Some(thinking), truncated_reply);

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Factor 2026"}]
    });

    let (status, resp_str) =
        handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
            .expect("payload succeeds");

    assert_eq!(status, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.content.len(), 2);
    match &resp.content[0] {
        AnthropicContentBlock::Thinking { thinking: th, .. } => {
            assert_eq!(th, thinking);
        }
        other => panic!("Expected Thinking, got {:?}", other),
    }
    match &resp.content[1] {
        AnthropicContentBlock::Text { text } => {
            assert_eq!(text, truncated_reply.trim());
            assert!(
                !text.contains("<thinking>") && !text.contains("</thinking>"),
                "Truncated text must not contain thinking tags"
            );
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[test]
fn challenge_edge_empty_reply_triggers_default_greeting_with_thinking() {
    let thinking = "Reasoning succeeded, but model emitted zero reply tokens.";
    let mut backend = MockAdversarialBackend::new(Some(thinking), "");

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Say hello"}]
    });

    let (status, resp_str) =
        handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
            .expect("payload succeeds");

    assert_eq!(status, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.content.len(), 2);
    match &resp.content[0] {
        AnthropicContentBlock::Thinking { thinking: th, .. } => {
            assert_eq!(th, thinking);
        }
        other => panic!("Expected Thinking, got {:?}", other),
    }
    match &resp.content[1] {
        AnthropicContentBlock::Text { text } => {
            assert!(
                text.contains("UOR-R4 Geometric Language Model"),
                "Empty reply must trigger canonical default greeting: {text}"
            );
            assert!(
                !text.contains("<thinking>") && !text.contains("</thinking>"),
                "Default greeting must not leak thinking tags"
            );
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

// ============================================================================
// 4. Edge Case: Multiple and Sibling Thinking Blocks Reconstitution
// ============================================================================

#[test]
fn challenge_edge_multiple_and_sibling_thinking_blocks() {
    let raw_mixed = "<thinking>Initial proof sketch: consider group (Z/pZ)*.</thinking>\n\
                     Intermediate prose explanation.\n\
                     <thought>Secondary deduction: pairing each element with its inverse.</thought>\n\
                     Final conclusion: Wilson's theorem is established.";

    let parsed = parse_assistant_turn(raw_mixed);

    assert!(parsed.thinking.is_some(), "Must extract thinking");
    let th = parsed.thinking.unwrap();
    assert!(th.contains("Initial proof sketch"));
    assert!(th.contains("Secondary deduction"));
    assert_eq!(
        th,
        "Initial proof sketch: consider group (Z/pZ)*.\n\nSecondary deduction: pairing each element with its inverse."
    );

    assert!(parsed.clean_text.is_some(), "Must extract clean text");
    let clean = parsed.clean_text.unwrap();
    assert!(clean.contains("Intermediate prose explanation."));
    assert!(clean.contains("Final conclusion: Wilson's theorem is established."));
    assert!(!clean.contains("<thinking>"));
    assert!(!clean.contains("</thinking>"));
    assert!(!clean.contains("<thought>"));
    assert!(!clean.contains("</thought>"));
}

// ============================================================================
// 5. Edge Case: Interleaved Tool Calls and Thinking Blocks
// ============================================================================

#[test]
fn challenge_edge_interleaved_thinking_tool_call_and_text() {
    let raw = "<thought>Checking if 1009 is prime via execute_command.</thought>\n\
               <tool_call>{\"name\": \"run_cmd\", \"arguments\": {\"command\": \"factor 1009\"}}</tool_call>\n\
               Please wait while the command executes.";

    let parsed = parse_assistant_turn(raw);

    assert_eq!(
        parsed.thinking.as_deref(),
        Some("Checking if 1009 is prime via execute_command.")
    );
    assert_eq!(parsed.tool_calls.len(), 1);
    assert_eq!(parsed.tool_calls[0].name, "run_cmd");
    assert_eq!(
        parsed.tool_calls[0].arguments_value,
        serde_json::json!({ "command": "factor 1009" })
    );
    assert_eq!(
        parsed.clean_text.as_deref(),
        Some("Please wait while the command executes.")
    );
}

// ============================================================================
// 6. OpenAI Chat Completions Parity & Separation
// ============================================================================

#[test]
fn challenge_openai_chat_completions_reasoning_content_parity() {
    let thinking = "OpenAI reasoning trace content.";
    let reply = "Clean response message in OpenAI format.";

    let mut backend = MockAdversarialBackend::new(Some(thinking), reply);

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Explain Q12 arithmetic"}]
    });

    let (status, resp_str) =
        handle_v1_chat_completions_payload_with_backend(&req.to_string(), &mut backend)
            .expect("openai payload succeeds");

    assert_eq!(status, 200);
    let resp: OpenAiChatCompletionResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.choices.len(), 1);
    let choice = &resp.choices[0];
    assert_eq!(choice.message.reasoning_content.as_deref(), Some(thinking));
    assert_eq!(choice.message.content.as_deref(), Some(reply));
    if let Some(ref c) = choice.message.content {
        assert!(
            !c.contains("<thinking>") && !c.contains("</thinking>"),
            "OpenAI content must not contain thinking tags"
        );
    }
}

// ============================================================================
// 7. Live default_model_backend() Inference & Reasoning Extraction
// ============================================================================

#[test]
fn challenge_live_default_model_backend_thinking_reconstitution() {
    let test_queries = [
        ("What is Wilson's Theorem?", "Wilson"),
        ("What is Fermat's Little Theorem?", "Fermat"),
    ];

    for (query, expected_stem) in test_queries {
        let req = serde_json::json!({
            "model": "uor-r4-glm-01",
            "messages": [{"role": "user", "content": query}]
        });

        let (status, resp_str) =
            handle_v1_messages_payload(&req.to_string()).expect("messages payload succeeds");

        assert_eq!(status, 200, "Query '{query}' must return HTTP 200");
        let resp: AnthropicMessagesResponse =
            serde_json::from_str(&resp_str).expect("deserializes cleanly");

        let mut found_thinking = false;
        let mut found_text = false;

        for block in &resp.content {
            match block {
                AnthropicContentBlock::Thinking { thinking, .. } => {
                    found_thinking = true;
                    assert!(
                        thinking.contains(expected_stem),
                        "Thinking trace must contain expected theorem stem '{expected_stem}': {thinking}"
                    );
                    assert!(
                        !thinking.contains("<thinking>") && !thinking.contains("</thinking>"),
                        "Thinking content itself must not contain envelope XML tags"
                    );
                }
                AnthropicContentBlock::Text { text } => {
                    found_text = true;
                    assert!(
                        text.contains(expected_stem),
                        "Prose must address expected theorem stem '{expected_stem}': {text}"
                    );
                    assert!(
                        !text.contains("<thinking>") && !text.contains("</thinking>"),
                        "Text block must NOT contain XML thinking tags: {text}"
                    );
                }
                _ => {}
            }
        }

        assert!(
            found_thinking,
            "Query '{query}' must reconstitute AnthropicContentBlock::Thinking"
        );
        assert!(
            found_text,
            "Query '{query}' must contain clean AnthropicContentBlock::Text"
        );
    }
}

// ============================================================================
// 8. Multi-Turn Context Persistence with Thinking Trace Accumulation
// ============================================================================

#[test]
fn challenge_multi_turn_thinking_trace_accumulation() {
    let mut backend = default_model_backend();

    // Turn 1
    let req1 = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "What is Wilson's Theorem?"}]
    });

    let (s1, r1) = handle_v1_messages_payload_with_backend(&req1.to_string(), &mut backend)
        .expect("turn 1 succeeds");
    assert_eq!(s1, 200);
    let resp1: AnthropicMessagesResponse = serde_json::from_str(&r1).unwrap();
    assert!(resp1.context.is_some(), "Context must be emitted");
    let ctx1 = resp1.context.unwrap();

    // Verify turn 1 thinking was recorded into context
    let tt1 = ctx1
        .get("thinking_trace")
        .and_then(|v| v.as_array())
        .expect("thinking_trace array");
    assert!(
        !tt1.is_empty(),
        "thinking_trace must accumulate turn 1 reasoning"
    );

    // Turn 2 passing back context
    let req2 = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [
            {"role": "user", "content": "What is Wilson's Theorem?"},
            {"role": "assistant", "content": "Wilson's Theorem states that (p-1)! = -1 mod p."},
            {"role": "user", "content": "What is Fermat's Little Theorem?"}
        ],
        "context": ctx1
    });

    let (s2, r2) = handle_v1_messages_payload_with_backend(&req2.to_string(), &mut backend)
        .expect("turn 2 succeeds");
    assert_eq!(s2, 200);
    let resp2: AnthropicMessagesResponse = serde_json::from_str(&r2).unwrap();
    let ctx2 = resp2.context.expect("turn 2 emits context");

    let tt2 = ctx2
        .get("thinking_trace")
        .and_then(|v| v.as_array())
        .expect("thinking_trace array");
    assert!(
        tt2.len() >= tt1.len(),
        "thinking_trace must persist and accumulate across turns"
    );
}

// ============================================================================
// 9. Stress & Fuzz: Near-Miss Tags and Angle Bracket Code Snippets
// ============================================================================

#[test]
fn challenge_stress_near_miss_tags_and_rust_generics() {
    // Text containing Rust generic type parameters like <T>, <Option<String>>, and near-miss tags
    let raw = "<thinking>\n\
               Analyzing Rust type signature: `Result<Option<Vec<u8>>, Error>`.\n\
               The type uses angle brackets `<` and `>` which must not be mistaken for XML tags.\n\
               Near-miss tags: `<think>`, `</think>`, `<<thinking>>`, `<thought >`.\n\
               </thinking>\n\n\
               The final verified signature is `pub fn parse<T: AsRef<[u8]>>(buf: T) -> Result<T, Error>`.";

    let parsed = parse_assistant_turn(raw);

    assert!(parsed.thinking.is_some());
    let th = parsed.thinking.unwrap();
    assert!(th.contains("Result<Option<Vec<u8>>, Error>"));
    assert!(th.contains("Near-miss tags: `<think>`"));

    assert!(parsed.clean_text.is_some());
    let clean = parsed.clean_text.unwrap();
    assert_eq!(
        clean,
        "The final verified signature is `pub fn parse<T: AsRef<[u8]>>(buf: T) -> Result<T, Error>`."
    );
    // Angle brackets in generic parameters must be fully preserved!
    assert!(clean.contains("<T: AsRef<[u8]>>"));
    assert!(!clean.contains("<thinking>") && !clean.contains("</thinking>"));
}

// ============================================================================
// 10. Adversarial Stress: HTML, XML, Math Equations Inside Reply Text
// ============================================================================

#[test]
fn challenge_stress_html_xml_in_reply_not_stripped() {
    let thinking = "Evaluating HTML snippet formatting in reply.";
    let html_reply = "Here is the code: `<div class=\"theorem\"><p>p is prime</p></div>`.\n\
                      Also note: 5 < 10 and 10 > 2.";

    let mut backend = MockAdversarialBackend::new(Some(thinking), html_reply);

    let req = serde_json::json!({
        "model": "uor-r4-glm-01",
        "messages": [{"role": "user", "content": "Format HTML theorem"}]
    });

    let (status, resp_str) =
        handle_v1_messages_payload_with_backend(&req.to_string(), &mut backend)
            .expect("payload succeeds");

    assert_eq!(status, 200);
    let resp: AnthropicMessagesResponse = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp.content.len(), 2);
    if let AnthropicContentBlock::Text { text } = &resp.content[1] {
        // Legitimate HTML tags and comparison operators in user reply MUST NOT be stripped
        assert!(text.contains("<div class=\"theorem\">"));
        assert!(text.contains("<p>p is prime</p>"));
        assert!(text.contains("5 < 10 and 10 > 2"));
        // Only thinking tags are stripped
        assert!(!text.contains("<thinking>") && !text.contains("</thinking>"));
    } else {
        panic!("Expected Text block at index 1");
    }
}
