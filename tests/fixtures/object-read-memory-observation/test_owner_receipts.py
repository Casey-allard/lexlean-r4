"""Owner retention policy and fixed receipt contracts; no native invocation."""

import io
import json
import unittest

import boundary
import observe
import root_linkage
import test_sample


class OwnerReceiptTests(unittest.TestCase):
    nonce, digest = 'a'*64, 'b'*64
    boot = '12345678-1234-1234-1234-123456789abc'

    def test_lifecycle_records_are_canonical_and_retained_once(self):
        self.assertTrue(hasattr(observe, 'encode_owner_launch'), 'owner codec missing')
        launch = observe.encode_owner_launch(self.nonce, self.digest, ['41', '0'],
                                              ['42', str(2**64-1)], self.boot)
        self.assertEqual(observe.decode_owner(launch), json.loads(launch))
        ended = observe.encode_owner_exit(self.nonce, self.digest, '42', None,
                                          ['exited', 127, None], 'natural')
        self.assertEqual(observe.decode_owner(ended)[6], ['exited', 127, None])
        retention = observe.Retention()
        stream = io.BytesIO()
        retention.write_owner('launch', launch, stream, lambda: None)
        self.assertEqual(stream.getvalue(), launch)
        with self.assertRaises(observe.ObservationError):
            retention.write_owner('launch', launch, stream, lambda: None)
        retention.write_owner('exit', ended, io.BytesIO(), lambda: None)

    def test_exit_state_reason_matrix_rejects_invented_outcomes(self):
        self.assertTrue(hasattr(observe, 'encode_owner_exit'), 'owner exit codec missing')
        states = {'not_run': [None, None, ['not_run', None, None]],
                  'exited': ['42', '0', ['exited', 0, None]],
                  'signaled': ['42', None, ['signaled', None, 15]],
                  'unknown': ['42', None, ['unknown', None, None]]}
        allowed = {'not_run': ('not_started', 'fork_failed'),
                   'exited': ('natural', 'host_eof', 'owner_error'),
                   'signaled': ('natural', 'host_eof', 'owner_error'),
                   'unknown': ('lost_custody', 'reap_timeout', 'owner_error')}
        reasons = ('not_started', 'fork_failed', 'natural', 'host_eof',
                   'owner_error', 'lost_custody', 'reap_timeout', 'exec_failed')
        for state, fields in states.items():
            for reason in reasons:
                with self.subTest(state=state, reason=reason):
                    if reason in allowed[state]:
                        raw = observe.encode_owner_exit(self.nonce, self.digest, *fields, reason)
                        self.assertEqual(observe.decode_owner(raw)[-1], reason)
                    else:
                        with self.assertRaises(observe.ObservationError):
                            observe.encode_owner_exit(self.nonce, self.digest, *fields, reason)

    def test_owner_wire_rejects_noncanonical_types_and_corruption(self):
        self.assertTrue(hasattr(observe, 'decode_owner'), 'owner wire decoder missing')
        row = ['ol', 1, self.nonce, self.digest, ['41', '0'], ['42', '1'], self.boot]
        valid = observe._encoded(row, 512)
        for raw in (b'', valid+b' ', valid[:-1], b' '*513, b'\xff', b'[NaN]\n',
                    b'[]\n', json.dumps(row).encode()+b'\n'):
            with self.subTest(raw=raw[:30]), self.assertRaises(observe.ObservationError):
                observe.decode_owner(raw)
        for index, value in ((0, 'other'), (1, True), (2, 'A'*64), (3, ''),
                             (4, ['0', '0']), (4, ['41', None]), (5, ['42', True]),
                             (5, ['42', str(2**64)]), (6, 'bad')):
            changed = row.copy(); changed[index] = value
            with self.assertRaises(observe.ObservationError):
                observe.decode_owner(observe._encoded(changed, 512))
        for process in (['running', None, None], ['exited', True, None],
                        ['exited', 256, None], ['signaled', None, 65]):
            with self.assertRaises(observe.ObservationError):
                observe.encode_owner_exit(self.nonce, self.digest, '42', '0', process, 'natural')

    def test_receipt_attempts_are_consumed_before_validation_or_failed_io(self):
        self.assertTrue(hasattr(observe.Retention, 'write_owner'), 'owner retention missing')
        retention = observe.Retention()
        with self.assertRaises(observe.ObservationError):
            retention.write_owner('launch', b'invalid', io.BytesIO(), lambda: None)
        self.assertTrue(retention.state('observer_launch')['attempted'])
        with self.assertRaises(observe.ObservationError):
            retention.write_owner('launch', b'invalid', io.BytesIO(), lambda: None)
        # The storage cap is distinct from the schema's maximum valid encoding.
        retention._write('observer_exit', b'x'*512, io.BytesIO(), lambda: None)
        with self.assertRaises(observe.ObservationError):
            observe.Retention()._write('observer_exit', b'x'*513, io.BytesIO(), lambda: None)
    def test_parent_slots_and_child_journal_share_one_reservation(self):
        journal = observe.Journal(io.BytesIO(), lambda: None)
        slots = observe.Retention().slots
        self.assertEqual(journal.reserved['markers'], 27648)
        self.assertEqual(slots['observer_launch'], 512)
        self.assertEqual(slots['observer_exit'], 512)
        self.assertEqual(sum(slots.values()), 35840)
        self.assertEqual(786432 + 196608 + 57344 - 27648 + sum(slots.values()), 1048576)

    def test_successor_bounds_and_header_profile_are_enforced(self):
        for value in ('1012737', '1013760'):
            with self.assertRaises(observe.ObservationError):
                observe._prefix([value, value, '1', value, True])
            with self.assertRaises(observe.ObservationError):
                boundary.encode_ack(boundary.encode_request('a'*64, 1), 1, value, 'b'*64)
        with self.assertRaises(observe.ObservationError):
            observe._quotas([0, 0, 27647, 8192])
        fixture = test_sample.SampleTests()
        fixture.setUp(); self.addCleanup(fixture.doCleanups)
        sampler = fixture.sampler()
        sampler.capture_boundary(boundary.encode_request(fixture.nonce, 1))
        import json
        header = json.loads(fixture.stream.getvalue().splitlines()[0])
        self.assertEqual(header[9]['quotaProfile'], 'owner-retention/1')
        context = root_linkage.RootContext(fixture.nonce, fixture.identity, 1, 2, 0)
        root_linkage._header(header, context)
        del header[9]['quotaProfile']
        with self.assertRaises(observe.ObservationError):
            root_linkage._header(header, context)
