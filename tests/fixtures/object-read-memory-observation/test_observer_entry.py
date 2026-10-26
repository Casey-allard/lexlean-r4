"""Concrete config-to-reader tests on owned files; no guest or SDK execution."""

import copy
import importlib.util
import io
import json
import os
import pathlib
import stat
import unittest
from unittest import mock

import boundary
import control_slots
import observe
import observer_session
import test_live_context
import test_sample
from test_owned_io import pin

if importlib.util.find_spec('observer_entry'):
    import observer_entry as entry
else:
    entry = None


class EntryTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(entry, 'functional guest entry is missing')
        self.fixture = test_live_context.ContextTests()
        self.fixture.fixture(); self.addCleanup(self.fixture.doCleanups)
        self.parent = self.fixture.root/'config'; self.parent.mkdir(mode=0o700)
        self.output = self.fixture.root/'output'; self.output.mkdir(mode=0o700)
        self.config = self.parent/'observer-config.json'
        spec = self.fixture.spec
        wire = lambda pair: [pair[0], [str(n) for n in pair[1]]]
        self.data = {'schema': 'lexlean-r4/observer-config/1', 'nonce': 'c'*64,
                     'context': {'docker': list(spec.docker), 'boot': spec.boot,
                     'init_start': spec.init_start, 'cgroup': wire(spec.cgroup),
                     'proc': wire(spec.proc), 'boot_directory': wire(spec.boot_directory),
                     'target': spec.target.decode()},
                     'output_directory': wire((str(self.output), pin(self.output)))}
        self.argv = ['--config-directory', str(self.parent), '--parent-pin',
                     *[str(n) for n in pin(self.parent)]]
        self.write()

    def write(self, raw=None):
        self.config.write_bytes(json.dumps(self.data).encode() if raw is None else raw)
        self.config.chmod(0o600)

    def reject(self, action, code):
        with self.assertRaises(entry.EntryError) as caught: action()
        self.assertEqual(caught.exception.code, code)
        return caught.exception

    def test_mapping_preserves_context_and_large_unsigned_pins_without_io(self):
        with mock.patch.object(os, 'open') as opened:
            spec, output, nonce = entry.decode_config(self.config.read_bytes())
        opened.assert_not_called()
        self.assertEqual(spec, self.fixture.spec)
        self.assertEqual(output, (str(self.output), pin(self.output)))
        self.assertEqual(nonce, 'c'*64)
        for value in ('0', str(2**53+1), str(2**64-1)):
            data = copy.deepcopy(self.data); data['context']['cgroup'][1][0] = value
            got = entry.decode_config(json.dumps(data).encode())[0]
            self.assertEqual(got.cgroup[1][0], int(value))

    def test_schema_keys_and_exact_types(self):
        for keys in ((), ('context',)):
            for name in (self.data if not keys else self.data['context']):
                for change in ('missing', 'extra'):
                    data = copy.deepcopy(self.data); part = data if not keys else data['context']
                    if change == 'missing': del part[name]
                    else: part['unrecognized'] = None
                    with self.subTest(keys=keys, name=name, change=change):
                        self.reject(lambda: entry.decode_config(json.dumps(data).encode()), 65)
        for bad in (None, False, 0, '', [], {}, {'schema': 'wrong'}):
            self.reject(lambda: entry.decode_config(json.dumps(bad).encode()), 65)

    def test_json_protocol_bounds_duplicates_and_invalid_encoding(self):
        raw = self.config.read_bytes()
        invalid = [b'', b'\xef\xbb\xbf'+raw, b'\xff', raw+b'{}', b' '*16385,
                   b'['*1100+b']'*1100, b'{"x":NaN}', b'{"x":Infinity}',
                   b'{"x":1.0}', b'{"x":'+b'9'*100+b'}',
                   raw.replace(b'"schema":', b'"schema":"wrong","schema":', 1),
                   raw.replace(b'"boot":', b'"boot":"wrong","boot":', 1),
                   raw.replace(b'"target": "/owned"', b'"target":"\\ud800"')]
        for bad in invalid:
            with self.subTest(raw=bad[:60]): self.reject(lambda: entry.decode_config(bad), 65)

    def test_wire_pin_numeric_falsey_and_range_negatives(self):
        for bad in (None, False, 0, 1, '', '-1', '+1', '01', ' 1', '1 ', '1.0',
                    '1e2', str(2**64), '\u0661'):
            data = copy.deepcopy(self.data); data['context']['cgroup'][1][0] = bad
            with self.subTest(value=bad):
                self.reject(lambda: entry.decode_config(json.dumps(data).encode()), 65)
        for field, bad in (('docker', ['bad']*4), ('boot', 'BAD'), ('init_start', '-1'),
                           ('target', 'relative'), ('target', '/bad\npath')):
            data = copy.deepcopy(self.data); data['context'][field] = bad
            self.reject(lambda: entry.decode_config(json.dumps(data).encode()), 65)

    def test_directory_rules_are_distinct_from_membership(self):
        for path in ('/', 'relative', '/a//b', '/a/../b', '/a/./b', '/a/',
                     '/bad\npath', '/bad\rpath', '/bad\x00path', '/'+('x/'*65)+'x',
                     '/'+'\u00e9'*2048):
            data = copy.deepcopy(self.data); data['context']['proc'][0] = path
            self.reject(lambda: entry.decode_config(json.dumps(data).encode()), 65)
        for mode in (str(stat.S_IFREG|0o700), '65536', str(stat.S_IFDIR|0o755)):
            data = copy.deepcopy(self.data); data['output_directory'][1][3] = mode
            self.reject(lambda: entry.decode_config(json.dumps(data).encode()), 65)

    def test_bad_arguments_reject_before_any_open(self):
        variants = [None, '', [], self.argv+['extra'], self.argv[:-1],
                    ['wrong', *self.argv[1:]], [*self.argv[:3], False, *self.argv[4:]],
                    [*self.argv[:-1], str(stat.S_IFDIR|0o755)],
                    [*self.argv[:5], str(os.getuid()+1), self.argv[6]]]
        with mock.patch.object(os, 'open') as opened:
            for argv in variants: self.reject(lambda: entry.main(argv), 64)
        opened.assert_not_called()

    def test_config_policy_never_opens_observation_roots(self):
        self.data['output_directory'][1][2] = str(os.getuid()+1); self.write()
        with mock.patch.object(observer_session, 'run_owned') as reader:
            self.reject(lambda: entry.main(self.argv), 65)
        reader.assert_not_called()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_real_config_custody_and_size_limits(self):
        raw = self.config.read_bytes()
        self.write(raw+b' '*(16384-len(raw)))
        self.assertEqual(entry.load_config(self.argv)[0], self.fixture.spec)
        for bad in (b'', b' '*16385):
            self.write(bad); self.reject(lambda: entry.load_config(self.argv), 74)
        self.write(); self.config.chmod(0o644)
        self.reject(lambda: entry.load_config(self.argv), 74)
        self.config.chmod(0o600)
        self.parent.chmod(0o755)
        self.reject(lambda: entry.load_config(self.argv), 74)

    def test_symlink_hardlink_fifo_and_directory_are_not_configs(self):
        saved = self.parent/'saved'; self.config.rename(saved)
        for kind in ('symlink', 'hardlink', 'fifo', 'directory'):
            with self.subTest(kind=kind):
                if kind == 'symlink': self.config.symlink_to(saved)
                elif kind == 'hardlink': os.link(saved, self.config)
                elif kind == 'fifo': os.mkfifo(self.config, 0o600)
                else: self.config.mkdir(mode=0o600)
                try:
                    self.reject(lambda: entry.load_config(self.argv), 74)
                finally:
                    if kind == 'directory': self.config.rmdir()
                    else: self.config.unlink()

    def test_parent_path_replacement_is_detected(self):
        read = observe.read_bounded
        def reading(*args):
            raw = read(*args); self.parent.rename(self.fixture.root/'old-config')
            self.parent.mkdir(mode=0o700); return raw
        with mock.patch.object(observe, 'read_bounded', reading):
            self.reject(lambda: entry.load_config(self.argv), 74)

    def test_config_name_replacement_is_detected(self):
        read = observe.read_bounded
        def reading(*args):
            raw = read(*args); self.config.rename(self.parent/'old-config')
            self.write(raw); return raw
        with mock.patch.object(observe, 'read_bounded', reading):
            self.reject(lambda: entry.load_config(self.argv), 74)

    def test_name_replacement_after_final_fd_stat_is_detected(self):
        checked, calls = entry._file, []
        def checking(fd):
            result = checked(fd); calls.append(fd)
            if len(calls) == 2:
                raw = self.config.read_bytes()
                self.config.rename(self.parent/'old-config'); self.write(raw)
            return result
        with mock.patch.object(entry, '_file', checking):
            self.reject(lambda: entry.load_config(self.argv), 74)

    def test_short_read_and_read_error_do_not_start_reader(self):
        for failure in (None, OSError('read failure')):
            with mock.patch.object(observe, 'read_bounded', return_value=b'x',
                                   side_effect=failure):
                self.reject(lambda: entry.main(self.argv), 74)
        self.assertEqual(list(self.output.iterdir()), [])

    def test_schema_and_close_errors_are_both_retained(self):
        self.write(b'not json'); real_close, real_open = os.close, os.open
        config_fd, closed = [], []
        class Falsey(OSError):
            def __bool__(self): return False
        failure = Falsey('config close')
        def opening(name, *args, **kwargs):
            fd = real_open(name, *args, **kwargs)
            if name == 'observer-config.json':
                config_fd.append(fd); closed.clear()
            return fd
        def closing(fd):
            closed.append(fd); real_close(fd)
            if config_fd and fd == config_fd[0]: raise failure
        with mock.patch.object(os, 'open', opening), mock.patch.object(os, 'close', closing):
            caught = self.reject(lambda: entry.load_config(self.argv), 74)
        self.assertIsInstance(caught.__cause__, BaseExceptionGroup)
        self.assertEqual(caught.__cause__.exceptions[0].code, 65)
        self.assertIs(caught.__cause__.exceptions[1], failure)
        self.assertEqual(closed.count(config_fd[0]), 1)

    def prepare_reader(self):
        files = {'memory.events': test_sample.SampleTests.events(),
                 'memory.events.local': test_sample.SampleTests.events(),
                 'memory.current': b'123\n', 'memory.swap.current': b'0\n',
                 'memory.peak': b'456\n', 'memory.stat': b'anon 100\nfile 23\n',
                 'memory.pressure': b'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n'}
        for name, data in files.items(): (self.fixture.cg/name).write_bytes(data)
        (self.fixture.proc/'41'/'cmdline').write_bytes(b'fixture-init\0')
        fd = observe.open_pinned_directory(str(self.output), pin(self.output))
        self.addCleanup(os.close, fd)
        host = control_slots.OwnedControlSlots((fd, pin(self.output)), 'host')
        host.publish(boundary.encode_request('c'*64, 1))
        return host

    def test_actual_main_drives_all_eleven_boundaries_and_closes_descriptors(self):
        host = self.prepare_reader(); sequence = 2
        def sleeping(seconds):
            nonlocal sequence
            self.fixture.now += int(seconds*1000000000)
            host.publish(boundary.encode_request('c'*64, sequence)); sequence += 1
        opened, active, calls = [], set(), []
        real_open, real_close, real_run = os.open, os.close, observer_session.run_owned
        def opening(*args, **kwargs):
            fd = real_open(*args, **kwargs); opened.append(fd); active.add(fd); return fd
        def closing(fd):
            active.discard(fd); real_close(fd)
        def running(*args):
            self.assertEqual(active, set(), 'config handles remain at reader entry')
            calls.append(args); return real_run(*args)
        with mock.patch.object(os, 'open', opening), mock.patch.object(os, 'close', closing), mock.patch.object(
                observer_session, 'run_owned', running), mock.patch.object(
                observer_session.time, 'sleep', sleeping):
            self.assertEqual(entry.main(self.argv), 0)
        self.assertEqual(len(calls), 1)
        self.assertIsNotNone(host.read_peer(), 'actual reader did not publish ACK')
        self.assertEqual(boundary.decode_ack(host.read_peer())[1], 11)
        self.assertEqual(json.loads((self.output/'guest-terminal.json').read_bytes())[5], 'ended')
        self.assertTrue((self.output/'journal.jsonl').read_bytes().startswith(b'["h",'))
        for fd in opened:
            with self.assertRaises(OSError): os.fstat(fd)

    def test_actual_reader_identity_error_and_partial_failure_are_runtime_errors(self):
        host = self.prepare_reader()
        (self.output/'request').write_bytes(b'invalid request')
        stderr = io.StringIO()
        with mock.patch('sys.stderr', stderr): self.assertEqual(entry.cli(self.argv), 70)
        self.assertEqual(json.loads((self.output/'guest-terminal.json').read_bytes())[5], 'stopped')
        self.assertLessEqual(len(stderr.getvalue().encode('ascii')), 128)
        self.assertNotIn(str(self.fixture.root), stderr.getvalue())

    def test_cli_fixed_categories_no_stdout_or_traceback(self):
        for argv, expected in (([], 64), (self.argv, 65)):
            self.write(b'invalid json'); out, err = io.StringIO(), io.StringIO()
            with mock.patch('sys.stdout', out), mock.patch('sys.stderr', err):
                self.assertEqual(entry.cli(argv), expected)
            self.assertEqual(out.getvalue(), '')
            self.assertLessEqual(len(err.getvalue().encode('ascii')), 128)
            self.assertNotIn('Traceback', err.getvalue())

    def test_import_has_no_owned_effect(self):
        source = pathlib.Path(entry.__file__).read_text()
        code = compile(source, entry.__file__, 'exec')
        with mock.patch.object(os, 'open') as opened, mock.patch.object(
                observer_session, 'run_owned') as reader:
            exec(code, {'__name__': 'entry_import_probe'})
        opened.assert_not_called(); reader.assert_not_called()

    def test_config_read_requests_are_bounded_and_stream_setup_failure_closes(self):
        fileio = entry.io.FileIO; requested = []
        class Stream:
            def __init__(self, *args, **kwargs): self.stream = fileio(*args, **kwargs)
            def read(self, size=-1): requested.append(size); return self.stream.read(size)
            def close(self): self.stream.close()
        with mock.patch.object(entry.io, 'FileIO', Stream):
            self.assertEqual(entry.load_config(self.argv)[0], self.fixture.spec)
        self.assertTrue(requested)
        self.assertTrue(all(0 < size <= 16385 for size in requested))
        with mock.patch.object(entry.io, 'FileIO', side_effect=OSError('setup')):
            self.reject(lambda: entry.load_config(self.argv), 74)

    def test_unexpected_adapter_internal_error_is_not_config_io(self):
        out = io.StringIO()
        with (mock.patch.object(observe, 'read_bounded', side_effect=RuntimeError('internal')),
              mock.patch('sys.stderr', out)):
            self.assertEqual(entry.cli(self.argv), 70)
        self.assertEqual(out.getvalue(), 'observer-entry: reader-or-internal\n')

    def test_actual_reader_open_failure_and_boot_mismatch_are_code70(self):
        real_open = os.open
        def opening(name, *args, **kwargs):
            if name == 'cg': raise OSError('reader root')
            return real_open(name, *args, **kwargs)
        with mock.patch.object(os, 'open', opening), mock.patch('sys.stderr', io.StringIO()):
            self.assertEqual(entry.cli(self.argv), 70)
        self.fixture.boot.joinpath('boot_id').write_bytes(b'wrong\n')
        with mock.patch('sys.stderr', io.StringIO()): self.assertEqual(entry.cli(self.argv), 70)
        self.assertEqual(list(self.output.iterdir()), [])

    def test_config_interrupt_closes_descriptors_and_is_not_swallowed(self):
        opened, real_open = [], os.open
        def opening(*args, **kwargs):
            fd = real_open(*args, **kwargs); opened.append(fd); return fd
        interrupt = KeyboardInterrupt()
        with mock.patch.object(os, 'open', opening), mock.patch.object(
                observe, 'read_bounded', side_effect=interrupt):
            with self.assertRaises(KeyboardInterrupt) as caught: entry.cli(self.argv)
        self.assertIs(caught.exception, interrupt)
        for fd in opened:
            with self.assertRaises(OSError): os.fstat(fd)

    def test_failed_error_reporting_cannot_return_success(self):
        with mock.patch('sys.stderr.write', side_effect=OSError('stderr')):
            with self.assertRaises(OSError): entry.cli([])


if __name__ == '__main__':
    unittest.main()
