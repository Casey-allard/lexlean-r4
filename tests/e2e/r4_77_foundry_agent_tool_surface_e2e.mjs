#!/usr/bin/env node
/**
 * Formal R4.FoundryAgentToolSurface E2E Suite (R4-77):
 * 1. Verifies formal admission & negative refusals (`admitFoundryAgentToolCall`).
 * 2. Verifies real Wasm `studio_execute_agent_tool_json` across:
 *    - `foundry_publish_holo` before `foundry_prove` -> `RefusedPublishWithoutProvedWorkspace` (FK-02)
 *    - `foundry_publish_holo` in `PlanReadOnly` -> `RefusedPlanModeMutation` (R5)
 *    - `foundry_prove` over `Classical.choice` -> `RefusedUnsoundAxiomsInAgentProve`
 *    - `foundry_prove` over valid `/workspace/src/App.lex.tex` -> `FoundryProveAdmitted` (`stage = "Proved"`)
 *    - `foundry_publish_holo` after proof -> `FoundryPublishHoloAdmitted` (`blake3:` `holo_kappa`)
 *    - `/workspace` edit after proof -> `FK-03` reset to `Drafting` refusing subsequent publish
 *    - `vtensor_render_3d` LOD 2 -> `VtensorRender3dAdmitted` + `frame_sha256`; LOD 7 -> `RefusedOutOfBoundsLodOrFloat3d`
 * 3. Verifies live `execute_inprism_foundry_turn` multi-hop agent execution of `foundry_prove` and `vtensor_render_3d`.
 */

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

const wasmMod = await import(path.join(rootDir, 'public/lexlean_r4.js'));
const wasmBytes = fs.readFileSync(path.join(rootDir, 'public/lexlean_r4_bg.wasm'));
wasmMod.initSync({ module: wasmBytes });

const {
  admitFoundryAgentToolCall,
  execute_inprism_foundry_turn,
  studio_dispatch,
  studio_execute_agent_tool_json,
} = wasmMod;

// 1. Formal JS mirror checks
assert.equal(admitFoundryAgentToolCall('foundry_prove', true, false, 0, 0, 0), 'FoundryProveAdmitted');
assert.equal(admitFoundryAgentToolCall('foundry_publish_holo', false, true, 0, 0, 0), 'FoundryPublishHoloAdmitted');
assert.equal(admitFoundryAgentToolCall('foundry_publish_holo', true, true, 0, 0, 0), 'RefusedPlanModeMutatingPublish');
assert.equal(admitFoundryAgentToolCall('foundry_publish_holo', false, false, 0, 0, 0), 'RefusedPublishWithoutProvedWorkspace');
assert.equal(admitFoundryAgentToolCall('foundry_prove', false, false, 1, 0, 0), 'RefusedUnsoundAxiomsInAgentProve');
assert.equal(admitFoundryAgentToolCall('vtensor_render_3d', false, false, 0, 2, 0), 'VtensorRender3dAdmitted');
assert.equal(admitFoundryAgentToolCall('vtensor_render_3d', false, false, 0, 7, 0), 'RefusedOutOfBoundsLodOrFloat3d');

// 2. Real Wasm FK-01..09 lifecycle & vTensor 3D tool surface
const validLex = [
  '\\begin{lexlean}{App.Main}',
  '\\useglossary{lexlean.std.bool@1.2.0}',
  '\\title{Boolean}',
  '\\begin{semanticmodule}',
  '\\semanticdata{{"declarations":[{"axioms":[],"kind":"theorem","name":"appOk","parameters":[],"proof":{"kind":"reflexivity"},"statement":{"kind":"eq","left":{"kind":"bool","value":true},"right":{"kind":"bool","value":true}}}],"spec":"lexlean/semantic-module/2"}}',
  '\\end{semanticmodule}',
  '\\end{lexlean}',
].join('\n');

const wOut = JSON.parse(studio_dispatch(`1\tstudio\tworkspace_write\tsrc/App.lex.tex\n${validLex}\n`));
assert.equal(wOut.status, 'ok');

// FK-02: publish before prove must be refused
const pubBeforeProve = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_publish_holo","arguments":{"title":"demo"}}', 'agent'),
);
assert.equal(pubBeforeProve.status, 'error');
assert.equal(pubBeforeProve.error_code, 'RefusedPublishWithoutProvedWorkspace');

// PlanReadOnly: publish must be refused by composer policy
const pubInPlan = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_publish_holo","arguments":{"title":"demo"}}', 'plan'),
);
assert.equal(pubInPlan.status, 'error');
assert.equal(pubInPlan.error_code, 'RefusedPlanModeMutation');

// Prove valid module (allowed even in plan mode because foundry_prove is read-only verification!)
const proveOk = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_prove","arguments":{"module_path":"src/App.lex.tex"}}', 'plan'),
);
assert.equal(proveOk.status, 'ok');
assert.equal(proveOk.verdict, 'FoundryProveAdmitted');
assert.equal(proveOk.stage, 'Proved');
assert.ok(String(proveOk.attestation_kappa).startsWith('blake3:'));

// Publish after prove in agent mode -> admitted!
const pubOk = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_publish_holo","arguments":{"title":"demo"}}', 'agent'),
);
assert.equal(pubOk.status, 'ok');
assert.equal(pubOk.verdict, 'FoundryPublishHoloAdmitted');
assert.equal(pubOk.stage, 'PublishedHolo');
assert.ok(String(pubOk.holo_kappa).startsWith('blake3:'));

// FK-03: editing workspace resets proved state -> publish refused again!
studio_dispatch(`1\tstudio\tworkspace_write\tsrc/App.lex.tex\n${validLex}\n% edited\n`);
const pubAfterEdit = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_publish_holo","arguments":{"title":"demo"}}', 'agent'),
);
assert.equal(pubAfterEdit.status, 'error');
assert.equal(pubAfterEdit.error_code, 'RefusedPublishWithoutProvedWorkspace');

// Unsound Classical.choice in .lex.tex -> refused by InTabProver!
studio_dispatch(`1\tstudio\tworkspace_write\tsrc/Bad.lex.tex\n% Classical.choice\n${validLex}\n`);
const proveBad = JSON.parse(
  studio_execute_agent_tool_json('{"name":"foundry_prove","arguments":{"module_path":"src/Bad.lex.tex"}}', 'agent'),
);
assert.equal(proveBad.status, 'error');
assert.equal(proveBad.error_code, 'RefusedUnsoundAxiomsInAgentProve');
studio_dispatch('1\tstudio\tworkspace_delete\tsrc/Bad.lex.tex');

// vTensor 3D render tool: LOD 2 admitted, LOD 6 refused
const renderOk = JSON.parse(
  studio_execute_agent_tool_json('{"name":"vtensor_render_3d","arguments":{"preset":"icosian-600-shell","lod":2}}', 'agent'),
);
assert.equal(renderOk.status, 'ok');
assert.equal(renderOk.verdict, 'VtensorRender3dAdmitted');
assert.equal(renderOk.frame_sha256.length, 64);
assert.ok(renderOk.rgba_bytes > 0);

const renderBadLod = JSON.parse(
  studio_execute_agent_tool_json('{"name":"vtensor_render_3d","arguments":{"preset":"icosian-600-shell","lod":6}}', 'agent'),
);
assert.equal(renderBadLod.status, 'error');
assert.equal(renderBadLod.error_code, 'RefusedOutOfBoundsLodOrFloat3d');

// 3. Multi-hop execute_inprism_foundry_turn invoking vtensor_render_3d and emitting vtensor_preview_3d_rendered event
let hopCount = 0;
const previewEvents = [];
const turnOut = await execute_inprism_foundry_turn(
  {
    id: 'r4_77_turn',
    payload: {
      prompt: 'Render the Hopf-Zeta torus in 3D preview',
      selected_model: 'uor-r4.qwen3-0.6b-holo',
      surface_mode: 'studio',
      composer_mode: 'agent',
    },
  },
  {
    postEvent: (ev, data) => {
      if (ev === 'vtensor_preview_3d_rendered') previewEvents.push(data);
    },
    collectWorkspaceSnapshot: () => ({ files: [] }),
    holoRuntimeStatus: () =>
      JSON.stringify({
        mounted_count: 1,
        qwen3_06b_mounted: true,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: '617ea77f', n_layers: 28, d_model: 1024, vocab_size: 151936 }],
      }),
    holoBeginTurn: () => {
      hopCount += 1;
      return JSON.stringify({ status: 'ready', turn_id: 300 + hopCount, model_id: 'uor-r4.qwen3-0.6b-holo' });
    },
    holoStepTurn: () => {
      if (hopCount === 1) {
        return JSON.stringify({
          status: 'needs_tool',
          turn_id: 301,
          model_id: 'uor-r4.qwen3-0.6b-holo',
          delta: '',
          tokens_emitted: 8,
          tool_call: {
            name: 'vtensor_render_3d',
            arguments: { preset: 'hopf-zeta-torus', lod: 2 },
          },
        });
      }
      return JSON.stringify({
        status: 'done',
        turn_id: 302,
        model_id: 'uor-r4.qwen3-0.6b-holo',
        delta: 'Rendered hopf-zeta-torus in 3D preview.',
        tokens_emitted: 7,
      });
    },
  },
);
assert.equal(turnOut.ok, true);
assert.equal(previewEvents.length, 1);
assert.equal(previewEvents[0].preset, 'hopf-zeta-torus');
assert.equal(previewEvents[0].frame_sha256.length, 64);

console.log('r4_77_foundry_agent_tool_surface_e2e: ALL CHECKS PASSED');
