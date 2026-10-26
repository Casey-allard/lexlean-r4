"""Synthetic owned-census and guest-clock tests; no live process sampling."""

import unittest

import observe


def stat(pid='41', ppid='7', start='900'):
    return (f'{pid} (compiler ) worker) S {ppid} ' + '0 ' * 17
            + f'{start}\n').encode()


def status(pid='41', ppid='7'):
    return f'Pid:\t{pid}\nPPid:\t{ppid}\nNSpid:\t{pid} 2\n'.encode()


class IdentityTests(unittest.TestCase):
    def test_pid_reuse_reparenting_and_membership_drift_reject(self):
        base = [stat(), b'0::/owned\n', status(), b'lean\0',
                stat(), b'0::/owned\n']
        changes = ((0, stat(pid='42')), (1, b'0::/other\n'),
                   (2, status(pid='42')), (2, status(ppid='8')),
                   (4, stat(pid='42')), (4, stat(start='901')),
                   (4, stat(ppid='8')), (5, b'0::/other\n'))
        for position, replacement in changes:
            values = list(base)
            values[position] = replacement
            calls = []

            def read(*args):
                calls.append(args)
                return values.pop(0)

            with self.subTest(position=position, replacement=replacement):
                with self.assertRaises(observe.ObservationError):
                    observe.process_snapshot(b'41\n', '41', b'/owned', read)
                self.assertEqual(len(calls), 2 if position < 2 else 6)

    def test_each_reader_failure_preserves_exception_and_stops_reads(self):
        class FalseyFailure(OSError):
            def __bool__(self):
                return False

        for position in range(6):
            for error in (FileNotFoundError(), PermissionError(),
                          FalseyFailure(), KeyboardInterrupt()):
                values = [stat(), b'0::/owned\n', status(), b'',
                          stat(), b'0::/owned\n']
                calls = []

                def read(*args):
                    calls.append(args)
                    if len(calls) - 1 == position:
                        raise error
                    return values[len(calls) - 1]

                with self.subTest(position=position, error=type(error).__name__):
                    with self.assertRaises(type(error)) as caught:
                        observe.process_snapshot(b'41\n', '41', b'/owned', read)
                    self.assertIs(caught.exception, error)
                    self.assertEqual(len(calls), position + 1)

    def test_raw_protocol_and_parse_failure_at_every_read_reject(self):
        base = [stat(), b'0::/owned\n', status(), b'lean\0',
                stat(), b'0::/owned\n']
        for position in range(6):
            for replacement in (None, bytearray(base[position]), b'x' * 8193):
                values = list(base)
                values[position] = replacement
                items = iter(values)
                with self.subTest(position=position), self.assertRaisesRegex(
                        observe.ObservationError, 'proc_read_protocol'):
                    observe.process_snapshot(b'41\n', '41', b'/owned',
                                             lambda *_: next(items))
            if position != 3:
                values = list(base)
                values[position] = b''
                items = iter(values)
                with self.subTest(position=position), self.assertRaises(
                        observe.ObservationError):
                    observe.process_snapshot(b'41\n', '41', b'/owned',
                                             lambda *_: next(items))

    def test_empty_or_maximum_cmdline_retains_bytes_without_classification(self):
        for cmdline in (b'', b'\xff' * 4096):
            values = iter([stat(), b'0::/owned\n',
                           status() + b'VmRSS: 0 kB\nVmHWM: 12 kB\n',
                           cmdline, stat(), b'0::/owned\n'])
            row = observe.process_snapshot(b'41\n', '41', b'/owned',
                                           lambda *_: next(values))
            self.assertIs(row['cmdline'], cmdline)
            self.assertEqual((row['rss_kib'], row['hwm_kib']), ('0', '12'))
            self.assertEqual(row['attribution'], 'owned_unattributed')
            self.assertIsNone(row['parent_start'])

    def test_invalid_request_rejects_before_any_proc_read(self):
        calls = []

        def read(*args):
            calls.append(args)
            return stat()

        cases = [(b'41\n', '41', x) for x in
                 (bytearray(b'/owned'), b'', b'owned', b'/x\n', b'/x\0',
                  b'/x\r', b'/' + b'x' * 4092)]
        cases += [(bytearray(b'41\n'), '41', b'/owned'),
                  (b'41\n', '42', b'/owned'), (b'41\n', 41, b'/owned')]
        for census, pid, target in cases:
            with self.subTest(pid=pid, target=repr(target)[:30]):
                with self.assertRaises(observe.ObservationError):
                    observe.process_snapshot(census, pid, target, read)
        self.assertEqual(calls, [])

    def test_snapshot_rejects_unbounded_or_mutable_reader_bytes(self):
        for bad in (b'x' * 4097, bytearray(b'lean\0'), 'lean', None):
            with self.subTest(bad=type(bad).__name__):
                values = iter([stat(), b'0::/owned\n', status(), bad,
                               stat(), b'0::/owned\n'])
                with self.assertRaisesRegex(observe.ObservationError,
                                            'proc_read_protocol'):
                    observe.process_snapshot(b'41\n', '41', b'/owned',
                                             lambda *_: next(values))

    def test_snapshot_reads_only_census_pid_in_required_order(self):
        calls = []
        values = [stat(), b'0::/owned\n', status(), b'lean\0module.lean\0',
                  stat(), b'0::/owned\n']

        def read(pid, name, cap):
            calls.append((pid, name, cap))
            return values.pop(0)

        row = observe.process_snapshot(b'7\n41\n', '41', b'/owned', read)
        self.assertEqual(calls, [('41', 'stat', 4096), ('41', 'cgroup', 4096),
                                ('41', 'status', 8192), ('41', 'cmdline', 4096),
                                ('41', 'stat', 4096), ('41', 'cgroup', 4096)])
        self.assertEqual(row, {'pid': '41', 'ppid': '7', 'start': '900',
                              'namespace': ('41', '2'), 'rss_kib': None,
                              'hwm_kib': None, 'cmdline': b'lean\0module.lean\0',
                              'parent_start': None,
                              'attribution': 'owned_unattributed'})


class ClockTests(unittest.TestCase):
    def test_schedule_rejects_bad_origin_grid_order_and_overflow(self):
        for values in ((1, 0, 2), (0, 100000000, 0), (0, 1, 2),
                       (-1, -1, 0), (0, 0, 2**64), (True, 1, 1),
                       (0, 0.0, 1), (0, 0, '1'),
                       (2**64 - 1, 2**64 - 1, 2**64 - 1)):
            with self.subTest(values=values), self.assertRaisesRegex(
                    observe.ObservationError, 'clock_schedule'):
                observe.next_tick(*values)

    def test_absolute_schedule_skips_elapsed_slots_without_catchup(self):
        origin, period = 9007199254740993, 100000000
        for elapsed, missed in ((0, 0), (period - 1, 0), (period, 1),
                                (3 * period + 1, 3)):
            due = origin + 2 * period
            self.assertEqual(observe.next_tick(origin, due, due + elapsed),
                             (due + (missed + 1) * period, missed))

    def test_guest_interval_rejects_invalid_clock_values(self):
        for values in ((1, 0, 2), (0, 2, 1), (-1, 0, 1), (0, 0, 2**64),
                       (False, 0, 0), (0, 0.0, 1), (0, 0, '1')):
            with self.subTest(values=values), self.assertRaisesRegex(
                    observe.ObservationError, 'clock_interval'):
                observe.guest_interval(*values)

    def test_guest_interval_widens_without_float_rounding(self):
        origin = 9007199254740993
        for start, end, expected in ((0, 0, ('0', '0')),
                                     (1, 999, ('0', '1')),
                                     (1000, 1000, ('1', '1')),
                                     (1001, 2001, ('1', '3'))):
            with self.subTest(start=start, end=end):
                self.assertEqual(observe.guest_interval(origin, origin + start,
                                                       origin + end), expected)


if __name__ == '__main__':
    unittest.main()
