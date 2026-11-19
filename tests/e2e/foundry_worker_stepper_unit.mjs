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

test('hydrate_holo_model and inprism_r4_turn auto-hydrate first-party catalog and enforce consent for explicit opt-out / untrusted mirrors', async () => {
  const prevLocation = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;
  const prevLsDesc = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  let offOriginFetchCount = 0;
  let untrustedFetchCount = 0;

  try {
    globalThis.location = { origin: 'http://localhost:4173', href: 'http://localhost:4173/' };
    globalThis.indexedDB = undefined;
    delete globalThis.localStorage;
    globalThis.fetch = async (url) => {
      const target = String(url || '');
      if (target.includes('untrusted.example')) {
        untrustedFetchCount += 1;
        throw new Error(`Untrusted mirror fetch forbidden without consent: ${target}`);
      }
      if (!target.startsWith('http://localhost:4173/')) {
        offOriginFetchCount += 1;
      }
      throw new Error('weight mirror offline');
    };

    // 1. Explicit allowOffOriginHydration: false blocks off-origin fetches with ConsentRequiredForWeightDownload
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

    // 2. Untrusted custom external mirrors still require explicit consent
    await assert.rejects(
      () =>
        hydrate_holo_model('uor-r4.qwen3-0.6b-holo', () => {}, false, {
          customMirrors: ['https://untrusted.example/weights/'],
        }),
      (err) =>
        err &&
        (err.code === 'ConsentRequiredForWeightDownload' ||
          err.error_code === 'ConsentRequiredForWeightDownload') &&
        Array.isArray(err.mirrors) &&
        err.mirrors.some((m) => m.includes('untrusted.example')),
    );
    assert.equal(untrustedFetchCount, 0);

    // 3. Default first-party catalog hydration automatically attempts first-party mirrors without consent error
    const fake = createFakeStepperHooks({ mounted: false });
    const res = await execute_inprism_foundry_turn(
      { id: 'req-auto-1', payload: { prompt: 'Build counter app' } },
      fake.hooks,
    );
    assert.equal(res.ok, false);
    assert.notEqual(
      res.error_code,
      'ConsentRequiredForWeightDownload',
      `First-party catalog turn must not fail with ConsentRequiredForWeightDownload, got: ${JSON.stringify(res)}`,
    );
    assert.ok(offOriginFetchCount > 0, 'Default first-party catalog turn must auto-attempt first-party mirror');
  } finally {
    globalThis.location = prevLocation;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
    if (prevLsDesc) Object.defineProperty(globalThis, 'localStorage', prevLsDesc);
    else delete globalThis.localStorage;
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

test('execute_inprism_foundry_turn streams observatory_frames across holo_trace_frame, holo_observatory_batch, chat_chunk, and final result', async () => {
  const events = [];
  const prefillFrame = {
    step_idx: 0,
    phase: 'prefill',
    seq_pos: 12,
    prefix_reused_tokens: 7,
    token_id: 10,
    token_text: 'hi',
    layer_residual_mag: [100],
    layer_max_bits: [12],
    layer_flock_spans: [4],
    final_attn_peaks: [],
    hamming_hist_32: new Array(32).fill(0),
    min_hamming: 18,
    q_manifold_xyz: [10, 20, 30],
    backend: 'wasm-simd128',
  };
  const decodeFrame1 = { ...prefillFrame, step_idx: 1, phase: 'decode', seq_pos: 13, token_id: 261, token_text: 'A' };
  const decodeFrame2 = { ...prefillFrame, step_idx: 5, phase: 'decode', seq_pos: 17, token_id: 262, token_text: 'B' };

  const fake = createFakeStepperHooks({
    scriptForHop: () => [
      {
        status: 'streaming',
        delta: 'Part1 ',
        tokens_emitted: 4,
        tok_per_sec: 48.5,
        prefix_reused_tokens: 7,
        observatory_frames: [prefillFrame, decodeFrame1],
        frame: decodeFrame1,
      },
      {
        status: 'done',
        delta: 'Part2',
        tokens_emitted: 6,
        tok_per_sec: 52.0,
        prefix_reused_tokens: 7,
        observatory_frames: [decodeFrame2],
        frame: decodeFrame2,
      },
    ],
  });

  const envelope = await execute_inprism_foundry_turn(
    {
      id: 'req-obs-stream-1',
      payload: { prompt: 'Stream frames', preferredModel: 'uor-r4.qwen3-0.6b-holo' },
    },
    {
      ...fake.hooks,
      holoBeginTurn: (payloadJson) => {
        const res = JSON.parse(fake.hooks.holoBeginTurn(payloadJson));
        res.prefix_reused_tokens = 7;
        return JSON.stringify(res);
      },
      postEvent: (event, data) => events.push({ event, data }),
    },
  );

  assert.equal(envelope.ok, true);
  assert.equal(envelope.result.prefix_reused_tokens, 7);
  assert.equal(envelope.result.frames?.length, 3);
  assert.equal(envelope.result.holo_observatory_frames?.length, 3);
  assert.deepEqual(envelope.result.frame, decodeFrame2);

  const traceEvents = events.filter((e) => e.event === 'holo_trace_frame');
  assert.equal(traceEvents.length, 3, 'every step observatory frame must emit holo_trace_frame');
  const batchEvents = events.filter((e) => e.event === 'holo_observatory_batch');
  assert.equal(batchEvents.length, 2, 'each step with frames must emit holo_observatory_batch');
  assert.equal(batchEvents[0].data.prefixReused, 7);

  const chunks = events.filter((e) => e.event === 'chat_chunk');
  assert.equal(chunks.length, 2);
  assert.equal(chunks[0].data.prefix_reused_tokens, 7);
  assert.deepEqual(chunks[0].data.frame, decodeFrame1);
  assert.equal(chunks[0].data.frames?.length, 2);
  assert.deepEqual(chunks[1].data.frame, decodeFrame2);
  assert.equal(chunks[1].data.frames?.length, 1);
});

test('Worker rejects concurrent null-ID inprism_r4_turn during async hydration with BusyTurn and normalizes ./ canonical paths', async () => {
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
  const prevSyncIn = globalThis.mvm_sync_workspace_in_json;
  const prevExportDiff = globalThis.mvm_export_workspace_diff_json;
  const prevLocation = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;

  const posted = [];
  const rpcWaiters = new Map();
  let readyResolve;
  let releaseHydration;
  const readyPromise = new Promise((r) => {
    readyResolve = r;
  });
  const waitForRpc = (id) =>
    new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error(`Timeout waiting for RPC ${id}`)), 2000);
      rpcWaiters.set(id, (msg) => {
        clearTimeout(t);
        resolve(msg);
      });
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

    let mounted = false;
    let markHydrationStarted;
    const hydrationStarted = new Promise((resolve) => {
      markHydrationStarted = resolve;
    });

    const syncInPayloads = [];
    let hostFiles = [{ path: 'src/App.tsx', language: 'tsx', content: 'v1' }];
    globalThis.studio_dispatch = (env) => {
      if (env.includes('\tworkspace_list\t')) {
        return JSON.stringify({ files: hostFiles });
      }
      return JSON.stringify({ status: 'ok', exit_code: 0, mvm_stdout: 'ok' });
    };
    globalThis.mvm_sync_workspace_in_json = (raw) => {
      syncInPayloads.push(JSON.parse(raw));
      return JSON.stringify({ status: 'ok', epoch: 1, applied_writes: 1, applied_deletes: 0 });
    };
    globalThis.mvm_export_workspace_diff_json = () =>
      JSON.stringify({ status: 'ok', epoch: 1, writes: [], deletes: [], ignored_heavy_paths: 0 });

    globalThis.studio_holo_runtime_status = () =>
      JSON.stringify({
        qwen3_06b_mounted: mounted,
        qwen3_17b_mounted: false,
        models: mounted ? [{ model_id: 'uor-r4.qwen3-0.6b-holo' }] : [],
      });
    globalThis.holo_begin_foundry_turn = () =>
      JSON.stringify({ status: 'ready', turn_id: 1, model_id: 'uor-r4.qwen3-0.6b-holo' });
    globalThis.holo_step_foundry_turn = () =>
      JSON.stringify({ status: 'done', turn_id: 1, delta: 'ok', tokens_emitted: 1, tok_per_sec: 10 });
    globalThis.holo_cancel_foundry_turn = () => JSON.stringify({ status: 'cancelled', turn_id: 1 });

    await import('../../crates/wasm/js/worker_host_bridge.js?worker_null_id_busy=1');
    await readyPromise;

    globalThis.location = { origin: 'http://localhost:4173', href: 'http://localhost:4173/' };
    globalThis.indexedDB = undefined;
    globalThis.fetch = () =>
      new Promise((_, reject) => {
        markHydrationStarted();
        releaseHydration = () => reject(new Error('offline mirror'));
      });

    // 1. Two concurrent null-ID inprism_r4_turn calls during pending hydrate_holo_model
    const startIdx = posted.length;
    mockSelf.onmessage({
      data: {
        id: null,
        op: 'inprism_r4_turn',
        payload: {
          prompt: 'First null-id turn',
          preferredModel: 'uor-r4.qwen3-0.6b-holo',
          allowOffOriginHydration: true,
        },
      },
    });
    await hydrationStarted;
    mockSelf.onmessage({
      data: {
        id: null,
        op: 'inprism_r4_turn',
        payload: {
          prompt: 'Concurrent null-id turn must get BusyTurn',
          preferredModel: 'uor-r4.qwen3-0.6b-holo',
          allowOffOriginHydration: true,
        },
      },
    });
    await new Promise((r) => setTimeout(r, 20));
    const nullBusyMsg = posted
      .slice(startIdx)
      .find((m) => m && m.id === null && m.ok === false && m.error_code === 'BusyTurn');
    assert.ok(
      nullBusyMsg,
      `concurrent null-ID turn during hydration must reject with BusyTurn, got: ${JSON.stringify(posted.slice(startIdx))}`,
    );
    if (releaseHydration) releaseHydration();
    await new Promise((r) => setTimeout(r, 20));

    // 2. ./ prefix normalization in applyMvmSyncInState for canonical prism/*.lex.tex, app/state, and ./src/App.tsx
    const syncRpc = waitForRpc('sync-dot-slash');
    mockSelf.onmessage({
      data: {
        id: 'sync-dot-slash',
        op: 'mvm_sync_workspace_in',
        payload: {
          epoch: 2,
          files: [
            { path: './prism/Rule.lex.tex', content_base64: Buffer.from('bad').toString('base64') },
            { path: './app/state', content_base64: Buffer.from('bad').toString('base64') },
            { path: './src//App.tsx', content_base64: Buffer.from('v1').toString('base64') },
          ],
        },
      },
    });
    await syncRpc;
    const beforeTerm = syncInPayloads.length;
    const termCheck = waitForRpc('term-after-dot-slash');
    mockSelf.onmessage({ data: { id: 'term-after-dot-slash', op: 'terminal:write', payload: { cmd: 'pwd' } } });
    await termCheck;
    const pushedFiles = syncInPayloads.slice(beforeTerm).flatMap((p) => p.files || []);
    assert.equal(
      pushedFiles.length,
      0,
      `./prism/Rule.lex.tex and ./app/state must be skipped and ./src//App.tsx normalized to src/App.tsx, got: ${JSON.stringify(pushedFiles)}`,
    );
  } finally {
    if (releaseHydration) releaseHydration();
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
    globalThis.mvm_sync_workspace_in_json = prevSyncIn;
    globalThis.mvm_export_workspace_diff_json = prevExportDiff;
    globalThis.location = prevLocation;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
  }
});

test('R4-62: CONCURRENCY = 3, 3-attempt shard retry, no-cache manifest + IDB overwrite on drift, and zero silent Q31 downgrade', async () => {
  const crypto = await import('node:crypto');
  const prevLoc = globalThis.location;
  const prevFetch = globalThis.fetch;
  const prevIdb = globalThis.indexedDB;
  const prevLsDesc = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const prevRetryDelay = globalThis.__uorHoloRetryBaseDelayMs;
  const prevBeginV = globalThis.studio_begin_holo_v13_mount_verified;
  const prevBegin = globalThis.studio_begin_holo_v13_mount;
  const prevPush = globalThis.studio_push_holo_v13_piece;
  const prevFin = globalThis.studio_finalize_holo_v13_mount;

  try {
    globalThis.location = { origin: 'https://casey-allard.github.io', href: 'https://casey-allard.github.io/lexlean-r4/' };
    globalThis.__uorHoloRetryBaseDelayMs = 1;

    // 7 pieces of 64 bytes each to verify max in-flight batch concurrency <= 3 and retry on piece_0001
    const pieceBytes = Array.from({ length: 7 }, (_, i) => new Uint8Array(64).fill(i + 1));
    const pieceMetas = pieceBytes.map((buf, idx) => ({
      index: idx,
      file: `piece_${String(idx).padStart(4, '0')}.bin`,
      byte_offset: idx * 64,
      byte_length: buf.byteLength,
      sha256: crypto.createHash('sha256').update(buf).digest('hex'),
    }));
    const staleSha = '0'.repeat(64);
    const freshSha = 'f'.repeat(64);
    const freshManifest = {
      format: 'UORR4V13',
      container_sha256: freshSha,
      total_bytes: 7 * 64,
      pieces: pieceMetas,
    };

    // Mock IndexedDB pre-seeded with stale manifest container_sha256
    const idbRecords = new Map([
      [
        'uor-r4.qwen3-0.6b-holo',
        { model_id: 'uor-r4.qwen3-0.6b-holo', container_sha256: staleSha, manifest: { format: 'UORR4V13', container_sha256: staleSha, pieces: [] } },
      ],
    ]);
    let deletedBeforeWrite = false;
    globalThis.indexedDB = {
      open: () => {
        const req = { result: null, onupgradeneeded: null, onsuccess: null, onerror: null };
        queueMicrotask(() => {
          const fakeDb = {
            objectStoreNames: { contains: () => true },
            createObjectStore: () => ({}),
            close: () => {},
            transaction: () => {
              const tx = {
                oncomplete: null,
                onerror: null,
                onabort: null,
                objectStore: () => ({
                  get: (k) => {
                    const r = { result: idbRecords.get(k) || null, onsuccess: null, onerror: null };
                    queueMicrotask(() => { if (r.onsuccess) r.onsuccess(); });
                    return r;
                  },
                  put: (rec) => {
                    idbRecords.set(rec.model_id, rec);
                    queueMicrotask(() => { if (tx.oncomplete) tx.oncomplete(); });
                  },
                  delete: (k) => {
                    if (idbRecords.has(k)) deletedBeforeWrite = true;
                    idbRecords.delete(k);
                    queueMicrotask(() => { if (tx.oncomplete) tx.oncomplete(); });
                  },
                }),
              };
              return tx;
            },
          };
          req.result = fakeDb;
          if (req.onsuccess) req.onsuccess();
        });
        return req;
      },
    };

    let manifestCacheInit = null;
    let inFlightShards = 0;
    let maxInFlightShards = 0;
    let piece1Attempts = 0;
    let pushedCount = 0;
    globalThis.fetch = async (url, init) => {
      const s = String(url || '');
      if (s.includes('https://casey-allard.github.io/') && s.endsWith('/manifest.json')) {
        manifestCacheInit = init && init.cache;
        return new Response(JSON.stringify(freshManifest), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        });
      }
      const match = s.match(/piece_(\d+)\.bin$/);
      if (match && s.includes('https://casey-allard.github.io/')) {
        const idx = Number(match[1]);
        inFlightShards += 1;
        if (inFlightShards > maxInFlightShards) maxInFlightShards = inFlightShards;
        try {
          await new Promise((r) => setTimeout(r, 8));
          if (idx === 1) {
            piece1Attempts += 1;
            if (piece1Attempts === 1) return new Response('Service Unavailable', { status: 503 });
            if (piece1Attempts === 2) return new Response('Too Many Requests', { status: 429 });
          }
          return new Response(pieceBytes[idx], { status: 200 });
        } finally {
          inFlightShards -= 1;
        }
      }
      return new Response('Not Found', { status: 404 });
    };

    globalThis.studio_begin_holo_v13_mount_verified = () => JSON.stringify({ status: 'ok' });
    globalThis.studio_begin_holo_v13_mount = () => JSON.stringify({ status: 'ok' });
    globalThis.studio_push_holo_v13_piece = () => {
      pushedCount += 1;
      return JSON.stringify({ status: 'ok' });
    };
    globalThis.studio_finalize_holo_v13_mount = () =>
      JSON.stringify({ status: 'ok', model_id: 'uor-r4.qwen3-0.6b-holo', mounted: true });

    const progressEvents = [];
    const hydrated = await hydrate_holo_model(
      'uor-r4.qwen3-0.6b-holo',
      (p) => progressEvents.push(p),
      false,
      {},
    );
    assert.equal(hydrated.status, 'ok');
    assert.equal(manifestCacheInit, 'no-cache', 'manifest.json fetch must set cache: no-cache');
    assert.equal(pushedCount, 7);
    assert.ok(maxInFlightShards > 0 && maxInFlightShards <= 3, `Expected max in-flight pieces <= 3, got ${maxInFlightShards}`);
    assert.equal(piece1Attempts, 3, 'Transient 503/429 on piece_0001.bin must retry up to 3 attempts');
    assert.ok(deletedBeforeWrite, 'Stale IndexedDB manifest must be invalidated/overwritten on container_sha256 drift');
    assert.equal(idbRecords.get('uor-r4.qwen3-0.6b-holo').container_sha256, freshSha);
    const lastProg = progressEvents[progressEvents.length - 1];
    assert.equal(lastProg.downloadedBytes, 7 * 64);
    assert.equal(lastProg.totalBytes, 7 * 64);
    assert.equal(lastProg.percent, 100);

    // Verify stale localStorage 'declined' does NOT silently downgrade explicit 0.6B turn to Q31
    const fakeStore = new Map([['uor_allow_off_origin_hydration', 'declined']]);
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: {
        getItem: (k) => (fakeStore.has(k) ? fakeStore.get(k) : null),
        setItem: (k, v) => fakeStore.set(k, String(v)),
        removeItem: (k) => fakeStore.delete(k),
      },
    });
    globalThis.fetch = async () => new Response('Not Found', { status: 404 });
    idbRecords.clear();
    const fake = createFakeStepperHooks({ mounted: false });
    const noDowngradeRes = await execute_inprism_foundry_turn(
      { id: 'req-no-downgrade', payload: { prompt: 'Explicit 0.6B test', preferredModel: 'uor-r4.qwen3-0.6b-holo' } },
      fake.hooks,
    );
    assert.equal(noDowngradeRes.ok, false);
    assert.notEqual(
      noDowngradeRes.result && noDowngradeRes.result.model_id,
      'uor-r4.native-q31-d85',
      'Stale uor_allow_off_origin_hydration=declined must never silently downgrade explicit Qwen3 turn to Q31',
    );
    assert.equal(fakeStore.has('uor_allow_off_origin_hydration'), false, 'Stale declined flag must be purged');

    // Explicit offlineEmergencyQ31=1 in Settings routes unadorned turns cleanly to Q31 without hydration
    fakeStore.set('uor_offline_emergency_q31', '1');
    const emergencyBeginCalls = [];
    const emergencyRes = await execute_inprism_foundry_turn(
      { id: 'req-emergency-q31', payload: { prompt: 'Offline emergency Q31 test' } },
      {
        ...fake.hooks,
        holoBeginTurn: (payloadJson) => {
          const req = JSON.parse(payloadJson);
          emergencyBeginCalls.push(req);
          return JSON.stringify({
            status: 'ready',
            turn_id: 77,
            model_id: req.selected_model || 'uor-r4.native-q31-d85',
          });
        },
        holoStepTurn: () => JSON.stringify({ status: 'done', turn_id: 77, delta: 'q31 ok', tokens_emitted: 1, tok_per_sec: 12 }),
      },
    );
    assert.equal(emergencyRes.ok, true);
    assert.equal(emergencyBeginCalls[0].selected_model, 'uor-r4.native-q31-d85');
    assert.equal(emergencyRes.result.model_id, 'uor-r4.native-q31-d85');
  } finally {
    globalThis.location = prevLoc;
    globalThis.fetch = prevFetch;
    globalThis.indexedDB = prevIdb;
    if (prevLsDesc) Object.defineProperty(globalThis, 'localStorage', prevLsDesc);
    else delete globalThis.localStorage;
    if (prevRetryDelay === undefined) delete globalThis.__uorHoloRetryBaseDelayMs;
    else globalThis.__uorHoloRetryBaseDelayMs = prevRetryDelay;
    globalThis.studio_begin_holo_v13_mount_verified = prevBeginV;
    globalThis.studio_begin_holo_v13_mount = prevBegin;
    globalThis.studio_push_holo_v13_piece = prevPush;
    globalThis.studio_finalize_holo_v13_mount = prevFin;
  }
});
