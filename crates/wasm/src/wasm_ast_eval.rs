//! In-Wasm Rust AST Parser, Static Typechecker, Fuel-Bounded Interpreter, and Live Test Runner (`R4-32`).
//!
//! Implements the runtime engine for `R4.WasmAstEval` (`src/R4/WasmAstEval.lex.tex`):
//! - Lexer and delimiter balance verifier with 1-based line diagnostics (`UnbalancedDelimiterAtLine`)
//! - Recursive-descent parser for `struct`, `impl`, `fn`, `#[test]`, `let`, assignments, `if/else`,
//!   `while`, `for`, `return`, `assert!`, `assert_eq!`, `assert_ne!`, and arithmetic/bitwise/method
//!   expressions (`SyntaxErrorAtLine`)
//! - Static typechecker verifying variable annotations, operator operand types, boolean conditions,
//!   and function return signatures (`TypeErrorAtLine`)
//! - Fuel-bounded evaluator calling `R4.WasmAstEval.evaluate_ast_step` and `verify_test_assertion`
//!   so `cargo test`, `cargo check`, `cargo clippy`, and `cargo run` in `lexlean-sh` genuinely
//!   parse, typecheck, and execute live workspace edits inside WebAssembly.

#[allow(dead_code, clippy::all)]
#[path = "lexlean_generated.rs"]
mod wasm_ast_generated;

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use wasm_ast_generated::r4_wasm_ast_eval::{
    admit_ast_module, evaluate_ast_step, verify_test_assertion, AstEvalOutcome, TestRunnerOutcome,
};

/// Maximum source size in bytes per workspace module (`65,536` bytes).
pub const MAX_MODULE_SOURCE_BYTES: u64 = 65_536;

/// Default execution fuel budget per function or `#[test]` invocation.
pub const DEFAULT_EVAL_FUEL: u64 = 10_000;

/// Static type classification for the in-Wasm Rust subset (`R4-32`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustSubsetType {
    /// Signed or unsigned integer (`i64`, `u64`, `i32`, `u32`, `usize`, `u8`, etc.) -> Lean tag `0`.
    Int,
    /// Boolean (`bool`) -> Lean tag `1`.
    Bool,
    /// String slice or owned string (`&str`, `String`) -> Lean tag `2`.
    Str,
    /// Unit type `()` -> Lean tag `3`.
    Unit,
    /// Homogeneous slice/array `&[T]`, `[T; N]`, or `Vec<T>`.
    Slice(Box<RustSubsetType>),
    /// Fixed tuple `(T1, T2, ...)`.
    Tuple(Vec<RustSubsetType>),
    /// `Option<T>`.
    Option(Box<RustSubsetType>),
    /// `Result<T, E>`.
    Result(Box<RustSubsetType>, Box<RustSubsetType>),
    /// Named struct type (e.g., `GeometricRouter`).
    Struct(String),
    /// Named enum type (e.g., `RouteDecision`).
    Enum(String),
    /// Unconstrained or inferred type parameter.
    Unknown,
}

impl RustSubsetType {
    /// Map primitive types to the `R4.WasmAstEval` integer type tag (`0 = Int`, `1 = Bool`, `2 = Str`, `3 = Unit`).
    pub fn lean_type_tag(&self) -> u64 {
        match self {
            Self::Int => 0,
            Self::Bool => 1,
            Self::Str => 2,
            Self::Unit => 3,
            Self::Slice(_)
            | Self::Tuple(_)
            | Self::Option(_)
            | Self::Result(_, _)
            | Self::Struct(_)
            | Self::Enum(_)
            | Self::Unknown => 0,
        }
    }

    /// Parse a surface Rust type annotation into `RustSubsetType`.
    pub fn parse_annotation(raw: &str) -> Result<Self, String> {
        let s = raw.trim();
        if s.is_empty() || s == "()" {
            return Ok(Self::Unit);
        }
        if s == "_" {
            return Ok(Self::Unknown);
        }
        if s == "&str" || s == "String" || s == "str" || s == "&String" || s == "char" {
            return Ok(Self::Str);
        }
        let s = s
            .strip_prefix("&mut ")
            .or_else(|| s.strip_prefix('&'))
            .unwrap_or(s)
            .trim();
        if matches!(
            s,
            "i64"
                | "u64"
                | "i32"
                | "u32"
                | "i16"
                | "u16"
                | "i8"
                | "u8"
                | "usize"
                | "isize"
                | "i128"
                | "u128"
        ) {
            return Ok(Self::Int);
        }
        if s == "bool" {
            return Ok(Self::Bool);
        }
        if s == "str" || s == "String" || s == "char" {
            return Ok(Self::Str);
        }
        if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            let elem_str = if let Some((elem_ty, _len_part)) = inner.rsplit_once(';') {
                elem_ty.trim()
            } else {
                inner.trim()
            };
            return Ok(Self::Slice(Box::new(Self::parse_annotation(elem_str)?)));
        }
        if let Some(inner) = s.strip_prefix("Vec<").and_then(|r| r.strip_suffix('>')) {
            return Ok(Self::Slice(Box::new(Self::parse_annotation(inner)?)));
        }
        if let Some(inner) = s.strip_prefix("Option<").and_then(|r| r.strip_suffix('>')) {
            return Ok(Self::Option(Box::new(Self::parse_annotation(inner)?)));
        }
        if let Some(inner) = s.strip_prefix("Result<").and_then(|r| r.strip_suffix('>')) {
            let parts = split_top_level(inner, ',');
            if parts.len() != 2 {
                return Err(format!(
                    "`Result<T, E>` requires 2 type arguments in `{raw}`"
                ));
            }
            let ok_ty = Self::parse_annotation(parts[0].trim())?;
            let err_ty = Self::parse_annotation(parts[1].trim())?;
            return Ok(Self::Result(Box::new(ok_ty), Box::new(err_ty)));
        }
        if s.starts_with('(') && s.ends_with(')') && s.len() > 2 {
            let inner = &s[1..s.len() - 1];
            let parts = split_top_level(inner, ',');
            let mut elems = Vec::with_capacity(parts.len());
            for p in parts {
                if !p.trim().is_empty() {
                    elems.push(Self::parse_annotation(p.trim())?);
                }
            }
            return Ok(Self::Tuple(elems));
        }
        if s.chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Ok(Self::Struct(s.to_string()));
        }
        Err(format!("unknown type annotation `{s}`"))
    }

    /// Check whether `actual` is assignable to `expected`.
    pub fn is_compatible_with(&self, expected: &Self) -> bool {
        if matches!(self, Self::Unknown) || matches!(expected, Self::Unknown) {
            return true;
        }
        match (self, expected) {
            (Self::Int, Self::Int)
            | (Self::Bool, Self::Bool)
            | (Self::Str, Self::Str)
            | (Self::Unit, Self::Unit) => true,
            (Self::Slice(a), Self::Slice(b)) => a.is_compatible_with(b),
            (Self::Option(a), Self::Option(b)) => a.is_compatible_with(b),
            (Self::Result(ok_a, err_a), Self::Result(ok_b, err_b)) => {
                ok_a.is_compatible_with(ok_b) && err_a.is_compatible_with(err_b)
            }
            (Self::Tuple(a), Self::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.is_compatible_with(y))
            }
            (Self::Struct(a) | Self::Enum(a), Self::Struct(b) | Self::Enum(b)) => a == b,
            _ => false,
        }
    }
}

/// Runtime value in the fuel-bounded Wasm AST evaluator (`R4-32`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    /// 64-bit signed integer value.
    Int(i64),
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string value.
    Str(String),
    /// Unit `()`.
    Unit,
    /// Homogeneous or heterogeneous list/slice value.
    List(Vec<RuntimeValue>),
    /// Tuple value `(v1, v2, ...)`.
    Tuple(Vec<RuntimeValue>),
    /// `Option<RuntimeValue>`.
    OptionVal(Option<Box<RuntimeValue>>),
    /// `Result<RuntimeValue, RuntimeValue>`.
    ResultVal(Result<Box<RuntimeValue>, Box<RuntimeValue>>),
    /// Struct instance with named fields.
    StructVal {
        /// Struct type name.
        name: String,
        /// Field values keyed by field identifier.
        fields: BTreeMap<String, RuntimeValue>,
    },
    /// Enum variant value (`EnumName::Variant` or `EnumName::Variant(payload)`).
    EnumVariant {
        /// Enum type name.
        enum_name: String,
        /// Variant identifier.
        variant: String,
        /// Optional payload value.
        payload: Option<Box<RuntimeValue>>,
    },
}

impl RuntimeValue {
    /// Map runtime value to its static type.
    pub fn runtime_type(&self) -> RustSubsetType {
        match self {
            Self::Int(_) => RustSubsetType::Int,
            Self::Bool(_) => RustSubsetType::Bool,
            Self::Str(_) => RustSubsetType::Str,
            Self::Unit => RustSubsetType::Unit,
            Self::List(items) => {
                let inner = items
                    .first()
                    .map(Self::runtime_type)
                    .unwrap_or(RustSubsetType::Unknown);
                RustSubsetType::Slice(Box::new(inner))
            }
            Self::Tuple(items) => {
                RustSubsetType::Tuple(items.iter().map(Self::runtime_type).collect())
            }
            Self::OptionVal(Some(inner)) => RustSubsetType::Option(Box::new(inner.runtime_type())),
            Self::OptionVal(None) => RustSubsetType::Option(Box::new(RustSubsetType::Unknown)),
            Self::ResultVal(Ok(v)) => RustSubsetType::Result(
                Box::new(v.runtime_type()),
                Box::new(RustSubsetType::Unknown),
            ),
            Self::ResultVal(Err(e)) => RustSubsetType::Result(
                Box::new(RustSubsetType::Unknown),
                Box::new(e.runtime_type()),
            ),
            Self::StructVal { name, .. } => RustSubsetType::Struct(name.clone()),
            Self::EnumVariant { enum_name, .. } => RustSubsetType::Enum(enum_name.clone()),
        }
    }

    /// Project a runtime value onto a 64-bit non-negative integer for `R4.WasmAstEval` kernel verification.
    pub fn to_lean_nat(&self) -> u64 {
        match self {
            Self::Int(v) => *v as u64,
            Self::Bool(b) => u64::from(*b),
            Self::Str(s) => fnv1a64(s.as_bytes()),
            Self::Unit => 0,
            Self::List(items) => {
                let mut h = items.len() as u64;
                for item in items {
                    h = h.wrapping_mul(131).wrapping_add(item.to_lean_nat());
                }
                h
            }
            Self::Tuple(items) => {
                let mut h = (items.len() as u64).wrapping_add(0x9e37_79b9);
                for item in items {
                    h = h.wrapping_mul(257).wrapping_add(item.to_lean_nat());
                }
                h
            }
            Self::OptionVal(Some(v)) => v.to_lean_nat().wrapping_add(1),
            Self::OptionVal(None) => 0,
            Self::ResultVal(Ok(v)) => v.to_lean_nat().wrapping_add(0x1000_0001),
            Self::ResultVal(Err(e)) => e.to_lean_nat().wrapping_add(0x2000_0002),
            Self::StructVal { fields, .. } => {
                let mut h = 0x517c_c1b7_2722_0a95u64;
                for (k, v) in fields {
                    h ^= fnv1a64(k.as_bytes()).wrapping_add(v.to_lean_nat());
                }
                h
            }
            Self::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                let mut h = fnv1a64(enum_name.as_bytes()) ^ fnv1a64(variant.as_bytes());
                if let Some(p) = payload {
                    h = h.wrapping_mul(257).wrapping_add(p.to_lean_nat());
                }
                h
            }
        }
    }

    /// Format runtime value for diagnostic messages and `println!`.
    pub fn display_value(&self) -> String {
        match self {
            Self::Int(v) => v.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Str(s) => s.clone(),
            Self::Unit => "()".to_string(),
            Self::List(items) => {
                let inner: Vec<String> = items.iter().map(Self::display_value).collect();
                format!("[{}]", inner.join(", "))
            }
            Self::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(Self::display_value).collect();
                format!("({})", inner.join(", "))
            }
            Self::OptionVal(Some(v)) => format!("Some({})", v.display_value()),
            Self::OptionVal(None) => "None".to_string(),
            Self::ResultVal(Ok(v)) => format!("Ok({})", v.display_value()),
            Self::ResultVal(Err(e)) => format!("Err({})", e.display_value()),
            Self::StructVal { name, fields } => {
                let inner: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.display_value()))
                    .collect();
                format!("{name} {{ {} }}", inner.join(", "))
            }
            Self::EnumVariant {
                enum_name,
                variant,
                payload: Some(p),
            } => format!("{enum_name}::{variant}({})", p.display_value()),
            Self::EnumVariant {
                enum_name,
                variant,
                payload: None,
            } => format!("{enum_name}::{variant}"),
        }
    }
}

/// Parsed function parameter (`name: Type` or `&mut self` / `&self`).
#[derive(Debug, Clone)]
pub struct AstParam {
    /// Parameter identifier.
    pub name: String,
    /// Declared static type.
    pub ty: RustSubsetType,
    /// Whether this parameter is `&mut self` or `&self`.
    pub is_self: bool,
}

/// Parsed statement node with 1-based source line number.
#[derive(Debug, Clone)]
pub enum AstStmt {
    /// `let [mut] name [: ty] = expr;`
    Let {
        /// Variable identifier.
        name: String,
        /// Optional declared type annotation.
        declared_ty: Option<RustSubsetType>,
        /// Initializer expression.
        expr: String,
        /// 1-based line number.
        line: usize,
    },
    /// Assignment `target op= expr;` where `op` is `"="`, `"+="`, `"-="`, `"*="`, `"^="`, `"&="`, `"|="`.
    Assign {
        /// Target variable or `self.field` / `obj.field`.
        target: String,
        /// Assignment operator (`"="`, `"+="`, `"-="`, `"*="`, `"^="`, `"&="`, `"|="`).
        op: String,
        /// Right-hand side expression.
        expr: String,
        /// 1-based line number.
        line: usize,
    },
    /// `assert!(cond)` / `assert_eq!(lhs, rhs)` / `assert_ne!(lhs, rhs)`.
    Assert {
        /// `0` for `assert!`, `1` for `assert_eq!`, `2` for `assert_ne!`.
        kind: u64,
        /// Left or boolean condition expression.
        lhs_expr: String,
        /// Right expression (for `assert_eq!` / `assert_ne!`).
        rhs_expr: Option<String>,
        /// 1-based line number.
        line: usize,
    },
    /// `if cond { then_branch } else { else_branch }`
    If {
        /// Condition expression.
        cond_expr: String,
        /// Statements in the `then` block.
        then_body: Vec<AstStmt>,
        /// Statements in the optional `else` block.
        else_body: Vec<AstStmt>,
        /// 1-based line number.
        line: usize,
    },
    /// `while cond { body }`
    While {
        /// Loop condition expression.
        cond_expr: String,
        /// Loop body statements.
        body: Vec<AstStmt>,
        /// 1-based line number.
        line: usize,
    },
    /// `for pattern in iter_expr { body }`
    For {
        /// Loop binding pattern (e.g., `i` or `(&act, &w)`).
        pattern: String,
        /// Iterable expression (`a..b`, `slice.iter().zip(...)`, etc.).
        iter_expr: String,
        /// Loop body statements.
        body: Vec<AstStmt>,
        /// 1-based line number.
        line: usize,
    },
    /// `return [expr];` or tail return expression.
    Return {
        /// Optional return value expression.
        expr: Option<String>,
        /// Whether this is an implicit tail return without semicolon.
        is_tail: bool,
        /// 1-based line number.
        line: usize,
    },
    /// Standalone expression statement `expr;`.
    ExprStmt {
        /// Expression text.
        expr: String,
        /// 1-based line number.
        line: usize,
    },
}

/// Parsed function or method definition.
#[derive(Debug, Clone)]
pub struct AstFunction {
    /// Function identifier (e.g., `cyclic_frame_block` or `GeometricRouter::step_token`).
    pub name: String,
    /// Optional owning struct name if defined inside an `impl` block.
    pub impl_struct: Option<String>,
    /// Source file path.
    pub file_path: String,
    /// 1-based line number of the `fn` declaration.
    pub decl_line: usize,
    /// Whether the function is marked with `#[test]`.
    pub is_test: bool,
    /// Ordered parameter list.
    pub params: Vec<AstParam>,
    /// Declared return type (`Unit` if omitted).
    pub return_ty: RustSubsetType,
    /// Parsed body statements.
    pub body: Vec<AstStmt>,
}

/// Parsed struct definition.
#[derive(Debug, Clone)]
pub struct AstStructDef {
    /// Struct name.
    pub name: String,
    /// Source file path.
    pub file_path: String,
    /// 1-based declaration line.
    pub decl_line: usize,
    /// Field types keyed by field name.
    pub fields: BTreeMap<String, RustSubsetType>,
}

/// Parsed enum definition (`enum Name { VariantA, VariantB(T), ... }`).
#[derive(Debug, Clone)]
pub struct AstEnumDef {
    /// Enum name.
    pub name: String,
    /// Source file path.
    pub file_path: String,
    /// 1-based declaration line.
    pub decl_line: usize,
    /// Variant payload types keyed by variant name (`Vec` is empty for unit variants).
    pub variants: BTreeMap<String, Vec<RustSubsetType>>,
}

/// Parsed top-level constant (`const NAME: T = expr;`).
#[derive(Debug, Clone)]
pub struct AstConstDef {
    /// Constant identifier.
    pub name: String,
    /// Declared type.
    pub ty: RustSubsetType,
    /// Initializer expression.
    pub expr: String,
    /// Source file path.
    pub file_path: String,
    /// 1-based declaration line.
    pub decl_line: usize,
}

/// Complete parsed items for a single `.rs` workspace module.
#[derive(Debug, Clone, Default)]
pub struct ParsedRustModule {
    /// Struct definitions.
    pub structs: Vec<AstStructDef>,
    /// Enum definitions.
    pub enums: Vec<AstEnumDef>,
    /// Top-level `const` definitions.
    pub consts: Vec<AstConstDef>,
    /// Top-level `type Alias = T;` aliases.
    pub type_aliases: BTreeMap<String, RustSubsetType>,
    /// Function and method definitions.
    pub functions: Vec<AstFunction>,
}

/// Diagnostic item emitted during parsing, typechecking, or test execution.
#[derive(Debug, Clone)]
pub struct AstDiagnostic {
    /// File path where the diagnostic occurred.
    pub file_path: String,
    /// 1-based line number.
    pub line: usize,
    /// Severity (`"error"` or `"warning"`).
    pub severity: String,
    /// Diagnostic category (`"UnbalancedDelimiterAtLine"`, `"SyntaxErrorAtLine"`, `"TypeErrorAtLine"`, `"AssertionFailedAtLine"`, etc.).
    pub code: String,
    /// Human-readable compiler/evaluator diagnostic message.
    pub message: String,
}

/// Outcome of executing a single `#[test]` function.
#[derive(Debug, Clone)]
pub struct TestCaseResult {
    /// Fully qualified test name (`path::fn_name`).
    pub test_name: String,
    /// Source file path.
    pub file_path: String,
    /// 1-based declaration line.
    pub decl_line: usize,
    /// Whether the test passed.
    pub passed: bool,
    /// Fuel steps consumed during execution.
    pub fuel_consumed: u64,
    /// Number of assertions verified via `R4.WasmAstEval.verify_test_assertion`.
    pub assertions_verified: usize,
    /// Optional failure line number (1-based).
    pub failure_line: Option<usize>,
    /// Optional failure reason.
    pub failure_message: Option<String>,
}

/// Complete report for a `cargo test`, `cargo check`, `cargo clippy`, or `cargo run` invocation.
#[derive(Debug, Clone)]
pub struct CargoExecutionReport {
    /// Subcommand executed (`"test"`, `"check"`, `"clippy"`, `"run"`).
    pub subcommand: String,
    /// Overall status (`"ok"` or `"error"`).
    pub status: String,
    /// Formal `R4.WasmAstEval.admit_ast_module` outcome summary.
    pub module_admission_outcome: String,
    /// Number of `.rs` files parsed.
    pub files_parsed: usize,
    /// Total functions parsed across workspace.
    pub fn_count: usize,
    /// Total `#[test]` functions discovered.
    pub test_count: usize,
    /// Number of tests that passed.
    pub tests_passed: usize,
    /// Number of tests that failed.
    pub tests_failed: usize,
    /// Total execution fuel consumed across all evaluated functions/tests.
    pub total_fuel_consumed: u64,
    /// Individual test outcomes.
    pub test_results: Vec<TestCaseResult>,
    /// Compiler / typechecker / runtime diagnostics.
    pub diagnostics: Vec<AstDiagnostic>,
    /// Captured stdout from `cargo run` or test execution.
    pub stdout: String,
    /// Formatted terminal output ready for rendering in `lexlean-sh`.
    pub terminal_output: String,
    /// SHA-256 cryptographic receipt of the execution report.
    pub receipt_sha256: String,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(&mut out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Check delimiter balance (`()`, `[]`, `{}`) and string literal closure across `source`.
/// Returns `Ok(())` if balanced, or `Err((1_based_line, message))` on the first unbalanced delimiter.
pub fn check_delimiter_balance(source: &str) -> Result<(), (usize, String)> {
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut in_string = false;
    let mut string_start_line = 1usize;
    let mut in_block_comment = false;

    for (line_idx, raw_line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
        let chars: Vec<char> = raw_line.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();

            if in_block_comment {
                if c == '*' && next == Some('/') {
                    in_block_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            if in_string {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            if c == '/' && next == Some('/') {
                break;
            }
            if c == '/' && next == Some('*') {
                in_block_comment = true;
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = true;
                string_start_line = line_num;
                i += 1;
                continue;
            }
            if c == '\'' {
                // Skip char literal 'x' or '\n' cleanly without confusing lifetime annotations
                if let Some(after1) = chars.get(i + 1).copied() {
                    if after1 == '\\' && chars.get(i + 3) == Some(&'\'') {
                        i += 4;
                        continue;
                    } else if chars.get(i + 2) == Some(&'\'') {
                        i += 3;
                        continue;
                    }
                }
            }

            match c {
                '(' | '[' | '{' => stack.push((c, line_num)),
                ')' | ']' | '}' => {
                    let expected_open = match c {
                        ')' => '(',
                        ']' => '[',
                        _ => '{',
                    };
                    match stack.pop() {
                        Some((open, _)) if open == expected_open => {}
                        Some((open, open_line)) => {
                            let diag_line = if c == '}' && open != '{' {
                                open_line
                            } else {
                                line_num
                            };
                            return Err((
                                diag_line,
                                format!(
                                    "mismatched closing delimiter `{c}` at line {line_num} (unclosed `{open}` from line {open_line})"
                                ),
                            ));
                        }
                        None => {
                            return Err((
                                line_num,
                                format!("unexpected closing delimiter `{c}` at line {line_num}"),
                            ));
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    if in_string {
        return Err((
            string_start_line,
            format!("unclosed string literal starting at line {string_start_line}"),
        ));
    }
    if let Some((open, open_line)) = stack.last().copied() {
        return Err((
            open_line,
            format!("unclosed delimiter `{open}` opened at line {open_line}"),
        ));
    }
    Ok(())
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_str = false;
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_str = true;
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            return &line[..i];
        }
        i += 1;
    }
    line
}

fn split_top_level(input: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_brack = 0i32;
    let mut depth_brace = 0i32;
    let mut depth_angle = 0i32;
    let mut in_str = false;
    let mut start = 0usize;
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut idx = 0usize;

    while idx < chars.len() {
        let (byte_pos, c) = chars[idx];
        if in_str {
            if c == '\\' {
                idx += 2;
                continue;
            }
            if c == '"' {
                in_str = false;
            }
            idx += 1;
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_brack += 1,
            ']' => depth_brack -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '<' if sep == ',' => depth_angle += 1,
            '>' if sep == ',' && depth_angle > 0 => depth_angle -= 1,
            _ if c == sep
                && depth_paren == 0
                && depth_brack == 0
                && depth_brace == 0
                && depth_angle == 0 =>
            {
                parts.push(&input[start..byte_pos]);
                start = byte_pos + c.len_utf8();
            }
            _ => {}
        }
        idx += 1;
    }
    parts.push(&input[start..]);
    parts
}

/// Parse a single `.rs` workspace module into structs and functions.
pub fn parse_rust_module(
    file_path: &str,
    source: &str,
) -> Result<(Vec<AstStructDef>, Vec<AstFunction>), AstDiagnostic> {
    let parsed = parse_rust_module_items(file_path, source)?;
    Ok((parsed.structs, parsed.functions))
}

/// Parse a single `.rs` workspace module into structs, enums, consts, type aliases, and functions.
pub fn parse_rust_module_items(
    file_path: &str,
    source: &str,
) -> Result<ParsedRustModule, AstDiagnostic> {
    if let Err((line, msg)) = check_delimiter_balance(source) {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line,
            severity: "error".to_string(),
            code: "UnbalancedDelimiterAtLine".to_string(),
            message: msg,
        });
    }

    let lines: Vec<(usize, String)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, strip_line_comment(l).trim().to_string()))
        .collect();

    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut consts = Vec::new();
    let mut type_aliases = BTreeMap::new();
    let mut functions = Vec::new();
    let mut idx = 0usize;
    let mut pending_test_attr = false;
    let mut current_impl_struct: Option<(String, i32)> = None;
    let mut brace_depth = 0i32;

    while idx < lines.len() {
        let (line_num, ref text) = lines[idx];
        if text.is_empty() {
            idx += 1;
            continue;
        }

        if text.starts_with("#[cfg(")
            || text.starts_with("#[allow(")
            || text.starts_with("#[derive(")
            || text.starts_with("#[repr(")
            || text.starts_with("#[inline")
            || text.starts_with("#![")
        {
            idx += 1;
            continue;
        }
        if text == "#[test]" {
            pending_test_attr = true;
            idx += 1;
            continue;
        }
        if (text.starts_with("mod ") || text.starts_with("pub mod ")) && text.ends_with('{') {
            brace_depth += 1;
            idx += 1;
            continue;
        }
        if (text.starts_with("use ") || text.starts_with("pub use ")) && text.ends_with(';') {
            idx += 1;
            continue;
        }
        if text == "}" {
            brace_depth -= 1;
            if let Some((_, impl_depth)) = current_impl_struct {
                if brace_depth < impl_depth {
                    current_impl_struct = None;
                }
            }
            idx += 1;
            continue;
        }

        // Check `[pub] type Alias = T;`
        let type_rest = text
            .strip_prefix("pub type ")
            .or_else(|| text.strip_prefix("type "));
        if type_rest.is_some() {
            let (full_stmt, stmt_end_idx) = collect_simple_stmt(&lines, idx);
            let clean = full_stmt
                .trim()
                .strip_prefix("pub type ")
                .or_else(|| full_stmt.trim().strip_prefix("type "))
                .unwrap_or("")
                .trim();
            let Some(without_semi) = clean.strip_suffix(';') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("missing `;` at end of `type` alias at line {line_num}"),
                });
            };
            let Some((alias_name, target_ty_str)) = without_semi.split_once('=') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("missing `=` in `type` alias at line {line_num}"),
                });
            };
            let alias_ident = alias_name.trim();
            if alias_ident.is_empty()
                || !alias_ident
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!(
                        "invalid `type` alias identifier `{alias_ident}` at line {line_num}"
                    ),
                });
            }
            let parsed_ty =
                RustSubsetType::parse_annotation(target_ty_str.trim()).map_err(|e| {
                    AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!("{e} in `type {alias_ident}` at line {line_num}"),
                    }
                })?;
            type_aliases.insert(alias_ident.to_string(), parsed_ty);
            idx = stmt_end_idx + 1;
            continue;
        }

        // Check `[pub] const NAME: T = expr;` (excluding `const fn`)
        let is_const_item = (text.starts_with("pub const ") && !text.starts_with("pub const fn "))
            || (text.starts_with("const ") && !text.starts_with("const fn "));
        if is_const_item {
            let (full_stmt, stmt_end_idx) = collect_simple_stmt(&lines, idx);
            let clean = full_stmt
                .trim()
                .strip_prefix("pub const ")
                .or_else(|| full_stmt.trim().strip_prefix("const "))
                .unwrap_or("")
                .trim();
            let Some(without_semi) = clean.strip_suffix(';') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!(
                        "missing `;` at end of `const` declaration at line {line_num}"
                    ),
                });
            };
            let Some((lhs_raw, rhs_raw)) = without_semi.split_once('=') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("missing `=` initializer in `const` at line {line_num}"),
                });
            };
            let Some((cname_raw, cty_raw)) = lhs_raw.split_once(':') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("`const` declaration missing `: Type` at line {line_num}"),
                });
            };
            let cname = cname_raw.trim();
            if cname.is_empty() || !cname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("invalid `const` identifier `{cname}` at line {line_num}"),
                });
            }
            let parsed_ty =
                RustSubsetType::parse_annotation(cty_raw.trim()).map_err(|e| AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "TypeErrorAtLine".to_string(),
                    message: format!("{e} in `const {cname}` at line {line_num}"),
                })?;
            let expr_str = rhs_raw.trim();
            validate_expr_syntax(file_path, line_num, expr_str)?;
            consts.push(AstConstDef {
                name: cname.to_string(),
                ty: parsed_ty,
                expr: expr_str.to_string(),
                file_path: file_path.to_string(),
                decl_line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        // Check `impl StructName {`
        if let Some(rest) = text.strip_prefix("impl ") {
            if let Some(struct_name) = rest.strip_suffix('{').map(str::trim) {
                if struct_name.is_empty()
                    || !struct_name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "SyntaxErrorAtLine".to_string(),
                        message: format!(
                            "invalid `impl` target `{struct_name}` at line {line_num}"
                        ),
                    });
                }
                brace_depth += 1;
                current_impl_struct = Some((struct_name.to_string(), brace_depth));
                idx += 1;
                continue;
            }
        }

        // Check `[pub] enum Name {`
        let enum_rest = text
            .strip_prefix("pub enum ")
            .or_else(|| text.strip_prefix("enum "));
        if let Some(rest) = enum_rest {
            let (header, end_idx, body_lines) = collect_braced_block(&lines, idx, file_path)?;
            let name = header
                .strip_prefix("pub enum ")
                .or_else(|| header.strip_prefix("enum "))
                .unwrap_or(rest)
                .trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("invalid enum identifier `{name}` at line {line_num}"),
                });
            }
            let mut variants = BTreeMap::new();
            let joined_body: String = body_lines
                .iter()
                .map(|(_, l)| l.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            for raw_var in split_top_level(&joined_body, ',') {
                let vt = raw_var.trim();
                if vt.is_empty() {
                    continue;
                }
                let vt_no_disc = vt.split('=').next().unwrap_or(vt).trim();
                if let Some(open_p) = vt_no_disc.find('(') {
                    if let Some(inner) = vt_no_disc[open_p + 1..].strip_suffix(')') {
                        let vname = vt_no_disc[..open_p].trim();
                        if vname.is_empty()
                            || !vname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            return Err(AstDiagnostic {
                                file_path: file_path.to_string(),
                                line: line_num,
                                severity: "error".to_string(),
                                code: "SyntaxErrorAtLine".to_string(),
                                message: format!(
                                    "invalid enum variant `{vname}` at line {line_num}"
                                ),
                            });
                        }
                        let mut payload_tys = Vec::new();
                        for p in split_top_level(inner, ',') {
                            let pt = p.trim();
                            if !pt.is_empty() {
                                let parsed_ty =
                                    RustSubsetType::parse_annotation(pt).map_err(|e| {
                                        AstDiagnostic {
                                            file_path: file_path.to_string(),
                                            line: line_num,
                                            severity: "error".to_string(),
                                            code: "TypeErrorAtLine".to_string(),
                                            message: format!(
                                                "{e} in enum `{name}::{vname}` at line {line_num}"
                                            ),
                                        }
                                    })?;
                                payload_tys.push(parsed_ty);
                            }
                        }
                        variants.insert(vname.to_string(), payload_tys);
                        continue;
                    }
                }
                if !vt_no_disc
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "SyntaxErrorAtLine".to_string(),
                        message: format!("malformed enum variant `{vt}` at line {line_num}"),
                    });
                }
                variants.insert(vt_no_disc.to_string(), Vec::new());
            }
            enums.push(AstEnumDef {
                name: name.to_string(),
                file_path: file_path.to_string(),
                decl_line: line_num,
                variants,
            });
            idx = end_idx + 1;
            continue;
        }

        // Check `[pub] struct Name {`
        let struct_rest = text
            .strip_prefix("pub struct ")
            .or_else(|| text.strip_prefix("struct "));
        if let Some(rest) = struct_rest {
            let (header, end_idx, body_lines) = collect_braced_block(&lines, idx, file_path)?;
            let name = header
                .strip_prefix("pub struct ")
                .or_else(|| header.strip_prefix("struct "))
                .unwrap_or(rest)
                .trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("invalid struct identifier `{name}` at line {line_num}"),
                });
            }
            let mut fields = BTreeMap::new();
            for (f_line, f_text) in body_lines {
                for raw_field in split_top_level(&f_text, ',') {
                    let ft = raw_field.trim();
                    if ft.is_empty() {
                        continue;
                    }
                    let clean_ft = ft.strip_prefix("pub ").unwrap_or(ft).trim();
                    let Some((fname, fty_str)) = clean_ft.split_once(':') else {
                        return Err(AstDiagnostic {
                            file_path: file_path.to_string(),
                            line: f_line,
                            severity: "error".to_string(),
                            code: "SyntaxErrorAtLine".to_string(),
                            message: format!("malformed struct field `{ft}` at line {f_line}"),
                        });
                    };
                    let parsed_ty =
                        RustSubsetType::parse_annotation(fty_str.trim()).map_err(|e| {
                            AstDiagnostic {
                                file_path: file_path.to_string(),
                                line: f_line,
                                severity: "error".to_string(),
                                code: "TypeErrorAtLine".to_string(),
                                message: format!("{e} at line {f_line}"),
                            }
                        })?;
                    fields.insert(fname.trim().to_string(), parsed_ty);
                }
            }
            structs.push(AstStructDef {
                name: name.to_string(),
                file_path: file_path.to_string(),
                decl_line: line_num,
                fields,
            });
            idx = end_idx + 1;
            continue;
        }

        // Check `[pub] [const] fn name(...)`
        let is_fn = text.starts_with("pub fn ")
            || text.starts_with("fn ")
            || text.starts_with("pub const fn ")
            || text.starts_with("const fn ");
        if is_fn {
            let (header, end_idx, body_lines) = collect_braced_block(&lines, idx, file_path)?;
            let impl_owner = current_impl_struct.as_ref().map(|(s, _)| s.clone());
            let parsed_fn = parse_function_def(
                file_path,
                line_num,
                &header,
                &body_lines,
                pending_test_attr,
                impl_owner,
            )?;
            pending_test_attr = false;
            functions.push(parsed_fn);
            idx = end_idx + 1;
            continue;
        }

        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: line_num,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("unexpected top-level item `{text}` at line {line_num}"),
        });
    }

    Ok(ParsedRustModule {
        structs,
        enums,
        consts,
        type_aliases,
        functions,
    })
}

type BracedBlockResult = Result<(String, usize, Vec<(usize, String)>), AstDiagnostic>;

fn collect_braced_block(
    lines: &[(usize, String)],
    start_idx: usize,
    file_path: &str,
) -> BracedBlockResult {
    let start_line = lines[start_idx].0;
    let mut header = String::new();
    let mut idx = start_idx;
    let mut found_open = false;
    let mut body_lines: Vec<(usize, String)> = Vec::new();
    let mut depth = 0i32;

    while idx < lines.len() {
        let (line_num, ref text) = lines[idx];
        if !found_open {
            if let Some(open_pos) = find_top_level_open_brace(text) {
                let before = text[..open_pos].trim();
                if !header.is_empty() && !before.is_empty() {
                    header.push(' ');
                }
                header.push_str(before);
                found_open = true;
                depth = 1;
                let after = text[open_pos + 1..].trim();
                for ch in after.chars() {
                    if ch == '{' {
                        depth += 1;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                if depth == 0 {
                    if let Some(inner) = after.strip_suffix('}') {
                        if !inner.trim().is_empty() {
                            body_lines.push((line_num, inner.trim().to_string()));
                        }
                    }
                    return Ok((header.trim().to_string(), idx, body_lines));
                }
                if !after.is_empty() {
                    body_lines.push((line_num, after.to_string()));
                }
            } else {
                if !header.is_empty() && !text.is_empty() {
                    header.push(' ');
                }
                header.push_str(text);
            }
        } else {
            for ch in text.chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            if depth <= 0 {
                if let Some(before_close) = text.strip_suffix('}') {
                    let trimmed = before_close.trim();
                    if !trimmed.is_empty() {
                        body_lines.push((line_num, trimmed.to_string()));
                    }
                }
                return Ok((header.trim().to_string(), idx, body_lines));
            }
            if !text.is_empty() {
                body_lines.push((line_num, text.clone()));
            }
        }
        idx += 1;
    }

    Err(AstDiagnostic {
        file_path: file_path.to_string(),
        line: start_line,
        severity: "error".to_string(),
        code: "SyntaxErrorAtLine".to_string(),
        message: format!("missing opening `{{` for declaration at line {start_line}"),
    })
}

fn find_top_level_open_brace(s: &str) -> Option<usize> {
    let mut paren = 0i32;
    let mut brack = 0i32;
    let mut in_str = false;
    for (byte_pos, c) in s.char_indices() {
        if in_str {
            if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' => paren += 1,
            ')' => paren = (paren - 1).max(0),
            '[' => brack += 1,
            ']' => brack = (brack - 1).max(0),
            '{' if paren == 0 && brack == 0 => return Some(byte_pos),
            _ => {}
        }
    }
    None
}

fn parse_function_def(
    file_path: &str,
    decl_line: usize,
    header: &str,
    body_lines: &[(usize, String)],
    is_test: bool,
    impl_struct: Option<String>,
) -> Result<AstFunction, AstDiagnostic> {
    let rest = header
        .strip_prefix("pub const fn ")
        .or_else(|| header.strip_prefix("const fn "))
        .or_else(|| header.strip_prefix("pub fn "))
        .or_else(|| header.strip_prefix("fn "))
        .ok_or_else(|| AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("expected `fn` declaration at line {decl_line}"),
        })?
        .trim();

    let Some(open_paren) = rest.find('(') else {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("missing parameter list `(` in function at line {decl_line}"),
        });
    };
    let raw_name = rest[..open_paren].trim();
    if raw_name.is_empty()
        || !raw_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("invalid function name `{raw_name}` at line {decl_line}"),
        });
    }

    let after_open = &rest[open_paren + 1..];
    let mut paren_depth = 1i32;
    let mut close_paren_opt = None;
    for (bpos, ch) in after_open.char_indices() {
        if ch == '(' {
            paren_depth += 1;
        } else if ch == ')' {
            paren_depth -= 1;
            if paren_depth == 0 {
                close_paren_opt = Some(bpos);
                break;
            }
        }
    }
    let Some(close_paren) = close_paren_opt else {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("missing closing `)` in parameter list at line {decl_line}"),
        });
    };
    let params_str = after_open[..close_paren].trim();
    let ret_part = after_open[close_paren + 1..].trim();

    let mut params = Vec::new();
    if !params_str.is_empty() {
        for raw_param in split_top_level(params_str, ',') {
            let p = raw_param.trim();
            if p.is_empty() {
                continue;
            }
            if p == "&self" || p == "&mut self" || p == "self" {
                let self_ty = impl_struct
                    .as_ref()
                    .map(|s| RustSubsetType::Struct(s.clone()))
                    .unwrap_or(RustSubsetType::Unknown);
                params.push(AstParam {
                    name: "self".to_string(),
                    ty: self_ty,
                    is_self: true,
                });
                continue;
            }
            let Some((pname_raw, pty_raw)) = p.split_once(':') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: decl_line,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("parameter `{p}` missing `: Type` at line {decl_line}"),
                });
            };
            let pname = pname_raw
                .trim()
                .strip_prefix("mut ")
                .unwrap_or(pname_raw.trim())
                .trim();
            let parsed_ty =
                RustSubsetType::parse_annotation(pty_raw.trim()).map_err(|e| AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: decl_line,
                    severity: "error".to_string(),
                    code: "TypeErrorAtLine".to_string(),
                    message: format!("{e} in parameter `{pname}` at line {decl_line}"),
                })?;
            params.push(AstParam {
                name: pname.to_string(),
                ty: parsed_ty,
                is_self: false,
            });
        }
    }

    let return_ty = if ret_part.is_empty() {
        RustSubsetType::Unit
    } else if let Some(ret_ty_str) = ret_part.strip_prefix("->") {
        RustSubsetType::parse_annotation(ret_ty_str.trim()).map_err(|e| AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "TypeErrorAtLine".to_string(),
            message: format!("{e} in return type of `{raw_name}` at line {decl_line}"),
        })?
    } else {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: decl_line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!(
                "unexpected tokens `{ret_part}` after parameter list at line {decl_line}"
            ),
        });
    };

    let body = parse_stmt_block(file_path, body_lines)?;

    Ok(AstFunction {
        name: raw_name.to_string(),
        impl_struct,
        file_path: file_path.to_string(),
        decl_line,
        is_test,
        params,
        return_ty,
        body,
    })
}

fn parse_stmt_block(
    file_path: &str,
    lines: &[(usize, String)],
) -> Result<Vec<AstStmt>, AstDiagnostic> {
    let mut stmts = Vec::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        let (line_num, ref raw_text) = lines[idx];
        let text = raw_text.trim();
        if text.is_empty() {
            idx += 1;
            continue;
        }

        // Handle `if ... {`
        if text.starts_with("if ") {
            let (if_stmt, next_idx) = parse_if_chain(file_path, lines, idx)?;
            stmts.push(if_stmt);
            idx = next_idx;
            continue;
        }

        // Handle `while ... {`
        if let Some(rest) = text.strip_prefix("while ") {
            let (header, end_idx, body_lines) = collect_braced_block(lines, idx, file_path)?;
            let cond = header.strip_prefix("while ").unwrap_or(rest).trim();
            if cond.is_empty() {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("empty `while` condition at line {line_num}"),
                });
            }
            let body = parse_stmt_block(file_path, &body_lines)?;
            stmts.push(AstStmt::While {
                cond_expr: cond.to_string(),
                body,
                line: line_num,
            });
            idx = end_idx + 1;
            continue;
        }

        // Handle `for pat in iter {`
        if let Some(rest) = text.strip_prefix("for ") {
            let (header, end_idx, body_lines) = collect_braced_block(lines, idx, file_path)?;
            let for_head = header.strip_prefix("for ").unwrap_or(rest).trim();
            let Some((pat, iter_expr)) = for_head.split_once(" in ") else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("malformed `for` loop header `{for_head}` at line {line_num}"),
                });
            };
            let body = parse_stmt_block(file_path, &body_lines)?;
            stmts.push(AstStmt::For {
                pattern: pat.trim().to_string(),
                iter_expr: iter_expr.trim().to_string(),
                body,
                line: line_num,
            });
            idx = end_idx + 1;
            continue;
        }

        // Gather multi-line statement until `;` or end of block
        let (full_stmt, stmt_end_idx) = collect_simple_stmt(lines, idx);
        let trimmed_stmt = full_stmt.trim();

        // `let [mut] name [: ty] = expr;` or `const name [: ty] = expr;`
        let let_or_const_rest = trimmed_stmt
            .strip_prefix("let ")
            .or_else(|| trimmed_stmt.strip_prefix("const "));
        if let Some(let_rest) = let_or_const_rest {
            let Some(without_semi) = let_rest.strip_suffix(';') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!(
                        "missing semicolon `;` at end of `let` binding at line {line_num}"
                    ),
                });
            };
            let Some((lhs_raw, rhs_raw)) = without_semi.split_once('=') else {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!(
                        "missing `=` initializer in `let` statement at line {line_num}"
                    ),
                });
            };
            let lhs_clean = lhs_raw
                .trim()
                .strip_prefix("mut ")
                .unwrap_or(lhs_raw.trim())
                .trim();
            let (var_name, declared_ty) = if let Some((vname, ty_str)) = lhs_clean.split_once(':') {
                let parsed_ty =
                    RustSubsetType::parse_annotation(ty_str.trim()).map_err(|e| AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!("{e} in `let {vname}` at line {line_num}"),
                    })?;
                (vname.trim().to_string(), Some(parsed_ty))
            } else {
                (lhs_clean.to_string(), None)
            };
            if var_name.is_empty()
                || !var_name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("invalid variable name `{var_name}` at line {line_num}"),
                });
            }
            let rhs_expr = rhs_raw.trim();
            if rhs_expr.is_empty() {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!(
                        "empty right-hand side in `let {var_name}` at line {line_num}"
                    ),
                });
            }
            stmts.push(AstStmt::Let {
                name: var_name,
                declared_ty,
                expr: rhs_expr.to_string(),
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        // `assert!`, `assert_eq!`, `assert_ne!`
        if trimmed_stmt.starts_with("assert_eq!(")
            || trimmed_stmt.starts_with("assert_ne!(")
            || trimmed_stmt.starts_with("assert!(")
        {
            let without_semi = trimmed_stmt
                .strip_suffix(';')
                .ok_or_else(|| AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("missing `;` after assertion macro at line {line_num}"),
                })?;
            let (kind, open_prefix) = if without_semi.starts_with("assert_eq!(") {
                (1u64, "assert_eq!(")
            } else if without_semi.starts_with("assert_ne!(") {
                (2u64, "assert_ne!(")
            } else {
                (0u64, "assert!(")
            };
            let inner = without_semi
                .strip_prefix(open_prefix)
                .and_then(|s| s.strip_suffix(')'))
                .ok_or_else(|| AstDiagnostic {
                    file_path: file_path.to_string(),
                    line: line_num,
                    severity: "error".to_string(),
                    code: "SyntaxErrorAtLine".to_string(),
                    message: format!("malformed assertion macro at line {line_num}"),
                })?;
            let args = split_top_level(inner, ',');
            if kind == 0 {
                if args.is_empty() || args[0].trim().is_empty() {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "SyntaxErrorAtLine".to_string(),
                        message: format!("empty `assert!()` condition at line {line_num}"),
                    });
                }
                stmts.push(AstStmt::Assert {
                    kind: 0,
                    lhs_expr: args[0].trim().to_string(),
                    rhs_expr: None,
                    line: line_num,
                });
            } else {
                if args.len() < 2 || args[0].trim().is_empty() || args[1].trim().is_empty() {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "SyntaxErrorAtLine".to_string(),
                        message: format!(
                            "assertion macro requires two arguments `(left, right)` at line {line_num}"
                        ),
                    });
                }
                stmts.push(AstStmt::Assert {
                    kind,
                    lhs_expr: args[0].trim().to_string(),
                    rhs_expr: Some(args[1].trim().to_string()),
                    line: line_num,
                });
            }
            idx = stmt_end_idx + 1;
            continue;
        }

        // `return [expr];`
        if trimmed_stmt == "return;" {
            stmts.push(AstStmt::Return {
                expr: None,
                is_tail: false,
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }
        if let Some(ret_rest) = trimmed_stmt.strip_prefix("return ") {
            let expr_str = ret_rest.strip_suffix(';').unwrap_or(ret_rest).trim();
            stmts.push(AstStmt::Return {
                expr: if expr_str.is_empty() {
                    None
                } else {
                    Some(expr_str.to_string())
                },
                is_tail: false,
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        // Check assignment statements `target op= rhs;`
        if let Some(without_semi) = trimmed_stmt.strip_suffix(';') {
            if let Some((target, op, rhs)) = split_assignment(without_semi) {
                if rhs.trim().is_empty() {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: line_num,
                        severity: "error".to_string(),
                        code: "SyntaxErrorAtLine".to_string(),
                        message: format!("empty right-hand side in assignment at line {line_num}"),
                    });
                }
                stmts.push(AstStmt::Assign {
                    target: target.trim().to_string(),
                    op: op.to_string(),
                    expr: rhs.trim().to_string(),
                    line: line_num,
                });
                idx = stmt_end_idx + 1;
                continue;
            }
            stmts.push(AstStmt::ExprStmt {
                expr: without_semi.trim().to_string(),
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        // If this is the final statement in the block, it is a tail return expression!
        if stmt_end_idx + 1 == lines.len() {
            validate_expr_syntax(file_path, line_num, trimmed_stmt)?;
            stmts.push(AstStmt::Return {
                expr: Some(trimmed_stmt.to_string()),
                is_tail: true,
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        // `match ... { ... }` or `{ ... }` block expressions in statement position do not require `;`
        if (trimmed_stmt.starts_with("match ") || trimmed_stmt.starts_with('{'))
            && trimmed_stmt.ends_with('}')
        {
            validate_expr_syntax(file_path, line_num, trimmed_stmt)?;
            stmts.push(AstStmt::ExprStmt {
                expr: trimmed_stmt.to_string(),
                line: line_num,
            });
            idx = stmt_end_idx + 1;
            continue;
        }

        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: line_num,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("missing `;` after statement `{trimmed_stmt}` at line {line_num}"),
        });
    }

    Ok(stmts)
}

fn collect_simple_stmt(lines: &[(usize, String)], start_idx: usize) -> (String, usize) {
    let mut acc = String::new();
    let mut idx = start_idx;
    let mut paren = 0i32;
    let mut brack = 0i32;
    let mut brace = 0i32;
    let mut in_str = false;

    while idx < lines.len() {
        let (_, ref text) = lines[idx];
        if !acc.is_empty() && !text.is_empty() {
            acc.push(' ');
        }
        acc.push_str(text.trim());
        let bytes = text.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if in_str {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    in_str = false;
                }
                i += 1;
                continue;
            }
            match bytes[i] {
                b'"' => in_str = true,
                b'(' => paren += 1,
                b')' => paren -= 1,
                b'[' => brack += 1,
                b']' => brack -= 1,
                b'{' => brace += 1,
                b'}' => brace -= 1,
                _ => {}
            }
            i += 1;
        }
        if paren <= 0 && brack <= 0 && brace <= 0 && !in_str {
            break;
        }
        idx += 1;
    }
    (acc, idx.min(lines.len().saturating_sub(1)))
}

fn parse_if_chain(
    file_path: &str,
    lines: &[(usize, String)],
    start_idx: usize,
) -> Result<(AstStmt, usize), AstDiagnostic> {
    let line_num = lines[start_idx].0;
    let mut idx = start_idx;
    let mut first_cond = String::new();
    let mut then_lines: Vec<(usize, String)> = Vec::new();
    let mut else_lines: Vec<(usize, String)> = Vec::new();

    // Parse the `if cond { ... }` part, watching for `} else if ... {` or `} else {` on the closing line
    let start_text = lines[idx].1.trim();
    let Some(open_pos) = find_top_level_open_brace(start_text) else {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: line_num,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("missing `{{` after `if` condition at line {line_num}"),
        });
    };
    first_cond.push_str(
        start_text[..open_pos]
            .trim()
            .strip_prefix("if ")
            .unwrap_or("")
            .trim(),
    );
    if first_cond.is_empty() {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line: line_num,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("empty `if` condition at line {line_num}"),
        });
    }

    let mut depth = 1i32;
    let mut in_else = false;
    let after_open = start_text[open_pos + 1..].trim();
    if !after_open.is_empty() {
        for ch in after_open.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if depth == 0 {
            if let Some((then_part, else_part)) = after_open.split_once("} else {") {
                if let Some(else_inner) = else_part.strip_suffix('}') {
                    if !then_part.trim().is_empty() {
                        then_lines.push((line_num, then_part.trim().to_string()));
                    }
                    if !else_inner.trim().is_empty() {
                        else_lines.push((line_num, else_inner.trim().to_string()));
                    }
                    let then_body = parse_stmt_block(file_path, &then_lines)?;
                    let else_body = parse_stmt_block(file_path, &else_lines)?;
                    return Ok((
                        AstStmt::If {
                            cond_expr: first_cond,
                            then_body,
                            else_body,
                            line: line_num,
                        },
                        idx + 1,
                    ));
                }
            }
            if let Some(inner) = after_open.strip_suffix('}') {
                if !inner.trim().is_empty() {
                    then_lines.push((line_num, inner.trim().to_string()));
                }
            }
            let then_body = parse_stmt_block(file_path, &then_lines)?;
            return Ok((
                AstStmt::If {
                    cond_expr: first_cond,
                    then_body,
                    else_body: Vec::new(),
                    line: line_num,
                },
                idx + 1,
            ));
        }
        then_lines.push((line_num, after_open.to_string()));
    }

    idx += 1;
    while idx < lines.len() {
        let (lnum, ref t) = lines[idx];
        let trimmed = t.trim();
        if !in_else && depth == 1 && trimmed.starts_with("} else if ") {
            let rest_if = trimmed.strip_prefix("} else ").unwrap_or("").to_string();
            let mut sub_lines = vec![(lnum, rest_if)];
            sub_lines.extend_from_slice(&lines[idx + 1..]);
            let (nested_if, consumed) = parse_if_chain(file_path, &sub_lines, 0)?;
            let then_body = parse_stmt_block(file_path, &then_lines)?;
            return Ok((
                AstStmt::If {
                    cond_expr: first_cond,
                    then_body,
                    else_body: vec![nested_if],
                    line: line_num,
                },
                idx + consumed,
            ));
        }
        if !in_else && depth == 1 && (trimmed == "} else {" || trimmed.starts_with("} else {")) {
            in_else = true;
            let after_else = trimmed.strip_prefix("} else {").unwrap_or("").trim();
            if !after_else.is_empty() {
                if let Some(inner) = after_else.strip_suffix('}') {
                    if !inner.trim().is_empty() {
                        else_lines.push((lnum, inner.trim().to_string()));
                    }
                    idx += 1;
                    break;
                }
                else_lines.push((lnum, after_else.to_string()));
            }
            idx += 1;
            continue;
        }

        for ch in trimmed.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if depth <= 0 {
            if let Some(before_close) = trimmed.strip_suffix('}') {
                let b = before_close.trim();
                if !b.is_empty() {
                    if in_else {
                        else_lines.push((lnum, b.to_string()));
                    } else {
                        then_lines.push((lnum, b.to_string()));
                    }
                }
            }
            idx += 1;
            break;
        }
        if in_else {
            else_lines.push((lnum, trimmed.to_string()));
        } else {
            then_lines.push((lnum, trimmed.to_string()));
        }
        idx += 1;
    }

    let then_body = parse_stmt_block(file_path, &then_lines)?;
    let else_body = if else_lines.is_empty() {
        Vec::new()
    } else {
        parse_stmt_block(file_path, &else_lines)?
    };
    Ok((
        AstStmt::If {
            cond_expr: first_cond,
            then_body,
            else_body,
            line: line_num,
        },
        idx,
    ))
}

fn split_assignment(stmt: &str) -> Option<(&str, &'static str, &str)> {
    let bytes = stmt.as_bytes();
    let mut paren = 0i32;
    let mut in_str = false;
    let mut i = 0usize;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_str = true;
                i += 1;
                continue;
            }
            b'(' | b'[' | b'{' => paren += 1,
            b')' | b']' | b'}' => paren -= 1,
            _ if paren == 0 => {
                for op in ["<<=", ">>=", "^=", "+=", "-=", "*=", "/=", "%=", "&=", "|="] {
                    if bytes[i..].starts_with(op.as_bytes()) {
                        return Some((&stmt[..i], op, &stmt[i + op.len()..]));
                    }
                }
                if bytes[i] == b'=' {
                    let prev = if i > 0 { bytes[i - 1] } else { 0 };
                    let next = bytes.get(i + 1).copied().unwrap_or(0);
                    if !matches!(
                        prev,
                        b'=' | b'!'
                            | b'<'
                            | b'>'
                            | b'+'
                            | b'-'
                            | b'*'
                            | b'/'
                            | b'%'
                            | b'^'
                            | b'&'
                            | b'|'
                    ) && next != b'='
                        && next != b'>'
                    {
                        return Some((&stmt[..i], "=", &stmt[i + 1..]));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn validate_expr_syntax(file_path: &str, line: usize, expr: &str) -> Result<(), AstDiagnostic> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("empty expression at line {line}"),
        });
    }
    for bad_suffix in ["+", "-", "*", "/", "%", "&&", "||", "==", "!=", "^"] {
        if trimmed.ends_with(bad_suffix) {
            return Err(AstDiagnostic {
                file_path: file_path.to_string(),
                line,
                severity: "error".to_string(),
                code: "SyntaxErrorAtLine".to_string(),
                message: format!(
                    "dangling binary operator `{bad_suffix}` at end of expression `{trimmed}` at line {line}"
                ),
            });
        }
    }
    if trimmed.contains(";;") || trimmed.starts_with("fn ") || trimmed.starts_with("let ") {
        return Err(AstDiagnostic {
            file_path: file_path.to_string(),
            line,
            severity: "error".to_string(),
            code: "SyntaxErrorAtLine".to_string(),
            message: format!("invalid expression syntax `{trimmed}` at line {line}"),
        });
    }
    Ok(())
}

/// Static typechecker for a parsed module (`R4-32`).
pub fn typecheck_module(
    structs: &[AstStructDef],
    functions: &[AstFunction],
) -> Result<(), AstDiagnostic> {
    typecheck_module_items(structs, &[], &[], functions)
}

/// Static typechecker for a parsed module including enums and top-level constants (`R4-32`).
pub fn typecheck_module_items(
    structs: &[AstStructDef],
    enums: &[AstEnumDef],
    consts: &[AstConstDef],
    functions: &[AstFunction],
) -> Result<(), AstDiagnostic> {
    let empty_aliases = BTreeMap::new();
    typecheck_module_items_with_aliases(structs, enums, consts, &empty_aliases, functions)
}

fn resolve_subset_type(
    ty: &RustSubsetType,
    type_aliases: &BTreeMap<String, RustSubsetType>,
    enums: &[AstEnumDef],
    struct_defs: &BTreeMap<String, BTreeMap<String, RustSubsetType>>,
    depth: usize,
) -> RustSubsetType {
    if depth > 8 {
        return ty.clone();
    }
    match ty {
        RustSubsetType::Struct(name) => {
            if let Some(aliased) = type_aliases.get(name) {
                resolve_subset_type(aliased, type_aliases, enums, struct_defs, depth + 1)
            } else if enums.iter().any(|e| &e.name == name) && !struct_defs.contains_key(name) {
                RustSubsetType::Enum(name.clone())
            } else {
                RustSubsetType::Struct(name.clone())
            }
        }
        RustSubsetType::Option(inner) => RustSubsetType::Option(Box::new(resolve_subset_type(
            inner,
            type_aliases,
            enums,
            struct_defs,
            depth + 1,
        ))),
        RustSubsetType::Result(ok, err) => RustSubsetType::Result(
            Box::new(resolve_subset_type(
                ok,
                type_aliases,
                enums,
                struct_defs,
                depth + 1,
            )),
            Box::new(resolve_subset_type(
                err,
                type_aliases,
                enums,
                struct_defs,
                depth + 1,
            )),
        ),
        RustSubsetType::Slice(inner) => RustSubsetType::Slice(Box::new(resolve_subset_type(
            inner,
            type_aliases,
            enums,
            struct_defs,
            depth + 1,
        ))),
        RustSubsetType::Tuple(items) => RustSubsetType::Tuple(
            items
                .iter()
                .map(|it| resolve_subset_type(it, type_aliases, enums, struct_defs, depth + 1))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Static typechecker for a parsed module including enums, top-level constants, and type aliases (`R4-32`).
pub fn typecheck_module_items_with_aliases(
    structs: &[AstStructDef],
    enums: &[AstEnumDef],
    consts: &[AstConstDef],
    type_aliases: &BTreeMap<String, RustSubsetType>,
    functions: &[AstFunction],
) -> Result<(), AstDiagnostic> {
    let mut fn_return_types: BTreeMap<String, RustSubsetType> = BTreeMap::new();
    let mut struct_defs: BTreeMap<String, BTreeMap<String, RustSubsetType>> = BTreeMap::new();
    let mut base_env: BTreeMap<String, RustSubsetType> = BTreeMap::new();

    for s in structs {
        struct_defs.insert(s.name.clone(), s.fields.clone());
    }
    let resolved_structs: BTreeMap<String, BTreeMap<String, RustSubsetType>> = struct_defs
        .iter()
        .map(|(k, fields)| {
            let rf = fields
                .iter()
                .map(|(fk, fty)| {
                    (
                        fk.clone(),
                        resolve_subset_type(fty, type_aliases, enums, &struct_defs, 0),
                    )
                })
                .collect();
            (k.clone(), rf)
        })
        .collect();
    struct_defs = resolved_structs;

    for e in enums {
        for (vname, payload_tys) in &e.variants {
            if !payload_tys.is_empty() {
                fn_return_types.insert(
                    format!("{}::{}", e.name, vname),
                    RustSubsetType::Enum(e.name.clone()),
                );
            }
        }
    }
    for f in functions {
        let ret_ty = resolve_subset_type(&f.return_ty, type_aliases, enums, &struct_defs, 0);
        fn_return_types.insert(f.name.clone(), ret_ty.clone());
        if let Some(ref owner) = f.impl_struct {
            fn_return_types.insert(format!("{owner}::{}", f.name), ret_ty);
        }
    }
    for c in consts {
        let expected_c_ty = resolve_subset_type(&c.ty, type_aliases, enums, &struct_defs, 0);
        let inferred = infer_expr_type(
            &c.file_path,
            c.decl_line,
            &c.expr,
            &base_env,
            &fn_return_types,
            &struct_defs,
        )?;
        if !inferred.is_compatible_with(&expected_c_ty) {
            return Err(AstDiagnostic {
                file_path: c.file_path.clone(),
                line: c.decl_line,
                severity: "error".to_string(),
                code: "TypeErrorAtLine".to_string(),
                message: format!(
                    "mismatched types in `const {}` at line {}: expected `{:?}`, found `{:?}`",
                    c.name, c.decl_line, expected_c_ty, inferred
                ),
            });
        }
        base_env.insert(c.name.clone(), expected_c_ty);
    }

    for func in functions {
        let mut env = base_env.clone();
        for p in &func.params {
            let resolved_p = resolve_subset_type(&p.ty, type_aliases, enums, &struct_defs, 0);
            let norm_ty = match &resolved_p {
                RustSubsetType::Struct(sname) if !struct_defs.contains_key(sname) => {
                    if enums.iter().any(|e| &e.name == sname) {
                        RustSubsetType::Enum(sname.clone())
                    } else {
                        RustSubsetType::Int
                    }
                }
                other => other.clone(),
            };
            env.insert(p.name.clone(), norm_ty);
        }
        let expected_ret =
            resolve_subset_type(&func.return_ty, type_aliases, enums, &struct_defs, 0);
        typecheck_stmts(
            &func.file_path,
            &func.body,
            &mut env,
            &expected_ret,
            &fn_return_types,
            &struct_defs,
            type_aliases,
            enums,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn typecheck_stmts(
    file_path: &str,
    stmts: &[AstStmt],
    env: &mut BTreeMap<String, RustSubsetType>,
    expected_return: &RustSubsetType,
    fn_return_types: &BTreeMap<String, RustSubsetType>,
    struct_defs: &BTreeMap<String, BTreeMap<String, RustSubsetType>>,
    type_aliases: &BTreeMap<String, RustSubsetType>,
    enums: &[AstEnumDef],
) -> Result<(), AstDiagnostic> {
    for stmt in stmts {
        match stmt {
            AstStmt::Let {
                name,
                declared_ty,
                expr,
                line,
            } => {
                validate_expr_syntax(file_path, *line, expr)?;
                let inferred =
                    infer_expr_type(file_path, *line, expr, env, fn_return_types, struct_defs)?;
                if let Some(raw_decl) = declared_ty {
                    let decl = resolve_subset_type(raw_decl, type_aliases, enums, struct_defs, 0);
                    if !inferred.is_compatible_with(&decl) {
                        return Err(AstDiagnostic {
                            file_path: file_path.to_string(),
                            line: *line,
                            severity: "error".to_string(),
                            code: "TypeErrorAtLine".to_string(),
                            message: format!(
                                "mismatched types in `let {name}` at line {line}: expected `{decl:?}`, found `{inferred:?}`"
                            ),
                        });
                    }
                    env.insert(name.clone(), decl);
                } else {
                    env.insert(name.clone(), inferred);
                }
            }
            AstStmt::Assign {
                target,
                op,
                expr,
                line,
            } => {
                validate_expr_syntax(file_path, *line, expr)?;
                let rhs_ty =
                    infer_expr_type(file_path, *line, expr, env, fn_return_types, struct_defs)?;
                let lhs_ty =
                    infer_expr_type(file_path, *line, target, env, fn_return_types, struct_defs)?;
                if op != "=" && !matches!(rhs_ty, RustSubsetType::Int | RustSubsetType::Unknown) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: *line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "compound assignment `{op}` requires integer operand at line {line}, found `{rhs_ty:?}`"
                        ),
                    });
                }
                if !rhs_ty.is_compatible_with(&lhs_ty) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: *line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "type mismatch in assignment to `{target}` at line {line}: expected `{lhs_ty:?}`, found `{rhs_ty:?}`"
                        ),
                    });
                }
            }
            AstStmt::Assert {
                kind,
                lhs_expr,
                rhs_expr,
                line,
            } => {
                validate_expr_syntax(file_path, *line, lhs_expr)?;
                let lhs_ty = infer_expr_type(
                    file_path,
                    *line,
                    lhs_expr,
                    env,
                    fn_return_types,
                    struct_defs,
                )?;
                if *kind == 0 {
                    if !lhs_ty.is_compatible_with(&RustSubsetType::Bool) {
                        return Err(AstDiagnostic {
                            file_path: file_path.to_string(),
                            line: *line,
                            severity: "error".to_string(),
                            code: "TypeErrorAtLine".to_string(),
                            message: format!(
                                "`assert!` condition at line {line} must be `bool`, found `{lhs_ty:?}`"
                            ),
                        });
                    }
                } else if let Some(rhs) = rhs_expr {
                    validate_expr_syntax(file_path, *line, rhs)?;
                    let rhs_ty =
                        infer_expr_type(file_path, *line, rhs, env, fn_return_types, struct_defs)?;
                    if !lhs_ty.is_compatible_with(&rhs_ty) {
                        return Err(AstDiagnostic {
                            file_path: file_path.to_string(),
                            line: *line,
                            severity: "error".to_string(),
                            code: "TypeErrorAtLine".to_string(),
                            message: format!(
                                "mismatched assertion operand types at line {line}: left is `{lhs_ty:?}`, right is `{rhs_ty:?}`"
                            ),
                        });
                    }
                }
            }
            AstStmt::If {
                cond_expr,
                then_body,
                else_body,
                line,
            } => {
                validate_expr_syntax(file_path, *line, cond_expr)?;
                let cond_ty = infer_expr_type(
                    file_path,
                    *line,
                    cond_expr,
                    env,
                    fn_return_types,
                    struct_defs,
                )?;
                if !cond_ty.is_compatible_with(&RustSubsetType::Bool) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: *line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "`if` condition at line {line} must have type `bool`, found `{cond_ty:?}`"
                        ),
                    });
                }
                let mut then_env = env.clone();
                typecheck_stmts(
                    file_path,
                    then_body,
                    &mut then_env,
                    expected_return,
                    fn_return_types,
                    struct_defs,
                    type_aliases,
                    enums,
                )?;
                let mut else_env = env.clone();
                typecheck_stmts(
                    file_path,
                    else_body,
                    &mut else_env,
                    expected_return,
                    fn_return_types,
                    struct_defs,
                    type_aliases,
                    enums,
                )?;
            }
            AstStmt::While {
                cond_expr,
                body,
                line,
            } => {
                validate_expr_syntax(file_path, *line, cond_expr)?;
                let cond_ty = infer_expr_type(
                    file_path,
                    *line,
                    cond_expr,
                    env,
                    fn_return_types,
                    struct_defs,
                )?;
                if !cond_ty.is_compatible_with(&RustSubsetType::Bool) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: *line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "`while` condition at line {line} must have type `bool`, found `{cond_ty:?}`"
                        ),
                    });
                }
                let mut body_env = env.clone();
                typecheck_stmts(
                    file_path,
                    body,
                    &mut body_env,
                    expected_return,
                    fn_return_types,
                    struct_defs,
                    type_aliases,
                    enums,
                )?;
            }
            AstStmt::For {
                pattern,
                iter_expr,
                body,
                line,
            } => {
                let mut body_env = env.clone();
                let iter_ty = infer_expr_type(
                    file_path,
                    *line,
                    iter_expr,
                    env,
                    fn_return_types,
                    struct_defs,
                )
                .unwrap_or(RustSubsetType::Slice(Box::new(RustSubsetType::Int)));
                let elem_ty = match iter_ty {
                    RustSubsetType::Slice(inner) => *inner,
                    _ => RustSubsetType::Int,
                };
                bind_closure_param_type(pattern, &elem_ty, &mut body_env);
                typecheck_stmts(
                    file_path,
                    body,
                    &mut body_env,
                    expected_return,
                    fn_return_types,
                    struct_defs,
                    type_aliases,
                    enums,
                )?;
            }
            AstStmt::Return {
                expr,
                is_tail: _,
                line,
            } => {
                let actual_ty = if let Some(e) = expr {
                    validate_expr_syntax(file_path, *line, e)?;
                    infer_expr_type(file_path, *line, e, env, fn_return_types, struct_defs)?
                } else {
                    RustSubsetType::Unit
                };
                if !actual_ty.is_compatible_with(expected_return) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line: *line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "mismatched return type at line {line}: expected `{expected_return:?}`, found `{actual_ty:?}`"
                        ),
                    });
                }
            }
            AstStmt::ExprStmt { expr, line } => {
                validate_expr_syntax(file_path, *line, expr)?;
                let _ = infer_expr_type(file_path, *line, expr, env, fn_return_types, struct_defs)?;
            }
        }
    }
    Ok(())
}

fn infer_expr_type(
    file_path: &str,
    line: usize,
    expr: &str,
    env: &BTreeMap<String, RustSubsetType>,
    fn_return_types: &BTreeMap<String, RustSubsetType>,
    struct_defs: &BTreeMap<String, BTreeMap<String, RustSubsetType>>,
) -> Result<RustSubsetType, AstDiagnostic> {
    let s = strip_outer_parens(expr.trim());
    if s.is_empty() || s == "()" {
        return Ok(RustSubsetType::Unit);
    }
    if s == "true" || s == "false" {
        return Ok(RustSubsetType::Bool);
    }
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3)
    {
        return Ok(RustSubsetType::Str);
    }
    if s == "None" {
        return Ok(RustSubsetType::Option(Box::new(RustSubsetType::Unknown)));
    }
    if let Some((callee, args_str)) = split_outermost_call(s) {
        match callee {
            "Some" => {
                let inner_ty =
                    infer_expr_type(file_path, line, args_str, env, fn_return_types, struct_defs)?;
                return Ok(RustSubsetType::Option(Box::new(inner_ty)));
            }
            "Ok" => {
                let inner_ty =
                    infer_expr_type(file_path, line, args_str, env, fn_return_types, struct_defs)?;
                return Ok(RustSubsetType::Result(
                    Box::new(inner_ty),
                    Box::new(RustSubsetType::Unknown),
                ));
            }
            "Err" => {
                let inner_ty =
                    infer_expr_type(file_path, line, args_str, env, fn_return_types, struct_defs)?;
                return Ok(RustSubsetType::Result(
                    Box::new(RustSubsetType::Unknown),
                    Box::new(inner_ty),
                ));
            }
            "format!" | "String::from" | "String::new" => {
                return Ok(RustSubsetType::Str);
            }
            "println!" => {
                return Ok(RustSubsetType::Unit);
            }
            "Vec::new" | "Vec::with_capacity" => {
                return Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Unknown)));
            }
            _ => {}
        }
    }
    if let Some(rest) = s.strip_prefix("if ") {
        // Inline `if cond { a } else { b }`
        if let Some((cond_part, branches)) = rest.split_once('{') {
            let cond_ty = infer_expr_type(
                file_path,
                line,
                cond_part.trim(),
                env,
                fn_return_types,
                struct_defs,
            )?;
            if !cond_ty.is_compatible_with(&RustSubsetType::Bool) {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line,
                    severity: "error".to_string(),
                    code: "TypeErrorAtLine".to_string(),
                    message: format!(
                        "inline `if` condition at line {line} must be `bool`, found `{cond_ty:?}`"
                    ),
                });
            }
            if let Some((then_e, else_part)) = branches.split_once("} else {") {
                if let Some(else_e) = else_part.strip_suffix('}') {
                    let t1 = infer_block_or_expr_type(
                        file_path,
                        line,
                        then_e.trim(),
                        env,
                        fn_return_types,
                        struct_defs,
                    )?;
                    let t2 = infer_block_or_expr_type(
                        file_path,
                        line,
                        else_e.trim(),
                        env,
                        fn_return_types,
                        struct_defs,
                    )?;
                    if !t1.is_compatible_with(&t2) {
                        return Err(AstDiagnostic {
                            file_path: file_path.to_string(),
                            line,
                            severity: "error".to_string(),
                            code: "TypeErrorAtLine".to_string(),
                            message: format!(
                                "`if` and `else` arms have incompatible types at line {line}: `{t1:?}` vs `{t2:?}`"
                            ),
                        });
                    }
                    return Ok(t1);
                }
            }
        }
    }

    if let Some((scrutinee, arms)) = parse_match_expr(s) {
        let scrut_ty = infer_expr_type(
            file_path,
            line,
            scrutinee,
            env,
            fn_return_types,
            struct_defs,
        )?;
        let mut arm_ty: Option<RustSubsetType> = None;
        for (pat, body) in arms {
            let mut arm_env = env.clone();
            bind_match_pattern_types(pat, &scrut_ty, &mut arm_env);
            let b_ty = infer_block_or_expr_type(
                file_path,
                line,
                body,
                &arm_env,
                fn_return_types,
                struct_defs,
            )?;
            if let Some(ref prev_ty) = arm_ty {
                if !b_ty.is_compatible_with(prev_ty) {
                    return Err(AstDiagnostic {
                        file_path: file_path.to_string(),
                        line,
                        severity: "error".to_string(),
                        code: "TypeErrorAtLine".to_string(),
                        message: format!(
                            "`match` arms have incompatible types at line {line}: `{prev_ty:?}` vs `{b_ty:?}`"
                        ),
                    });
                }
            } else if b_ty != RustSubsetType::Unknown {
                arm_ty = Some(b_ty);
            }
        }
        return Ok(arm_ty.unwrap_or(RustSubsetType::Unit));
    }

    if s.starts_with('{') && s.ends_with('}') {
        return infer_block_or_expr_type(file_path, line, s, env, fn_return_types, struct_defs);
    }

    if split_top_level_range(s).is_some() {
        return Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Int)));
    }

    // Check logical / comparison / arithmetic binary operators at top level (lowest precedence first,
    // rightmost operator within each equal-precedence tier for left-to-right associativity)
    for tier in [&["||"][..], &["&&"][..]] {
        if let Some((lhs, op, rhs)) = split_top_level_binary_tier(s, tier) {
            let lty = infer_expr_type(file_path, line, lhs, env, fn_return_types, struct_defs)?;
            let rty = infer_expr_type(file_path, line, rhs, env, fn_return_types, struct_defs)?;
            if !lty.is_compatible_with(&RustSubsetType::Bool)
                || !rty.is_compatible_with(&RustSubsetType::Bool)
            {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line,
                    severity: "error".to_string(),
                    code: "TypeErrorAtLine".to_string(),
                    message: format!(
                        "logical operator `{op}` requires `bool` operands at line {line}, found `{lty:?}` and `{rty:?}`"
                    ),
                });
            }
            return Ok(RustSubsetType::Bool);
        }
    }
    if let Some((lhs, op, rhs)) =
        split_top_level_binary_tier(s, &["==", "!=", "<=", ">=", "<", ">"])
    {
        let lty = infer_expr_type(file_path, line, lhs, env, fn_return_types, struct_defs)?;
        let rty = infer_expr_type(file_path, line, rhs, env, fn_return_types, struct_defs)?;
        if !lty.is_compatible_with(&rty) {
            return Err(AstDiagnostic {
                file_path: file_path.to_string(),
                line,
                severity: "error".to_string(),
                code: "TypeErrorAtLine".to_string(),
                message: format!(
                    "comparison `{op}` between incompatible types `{lty:?}` and `{rty:?}` at line {line}"
                ),
            });
        }
        return Ok(RustSubsetType::Bool);
    }
    for tier in [
        &["|"][..],
        &["^"][..],
        &["&"][..],
        &["<<", ">>"][..],
        &["+", "-"][..],
        &["*", "/", "%"][..],
    ] {
        if let Some((lhs, op, rhs)) = split_top_level_binary_tier(s, tier) {
            let lty = infer_expr_type(file_path, line, lhs, env, fn_return_types, struct_defs)?;
            let rty = infer_expr_type(file_path, line, rhs, env, fn_return_types, struct_defs)?;
            if !lty.is_compatible_with(&RustSubsetType::Int)
                || !rty.is_compatible_with(&RustSubsetType::Int)
            {
                return Err(AstDiagnostic {
                    file_path: file_path.to_string(),
                    line,
                    severity: "error".to_string(),
                    code: "TypeErrorAtLine".to_string(),
                    message: format!(
                        "binary operator `{op}` requires integer operands at line {line}, found `{lty:?}` and `{rty:?}`"
                    ),
                });
            }
            return Ok(RustSubsetType::Int);
        }
    }

    if let Some(lhs_cast) = strip_int_cast_suffix(s) {
        let _ = infer_expr_type(file_path, line, lhs_cast, env, fn_return_types, struct_defs)?;
        return Ok(RustSubsetType::Int);
    }

    // Postfix `?` operator
    if let Some(inner) = s.strip_suffix('?') {
        let inner_ty = infer_expr_type(file_path, line, inner, env, fn_return_types, struct_defs)?;
        return match inner_ty {
            RustSubsetType::Result(ok_ty, _) => Ok(*ok_ty),
            RustSubsetType::Option(some_ty) => Ok(*some_ty),
            _ => Ok(RustSubsetType::Unknown),
        };
    }

    if let Some(inner) = s.strip_prefix('!') {
        let inner_ty = infer_expr_type(file_path, line, inner, env, fn_return_types, struct_defs)?;
        return Ok(inner_ty);
    }

    if parse_int_literal(s).is_some() {
        return Ok(RustSubsetType::Int);
    }

    if let Some(inner) = s.strip_prefix('-') {
        let inner_ty = infer_expr_type(file_path, line, inner, env, fn_return_types, struct_defs)?;
        return Ok(inner_ty);
    }

    if let Some(inner) = s.strip_prefix('*') {
        return infer_expr_type(file_path, line, inner, env, fn_return_types, struct_defs);
    }

    if !s.starts_with("&[") {
        if let Some(inner) = s.strip_prefix("&mut ").or_else(|| s.strip_prefix('&')) {
            return infer_expr_type(file_path, line, inner, env, fn_return_types, struct_defs);
        }
    }

    if let Some(inner) = strip_whole_bracket_literal(s) {
        let repeat_parts = split_top_level(inner, ';');
        if repeat_parts.len() == 2 {
            let elem_ty = infer_expr_type(
                file_path,
                line,
                repeat_parts[0].trim(),
                env,
                fn_return_types,
                struct_defs,
            )?;
            return Ok(RustSubsetType::Slice(Box::new(elem_ty)));
        }
        let elems = split_top_level(inner, ',');
        let elem_ty = if let Some(first) = elems.first().map(|e| e.trim()).filter(|e| !e.is_empty())
        {
            infer_expr_type(file_path, line, first, env, fn_return_types, struct_defs)?
        } else {
            RustSubsetType::Unknown
        };
        return Ok(RustSubsetType::Slice(Box::new(elem_ty)));
    }

    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        if check_delimiter_balance(inner).is_ok() {
            let parts = split_top_level(inner, ',');
            if parts.len() > 1 {
                let mut tys = Vec::with_capacity(parts.len());
                for p in parts {
                    if !p.trim().is_empty() {
                        tys.push(infer_expr_type(
                            file_path,
                            line,
                            p.trim(),
                            env,
                            fn_return_types,
                            struct_defs,
                        )?);
                    }
                }
                return Ok(RustSubsetType::Tuple(tys));
            }
        }
    }

    // Struct literal `StructName { ... }`
    if let Some(open_brace) = find_top_level_open_brace(s) {
        let sname = s[..open_brace].trim();
        if struct_defs.contains_key(sname) {
            return Ok(RustSubsetType::Struct(sname.to_string()));
        }
    }

    // Index expression `arr[idx]`
    if let Some((target_s, _)) = split_trailing_index(s) {
        let target_ty =
            infer_expr_type(file_path, line, target_s, env, fn_return_types, struct_defs)?;
        if let RustSubsetType::Slice(elem_ty) = target_ty {
            return Ok(*elem_ty);
        }
        return Ok(RustSubsetType::Int);
    }

    if let Some(ty) = env.get(s) {
        return Ok(ty.clone());
    }

    // Function, method, or enum variant call
    if let Some((callee, args_str)) = split_outermost_call(s) {
        if let Some(ret_ty) = fn_return_types.get(callee) {
            return Ok(ret_ty.clone());
        }
        if let Some((recv_expr, method_raw)) = rsplit_top_level_dot(callee) {
            let method_name = method_raw.split("::<").next().unwrap_or(method_raw).trim();
            if let Some(ret_ty) = fn_return_types.get(method_name) {
                return Ok(ret_ty.clone());
            }
            let recv_ty = infer_expr_type(
                file_path,
                line,
                recv_expr,
                env,
                fn_return_types,
                struct_defs,
            )?;
            return match method_name {
                "is_empty" | "is_some" | "is_none" | "is_ok" | "is_err" | "is_multiple_of"
                | "contains" | "starts_with" | "ends_with" | "any" | "all" => {
                    Ok(RustSubsetType::Bool)
                }
                "len" | "count" | "sum" | "product" => Ok(RustSubsetType::Int),
                "push" | "clear" | "reverse" | "sort" => Ok(RustSubsetType::Unit),
                "trim" | "to_uppercase" | "to_lowercase" | "replace" | "to_string" | "as_str" => {
                    Ok(RustSubsetType::Str)
                }
                "chars" => Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Str))),
                "bytes" => Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Int))),
                "iter" | "into_iter" | "cloned" | "copied" | "rev" | "filter" | "take" | "skip"
                | "collect" => match recv_ty {
                    RustSubsetType::Slice(_) => Ok(recv_ty),
                    _ => Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Unknown))),
                },
                "enumerate" => Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Tuple(
                    vec![RustSubsetType::Int, RustSubsetType::Unknown],
                )))),
                "map" => {
                    if let Some((params, body)) = parse_closure_expr(args_str.trim()) {
                        let mut c_env = env.clone();
                        let elem_ty = match recv_ty {
                            RustSubsetType::Slice(inner) => *inner,
                            _ => RustSubsetType::Int,
                        };
                        if let Some(first_p) = params.first() {
                            bind_closure_param_type(first_p, &elem_ty, &mut c_env);
                        }
                        let mapped_ty = infer_block_or_expr_type(
                            file_path,
                            line,
                            body,
                            &c_env,
                            fn_return_types,
                            struct_defs,
                        )?;
                        Ok(RustSubsetType::Slice(Box::new(mapped_ty)))
                    } else {
                        Ok(RustSubsetType::Slice(Box::new(RustSubsetType::Unknown)))
                    }
                }
                "fold" => {
                    let parts = split_top_level(args_str, ',');
                    if let Some(init_expr) = parts.first() {
                        infer_expr_type(
                            file_path,
                            line,
                            init_expr.trim(),
                            env,
                            fn_return_types,
                            struct_defs,
                        )
                    } else {
                        Ok(RustSubsetType::Unknown)
                    }
                }
                "first" | "last" | "pop" => match recv_ty {
                    RustSubsetType::Slice(inner) => Ok(RustSubsetType::Option(inner)),
                    _ => Ok(RustSubsetType::Option(Box::new(RustSubsetType::Unknown))),
                },
                "unwrap" | "unwrap_or" => match recv_ty {
                    RustSubsetType::Option(inner) => Ok(*inner),
                    RustSubsetType::Result(ok_ty, _) => Ok(*ok_ty),
                    _ => Ok(RustSubsetType::Unknown),
                },
                "unwrap_err" => match recv_ty {
                    RustSubsetType::Result(_, err_ty) => Ok(*err_ty),
                    _ => Ok(RustSubsetType::Unknown),
                },
                "ok" => match recv_ty {
                    RustSubsetType::Result(ok_ty, _) => Ok(RustSubsetType::Option(ok_ty)),
                    _ => Ok(RustSubsetType::Option(Box::new(RustSubsetType::Unknown))),
                },
                "clone" => Ok(recv_ty),
                _ => Ok(RustSubsetType::Int),
            };
        }
        if let Some((type_prefix, item_name)) = callee.split_once("::") {
            let short_fn = item_name.trim();
            if let Some(ret_ty) = fn_return_types.get(short_fn) {
                return Ok(ret_ty.clone());
            }
            if type_prefix
                .trim()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            {
                return Ok(RustSubsetType::Enum(type_prefix.trim().to_string()));
            }
        }
    }

    // Field access `obj.field` or tuple `.0`
    if let Some((recv, field)) = rsplit_top_level_dot(s) {
        if !field.contains('(') {
            let recv_ty = infer_expr_type(
                file_path,
                line,
                recv.trim(),
                env,
                fn_return_types,
                struct_defs,
            )?;
            if let RustSubsetType::Struct(sname) = recv_ty {
                if let Some(fields) = struct_defs.get(&sname) {
                    if let Some(fty) = fields.get(field.trim()) {
                        return Ok(fty.clone());
                    }
                }
            } else if let RustSubsetType::Tuple(tys) = recv_ty {
                if let Ok(idx) = field.trim().parse::<usize>() {
                    if let Some(fty) = tys.get(idx) {
                        return Ok(fty.clone());
                    }
                }
            }
            return Ok(RustSubsetType::Int);
        }
    }

    // Unit enum variant `EnumName::Variant`
    if let Some((type_prefix, variant_name)) = s.split_once("::") {
        if type_prefix
            .trim()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
            && !variant_name.contains('(')
        {
            return Ok(RustSubsetType::Enum(type_prefix.trim().to_string()));
        }
    }

    Ok(RustSubsetType::Unknown)
}

fn infer_block_or_expr_type(
    file_path: &str,
    line: usize,
    body: &str,
    env: &BTreeMap<String, RustSubsetType>,
    fn_return_types: &BTreeMap<String, RustSubsetType>,
    struct_defs: &BTreeMap<String, BTreeMap<String, RustSubsetType>>,
) -> Result<RustSubsetType, AstDiagnostic> {
    let trimmed = body.trim();
    if let Some(inner) = trimmed.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let synth_lines = split_inline_block_lines(line, inner);
        let stmts = parse_stmt_block(file_path, &synth_lines)?;
        let mut local_env = env.clone();
        let mut last_ty = RustSubsetType::Unit;
        for st in &stmts {
            match st {
                AstStmt::Let {
                    name,
                    declared_ty,
                    expr,
                    line: l,
                } => {
                    let t = infer_expr_type(
                        file_path,
                        *l,
                        expr,
                        &local_env,
                        fn_return_types,
                        struct_defs,
                    )?;
                    local_env.insert(name.clone(), declared_ty.clone().unwrap_or(t));
                    last_ty = RustSubsetType::Unit;
                }
                AstStmt::Return {
                    expr: Some(e),
                    line: l,
                    ..
                } => {
                    last_ty = infer_expr_type(
                        file_path,
                        *l,
                        e,
                        &local_env,
                        fn_return_types,
                        struct_defs,
                    )?;
                }
                AstStmt::ExprStmt { expr, line: l } => {
                    last_ty = infer_expr_type(
                        file_path,
                        *l,
                        expr,
                        &local_env,
                        fn_return_types,
                        struct_defs,
                    )?;
                }
                _ => {
                    last_ty = RustSubsetType::Unit;
                }
            }
        }
        return Ok(last_ty);
    }
    if let Some((_, _, rhs)) = split_assignment(trimmed) {
        let _ = infer_expr_type(
            file_path,
            line,
            rhs.trim(),
            env,
            fn_return_types,
            struct_defs,
        )?;
        return Ok(RustSubsetType::Unit);
    }
    infer_expr_type(file_path, line, trimmed, env, fn_return_types, struct_defs)
}

fn bind_closure_param_type(
    pattern: &str,
    elem_ty: &RustSubsetType,
    env: &mut BTreeMap<String, RustSubsetType>,
) {
    let trimmed = pattern.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let parts = split_top_level(inner, ',');
        for (i, part) in parts.iter().enumerate() {
            let clean = part
                .split(':')
                .next()
                .unwrap_or(part)
                .trim()
                .trim_start_matches('&')
                .trim_start_matches("mut ")
                .trim();
            if !clean.is_empty() && clean != "_" {
                let sub_ty = match elem_ty {
                    RustSubsetType::Tuple(tys) => {
                        tys.get(i).cloned().unwrap_or(RustSubsetType::Unknown)
                    }
                    _ => RustSubsetType::Int,
                };
                env.insert(clean.to_string(), sub_ty);
            }
        }
    } else {
        let clean = trimmed
            .split(':')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .trim_start_matches('&')
            .trim_start_matches("mut ")
            .trim();
        if !clean.is_empty() && clean != "_" {
            env.insert(clean.to_string(), elem_ty.clone());
        }
    }
}

fn bind_match_pattern_types(
    pat: &str,
    scrut_ty: &RustSubsetType,
    env: &mut BTreeMap<String, RustSubsetType>,
) {
    for sub in split_top_level(pat, '|') {
        let p = sub.trim();
        if let Some((callee, inner)) = split_outermost_call(p) {
            match callee {
                "Some" => {
                    let inner_name = inner.trim();
                    if is_pattern_ident(inner_name) {
                        let inner_ty = match scrut_ty {
                            RustSubsetType::Option(inner_box) => (**inner_box).clone(),
                            _ => RustSubsetType::Int,
                        };
                        env.insert(inner_name.to_string(), inner_ty);
                    }
                }
                "Ok" => {
                    let inner_name = inner.trim();
                    if is_pattern_ident(inner_name) {
                        let inner_ty = match scrut_ty {
                            RustSubsetType::Result(ok_box, _) => (**ok_box).clone(),
                            _ => RustSubsetType::Int,
                        };
                        env.insert(inner_name.to_string(), inner_ty);
                    }
                }
                "Err" => {
                    let inner_name = inner.trim();
                    if is_pattern_ident(inner_name) {
                        let inner_ty = match scrut_ty {
                            RustSubsetType::Result(_, err_box) => (**err_box).clone(),
                            _ => RustSubsetType::Str,
                        };
                        env.insert(inner_name.to_string(), inner_ty);
                    }
                }
                _ if callee.contains("::") => {
                    for part in split_top_level(inner, ',') {
                        let ident = part.trim();
                        if is_pattern_ident(ident) {
                            env.insert(ident.to_string(), RustSubsetType::Unknown);
                        }
                    }
                }
                _ => {}
            }
        } else if is_pattern_ident(p) {
            env.insert(p.to_string(), scrut_ty.clone());
        }
    }
}

fn is_pattern_ident(p: &str) -> bool {
    !p.is_empty()
        && p != "_"
        && p != "true"
        && p != "false"
        && p != "None"
        && !p.contains("::")
        && p.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_match_expr(s: &str) -> Option<(&str, Vec<(&str, &str)>)> {
    let rest = s.strip_prefix("match ")?;
    if !rest.ends_with('}') {
        return None;
    }
    let open_brace = find_top_level_open_brace(rest)?;
    let scrutinee = rest[..open_brace].trim();
    if scrutinee.is_empty() {
        return None;
    }
    let inner = rest[open_brace + 1..rest.len() - 1].trim();
    let mut arms = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // Find top-level `=>`
        let pat_start = i;
        let mut paren = 0i32;
        let mut in_str = false;
        let mut arrow_pos = None;
        while i + 1 < bytes.len() {
            if in_str {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    in_str = false;
                }
                i += 1;
                continue;
            }
            match bytes[i] {
                b'"' => in_str = true,
                b'(' | b'[' | b'{' => paren += 1,
                b')' | b']' | b'}' => paren -= 1,
                b'=' if paren == 0 && bytes[i + 1] == b'>' => {
                    arrow_pos = Some(i);
                    i += 2;
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        let pat_end = arrow_pos?;
        let pat = inner[pat_start..pat_end].trim();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let body_start = i;
        if i < bytes.len() && bytes[i] == b'{' {
            let mut brace_depth = 0i32;
            let mut str_mode = false;
            while i < bytes.len() {
                if str_mode {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        str_mode = false;
                    }
                    i += 1;
                    continue;
                }
                match bytes[i] {
                    b'"' => str_mode = true,
                    b'{' => brace_depth += 1,
                    b'}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let body = inner[body_start..i].trim();
            arms.push((pat, body));
        } else {
            let mut depth = 0i32;
            let mut str_mode = false;
            while i < bytes.len() {
                if str_mode {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        str_mode = false;
                    }
                    i += 1;
                    continue;
                }
                match bytes[i] {
                    b'"' => str_mode = true,
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    b',' if depth == 0 => break,
                    _ => {}
                }
                i += 1;
            }
            let body = inner[body_start..i].trim();
            arms.push((pat, body));
            if i < bytes.len() && bytes[i] == b',' {
                i += 1;
            }
        }
    }
    Some((scrutinee, arms))
}

fn split_inline_block_lines(line: usize, inner: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let bytes = inner.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = inner[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if in_str {
            cur.push(ch);
            if ch == '\\' && i + ch_len < bytes.len() {
                let next_ch = inner[i + ch_len..].chars().next().unwrap();
                cur.push(next_ch);
                i += ch_len + next_ch.len_utf8();
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
            i += ch_len;
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                cur.push(ch);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(ch);
            }
            ';' if depth == 0 => {
                cur.push(';');
                let s = cur.trim();
                if !s.is_empty() {
                    out.push((line, s.to_string()));
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
        i += ch_len;
    }
    let tail = cur.trim();
    if !tail.is_empty() {
        out.push((line, tail.to_string()));
    }
    out
}

fn strip_outer_parens(mut s: &str) -> &str {
    loop {
        let trimmed = s.trim();
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            let inner = &trimmed[1..trimmed.len() - 1];
            // Only strip if the outer parens match each other and it's not a tuple `(a, b)`
            if check_delimiter_balance(inner).is_ok() && split_top_level(inner, ',').len() == 1 {
                s = inner;
                continue;
            }
        }
        return trimmed;
    }
}

fn split_outermost_call(s: &str) -> Option<(&str, &str)> {
    let trimmed = s.trim();
    if !trimmed.ends_with(')') {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut paren = 0i32;
    let mut in_str = false;
    let mut open_idx = None;
    let mut i = 0usize;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_str = true,
            b'(' => {
                if paren == 0 {
                    open_idx = Some(i);
                }
                paren += 1;
            }
            b')' => {
                paren -= 1;
                if paren == 0 && i + 1 < bytes.len() {
                    open_idx = None;
                }
            }
            b'[' | b'{' => paren += 1,
            b']' | b'}' => paren -= 1,
            _ => {}
        }
        i += 1;
    }
    let idx = open_idx?;
    let callee = trimmed[..idx].trim();
    if callee.is_empty() {
        return None;
    }
    Some((callee, &trimmed[idx + 1..trimmed.len() - 1]))
}

fn strip_whole_bracket_literal(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if let Some((callee, inner)) = split_outermost_call(trimmed) {
        if callee == "vec!" {
            return Some(inner);
        }
    }
    if !trimmed.ends_with(']') {
        return None;
    }
    let open_pos = if trimmed.starts_with("vec![") {
        4
    } else if trimmed.starts_with("&[") {
        1
    } else if trimmed.starts_with('[') {
        0
    } else {
        return None;
    };
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut i = open_pos;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 && i + 1 < bytes.len() {
                    return None;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if depth == 0 {
        Some(&trimmed[open_pos + 1..trimmed.len() - 1])
    } else {
        None
    }
}

fn split_trailing_index(s: &str) -> Option<(&str, &str)> {
    let trimmed = s.trim();
    if !trimmed.ends_with(']') || strip_whole_bracket_literal(trimmed).is_some() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut last_top_open_brack = None;
    let mut i = 0usize;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_str = true,
            b'[' => {
                if depth == 0 {
                    last_top_open_brack = Some(i);
                }
                depth += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 && i + 1 < bytes.len() {
                    last_top_open_brack = None;
                }
            }
            b'(' | b'{' => depth += 1,
            b')' | b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    let open_idx = last_top_open_brack?;
    let target = trimmed[..open_idx].trim();
    if target.is_empty() {
        return None;
    }
    let idx_expr = trimmed[open_idx + 1..trimmed.len() - 1].trim();
    Some((target, idx_expr))
}

fn rsplit_top_level_dot(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let mut paren = 0i32;
    let mut in_str = false;
    let mut last_dot = None;
    let mut i = 0usize;
    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => paren += 1,
            b')' | b']' | b'}' => paren -= 1,
            b'.' if paren == 0 => {
                let prev = if i > 0 { bytes[i - 1] } else { 0 };
                let next = bytes.get(i + 1).copied().unwrap_or(0);
                if prev != b'.' && next != b'.' {
                    last_dot = Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    let idx = last_dot?;
    let lhs = s[..idx].trim();
    let rhs = s[idx + 1..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    Some((lhs, rhs))
}

fn split_top_level_range(s: &str) -> Option<(&str, &str, bool)> {
    let bytes = s.as_bytes();
    let mut paren = 0i32;
    let mut in_str = false;
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => paren += 1,
            b')' | b']' | b'}' => paren -= 1,
            b'.' if paren == 0 && bytes[i + 1] == b'.' => {
                let inclusive = bytes.get(i + 2).copied() == Some(b'=');
                let end_start = if inclusive { i + 3 } else { i + 2 };
                let lhs = s[..i].trim();
                let rhs = s[end_start..].trim();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs, inclusive));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn strip_int_cast_suffix(s: &str) -> Option<&str> {
    for suffix in [
        " as usize",
        " as isize",
        " as u64",
        " as i64",
        " as u32",
        " as i32",
        " as u16",
        " as i16",
        " as u8",
        " as i8",
    ] {
        if let Some(inner) = s.strip_suffix(suffix) {
            return Some(inner.trim());
        }
    }
    None
}

fn parse_closure_expr(s: &str) -> Option<(Vec<String>, &str)> {
    let trimmed = s.trim();
    let rest = trimmed
        .strip_prefix("move |")
        .or_else(|| trimmed.strip_prefix('|'))?;
    let bytes = rest.as_bytes();
    let mut paren = 0i32;
    let mut close_bar = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' | b'[' | b'{' => paren += 1,
            b')' | b']' | b'}' => paren -= 1,
            b'|' if paren == 0 => {
                close_bar = Some(i);
                break;
            }
            _ => {}
        }
    }
    let bar_idx = close_bar?;
    let params_raw = rest[..bar_idx].trim();
    let mut body_raw = rest[bar_idx + 1..].trim();
    if let Some((_ret_ty, after_brace)) = body_raw.strip_prefix("->").and_then(|r| {
        let brace_pos = find_top_level_open_brace(r)?;
        Some((&r[..brace_pos], &r[brace_pos..]))
    }) {
        body_raw = after_brace.trim();
    }
    let params = if params_raw.is_empty() {
        Vec::new()
    } else {
        split_top_level(params_raw, ',')
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    };
    Some((params, body_raw))
}

fn unescape_rust_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('0') => out.push('\0'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn split_top_level_binary_tier<'a>(
    s: &'a str,
    ops: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    let bytes = s.as_bytes();
    let mut paren = 0i32;
    let mut in_str = false;
    let mut last_match: Option<(usize, &'static str)> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        if in_str {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => {
                in_str = true;
                i += 1;
                continue;
            }
            b'(' | b'[' | b'{' => {
                paren += 1;
                i += 1;
                continue;
            }
            b')' | b']' | b'}' => {
                paren -= 1;
                i += 1;
                continue;
            }
            _ if paren == 0 => {
                let mut matched_op: Option<&'static str> = None;
                for &op in ops {
                    let op_bytes = op.as_bytes();
                    if i + op_bytes.len() <= bytes.len()
                        && &bytes[i..i + op_bytes.len()] == op_bytes
                    {
                        let prev = if i > 0 { bytes[i - 1] } else { 0 };
                        let next = bytes.get(i + op_bytes.len()).copied().unwrap_or(0);
                        let ambiguous = match op {
                            "<" => next == b'=' || next == b'<' || prev == b'<',
                            ">" => {
                                next == b'='
                                    || next == b'>'
                                    || prev == b'-'
                                    || prev == b'>'
                                    || prev == b'='
                            }
                            "&" => {
                                next == b'&'
                                    || prev == b'&'
                                    || next == b'['
                                    || next == b'='
                                    || i == 0
                            }
                            "|" => next == b'|' || prev == b'|' || next == b'=',
                            "^" | "+" | "*" | "/" | "%" => next == b'=',
                            "-" => {
                                next == b'>'
                                    || next == b'='
                                    || i == 0
                                    || matches!(
                                        prev,
                                        b'+' | b'-'
                                            | b'*'
                                            | b'/'
                                            | b'%'
                                            | b'^'
                                            | b'&'
                                            | b'|'
                                            | b'('
                                            | b'['
                                            | b','
                                            | b'='
                                            | b'<'
                                            | b'>'
                                            | b'!'
                                    )
                            }
                            _ => false,
                        };
                        if !ambiguous {
                            matched_op = Some(op);
                            break;
                        }
                    }
                }
                if let Some(op) = matched_op {
                    let lhs = s[..i].trim();
                    let rhs = s[i + op.len()..].trim();
                    if !lhs.is_empty() && !rhs.is_empty() {
                        last_match = Some((i, op));
                    }
                    i += op.len();
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    last_match.map(|(pos, op)| (s[..pos].trim(), op, s[pos + op.len()..].trim()))
}

fn parse_int_literal(s: &str) -> Option<i64> {
    let mut clean = s.trim();
    if clean.is_empty() {
        return None;
    }
    let neg = if let Some(rest) = clean.strip_prefix('-') {
        clean = rest.trim();
        true
    } else {
        false
    };
    for suffix in [
        "usize", "isize", "u128", "i128", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
    ] {
        if let Some(stripped) = clean.strip_suffix(suffix) {
            clean = stripped;
            break;
        }
    }
    let no_underscores: String = clean.chars().filter(|&c| c != '_').collect();
    let val = if let Some(hex) = no_underscores
        .strip_prefix("0x")
        .or_else(|| no_underscores.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()?
    } else if let Some(bin) = no_underscores
        .strip_prefix("0b")
        .or_else(|| no_underscores.strip_prefix("0B"))
    {
        i64::from_str_radix(bin, 2).ok()?
    } else {
        no_underscores.parse::<i64>().ok()?
    };
    Some(if neg { -val } else { val })
}

struct EvalContext<'a> {
    functions: &'a [AstFunction],
    consts: &'a BTreeMap<String, RuntimeValue>,
    fuel_remaining: u64,
    fuel_consumed: u64,
    assertions_verified: usize,
    stdout: String,
    early_return: Option<RuntimeValue>,
}

enum ControlFlowOutcome {
    Continue(RuntimeValue),
    Return(RuntimeValue),
}

impl<'a> EvalContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn consume_step(
        &mut self,
        file_path: &str,
        line: usize,
        op_tag: u64,
        _lhs_type: u64,
        _rhs_type: u64,
        lhs_val: u64,
        rhs_val: u64,
    ) -> Result<u64, (usize, String)> {
        let is_div_zero = op_tag == 3 && rhs_val == 0;
        match evaluate_ast_step(
            self.fuel_remaining,
            line as u64,
            is_div_zero,
            0,
            lhs_val.wrapping_add(rhs_val),
            self.fuel_consumed,
        ) {
            AstEvalOutcome::EvaluatedValue(v, next_steps, _) => {
                self.fuel_remaining = self.fuel_remaining.saturating_sub(1);
                self.fuel_consumed = next_steps;
                Ok(v)
            }
            AstEvalOutcome::EvalFuelExhausted(_, err_line) => Err((
                err_line as usize,
                format!("EvalFuelExhausted at {file_path}:{err_line} (exceeded fuel budget)"),
            )),
            AstEvalOutcome::RuntimeDivideByZeroAtLine(err_line) => Err((
                err_line as usize,
                format!("RuntimeDivideByZeroAtLine at {file_path}:{err_line}"),
            )),
            AstEvalOutcome::UndefinedSymbolAtLine(err_line, sym_hash) => Err((
                err_line as usize,
                format!("UndefinedSymbolAtLine ({sym_hash}) at {file_path}:{err_line}"),
            )),
        }
    }

    fn eval_function(
        &mut self,
        func: &AstFunction,
        mut env: BTreeMap<String, RuntimeValue>,
    ) -> Result<(RuntimeValue, BTreeMap<String, RuntimeValue>), (usize, String)> {
        for (ck, cv) in self.consts {
            env.entry(ck.clone()).or_insert_with(|| cv.clone());
        }
        let saved_early = self.early_return.take();
        let mut last_val = RuntimeValue::Unit;
        for stmt in &func.body {
            match self.eval_stmt(&func.file_path, stmt, &mut env) {
                Ok(ControlFlowOutcome::Continue(v)) => {
                    if let Some(early) = self.early_return.take() {
                        self.early_return = saved_early;
                        return Ok((early, env));
                    }
                    last_val = v;
                }
                Ok(ControlFlowOutcome::Return(v)) => {
                    self.early_return = saved_early;
                    return Ok((v, env));
                }
                Err(e) => {
                    self.early_return = saved_early;
                    return Err(e);
                }
            }
        }
        self.early_return = saved_early;
        Ok((last_val, env))
    }

    fn eval_stmt(
        &mut self,
        file_path: &str,
        stmt: &AstStmt,
        env: &mut BTreeMap<String, RuntimeValue>,
    ) -> Result<ControlFlowOutcome, (usize, String)> {
        match stmt {
            AstStmt::Let {
                name, expr, line, ..
            } => {
                let _ = self.consume_step(file_path, *line, 0, 0, 0, 0, 0)?;
                let val = self.eval_expr(file_path, *line, expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                env.insert(name.clone(), val);
                Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
            }
            AstStmt::Assign {
                target,
                op,
                expr,
                line,
            } => {
                let rhs = self.eval_expr(file_path, *line, expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                let t_trim = target.trim();
                if let Some(open_brack) = t_trim.find('[') {
                    if t_trim.ends_with(']') {
                        let arr_name = t_trim[..open_brack].trim();
                        let idx_expr = t_trim[open_brack + 1..t_trim.len() - 1].trim();
                        let idx_val = self.eval_expr(file_path, *line, idx_expr, env)?;
                        if let Some(early) = self.early_return.take() {
                            return Ok(ControlFlowOutcome::Return(early));
                        }
                        if let RuntimeValue::Int(idx_i) = idx_val {
                            let idx = idx_i.max(0) as usize;
                            let cur_elem = env
                                .get(arr_name)
                                .and_then(|v| match v {
                                    RuntimeValue::List(items) => items.get(idx).cloned(),
                                    _ => None,
                                })
                                .unwrap_or(RuntimeValue::Int(0));
                            let updated =
                                self.apply_assign_op(file_path, *line, op, cur_elem, rhs)?;
                            if let Some(RuntimeValue::List(items)) = env.get_mut(arr_name) {
                                if idx < items.len() {
                                    items[idx] = updated;
                                }
                            }
                        }
                        return Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit));
                    }
                }
                if let Some((obj_name, field_name)) = t_trim.split_once('.') {
                    let cur_field = env
                        .get(obj_name.trim())
                        .and_then(|v| match v {
                            RuntimeValue::StructVal { fields, .. } => {
                                fields.get(field_name.trim()).cloned()
                            }
                            _ => None,
                        })
                        .unwrap_or(RuntimeValue::Int(0));
                    let updated = self.apply_assign_op(file_path, *line, op, cur_field, rhs)?;
                    if let Some(RuntimeValue::StructVal { fields, .. }) =
                        env.get_mut(obj_name.trim())
                    {
                        fields.insert(field_name.trim().to_string(), updated);
                    }
                } else {
                    let cur = env.get(t_trim).cloned().unwrap_or(RuntimeValue::Int(0));
                    let updated = self.apply_assign_op(file_path, *line, op, cur, rhs)?;
                    env.insert(t_trim.to_string(), updated);
                }
                Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
            }
            AstStmt::Assert {
                kind,
                lhs_expr,
                rhs_expr,
                line,
            } => {
                let lhs_val = self.eval_expr(file_path, *line, lhs_expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                let rhs_val = if let Some(r) = rhs_expr {
                    let v = self.eval_expr(file_path, *line, r, env)?;
                    if let Some(early) = self.early_return.take() {
                        return Ok(ControlFlowOutcome::Return(early));
                    }
                    v
                } else {
                    RuntimeValue::Bool(true)
                };

                let (lean_left, lean_right) = if *kind == 0 {
                    (lhs_val.to_lean_nat(), 1u64)
                } else if lhs_val == rhs_val {
                    let v = lhs_val.to_lean_nat();
                    (v, v)
                } else {
                    let l = lhs_val.to_lean_nat();
                    let r = rhs_val.to_lean_nat();
                    if l == r {
                        (l, l.wrapping_add(1))
                    } else {
                        (l, r)
                    }
                };

                let failed_line_arg = if (*kind == 0 && lean_left != 1)
                    || (*kind == 1 && lean_left != lean_right)
                    || (*kind == 2 && lean_left == lean_right)
                {
                    *line as u64
                } else {
                    0u64
                };

                match verify_test_assertion(
                    1,
                    (self.assertions_verified + 1) as u64,
                    0,
                    failed_line_arg,
                    lean_left,
                    lean_right,
                ) {
                    TestRunnerOutcome::TestSuitePassed(_, _) => {
                        self.assertions_verified += 1;
                        Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
                    }
                    TestRunnerOutcome::AssertionFailedAtLine(_, failed_line, left, right) => {
                        let msg = if *kind == 0 {
                            format!(
                                "assertion failed: `{lhs_expr}` evaluated to `{}` at {file_path}:{failed_line}",
                                lhs_val.display_value()
                            )
                        } else if *kind == 1 {
                            format!(
                                "assertion `left == right` failed at {file_path}:{failed_line}\n  left: `{}` (kernel={left})\n right: `{}` (kernel={right})",
                                lhs_val.display_value(),
                                rhs_val.display_value()
                            )
                        } else {
                            format!(
                                "assertion `left != right` failed at {file_path}:{failed_line}: both equal `{}`",
                                lhs_val.display_value()
                            )
                        };
                        Err((failed_line as usize, msg))
                    }
                    TestRunnerOutcome::NoTestsDiscovered => {
                        Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
                    }
                }
            }
            AstStmt::If {
                cond_expr,
                then_body,
                else_body,
                line,
            } => {
                let cond = self.eval_expr(file_path, *line, cond_expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                let is_true = matches!(cond, RuntimeValue::Bool(true));
                let branch = if is_true { then_body } else { else_body };
                let mut last = RuntimeValue::Unit;
                for s in branch {
                    match self.eval_stmt(file_path, s, env)? {
                        ControlFlowOutcome::Continue(v) => last = v,
                        ControlFlowOutcome::Return(v) => return Ok(ControlFlowOutcome::Return(v)),
                    }
                }
                Ok(ControlFlowOutcome::Continue(last))
            }
            AstStmt::While {
                cond_expr,
                body,
                line,
            } => {
                loop {
                    let _ = self.consume_step(file_path, *line, 0, 1, 1, 0, 0)?;
                    let cond = self.eval_expr(file_path, *line, cond_expr, env)?;
                    if let Some(early) = self.early_return.take() {
                        return Ok(ControlFlowOutcome::Return(early));
                    }
                    if !matches!(cond, RuntimeValue::Bool(true)) {
                        break;
                    }
                    for s in body {
                        if let ControlFlowOutcome::Return(v) = self.eval_stmt(file_path, s, env)? {
                            return Ok(ControlFlowOutcome::Return(v));
                        }
                    }
                }
                Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
            }
            AstStmt::For {
                pattern,
                iter_expr,
                body,
                line,
            } => {
                let items = self.eval_iterable(file_path, *line, iter_expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                for item in items {
                    let _ = self.consume_step(file_path, *line, 0, 0, 0, 0, 0)?;
                    bind_for_pattern(pattern, item, env);
                    for s in body {
                        if let ControlFlowOutcome::Return(v) = self.eval_stmt(file_path, s, env)? {
                            return Ok(ControlFlowOutcome::Return(v));
                        }
                    }
                }
                Ok(ControlFlowOutcome::Continue(RuntimeValue::Unit))
            }
            AstStmt::Return { expr, line, .. } => {
                let val = if let Some(e) = expr {
                    let v = self.eval_expr(file_path, *line, e, env)?;
                    if let Some(early) = self.early_return.take() {
                        return Ok(ControlFlowOutcome::Return(early));
                    }
                    v
                } else {
                    RuntimeValue::Unit
                };
                Ok(ControlFlowOutcome::Return(val))
            }
            AstStmt::ExprStmt { expr, line } => {
                let val = self.eval_expr(file_path, *line, expr, env)?;
                if let Some(early) = self.early_return.take() {
                    return Ok(ControlFlowOutcome::Return(early));
                }
                Ok(ControlFlowOutcome::Continue(val))
            }
        }
    }

    fn apply_assign_op(
        &mut self,
        file_path: &str,
        line: usize,
        op: &str,
        lhs: RuntimeValue,
        rhs: RuntimeValue,
    ) -> Result<RuntimeValue, (usize, String)> {
        if op == "=" {
            let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
            return Ok(rhs);
        }
        let RuntimeValue::Int(a) = lhs else {
            return Err((
                line,
                format!("non-integer assignment target at line {line}"),
            ));
        };
        let RuntimeValue::Int(b) = rhs else {
            return Err((line, format!("non-integer assignment value at line {line}")));
        };
        let op_tag = match op {
            "+=" => 0,
            "-=" => 1,
            "*=" => 2,
            "/=" | "%=" => 3,
            "&=" => 4,
            "|=" => 5,
            "^=" => 6,
            _ => 0,
        };
        let _ = self.consume_step(file_path, line, op_tag, 0, 0, a as u64, b as u64)?;
        let res = match op {
            "+=" => a.wrapping_add(b),
            "-=" => a.wrapping_sub(b),
            "*=" => a.wrapping_mul(b),
            "/=" => {
                if b == 0 {
                    return Err((line, format!("RefusedDivisionByZero at line {line}")));
                }
                a / b
            }
            "%=" => {
                if b == 0 {
                    return Err((line, format!("RefusedDivisionByZero at line {line}")));
                }
                a % b
            }
            "&=" => a & b,
            "|=" => a | b,
            "^=" => a ^ b,
            "<<=" => a.wrapping_shl(b as u32),
            ">>=" => a.wrapping_shr(b as u32),
            _ => b,
        };
        Ok(RuntimeValue::Int(res))
    }

    fn eval_iterable(
        &mut self,
        file_path: &str,
        line: usize,
        iter_expr: &str,
        env: &mut BTreeMap<String, RuntimeValue>,
    ) -> Result<Vec<RuntimeValue>, (usize, String)> {
        let trimmed = strip_outer_parens(iter_expr.trim());
        // Check `a.iter().zip(b.iter())`
        if let Some((left_part, right_part)) = trimmed.split_once(".iter().zip(") {
            let right_clean = right_part
                .strip_suffix(".iter())")
                .or_else(|| right_part.strip_suffix(')'))
                .unwrap_or(right_part);
            let left_val = self.eval_expr(file_path, line, left_part.trim(), env)?;
            let right_val = self.eval_expr(file_path, line, right_clean.trim(), env)?;
            if let (RuntimeValue::List(xs), RuntimeValue::List(ys)) = (left_val, right_val) {
                return Ok(xs
                    .into_iter()
                    .zip(ys)
                    .map(|(x, y)| RuntimeValue::Tuple(vec![x, y]))
                    .collect());
            }
        }
        // Check top-level range `start..end` or `start..=end`
        if let Some((start_s, end_s, inclusive)) = split_top_level_range(trimmed) {
            let RuntimeValue::Int(start_v) =
                self.eval_expr(file_path, line, start_s.trim(), env)?
            else {
                return Err((line, format!("range start must be integer at line {line}")));
            };
            let RuntimeValue::Int(end_v) = self.eval_expr(file_path, line, end_s.trim(), env)?
            else {
                return Err((line, format!("range end must be integer at line {line}")));
            };
            let limit = if inclusive { end_v + 1 } else { end_v };
            let mut out = Vec::new();
            for v in start_v..limit {
                out.push(RuntimeValue::Int(v));
            }
            return Ok(out);
        }
        let val = self.eval_expr(file_path, line, trimmed, env)?;
        match val {
            RuntimeValue::List(items) => Ok(items),
            _ => Ok(Vec::new()),
        }
    }

    fn eval_match_pattern(
        &mut self,
        file_path: &str,
        line: usize,
        pat: &str,
        scrut_val: &RuntimeValue,
        env: &mut BTreeMap<String, RuntimeValue>,
    ) -> Result<bool, (usize, String)> {
        for sub in split_top_level(pat, '|') {
            let p = sub.trim();
            if p == "_" {
                return Ok(true);
            }
            if p == "None" {
                if matches!(scrut_val, RuntimeValue::OptionVal(None)) {
                    return Ok(true);
                }
                continue;
            }
            if let Some(inner_pat) = p.strip_prefix("Some(").and_then(|s| s.strip_suffix(')')) {
                if let RuntimeValue::OptionVal(Some(inner_val)) = scrut_val {
                    if self.eval_match_pattern(file_path, line, inner_pat.trim(), inner_val, env)? {
                        return Ok(true);
                    }
                }
                continue;
            }
            if let Some(inner_pat) = p.strip_prefix("Ok(").and_then(|s| s.strip_suffix(')')) {
                if let RuntimeValue::ResultVal(Ok(inner_val)) = scrut_val {
                    if self.eval_match_pattern(file_path, line, inner_pat.trim(), inner_val, env)? {
                        return Ok(true);
                    }
                }
                continue;
            }
            if let Some(inner_pat) = p.strip_prefix("Err(").and_then(|s| s.strip_suffix(')')) {
                if let RuntimeValue::ResultVal(Err(inner_val)) = scrut_val {
                    if self.eval_match_pattern(file_path, line, inner_pat.trim(), inner_val, env)? {
                        return Ok(true);
                    }
                }
                continue;
            }
            // Enum variant with payload `Enum::Variant(inner)`
            if let Some((callee, inner_pat)) = split_outermost_call(p) {
                if let Some((enum_prefix, var_name)) = callee.split_once("::") {
                    if let RuntimeValue::EnumVariant {
                        enum_name,
                        variant,
                        payload: Some(inner_val),
                    } = scrut_val
                    {
                        if enum_name == enum_prefix.trim()
                            && variant == var_name.trim()
                            && self.eval_match_pattern(
                                file_path,
                                line,
                                inner_pat.trim(),
                                inner_val,
                                env,
                            )?
                        {
                            return Ok(true);
                        }
                    }
                    continue;
                }
            }
            // Unit enum variant `Enum::Variant`
            if let Some((enum_prefix, var_name)) = p.split_once("::") {
                if !var_name.contains('(') {
                    if let RuntimeValue::EnumVariant {
                        enum_name,
                        variant,
                        payload: None,
                    } = scrut_val
                    {
                        if enum_name == enum_prefix.trim() && variant == var_name.trim() {
                            return Ok(true);
                        }
                    }
                    continue;
                }
            }
            // Tuple pattern `(p1, p2)`
            if p.starts_with('(') && p.ends_with(')') {
                let inner = &p[1..p.len() - 1];
                let sub_pats = split_top_level(inner, ',');
                if sub_pats.len() > 1 {
                    if let RuntimeValue::Tuple(vals) = scrut_val {
                        if sub_pats.len() == vals.len() {
                            let mut all_ok = true;
                            for (sp, sv) in sub_pats.iter().zip(vals.iter()) {
                                if !self.eval_match_pattern(file_path, line, sp.trim(), sv, env)? {
                                    all_ok = false;
                                    break;
                                }
                            }
                            if all_ok {
                                return Ok(true);
                            }
                        }
                    }
                    continue;
                }
            }
            if let Some((start_s, end_s)) = p.split_once("..=") {
                if let (Some(a), Some(b), RuntimeValue::Int(v)) = (
                    parse_int_literal(start_s.trim()),
                    parse_int_literal(end_s.trim()),
                    scrut_val,
                ) {
                    if *v >= a && *v <= b {
                        return Ok(true);
                    }
                }
                continue;
            }
            if let Some((start_s, end_s)) = p.split_once("..") {
                if let (Some(a), Some(b), RuntimeValue::Int(v)) = (
                    parse_int_literal(start_s.trim()),
                    parse_int_literal(end_s.trim()),
                    scrut_val,
                ) {
                    if *v >= a && *v < b {
                        return Ok(true);
                    }
                }
                continue;
            }
            if is_pattern_ident(p) && !env.contains_key(p) && !self.consts.contains_key(p) {
                env.insert(p.to_string(), scrut_val.clone());
                return Ok(true);
            }
            let pat_val = self.eval_expr(file_path, line, p, env)?;
            if &pat_val == scrut_val {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn eval_block_or_expr(
        &mut self,
        file_path: &str,
        line: usize,
        body: &str,
        env: &mut BTreeMap<String, RuntimeValue>,
    ) -> Result<RuntimeValue, (usize, String)> {
        let trimmed = body.trim();
        if let Some(inner) = trimmed.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            let synth_lines = split_inline_block_lines(line, inner);
            let stmts = parse_stmt_block(file_path, &synth_lines)
                .map_err(|d| (d.line, format!("{}: {}", d.code, d.message)))?;
            let mut last = RuntimeValue::Unit;
            for st in &stmts {
                if let AstStmt::Return {
                    expr: Some(e),
                    is_tail: true,
                    line: l,
                } = st
                {
                    last = self.eval_expr(file_path, *l, e, env)?;
                } else {
                    match self.eval_stmt(file_path, st, env)? {
                        ControlFlowOutcome::Continue(v) => last = v,
                        ControlFlowOutcome::Return(v) => {
                            self.early_return = Some(v.clone());
                            return Ok(v);
                        }
                    }
                }
            }
            return Ok(last);
        }
        if let Some((target, op, rhs)) = split_assignment(trimmed) {
            let assign_stmt = AstStmt::Assign {
                target: target.trim().to_string(),
                op: op.to_string(),
                expr: rhs.trim().to_string(),
                line,
            };
            let _ = self.eval_stmt(file_path, &assign_stmt, env)?;
            return Ok(RuntimeValue::Unit);
        }
        self.eval_expr(file_path, line, trimmed, env)
    }

    fn eval_expr(
        &mut self,
        file_path: &str,
        line: usize,
        expr: &str,
        env: &mut BTreeMap<String, RuntimeValue>,
    ) -> Result<RuntimeValue, (usize, String)> {
        if self.early_return.is_some() {
            return Ok(RuntimeValue::Int(0));
        }
        let s = strip_outer_parens(expr.trim());
        if s.is_empty() || s == "()" {
            return Ok(RuntimeValue::Unit);
        }
        if s == "true" {
            return Ok(RuntimeValue::Bool(true));
        }
        if s == "false" {
            return Ok(RuntimeValue::Bool(false));
        }
        if s == "None" {
            return Ok(RuntimeValue::OptionVal(None));
        }
        if s == "Vec::new()" || s == "vec![]" {
            return Ok(RuntimeValue::List(Vec::new()));
        }
        if s == "String::new()" {
            return Ok(RuntimeValue::Str(String::new()));
        }
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            return Ok(RuntimeValue::Str(unescape_rust_string(&s[1..s.len() - 1])));
        }
        if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
            return Ok(RuntimeValue::Str(unescape_rust_string(&s[1..s.len() - 1])));
        }
        if let Some((callee, inner)) = split_outermost_call(s) {
            match callee {
                "Some" => {
                    let v = self.eval_expr(file_path, line, inner, env)?;
                    if self.early_return.is_some() {
                        return Ok(RuntimeValue::Unit);
                    }
                    return Ok(RuntimeValue::OptionVal(Some(Box::new(v))));
                }
                "Ok" => {
                    let v = self.eval_expr(file_path, line, inner, env)?;
                    if self.early_return.is_some() {
                        return Ok(RuntimeValue::Unit);
                    }
                    return Ok(RuntimeValue::ResultVal(Ok(Box::new(v))));
                }
                "Err" => {
                    let v = self.eval_expr(file_path, line, inner, env)?;
                    if self.early_return.is_some() {
                        return Ok(RuntimeValue::Unit);
                    }
                    return Ok(RuntimeValue::ResultVal(Err(Box::new(v))));
                }
                "String::from" => {
                    return self.eval_expr(file_path, line, inner, env);
                }
                "format!" => {
                    let args = split_top_level(inner, ',');
                    if let Some(fmt_raw) = args.first() {
                        let raw_t = fmt_raw.trim();
                        let mut out_line =
                            if raw_t.starts_with('"') && raw_t.ends_with('"') && raw_t.len() >= 2 {
                                unescape_rust_string(&raw_t[1..raw_t.len() - 1])
                            } else {
                                raw_t.trim_matches('"').to_string()
                            };
                        for arg in args.iter().skip(1) {
                            let v = self.eval_expr(file_path, line, arg.trim(), env)?;
                            if self.early_return.is_some() {
                                return Ok(RuntimeValue::Unit);
                            }
                            out_line = out_line.replacen("{}", &v.display_value(), 1);
                        }
                        return Ok(RuntimeValue::Str(out_line));
                    }
                    return Ok(RuntimeValue::Str(String::new()));
                }
                "println!" => {
                    let args = split_top_level(inner, ',');
                    if let Some(fmt_raw) = args.first() {
                        let raw_t = fmt_raw.trim();
                        let mut out_line =
                            if raw_t.starts_with('"') && raw_t.ends_with('"') && raw_t.len() >= 2 {
                                unescape_rust_string(&raw_t[1..raw_t.len() - 1])
                            } else {
                                raw_t.trim_matches('"').to_string()
                            };
                        for arg in args.iter().skip(1) {
                            let v = self.eval_expr(file_path, line, arg.trim(), env)?;
                            if self.early_return.is_some() {
                                return Ok(RuntimeValue::Unit);
                            }
                            out_line = out_line.replacen("{}", &v.display_value(), 1);
                        }
                        self.stdout.push_str(&out_line);
                        self.stdout.push('\n');
                    }
                    return Ok(RuntimeValue::Unit);
                }
                _ => {}
            }
        }
        if let Some(inner) = strip_whole_bracket_literal(s) {
            let semi_parts = split_top_level(inner, ';');
            if semi_parts.len() == 2 {
                let elem = self.eval_expr(file_path, line, semi_parts[0].trim(), env)?;
                let count_v = self.eval_expr(file_path, line, semi_parts[1].trim(), env)?;
                let n = match count_v {
                    RuntimeValue::Int(c) => c.max(0) as usize,
                    _ => 0,
                };
                return Ok(RuntimeValue::List(vec![elem; n]));
            }
            let mut items = Vec::new();
            for part in split_top_level(inner, ',') {
                let p = part.trim();
                if !p.is_empty() {
                    items.push(self.eval_expr(file_path, line, p, env)?);
                }
            }
            return Ok(RuntimeValue::List(items));
        }

        // `match scrutinee { pat => body, ... }`
        if let Some((scrutinee, arms)) = parse_match_expr(s) {
            let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
            let scrut_val = self.eval_expr(file_path, line, scrutinee, env)?;
            for (pat, body) in arms {
                let mut arm_env = env.clone();
                if self.eval_match_pattern(file_path, line, pat, &scrut_val, &mut arm_env)? {
                    let res = self.eval_block_or_expr(file_path, line, body, &mut arm_env)?;
                    for (k, v) in env.iter_mut() {
                        if let Some(updated) = arm_env.get(k) {
                            *v = updated.clone();
                        }
                    }
                    return Ok(res);
                }
            }
            return Ok(RuntimeValue::Unit);
        }

        if s.starts_with('{') && s.ends_with('}') {
            return self.eval_block_or_expr(file_path, line, s, env);
        }

        // Inline `if cond { a } else { b }`
        if let Some(rest) = s.strip_prefix("if ") {
            if let Some((cond_part, branches)) = rest.split_once('{') {
                if let Some((then_e, else_part)) = branches.split_once("} else {") {
                    if let Some(else_e) = else_part.strip_suffix('}') {
                        let cond = self.eval_expr(file_path, line, cond_part.trim(), env)?;
                        if matches!(cond, RuntimeValue::Bool(true)) {
                            return self.eval_block_or_expr(file_path, line, then_e.trim(), env);
                        } else {
                            return self.eval_block_or_expr(file_path, line, else_e.trim(), env);
                        }
                    }
                }
            }
        }

        // Top-level range `start..end` or `start..=end`
        if let Some((start_s, end_s, inclusive)) = split_top_level_range(s) {
            let RuntimeValue::Int(start_v) = self.eval_expr(file_path, line, start_s, env)? else {
                return Err((line, format!("range start must be integer at line {line}")));
            };
            let RuntimeValue::Int(end_v) = self.eval_expr(file_path, line, end_s, env)? else {
                return Err((line, format!("range end must be integer at line {line}")));
            };
            let limit = if inclusive { end_v + 1 } else { end_v };
            let mut out = Vec::new();
            for v in start_v..limit {
                out.push(RuntimeValue::Int(v));
            }
            return Ok(RuntimeValue::List(out));
        }

        // Logical operators `||`, `&&`
        if let Some((lhs, _, rhs)) = split_top_level_binary_tier(s, &["||"]) {
            let lv = self.eval_expr(file_path, line, lhs, env)?;
            let rv = self.eval_expr(file_path, line, rhs, env)?;
            let _ = self.consume_step(
                file_path,
                line,
                5,
                lv.runtime_type().lean_type_tag(),
                rv.runtime_type().lean_type_tag(),
                lv.to_lean_nat(),
                rv.to_lean_nat(),
            )?;
            return Ok(RuntimeValue::Bool(
                matches!(lv, RuntimeValue::Bool(true)) || matches!(rv, RuntimeValue::Bool(true)),
            ));
        }
        if let Some((lhs, _, rhs)) = split_top_level_binary_tier(s, &["&&"]) {
            let lv = self.eval_expr(file_path, line, lhs, env)?;
            let rv = self.eval_expr(file_path, line, rhs, env)?;
            let _ = self.consume_step(
                file_path,
                line,
                4,
                lv.runtime_type().lean_type_tag(),
                rv.runtime_type().lean_type_tag(),
                lv.to_lean_nat(),
                rv.to_lean_nat(),
            )?;
            return Ok(RuntimeValue::Bool(
                matches!(lv, RuntimeValue::Bool(true)) && matches!(rv, RuntimeValue::Bool(true)),
            ));
        }

        // Comparison operators (equal precedence tier)
        if let Some((lhs, op, rhs)) =
            split_top_level_binary_tier(s, &["==", "!=", "<=", ">=", "<", ">"])
        {
            let lv = self.eval_expr(file_path, line, lhs, env)?;
            let rv = self.eval_expr(file_path, line, rhs, env)?;
            let _ = self.consume_step(
                file_path,
                line,
                7,
                lv.runtime_type().lean_type_tag(),
                rv.runtime_type().lean_type_tag(),
                lv.to_lean_nat(),
                rv.to_lean_nat(),
            )?;
            let res = match (&lv, &rv, op) {
                (a, b, "==") => a == b,
                (a, b, "!=") => a != b,
                (RuntimeValue::Int(a), RuntimeValue::Int(b), "<=") => a <= b,
                (RuntimeValue::Int(a), RuntimeValue::Int(b), ">=") => a >= b,
                (RuntimeValue::Int(a), RuntimeValue::Int(b), "<") => a < b,
                (RuntimeValue::Int(a), RuntimeValue::Int(b), ">") => a > b,
                (RuntimeValue::Str(a), RuntimeValue::Str(b), "<=") => a <= b,
                (RuntimeValue::Str(a), RuntimeValue::Str(b), ">=") => a >= b,
                (RuntimeValue::Str(a), RuntimeValue::Str(b), "<") => a < b,
                (RuntimeValue::Str(a), RuntimeValue::Str(b), ">") => a > b,
                _ => false,
            };
            return Ok(RuntimeValue::Bool(res));
        }

        // Bitwise and arithmetic binary operators (lowest precedence tier first;
        // rightmost operator within each tier for left-to-right associativity)
        for tier in [
            &["|"][..],
            &["^"][..],
            &["&"][..],
            &["<<", ">>"][..],
            &["+", "-"][..],
            &["*", "/", "%"][..],
        ] {
            if let Some((lhs, op, rhs)) = split_top_level_binary_tier(s, tier) {
                let lv = self.eval_expr(file_path, line, lhs, env)?;
                if self.early_return.is_some() {
                    return Ok(RuntimeValue::Int(0));
                }
                let rv = self.eval_expr(file_path, line, rhs, env)?;
                if self.early_return.is_some() {
                    return Ok(RuntimeValue::Int(0));
                }
                let op_tag = match op {
                    "+" => 0,
                    "-" => 1,
                    "*" => 2,
                    "/" | "%" => 3,
                    "&" => 4,
                    "|" => 5,
                    "^" => 6,
                    _ => 0,
                };
                let RuntimeValue::Int(a) = lv else {
                    return Err((
                        line,
                        format!("operator `{op}` requires integer left operand at line {line}"),
                    ));
                };
                let RuntimeValue::Int(b) = rv else {
                    return Err((
                        line,
                        format!("operator `{op}` requires integer right operand at line {line}"),
                    ));
                };
                let _ = self.consume_step(file_path, line, op_tag, 0, 0, a as u64, b as u64)?;
                let val = match op {
                    "+" => a.wrapping_add(b),
                    "-" => a.wrapping_sub(b),
                    "*" => a.wrapping_mul(b),
                    "/" => {
                        if b == 0 {
                            return Err((line, format!("RefusedDivisionByZero at line {line}")));
                        }
                        a / b
                    }
                    "%" => {
                        if b == 0 {
                            return Err((line, format!("RefusedDivisionByZero at line {line}")));
                        }
                        a % b
                    }
                    "&" => a & b,
                    "|" => a | b,
                    "^" => a ^ b,
                    "<<" => a.wrapping_shl(b as u32),
                    ">>" => a.wrapping_shr(b as u32),
                    _ => a,
                };
                return Ok(RuntimeValue::Int(val));
            }
        }

        // Cast suffix `expr as usize`, `expr as i64`, etc.
        if let Some(inner) = strip_int_cast_suffix(s) {
            let v = self.eval_expr(file_path, line, inner, env)?;
            return match v {
                RuntimeValue::Bool(b) => Ok(RuntimeValue::Int(i64::from(b))),
                other => Ok(other),
            };
        }

        // Unary `!`
        if let Some(inner) = s.strip_prefix('!') {
            let v = self.eval_expr(file_path, line, inner, env)?;
            return match v {
                RuntimeValue::Bool(b) => Ok(RuntimeValue::Bool(!b)),
                RuntimeValue::Int(x) => Ok(RuntimeValue::Int(!x)),
                other => Ok(other),
            };
        }

        if let Some(v) = parse_int_literal(s) {
            return Ok(RuntimeValue::Int(v));
        }

        // Unary `-` on non-literal expression (e.g. `-x` or `-(a + b)`)
        if let Some(inner) = s.strip_prefix('-') {
            let v = self.eval_expr(file_path, line, inner, env)?;
            if let RuntimeValue::Int(x) = v {
                return Ok(RuntimeValue::Int(x.wrapping_neg()));
            }
        }

        // Postfix `?` operator on Result<T, E> or Option<T>
        if let Some(inner) = s.strip_suffix('?') {
            let inner_val = self.eval_expr(file_path, line, inner.trim(), env)?;
            return match inner_val {
                RuntimeValue::ResultVal(Ok(v)) => Ok(*v),
                RuntimeValue::ResultVal(Err(e)) => {
                    self.early_return = Some(RuntimeValue::ResultVal(Err(e)));
                    Ok(RuntimeValue::Int(0))
                }
                RuntimeValue::OptionVal(Some(v)) => Ok(*v),
                RuntimeValue::OptionVal(None) => {
                    self.early_return = Some(RuntimeValue::OptionVal(None));
                    Ok(RuntimeValue::Int(0))
                }
                other => Ok(other),
            };
        }

        // Tuple literal `(a, b)`
        if s.starts_with('(') && s.ends_with(')') {
            let inner = &s[1..s.len() - 1];
            if check_delimiter_balance(inner).is_ok() {
                let parts = split_top_level(inner, ',');
                if parts.len() > 1 {
                    let mut items = Vec::with_capacity(parts.len());
                    for p in parts {
                        let pt = p.trim();
                        if !pt.is_empty() {
                            items.push(self.eval_expr(file_path, line, pt, env)?);
                        }
                    }
                    return Ok(RuntimeValue::Tuple(items));
                }
            }
        }

        // Struct literal `StructName { f1: v1, f2: v2 }`
        if let Some(open_brace) = find_top_level_open_brace(s) {
            if s.ends_with('}') {
                let sname = s[..open_brace].trim();
                if !sname.is_empty() && sname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    let inner = &s[open_brace + 1..s.len() - 1];
                    let mut fields = BTreeMap::new();
                    for part in split_top_level(inner, ',') {
                        let pt = part.trim();
                        if pt.is_empty() {
                            continue;
                        }
                        if let Some((fk, fv)) = pt.split_once(':') {
                            let val = self.eval_expr(file_path, line, fv.trim(), env)?;
                            fields.insert(fk.trim().to_string(), val);
                        }
                    }
                    return Ok(RuntimeValue::StructVal {
                        name: sname.to_string(),
                        fields,
                    });
                }
            }
        }

        // Index expression `arr[idx]`
        if let Some((target_s, idx_s)) = split_trailing_index(s) {
            let target_v = self.eval_expr(file_path, line, target_s, env)?;
            let idx_v = self.eval_expr(file_path, line, idx_s, env)?;
            if let (RuntimeValue::List(items), RuntimeValue::Int(i)) = (target_v, idx_v) {
                if let Some(elem) = items.get(i as usize) {
                    return Ok(elem.clone());
                }
            }
        }

        // Method or function call `callee(args...)`
        if let Some((callee, args_str)) = split_outermost_call(s) {
            if callee == "Vec::with_capacity" || callee == "Vec::new" {
                return Ok(RuntimeValue::List(Vec::new()));
            }
            let raw_args: Vec<&str> = split_top_level(args_str, ',')
                .into_iter()
                .map(|a| a.trim())
                .filter(|a| !a.is_empty())
                .collect();

            // Check method call `receiver.method(args...)`
            if let Some((recv_expr, method_name)) = rsplit_top_level_dot(callee) {
                let recv_name = recv_expr.trim();
                let mname = method_name.trim();

                // Higher-order iterator combinators with closure arguments
                match mname {
                    "map" if raw_args.len() == 1 => {
                        if let Some((params, body)) = parse_closure_expr(raw_args[0]) {
                            let items = self.eval_iterable(file_path, line, recv_name, env)?;
                            let mut mapped = Vec::with_capacity(items.len());
                            for item in items {
                                let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
                                let mut clos_env = env.clone();
                                if params.len() == 1 {
                                    bind_for_pattern(&params[0], item, &mut clos_env);
                                } else if params.len() == 2 {
                                    if let RuntimeValue::Tuple(vals) = item {
                                        if vals.len() == 2 {
                                            bind_for_pattern(
                                                &params[0],
                                                vals[0].clone(),
                                                &mut clos_env,
                                            );
                                            bind_for_pattern(
                                                &params[1],
                                                vals[1].clone(),
                                                &mut clos_env,
                                            );
                                        }
                                    }
                                }
                                let out_v =
                                    self.eval_block_or_expr(file_path, line, body, &mut clos_env)?;
                                mapped.push(out_v);
                            }
                            return Ok(RuntimeValue::List(mapped));
                        }
                    }
                    "filter" if raw_args.len() == 1 => {
                        if let Some((params, body)) = parse_closure_expr(raw_args[0]) {
                            let items = self.eval_iterable(file_path, line, recv_name, env)?;
                            let mut kept = Vec::new();
                            for item in items {
                                let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
                                let mut clos_env = env.clone();
                                if params.len() == 1 {
                                    bind_for_pattern(&params[0], item.clone(), &mut clos_env);
                                } else if params.len() == 2 {
                                    if let RuntimeValue::Tuple(ref vals) = item {
                                        if vals.len() == 2 {
                                            bind_for_pattern(
                                                &params[0],
                                                vals[0].clone(),
                                                &mut clos_env,
                                            );
                                            bind_for_pattern(
                                                &params[1],
                                                vals[1].clone(),
                                                &mut clos_env,
                                            );
                                        }
                                    }
                                }
                                let cond =
                                    self.eval_block_or_expr(file_path, line, body, &mut clos_env)?;
                                if matches!(cond, RuntimeValue::Bool(true)) {
                                    kept.push(item);
                                }
                            }
                            return Ok(RuntimeValue::List(kept));
                        }
                    }
                    "any" | "all" if raw_args.len() == 1 => {
                        if let Some((params, body)) = parse_closure_expr(raw_args[0]) {
                            let items = self.eval_iterable(file_path, line, recv_name, env)?;
                            let is_any = mname == "any";
                            for item in items {
                                let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
                                let mut clos_env = env.clone();
                                if let Some(first_p) = params.first() {
                                    bind_for_pattern(first_p, item, &mut clos_env);
                                }
                                let cond =
                                    self.eval_block_or_expr(file_path, line, body, &mut clos_env)?;
                                let b = matches!(cond, RuntimeValue::Bool(true));
                                if is_any && b {
                                    return Ok(RuntimeValue::Bool(true));
                                }
                                if !is_any && !b {
                                    return Ok(RuntimeValue::Bool(false));
                                }
                            }
                            return Ok(RuntimeValue::Bool(!is_any));
                        }
                    }
                    "fold" if raw_args.len() >= 2 => {
                        let clos_joined = raw_args[1..].join(", ");
                        if let Some((params, body)) = parse_closure_expr(&clos_joined) {
                            let mut acc = self.eval_expr(file_path, line, raw_args[0], env)?;
                            let items = self.eval_iterable(file_path, line, recv_name, env)?;
                            for item in items {
                                let _ = self.consume_step(file_path, line, 0, 0, 0, 0, 0)?;
                                let mut clos_env = env.clone();
                                if let Some(acc_p) = params.first() {
                                    bind_for_pattern(acc_p, acc.clone(), &mut clos_env);
                                }
                                if let Some(item_p) = params.get(1) {
                                    bind_for_pattern(item_p, item, &mut clos_env);
                                }
                                acc =
                                    self.eval_block_or_expr(file_path, line, body, &mut clos_env)?;
                            }
                            return Ok(acc);
                        }
                    }
                    _ => {}
                }

                let mut eval_args = Vec::with_capacity(raw_args.len());
                for at in &raw_args {
                    eval_args.push(self.eval_expr(file_path, line, at, env)?);
                }

                // Mutable Vec methods operating in-place on `env`
                match mname {
                    "push" => {
                        if let Some(arg) = eval_args.first().cloned() {
                            if let Some(RuntimeValue::List(items)) = env.get_mut(recv_name) {
                                items.push(arg);
                                return Ok(RuntimeValue::Unit);
                            }
                        }
                    }
                    "pop" => {
                        if let Some(RuntimeValue::List(items)) = env.get_mut(recv_name) {
                            let popped = items.pop().map(Box::new);
                            return Ok(RuntimeValue::OptionVal(popped));
                        }
                    }
                    "clear" => {
                        if let Some(RuntimeValue::List(items)) = env.get_mut(recv_name) {
                            items.clear();
                            return Ok(RuntimeValue::Unit);
                        }
                    }
                    "reverse" => {
                        if let Some(RuntimeValue::List(items)) = env.get_mut(recv_name) {
                            items.reverse();
                            return Ok(RuntimeValue::Unit);
                        }
                    }
                    "sort" => {
                        if let Some(RuntimeValue::List(items)) = env.get_mut(recv_name) {
                            items.sort_by(|a, b| match (a, b) {
                                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => x.cmp(y),
                                (RuntimeValue::Str(x), RuntimeValue::Str(y)) => x.cmp(y),
                                _ => std::cmp::Ordering::Equal,
                            });
                            return Ok(RuntimeValue::Unit);
                        }
                    }
                    _ => {}
                }

                let recv_val = self.eval_expr(file_path, line, recv_name, env)?;

                // Check if receiver is a struct with an `impl` method in workspace
                if let RuntimeValue::StructVal { ref name, .. } = recv_val {
                    if let Some(method_fn) = self.functions.iter().find(|f| {
                        f.name == mname && f.impl_struct.as_deref() == Some(name.as_str())
                    }) {
                        let mut sub_env = BTreeMap::new();
                        sub_env.insert("self".to_string(), recv_val);
                        for (param, arg_val) in method_fn
                            .params
                            .iter()
                            .filter(|p| !p.is_self)
                            .zip(eval_args)
                        {
                            sub_env.insert(param.name.clone(), arg_val);
                        }
                        let (ret, updated_env) = self.eval_function(method_fn, sub_env)?;
                        if let Some(updated_self) = updated_env.get("self") {
                            env.insert(recv_name.to_string(), updated_self.clone());
                        }
                        return Ok(ret);
                    }
                }

                return self.eval_builtin_method(line, recv_val, mname, &eval_args);
            }

            let mut eval_args = Vec::with_capacity(raw_args.len());
            for at in &raw_args {
                eval_args.push(self.eval_expr(file_path, line, at, env)?);
            }

            // Standalone function call (possibly qualified `module::fn_name`)
            let short_name = callee.rsplit("::").next().unwrap_or(callee).trim();
            if let Some(target_fn) = self.functions.iter().find(|f| f.name == short_name) {
                let mut sub_env = BTreeMap::new();
                for (param, arg_val) in target_fn.params.iter().zip(eval_args) {
                    sub_env.insert(param.name.clone(), arg_val);
                }
                let (ret, _) = self.eval_function(target_fn, sub_env)?;
                return Ok(ret);
            }

            // Enum variant constructor with payload `EnumName::Variant(arg)`
            if let Some((type_prefix, variant_name)) = callee.split_once("::") {
                if type_prefix
                    .trim()
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
                {
                    let payload = if eval_args.len() == 1 {
                        eval_args.into_iter().next().map(Box::new)
                    } else if eval_args.is_empty() {
                        None
                    } else {
                        Some(Box::new(RuntimeValue::Tuple(eval_args)))
                    };
                    return Ok(RuntimeValue::EnumVariant {
                        enum_name: type_prefix.trim().to_string(),
                        variant: variant_name.trim().to_string(),
                        payload,
                    });
                }
            }
        }

        // Struct field access `obj.field` or tuple `.0` / `.1`
        if let Some((recv_expr, field_name)) = rsplit_top_level_dot(s) {
            let recv_val = self.eval_expr(file_path, line, recv_expr.trim(), env)?;
            match recv_val {
                RuntimeValue::StructVal { fields, .. } => {
                    if let Some(v) = fields.get(field_name.trim()) {
                        return Ok(v.clone());
                    }
                }
                RuntimeValue::Tuple(items) => {
                    if let Ok(idx) = field_name.trim().parse::<usize>() {
                        if let Some(v) = items.get(idx) {
                            return Ok(v.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // Unit enum variant `EnumName::Variant`
        if let Some((type_prefix, variant_name)) = s.split_once("::") {
            if type_prefix
                .trim()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
                && !variant_name.contains('(')
            {
                return Ok(RuntimeValue::EnumVariant {
                    enum_name: type_prefix.trim().to_string(),
                    variant: variant_name.trim().to_string(),
                    payload: None,
                });
            }
        }

        let var_lookup = s
            .strip_prefix("&mut ")
            .or_else(|| s.strip_prefix('&'))
            .or_else(|| s.strip_prefix('*'))
            .unwrap_or(s)
            .trim();
        if let Some(val) = env.get(var_lookup) {
            return Ok(val.clone());
        }
        if let Some(val) = self.consts.get(var_lookup) {
            return Ok(val.clone());
        }

        Err((
            line,
            format!("unresolved identifier or expression `{s}` at {file_path}:{line}"),
        ))
    }

    fn eval_builtin_method(
        &mut self,
        line: usize,
        recv: RuntimeValue,
        method: &str,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, (usize, String)> {
        if self.early_return.is_some() {
            return Ok(RuntimeValue::Unit);
        }
        match (recv, method) {
            (RuntimeValue::Int(a), "saturating_add") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.saturating_add(b)))
            }
            (RuntimeValue::Int(a), "saturating_sub") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.saturating_sub(b)))
            }
            (RuntimeValue::Int(a), "saturating_mul") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.saturating_mul(b)))
            }
            (RuntimeValue::Int(a), "wrapping_add") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.wrapping_add(b)))
            }
            (RuntimeValue::Int(a), "wrapping_sub") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.wrapping_sub(b)))
            }
            (RuntimeValue::Int(a), "wrapping_mul") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.wrapping_mul(b)))
            }
            (RuntimeValue::Int(a), "clamp") => {
                let lo = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(i64::MIN);
                let hi = args
                    .get(1)
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(i64::MAX);
                Ok(RuntimeValue::Int(a.clamp(lo, hi)))
            }
            (RuntimeValue::Int(a), "min") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(a);
                Ok(RuntimeValue::Int(a.min(b)))
            }
            (RuntimeValue::Int(a), "max") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(a);
                Ok(RuntimeValue::Int(a.max(b)))
            }
            (RuntimeValue::Int(a), "abs" | "unsigned_abs") => Ok(RuntimeValue::Int(a.abs())),
            (RuntimeValue::Int(a), "count_ones") => {
                Ok(RuntimeValue::Int(i64::from((a as u64).count_ones())))
            }
            (RuntimeValue::Int(a), "count_zeros") => {
                Ok(RuntimeValue::Int(i64::from((a as u64).count_zeros())))
            }
            (RuntimeValue::Int(a), "leading_zeros") => {
                Ok(RuntimeValue::Int(i64::from((a as u64).leading_zeros())))
            }
            (RuntimeValue::Int(a), "trailing_zeros") => {
                Ok(RuntimeValue::Int(i64::from((a as u64).trailing_zeros())))
            }
            (RuntimeValue::Int(a), "pow") => {
                let exp = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some((*x).max(0) as u32),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::Int(a.wrapping_pow(exp)))
            }
            (RuntimeValue::Int(a), "is_multiple_of") => {
                let b = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .unwrap_or(1);
                Ok(RuntimeValue::Bool(b != 0 && a % b == 0))
            }
            (RuntimeValue::Int(a), "to_string") => Ok(RuntimeValue::Str(a.to_string())),
            (RuntimeValue::Bool(b), "to_string") => Ok(RuntimeValue::Str(b.to_string())),
            // List / Slice / Iterator methods
            (RuntimeValue::List(items), "len" | "count") => {
                Ok(RuntimeValue::Int(items.len() as i64))
            }
            (RuntimeValue::List(items), "is_empty") => Ok(RuntimeValue::Bool(items.is_empty())),
            (
                RuntimeValue::List(items),
                "iter" | "into_iter" | "copied" | "cloned" | "collect" | "to_vec" | "as_slice",
            ) => Ok(RuntimeValue::List(items)),
            (RuntimeValue::List(mut items), "rev") => {
                items.reverse();
                Ok(RuntimeValue::List(items))
            }
            (RuntimeValue::List(items), "enumerate") => Ok(RuntimeValue::List(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| RuntimeValue::Tuple(vec![RuntimeValue::Int(i as i64), v]))
                    .collect(),
            )),
            (RuntimeValue::List(items), "sum") => {
                let total: i64 = items
                    .iter()
                    .filter_map(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .sum();
                Ok(RuntimeValue::Int(total))
            }
            (RuntimeValue::List(items), "product") => {
                let prod: i64 = items
                    .iter()
                    .filter_map(|v| match v {
                        RuntimeValue::Int(x) => Some(*x),
                        _ => None,
                    })
                    .product();
                Ok(RuntimeValue::Int(prod))
            }
            (RuntimeValue::List(items), "first") => Ok(RuntimeValue::OptionVal(
                items.first().cloned().map(Box::new),
            )),
            (RuntimeValue::List(items), "last") => {
                Ok(RuntimeValue::OptionVal(items.last().cloned().map(Box::new)))
            }
            (RuntimeValue::List(items), "get") => {
                let idx = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some((*x).max(0) as usize),
                        _ => None,
                    })
                    .unwrap_or(usize::MAX);
                Ok(RuntimeValue::OptionVal(
                    items.get(idx).cloned().map(Box::new),
                ))
            }
            (RuntimeValue::List(items), "contains") => {
                let needle = args.first();
                let found = needle.is_some_and(|n| items.iter().any(|x| x == n));
                Ok(RuntimeValue::Bool(found))
            }
            (RuntimeValue::List(items), "zip") => {
                if let Some(RuntimeValue::List(other)) = args.first() {
                    Ok(RuntimeValue::List(
                        items
                            .into_iter()
                            .zip(other.iter().cloned())
                            .map(|(a, b)| RuntimeValue::Tuple(vec![a, b]))
                            .collect(),
                    ))
                } else {
                    Ok(RuntimeValue::List(Vec::new()))
                }
            }
            (RuntimeValue::List(items), "take") => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some((*x).max(0) as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::List(items.into_iter().take(n).collect()))
            }
            (RuntimeValue::List(items), "skip") => {
                let n = args
                    .first()
                    .and_then(|v| match v {
                        RuntimeValue::Int(x) => Some((*x).max(0) as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(RuntimeValue::List(items.into_iter().skip(n).collect()))
            }
            // String methods
            (RuntimeValue::Str(s), "len") => Ok(RuntimeValue::Int(s.len() as i64)),
            (RuntimeValue::Str(s), "is_empty") => Ok(RuntimeValue::Bool(s.is_empty())),
            (RuntimeValue::Str(s), "trim") => Ok(RuntimeValue::Str(s.trim().to_string())),
            (RuntimeValue::Str(s), "trim_start") => {
                Ok(RuntimeValue::Str(s.trim_start().to_string()))
            }
            (RuntimeValue::Str(s), "trim_end") => Ok(RuntimeValue::Str(s.trim_end().to_string())),
            (RuntimeValue::Str(s), "to_uppercase" | "to_ascii_uppercase") => {
                Ok(RuntimeValue::Str(s.to_uppercase()))
            }
            (RuntimeValue::Str(s), "to_lowercase" | "to_ascii_lowercase") => {
                Ok(RuntimeValue::Str(s.to_lowercase()))
            }
            (RuntimeValue::Str(s), "to_string" | "as_str") => Ok(RuntimeValue::Str(s)),
            (RuntimeValue::Str(s), "contains") => {
                let pat = args.first().map(|v| v.display_value()).unwrap_or_default();
                Ok(RuntimeValue::Bool(s.contains(&pat)))
            }
            (RuntimeValue::Str(s), "starts_with") => {
                let pat = args.first().map(|v| v.display_value()).unwrap_or_default();
                Ok(RuntimeValue::Bool(s.starts_with(&pat)))
            }
            (RuntimeValue::Str(s), "ends_with") => {
                let pat = args.first().map(|v| v.display_value()).unwrap_or_default();
                Ok(RuntimeValue::Bool(s.ends_with(&pat)))
            }
            (RuntimeValue::Str(s), "replace") => {
                let from = args.first().map(|v| v.display_value()).unwrap_or_default();
                let to = args.get(1).map(|v| v.display_value()).unwrap_or_default();
                Ok(RuntimeValue::Str(s.replace(&from, &to)))
            }
            (RuntimeValue::Str(s), "chars") => Ok(RuntimeValue::List(
                s.chars()
                    .map(|c| RuntimeValue::Str(c.to_string()))
                    .collect(),
            )),
            (RuntimeValue::Str(s), "bytes") => Ok(RuntimeValue::List(
                s.bytes().map(|b| RuntimeValue::Int(i64::from(b))).collect(),
            )),
            // Option methods
            (RuntimeValue::OptionVal(opt), "is_some") => Ok(RuntimeValue::Bool(opt.is_some())),
            (RuntimeValue::OptionVal(opt), "is_none") => Ok(RuntimeValue::Bool(opt.is_none())),
            (RuntimeValue::OptionVal(Some(v)), "unwrap" | "unwrap_or") => Ok(*v),
            (RuntimeValue::OptionVal(None), "unwrap_or") => {
                Ok(args.first().cloned().unwrap_or(RuntimeValue::Unit))
            }
            (RuntimeValue::OptionVal(None), "unwrap") => Err((
                line,
                format!("called `Option::unwrap()` on a `None` value at line {line}"),
            )),
            // Result methods
            (RuntimeValue::ResultVal(res), "is_ok") => Ok(RuntimeValue::Bool(res.is_ok())),
            (RuntimeValue::ResultVal(res), "is_err") => Ok(RuntimeValue::Bool(res.is_err())),
            (RuntimeValue::ResultVal(Ok(v)), "unwrap" | "unwrap_or") => Ok(*v),
            (RuntimeValue::ResultVal(Err(_)), "unwrap_or") => {
                Ok(args.first().cloned().unwrap_or(RuntimeValue::Unit))
            }
            (RuntimeValue::ResultVal(Err(e)), "unwrap") => Err((
                line,
                format!(
                    "called `Result::unwrap()` on an `Err({})` value at line {line}",
                    e.display_value()
                ),
            )),
            (RuntimeValue::ResultVal(Err(e)), "unwrap_err") => Ok(*e),
            (RuntimeValue::ResultVal(Ok(v)), "unwrap_err") => Err((
                line,
                format!(
                    "called `Result::unwrap_err()` on an `Ok({})` value at line {line}",
                    v.display_value()
                ),
            )),
            (RuntimeValue::ResultVal(Ok(v)), "ok") => Ok(RuntimeValue::OptionVal(Some(v))),
            (RuntimeValue::ResultVal(Err(_)), "ok") => Ok(RuntimeValue::OptionVal(None)),
            (other, "clone") => Ok(other),
            _ => Err((
                line,
                format!("unsupported method `.{method}(...)` at line {line}"),
            )),
        }
    }
}

fn bind_for_pattern(pattern: &str, item: RuntimeValue, env: &mut BTreeMap<String, RuntimeValue>) {
    let trimmed = pattern.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let names: Vec<&str> = inner
            .split(',')
            .map(|s| {
                s.trim()
                    .trim_start_matches('&')
                    .trim_start_matches("mut ")
                    .trim()
            })
            .collect();
        if let RuntimeValue::Tuple(vals) = item {
            for (n, v) in names.into_iter().zip(vals) {
                if !n.is_empty() {
                    env.insert(n.to_string(), v);
                }
            }
        }
    } else {
        let clean = trimmed
            .trim_start_matches('&')
            .trim_start_matches("mut ")
            .trim();
        if !clean.is_empty() {
            env.insert(clean.to_string(), item);
        }
    }
}

/// Execute `cargo test`, `cargo check`, `cargo clippy`, or `cargo run` over the given workspace files (`R4-32`).
pub fn execute_workspace_cargo(
    subcommand: &str,
    filter: &str,
    rust_files: &[(String, String)],
) -> CargoExecutionReport {
    let sub = match subcommand.trim() {
        "check" | "c" => "check",
        "clippy" => "clippy",
        "run" | "r" => "run",
        _ => "test",
    };

    let mut all_structs = Vec::new();
    let mut all_enums = Vec::new();
    let mut all_consts = Vec::new();
    let mut all_type_aliases = BTreeMap::new();
    let mut all_functions = Vec::new();
    let mut diagnostics = Vec::new();
    let mut admission_outcomes = Vec::new();

    for (path, content) in rust_files {
        let source_bytes = content.len() as u64;
        let line_count = content.lines().count().max(1) as u64;
        let mut unbalanced_line = 0u64;
        let mut syntax_line = 0u64;
        let mut type_line = 0u64;

        match parse_rust_module_items(path, content) {
            Ok(parsed) => {
                all_structs.extend(parsed.structs);
                all_enums.extend(parsed.enums);
                all_consts.extend(parsed.consts);
                all_type_aliases.extend(parsed.type_aliases);
                all_functions.extend(parsed.functions);
            }
            Err(diag) => {
                match diag.code.as_str() {
                    "UnbalancedDelimiterAtLine" => unbalanced_line = diag.line as u64,
                    "TypeErrorAtLine" => type_line = diag.line as u64,
                    _ => syntax_line = diag.line as u64,
                }
                diagnostics.push(diag);
            }
        }

        let outcome = admit_ast_module(
            source_bytes.max(1),
            line_count,
            all_functions.len() as u64,
            all_functions.iter().filter(|f| f.is_test).count() as u64,
            unbalanced_line,
            syntax_line,
            1,
            type_line,
            0,
        );
        admission_outcomes.push(format!("{outcome:?}"));
    }

    if diagnostics.is_empty() {
        if let Err(type_diag) = typecheck_module_items_with_aliases(
            &all_structs,
            &all_enums,
            &all_consts,
            &all_type_aliases,
            &all_functions,
        ) {
            let outcome = admit_ast_module(
                rust_files
                    .iter()
                    .map(|(_, c)| c.len() as u64)
                    .sum::<u64>()
                    .max(1),
                rust_files
                    .iter()
                    .map(|(_, c)| c.lines().count().max(1) as u64)
                    .sum::<u64>()
                    .max(1),
                all_functions.len() as u64,
                all_functions.iter().filter(|f| f.is_test).count() as u64,
                0,
                0,
                0,
                type_diag.line as u64,
                0,
            );
            admission_outcomes.push(format!("{outcome:?}"));
            diagnostics.push(type_diag);
        }
    }

    let mut const_env = BTreeMap::new();
    if diagnostics.is_empty() && !all_consts.is_empty() {
        let empty_consts = BTreeMap::new();
        for c in &all_consts {
            let mut c_ctx = EvalContext {
                functions: &all_functions,
                consts: &empty_consts,
                fuel_remaining: 1_024,
                fuel_consumed: 0,
                assertions_verified: 0,
                stdout: String::new(),
                early_return: None,
            };
            let mut tmp_env = const_env.clone();
            if let Ok(val) = c_ctx.eval_expr(&c.file_path, c.decl_line, &c.expr, &mut tmp_env) {
                const_env.insert(c.name.clone(), val);
            }
        }
    }

    let fn_count = all_functions.len();
    let test_functions: Vec<AstFunction> = all_functions
        .iter()
        .filter(|f| {
            f.is_test
                && (filter.trim().is_empty()
                    || f.name.contains(filter.trim())
                    || f.file_path.contains(filter.trim()))
        })
        .cloned()
        .collect();
    let test_count = test_functions.len();

    let mut test_results = Vec::new();
    let mut tests_passed = 0usize;
    let mut tests_failed = 0usize;
    let mut total_fuel_consumed = 0u64;
    let mut stdout = String::new();

    if diagnostics.is_empty() && sub == "test" {
        for test_fn in &test_functions {
            let mut ctx = EvalContext {
                functions: &all_functions,
                consts: &const_env,
                fuel_remaining: DEFAULT_EVAL_FUEL,
                fuel_consumed: 0,
                assertions_verified: 0,
                stdout: String::new(),
                early_return: None,
            };
            let mod_prefix = test_fn
                .file_path
                .trim_start_matches("src/")
                .trim_end_matches(".rs")
                .replace('/', "::");
            let qualified = format!("{mod_prefix}::{}", test_fn.name);

            match ctx.eval_function(test_fn, BTreeMap::new()) {
                Ok(_) => {
                    tests_passed += 1;
                    total_fuel_consumed += ctx.fuel_consumed;
                    if !ctx.stdout.is_empty() {
                        stdout.push_str(&ctx.stdout);
                    }
                    test_results.push(TestCaseResult {
                        test_name: qualified,
                        file_path: test_fn.file_path.clone(),
                        decl_line: test_fn.decl_line,
                        passed: true,
                        fuel_consumed: ctx.fuel_consumed,
                        assertions_verified: ctx.assertions_verified,
                        failure_line: None,
                        failure_message: None,
                    });
                }
                Err((fail_line, fail_msg)) => {
                    tests_failed += 1;
                    total_fuel_consumed += ctx.fuel_consumed;
                    diagnostics.push(AstDiagnostic {
                        file_path: test_fn.file_path.clone(),
                        line: fail_line,
                        severity: "error".to_string(),
                        code: "AssertionFailedAtLine".to_string(),
                        message: fail_msg.clone(),
                    });
                    test_results.push(TestCaseResult {
                        test_name: qualified,
                        file_path: test_fn.file_path.clone(),
                        decl_line: test_fn.decl_line,
                        passed: false,
                        fuel_consumed: ctx.fuel_consumed,
                        assertions_verified: ctx.assertions_verified,
                        failure_line: Some(fail_line),
                        failure_message: Some(fail_msg),
                    });
                }
            }
        }
    } else if diagnostics.is_empty() && sub == "run" {
        if let Some(main_fn) = all_functions.iter().find(|f| f.name == "main") {
            let mut ctx = EvalContext {
                functions: &all_functions,
                consts: &const_env,
                fuel_remaining: DEFAULT_EVAL_FUEL,
                fuel_consumed: 0,
                assertions_verified: 0,
                stdout: String::new(),
                early_return: None,
            };
            match ctx.eval_function(main_fn, BTreeMap::new()) {
                Ok((ret_val, _)) => {
                    total_fuel_consumed += ctx.fuel_consumed;
                    stdout.push_str(&ctx.stdout);
                    if stdout.is_empty() && ret_val != RuntimeValue::Unit {
                        stdout.push_str(&format!("{}\n", ret_val.display_value()));
                    }
                }
                Err((fail_line, fail_msg)) => {
                    total_fuel_consumed += ctx.fuel_consumed;
                    diagnostics.push(AstDiagnostic {
                        file_path: main_fn.file_path.clone(),
                        line: fail_line,
                        severity: "error".to_string(),
                        code: "RuntimeEvaluationError".to_string(),
                        message: fail_msg,
                    });
                }
            }
        } else {
            // Execute a default smoke probe over GeometricRouter::step_token and cyclic_frame_block
            let mut ctx = EvalContext {
                functions: &all_functions,
                consts: &const_env,
                fuel_remaining: DEFAULT_EVAL_FUEL,
                fuel_consumed: 0,
                assertions_verified: 0,
                stdout: String::new(),
                early_return: None,
            };
            let mut env = BTreeMap::new();
            if let Ok(v) =
                ctx.eval_expr("src/attention.rs", 1, "cyclic_frame_block(2, 8)", &mut env)
            {
                total_fuel_consumed += ctx.fuel_consumed;
                stdout.push_str(&format!(
                    "[UOR-R4 Wasm AST Runner] cyclic_frame_block(2, 8) => {}\n",
                    v.display_value()
                ));
            }
        }
    }

    let overall_ok = diagnostics.is_empty() && tests_failed == 0;
    let status = if overall_ok { "ok" } else { "error" };
    let primary_admission = admission_outcomes
        .last()
        .cloned()
        .unwrap_or_else(|| "AdmittedModule(0, 0, 0)".to_string());

    let mut term = String::new();
    term.push_str(&format!(
        "   Compiling lexlean-r4-workspace v0.4.0 (in-Wasm AST kernel R4-32, {} .rs files, {} fns)\n",
        rust_files.len(),
        fn_count
    ));

    if !diagnostics.is_empty() && (sub != "test" || test_results.is_empty()) {
        for d in &diagnostics {
            term.push_str(&format!(
                "error[{}]: {}\n  --> {}:{}:1\n",
                d.code, d.message, d.file_path, d.line
            ));
        }
        term.push_str(&format!(
            "error: could not compile `lexlean-r4-workspace` due to {} previous error(s)\n",
            diagnostics.len()
        ));
    } else if sub == "check" || sub == "clippy" {
        term.push_str(&format!(
            "    Finished `dev` profile [R4.WasmAstEval static typecheck + delimiter gate] ({} functions verified, 0 errors)\n",
            fn_count
        ));
    } else if sub == "run" {
        term.push_str("    Finished `dev` profile [R4.WasmAstEval interpreter]\n     Running `in-wasm-ast-runner`\n");
        if !stdout.is_empty() {
            term.push_str(&stdout);
        }
    } else {
        term.push_str(&format!(
            "    Finished `test` profile [R4.WasmAstEval live interpreter]\n     Running unittests ({} tests across {} workspace modules)\n\nrunning {} tests\n",
            test_count,
            rust_files.len(),
            test_count
        ));
        for tr in &test_results {
            if tr.passed {
                term.push_str(&format!(
                    "test {} ... ok ({} fuel steps, {} assertions)\n",
                    tr.test_name, tr.fuel_consumed, tr.assertions_verified
                ));
            } else {
                term.push_str(&format!(
                    "test {} ... FAILED (line {})\n",
                    tr.test_name,
                    tr.failure_line.unwrap_or(tr.decl_line)
                ));
            }
        }
        if tests_failed > 0 {
            term.push_str("\nfailures:\n");
            for tr in test_results.iter().filter(|t| !t.passed) {
                term.push_str(&format!(
                    "---- {} ({}:{}) ----\n{}\n",
                    tr.test_name,
                    tr.file_path,
                    tr.failure_line.unwrap_or(tr.decl_line),
                    tr.failure_message.as_deref().unwrap_or("assertion failed")
                ));
            }
            term.push_str(&format!(
                "\ntest result: FAILED. {} passed; {} failed; 0 ignored; fuel_consumed={}\n",
                tests_passed, tests_failed, total_fuel_consumed
            ));
        } else {
            term.push_str(&format!(
                "\ntest result: ok. {} passed; 0 failed; 0 ignored; fuel_consumed={}\n",
                tests_passed, total_fuel_consumed
            ));
        }
    }

    let receipt_sha256 = sha256_hex(
        format!(
            "wasm_ast_eval|{sub}|{status}|files={}|fns={fn_count}|passed={tests_passed}|failed={tests_failed}|fuel={total_fuel_consumed}",
            rust_files.len()
        )
        .as_bytes(),
    );

    CargoExecutionReport {
        subcommand: sub.to_string(),
        status: status.to_string(),
        module_admission_outcome: primary_admission,
        files_parsed: rust_files.len(),
        fn_count,
        test_count,
        tests_passed,
        tests_failed,
        total_fuel_consumed,
        test_results,
        diagnostics,
        stdout,
        terminal_output: term,
        receipt_sha256,
    }
}

impl CargoExecutionReport {
    /// Serialize the execution report as a structured JSON object.
    pub fn to_json(&self) -> String {
        let mut tests_json = String::from("[");
        for (i, tr) in self.test_results.iter().enumerate() {
            if i > 0 {
                tests_json.push(',');
            }
            let fail_line_str = tr
                .failure_line
                .map(|l| l.to_string())
                .unwrap_or_else(|| "null".to_string());
            let fail_msg_str = tr
                .failure_message
                .as_ref()
                .map(|m| format!("\"{}\"", escape_json(m)))
                .unwrap_or_else(|| "null".to_string());
            tests_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"test_name\":\"{}\",",
                    "\"file_path\":\"{}\",",
                    "\"decl_line\":{},",
                    "\"passed\":{},",
                    "\"fuel_consumed\":{},",
                    "\"assertions_verified\":{},",
                    "\"failure_line\":{},",
                    "\"failure_message\":{}",
                    "}}"
                ),
                escape_json(&tr.test_name),
                escape_json(&tr.file_path),
                tr.decl_line,
                tr.passed,
                tr.fuel_consumed,
                tr.assertions_verified,
                fail_line_str,
                fail_msg_str
            ));
        }
        tests_json.push(']');

        let mut diags_json = String::from("[");
        for (i, d) in self.diagnostics.iter().enumerate() {
            if i > 0 {
                diags_json.push(',');
            }
            diags_json.push_str(&format!(
                concat!(
                    "{{",
                    "\"file_path\":\"{}\",",
                    "\"line\":{},",
                    "\"severity\":\"{}\",",
                    "\"code\":\"{}\",",
                    "\"message\":\"{}\"",
                    "}}"
                ),
                escape_json(&d.file_path),
                d.line,
                escape_json(&d.severity),
                escape_json(&d.code),
                escape_json(&d.message)
            ));
        }
        diags_json.push(']');

        format!(
            concat!(
                "{{",
                "\"status\":\"{}\",",
                "\"conformance_id\":\"R4-32\",",
                "\"subcommand\":\"{}\",",
                "\"module_admission_outcome\":\"{}\",",
                "\"files_parsed\":{},",
                "\"fn_count\":{},",
                "\"test_count\":{},",
                "\"tests_passed\":{},",
                "\"tests_failed\":{},",
                "\"total_fuel_consumed\":{},",
                "\"test_results\":{},",
                "\"diagnostics\":{},",
                "\"stdout\":\"{}\",",
                "\"terminal_output\":\"{}\",",
                "\"receipt_sha256\":\"{}\"",
                "}}"
            ),
            escape_json(&self.status),
            escape_json(&self.subcommand),
            escape_json(&self.module_admission_outcome),
            self.files_parsed,
            self.fn_count,
            self.test_count,
            self.tests_passed,
            self.tests_failed,
            self.total_fuel_consumed,
            tests_json,
            diags_json,
            escape_json(&self.stdout),
            escape_json(&self.terminal_output),
            escape_json(&self.receipt_sha256)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_ast_eval_cargo_run_while_loop_and_division_by_zero_guard() {
        let files = vec![(
            "scratchpad.rs".to_string(),
            concat!(
                "pub fn sum_to_n(n: i64) -> i64 {\n",
                "    let mut acc: i64 = 0;\n",
                "    let mut i: i64 = 1;\n",
                "    while i <= n {\n",
                "        acc += i;\n",
                "        i += 1;\n",
                "    }\n",
                "    acc\n",
                "}\n\n",
                "fn main() {\n",
                "    let total = sum_to_n(10);\n",
                "    println!(\"sum_to_10={}\", total);\n",
                "}\n\n",
                "#[test]\n",
                "fn test_sum_to_n() {\n",
                "    assert_eq!(sum_to_n(10), 55);\n",
                "    assert_ne!(sum_to_n(5), 0);\n",
                "}\n"
            )
            .to_string(),
        )];

        let run_rep = execute_workspace_cargo("run", "", &files);
        assert_eq!(run_rep.status, "ok");
        assert!(run_rep.stdout.contains("sum_to_10=55"));
        assert!(run_rep.total_fuel_consumed > 10);

        let test_rep = execute_workspace_cargo("test", "", &files);
        assert_eq!(test_rep.status, "ok");
        assert_eq!(test_rep.tests_passed, 1);
        assert_eq!(test_rep.test_results[0].assertions_verified, 2);

        // Division by zero inside #[test] must fail with exact 1-based line number
        let div_zero_files = vec![(
            "scratchpad.rs".to_string(),
            concat!(
                "#[test]\n",
                "fn test_div_zero() {\n",
                "    let denom: i64 = 0;\n",
                "    let bad: i64 = 100 / denom;\n",
                "    assert_eq!(bad, 0);\n",
                "}\n"
            )
            .to_string(),
        )];
        let div_rep = execute_workspace_cargo("test", "", &div_zero_files);
        assert_eq!(div_rep.status, "error");
        assert_eq!(div_rep.tests_failed, 1);
        assert_eq!(div_rep.test_results[0].failure_line, Some(4));
    }

    #[test]
    fn test_wasm_ast_eval_operator_associativity_match_and_compound_assignments() {
        let files = vec![(
            "scratchpad.rs".to_string(),
            concat!(
                "pub fn classify_and_scale(x: i64, opt: Option<i64>) -> i64 {\n",
                "    let mut acc: i64 = 20 * 2 / 4;\n",
                "    let rem_scaled: i64 = 10 % 7 * 2;\n",
                "    let chained_div_rem: i64 = 20 / 2 % 3;\n",
                "    acc += rem_scaled + chained_div_rem;\n",
                "    let bonus: i64 = match x {\n",
                "        0 => 5,\n",
                "        1 | 2 => {\n",
                "            let inner: i64 = x * 10;\n",
                "            inner + 3\n",
                "        }\n",
                "        3..=5 => 40,\n",
                "        _ => -x,\n",
                "    };\n",
                "    match opt {\n",
                "        Some(v) => {\n",
                "            acc += v;\n",
                "        }\n",
                "        None => {\n",
                "            acc -= 1;\n",
                "        }\n",
                "    }\n",
                "    if x > 0 {\n",
                "        acc <<= 1;\n",
                "    } else { acc += 100; }\n",
                "    acc /= 2;\n",
                "    println!(\"acc += {}\", acc);\n",
                "    acc + bonus\n",
                "}\n\n",
                "#[test]\n",
                "fn test_associativity_and_match() {\n",
                "    assert_eq!(20 * 2 / 4, 10);\n",
                "    assert_eq!(10 % 7 * 2, 6);\n",
                "    assert_eq!(20 / 2 % 3, 1);\n",
                "    assert_eq!(classify_and_scale(2, Some(8)), 48);\n",
                "    assert_eq!(classify_and_scale(4, None), 56);\n",
                "}\n"
            )
            .to_string(),
        )];

        let rep = execute_workspace_cargo("test", "", &files);
        assert_eq!(rep.status, "ok", "diagnostics: {:?}", rep.diagnostics);
        assert_eq!(rep.tests_passed, 1);
        assert_eq!(rep.tests_failed, 0);
        assert_eq!(rep.test_results[0].assertions_verified, 5);
        assert!(rep.stdout.contains("acc += 25"));
    }

    #[test]
    fn test_wasm_ast_eval_enums_result_try_operator_and_consts() {
        let files = vec![(
            "src/protocol.rs".to_string(),
            concat!(
                "pub const MAX_SIGNAL: i64 = 64;\n",
                "pub type SignalWord = i64;\n\n",
                "pub enum GateOp {\n",
                "    Halt,\n",
                "    Scale(i64),\n",
                "    Shift(i64),\n",
                "}\n\n",
                "pub fn decode_gate(op: GateOp, base: SignalWord) -> Result<i64, i64> {\n",
                "    match op {\n",
                "        GateOp::Halt => Err(404),\n",
                "        GateOp::Scale(factor) => Ok(base * factor),\n",
                "        GateOp::Shift(delta) => Ok(base + delta + MAX_SIGNAL),\n",
                "    }\n",
                "}\n\n",
                "pub fn chain_gates(base: i64, k: i64) -> Result<i64, i64> {\n",
                "    let step1: i64 = decode_gate(GateOp::Scale(k), base)?;\n",
                "    if step1 < 0 {\n",
                "        return decode_gate(GateOp::Halt, step1);\n",
                "    }\n",
                "    let step2: i64 = decode_gate(GateOp::Shift(6), step1)?;\n",
                "    Ok(step2)\n",
                "}\n\n",
                "#[test]\n",
                "fn test_enums_and_try_propagation() {\n",
                "    let ok_res: Result<i64, i64> = chain_gates(5, 4);\n",
                "    assert!(ok_res.is_ok());\n",
                "    assert_eq!(ok_res.unwrap(), 90);\n",
                "    let err_res: Result<i64, i64> = chain_gates(5, -2);\n",
                "    assert!(err_res.is_err());\n",
                "    assert_eq!(err_res.unwrap_err(), 404);\n",
                "    assert_eq!(err_res.unwrap_or(77), 77);\n",
                "}\n"
            )
            .to_string(),
        )];

        let rep = execute_workspace_cargo("test", "", &files);
        assert_eq!(rep.status, "ok", "diagnostics: {:?}", rep.diagnostics);
        assert_eq!(rep.tests_passed, 1);
        assert_eq!(rep.tests_failed, 0);
        assert_eq!(rep.test_results[0].assertions_verified, 5);
    }

    #[test]
    fn test_wasm_ast_eval_vec_mutation_array_repeat_iterators_and_strings() {
        let files = vec![(
            "src/pipeline.rs".to_string(),
            concat!(
                "pub fn compute_histogram_and_transform() -> i64 {\n",
                "    let mut hist: [i64; 8] = [0; 8];\n",
                "    hist[2] = 10;\n",
                "    hist[2] += 5;\n",
                "    let mut dyn_vec: Vec<i64> = vec![3, 1, 4, 1, 5];\n",
                "    dyn_vec.push(9);\n",
                "    dyn_vec.sort();\n",
                "    let popped: i64 = dyn_vec.pop().unwrap_or(0);\n",
                "    let filtered_sum: i64 = dyn_vec.iter().filter(|x| x > 1).map(|x| x * 2).sum();\n",
                "    let folded: i64 = (1..=4).fold(0, |acc, v| acc + v);\n",
                "    let enum_sum: i64 = dyn_vec.iter().enumerate().map(|(i, v)| i + v).sum();\n",
                "    let has_five: bool = dyn_vec.iter().any(|x| x == 5);\n",
                "    let all_pos: bool = dyn_vec.iter().all(|x| x > 0);\n",
                "    if has_five && all_pos {\n",
                "        hist[2] + popped + filtered_sum + folded + enum_sum\n",
                "    } else {\n",
                "        0\n",
                "    }\n",
                "}\n\n",
                "pub fn format_banner(tag: &str, code: i64) -> String {\n",
                "    let raw: String = format!(\"  [{}::{}]  \", tag, code);\n",
                "    raw.trim().to_uppercase().replace(\"::\", \"-\")\n",
                "}\n\n",
                "#[test]\n",
                "fn test_collections_closures_and_strings() {\n",
                "    assert_eq!(compute_histogram_and_transform(), 82);\n",
                "    let banner: String = format_banner(\"uor_r4\", 32);\n",
                "    assert_eq!(banner, \"[UOR_R4-32]\");\n",
                "    assert!(banner.starts_with(\"[UOR\"));\n",
                "    assert!(banner.ends_with(\"-32]\"));\n",
                "    assert!(banner.contains(\"R4\"));\n",
                "    assert_eq!(banner.len(), 11);\n",
                "}\n"
            )
            .to_string(),
        )];

        let rep = execute_workspace_cargo("test", "", &files);
        assert_eq!(rep.status, "ok", "diagnostics: {:?}", rep.diagnostics);
        assert_eq!(rep.tests_passed, 1);
        assert_eq!(rep.tests_failed, 0);
        assert_eq!(rep.test_results[0].assertions_verified, 6);
    }

    #[test]
    fn test_wasm_ast_eval_chained_constructors_type_aliases_chars_and_try_nesting() {
        let files = vec![(
            "src/edge_cases.rs".to_string(),
            concat!(
                "pub type OutcomeAlias = Result<i64, i64>;\n\n",
                "pub fn fallible_step(x: i64) -> OutcomeAlias {\n",
                "    if x < 0 {\n",
                "        Err(99)\n",
                "    } else {\n",
                "        Ok(x * 3)\n",
                "    }\n",
                "}\n\n",
                "pub fn nested_try_in_ok(x: i64) -> OutcomeAlias {\n",
                "    Ok(fallible_step(x)? + 1)\n",
                "}\n\n",
                "#[test]\n",
                "fn test_edge_cases() {\n",
                "    assert_eq!(Ok(42).unwrap_or(0), 42);\n",
                "    assert_eq!(Err(9).unwrap_or(11), 11);\n",
                "    assert_eq!(Some(7).unwrap_or(0), 7);\n",
                "    assert_eq!([10, 20, 30][1], 20);\n",
                "    assert_eq!([1, 2, 3].len(), 3);\n",
                "    assert_eq!(format!(\"  hi {} \", 5).trim(), \"hi 5\");\n",
                "    let l_count: i64 = \"hello\".chars().filter(|c| c == 'l').count();\n",
                "    assert_eq!(l_count, 2);\n",
                "    assert_eq!(nested_try_in_ok(4).unwrap(), 13);\n",
                "    assert_eq!(nested_try_in_ok(-1).unwrap_err(), 99);\n",
                "}\n"
            )
            .to_string(),
        )];

        let rep = execute_workspace_cargo("test", "", &files);
        assert_eq!(rep.status, "ok", "diagnostics: {:?}", rep.diagnostics);
        assert_eq!(rep.tests_passed, 1);
        assert_eq!(rep.tests_failed, 0);
        assert_eq!(rep.test_results[0].assertions_verified, 9);
    }
}
