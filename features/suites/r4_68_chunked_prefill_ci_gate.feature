Feature: Chunked Asynchronous Prompt Prefill, Live-HUD Speculative Drafting, and Single-Pass CI Gate (R4-68)

  @R4-68 @build
  Scenario: Chunked <= 24-token async prefill slices, live-HUD speculative drafting, and single-pass deduplicated validation
    Given the formal R4.ChunkedPrefillCiGate module in src/R4/ChunkedPrefillCiGate.lex.tex and controls in tests/lexlean/ChunkedPrefillCiGateControls.lex.tex
    When evaluating Foundry turn prefill slice sizing, live-HUD speculative draft budgets, and single-pass xtask CI gate invariants
    Then <= 24-token prompts admit as SyncFastPrefillAdmitted, > 24-token prompts step in <= 24-token slices as ChunkedAsyncPrefillAdmitted, live-HUD turns admit speculative drafting as LiveHudSpeculativeAdmitted, and deduplicated single-pass validation with precomputed BPE token flags admits as DeduplicatedGateAdmitted
    And oversized prefill slices refuse with RefusedZeroOrOversizedPrefillSlice, zeroed live-HUD draft budgets refuse with RefusedDisabledDrafterUnderLiveHud, and duplicate mechanizer/ratchet runs refuse with RefusedDuplicateMechanizerOrRatchetRun
