#![allow(
    missing_docs,
    clippy::needless_range_loop,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::identity_op,
    clippy::manual_range_contains,
    clippy::useless_vec,
    unused_imports
)]
//! Consolidated Empirical Challenge Suite for M1–M5 Next-Token Prediction Architecture.

use lexlean_r4_wasm::lut_stack_exec::LutStackModel;
use std::sync::{Arc, OnceLock};

pub const BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub fn shared_bootstrap_model_arc() -> Arc<LutStackModel> {
    static MODEL: OnceLock<Arc<LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(LutStackModel::parse(BOOTSTRAP_BYTES).expect("bootstrap parses successfully"))
    }))
}

pub fn shared_bootstrap_model_clone() -> LutStackModel {
    shared_bootstrap_model_arc().as_ref().clone()
}

#[path = "next_token_arch/empirical_challenge_m1_basin.rs"]
mod empirical_challenge_m1_basin;
#[path = "next_token_arch/empirical_challenge_m1_ngram.rs"]
mod empirical_challenge_m1_ngram;
#[path = "next_token_arch/empirical_challenge_m2_adversary.rs"]
mod empirical_challenge_m2_adversary;
#[path = "next_token_arch/empirical_challenge_m2_reattention.rs"]
mod empirical_challenge_m2_reattention;
#[path = "next_token_arch/empirical_challenge_m2_reattention_adv.rs"]
mod empirical_challenge_m2_reattention_adv;
#[path = "next_token_arch/empirical_challenge_m3_barrier_adv.rs"]
mod empirical_challenge_m3_barrier_adv;
#[path = "next_token_arch/empirical_challenge_m3_steerage_adv.rs"]
mod empirical_challenge_m3_steerage_adv;
#[path = "next_token_arch/empirical_challenge_m3_thinking.rs"]
mod empirical_challenge_m3_thinking;
#[path = "next_token_arch/empirical_challenge_m4_basin_fuzz.rs"]
mod empirical_challenge_m4_basin_fuzz;
#[path = "next_token_arch/empirical_challenge_m4_iter2_chal2.rs"]
mod empirical_challenge_m4_iter2_chal2;
#[path = "next_token_arch/empirical_challenge_m4_optimizer_adv.rs"]
mod empirical_challenge_m4_optimizer_adv;
#[path = "next_token_arch/empirical_challenge_m4_readback_adv.rs"]
mod empirical_challenge_m4_readback_adv;
#[path = "next_token_arch/empirical_challenge_m4_residuals.rs"]
mod empirical_challenge_m4_residuals;
#[path = "next_token_arch/empirical_challenge_m5_direct_mount.rs"]
mod empirical_challenge_m5_direct_mount;
#[path = "next_token_arch/empirical_challenge_m5_idempotency_concurrency.rs"]
mod empirical_challenge_m5_idempotency_concurrency;
#[path = "next_token_arch/empirical_challenge_m5_it2_concurrency.rs"]
mod empirical_challenge_m5_it2_concurrency;
#[path = "next_token_arch/empirical_challenge_m5_it2_content_blocks.rs"]
mod empirical_challenge_m5_it2_content_blocks;
