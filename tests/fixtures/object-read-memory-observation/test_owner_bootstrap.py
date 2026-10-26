"""Actual isolated bootstrap with finite invalid adapter arguments only."""

import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

import observer_owner as owner


class BootstrapTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(hasattr(owner, '_BOOTSTRAP'), 'fixed bootstrap missing')
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()/'source'
        self.root.mkdir(mode=0o700)
        for name, _, _ in owner._MODULES:
            shutil.copyfile(Path(owner.__file__).with_name(name+'.py'), self.root/(name+'.py'))

    def run_bootstrap(self, script=None):
        # Test-only literal relocation; no production root/program argument.
        script = (owner._BOOTSTRAP if script is None else script).replace(
            repr(owner._SOURCE_ROOT), repr(str(self.root)))
        return subprocess.run([sys.executable, '-I', '-B', '-c', script],
                              capture_output=True, timeout=5, env={})

    def test_exact_manifest_and_isolated_entry_reached(self):
        for name, length, digest in owner._MODULES:
            raw = (self.root/(name+'.py')).read_bytes()
            self.assertEqual((len(raw), hashlib.sha256(raw).hexdigest()), (length, digest))
        result = self.run_bootstrap()
        self.assertEqual(result.returncode, 64, result.stderr)
        self.assertEqual(result.stdout, b'')
        self.assertEqual(result.stderr, b'observer-entry: arguments\n')
        self.assertEqual(list(self.root.glob('**/*.pyc')), [])

    def test_source_hash_missing_symlink_and_root_custody_reject(self):
        source = self.root/'observe.py'; raw = source.read_bytes()
        source.write_bytes(raw[:-1]+b' ')
        self.assertEqual(self.run_bootstrap().returncode, 74)
        source.write_bytes(raw+b'\n')
        self.assertEqual(self.run_bootstrap().returncode, 74)
        source.unlink()
        self.assertEqual(self.run_bootstrap().returncode, 74)
        source.symlink_to(Path(owner.__file__).with_name('observe.py'))
        self.assertEqual(self.run_bootstrap().returncode, 74)
        source.unlink(); source.write_bytes(raw)
        self.root.chmod(0o777)
        self.assertEqual(self.run_bootstrap().returncode, 74)

    def test_applicable_and_legacy_caches_reject_without_deleting(self):
        import importlib.util
        for path in (Path(importlib.util.cache_from_source(str(self.root/'observe.py'))),
                     self.root/'observe.pyc'):
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(b'fixture cache')
            self.assertEqual(self.run_bootstrap().returncode, 74)
            self.assertEqual(path.read_bytes(), b'fixture cache')
            path.unlink()

    def test_unexpected_project_dependency_rejects(self):
        source = self.root/'observer_entry.py'; before = source.read_bytes()
        after = b'import unwanted\n'+before
        source.write_bytes(after); (self.root/'unwanted.py').write_text('value = 1\n')
        script = owner._BOOTSTRAP.replace(repr(owner._MODULES), repr(tuple(
            (name, len(after), hashlib.sha256(after).hexdigest()) if name == 'observer_entry'
            else (name, size, digest) for name, size, digest in owner._MODULES)))
        result = self.run_bootstrap(script)
        self.assertEqual(result.returncode, 74, result.stderr)


if __name__ == '__main__':
    unittest.main()
