"""One-off admitted-range diagnostic parsing; no live observer entrypoint yet."""

import base64
import io
import json
import os
import stat
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


def process_snapshot(census: bytes, pid: str, target: bytes,
                     read: Callable[[str, str, int], bytes]) -> dict[str, object]:
    """Sample one census PID through a trusted bounded reader, without attribution.

    Caller owns census acquisition, path confinement and epoch validation. The
    reader receives only this PID and fixed proc filenames, and must enforce the
    supplied cap. This consistency check cannot exclude unobserved ABA changes.
    """
    def raw(name: str, cap: int) -> bytes:
        data = read(pid, name, cap)
        if type(data) is not bytes or len(data) > cap:
            raise ObservationError("proc_read_protocol")
        return data

    if (type(census) is not bytes or type(pid) is not str
            or type(target) is not bytes or not 1 <= len(target) <= 4092
            or not target.startswith(b"/")
            or any(c in target for c in (b"\x00", b"\n", b"\r"))):
        raise ObservationError("invalid_process_request")
    if pid not in parse_procs(census):
        raise ObservationError("pid_not_in_census")
    before = parse_stat(raw("stat", 4096))
    membership = parse_membership(raw("cgroup", 4096))
    if before[0] != pid or membership != target:
        raise ObservationError("initial_identity")
    fields = parse_status(raw("status", 8192))
    cmdline = raw("cmdline", 4096)
    after = parse_stat(raw("stat", 4096))
    membership = parse_membership(raw("cgroup", 4096))
    if before != after or fields[:2] != before[:2] or membership != target:
        raise ObservationError("raced_identity")
    return {"pid": pid, "ppid": before[1], "start": before[2],
            "namespace": fields[2], "rss_kib": fields[3], "hwm_kib": fields[4],
            "cmdline": cmdline, "parent_start": None,
            "attribution": "owned_unattributed"}


def process_family_snapshot(
        census: bytes, pid: str, target: bytes,
        read: Callable[[str, str, int], bytes]
) -> tuple[dict[str, object], dict[str, object] | None, Exception | None]:
    """Nest one owned parent read inside the child's checks; never infer ancestry."""
    parent = None
    parent_pid = None
    parent_failure = None
    members = parse_procs(census)

    def nested(child_pid: str, name: str, cap: int) -> bytes:
        nonlocal parent, parent_pid, parent_failure
        data = read(child_pid, name, cap)
        if type(data) is not bytes or len(data) > cap:
            raise ObservationError("proc_read_protocol")
        if name == "stat" and parent_pid is None:
            parent_pid = parse_stat(data)[1]
        if name == "cmdline" and parent_pid in members and parent_pid != pid:
            try:
                parent = process_snapshot(census, parent_pid, target, read)
            except (OSError, ObservationError) as error:
                parent_failure = error
        return data

    try:
        child = process_snapshot(census, pid, target, nested)
    except BaseException as error:
        if parent_failure is not None:
            raise BaseExceptionGroup("family_observation_failures",
                                     [parent_failure, error])
        raise
    if parent is not None:
        child["parent_start"] = parent["start"]
    return child, parent, parent_failure


def guest_interval(origin_ns: int, start_ns: int, end_ns: int) -> tuple[str, str]:
    """Widen one guest BOOTTIME interval; caller must establish the clock domain."""
    if (any(type(n) is not int for n in (origin_ns, start_ns, end_ns))
            or not 0 <= origin_ns <= start_ns <= end_ns < 2**64):
        raise ObservationError("clock_interval")
    return (str((start_ns - origin_ns) // 1000),
            str((end_ns - origin_ns + 999) // 1000))


def next_tick(origin_ns: int, due_ns: int, finished_ns: int) -> tuple[int, int]:
    """Advance the absolute 100ms schedule; caller samples once, never catches up."""
    if (any(type(n) is not int for n in (origin_ns, due_ns, finished_ns))
            or not 0 <= origin_ns <= due_ns <= finished_ns < 2**64
            or (due_ns - origin_ns) % 100000000):
        raise ObservationError("clock_schedule")
    missed = (finished_ns - due_ns) // 100000000
    future = due_ns + (missed + 1) * 100000000
    if future >= 2**64:
        raise ObservationError("clock_schedule_overflow")
    return future, missed


def _validate_directory_pin(expected: tuple[int, int, int, int]) -> None:
    if (type(expected) is not tuple or len(expected) != 4
            or any(type(n) is not int or not 0 <= n < 2**64 for n in expected)
            or expected[3] > 0xffff or not stat.S_ISDIR(expected[3])):
        raise ObservationError("directory_pin")


def _directory_parts(path: str, expected: tuple[int, int, int, int]) -> list[str]:
    _validate_directory_pin(expected)
    if (type(path) is not str or not 1 <= len(path) <= 4096
            or not path.startswith("/") or "\x00" in path):
        raise ObservationError("directory_path")
    if len(os.fsencode(path)) > 4096:
        raise ObservationError("directory_path")
    parts = path.split("/")[1:]
    if len(parts) > 64 or any(p in ("", ".", "..") for p in parts):
        raise ObservationError("directory_path")
    return parts


def _check_directory(fd: int, expected: tuple[int, int, int, int]) -> None:
    _validate_directory_pin(expected)
    if type(fd) is not int or not 0 <= fd <= 0x7fffffff:
        raise ObservationError("directory_descriptor")
    item = os.fstat(fd)
    if (item.st_dev, item.st_ino, item.st_uid, item.st_mode) != expected:
        raise ObservationError("directory_identity")


def _raise_io_errors(errors: list[BaseException]) -> None:
    if len(errors) == 1:
        raise errors[0]
    if errors:
        raise BaseExceptionGroup("diagnostic_io_failures", errors)


def _close_fds(owned: list[int], primary: BaseException | None = None) -> None:
    errors = [] if primary is None else [primary]
    while owned:
        fd = owned.pop()
        try:
            os.close(fd)
        except BaseException as error:
            errors.append(error)
    _raise_io_errors(errors)


def open_pinned_directory(path: str, expected: tuple[int, int, int, int]) -> int:
    """Open one directory matching a caller-supplied identity; transfer its fd."""
    parts = _directory_parts(path, expected)
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    owned = []
    try:
        owned.append(os.open("/", flags))
        for part in parts:
            owned.append(os.open(part, flags, dir_fd=owned[-1]))
        _check_directory(owned[-1], expected)
    except BaseException as error:
        _close_fds(owned, error)
    result = owned.pop()
    try:
        _close_fds(owned)
    except BaseException as error:
        _close_fds([result], error)
    return result


_OBSERVATION_CAPS = {"cgroup.procs": 4096, "stat": 4096, "status": 8192,
                     "cmdline": 4096, "cgroup": 4096, "memory.events": 4096,
                     "memory.current": 64, "memory.swap.current": 64,
                     "memory.peak": 64, "memory.stat": 16384,
                     "memory.pressure": 4096, "memory.events.local": 4096,
                     "boot_id": 64}


def _close_file(fd: int, stream: BinaryIO | None,
                primary: BaseException | None = None) -> None:
    errors = [] if primary is None else [primary]
    if stream is not None:
        try:
            stream.close()
        except BaseException as error:
            errors.append(error)
    try:
        os.close(fd)
    except BaseException as error:
        errors.append(error)
    _raise_io_errors(errors)


def read_observation_file(directory: int, expected: tuple[int, int, int, int],
                          name: str) -> bytes:
    """Read a fixed observation file; never close the caller's directory fd."""
    if type(name) is not str or name not in _OBSERVATION_CAPS:
        raise ObservationError("observation_file_name")
    _check_directory(directory, expected)
    cap = _OBSERVATION_CAPS[name]
    fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK,
                 dir_fd=directory)
    stream = None
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            raise ObservationError("observation_file_type")
        stream = io.FileIO(fd, "rb", closefd=False)
        data = read_bounded(stream, cap)
        _check_directory(directory, expected)
    except BaseException as error:
        _close_file(fd, stream, error)
    _close_file(fd, stream)
    return data


def check_no_descendants(directory: int,
                         expected: tuple[int, int, int, int]) -> None:
    """Inspect at most 64 immediate entries; not an atomic absence guarantee."""
    _check_directory(directory, expected)
    entries = os.scandir(directory)
    errors = []
    try:
        for count, entry in enumerate(entries, 1):
            if count > 64:
                raise ObservationError("child_count")
            item = os.stat(entry.name, dir_fd=directory, follow_symlinks=False)
            if stat.S_ISDIR(item.st_mode) or stat.S_ISLNK(item.st_mode):
                raise ObservationError("child_type")
        _check_directory(directory, expected)
    except BaseException as error:
        errors.append(error)
    try:
        entries.close()
    except BaseException as error:
        errors.append(error)
    _raise_io_errors(errors)


def owned_process_snapshot(
        cgroup: tuple[int, tuple[int, int, int, int]],
        proc: tuple[int, tuple[int, int, int, int]],
        pid: str, target: bytes) -> dict[str, object]:
    """Read one currently listed PID; borrowed roots and epoch belong to caller."""
    return owned_process_evidence(cgroup, proc, pid, target)[0]


def owned_process_evidence(cgroup, proc, pid: str, target: bytes) -> tuple:
    """Retain raw identity bytes from the same bounded snapshot reads."""
    cfd, cpin = cgroup
    pfd, ppin = proc
    check_no_descendants(cfd, cpin)
    census = read_observation_file(cfd, cpin, "cgroup.procs")
    if type(pid) is not str or pid not in parse_procs(census):
        raise ObservationError("pid_not_in_census")
    _check_directory(pfd, ppin)
    fd = os.open(pid, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                 dir_fd=pfd)
    try:
        item = os.fstat(fd)
        identity = (item.st_dev, item.st_ino, item.st_uid, item.st_mode)
        stats, memberships = [], []

        def read(_pid: str, name: str, _cap: int) -> bytes:
            raw = read_observation_file(fd, identity, name)
            if name == "stat":
                stats.append(raw)
            if name == "cgroup":
                memberships.append(raw)
            return raw

        result = process_snapshot(census, pid, target, read)
        _check_directory(fd, identity)
        _check_directory(pfd, ppin)
        check_no_descendants(cfd, cpin)
    except BaseException as error:
        _close_fds([fd], error)
    _close_fds([fd])
    return result, stats[0], stats[1], memberships[0]


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
