Feature: Non-Blocking 4-Token Foundry Turn Stepper & Worker Chunk Bridge (R4-56)

  @R4-56 @build
  Scenario: R4-56 FoundryTurnSession steps 4-token slices with BUILD_SYSTEM_PREFIX prompt assembly, tool-call stop sequences, Worker chunking, and typed refusals
    Given a StudioEngine runtime with mounted HoloV13Model weights and Foundry workspace files
    When holo_begin_foundry_turn prepares a turn with default max_new_tokens 1536 and format_foundry_system_prompt includes BUILD_SYSTEM_PREFIX and mvm_exec instructions
    Then each holo_step_foundry_turn yields at most 4 decoded tokens with incremental UTF-8 flushing and integer-derived telemetry
    And a </tool_call> stop sequence emits status needs_tool with mvm_exec command bounded to at most 6 tool iterations
    And EmptyPrompt, ModelNotMounted, BusyTurn, Cancelled, and ConsentRequiredForWeightDownload are reported with typed exits without corrupting subsequent turns
