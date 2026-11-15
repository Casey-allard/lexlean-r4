//! Conformance test for R4-43: GLLM v11 multi-turn integer RoPE, 16-ch PRLL attention & agentic git loop.

#[path = "../../wasm/src/lexlean_generated.rs"]
mod lexlean_generated;

use lexlean_generated::r4_compositional_gllm::{
    admit_agentic_git_loop, admit_gllm_container, admit_gllm_v11_rope_prll, AgenticGitToolCheck,
    AgenticGitToolVerdict, ContainerSummary, ContainerVerdict, GllmV11RopePrllCheck,
    GllmV11RopePrllVerdict,
};
use serde_json::Value;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn valid_container(version_code: u64) -> ContainerSummary {
    ContainerSummary {
        magic_valid: true,
        version_code,
        vocab_size: 4096,
        d_model: 256,
        spin_aligned: true,
        n_layers: 8,
        n_heads: 32,
        turn_memory_entries: 0,
        dense_matmul_ops: 0,
        softmax_ops: 0,
    }
}

fn valid_v11_rope_prll() -> GllmV11RopePrllCheck {
    GllmV11RopePrllCheck {
        version_code: 11,
        prll_channels: 16,
        position_idx: 12,
        max_seq_len: 512,
        query_pos: 12,
        key_pos: 10,
        mass_preserved: true,
        integer_rope_table_valid: true,
    }
}

fn valid_git_tool() -> AgenticGitToolCheck {
    AgenticGitToolCheck {
        repo_connected: true,
        tool_tag_valid: true,
        branch_nonempty: true,
        host_allowlisted: true,
        secret_in_prompt: false,
    }
}

#[test]
fn test_gllm_v11_rope_prll_container_and_git_loop_r4_43() {
    let root = repo_root();
    let raw = std::fs::read_to_string(root.join("src/R4/CompositionalGllm.lex.tex"))
        .expect("src/R4/CompositionalGllm.lex.tex must exist");
    let (_, after) = raw
        .split_once("\\semanticdata{")
        .expect("semanticdata tag in CompositionalGllm.lex.tex");
    let (json_block, _) = after
        .split_once("\n\\end{semanticmodule}")
        .expect("end semanticmodule in CompositionalGllm.lex.tex");
    let json_line = json_block
        .trim()
        .strip_suffix('}')
        .unwrap_or(json_block.trim());
    let parsed: Value = serde_json::from_str(json_line).expect("valid JSON semantic module");
    assert!(raw.contains("\\begin{lexlean}{R4.CompositionalGllm}"));
    assert_eq!(parsed["spec"], "lexlean/semantic-module/2");
    assert!(raw.contains("admitGllmV11RopePrll"));
    assert!(raw.contains("admitAgenticGitLoop"));

    // Container admission accepts version_code 9, 10, and 11; rejects others.
    for v in [9, 10, 11] {
        assert_eq!(
            admit_gllm_container(valid_container(v)),
            ContainerVerdict::ContainerAdmitted,
            "container v{v} must be admitted"
        );
    }
    for bad_v in [0, 8, 12] {
        assert_eq!(
            admit_gllm_container(valid_container(bad_v)),
            ContainerVerdict::RefusedBadMagicOrVersion,
            "container v{bad_v} must be refused"
        );
    }
    assert_eq!(
        admit_gllm_container(ContainerSummary {
            turn_memory_entries: 1,
            ..valid_container(11)
        }),
        ContainerVerdict::RefusedStampedTurnMemory
    );
    assert_eq!(
        admit_gllm_container(ContainerSummary {
            spin_aligned: false,
            ..valid_container(11)
        }),
        ContainerVerdict::RefusedInvalidGeometry
    );

    // GLLM v11 integer RoPE + 16-ch PRLL attention admission and negative tests.
    assert_eq!(
        admit_gllm_v11_rope_prll(valid_v11_rope_prll()),
        GllmV11RopePrllVerdict::V11RopePrllAdmitted
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            version_code: 9,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedUnsupportedV11Version
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            prll_channels: 8,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedWrongPrllChannels
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            position_idx: 512,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedRopePositionOverflow
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            integer_rope_table_valid: false,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedRopePositionOverflow
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            query_pos: 4,
            key_pos: 5,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedCausalMaskViolation
    );
    assert_eq!(
        admit_gllm_v11_rope_prll(GllmV11RopePrllCheck {
            mass_preserved: false,
            ..valid_v11_rope_prll()
        }),
        GllmV11RopePrllVerdict::RefusedPrllMassConservation
    );

    // Agentic git tool loop admission and negative tests.
    assert_eq!(
        admit_agentic_git_loop(valid_git_tool()),
        AgenticGitToolVerdict::GitToolCallAdmitted
    );
    assert_eq!(
        admit_agentic_git_loop(AgenticGitToolCheck {
            repo_connected: false,
            ..valid_git_tool()
        }),
        AgenticGitToolVerdict::RefusedRepoNotConnected
    );
    assert_eq!(
        admit_agentic_git_loop(AgenticGitToolCheck {
            tool_tag_valid: false,
            ..valid_git_tool()
        }),
        AgenticGitToolVerdict::RefusedMalformedToolTag
    );
    assert_eq!(
        admit_agentic_git_loop(AgenticGitToolCheck {
            branch_nonempty: false,
            ..valid_git_tool()
        }),
        AgenticGitToolVerdict::RefusedInvalidBranchRef
    );
    assert_eq!(
        admit_agentic_git_loop(AgenticGitToolCheck {
            host_allowlisted: false,
            ..valid_git_tool()
        }),
        AgenticGitToolVerdict::RefusedUntrustedHost
    );
    assert_eq!(
        admit_agentic_git_loop(AgenticGitToolCheck {
            secret_in_prompt: true,
            ..valid_git_tool()
        }),
        AgenticGitToolVerdict::RefusedSecretInPrompt
    );
}
