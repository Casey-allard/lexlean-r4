"""Bounded root content linkage, conditional on external capture/namespace custody.

No IO, execution authority, descendant attribution or memory-cause inference.
"""

import base64
import binascii
import hashlib
import json
from dataclasses import dataclass

import boundary
import observe
from observe import ObservationError

VERIFY_ARGV = (b'lexlean', b'--diagnostic-format', b'json', b'verify',
               b'tests/lexlean/ObjectAdmittedRangeResult.lex.tex')


def _decimal(value):
    if type(value) is not str or observe._decimal(value) is None:
        raise ObservationError('root_decimal')
    return value


def _positive(value):
    if _decimal(value) == '0':
        raise ObservationError('root_positive')
    return value


def parse_root_marker(raw: bytes, expected_nonce: str) -> tuple[str, str]:
    """Decode the one-line proc stat wrapper marker; preserve no inferred exec."""
    observe._hex64(expected_nonce)
    if (type(raw) is not bytes or not 1 <= len(raw) <= 4096
            or raw.count(b'\n') != 4 or not raw.endswith(b'\n')
            or b'\r' in raw or b'\0' in raw):
        raise ObservationError('root_marker_shape')
    version, nonce, pid_raw, raw_stat = raw.split(b'\n', 3)
    if version != b'R4ROOT/1' or nonce != expected_nonce.encode('ascii'):
        raise ObservationError('root_marker_nonce_version')
    pid = _positive(observe.u64(pid_raw))
    stat_pid, _, start = observe.parse_stat(raw_stat)
    if stat_pid != pid:
        raise ObservationError('root_marker_pid')
    return pid, start


@dataclass(frozen=True)
class RootContext:
    """Invocation pins and retained host time; not proof of their live acquisition."""

    nonce: str
    identity: tuple
    namespace_coordinate: int
    namespace_depth: int
    return_ack_received_ns: int

    def validate(self):
        observe._hex64(self.nonce)
        if (type(self.identity) is not tuple or len(self.identity) != 6
                or type(self.identity[4]) is not tuple or len(self.identity[4]) != 4):
            raise ObservationError('root_identity_context')
        cg = self.identity[4]
        target = boundary._path(cg[0]).encode('utf-8')
        boundary._sample_identity(self.identity, target, (cg[1], cg[2], 0, cg[3]))
        if (type(self.namespace_coordinate) is not int or type(self.namespace_depth) is not int
                or not 0 <= self.namespace_coordinate < self.namespace_depth <= 64
                or type(self.return_ack_received_ns) is not int
                or not 0 <= self.return_ack_received_ns < 2**64):
            raise ObservationError('root_context_numbers')
        return hashlib.sha256(observe._encoded(list(self.identity), 8192)).hexdigest()


def _json(raw, cap):
    if type(raw) is not bytes or not 1 <= len(raw) <= cap:
        raise ObservationError('root_json_size')
    try:
        row = json.loads(raw)
        encoded = observe._encoded(row, cap)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise ObservationError('root_json') from error
    if type(row) is not list or encoded != raw:
        raise ObservationError('root_json_canonical')
    return row


def _bytes(value, cap):
    if type(value) is not str or len(value) > 4*((cap+2)//3):
        raise ObservationError('root_base64_size')
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise ObservationError('root_base64') from error
    if len(raw) > cap or base64.b64encode(raw).decode('ascii') != value:
        raise ObservationError('root_base64_canonical')
    return raw


def _retrieval(raw, marker, context, binding):
    row = _json(raw, 16384)
    if len(row) != 12 or row[:4] != ['r', 1, context.nonce, binding] or type(row[1]) is not int:
        raise ObservationError('root_retrieval_identity')
    stdout, stderr = _bytes(row[9], 4096), _bytes(row[10], 4096)
    encoded = observe.encode_retrieval(
        nonce=row[2], binding=row[3], start=row[4], end=row[5], process=row[6],
        stdout=stdout, stderr=stderr, stdout_complete=row[7], stderr_complete=row[8], reason=row[11])
    if (encoded != raw or row[6] != ['exited', 0, None] or type(row[6][1]) is not int
            or row[7:9] != [True, True] or row[11] != 'none' or stdout != marker
            or row[4] is None or int(row[4]) < context.return_ack_received_ns):
        raise ObservationError('root_retrieval_failed_or_order')


def _controls(values, context, binding):
    if type(values) is not tuple or len(values) != 4:
        raise ObservationError('root_controls')
    result = {}
    for seq, request, ack_raw in ((7, *values[:2]), (8, *values[2:])):
        req, ack = boundary.decode_request(request), boundary.decode_ack(ack_raw)
        if (req != (context.nonce, seq, boundary.LABELS[seq-1])
                or ack[:3] != req or ack[-1] != binding):
            raise ObservationError('root_control_identity')
        result[seq] = (request, ack_raw, ack)
    if (result[7][2][3] >= result[8][2][3]
            or int(result[7][2][4]) >= int(result[8][2][4])):
        raise ObservationError('root_control_order')
    return result


def _rows(journal):
    if type(journal) is not bytes or not 1 <= len(journal) <= 1012736:
        raise ObservationError('root_journal_size')
    caps = {'h': 8192, 'e': 8192, 'i': 8192, 's': 16384, 'm': 32768, 'g': 8192, 'b': 8192}
    offset = 0
    while offset < len(journal):
        end = journal.find(b'\n', offset, offset+32769)
        if end < 0:
            raise ObservationError('root_journal_line')
        raw = journal[offset:end+1]
        row = _json(raw, 32768)
        if not row or type(row[0]) is not str or row[0] not in caps or len(raw) > caps[row[0]]:
            raise ObservationError('root_journal_tag_size')
        yield offset, end+1, row
        offset = end+1


def _header(row, context):
    cid, image, started, boot, cg, observer = context.identity
    expected = ['h', 3, context.nonce, boot, [cid, image, started], list(cg), list(observer)]
    if (len(row) != 10 or observe._encoded(row[:7], 8192) != observe._encoded(expected, 8192)
            or row[8] != 'relative_us' or type(row[9]) is not dict
            or row[9].get('quotaProfile') != 'owner-retention/1'):
        raise ObservationError('root_header_identity')
    _decimal(row[7])


def _identity(row, target):
    if len(row) != 14 or row[6:8] != ['owned_unattributed', None]:
        raise ObservationError('root_identity_shape')
    boundary._number(row[1], 1024)
    pid, start, ppid = _positive(row[2]), _decimal(row[3]), _decimal(row[4])
    before, after = observe.parse_stat(_bytes(row[11], 4096)), observe.parse_stat(_bytes(row[12], 4096))
    if before != after or before != (pid, ppid, start):
        raise ObservationError('root_raw_stat_identity')
    if observe.parse_membership(_bytes(row[10], 4096)) != target:
        raise ObservationError('root_raw_membership')
    ns = row[9]
    if ns is not None:
        if type(ns) is not list or not 1 <= len(ns) <= 64:
            raise ObservationError('root_namespace')
        ns = tuple(_positive(n) for n in ns)
        if ns[0] != pid:
            raise ObservationError('root_namespace_pid')
    cmdline = _bytes(row[8], 4096)
    return (pid, start), ns, cmdline == b'\0'.join(VERIFY_ARGV)+b'\0'


class _Scan:
    """Bounded metadata only; non-root ancestry and unused measurement fields untouched."""

    def __init__(self, context, marker, controls):
        self.context, self.marker, self.controls = context, marker, controls
        self.identities, self.namespaces, self.matches = {}, {}, {}
        self.unknown_namespace, self.seen = False, set()
        self.sample, self.interval, self.pending = 0, None, None
        self.target = context.identity[4][0].encode('utf-8')

    def identity(self, row):
        item = _identity(row, self.target)
        if row[1] != len(self.identities)+1:
            raise ObservationError('root_identity_order')
        key, ns, _ = item
        if ns is not None:
            if key in self.namespaces and self.namespaces[key] != ns:
                raise ObservationError('root_namespace_conflict')
            self.namespaces[key] = ns
        self.identities[row[1]] = item

    def sample_row(self, row):
        if (len(row) != 12 or type(row[9]) is not list or len(row[9]) > 64
                or type(row[1]) is not int or row[1] != self.sample+1):
            raise ObservationError('root_sample_shape')
        start, end = int(_decimal(row[2])), int(_decimal(row[3]))
        if start > end or (self.interval is not None and start < self.interval[1]):
            raise ObservationError('root_sample_time')
        self.sample, self.interval = row[1], (start, end)
        active = self.controls[7][2][3] < self.sample <= self.controls[8][2][3]
        if active and 7 not in self.seen:
            raise ObservationError('root_before_ack_missing')
        ids, pids = set(), set()
        for value in row[9]:
            if type(value) is not list or len(value) != 4:
                raise ObservationError('root_process_reference')
            index = boundary._number(value[0], 1024)
            if index not in self.identities or index in ids:
                raise ObservationError('root_process_reference')
            item = self.identities[index]
            if item[0][0] in pids:
                raise ObservationError('root_duplicate_sample_pid')
            ids.add(index); pids.add(item[0][0])
            if active:
                self.match(item)

    def match(self, item):
        key, ns, argv = item
        if key[1] != self.marker[1]:
            return
        if ns is None or len(ns) != self.context.namespace_depth:
            self.unknown_namespace = True
            return
        if ns[self.context.namespace_coordinate] != self.marker[0]:
            return
        previous, samples = self.matches.get(key, (False, []))
        self.matches[key] = previous or argv, samples + [self.sample]

    def boundary_row(self, row, offset, end):
        if len(row) != 8 or type(row[1]) is not int:
            raise ObservationError('root_boundary_shape')
        seq = row[1]
        if seq not in self.controls:
            return
        request, ack_raw, ack = self.controls[seq]
        if (row[2] != ack[2] or row[4] != ack[3] or type(row[4]) is not int
                or self.sample != ack[3] or row[3] != str(self.interval[0])
                or row[5] != str(offset) or seq in self.seen):
            raise ObservationError('root_boundary_identity_offset')
        wire = row[7]
        if type(wire) is not list or len(wire) != 3 or type(wire[0]) is not int:
            raise ObservationError('root_boundary_status')
        first = [0, base64.b64encode(request).decode('ascii'), None]
        second = [0, None, base64.b64encode(ack_raw).decode('ascii')]
        if wire == first and self.pending is None and row[6] in ('synced', 'unsynced'):
            if end != int(ack[4]):
                raise ObservationError('root_ack_prefix')
            self.pending = seq, end
        elif wire == second and self.pending == (seq, offset) and row[6] == 'synced':
            self.seen.add(seq)
            self.pending = None
        else:
            raise ObservationError('root_boundary_copy')

    def result(self):
        if self.seen != {7, 8} or self.pending is not None:
            raise ObservationError('root_ack_copies_missing')
        reason = ('ambiguous_root' if len(self.matches) > 1 else 'namespace_unknown'
                  if self.unknown_namespace else 'root_not_observed' if not self.matches
                  else 'exec_not_observed' if not next(iter(self.matches.values()))[0] else None)
        key = next(iter(self.matches)) if reason is None else None
        return {'status': 'matched_observation' if key else 'unknown', 'reason': reason,
                'root': key, 'marker': self.marker,
                'samples': tuple(self.matches[key][1]) if key else (),
                'descendants': 'owned_unattributed', 'live_authority': False}


def join_invocation_root(journal: bytes, marker: bytes, retrieval: bytes,
                         controls: tuple, context: RootContext) -> dict:
    """Join retained contents only; external live custody/mapping remain required."""
    if type(context) is not RootContext:
        raise ObservationError('root_context')
    binding = context.validate()
    parsed = parse_root_marker(marker, context.nonce)
    _retrieval(retrieval, marker, context, binding)
    scan = _Scan(context, parsed, _controls(controls, context, binding))
    for offset, end, row in _rows(journal):
        if offset == 0:
            _header(row, context)
        elif row[0] == 'h':
            raise ObservationError('root_duplicate_header')
        elif scan.pending is not None and row[0] != 'b':
            raise ObservationError('root_nonadjacent_ack')
        elif row[0] == 'i':
            scan.identity(row)
        elif row[0] == 's':
            scan.sample_row(row)
        elif row[0] == 'b':
            scan.boundary_row(row, offset, end)
    return scan.result()
