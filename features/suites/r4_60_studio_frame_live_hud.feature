Feature: Responsive Prism Studio Frame, Agent/Plan Policy, Off-Origin Hydration Consent & Live Token HUD (R4-60)

  @R4-60 @build
  Scenario: R4.StudioFrame classifies responsive window widths, gates Agent/Plan mutating tools, enforces off-origin hydration consent, and validates live token HUD frames
    Given the formal LexLean module R4.StudioFrame and generated Rust verifier r4_studio_frame
    When responsive viewport breakpoints, composer tool policies, off-origin shard hydration consent, and live token HUD frames are evaluated
    Then Compact (<600px), Medium (600..839px), and Expanded (>=840px) window classes are resolved across Chat, Studio, ObservatoryEmbed, and PreviewEmbed surface modes
    And AgentFull admits mutating and read-only tools while PlanReadOnly admits read-only tools and refuses mutating tools
    And same-origin hydration, consented off-origin hydration, and explicit offline Q31 fallback are admitted while unconsented off-origin hydration without fallback is refused with RefusedOffOriginConsentRequired
    And bounded live token frames with valid shortlist state are admitted while synthetic Hamming reporting on Q31 (RefusedSyntheticHammingOnQ31) and preview fuel exhaustion (RefusedPreviewFuelExhausted) are refused
