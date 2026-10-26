"""Scripted guest clocks and actual owned sampler, without a live observer."""

import importlib.util
import json
import unittest
from unittest import mock

import boundary
import observe
import test_sample

if importlib.util.find_spec('cadence'):
    import cadence
else:
    cadence = None


class CadenceTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(cadence, 'periodic composition is not implemented')
        self.fixture = test_sample.SampleTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.now = 1000000000
        self.fixture.clock = lambda: self.now
        self.sampler = self.fixture.sampler()
        self.driver = cadence.Cadence(self.sampler)

    def rows(self, tag):
        return [r for r in map(json.loads, self.fixture.stream.getvalue().splitlines()) if r[0] == tag]

    def test_periodic_100ms_and_slow_5s_do_not_catch_up(self):
        self.driver.step()
        self.assertEqual(len(self.rows('s')), 1)
        self.assertEqual(len(self.rows('m')), 1)
        self.assertIsNone(self.driver.step())
        self.now += 100000000
        self.driver.step()
        self.assertEqual(len(self.rows('m')), 1)
        self.now = 6000000000
        self.driver.step()
        self.assertEqual(len(self.rows('s')), 3)
        self.assertEqual(len(self.rows('m')), 2)
        self.assertEqual(self.rows('s')[-1][10], 48)
        self.assertEqual(self.driver.due_ns, 6100000000)

    def test_skipped_slow_files_are_not_read_and_history_is_preserved(self):
        self.driver.step()
        previous = dict(self.sampler.previous_local)
        for name in ('memory.stat', 'memory.pressure', 'memory.events.local'):
            self.fixture.cg.joinpath(name).write_bytes(b'invalid')
        self.now += 100000000
        self.driver.step()
        self.assertEqual(self.sampler.previous_local, previous)
        self.now = 6000000000
        with self.assertRaises(observe.ObservationError): self.driver.step()
        self.assertTrue(self.driver.finish_snapshot()['incomplete'])

    def test_forced_sample_has_priority_and_counts_equal_deadline_once(self):
        ack = self.driver.step(boundary.encode_request(self.fixture.nonce, 1))
        self.assertEqual(boundary.decode_ack(ack)[1], 1)
        self.assertEqual(self.rows('s')[-1][10], 1)
        self.assertEqual(self.driver.due_ns, 1100000000)
        self.assertIsNone(self.driver.step())
        self.driver.step(boundary.encode_request(self.fixture.nonce, 2))
        self.assertEqual(self.rows('s')[-1][10], 0)
        self.assertEqual(len(self.rows('m')), 2)
        self.assertEqual(self.driver.slow_due_ns, 6000000000)

    def test_post_write_misses_are_carried_and_visible_on_stop(self):
        write = self.fixture.stream.write
        def delayed(raw):
            result = write(raw)
            if bytes(raw).startswith(b'["s",'): self.now += 350000000
            return result
        with mock.patch.object(self.fixture.stream, 'write', delayed): self.driver.step()
        self.assertEqual(self.rows('s')[-1][10], 0)
        self.assertEqual(self.driver.pending_misses, 3)
        self.assertEqual(self.driver.due_ns, 1400000000)
        stopped = self.driver.finish()
        self.assertEqual(stopped['pending_misses'], 3)
        self.assertTrue(stopped['incomplete'])

    def test_pending_post_write_misses_enter_next_row_exactly_once(self):
        write = self.fixture.stream.write
        def delayed(raw):
            result = write(raw)
            if bytes(raw).startswith(b'["s",'): self.now += 100000000
            return result
        with mock.patch.object(self.fixture.stream, 'write', delayed): self.driver.step()
        self.now = self.driver.due_ns
        self.driver.step()
        self.assertEqual([r[10] for r in self.rows('s')], [0, 1])
        self.assertEqual(self.driver.pending_misses, 0)

    def test_sync_delay_crosses_slots_and_keeps_sampler_ownership(self):
        self.driver.step()
        # Replacing the callback itself would violate ownership; wrap its underlying list instead.
        class SlowList(list):
            def append(inner, value): self.now += 250000000; super().append(value)
        self.fixture.syncs = SlowList()
        self.now = 2000000000
        self.driver.step()
        self.assertEqual(self.driver.pending_misses, 2)
        self.assertEqual(self.sampler.journal.synced_offset, self.sampler.journal.complete_offset)
        self.now = self.driver.due_ns
        self.driver.step()

    def test_backward_clock_and_failed_write_stop_without_schedule_commit(self):
        self.driver.step()
        due = self.driver.due_ns
        self.now -= 1
        with self.assertRaises(observe.ObservationError): self.driver.step()
        self.assertEqual(self.driver.due_ns, due)
        self.assertTrue(self.driver.finish_snapshot()['incomplete'])

    def test_periodic_local_nonzero_still_blocks_fresh_preverify(self):
        self.fixture.cg.joinpath('memory.events.local').write_bytes(self.fixture.events('1'))
        self.driver.step()
        self.fixture.cg.joinpath('memory.events.local').write_bytes(self.fixture.events('0'))
        with self.assertRaises(observe.ObservationError):
            self.driver.step(boundary.encode_request(self.fixture.nonce, 1))
        self.assertTrue(self.sampler.ever_nonzero)

    def test_failed_sync_keeps_pending_coverage_and_stops_future_calls(self):
        self.driver.step()
        self.now = 2000000000
        previous = (self.driver.due_ns, self.driver.pending_misses, self.driver.accounted_ns)
        with mock.patch.object(self.sampler.journal, 'sync', side_effect=OSError('synthetic sync')):
            with self.assertRaises(OSError): self.driver.step()
        with self.assertRaises(observe.ObservationError): self.driver.step()
        self.assertTrue(self.driver.finish_snapshot()['incomplete'])
        self.assertEqual((self.driver.due_ns, self.driver.pending_misses,
                          self.driver.accounted_ns), previous)

    def test_swallowed_periodic_reentry_poisoned_outer_capture(self):
        def identity():
            try: self.sampler.capture_periodic(self.driver.due_ns, 0, self.driver.slow_due_ns)
            except observe.ObservationError: pass
            return self.fixture.identity
        self.sampler.read_identity = identity
        with self.assertRaises(observe.ObservationError): self.driver.step()
        self.assertTrue(self.sampler.failed)
        self.assertTrue(self.sampler.journal.stopped)

    def test_owned_sync_reentry_poisoned_sampler(self):
        self.sampler.busy = True
        with self.assertRaises(observe.ObservationError):
            self.sampler.synchronize_owned()
        self.assertTrue(self.sampler.failed)
        self.assertTrue(self.sampler.journal.stopped)

    def driver_clock_reentry(self, action):
        entered = False
        def clock():
            nonlocal entered
            if not entered:
                entered = True
                try: action()
                except observe.ObservationError: pass
            return self.now
        self.sampler.clock = clock
        before = self.driver.due_ns
        with self.assertRaises(observe.ObservationError):
            self.driver.step(boundary.encode_request(self.fixture.nonce, 1))
        self.assertTrue(self.driver.failed)
        self.assertTrue(self.sampler.failed)
        self.assertTrue(self.sampler.journal.stopped)
        self.assertEqual(self.driver.due_ns, before)
        self.assertEqual(len(self.rows('s')), 0)

    def test_swallowed_driver_finish_from_initial_clock_blocks_ack(self):
        self.driver_clock_reentry(self.driver.finish)

    def test_swallowed_driver_step_from_initial_clock_blocks_ack(self):
        self.driver_clock_reentry(self.driver.step)

    def test_initial_now_owns_sampler_against_nested_capture(self):
        self.driver_clock_reentry(lambda: self.sampler.capture_boundary(
            boundary.encode_request(self.fixture.nonce, 1)))

    def test_sync_callback_driver_finish_blocks_completion_and_rolls_back(self):
        self.driver.step()
        before = (self.driver.due_ns, self.driver.pending_misses)
        driver = self.driver
        class ReentryList(list):
            def append(inner, value):
                try: driver.finish()
                except observe.ObservationError: pass
                super().append(value)
        self.fixture.syncs = ReentryList()
        self.now = 2000000000
        with self.assertRaises(observe.ObservationError): self.driver.step()
        self.assertTrue(self.driver.failed)
        self.assertTrue(self.sampler.journal.stopped)
        self.assertEqual((self.driver.due_ns, self.driver.pending_misses), before)

    def test_force_sync_reentry_cannot_mint_retained_ack(self):
        driver = self.driver
        class ReentryList(list):
            def append(inner, value):
                try: driver.finish()
                except observe.ObservationError: pass
                super().append(value)
        self.fixture.syncs = ReentryList()
        with self.assertRaises(observe.ObservationError):
            self.driver.step(boundary.encode_request(self.fixture.nonce, 1))
        self.assertTrue(self.driver.failed)
        self.assertEqual(len(self.rows('b')), 1)
        self.assertIsNone(self.rows('b')[0][7][2])

    def test_force_only_compatibility_and_snapshot_do_not_add_io(self):
        ack = self.sampler.capture_boundary(boundary.encode_request(self.fixture.nonce, 1))
        self.assertEqual(boundary.decode_ack(ack)[1], 1)
        self.assertEqual(self.rows('s')[0][10], 0)
        before = self.fixture.stream.getvalue()
        self.driver.finish_snapshot()
        self.assertEqual(self.fixture.stream.getvalue(), before)
