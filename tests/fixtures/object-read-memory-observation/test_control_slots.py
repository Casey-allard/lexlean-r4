"""Tiny actual owned host files for fixed-slot publication, not virtiofs proof."""

import importlib.util
import os
import pathlib
import tempfile
import types
import unittest
from unittest import mock

import boundary
import observe
from test_owned_io import pin

if importlib.util.find_spec('control_slots'):
    import control_slots
else:
    control_slots = None


class ControlSlotsTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(control_slots, 'owned control publication is not implemented')
        temp = tempfile.TemporaryDirectory(prefix='r4-controls-')
        self.addCleanup(temp.cleanup)
        self.path = pathlib.Path(temp.name).resolve()
        self.pin = pin(self.path)
        self.fd = observe.open_pinned_directory(str(self.path), self.pin)
        self.addCleanup(os.close, self.fd)
        self.host = control_slots.OwnedControlSlots((self.fd, self.pin), 'host')
        self.guest = control_slots.OwnedControlSlots((self.fd, self.pin), 'guest')
        self.request = boundary.encode_request('a'*64, 1)

    def proxy(self, **overrides):
        names = ('open', 'close', 'fstat', 'read', 'write', 'fsync', 'lseek', 'stat', 'replace',
                 'getuid', 'O_RDONLY', 'O_RDWR', 'O_CREAT', 'O_EXCL', 'O_NOFOLLOW', 'O_CLOEXEC',
                 'O_NONBLOCK', 'SEEK_SET')
        return types.SimpleNamespace(**{n: overrides.get(n, getattr(os, n)) for n in names})

    def test_peer_missing_is_normal_and_publication_roundtrips_atomically(self):
        self.assertIsNone(self.guest.read_peer())
        self.host.publish(self.request)
        self.assertEqual(self.guest.read_peer(), self.request)
        ack = boundary.encode_ack(self.request, 1, '42', 'b'*64)
        self.guest.publish(ack)
        self.assertEqual(self.host.read_peer(), ack)
        second = boundary.encode_request('a'*64, 2)
        self.host.publish(second)
        self.assertEqual(self.guest.read_peer(), second)
        self.assertEqual(sorted(p.name for p in self.path.iterdir()), ['ack', 'request'])
        self.assertEqual(self.path.joinpath('request').stat().st_mode & 0o777, 0o600)
        os.fstat(self.fd)

    def test_wrong_role_and_oversize_reject_without_files(self):
        with self.assertRaises(observe.ObservationError): self.guest.publish(self.request)
        self.assertEqual(list(self.path.iterdir()), [])
        with self.assertRaises(observe.ObservationError): self.host.publish(b'x'*513)
        self.assertEqual(list(self.path.iterdir()), [])

    def test_unknown_existing_destination_or_temp_never_overwritten(self):
        for name in ('request', 'request_temp'):
            path = self.path/name; path.write_bytes(b'old')
            owner = control_slots.OwnedControlSlots((self.fd, self.pin), 'host')
            with self.assertRaises((observe.ObservationError, FileExistsError)):
                owner.publish(self.request)
            self.assertEqual(path.read_bytes(), b'old')
            path.unlink()

    def test_symlink_peer_or_temp_is_not_followed(self):
        target = self.path/'outside'; target.write_bytes(b'preserved')
        for name, action in (('request', self.guest.read_peer), ('request_temp', lambda: self.host.publish(self.request))):
            path = self.path/name; path.symlink_to(target)
            with self.assertRaises((OSError, observe.ObservationError)): action()
            self.assertEqual(target.read_bytes(), b'preserved')
            path.unlink()

    def test_partial_writes_complete_and_sync_precedes_rename(self):
        events = []
        def write(fd, raw): events.append('write'); return os.write(fd, raw[:3])
        def sync(fd): events.append('sync'); return os.fsync(fd)
        def replace(*args, **kwargs): events.append('rename'); return os.replace(*args, **kwargs)
        with mock.patch.object(control_slots, 'os', self.proxy(write=write, fsync=sync, replace=replace)):
            self.host.publish(self.request)
        self.assertEqual(events[-2:], ['sync', 'rename'])
        self.assertEqual(self.guest.read_peer(), self.request)

    def test_sync_failure_preserves_old_slot_and_stops_retry(self):
        self.host.publish(self.request)
        error = OSError('synthetic fsync')
        with mock.patch.object(control_slots, 'os', self.proxy(fsync=mock.Mock(side_effect=error))):
            with self.assertRaises(OSError) as caught:
                self.host.publish(boundary.encode_request('a'*64, 2))
        self.assertIs(caught.exception, error)
        self.assertEqual(self.guest.read_peer(), self.request)
        self.assertTrue((self.path/'request_temp').exists())
        with self.assertRaises(observe.ObservationError): self.host.publish(self.request)

    def test_peer_oversize_mode_and_fifo_reject(self):
        path = self.path/'request'
        path.write_bytes(b'x'*513); path.chmod(0o600)
        with self.assertRaises(observe.ObservationError): self.guest.read_peer()
        path.write_bytes(self.request); path.chmod(0o644)
        fresh = control_slots.OwnedControlSlots((self.fd, self.pin), 'guest')
        with self.assertRaises(observe.ObservationError): fresh.read_peer()
        path.unlink(); os.mkfifo(path, 0o600)
        fresh = control_slots.OwnedControlSlots((self.fd, self.pin), 'guest')
        with self.assertRaises(observe.ObservationError): fresh.read_peer()

    def test_destination_changed_since_owned_publication_rejects(self):
        self.host.publish(self.request)
        (self.path/'request').write_bytes(boundary.encode_request('a'*64, 3))
        with self.assertRaises(observe.ObservationError):
            self.host.publish(boundary.encode_request('a'*64, 2))
        self.assertEqual((self.path/'request').read_bytes(), boundary.encode_request('a'*64, 3))

    def test_rename_is_last_fallible_operation(self):
        state = {'renamed': False}
        def guard(operation):
            def call(*args, **kwargs):
                self.assertFalse(state['renamed'])
                return operation(*args, **kwargs)
            return call
        def replace(*args, **kwargs):
            os.replace(*args, **kwargs); state['renamed'] = True
        proxy = self.proxy(**{n: guard(getattr(os,n)) for n in ('open','close','fstat','read','write','fsync','lseek','stat')}, replace=replace)
        with mock.patch.object(control_slots, 'os', proxy): self.host.publish(self.request)
        self.assertTrue(state['renamed'])

    def test_borrowed_root_survives_publication_failure(self):
        with mock.patch.object(control_slots, 'os', self.proxy(write=mock.Mock(return_value=0))):
            with self.assertRaises(observe.ObservationError): self.host.publish(self.request)
        os.fstat(self.fd)

    def test_enoent_after_open_is_fatal_not_missing_poll(self):
        self.host.publish(self.request)
        error = FileNotFoundError('synthetic read failure after successful open')
        with mock.patch.object(control_slots, 'os', self.proxy(read=mock.Mock(side_effect=error))):
            with self.assertRaises(FileNotFoundError) as caught: self.guest.read_peer()
        self.assertIs(caught.exception, error)

    def test_replaced_prepared_temp_cannot_be_published(self):
        original = self.host._prepare
        replacement = boundary.encode_request('a'*64, 3)
        def prepare(raw, name):
            result = original(raw, name)
            self.path.joinpath(name).write_bytes(replacement)
            return result
        with mock.patch.object(self.host, '_prepare', prepare):
            with self.assertRaises(observe.ObservationError): self.host.publish(self.request)
        self.assertFalse((self.path/'request').exists())
