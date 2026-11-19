Feature: Pure-Integer WebGPU WGSL Compute Pipelines, Geodesic SwiGLU Active-Neuron Shortlisting & Single-Slab HOLOSLAB2 OPFS Store (R4-61)

  @R4-61 @build
  Scenario: R4.WebGpuGeodesicRuntime enforces 0-LSB WGSL parity, receipt-gated SwiGLU shortlisting, HOLOSLAB2 OPFS custody, and MAX_PREFILL_TILE clamping
    Given the formal LexLean module R4.WebGpuGeodesicRuntime and generated Rust verifier r4_webgpu_geodesic_runtime
    When WebGPU pure-integer WGSL parity checks, geodesic SwiGLU shortlisting policies, HOLOSLAB2 slab mount verdicts, and prefill tile widths are evaluated
    Then ZeroLsbAdmitted admits zero-float 0-LSB WGSL kernels within the 4 MiB staging budget while RefusedFloatOpInWgsl, RefusedWgslCpuParityDrift, and RefusedWasmStagingBudgetExceeded reject violations
    And AutoReceiptGated activates GeodesicSparse1536 only when top-1 parity >= 9000 bps and mean KL <= 35000 micros while allowing DenseExact / ForceGeodesicSparse overrides and refusing shortlist_k > ffn_dim with RefusedInvalidNeuronShortlist
    And HOLOSLAB2 admits warm metadata seals with SlabSealAdmitted and full scans with SlabFullMerkleVerified while failing closed on insufficient storage quota (RefusedInsufficientStorageQuota), seal mismatch (RefusedSlabSealMismatch), and piece digest corruption (RefusedSlabPieceHashMismatch)
    And MAX_PREFILL_TILE clamps batched prefill sequences up to 32 tokens with zero-LSB CPU parity against sequential step_token execution
