"""Host-only tests for the one-off admitted-range diagnostic reader."""

import io
import unittest

import observe

ZERO_EVENTS = (b"low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\n"
               b"oom_group_kill 0\n")


class CounterTests(unittest.TestCase):
    def test_counter_keeps_decimal_identity(self):
        self.assertEqual(observe.u64(b"6325"), "6325")

    def test_counter_rejects_noncanonical_and_out_of_range_values(self):
        invalid = (b"", b"00", b"01", b"-1", b"+1", b" 1", b"1\n",
                   b"1.0", b"\xff", b"18446744073709551616", b"9" * 21)
        for raw in invalid:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                observe.u64(raw)
        self.assertEqual(observe.u64(b"0"), "0")
        self.assertEqual(observe.u64(b"18446744073709551615"),
                         "18446744073709551615")

    def test_events_preserve_all_valid_unknown_keys(self):
        actual = observe.parse_events(ZERO_EVENTS + b"future_key 3\n")
        self.assertEqual(actual["max"], "0")
        self.assertEqual(actual["future_key"], "3")
        self.assertEqual(len(actual), 7)

    def test_events_reject_incomplete_duplicate_or_unbounded_records(self):
        invalid = (
            b"", ZERO_EVENTS[:-1], ZERO_EVENTS.replace(b"low 0\n", b""),
            ZERO_EVENTS.replace(b"max 0", b"max 6325") + b"max 0\n",
            ZERO_EVENTS + b"\n", ZERO_EVENTS + b"\xff 0\n",
            ZERO_EVENTS + b"\x01key 0\n", ZERO_EVENTS + b"k" * 65 + b" 0\n",
            ZERO_EVENTS * 100, ZERO_EVENTS + b"bad 00\n",
            ZERO_EVENTS + b"bad 0 extra\n",
            ZERO_EVENTS + b"".join(b"k%d 0\n" % n for n in range(59)),
        )
        for raw in invalid:
            with self.subTest(raw=raw[:80]), self.assertRaises(
                    observe.ObservationError):
                observe.parse_events(raw)
        exact_keys = ZERO_EVENTS + b"".join(
            b"k%d 0\n" % n for n in range(58))
        self.assertEqual(len(observe.parse_events(exact_keys)), 64)


class ReadTests(unittest.TestCase):
    def test_finite_read_keeps_exact_bytes_and_does_not_close_owner_stream(self):
        stream = io.BytesIO(b"max 6325\n")
        self.assertEqual(observe.read_bounded(stream, 64), b"max 6325\n")
        self.assertFalse(stream.closed)

    def test_short_reads_continue_to_eof_and_overflow_is_not_truncated(self):
        class ShortStream(io.BytesIO):
            def read(self, size):
                self.requests.append(size)
                return super().read(min(size, 2))

        for data in (b"", b"abc", b"abcd", b"abcde", b"abcdef"):
            stream = ShortStream(data)
            stream.requests = []
            if len(data) > 4:
                with self.assertRaisesRegex(observe.ObservationError,
                                            "truncated_read"):
                    observe.read_bounded(stream, 4)
            else:
                self.assertEqual(observe.read_bounded(stream, 4), data)
            self.assertLessEqual(stream.tell(), 5)
            self.assertTrue(all(0 < n <= 5 for n in stream.requests))

    def test_bad_cap_rejects_before_read(self):
        class NoRead:
            def read(self, size):
                raise AssertionError("read must not happen")

        for cap in (0, -1, True, 1.5, None, 16385):
            with self.subTest(cap=cap), self.assertRaisesRegex(
                    observe.ObservationError, "read_cap"):
                observe.read_bounded(NoRead(), cap)
        self.assertEqual(observe.read_bounded(io.BytesIO(b"x" * 16384),
                                             16384), b"x" * 16384)

    def test_failed_or_invalid_reads_never_become_empty_success(self):
        denied = PermissionError("synthetic denial")

        class FaultStream:
            def __init__(self, result):
                self.result = result

            def read(self, size):
                if isinstance(self.result, Exception):
                    raise self.result
                return self.result

        with self.assertRaises(PermissionError) as caught:
            observe.read_bounded(FaultStream(denied), 4)
        self.assertIs(caught.exception, denied)
        for result in (None, "bytes", b"a" * 6):
            with self.subTest(result=result), self.assertRaisesRegex(
                    observe.ObservationError, "read_protocol"):
                observe.read_bounded(FaultStream(result), 4)


class ProcessTests(unittest.TestCase):
    @staticmethod
    def stat_record(comm=b"lean", pid=b"1594", ppid=b"1", start=b"11313"):
        # Synthetic Linux stat fields 3..22; later fields are irrelevant here.
        fields = [b"S", ppid] + [b"0"] * 17 + [start]
        return pid + b" (" + comm + b") " + b" ".join(fields) + b"\n"

    def test_census_preserves_kernel_order_without_scanning_other_processes(self):
        self.assertEqual(observe.parse_procs(b"1594\n1607\n"), ("1594", "1607"))
        self.assertEqual(observe.parse_procs(b""), ())

    def test_census_rejects_partial_duplicate_invalid_or_over_capacity(self):
        invalid = (b"1594", b"0\n", b"1594\n1594\n", b"1\r\n", b"\n",
                   b"1\n\n", b"01\n", b"-1\n", b"1 2\n", None,
                   b"1\n" * 2049,
                   b"".join(b"%d\n" % n for n in range(1, 66)))
        for raw in invalid:
            with self.subTest(raw=str(raw)[:80]), self.assertRaises(
                    observe.ObservationError):
                observe.parse_procs(raw)
        self.assertEqual(len(observe.parse_procs(
            b"".join(b"%d\n" % n for n in range(1, 65)))), 64)

    def test_stat_identity_uses_documented_positions_after_comm(self):
        for comm in (b"lean", b"x ) ( y", b"a\nb", b"\xff ) S "):
            with self.subTest(comm=comm):
                self.assertEqual(observe.parse_stat(self.stat_record(comm)),
                                 ("1594", "1", "11313"))

    def test_stat_rejects_incomplete_oversized_or_invalid_identity(self):
        valid = self.stat_record()
        invalid = (None, b"", valid[:-1], b"x" * 4097,
                   valid.replace(b" (lean)", b" lean)"),
                   valid.replace(b") S ", b") RUN "),
                   valid.replace(b") S ", b") \xff "),
                   self.stat_record(pid=b"0"), self.stat_record(pid=b"01"),
                   self.stat_record(ppid=b"-1"), self.stat_record(start=b"01"),
                   b"1594 (lean) S 1\n", valid + b"\n",
                   valid.replace(b" S 1 ", b" S 1\n"))
        for raw in invalid:
            with self.subTest(raw=str(raw)[:80]), self.assertRaises(
                    observe.ObservationError):
                observe.parse_stat(raw)
        maximum = b"18446744073709551615"
        self.assertEqual(observe.parse_stat(self.stat_record(
            ppid=b"0", start=maximum)), ("1594", "0", maximum.decode()))

    def test_status_preserves_identity_namespace_and_kib_units(self):
        raw = (b"Name:\tlean\nPid:\t1594\nPPid:\t1\nNSpid:\t1594\t2\n"
               b"VmRSS:\t 4096 kB\nVmHWM:\t 8192 kB\nFuture:\tvalue\n")
        self.assertEqual(observe.parse_status(raw),
                         ("1594", "1", ("1594", "2"), "4096", "8192"))
        self.assertEqual(observe.parse_status(b"Pid:\t1594\nPPid:\t0\n"),
                         ("1594", "0", None, None, None))

    def test_membership_preserves_exact_unified_path(self):
        self.assertEqual(observe.parse_membership(b"0::/docker/owned\n"),
                         b"/docker/owned")
        self.assertEqual(observe.parse_membership(b"0::/\n"), b"/")

    def test_membership_rejects_nonunified_partial_or_ambiguous_path(self):
        for raw in (None, b"", b"0::/owned", b"1:memory:/owned\n",
                    b"0::relative\n", b"0::/owned\n0::/other\n",
                    b"0::/bad\x00path\n", b"0::/bad\rpath\n",
                    b"0::/" + b"a" * 4096 + b"\n"):
            with self.subTest(raw=str(raw)[:80]), self.assertRaises(
                    observe.ObservationError):
                observe.parse_membership(raw)

    def test_status_rejects_bad_required_fields_units_duplicates_and_vectors(self):
        base = b"Pid:\t1594\nPPid:\t1\n"
        invalid = (None, b"", base[:-1], b"a" * 8193,
                   base.replace(b"Pid:\t1594\n", b""), base + b"Pid: 0\n",
                   base.replace(b"1594", b"0"), base + b"broken\n",
                   base + b"VmRSS: 1 MB\n", base + b"VmRSS:\n",
                   base + b"VmHWM: -1 kB\n", base + b"VmRSS: 1 kB extra\n",
                   base + b"VmRSS: 1 kB\nVmRSS: 0 kB\n",
                   base + b"NSpid: 2 1594\n", base + b"NSpid: 1594 0\n",
                   base + b"NSpid:\n", base + b"NSpid: " + b"1594 " * 65 + b"\n")
        for raw in invalid:
            with self.subTest(raw=str(raw)[:80]), self.assertRaises(
                    observe.ObservationError):
                observe.parse_status(raw)
        # Namespace coordinates may repeat; that does not mean PID reuse.
        self.assertEqual(observe.parse_status(base + b"NSpid: 1594 1594\n")[2],
                         ("1594", "1594"))


class JournalTests(unittest.TestCase):
    def test_all_row_kinds_charge_their_encoded_bytes_to_the_same_shared_pools(self):
        journal = observe.Journal(io.BytesIO(), lambda: None)
        totals = {"samples": 0, "identities": 0, "markers": 2048, "terminal": 8192}
        for tag, pool, size in (("s", "samples", 16384), ("m", "samples", 32768),
                                ("i", "identities", 8192), ("e", "identities", 8192),
                                ("h", "markers", 8192), ("b", "markers", 8192),
                                ("g", "markers", 8192)):
            journal.append(self.sized_row(tag, size))
            totals[pool] += size
            self.assertEqual(journal.reserved, totals)
            invalid = observe.Journal(io.BytesIO(), lambda: None)
            with self.assertRaises(observe.ObservationError):
                invalid.append(self.sized_row(tag, size + 1))
            self.assertEqual(invalid.complete_offset, 0)
        snapshot = journal.reserved
        snapshot["samples"] = 0
        self.assertEqual(journal.reserved, totals)

    def test_encoded_escape_and_multibyte_lengths_are_charged_not_character_counts(self):
        stream = io.BytesIO()
        journal = observe.Journal(stream, lambda: None)
        rows = (b'["i","\\u0000\\u0000"]\n', '["i","é🙂"]\n'.encode())
        for row in rows:
            journal.append(row)
        self.assertEqual(journal.reserved["identities"], sum(map(len, rows)))
        self.assertEqual(stream.getvalue(), b"".join(rows))

    def test_partial_write_charges_full_reservation_and_tracks_only_reported_bytes(self):
        failure = OSError("write failed")
        class PartialStream:
            calls = 0
            def write(self, value):
                self.calls += 1
                if self.calls == 1:
                    return 2
                raise failure
        journal = observe.Journal(PartialStream(), lambda: None)
        row = b'["s"]\n'
        with self.assertRaises(OSError) as caught:
            journal.append(row)
        self.assertIs(caught.exception, failure)
        self.assertEqual(journal.written_offset, 2)
        self.assertEqual(journal.complete_offset, 0)
        self.assertEqual(journal.complete_rows, 0)
        self.assertFalse(journal.offset_certain)
        self.assertEqual(journal.reserved["samples"], len(row))
        self.assertEqual(journal.reserved["terminal"], 8192)

    @staticmethod
    def sized_row(tag, size):
        prefix = b'["' + tag.encode() + b'","'
        suffix = b'"]\n'
        return prefix + b'x' * (size - len(prefix) - len(suffix)) + suffix

    def test_fixed_pools_allow_exact_fill_and_reject_one_byte_over_before_write(self):
        class CountingStream:
            size = 0
            def write(self, value):
                self.size += len(value)
                return len(value)
        for pool, tag, cap, rowcap in (("samples", "s", 786432, 16384),
                                      ("identities", "i", 196608, 8192),
                                      ("markers", "b", 55296, 8192)):
            for overflow in (False, True):
                with self.subTest(pool=pool, overflow=overflow):
                    stream = CountingStream()
                    journal = observe.Journal(stream, lambda: None)
                    remaining = cap - (9 if overflow else 0)
                    while remaining:
                        size = min(rowcap, remaining)
                        journal.append(self.sized_row(tag, size))
                        remaining -= size
                    before = stream.size
                    if overflow:
                        with self.assertRaises(observe.ObservationError):
                            journal.append(self.sized_row(tag, 10))
                        self.assertTrue(journal.stopped)
                        self.assertEqual(stream.size, before)
                    self.assertEqual(journal.reserved["terminal"], 8192)
                    self.assertEqual(journal.reserved[pool], before + (2048 if pool == "markers" else 0))
                    self.assertEqual(journal.complete_offset, before)
                    self.assertLessEqual(sum(journal.reserved.values()), 1048576)

    def test_row_envelope_rejects_before_io_and_preserves_terminal_capacity(self):
        bad_rows = (b"", None, "x", bytearray(b'["s"]\n'), b'["s"]',
                    b'["s"]\n["s"]\n', b'{}\n', b'[]\n', b'[1]\n',
                    b'["unknown"]\n', b'["s",NaN]\n', b'["s",\xff]\n',
                    b'["s",[}\n', b'["s"]\r\n', b'["s","' + b'x' * 16384 + b'"]\n')
        for row in bad_rows:
            with self.subTest(row=repr(row)[:80]):
                stream = io.BytesIO()
                journal = observe.Journal(stream, lambda: None)
                with self.assertRaises(observe.ObservationError):
                    journal.append(row)
                self.assertEqual(stream.getvalue(), b"")
                self.assertEqual(journal.complete_offset, 0)
                self.assertTrue(journal.stopped)
                self.assertTrue(journal.offset_certain)

    def test_sync_failure_keeps_prior_sync_bound_and_stops_without_retry(self):
        for result in (OSError("sync failed"), KeyboardInterrupt(), False, 0, "ok"):
            with self.subTest(result=result):
                calls = []
                def synchronize():
                    calls.append("sync")
                    if len(calls) == 1:
                        return None
                    if isinstance(result, BaseException):
                        raise result
                    return result
                stream = io.BytesIO()
                journal = observe.Journal(stream, synchronize)
                row = b'["s"]\n'
                journal.append(row)
                journal.sync()
                journal.append(row)
                expected = type(result) if isinstance(result, BaseException) else observe.ObservationError
                with self.assertRaises(expected) as raised:
                    journal.sync()
                if isinstance(result, BaseException):
                    self.assertIs(raised.exception, result)
                self.assertEqual(journal.synced_offset, len(row))
                self.assertEqual(journal.complete_offset, 2 * len(row))
                self.assertTrue(journal.stopped)
                self.assertTrue(journal.offset_certain)
                with self.assertRaises(observe.ObservationError):
                    journal.sync()
                with self.assertRaises(observe.ObservationError):
                    journal.append(row)
                self.assertEqual(calls, ["sync", "sync"])

    def test_invalid_write_counts_stop_on_first_call_without_claiming_a_row(self):
        for bad in (0, -1, True, None, 1.5, "1", 999):
            with self.subTest(count=bad):
                class InvalidStream:
                    calls = 0
                    def write(self, value):
                        self.calls += 1
                        if self.calls > 1:
                            raise AssertionError("invalid write was retried")
                        return bad
                stream = InvalidStream()
                journal = observe.Journal(stream, lambda: None)
                with self.assertRaises(observe.ObservationError):
                    journal.append(b'["s"]\n')
                self.assertEqual(stream.calls, 1)
                self.assertEqual(journal.complete_offset, 0)
                self.assertEqual(journal.written_offset, 0)
                self.assertEqual(journal.complete_rows, 0)
                self.assertTrue(journal.stopped)
                self.assertFalse(journal.offset_certain)

    def test_partial_exception_preserves_prefix_and_prevents_further_io(self):
        failure = OSError("ENOSPC after an uncertain partial write")
        class FailedStream(io.BytesIO):
            fail = False
            calls = 0
            def write(self, value):
                self.calls += 1
                if self.fail:
                    super().write(value[:2])
                    raise failure
                return super().write(value)

        stream = FailedStream()
        sync_calls = []
        journal = observe.Journal(stream, lambda: sync_calls.append("sync"))
        row = b'["s",1,"0","1"]\n'
        journal.append(row)
        journal.sync()
        stream.fail = True
        with self.assertRaises(OSError) as raised:
            journal.append(row)
        self.assertIs(raised.exception, failure)
        self.assertEqual(stream.getvalue(), row + row[:2])
        self.assertEqual(journal.complete_offset, len(row))
        self.assertEqual(journal.synced_offset, len(row))
        self.assertEqual(journal.complete_rows, 1)
        self.assertTrue(journal.stopped)
        self.assertFalse(journal.offset_certain)
        calls = stream.calls
        with self.assertRaises(observe.ObservationError):
            journal.append(row)
        with self.assertRaises(observe.ObservationError):
            journal.sync()
        self.assertEqual(stream.calls, calls)
        self.assertEqual(sync_calls, ["sync"])
        self.assertFalse(stream.closed)

    def test_short_writes_continue_without_duplicate_or_missing_bytes(self):
        class ShortStream(io.BytesIO):
            def write(self, value):
                return super().write(value[:3])

        stream = ShortStream()
        journal = observe.Journal(stream, lambda: None)
        row = b'["s",1,"0","1"]\n'
        journal.append(row)
        self.assertEqual(stream.getvalue(), row)
        self.assertEqual(journal.written_offset, len(row))
        self.assertEqual(journal.complete_offset, len(row))
        self.assertEqual(journal.complete_rows, 1)

    def test_complete_row_is_written_once_and_sync_tracks_prefix(self):
        stream = io.BytesIO()
        sync_calls = []
        journal = observe.Journal(stream, lambda: sync_calls.append("sync"))
        row = b'["s",1,"0","1"]\n'
        journal.append(row)
        self.assertEqual(stream.getvalue(), row)
        self.assertEqual(journal.complete_offset, len(row))
        self.assertEqual(journal.complete_rows, 1)
        self.assertEqual(journal.synced_offset, 0)
        journal.sync()
        self.assertEqual(sync_calls, ["sync"])
        self.assertEqual(journal.synced_offset, len(row))
        self.assertFalse(stream.closed)


if __name__ == "__main__":
    unittest.main()
