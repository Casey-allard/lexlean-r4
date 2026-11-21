Feature: R4-70 Zero-Float vTensor Certified Storage Planner and Semiring 3D PBR WebGUI Engine
  As a UOR-R4 Holo Studio operator and app builder
  I want certified non-float vTensor buffer placement, exact semiring contractions, and 16x16 tile-binned 3D PBR rendering across the Observatory/HUD and Studio preview surface
  So that 3D Hopf-Poincare-Zeta neural visualizations and Studio apps render deterministically with zero float, dense matmul, or softmax operations

  @R4-70 @build
  Scenario: Certified vTensor storage graph and 16x16 tile-binned 3D PBR WebGUI pass formal and runtime verification
    Given the LexLean module "src/R4/VtensorSemiring3d.lex.tex" and negative controls "tests/lexlean/VtensorSemiring3dControls.lex.tex"
    When the vTensor linear arena, 4-axis REPEAT indexer, Laurent/tropical/Q15 semiring contractions, and 16x16 tile-binned UORC LOD0..LOD4 PBR rasterizer execute
    Then float dtypes, unaligned placements, transfer overflows, and out-of-range LOD levels are rejected and both Observatory/HUD and Studio preview RGBA8 3D surfaces render with zero floats
