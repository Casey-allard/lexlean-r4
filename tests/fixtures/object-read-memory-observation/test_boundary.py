"""Finite boundary wire tests; no channel IO or native admission."""

import base64
import io
import json
import unittest

import boundary
from observe import Journal, ObservationError


class WireTests(unittest.TestCase):
    def test_exact_request_and_ack_encoding(self):
        nonce, binding = 'a' * 64, 'b' * 64
        request = boundary.encode_request(nonce, 7)
        self.assertEqual(request,
                         ('["q",1,"' + nonce + '",7,"before-verify"]\n').encode())
        self.assertEqual(boundary.decode_request(request), (nonce, 7, 'before-verify'))
        ack = boundary.encode_ack(request, 32, '5432', binding)
        self.assertEqual(ack, ('["a",1,"' + nonce
                              + '",7,"before-verify",32,"5432","5432","'
                              + binding + '"]\n').encode())
        self.assertEqual(boundary.decode_ack(ack),
                         (nonce, 7, 'before-verify', 32, '5432', '5432', binding))

    def test_wire_rejects_noncanonical_shapes_and_bounded_field_errors(self):
        q = ['q', 1, 'a' * 64, 7, 'before-verify']
        a = ['a', 1, 'a' * 64, 7, 'before-verify', 32, '5432', '5432', 'b' * 64]
        for value, decode in ((q, boundary.decode_request), (a, boundary.decode_ack)):
            raw = (json.dumps(value, separators=(',', ':')) + '\n').encode()
            malformed = [None, bytearray(raw), raw[:-1], raw + b'\n', b' ' + raw,
                         b'\xef\xbb\xbf' + raw, b'x' * 513,
                         (json.dumps(value) + '\n').encode(), b'[]\n', b'null\n']
            changes = [(0, 'bad'), (1, True), (1, 2), (2, 'A' * 64),
                       (2, ''), (3, 0), (3, 12), (3, True), (3, 7.0),
                       (4, 'copy-start'), (4, None)]
            if value is a:
                changes += [(5, 0), (5, True), (5, 1013761), (6, '0'),
                            (6, '05432'), (6, 5432), (6, '1013761'),
                            (7, '5431'), (8, 'g' * 64)]
            for position, replacement in changes:
                changed = list(value)
                changed[position] = replacement
                malformed.append((json.dumps(changed, separators=(',', ':')) + '\n').encode())
            for bad in malformed:
                with self.subTest(kind=value[0], raw=repr(bad)[:100]):
                    with self.assertRaises(ObservationError):
                        decode(bad)

    def test_encoder_rejects_invalid_values_before_publishing_wire(self):
        for nonce, seq in (('z' * 64, 1), ('a' * 64, True), ('a' * 64, 0),
                           ('a' * 64, 12), (None, 1)):
            with self.subTest(nonce=nonce, seq=seq), self.assertRaises(ObservationError):
                boundary.encode_request(nonce, seq)
        request = boundary.encode_request('a' * 64, 1)
        for sample, offset, binding in ((True, '5', 'b' * 64), (0, '5', 'b' * 64),
                                        (1, '05', 'b' * 64), (1, '1', None)):
            with self.subTest(sample=sample, offset=offset), self.assertRaises(ObservationError):
                boundary.encode_ack(request, sample, offset, binding)

    def test_all_labels_fit_slots_with_maximal_wire_numbers(self):
        for sequence, label in enumerate(boundary.LABELS, 1):
            request = boundary.encode_request('f' * 64, sequence)
            ack = boundary.encode_ack(request, 1013760, '1013760', 'f' * 64)
            self.assertLessEqual(len(request), 512)
            self.assertLessEqual(len(ack), 512)
            self.assertEqual(boundary.decode_request(request)[2], label)
            self.assertEqual(boundary.decode_ack(ack)[3:6],
                             (1013760, '1013760', '1013760'))


class BoundaryWireTests(unittest.TestCase):
    def test_eleven_boundaries_and_unchanged_old_ack_poll(self):
        nonce, binding = 'a' * 64, 'b' * 64
        state = boundary.BoundaryWire(nonce, binding)
        last_ack = None
        for sequence in range(1, 12):
            start = sequence * 10000000000
            request = state.request(start)
            self.assertEqual(boundary.decode_request(request)[1], sequence)
            self.assertIsNone(state.poll_ack(last_ack, start + 1))
            ack = boundary.encode_ack(request, sequence, str(100 * sequence), binding)
            self.assertEqual(state.poll_ack(ack, start + 2), boundary.decode_ack(ack))
            last_ack = ack
        with self.assertRaises(ObservationError):
            state.request(120000000000)

    def test_mismatch_and_malformed_ack_permanently_fail(self):
        for position, replacement in ((2, 'c' * 64), (3, 2), (8, 'c' * 64),
                                       (5, True), (6, '0')):
            state = boundary.BoundaryWire('a' * 64, 'b' * 64)
            request = state.request(10)
            value = json.loads(boundary.encode_ack(request, 1, '100', 'b' * 64))
            value[position] = replacement
            if position == 3:
                value[4] = boundary.LABELS[replacement - 1]
            raw = (json.dumps(value, separators=(',', ':')) + '\n').encode()
            with self.subTest(position=position):
                with self.assertRaises(ObservationError):
                    state.poll_ack(raw, 11)
                with self.assertRaises(ObservationError):
                    state.poll_ack(boundary.encode_ack(request, 1, '100', 'b' * 64), 12)
                with self.assertRaises(ObservationError):
                    state.request(13)

    def test_sample_and_prefix_must_advance(self):
        for sample, offset in ((1, '101'), (2, '100'), (1, '99')):
            state = boundary.BoundaryWire('a' * 64, 'b' * 64)
            request = state.request(0)
            state.poll_ack(boundary.encode_ack(request, 1, '100', 'b' * 64), 1)
            request = state.request(2)
            with self.subTest(sample=sample, offset=offset), self.assertRaises(ObservationError):
                state.poll_ack(boundary.encode_ack(request, sample, offset, 'b' * 64), 3)

    def test_deadline_invalid_clock_and_second_request_fail_closed(self):
        for now in (-1, True, 0.5, 2**64):
            state = boundary.BoundaryWire('a' * 64, 'b' * 64)
            with self.subTest(now=now), self.assertRaises(ObservationError):
                state.request(now)
            with self.assertRaises(ObservationError):
                state.request(0)
        for later in (9, True, 2000000011):
            state = boundary.BoundaryWire('a' * 64, 'b' * 64)
            request = state.request(10)
            with self.subTest(later=later), self.assertRaises(ObservationError):
                state.poll_ack(None, later)
            with self.assertRaises(ObservationError):
                state.poll_ack(boundary.encode_ack(request, 1, '100', 'b' * 64), 11)
        state = boundary.BoundaryWire('a' * 64, 'b' * 64)
        state.request(10)
        with self.assertRaises(ObservationError):
            state.request(11)
        with self.assertRaises(ObservationError):
            state.poll_ack(None, 12)

    def test_exact_host_deadline_and_mutable_old_slot(self):
        state = boundary.BoundaryWire('a' * 64, 'b' * 64)
        request = state.request(9007199254740993)
        ack = boundary.encode_ack(request, 1, '100', 'b' * 64)
        self.assertEqual(state.poll_ack(ack, 9007199256740993), boundary.decode_ack(ack))
        state.request(9007199256740994)
        with self.assertRaises(ObservationError):
            state.poll_ack(bytearray(ack), 9007199256740995)


class RetainedAckTests(unittest.TestCase):
    def test_ack_names_synced_prefix_and_its_exact_copy_precedes_return(self):
        stream = io.BytesIO()
        synced = []
        journal = Journal(stream, lambda: synced.append(stream.tell()))
        journal.append(b'["s",1]\n')
        request = boundary.encode_request('a' * 64, 7)
        ack = boundary.retain_ack_copy(journal, request, 1, '1000', 'b' * 64)
        rows = [json.loads(line) for line in stream.getvalue().splitlines()]
        decoded = boundary.decode_ack(ack)
        self.assertEqual(len(synced), 2)
        self.assertEqual(int(decoded[4]), synced[0])
        self.assertEqual(int(decoded[5]), synced[0])
        self.assertLess(synced[0], synced[1])
        self.assertEqual(journal.synced_offset, synced[1])
        self.assertEqual(base64.b64decode(rows[1][-1][1]), request)
        self.assertIsNone(rows[1][-1][2])
        self.assertIsNone(rows[2][-1][1])
        self.assertEqual(base64.b64decode(rows[2][-1][2]), ack)

    def test_invalid_context_and_two_row_quota_fail_before_any_write(self):
        request = boundary.encode_request('a' * 64, 7)
        for sample, received, binding in ((True, '10', 'b' * 64),
                                         (1, None, 'b' * 64),
                                         (1, '01', 'b' * 64),
                                         (1, '10', 'bad')):
            stream = io.BytesIO()
            journal = Journal(stream, lambda: None)
            journal.append(b'["s",1]\n')
            before = stream.getvalue()
            with self.subTest(sample=sample, received=received):
                with self.assertRaises(ObservationError):
                    boundary.retain_ack_copy(journal, request, sample, received, binding)
                self.assertEqual(stream.getvalue(), before)
                self.assertTrue(journal.stopped)
        stream = io.BytesIO()
        journal = Journal(stream, lambda: None)
        journal.append(b'["s",1]\n')
        journal._reserved['markers'] = 57344 - 250
        before = stream.getvalue()
        with self.assertRaisesRegex(ObservationError, 'quota_stop'):
            boundary.retain_ack_copy(journal, request, 1, '10', 'b' * 64)
        self.assertEqual(stream.getvalue(), before)
        self.assertTrue(journal.stopped)

    def test_empty_prefix_cannot_yield_ack_and_prior_sync_status_is_accurate(self):
        request = boundary.encode_request('a' * 64, 1)
        stream = io.BytesIO()
        journal = Journal(stream, lambda: None)
        with self.assertRaises(ObservationError):
            boundary.retain_ack_copy(journal, request, 1, '10', 'b' * 64)
        self.assertEqual(stream.getvalue(), b'')
        for synced in (False, True):
            stream = io.BytesIO()
            journal = Journal(stream, lambda: None)
            journal.append(b'["s",1]\n')
            if synced:
                journal.sync()
            boundary.retain_ack_copy(journal, request, 1, '10', 'b' * 64)
            rows = [json.loads(line) for line in stream.getvalue().splitlines()]
            self.assertEqual(rows[1][6], 'synced' if synced else 'unsynced')
            self.assertEqual(rows[2][6], 'synced')

    def test_each_write_and_sync_failure_prevents_return_and_retry(self):
        for phase in ('first-write', 'first-sync', 'second-write', 'second-sync'):
            failure = OSError(phase)

            class Stream(io.BytesIO):
                writes = 0

                def write(self, raw):
                    self.writes += 1
                    if self.writes == (2 if phase == 'first-write' else
                                       3 if phase == 'second-write' else -1):
                        super().write(raw[:3])
                        raise failure
                    return super().write(raw)

            stream = Stream()
            syncs = []

            def sync():
                syncs.append(stream.tell())
                if len(syncs) == (1 if phase == 'first-sync' else
                                  2 if phase == 'second-sync' else -1):
                    raise failure

            journal = Journal(stream, sync)
            journal.append(b'["s",1]\n')
            request = boundary.encode_request('a' * 64, 7)
            with self.subTest(phase=phase):
                with self.assertRaises(OSError) as caught:
                    boundary.retain_ack_copy(journal, request, 1, '1000', 'b' * 64)
                self.assertIs(caught.exception, failure)
                self.assertTrue(journal.stopped)
                before = stream.getvalue()
                with self.assertRaises(ObservationError):
                    boundary.retain_ack_copy(journal, request, 1, '1000', 'b' * 64)
                self.assertEqual(stream.getvalue(), before)
                self.assertLessEqual(journal.synced_offset, journal.complete_offset)


if __name__ == '__main__':
    unittest.main()
