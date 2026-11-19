//! Consolidated empirical challenge test suite for `xtask` engines and provenance.

#![allow(missing_docs, dead_code, unused_imports, clippy::all)]

use std::sync::{Arc, Mutex, OnceLock};

pub(crate) static XTASK_DAEMON_LOCK: Mutex<()> = Mutex::new(());
pub(crate) static BOOTSTRAP_BYTES: &[u8] =
    include_bytes!("../../crates/wasm/src/bootstrap_v8.uorlut");

pub(crate) fn shared_bootstrap_lut() -> Arc<xtask::lut_stack_exec::LutStackModel> {
    static MODEL: OnceLock<Arc<xtask::lut_stack_exec::LutStackModel>> = OnceLock::new();
    Arc::clone(MODEL.get_or_init(|| {
        Arc::new(
            xtask::lut_stack_exec::LutStackModel::parse(BOOTSTRAP_BYTES)
                .expect("bootstrap parses successfully"),
        )
    }))
}

#[path = "empirical_suite/m2.rs"]
mod m2;
#[path = "empirical_suite/m2_iter2.rs"]
mod m2_iter2;
#[path = "empirical_suite/m3_api.rs"]
mod m3_api;
#[path = "empirical_suite/m4_iter2_chal2.rs"]
mod m4_iter2_chal2;
#[path = "empirical_suite/m4_provenance_concurrency.rs"]
mod m4_provenance_concurrency;

#[test]
fn test_wave0_xtask_empirical_suite_preserves_all_42_tests() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suite_dir = manifest_dir.join("tests/empirical_suite");
    let mut total_tests = 0usize;
    let mut files_checked = 0usize;
    let ignore_marker = concat!("#[ig", "nore");
    for file in [
        "m2.rs",
        "m2_iter2.rs",
        "m3_api.rs",
        "m4_iter2_chal2.rs",
        "m4_provenance_concurrency.rs",
    ] {
        let src = std::fs::read_to_string(suite_dir.join(file))
            .unwrap_or_else(|e| panic!("failed to read {file}: {e}"));
        files_checked += 1;
        total_tests += src.lines().filter(|l| l.trim() == "#[test]").count();
        assert!(
            !src.contains(ignore_marker),
            "{file} must not ignore any empirical test"
        );
        assert!(
            !src.contains("../src/engine.rs"),
            "{file} must not duplicate-include engine.rs via #[path]"
        );
    }
    assert_eq!(files_checked, 5);
    assert!(
        total_tests >= 42,
        "expected >= 42 preserved empirical tests in xtask/tests/empirical_suite, got {total_tests}"
    );
}
