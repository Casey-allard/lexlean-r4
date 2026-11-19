//! Consolidated empirical challenge test suite for `lexlean-r4-wasm`.

#![allow(missing_docs, dead_code, unused_imports, unused_variables, clippy::all)]

use std::sync::{Arc, Mutex, OnceLock};

pub(crate) static STUDIO_DISPATCH_LOCK: Mutex<()> = Mutex::new(());
pub(crate) static BOOTSTRAP_BYTES: &[u8] = include_bytes!("../src/bootstrap_v8.uorlut");

pub(crate) fn shared_bootstrap_lut() -> Arc<lexlean_r4_wasm::lut_stack_exec::LutStackModel> {
    static MODEL: OnceLock<Arc<lexlean_r4_wasm::lut_stack_exec::LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(
            lexlean_r4_wasm::lut_stack_exec::LutStackModel::parse(BOOTSTRAP_BYTES)
                .expect("bootstrap parses successfully"),
        )
    }))
}

#[path = "empirical_suite/empirical_challenge_3d_math.rs"]
mod empirical_challenge_3d_math;
#[path = "empirical_suite/empirical_challenge_chatbox.rs"]
mod empirical_challenge_chatbox;
#[path = "empirical_suite/empirical_challenge_m1.rs"]
mod empirical_challenge_m1;
#[path = "empirical_suite/empirical_challenge_m1_challenger.rs"]
mod empirical_challenge_m1_challenger;
#[path = "empirical_suite/empirical_challenge_m2_challenger1.rs"]
mod empirical_challenge_m2_challenger1;
#[path = "empirical_suite/empirical_challenge_m2_iter2_chal1.rs"]
mod empirical_challenge_m2_iter2_chal1;
#[path = "empirical_suite/empirical_challenge_m2_iter3_chal1.rs"]
mod empirical_challenge_m2_iter3_chal1;
#[path = "empirical_suite/empirical_challenge_m2_iter3_chal2.rs"]
mod empirical_challenge_m2_iter3_chal2;
#[path = "empirical_suite/empirical_challenge_m2_iter4_chal1.rs"]
mod empirical_challenge_m2_iter4_chal1;
#[path = "empirical_suite/empirical_challenge_m2_iter4_chal2.rs"]
mod empirical_challenge_m2_iter4_chal2;
#[path = "empirical_suite/empirical_challenge_m3_chal2.rs"]
mod empirical_challenge_m3_chal2;
#[path = "empirical_suite/empirical_challenge_m3_challenger1.rs"]
mod empirical_challenge_m3_challenger1;
#[path = "empirical_suite/empirical_challenge_m3_closure.rs"]
mod empirical_challenge_m3_closure;
#[path = "empirical_suite/empirical_challenge_m4_budget_stress.rs"]
mod empirical_challenge_m4_budget_stress;
#[path = "empirical_suite/empirical_challenge_m4_iter2_chal1.rs"]
mod empirical_challenge_m4_iter2_chal1;
#[path = "empirical_suite/empirical_challenge_m4_otel.rs"]
mod empirical_challenge_m4_otel;
#[path = "empirical_suite/empirical_challenge_terminal.rs"]
mod empirical_challenge_terminal;

#[test]
fn test_wave0_wasm_empirical_suite_preserves_all_98_tests() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite_dir = manifest_dir.join("tests/empirical_suite");
    let files = [
        "empirical_challenge_3d_math.rs",
        "empirical_challenge_chatbox.rs",
        "empirical_challenge_m1.rs",
        "empirical_challenge_m1_challenger.rs",
        "empirical_challenge_m2_challenger1.rs",
        "empirical_challenge_m2_iter2_chal1.rs",
        "empirical_challenge_m2_iter3_chal1.rs",
        "empirical_challenge_m2_iter3_chal2.rs",
        "empirical_challenge_m2_iter4_chal1.rs",
        "empirical_challenge_m2_iter4_chal2.rs",
        "empirical_challenge_m3_chal2.rs",
        "empirical_challenge_m3_challenger1.rs",
        "empirical_challenge_m3_closure.rs",
        "empirical_challenge_m4_budget_stress.rs",
        "empirical_challenge_m4_iter2_chal1.rs",
        "empirical_challenge_m4_otel.rs",
        "empirical_challenge_terminal.rs",
    ];
    let mut total_tests = 0usize;
    let mut files_checked = 0usize;
    let ignore_marker = concat!("#[ig", "nore");
    for file in files {
        let src = std::fs::read_to_string(suite_dir.join(file))
            .unwrap_or_else(|e| panic!("failed to read {file}: {e}"));
        files_checked += 1;
        total_tests += src.lines().filter(|l| l.trim() == "#[test]").count();
        assert!(
            !src.contains(ignore_marker),
            "{file} must not ignore any empirical test"
        );
        assert!(
            !src.contains("include_bytes!(\"../src/bootstrap_v8.uorlut\")"),
            "{file} must reuse crate::BOOTSTRAP_BYTES"
        );
    }
    assert_eq!(files_checked, 17);
    assert!(
        total_tests >= 98,
        "expected >= 98 preserved empirical tests in crates/wasm/tests/empirical_suite, got {total_tests}"
    );
}
