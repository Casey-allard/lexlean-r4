Feature: R4-81 Zero-Download Instant Boot, 10k+ tok/s UORR4V15 Turn Routing & Coalesced HoloV13 Hydration

  @R4-81 @build
  Scenario: Default Studio Worker boots in <= 150 ms with zero weight downloads, serves unmounted 0.6B turns at >= 10000 tok/s via UORR4V15, and coalesces explicit HoloV13 warm streaming into <= 16 chunks
    Given the LexLean semantic module R4.ZeroCopyFastHydration and control suite ZeroCopyFastHydrationControls
    When admitFastBootAndZeroRedundancySlice evaluates boot network bytes, boot readiness latency, warm OPFS chunk calls, duplicate GPU/CPU slice execution, and timer clamp milliseconds
    Then zero-download <= 150 ms boot with <= 16 coalesced 32 MiB OPFS chunks, non-redundant GPU/CPU slices, and zero-clamp MessageChannel yields is admitted while all five R5 violation classes fail closed
