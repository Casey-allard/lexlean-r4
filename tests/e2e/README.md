# LexLean-R4 Opaque-Box E2E Test Suite

This directory contains the comprehensive, requirement-driven, opaque-box End-to-End (E2E) test suite for the UOR-R4 LexLean implementation, authored under `uor/template-contract/1` and `AGENTS.md`.

## Test Philosophy
- **Opaque-Box**: Tests verify observable public behavior, standard CLI invocations, and cryptographic receipts without inspecting internal struct fields or compiler ASTs.
- **Fail-Closed (Remediation of F31)**: Any failing assertion immediately calls `std::process::exit(1)`. Tests never swallow failures or return exit code 0 on errors.
- **Authoritative Coverage**: Enforces $\ge 299$ total test cases across 26 features in 4 tiers (+ Tier 5 adversarial hardening).

## Directory Structure
- `common/`: Shared test harness, fail-closed assertion macros, canonical receipt validators, and CLI execution wrappers.
- `tier1_functional/`: Primary happy-path contract tests ($\ge 5$ tests per feature, $\ge 130$ tests).
- `tier2_boundary/`: Boundary, extreme, and typed negative tests per Rule R5 and Codex audit counterexamples ($\ge 5$ tests per feature, $\ge 130$ tests).
- `tier3_interaction/`: Pairwise cross-feature interaction tests ($\ge 26$ tests).
- `tier4_scenarios/`: Multi-step real-world workload application scenarios ($\ge 13$ scenarios).
- `tier5_adversarial/`: Adversarial hardening, non-canonical encodings, Byzantine attacks, and resource stress.

## Execution
```bash
# Run all E2E tests
cargo test --test e2e -- --nocapture

# Run specific tier
cargo test --test e2e tier1_functional::
cargo test --test e2e tier2_boundary::
cargo test --test e2e tier3_interaction::
cargo test --test e2e tier4_scenarios::
```

Refer to `/TEST_INFRA.md` for the complete authoritative specification.
