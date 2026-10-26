"""Controls and owned forced samples for one diagnostic; no launch authority."""

import base64
import errno
import datetime
import hashlib
import json
import uuid

import observe
import family
from observe import Journal, ObservationError, _decimal, _encoded, _hex64


LABELS = ('staging-before', 'staging-after', 'lock-before', 'lock-after',
          'check-before', 'check-after', 'before-verify',
          'verify-return-before-copy', 'copy-start', 'copy-end',
          'postguard-complete')


def _wire(value: list) -> bytes:
    raw = (json.dumps(value, separators=(',', ':'), ensure_ascii=True) + '\n').encode()
    if len(raw) > 512:
        raise ObservationError('control_size')
    return raw


def _number(value: int, maximum: int) -> int:
    if type(value) is not int or not 1 <= value <= maximum:
        raise ObservationError('control_number')
    return value


def _decode(raw: bytes, length: int, tag: str) -> list:
    if type(raw) is not bytes or not 1 <= len(raw) <= 512:
        raise ObservationError('control_size')
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise ObservationError('control_json') from error
    if (type(value) is not list or len(value) != length or value[0] != tag
            or type(value[1]) is not int or value[1] != 1):
        raise ObservationError('control_shape')
    if _wire(value) != raw:
        raise ObservationError('control_canonical')
    return value


def _request_fields(nonce: str, sequence: int, label: str) -> tuple[str, int, str]:
    _hex64(nonce)
    _number(sequence, 11)
    if type(label) is not str or label != LABELS[sequence - 1]:
        raise ObservationError('control_label')
    return nonce, sequence, label


def _ack_fields(sample: int, complete: str, synced: str,
                binding: str) -> tuple[int, str, str, str]:
    _number(sample, 1012736)
    _decimal(complete)
    if complete is None or not 1 <= int(complete) <= 1012736:
        raise ObservationError('control_offset')
    if type(synced) is not str or synced != complete:
        raise ObservationError('control_sync')
    _hex64(binding)
    return sample, complete, synced, binding


def encode_request(nonce: str, sequence: int) -> bytes:
    _hex64(nonce)
    _number(sequence, 11)
    return _wire(['q', 1, nonce, sequence, LABELS[sequence - 1]])


def decode_request(raw: bytes) -> tuple[str, int, str]:
    value = _decode(raw, 5, 'q')
    return _request_fields(*value[2:])


def encode_ack(request: bytes, sample: int, complete: str, binding: str) -> bytes:
    nonce, sequence, label = decode_request(request)
    _ack_fields(sample, complete, complete, binding)
    return _wire(['a', 1, nonce, sequence, label, sample, complete, complete, binding])


def decode_ack(raw: bytes) -> tuple[str, int, str, int, str, str, str]:
    value = _decode(raw, 9, 'a')
    return _request_fields(*value[2:5]) + _ack_fields(*value[5:])


class BoundaryWire:
    """Correlate host wire records only; caller must validate retained evidence."""

    def __init__(self, nonce: str, binding: str) -> None:
        self.nonce, self.binding = _hex64(nonce), _hex64(binding)
        self.sequence = 1
        self.pending = None
        self.last_ack = None
        self.failed = False
        self.last_time = None
        self.started = None
        self.sample = 0
        self.complete = 0

    def _fail(self, reason: str) -> None:
        self.failed = True
        raise ObservationError(reason)

    def _clock(self, now_ns: int) -> None:
        if self.failed:
            raise ObservationError('control_failed')
        if (type(now_ns) is not int or not 0 <= now_ns < 2**64
                or (self.last_time is not None and now_ns < self.last_time)):
            self._fail('control_clock')
        self.last_time = now_ns

    def request(self, now_ns: int) -> bytes:
        self._clock(now_ns)
        if self.pending is not None or self.sequence > 11:
            self._fail('control_state')
        raw = encode_request(self.nonce, self.sequence)
        self.pending = raw
        self.started = now_ns
        return raw

    def poll_ack(self, raw: bytes | None, now_ns: int) -> tuple | None:
        self._clock(now_ns)
        if self.pending is None:
            self._fail('control_state')
        if now_ns - self.started > 2000000000:
            self._fail('control_deadline')
        if raw is None:
            return None
        if type(raw) is not bytes:
            self._fail('control_size')
        if raw == self.last_ack:
            return None
        try:
            ack = decode_ack(raw)
        except BaseException:
            self.failed = True
            raise
        if ack[:3] != decode_request(self.pending) or ack[-1] != self.binding:
            self._fail('control_mismatch')
        if ack[3] <= self.sample or int(ack[4]) <= self.complete:
            self._fail('control_prefix_order')
        self.sample, self.complete = ack[3], int(ack[4])
        self.last_ack = raw
        self.pending = None
        self.sequence += 1
        return ack


def retain_ack_copy(journal: Journal, request: bytes, sample: int,
                    received_us: str, binding: str) -> bytes:
    """Retain wire copies only; no sample validation or channel publication."""
    if type(journal) is not Journal:
        raise ObservationError('boundary_journal')
    try:
        _, sequence, label = decode_request(request)
        _number(sample, 1012736)
        if _decimal(received_us) is None:
            raise ObservationError('boundary_clock')
        _hex64(binding)
        if journal.stopped or journal.complete_offset == 0:
            raise ObservationError('boundary_prefix')
        prior = 'synced' if journal.synced_offset == journal.complete_offset else 'unsynced'
        first = _encoded(['b', sequence, label, received_us, sample,
                          str(journal.complete_offset), prior,
                          [0, base64.b64encode(request).decode('ascii'), None]], 8192)
        prefix = journal.complete_offset + len(first)
        ack = encode_ack(request, sample, str(prefix), binding)
        second = _encoded(['b', sequence, label, received_us, sample,
                           str(prefix), 'synced',
                           [0, None, base64.b64encode(ack).decode('ascii')]], 8192)
        if len(first) + len(second) > 57344 - journal.reserved['markers']:
            raise ObservationError('quota_stop:markers')
        journal.append(first)
        journal.sync()
        if (journal.complete_offset, journal.synced_offset) != (prefix, prefix):
            raise ObservationError('boundary_prefix')
        journal.append(second)
        journal.sync()
        end = prefix + len(second)
        if (journal.complete_offset, journal.synced_offset) != (end, end):
            raise ObservationError('boundary_prefix')
        return ack
    except BaseException:
        journal.stopped = True
        raise


def _numeric_line(raw):
    if type(raw) is not bytes or not raw.endswith(b'\n') or raw.count(b'\n') != 1:
        raise ObservationError('sample_numeric_line')
    return observe.u64(raw[:-1])


def _slow_stat(raw):
    if type(raw) is not bytes or not raw or len(raw) > 16384 or not raw.endswith(b'\n'):
        raise ObservationError('sample_stat')
    lines, keys = raw.split(b'\n')[:-1], set()
    if len(lines) > 1024:
        raise ObservationError('sample_stat_count')
    for line in lines:
        parts = line.split()
        if len(parts) != 2:
            raise ObservationError('sample_stat_fields')
        key, value = parts
        if not 1 <= len(key) <= 64 or any(n < 33 or n > 126 for n in key) or key in keys:
            raise ObservationError('sample_stat_key')
        observe.u64(value)
        keys.add(key)
    return raw.decode('ascii')


def _pressure(raw):
    if type(raw) is not bytes or not raw or len(raw) > 4096 or not raw.endswith(b'\n'):
        raise ObservationError('sample_pressure')
    categories = set()
    for line in raw.split(b'\n')[:-1]:
        parts = line.split()
        if len(parts) != 5 or parts[0] not in (b'some', b'full') or parts[0] in categories:
            raise ObservationError('sample_pressure_fields')
        categories.add(parts[0])
        fields = {}
        for part in parts[1:]:
            key, sep, value = part.partition(b'=')
            if not sep or key in fields:
                raise ObservationError('sample_pressure_key')
            fields[key] = value
        if set(fields) != {b'avg10', b'avg60', b'avg300', b'total'}:
            raise ObservationError('sample_pressure_key')
        observe.u64(fields.pop(b'total'))
        for value in fields.values():
            pieces = value.split(b'.')
            if len(pieces) != 2 or any(not 1 <= len(p) <= 20 or not p.isdigit() for p in pieces):
                raise ObservationError('sample_pressure_decimal')
    return raw.decode('ascii')


def _path(value):
    if (type(value) is not str or not value.startswith('/')
            or not 1 <= len(value.encode('utf-8')) <= 4092
            or any(c in value for c in ('\0', '\r', '\n'))):
        raise ObservationError('sample_path')
    return value


def _sample_identity(value, target, pin):
    if type(value) is not tuple or len(value) != 6:
        raise ObservationError('sample_identity_tuple')
    cid, image, started, boot, cg, observer = value
    _hex64(cid)
    if type(image) is not str or not image.startswith('sha256:'):
        raise ObservationError('sample_image')
    _hex64(image[7:])
    if (type(started) is not str or not 1 <= len(started) <= 64
            or not started.isascii() or 'T' not in started or not started.endswith('Z')):
        raise ObservationError('sample_started')
    try:
        datetime.datetime.fromisoformat(started)
        if type(boot) is not str or str(uuid.UUID(boot)) != boot:
            raise ValueError('boot')
    except (ValueError, TypeError, AttributeError) as error:
        raise ObservationError('sample_epoch') from error
    if (type(cg) is not tuple or len(cg) != 4 or _path(cg[0]).encode('utf-8') != target
            or any(type(x) is not int or not 0 <= x < 2**64 for x in cg[1:])
            or cg[1:] != (pin[0], pin[1], pin[3])):
        raise ObservationError('sample_cgroup_identity')
    if type(observer) is not tuple or len(observer) != 3:
        raise ObservationError('sample_observer_identity')
    if _decimal(observer[0]) in (None, '0') or _decimal(observer[1]) is None:
        raise ObservationError('sample_observer_identity')
    path = _path(observer[2])
    if path == cg[0] or path.startswith(cg[0].rstrip('/') + '/'):
        raise ObservationError('sample_observer_placement')
    return value


def _root_pair(value):
    if (type(value) is not tuple or len(value) != 2
            or type(value[0]) is not int or value[0] < 0):
        raise ObservationError('sample_root_pair')
    observe._validate_directory_pin(value[1])


class SampleBoundary:
    """One owner's forced samples; borrowed roots and live identity need preflight."""

    def __init__(self, journal, nonce, expected_identity, clock_origin_ns,
                 cgroup, proc, target, read_identity, clock_ns):
        if type(journal) is not Journal or journal.stopped or journal.written_offset != 0:
            raise ObservationError('sample_journal')
        _hex64(nonce)
        _root_pair(cgroup)
        _root_pair(proc)
        if type(target) is not bytes or not callable(read_identity) or not callable(clock_ns):
            raise ObservationError('sample_context')
        _sample_identity(expected_identity, target, cgroup[1])
        observe.guest_interval(clock_origin_ns, clock_origin_ns, clock_origin_ns)
        self.journal, self.nonce = journal, nonce
        self.stream, self.synchronize = journal.stream, journal.synchronize
        self.identity, self.origin = expected_identity, clock_origin_ns
        self.cgroup, self.proc, self.target = cgroup, proc, target
        self.read_identity, self.clock = read_identity, clock_ns
        self.binding = hashlib.sha256(_encoded(list(expected_identity), 8192)).hexdigest()
        self.sample, self.keyset, self.identity_count = 0, 0, 0
        self.keys, self.live = (), {}
        self.sequence, self.failed = 1, False
        self.previous_events, self.previous_local = {}, None
        self.ever_nonzero = False
        self.busy, self.header_written = False, False
        self.last_end = self.origin
        self.last_clock = self.origin
        self.started, self.ended, self.attempt = None, None, None
        self.field = 'context'
        self.seal = self._signature()

    def _signature(self):
        journal = self.journal
        return (journal.complete_rows, journal.written_offset, journal.complete_offset,
                journal.synced_offset, tuple(sorted(journal.reserved.items())),
                journal.offset_certain)

    def _journal_guard(self):
        self._owner_guard()
        if self.failed or self.journal.stopped or self._signature() != self.seal:
            raise ObservationError('sample_journal_or_failed')

    def _owns_journal(self):
        return (self.journal.stream is self.stream
                and self.journal.synchronize is self.synchronize)

    def _owner_guard(self):
        if not self._owns_journal():
            raise ObservationError('sample_journal_owner')

    def _identity_guard(self):
        self.field = 'identity'
        observed = self.read_identity()
        self._journal_guard()
        if _sample_identity(observed, self.target, self.cgroup[1]) != self.identity:
            raise ObservationError('sample_identity')

    def _append(self, row, cap):
        self._journal_guard()
        raw = _encoded(row, cap)
        previous = self._signature()
        self.journal.append(raw)
        self._owner_guard()
        if (self.failed or self.journal.complete_rows != previous[0] + 1
                or self.journal.written_offset != previous[1] + len(raw)
                or self.journal.complete_offset != previous[2] + len(raw)
                or self.journal.synced_offset != previous[3]):
            raise ObservationError('sample_journal_append')
        self.seal = self._signature()

    def _read(self, name):
        self.field = name
        raw = observe.read_observation_file(*self.cgroup, name)
        self._journal_guard()
        return raw

    def _time(self):
        self.field = 'clock'
        value = self.clock()
        self._journal_guard()
        if type(value) is not int or not max(self.last_end, self.last_clock) <= value < 2**64:
            raise ObservationError('sample_clock')
        self.last_clock = value
        return value

    def _header(self):
        cid, image, started, boot, cg, observer = self.identity
        self._append(['h', 3, self.nonce, boot, [cid, image, started], cg,
                      observer, str(self.origin), 'relative_us',
                      {'quotaProfile': 'owner-retention/1', 'pidFlags': {'rssAbsent': 1, 'hwmAbsent': 2,
                                    'namespaceAbsent': 4, 'parentUnknown': 8},
                       'sampleFlags': {'peakUnsupported': 1},
                       'slowFlags': {'pressureUnsupported': 1, 'localUnsupported': 2},
                       'maxPids': 64, 'maxIdentities': 1024, 'maxKeysets': 32}], 8192)
        self.header_written = True

    def _memory(self, slow=True):
        events = observe.parse_events(self._read('memory.events'))
        numbers = [_numeric_line(self._read(name)) for name in
                   ('memory.current', 'memory.swap.current')]
        peak = self._optional('memory.peak')
        numbers.append(None if peak is None else _numeric_line(peak))
        if not slow:
            return events, numbers, None, None, (int(peak is None), None)
        stat = _slow_stat(self._read('memory.stat'))
        pressure = self._optional('memory.pressure')
        self.field = 'memory.pressure'
        pressure_text = None if pressure is None else _pressure(pressure)
        local = self._optional('memory.events.local')
        self.field = 'memory.events.local'
        local_events = None if local is None else observe.parse_events(local)
        slow = [stat, pressure_text, None if local is None else local.decode('ascii')]
        flags = (int(peak is None), int(pressure is None) + 2 * int(local is None))
        return events, numbers, slow, local_events, flags

    def _optional(self, name):
        try:
            return self._read(name)
        except FileNotFoundError as error:
            if error.errno != errno.ENOENT:
                raise
            observe.check_no_descendants(*self.cgroup)
            self._identity_guard()
            return None

    def _history(self, events, local, local_observed=True):
        pairs = [(self.previous_events, events)]
        if local_observed:
            pairs.append((self.previous_local, local))
        for previous, current in pairs:
            if previous is not None and (current is None or any(
                    key not in current or int(current[key]) < int(value)
                    for key, value in previous.items())):
                raise ObservationError('sample_event_continuity')
        self.ever_nonzero |= any(value != '0' for value in events.values())
        self.ever_nonzero |= local is not None and any(value != '0' for value in local.values())
        self.previous_events = events
        if local_observed:
            self.previous_local = local
        if self.sequence == 7 and self.ever_nonzero:
            raise ObservationError('sample_nonzero_preverify')

    def _dictionary(self, events):
        keys = tuple(sorted(events))
        if keys != self.keys:
            if self.keyset >= 32:
                raise ObservationError('sample_keyset_limit')
            self.keyset += 1
            self._append(['e', self.keyset, keys], 8192)
            self.keys = keys

    def _processes(self, census):
        self.field = 'process'
        rows, live = [], {}
        for pid in observe.parse_procs(census):
            value, before, after, membership, parent = family.identity_evidence(
                family.owned_family_evidence(self.cgroup, self.proc, pid, self.target))
            raw = lambda data: base64.b64encode(data).decode('ascii')
            fields = [pid, value['start'], value['ppid'], value['parent_start'],
                      'owned_unattributed', None, raw(value['cmdline']),
                      value['namespace'], raw(membership), raw(before), raw(after), parent]
            fingerprint = hashlib.sha256(_encoded(fields, 8192)).hexdigest()
            old = self.live.get(pid)
            if old is not None and old[0] == fingerprint:
                identity = old[1]
            else:
                if self.identity_count >= 1024:
                    raise ObservationError('sample_identity_limit')
                self.identity_count += 1
                identity = self.identity_count
                self._append(['i', identity, *fields], 8192)
            live[pid] = (fingerprint, identity)
            flags = sum(bit for key, bit in [('rss_kib', 1), ('hwm_kib', 2),
                        ('namespace', 4), ('parent_start', 8)] if value[key] is None)
            rows.append([identity, value['rss_kib'], value['hwm_kib'], flags])
        self.live = live
        return rows

    def _schedule_count(self, schedule):
        if schedule is None:
            return 0, None
        due, pending, periodic = schedule
        if (type(due) is not int or not self.origin <= due < 2**64
                or (due-self.origin) % 100000000 or type(pending) is not int
                or not 0 <= pending < 2**53 or type(periodic) is not bool
                or (periodic and self.started < due)):
            raise ObservationError('sample_schedule')
        crossed = 0 if self.ended < due else 1+(self.ended-due)//100000000
        missed = pending+crossed-int(periodic)
        future = due+crossed*100000000
        if not 0 <= missed < 2**53 or future >= 2**64:
            raise ObservationError('sample_schedule_overflow')
        return missed, future

    def _capture(self, schedule=None, slow_due_ns=None):
        self._identity_guard()
        self.started = self._time()
        slow = slow_due_ns is None or self.started >= slow_due_ns
        if not self.header_written:
            self._header()
        observe.check_no_descendants(*self.cgroup)
        census = self._read('cgroup.procs')
        events, numbers, slow_values, local, flags = self._memory(slow)
        self._dictionary(events)
        processes = self._processes(census)
        if set(observe.parse_procs(self._read('cgroup.procs'))) != set(observe.parse_procs(census)):
            raise ObservationError('sample_census')
        observe.check_no_descendants(*self.cgroup)
        self._identity_guard()
        self.ended = self._time()
        interval = observe.guest_interval(self.origin, self.started, self.ended)
        missed, future = self._schedule_count(schedule)
        self.sample += 1
        self._append(['s', self.sample, *interval, self.keyset,
                      [events[key] for key in self.keys], *numbers,
                      processes, missed, flags[0]], 16384)
        if slow:
            self._append(['m', self.sample, *interval, *slow_values, flags[1]], 32768)
        self._history(events, local, local_observed=slow)
        self._identity_guard()
        return interval, future, slow

    def _failure(self, error):
        self.failed = True
        try:
            if (self.header_written and self._owns_journal()
                    and not self.journal.stopped and self._signature() == self.seal):
                number = error.errno if type(error) in (OSError, FileNotFoundError, PermissionError) else None
                if type(number) is not int or not 0 <= number <= 65535:
                    number = None
                reason = ('vanished' if number == errno.ENOENT else 'denied'
                          if number == errno.EACCES else 'io_failure'
                          if isinstance(error, OSError) else 'parse_error')
                times = [None if self.started is None else str((self.started-self.origin)//1000), None]
                if self.started is not None and self.ended is not None and self.ended >= self.started:
                    times[1] = str((self.ended-self.origin+999)//1000)
                self.journal.append(_encoded(['g', self.attempt, *times, reason,
                                               [self.field, number]], 8192))
        except BaseException as gap_error:
            raise BaseExceptionGroup('sample_and_gap_failures', [error, gap_error])
        finally:
            self.journal.stopped = True

    def capture_boundary(self, request, *, due_ns=None, pending_misses=0):
        if self.busy:
            self.failed = True
            self.journal.stopped = True
            raise ObservationError('sample_reentrant')
        self.busy = True
        self.started, self.ended, self.attempt = None, None, self.sample + 1
        try:
            self._journal_guard()
            nonce, sequence, _ = decode_request(request)
            if (nonce, sequence) != (self.nonce, self.sequence):
                raise ObservationError('sample_request')
            schedule = None if due_ns is None else (due_ns, pending_misses, False)
            interval, future, slow = self._capture(schedule)
            ack = retain_ack_copy(self.journal, request, self.sample, interval[0], self.binding)
            self._owner_guard()
            if self.failed:
                raise ObservationError('sample_failed')
            self.seal = self._signature()
            self.sequence += 1
            self.last_end = self.ended
            if schedule is None:
                return ack
            return self._completion(future, slow, ack)
        except BaseException as error:
            self._failure(error)
            raise
        finally:
            self.busy = False

    def _completion(self, future, slow, ack=None):
        finished = self._time()
        self._journal_guard()
        return {'ack': ack, 'next_due': future, 'started_ns': self.started,
                'read_end_ns': self.ended, 'finished_ns': finished, 'slow': slow}

    def capture_periodic(self, due_ns, pending_misses, slow_due_ns):
        if self.busy:
            self.failed = True
            self.journal.stopped = True
            raise ObservationError('sample_reentrant')
        self.busy = True
        self.started, self.ended, self.attempt = None, None, self.sample+1
        try:
            self._journal_guard()
            if (type(slow_due_ns) is not int or not self.origin <= slow_due_ns < 2**64
                    or (slow_due_ns-self.origin) % 5000000000):
                raise ObservationError('sample_slow_schedule')
            _, future, slow = self._capture((due_ns, pending_misses, True), slow_due_ns)
            self.last_end = self.ended
            return self._completion(future, slow)
        except BaseException as error:
            self._failure(error)
            raise
        finally:
            self.busy = False

    def synchronize_owned(self):
        if self.busy:
            self.failed = True
            self.journal.stopped = True
            raise ObservationError('sample_reentrant')
        self.busy = True
        try:
            self._journal_guard()
            before = self._signature()
            self.journal.sync()
            self._owner_guard()
            if self._signature() != (*before[:3], before[2], *before[4:]):
                raise ObservationError('sample_sync_mutation')
            self.seal = self._signature()
            return self._time()
        except BaseException as error:
            self._failure(error)
            raise
        finally:
            self.busy = False

    def now(self):
        if self.busy:
            self.failed = True
            self.journal.stopped = True
            raise ObservationError('sample_reentrant')
        self.busy = True
        try:
            self._journal_guard()
            value = self._time()
            self._journal_guard()
            return value
        except BaseException:
            self.failed = True
            self.journal.stopped = True
            raise
        finally:
            self.busy = False
