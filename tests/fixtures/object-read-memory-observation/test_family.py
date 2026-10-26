"""Synthetic nested parent sampling; no live process or root attribution."""

import unittest

import observe
from test_identity import stat, status


class FamilyTests(unittest.TestCase):
    @staticmethod
    def values(pid, name):
        parent = '7' if pid == '41' else '1'
        return {'stat': stat(pid, parent, '900' if pid == '41' else '100'),
                'status': status(pid, parent), 'cgroup': b'0::/owned\n',
                'cmdline': b'owned\0'}[name]

    def test_parent_is_sampled_inside_child_identity_interval(self):
        calls = []

        def read(pid, name, cap):
            calls.append((pid, name))
            parent = '7' if pid == '41' else '1'
            return {'stat': stat(pid, parent, '900' if pid == '41' else '100'),
                    'status': status(pid, parent), 'cgroup': b'0::/owned\n',
                    'cmdline': b'owned\0'}[name]

        child, parent, failure = observe.process_family_snapshot(
            b'41\n7\n', '41', b'/owned', read)
        self.assertEqual(child['parent_start'], '100')
        self.assertEqual(parent['pid'], '7')
        self.assertIsNone(failure)
        self.assertEqual(child['attribution'], 'owned_unattributed')
        self.assertEqual(calls, [('41', n) for n in
                                ('stat', 'cgroup', 'status', 'cmdline')]
                         + [('7', n) for n in
                            ('stat', 'cgroup', 'status', 'cmdline', 'stat', 'cgroup')]
                         + [('41', n) for n in ('stat', 'cgroup')])

    def test_parent_failure_is_retained_and_child_is_rechecked(self):
        class FalseyFailure(OSError):
            def __bool__(self):
                return False

        for error in (FileNotFoundError('gone'), PermissionError('denied'),
                      FalseyFailure('io'), observe.ObservationError('raced')):
            calls = []

            def read(pid, name, cap):
                calls.append((pid, name))
                if pid == '7':
                    raise error
                return self.values(pid, name)

            with self.subTest(error=type(error).__name__):
                child, parent, failure = observe.process_family_snapshot(
                    b'41\n7\n', '41', b'/owned', read)
                self.assertIs(failure, error)
                self.assertIsNone(parent)
                self.assertIsNone(child['parent_start'])
                self.assertEqual(calls[-2:], [('41', 'stat'), ('41', 'cgroup')])

    def test_parent_and_subsequent_child_failure_both_survive(self):
        parent_failure, child_failure = OSError('parent'), OSError('child')
        parent_attempted = False

        def read(pid, name, cap):
            nonlocal parent_attempted
            if pid == '7':
                parent_attempted = True
                raise parent_failure
            if parent_attempted:
                raise child_failure
            return self.values(pid, name)

        with self.assertRaises(BaseExceptionGroup) as caught:
            observe.process_family_snapshot(b'41\n7\n', '41', b'/owned', read)
        self.assertEqual(caught.exception.exceptions, (parent_failure, child_failure))

    def test_invalid_child_cmdline_blocks_parent_read(self):
        for bad in (None, bytearray(b'x'), b'x' * 4097):
            calls = []

            def read(pid, name, cap):
                calls.append((pid, name))
                if pid == '41' and name == 'cmdline':
                    return bad
                return self.values(pid, name)

            with self.subTest(kind=type(bad).__name__):
                with self.assertRaisesRegex(observe.ObservationError,
                                            'proc_read_protocol'):
                    observe.process_family_snapshot(b'41\n7\n', '41', b'/owned', read)
                self.assertFalse(any(pid == '7' for pid, _ in calls))

    def test_no_outside_parent_or_recursive_grandparent_reads(self):
        for census, expected_parent, expected_pids in (
                (b'41\n', None, {'41'}), (b'41\n7\n1\n', '7', {'41', '7'})):
            calls = []

            def read(pid, name, cap):
                calls.append(pid)
                return self.values(pid, name)

            child, parent, failure = observe.process_family_snapshot(
                census, '41', b'/owned', read)
            self.assertEqual(set(calls), expected_pids)
            self.assertEqual(None if parent is None else parent['pid'], expected_parent)
            self.assertIsNone(failure)
            self.assertEqual(child['attribution'], 'owned_unattributed')

    def test_child_reparenting_after_parent_read_invalidates_edge(self):
        parent_seen = False

        def read(pid, name, cap):
            nonlocal parent_seen
            if pid == '7':
                parent_seen = True
            if parent_seen and pid == '41' and name == 'stat':
                return stat('41', '8', '900')
            return self.values(pid, name)

        with self.assertRaisesRegex(observe.ObservationError, 'raced_identity'):
            observe.process_family_snapshot(b'41\n7\n', '41', b'/owned', read)

    def test_parent_interrupt_and_cleanup_group_are_fatal_not_missing(self):
        for failure in (KeyboardInterrupt('stop'),
                        ExceptionGroup('close', [OSError('first'), OSError('second')])):
            def read(pid, name, cap):
                if pid == '7':
                    raise failure
                return self.values(pid, name)

            with self.subTest(failure=type(failure).__name__):
                with self.assertRaises(type(failure)) as caught:
                    observe.process_family_snapshot(b'41\n7\n', '41', b'/owned', read)
                self.assertIs(caught.exception, failure)


if __name__ == '__main__':
    unittest.main()
