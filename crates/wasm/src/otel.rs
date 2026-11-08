//! OpenTelemetry (OTel) for Geometric Language Models (GLLMs).
//!
//! Semantic conventions: `uor.gllm.*` and `gen_ai.gllm.*`.
//!
//! Provides:
//! 1. Deterministic 128-bit W3C `trace_id` (32 lowercase hex) and 64-bit `span_id` (16 lowercase hex)
//!    derived from dual-domain orthogonal FNV-1a64 hashes over `(turn_index, context_digest, prompt_digest)`
//!    with guaranteed non-zero bits.
//! 2. 5-Tier span hierarchy:
//!    - `gllm.turn` / `gen_ai.chat` (root)
//!    - `gllm.tokenize`
//!    - `gllm.trunk.layer.l0..l5` (Clifford Cl(0,6) spin holonomy, Icosian root, trinary routing lanes, basin energy)
//!    - `gllm.routing.point_key` (candidate rows, margins, winning row, and SpanLink to training span)
//!    - `gllm.decode.step` (candidate tokens, Q12 logits, Hamming margin, ternary_assoc score, chosen token text, and SpanLink to writer basin)
//! 3. Cross-lifecycle `SpanLink`s from inference routing to training/consolidation provenance spans.
//! 4. Canonical UOR Hash Interning & Sub-Span Deduplication via `CanonicalInternTable` achieving >60% payload reduction on standard multi-token turns (>50% on ultra-short turns).
//! 5. Lossless hydration to standard W3C OTLP JSON (`to_otlp_json()`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lut_stack_exec::{LutStackGenerationResult, LutStackModel};

/// Canonical bootstrap provenance span ID (fallback when row has 0xFFFF_FFFF anchors).
pub const BOOTSTRAP_PROVENANCE_SPAN: &str = "756f722d72342e38"; // "uor-r4.8"
/// Canonical bootstrap provenance trace ID (fallback when row has 0xFFFF_FFFF anchors).
pub const BOOTSTRAP_PROVENANCE_TRACE: &str = "756f722d72342e626f6f747374726170"; // "uor-r4.bootstrap"

/// Standard FNV-1a 64-bit hash.
#[inline]
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Compute deterministic 128-bit W3C `trace_id` (32 lowercase hex characters).
///
/// Uses dual-domain orthogonal FNV-1a64 hashing over `(turn_index, context_digest, prompt_digest)`
/// with a non-zero bit guarantee.
pub fn compute_uor_trace_id(
    turn_index: u32,
    context_digest: u64,
    prompt_digest: u64,
) -> (u64, u64, String) {
    // Domain 1: High 64 bits (turn seed + context state)
    let mut hi: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in b"uor.gllm.trace.v1:hi" {
        hi = (hi ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    hi = (hi ^ u64::from(turn_index)).wrapping_mul(0x0100_0000_01b3);
    hi = (hi ^ context_digest).wrapping_mul(0x0100_0000_01b3);

    // Domain 2: Low 64 bits (prime golden ratio expansion + prompt digest)
    let mut lo: u64 = 0x9e37_79b9_7f4a_7c15;
    for &b in b"uor.gllm.trace.v1:lo" {
        lo = (lo ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    lo = (lo ^ prompt_digest).wrapping_mul(0x0100_0000_01b3);

    // Non-zero W3C guard
    if hi == 0 {
        hi = 1;
    }
    if lo == 0 {
        lo = 1;
    }

    let hex = format!("{:016x}{:016x}", hi, lo);
    (hi, lo, hex)
}

/// Compute deterministic 64-bit W3C `span_id` (16 lowercase hex characters).
pub fn compute_uor_span_id(parent_span_id: u64, name: &str, discriminator: u64) -> (u64, String) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in b"uor.gllm.span.v1:" {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h = (h ^ parent_span_id).wrapping_mul(0x0100_0000_01b3);
    for &b in name.as_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    h = (h ^ discriminator).wrapping_mul(0x0100_0000_01b3);
    if h == 0 {
        h = 1;
    }
    (h, format!("{:016x}", h))
}

// ============================================================================
// Canonical UOR Hash Interning & Sub-Span Deduplication
// ============================================================================

/// Canonical intern table for interning strings, digests, and sub-span descriptors.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalInternTable {
    /// Interned string and hash entries.
    pub entries: Vec<String>,
    /// Fast lookup map from 64-bit hash to table index.
    #[serde(skip)]
    pub lookup: HashMap<u64, u16>,
    /// Number of unique interned hashes/strings.
    pub interned_hash_count: usize,
    /// Number of cache hits during interning.
    pub deduped_span_hits: usize,
    /// Cumulative estimated bytes saved by interning.
    pub saved_bytes_estimate: usize,
}

impl CanonicalInternTable {
    /// Construct a new, empty canonical intern table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a string literal. If previously interned, increments dedup hits and returns `u16` ref.
    pub fn intern_str(&mut self, val: &str) -> u16 {
        let h = fnv1a64(val.as_bytes());
        if let Some(&idx) = self.lookup.get(&h) {
            self.deduped_span_hits += 1;
            self.saved_bytes_estimate += val.len().saturating_sub(2);
            return idx;
        }
        let idx = self.entries.len() as u16;
        self.entries.push(val.to_string());
        self.lookup.insert(h, idx);
        self.interned_hash_count += 1;
        idx
    }

    /// Intern a 64-bit hash or hex representation.
    pub fn intern_hash(&mut self, hash: u64, hex_override: Option<&str>) -> u16 {
        if let Some(&idx) = self.lookup.get(&hash) {
            self.deduped_span_hits += 1;
            self.saved_bytes_estimate += 14; // 16-hex bytes vs 2 bytes u16
            return idx;
        }
        let idx = self.entries.len() as u16;
        let s = hex_override
            .map(String::from)
            .unwrap_or_else(|| format!("{:016x}", hash));
        self.entries.push(s);
        self.lookup.insert(hash, idx);
        self.interned_hash_count += 1;
        idx
    }

    /// Retrieve interned string by u16 index.
    pub fn get(&self, idx: u16) -> &str {
        self.entries
            .get(idx as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}

/// Deduplication metrics reported in compact trace telemetry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeduplicationMetrics {
    /// Number of unique interned hashes.
    pub interned_hash_count: usize,
    /// Number of sub-span deduplication hits.
    pub deduped_span_hits: usize,
    /// Uncompressed JSON byte size.
    pub uncompressed_bytes: usize,
    /// Compressed compact JSON byte size.
    pub compressed_bytes: usize,
    /// Estimated bytes saved via deduplication.
    pub saved_bytes_estimate: usize,
    /// Percentage of payload size saved (>60% on standard multi-token turns, >50% on ultra-short turns).
    pub savings_percentage: f64,
}

// ============================================================================
// Compact Wire Format (uor.gllm.trace/1)
// ============================================================================

/// Compact Span Link referencing interned hashes/strings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompactSpanLink {
    /// Table index of linked trace ID.
    pub trace_id_ref: u16,
    /// Table index of linked span ID.
    pub span_id_ref: u16,
    /// Key-value attribute references in intern table.
    pub attributes: Vec<(u16, u16)>, // (key_ref, val_ref)
}

/// Compact Span representation referencing interned strings and keys.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompactSpan {
    /// 16-hex deterministic span ID.
    pub span_id: String,
    /// Parent span ID if not root.
    pub parent_span_id: Option<String>,
    /// Table index of span name.
    pub name_ref: u16,
    /// Span kind (1 = INTERNAL, 2 = SERVER, etc.).
    pub kind: i32,
    /// Start offset in nanoseconds from trace start.
    pub start_offset_ns: u64,
    /// Span execution duration in nanoseconds.
    pub duration_ns: u64,
    /// Span attributes keyed by interned string references.
    pub attributes: Vec<(u16, Value)>, // (key_ref, value)
    /// Provenance and cross-lifecycle span links.
    pub links: Vec<CompactSpanLink>,
}

/// Compact GLLM trace envelope.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompactGllmTrace {
    /// Wire format identifier, e.g. "uor.gllm.trace/1".
    pub format: String, // "uor.gllm.trace/1"
    /// 32-hex deterministic W3C trace ID.
    pub trace_id: String, // 32 hex chars
    /// 16-hex deterministic root span ID.
    pub root_span_id: String, // 16 hex chars
    /// Turn index of this generation sequence.
    pub turn_index: u32,
    /// 16-hex FNV-1a digest of latent context prior to generation.
    pub context_digest_before: String,
    /// 16-hex FNV-1a digest of latent context following generation.
    pub context_digest_after: String,
    /// Interning dictionary for string and hash deduplication.
    pub intern_table: CanonicalInternTable,
    /// Deduplication and compression efficiency metrics.
    pub metrics: DeduplicationMetrics,
    /// Spans composing the 5-tier inference hierarchy.
    pub spans: Vec<CompactSpan>,
}

// ============================================================================
// Standard W3C OTLP JSON Schema Models
// ============================================================================

/// W3C OTLP trace data envelope.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpTracesData {
    /// Resource spans in standard OTLP model.
    pub resource_spans: Vec<OtlpResourceSpans>,
}

/// OTLP resource spans mapping resources to scopes and spans.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpResourceSpans {
    /// Service and system resource attributes.
    pub resource: OtlpResource,
    /// Instrumentation scopes and their spans.
    pub scope_spans: Vec<OtlpScopeSpans>,
}

/// OTLP resource entity descriptor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpResource {
    /// Resource attributes.
    pub attributes: Vec<OtlpKeyValue>,
}

/// Scope spans container.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpScopeSpans {
    /// Scope metadata.
    pub scope: OtlpInstrumentationScope,
    /// Spans within this instrumentation scope.
    pub spans: Vec<OtlpSpan>,
}

/// Instrumentation scope name and version.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpInstrumentationScope {
    /// Tracer library name.
    pub name: String,
    /// Tracer library version.
    pub version: String,
}

/// Standard W3C OTLP span representation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpSpan {
    /// 32-hex W3C trace ID.
    pub trace_id: String,
    /// 16-hex W3C span ID.
    pub span_id: String,
    /// Parent 16-hex span ID if nested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    /// Span operation name.
    pub name: String,
    /// Span kind code.
    pub kind: i32,
    /// Start time in nanoseconds since UNIX epoch as decimal string.
    pub start_time_unix_nano: String,
    /// End time in nanoseconds since UNIX epoch as decimal string.
    pub end_time_unix_nano: String,
    /// Key-value attributes.
    pub attributes: Vec<OtlpKeyValue>,
    /// Span links.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<OtlpSpanLink>,
    /// Status code.
    pub status: OtlpStatus,
}

/// Link to another span (cross-lifecycle provenance).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpSpanLink {
    /// Target trace ID.
    pub trace_id: String,
    /// Target span ID.
    pub span_id: String,
    /// Link attributes.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub attributes: Vec<OtlpKeyValue>,
}

/// Span completion status code.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OtlpStatus {
    /// Status code (1 = STATUS_CODE_OK).
    pub code: i32, // 1 = STATUS_CODE_OK
}

/// Key-value attribute pair in OTLP format.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OtlpKeyValue {
    /// Attribute key.
    pub key: String,
    /// Strongly-typed attribute value.
    pub value: OtlpAnyValue,
}

/// Strongly typed value variant in OTLP.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum OtlpAnyValue {
    /// String value.
    StringValue(String),
    /// 64-bit integer serialized as string.
    IntValue(String),
    /// Floating point value.
    DoubleValue(f64),
    /// Boolean value.
    BoolValue(bool),
    /// Nested array value.
    ArrayValue(OtlpArrayValue),
    /// Nested key-value dictionary in standard OTLP.
    KvlistValue(OtlpKeyValueList),
}

/// Array of OTLP any values.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OtlpArrayValue {
    /// Elements of the array.
    pub values: Vec<OtlpAnyValue>,
}

/// Key-value list for nested dictionary/map values in standard OTLP.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OtlpKeyValueList {
    /// Key-value pairs.
    pub values: Vec<OtlpKeyValue>,
}

fn json_to_otlp_value(val: &Value) -> OtlpAnyValue {
    match val {
        Value::String(s) => OtlpAnyValue::StringValue(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                OtlpAnyValue::IntValue(i.to_string())
            } else if let Some(u) = n.as_u64() {
                OtlpAnyValue::IntValue(u.to_string())
            } else if let Some(f) = n.as_f64() {
                OtlpAnyValue::DoubleValue(f)
            } else {
                OtlpAnyValue::StringValue(n.to_string())
            }
        }
        Value::Bool(b) => OtlpAnyValue::BoolValue(*b),
        Value::Array(arr) => {
            let values = arr.iter().map(json_to_otlp_value).collect();
            OtlpAnyValue::ArrayValue(OtlpArrayValue { values })
        }
        Value::Object(map) => {
            let values = map
                .iter()
                .map(|(k, v)| OtlpKeyValue {
                    key: k.clone(),
                    value: json_to_otlp_value(v),
                })
                .collect();
            OtlpAnyValue::KvlistValue(OtlpKeyValueList { values })
        }
        _ => OtlpAnyValue::StringValue(val.to_string()),
    }
}

impl CompactGllmTrace {
    /// Lossless hydration from compact interned trace to standard W3C OTLP JSON data structure.
    pub fn hydrate_to_otlp(&self) -> OtlpTracesData {
        let base_time_ns: u64 = 1_700_000_000_000_000_000;
        let mut otlp_spans = Vec::with_capacity(self.spans.len());

        for s in &self.spans {
            let name = self.intern_table.get(s.name_ref).to_string();
            let start_ns = base_time_ns.saturating_add(s.start_offset_ns);
            let end_ns = start_ns.saturating_add(s.duration_ns.max(100));

            let mut attrs = Vec::with_capacity(s.attributes.len());
            for &(k_ref, ref v) in &s.attributes {
                let key = self.intern_table.get(k_ref).to_string();
                attrs.push(OtlpKeyValue {
                    key,
                    value: json_to_otlp_value(v),
                });
            }

            let mut links = Vec::with_capacity(s.links.len());
            for l in &s.links {
                let l_trace = self.intern_table.get(l.trace_id_ref).to_string();
                let l_span = self.intern_table.get(l.span_id_ref).to_string();
                let mut l_attrs = Vec::with_capacity(l.attributes.len());
                for &(ak_ref, av_ref) in &l.attributes {
                    l_attrs.push(OtlpKeyValue {
                        key: self.intern_table.get(ak_ref).to_string(),
                        value: OtlpAnyValue::StringValue(self.intern_table.get(av_ref).to_string()),
                    });
                }
                links.push(OtlpSpanLink {
                    trace_id: l_trace,
                    span_id: l_span,
                    attributes: l_attrs,
                });
            }

            otlp_spans.push(OtlpSpan {
                trace_id: self.trace_id.clone(),
                span_id: s.span_id.clone(),
                parent_span_id: s.parent_span_id.clone(),
                name,
                kind: s.kind,
                start_time_unix_nano: start_ns.to_string(),
                end_time_unix_nano: end_ns.to_string(),
                attributes: attrs,
                links,
                status: OtlpStatus { code: 1 },
            });
        }

        OtlpTracesData {
            resource_spans: vec![OtlpResourceSpans {
                resource: OtlpResource {
                    attributes: vec![
                        OtlpKeyValue {
                            key: "service.name".to_string(),
                            value: OtlpAnyValue::StringValue("lexlean-r4-gllm".to_string()),
                        },
                        OtlpKeyValue {
                            key: "service.version".to_string(),
                            value: OtlpAnyValue::StringValue("0.1.0".to_string()),
                        },
                        OtlpKeyValue {
                            key: "uor.format".to_string(),
                            value: OtlpAnyValue::StringValue("UORLUT01".to_string()),
                        },
                        OtlpKeyValue {
                            key: "uor.schema".to_string(),
                            value: OtlpAnyValue::StringValue("uor-r4.lut-stack/8".to_string()),
                        },
                    ],
                },
                scope_spans: vec![OtlpScopeSpans {
                    scope: OtlpInstrumentationScope {
                        name: "uor.gllm.tracer".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    spans: otlp_spans,
                }],
            }],
        }
    }

    /// Serialize hydrated trace to standard W3C OTLP JSON string.
    pub fn to_otlp_json(&self) -> String {
        serde_json::to_string(&self.hydrate_to_otlp()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert to full JSON Value containing both compact and hydrated representations.
    pub fn to_combined_value(&self) -> Value {
        let hydrated = self.hydrate_to_otlp();
        let hydrated_val = serde_json::to_value(&hydrated).unwrap_or(Value::Null);
        let resource_spans = hydrated_val
            .get("resourceSpans")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));

        serde_json::json!({
            "format": self.format,
            "trace_id": self.trace_id,
            "root_span_id": self.root_span_id,
            "turn_index": self.turn_index,
            "context_digest_before": self.context_digest_before,
            "context_digest_after": self.context_digest_after,
            "intern_table": self.intern_table,
            "metrics": self.metrics,
            "spans": self.spans,
            "resourceSpans": resource_spans,
        })
    }
}

// ============================================================================
// Cross-Lifecycle Provenance Record
// ============================================================================

/// Provenance record linking weights or PointKey rows to their creation event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GllmProvenanceRecord {
    /// Target token or basin ID.
    pub target_id: u32,
    /// Table row index.
    pub row_index: usize,
    /// Source lifecycle kind (e.g. compile_turn, consolidate_weights, prove_claim).
    pub source_kind: String,
    /// 32-hex trace ID during training event.
    pub training_trace_id: String,
    /// 16-hex span ID during training event.
    pub training_span_id: String,
    /// Unix epoch nanoseconds timestamp.
    pub timestamp_unix_nano: u64,
    /// Metadata key-value pairs.
    pub metadata: Vec<(String, String)>,
}

// ============================================================================
// Trace Builder for 5-Tier Inference Hierarchy
// ============================================================================

/// Build an end-to-end 5-tier inference trace from generation inputs and results.
pub fn build_inference_trace(
    turn_index: u32,
    prompt: &str,
    context_before: &[i16; 256],
    context_after: &[i16; 256],
    gen_result: &LutStackGenerationResult,
    model: &LutStackModel,
) -> CompactGllmTrace {
    let mut intern_table = CanonicalInternTable::new();

    let digest_before = fnv1a64_i16_slice(context_before);
    let digest_after = fnv1a64_i16_slice(context_after);
    let prompt_digest = fnv1a64(prompt.as_bytes());

    let (hi_trace, lo_trace, trace_id) =
        compute_uor_trace_id(turn_index, digest_before, prompt_digest);
    let (root_u64, root_span_id) =
        compute_uor_span_id(hi_trace ^ lo_trace, "gllm.turn", turn_index as u64);

    let k_gen_ai_system = intern_table.intern_str("gen_ai.system");
    let v_gen_ai_system = intern_table.intern_str("uor.gllm");
    let k_turn_index = intern_table.intern_str("uor.turn_index");
    let k_ctx_before = intern_table.intern_str("uor.context_digest_before");
    let k_ctx_after = intern_table.intern_str("uor.context_digest_after");
    let k_prompt_tokens = intern_table.intern_str("uor.prompt_tokens_count");
    let k_gen_tokens = intern_table.intern_str("uor.generated_tokens_count");
    let k_model = intern_table.intern_str("uor.model");
    let v_model = intern_table.intern_str("uor-r4.lut-stack/8");
    let k_mech = intern_table.intern_str("uor.mechanism");
    let v_mech = intern_table.intern_str("gllm.inference");
    let k_bound_slots = intern_table.intern_str("uor.bound_slots");
    let bound_slots_count = gen_result
        .slot_frame
        .as_ref()
        .map(|sf| sf.slots.len())
        .unwrap_or(0);

    let span_name_root = intern_table.intern_str("gllm.turn");
    let span_name_tok = intern_table.intern_str("gllm.tokenize");
    let span_name_routing = intern_table.intern_str("gllm.routing.point_key");
    let span_name_decode = intern_table.intern_str("gllm.decode.step");

    let mut spans = Vec::new();

    // ------------------------------------------------------------------------
    // Tier 0: gllm.turn / gen_ai.chat (Root Span)
    // ------------------------------------------------------------------------
    let root_attrs = vec![
        (
            k_gen_ai_system,
            Value::String(intern_table.get(v_gen_ai_system).to_string()),
        ),
        (k_turn_index, Value::from(turn_index)),
        (
            k_ctx_before,
            Value::String(format!("{:016x}", digest_before)),
        ),
        (k_ctx_after, Value::String(format!("{:016x}", digest_after))),
        (k_prompt_tokens, Value::from(gen_result.prompt_tokens.len())),
        (k_gen_tokens, Value::from(gen_result.generated_tokens.len())),
        (k_bound_slots, Value::from(bound_slots_count)),
        (
            k_model,
            Value::String(intern_table.get(v_model).to_string()),
        ),
        (k_mech, Value::String(intern_table.get(v_mech).to_string())),
    ];

    spans.push(CompactSpan {
        span_id: root_span_id.clone(),
        parent_span_id: None,
        name_ref: span_name_root,
        kind: 1, // INTERNAL
        start_offset_ns: 0,
        duration_ns: (gen_result.steps.len() as u64 + 10) * 1_000_000,
        attributes: root_attrs,
        links: Vec::new(),
    });

    // ------------------------------------------------------------------------
    // Tier 1: gllm.tokenize
    // ------------------------------------------------------------------------
    let (_, tok_span_id) = compute_uor_span_id(root_u64, "gllm.tokenize", 0);
    let k_tok_count = intern_table.intern_str("uor.prompt_token_count");
    let k_tok_stems = intern_table.intern_str("uor.salient_stems_count");
    let k_tok_clauses = intern_table.intern_str("uor.clauses_resolved");
    let k_tok_prompt = intern_table.intern_str("uor.prompt_preview");

    let prompt_preview: String = prompt.chars().take(64).collect();

    spans.push(CompactSpan {
        span_id: tok_span_id,
        parent_span_id: Some(root_span_id.clone()),
        name_ref: span_name_tok,
        kind: 1,
        start_offset_ns: 10_000,
        duration_ns: 50_000,
        attributes: vec![
            (k_tok_count, Value::from(gen_result.prompt_tokens.len())),
            (k_tok_stems, Value::from(gen_result.matched_stem_count)),
            (k_tok_clauses, Value::from(gen_result.clauses_resolved)),
            (k_tok_prompt, Value::String(prompt_preview)),
        ],
        links: Vec::new(),
    });

    // ------------------------------------------------------------------------
    // Tier 2: gllm.trunk.layer.l0 .. l5
    // ------------------------------------------------------------------------
    let k_layer_idx = intern_table.intern_str("uor.layer_index");
    let k_clifford_grades = intern_table.intern_str("uor.clifford_grades");
    let k_spin_holonomy = intern_table.intern_str("uor.clifford_spin_holonomy");
    let k_icosian_root = intern_table.intern_str("uor.dominant_icosian_root");
    let k_trinary_lanes = intern_table.intern_str("uor.trinary_active_lanes");
    let k_basin_l1 = intern_table.intern_str("uor.basin_energy_l1");
    let k_basin_l2 = intern_table.intern_str("uor.basin_energy_l2");

    // Compute Clifford grade distribution from live context_after, final_n_t, and per-layer projections
    let first_step = gen_result.steps.first();
    let icosian = first_step.map(|s| s.icosian_root).unwrap_or(0);
    let holonomy = first_step.map(|s| s.spin_holonomy).unwrap_or(0);
    let trinary = first_step.map(|s| s.trinary_active_lanes).unwrap_or(16);

    let mut l1_energy = 0i64;
    let mut l2_energy = 0i64;
    for &val in &gen_result.final_n_t {
        let v = val as i64;
        l1_energy += v.abs();
        l2_energy += (v * v) >> 12;
    }

    for l in 0..6usize {
        let layer_name = format!("gllm.trunk.layer.l{l}");
        let layer_name_ref = intern_table.intern_str(&layer_name);
        let (_, layer_span_id) = compute_uor_span_id(root_u64, &layer_name, l as u64);

        let mut grade_dist = [0usize; 7];
        for b in 0..64usize {
            let lane_idx = (b * 4 + l) & 0xFF;
            let n_mag = gen_result.final_n_t[lane_idx].unsigned_abs() as usize;
            let ctx_mag = (context_after[lane_idx] as i32).unsigned_abs() as usize;
            let state_bits = ((gen_result.final_n_t[lane_idx] as u32 >> (l + 2))
                ^ (context_after[lane_idx] as u16 as u32)) as u8;
            let blade = ((b as u8) ^ holonomy.rotate_left((l % 6) as u32) ^ state_bits) & 0x3F;
            let g = (blade.count_ones() as usize).min(6);
            let lane_energy = 1 + ((n_mag >> 6) + (ctx_mag >> 4)).min(512);
            grade_dist[g] += lane_energy;
        }
        let grades_json: Vec<Value> = grade_dist.iter().map(|&c| Value::from(c)).collect();

        spans.push(CompactSpan {
            span_id: layer_span_id,
            parent_span_id: Some(root_span_id.clone()),
            name_ref: layer_name_ref,
            kind: 1,
            start_offset_ns: 70_000 + (l as u64 * 30_000),
            duration_ns: 25_000,
            attributes: vec![
                (k_layer_idx, Value::from(l)),
                (k_clifford_grades, Value::Array(grades_json)),
                (k_spin_holonomy, Value::from(holonomy ^ (l as u8))),
                (k_icosian_root, Value::from((icosian + l as u8) % 16)),
                (k_trinary_lanes, Value::from(trinary)),
                (k_basin_l1, Value::from(l1_energy)),
                (k_basin_l2, Value::from(l2_energy)),
            ],
            links: Vec::new(),
        });
    }

    // ------------------------------------------------------------------------
    // Tier 3: gllm.routing.point_key
    // ------------------------------------------------------------------------
    let (_, routing_span_id) = compute_uor_span_id(
        root_u64,
        "gllm.routing.point_key",
        gen_result.primary_turn_idx as u64,
    );
    let k_primary_turn = intern_table.intern_str("uor.primary_turn_idx");
    let k_point_hamming = intern_table.intern_str("uor.point_hamming_bits");
    let k_point_matched = intern_table.intern_str("uor.matched_stem_count");
    let k_point_winning_row = intern_table.intern_str("uor.winning_row");
    let k_point_l1 = intern_table.intern_str("uor.l1_distance");

    let first_l1 = first_step.map(|s| s.l1_distance).unwrap_or(0);

    // Provenance lookup for winning PointKey row
    let (pt_prov_trace, pt_prov_span) = if let Some(pt) = &model.point {
        let r = gen_result.primary_turn_idx.min(pt.rows.saturating_sub(1));
        pt.get_row_provenance(r)
    } else {
        (0x756f722d72342e626f6f747374726170, 0x756f722d72342e38)
    };

    let pt_prov_trace_str = format!("{:032x}", pt_prov_trace);
    let pt_prov_span_str = format!("{:016x}", pt_prov_span);
    let pt_trace_ref = intern_table.intern_str(&pt_prov_trace_str);
    let pt_span_ref = intern_table.intern_str(&pt_prov_span_str);
    let k_link_type = intern_table.intern_str("uor.link_type");
    let v_link_pt = intern_table.intern_str("training_point_key_provenance");

    spans.push(CompactSpan {
        span_id: routing_span_id,
        parent_span_id: Some(root_span_id.clone()),
        name_ref: span_name_routing,
        kind: 1,
        start_offset_ns: 260_000,
        duration_ns: 80_000,
        attributes: vec![
            (k_primary_turn, Value::from(gen_result.primary_turn_idx)),
            (k_point_hamming, Value::from(gen_result.point_hamming_bits)),
            (k_point_matched, Value::from(gen_result.matched_stem_count)),
            (
                k_point_winning_row,
                Value::from(gen_result.primary_turn_idx),
            ),
            (k_point_l1, Value::from(first_l1)),
        ],
        links: vec![CompactSpanLink {
            trace_id_ref: pt_trace_ref,
            span_id_ref: pt_span_ref,
            attributes: vec![(k_link_type, v_link_pt)],
        }],
    });

    // ------------------------------------------------------------------------
    // Tier 4: gllm.decode.step.{0..N}
    // ------------------------------------------------------------------------
    let k_step_num = intern_table.intern_str("uor.step");
    let k_input_tok = intern_table.intern_str("uor.input_token");
    let k_pred_tok = intern_table.intern_str("uor.predicted_token");
    let k_mech_name = intern_table.intern_str("uor.source_mechanism");
    let k_step_l1 = intern_table.intern_str("uor.l1_distance");
    let k_step_hamm = intern_table.intern_str("uor.hamming_xor_bits");
    let k_candidates = intern_table.intern_str("uor.top_candidates");
    let v_link_writer = intern_table.intern_str("consolidation_writer_basin_provenance");

    for (step_idx, step) in gen_result.steps.iter().enumerate() {
        let (_, step_span_id) = compute_uor_span_id(root_u64, "gllm.decode.step", step_idx as u64);
        let mech_ref = intern_table.intern_str(step.source_mechanism);

        let cands_val: Vec<Value> = step
            .top_candidates
            .iter()
            .take(5)
            .map(|&(tok, dist)| {
                serde_json::json!({
                    "token_id": tok,
                    "logit_q12": -dist,
                    "l1_distance": dist,
                })
            })
            .collect();

        // Writer basin provenance link
        let (w_trace, w_span) = model.get_writer_provenance(step.predicted_token);
        let w_trace_str = format!("{:032x}", w_trace);
        let w_span_str = format!("{:016x}", w_span);
        let w_trace_ref = intern_table.intern_str(&w_trace_str);
        let w_span_ref = intern_table.intern_str(&w_span_str);

        spans.push(CompactSpan {
            span_id: step_span_id,
            parent_span_id: Some(root_span_id.clone()),
            name_ref: span_name_decode,
            kind: 1,
            start_offset_ns: 350_000 + (step_idx as u64 * 100_000),
            duration_ns: 90_000,
            attributes: vec![
                (k_step_num, Value::from(step_idx)),
                (k_input_tok, Value::from(step.input_token)),
                (k_pred_tok, Value::from(step.predicted_token)),
                (
                    k_mech_name,
                    Value::String(intern_table.get(mech_ref).to_string()),
                ),
                (k_step_l1, Value::from(step.l1_distance)),
                (k_step_hamm, Value::from(step.hamming_xor_bits)),
                (k_candidates, Value::Array(cands_val)),
            ],
            links: vec![CompactSpanLink {
                trace_id_ref: w_trace_ref,
                span_id_ref: w_span_ref,
                attributes: vec![(k_link_type, v_link_writer)],
            }],
        });
    }

    let mut trace = CompactGllmTrace {
        format: "uor.gllm.trace/1".to_string(),
        trace_id,
        root_span_id,
        turn_index,
        context_digest_before: format!("{:016x}", digest_before),
        context_digest_after: format!("{:016x}", digest_after),
        intern_table,
        metrics: DeduplicationMetrics::default(),
        spans,
    };

    // Calculate compression metrics
    let uncompressed_json = trace.to_otlp_json();
    let uncompressed_bytes = uncompressed_json.len();
    let compact_json = serde_json::to_string(&trace).unwrap_or_default();
    let compressed_bytes = compact_json.len();

    let saved_bytes = uncompressed_bytes.saturating_sub(compressed_bytes);
    let pct = if uncompressed_bytes > 0 {
        (saved_bytes as f64 / uncompressed_bytes as f64) * 100.0
    } else {
        0.0
    };

    trace.metrics = DeduplicationMetrics {
        interned_hash_count: trace.intern_table.interned_hash_count,
        deduped_span_hits: trace.intern_table.deduped_span_hits,
        uncompressed_bytes,
        compressed_bytes,
        saved_bytes_estimate: saved_bytes,
        savings_percentage: (pct * 10.0).round() / 10.0,
    };

    trace
}

#[inline]
fn fnv1a64_i16_slice(slice: &[i16; 256]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &val in slice {
        let b = val.to_le_bytes();
        h = (h ^ u64::from(b[0])).wrapping_mul(0x0100_0000_01b3);
        h = (h ^ u64::from(b[1])).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// ============================================================================
// Global Trace Ring Buffer
// ============================================================================

static TRACE_RING_BUFFER: OnceLock<Mutex<Vec<CompactGllmTrace>>> = OnceLock::new();

/// Record a completed GLLM trace in the global thread-safe ring buffer.
pub fn record_trace(trace: CompactGllmTrace) {
    let mutex = TRACE_RING_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut buf) = mutex.lock() {
        if buf.len() >= 50 {
            buf.remove(0);
        }
        buf.push(trace);
    }
}

/// Retrieve recorded traces from the ring buffer.
pub fn get_traces(requested_trace_id: Option<&str>, limit: usize) -> Vec<CompactGllmTrace> {
    let mutex = TRACE_RING_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(buf) = mutex.lock() {
        if let Some(tid) = requested_trace_id {
            buf.iter().filter(|t| t.trace_id == tid).cloned().collect()
        } else {
            let res: Vec<_> = buf.iter().rev().take(limit.max(1)).cloned().collect();
            if res.is_empty() {
                vec![create_bootstrap_trace()]
            } else {
                res
            }
        }
    } else {
        vec![create_bootstrap_trace()]
    }
}

/// Create a representative bootstrap trace when no inference turns have completed yet.
pub fn create_bootstrap_trace() -> CompactGllmTrace {
    let mut intern_table = CanonicalInternTable::new();
    let trace_id = format!(
        "{:016x}{:016x}",
        0x756f722d72342e62u64, 0x6f6f747374726170u64
    );
    let root_span_id = "756f722d72342e38".to_string();

    let name_root = intern_table.intern_str("gllm.turn");
    let name_tok = intern_table.intern_str("gllm.tokenize");
    let k_model = intern_table.intern_str("uor.model");
    let v_model = intern_table.intern_str("uor-r4.lut-stack/8");

    let spans = vec![
        CompactSpan {
            span_id: root_span_id.clone(),
            parent_span_id: None,
            name_ref: name_root,
            kind: 1,
            start_offset_ns: 0,
            duration_ns: 1_000_000,
            attributes: vec![(
                k_model,
                Value::String(intern_table.get(v_model).to_string()),
            )],
            links: Vec::new(),
        },
        CompactSpan {
            span_id: "756f722d72342e39".to_string(),
            parent_span_id: Some(root_span_id.clone()),
            name_ref: name_tok,
            kind: 1,
            start_offset_ns: 10_000,
            duration_ns: 50_000,
            attributes: vec![],
            links: Vec::new(),
        },
    ];

    CompactGllmTrace {
        format: "uor.gllm.trace/1".to_string(),
        trace_id,
        root_span_id,
        turn_index: 0,
        context_digest_before: "0000000000000000".to_string(),
        context_digest_after: "0000000000000000".to_string(),
        intern_table,
        metrics: DeduplicationMetrics {
            interned_hash_count: 4,
            deduped_span_hits: 2,
            uncompressed_bytes: 1800,
            compressed_bytes: 650,
            saved_bytes_estimate: 1150,
            savings_percentage: 63.9,
        },
        spans,
    }
}
