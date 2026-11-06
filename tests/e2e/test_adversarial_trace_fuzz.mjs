/**
 * Challenger M4 Iteration 2: Deep Adversarial Trace Fuzzing & Resilience Harness
 *
 * Verifies that public/app.js handles all forms of trace corruption, malformed spans,
 * NaN fields, type mutations, and boundary conditions without throwing exceptions or
 * leaking NaN/Inf floats into WebGL vertex buffers.
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

console.log('================================================================');
console.log('  CHALLENGER M4.2 DEEP ADVERSARIAL TRACE INGESTION HARNESS');
console.log('================================================================\n');

const appJsPath = resolve(REPO_ROOT, 'public/app.js');
const appJsContent = readFileSync(appJsPath, 'utf8');

const startMath = appJsContent.indexOf('function mat4_create()');
const startClass = appJsContent.indexOf('class LexLean3DVisualizer');
const endClass = appJsContent.indexOf('\nlet lexLean3dVisualizer', startClass);
assert.ok(startMath !== -1 && startClass !== -1 && endClass !== -1, 'Failed to extract 3D visualizer from app.js');

const codeToEval = appJsContent.slice(startMath, endClass);

function createHeadlessVisualizer() {
  const bufferCalls = [];
  const eventListeners = {};

  const mockGl = {
    createShader: () => ({}),
    shaderSource: () => {},
    compileShader: () => {},
    getShaderParameter: () => true,
    createProgram: () => ({}),
    attachShader: () => {},
    linkProgram: () => {},
    getProgramParameter: () => true,
    createBuffer: () => ({}),
    bindBuffer: () => {},
    bufferData: (target, data) => {
      bufferCalls.push(new Float32Array(data));
    },
    enable: () => {},
    blendFunc: () => {},
    depthFunc: () => {},
    clearColor: () => {},
    clear: () => {},
    viewport: () => {},
    useProgram: () => {},
    getUniformLocation: () => ({}),
    uniformMatrix4fv: () => {},
    uniform1f: () => {},
    getAttribLocation: () => 0,
    enableVertexAttribArray: () => {},
    vertexAttribPointer: () => {},
    drawArrays: () => {},
    COLOR_BUFFER_BIT: 16384,
    DEPTH_BUFFER_BIT: 256,
    VERTEX_SHADER: 35633,
    FRAGMENT_SHADER: 35632,
    ARRAY_BUFFER: 34962,
    DYNAMIC_DRAW: 35048,
    LINES: 1,
    POINTS: 0,
    BLEND: 3042,
    DEPTH_TEST: 2929,
    SRC_ALPHA: 770,
    ONE_MINUS_SRC_ALPHA: 771,
    LEQUAL: 515,
    FLOAT: 5126
  };

  const mockCanvas = {
    getContext: () => mockGl,
    clientWidth: 800,
    clientHeight: 600,
    width: 800,
    height: 600,
    addEventListener: (evt, fn) => {
      if (!eventListeners[evt]) eventListeners[evt] = [];
      eventListeners[evt].push(fn);
    },
    setPointerCapture: () => {},
    releasePointerCapture: () => {}
  };

  const mockStatusEl = {
    textContent: ''
  };

  const factory = new Function('mockCanvas', 'mockStatusEl', `
    ${codeToEval}
    const vis = new LexLean3DVisualizer();
    vis.canvas = mockCanvas;
    vis.statusEl = mockStatusEl;
    vis.initGL();
    vis.buildStaticGeometry();
    vis.buildDefaultDecodeSpline();
    vis.bindEvents();
    return vis;
  `);

  const vis = factory(mockCanvas, mockStatusEl);
  return { vis, bufferCalls, mockGl, mockCanvas };
}

function verifyNoNaNInBufferCalls(bufferCalls, contextMsg) {
  let nanCount = 0;
  let infCount = 0;
  let totalFloats = 0;

  for (const buf of bufferCalls) {
    totalFloats += buf.length;
    for (let i = 0; i < buf.length; i++) {
      if (Number.isNaN(buf[i])) nanCount++;
      if (!Number.isFinite(buf[i])) infCount++;
    }
  }

  assert.equal(nanCount, 0, `Detected ${nanCount} NaN floats in WebGL buffers during: ${contextMsg}`);
  assert.equal(infCount, 0, `Detected ${infCount} non-finite floats in WebGL buffers during: ${contextMsg}`);
  return totalFloats;
}

const discoveredDefects = [];

// ============================================================================
// TEST 1: DEFECT-1 Null / Undefined / Non-Object Spans & Malformed Attributes
// ============================================================================
console.log('--- Test 1: Spans & Attributes Defensive Boundary Testing ---');
{
  const { vis, bufferCalls } = createHeadlessVisualizer();

  const corruptSpansPayloads = [
    { desc: 'null trace', trace: null },
    { desc: 'undefined trace', trace: undefined },
    { desc: 'primitive number trace', trace: 42 },
    { desc: 'primitive string trace', trace: 'not a trace' },
    { desc: 'empty object trace', trace: {} },
    { desc: 'trace with non-array spans', trace: { spans: 'invalid' } },
    { desc: 'trace with null spans', trace: { spans: null } },
    { desc: 'trace with spans array containing nulls and primitives',
      trace: { spans: [null, undefined, 0, false, true, '', 'string_span', 42, [], [1, 2]] } },
    { desc: 'spans with null attributes',
      trace: { spans: [{ name_ref: 'gllm.trunk.layer.l0', attributes: null }] } },
    { desc: 'spans with non-array attributes',
      trace: { spans: [{ name_ref: 'gllm.trunk.layer.l0', attributes: 12345 }] } },
    { desc: 'spans with malformed attribute entries',
      trace: { spans: [{ name_ref: 'gllm.trunk.layer.l0', attributes: [null, undefined, 0, 'bad', [], [1], ['key_only']] }] } },
  ];

  for (const { desc, trace } of corruptSpansPayloads) {
    try {
      vis.loadTrace(trace);
      bufferCalls.length = 0;
      vis.render(0.016);
      const floats = verifyNoNaNInBufferCalls(bufferCalls, desc);
      assert.ok(floats > 0, `Expected WebGL floats rendered for ${desc}`);
    } catch (e) {
      discoveredDefects.push({
        id: 'DEFECT-FUZZ-1',
        title: `Crash on ${desc}: ${e.message}`,
        severity: 'HIGH'
      });
      console.log(`  ✗ FAILED: ${desc} threw ${e.message}`);
    }
  }

  if (discoveredDefects.length === 0) {
    console.log('  ✓ 1.1 All corrupt span & attribute structures handled gracefully with 0 crashes.');
  }
}

// ============================================================================
// TEST 2: DEFECT-2 Winning PointKey NaN & Extreme Boundary Ingestion
// ============================================================================
console.log('\n--- Test 2: Winning PointKey NaN & Extreme Boundary Testing ---');
{
  const { vis, bufferCalls } = createHeadlessVisualizer();

  const pointKeyCases = [
    { desc: 'winning_row as NaN', val: NaN },
    { desc: 'winning_row as non-numeric string', val: 'corrupted_string' },
    { desc: 'winning_row as Infinity', val: Infinity },
    { desc: 'winning_row as -Infinity', val: -Infinity },
    { desc: 'winning_row as huge float (+1e12)', val: 1e12 },
    { desc: 'winning_row as negative huge float (-1e12)', val: -1e12 },
    { desc: 'winning_row as null', val: null },
    { desc: 'winning_row as undefined', val: undefined },
    { desc: 'winning_row as object', val: {} },
    { desc: 'winning_row as array', val: [12] },
    { desc: 'winning_row as valid integer 0', val: 0 },
    { desc: 'winning_row as valid integer 255', val: 255 },
    { desc: 'winning_row as valid integer 99999', val: 99999 },
  ];

  for (const { desc, val } of pointKeyCases) {
    try {
      vis.loadTrace({
        spans: [
          {
            name_ref: 'gllm.routing.point_key',
            attributes: [['uor.winning_row', val]]
          }
        ]
      });

      bufferCalls.length = 0;
      vis.render(0.033);
      const floats = verifyNoNaNInBufferCalls(bufferCalls, desc);
      assert.ok(floats > 0, `Expected WebGL floats rendered for ${desc}`);
    } catch (e) {
      discoveredDefects.push({
        id: 'DEFECT-FUZZ-2',
        title: `Crash on ${desc}: ${e.message}`,
        severity: 'HIGH'
      });
      console.log(`  ✗ FAILED: ${desc} threw ${e.message}`);
    }
  }

  if (!discoveredDefects.some(d => d.id === 'DEFECT-FUZZ-2')) {
    console.log('  ✓ 2.1 All winning_row edge cases handled with zero animation loop crashes and zero buffer NaNs.');
  }
}

// ============================================================================
// TEST 3: DEFECT-3 Basin Energy, Clifford Grades & Icosian Root Robustness
// ============================================================================
console.log('\n--- Test 3: Basin Energy, Clifford Grades & Icosian Root Robustness ---');
{
  const { vis, bufferCalls } = createHeadlessVisualizer();

  const energyCases = [
    { desc: 'basin_energy_l1 as NaN', val: NaN },
    { desc: 'basin_energy_l1 as string NaN', val: 'NaN' },
    { desc: 'basin_energy_l1 as corrupted string', val: 'corrupted_energy' },
    { desc: 'basin_energy_l1 as +Infinity', val: Infinity },
    { desc: 'basin_energy_l1 as -Infinity', val: -Infinity },
    { desc: 'basin_energy_l1 as null', val: null },
    { desc: 'basin_energy_l1 as undefined', val: undefined },
    { desc: 'basin_energy_l1 as 0', val: 0 },
    { desc: 'basin_energy_l1 as extreme 1e12', val: 1e12 },
    { desc: 'basin_energy_l1 as negative -500', val: -500 },
  ];

  for (const { desc, val } of energyCases) {
    try {
      vis.loadTrace({
        spans: [
          {
            name_ref: 'gllm.trunk.layer.l0',
            attributes: [
              ['uor.layer_index', 0],
              ['uor.basin_energy_l1', val]
            ]
          }
        ]
      });

      bufferCalls.length = 0;
      vis.render(0.05);
      const floats = verifyNoNaNInBufferCalls(bufferCalls, desc);
      assert.ok(floats > 0, `Expected WebGL floats rendered for ${desc}`);
    } catch (e) {
      discoveredDefects.push({
        id: 'DEFECT-FUZZ-3',
        title: `Crash on ${desc}: ${e.message}`,
        severity: 'HIGH'
      });
      console.log(`  ✗ FAILED: ${desc} threw ${e.message}`);
    }
  }

  // Fuzz Clifford grades & Icosian roots
  const cliffordAndIcosianCases = [
    {
      desc: 'Clifford grades with NaN / Inf / strings / null',
      spans: [{
        name_ref: 'gllm.trunk.layer.l1',
        attributes: [
          ['uor.layer_index', 1],
          ['uor.clifford_grades', [NaN, Infinity, -Infinity, 'corrupted', null, undefined, 42, 999]],
          ['uor.dominant_icosian_root', NaN]
        ]
      }]
    },
    {
      desc: 'Dominant icosian root as extreme / negative / string',
      spans: [{
        name_ref: 'gllm.trunk.layer.l2',
        attributes: [
          ['uor.layer_index', 2],
          ['uor.dominant_icosian_root', -999999]
        ]
      }, {
        name_ref: 'gllm.trunk.layer.l3',
        attributes: [
          ['uor.layer_index', 3],
          ['uor.dominant_icosian_root', 'not_a_root']
        ]
      }, {
        name_ref: 'gllm.trunk.layer.l4',
        attributes: [
          ['uor.layer_index', 4],
          ['uor.dominant_icosian_root', Infinity]
        ]
      }]
    }
  ];

  for (const { desc, spans } of cliffordAndIcosianCases) {
    try {
      vis.loadTrace({ spans });
      bufferCalls.length = 0;
      vis.render(0.06);
      const floats = verifyNoNaNInBufferCalls(bufferCalls, desc);
      assert.ok(floats > 0, `Expected WebGL floats rendered for ${desc}`);
    } catch (e) {
      discoveredDefects.push({
        id: 'DEFECT-FUZZ-3',
        title: `Crash on ${desc}: ${e.message}`,
        severity: 'HIGH'
      });
      console.log(`  ✗ FAILED: ${desc} threw ${e.message}`);
    }
  }

  if (!discoveredDefects.some(d => d.id === 'DEFECT-FUZZ-3')) {
    console.log('  ✓ 3.1 All basin energy, Clifford grades & Icosian roots handled with zero buffer NaNs.');
  }
}

// ============================================================================
// TEST 4: Extended Decode Waypoint & Spline Ingestion Fuzzing
// ============================================================================
console.log('\n--- Test 4: Extended Decode Waypoint & Spline Ingestion Fuzzing ---');
{
  const { vis, bufferCalls } = createHeadlessVisualizer();

  const decodeCases = [
    {
      desc: 'Single decode step (count = 1)',
      spans: [{ name_ref: 'gllm.decode.step', attributes: [['uor.step', 0], ['uor.predicted_token', 101]] }]
    },
    {
      desc: 'Decode steps with corrupted step numbers (NaN, Inf, negative, strings)',
      spans: [
        { name_ref: 'gllm.decode.step', attributes: [['uor.step', NaN], ['uor.predicted_token', 'token_str']] },
        { name_ref: 'gllm.decode.step', attributes: [['uor.step', -50], ['uor.predicted_token', null]] },
        { name_ref: 'gllm.decode.step', attributes: [['uor.step', 'not_step'], ['uor.predicted_token', 103]] },
      ]
    },
    {
      desc: '500 rapid decode steps',
      spans: Array.from({ length: 500 }, (_, i) => ({
        name_ref: 'gllm.decode.step',
        attributes: [['uor.step', i], ['uor.predicted_token', (i * 37) % 50000]]
      }))
    }
  ];

  for (const { desc, spans } of decodeCases) {
    try {
      vis.loadTrace({ spans });
      bufferCalls.length = 0;
      vis.render(0.1);
      const floats = verifyNoNaNInBufferCalls(bufferCalls, desc);
      assert.ok(floats > 0, `Expected WebGL floats rendered for ${desc}`);
    } catch (e) {
      discoveredDefects.push({
        id: 'DEFECT-FUZZ-4',
        title: `Crash on ${desc}: ${e.message}`,
        severity: 'HIGH'
      });
      console.log(`  ✗ FAILED: ${desc} threw ${e.message}`);
    }
  }

  if (!discoveredDefects.some(d => d.id === 'DEFECT-FUZZ-4')) {
    console.log('  ✓ 4.1 Decode waypoint spline rebuilding passed 1..500 step ranges without NaN leakage.');
  }
}

// ============================================================================
// TEST 5: Long-Duration Animation Loop Stress (1,000 Continuous Render Frames)
// ============================================================================
console.log('\n--- Test 5: Long-Duration Continuous Render Loop Stress (1,000 Frames) ---');
{
  const { vis, bufferCalls } = createHeadlessVisualizer();

  // Load a rich multi-tier trace
  vis.loadTrace({
    trace_id: '1234567890abcdef',
    turn_index: 3,
    spans: [
      { name_ref: 'gllm.prompt.tokenize', attributes: [['uor.token_count', 24]] },
      { name_ref: 'gllm.trunk.layer.l0', attributes: [['uor.layer_index', 0], ['uor.basin_energy_l1', 750]] },
      { name_ref: 'gllm.trunk.layer.l1', attributes: [['uor.layer_index', 1], ['uor.basin_energy_l1', 480]] },
      { name_ref: 'gllm.trunk.layer.l2', attributes: [['uor.layer_index', 2], ['uor.basin_energy_l1', 620]] },
      { name_ref: 'gllm.trunk.layer.l3', attributes: [['uor.layer_index', 3], ['uor.basin_energy_l1', 890]] },
      { name_ref: 'gllm.trunk.layer.l4', attributes: [['uor.layer_index', 4], ['uor.basin_energy_l1', 310]] },
      { name_ref: 'gllm.trunk.layer.l5', attributes: [['uor.layer_index', 5], ['uor.basin_energy_l1', 940]] },
      { name_ref: 'gllm.routing.point_key', attributes: [['uor.winning_row', 42]] },
      { name_ref: 'gllm.decode.step', attributes: [['uor.step', 0], ['uor.predicted_token', 101]] },
      { name_ref: 'gllm.decode.step', attributes: [['uor.step', 1], ['uor.predicted_token', 204]] },
      { name_ref: 'gllm.decode.step', attributes: [['uor.step', 2], ['uor.predicted_token', 309]] }
    ]
  });

  let totalFloatsChecked = 0;
  for (let frame = 0; frame < 1000; frame++) {
    bufferCalls.length = 0;
    const timeSec = frame * 0.016;
    vis.render(timeSec);
    totalFloatsChecked += verifyNoNaNInBufferCalls(bufferCalls, `Frame ${frame}`);
  }

  console.log(`  ✓ 5.1 Rendered 1,000 frames successfully. Verified ${totalFloatsChecked.toLocaleString()} vertex floats with 0 NaN/Inf.`);
}

// ============================================================================
// FINAL REPORT
// ============================================================================
console.log('\n================================================================');
console.log(`  ADVERSARIAL FUZZING COMPLETE: ${discoveredDefects.length} DEFECTS FOUND`);
console.log('================================================================');

for (const d of discoveredDefects) {
  console.log(`- [${d.id}] ${d.title} (${d.severity})`);
}

if (discoveredDefects.length > 0) {
  process.exit(1);
} else {
  console.log('ALL DEFECT REMEDIATIONS CONFIRMED SOUND & RESILIENT.\n');
  process.exit(0);
}
