// Node E2E verification for R4-70: Zero-Float vTensor Certified Storage Planner
// and 16x16 Tile-Binned UORC LOD0..LOD4 3D PBR WebGUI (Observatory + ?surface=preview).

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..', '..');

const wasmBytes = fs.readFileSync(path.join(rootDir, 'public', 'lexlean_r4_bg.wasm'));
const wasmJsUrl = pathToFileURL(path.join(rootDir, 'public', 'lexlean_r4.js')).href;
const wasmMod = await import(wasmJsUrl);
await wasmMod.default(wasmBytes);

const selfCheck = JSON.parse(wasmMod.studio_vtensor_3d_self_check_json());
assert.equal(selfCheck.spec, 'uor-vtensor-3d/1');
assert.equal(selfCheck.conformance_id, 'R4-70');
assert.equal(selfCheck.lexlean_admitted, true);
assert.equal(selfCheck.float_ops, 0);
assert.equal(selfCheck.dense_matmul_ops, 0);
assert.equal(selfCheck.softmax_ops, 0);
assert.equal(selfCheck.arena_offset_aligned_64, true);
assert.deepEqual(selfCheck.tile_bin, [16, 16]);
assert.deepEqual(selfCheck.lod_range, [0, 4]);

// Verify Observatory/HUD 3D Hopf-Poincare-Zeta RGBA8 rendering
const hudRgba = wasmMod.studio_render_hopf_zeta_3d_rgba(128, 96, 180, 3, '-500,300,100;400,-200,300');
assert.equal(hudRgba.length, 128 * 96 * 4);
let litCount = 0;
for (let i = 0; i < hudRgba.length; i += 4) {
  if (hudRgba[i] > 35 || hudRgba[i + 1] > 90 || hudRgba[i + 2] > 110) litCount += 1;
}
assert.ok(litCount > 250, `expected >250 lit PBR pixels in Observatory 3D frame, got ${litCount}`);

// Verify ?surface=preview 3D scene rendering + out-of-range LOD5 rejection
const previewRgba = wasmMod.studio_render_preview_scene_3d_rgba(
  JSON.stringify({ preset: 'icosian-600-shell', lod: 4, yaw_phase1024: 260, pitch_phase1024: 120 }),
  96,
  64,
);
assert.equal(previewRgba.length, 96 * 64 * 4);

const refusedLod5 = wasmMod.studio_render_preview_scene_3d_rgba(
  JSON.stringify({ preset: 'icosian-600-shell', lod: 5 }),
  64,
  64,
);
assert.equal(refusedLod5.length, 0, 'LOD5 must be rejected by admitSemiringRaster3d');

// Verify bridge helper mountPreviewScene3d attaches #preview-vtensor-3d-canvas
let appendedCanvas = null;
const mockHost = {
  querySelector: () => appendedCanvas,
  appendChild: (el) => { appendedCanvas = el; },
};
const mockDoc = {
  createElement: (tag) => ({
    tagName: tag.toUpperCase(),
    id: '',
    width: 0,
    height: 0,
    getContext: () => null,
  }),
};
const mountRes = globalThis.__uorSelfHostRuntime.mountPreviewScene3d(
  { preset: 'zeta-harmonic-crown', lod: 2, width: 160, height: 112 },
  mockHost,
  mockDoc,
);
assert.equal(mountRes.spec, 'uor-vtensor-3d/1');
assert.equal(mountRes.rgbaLength, 160 * 112 * 4);
assert.equal(appendedCanvas.id, 'preview-vtensor-3d-canvas');

console.log('R4-70 vTensor Semiring 3D PBR WebGUI E2E: PASS', {
  frameSha256: selfCheck.frame_sha256.slice(0, 16),
  litCount,
  uorcBytes: selfCheck.uorc_bytes,
});
