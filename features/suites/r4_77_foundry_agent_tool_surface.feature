Feature: R4-77 Live Foundry InTabProver (FK-01..09) & vTensor 3D Agent <tool_call> Tools

  @R4-77 @build
  Scenario: Studio and Chat composer turns invoke policy-checked foundry_prove, foundry_publish_holo, and vtensor_render_3d <tool_call> tools
    Given the formal LexLean module R4.FoundryAgentToolSurface and its proof controls FoundryAgentToolSurfaceControls
    When the agent emits <tool_call> invocations for foundry_prove, foundry_publish_holo, or vtensor_render_3d
    Then foundry_prove verifies /workspace .lex.tex semantic modules in-tab with propext/Quot.sound-only soundness (Drafting -> Checked -> Proved)
    And foundry_publish_holo emits a blake3: kappa-addressed .holo capsule iff workspace is Proved and composer policy is AgentFull
    And vtensor_render_3d renders a zero-float 16x16 tile-binned 3D PBR scene (LOD0..LOD4) for ?surface=preview and returns frame_sha256
    And negative controls refuse PlanReadOnly publication, publish-before-prove (FK-02), unsound kernel axioms, and out-of-bounds LOD > 4
