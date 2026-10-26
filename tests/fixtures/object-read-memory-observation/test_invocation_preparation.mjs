import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import crypto from 'node:crypto';
import childProcess from 'node:child_process';
import {registerHooks, syncBuiltinESMExports} from 'node:module';

const candidate = new URL('./invocation-preparation.mjs', import.meta.url);
const retained = JSON.parse(fs.readFileSync(new URL('./design/original-invocation.json', import.meta.url)));
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const oldGate = "if(freshEpochBinding===null)throw Error('NO_REVIEWED_FRESH_EPOCH: preparation only; no output or process invocation');\n";
const oldArgs = 'if(process.argv.length!==3||!["child-accounting-negative","original-admitted-negative","complete-admitted-positive"].includes(phase))throw Error(\'select exactly one reviewed admitted product phase; no automatic progression\');\n';
const newMain = '\nexport function main(argv){\n if(!Array.isArray(argv)||argv.length!==1||argv[0]!==phase)throw Error(\'select exactly complete-admitted-positive; preparation only\');\n '+oldGate+' fs.mkdirSync(out);\n runOriginal();\n}\n';

function expected(record = retained) {
  assert.equal(record.bytes, 12797);
  assert.equal(Buffer.byteLength(record.text), 12797);
  assert.equal(sha(record.text), '3cc48e7aef25481a08f64bb9e2ea2f2b6b32a2e48109659b0322efd90d099b27');
  let source = record.text;
  const once = (oldText, newText) => {
    assert.equal(source.split(oldText).length, 2, 'unique pinned edit');
    source = source.replace(oldText, newText);
  };
  once('const phase=process.argv[2];', "const phase='complete-admitted-positive';");
  once(oldArgs, '');
  once('// Binding below records the actual reviewed restart snapshot; it does not assert present storage or readiness.\n', '// Frozen source preparation has no live resource epoch or execution authority.\n');
  const epochLine = source.split('\n').filter(line => line.startsWith('const freshEpochBinding='));
  assert.equal(epochLine.length, 1);
  once(epochLine[0], 'const freshEpochBinding=null;');
  once(oldGate, '');
  once('fs.mkdirSync(out);\n', '');
  once('\ntry{\n', '\nfunction runOriginal(){\ntry{\n');
  return source+'}\n'+newMain;
}

function guarded(callback) {
  const calls = [], restored = [];
  for (const [owner, names] of [[fs, ['mkdirSync', 'writeFileSync', 'statfsSync', 'readFileSync']],
                               [childProcess, ['spawnSync']]]) {
    for (const name of names) {
      const original = owner[name];
      restored.push(() => { owner[name] = original; });
      owner[name] = () => { calls.push(name); throw Error('forbidden side effect '+name); };
    }
  }
  syncBuiltinESMExports();
  return Promise.resolve().then(callback).then(value => {
    assert.deepEqual(calls, []);
    return value;
  }).finally(() => { for (const restore of restored) restore(); syncBuiltinESMExports(); });
}

test('retained original has independent fixed byte identity; corrupt data rejects', () => {
  expected();
  assert.throws(() => expected({...retained, bytes: 1}));
  assert.throws(() => expected({...retained, text: retained.text+' '}));
  const corrupted = retained.text.replace('const phase=process.argv[2];', 'const phase=process.argv[3];');
  assert.notEqual(corrupted, retained.text);
  assert.throws(() => expected({...retained, text: corrupted}));
});

test('prepared source is the exact allowlisted adaptation, with no other edits', () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  assert.equal(fs.readFileSync(candidate, 'utf8'), expected());
});

test('import is inert and exports only the closed main entry', async () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  const source = fs.readFileSync(candidate, 'utf8');
  assert.equal(source, expected());
  let loads = 0;
  const hook = registerHooks({load(url, context, nextLoad) {
    if (url !== candidate.href) return nextLoad(url, context);
    loads++;
    return {format: 'module', source, shortCircuit: true};
  }});
  try {
    const module = await guarded(() => import(candidate.href));
    assert.equal(loads, 1);
    assert.deepEqual(Object.keys(module), ['main']);
  } finally { hook.deregister(); }
});

test('positive-only arguments and immutable null gate reject before owned effects', async () => {
  assert.equal(fs.existsSync(candidate), true, 'preparation source missing');
  const {main} = await import(candidate.href);
  await guarded(() => {
    for (const argv of [undefined, null, {}, '', [], ['child-accounting-negative'],
                       ['original-admitted-negative'], ['complete-admitted-positive', 'extra']]) {
      assert.throws(() => main(argv), /select exactly complete-admitted-positive/);
    }
    assert.throws(() => main(['complete-admitted-positive']), /NO_REVIEWED_FRESH_EPOCH/);
  });
});

test('preservation oracle rejects representative changed obligations as text only', () => {
  const source = expected();
  const changes = [
    ['if(inputs.length!==50', 'if(inputs.length!==49'],
    ['ObjectAdmittedRangeResult.lex.tex', 'WrongResult.lex.tex'],
    ['107e02328a8052d1560d6e8addcfd1c70af6145bbf836650cac32dc044f550e5', '0'.repeat(64)],
    ['maxBuffer:20*1024*1024', 'maxBuffer:21*1024*1024'],
    ["['memory-after',()=>zeroMemory('memory-after')]", "['memory-after',()=>undefined]"],
    ["const result=cli('verify'", "cli('duplicate',['verify',target],false);const result=cli('verify'"],
  ];
  for (const [before, after] of changes) {
    assert.ok(source.includes(before));
    assert.throws(() => assert.equal(source.replace(before, after), expected()));
  }
  assert.throws(() => assert.equal(source+'fs.mkdirSync("unexpected");\n', expected()));
});
