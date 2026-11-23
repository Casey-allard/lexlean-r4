# uor-foundry-v13-wasm

D90 Studio-only external `UORR4V13` + Stitch `stitch/xor-2-of-3/1` pure-integer WASM module for `UOR-Foundation/uor-r4` (`web/studio/`).

- `pkg/uor_foundry_v13_wasm.js`: 73505 bytes, sha256 `e4ea92b79bea247f51366d77b0c630a8defef744f944dee028d3bebb8b9ad63f`
- `pkg/uor_foundry_v13_wasm_bg.wasm`: 11431737 bytes, sha256 `855213514627bf885a38b0e1799d68a3d1f3b971366358700f67b2621e710ded`
- WAT float audit (`wasm_float_audit.json`): 50 `HoloV13Model` functions scanned, `compute_float_ops: 0`, `passed: true`, SIMD128 (`11277` ops)
- Features: `0-LSB` exact `HoloTok13SpeculativeDrafter` + `step_speculative_tile`, `v13_prewarm_prefix`, `v13_configure`
- Weight container target: `Casey-allard/lexlean-r4@weights-v13-qwen3-0.6b-foundry` (`3b04cd460735af679d912ce749e6e4bb2bccc0b0`, `container_sha256: 789333898aed75c3be0f1db85a4da5b50bb1067481c37a79af38c605b4488d61`)
