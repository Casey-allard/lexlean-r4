"""Owned synthetic parent observations; no live process or invocation claims."""

import base64
import json
import os
import types
import unittest
from unittest import mock

import boundary
import observe
import family
import test_sample
from test_identity import stat, status


class OwnedFamilyTests(unittest.TestCase):
    setUp = test_sample.SampleTests.setUp
    events = staticmethod(test_sample.SampleTests.events)
    add_pid = test_sample.SampleTests.add_pid
    clock = test_sample.SampleTests.clock
    read_identity = test_sample.SampleTests.read_identity
    sampler = test_sample.SampleTests.sampler

    def parent(self):
        self.add_pid('7')
        (self.proc/'7'/'stat').write_bytes(stat('7', '1', '100'))
        (self.proc/'7'/'status').write_bytes(status('7', '1'))
        (self.cg/'cgroup.procs').write_bytes(b'41\n7\n')

    def capture(self):
        self.sampler().capture_boundary(boundary.encode_request(self.nonce, 1))
        return [json.loads(line) for line in self.stream.getvalue().splitlines()]

    def test_owned_parent_identity_and_same_read_raw_bytes_reach_journal(self):
        self.parent()
        rows = self.capture()
        child = next(row for row in rows if row[0] == 'i' and row[2] == '41')
        self.assertEqual(child[5], '100')
        self.assertEqual(rows[0][1], 3)
        self.assertEqual(child[-1][0], 'observed')
        raw = child[-1][1]
        self.assertEqual([name for name, data in raw],
                         ['stat', 'cgroup', 'status', 'cmdline', 'stat', 'cgroup'])
        for name, data in raw:
            self.assertEqual(base64.b64decode(data), (self.proc/'7'/name).read_bytes())
        self.assertEqual(child[6:8], ['owned_unattributed', None])

    def test_parent_observation_failure_is_typed_and_preserves_unknown_edge(self):
        self.parent()
        original = observe.read_observation_file
        calls = 0

        def read(fd, pin, name, **options):
            nonlocal calls
            data = original(fd, pin, name, **options)
            if name == 'stat' and data.startswith(b'7 '):
                calls += 1
                if calls == 1:
                    raise FileNotFoundError(2, 'synthetic parent vanished')
            return data

        with mock.patch.object(observe, 'read_observation_file', read):
            rows = self.capture()
        child = next(row for row in rows if row[0] == 'i' and row[2] == '41')
        self.assertIsNone(child[5])
        self.assertEqual(child[-1], ['unavailable', 'vanished', 2, []])
        self.assertEqual(child[6:8], ['owned_unattributed', None])

    def snapshot(self):
        return family.owned_family_evidence((self.cfd, self.cpin),
                                           (self.pfd, self.ppin), '41', b'/owned')

    def test_raw_transcript_has_exact_nested_order_and_borrowed_roots_survive(self):
        self.parent()
        child, parent, failure, raw = self.snapshot()
        self.assertIsNone(failure)
        self.assertEqual(child['parent_start'], parent['start'])
        self.assertEqual([(p, n) for p, n, _ in raw],
                         [('41', n) for n in ('stat', 'cgroup', 'status', 'cmdline')]
                         + [('7', n) for n in ('stat', 'cgroup', 'status', 'cmdline',
                                               'stat', 'cgroup')]
                         + [('41', n) for n in ('stat', 'cgroup')])
        os.fstat(self.cfd)
        os.fstat(self.pfd)

    def test_outside_parent_self_and_grandparent_are_not_opened(self):
        for mode in ('outside', 'self', 'grandparent'):
            with self.subTest(mode=mode):
                if mode == 'self':
                    (self.proc/'41'/'stat').write_bytes(stat('41', '41', '900'))
                    (self.proc/'41'/'status').write_bytes(status('41', '41'))
                if mode == 'grandparent':
                    (self.proc/'41'/'stat').write_bytes(stat('41', '7', '900'))
                    (self.proc/'41'/'status').write_bytes(status('41', '7'))
                    self.parent()
                    (self.cg/'cgroup.procs').write_bytes(b'41\n7\n1\n')
                result = self.snapshot()
                self.assertEqual({p for p, _, _ in result[3]},
                                 {'41', '7'} if mode == 'grandparent' else {'41'})

    def test_parent_change_invalidates_edge_and_child_change_is_fatal(self):
        self.parent()
        original = observe.read_observation_file
        for who in ('7', '41'):
            seen = 0

            def read(fd, pin, name, **options):
                nonlocal seen
                data = original(fd, pin, name, **options)
                if name == 'stat' and data.startswith(who.encode()+b' '):
                    seen += 1
                    if seen == 2:
                        return stat(who, '8', '999')
                return data

            with self.subTest(who=who), mock.patch.object(observe, 'read_observation_file', read):
                if who == '41':
                    with self.assertRaisesRegex(observe.ObservationError, 'raced_identity'):
                        self.snapshot()
                else:
                    child, parent, failure, raw = self.snapshot()
                    self.assertIsNone(child['parent_start'])
                    self.assertIsNone(parent)
                    self.assertIsInstance(failure, observe.ObservationError)
                    self.assertEqual(raw[-1][:2], ('41', 'cgroup'))

    def test_directory_closes_all_preserving_falsey_parent_and_close_errors(self):
        self.parent()
        class Falsey(OSError):
            def __bool__(self):
                return False
        primary, close_error = Falsey('parent read'), OSError('directory close')
        original = observe.read_observation_file
        opened, closed = [], []

        def read(fd, pin, name, **options):
            data = original(fd, pin, name, **options)
            if name == 'stat' and data.startswith(b'7 '):
                raise primary
            return data

        def opening(*args, **kwargs):
            fd = os.open(*args, **kwargs)
            opened.append(fd)
            return fd

        def closing(fd):
            os.close(fd)
            if fd in opened:
                closed.append(fd)
                if len(closed) == 1:
                    raise close_error

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open = opening
        observer_proxy = types.SimpleNamespace(**vars(os))
        observer_proxy.close = closing
        with mock.patch.object(family, 'os', proxy), mock.patch.object(
                observe, 'read_observation_file', read), mock.patch.object(
                observe, 'os', observer_proxy), self.assertRaises(BaseExceptionGroup) as caught:
            self.snapshot()
        self.assertEqual(caught.exception.exceptions, (primary, close_error))
        self.assertEqual(closed, list(reversed(opened)))

    def test_parent_raw_change_changes_child_dictionary_and_oversize_stops_ack(self):
        self.parent()
        sampler = self.sampler()
        sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        (self.proc/'7'/'cmdline').write_bytes(b'changed\0')
        sampler.capture_boundary(boundary.encode_request(self.nonce, 2))
        rows = [json.loads(x) for x in self.stream.getvalue().splitlines()]
        children = [r for r in rows if r[0] == 'i' and r[2] == '41']
        self.assertEqual(len(children), 2)
        self.assertNotEqual(children[0][1], children[1][1])
        (self.proc/'7'/'cmdline').write_bytes(b'p' * 4096)
        (self.proc/'41'/'cmdline').write_bytes(b'c' * 4096)
        sampler = self.sampler()
        with self.assertRaisesRegex(observe.ObservationError, 'record_cap'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertTrue(self.journal.stopped)
        self.assertNotIn(b'["b"', self.stream.getvalue())

    def test_parent_raw_file_close_failure_is_fatal_not_missing(self):
        self.parent()
        target = os.stat(self.proc/'7'/'stat')
        failure = OSError(5, 'synthetic raw close failure')
        injected = []

        def close(fd):
            item = os.fstat(fd)
            os.close(fd)
            if not injected and (item.st_dev, item.st_ino) == (target.st_dev, target.st_ino):
                injected.append(fd)
                raise failure

        proxy = types.SimpleNamespace(**vars(os))
        proxy.close = close
        with mock.patch.object(observe, 'os', proxy), self.assertRaises(BaseExceptionGroup) as caught:
            self.snapshot()
        self.assertEqual(caught.exception.exceptions, (failure,))
        self.assertEqual(len(injected), 1)

    def test_parent_fstat_failure_closes_registered_directory_and_retains_error(self):
        self.parent()
        failure = OSError('parent directory fstat')
        target = os.stat(self.proc/'7')
        acquired = []

        def opening(*args, **kwargs):
            fd = os.open(*args, **kwargs)
            acquired.append(fd)
            return fd

        def fstat(fd):
            item = os.fstat(fd)
            if (item.st_dev, item.st_ino) == (target.st_dev, target.st_ino):
                raise failure
            return item

        proxy = types.SimpleNamespace(**vars(os))
        proxy.open, proxy.fstat = opening, fstat
        with mock.patch.object(family, 'os', proxy):
            child, parent, error, _ = self.snapshot()
        self.assertIs(error, failure)
        self.assertIsNone(parent)
        self.assertIsNone(child['parent_start'])
        for fd in acquired:
            with self.assertRaises(OSError):
                os.fstat(fd)
        os.fstat(self.cfd)
        os.fstat(self.pfd)

    def test_parent_symlink_is_not_followed_and_child_symlink_is_fatal(self):
        self.parent()
        (self.proc/'7').rename(self.proc/'saved7')
        (self.proc/'7').symlink_to(self.proc/'saved7', target_is_directory=True)
        child, parent, error, _ = self.snapshot()
        self.assertIsNone(parent)
        self.assertIsInstance(error, OSError)
        (self.proc/'41').rename(self.proc/'saved41')
        (self.proc/'41').symlink_to(self.proc/'saved41', target_is_directory=True)
        with self.assertRaises(OSError):
            self.snapshot()

    def test_outer_topology_and_root_guard_failures_are_not_parent_loss(self):
        self.parent()
        original = observe.read_observation_file
        for mode in ('topology', 'root'):
            injected = False

            def read(fd, pin, name, **options):
                nonlocal injected
                data = original(fd, pin, name, **options)
                if not injected and name == 'stat' and data.startswith(b'7 '):
                    injected = True
                    if mode == 'topology':
                        (self.cg/'newchild').mkdir()
                    else:
                        os.chmod(self.proc, 0o750)
                return data

            try:
                with self.subTest(mode=mode), mock.patch.object(
                        observe, 'read_observation_file', read), self.assertRaises(
                        observe.ObservationError):
                    self.snapshot()
            finally:
                if mode == 'topology':
                    (self.cg/'newchild').rmdir()
                else:
                    os.chmod(self.proc, self.ppin[3] & 0o777)

    def test_parent_wrapper_close_is_fatal_and_all_cleanup_causes_survive(self):
        self.parent()
        target = os.stat(self.proc/'7'/'stat')
        original_io = observe.io.FileIO
        primary, wrapper, raw = OSError('read'), OSError('wrapper'), OSError('raw')
        selected = []

        class Stream:
            def read(self, size):
                raise primary
            def close(self):
                raise wrapper

        def make_stream(fd, *args, **kwargs):
            item = os.fstat(fd)
            if (item.st_dev, item.st_ino) == (target.st_dev, target.st_ino):
                selected.append(fd)
                return Stream()
            return original_io(fd, *args, **kwargs)

        def close(fd):
            os.close(fd)
            if fd in selected:
                raise raw

        proxy = types.SimpleNamespace(**vars(os))
        proxy.close = close
        with mock.patch.object(observe, 'io', types.SimpleNamespace(FileIO=make_stream)), \
                mock.patch.object(observe, 'os', proxy), self.assertRaises(BaseExceptionGroup) as caught:
            self.snapshot()
        self.assertEqual(caught.exception.exceptions, (primary, wrapper, raw))


if __name__ == '__main__':
    unittest.main()
