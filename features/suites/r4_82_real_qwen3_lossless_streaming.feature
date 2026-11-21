Feature: R4-82 Real Qwen3-0.6B/1.7B Lossless Streaming, Fast Warm Mount & Zero Canned Prose

  @R4-82 @build
  Scenario: Qwen3-0.6B and Qwen3-1.7B lanes execute strictly on real HoloV13Model neural weights with deferred idle prefix prewarming, instant Token-1 TTFT flush, and monotonic Ready badge state
    Given the LexLean semantic module R4.RealQwen3LosslessStreaming and control suite RealQwen3LosslessStreamingControls
    When admitRealQwen3LosslessTurn evaluates canned passage routing, synchronous mount prewarming, Token-1 TTFT flush, and Ready badge monotonicity
    Then real lossless Qwen3 streaming with non-blocking idle prewarm and immediate first-token flush is admitted while preplanned canned routing, mount-blocking prewarm, delayed first-word buffering, and cold badge overwrites fail closed
