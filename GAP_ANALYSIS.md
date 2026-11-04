# Structural & Mathematical Gap Analysis: `uor-r4` vs. `lexlean-r4`

This document records the comprehensive architectural comparison between the sister repository `uor-r4` (`crates/uor-r4-core`, `crates/uor-r4-model`, `crates/uor-r4-serve`, `crates/uor-r4-train`, `crates/uor-r4-tokenizer`) and `lexlean-r4`, detailing the six structural gaps that previously separated `lexlean-r4`'s browser runtime from a genuine end-to-end integer autoregressive language model, and how each gap is now closed.

---

## 1. Transformer Mechanism vs. UOR-R4 Integer Mechanism Equivalence Table

To achieve general language modeling without dense floating-point matrix multiplications (`matmul`) or transcendental `softmax`, every functional mechanism of a causal Transformer must have a closed, first-principles integer counterpart in the `uor-r4.lut-stack/8` (`UORLUT01`) pipeline:

| Transformer Mechanism | `uor-r4` (`uor-r4-serve` / `uor-r4-model`) | Previous `lexlean-r4` State (Before Closure) | Closed `lexlean-r4` State (`crates/wasm/src/lut_stack_exec.rs` + `lexlean/R4/*.lean`) |
| --- | --- | --- | --- |
| **Subword Tokenization** | Pinned 4,096-symbol byte-level BPE (`tokenizer.json`, SHA-256 `d36d3e8700a123e620012df77de195f244fbdb4d05d9e1aa7e77fa9407590f89`) with `<BOS>=0`, `<EOS>=1`, `<PAD>=2`, `<UNK>=3` and `Ġ` word-prefix encoding. | Word-piece/stem hashing into 280 concept basins (`UR4GLM08`) with ad-hoc punctuation splitting. | **Closed**: Exact `ByteBpe4096` tokenizer (`crates/wasm/src/tokenizer.json`, verified SHA-256 `d36d3e87...`) compiled into WASM and executed on every prompt and reply token. |
| **Token Embedding Table ($W_E$)** | `writer.table`: `4,096 × 256` `i16` (`Q12` fixed-point, `ONE_Q12 = 4096`, 64 quaternion cells × 4 lanes). | 8D summary vector (`[i32; 8]`) per stem/basin without full 256D `Q12` token table. | **Closed**: Full `4,096 × 256` `i16` `writer.table` (`2,097,152` bytes) loaded from `UORLUT01` and refined by `gpu/train_uorlut_v8.rs`. |
| **Causal Self-Attention / Context Mixing** | 6-layer `SpiralCore/FBS` (`l0..l5`): 16-head integer retention (`RETAIN_LAMBDA_Q31`), 64-cell Clifford/Fano/Octonion route maps (`route_idx`, `route_sign`, `time_mix`, `k_gate`, `v_scale`, `r_gate`, `out_scale`), plus depth-2 (`hamming.d2`) and depth-6 (`hamming.d6`) SWAR popcount read heads. | Verify-only container header check (`uorlut.rs` inspected the 64-byte header without executing `l0..l5` or `hamming.d2`/`d6`). | **Closed**: Full 6-layer `SpiralCore/FBS` recurrence (`stack_recurrence_core`) + `ServedHammingRead` (`d2`/`d6`) executed in `LutStackSession` on `crates/wasm/src/bootstrap_v8.uorlut`. |
| **Feed-Forward Network (FFN)** | Quaternion Hamilton product (`stack_hamilton_ffn`: `q_mul_q12(normed, prev_cell)` + `ffn_up`, `ffn_down`, `res_scale`) + `stack_norm_gain` (`rms_norm_64_q12`). | Single-cell `R4-24` Icosian probe without 6-layer `ffn_up`/`ffn_down` residual execution. | **Closed**: Exact 6-layer `stack_hamilton_ffn` + `stack_norm_gain` (`final_norm_gain`) matching `uor-r4-serve/src/lut.rs` bit-for-bit. |
| **Episodic / In-Context Induction Heads** | `/7` `ServedTurnMemory` (`memory.opening` sorted `[u32; 8] -> u32` + `memory.reply` sorted `[u32; 9] -> u32` binary search) + `/8` `ServedPointKey` (`memory.point.points` 8D `Q12` L1 plurality over 8 salient token vectors). | Hardcoded `if/else` string intercepts (`is_self_inquiry`, `is_how_you_work`, `is_pure_greeting`, `has_day_or_mood`, `fun fact`) and basin-horizon templates (`UR4GLM08`). | **Closed**: All hardcoded conversational string intercepts deleted; `/7` `memory.opening` + `memory.reply` (`220,307` causal transitions) and `/8` `memory.point` (`2,792` Q12 rows) drive autoregressive token-by-token generation. |
| **Paraphrase Continuation Handshake (`/8 -> /7`)** | **Bug in `uor-r4`**: When `memory.opening` missed on a paraphrased prompt and `/8` `memory.point` won step 0, `anchors.reply` remained the user's raw prompt tokens, causing step 1+ to miss `memory.reply` and collapse into whole-table L1 nearest fallback. | Same disconnect between `/8` opening recovery and subsequent token continuation. | **Closed & Fixed Beyond `uor-r4`**: `memory.point.anchors` (`anchor_len = 24`) stores `[canon_reply_ids (8), p_open_ids (8), salient (8)]`. On any `/8` point-key recovery or paraphrased `/7` opening hit, `anchors.reply` synchronizes to the winning training turn's `canon_reply_ids` so `memory.reply` continues predicting token-by-token across `step = 1, 2, ...`. |
| **Output Logit Projection & Softmax ($W_U$)** | `nearest_token_l1(target, writer.table)`: whole-table `L1` distance over all 4,096×256 `Q12` token rows (zero `matmul`, zero `softmax`). | Stem/horizon lookup with generic fallback prose when terms were unknown. | **Closed**: Full 4,096-token×256D `Q12` `nearest_token_l1` readout on every un-anchored step, plus top-8 `Q12` L1 candidate ranking in the Studio diagnostics inspector. |

---

## 2. Summary of the Six Gaps Closed in `lexlean-r4`

### Gap 1: Container Header-Only Inspection vs. Full `UORLUT01` (`uor-r4.lut-stack/8`) Execution
- **Root Cause**: `crates/wasm/src/uorlut.rs` previously validated only the 64-byte `UORLUT01` magic header and section directory (`gpu/short.uorlut` was a 256-byte header stub), while conversational chat ran on `UR4GLM08` basin templates.
- **Resolution**: Implemented `crates/wasm/src/lut_stack_exec.rs` (`LutStackModel` + `LutStackSession`), parsing all 111 sections of the `uor-r4.lut-stack/8` (`UORLUT01`) binary container (`meta.json`, `writer.table`, `l0..l5.*`, `final_norm_gain`, `hamming.d2.*`, `hamming.d6.*`, `memory.opening.*`, `memory.reply.*`, `memory.point.*`) and executing the full 256D `Q12` recurrence and BPE-4096 token decoder in WebAssembly.

### Gap 2: Subword Tokenizer Mismatch
- **Root Cause**: `lexlean-r4` lacked the pinned 4,096-token byte-level BPE vocabulary from `uor-r4/crates/uor-r4-tokenizer/data/tokenizer.json`.
- **Resolution**: Embedded `crates/wasm/src/tokenizer.json` (`sha256 = d36d3e8700a123e620012df77de195f244fbdb4d05d9e1aa7e77fa9407590f89`) and implemented `ByteBpe4096` (`encode`, `decode`, `encode_dialogue_prompt` with `User: ... \nAssistant:` framing) directly inside `crates/wasm/src/lut_stack_exec.rs`.

### Gap 3: Hardcoded Conversational Intercepts & Canned Mad-Libs Responses
- **Root Cause**: `crates/wasm/src/assistant.rs` and `gpu/glm_reasoner_template.rs` contained hardcoded `if/else` string intercepts (`is_self_inquiry`, `is_how_you_work`, `is_pure_greeting`, `"how are you"`, `"joke"`, `has_day_or_mood`, `fun fact`, `content_words.is_empty()`) that returned static prebuilt paragraphs instead of running next-token prediction.
- **Resolution**: Deleted all hardcoded conversational string intercepts in `crates/wasm/src/assistant.rs` and `gpu/glm_reasoner_template.rs`. Every conversational and single-turn knowledge prompt now routes through `self.lut_stack.generate_reply(&self.chat_history, trimmed, 768)`.

### Gap 4: Paraphrased Prompt Continuation Collapse (`/8` Point-Key -> `/7` Reply Chain)
- **Root Cause**: In `uor-r4-serve/src/lut.rs`, `/8` `ServedPointKey` recovered only the opening token (`reply_step == 0`) when a user rephrased a question, leaving `self.anchors.reply` set to the non-canonical prompt tokens; at `reply_step == 1`, `memory.reply` missed and collapsed into repetitive `nearest_token_l1` tokens.
- **Resolution**: Extended `memory.point.anchors` (`anchor_len = 24`) to store `[canon_reply_ids (8), p_open_ids (8), salient (8)]`. Whenever `/8` `memory.point` wins on a novel phrasing—or `/7` `memory.opening` hits a registered paraphrase variation—`self.anchors.reply` updates to `canon_reply_ids`, enabling `memory.reply` to predict token-by-token (`step = 1, 2, 3, ...`) for the entire multi-sentence response.

### Gap 5: End-to-End Multi-Domain `UORLUT01` (`uor-r4.lut-stack/8`) Training Artifact
- **Root Cause**: `uor-r4`'s `k2-s1.uorlut` was trained on a narrow code/architecture slice (`1,341` openings, `114,507` reply rows) and lacked general conversational, mathematical, scientific, historical, and world-knowledge turns.
- **Resolution**: Created and executed `gpu/train_uorlut_v8.rs`, starting from the RTX-5090-trained 6-layer `SpiralCore/FBS` trunk (`k2-s1.uorlut`), refining `2,836` active BPE token basins in `writer.table` (`4096 × 256` `Q12`) via discrete coordinate sweeps across **542** multi-domain dialogue turns, and compiling `crates/wasm/src/bootstrap_v8.uorlut` (`22,449,792` bytes, `2,118` `/7` opening entries, `220,307` `/7` causal reply transitions, `2,792` `/8` Q12 point-key rows, `sha256 = 04be6a8885e96c901f2c4f50c79a604556a324d7510fe0b753dc0b456284e02e`).

### Gap 6: Browser On-Demand Weight Serving & `>100 MB` `IndexedDB` Persistence
- **Root Cause**: GitHub Pages enforces file size limits and static-only serving, while `app.js` previously restricted WebCache snapshot imports to `<= 65,536` bytes of hex-encoded text.
- **Resolution**:
  1. Embedded the full `22.45 MB` `bootstrap_v8.uorlut` inside `public/lexlean_r4_bg.wasm` (automatically cached by `CacheStorage` `uor-r4-wasm-weights-v2`).
  2. Exported 43 chunked 512 KB shards (`gpu/weights/uor_r4_glm_v8.uorlut.part00..part42` + `gpu/weights/manifest.json`).
  3. Added `studio_load_uorlut_bytes` and `studio_weight_pack_status` WASM bindings plus `IndexedDB` (`UOR_R4_WEIGHT_CACHE`) persistence in `public/lexlean_r4.js` and `public/app.js`, enabling users to hot-load and cache any standalone `.uorlut` binary or multi-part `.uorlut.part*` shard set (`>100 MB`) directly in browser `IndexedDB`.
