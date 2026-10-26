"""Owned synthetic cgroup fixtures for sample-to-ACK composition; no native run."""

import base64
import io
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock

import boundary
import observe
from test_identity import stat, status
from test_owned_io import pin


class SampleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='r4-sample-')
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name).resolve()
        self.cg, self.proc = self.root / 'cg', self.root / 'proc'
        self.cg.mkdir(mode=0o700)
        self.proc.mkdir(mode=0o700)
        files = {'cgroup.procs': b'41\n', 'memory.events': self.events(),
                 'memory.current': b'123\n', 'memory.swap.current': b'0\n',
                 'memory.peak': b'456\n', 'memory.stat': b'anon 100\nfile 23\n',
                 'memory.pressure': b'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n',
                 'memory.events.local': self.events()}
        for name, value in files.items():
            (self.cg / name).write_bytes(value)
        self.add_pid('41')
        self.cpin, self.ppin = pin(self.cg), pin(self.proc)
        self.cfd = observe.open_pinned_directory(str(self.cg), self.cpin)
        self.addCleanup(os.close, self.cfd)
        self.pfd = observe.open_pinned_directory(str(self.proc), self.ppin)
        self.addCleanup(os.close, self.pfd)
        self.identity = ('a' * 64, 'sha256:' + 'b' * 64,
                         '2026-10-26T00:00:00Z', '12345678-1234-1234-1234-123456789abc',
                         ('/owned', self.cpin[0], self.cpin[1], self.cpin[3]),
                         ('99', '100', '/observer'))
        self.nonce = 'c' * 64
        self.stream = io.BytesIO()
        self.syncs = []
        self.journal = observe.Journal(self.stream, lambda: self.syncs.append(self.stream.tell()))
        self.now = 1000000000
        self.identity_calls = 0

    @staticmethod
    def events(value='0'):
        return ('low 0\nhigh 0\nmax ' + value
                + '\noom 0\noom_kill 0\noom_group_kill 0\nnew_counter 0\n').encode()

    def add_pid(self, pid):
        directory = self.proc / pid
        directory.mkdir()
        for name, value in {'stat': stat(pid, '7', '900'), 'status': status(pid, '7'),
                            'cgroup': b'0::/owned\n', 'cmdline': b'fixture\0'}.items():
            (directory / name).write_bytes(value)

    def clock(self):
        self.now += 1000
        return self.now

    def read_identity(self):
        self.identity_calls += 1
        return self.identity

    def sampler(self):
        self.stream, self.syncs = io.BytesIO(), []
        self.journal = observe.Journal(self.stream, lambda: self.syncs.append(self.stream.tell()))
        return boundary.SampleBoundary(self.journal, self.nonce, self.identity,
                                       1000000000, (self.cfd, self.cpin),
                                       (self.pfd, self.ppin), b'/owned',
                                       self.read_identity, self.clock)

    def test_owned_capture_retains_complete_fresh_sample_before_ack(self):
        sampler = self.sampler()
        request = boundary.encode_request(self.nonce, 1)
        ack = boundary.decode_ack(sampler.capture_boundary(request))
        rows = [json.loads(line) for line in self.stream.getvalue().splitlines()]
        self.assertEqual([r[0] for r in rows], ['h', 'e', 'i', 's', 'm', 'b', 'b'])
        self.assertEqual(rows[0][1:4], [3, self.nonce, self.identity[3]])
        self.assertEqual(rows[1][2], sorted(observe.parse_events(self.events())))
        self.assertEqual(rows[3][5], ['0'] * 7)
        self.assertEqual(rows[3][6:9], ['123', '0', '456'])
        self.assertEqual(rows[4][4:7], [p.read_text() for p in
                         (self.cg/'memory.stat', self.cg/'memory.pressure',
                          self.cg/'memory.events.local')])
        self.assertEqual(base64.b64decode(rows[2][11]), (self.proc/'41'/'stat').read_bytes())
        self.assertEqual(base64.b64decode(rows[2][12]), (self.proc/'41'/'stat').read_bytes())
        self.assertEqual(rows[2][6], 'owned_unattributed')
        self.assertEqual(rows[3][9][0][0], rows[2][1])
        self.assertGreaterEqual(self.identity_calls, 3)
        self.assertEqual(ack[3], rows[3][1])
        self.assertEqual(ack[4], str(self.syncs[0]))
        self.assertGreater(self.syncs[0], sum(len(x)+1 for x in
                           self.stream.getvalue().splitlines()[:4]))
        self.assertEqual(self.syncs[-1], len(self.stream.getvalue()))

    def test_mandatory_numeric_and_slow_syntax_cannot_earn_ack(self):
        cases = [('memory.current', b'123'), ('memory.swap.current', b'0x'),
                 ('memory.stat', b'anon nope\n'), ('memory.stat', b'anon 0\nanon 1\n'),
                 ('memory.pressure', b'some avg10=NaN avg60=0.00 avg300=0.00 total=0\n'),
                 ('memory.pressure', b'some avg10=0.00 total=0\n'),
                 ('memory.events.local', b'max 0\n')]
        for name, bad in cases:
            path = self.cg/name
            original = path.read_bytes()
            sampler = self.sampler()
            try:
                path.write_bytes(bad)
                with self.subTest(name=name, bad=bad), self.assertRaises(observe.ObservationError):
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
            finally:
                path.write_bytes(original)

    def test_optional_absence_has_derived_flags_but_denial_is_not_absence(self):
        for name in ('memory.peak', 'memory.pressure', 'memory.events.local'):
            (self.cg/name).unlink()
        sampler = self.sampler()
        sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        rows = [json.loads(x) for x in self.stream.getvalue().splitlines()]
        ordinary = next(x for x in rows if x[0] == 's')
        slow = next(x for x in rows if x[0] == 'm')
        self.assertEqual((ordinary[8], ordinary[11]), (None, 1))
        self.assertEqual((slow[5], slow[6], slow[7]), (None, None, 3))

    def test_nonzero_history_never_becomes_zero_preverify_admission(self):
        (self.cg/'memory.events').write_bytes(self.events('1'))
        sampler = self.sampler()
        for seq in range(1, 7):
            sampler.capture_boundary(boundary.encode_request(self.nonce, seq))
        before = len(self.stream.getvalue())
        with self.assertRaisesRegex(observe.ObservationError, 'nonzero'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 7))
        later = [json.loads(x) for x in self.stream.getvalue()[before:].splitlines()]
        self.assertIn('s', [x[0] for x in later])
        self.assertIn('m', [x[0] for x in later])
        self.assertNotIn('b', [x[0] for x in later])

    def test_counter_decrease_and_unknown_key_disappearance_fail_continuity(self):
        for next_events in (self.events('0'), self.events('1').replace(b'new_counter 0\n', b'')):
            (self.cg/'memory.events').write_bytes(self.events('1'))
            sampler = self.sampler()
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
            (self.cg/'memory.events').write_bytes(next_events)
            with self.subTest(next_events=next_events), self.assertRaisesRegex(
                    observe.ObservationError, 'continuity'):
                sampler.capture_boundary(boundary.encode_request(self.nonce, 2))

    def test_invalid_constructor_context_rejects_without_observation_or_output(self):
        cases = [None, list(self.identity), (*self.identity[:4], ('/wrong', *self.identity[4][1:]),
                                           self.identity[5]),
                 ('bad', *self.identity[1:]),
                 (*self.identity[:3], 'bad-boot', *self.identity[4:]),
                 (*self.identity[:5], ('99', '100', '/owned'))]
        for identity in cases:
            calls = []
            stream = io.BytesIO()
            journal = observe.Journal(stream, lambda: None)
            with self.subTest(identity=identity), self.assertRaises(observe.ObservationError):
                boundary.SampleBoundary(journal, self.nonce, identity, 1000000000,
                                        (self.cfd, self.cpin), (self.pfd, self.ppin),
                                        b'/owned', lambda: calls.append(1), self.clock)
            self.assertEqual(calls, [])
            self.assertEqual(stream.getvalue(), b'')

    def test_external_journal_mutation_poisoned_requests_and_clock_regression(self):
        sampler = self.sampler()
        sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.journal.append(b'["g",1]\n')
        with self.assertRaisesRegex(observe.ObservationError, 'journal'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 2))
        sampler = self.sampler()
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 2))
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        sampler = self.sampler()
        sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.now = 1000000000
        with self.assertRaisesRegex(observe.ObservationError, 'clock'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 2))

    def test_reentrant_callback_cannot_mask_failure_and_return_ack(self):
        sampler = self.sampler()
        request = boundary.encode_request(self.nonce, 1)
        entered = False

        def clock():
            nonlocal entered
            if not entered:
                entered = True
                try:
                    sampler.capture_boundary(request)
                except observe.ObservationError:
                    pass
            return self.clock()

        sampler.clock = clock
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(request)

    def test_identity_and_dictionary_limits_stop_before_excess_records(self):
        sampler = self.sampler()
        sampler.identity_count = 1024
        with self.assertRaisesRegex(observe.ObservationError, 'identity_limit'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertNotIn('i', [json.loads(x)[0] for x in self.stream.getvalue().splitlines()])
        sampler = self.sampler()
        sampler.keyset = 32
        with self.assertRaisesRegex(observe.ObservationError, 'keyset_limit'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertNotIn('e', [json.loads(x)[0] for x in self.stream.getvalue().splitlines()])

    def test_capture_failure_retains_typed_gap_and_original_error(self):
        failure = OSError(5, 'synthetic read failure')
        sampler = self.sampler()
        real = observe.read_observation_file

        def read(fd, identity, name):
            if name == 'memory.stat':
                raise failure
            return real(fd, identity, name)

        with mock.patch.object(observe, 'read_observation_file', read):
            with self.assertRaises(OSError) as caught:
                sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertIs(caught.exception, failure)
        rows = [json.loads(x) for x in self.stream.getvalue().splitlines()]
        self.assertEqual(rows[-1][0], 'g')
        self.assertEqual(rows[-1][-1], ['memory.stat', 5])
        self.assertNotIn('synthetic', self.stream.getvalue().decode())
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))

    def test_gap_field_names_the_failed_parser(self):
        (self.cg/'memory.pressure').write_bytes(b'invalid\n')
        sampler = self.sampler()
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        gap = json.loads(self.stream.getvalue().splitlines()[-1])
        self.assertEqual(gap[-1][0], 'memory.pressure')

    def test_invalid_clock_end_stays_unknown_in_gap(self):
        (self.cg/'memory.pressure').unlink()
        sampler = self.sampler()
        times = iter([1000002000, 1000001000])
        sampler.clock = lambda: next(times)
        with self.assertRaises(observe.ObservationError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        gap = json.loads(self.stream.getvalue().splitlines()[-1])
        self.assertEqual(gap[2:4], ['2', None])

    def test_zero_history_unknown_keys_and_local_nonzero_are_checked(self):
        sampler = self.sampler()
        for seq in range(1, 8):
            ack = boundary.decode_ack(sampler.capture_boundary(boundary.encode_request(self.nonce, seq)))
            self.assertEqual(ack[3], seq)
        for field in ('memory.events', 'memory.events.local'):
            (self.cg/field).write_bytes(self.events().replace(b'new_counter 0', b'new_counter 1'))
            sampler = self.sampler()
            for seq in range(1, 7):
                sampler.capture_boundary(boundary.encode_request(self.nonce, seq))
            with self.subTest(field=field), self.assertRaisesRegex(observe.ObservationError, 'nonzero'):
                sampler.capture_boundary(boundary.encode_request(self.nonce, 7))
            (self.cg/field).write_bytes(self.events())

    def test_every_original_census_pid_is_retained_or_capture_fails(self):
        self.add_pid('42')
        (self.cg/'cgroup.procs').write_bytes(b'41\n42\n')
        sampler = self.sampler()
        sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        rows = [json.loads(x) for x in self.stream.getvalue().splitlines()]
        self.assertEqual([x[2] for x in rows if x[0] == 'i'], ['41', '42'])
        self.assertEqual(len(next(x for x in rows if x[0] == 's')[9]), 2)
        (self.proc/'42'/'stat').unlink()
        sampler = self.sampler()
        with self.assertRaises(FileNotFoundError):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertNotIn('s', [json.loads(x)[0] for x in self.stream.getvalue().splitlines()])

    def test_optional_denial_and_final_identity_change_never_earn_ack(self):
        sampler = self.sampler()
        failure = PermissionError(13, 'denied')
        real = observe.read_observation_file
        def read(fd, identity, name):
            if name == 'memory.peak':
                raise failure
            return real(fd, identity, name)
        with mock.patch.object(observe, 'read_observation_file', read):
            with self.assertRaises(PermissionError) as caught:
                sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertIs(caught.exception, failure)
        sampler = self.sampler()
        identities = iter([self.identity, self.identity, ('d' * 64, *self.identity[1:])])
        sampler.read_identity = lambda: next(identities)
        with self.assertRaisesRegex(observe.ObservationError, 'identity'):
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        tags = [json.loads(x)[0] for x in self.stream.getvalue().splitlines()]
        self.assertIn('m', tags)
        self.assertNotIn('b', tags)

    def test_replaced_borrowed_stream_or_sync_blocks_without_new_writes(self):
        for field in ('stream', 'synchronize'):
            sampler = self.sampler()
            sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
            old = self.stream.getvalue()
            replacement, calls = io.BytesIO(), []
            setattr(self.journal, field, replacement if field == 'stream' else lambda: calls.append(1))
            with self.subTest(field=field):
                with self.assertRaisesRegex(observe.ObservationError, 'owner'):
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 2))
                self.assertEqual(replacement.getvalue(), b'')
                self.assertEqual(self.stream.getvalue(), old)
                self.assertEqual(calls, [])

    def test_replacement_during_sync_cannot_return_ack(self):
        for field in ('stream', 'synchronize'):
            sampler = self.sampler()
            old_sync = self.journal.synchronize
            replaced = False
            def sync():
                nonlocal replaced
                old_sync()
                if not replaced:
                    replaced = True
                    setattr(self.journal, field, io.BytesIO() if field == 'stream' else lambda: None)
            # The injected callback is the original owner for this new sampler.
            self.journal.synchronize = sync
            sampler = boundary.SampleBoundary(self.journal, self.nonce, self.identity,
                                               1000000000, (self.cfd, self.cpin),
                                               (self.pfd, self.ppin), b'/owned',
                                               self.read_identity, self.clock)
            with self.subTest(field=field):
                with self.assertRaisesRegex(observe.ObservationError, 'owner'):
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
                self.assertTrue(self.journal.stopped)

    def sampler_with_journal(self, stream, sync):
        self.journal = observe.Journal(stream, sync)
        return boundary.SampleBoundary(self.journal, self.nonce, self.identity,
                                       1000000000, (self.cfd, self.cpin),
                                       (self.pfd, self.ppin), b'/owned',
                                       self.read_identity, self.clock)

    def test_partial_sample_and_slow_write_never_ack_or_retry(self):
        for tag in (b'["s",', b'["m",'):
            failure = OSError(5, 'synthetic write failure')
            class Stream(io.BytesIO):
                partial = False
                def write(this, raw):
                    if this.partial:
                        raise failure
                    if bytes(raw).startswith(tag):
                        this.partial = True
                        return super(Stream, this).write(raw[:3])
                    return super(Stream, this).write(raw)
            stream, syncs = Stream(), []
            sampler = self.sampler_with_journal(stream, lambda: syncs.append(1))
            with self.subTest(tag=tag):
                with self.assertRaises(OSError) as caught:
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
                self.assertIs(caught.exception, failure)
                self.assertFalse(self.journal.offset_certain)
                self.assertEqual(syncs, [])
                before = stream.getvalue()
                self.assertGreater(len(before), self.journal.complete_offset)
                with self.assertRaises(observe.ObservationError):
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
                self.assertEqual(stream.getvalue(), before)

    def test_first_or_second_sync_failure_preserves_cause_and_stops(self):
        for position in (1, 2):
            failure, calls = OSError(5, 'synthetic fsync failure'), []
            def sync():
                calls.append(1)
                if len(calls) == position:
                    raise failure
            stream = io.BytesIO()
            sampler = self.sampler_with_journal(stream, sync)
            with self.subTest(position=position):
                with self.assertRaises(OSError) as caught:
                    sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
                self.assertIs(caught.exception, failure)
                self.assertTrue(self.journal.stopped)
                self.assertLess(self.journal.synced_offset, self.journal.complete_offset)
                self.assertEqual(len(calls), position)

    def test_gap_write_failure_preserves_primary_and_secondary(self):
        primary, secondary = OSError(5, 'read failure'), OSError(28, 'gap failure')
        class Stream(io.BytesIO):
            def write(this, raw):
                if bytes(raw).startswith(b'["g",'):
                    raise secondary
                return super(Stream, this).write(raw)
        sampler = self.sampler_with_journal(Stream(), lambda: None)
        original = observe.read_observation_file
        def read(fd, identity, name):
            if name == 'memory.stat':
                raise primary
            return original(fd, identity, name)
        with mock.patch.object(observe, 'read_observation_file', read):
            with self.assertRaises(BaseExceptionGroup) as caught:
                sampler.capture_boundary(boundary.encode_request(self.nonce, 1))
        self.assertEqual(caught.exception.exceptions, (primary, secondary))
        self.assertTrue(self.journal.stopped)
