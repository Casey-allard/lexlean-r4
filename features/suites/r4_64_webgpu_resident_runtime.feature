Feature: Resident WebGPU Per-Layer Serving, HoloTok13 Speculative Decoding & Studio LoRA Lockstep (R4-64)

  @R4-64 @build
  Scenario: Per-layer <=128 MiB WebGPU slab admission, 0-LSB speculative tile verification with exact KV rewind, and lockstep <=40-token Studio LoRA prompt shrink
    Given the formal R4.WebGpuResidentRuntime module in src/R4/WebGpuResidentRuntime.lex.tex and controls in tests/lexlean/WebGpuResidentRuntimeControls.lex.tex
    When evaluating per-layer 128 MiB WebGPU buffer limits, HoloTok13 speculative tile parity and KV rewind, and fine-tuned UORR4V13 Studio LoRA prompt shrink gates
    Then zero-float <=128 MiB layer slabs within adapter limits admit as ResidentSlabAdmitted while >128 MiB slabs refuse with RefusedLayerExceeds128MiB and insufficient device limits refuse with RefusedInsufficientGpuBufferLimits
    And 0-LSB speculative tile verification with exact KV rewind admits as SpeculativeExact0LsbAdmitted while LSB drift refuses with RefusedSpeculativeParityDrift and KV mismatch refuses with RefusedKvRewindMismatch
    And compact <=40-token Studio prompts admit as StudioLoraUorV13Admitted only when zero-float Stitch-verified UORR4V13 LoRA manifests are pinned while unpinned shrinks refuse with RefusedPromptShrinkWithoutFineTunedManifest
