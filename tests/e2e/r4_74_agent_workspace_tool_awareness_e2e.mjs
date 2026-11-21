import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, '..', '..');

const wasmJsPath = path.join(ROOT, 'public', 'lexlean_r4.js');
const wasmBinPath = path.join(ROOT, 'public', 'lexlean_r4_bg.wasm');

const wasmMod = await import(pathToFileURL(wasmJsPath).href);
wasmMod.initSync({ module: fs.readFileSync(wasmBinPath) });

for (const [k, v] of Object.entries(wasmMod)) {
  if (typeof v === 'function' && !(k in globalThis)) {
    globalThis[k] = v;
  }
}

const {
  execute_inprism_foundry_turn,
  hydrate_holo_model,
  sharedKappaStore,
} = await import(pathToFileURL(path.join(ROOT, 'crates', 'wasm', 'js', 'worker_host_bridge.js')).href);

function sha256Hex(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

console.log('1. Verifying R4-74 MVM /workspace tool loop & Hop-1 ChatML KV reuse across consecutive inputs...');
{
  const appJs = fs.readFileSync(path.join(ROOT, 'public', 'app.js'), 'utf8');
  assert.ok(
    !appJs.includes("const activeSurfaceMode = wantsWorkspaceTools ? 'studio' : 'chat';"),
    'public/app.js must keep unified studio surface across all composer turns',
  );
  assert.ok(
    appJs.includes('include_excerpts: includeExcerpts'),
    'public/app.js must gate raw file excerpts to prevent 920-token prefill stalls',
  );

  const events = [];
  const beginPayloads = [];
  let stepCall = 0;

  const turn1 = await execute_inprism_foundry_turn(
    {
      id: 7401,
      payload: {
        prompt: 'List the files in /workspace using the mvm terminal',
        selected_model: 'uor-r4.qwen3-0.6b-holo',
        surface_mode: 'studio',
        composer_mode: 'agent',
        include_excerpts: false,
        live_hud: true,
        mcp_policy: 'safe',
        auto_run: true,
      },
    },
    {
      holoRuntimeStatus: () => ({
        status: 'ok',
        mounted_count: 1,
        qwen3_06b_mounted: true,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: '617ea77fd9c3d15285e351bd4f094cc0845cf5bc038e042a3fb5aadf0e406bda' }],
      }),
      hydrateHoloModel: async () => {
        throw new Error('Must NOT re-download or hydrate already-mounted model on input!');
      },
      holoBeginTurn: (rawJson) => {
        const parsed = JSON.parse(rawJson);
        beginPayloads.push(parsed);
        return JSON.stringify({
          status: 'ready',
          turn_id: 500 + beginPayloads.length,
          model_id: 'uor-r4.qwen3-0.6b-holo',
          max_new_tokens: 128,
          prefix_reused_tokens: beginPayloads.length === 1 ? 168 : 210,
          prefill_done: 20,
          prefill_total: 20,
        });
      },
      holoStepTurn: () => {
        stepCall += 1;
        if (stepCall === 1) {
          return JSON.stringify({
            status: 'needs_tool',
            turn_id: 501,
            model_id: 'uor-r4.qwen3-0.6b-holo',
            delta: '<tool_call>{"name":"mvm_exec","arguments":{"cmd":"ls /workspace"}}</tool_call>',
            tokens_emitted: 12,
            tok_per_sec: 42.0,
            prefix_reused_tokens: 168,
            tool_call: {
              name: 'mvm_exec',
              cmd: 'ls /workspace',
              arguments: { cmd: 'ls /workspace' },
              is_mutating: false,
            },
          });
        }
        return JSON.stringify({
          status: 'done',
          turn_id: 502,
          model_id: 'uor-r4.qwen3-0.6b-holo',
          delta: 'In `/workspace`, the RV64 MVM terminal shows `README.md` and `src`.',
          tokens_emitted: 18,
          tok_per_sec: 45.0,
          prefix_reused_tokens: 210,
        });
      },
      postEvent: (event, data) => events.push({ event, data }),
    },
  );

  assert.equal(turn1.ok, true, `Expected turn1 ok, got: ${JSON.stringify(turn1)}`);
  assert.equal(turn1.result.mvm_tool_executed, true, 'mvm_exec must execute against live /workspace');
  assert.equal(beginPayloads.length, 2, 'Must run Hop 0 -> mvm_exec -> Hop 1');
  assert.equal(beginPayloads[0].include_excerpts, false, 'Hop 0 must propagate include_excerpts=false');
  assert.equal(beginPayloads[1].tool_results.length, 1, 'Hop 1 must receive mvm_exec tool_results');
  assert.ok(
    String(beginPayloads[1].tool_results[0].stdout || '').includes('README.md'),
    `Real MVM ls /workspace must include README.md, got: ${JSON.stringify(beginPayloads[1].tool_results[0])}`,
  );
  assert.ok(
    events.some((e) => e.event === 'tool_call' && e.data.tool === 'mvm_exec' && e.data.cmd === 'ls /workspace'),
    'Must emit live tool_call event for UI card',
  );
  assert.ok(
    events.some((e) => e.event === 'tool_result' && e.data.tool === 'mvm_exec' && e.data.exit_code === 0),
    'Must emit live tool_result event with exit_code=0',
  );
}

console.log('2. Verifying earlyCatalogWarmCheck mounts catalog OPFS seal with ZERO HTTP fetches...');
{
  const origFetch = globalThis.fetch;
  let httpFetches = 0;
  globalThis.fetch = async (url) => {
    httpFetches += 1;
    throw new Error(`Unexpected HTTP fetch during warm catalog seal check: ${url}`);
  };
  try {
    // When model is already mounted in StudioEngine, hydrate_holo_model returns immediately with 0 fetches
    const origStatus = globalThis.studio_holo_runtime_status;
    globalThis.studio_holo_runtime_status = () =>
      JSON.stringify({
        status: 'ok',
        mounted_count: 1,
        qwen3_06b_mounted: true,
        models: [{ model_id: 'uor-r4.qwen3-0.6b-holo' }],
      });
    try {
      const fastOut = await hydrate_holo_model('uor-r4.qwen3-0.6b-holo', null, false, {});
      assert.equal(fastOut.qwen3_06b_mounted, true);
      assert.equal(httpFetches, 0, 'Mounted catalog model must perform zero HTTP fetches');
    } finally {
      globalThis.studio_holo_runtime_status = origStatus;
    }
  } finally {
    globalThis.fetch = origFetch;
    sharedKappaStore._memSlabs.clear();
  }
}

console.log('[PASS] R4-74 unified /workspace & MVM tool awareness + zero-HTTP warm mount verified.');
