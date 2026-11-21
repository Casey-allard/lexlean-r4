// Node E2E verification for R4-71: Foundry Hologram Quantum PC, FK-01..09 InTabProver
// Proof-Gated Studio Lifecycle, FR-05 Sealed MlsChannel, and Canonical JSON BLAKE3 kappa.

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

const selfCheck = globalThis.__uorSelfHostRuntime.foundryQuantumSelfCheck();
assert.equal(selfCheck.spec, 'uor-foundry-quantum-studio/1');
assert.equal(selfCheck.conformance_id, 'R4-71');
assert.equal(selfCheck.lexlean_admitted, true);
assert.equal(selfCheck.float_ops, 0);
assert.equal(selfCheck.publish_before_prove_refused, true);
assert.equal(selfCheck.sound_kernel_axioms, true);
assert.equal(selfCheck.mls_roundtrip_ok, true);
assert.equal(selfCheck.mls_ciphertext_hides_plaintext, true);
assert.ok(selfCheck.holo_kappa.startsWith('blake3:'));

// Verify float-forbidding unique-key canonical JSON kappa
const okKappa = globalThis.__uorSelfHostRuntime.uorCanonicalKappa('{ "b": 2, "a": 1 }');
assert.equal(okKappa.ok, true);
assert.equal(okKappa.canonical, '{"a":1,"b":2}');
assert.ok(okKappa.kappa.startsWith('blake3:'));

const badFloat = globalThis.__uorSelfHostRuntime.uorCanonicalKappa('{"lr": 0.001}');
assert.equal(badFloat.ok, false);
assert.ok(badFloat.error.includes('RefusedCanonicalJsonFloat'));

const badDup = globalThis.__uorSelfHostRuntime.uorCanonicalKappa('{"id": 1, "id": 2}');
assert.equal(badDup.ok, false);
assert.ok(badDup.error.includes('RefusedCanonicalJsonDuplicateKey'));

console.log('R4-71 Foundry Hologram Quantum PC & InTabProver E2E: PASS', {
  holoKappa: selfCheck.holo_kappa.slice(0, 23),
  canonicalKappa: okKappa.kappa.slice(0, 23),
});
