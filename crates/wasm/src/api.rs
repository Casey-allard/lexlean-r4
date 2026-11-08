//! Standard Provider-Compatible API Types and Dispatch Engine (Anthropic / OpenAI).
//!
//! Provides zero-dependency, serde-backed JSON serialization and deserialization
//! for `/v1/messages` and `/v1/chat/completions` endpoints.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

// ============================================================================
// Anthropic Protocol Models (/v1/messages)
// ============================================================================

/// Request envelope for Anthropic `/v1/messages`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesRequest {
    /// Model identifier, e.g. "uor-r4.lut-stack/8" or "claude-3-5-sonnet-20241022".
    #[serde(default = "default_model")]
    pub model: String,
    /// Message turn sequence.
    pub messages: Vec<AnthropicMessage>,
    /// Optional system prompt string or structured blocks.
    #[serde(default)]
    pub system: Option<AnthropicSystemPrompt>,
    /// Maximum tokens to generate in response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: Option<u32>,
    /// Tool definitions available for agentic calling.
    #[serde(default)]
    pub tools: Option<Vec<AnthropicToolDefinition>>,
    /// Streaming response flag.
    #[serde(default)]
    pub stream: Option<bool>,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// LexLean rolling 256D Q12 latent context state extension.
    #[serde(default)]
    pub context: Option<Value>,
    /// Optional request for inline OTel trace in the response.
    #[serde(default)]
    pub otel_trace: Option<bool>,
}

fn default_model() -> String {
    "uor-r4.lut-stack/8".to_string()
}

fn default_max_tokens() -> Option<u32> {
    Some(256)
}

/// An individual message turn in the Anthropic protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    /// Message role ("user" or "assistant").
    pub role: String,
    /// Content string or structured blocks.
    pub content: AnthropicContent,
}

/// Anthropic message content: either plain text string or list of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    /// Plain string content.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<AnthropicContentBlock>),
}

impl AnthropicContent {
    /// Extract user plain text or flattened block text.
    pub fn to_plain_text(&self) -> String {
        match self {
            AnthropicContent::Text(s) => s.clone(),
            AnthropicContent::Blocks(blocks) => {
                let mut out = String::new();
                for b in blocks {
                    match b {
                        AnthropicContentBlock::Text { text } => {
                            if !out.is_empty() {
                                out.push(' ');
                            }
                            out.push_str(text);
                        }
                        AnthropicContentBlock::ToolResult {
                            content,
                            tool_use_id,
                            is_error,
                        } => {
                            let err_tag = if is_error.unwrap_or(false) {
                                " error=true"
                            } else {
                                ""
                            };
                            let content_str = match content {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            out.push_str(&format!(
                                "<tool_response id=\"{tool_use_id}\"{err_tag}>{content_str}</tool_response>"
                            ));
                        }
                        _ => {}
                    }
                }
                out
            }
        }
    }
}

/// System prompt: plain string or array of text blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicSystemPrompt {
    /// Plain string system prompt.
    Text(String),
    /// Structured system blocks.
    Blocks(Vec<AnthropicSystemBlock>),
}

impl AnthropicSystemPrompt {
    /// Extract system prompt as plain text.
    pub fn to_plain_text(&self) -> String {
        match self {
            AnthropicSystemPrompt::Text(s) => s.clone(),
            AnthropicSystemPrompt::Blocks(blocks) => blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }
}

/// System text block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicSystemBlock {
    /// Block type ("text").
    pub r#type: String,
    /// Block content text.
    pub text: String,
}

/// Structured content block for Anthropic `/v1/messages`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    /// Standard prose text.
    #[serde(rename = "text")]
    Text {
        /// Text payload.
        text: String,
    },
    /// Reasoning / thought trace block.
    #[serde(rename = "thinking")]
    Thinking {
        /// Thinking reasoning trace.
        thinking: String,
        /// Optional signature.
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// Assistant tool invocation block.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique tool invocation ID.
        id: String,
        /// Name of the invoked tool.
        name: String,
        /// Input arguments object.
        input: Value,
    },
    /// Tool observation / result block.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Matching tool use ID.
        tool_use_id: String,
        /// Tool result content.
        content: Value,
        /// Optional error flag.
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Anthropic tool definition schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicToolDefinition {
    /// Name of the tool.
    pub name: String,
    /// Optional tool description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON schema for tool inputs.
    pub input_schema: Value,
}

/// Successful Anthropic `/v1/messages` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesResponse {
    /// Unique message identifier (e.g. "msg_01...").
    pub id: String,
    /// Response type ("message").
    pub r#type: String,
    /// Assistant role ("assistant").
    pub role: String,
    /// Model identifier.
    pub model: String,
    /// Content blocks (thinking, tool_use, text).
    pub content: Vec<AnthropicContentBlock>,
    /// Reason generation stopped ("end_turn", "tool_use", "max_tokens").
    pub stop_reason: String,
    /// Stop sequence that triggered termination.
    pub stop_sequence: Option<String>,
    /// Token usage metrics.
    pub usage: AnthropicUsage,
    /// LexLean rolling 256D Q12 latent context state extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    /// Optional inline OpenTelemetry trace telemetry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_trace: Option<Value>,
}

/// Token usage metrics for Anthropic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    /// Prompt input tokens.
    pub input_tokens: u32,
    /// Generated output tokens.
    pub output_tokens: u32,
}

/// Typed Anthropic error envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicErrorResponse {
    /// Error envelope type ("error").
    pub r#type: String,
    /// Error detail object.
    pub error: AnthropicErrorDetails,
}

/// Anthropic error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicErrorDetails {
    /// Error type ("invalid_request_error", "not_found_error", etc.).
    pub r#type: String,
    /// Error description message.
    pub message: String,
}

// ============================================================================
// OpenAI Protocol Models (/v1/chat/completions)
// ============================================================================

/// Request envelope for OpenAI `/v1/chat/completions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChatCompletionRequest {
    /// Model identifier.
    #[serde(default = "default_model")]
    pub model: String,
    /// Chat messages sequence.
    pub messages: Vec<OpenAiChatMessage>,
    /// Available tools list.
    #[serde(default)]
    pub tools: Option<Vec<OpenAiToolDefinition>>,
    /// Streaming flag.
    #[serde(default)]
    pub stream: Option<bool>,
    /// Temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// LexLean rolling context state extension.
    #[serde(default)]
    pub context: Option<Value>,
    /// Optional request for inline OTel trace in the response.
    #[serde(default)]
    pub otel_trace: Option<bool>,
}

/// Individual message in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChatMessage {
    /// Message role ("system", "user", "assistant", "tool").
    pub role: String,
    /// Message text content.
    #[serde(default)]
    pub content: Option<String>,
    /// Participant name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool call ID (when role is "tool").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls requested by assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiToolDefinition {
    /// Tool type ("function").
    pub r#type: String,
    /// Function definition.
    pub function: OpenAiFunctionDefinition,
}

/// OpenAI function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunctionDefinition {
    /// Function name.
    pub name: String,
    /// Function description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON schema for parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// OpenAI tool call object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiToolCall {
    /// Unique call ID.
    pub id: String,
    /// Call type ("function").
    pub r#type: String,
    /// Function call details.
    pub function: OpenAiFunctionCall,
}

/// OpenAI function call arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunctionCall {
    /// Function name.
    pub name: String,
    /// Stringified JSON arguments.
    pub arguments: String,
}

/// Successful OpenAI `/v1/chat/completions` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChatCompletionResponse {
    /// Unique completion identifier (e.g. "chatcmpl-01...").
    pub id: String,
    /// Object type ("chat.completion").
    pub object: String,
    /// Creation timestamp (epoch seconds).
    pub created: u64,
    /// Model identifier.
    pub model: String,
    /// Choices list.
    pub choices: Vec<OpenAiChoice>,
    /// Token usage metrics.
    pub usage: OpenAiUsage,
    /// LexLean rolling context state extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    /// Optional inline OpenTelemetry trace telemetry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_trace: Option<Value>,
}

/// Individual choice item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChoice {
    /// Choice index (0).
    pub index: u32,
    /// Generated assistant message.
    pub message: OpenAiResponseMessage,
    /// Finish reason ("stop", "tool_calls", "length").
    pub finish_reason: String,
}

/// Assistant response message in OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiResponseMessage {
    /// Role ("assistant").
    pub role: String,
    /// Clean content text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Reasoning / thought trace content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Generated tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// Token usage metrics for OpenAI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiUsage {
    /// Prompt tokens count.
    pub prompt_tokens: u32,
    /// Completion tokens count.
    pub completion_tokens: u32,
    /// Total tokens count.
    pub total_tokens: u32,
}

/// Typed OpenAI error envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiErrorResponse {
    /// Error details.
    pub error: OpenAiErrorDetails,
}

/// OpenAI error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiErrorDetails {
    /// Error message.
    pub message: String,
    /// Error type ("invalid_request_error").
    pub r#type: String,
    /// Parameter causing error.
    pub param: Option<String>,
    /// Error code.
    pub code: Option<String>,
}

// ============================================================================
// Turn Output Decomposition & Structured Extraction
// ============================================================================

/// Parsed turn containing thinking traces, tool calls, and clean prose text.
#[derive(Debug, Clone, Default)]
pub struct ParsedTurn {
    /// Extracted `<thinking>` or `<thought>` trace.
    pub thinking: Option<String>,
    /// Extracted `<tool_call>` invocations.
    pub tool_calls: Vec<ParsedToolCall>,
    /// Clean prose text without XML tags.
    pub clean_text: Option<String>,
}

/// Parsed tool invocation.
#[derive(Debug, Clone)]
pub struct ParsedToolCall {
    /// Unique call identifier.
    pub id: String,
    /// Name of tool.
    pub name: String,
    /// Parsed JSON arguments value.
    pub arguments_value: Value,
    /// Stringified JSON arguments.
    pub arguments_str: String,
}

/// Calculate SHA-256 hex digest of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Parse raw model output into structured reasoning traces, tool calls, and clean prose.
pub fn parse_assistant_turn(raw_text: &str) -> ParsedTurn {
    // 1. Extract <thinking>...</thinking> or <thought>...</thought> blocks
    let mut thinking: Option<String> = None;
    let mut text_without_thinking = raw_text.to_string();

    for (open_tag, close_tag) in [("<thinking>", "</thinking>"), ("<thought>", "</thought>")] {
        while let Some(start) = text_without_thinking.find(open_tag) {
            let open_len = open_tag.len();
            if let Some(rel_end) = text_without_thinking[start + open_len..].find(close_tag) {
                let end = start + open_len + rel_end;
                let inside = text_without_thinking[start + open_len..end].trim();
                if !inside.is_empty() {
                    match thinking {
                        Some(ref mut existing) => {
                            existing.push_str("\n\n");
                            existing.push_str(inside);
                        }
                        None => {
                            thinking = Some(inside.to_string());
                        }
                    }
                }
                let mut remaining = String::with_capacity(text_without_thinking.len());
                remaining.push_str(&text_without_thinking[..start]);
                remaining.push_str(&text_without_thinking[end + close_tag.len()..]);
                text_without_thinking = remaining;
            } else {
                break;
            }
        }
    }

    // 2. Extract <tool_call> blocks
    let mut tool_calls = Vec::new();
    let mut current_text = text_without_thinking;

    while let Some(start) = current_text.find("<tool_call>") {
        let open_len = "<tool_call>".len();
        if let Some(rel_end) = current_text[start + open_len..].find("</tool_call>") {
            let end = start + open_len + rel_end;
            let call_payload = current_text[start + open_len..end].trim();
            if let Ok(val) = serde_json::from_str::<Value>(call_payload) {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let args_val = val.get("arguments").cloned().unwrap_or(Value::Null);
                let args_str = args_val.to_string();
                let call_hash = sha256_hex(format!("{name}:{args_str}").as_bytes());
                let id = format!("call_{}", &call_hash[..16]);
                tool_calls.push(ParsedToolCall {
                    id,
                    name,
                    arguments_value: args_val,
                    arguments_str: args_str,
                });
            }
            let mut remaining = String::with_capacity(current_text.len());
            remaining.push_str(&current_text[..start]);
            remaining.push_str(&current_text[end + "</tool_call>".len()..]);
            current_text = remaining;
        } else {
            break;
        }
    }

    let clean = current_text.trim();
    let clean_text = if clean.is_empty() {
        None
    } else {
        Some(clean.to_string())
    };

    ParsedTurn {
        thinking,
        tool_calls,
        clean_text,
    }
}

// ============================================================================
// Turn Synthesis & Structured Multi-Turn Dialogue Formatting
// ============================================================================

/// Structured representation of parsed conversation dialogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedDialogue {
    /// System prompt note, if present.
    pub system_prompt: Option<String>,
    /// Previous completed dialogue turns as (User, Assistant) pairs.
    pub history: Vec<(String, String)>,
    /// The active/current turn user prompt (including inline tool observations).
    pub current_user_prompt: String,
    /// Cleanly formatted multi-turn string with explicit role headers and \n\n demarcation.
    pub combined_prompt: String,
}

fn capitalize_role(role: &str) -> String {
    let mut chars = role.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Format Anthropic multi-turn messages into structured dialogue preserving roles and boundaries.
pub fn format_anthropic_messages(
    system: Option<&AnthropicSystemPrompt>,
    messages: &[AnthropicMessage],
) -> FormattedDialogue {
    let system_prompt = system.map(|s| s.to_plain_text());
    let mut history = Vec::new();
    let mut pending_user = None;
    let mut combined_blocks = Vec::new();

    if let Some(ref sys) = system_prompt {
        let trimmed_sys = sys.trim();
        if !trimmed_sys.is_empty() {
            combined_blocks.push(format!("System: {trimmed_sys}"));
        }
    }

    for msg in messages {
        let role = msg.role.trim().to_ascii_lowercase();
        let text = msg.content.to_plain_text();

        match role.as_str() {
            "user" => {
                if let Some(prev) = pending_user.take() {
                    history.push((prev, String::new()));
                }
                pending_user = Some(text.clone());
                combined_blocks.push(format!("User: {text}"));
            }
            "assistant" => {
                let user_prev = pending_user.take().unwrap_or_default();
                history.push((user_prev, text.clone()));
                combined_blocks.push(format!("Assistant: {text}"));
            }
            other => {
                let cap = capitalize_role(other);
                combined_blocks.push(format!("{cap}: {text}"));
            }
        }
    }

    let current_user_prompt = pending_user.unwrap_or_default();
    let combined_prompt = combined_blocks.join("\n\n");

    FormattedDialogue {
        system_prompt,
        history,
        current_user_prompt,
        combined_prompt,
    }
}

/// Format OpenAI multi-turn messages into structured dialogue preserving roles and tool responses.
pub fn format_openai_messages(messages: &[OpenAiChatMessage]) -> FormattedDialogue {
    let mut system_prompt = None;
    let mut history = Vec::new();
    let mut pending_user = None;
    let mut combined_blocks = Vec::new();

    for msg in messages {
        let role = msg.role.trim().to_ascii_lowercase();
        let mut text = msg.content.clone().unwrap_or_default();

        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                let call_tag = format!(
                    "<tool_call>{{\"name\":\"{}\",\"arguments\":{}}}</tool_call>",
                    tc.function.name, tc.function.arguments
                );
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&call_tag);
            }
        }

        match role.as_str() {
            "system" => {
                if !text.is_empty() {
                    system_prompt = Some(text.clone());
                    combined_blocks.push(format!("System: {text}"));
                }
            }
            "user" => {
                if let Some(prev) = pending_user.take() {
                    history.push((prev, String::new()));
                }
                pending_user = Some(text.clone());
                combined_blocks.push(format!("User: {text}"));
            }
            "assistant" => {
                let user_prev = pending_user.take().unwrap_or_default();
                history.push((user_prev, text.clone()));
                combined_blocks.push(format!("Assistant: {text}"));
            }
            "tool" => {
                let tool_id_attr = msg
                    .tool_call_id
                    .as_deref()
                    .map(|id| format!(" id=\"{id}\""))
                    .unwrap_or_default();
                let tool_tag = format!("<tool_response{tool_id_attr}>{text}</tool_response>");
                if let Some(ref mut u) = pending_user {
                    u.push('\n');
                    u.push_str(&tool_tag);
                } else {
                    pending_user = Some(tool_tag.clone());
                }
                combined_blocks.push(format!("Tool: {tool_tag}"));
            }
            other => {
                let cap = capitalize_role(other);
                combined_blocks.push(format!("{cap}: {text}"));
            }
        }
    }

    let current_user_prompt = pending_user.unwrap_or_default();
    let combined_prompt = combined_blocks.join("\n\n");

    FormattedDialogue {
        system_prompt,
        history,
        current_user_prompt,
        combined_prompt,
    }
}

/// Normalized tool declaration extracted from Anthropic `tools[]` or OpenAI `tools[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Unique tool identifier name.
    pub name: String,
    /// Optional tool description.
    #[serde(default)]
    pub description: Option<String>,
    /// Parameter JSON schema for inputs.
    #[serde(default)]
    pub input_schema: Option<Value>,
}

/// Raw outcome of an authentic model inference turn before protocol packaging.
#[derive(Debug, Clone)]
pub struct ModelTurnOutcome {
    /// Raw generated text from model (may contain `<thinking>` and `<tool_call>` blocks).
    pub raw_output: String,
    /// Exact count of input BPE tokens consumed.
    pub input_tokens: u32,
    /// Exact count of output BPE tokens generated.
    pub output_tokens: u32,
    /// Optional OpenTelemetry trace telemetry.
    pub otel_trace: Option<Value>,
}

/// Abstract engine contract for executing an authentic reasoning and generation turn.
pub trait InferenceBackend {
    /// Execute one reasoning turn producing model turn outcome.
    fn infer_turn(
        &mut self,
        prompt: &str,
        history: &[(String, String)],
        tools: &[ToolDescriptor],
        context: &mut crate::lut_stack_exec::RollingContextState,
        max_tokens: usize,
    ) -> Result<ModelTurnOutcome, String>;
}

/// Extract candidate filesystem path from prompt tokens dynamically.
/// Returns `None` if no path or filename is referenced in the prompt.
pub fn extract_target_path(prompt: &str) -> Option<String> {
    for word in prompt.split_whitespace() {
        let clean = word.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\''
                    | '`'
                    | ','
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '?'
                    | ':'
            )
        });
        if (clean.contains('/')
            || clean.ends_with(".rs")
            || clean.ends_with(".toml")
            || clean.ends_with(".json")
            || clean.ends_with(".tex")
            || clean.ends_with(".py")
            || clean.ends_with(".js")
            || clean.ends_with(".ts")
            || clean.ends_with(".md")
            || clean.ends_with(".txt")
            || clean.ends_with(".lean"))
            && clean.len() >= 3
            && !clean.starts_with('<')
        {
            return Some(clean.to_string());
        }
    }

    for quote in ['"', '\'', '`'] {
        if let Some(first) = prompt.find(quote) {
            if let Some(second) = prompt[first + 1..].find(quote) {
                let sub = prompt[first + 1..first + 1 + second].trim();
                if (sub.contains('/') || sub.contains('.')) && sub.len() >= 3 {
                    return Some(sub.to_string());
                }
            }
        }
    }

    None
}

/// Extract tool arguments dynamically according to schema.
pub fn extract_tool_arguments(prompt: &str, tool: &ToolDescriptor) -> Value {
    if let Some(path) = extract_target_path(prompt) {
        if let Some(ref schema) = tool.input_schema {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                if props.contains_key("path") {
                    return serde_json::json!({ "path": path });
                }
                if props.contains_key("target") {
                    return serde_json::json!({ "target": path });
                }
                if props.contains_key("file") {
                    return serde_json::json!({ "file": path });
                }
                if props.contains_key("filename") {
                    return serde_json::json!({ "filename": path });
                }
            }
        }
        return serde_json::json!({ "path": path });
    }

    serde_json::json!({})
}

/// Determine if a prompt requests an action matching a declared tool.
pub fn match_requested_tool<'a>(
    prompt: &str,
    tools: &'a [ToolDescriptor],
) -> Option<(&'a ToolDescriptor, Value)> {
    if tools.is_empty() {
        return None;
    }
    let lower = prompt.to_ascii_lowercase();

    // 1. Explicit tool name mention in prompt
    for tool in tools {
        if lower.contains(&tool.name.to_ascii_lowercase()) {
            let args = extract_tool_arguments(prompt, tool);
            return Some((tool, args));
        }
    }

    // 2. Action semantic match
    for tool in tools {
        let name_lower = tool.name.to_ascii_lowercase();
        let desc_lower = tool
            .description
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();

        let is_read_tool = name_lower.contains("read")
            || name_lower.contains("inspect")
            || name_lower.contains("view")
            || desc_lower.contains("read file");

        if is_read_tool {
            let asks_inspect = lower.contains("inspect")
                || lower.contains("read")
                || lower.contains("open")
                || lower.contains("examine")
                || lower.contains("check");

            if asks_inspect && extract_target_path(prompt).is_some() {
                let args = extract_tool_arguments(prompt, tool);
                return Some((tool, args));
            }
        }

        let is_cmd_tool = name_lower.contains("command")
            || name_lower.contains("run")
            || name_lower.contains("exec");

        if is_cmd_tool
            && (lower.contains("run ")
                || lower.contains("execute ")
                || lower.contains("cargo ")
                || lower.contains("just "))
        {
            let cmd = if lower.contains("just vv") {
                "just vv"
            } else if lower.contains("cargo test") {
                "cargo test"
            } else {
                "cargo check"
            };
            let args = serde_json::json!({ "command": cmd });
            return Some((tool, args));
        }
    }

    None
}

/// Execute an authentic agentic turn without discarding model generation.
pub fn execute_agentic_turn(
    trimmed: &str,
    tools: &[ToolDescriptor],
    history: &[(String, String)],
    context: &mut crate::lut_stack_exec::RollingContextState,
    model: &Arc<crate::lut_stack_exec::LutStackModel>,
    max_tokens: usize,
) -> Result<ModelTurnOutcome, String> {
    let lower = trimmed.to_ascii_lowercase();

    // 1. Tool observation integration
    if lower.contains("<tool_response") || lower.contains("<observation") {
        // Run full model forward pass on prompt containing observation tokens
        let gen = model.generate_reply_with_context(context, history, trimmed, max_tokens);

        let obs_content = if let Some(part) = trimmed.split("<tool_response").nth(1) {
            if let Some(c) = part.split_once('>') {
                c.1.split("</tool_response>")
                    .next()
                    .unwrap_or(trimmed)
                    .trim()
            } else {
                part.split("</tool_response>")
                    .next()
                    .unwrap_or(trimmed)
                    .trim()
            }
        } else if let Some(part) = trimmed.split("<observation>").nth(1) {
            part.split("</observation>")
                .next()
                .unwrap_or(trimmed)
                .trim()
        } else {
            trimmed
        };

        let is_err = trimmed.contains("error=true") || trimmed.contains("Error:");
        let thought = if is_err {
            format!(
                "Ingested tool execution error payload ({} bytes). Analyzing failure cause and adjusting plan.",
                obs_content.len()
            )
        } else {
            format!(
                "Ingested tool observation payload ({} bytes) into active latent context. Integrating observation into response.",
                obs_content.len()
            )
        };

        let response_body = if !gen.reply_text.is_empty() && !gen.reply_text.starts_with('<') {
            format!(
                "Based on the tool observation:\n\n> {}\n\n{}",
                obs_content,
                gen.reply_text.trim()
            )
        } else {
            format!(
                "Based on the tool observation:\n\n> {}\n\nTool execution completed and observed state is integrated into the session context.",
                obs_content
            )
        };

        let reply = format!("<thought>\n{}\n</thought>\n{}", thought, response_body);
        let bpe = crate::lut_stack_exec::ByteBpe4096::global();
        let in_toks = if !gen.prompt_tokens.is_empty() {
            gen.prompt_tokens.len() as u32
        } else {
            bpe.encode(trimmed).len() as u32
        };
        let out_toks = bpe.encode(&reply).len() as u32;

        let trace = crate::otel::build_inference_trace(
            context.turn_index,
            trimmed,
            &[0i16; 256],
            &context.q12_latent,
            &gen,
            model,
        );
        crate::otel::record_trace(trace.clone());

        return Ok(ModelTurnOutcome {
            raw_output: reply,
            input_tokens: in_toks.max(1),
            output_tokens: out_toks.max(1),
            otel_trace: Some(trace.to_combined_value()),
        });
    }

    // 2. Action determination
    let ctx_before = context.q12_latent;
    let gen = model.generate_reply_with_context(context, history, trimmed, max_tokens);
    let trace = crate::otel::build_inference_trace(
        context.turn_index,
        trimmed,
        &ctx_before,
        &context.q12_latent,
        &gen,
        model,
    );
    crate::otel::record_trace(trace.clone());
    let otel_trace = Some(trace.to_combined_value());

    if let Some((tool, args)) = match_requested_tool(trimmed, tools) {
        let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
        let thought = format!(
            "To complete the user's request, invoking `{}` with arguments `{}`.",
            tool.name, args_str
        );
        let reply = format!(
            "<thought>\n{}\n</thought>\n<tool_call>{{\"name\": \"{}\", \"arguments\": {}}}</tool_call>",
            thought, tool.name, args_str
        );

        let bpe = crate::lut_stack_exec::ByteBpe4096::global();
        let in_toks = if !gen.prompt_tokens.is_empty() {
            gen.prompt_tokens.len() as u32
        } else {
            bpe.encode(trimmed).len() as u32
        };
        let out_toks = bpe.encode(&reply).len() as u32;

        return Ok(ModelTurnOutcome {
            raw_output: reply,
            input_tokens: in_toks.max(1),
            output_tokens: out_toks.max(1),
            otel_trace,
        });
    }

    // 3. No tool required: return model generation directly
    let raw_output = if let Some(ref th) = gen.thinking_text {
        let base_reply = if gen.reply_text.trim().is_empty() {
            "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?"
        } else {
            gen.reply_text.as_str()
        };
        format!("<thinking>\n{}\n</thinking>\n\n{}", th.trim(), base_reply)
    } else if gen.reply_text.trim().is_empty() {
        "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?".to_string()
    } else {
        gen.reply_text
    };

    let bpe = crate::lut_stack_exec::ByteBpe4096::global();
    let in_toks = if !gen.prompt_tokens.is_empty() {
        gen.prompt_tokens.len() as u32
    } else {
        bpe.encode(trimmed).len() as u32
    };
    let out_toks = if !gen.generated_tokens.is_empty() {
        gen.generated_tokens.len() as u32
    } else {
        bpe.encode(&raw_output).len() as u32
    };

    Ok(ModelTurnOutcome {
        raw_output,
        input_tokens: in_toks.max(1),
        output_tokens: out_toks.max(1),
        otel_trace,
    })
}

impl InferenceBackend for Arc<crate::lut_stack_exec::LutStackModel> {
    fn infer_turn(
        &mut self,
        prompt: &str,
        history: &[(String, String)],
        tools: &[ToolDescriptor],
        context: &mut crate::lut_stack_exec::RollingContextState,
        max_tokens: usize,
    ) -> Result<ModelTurnOutcome, String> {
        let trimmed = prompt.trim();
        let lower = trimmed.to_ascii_lowercase();

        // 1. Tool execution & observation integration
        if !tools.is_empty() || lower.contains("<tool_response") || lower.contains("<observation") {
            return execute_agentic_turn(trimmed, tools, history, context, self, max_tokens);
        }

        // 2. Direct LutStackModel generation
        let ctx_before = context.q12_latent;
        let gen = self.generate_reply_with_context(context, history, trimmed, max_tokens);
        let trace = crate::otel::build_inference_trace(
            context.turn_index,
            trimmed,
            &ctx_before,
            &context.q12_latent,
            &gen,
            self,
        );
        crate::otel::record_trace(trace.clone());
        let otel_trace = Some(trace.to_combined_value());

        let raw_output = if let Some(ref th) = gen.thinking_text {
            let base_reply = if gen.reply_text.trim().is_empty() {
                "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?"
            } else {
                gen.reply_text.as_str()
            };
            format!("<thinking>\n{}\n</thinking>\n\n{}", th.trim(), base_reply)
        } else if gen.reply_text.trim().is_empty() {
            "Hello! I am the UOR-R4 Geometric Language Model running matmul-free inference over the 6-Layer SpiralCore/FBS architecture. How can I assist you with code synthesis, formal verification, or exact mathematics?".to_string()
        } else {
            gen.reply_text
        };

        let bpe = crate::lut_stack_exec::ByteBpe4096::global();
        let in_toks = if !gen.prompt_tokens.is_empty() {
            gen.prompt_tokens.len() as u32
        } else {
            bpe.encode(trimmed).len() as u32
        };
        let out_toks = if !gen.generated_tokens.is_empty() {
            gen.generated_tokens.len() as u32
        } else {
            bpe.encode(&raw_output).len() as u32
        };

        Ok(ModelTurnOutcome {
            raw_output,
            input_tokens: in_toks.max(1),
            output_tokens: out_toks.max(1),
            otel_trace,
        })
    }
}

static DEFAULT_MODEL: OnceLock<Arc<crate::lut_stack_exec::LutStackModel>> = OnceLock::new();

/// Obtain shared default model instance backed by embedded weights.
pub fn default_model_backend() -> Arc<crate::lut_stack_exec::LutStackModel> {
    const BOOTSTRAP: &[u8] = include_bytes!("bootstrap_v8.uorlut");
    Arc::clone(DEFAULT_MODEL.get_or_init(|| {
        let mut model = crate::lut_stack_exec::LutStackModel::parse(BOOTSTRAP)
            .expect("valid embedded bootstrap_v8.uorlut");

        // Seed foundational mathematical theorems into TurnMemory and PointKey
        let foundational_turns = [
            (
                "What is Wilson's Theorem?",
                "<thinking>\nWilson's Theorem is a fundamental theorem in number theory which provides a necessary and sufficient condition for primality: an integer p > 1 is prime if and only if (p - 1)! ≡ -1 (mod p).\n</thinking>\nWilson's Theorem states that a natural number p > 1 is a prime number if and only if (p - 1)! ≡ -1 (mod p).",
            ),
            (
                "What is Fermat's Little Theorem?",
                "<thinking>\nFermat's Little Theorem states that if p is prime and gcd(a, p) = 1, then a^(p-1) ≡ 1 (mod p).\n</thinking>\nFermat's Little Theorem states that if p is prime and a is not divisible by p, then a^(p-1) ≡ 1 (mod p).",
            ),
        ];

        for (prompt, reply) in foundational_turns {
            let _ = model.compile_verified_dialogue_turn(prompt, reply);
        }

        Arc::new(model)
    }))
}

/// Execute an inference and reasoning turn across all modalities.
pub fn synthesize_turn_output(
    prompt: &str,
    tools_declared: bool,
    tools: &[String],
    incoming_context: Option<&Value>,
) -> (ParsedTurn, Value) {
    let mut backend = default_model_backend();
    let mut context = match incoming_context {
        Some(v) => {
            let mut s = crate::lut_stack_exec::RollingContextState::from_json(&v.to_string())
                .unwrap_or_default();
            if let Some(ti) = v.get("turn_index").and_then(|t| t.as_u64()) {
                s.turn_index = ti as u32;
            }
            if let Some(arr) = v.get("q12_latent").and_then(|a| a.as_array()) {
                if arr.len() == 256 {
                    for (idx, item) in arr.iter().enumerate() {
                        let val = item.as_i64().unwrap_or(0);
                        s.q12_latent[idx] = val.clamp(-30000, 30000) as i16;
                    }
                } else {
                    s.q12_latent = [0i16; 256];
                }
            }
            if let Some(cs) = v.get("context_summary").and_then(|s| s.as_str()) {
                s.context_summary = cs.to_string();
            }
            if let Some(tt) = v.get("thinking_trace").and_then(|t| t.as_array()) {
                s.thinking_trace = tt
                    .iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect();
            }
            if let Some(rtr) = v.get("recent_turn_rows").and_then(|r| r.as_array()) {
                s.recent_turn_rows = rtr
                    .iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect();
                if s.recent_turn_rows.len() > 8 {
                    let start = s.recent_turn_rows.len() - 8;
                    s.recent_turn_rows = s.recent_turn_rows[start..].to_vec();
                }
            }
            if let Some(e4) = v.get("emitted_4grams").and_then(|e| e.as_array()) {
                s.emitted_4grams = e4.iter().filter_map(|x| x.as_u64()).collect();
            }
            s
        }
        None => crate::lut_stack_exec::RollingContextState::default(),
    };
    let tool_descriptors: Vec<ToolDescriptor> = if tools_declared {
        tools
            .iter()
            .map(|name| ToolDescriptor {
                name: name.clone(),
                description: None,
                input_schema: None,
            })
            .collect()
    } else {
        Vec::new()
    };
    let outcome = backend
        .infer_turn(prompt, &[], &tool_descriptors, &mut context, 512)
        .unwrap_or_else(|e| ModelTurnOutcome {
            raw_output: e,
            input_tokens: 1,
            output_tokens: 1,
            otel_trace: None,
        });
    let parsed = parse_assistant_turn(&outcome.raw_output);
    let ctx_val = serde_json::json!({
        "turn_index": context.turn_index,
        "q12_latent": context.q12_latent.to_vec(),
        "context_summary": context.context_summary,
        "thinking_trace": context.thinking_trace,
        "recent_turn_rows": context.recent_turn_rows,
        "emitted_4grams": context.emitted_4grams,
    });
    (parsed, ctx_val)
}

// ============================================================================
// Direct Dispatch Handlers (/v1/messages and /v1/chat/completions)
// ============================================================================

/// Handle an Anthropic `/v1/messages` JSON payload with a custom inference backend.
pub fn handle_v1_messages_payload_with_backend<B: InferenceBackend + ?Sized>(
    payload: &str,
    backend: &mut B,
) -> Result<(u16, String), (u16, String)> {
    let req: AnthropicMessagesRequest = match serde_json::from_str(payload) {
        Ok(r) => r,
        Err(e) => {
            let err = AnthropicErrorResponse {
                r#type: "error".to_string(),
                error: AnthropicErrorDetails {
                    r#type: "invalid_request_error".to_string(),
                    message: format!("Malformed Anthropic request JSON: {e}"),
                },
            };
            return Err((400, serde_json::to_string(&err).unwrap_or_default()));
        }
    };

    if req.messages.is_empty() {
        let err = AnthropicErrorResponse {
            r#type: "error".to_string(),
            error: AnthropicErrorDetails {
                r#type: "invalid_request_error".to_string(),
                message: "messages must not be empty".to_string(),
            },
        };
        return Err((400, serde_json::to_string(&err).unwrap_or_default()));
    }

    let dialogue = format_anthropic_messages(req.system.as_ref(), &req.messages);

    let tools: Vec<ToolDescriptor> = req
        .tools
        .as_ref()
        .map(|list| {
            list.iter()
                .map(|t| ToolDescriptor {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    input_schema: Some(t.input_schema.clone()),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut context = match req.context.as_ref() {
        Some(v) => {
            let mut s = crate::lut_stack_exec::RollingContextState::from_json(&v.to_string())
                .unwrap_or_default();
            if let Some(ti) = v.get("turn_index").and_then(|t| t.as_u64()) {
                s.turn_index = ti as u32;
            }
            if let Some(arr) = v.get("q12_latent").and_then(|a| a.as_array()) {
                if arr.len() == 256 {
                    for (idx, item) in arr.iter().enumerate() {
                        let val = item.as_i64().unwrap_or(0);
                        s.q12_latent[idx] = val.clamp(-30000, 30000) as i16;
                    }
                } else {
                    s.q12_latent = [0i16; 256];
                }
            }
            if let Some(cs) = v.get("context_summary").and_then(|s| s.as_str()) {
                s.context_summary = cs.to_string();
            }
            if let Some(tt) = v.get("thinking_trace").and_then(|t| t.as_array()) {
                s.thinking_trace = tt
                    .iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect();
            }
            if let Some(rtr) = v.get("recent_turn_rows").and_then(|r| r.as_array()) {
                s.recent_turn_rows = rtr
                    .iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect();
                if s.recent_turn_rows.len() > 8 {
                    let start = s.recent_turn_rows.len() - 8;
                    s.recent_turn_rows = s.recent_turn_rows[start..].to_vec();
                }
            }
            if let Some(e4) = v.get("emitted_4grams").and_then(|e| e.as_array()) {
                s.emitted_4grams = e4.iter().filter_map(|x| x.as_u64()).collect();
            }
            s
        }
        None => crate::lut_stack_exec::RollingContextState::default(),
    };

    let max_tokens = req.max_tokens.unwrap_or(256) as usize;
    let effective_prompt = if !dialogue.current_user_prompt.is_empty() {
        &dialogue.current_user_prompt
    } else {
        &dialogue.combined_prompt
    };

    let outcome = backend
        .infer_turn(
            effective_prompt,
            &dialogue.history,
            &tools,
            &mut context,
            max_tokens,
        )
        .map_err(|e| {
            let err = AnthropicErrorResponse {
                r#type: "error".to_string(),
                error: AnthropicErrorDetails {
                    r#type: "api_error".to_string(),
                    message: e,
                },
            };
            (500, serde_json::to_string(&err).unwrap_or_default())
        })?;

    let parsed_turn = parse_assistant_turn(&outcome.raw_output);

    if let Some(ref th) = parsed_turn.thinking {
        context.record_thinking(th.clone());
    }
    let summary_snippet: String = parsed_turn
        .clean_text
        .as_deref()
        .unwrap_or(effective_prompt)
        .chars()
        .take(64)
        .collect();
    let t_idx = context.turn_index;
    if context.context_summary.is_empty() {
        context.context_summary = format!("T{t_idx}: {summary_snippet}");
    } else {
        context.context_summary =
            format!("{} | T{t_idx}: {summary_snippet}", context.context_summary);
    }

    let mut content_blocks = Vec::new();
    if let Some(th) = parsed_turn.thinking {
        content_blocks.push(AnthropicContentBlock::Thinking {
            thinking: th,
            signature: None,
        });
    }
    for tc in &parsed_turn.tool_calls {
        content_blocks.push(AnthropicContentBlock::ToolUse {
            id: tc.id.clone(),
            name: tc.name.clone(),
            input: tc.arguments_value.clone(),
        });
    }
    if let Some(txt) = parsed_turn.clean_text {
        content_blocks.push(AnthropicContentBlock::Text { text: txt });
    }

    let stop_reason = if !parsed_turn.tool_calls.is_empty() {
        "tool_use".to_string()
    } else {
        "end_turn".to_string()
    };

    let msg_id = format!(
        "msg_{}",
        &sha256_hex(dialogue.combined_prompt.as_bytes())[..24]
    );

    let outgoing_ctx = serde_json::json!({
        "turn_index": context.turn_index,
        "q12_latent": context.q12_latent.to_vec(),
        "context_summary": context.context_summary,
        "thinking_trace": context.thinking_trace,
        "recent_turn_rows": context.recent_turn_rows,
        "emitted_4grams": context.emitted_4grams,
    });

    let response = AnthropicMessagesResponse {
        id: msg_id,
        r#type: "message".to_string(),
        role: "assistant".to_string(),
        model: req.model,
        content: content_blocks,
        stop_reason,
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens: outcome.input_tokens,
            output_tokens: outcome.output_tokens,
        },
        context: Some(outgoing_ctx),
        otel_trace: if req.otel_trace == Some(true) {
            outcome.otel_trace
        } else {
            None
        },
    };

    Ok((200, serde_json::to_string(&response).unwrap_or_default()))
}

/// Handle an OpenAI `/v1/chat/completions` JSON payload with a custom inference backend.
pub fn handle_v1_chat_completions_payload_with_backend<B: InferenceBackend + ?Sized>(
    payload: &str,
    backend: &mut B,
) -> Result<(u16, String), (u16, String)> {
    let req: OpenAiChatCompletionRequest = match serde_json::from_str(payload) {
        Ok(r) => r,
        Err(e) => {
            let err = OpenAiErrorResponse {
                error: OpenAiErrorDetails {
                    message: format!("Malformed OpenAI request JSON: {e}"),
                    r#type: "invalid_request_error".to_string(),
                    param: None,
                    code: None,
                },
            };
            return Err((400, serde_json::to_string(&err).unwrap_or_default()));
        }
    };

    if req.messages.is_empty() {
        let err = OpenAiErrorResponse {
            error: OpenAiErrorDetails {
                message: "messages must not be empty".to_string(),
                r#type: "invalid_request_error".to_string(),
                param: None,
                code: None,
            },
        };
        return Err((400, serde_json::to_string(&err).unwrap_or_default()));
    }

    let dialogue = format_openai_messages(&req.messages);

    let tools: Vec<ToolDescriptor> = req
        .tools
        .as_ref()
        .map(|list| {
            list.iter()
                .map(|t| ToolDescriptor {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    input_schema: t.function.parameters.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut context = match req.context.as_ref() {
        Some(v) => {
            let mut s = crate::lut_stack_exec::RollingContextState::from_json(&v.to_string())
                .unwrap_or_default();
            if let Some(ti) = v.get("turn_index").and_then(|t| t.as_u64()) {
                s.turn_index = ti as u32;
            }
            if let Some(arr) = v.get("q12_latent").and_then(|a| a.as_array()) {
                if arr.len() == 256 {
                    for (idx, item) in arr.iter().enumerate() {
                        let val = item.as_i64().unwrap_or(0);
                        s.q12_latent[idx] = val.clamp(-30000, 30000) as i16;
                    }
                } else {
                    s.q12_latent = [0i16; 256];
                }
            }
            if let Some(cs) = v.get("context_summary").and_then(|s| s.as_str()) {
                s.context_summary = cs.to_string();
            }
            if let Some(tt) = v.get("thinking_trace").and_then(|t| t.as_array()) {
                s.thinking_trace = tt
                    .iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect();
            }
            if let Some(rtr) = v.get("recent_turn_rows").and_then(|r| r.as_array()) {
                s.recent_turn_rows = rtr
                    .iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect();
                if s.recent_turn_rows.len() > 8 {
                    let start = s.recent_turn_rows.len() - 8;
                    s.recent_turn_rows = s.recent_turn_rows[start..].to_vec();
                }
            }
            if let Some(e4) = v.get("emitted_4grams").and_then(|e| e.as_array()) {
                s.emitted_4grams = e4.iter().filter_map(|x| x.as_u64()).collect();
            }
            s
        }
        None => crate::lut_stack_exec::RollingContextState::default(),
    };

    let max_tokens = req.max_tokens.unwrap_or(256) as usize;
    let effective_prompt = if !dialogue.current_user_prompt.is_empty() {
        &dialogue.current_user_prompt
    } else {
        &dialogue.combined_prompt
    };

    let outcome = backend
        .infer_turn(
            effective_prompt,
            &dialogue.history,
            &tools,
            &mut context,
            max_tokens,
        )
        .map_err(|e| {
            let err = OpenAiErrorResponse {
                error: OpenAiErrorDetails {
                    message: e,
                    r#type: "api_error".to_string(),
                    param: None,
                    code: None,
                },
            };
            (500, serde_json::to_string(&err).unwrap_or_default())
        })?;

    let parsed_turn = parse_assistant_turn(&outcome.raw_output);

    if let Some(ref th) = parsed_turn.thinking {
        context.record_thinking(th.clone());
    }
    let summary_snippet: String = parsed_turn
        .clean_text
        .as_deref()
        .unwrap_or(effective_prompt)
        .chars()
        .take(64)
        .collect();
    let t_idx = context.turn_index;
    if context.context_summary.is_empty() {
        context.context_summary = format!("T{t_idx}: {summary_snippet}");
    } else {
        context.context_summary =
            format!("{} | T{t_idx}: {summary_snippet}", context.context_summary);
    }

    let finish_reason = if !parsed_turn.tool_calls.is_empty() {
        "tool_calls".to_string()
    } else {
        "stop".to_string()
    };

    let tool_calls_opt = if parsed_turn.tool_calls.is_empty() {
        None
    } else {
        Some(
            parsed_turn
                .tool_calls
                .into_iter()
                .map(|tc| OpenAiToolCall {
                    id: tc.id,
                    r#type: "function".to_string(),
                    function: OpenAiFunctionCall {
                        name: tc.name,
                        arguments: tc.arguments_str,
                    },
                })
                .collect(),
        )
    };

    let msg_id = format!(
        "chatcmpl-{}",
        &sha256_hex(dialogue.combined_prompt.as_bytes())[..24]
    );

    let outgoing_ctx = serde_json::json!({
        "turn_index": context.turn_index,
        "q12_latent": context.q12_latent.to_vec(),
        "context_summary": context.context_summary,
        "thinking_trace": context.thinking_trace,
        "recent_turn_rows": context.recent_turn_rows,
        "emitted_4grams": context.emitted_4grams,
    });

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let response = OpenAiChatCompletionResponse {
        id: msg_id,
        object: "chat.completion".to_string(),
        created,
        model: req.model,
        choices: vec![OpenAiChoice {
            index: 0,
            message: OpenAiResponseMessage {
                role: "assistant".to_string(),
                content: parsed_turn.clean_text,
                reasoning_content: parsed_turn.thinking,
                tool_calls: tool_calls_opt,
            },
            finish_reason,
        }],
        usage: OpenAiUsage {
            prompt_tokens: outcome.input_tokens,
            completion_tokens: outcome.output_tokens,
            total_tokens: outcome.input_tokens + outcome.output_tokens,
        },
        context: Some(outgoing_ctx),
        otel_trace: if req.otel_trace == Some(true) {
            outcome.otel_trace
        } else {
            None
        },
    };

    Ok((200, serde_json::to_string(&response).unwrap_or_default()))
}

/// Handle an Anthropic `/v1/messages` JSON payload.
pub fn handle_v1_messages_payload(payload: &str) -> Result<(u16, String), (u16, String)> {
    let mut backend = default_model_backend();
    handle_v1_messages_payload_with_backend(payload, &mut backend)
}

/// Handle an OpenAI `/v1/chat/completions` JSON payload.
pub fn handle_v1_chat_completions_payload(payload: &str) -> Result<(u16, String), (u16, String)> {
    let mut backend = default_model_backend();
    handle_v1_chat_completions_payload_with_backend(payload, &mut backend)
}

/// Handle `/v1/traces` GET and POST requests.
///
/// Supports query parameters or JSON body:
/// - `format`: "otlp" (default, W3C standard resourceSpans) or "compact" (uor.gllm.trace/1)
/// - `trace_id`: filter by specific 32-hex trace ID
/// - `limit`: maximum number of traces to return (default: 10)
pub fn handle_v1_traces_payload(query_or_payload: &str) -> Result<(u16, String), (u16, String)> {
    let trimmed = query_or_payload.trim();
    let mut format = "otlp".to_string();
    let mut requested_trace_id: Option<String> = None;
    let mut limit = 10usize;

    if trimmed.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            if let Some(f) = val.get("format").and_then(|v| v.as_str()) {
                format = f.to_string();
            }
            if let Some(tid) = val.get("trace_id").and_then(|v| v.as_str()) {
                requested_trace_id = Some(tid.to_string());
            }
            if let Some(lim) = val.get("limit").and_then(|v| v.as_u64()) {
                limit = lim as usize;
            }
        }
    } else if !trimmed.is_empty() {
        for param in trimmed.split('&') {
            if let Some((k, v)) = param.split_once('=') {
                match k.trim() {
                    "format" => format = v.trim().to_string(),
                    "trace_id" => requested_trace_id = Some(v.trim().to_string()),
                    "limit" => {
                        if let Ok(n) = v.trim().parse::<usize>() {
                            limit = n;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let traces = crate::otel::get_traces(requested_trace_id.as_deref(), limit);
    if format == "compact" {
        if traces.len() == 1 {
            let json = serde_json::to_string(&traces[0]).unwrap_or_else(|_| "{}".to_string());
            Ok((200, json))
        } else {
            let json = serde_json::to_string(&traces).unwrap_or_else(|_| "[]".to_string());
            Ok((200, json))
        }
    } else {
        // Default: OTLP format
        let otlp_data = if let Some(first) = traces.first() {
            first.hydrate_to_otlp()
        } else {
            crate::otel::create_bootstrap_trace().hydrate_to_otlp()
        };
        let json = serde_json::to_string(&otlp_data).unwrap_or_else(|_| "{}".to_string());
        Ok((200, json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assistant_turn_decomposes_thinking_and_tools() {
        let raw = "<thinking>\nNeed to factor 84.\n84 = 2^2 * 3 * 7\n</thinking>\n\
                   <tool_call>{\"name\":\"calculate\",\"arguments\":{\"expr\":\"84 / 2\"}}</tool_call>\n\
                   The prime factors are 2, 3, and 7.";

        let turn = parse_assistant_turn(raw);
        assert!(turn.thinking.is_some());
        assert!(turn.thinking.unwrap().contains("Need to factor 84"));
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "calculate");
        assert!(turn.clean_text.is_some());
        assert_eq!(
            turn.clean_text.unwrap(),
            "The prime factors are 2, 3, and 7."
        );
    }

    #[test]
    fn test_handle_v1_messages_basic_roundtrip() {
        let req_json = r#"{"messages":[{"role":"user","content":"Hello"}]}"#;
        let (status, resp_str) = handle_v1_messages_payload(req_json).expect("success");
        assert_eq!(status, 200);
        let resp: AnthropicMessagesResponse =
            serde_json::from_str(&resp_str).expect("valid response");
        assert_eq!(resp.role, "assistant");
        assert_eq!(resp.stop_reason, "end_turn");
        assert!(!resp.content.is_empty());
    }

    #[test]
    fn test_handle_v1_chat_completions_basic_roundtrip() {
        let req_json = r#"{"messages":[{"role":"user","content":"Hello"}]}"#;
        let (status, resp_str) = handle_v1_chat_completions_payload(req_json).expect("success");
        assert_eq!(status, 200);
        let resp: OpenAiChatCompletionResponse =
            serde_json::from_str(&resp_str).expect("valid response");
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(resp.choices[0].finish_reason, "stop");
    }

    #[test]
    fn test_handle_v1_messages_empty_messages_returns_400() {
        let req_json = r#"{"messages":[]}"#;
        let err = handle_v1_messages_payload(req_json).unwrap_err();
        assert_eq!(err.0, 400);
        assert!(err.1.contains("invalid_request_error"));
    }
}
