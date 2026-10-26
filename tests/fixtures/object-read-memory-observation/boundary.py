"""Pure controls for one diagnostic; no IO, clock acquisition or launch authority."""

import base64
import json

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
    _number(sample, 1013760)
    _decimal(complete)
    if complete is None or not 1 <= int(complete) <= 1013760:
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
        _number(sample, 1013760)
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
