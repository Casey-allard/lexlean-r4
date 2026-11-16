Feature: UORust Route Totality & Prism-DevRel ISO 15939 / PROV-O Verification Gates (R4-53)

  @R4-53 @build
  Scenario: Closed classification routing and ISO 15939 / W3C PROV-O quality measures refuse unknown, ambiguous, or threshold-violating inputs
    Given the formal LexLean module R4.UorustPrismDevrel and generated Rust verifier r4_uorust_prism_devrel
    When repository facts in model/routes.toml, quality criteria in model/measures.toml, and primary source bindings in model/provenance.toml are evaluated
    Then unambiguous fact routes mapped across Foundation facets with valid primary sources and satisfied measurement bounds are admitted
    And unknown facts, ambiguous multi-route requests, invalid measurement directions, missing provenance sources, and violated thresholds are refused
