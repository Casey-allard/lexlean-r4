import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

const wasmMod = await import('../../public/lexlean_r4.js');
const {
  default: initWasm,
  studio_dispatch,
  admitCargoTerminalSubcommand,
  admitNpmTerminalSubcommand,
  admitC99StdlibBrkAlign,
  SpiralCoreHostBridge,
} = wasmMod;

const wasmBytes = fs.readFileSync(path.join(rootDir, 'public/lexlean_r4_bg.wasm'));
await initWasm({ module_or_path: wasmBytes });

// 1. Formal R4-67 Admission Verdicts & Refusal Gates
assert.equal(admitCargoTerminalSubcommand(true, false), 'CargoAstToolchainAdmitted');
assert.equal(admitCargoTerminalSubcommand(false, false), 'RefusedUnsupportedToolchainSubcommand');
assert.equal(admitCargoTerminalSubcommand(true, true), 'RefusedProtectedPathToolchainMutation');

assert.equal(admitNpmTerminalSubcommand(true, false), 'NpmSubcommandAdmitted');
assert.equal(admitNpmTerminalSubcommand(false, false), 'RefusedUnsupportedToolchainSubcommand');
assert.equal(admitNpmTerminalSubcommand(true, true), 'RefusedProtectedPathToolchainMutation');

assert.equal(admitC99StdlibBrkAlign(0, false), 'C99StdlibHeapAdmitted');
assert.equal(admitC99StdlibBrkAlign(4, false), 'RefusedUnalignedHeapBreak');
assert.equal(admitC99StdlibBrkAlign(0, true), 'RefusedProtectedPathToolchainMutation');

const runMvm = (cmd) => {
  const out = JSON.parse(studio_dispatch(`1\tstudio\tmvm_exec\t${JSON.stringify({ cmd })}`));
  return {
    ...out,
    stdout: out.mvm_stdout ?? out.stdout ?? '',
    stderr: out.mvm_stderr ?? out.stderr ?? '',
  };
};

// 2. Verify In-MVM `cargo` (`--version`, `check`, `test`, `run`, refusal on unknown subcommand) & `rustc`
const verOut = runMvm('cargo --version && rustc --version && which cargo');
assert.equal(verOut.exit_code, 0, `cargo/rustc version failed: ${JSON.stringify(verOut)}`);
assert.ok(verOut.stdout.includes('cargo 1.97.1-mvm-uor'));
assert.ok(verOut.stdout.includes('rustc 1.97.1-mvm-uor'));
assert.ok(verOut.stdout.includes('/bin/cargo'));

const rsSrc = [
  'pub fn fib(n: i64) -> i64 {',
  '    let mut a: i64 = 0;',
  '    let mut b: i64 = 1;',
  '    let mut i: i64 = 0;',
  '    while i < n {',
  '        let next: i64 = a + b;',
  '        a = b;',
  '        b = next;',
  '        i += 1;',
  '    }',
  '    a',
  '}',
  '',
  'fn main() {',
  '    println!("mvm_cargo_fib10={}", fib(10));',
  '}',
  '',
  '#[test]',
  'fn test_fib_10() {',
  '    assert_eq!(fib(10), 55);',
  '}',
].join('\n');

const writeRs = JSON.parse(
  studio_dispatch(`1\tstudio\tworkspace_write\tsrc/polyglot_fib.rs\n${rsSrc}`),
);
assert.equal(writeRs.status, 'ok');

const cargoCheck = runMvm('cargo check');
assert.equal(cargoCheck.exit_code, 0, `cargo check failed: ${JSON.stringify(cargoCheck)}`);

const cargoTest = runMvm('cargo test test_fib_10');
assert.equal(cargoTest.exit_code, 0, `cargo test failed: ${JSON.stringify(cargoTest)}`);
assert.ok(
  cargoTest.stdout.includes('test_fib_10') && cargoTest.stdout.includes('ok'),
  `Expected test_fib_10 ok in stdout, got: ${cargoTest.stdout}`,
);

const rustcSingle = runMvm('rustc /workspace/src/polyglot_fib.rs');
assert.equal(rustcSingle.exit_code, 0, `rustc single-file check failed: ${JSON.stringify(rustcSingle)}`);

const cargoBad = runMvm('cargo frobnicate');
assert.equal(cargoBad.exit_code, 101);
assert.ok(cargoBad.stderr.includes('RefusedUnsupportedToolchainSubcommand'));

// 3. Verify Extended C99 (`<stdlib.h>` `malloc`/`calloc`/`free`/`atoi`/`abs` + `<stdbool.h>` `bool`/`true`/`false`)
const c99Src = [
  '#include <stdio.h>',
  '#include <stdlib.h>',
  '#include <stdbool.h>',
  'int main() {',
  '    bool ok = true;',
  '    int *buf = malloc(32);',
  '    int *zbuf = calloc(4, 8);',
  '    if (!buf || !zbuf) return 2;',
  '    if (((long)buf & 7) != 0 || ((long)zbuf & 7) != 0) return 3;',
  '    if (zbuf[0] != 0 || zbuf[3] != 0) return 4;',
  '    int parsed = atoi("  -42");',
  '    int mag = abs(parsed);',
  '    buf[0] = mag + (ok ? 8 : 0);',
  '    printf("C99_STDLIB_OK val=%d\\n", buf[0]);',
  '    free(buf);',
  '    free(zbuf);',
  '    return buf[0] == 50 ? 0 : 5;',
  '}',
].join('\n');

const writeC = JSON.parse(
  studio_dispatch(`1\tstudio\tworkspace_write\tpolyglot_c99.c\n${c99Src}`),
);
assert.equal(writeC.status, 'ok');

const ccOut = runMvm('cc /workspace/polyglot_c99.c -o /workspace/polyglot_c99 && /workspace/polyglot_c99');
assert.equal(ccOut.exit_code, 0, `C99 stdlib execution failed: ${JSON.stringify(ccOut)}`);
assert.ok(ccOut.stdout.includes('C99_STDLIB_OK val=50'), `Unexpected C99 stdout: ${ccOut.stdout}`);

// 4. Verify Extended `npm` Lifecycle Subcommands (`--version`, `init`, `test`, `start`, `build`, `ls`, `uninstall`)
const bridge = new SpiralCoreHostBridge();
const baseWorkspace = [
  {
    path: 'package.json',
    content_utf8: JSON.stringify(
      {
        name: 'polyglot-demo',
        version: '1.2.0',
        scripts: {
          test: 'node -e "console.log(\'NPM_TEST_OK\')"',
          start: 'node -e "console.log(\'NPM_START_OK\')"',
          build: 'node -e "console.log(\'NPM_BUILD_OK\')"',
        },
        dependencies: {
          'tiny-pkg': '^1.0.0',
        },
      },
      null,
      2,
    ),
  },
  {
    path: 'node_modules/tiny-pkg/package.json',
    content_utf8: JSON.stringify({ name: 'tiny-pkg', version: '1.0.4' }),
  },
  {
    path: 'node_modules/tiny-pkg/index.js',
    content_utf8: 'module.exports = 42;\n',
  },
];

const npmVer = await bridge.handleNpmBridge({ args: ['--version'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmVer.exit_code, 0);
assert.ok(npmVer.stdout.includes('10.9.0-mvm'));

const npmTest = await bridge.handleNpmBridge({ args: ['test'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmTest.exit_code, 0, `npm test failed: ${JSON.stringify(npmTest)}`);
assert.ok(npmTest.stdout.includes('NPM_TEST_OK'));

const npmStart = await bridge.handleNpmBridge({ args: ['start'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmStart.exit_code, 0);
assert.ok(npmStart.stdout.includes('NPM_START_OK'));

const npmBuild = await bridge.handleNpmBridge({ args: ['build'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmBuild.exit_code, 0);
assert.ok(npmBuild.stdout.includes('NPM_BUILD_OK'));

const npmLs = await bridge.handleNpmBridge({ args: ['ls'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmLs.exit_code, 0);
assert.ok(npmLs.stdout.includes('polyglot-demo@1.2.0') && npmLs.stdout.includes('tiny-pkg@1.0.4'));

const npmUninstall = await bridge.handleNpmBridge({ args: ['uninstall', 'tiny-pkg'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmUninstall.exit_code, 0);
assert.ok(npmUninstall.stdout.includes('removed 1 package(s)'));
assert.ok(npmUninstall.deletes.includes('node_modules/tiny-pkg/index.js'));

const npmBad = await bridge.handleNpmBridge({ args: ['frobnicate'], workspace: baseWorkspace, cwd: '/workspace' });
assert.equal(npmBad.exit_code, 1);
assert.ok(npmBad.stderr.includes('RefusedUnsupportedToolchainSubcommand'));

const npmProtected = await bridge.handleNpmBridge({
  args: ['install', '/workspace/prism/Kernel.lex.tex'],
  workspace: baseWorkspace,
  cwd: '/workspace',
});
assert.equal(npmProtected.ok, false);
assert.equal(npmProtected.error_code, 'CanonicalPathProtected');

console.log('[PASS] R4-67 MVM cargo/rustc, npm lifecycle subcommands, and C99 <stdlib.h>/<stdbool.h> verified.');
