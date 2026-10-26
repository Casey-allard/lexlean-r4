"""Owned synthetic preflight records; no live VM or compiler operations."""

import importlib.util
from dataclasses import replace
import os
import pathlib
import tempfile
import types
import unittest
from unittest import mock

import observe
from test_identity import stat, status
from test_owned_io import pin

if importlib.util.find_spec('live_context'):
    import live_context
else:
    live_context = None


class ContextTests(unittest.TestCase):
    def test_null_context_rejects_before_observation_or_clock(self):
        self.assertIsNotNone(live_context, 'live preflight is not implemented')
        with mock.patch.object(os, 'open') as opened:
            with self.assertRaises(observe.ObservationError):
                live_context.open_context(None)
        opened.assert_not_called()

    def fixture(self):
        self.assertIsNotNone(live_context, 'live preflight is not implemented')
        temp = tempfile.TemporaryDirectory(prefix='r4-context-')
        self.addCleanup(temp.cleanup)
        self.root = pathlib.Path(temp.name).resolve()
        for name in ('cg', 'proc', 'boot'): (self.root / name).mkdir()
        self.cg, self.proc, self.boot = (self.root / n for n in ('cg', 'proc', 'boot'))
        self.pid = str(os.getpid())
        for pid, start, member in [(self.pid, '100', b'/observer'), ('41', '900', b'/owned')]:
            p = self.proc / pid; p.mkdir()
            (p/'stat').write_bytes(stat(pid, '7', start))
            (p/'status').write_bytes(status(pid, '7'))
            (p/'cgroup').write_bytes(b'0::'+member+b'\n')
        self.cg.joinpath('cgroup.procs').write_bytes(b'41\n')
        boot = '12345678-1234-1234-1234-123456789abc'
        self.boot.joinpath('boot_id').write_text(boot+'\n')
        self.spec = live_context.ContextSpec(
            docker=('a'*64, 'sha256:'+'b'*64, '2026-10-26T00:00:00Z', '41'),
            boot=boot, init_start='900', cgroup=(str(self.cg), pin(self.cg)),
            proc=(str(self.proc), pin(self.proc)),
            boot_directory=(str(self.boot), pin(self.boot)), target=b'/owned')
        self.now = 1000000000
        def clock(domain):
            self.assertEqual(domain, 7)
            self.now += 1000
            return self.now
        patch = mock.patch.object(live_context, 'time', types.SimpleNamespace(
            CLOCK_BOOTTIME=7, clock_gettime_ns=clock))
        patch.start(); self.addCleanup(patch.stop)

    def test_reads_actual_self_init_boot_and_clock_then_closes_owned_roots(self):
        self.fixture()
        context = live_context.open_context(self.spec)
        self.addCleanup(context.close)
        expected = (*self.spec.docker[:3], self.spec.boot,
                    ('/owned', *self.spec.cgroup[1][:2], self.spec.cgroup[1][3]),
                    (self.pid, '100', '/observer'))
        self.assertEqual(context.identity, expected)
        self.assertEqual(context.read_identity(), expected)
        self.assertGreater(context.clock_ns(), context.origin_ns)
        roots = [context.cgroup[0], context.proc[0]]
        context.close(); context.close()
        for fd in roots:
            with self.assertRaises(OSError): os.fstat(fd)
        with self.assertRaises(observe.ObservationError): context.read_identity()

    def test_init_outside_owned_census_is_not_opened(self):
        self.fixture()
        self.cg.joinpath('cgroup.procs').write_bytes(b'42\n')
        calls, real_open = [], os.open
        def opened(path, *args, **kwargs):
            calls.append(path)
            return real_open(path, *args, **kwargs)
        with mock.patch.object(os, 'open', opened):
            with self.assertRaises(observe.ObservationError): live_context.open_context(self.spec)
        self.assertNotIn('41', calls)

    def test_recheck_does_not_reopen_init_after_leaving_owned_census(self):
        self.fixture()
        context = live_context.open_context(self.spec); self.addCleanup(context.close)
        self.cg.joinpath('cgroup.procs').write_bytes(b'42\n')
        calls, real_open = [], os.open
        def opened(path, *args, **kwargs):
            calls.append(path)
            return real_open(path, *args, **kwargs)
        with mock.patch.object(os, 'open', opened):
            with self.assertRaises(observe.ObservationError): context.read_identity()
        self.assertNotIn('41', calls)

    def reject_change(self, change):
        self.fixture()
        context = live_context.open_context(self.spec); self.addCleanup(context.close)
        change()
        with self.assertRaises((observe.ObservationError, OSError)): context.read_identity()
        self.assertTrue(context.failed)
        with self.assertRaises(observe.ObservationError): context.clock_ns()

    def test_boot_change_is_permanent_failure(self):
        self.reject_change(lambda: self.boot.joinpath('boot_id').write_text(
            '12345678-1234-1234-1234-123456789abd\n'))

    def test_observer_start_change_is_not_adopted(self):
        self.reject_change(lambda: (self.proc/self.pid/'stat').write_bytes(stat(self.pid, '7', '101')))

    def test_init_start_change_is_not_adopted(self):
        self.reject_change(lambda: (self.proc/'41'/'stat').write_bytes(stat('41', '7', '901')))

    def test_observer_cannot_move_into_target_subtree(self):
        self.reject_change(lambda: (self.proc/self.pid/'cgroup').write_bytes(b'0::/owned/child\n'))

    def test_init_membership_change_rejects(self):
        self.reject_change(lambda: (self.proc/'41'/'cgroup').write_bytes(b'0::/elsewhere\n'))

    def test_status_ppid_must_agree_with_both_stat_reads(self):
        self.reject_change(lambda: (self.proc/'41'/'status').write_bytes(status('41', '8')))

    def test_new_child_cgroup_rejects(self):
        self.reject_change(lambda: (self.cg/'child').mkdir())

    def test_replaced_numeric_pid_directory_rejects_same_content(self):
        def change():
            old = self.proc/(self.pid+'-old'); (self.proc/self.pid).rename(old)
            new = self.proc/self.pid; new.mkdir()
            for name in ('stat', 'status', 'cgroup'): (new/name).write_bytes((old/name).read_bytes())
        self.reject_change(change)

    def test_replaced_target_path_rejects_old_open_directory(self):
        def change():
            old = self.root/'cg-old'; self.cg.rename(old); self.cg.mkdir()
            (self.cg/'cgroup.procs').write_bytes(b'41\n')
        self.reject_change(change)

    def test_malformed_spec_is_rejected_before_open(self):
        self.fixture()
        bad = [replace(self.spec, docker=list(self.spec.docker)),
               replace(self.spec, boot='not-uuid'), replace(self.spec, init_start='01'),
               replace(self.spec, init_start=True), replace(self.spec, target=b'/bad\0'),
               replace(self.spec, cgroup=('relative', self.spec.cgroup[1])),
               replace(self.spec, docker=(*self.spec.docker[:3], '0'))]
        with mock.patch.object(os, 'open') as opened:
            for spec in bad:
                with self.subTest(spec=spec), self.assertRaises(observe.ObservationError):
                    live_context.open_context(spec)
        opened.assert_not_called()

    def test_clock_invalid_absent_and_unsupported_have_no_fallback(self):
        self.fixture()
        for value in (True, -1, 2**64):
            with self.subTest(value=value), mock.patch.object(
                    live_context.time, 'clock_gettime_ns', return_value=value):
                with self.assertRaises(observe.ObservationError): live_context.open_context(self.spec)
        with mock.patch.object(live_context, 'time', types.SimpleNamespace()):
            with self.assertRaises(AttributeError): live_context.open_context(self.spec)
        error = OSError('synthetic unsupported domain')
        with mock.patch.object(live_context.time, 'clock_gettime_ns', side_effect=error):
            with self.assertRaises(OSError) as caught: live_context.open_context(self.spec)
        self.assertIs(caught.exception, error)

    def test_clock_backward_and_actual_pid_change_fail_permanently(self):
        self.fixture()
        context = live_context.open_context(self.spec); self.addCleanup(context.close)
        self.now = context.origin_ns-1001
        with self.assertRaises(observe.ObservationError): context.clock_ns()
        other = live_context.open_context(self.spec); self.addCleanup(other.close)
        with mock.patch.object(os, 'getpid', return_value=int(self.pid)+1):
            with self.assertRaises(observe.ObservationError): other.read_identity()
        self.assertTrue(other.failed)

    def test_reentrant_clock_cannot_mask_failure(self):
        self.fixture()
        context = live_context.open_context(self.spec); self.addCleanup(context.close)
        def callback(domain):
            try: context.read_identity()
            except observe.ObservationError: pass
            return self.now
        with mock.patch.object(live_context.time, 'clock_gettime_ns', callback):
            with self.assertRaises(observe.ObservationError): context.clock_ns()
        self.assertTrue(context.failed)

    def test_partial_open_closes_every_acquired_descriptor(self):
        self.fixture()
        (self.proc/'41'/'status').write_bytes(b'bad\n')
        opened, closed = [], []
        real_open, real_close = os.open, os.close
        def opening(*args, **kwargs):
            fd = real_open(*args, **kwargs); opened.append(fd); return fd
        def closing(fd):
            closed.append(fd); return real_close(fd)
        with mock.patch.object(os, 'open', opening), mock.patch.object(os, 'close', closing):
            with self.assertRaises(observe.ObservationError): live_context.open_context(self.spec)
        self.assertCountEqual(opened, closed)

    def test_close_preserves_falsey_primary_and_all_cleanup_failures_without_retry(self):
        self.fixture()
        context = live_context.open_context(self.spec)
        class Falsey(OSError):
            def __bool__(self): return False
        primary = Falsey('original'); cleanup = OSError('close failed after real close')
        real_close, calls = os.close, []
        def closing(fd):
            real_close(fd); calls.append(fd)
            if len(calls) <= 2: raise cleanup
        with mock.patch.object(os, 'close', closing):
            with self.assertRaises(BaseExceptionGroup) as caught: context.close(primary)
            context.close()
        self.assertEqual(len(calls), 5)
        self.assertEqual(caught.exception.exceptions, (primary, cleanup, cleanup))


if __name__ == '__main__':
    unittest.main()
