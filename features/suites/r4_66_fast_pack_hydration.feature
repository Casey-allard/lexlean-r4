Feature: Fast <= 48 MiB Streaming Pack Hydration, 404 Mirror Pruning & Single-Truth UI (R4-66)

  @R4-66 @build
  Scenario: Streaming <= 48 MiB pack hydration, immediate 404 mirror pruning, queue-not-downgrade Turn 1, and single-truth badge
    Given the formal R4.FastPackHydration module in src/R4/FastPackHydration.lex.tex and controls in tests/lexlean/FastPackHydrationControls.lex.tex
    When evaluating streaming weight pack manifests, mirror 404 pruning, Turn 1 hydration queueing, and topbar status badge consistency
    Then packs <= 50,331,648 bytes (48 MiB) admit as StreamingPackAdmitted while oversized packs refuse with RefusedPackExceeds50MiB, repeated 404 mirror retries refuse with RefusedPiece404MirrorRetry, and silent Q31 downgrades refuse with RefusedSilentEmergencyDowngrade
    And topbar model/hydration status badges admit as SingleTruthBadgeAdmitted while contradictory Offline Q31 Emergency badges on Qwen3 sessions refuse with RefusedEmergencyBadgeOnQwenSession
