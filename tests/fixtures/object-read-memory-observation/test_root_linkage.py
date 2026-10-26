"""Synthetic actual h3 captures; no live exec, namespace or receipt authority."""

import base64
import dataclasses
import importlib.util
import json
import unittest

import boundary
import observe
import test_sample
from test_identity import stat

if importlib.util.find_spec('root_linkage'):
    import root_linkage as root
else:
    root = None


class RootLinkageTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(root, 'root linkage is not implemented')
        self.fixture = test_sample.SampleTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.sampler = self.fixture.sampler()
        self.acks = []
        for seq in range(1, 8):
            self.acks.append(self.sampler.capture_boundary(
                boundary.encode_request(self.fixture.nonce, seq)))
        self.marker = (b'R4ROOT/1\n' + self.fixture.nonce.encode() + b'\n2\n'
                       + stat('2', '1', '900'))
        self.context = root.RootContext(self.fixture.nonce, self.fixture.identity, 1, 2, 10000)

    def capture(self, argv=None):
        data = b'\0'.join(root.VERIFY_ARGV) + b'\0' if argv is None else argv
        (self.fixture.proc/'41'/'cmdline').write_bytes(data)
        self.acks.append(self.sampler.capture_boundary(
            boundary.encode_request(self.fixture.nonce, 8)))
        controls = (boundary.encode_request(self.fixture.nonce, 7), self.acks[6],
                    boundary.encode_request(self.fixture.nonce, 8), self.acks[7])
        receipt = observe.encode_retrieval(
            nonce=self.fixture.nonce, binding=self.sampler.binding, start='11000', end='12000',
            process=['exited', 0, None], stdout=self.marker, stderr=b'',
            stdout_complete=True, stderr_complete=True, reason='none')
        return [self.fixture.stream.getvalue(), self.marker, receipt, controls, self.context]

    def test_actual_sampler_root_requires_matching_observed_argv(self):
        result = root.join_invocation_root(*self.capture())
        self.assertEqual(result['status'], 'matched_observation')
        self.assertEqual(result['root'], ('41', '900'))
        self.assertEqual(result['samples'], (8,))
        self.assertEqual(result['descendants'], 'owned_unattributed')
        self.assertFalse(result['live_authority'])

    def test_marker_shape_nonce_and_pid_are_strict(self):
        self.assertEqual(root.parse_root_marker(self.marker, self.fixture.nonce), ('2', '900'))
        bad = [self.marker[:-1], self.marker+b'\n', self.marker.replace(b'R4ROOT/1', b'R4ROOT/2'),
               self.marker.replace(b'\n2\n', b'\n02\n'), self.marker.replace(b'\n2\n', b'\n3\n'),
               self.marker.replace(b'\n2\n', b'\n0\n'), self.marker+b'\0', b'x'*4097,
               self.marker.replace(b'worker)', b'worker\r)'), self.marker.replace(b'worker)', b'w\n)')]
        for raw in bad:
            with self.subTest(raw=raw[:100]), self.assertRaises(observe.ObservationError):
                root.parse_root_marker(raw, self.fixture.nonce)
        with self.assertRaises(observe.ObservationError):
            root.parse_root_marker(self.marker, 'd'*64)

    def test_marker_and_wrong_or_missing_argv_never_prove_exec(self):
        args = self.capture(b'lexlean\0verify\0other.lex.tex\0')
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'exec_not_observed')

    def test_wrong_start_is_not_pid_identity(self):
        args = self.capture()
        args[1] = self.marker.replace(b'900\n', b'901\n')
        row = json.loads(args[2]); row[9] = base64.b64encode(args[1]).decode()
        args[2] = observe._encoded(row, 16384)
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'root_not_observed')

    def test_namespace_missing_or_unexercised_shape_stays_unknown(self):
        (self.fixture.proc/'41'/'status').write_bytes(b'Pid:\t41\nPPid:\t7\n')
        args = self.capture()
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'namespace_unknown')

    def test_two_matching_incarnations_are_ambiguous(self):
        self.fixture.add_pid('42')
        (self.fixture.cg/'cgroup.procs').write_bytes(b'41\n42\n')
        args = self.capture()
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'ambiguous_root')

    def test_receipt_rejects_wrong_capture_status_binding_clock_and_completeness(self):
        args = self.capture()
        cases = [(2, 'd'*64), (3, 'e'*64), (4, '9999'), (5, None),
                 (6, ['exited', 1, None]), (7, False), (9, ''), (11, 'unknown')]
        for index, value in cases:
            row = json.loads(args[2]); row[index] = value
            with self.subTest(index=index), self.assertRaises(observe.ObservationError):
                root.join_invocation_root(args[0], args[1], observe._encoded(row, 16384),
                                          args[3], args[4])

    def test_epoch_or_control_replay_and_missing_prefix_reject(self):
        args = self.capture()
        changed = list(self.fixture.identity); changed[2] = '2026-10-27T00:00:00Z'
        context = dataclasses.replace(self.context, identity=tuple(changed))
        with self.assertRaises(observe.ObservationError):
            root.join_invocation_root(*args[:4], context)
        with self.assertRaises(observe.ObservationError):
            root.join_invocation_root(*args[:3], args[3][:2]*2, args[4])
        with self.assertRaises(observe.ObservationError):
            root.join_invocation_root(args[0][:-1], *args[1:])
        with self.assertRaises(observe.ObservationError):
            root.join_invocation_root(args[0].rsplit(b'\n', 2)[0]+b'\n', *args[1:])

    def test_raw_identity_and_membership_disagreement_reject(self):
        args = self.capture()
        original = base64.b64encode(stat('41', '7', '900'))
        for changed in (stat('42', '7', '900'), stat('41', '7', '901')):
            journal = args[0].replace(original, base64.b64encode(changed))
            with self.assertRaises(observe.ObservationError):
                root.join_invocation_root(journal, *args[1:])
        journal = args[0].replace(base64.b64encode(b'0::/owned\n'),
                                  base64.b64encode(b'0::/other\n'))
        with self.assertRaises(observe.ObservationError):
            root.join_invocation_root(journal, *args[1:])

    def test_unreferenced_identity_cannot_supply_exec_evidence(self):
        args = self.capture(b'shell\0')
        rows = [json.loads(x) for x in args[0].splitlines()]
        item = list(next(x for x in reversed(rows) if x[0] == 'i'))
        item[1] += 1
        item[8] = base64.b64encode(b'\0'.join(root.VERIFY_ARGV)+b'\0').decode()
        args[0] += observe._encoded(item, 8192)
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'exec_not_observed')

    def test_bounds_and_duplicate_identity_reject(self):
        args = self.capture()
        for journal in (b'x'*1048577, args[0]+next(
                x+b'\n' for x in args[0].splitlines() if x.startswith(b'["i",'))):
            with self.assertRaises(observe.ObservationError):
                root.join_invocation_root(journal, *args[1:])

    def test_guest_time_is_never_compared_with_host_receipt_clock(self):
        self.context = dataclasses.replace(self.context, return_ack_received_ns=0)
        args = self.capture()
        row = json.loads(args[2]); row[4:6] = ['0', '1']
        args[2] = observe._encoded(row, 16384)
        self.assertEqual(root.join_invocation_root(*args)['status'], 'matched_observation')

    def test_header_numeric_types_cannot_alias_boolean(self):
        identity = list(self.context.identity)
        cg = list(identity[4]); cg[1] = 0; identity[4] = tuple(cg)
        context = dataclasses.replace(self.context, identity=tuple(identity))
        context.validate()
        row = ['h', 3, context.nonce, identity[3], list(identity[:3]), cg,
               list(identity[5]), '0', 'relative_us', {'quotaProfile': 'owner-retention/1'}]
        root._header(row, context)
        cg[1] = False
        with self.assertRaises(observe.ObservationError):
            root._header(row, context)

    def test_nonfinite_json_and_wrong_namespace_coordinates_are_rejected(self):
        for raw in (b'["g",NaN]\n', b'["g",Infinity]\n', b'["g",-Infinity]\n'):
            with self.subTest(raw=raw), self.assertRaises(observe.ObservationError):
                root._json(raw, 8192)
        args = self.capture()
        for key, value in (('namespace_coordinate', True), ('namespace_depth', 65),
                           ('return_ack_received_ns', True)):
            with self.subTest(key=key), self.assertRaises(observe.ObservationError):
                root.join_invocation_root(*args[:4], dataclasses.replace(args[4], **{key: value}))

    def test_preverify_argv_cannot_bind_postverify_shell(self):
        # A matching identity dictionary entry before the selected window is insufficient.
        (self.fixture.proc/'41'/'cmdline').write_bytes(b'\0'.join(root.VERIFY_ARGV)+b'\0')
        self.sampler = self.fixture.sampler()
        self.acks = [self.sampler.capture_boundary(boundary.encode_request(self.fixture.nonce, seq))
                     for seq in range(1, 8)]
        args = self.capture(b'shell\0')
        self.assertEqual(root.join_invocation_root(*args)['reason'], 'exec_not_observed')

    def test_boolean_boundary_status_is_not_exit_zero(self):
        args = self.capture()
        scan = root._Scan(self.context, ('2', '900'),
                          root._controls(args[3], self.context, self.sampler.binding))
        for offset, end, row in root._rows(args[0]):
            if row[0] == 's' and row[1] == 7:
                scan.sample, scan.interval = 7, tuple(map(int, row[2:4]))
            if row[0] == 'b' and row[1] == 7:
                row[7][0] = False
                with self.assertRaises(observe.ObservationError):
                    scan.boundary_row(row, offset, end)
                break

    def test_missing_namespace_never_erases_a_known_conflict(self):
        args = self.capture()
        entry = next(row for _, _, row in root._rows(args[0]) if row[0] == 'i')
        scan = root._Scan(self.context, ('2', '900'), {})
        item = list(entry); item[9] = None
        scan.identity(item)
        item = list(entry); item[1] = 2
        scan.identity(item)
        item = list(entry); item[1] = 3; item[9] = None
        scan.identity(item)
        item = list(entry); item[1] = 4; item[9] = ['41', '3']
        with self.assertRaisesRegex(observe.ObservationError, 'namespace_conflict'):
            scan.identity(item)
