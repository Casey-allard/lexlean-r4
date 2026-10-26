"""Tiny owned host filesystem fixtures; no VM, SDK or live proc sampling."""

import os
import pathlib
import tempfile
import types
import unittest
from unittest import mock

import observe


def pin(path):
    item = os.stat(path, follow_symlinks=False)
    return (item.st_dev, item.st_ino, item.st_uid, item.st_mode)


class OwnedIOTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='r4-owned-io-')
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.owned = self.root / 'owned'
        self.owned.mkdir(mode=0o700)

    def test_open_directory_binds_exact_identity_without_inheritable_fd(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        try:
            actual = os.fstat(fd)
            self.assertEqual((actual.st_dev, actual.st_ino,
                              actual.st_uid, actual.st_mode), expected)
            self.assertFalse(os.get_inheritable(fd))
        finally:
            os.close(fd)

    def test_intermediate_and_final_symlink_are_never_followed(self):
        (self.root / 'alias').symlink_to(self.owned, target_is_directory=True)
        (self.owned / 'child').mkdir()
        for path in (self.root / 'alias', self.root / 'alias' / 'child'):
            expected = pin(path.resolve())
            fd = None
            try:
                with self.subTest(path=path), self.assertRaises(OSError):
                    fd = observe.open_pinned_directory(str(path), expected)
            finally:
                if fd is not None:
                    os.close(fd)

    def test_invalid_path_or_pin_rejects_before_open(self):
        expected = pin(self.owned)
        bad_paths = ('relative', '/', '/a/', '/a//b', '/a/./b', '/a/../b',
                     '/a\0b', '/' + 'a' * 4096, '/' + 'é' * 2048,
                     '/' + '/'.join(['a'] * 65), b'/a', None)
        bad_pins = (None, list(expected), expected[:3], (True, *expected[1:]),
                    (-1, *expected[1:]), (2**64, *expected[1:]),
                    (*expected[:3], 0o100600), (*expected[:3], 2**64))
        calls = []

        def fail_open(*args, **kwargs):
            calls.append(args)
            raise AssertionError('invalid request reached open')

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open = fail_open
        with mock.patch.object(observe, 'os', proxy):
            for path, identity in ([(x, expected) for x in bad_paths]
                                   + [(str(self.owned), x) for x in bad_pins]):
                with self.subTest(path=repr(path)[:30], identity=identity):
                    with self.assertRaises(observe.ObservationError):
                        observe.open_pinned_directory(path, identity)
        self.assertEqual(calls, [])

    def test_primary_and_cleanup_failures_all_survive_in_order(self):
        primary = OSError('synthetic fstat failure')
        first = KeyboardInterrupt('synthetic close interrupt')
        second = OSError('synthetic later close failure')
        opened, closed = [], []

        def fake_open(*args, **kwargs):
            fd = 100 + len(opened)
            opened.append(fd)
            return fd

        def fake_close(fd):
            closed.append(fd)
            if fd == 102:
                raise first
            if fd == 101:
                raise second

        def fake_stat(fd):
            raise primary

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open, proxy.close, proxy.fstat = fake_open, fake_close, fake_stat
        caught = None
        with mock.patch.object(observe, 'os', proxy):
            try:
                observe.open_pinned_directory('/a/b', (1, 2, 3, 0o40700))
            except BaseException as error:
                caught = error
        self.assertIsInstance(caught, BaseExceptionGroup)
        self.assertEqual(caught.exceptions, (primary, first, second))
        self.assertEqual(closed, [102, 101, 100])
        self.assertEqual(len(set(closed)), len(opened))

    def test_fixed_observation_read_keeps_caller_directory_open(self):
        data = b'low 0\nhigh 0\nmax 6325\noom 0\noom_kill 0\noom_group_kill 0\n'
        (self.owned / 'memory.events').write_bytes(data)
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        try:
            self.assertEqual(observe.read_observation_file(
                fd, expected, 'memory.events'), data)
            self.assertEqual(os.fstat(fd).st_ino, expected[1])
        finally:
            os.close(fd)

    def test_file_rejects_fifo_and_directory_before_reading(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        os.mkfifo(self.owned / 'memory.events', 0o600)
        (self.owned / 'memory.stat').mkdir()
        try:
            for name in ('memory.events', 'memory.stat'):
                with self.subTest(name=name), self.assertRaisesRegex(
                        observe.ObservationError, 'observation_file_type'):
                    observe.read_observation_file(fd, expected, name)
        finally:
            os.close(fd)

    def test_fixed_file_request_requires_valid_pin_and_name_before_open(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        calls = []

        def fail_open(*args, **kwargs):
            calls.append(args)
            raise AssertionError('invalid file request reached open')

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open = fail_open
        cases = [(fd, expected, name) for name in
                 ('environ', '../stat', '/stat', '', None, ['stat'])]
        cases += [(True, expected, 'stat'), (-1, expected, 'stat'),
                  (fd, None, 'stat'), (fd, list(expected), 'stat'),
                  (fd, (expected[0], expected[1] + 1, *expected[2:]), 'stat')]
        try:
            with mock.patch.object(observe, 'os', proxy):
                for descriptor, identity, name in cases:
                    with self.subTest(name=name, descriptor=descriptor):
                        with self.assertRaises(observe.ObservationError):
                            observe.read_observation_file(descriptor, identity,
                                                          name)
            self.assertEqual(calls, [])
        finally:
            os.close(fd)

    def test_file_primary_wrapper_and_raw_close_failures_are_preserved(self):
        (self.owned / 'memory.events').write_bytes(b'x')
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        primary, wrapper, raw = OSError('read'), SystemExit('wrapper'), OSError('raw')
        events = []

        class Stream:
            def read(self, size):
                events.append('read')
                raise primary

            def close(self):
                events.append('wrapper')
                raise wrapper

            def __enter__(self):
                return self

            def __exit__(self, *args):
                self.close()

        def close(file_fd):
            self.assertNotEqual(file_fd, fd)
            events.append('raw')
            os.close(file_fd)
            raise raw

        proxy = types.SimpleNamespace(**vars(os))
        proxy.close = close
        fake_io = types.SimpleNamespace(FileIO=lambda *a, **kw: Stream())
        caught = None
        try:
            with mock.patch.object(observe, 'os', proxy), mock.patch.object(
                    observe, 'io', fake_io):
                try:
                    observe.read_observation_file(fd, expected, 'memory.events')
                except BaseException as error:
                    caught = error
            self.assertIsInstance(caught, BaseExceptionGroup)
            self.assertEqual(caught.exceptions, (primary, wrapper, raw))
            self.assertEqual(events, ['read', 'wrapper', 'raw'])
            os.fstat(fd)
        finally:
            os.close(fd)

    def test_directory_change_during_read_rejects_and_closes_file(self):
        (self.owned / 'memory.current').write_bytes(b'7\n')
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        file_fds = []

        def make_stream(file_fd, *args, **kwargs):
            file_fds.append(file_fd)
            os.chmod(self.owned, 0o750)
            return __import__('io').FileIO(file_fd, *args, **kwargs)

        try:
            with mock.patch.object(observe, 'io', types.SimpleNamespace(
                    FileIO=make_stream)), self.assertRaisesRegex(
                    observe.ObservationError, 'directory_identity'):
                observe.read_observation_file(fd, expected, 'memory.current')
            for file_fd in file_fds:
                with self.assertRaises(OSError):
                    os.fstat(file_fd)
            os.fstat(fd)
        finally:
            os.close(fd)

    def test_owned_cgroup_children_are_bounded_without_following_links(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        try:
            self.assertIsNone(observe.check_no_descendants(fd, expected))
            for number in range(64):
                (self.owned / str(number)).touch()
            self.assertIsNone(observe.check_no_descendants(fd, expected))
            extra = self.owned / 'extra'
            extra.touch()
            with self.assertRaisesRegex(observe.ObservationError, 'child_count'):
                observe.check_no_descendants(fd, expected)
            extra.unlink()
            for number in range(64):
                (self.owned / str(number)).unlink()
            extra.mkdir()
            with self.assertRaisesRegex(observe.ObservationError, 'child_type'):
                observe.check_no_descendants(fd, expected)
            extra.rmdir()
            extra.symlink_to(self.root)
            with self.assertRaisesRegex(observe.ObservationError, 'child_type'):
                observe.check_no_descendants(fd, expected)
            os.fstat(fd)
        finally:
            os.close(fd)

    def test_snapshot_opens_only_census_pid_and_preserves_owned_roots(self):
        from test_identity import stat, status
        proc = self.root / 'proc'
        proc.mkdir()
        child = proc / '41'
        child.mkdir()
        for name, data in {'stat': stat(), 'status': status(),
                           'cgroup': b'0::/owned\n', 'cmdline': b'lean\0'}.items():
            (child / name).write_bytes(data)
        (self.owned / 'cgroup.procs').write_bytes(b'41\n')
        cp, pp = pin(self.owned), pin(proc)
        cfd = observe.open_pinned_directory(str(self.owned), cp)
        pfd = observe.open_pinned_directory(str(proc), pp)
        try:
            row = observe.owned_process_snapshot((cfd, cp), (pfd, pp), '41', b'/owned')
            self.assertEqual(row['start'], '900')
            self.assertEqual(row['cmdline'], b'lean\0')
            self.assertEqual(row['attribution'], 'owned_unattributed')
            with self.assertRaisesRegex(observe.ObservationError, 'pid_not_in_census'):
                observe.owned_process_snapshot((cfd, cp), (pfd, pp), '42', b'/owned')
            os.fstat(cfd)
            os.fstat(pfd)
        finally:
            os.close(cfd)
            os.close(pfd)

    def test_each_file_cap_and_symlink_boundary(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        caps = {'cgroup.procs': 4096, 'stat': 4096, 'status': 8192,
                'cmdline': 4096, 'cgroup': 4096, 'memory.events': 4096,
                'memory.current': 64, 'memory.swap.current': 64,
                'memory.peak': 64, 'memory.stat': 16384,
                'memory.pressure': 4096, 'memory.events.local': 4096,
                'boot_id': 64}
        try:
            for name, cap in caps.items():
                with self.subTest(name=name):
                    path = self.owned / name
                    path.write_bytes(b'x' * cap)
                    self.assertEqual(observe.read_observation_file(
                        fd, expected, name), b'x' * cap)
                    path.write_bytes(b'x' * (cap + 1))
                    with self.assertRaisesRegex(observe.ObservationError,
                                                'truncated_read'):
                        observe.read_observation_file(fd, expected, name)
                    path.unlink()
                    path.symlink_to(self.root / 'missing')
                    with self.assertRaises(OSError):
                        observe.read_observation_file(fd, expected, name)
        finally:
            os.close(fd)

    def test_descriptor_does_not_claim_current_path_identity(self):
        (self.owned / 'memory.current').write_bytes(b'old\n')
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        self.owned.rename(self.root / 'retained')
        self.owned.mkdir(mode=0o700)
        (self.owned / 'memory.current').write_bytes(b'new\n')
        try:
            self.assertEqual(observe.read_observation_file(
                fd, expected, 'memory.current'), b'old\n')
            with self.assertRaisesRegex(observe.ObservationError,
                                        'directory_identity'):
                observe.open_pinned_directory(str(self.owned), expected)
        finally:
            os.close(fd)

    def test_wrapper_construction_failure_closes_raw_fd_once(self):
        (self.owned / 'memory.current').write_bytes(b'0\n')
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        failure = KeyboardInterrupt('construct')
        acquired, closed = [], []

        def construct(file_fd, *args, **kwargs):
            acquired.append(file_fd)
            raise failure

        def close(file_fd):
            closed.append(file_fd)
            os.close(file_fd)

        proxy = types.SimpleNamespace(**vars(os))
        proxy.close = close
        try:
            with mock.patch.object(observe, 'os', proxy), mock.patch.object(
                    observe, 'io', types.SimpleNamespace(FileIO=construct)):
                with self.assertRaises(KeyboardInterrupt) as caught:
                    observe.read_observation_file(fd, expected, 'memory.current')
                self.assertIs(caught.exception, failure)
            self.assertEqual(acquired, closed)
            self.assertEqual(len(closed), 1)
            os.fstat(fd)
        finally:
            os.close(fd)

    def test_directory_open_failure_closes_exact_acquired_prefix(self):
        for stop in range(3):
            opened, closed = [], []
            failure = OSError('open interrupted')

            def fake_open(*args, **kwargs):
                if len(opened) == stop:
                    raise failure
                descriptor = 100 + len(opened)
                opened.append(descriptor)
                return descriptor

            proxy = types.SimpleNamespace(**vars(os))
            proxy.open, proxy.close = fake_open, closed.append
            with self.subTest(stop=stop), mock.patch.object(observe, 'os', proxy):
                with self.assertRaises(OSError) as caught:
                    observe.open_pinned_directory('/a/b', (1, 2, 3, 0o40700))
                self.assertIs(caught.exception, failure)
                self.assertEqual(closed, list(reversed(opened)))

    def test_parent_close_failure_also_closes_leaf_and_remaining_parents(self):
        opened, closed = [], []
        failure = OSError('parent close')

        def fake_open(*args, **kwargs):
            descriptor = 100 + len(opened)
            opened.append(descriptor)
            return descriptor

        def close(descriptor):
            closed.append(descriptor)
            if descriptor == 101:
                raise failure

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open, proxy.close = fake_open, close
        proxy.fstat = lambda _: types.SimpleNamespace(
            st_dev=1, st_ino=2, st_uid=3, st_mode=0o40700)
        with mock.patch.object(observe, 'os', proxy):
            with self.assertRaises(OSError) as caught:
                observe.open_pinned_directory('/a/b', (1, 2, 3, 0o40700))
            self.assertIs(caught.exception, failure)
        self.assertEqual(closed, [101, 100, 102])

    def test_child_lookup_and_iterator_close_failures_both_survive(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        first, second = FileNotFoundError('vanished'), SystemExit('iterator')
        closed = []

        class Entries:
            def __iter__(self):
                yield types.SimpleNamespace(name='vanished')

            def close(self):
                closed.append('iterator')
                raise second

        def lookup(*args, **kwargs):
            self.assertEqual(kwargs, {'dir_fd': fd, 'follow_symlinks': False})
            raise first

        proxy = types.SimpleNamespace(**vars(os))
        proxy.scandir, proxy.stat = lambda _: Entries(), lookup
        try:
            with mock.patch.object(observe, 'os', proxy):
                with self.assertRaises(BaseExceptionGroup) as caught:
                    observe.check_no_descendants(fd, expected)
            self.assertEqual(caught.exception.exceptions, (first, second))
            self.assertEqual(closed, ['iterator'])
            os.fstat(fd)
        finally:
            os.close(fd)

    def test_entry_stat_is_fresh_and_overflow_never_stats_65th_entry(self):
        expected = pin(self.owned)
        fd = observe.open_pinned_directory(str(self.owned), expected)
        looked_up, yielded, closed = [], [], []

        class Entries:
            def __iter__(self):
                for number in range(100):
                    yielded.append(number)
                    yield types.SimpleNamespace(name=str(number))

            def close(self):
                closed.append(True)

        def lookup(name, **kwargs):
            looked_up.append(name)
            self.assertEqual(kwargs, {'dir_fd': fd, 'follow_symlinks': False})
            return types.SimpleNamespace(st_mode=0o100600)

        proxy = types.SimpleNamespace(**vars(os))
        proxy.scandir, proxy.stat = lambda _: Entries(), lookup
        try:
            with mock.patch.object(observe, 'os', proxy):
                with self.assertRaisesRegex(observe.ObservationError, 'child_count'):
                    observe.check_no_descendants(fd, expected)
            self.assertEqual(len(looked_up), 64)
            self.assertEqual(len(yielded), 65)
            self.assertEqual(closed, [True])
            os.fstat(fd)
        finally:
            os.close(fd)


if __name__ == '__main__':
    unittest.main()
