//! Conformance test for R4-76: Studio Agent Multi-Hop LoRA Corpus & `UORR4V14` `C1/S5`
//! Coding Trajectory Distillation in lockstep with `STUDIO_AGENT_TOOL_SCHEMA`.

#[path = "../../wasm/src/lexlean_generated.rs"]
#[allow(dead_code)]
mod lexlean_generated;

use lexlean_generated::r4_studio_agent_lora_distill::{
    admit_distilled_agent_atlas, verify_studio_agent_lora_distill_admitted, AgentDistillVerdict,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn check_lex_module(root: &Path, rel: &str, expected_mod: &str, required_decls: &[&str]) {
    let raw =
        std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(raw.contains(&format!("\\begin{{lexlean}}{{{expected_mod}}}")));
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .unwrap_or_else(|| panic!("missing \\semanticdata{{ in {rel}"));
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .unwrap_or_else(|| panic!("missing \\end{{semanticmodule}} in {rel}"));
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    for sym in required_decls {
        assert!(raw.contains(sym), "{rel} must declare {sym}");
    }
}

#[test]
fn test_r4_76_studio_agent_lora_distill_r4_76() {
    let root = repo_root();

    // 1. Formal LexLean R4-76 admission and R5 negative refusals
    assert!(verify_studio_agent_lora_distill_admitted());
    assert_eq!(
        admit_distilled_agent_atlas(0, 64, 16, 262_144, true, true, true),
        AgentDistillVerdict::DistilledAgentAtlasAdmitted
    );
    assert_eq!(
        admit_distilled_agent_atlas(1, 64, 16, 262_144, true, true, true),
        AgentDistillVerdict::RefusedFloatOpsInDistillation
    );
    assert_eq!(
        admit_distilled_agent_atlas(0, 64, 16, 262_144, true, true, false),
        AgentDistillVerdict::RefusedMissingToolSchemaLockstep
    );
    assert_eq!(
        admit_distilled_agent_atlas(0, 64, 16, 262_144, false, true, true),
        AgentDistillVerdict::RefusedBrokenStitchOrChatMlHopParity
    );
    assert_eq!(
        admit_distilled_agent_atlas(0, 64, 4, 262_144, true, true, true),
        AgentDistillVerdict::RefusedInsufficientMultiHopTrajectories
    );
    assert_eq!(
        admit_distilled_agent_atlas(0, 64, 16, 1_048_577, true, true, true),
        AgentDistillVerdict::RefusedC1S5ByteBudgetOverflow
    );

    // 2. Verify .lex.tex semantic modules exist and declare all required symbols
    check_lex_module(
        &root,
        "src/R4/StudioAgentLoraDistill.lex.tex",
        "R4.StudioAgentLoraDistill",
        &[
            "AgentDistillVerdict",
            "admitDistilledAgentAtlas",
            "verifyStudioAgentLoraDistillAdmitted",
            "breaksDistillFloatOpsAdmitted",
            "breaksDistillInsufficientTrajectoriesAdmitted",
            "breaksDistillC1S5OverflowAdmitted",
            "breaksDistillMissingChatMlOrSchemaLockstepAdmitted",
        ],
    );
    check_lex_module(
        &root,
        "tests/lexlean/StudioAgentLoraDistillControls.lex.tex",
        "StudioAgentLoraDistillControls",
        &[
            "studioAgentLoraDistillAdmitted",
            "breaksDistillFloatOpsAdmittedCtrl",
            "breaksDistillInsufficientTrajectoriesAdmittedCtrl",
            "breaksDistillC1S5OverflowAdmittedCtrl",
            "breaksDistillMissingChatMlOrSchemaLockstepAdmittedCtrl",
        ],
    );

    // 3. Verify xtask/src/studio_lora_corpus.rs exists and wires studio-compile-v14-corpus
    let corpus_path = root.join("xtask/src/studio_lora_corpus.rs");
    assert!(
        corpus_path.is_file(),
        "xtask/src/studio_lora_corpus.rs must exist"
    );
    let corpus_src = std::fs::read_to_string(&corpus_path).expect("read studio_lora_corpus.rs");
    assert!(
        corpus_src.contains("build_studio_agent_distill_corpus")
            && corpus_src.contains("run_studio_compile_v14_corpus_cmd")
            && corpus_src.contains("<tool_response>")
            && corpus_src.contains("<prism-write"),
        "studio_lora_corpus.rs must synthesize multi-hop ChatML <tool_call> -> <tool_response> -> <prism-write> episodes"
    );

    let xtask_main =
        std::fs::read_to_string(root.join("xtask/src/main.rs")).expect("read xtask/src/main.rs");
    assert!(
        xtask_main.contains("\"studio-compile-v14-corpus\""),
        "xtask/src/main.rs must expose studio-compile-v14-corpus CLI subcommand"
    );

    // 4. Verify embedded HopfZetaPoincareEngineV14 includes Studio <tool_call> & <prism-write> chords
    let hopf_src = std::fs::read_to_string(root.join("crates/wasm/src/hopf_zeta_engine.rs"))
        .expect("read hopf_zeta_engine.rs");
    assert!(
        hopf_src.contains("<tool_call>") && hopf_src.contains("<prism-write"),
        "default_hopf_zeta_v14_engine must index Studio <tool_call> and <prism-write> coding chords"
    );
}
