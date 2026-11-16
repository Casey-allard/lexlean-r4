//! Comprehensive End-to-End (E2E) Opaque-Box Test Suite for LexLean-R4 Long-Horizon Overhaul (R1–R6).
//!
//! Enforces:
//! - Tier 1: Feature Coverage (>=5 test cases per feature for F01 through F19 = 95 tests)
//! - Tier 2: Boundary & Corner Cases (>=5 test cases per feature for F01 through F19 = 95 tests)
//! - Tier 3: Cross-Feature Combinations (16 pairwise/multi-feature interaction tests)
//! - Tier 4: Real-World Application Scenarios (6 comprehensive multi-turn scenarios, including Node.js web harness)
//!
//! Total Tests: 212 tests.

#![allow(
    missing_docs,
    dead_code,
    unused_imports,
    unused_variables,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::erasing_op,
    clippy::manual_range_contains,
    clippy::useless_vec,
    clippy::unnecessary_get_then_check,
    clippy::len_zero,
    clippy::unnecessary_cast,
    clippy::vec_init_then_push,
    clippy::unnecessary_literal_unwrap,
    clippy::manual_slice_size_calculation
)]

#[path = "overhaul_e2e/common.rs"]
pub mod common;

#[path = "overhaul_e2e/tier1_feature.rs"]
pub mod tier1_feature;

#[path = "overhaul_e2e/tier2_boundary.rs"]
pub mod tier2_boundary;

#[path = "overhaul_e2e/tier3_combinations.rs"]
pub mod tier3_combinations;

#[path = "overhaul_e2e/tier4_scenarios.rs"]
pub mod tier4_scenarios;
