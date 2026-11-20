Feature: Live Bound WebGPU Turn Bridge, COEP Credentialless Isolation & Adaptive Speculative Drafter (R4-65)

  @R4-65 @build
  Scenario: Bound GPUBindGroup live WebGPU turn shortlist stepping, COEP credentialless Service Worker isolation, and adaptive CPU miss throttling
    Given the formal R4.LiveWebGpuTurnBridge module in src/R4/LiveWebGpuTurnBridge.lex.tex and controls in tests/lexlean/LiveWebGpuTurnBridgeControls.lex.tex
    When evaluating bound WebGPU compute dispatches, COEP credentialless Service Worker isolation, and rolling-EMA speculative draft budgets
    Then zero-float bound WebGPU passes with 0-LSB parity admit as LiveGpuBoundStepAdmitted while unbound dispatches refuse with RefusedUnboundComputePass and logit drift refuses with RefusedGpuCpuLogitDrift
    And single-reload COEP credentialless Service Worker isolation admits as CrossOriginCredentiallessAdmitted while require-corp policies refuse with RefusedRequireCorpBreaksMirrors
    And full K<=4 speculative tiles admit as AdaptiveTileAdmitted on GPU or high-hit turns while low-EMA CPU turns throttle to <=1 draft (ThrottledCpuDraftsAdmitted) and unthrottled CPU miss storms refuse with RefusedUnthrottledCpuMissStorm
