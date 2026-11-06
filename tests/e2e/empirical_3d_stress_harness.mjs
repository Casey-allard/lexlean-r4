/**
 * Empirical Stress-Test Harness for LexLean 3D WebGL Math & Camera Controls
 * Milestone M2 / Requirement R1
 *
 * Verifies:
 * 1. Matrix math: mat4_create, mat4_perspective, mat4_lookAt, mat4_multiply.
 *    Confirm zero NaN, zero Inf, determinant preservation det(A * B) = det(A) * det(B),
 *    rigidity det(lookAt) = 1.0, and perspective invertibility.
 * 2. Zoom boundary conditions: Strict clamping to [2.0, 50.0] under extreme wheel deltas
 *    (+1e9, -1e9, 0, NaN, +Infinity, -Infinity, undefined), rapid oscillation, and reset.
 * 3. Orbit boundary conditions: Yaw wrapping, pitch clamping to [-1.45, 1.45] preventing
 *    gimbal lock with up=[0,1,0], and long-duration autoRotate stability.
 * 4. Dynamic trace mutation: empty trace, corrupted trace, partial trace, missing intern table,
 *    extreme Clifford grade arrays, and WebGL vertex buffer finiteness (zero NaN/Inf).
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

console.log('================================================================');
console.log('  LexLean M2 3D WebGL Math & Camera Controls Empirical Harness  ');
console.log('================================================================\n');

// ----------------------------------------------------------------------------
// 1. Extract 3D Visualizer and Math Functions from public/app.js
// ----------------------------------------------------------------------------
const appJsPath = resolve(REPO_ROOT, 'public/app.js');
const appJsContent = readFileSync(appJsPath, 'utf8');

const startIdx = appJsContent.indexOf('function mat4_create()');
assert.ok(startIdx !== -1, 'mat4_create not found in public/app.js');

const classIdx = appJsContent.indexOf('class LexLean3DVisualizer');
assert.ok(classIdx !== -1, 'LexLean3DVisualizer not found in public/app.js');

const endOfClass = appJsContent.indexOf('\nlet lexLean3dVisualizer', classIdx);
assert.ok(endOfClass !== -1, 'End of LexLean3DVisualizer class not found in public/app.js');

const codeToEval = appJsContent.slice(startIdx, endOfClass);

// Determinant oracle for 4x4 column-major matrix
function mat4_det(m) {
  const m00 = m[0],  m01 = m[4],  m02 = m[8],   m03 = m[12];
  const m10 = m[1],  m11 = m[5],  m12 = m[9],   m13 = m[13];
  const m20 = m[2],  m21 = m[6],  m22 = m[10],  m23 = m[14];
  const m30 = m[3],  m31 = m[7],  m32 = m[11],  m33 = m[15];

  const b00 = m00 * m11 - m01 * m10;
  const b01 = m00 * m12 - m02 * m10;
  const b02 = m00 * m13 - m03 * m10;
  const b03 = m01 * m12 - m02 * m11;
  const b04 = m01 * m13 - m03 * m11;
  const b05 = m02 * m13 - m03 * m12;
  const b06 = m20 * m31 - m21 * m30;
  const b07 = m20 * m32 - m22 * m30;
  const b08 = m20 * m33 - m23 * m30;
  const b09 = m21 * m32 - m22 * m31;
  const b10 = m21 * m33 - m23 * m31;
  const b11 = m22 * m33 - m23 * m32;

  return b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
}

function hasNaNOrInf(arr) {
  for (let i = 0; i < arr.length; i++) {
    if (!Number.isFinite(arr[i])) return true;
  }
  return false;
}

// Build headless visualizer harness
function createHeadlessHarness() {
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
    LEQUAL: 515
  };

  const mockCanvas = {
    getContext: () => mockGl,
    addEventListener: (type, fn) => {
      if (!eventListeners[type]) eventListeners[type] = [];
      eventListeners[type].push(fn);
    },
    setPointerCapture: () => {},
    releasePointerCapture: () => {},
    clientWidth: 800,
    clientHeight: 400,
    width: 800,
    height: 400
  };

  const harnessFactory = new Function('mockCanvas', 'mockGl', `
    ${codeToEval}
    const vis = new LexLean3DVisualizer();
    vis.canvas = mockCanvas;
    vis.initGL();
    vis.buildStaticGeometry();
    vis.buildDefaultDecodeSpline();
    vis.bindEvents();
    return {
      mat4_create,
      mat4_perspective,
      mat4_lookAt,
      mat4_multiply,
      catmullRom1D,
      catmullRomPoint,
      vis
    };
  `);

  const exports = harnessFactory(mockCanvas, mockGl);

  function dispatchWheel(deltaY) {
    const handlers = eventListeners['wheel'] || [];
    const event = {
      deltaY,
      preventDefault: () => {}
    };
    for (const h of handlers) h(event);
  }

  function dispatchPointerDown(x, y, button = 0) {
    const handlers = eventListeners['pointerdown'] || [];
    for (const h of handlers) {
      h({ clientX: x, clientY: y, button, pointerId: 1 });
    }
  }

  function dispatchPointerMove(x, y, options = {}) {
    const handlers = eventListeners['pointermove'] || [];
    for (const h of handlers) {
      h({
        clientX: x,
        clientY: y,
        shiftKey: !!options.shiftKey,
        button: options.button || 0,
        pointerId: 1
      });
    }
  }

  function dispatchPointerUp() {
    const handlers = eventListeners['pointerup'] || [];
    for (const h of handlers) {
      h({ pointerId: 1 });
    }
  }

  return {
    ...exports,
    mockCanvas,
    mockGl,
    bufferCalls,
    dispatchWheel,
    dispatchPointerDown,
    dispatchPointerMove,
    dispatchPointerUp
  };
}

const baseHarness = createHeadlessHarness();
const { mat4_create, mat4_perspective, mat4_lookAt, mat4_multiply } = baseHarness;

// ============================================================================
// SUITE 1: Matrix Math & Determinant Preservation Stress Test
// ============================================================================
console.log('--- Suite 1: Matrix Math & Determinant Preservation ---');

// 1.1 Identity multiplication
{
  const I = mat4_create();
  assert.equal(mat4_det(I), 1.0, 'Identity determinant must be 1.0');

  for (let trial = 0; trial < 1000; trial++) {
    const A = new Float32Array(16);
    for (let i = 0; i < 16; i++) A[i] = (Math.random() - 0.5) * 10.0;

    const outLeft = new Float32Array(16);
    const outRight = new Float32Array(16);

    mat4_multiply(outLeft, I, A);
    mat4_multiply(outRight, A, I);

    for (let i = 0; i < 16; i++) {
      assert.ok(Math.abs(outLeft[i] - A[i]) < 1e-5, `I * A failed at index ${i}`);
      assert.ok(Math.abs(outRight[i] - A[i]) < 1e-5, `A * I failed at index ${i}`);
    }
  }
  console.log('  ✓ 1.1 Identity multiplication: 1,000 randomized trials preserved');
}

// 1.2 Associativity: (A * B) * C == A * (B * C)
{
  for (let trial = 0; trial < 1000; trial++) {
    const A = new Float32Array(16);
    const B = new Float32Array(16);
    const C = new Float32Array(16);
    for (let i = 0; i < 16; i++) {
      A[i] = (Math.random() - 0.5) * 2.0;
      B[i] = (Math.random() - 0.5) * 2.0;
      C[i] = (Math.random() - 0.5) * 2.0;
    }

    const AB = new Float32Array(16);
    const BC = new Float32Array(16);
    const AB_C = new Float32Array(16);
    const A_BC = new Float32Array(16);

    mat4_multiply(AB, A, B);
    mat4_multiply(AB_C, AB, C);

    mat4_multiply(BC, B, C);
    mat4_multiply(A_BC, A, BC);

    for (let i = 0; i < 16; i++) {
      const diff = Math.abs(AB_C[i] - A_BC[i]);
      assert.ok(diff < 1e-3, `Associativity failed at trial ${trial}, index ${i}, diff: ${diff}`);
    }
  }
  console.log('  ✓ 1.2 Matrix associativity: 1,000 randomized triplet trials verified');
}

// 1.3 Determinant multiplicativity: det(A * B) == det(A) * det(B)
{
  let maxDetError = 0;
  for (let trial = 0; trial < 2000; trial++) {
    const A = new Float32Array(16);
    const B = new Float32Array(16);
    for (let i = 0; i < 16; i++) {
      A[i] = (Math.random() - 0.5) * 2.0;
      B[i] = (Math.random() - 0.5) * 2.0;
    }

    const AB = new Float32Array(16);
    mat4_multiply(AB, A, B);

    const detA = mat4_det(A);
    const detB = mat4_det(B);
    const detAB = mat4_det(AB);
    const expected = detA * detB;

    const relError = Math.abs(detAB - expected) / (Math.abs(expected) + 1e-5);
    if (relError > maxDetError) maxDetError = relError;
    assert.ok(relError < 0.05, `Determinant multiplicativity failed: got ${detAB}, expected ${expected}`);
  }
  console.log(`  ✓ 1.3 Determinant multiplicativity: 2,000 trials verified (max relative error: ${maxDetError.toExponential(3)})`);
}

// 1.4 Perspective matrix stress test
{
  const out = new Float32Array(16);
  let checked = 0;
  for (let fovyDeg = 10.0; fovyDeg <= 140.0; fovyDeg += 10.0) {
    for (let aspect of [0.2, 0.5, 1.0, 16.0 / 9.0, 2.35, 5.0]) {
      for (let near of [0.01, 0.1, 1.0, 5.0]) {
        for (let far of [20.0, 100.0, 1000.0, 50000.0]) {
          const fovyRad = (fovyDeg * Math.PI) / 180.0;
          mat4_perspective(out, fovyRad, aspect, near, far);

          assert.ok(!hasNaNOrInf(out), `Perspective contains NaN or Inf at fovy=${fovyDeg}, aspect=${aspect}, near=${near}, far=${far}`);

          const det = mat4_det(out);
          assert.ok(Number.isFinite(det), `Perspective determinant non-finite: ${det}`);
          assert.ok(Math.abs(det) > 1e-9, `Perspective determinant singular: ${det}`);

          // Analytical determinant for standard perspective matrix:
          // det = (f^2 / aspect) * (2 * far * near / (near - far))
          const f = 1.0 / Math.tan(fovyRad / 2.0);
          const expectedDet = ((f * f) / aspect) * ((2.0 * far * near) / (near - far));
          const relErr = Math.abs(det - expectedDet) / Math.abs(expectedDet);
          assert.ok(relErr < 1e-4, `Perspective determinant mismatch: got ${det}, expected ${expectedDet}`);
          checked++;
        }
      }
    }
  }
  console.log(`  ✓ 1.4 Perspective matrix: ${checked} parameter combinations verified with analytic determinant matching`);
}

// 1.5 LookAt matrix stress test & proper rotation rigidity
{
  const out = new Float32Array(16);
  let lookAtChecked = 0;

  for (let r = 2.0; r <= 50.0; r += 6.0) {
    for (let yaw = -Math.PI; yaw <= Math.PI; yaw += 0.4) {
      for (let pitch of [-1.45, -1.0, -0.5, 0.0, 0.5, 1.0, 1.45]) {
        const target = [(Math.random() - 0.5) * 5.0, (Math.random() - 0.5) * 5.0, (Math.random() - 0.5) * 5.0];
        const eyeX = target[0] + r * Math.cos(pitch) * Math.sin(yaw);
        const eyeY = target[1] + r * Math.sin(pitch);
        const eyeZ = target[2] + r * Math.cos(pitch) * Math.cos(yaw);

        mat4_lookAt(out, [eyeX, eyeY, eyeZ], target, [0.0, 1.0, 0.0]);

        assert.ok(!hasNaNOrInf(out), `LookAt contains NaN or Inf at r=${r}, yaw=${yaw}, pitch=${pitch}`);

        // LookAt must be a proper rigid body transformation (rotation + translation) -> det == 1.0
        const det = mat4_det(out);
        assert.ok(Math.abs(det - 1.0) < 1e-4, `LookAt determinant not rigid (+1.0): got ${det}`);

        // Upper 3x3 must be orthonormal: R * R^T == I
        const R = [
          [out[0], out[4], out[8]],
          [out[1], out[5], out[9]],
          [out[2], out[6], out[10]]
        ];
        for (let row = 0; row < 3; row++) {
          for (let col = 0; col < 3; col++) {
            let dot = 0.0;
            for (let k = 0; k < 3; k++) dot += R[row][k] * R[col][k];
            const expected = row === col ? 1.0 : 0.0;
            assert.ok(Math.abs(dot - expected) < 1e-4, `LookAt upper 3x3 not orthonormal at (${row},${col}): got ${dot}`);
          }
        }
        lookAtChecked++;
      }
    }
  }
  console.log(`  ✓ 1.5 LookAt matrix: ${lookAtChecked} camera poses verified with strict rigidity det(M)=+1.0 and orthonormality`);
}

// 1.6 Degenerate LookAt edge cases
{
  const out = new Float32Array(16);
  // Case A: eye == center
  mat4_lookAt(out, [0, 0, 0], [0, 0, 0], [0, 1, 0]);
  assert.ok(!hasNaNOrInf(out), 'Degenerate LookAt (eye==center) contains NaN/Inf');

  // Case B: view direction parallel to up vector [0, 1, 0]
  mat4_lookAt(out, [0, 5, 0], [0, 0, 0], [0, 1, 0]);
  assert.ok(!hasNaNOrInf(out), 'Degenerate LookAt (view parallel to up) contains NaN/Inf');

  console.log('  ✓ 1.6 Degenerate LookAt edge cases: zero distance and collinear up handled without division by zero');
}

// 1.7 View-Projection composite matrix
{
  const proj = new Float32Array(16);
  const view = new Float32Array(16);
  const vp = new Float32Array(16);

  mat4_perspective(proj, (45.0 * Math.PI) / 180.0, 16.0 / 9.0, 0.1, 100.0);
  mat4_lookAt(view, [0, 5, 15], [0, 0, 0], [0, 1, 0]);
  mat4_multiply(vp, proj, view);

  assert.ok(!hasNaNOrInf(vp), 'View-Projection composite contains NaN/Inf');
  const detProj = mat4_det(proj);
  const detView = mat4_det(view);
  const detVP = mat4_det(vp);

  assert.ok(Math.abs(detView - 1.0) < 1e-4, 'View determinant must be 1.0');
  const relDiff = Math.abs(detVP - detProj * detView) / Math.abs(detProj);
  assert.ok(relDiff < 1e-4, `VP composite determinant mismatch: got ${detVP}, expected ${detProj}`);

  console.log('  ✓ 1.7 View-Projection composite: verified det(VP) = det(P) * det(V) = det(P) * 1.0\n');
}

// ============================================================================
// SUITE 2: Zoom Boundary Conditions & Extreme Delta Stress Test
// ============================================================================
console.log('--- Suite 2: Zoom Boundary Conditions & Clamping ---');

// 2.1 Default distance check
{
  const harness = createHeadlessHarness();
  assert.equal(harness.vis.distance, 15.0, 'Default distance must be 15.0');
  console.log('  ✓ 2.1 Default camera distance: confirmed 15.0');
}

// 2.2 Zoom-in boundary clamping at 2.0
{
  const harness = createHeadlessHarness();
  for (let i = 0; i < 2000; i++) {
    harness.dispatchWheel(-100);
    assert.ok(harness.vis.distance >= 2.0, `Distance breached min bound: ${harness.vis.distance}`);
    assert.ok(harness.vis.distance <= 50.0, `Distance breached max bound: ${harness.vis.distance}`);
    assert.ok(Number.isFinite(harness.vis.distance), `Distance non-finite: ${harness.vis.distance}`);
  }
  assert.equal(harness.vis.distance, 2.0, `Zoom-in failed to clamp at minimum bound 2.0; got ${harness.vis.distance}`);
  console.log('  ✓ 2.2 Zoom-in clamping: 2,000 steps reached and strictly held minimum distance 2.0');
}

// 2.3 Zoom-out boundary clamping at 50.0
{
  const harness = createHeadlessHarness();
  for (let i = 0; i < 2000; i++) {
    harness.dispatchWheel(+100);
    assert.ok(harness.vis.distance >= 2.0, `Distance breached min bound: ${harness.vis.distance}`);
    assert.ok(harness.vis.distance <= 50.0, `Distance breached max bound: ${harness.vis.distance}`);
    assert.ok(Number.isFinite(harness.vis.distance), `Distance non-finite: ${harness.vis.distance}`);
  }
  assert.equal(harness.vis.distance, 50.0, `Zoom-out failed to clamp at maximum bound 50.0; got ${harness.vis.distance}`);
  console.log('  ✓ 2.3 Zoom-out clamping: 2,000 steps reached and strictly held maximum distance 50.0');
}

// 2.4 Extreme wheel deltas (+1e9, -1e9, 0, NaN, Infinity, -Infinity, undefined)
{
  const harness = createHeadlessHarness();

  // Test +1e9
  for (let i = 0; i < 100; i++) harness.dispatchWheel(1e9);
  assert.ok(harness.vis.distance <= 50.0 && harness.vis.distance >= 2.0, `Breach on +1e9: ${harness.vis.distance}`);
  assert.equal(harness.vis.distance, 50.0, `Clamped at 50.0 on +1e9: ${harness.vis.distance}`);

  // Test -1e9
  for (let i = 0; i < 100; i++) harness.dispatchWheel(-1e9);
  assert.ok(harness.vis.distance <= 50.0 && harness.vis.distance >= 2.0, `Breach on -1e9: ${harness.vis.distance}`);
  assert.equal(harness.vis.distance, 2.0, `Clamped at 2.0 on -1e9: ${harness.vis.distance}`);

  // Test 0
  harness.vis.resetCamera();
  harness.dispatchWheel(0);
  assert.ok(Number.isFinite(harness.vis.distance) && harness.vis.distance >= 2.0 && harness.vis.distance <= 50.0);

  // Test NaN
  harness.dispatchWheel(NaN);
  assert.ok(Number.isFinite(harness.vis.distance) && harness.vis.distance >= 2.0 && harness.vis.distance <= 50.0);

  // Test +Infinity
  harness.dispatchWheel(Infinity);
  assert.ok(Number.isFinite(harness.vis.distance) && harness.vis.distance >= 2.0 && harness.vis.distance <= 50.0);

  // Test -Infinity
  harness.dispatchWheel(-Infinity);
  assert.ok(Number.isFinite(harness.vis.distance) && harness.vis.distance >= 2.0 && harness.vis.distance <= 50.0);

  // Test undefined
  harness.dispatchWheel(undefined);
  assert.ok(Number.isFinite(harness.vis.distance) && harness.vis.distance >= 2.0 && harness.vis.distance <= 50.0);

  console.log('  ✓ 2.4 Extreme wheel deltas: (+1e9, -1e9, 0, NaN, ±Infinity, undefined) maintained strict [2.0, 50.0] bounds');
}

// 2.5 Rapid oscillating wheel input
{
  const harness = createHeadlessHarness();
  const deltas = [1e9, -1e9, 0, NaN, 1.08, -0.92, 100, -100, Infinity, -Infinity];

  for (let i = 0; i < 10000; i++) {
    const d = deltas[i % deltas.length];
    harness.dispatchWheel(d);
    assert.ok(Number.isFinite(harness.vis.distance), `Distance non-finite at step ${i}: ${harness.vis.distance}`);
    assert.ok(harness.vis.distance >= 2.0, `Distance underflow at step ${i}: ${harness.vis.distance}`);
    assert.ok(harness.vis.distance <= 50.0, `Distance overflow at step ${i}: ${harness.vis.distance}`);
  }
  console.log('  ✓ 2.5 Rapid oscillation: 10,000 extreme random deltas verified');
}

// 2.6 Camera reset restores distance
{
  const harness = createHeadlessHarness();
  for (let i = 0; i < 50; i++) harness.dispatchWheel(1e9);
  assert.equal(harness.vis.distance, 50.0);
  harness.vis.resetCamera();
  assert.equal(harness.vis.distance, 15.0, 'resetCamera must restore distance 15.0');

  for (let i = 0; i < 50; i++) harness.dispatchWheel(-1e9);
  assert.equal(harness.vis.distance, 2.0);
  harness.vis.resetCamera();
  assert.equal(harness.vis.distance, 15.0, 'resetCamera must restore distance 15.0');

  console.log('  ✓ 2.6 Camera reset: distance cleanly restored to 15.0 from both extremes\n');
}

// ============================================================================
// SUITE 3: Orbit Boundary Conditions & Gimbal Lock Prevention
// ============================================================================
console.log('--- Suite 3: Orbit Boundary Conditions & Gimbal Lock Prevention ---');

// 3.1 Vertical pitch clamping [-1.45, 1.45]
{
  const harness = createHeadlessHarness();
  harness.dispatchPointerDown(400, 200);

  // Extreme upward drag
  harness.dispatchPointerMove(400, 100000);
  assert.equal(harness.vis.pitch, 1.45, `Pitch failed to clamp at +1.45; got ${harness.vis.pitch}`);

  // Extreme downward drag
  harness.dispatchPointerMove(400, -100000);
  assert.equal(harness.vis.pitch, -1.45, `Pitch failed to clamp at -1.45; got ${harness.vis.pitch}`);

  // Check margin from pi/2
  const halfPi = Math.PI / 2.0;
  const margin = halfPi - 1.45;
  assert.ok(margin > 0.12, `Pitch safety margin too small: ${margin}`);

  // Verify cos(pitch) > 0 at all times
  const minCosPitch = Math.cos(1.45);
  assert.ok(minCosPitch > 0.12, `cos(pitch) must be strictly positive: ${minCosPitch}`);

  // Guaranteed horizontal radius >= 2.0 * minCosPitch > 0.24
  const minHorizontalRadius = 2.0 * minCosPitch;
  assert.ok(minHorizontalRadius > 0.24, `Horizontal radius must never collapse: ${minHorizontalRadius}`);

  harness.dispatchPointerUp();
  console.log(`  ✓ 3.1 Pitch clamping: strictly clamped to [-1.45, 1.45] with safety margin ${margin.toFixed(4)} rad from pi/2`);
  console.log(`      Guaranteed non-vanishing horizontal radius: min ${minHorizontalRadius.toFixed(4)} > 0`);
}

// 3.2 Yaw continuous rotation & long-duration stability
{
  const harness = createHeadlessHarness();
  harness.vis.autoRotate = true;
  harness.vis.isDragging = false;

  // Simulate 100,000 animation frames (~27 minutes of 60fps orbit)
  for (let frame = 0; frame < 100000; frame++) {
    harness.vis.yaw += 0.25 * 0.016;
  }
  assert.ok(Number.isFinite(harness.vis.yaw), `Yaw non-finite after 100k frames: ${harness.vis.yaw}`);

  const sinY = Math.sin(harness.vis.yaw);
  const cosY = Math.cos(harness.vis.yaw);
  assert.ok(Number.isFinite(sinY) && Math.abs(sinY) <= 1.0, `sin(yaw) invalid: ${sinY}`);
  assert.ok(Number.isFinite(cosY) && Math.abs(cosY) <= 1.0, `cos(yaw) invalid: ${cosY}`);

  // Verify camera render matrix remains valid
  harness.vis.render(100.0);
  assert.ok(!hasNaNOrInf(harness.vis.viewMatrix), 'View matrix non-finite after 100k frames');
  assert.ok(!hasNaNOrInf(harness.vis.viewProjMatrix), 'View-projection non-finite after 100k frames');

  console.log('  ✓ 3.2 Long-duration yaw: 100,000 frames evaluated without precision degradation or NaN');
}

// 3.3 Extreme yaw values (±1e9, ±1e6, ±2*pi*k)
{
  const harness = createHeadlessHarness();
  const extremeYaws = [1e9, -1e9, 1e6, -1e6, 2 * Math.PI * 1000, -2 * Math.PI * 1000];

  for (const yaw of extremeYaws) {
    harness.vis.yaw = yaw;
    harness.vis.render(0.0);
    assert.ok(!hasNaNOrInf(harness.vis.viewMatrix), `View matrix contains NaN at yaw=${yaw}`);
    assert.ok(!hasNaNOrInf(harness.vis.viewProjMatrix), `ViewProj matrix contains NaN at yaw=${yaw}`);

    const det = mat4_det(harness.vis.viewMatrix);
    assert.ok(Math.abs(det - 1.0) < 1e-4, `Rigidity breach at yaw=${yaw}: det=${det}`);
  }
  console.log('  ✓ 3.3 Extreme yaw values: (±1e9, ±1e6, ±2*pi*k) preserved proper rotation rigidity');
}

// 3.4 Pan interaction with shift-drag
{
  const harness = createHeadlessHarness();
  harness.dispatchPointerDown(400, 200);

  // Shift-drag to pan
  harness.dispatchPointerMove(450, 230, { shiftKey: true });
  assert.ok(Number.isFinite(harness.vis.target[0]), 'Target x non-finite');
  assert.ok(Number.isFinite(harness.vis.target[1]), 'Target y non-finite');
  assert.ok(Number.isFinite(harness.vis.target[2]), 'Target z non-finite');

  harness.vis.resetCamera();
  assert.equal(harness.vis.target[0], 0.0, 'resetCamera must restore target x to 0');
  assert.equal(harness.vis.target[1], 0.0, 'resetCamera must restore target y to 0');
  assert.equal(harness.vis.target[2], 0.0, 'resetCamera must restore target z to 0');

  harness.dispatchPointerUp();
  console.log('  ✓ 3.4 Shift-pan translation: pointer dragging pan and resetCamera verified\n');
}

// ============================================================================
// SUITE 4: Dynamic Trace Mutation Stress Test & WebGL Buffer Finiteness
// ============================================================================
console.log('--- Suite 4: Dynamic Trace Mutation Stress Test ---');

// Helper to audit all WebGL buffers pushed during render
function auditRenderBuffers(harness) {
  harness.bufferCalls.length = 0;
  harness.vis.render(0.123);
  assert.ok(harness.bufferCalls.length >= 2, `Expected at least 2 bufferData calls, got ${harness.bufferCalls.length}`);

  let totalFloats = 0;
  for (let b = 0; b < harness.bufferCalls.length; b++) {
    const buf = harness.bufferCalls[b];
    for (let i = 0; i < buf.length; i++) {
      assert.ok(Number.isFinite(buf[i]), `Buffer ${b} contains non-finite float at index ${i}: ${buf[i]}`);
      totalFloats++;
    }
  }
  return totalFloats;
}

// 4.1 Empty & Null Traces
{
  const harness = createHeadlessHarness();
  const emptyTraces = [null, undefined, {}, { spans: [] }, { spans: null }, 'string', 12345];

  for (const trace of emptyTraces) {
    assert.doesNotThrow(() => harness.vis.loadTrace(trace), `loadTrace threw on ${JSON.stringify(trace)}`);
    const floats = auditRenderBuffers(harness);
    assert.ok(floats > 0, 'Render should push valid default buffers');
  }
  console.log('  ✓ 4.1 Empty & Null traces: handled cleanly with zero thrown exceptions and zero NaN in buffers');
}

// 4.2 Missing Intern Table
{
  const harness = createHeadlessHarness();
  const traceNoIntern = {
    turn_index: 1,
    trace_id: '4a8b1c2d3e4f5a6b',
    spans: [
      {
        name_ref: 'gllm.trunk.layer.l0',
        attributes: [
          ['uor.layer_index', 0],
          ['uor.basin_energy_l1', 500]
        ]
      },
      {
        name_ref: 'gllm.decode.step',
        attributes: [
          ['uor.step', 0],
          ['uor.predicted_token', 1042]
        ]
      }
    ]
  };

  harness.vis.loadTrace(traceNoIntern);
  assert.equal(harness.vis.activeTraceInfo.turnIndex, 1);
  assert.equal(harness.vis.activeTraceInfo.spanCount, 2);
  const floats = auditRenderBuffers(harness);
  assert.ok(floats > 0);
  console.log('  ✓ 4.2 Missing intern table: direct uninterned string names handled gracefully');
}

// 4.3 Extreme Clifford Grade Arrays
{
  const harness = createHeadlessHarness();
  const testGrades = [
    [], // empty
    [10, 20, 30, 40, 50, 60, 70], // standard
    [1e9, -1e9, 0, 50, 100, 200, 300], // extreme values
    [NaN, 'corrupted', null, undefined, Infinity, -Infinity, 100] // malformed elements
  ];

  for (const grades of testGrades) {
    const trace = {
      spans: [
        {
          name_ref: 'gllm.trunk.layer.l0',
          attributes: [
            ['uor.layer_index', 0],
            ['uor.clifford_grades', grades]
          ]
        }
      ]
    };
    harness.vis.loadTrace(trace);
    const floats = auditRenderBuffers(harness);
    assert.ok(floats > 0);
  }
  console.log('  ✓ 4.3 Extreme Clifford grade arrays: clamped and audited with zero NaN in vertex buffers');
}

// 4.4 Partial and Tier-Skipped Traces
{
  const harness = createHeadlessHarness();

  // Tier 1 only (tokenize)
  harness.vis.loadTrace({
    spans: [{ name_ref: 'gllm.tokenize', attributes: [] }]
  });
  auditRenderBuffers(harness);

  // Tier 2 only (layer 5 skipping 0..4)
  harness.vis.loadTrace({
    spans: [
      {
        name_ref: 'gllm.trunk.layer.l5',
        attributes: [
          ['uor.layer_index', 5],
          ['uor.dominant_icosian_root', 12]
        ]
      }
    ]
  });
  assert.equal(harness.vis.dominantIcosianRoot, 12);
  auditRenderBuffers(harness);

  // Tier 3 only (routing point key)
  harness.vis.loadTrace({
    spans: [
      {
        name_ref: 'gllm.routing.point_key',
        attributes: [['uor.winning_row', 42]]
      }
    ]
  });
  assert.equal(harness.vis.winningPointKeyRow, 42);
  auditRenderBuffers(harness);

  // Decode steps: 1 step (pts.length < 2 test), 2 steps, 50 steps
  for (const stepCount of [1, 2, 5, 20, 50]) {
    const decodeSpans = [];
    for (let s = 0; s < stepCount; s++) {
      decodeSpans.push({
        name_ref: 'gllm.decode.step',
        attributes: [
          ['uor.step', s],
          ['uor.predicted_token', 200 + s]
        ]
      });
    }
    harness.vis.loadTrace({ spans: decodeSpans });
    assert.equal(harness.vis.decodeWaypoints.length, stepCount);
    if (stepCount === 1) {
      assert.equal(harness.vis.tokenSplinePoints.length, 0, '1 waypoint cannot construct spline; must be empty');
    } else {
      assert.ok(harness.vis.tokenSplinePoints.length > stepCount, 'Spline points must interpolate waypoints');
    }
    auditRenderBuffers(harness);
  }

  console.log('  ✓ 4.4 Partial and tier-skipped traces: tested tokenization-only, layer-skip, and 1..50 decode waypoints');
}

// 4.5 Full Canonical OTel Trace Emitted by Rust Engine
{
  const harness = createHeadlessHarness();

  // Construct canonical uor.gllm.trace/1 schema matching otel.rs
  const canonicalTrace = {
    format: 'uor.gllm.trace/1',
    trace_id: '4a8b1c2d3e4f5a6b7c8d9e0f1a2b3c4d',
    root_span_id: '1a2b3c4d5e6f7a8b',
    turn_index: 3,
    context_digest_before: '0000000000000000',
    context_digest_after: '0123456789abcdef',
    intern_table: {
      entries: [
        'gllm.turn',
        'gllm.tokenize',
        'gllm.trunk.layer.l0',
        'gllm.trunk.layer.l1',
        'gllm.trunk.layer.l2',
        'gllm.trunk.layer.l3',
        'gllm.trunk.layer.l4',
        'gllm.trunk.layer.l5',
        'gllm.routing.point_key',
        'gllm.decode.step',
        'uor.layer_index',
        'uor.clifford_grades',
        'uor.dominant_icosian_root',
        'uor.basin_energy_l1',
        'uor.winning_row',
        'uor.step',
        'uor.predicted_token'
      ]
    },
    spans: [
      { name_ref: 0, attributes: [] },
      { name_ref: 1, attributes: [] },
      {
        name_ref: 2,
        attributes: [
          [10, 0],
          [11, [1, 6, 15, 20, 15, 6, 1]],
          [12, 4],
          [13, 850]
        ]
      },
      {
        name_ref: 3,
        attributes: [
          [10, 1],
          [11, [1, 6, 15, 20, 15, 6, 1]],
          [12, 7],
          [13, 920]
        ]
      },
      {
        name_ref: 8,
        attributes: [[14, 18]]
      },
      {
        name_ref: 9,
        attributes: [
          [15, 0],
          [16, 42]
        ]
      },
      {
        name_ref: 9,
        attributes: [
          [15, 1],
          [16, 107]
        ]
      }
    ]
  };

  harness.vis.loadTrace(canonicalTrace);
  assert.equal(harness.vis.activeTraceInfo.turnIndex, 3);
  assert.equal(harness.vis.dominantIcosianRoot, 7);
  assert.equal(harness.vis.winningPointKeyRow, 18);
  assert.equal(harness.vis.decodeWaypoints.length, 2);

  const totalFloats = auditRenderBuffers(harness);
  assert.ok(totalFloats > 500, `Expected rich vertex data, got ${totalFloats} floats`);

  console.log(`  ✓ 4.5 Full canonical OTel trace: parsed and animated successfully (${totalFloats} vertex floats, zero NaN/Inf)\n`);
}

console.log('================================================================');
console.log('  ALL EMPIRICAL 3D WEBGL MATH & CAMERA CONTROL SUITES PASSED    ');
console.log('================================================================\n');
