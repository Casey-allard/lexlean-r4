import test from 'node:test';
import assert from 'node:assert/strict';

import {
  hydrate_holo_model,
  execute_inprism_foundry_turn,
} from '../../crates/wasm/js/worker_host_bridge.js';

function createFakeStepperHooks(options = {}) {
  let nextTurnId = 1;
  let activeTurn = null;
  let cancelledTurns = new Set();
  let beginCalls = [];
  let cancelCalls = [];
  let stepCalls = [];

  const mounted = options.mounted !== false;
  const runtimeStatusObj = mounted
    ? {
        qwen3_06b_mounted: true,
        qwen3_17b_mounted: false,
        models: [
          {
            model_id: 'uor-r4.qwen3-0.6b-holo',
            container_sha256: 'a'.repeat(64),
            n_layers: 2,
            d_model: 32,
            vocab_size: 270,
          },
        ],
      }
    : { qwen3_06b_mounted: false, qwen3_17b_mounted: false, models: [] };

  return {
    beginCalls,
    stepCalls,
    cancelCalls,
    hooks: {
      holoRuntimeStatus: () => JSON.stringify(runtimeStatusObj),
      collectWorkspaceSnapshot: options.collectWorkspaceSnapshot || (() => ({ files: [] })),
      dispatchBridge:
        options.dispatchBridge ||
        (async (env) => ({
          status: 'ok',
          exit_code: 0,
          mvm_stdout: `/workspace\n`,
          mvm_stderr: '',
          env,
        })),
      ensureIdentity: async () => ({ did: 'did:prism:test' }),
      signAction: async (action, targetKappa) => ({ action, targetKappa }),
      holoBeginTurn: (payloadJson) => {
        const req = JSON.parse(payloadJson);
        beginCalls.push(req);
        if (!mounted) {
          return JSON.stringify({
            status: 'error',
            error_code: 'ModelNotMounted',
            message: 'No model mounted',
          });
        }
        if (activeTurn) {
          return JSON.stringify({
            status: 'error',
            error_code: 'BusyTurn',
            message: 'Turn already active',
          });
        }
        const hopIndex = Array.isArray(req.tool_results) ? req.tool_results.length : 0;
        const turnId = nextTurnId++;
        const script =
          typeof options.scriptForHop === 'function'
            ? options.scriptForHop(hopIndex, req)
            : [{ status: 'done', delta: 'ok', tokens_emitted: 1, tok_per_sec: 12.5 }];
        activeTurn = {
          turnId,
          stepIndex: 0,
          script,
        };
        return JSON.stringify({
          status: 'ready',
          turn_id: turnId,
          model_id: req.selected_model || 'uor-r4.qwen3-0.6b-holo',
          max_new_tokens: req.max_new_tokens || 1536,
        });
      },
      holoStepTurn: (turnIdBig) => {
        const turnId = Number(turnIdBig);
        stepCalls.push(turnId);
        if (!activeTurn || activeTurn.turnId !== turnId) {
          return JSON.stringify({
            status: 'error',
            error_code: 'TurnNotFound',
            message: 'Unknown turn',
          });
        }
        if (cancelledTurns.has(turnId)) {
          activeTurn = null;
          return JSON.stringify({
            status: 'cancelled',
            turn_id: turnId,
            delta: '',
            tokens_emitted: 0,
            tok_per_sec: 0,
          });
        }
        const entry = activeTurn.script[activeTurn.stepIndex++];
        if (!entry) {
          activeTurn = null;
          return JSON.stringify({
            status: 'done',
            turn_id: turnId,
            delta: '',
            tokens_emitted: 0,
            tok_per_sec: 20,
            receipt_sha256: 'b'.repeat(64),
            weights_kappa: `sha256:${'a'.repeat(64)}`,
            geometric_metrics: { float_ops: 0, dense_matmul_ops: 0, n_layers: 2 },
          });
        }
        if (entry.status !== 'streaming') {
          activeTurn = null;
        }
        return JSON.stringify({
          turn_id: turnId,
          receipt_sha256: entry.status === 'done' ? 'b'.repeat(64) : null,
          weights_kappa: entry.status === 'done' ? `sha256:${'a'.repeat(64)}` : null,
          geometric_metrics: {
            float_ops: 0,
            dense_matmul_ops: 0,
            n_layers: 2,
            step_index: activeTurn ? activeTurn.stepIndex : 99,
          },
          ...entry,
        });
      },
      holoCancelTurn: (turnIdBig) => {
        const turnId = Number(turnIdBig);
        cancelCalls.push(turnId);
        cancelledTurns.add(turnId);
        if (activeTurn && activeTurn.turnId === turnId) {
          activeTurn = null;
          return JSON.stringify({ status: 'cancelled', turn_id: turnId });
        }
        return JSON.stringify({ status: 'not_found', turn_id: turnId });
      },
    },
  };
}

test('hydrate_holo_model and inprism_r4_turn enforce ConsentRequiredForWeightDownload off-origin', async () => {
  const prevLocation = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;
  let offOriginFetchCount = 0;

  try {
    globalThis.location = { origin: 'http://localhost:4173', href: 'http://localhost:4173/' };
    globalThis.indexedDB = undefined;
    globalThis.fetch = async (url) => {
      const target = String(url || '');
      if (!target.startsWith('http://localhost:4173/')) {
        offOriginFetchCount += 1;
        throw new Error(`Off-origin fetch forbidden without consent: ${target}`);
      }
      throw new Error('local weight mirror absent');
    };

    await assert.rejects(
      () =>
        hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, false, {
          allowOffOriginHydration: false,
        }),
      (err) =>
        err &&
        (err.code === 'ConsentRequiredForWeightDownload' ||
          err.error_code === 'ConsentRequiredForWeightDownload'),
    );
    assert.equal(offOriginFetchCount, 0);

    const fake = createFakeStepperHooks({ mounted: false });
    const res = await execute_inprism_foundry_turn(
      { id: 'req-consent-1', payload: { prompt: 'Build counter app' } },
      fake.hooks,
    );
    assert.equal(res.ok, false);
    assert.equal(res.error_code, 'ConsentRequiredForWeightDownload');
    assert.equal(offOriginFetchCount, 0);
  } finally {
    globalThis.location = prevLocation;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
  }
});

test('execute_inprism_foundry_turn emits EmptyPrompt on blank prompt', async () => {
  const fake = createFakeStepperHooks();
  const res = await execute_inprism_foundry_turn(
    { id: 'req-empty', payload: { prompt: '   ' } },
    fake.hooks,
  );
  assert.equal(res.ok, false);
  assert.equal(res.error_code, 'EmptyPrompt');
});

test('execute_inprism_foundry_turn streams 4-token chunks with telemetry and bounded mvm_exec hops', async () => {
  const events = [];
  let wsVersion = 0;
  const fake = createFakeStepperHooks({
    collectWorkspaceSnapshot: () => ({
      files: [
        {
          path: 'src/App.tsx',
          content: `export const v = ${wsVersion};`,
          sha256: `sha-${wsVersion}`,
        },
      ],
    }),
    dispatchBridge: async (env) => {
      wsVersion += 1;
      assert.match(env, /\tmvm_exec\t/);
      return {
        status: 'ok',
        exit_code: 0,
        mvm_stdout: 'updated\n',
        mvm_stderr: '',
      };
    },
    scriptForHop: (hopIndex) => {
      if (hopIndex === 0) {
        return [
          { status: 'streaming', delta: 'Checking: ', tokens_emitted: 4, tok_per_sec: 42.5 },
          {
            status: 'needs_tool',
            delta: '',
            tokens_emitted: 7,
            tok_per_sec: 48.0,
            tool_call: { name: 'mvm_exec', cmd: 'echo hi > out.txt' },
          },
        ];
      }
      return [
        {
          status: 'streaming',
          delta: '<prism-write path="src/App.tsx">',
          tokens_emitted: 4,
          tok_per_sec: 55.25,
        },
        { status: 'done', delta: 'done</prism-write>', tokens_emitted: 7, tok_per_sec: 60.5 },
      ];
    },
  });

  const envelope = await execute_inprism_foundry_turn(
    {
      id: 'req-stream-1',
      payload: {
        prompt: 'Update App',
        preferredModel: 'uor-r4.qwen3-0.6b-holo',
      },
    },
    {
      ...fake.hooks,
      postEvent: (event, data) => events.push({ event, data }),
    },
  );

  assert.equal(envelope.ok, true);
  assert.equal(
    envelope.result.reply,
    'Checking: <prism-write path="src/App.tsx">done</prism-write>',
  );
  assert.equal(envelope.result.tokens_emitted, 14);
  assert.equal(envelope.result.tok_per_sec, 60.5);
  assert.equal(envelope.result.mvm_tool_executed, true);
  assert.equal(envelope.result.tool_results.length, 1);
  assert.deepEqual(envelope.result.tool_results[0].changed_files, ['src/App.tsx']);

  const finalMetrics = envelope.result['telemetry'];
  assert.ok(finalMetrics && typeof finalMetrics === 'object');
  assert.equal(finalMetrics.tokens_emitted, 14);
  assert.equal(finalMetrics.tok_per_sec, 60.5);
  assert.equal(finalMetrics.geometric_metrics.float_ops, 0);
  assert.equal(finalMetrics.holo_v13.container_format, 'UORR4V13');
  assert.equal(finalMetrics.holo_v13.model_id, 'uor-r4.qwen3-0.6b-holo');
  assert.equal(finalMetrics.holo_v13.receipt_sha256, 'b'.repeat(64));

  const chunks = events.filter((e) => e.event === 'chat_chunk');
  assert.equal(chunks.length, 3);
  for (const ch of chunks) {
    assert.ok(ch.data['telemetry'], 'every chat_chunk must include telemetry object');
    assert.equal(ch.data['telemetry'].tokens_emitted, ch.data.tokens_emitted);
    assert.equal(ch.data['telemetry'].tok_per_sec, ch.data.tok_per_sec);
    assert.equal(ch.data['telemetry'].holo_v13.container_format, 'UORR4V13');
  }
  assert.ok(events.some((e) => e.event === 'tool_call' && e.data.cmd === 'echo hi > out.txt'));
  assert.ok(events.some((e) => e.event === 'tool_result' && e.data.exit_code === 0));
  assert.ok(events.some((e) => e.event === 'workspace_sync'));
});

test('execute_inprism_foundry_turn caps mvm_exec tool loop at 6 iterations', async () => {
  let toolExecCount = 0;
  const fake = createFakeStepperHooks({
    dispatchBridge: async () => {
      toolExecCount += 1;
      return { status: 'ok', exit_code: 0, mvm_stdout: 'ok', mvm_stderr: '' };
    },
    scriptForHop: () => [
      {
        status: 'needs_tool',
        delta: 'hop;',
        tokens_emitted: 2,
        tok_per_sec: 30,
        tool_call: { name: 'mvm_exec', cmd: 'pwd' },
      },
    ],
  });

  const envelope = await execute_inprism_foundry_turn(
    { id: 'req-cap', payload: { prompt: 'Loop test', preferredModel: 'uor-r4.qwen3-0.6b-holo' } },
    fake.hooks,
  );
  assert.equal(envelope.ok, true);
  assert.equal(toolExecCount, 6);
  assert.equal(envelope.result.tool_results.length, 6);
});

test('Worker message loop interleaves terminal:write and cancel_inprism_turn between 4-token slices', async () => {
  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevBegin = globalThis.holo_begin_foundry_turn;
  const prevStep = globalThis.holo_step_foundry_turn;
  const prevCancel = globalThis.holo_cancel_foundry_turn;
  const prevStatus = globalThis.studio_holo_runtime_status;
  const prevDispatch = globalThis.studio_dispatch;

  const posted = [];
  let resolveReady;
  let resolveDone;
  let timerId;
  const readyPromise = new Promise((r) => {
    resolveReady = r;
  });
  const finished = new Promise((resolve, reject) => {
    resolveDone = resolve;
    timerId = setTimeout(
      () => reject(new Error('Timed out waiting for Worker turn completion')),
      2000,
    );
  });

  try {
    class MockWorkerGlobalScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
      postMessage(msg) {
        posted.push(msg);
        if (msg && msg.event === 'ready') {
          resolveReady(msg);
        }
        if (msg && msg.id === 'turn-interleave-1' && typeof msg.ok === 'boolean') {
          clearTimeout(timerId);
          resolveDone(msg);
        }
      }
    }
    const mockSelf = new MockWorkerGlobalScope();
    globalThis.WorkerGlobalScope = MockWorkerGlobalScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });

    const fake = createFakeStepperHooks({
      scriptForHop: () => [
        { status: 'streaming', delta: 'step1 ', tokens_emitted: 4, tok_per_sec: 40 },
        { status: 'streaming', delta: 'step2 ', tokens_emitted: 8, tok_per_sec: 45 },
        { status: 'streaming', delta: 'step3 ', tokens_emitted: 12, tok_per_sec: 50 },
        { status: 'done', delta: 'step4', tokens_emitted: 16, tok_per_sec: 55 },
      ],
    });
    globalThis.holo_begin_foundry_turn = fake.hooks.holoBeginTurn;
    globalThis.holo_step_foundry_turn = fake.hooks.holoStepTurn;
    globalThis.holo_cancel_foundry_turn = fake.hooks.holoCancelTurn;
    globalThis.studio_holo_runtime_status = fake.hooks.holoRuntimeStatus;
    globalThis.studio_dispatch = (env) =>
      JSON.stringify({ status: 'ok', exit_code: 0, mvm_stdout: 'term-ok', env });

    await import('../../crates/wasm/js/worker_host_bridge.js?worker_interleave_test=2');
    assert.equal(typeof mockSelf.onmessage, 'function');
    await readyPromise;

    mockSelf.onmessage({
      data: {
        id: 'turn-interleave-1',
        op: 'inprism_r4_turn',
        payload: { prompt: 'Build counter', preferredModel: 'uor-r4.qwen3-0.6b-holo' },
      },
    });

    await new Promise((r) => setTimeout(r, 0));
    mockSelf.onmessage({
      data: {
        id: 'term-1',
        op: 'terminal:write',
        payload: { cmd: 'pwd' },
      },
    });
    await new Promise((r) => setTimeout(r, 0));
    mockSelf.onmessage({
      data: {
        id: 'cancel-1',
        op: 'cancel_inprism_turn',
        payload: { targetId: 'turn-interleave-1' },
      },
    });

    const finalTurnMsg = await finished;
    assert.equal(finalTurnMsg.ok, false);
    assert.equal(finalTurnMsg.cancelled, true);
    assert.equal(finalTurnMsg.error_code, 'Cancelled');
    assert.ok(fake.stepCalls.length < 4, 'cancellation must preempt remaining slices');

    const termIdx = posted.findIndex((m) => m && m.id === 'term-1' && typeof m.ok === 'boolean');
    const finalTurnIdx = posted.findIndex(
      (m) => m && m.id === 'turn-interleave-1' && typeof m.ok === 'boolean',
    );
    assert.ok(termIdx >= 0, 'terminal:write must respond');
    assert.ok(
      termIdx < finalTurnIdx,
      `terminal:write (idx=${termIdx}) must complete before inprism_r4_turn finishes (idx=${finalTurnIdx})`,
    );
  } finally {
    clearTimeout(timerId);
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.holo_begin_foundry_turn = prevBegin;
    globalThis.holo_step_foundry_turn = prevStep;
    globalThis.holo_cancel_foundry_turn = prevCancel;
    globalThis.studio_holo_runtime_status = prevStatus;
    globalThis.studio_dispatch = prevDispatch;
  }
});

test('hydrate_holo_model with allowOffOriginHydration=true invokes off-origin fetch without ConsentRequiredForWeightDownload', async () => {
  const prevLocation = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;
  const fetchedUrls = [];

  try {
    globalThis.location = { origin: 'http://localhost:4173', href: 'http://localhost:4173/' };
    globalThis.indexedDB = undefined;
    globalThis.fetch = async (url) => {
      fetchedUrls.push(String(url || ''));
      throw new Error('simulated mirror offline');
    };

    await assert.rejects(
      () =>
        hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, false, {
          allowOffOriginHydration: true,
        }),
      (err) =>
        err &&
        err.code !== 'ConsentRequiredForWeightDownload' &&
        !String(err.message).includes('ConsentRequiredForWeightDownload'),
    );
    assert.ok(
      fetchedUrls.some((u) => !u.startsWith('http://localhost:4173/')),
      `expected off-origin fetch attempt once consent granted, got: ${JSON.stringify(fetchedUrls)}`,
    );
  } finally {
    globalThis.location = prevLocation;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
  }
});

test('execute_inprism_foundry_turn preserves native-q31-d85 and propagates turn_id on stepper errors', async () => {
  let hydrated = false;
  const fake = createFakeStepperHooks({
    mounted: false,
    scriptForHop: () => [{ status: 'done', delta: 'ok', tokens_emitted: 1, tok_per_sec: 12 }],
  });
  const res = await execute_inprism_foundry_turn(
    {
      id: 'req-native-1',
      payload: { prompt: 'Offline prompt', preferredModel: 'uor-r4.native-q31-d85' },
    },
    {
      ...fake.hooks,
      hydrateHoloModel: async () => {
        hydrated = true;
      },
      holoBeginTurn: (payloadJson) => {
        const req = JSON.parse(payloadJson);
        fake.beginCalls.push(req);
        return JSON.stringify({ status: 'ready', turn_id: 77, model_id: req.selected_model });
      },
      holoStepTurn: () =>
        JSON.stringify({
          status: 'done',
          turn_id: 77,
          model_id: 'uor-r4.native-q31-d85',
          delta: 'hello',
          tokens_emitted: 1,
          tok_per_sec: 20,
        }),
    },
  );
  assert.equal(hydrated, false, 'native-q31-d85 must not invoke Holo hydration');
  assert.equal(fake.beginCalls[0].selected_model, 'uor-r4.native-q31-d85');
  assert.equal(res.ok, true);
  assert.equal(res.result.model_id, 'uor-r4.native-q31-d85');

  const nullStepRes = await execute_inprism_foundry_turn(
    { id: 'req-null-step', payload: { prompt: 'Trigger null step', preferredModel: 'uor-r4.native-q31-d85' } },
    {
      ...fake.hooks,
      holoBeginTurn: () => JSON.stringify({ status: 'ready', turn_id: 88, model_id: 'uor-r4.native-q31-d85' }),
      holoStepTurn: () => null,
    },
  );
  assert.equal(nullStepRes.ok, false);
  assert.equal(nullStepRes.error_code, 'InvalidStepperOutcome');
  assert.equal(nullStepRes.turn_id, 88);
});

test('Worker BusyTurn preserves active Turn 1 ownership, handles targetId hydration cancel, and validates terminal:write', async () => {
  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevBegin = globalThis.holo_begin_foundry_turn;
  const prevStep = globalThis.holo_step_foundry_turn;
  const prevCancel = globalThis.holo_cancel_foundry_turn;
  const prevStatus = globalThis.studio_holo_runtime_status;
  const prevDispatch = globalThis.studio_dispatch;
  const prevLocation = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;

  const posted = [];
  const rpcWaiters = new Map();
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for RPC ${id}`)), 2000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
    });

  let readyResolve;
  const readyPromise = new Promise((r) => {
    readyResolve = r;
  });

  try {
    class MockWorkerGlobalScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
      postMessage(msg) {
        posted.push(msg);
        if (msg && msg.event === 'ready') readyResolve(msg);
        if (msg && msg.id != null && typeof msg.ok === 'boolean' && rpcWaiters.has(msg.id)) {
          const cb = rpcWaiters.get(msg.id);
          rpcWaiters.delete(msg.id);
          cb(msg);
        }
      }
    }
    const mockSelf = new MockWorkerGlobalScope();
    globalThis.WorkerGlobalScope = MockWorkerGlobalScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });

    const fake = createFakeStepperHooks({
      scriptForHop: () => [
        { status: 'streaming', delta: 't1 ', tokens_emitted: 4, tok_per_sec: 40 },
        { status: 'streaming', delta: 't2 ', tokens_emitted: 8, tok_per_sec: 40 },
        { status: 'done', delta: 't3', tokens_emitted: 12, tok_per_sec: 40 },
      ],
    });
    let runtimeMounted = true;
    globalThis.holo_begin_foundry_turn = fake.hooks.holoBeginTurn;
    globalThis.holo_step_foundry_turn = fake.hooks.holoStepTurn;
    globalThis.holo_cancel_foundry_turn = fake.hooks.holoCancelTurn;
    globalThis.studio_holo_runtime_status = () =>
      JSON.stringify(
        runtimeMounted
          ? {
              qwen3_06b_mounted: true,
              qwen3_17b_mounted: false,
              models: [{ model_id: 'uor-r4.qwen3-0.6b-holo' }],
            }
          : { qwen3_06b_mounted: false, qwen3_17b_mounted: false, models: [] },
      );
    globalThis.studio_dispatch = () =>
      JSON.stringify({ status: 'ok', exit_code: 0, mvm_stdout: 'ok' });

    await import('../../crates/wasm/js/worker_host_bridge.js?worker_concurrency_test=1');
    await readyPromise;

    // 1. Validate terminal:* rejects invalid empty terminal:write and accepts payload.data
    const invalidTermP = waitForRpc('term-empty');
    mockSelf.onmessage({ data: { id: 'term-empty', op: 'terminal:write', payload: { cmd: '   ' } } });
    const invalidTerm = await invalidTermP;
    assert.equal(invalidTerm.ok, false);
    assert.equal(invalidTerm.error_code, 'InvalidPayload');

    const badResizeP = waitForRpc('term-resize-bad');
    mockSelf.onmessage({
      data: { id: 'term-resize-bad', op: 'terminal:resize', payload: { cols: 0, rows: 24 } },
    });
    const badResize = await badResizeP;
    assert.equal(badResize.ok, false);
    assert.equal(badResize.error_code, 'InvalidPayload');

    const rawDataTermP = waitForRpc('term-data');
    mockSelf.onmessage({
      data: { id: 'term-data', op: 'terminal:write', payload: { data: 'ls\n' } },
    });
    const rawDataTerm = await rawDataTermP;
    assert.equal(rawDataTerm.ok, true);
    assert.equal(rawDataTerm.result.bytes, 3);

    // 2. Concurrent BusyTurn: Turn 1 streams while Turn 2 arrives and gets BusyTurn WITHOUT clobbering Turn 1
    const turn1P = waitForRpc('turn-owner-1');
    mockSelf.onmessage({
      data: {
        id: 'turn-owner-1',
        op: 'inprism_r4_turn',
        payload: { prompt: 'Turn 1 owner', preferredModel: 'uor-r4.qwen3-0.6b-holo' },
      },
    });
    await new Promise((r) => setTimeout(r, 0));

    // Flip mounted flag false to prove Turn 2 hits BusyTurn BEFORE hydration / consent guard
    runtimeMounted = false;
    const turn2P = waitForRpc('turn-busy-2');
    mockSelf.onmessage({
      data: {
        id: 'turn-busy-2',
        op: 'inprism_r4_turn',
        payload: { prompt: 'Turn 2 concurrent', preferredModel: 'uor-r4.qwen3-0.6b-holo' },
      },
    });
    const turn2 = await turn2P;
    runtimeMounted = true;
    assert.equal(turn2.ok, false);
    assert.equal(turn2.error_code, 'BusyTurn');
    assert.equal(fake.beginCalls.length, 1, 'BusyTurn must reject before invoking fnBegin a second time');

    // Cancel Turn 1 AFTER Turn 2 BusyTurn rejection: must STILL cancel Turn 1 cleanly
    const cancel1P = waitForRpc('cancel-owner-1');
    mockSelf.onmessage({
      data: { id: 'cancel-owner-1', op: 'cancel_inprism_turn', payload: { targetId: 'turn-owner-1' } },
    });
    const cancel1 = await cancel1P;
    assert.equal(cancel1.ok, true);
    assert.equal(cancel1.result.status, 'cancelled');

    const turn1 = await turn1P;
    assert.equal(turn1.ok, false);
    assert.equal(turn1.cancelled, true);
    assert.equal(turn1.error_code, 'Cancelled');

    // 3. Pre-fnBegin hydration cancellation via payload.targetId
    runtimeMounted = false;
    globalThis.location = { origin: 'http://localhost:4173', href: 'http://localhost:4173/' };
    globalThis.indexedDB = undefined;
    let releaseManifestFetch;
    globalThis.fetch = () =>
      new Promise((_, reject) => {
        releaseManifestFetch = () => reject(new Error('aborted'));
      });

    const beginCountBeforeHydrate = fake.beginCalls.length;
    const hydrateTurnP = waitForRpc('turn-hydrate-cancel');
    mockSelf.onmessage({
      data: {
        id: 'turn-hydrate-cancel',
        op: 'inprism_r4_turn',
        payload: {
          prompt: 'Cancel during hydration',
          preferredModel: 'uor-r4.qwen3-0.6b-holo',
          allowOffOriginHydration: true,
        },
      },
    });
    await new Promise((r) => setTimeout(r, 5));

    const cancelHydrateP = waitForRpc('cancel-hydrate-1');
    mockSelf.onmessage({
      data: {
        id: 'cancel-hydrate-1',
        op: 'cancel_inprism_turn',
        payload: { targetId: 'turn-hydrate-cancel' },
      },
    });
    const cancelHydrateAck = await cancelHydrateP;
    assert.equal(cancelHydrateAck.ok, true);
    assert.equal(cancelHydrateAck.result.status, 'cancelled');

    const hydrateTurnRes = await hydrateTurnP;
    assert.equal(hydrateTurnRes.ok, false);
    assert.equal(hydrateTurnRes.cancelled, true);
    assert.equal(hydrateTurnRes.error_code, 'Cancelled');
    assert.equal(
      fake.beginCalls.length,
      beginCountBeforeHydrate,
      'pre-fnBegin hydration cancel must never invoke holoBeginTurn',
    );
    if (releaseManifestFetch) releaseManifestFetch();
  } finally {
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.holo_begin_foundry_turn = prevBegin;
    globalThis.holo_step_foundry_turn = prevStep;
    globalThis.holo_cancel_foundry_turn = prevCancel;
    globalThis.studio_holo_runtime_status = prevStatus;
    globalThis.studio_dispatch = prevDispatch;
    globalThis.location = prevLocation;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
  }
});

test('Worker terminal:write and mvm_exec run bidirectional base64 /workspace sync hooks', async () => {
  const prevWgs = globalThis.WorkerGlobalScope;
  const prevSelf = globalThis.self;
  const prevWbg = globalThis.__wbg_init;
  const prevPanic = globalThis.init_panic_hook;
  const prevBoot = globalThis.studio_bootstrap;
  const prevPack = globalThis.studio_weight_pack_status;
  const prevDispatch = globalThis.studio_dispatch;
  const prevSyncIn = globalThis.mvm_sync_workspace_in_json;
  const prevExportDiff = globalThis.mvm_export_workspace_diff_json;
  const prevCowExport = globalThis.studio_export_mvm_cow_sectors;
  const prevCowImport = globalThis.studio_import_mvm_cow_sectors;

  const posted = [];
  const rpcWaiters = new Map();
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for RPC ${id}`)), 2000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
    });

  let readyResolve;
  const readyPromise = new Promise((r) => {
    readyResolve = r;
  });

  try {
    class MockWorkerGlobalScope {
      constructor() {
        this.name = 'uor-r4-engine-worker';
      }
      postMessage(msg) {
        posted.push(msg);
        if (msg && msg.event === 'ready') readyResolve(msg);
        if (msg && msg.id != null && typeof msg.ok === 'boolean' && rpcWaiters.has(msg.id)) {
          const cb = rpcWaiters.get(msg.id);
          rpcWaiters.delete(msg.id);
          cb(msg);
        }
      }
    }
    const mockSelf = new MockWorkerGlobalScope();
    globalThis.WorkerGlobalScope = MockWorkerGlobalScope;
    globalThis.self = mockSelf;
    globalThis.__wbg_init = async () => ({});
    globalThis.init_panic_hook = () => {};
    globalThis.studio_bootstrap = () => '{}';
    globalThis.studio_weight_pack_status = () => JSON.stringify({ mounted_packs: [] });
    globalThis.studio_export_mvm_cow_sectors = () => new Uint8Array([77, 86, 77]);
    globalThis.studio_import_mvm_cow_sectors = () => JSON.stringify({ status: 'ok', sectors: 0 });

    const hostFiles = [
      { path: 'src/App.tsx', language: 'typescript', content: 'export const x = 1;\n' },
      { path: 'README.md', language: 'markdown', content: '# App\n' },
    ];
    const callOrder = [];
    const syncInPayloads = [];
    let epoch = 0;

    globalThis.mvm_sync_workspace_in_json = (jsonStr) => {
      callOrder.push('sync_in');
      const req = JSON.parse(jsonStr);
      syncInPayloads.push(req);
      epoch = Number(req.epoch) || epoch;
      return JSON.stringify({
        status: 'ok',
        epoch,
        applied_writes: req.files.length,
        applied_deletes: 0,
      });
    };

    const diffQueue = [
      {
        status: 'ok',
        epoch: 1,
        writes: [
          {
            path: 'generated.txt',
            content_base64: Buffer.from('hello from mvm\n', 'utf8').toString('base64'),
          },
        ],
        deletes: [],
        ignored_heavy_paths: 2,
        cow_state_sha256: 'b'.repeat(64),
      },
      {
        status: 'ok',
        epoch: 2,
        writes: [],
        deletes: ['src/App.tsx'],
        ignored_heavy_paths: 0,
        cow_state_sha256: 'c'.repeat(64),
      },
    ];
    globalThis.mvm_export_workspace_diff_json = () => {
      callOrder.push('export_diff');
      const next = diffQueue.shift() || {
        status: 'ok',
        epoch,
        writes: [],
        deletes: [],
        ignored_heavy_paths: 0,
        cow_state_sha256: 'd'.repeat(64),
      };
      return JSON.stringify(next);
    };

    globalThis.studio_dispatch = (env) => {
      if (env.includes('\tworkspace_list\t')) {
        return JSON.stringify({ status: 'ok', files: hostFiles });
      }
      if (env.includes('\tmvm_exec\t')) {
        callOrder.push('mvm_exec');
        return JSON.stringify({ status: 'ok', exit_code: 0, mvm_stdout: 'ran\n', mvm_stderr: '' });
      }
      return JSON.stringify({ status: 'ok' });
    };

    await import('../../crates/wasm/js/worker_host_bridge.js?worker_sync_test=1');
    await readyPromise;

    const term1P = waitForRpc('sync-term-1');
    mockSelf.onmessage({
      data: {
        id: 'sync-term-1',
        op: 'terminal:write',
        payload: { cmd: 'echo hello > /workspace/generated.txt' },
      },
    });
    const term1 = await term1P;
    assert.equal(term1.ok, true);
    assert.deepEqual(callOrder, ['sync_in', 'mvm_exec', 'export_diff']);
    assert.equal(syncInPayloads[0].files.length, 2);
    assert.equal(
      Buffer.from(syncInPayloads[0].files[0].content_base64, 'base64').toString('utf8'),
      'export const x = 1;\n',
    );
    assert.equal(term1.result.ignored_heavy_paths, 2);
    assert.deepEqual(term1.result.changed_files, ['generated.txt']);

    const syncEvents1 = posted.filter((m) => m.event === 'workspace_sync');
    assert.ok(syncEvents1.some((e) => e.diff && e.diff.ignored_heavy_paths === 2));

    hostFiles.splice(1, 1);
    hostFiles.push({ path: 'generated.txt', language: 'text', content: 'hello from mvm\n' });
    const term2P = waitForRpc('sync-term-2');
    mockSelf.onmessage({
      data: {
        id: 'sync-term-2',
        op: 'terminal:write',
        payload: { cmd: 'rm /workspace/src/App.tsx' },
      },
    });
    const term2 = await term2P;
    assert.equal(term2.ok, true);
    assert.deepEqual(syncInPayloads[1].files, [{ path: 'README.md', content_base64: '', deleted: true }]);
    assert.equal(syncInPayloads[1].epoch, 1);
    assert.deepEqual(term2.result.changed_files, ['src/App.tsx']);

    const directInP = waitForRpc('direct-sync-in');
    mockSelf.onmessage({
      data: {
        id: 'direct-sync-in',
        op: 'mvm_sync_workspace_in',
        payload: { epoch: 5, files: [{ path: 'a.txt', content_base64: Buffer.from('a').toString('base64') }] },
      },
    });
    const directIn = await directInP;
    assert.equal(directIn.ok, true);
    assert.equal(directIn.result.epoch, 5);

    // Direct mvm_sync_workspace_in MUST update syncedHostWorkspaceFiles so subsequent terminal:write skips a.txt
    // Also mirror term2's deletion of src/App.tsx on hostFiles so only a.txt differs if untracked
    const appIdx = hostFiles.findIndex((f) => f.path === 'src/App.tsx');
    if (appIdx >= 0) hostFiles.splice(appIdx, 1);
    hostFiles.push({ path: 'a.txt', language: 'text', content: 'a' });
    const countAfterDirectIn = syncInPayloads.length;
    diffQueue.push({
      status: 'ok',
      epoch: 6,
      writes: [{ path: 'c.txt', content_base64: Buffer.from('c').toString('base64') }],
      deletes: [],
      ignored_heavy_paths: 0,
      cow_state_sha256: 'e'.repeat(64),
    });
    const term3P = waitForRpc('sync-term-3');
    mockSelf.onmessage({
      data: {
        id: 'sync-term-3',
        op: 'terminal:write',
        payload: { cmd: 'echo c > /workspace/c.txt' },
      },
    });
    const term3 = await term3P;
    assert.equal(term3.ok, true);
    // All host files (generated.txt, a.txt) were already synced -> term3 skips sync_in completely
    assert.equal(
      syncInPayloads.length,
      countAfterDirectIn,
      'direct mvm_sync_workspace_in must record a.txt in syncedHostWorkspaceFiles',
    );

    // Direct mvm_export_workspace_diff MUST update syncedHostWorkspaceFiles as well (c.txt & d.txt tracked)
    diffQueue.push({
      status: 'ok',
      epoch: 7,
      writes: [{ path: 'd.txt', content_base64: Buffer.from('d').toString('base64') }],
      deletes: [],
      ignored_heavy_paths: 0,
      cow_state_sha256: 'f'.repeat(64),
    });
    const directDiffP = waitForRpc('direct-export-diff');
    mockSelf.onmessage({ data: { id: 'direct-export-diff', op: 'mvm_export_workspace_diff' } });
    const directDiff = await directDiffP;
    assert.equal(directDiff.ok, true);
    assert.equal(directDiff.result.epoch, 7);

    // Even if a direct sync returns epoch 0, Worker mvmWorkspaceSyncEpoch MUST remain monotonic (>= 7)
    const directSyncZeroP = waitForRpc('direct-sync-zero');
    mockSelf.onmessage({
      data: {
        id: 'direct-sync-zero',
        op: 'mvm_sync_workspace_in',
        payload: { epoch: 0, files: [{ path: 'd.txt', content_base64: Buffer.from('d').toString('base64') }] },
      },
    });
    await directSyncZeroP;

    hostFiles.push({ path: 'd.txt', language: 'text', content: 'd' });
    hostFiles.push({ path: 'e.txt', language: 'text', content: 'e' });
    const countBeforeTerm4 = syncInPayloads.length;
    const term4P = waitForRpc('sync-term-4');
    mockSelf.onmessage({
      data: {
        id: 'sync-term-4',
        op: 'terminal:write',
        payload: { cmd: 'pwd' },
      },
    });
    const term4 = await term4P;
    assert.equal(term4.ok, true);
    assert.equal(syncInPayloads.length, countBeforeTerm4 + 1);
    const term4SyncReq = syncInPayloads[syncInPayloads.length - 1];
    assert.deepEqual(term4SyncReq.files, [
      { path: 'e.txt', content_base64: Buffer.from('e').toString('base64') },
      { path: 'c.txt', content_base64: '', deleted: true },
    ]);
    assert.ok(term4SyncReq.epoch >= 7, `Worker epoch must stay monotonic >= 7, got ${term4SyncReq.epoch}`);

    // inprism_r4_turn with mvm_exec must emit workspace_sync ONCE (no duplicate burst)
    const prevBegin = globalThis.holo_begin_foundry_turn;
    const prevStep = globalThis.holo_step_foundry_turn;
    const prevCancel = globalThis.holo_cancel_foundry_turn;
    const prevStatus = globalThis.studio_holo_runtime_status;
    try {
      const turnEventsBefore = posted.length;
      const fake = createFakeStepperHooks({
        scriptForHop: (hopIndex) =>
          hopIndex === 0
            ? [
                {
                  status: 'needs_tool',
                  delta: '',
                  tokens_emitted: 4,
                  tok_per_sec: 44,
                  tool_call: { name: 'mvm_exec', cmd: 'echo from_turn > /workspace/t.txt' },
                },
              ]
            : [{ status: 'done', delta: 'ok', tokens_emitted: 8, tok_per_sec: 52 }],
      });
      globalThis.holo_begin_foundry_turn = fake.hooks.holoBeginTurn;
      globalThis.holo_step_foundry_turn = fake.hooks.holoStepTurn;
      globalThis.holo_cancel_foundry_turn = fake.hooks.holoCancelTurn;
      globalThis.studio_holo_runtime_status = fake.hooks.holoRuntimeStatus;
      diffQueue.push({
        status: 'ok',
        epoch: 8,
        writes: [{ path: 't.txt', content_base64: Buffer.from('from_turn\n').toString('base64') }],
        deletes: [],
        ignored_heavy_paths: 0,
        cow_state_sha256: '1'.repeat(64),
      });

      const turnP = waitForRpc('sync-turn-1');
      mockSelf.onmessage({
        data: {
          id: 'sync-turn-1',
          op: 'inprism_r4_turn',
          payload: { prompt: 'Run tool', preferredModel: 'uor-r4.qwen3-0.6b-holo' },
        },
      });
      const turnRes = await turnP;
      assert.equal(turnRes.ok, true);
      assert.equal(turnRes.result.mvm_tool_executed, true);
      const turnSyncEvents = posted.slice(turnEventsBefore).filter((m) => m && m.event === 'workspace_sync');
      assert.equal(
        turnSyncEvents.length,
        1,
        `expected exactly 1 workspace_sync event during mvm_exec turn, got ${turnSyncEvents.length}`,
      );
    } finally {
      globalThis.holo_begin_foundry_turn = prevBegin;
      globalThis.holo_step_foundry_turn = prevStep;
      globalThis.holo_cancel_foundry_turn = prevCancel;
      globalThis.studio_holo_runtime_status = prevStatus;
    }
  } finally {
    globalThis.WorkerGlobalScope = prevWgs;
    globalThis.self = prevSelf;
    globalThis.__wbg_init = prevWbg;
    globalThis.init_panic_hook = prevPanic;
    globalThis.studio_bootstrap = prevBoot;
    globalThis.studio_weight_pack_status = prevPack;
    globalThis.studio_dispatch = prevDispatch;
    globalThis.mvm_sync_workspace_in_json = prevSyncIn;
    globalThis.mvm_export_workspace_diff_json = prevExportDiff;
    globalThis.studio_export_mvm_cow_sectors = prevCowExport;
    globalThis.studio_import_mvm_cow_sectors = prevCowImport;
  }
});
