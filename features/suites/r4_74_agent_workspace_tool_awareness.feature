Feature: R4-74 Unified Agent /workspace VFS, MVM Terminal Tool Self-Awareness, and Zero-Fetch Warm Mount

  @R4-74 @build
  Scenario: Unified composer turns bind prewarmed MVM terminal and /workspace tool schemas without excerpt prefill bloat or redundant HTTP fetches
    Given the formal LexLean module R4.AgentWorkspaceToolAwareness and its proof controls AgentWorkspaceToolAwarenessControls
    When the Studio composer dispatches turns across Chat and Studio layout modes with mounted or OPFS warm-sealed Holo models
    Then both mvm_exec and /workspace tool schemas and the live /workspace file tree are bound within <= 160 uncached tail tokens
    And multi-hop tool continuations bind the assistant <tool_call> ChatML turn before <tool_response>
    And already-mounted or OPFS warm-sealed catalog models perform zero HTTP manifest/pack downloads and preserve KV prefix reuse
    And negative controls refuse tool-blind prompts, 920-token uncached excerpt bloat, redundant HTTP fetches, and surface-flip KV cache wipes
