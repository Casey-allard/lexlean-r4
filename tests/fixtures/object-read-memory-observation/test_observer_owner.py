"""Finite owned POSIX children; no guest, SDK, or indefinite observer."""

import errno
import importlib.util
import os
import signal
import sys
import tempfile
from pathlib import Path
import time
import traceback
import unittest
from unittest import mock

if importlib.util.find_spec('observer_owner'):
    import observer_owner as owner
else:
    owner = None


class CleanupTests(unittest.TestCase):
    def test_public_cli_codes_and_bounded_diagnostics(self):
        import io
        fixture = OwnerCompositionTests().fixture()
        try:
            for argv, code in (([], 64), (fixture.argv, 65)):
                fixture.write(b'bad json')
                out, err = io.StringIO(), io.StringIO()
                with mock.patch('sys.stdout', out), mock.patch('sys.stderr', err), mock.patch.object(
                        os, 'fork') as forked:
                    self.assertEqual(owner.cli(argv), code)
                forked.assert_not_called()
                self.assertEqual(out.getvalue(), '')
                self.assertEqual(err.getvalue(), 'observer-owner: failed\n')
            fixture.config.unlink()
            with mock.patch('sys.stderr', io.StringIO()):
                self.assertEqual(owner.cli(fixture.argv), 74)
            for state, code in ((['exited', 0, None], 0), (['exited', 7, None], 70),
                                (['unknown', None, None], 70)):
                with mock.patch.object(owner, 'main', return_value=state):
                    self.assertEqual(owner.cli([]), code)
            with mock.patch.object(owner, 'main', side_effect=OSError('private')), mock.patch(
                    'sys.stderr', io.StringIO()) as err:
                self.assertEqual(owner.cli([]), 70)
            self.assertNotIn('private', err.getvalue())
        finally: fixture.doCleanups()

    def test_capture_requires_actual_ppid_and_outside_target_membership(self):
        from test_identity import stat, status
        fixture = OwnerCompositionTests().fixture()
        try:
            process = fixture.fixture.proc/'123456'; process.mkdir()
            for ppid, member in [('7', b'/observer'), (str(os.getpid()), b'/owned'),
                                  (str(os.getpid()), b'/owned/child')]:
                (process/'stat').write_bytes(stat('123456', ppid, '222'))
                (process/'status').write_bytes(status('123456', ppid))
                (process/'cgroup').write_bytes(b'0::'+member+b'\n')
                with self.assertRaisesRegex(Exception, 'owner_child_identity'):
                    owner._capture_identity(fixture.fixture.spec, 123456)
        finally: fixture.doCleanups()

    def test_watchdog_never_blocking_waits_and_lost_custody_never_signals(self):
        ticks = iter(x/2 for x in range(100))
        with mock.patch.object(os, 'fork', return_value=123456), mock.patch.object(
                os, 'waitpid', return_value=(0, 0)) as waited, mock.patch.object(
                os, 'kill') as killed, mock.patch.object(time, 'monotonic', side_effect=lambda: next(ticks)), \
                mock.patch.object(time, 'sleep'):
            with self.assertRaises(AssertionError): finite_worker(lambda: None)
        self.assertTrue(all(call.args == (123456, os.WNOHANG) for call in waited.call_args_list))
        killed.assert_called_once_with(123456, signal.SIGKILL)
        with mock.patch.object(os, 'fork', return_value=123456), mock.patch.object(
                os, 'waitpid', side_effect=ChildProcessError(errno.ECHILD, 'lost')), mock.patch.object(
                os, 'kill') as killed:
            with self.assertRaises(AssertionError): finite_worker(lambda: None)
        killed.assert_not_called()

    def test_launch_sync_failure_reaps_child_and_preserves_exit(self):
        def action():
            import observe
            from test_identity import stat, status
            fixture = OwnerCompositionTests().fixture()
            original, sync = owner._capture_identity, os.fsync
            def capture(spec, pid):
                process = fixture.fixture.proc/str(pid); process.mkdir()
                (process/'stat').write_bytes(stat(str(pid), str(os.getpid()), '222'))
                (process/'status').write_bytes(status(str(pid), str(os.getpid())))
                (process/'cgroup').write_bytes(b'0::/observer\n')
                return original(spec, pid)
            calls = []
            def failing_sync(fd):
                calls.append(fd)
                if len(calls) == 1: raise OSError('launch fsync')
                return sync(fd)
            first, second = OwnerProcessTests().child('import time; time.sleep(4)')
            try:
                with first, second, mock.patch.object(owner, '_capture_identity', capture), \
                     mock.patch.object(os, 'fsync', failing_sync):
                    with self.assertRaisesRegex(OSError, 'launch fsync'):
                        owner.main(fixture.argv, 0)
                end = observe.decode_owner((fixture.output/'observer-exit.json').read_bytes())
                self.assertEqual(end[7], 'owner_error')
                self.assertIn(end[6][0], ('exited', 'signaled'))
                self.assertEqual(len(calls), 2)
                with self.assertRaises(ChildProcessError): os.waitpid(int(end[4]), os.WNOHANG)
                self.assertNotEqual((fixture.output/'observer-launch.json').read_bytes(), b'')
            finally: fixture.doCleanups()
        finite_worker(action)

    def fake_child(self):
        with mock.patch.object(owner, '_preconditions'), mock.patch.object(
                owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), mock.patch.object(
                os, 'fork', return_value=123456):
            child = owner._OwnedChild(); child.start([])
        self.addCleanup(child.close_go)
        return child

    def test_esrch_timeout_and_uncertain_wait_have_no_later_signal_authority(self):
        child = self.fake_child()
        with mock.patch.object(os, 'waitpid', return_value=(0, 0)), mock.patch.object(
                os, 'kill', side_effect=ProcessLookupError()) as killed, mock.patch.object(
                child, 'wait_until', return_value=False):
            child.stop('host_eof')
        self.assertEqual(child.state, 'LOST')
        self.assertEqual(child.reason, 'reap_timeout')
        self.assertEqual(killed.call_args_list, [mock.call(123456, signal.SIGTERM),
                                               mock.call(123456, signal.SIGKILL)])
        with mock.patch.object(os, 'kill') as killed: child.stop('owner_error')
        killed.assert_not_called()
        child = self.fake_child()
        with mock.patch.object(os, 'waitpid', side_effect=OSError(errno.EIO, 'wait')):
            with self.assertRaises(OSError): child.poll()
        with mock.patch.object(os, 'kill') as killed: child.stop('owner_error')
        killed.assert_not_called()
        self.assertEqual(child.reason, 'lost_custody')

    def test_preconditions_release_and_signal_error_accounting(self):
        with mock.patch.object(signal, 'getsignal', return_value=signal.SIG_IGN):
            with self.assertRaises(Exception): owner._preconditions()
        with mock.patch.object(owner.threading, 'active_count', return_value=2):
            with self.assertRaises(Exception): owner._preconditions()
        with self.assertRaises(Exception): owner._OwnedChild().release()
        child = self.fake_child()
        with mock.patch.object(os, 'waitpid', return_value=(0, 0)), mock.patch.object(
                os, 'kill', side_effect=PermissionError('signal')), mock.patch.object(
                child, 'wait_until', return_value=False):
            with self.assertRaises(BaseExceptionGroup) as error: child.stop('owner_error')
        self.assertEqual(len(error.exception.exceptions), 2)
        self.assertEqual(child.state, 'LOST')

    def test_start_attempt_and_release_compound_errors(self):
        with mock.patch.object(owner, '_preconditions'), mock.patch.object(
                owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), mock.patch.object(
                os, 'fork', side_effect=OSError('fork')) as forked:
            child = owner._OwnedChild()
            with self.assertRaises(OSError): child.start([])
            with self.assertRaises(Exception): child.start([])
            self.assertEqual(forked.call_count, 1)
        with mock.patch.object(owner, '_preconditions'), mock.patch.object(
                owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), mock.patch.object(
                os, 'fork', return_value=123456):
            child = owner._OwnedChild(); child.start([])
        close = child.close_go
        def broken_close():
            close(); raise OSError('close')
        with mock.patch.object(os, 'write', side_effect=OSError('write')), mock.patch.object(
                child, 'close_go', broken_close):
            with self.assertRaises(BaseExceptionGroup) as error: child.release()
        self.assertEqual([str(x) for x in error.exception.exceptions], ['write', 'close'])

    def test_eof_after_reap_and_lost_wait_never_signal(self):
        with mock.patch.object(owner, '_preconditions'), mock.patch.object(
                owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), mock.patch.object(
                os, 'fork', return_value=123456):
            child = owner._OwnedChild(); child.start([])
        read, write = os.pipe(); os.close(write)
        try:
            with mock.patch.object(os, 'waitpid', return_value=(123456, 0)), mock.patch.object(
                    os, 'kill') as killed:
                child.supervise(read); child.stop('host_eof')
            killed.assert_not_called()
            self.assertEqual(child.reason, 'natural')
        finally: os.close(read); child.close_go()

    def test_close_go_failure_does_not_abandon_child(self):
        def action():
            first, second = OwnerProcessTests().child('import time; time.sleep(4)')
            with first, second:
                child = owner._OwnedChild(); child.start([])
                close = child.close_go
                def broken_close():
                    close()
                    raise OSError('close fixture')
                try:
                    with mock.patch.object(child, 'close_go', broken_close):
                        with self.assertRaisesRegex(OSError, 'close fixture'):
                            child.stop('owner_error')
                    self.assertEqual(child.state, 'REAPED')
                    self.assertEqual(child.reason, 'owner_error')
                finally: child.stop('owner_error')
        finite_worker(action)


def finite_worker(action):
    """Bounded user-space watchdog; every grandchild has its own 5s alarm."""
    read, write = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(read)
        signal.alarm(7)
        try:
            action()
        except BaseException:
            os.write(write, traceback.format_exc().encode()[:8192])
            os._exit(1)
        os._exit(0)
    os.close(write)
    started = time.monotonic()
    normal, final = started+8, started+10
    status, owned, intervened, raw = None, True, False, b''
    def poll_until(deadline):
        nonlocal status, owned
        while time.monotonic() < deadline and owned:
            try:
                found, value = os.waitpid(pid, os.WNOHANG)
            except InterruptedError:
                continue
            except OSError:
                owned = False
                break
            if found not in (0, pid):
                owned = False
                break
            if found == pid and (os.WIFEXITED(value) or os.WIFSIGNALED(value)):
                status, owned = value, False
                break
            time.sleep(min(.01, max(0, deadline-time.monotonic())))
    try:
        poll_until(normal)
        if owned:
            intervened = True
            try: os.kill(pid, signal.SIGKILL)
            except OSError: pass
            poll_until(final)
        os.set_blocking(read, False)
        while len(raw) < 8193 and time.monotonic() < final:
            try: part = os.read(read, 8193-len(raw))
            except BlockingIOError: break
            except InterruptedError: continue
            if not part: break
            raw += part
    finally:
        os.close(read)
    if status != 0 or intervened:
        raise AssertionError('finite worker failed or watchdog intervened: '+raw.decode())


class OwnerProcessTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(owner, 'fixed owner missing')

    def child(self, code):
        entry = owner._child_entry
        def finite_entry(*args):
            signal.alarm(5)
            entry(*args)
        # Fixed test programs only; production has no executable override API.
        argv = (sys.executable, '-I', '-B', '-c', 'import signal; signal.alarm(5); '+code)
        return mock.patch.object(owner, '_child_entry', finite_entry), mock.patch.object(
            owner, '_exec_argv', return_value=argv)

    def test_real_fork_wait_and_saturated_stdio(self):
        def action():
            for code, expected in [('import os; os._exit(7)', 7),
                                   ('import os; os.write(1,b"x"*1048576); '
                                    'os.write(2,b"y"*1048576)', 0)]:
                first, second = self.child(code)
                with first, second:
                    child = owner._OwnedChild()
                    child.start([]); child.release()
                    self.assertTrue(child.wait_until(time.monotonic()+3))
                    self.assertEqual(child.process, ['exited', expected, None])
                    with mock.patch.object(os, 'kill') as killed:
                        child.stop('host_eof')
                    killed.assert_not_called()
                    self.assertEqual(child.reason, 'natural')
        finite_worker(action)

    def test_go_eof_does_not_exec_and_real_host_eof_stops_child(self):
        def action():
            first, second = self.child('import time; time.sleep(4)')
            with first, second:
                child = owner._OwnedChild(); child.start([]); child.close_go()
                self.assertTrue(child.wait_until(time.monotonic()+2))
                self.assertEqual(child.process, ['exited', 125, None])
                child = owner._OwnedChild(); child.start([]); child.release()
                read, write = os.pipe(); os.close(write)
                try: child.supervise(read)
                finally: os.close(read)
                self.assertEqual(child.state, 'REAPED')
                self.assertEqual(child.reason, 'host_eof')
        finite_worker(action)

    def test_echild_is_irreversible_and_eintr_does_not_reap(self):
        with mock.patch.object(owner, '_preconditions'), \
             mock.patch.object(owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), \
             mock.patch.object(os, 'fork', return_value=123456) as forked:
            child = owner._OwnedChild(); child.start([])
        forked.assert_called_once_with()
        self.addCleanup(child.close_go)
        with mock.patch.object(os, 'waitpid', side_effect=InterruptedError(errno.EINTR, 'fixture')):
            self.assertFalse(child.poll())
        with mock.patch.object(os, 'waitpid', side_effect=ChildProcessError(errno.ECHILD, 'fixture')):
            self.assertTrue(child.poll())
        self.assertEqual(child.state, 'LOST')
        self.assertEqual(child.reason, 'lost_custody')
        with mock.patch.object(os, 'kill') as killed:
            child.stop('owner_error')
        killed.assert_not_called()

    def test_stopped_status_is_not_final_and_foreign_wait_result_loses_custody(self):
        with mock.patch.object(owner, '_preconditions'), mock.patch.object(
                owner, '_exec_argv', return_value=('/unused/fixed-fixture',)), mock.patch.object(
                os, 'fork', return_value=123456) as forked:
            child = owner._OwnedChild(); child.start([])
        forked.assert_called_once_with()
        self.addCleanup(child.close_go)
        with mock.patch.object(os, 'waitpid', return_value=(123456, (signal.SIGSTOP << 8) | 0x7f)):
            self.assertFalse(child.poll())
        self.assertEqual(child.state, 'OWNED')
        with mock.patch.object(os, 'waitpid', return_value=(999999, 0)):
            with self.assertRaises(Exception): child.poll()
        self.assertEqual(child.state, 'LOST')

    def test_exec_failure_and_term_ignoring_child_are_finally_reaped(self):
        def action():
            first, _ = self.child('pass')
            with first, mock.patch.object(owner, '_exec_argv', return_value=('/no/such/fixture',)):
                child = owner._OwnedChild(); child.start([]); child.release()
                self.assertTrue(child.wait_until(time.monotonic()+2))
                self.assertEqual(child.process, ['exited', 127, None])
            with tempfile.TemporaryDirectory() as temp:
                ready = Path(temp)/'ready'
                code = ('import os,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); '
                        'open('+repr(str(ready))+',"w").close(); time.sleep(4)')
                first, second = self.child(code)
                with first, second:
                    child = owner._OwnedChild(); child.start([]); child.release()
                    until = time.monotonic()+1
                    while not ready.exists() and time.monotonic() < until: time.sleep(.005)
                    self.assertTrue(ready.exists())
                    child.stop('owner_error')
                    self.assertEqual(child.state, 'REAPED')
                    self.assertEqual(child.process, ['signaled', None, signal.SIGKILL])
        finite_worker(action)

    def test_invalid_go_and_exit_before_release(self):
        def action():
            first, second = self.child('pass')
            with first, second:
                child = owner._OwnedChild(); child.start([])
                os.write(child.go, b'X'); child.close_go()
                self.assertTrue(child.wait_until(time.monotonic()+2))
                self.assertEqual(child.process, ['exited', 125, None])
            with mock.patch.object(owner, '_child_entry', lambda *args: os._exit(8)), second:
                child = owner._OwnedChild(); child.start([])
                self.assertTrue(child.wait_until(time.monotonic()+2))
                with self.assertRaises(Exception): child.release()
                child.close_go()
                self.assertEqual(child.process, ['exited', 8, None])
        finite_worker(action)


class OwnerCompositionTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(hasattr(owner, 'main'), 'owner composition missing')

    def fixture(self):
        import test_observer_entry
        fixture = test_observer_entry.EntryTests(); fixture.setUp()
        return fixture

    def test_actual_files_receipts_and_fork_wait_with_synthetic_proc(self):
        def action():
            from test_identity import stat, status
            import observe
            fixture = self.fixture()
            original = owner._capture_identity
            pids = []
            def capture(spec, pid):
                pids.append(pid)
                process = fixture.fixture.proc/str(pid); process.mkdir()
                (process/'stat').write_bytes(stat(str(pid), str(os.getpid()), '222'))
                (process/'status').write_bytes(status(str(pid), str(os.getpid())))
                (process/'cgroup').write_bytes(b'0::/observer\n')
                return original(spec, pid)
            first, second = OwnerProcessTests().child('import os; os._exit(7)')
            read, write = os.pipe()
            try:
                with first, second, mock.patch.object(owner, '_capture_identity', capture):
                    result = owner.main(fixture.argv, read)
                launch = observe.decode_owner((fixture.output/'observer-launch.json').read_bytes())
                end = observe.decode_owner((fixture.output/'observer-exit.json').read_bytes())
                self.assertEqual(launch[4], [str(os.getpid()), '100'])
                self.assertEqual(launch[5], [str(pids[0]), '222'])
                self.assertEqual(end[4:7], [str(pids[0]), '222', ['exited', 7, None]])
                self.assertEqual(end[7], 'natural')
                self.assertEqual(result, ['exited', 7, None])
                with self.assertRaises(ChildProcessError): os.waitpid(pids[0], os.WNOHANG)
                for path in fixture.output.iterdir():
                    self.assertEqual(path.stat().st_mode & 0o777, 0o600)
                    self.assertLessEqual(path.stat().st_size, 512)
            finally:
                os.close(read); os.close(write); fixture.doCleanups()
        finite_worker(action)

    def test_fork_error_and_partial_output_setup_leave_no_child(self):
        import observe
        fixture = self.fixture()
        try:
            with mock.patch.object(os, 'fork', side_effect=OSError('fork failed')):
                with self.assertRaises(OSError): owner.main(fixture.argv, 0)
            row = observe.decode_owner((fixture.output/'observer-exit.json').read_bytes())
            self.assertEqual(row[4:], [None, None, ['not_run', None, None], 'fork_failed'])
            self.assertEqual((fixture.output/'observer-launch.json').read_bytes(), b'')
            with mock.patch.object(os, 'fork') as forked:
                with self.assertRaises(FileExistsError): owner.main(fixture.argv, 0)
            forked.assert_not_called()
        finally: fixture.doCleanups()

    def test_capture_and_receipt_failure_cleanup_preserves_primary(self):
        def action():
            import observe
            fixture = self.fixture()
            first, second = OwnerProcessTests().child('import time; time.sleep(4)')
            try:
                with first, second, mock.patch.object(owner, '_capture_identity', side_effect=ValueError('capture')):
                    with self.assertRaisesRegex(ValueError, 'capture'):
                        owner.main(fixture.argv, 0)
                end = observe.decode_owner((fixture.output/'observer-exit.json').read_bytes())
                self.assertEqual(end[5], None)
                self.assertIn(end[6][0], ('exited', 'signaled'))
                self.assertEqual(end[7], 'owner_error')
                with self.assertRaises(ChildProcessError): os.waitpid(int(end[4]), os.WNOHANG)
            finally: fixture.doCleanups()
        finite_worker(action)


class InterruptionTests(unittest.TestCase):
    """Mock-only wait/publication/cleanup faults; never interrupt a real child."""

    def child(self):
        return CleanupTests.fake_child(self)

    def test_uncertain_wait_interrupt_revokes_signals_and_retains_error(self):
        child, error = self.child(), KeyboardInterrupt('wait boundary')
        with mock.patch.object(os, 'waitpid', side_effect=error), mock.patch.object(os, 'kill') as killed:
            with self.assertRaises(KeyboardInterrupt) as caught: child.stop('owner_error')
        self.assertIs(caught.exception, error)
        self.assertEqual(child.state, 'LOST')
        self.assertEqual(child.reason, 'lost_custody')
        killed.assert_not_called()

    def interrupted_cleanup(self, where):
        child, error, signals = self.child(), KeyboardInterrupt(where), []
        clock = [0.0]
        def now():
            clock[0] += .5
            return clock[0]
        def kill(pid, sig):
            self.assertEqual(pid, 123456)
            signals.append(sig)
            if where == 'kill' and len(signals) == 1: raise error
        def wait(pid, flags):
            self.assertEqual((pid, flags), (123456, os.WNOHANG))
            return (123456, int(signal.SIGKILL)) if signal.SIGKILL in signals else (0, 0)
        sleeping = [False]
        def sleep(delay):
            if where == 'sleep' and not sleeping[0]:
                sleeping[0] = True
                raise error
        with mock.patch.object(os, 'waitpid', wait), mock.patch.object(os, 'kill', kill), \
                mock.patch.object(time, 'monotonic', now), mock.patch.object(time, 'sleep', sleep):
            with self.assertRaises(KeyboardInterrupt) as caught: child.stop('owner_error')
        self.assertIs(caught.exception, error)
        self.assertEqual(signals, [signal.SIGTERM, signal.SIGKILL])
        self.assertEqual(child.state, 'REAPED')
        self.assertEqual(child.process, ['signaled', None, signal.SIGKILL])

    def test_known_owned_sleep_interrupt_continues_bounded_cleanup(self):
        self.interrupted_cleanup('sleep')

    def test_known_owned_kill_interrupt_continues_bounded_cleanup(self):
        self.interrupted_cleanup('kill')

    def test_terminal_decode_and_publication_interrupt_never_restore_signals(self):
        for where in ('decode', 'publish'):
            child, error = self.child(), KeyboardInterrupt(where)
            setter = owner._OwnedChild.__setattr__
            def assigning(obj, name, value):
                if where == 'publish' and name == 'process' and value[0] == 'exited': raise error
                setter(obj, name, value)
            with mock.patch.object(os, 'waitpid', return_value=(123456, 0)), mock.patch.object(
                    os, 'WEXITSTATUS', side_effect=error if where == 'decode' else None,
                    return_value=0), mock.patch.object(owner._OwnedChild, '__setattr__', assigning):
                with self.assertRaises(KeyboardInterrupt) as caught: child.poll()
            self.assertIs(caught.exception, error)
            self.assertNotEqual(child.state, 'OWNED')
            with mock.patch.object(os, 'kill') as killed: child.stop('owner_error')
            killed.assert_not_called()

    def test_only_genuine_wait_eintr_restores_owned(self):
        for code in (errno.EIO, None):
            child, error = self.child(), InterruptedError(code, 'not EINTR')
            with mock.patch.object(os, 'waitpid', side_effect=error):
                with self.assertRaises(InterruptedError) as caught: child.poll()
            self.assertIs(caught.exception, error)
            self.assertEqual(child.state, 'LOST')
        child = self.child()
        with mock.patch.object(os, 'waitpid', side_effect=InterruptedError(errno.EINTR, 'EINTR')):
            self.assertFalse(child.poll())
        self.assertEqual(child.state, 'OWNED')
        # An EINTR-shaped exception during decoding is not a syscall EINTR.
        with mock.patch.object(os, 'waitpid', return_value=(123456, 0)), mock.patch.object(
                os, 'WEXITSTATUS', side_effect=InterruptedError(errno.EINTR, 'decode')):
            with self.assertRaises(InterruptedError): child.poll()
        self.assertNotEqual(child.state, 'OWNED')

    def test_cleanup_retries_reuse_absolute_deadlines(self):
        child, error = self.child(), KeyboardInterrupt('sleep')
        with mock.patch.object(time, 'monotonic', return_value=100), mock.patch.object(
                os, 'waitpid', return_value=(0, 0)), mock.patch.object(os, 'kill') as killed, \
                mock.patch.object(child, 'wait_until', side_effect=[error, False]) as waited:
            with self.assertRaises(KeyboardInterrupt) as caught: child.stop('owner_error')
        self.assertIs(caught.exception, error)
        self.assertEqual([c.args[0] for c in waited.call_args_list], [102, 104])
        self.assertEqual([c.args for c in killed.call_args_list],
                         [(123456, signal.SIGTERM), (123456, signal.SIGKILL)])
        self.assertEqual((child.state, child.reason), ('LOST', 'reap_timeout'))

    def test_clock_unavailable_uses_only_finite_no_sleep_phases(self):
        for later, final in ((False, False), (False, True), (True, True)):
            child, error, signals = self.child(), KeyboardInterrupt('clock'), []
            def kill(pid, sig):
                self.assertEqual(pid, 123456); signals.append(sig)
            def wait(pid, flags):
                self.assertEqual((pid, flags), (123456, os.WNOHANG))
                return (123456, 9) if final and signal.SIGKILL in signals else (0, 0)
            with mock.patch.object(os, 'waitpid', wait), mock.patch.object(os, 'kill', kill), \
                    mock.patch.object(time, 'sleep') as slept, mock.patch.object(
                    time, 'monotonic', side_effect=[100, error] if later else error) as clock:
                with self.assertRaises(KeyboardInterrupt) as caught: child.stop('owner_error')
            self.assertIs(caught.exception, error)
            self.assertEqual(signals, [signal.SIGTERM, signal.SIGKILL])
            self.assertEqual(clock.call_count, 2 if later else 1)
            slept.assert_not_called()
            if final: self.assertEqual(child.state, 'REAPED')
            else:
                self.assertEqual(child.state, 'LOST')
                self.assertEqual(child.reason, 'owner_error')
                self.assertEqual(child.process, ['unknown', None, None])

    def test_wait_transition_is_nonsignalable_and_nonfinal_restores_owned(self):
        for status in ((int(signal.SIGSTOP) << 8) | 0x7f, 0xffff):
            child = self.child()
            def waiting(pid, flags):
                self.assertNotEqual(child.state, 'OWNED')
                return pid, status
            with mock.patch.object(os, 'waitpid', waiting):
                self.assertFalse(child.poll())
            self.assertEqual(child.state, 'OWNED')



class FinalWaitTests(unittest.TestCase):
    """Mock-only final-phase continuation; no real interrupted child."""

    def scenario(self, mode):
        child = CleanupTests.fake_child(self)
        sleep_error, clock_error, wait_error = (KeyboardInterrupt(x) for x in ('sleep', 'clock', 'wait'))
        signals, waits, deadlines, clocks, sleeps = [], [], [], [], []
        values = [0, 2, 2.5, 4 if mode == 'expire' else 3]
        if mode == 'repeated': values = [0, 2, 2.5, 3, 4]
        def clock():
            index = len(clocks); clocks.append(index)
            if index >= len(values): raise AssertionError('unexpected clock retry')
            if index == 3 and mode.startswith('clock'): raise clock_error
            return values[index]
        def kill(pid, sig):
            self.assertEqual(pid, 123456); signals.append(sig)
        def wait(pid, flags):
            self.assertEqual((pid, flags), (123456, os.WNOHANG)); waits.append(pid)
            if len(waits) <= 3: return 0, 0
            if mode in ('uncertain', 'clock_uncertain'): raise wait_error
            if mode in ('expire', 'clock_unready', 'repeated'): return 0, 0
            return pid, 9
        def sleep(delay):
            self.assertGreater(delay, 0); sleeps.append(delay)
            raise sleep_error
        real_wait = child.wait_until
        def until(deadline):
            deadlines.append(deadline); return real_wait(deadline)
        with mock.patch.object(time, 'monotonic', clock), mock.patch.object(time, 'sleep', sleep), \
                mock.patch.object(os, 'kill', kill), mock.patch.object(os, 'waitpid', wait), \
                mock.patch.object(child, 'wait_until', until):
            with self.assertRaises(BaseException) as caught: child.stop('owner_error')
        def leaves(error):
            if isinstance(error, BaseExceptionGroup):
                return [leaf for item in error.exceptions for leaf in leaves(item)]
            return [error]
        errors = leaves(caught.exception)
        self.assertEqual(signals, [signal.SIGTERM, signal.SIGKILL])
        self.assertEqual(deadlines, [2, 4, 4, 4] if mode == 'repeated' else [2, 4, 4])
        self.assertEqual(len(waits), 5 if mode == 'repeated' else 4)
        self.assertIs(errors[0], sleep_error)
        if mode.startswith('clock'):
            self.assertIs(errors[1], clock_error)
            self.assertEqual(len(clocks), 4)
        if mode in ('uncertain', 'clock_uncertain'):
            self.assertIs(errors[-1], wait_error)
            self.assertEqual((child.state, child.reason), ('LOST', 'lost_custody'))
        elif mode in ('expire', 'repeated'):
            self.assertEqual((child.state, child.reason), ('LOST', 'reap_timeout'))
        elif mode == 'clock_unready':
            self.assertEqual((child.state, child.reason), ('LOST', 'owner_error'))
        else:
            self.assertEqual(child.state, 'REAPED')
            self.assertEqual(child.process, ['signaled', None, 9])
        with mock.patch.object(os, 'kill') as killed, mock.patch.object(os, 'waitpid') as waited:
            child.stop('owner_error')
        killed.assert_not_called(); waited.assert_not_called()

    def test_final_sleep_interrupt_resumes_same_deadline_and_reaps(self):
        self.scenario('reap')

    def test_final_retry_expiry_uses_original_deadline_and_last_poll(self):
        self.scenario('expire')

    def test_repeated_final_sleep_interrupts_do_not_reset_or_resignal(self):
        self.scenario('repeated')

    def test_clock_loss_on_final_retry_has_one_fallback_poll(self):
        for mode in ('clock_reap', 'clock_unready', 'clock_uncertain'):
            with self.subTest(mode=mode): self.scenario(mode)

    def test_uncertain_wait_on_final_retry_never_restores_custody(self):
        self.scenario('uncertain')

    def test_final_clock_loss_before_sleep_gets_one_actual_fallback_poll(self):
        for terminal in (False, True):
            with self.subTest(terminal=terminal):
                child = CleanupTests.fake_child(self)
                error = KeyboardInterrupt('final clock')
                last = (123456, 9) if terminal else (0, 0)
                with mock.patch.object(time, 'monotonic', side_effect=[0, 2, error]) as clock, \
                        mock.patch.object(time, 'sleep') as slept, mock.patch.object(
                        os, 'waitpid', side_effect=[(0, 0), (0, 0), last]) as waited, \
                        mock.patch.object(os, 'kill') as killed:
                    with self.assertRaises(KeyboardInterrupt) as caught: child.stop('owner_error')
                self.assertIs(caught.exception, error)
                self.assertEqual(clock.call_count, 3); self.assertEqual(waited.call_count, 3)
                self.assertEqual(killed.call_args_list,
                                 [mock.call(123456, signal.SIGTERM), mock.call(123456, signal.SIGKILL)])
                slept.assert_not_called()
                self.assertEqual(child.state, 'REAPED' if terminal else 'LOST')
                if not terminal: self.assertEqual(child.reason, 'owner_error')

    def test_final_fallback_exception_ends_even_if_custody_stays_known(self):
        child = CleanupTests.fake_child(self)
        clock_error, fallback_error = KeyboardInterrupt('clock'), RuntimeError('fallback')
        with mock.patch.object(time, 'monotonic', side_effect=[0, 2, clock_error]) as clock, \
                mock.patch.object(child, 'poll', side_effect=[False, False, fallback_error]) as polled, \
                mock.patch.object(os, 'kill') as killed, mock.patch.object(time, 'sleep') as slept:
            with self.assertRaises(BaseExceptionGroup) as caught: child.stop('owner_error')
        self.assertEqual(caught.exception.exceptions, (clock_error, fallback_error))
        self.assertEqual(clock.call_count, 3); self.assertEqual(polled.call_count, 3)
        self.assertEqual(killed.call_args_list,
                         [mock.call(123456, signal.SIGTERM), mock.call(123456, signal.SIGKILL)])
        slept.assert_not_called()
        self.assertEqual((child.state, child.reason), ('LOST', 'owner_error'))
