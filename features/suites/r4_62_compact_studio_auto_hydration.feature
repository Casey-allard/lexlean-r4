Feature: Compact Cursor-Grade Studio UI, Non-Blocking Dropdown Neural HUD & Zero-Prompt Auto-Hydration (R4-62)

  @R4-62 @build
  Scenario: R4.StudioFrame admits first-party Qwen3 auto-hydration and non-blocking HUD overlays while refusing silent model downgrades, unconsented untrusted origins, and live HUD blocking backdrops
    Given the formal LexLean module R4.StudioFrame and generated Rust verifier r4_studio_frame
    When catalog hydration policies, emergency offline Q31 overrides, and live Neural HUD backdrop states are evaluated
    Then SameOrigin and FirstPartyCatalog models admit automatic hydration at concurrency ceiling 3 without blocking consent prompts
    And ExternalUntrustedOrigin admits hydration only with explicit consent and refuses unconsented downloads with RefusedExternalOriginConsentRequired
    And sticky background declined flags never override an explicit Qwen3 selection unless emergencyOfflineToggle is active, refusing silent downgrades with RefusedSilentModelDowngrade
    And DropdownCompactStrip, DropdownExpandedDeck, and ChromelessEmbed admit non-blocking overlays with HudNonBlockingAdmitted while blocking backdrops are refused with RefusedBlockingBackdropOnLiveHud
