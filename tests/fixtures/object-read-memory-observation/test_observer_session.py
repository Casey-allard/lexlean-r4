"""One reader composed from actual helpers and owned synthetic kernel records."""

import importlib.util
import io
import json
import os
import types
import unittest
from unittest import mock

import boundary
import cadence
import control_slots
import live_context
import observe
import test_live_context
import test_sample
from test_owned_io import pin

if importlib.util.find_spec('observer_session'):
    import observer_session
else:
    observer_session = None


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(observer_session, 'serial reader composition is not implemented')
        self.fixture = test_live_context.ContextTests()
        self.fixture.fixture(); self.addCleanup(self.fixture.doCleanups)
        self.context = live_context.open_context(self.fixture.spec)
        self.addCleanup(self.context.close)
        files = {'memory.events': test_sample.SampleTests.events(),
                 'memory.events.local': test_sample.SampleTests.events(),
                 'memory.current': b'123\n', 'memory.swap.current': b'0\n',
                 'memory.peak': b'456\n', 'memory.stat': b'anon 100\nfile 23\n',
                 'memory.pressure': b'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n'}
        for name, data in files.items(): (self.fixture.cg/name).write_bytes(data)
        (self.fixture.proc/'41'/'cmdline').write_bytes(b'fixture-init\0')
        directory = self.fixture.root/'controls'; directory.mkdir(mode=0o700)
        identity = pin(directory)
        fd = observe.open_pinned_directory(str(directory), identity); self.addCleanup(os.close, fd)
        self.host = control_slots.OwnedControlSlots((fd, identity), 'host')
        self.guest = control_slots.OwnedControlSlots((fd, identity), 'guest')
        self.stream, self.terminal, self.syncs = io.BytesIO(), io.BytesIO(), []
        self.journal = observe.Journal(self.stream, lambda: self.syncs.append(self.stream.tell()))
        self.nonce = 'c'*64
        self.sampler = boundary.SampleBoundary(
            self.journal, self.nonce, self.context.identity, self.context.origin_ns,
            self.context.cgroup, self.context.proc, self.context.target,
            self.context.read_identity, self.context.clock_ns)
        self.driver = cadence.Cadence(self.sampler)
        self.retention = observe.Retention()
        self.session = observer_session.ObserverSession(
            self.context, self.driver, self.guest, self.retention, self.terminal, lambda: None)

    def request(self, sequence):
        raw = boundary.encode_request(self.nonce, sequence)
        self.host.publish(raw)
        return raw

    def test_actual_boundary_ack_is_published_after_journal_sync(self):
        self.request(1)
        ack = self.session.step()
        self.assertEqual(self.host.read_peer(), ack)
        self.assertEqual(boundary.decode_ack(ack)[1], 1)
        self.assertEqual(self.journal.synced_offset, self.journal.complete_offset)
        self.assertEqual(self.terminal.getvalue(), b'')
        self.assertFalse(self.session.ended)

    def test_unchanged_request_still_allows_periodic_sample_without_new_ack(self):
        self.request(1); first = self.session.step()
        self.fixture.now = self.driver.due_ns
        self.assertIsNone(self.session.step())
        self.assertEqual(self.host.read_peer(), first)
        self.assertEqual(self.sampler.sample, 2)

    def test_all_eleven_boundaries_end_once_with_terminal_and_no_later_capture(self):
        for sequence in range(1, 12):
            self.request(sequence)
            self.assertEqual(boundary.decode_ack(self.session.step())[1], sequence)
        self.assertTrue(self.session.ended)
        terminal = json.loads(self.terminal.getvalue())
        self.assertEqual(terminal[:3], ['t', 1, 'guest'])
        self.assertEqual(terminal[5:7], ['ended', 'ended'])
        before = self.terminal.getvalue()
        self.session.finish()
        self.assertEqual(self.terminal.getvalue(), before)
        with self.assertRaises(observe.ObservationError): self.session.step()
        self.assertEqual(self.sampler.sample, 11)

    def test_changed_previous_request_stops_without_second_ack(self):
        self.request(1); ack = self.session.step()
        self.host.publish(boundary.encode_request('d'*64, 1))
        with self.assertRaises(observe.ObservationError): self.session.step()
        self.assertEqual(self.host.read_peer(), ack)
        self.assertEqual(json.loads(self.terminal.getvalue())[5], 'stopped')
        with self.assertRaises(observe.ObservationError): self.session.step()
        os.fstat(self.context.cgroup[0])

    def test_publication_failure_keeps_captured_prefix_but_stops(self):
        self.request(1); primary = OSError('synthetic publication')
        with mock.patch.object(self.guest, 'publish', side_effect=primary):
            with self.assertRaises(OSError) as caught: self.session.step()
        self.assertIs(caught.exception, primary)
        self.assertIsNone(self.host.read_peer())
        self.assertGreater(self.journal.synced_offset, 0)
        self.assertEqual(json.loads(self.terminal.getvalue())[5], 'stopped')

    def test_falsey_primary_and_terminal_failure_both_survive_without_retry(self):
        class Falsey(OSError):
            def __bool__(self): return False
        primary, secondary = Falsey('primary'), OSError('terminal')
        with mock.patch.object(self.terminal, 'write', side_effect=secondary) as writer:
            with self.assertRaises(BaseExceptionGroup) as caught: self.session.finish(primary)
            self.session.finish()
        self.assertEqual(caught.exception.exceptions, (primary, secondary))
        self.assertEqual(writer.call_count, 1)
        self.assertTrue(self.retention.state('guest_terminal')['attempted'])

    def test_constructor_rejects_echo_identity_callback_and_host_role(self):
        with mock.patch.object(self.sampler, 'read_identity', lambda: self.context.identity):
            with self.assertRaises(observe.ObservationError):
                observer_session.ObserverSession(self.context, self.driver, self.guest,
                    self.retention, self.terminal, lambda: None)
        with self.assertRaises(observe.ObservationError):
            observer_session.ObserverSession(self.context, self.driver, self.host,
                self.retention, self.terminal, lambda: None)
        self.assertEqual(self.terminal.getvalue(), b'')

    def test_swallowed_finish_reentry_during_terminal_sync_cannot_report_success(self):
        def sync():
            try: self.session.finish()
            except observe.ObservationError: pass
        self.session.terminal_sync = sync
        with self.assertRaises(observe.ObservationError): self.session.finish()
        self.assertTrue(self.session.failed)

    def test_serial_run_polls_until_final_boundary_without_process_api(self):
        self.assertTrue(hasattr(self.session, 'run'), 'one serial reader loop missing')
        self.request(1); next_sequence = 2; sleeps = []
        def sleeping(seconds):
            nonlocal next_sequence
            sleeps.append(seconds)
            self.fixture.now += int(seconds*1000000000)
            self.request(next_sequence); next_sequence += 1
        with mock.patch.object(observer_session.time, 'sleep', sleeping): self.session.run()
        self.assertEqual(len(sleeps), 10)
        self.assertTrue(all(0 <= value <= 0.01 for value in sleeps))
        self.assertTrue(self.session.ended)

    def test_sleep_failure_preserves_original_and_finishes_observation_only(self):
        self.request(1); primary = OSError('synthetic sleep')
        with mock.patch.object(observer_session.time, 'sleep', side_effect=primary):
            with self.assertRaises(OSError) as caught: self.session.run()
        self.assertIs(caught.exception, primary)
        self.assertTrue(self.session.failed)
        self.assertEqual(json.loads(self.terminal.getvalue())[5], 'stopped')
        os.fstat(self.context.proc[0])

    def test_reentrant_finish_from_sleep_cannot_escape_stopped_state(self):
        self.request(1)
        def sleeping(seconds):
            try: self.session.finish()
            except observe.ObservationError: pass
        with mock.patch.object(observer_session.time, 'sleep', sleeping):
            with self.assertRaises(observe.ObservationError): self.session.run()
        self.assertTrue(self.session.failed)
        self.assertFalse(self.session.running)

    def test_owned_run_writes_real_files_and_releases_all_acquired_descriptors(self):
        self.assertTrue(hasattr(observer_session, 'run_owned'), 'owned reader entry missing')
        output = self.fixture.root/'output'; output.mkdir(mode=0o700)
        identity = pin(output)
        fd = observe.open_pinned_directory(str(output), identity); self.addCleanup(os.close, fd)
        host = control_slots.OwnedControlSlots((fd, identity), 'host')
        host.publish(boundary.encode_request(self.nonce, 1))
        next_sequence = 2
        def sleeping(seconds):
            nonlocal next_sequence
            self.fixture.now += int(seconds*1000000000)
            host.publish(boundary.encode_request(self.nonce, next_sequence)); next_sequence += 1
        opened, real_open = [], os.open
        def opening(*args, **kwargs):
            value = real_open(*args, **kwargs); opened.append(value); return value
        with mock.patch.object(os, 'open', opening), mock.patch.object(
                observer_session.time, 'sleep', sleeping):
            observer_session.run_owned(self.fixture.spec, (str(output), identity), self.nonce)
        self.assertEqual(json.loads((output/'guest-terminal.json').read_bytes())[5], 'ended')
        self.assertEqual(boundary.decode_ack(host.read_peer())[1], 11)
        self.assertTrue((output/'journal.jsonl').read_bytes().startswith(b'["h",'))
        os.fstat(fd)
        for acquired in opened:
            with self.assertRaises(OSError): os.fstat(acquired)

    def test_owned_null_configuration_rejects_before_any_open(self):
        self.assertTrue(hasattr(observer_session, 'run_owned'), 'owned reader entry missing')
        with mock.patch.object(os, 'open') as opened:
            with self.assertRaises(observe.ObservationError):
                observer_session.run_owned(None, None, None)
        opened.assert_not_called()

    def test_owned_second_output_failure_closes_every_descriptor_and_preserves_cause(self):
        output = self.fixture.root/'partial'; output.mkdir(mode=0o700)
        primary, opened, real_open = OSError('second output'), [], os.open
        def opening(name, *args, **kwargs):
            if name == 'guest-terminal.json': raise primary
            value = real_open(name, *args, **kwargs); opened.append(value); return value
        with mock.patch.object(os, 'open', opening):
            with self.assertRaises(OSError) as caught:
                observer_session.run_owned(self.fixture.spec, (str(output), pin(output)), self.nonce)
        self.assertIs(caught.exception, primary)
        self.assertEqual((output/'journal.jsonl').read_bytes(), b'')
        for acquired in opened:
            with self.assertRaises(OSError): os.fstat(acquired)

    def test_owned_existing_output_and_symlink_are_never_overwritten(self):
        for link in (False, True):
            with self.subTest(link=link):
                output = self.fixture.root/('link' if link else 'existing'); output.mkdir(mode=0o700)
                sentinel = output/'sentinel'; sentinel.write_bytes(b'preserve')
                journal = output/'journal.jsonl'
                if link: journal.symlink_to(sentinel)
                else: journal.write_bytes(b'preserve')
                with self.assertRaises(FileExistsError):
                    observer_session.run_owned(self.fixture.spec, (str(output), pin(output)), self.nonce)
                self.assertEqual(journal.read_bytes(), b'preserve')
                self.assertEqual(sentinel.read_bytes(), b'preserve')
                self.assertFalse((output/'guest-terminal.json').exists())

    def test_owned_invalid_mode_or_nonce_has_no_open_effect(self):
        output = self.fixture.root/'invalid'; output.mkdir(mode=0o755)
        for mode, nonce in ((0o755, self.nonce), (0o700, 'bad')):
            output.chmod(mode)
            with mock.patch.object(os, 'open') as opened:
                with self.assertRaises(observe.ObservationError):
                    observer_session.run_owned(self.fixture.spec, (str(output), pin(output)), nonce)
            opened.assert_not_called()

    def test_cleanup_preserves_falsey_primary_and_all_errors_without_retry(self):
        class Falsey(OSError):
            def __bool__(self): return False
        primary, first, second = Falsey('primary'), OSError('first'), OSError('second')
        calls = []
        def close_one(): calls.append('one'); raise first
        def close_two(): calls.append('two'); raise second
        cleanup = [close_two, close_one]
        with self.assertRaises(BaseExceptionGroup) as caught:
            observer_session._close_owned(cleanup, primary)
        self.assertEqual(caught.exception.exceptions, (primary, first, second))
        self.assertEqual(calls, ['one', 'two'])
        observer_session._close_owned(cleanup, None)
        self.assertEqual(calls, ['one', 'two'])

    def test_fileio_setup_failure_closes_owned_output(self):
        output = self.fixture.root/'fileio'; output.mkdir(mode=0o700)
        primary = OSError('fileio construction')
        opened, journal_fds, constructed, real_open = [], [], [], os.open
        def opening(name, *args, **kwargs):
            value = real_open(name, *args, **kwargs); opened.append(value)
            if name == 'journal.jsonl': journal_fds.append(value)
            return value
        def fileio(fd, mode, **kwargs):
            constructed.append((fd, mode, kwargs))
            self.assertEqual(mode, 'wb')
            raise primary
        with mock.patch.object(os, 'open', opening), mock.patch.object(
                observer_session, 'io', types.SimpleNamespace(FileIO=fileio)):
            with self.assertRaises(OSError) as caught:
                observer_session.run_owned(self.fixture.spec, (str(output), pin(output)), self.nonce)
        self.assertIs(caught.exception, primary)
        self.assertEqual(len(journal_fds), 1)
        self.assertEqual(constructed, [(journal_fds[0], 'wb', {'closefd': False})])
        self.assertEqual((output/'journal.jsonl').read_bytes(), b'')
        self.assertFalse((output/'guest-terminal.json').exists())
        for acquired in opened:
            with self.assertRaises(OSError): os.fstat(acquired)


if __name__ == '__main__':
    unittest.main()
