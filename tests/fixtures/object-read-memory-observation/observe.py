"""One-off admitted-range diagnostic parsing; no live observer entrypoint yet."""

from typing import BinaryIO


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
