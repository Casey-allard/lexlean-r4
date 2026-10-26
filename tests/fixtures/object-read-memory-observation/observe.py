"""One-off admitted-range diagnostic parsing; no live observer entrypoint yet."""

import base64
import json
from typing import BinaryIO, Callable


class ObservationError(ValueError):
    """A failed observation; its reason must never be replaced with zero."""


def u64(raw: bytes) -> str:
    """Keep a counter's decimal representation without a float conversion."""
    if (not isinstance(raw, bytes) or not 1 <= len(raw) <= 20
            or not raw.isdigit() or (len(raw) > 1 and raw[:1] == b"0")
            or int(raw) > 18446744073709551615):
        raise ObservationError("invalid_u64")
    return raw.decode("ascii")


def parse_events(raw: bytes) -> dict[str, str]:
    """Decode keyed counters without discarding future kernel fields."""
    required = {"low", "high", "max", "oom", "oom_kill", "oom_group_kill"}
    if not isinstance(raw, bytes) or len(raw) > 4096 or not raw.endswith(b"\n"):
        raise ObservationError("incomplete_events")
    lines = raw.split(b"\n")[:-1]
    if not 1 <= len(lines) <= 64:
        raise ObservationError("event_key_limit")
    result = {}
    for line in lines:
        fields = line.split()
        if len(fields) != 2:
            raise ObservationError("event_fields")
        key, value = fields
        if not 1 <= len(key) <= 64 or any(c < 33 or c > 126 for c in key):
            raise ObservationError("event_key")
        name = key.decode("ascii")
        if name in result:
            raise ObservationError("duplicate_event_key")
        result[name] = u64(value)
    if not required.issubset(result):
        raise ObservationError("missing_event_key")
    return result


def read_bounded(stream: BinaryIO, cap: int) -> bytes:
    """Read bytes from a caller-owned stream; ownership is checked by its caller."""
    if isinstance(cap, bool) or not isinstance(cap, int) or not 1 <= cap <= 16384:
        raise ObservationError("read_cap")
    data = bytearray()
    while len(data) <= cap:
        remaining = cap + 1 - len(data)
        part = stream.read(remaining)
        if not isinstance(part, bytes) or len(part) > remaining:
            raise ObservationError("read_protocol")
        if not part:
            return bytes(data)
        data.extend(part)
    raise ObservationError("truncated_read")


def parse_procs(raw: bytes) -> tuple[str, ...]:
    """Decode only the supplied owned cgroup census, preserving its order."""
    if not isinstance(raw, bytes) or len(raw) > 4096:
        raise ObservationError("census_bytes")
    if not raw:
        return ()
    if not raw.endswith(b"\n"):
        raise ObservationError("incomplete_census")
    lines = raw.split(b"\n")[:-1]
    if len(lines) > 64:
        raise ObservationError("census_count")
    pids = tuple(u64(line) for line in lines)
    if "0" in pids or len(set(pids)) != len(pids):
        raise ObservationError("invalid_census")
    return pids


def parse_stat(raw: bytes) -> tuple[str, str, str]:
    """Return PID, PPID and start ticks; comm may contain spaces or parentheses."""
    # Field numbering: https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html
    if not isinstance(raw, bytes) or len(raw) > 4096 or not raw.endswith(b"\n"):
        raise ObservationError("incomplete_stat")
    begin, end = raw.find(b" ("), raw.rfind(b") ")
    if begin < 1 or end < begin + 2:
        raise ObservationError("stat_delimiters")
    tail = raw[end + 2:-1]
    if b"\n" in tail or b"\r" in tail:
        raise ObservationError("stat_tail")
    fields = tail.split()
    if len(fields) < 20 or len(fields[0]) != 1 or not fields[0].isalpha():
        raise ObservationError("stat_fields")
    pid, ppid, start = u64(raw[:begin]), u64(fields[1]), u64(fields[19])
    if pid == "0":
        raise ObservationError("stat_pid")
    return pid, ppid, start


def parse_status(raw: bytes) -> tuple[
        str, str, tuple[str, ...] | None, str | None, str | None]:
    """Return PID, PPID, NSpid, RSS KiB and HWM KiB; absent memory stays unknown."""
    if not isinstance(raw, bytes) or len(raw) > 8192 or not raw.endswith(b"\n"):
        raise ObservationError("incomplete_status")
    fields = {}
    selected = {b"Pid", b"PPid", b"NSpid", b"VmRSS", b"VmHWM"}
    for line in raw.split(b"\n")[:-1]:
        key, separator, value = line.partition(b":")
        if not separator:
            raise ObservationError("status_fields")
        if key in selected:
            if key in fields:
                raise ObservationError("duplicate_status_field")
            fields[key] = value.strip()
    if b"Pid" not in fields or b"PPid" not in fields:
        raise ObservationError("missing_status_identity")
    pid, ppid = u64(fields[b"Pid"]), u64(fields[b"PPid"])
    if pid == "0":
        raise ObservationError("status_pid")
    namespace = None
    if b"NSpid" in fields:
        values = fields[b"NSpid"].split()
        if not 1 <= len(values) <= 64:
            raise ObservationError("namespace_count")
        namespace = tuple(u64(n) for n in values)
        if namespace[0] != pid or "0" in namespace:
            raise ObservationError("namespace_identity")
    memory = []
    for key in (b"VmRSS", b"VmHWM"):
        if key not in fields:
            memory.append(None)
            continue
        parts = fields[key].split()
        if len(parts) != 2 or parts[1] != b"kB":
            raise ObservationError("status_memory_units")
        memory.append(u64(parts[0]))
    return pid, ppid, namespace, memory[0], memory[1]


def parse_membership(raw: bytes) -> bytes:
    """Return an exact v2 path, never a normalized or traversed filesystem path."""
    if (not isinstance(raw, bytes) or len(raw) > 4096
            or not raw.startswith(b"0::/") or not raw.endswith(b"\n")
            or raw.count(b"\n") != 1 or b"\x00" in raw or b"\r" in raw):
        raise ObservationError("invalid_membership")
    return raw[3:-1]


def row_pool(row: bytes) -> str:
    """Validate only JSONL envelope/tag/size; full observation schema is separate."""
    caps = {"s": 16384, "m": 32768, "i": 8192, "e": 8192,
            "h": 8192, "b": 8192, "g": 8192}
    if (type(row) is not bytes or not 1 <= len(row) <= 32768
            or not row.endswith(b"\n") or row.count(b"\n") != 1 or b"\r" in row):
        raise ObservationError("row_envelope")
    def reject_constant(value: str) -> None:
        raise ObservationError("json_constant")
    try:
        value = json.loads(row.decode("utf-8"), parse_constant=reject_constant)
    except (ValueError, RecursionError) as error:
        raise ObservationError("row_json") from error
    if (not isinstance(value, list) or not value or type(value[0]) is not str
            or value[0] not in caps or len(row) > caps[value[0]]):
        raise ObservationError("row_tag_or_limit")
    tag = value[0]
    return "samples" if tag in ("s", "m") else "identities" if tag in ("i", "e") else "markers"


class Journal:
    """Single-owner append-only journal; no path, terminal writer or ACK issuance.

    Caller supplies an empty exclusive unbuffered stream and a trusted synchronous
    fsync operation returning None. No concurrent or reentrant use is supported.
    After an IO failure, written_offset counts only reported bytes; the true end
    may be unknown. Reserved bytes conservatively include the entire failed row.
    """

    def __init__(self, stream: BinaryIO, synchronize: Callable[[], None]) -> None:
        self.stream = stream
        self.synchronize = synchronize
        self.written_offset = 0
        self.complete_offset = 0
        self.complete_rows = 0
        self.synced_offset = 0
        self.stopped = False
        self.offset_certain = True
        self._reserved = {"samples": 0, "identities": 0, "markers": 26624,
                          "terminal": 8192}

    @property
    def reserved(self) -> dict[str, int]:
        return self._reserved.copy()

    def append(self, row: bytes) -> None:
        if self.stopped:
            raise ObservationError("journal_stopped")
        try:
            pool = row_pool(row)
            limits = {"samples": 786432, "identities": 196608, "markers": 57344}
            if len(row) > limits[pool] - self._reserved[pool]:
                raise ObservationError("quota_stop:" + pool)
            self._reserved[pool] += len(row)
        except BaseException:
            self.stopped = True
            raise
        remaining = memoryview(row)
        try:
            while remaining:
                count = self.stream.write(remaining)
                if type(count) is not int or not 0 < count <= len(remaining):
                    raise ObservationError("write_protocol")
                self.written_offset += count
                remaining = remaining[count:]
        except BaseException:
            self.stopped = True
            self.offset_certain = False
            raise
        self.complete_offset += len(row)
        self.complete_rows += 1

    def sync(self) -> None:
        if self.stopped:
            raise ObservationError("journal_stopped")
        try:
            if self.synchronize() is not None:
                raise ObservationError("sync_protocol")
        except BaseException:
            self.stopped = True
            raise
        self.synced_offset = self.complete_offset


def _choice(value, choices):
    if type(value) is not str or value not in choices:
        raise ObservationError("record_enum")
    return value


def _array(value, length):
    if type(value) is not list or len(value) != length:
        raise ObservationError("record_shape")
    return value.copy()


def _decimal(value):
    if value is None:
        return None
    if type(value) is not str or not 1 <= len(value) <= 20 or not value.isascii():
        raise ObservationError("record_decimal")
    return u64(value.encode("ascii"))


def _hex64(value):
    if (type(value) is not str or len(value) != 64
            or any(c not in "0123456789abcdef" for c in value)):
        raise ObservationError("record_hex")
    return value


def _process(value):
    state, code, signal = _array(value, 3)
    _choice(state, ("not_run", "running", "exited", "signaled", "unknown"))
    if state == "exited":
        valid = type(code) is int and 0 <= code <= 255 and signal is None
    elif state == "signaled":
        valid = code is None and type(signal) is int and 1 <= signal <= 64
    else:
        valid = code is None and signal is None
    if not valid:
        raise ObservationError("record_process")
    return [state, code, signal]


def _prefix(value):
    result = _array(value, 5)
    result[:4] = [_decimal(v) for v in result[:4]]
    if result[4] is not None and type(result[4]) is not bool:
        raise ObservationError("prefix_certainty")
    written, complete, rows, synced = [None if v is None else int(v) for v in result[:4]]
    if any(v is not None and v > 1013760 for v in (written, complete, rows, synced)):
        raise ObservationError("prefix_limit")
    for low, high in ((synced, complete), (complete, written), (synced, written), (rows, complete)):
        if low is not None and high is not None and low > high:
            raise ObservationError("prefix_order")
    if complete is not None and rows is not None and (complete == 0) != (rows == 0):
        raise ObservationError("prefix_rows")
    return result


def _coverage(value):
    result = [_decimal(v) for v in _array(value, 4)]
    if result[1] is not None and result[2] is not None and int(result[1]) > int(result[2]):
        raise ObservationError("coverage_order")
    return result


def _quotas(value):
    if value is None:
        return None, None
    result = _array(value, 4)
    limits = (786432, 196608, 57344, 8192)
    for number, minimum, maximum in zip(result, (0, 0, 26624, 8192), limits):
        if type(number) is not int or not minimum <= number <= maximum:
            raise ObservationError("record_quota")
    return result, [limit - number for limit, number in zip(limits, result)]


def _encoded(row, cap):
    result = (json.dumps(row, ensure_ascii=True, separators=(",", ":"),
                         allow_nan=False) + "\n").encode("utf-8")
    if len(result) > cap:
        raise ObservationError("record_cap")
    return result


def encode_terminal(role: object, *, nonce: object, binding: object,
                    final_state: object, reason: object, prefix: object,
                    journal_sync: object, coverage: object, detailed_gaps: object,
                    reserved: object, observer_process: object,
                    sdk_process: object) -> bytes:
    """Encode a prior observation snapshot, never this terminal's future sync."""
    _choice(role, ("guest", "host"))
    _hex64(nonce)
    _hex64(binding)
    _choice(final_state, ("ended", "stopped", "unobserved"))
    _choice(reason, ("ended", "quota_stop", "parse_error", "read_error", "io_failure",
                     "clock_error", "ownership_changed", "namespace_unknown",
                     "unsupported_subtree", "marker_timeout", "parent_missing",
                     "interrupted", "unknown"))
    if final_state == "unobserved" and reason != "unknown":
        raise ObservationError("terminal_unobserved")
    prefix = _prefix(prefix)
    _choice(journal_sync, ("not_attempted", "reported", "failed", "unknown"))
    if journal_sync == "reported" and prefix[3] is None:
        raise ObservationError("terminal_sync_unknown")
    coverage = _coverage(coverage)
    _choice(detailed_gaps, ("retained", "incomplete", "unknown"))
    reserved, remaining = _quotas(reserved)
    observer_process, sdk_process = _process(observer_process), _process(sdk_process)
    if role == "guest" and observer_process[0] not in ("running", "unknown"):
        raise ObservationError("terminal_future_exit")
    row = ["t", 1, role, nonce, binding, final_state, reason, prefix,
           journal_sync, coverage, detailed_gaps, reserved, remaining,
           observer_process, sdk_process]
    return _encoded(row, 4096)


def encode_retrieval(*, nonce: object, binding: object, start: object, end: object,
                     process: object, stdout: object, stderr: object,
                     stdout_complete: object, stderr_complete: object,
                     reason: object) -> bytes:
    """Encode supplied captures; no process launch or cause inference."""
    _hex64(nonce)
    _hex64(binding)
    start, end = _decimal(start), _decimal(end)
    if (start is None) != (end is None) or (start is not None and int(start) > int(end)):
        raise ObservationError("retrieval_time")
    process = _process(process)
    for raw in (stdout, stderr):
        if type(raw) is not bytes or len(raw) > 4096:
            raise ObservationError("retrieval_capture")
    if type(stdout_complete) is not bool or type(stderr_complete) is not bool:
        raise ObservationError("retrieval_completeness")
    _retrieval_reason(reason, process, stdout_complete, stderr_complete)
    row = ["r", 1, nonce, binding, start, end, process, stdout_complete,
           stderr_complete, base64.b64encode(stdout).decode("ascii"),
           base64.b64encode(stderr).decode("ascii"), reason]
    return _encoded(row, 16384)


def _retrieval_reason(reason, process, out_complete, err_complete):
    state, code, signal = process
    predicates = {
        "none": state == "exited" and code == 0 and out_complete and err_complete,
        "signaled": state == "signaled",
        "exit_nonzero": state == "exited" and code != 0,
        "truncated_stdout": not out_complete and err_complete,
        "truncated_stderr": out_complete and not err_complete,
        "truncated_both": not out_complete and not err_complete,
        "spawn_error": state in ("not_run", "unknown"),
        "read_error": not out_complete or not err_complete,
        "unknown": True,
    }
    _choice(reason, predicates)
    if not predicates[reason]:
        raise ObservationError("retrieval_reason")


class Retention:
    """Fixed one-run slots; caller owns exclusive streams and cross-process roles.

    Controls are reserved but their replacement protocol is not implemented here.
    No concurrent/reentrant use, path operations or successful root binding implied.
    """

    def __init__(self) -> None:
        self._slots = dict(request=512, request_temp=512, ack=512, ack_temp=512,
                           container_root=4096, host_root=4096, retrieval_receipt=16384,
                           guest_terminal=4096, host_terminal=4096)
        self._states = {name: dict(attempted=False, written=0, complete=False,
                                  synced=False, extent_known=True)
                        for name in tuple(self._slots)[4:]}

    @property
    def slots(self) -> dict[str, int]:
        return self._slots.copy()

    def state(self, slot: object) -> dict[str, int | bool]:
        _choice(slot, self._states)
        return self._states[slot].copy()

    def _claim(self, slot):
        if self._states[slot]["attempted"]:
            raise ObservationError("slot_already_attempted")
        self._states[slot]["attempted"] = True

    def write_root(self, role: object, raw: object, stream: BinaryIO,
                   synchronize: Callable[[], None]) -> None:
        _choice(role, ("container", "host"))
        slot = role + "_root"
        self._claim(slot)
        self._write(slot, raw, stream, synchronize)

    def write_retrieval(self, stream: BinaryIO, synchronize: Callable[[], None],
                        **fields: object) -> None:
        self._claim("retrieval_receipt")
        self._write("retrieval_receipt", encode_retrieval(**fields), stream, synchronize)

    def write_terminal(self, role: object, stream: BinaryIO,
                       synchronize: Callable[[], None], **fields: object) -> None:
        _choice(role, ("guest", "host"))
        slot = role + "_terminal"
        self._claim(slot)
        self._write(slot, encode_terminal(role, **fields), stream, synchronize)

    def _write(self, slot, raw, stream, synchronize):
        if type(raw) is not bytes or len(raw) > self._slots[slot]:
            raise ObservationError("slot_payload")
        state = self._states[slot]
        remaining = memoryview(raw)
        try:
            while remaining:
                count = stream.write(remaining)
                if type(count) is not int or not 0 < count <= len(remaining):
                    raise ObservationError("write_protocol")
                state["written"] += count
                remaining = remaining[count:]
        except BaseException:
            state["extent_known"] = False
            raise
        state["complete"] = True
        if synchronize() is not None:
            raise ObservationError("sync_protocol")
        state["synced"] = True
