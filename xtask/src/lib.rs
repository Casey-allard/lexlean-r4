//! Shared library root for `xtask` gate and homeostasis modules.

#![allow(missing_docs)]

#[allow(dead_code, clippy::all, missing_docs)]
#[path = "../../crates/wasm/src/lexlean_generated.rs"]
pub mod lexlean_generated;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/lut_stack_exec.rs"]
pub mod lut_stack_exec;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/otel.rs"]
pub mod otel;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/prism_holo_compiler.rs"]
pub mod prism_holo_compiler;
#[allow(dead_code, clippy::all)]
#[path = "../../crates/wasm/src/wasm_ast_eval.rs"]
pub mod wasm_ast_eval;

pub mod audit;
pub mod bootstrap;
pub mod engine;
pub mod routes_and_measures;

/// A gate failure, reported with the rule it broke.
pub type Fail = Box<dyn std::error::Error>;
