Feature: R4-75 Honest Resident-RAM Turn UX, Digest Cache-Busting, Real 28-Layer WebGPU Weight Slabs, and Hybrid UORR4V14 Drafter

  @R4-75 @build
  Scenario: Resident turns execute with zero network/OPFS bytes, digest-stamped asset URLs, real 28-layer WebGPU slabs, and 0-LSB UORR4V14 speculative waves
    Given the formal LexLean module R4.ResidentGpuHybridDrafter and its proof controls ResidentGpuHybridDrafterControls
    When build-wasm emits the 6-file public closure and the user dispatches turns on a mounted Holo model
    Then public/index.html and public/app.js stamp content-addressed ?v=v12r4-<js_sha12> asset query strings
    And public/app.js reads visible #model-select ahead of #composer-model-select and labels mounted turns Resident · Generating (0 B dl) instead of Streaming
    And worker_host_bridge.js emits holo_ready on hydrate_holo_model completion, skips mapAsync stalls during prefill, and streams real 28-layer Int4 RHT-32 slabs via studio_export_holo_layer_slab_bytes
    And HoloTok13SpeculativeDrafter integrates HopfZetaPoincareEngineV14::propose_speculative_wave_holo_ids verified 0-LSB against Qwen3
    And negative controls refuse hydration on resident models, desynchronized pickers, static unstamped query strings, 64-byte placeholder GPU slabs, prefill mapAsync stalls, and verifier drift
