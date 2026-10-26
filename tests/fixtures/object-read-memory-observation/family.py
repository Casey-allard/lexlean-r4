"""One bounded owned parent observation; no invocation or allocation attribution."""

import base64
import errno
import os

import observe
from observe import ObservationError


class _OwnedFamily:
    """Borrow roots; own at most two PID directories and twelve bounded reads."""

    def __init__(self, proc: tuple, members: list[str], child: str) -> None:
        self.proc, self.members, self.child = proc, members, child
        self.owned, self.pins, self.raw = [], {}, []

    def read(self, pid: str, name: str, cap: int) -> bytes:
        if (pid not in self.members or name not in ('stat', 'cgroup', 'status', 'cmdline')
                or cap != observe._OBSERVATION_CAPS[name] or len(self.raw) >= 12):
            raise ObservationError('family_read_request')
        if pid not in self.pins:
            if len(self.owned) >= 2:
                raise ObservationError('family_directory_limit')
            observe._check_directory(*self.proc)
            fd = os.open(pid, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                         dir_fd=self.proc[0])
            self.owned.append(fd)
            item = os.fstat(fd)
            pin = (item.st_dev, item.st_ino, item.st_uid, item.st_mode)
            self.pins[pid] = (fd, pin)
        options = {} if pid == self.child else {'cleanup_fatal': True}
        data = observe.read_observation_file(*self.pins[pid], name, **options)
        self.raw.append((pid, name, data))
        return data

    def finish(self, primary: BaseException | None, parent_error: Exception | None) -> None:
        try:
            observe._close_fds(self.owned, primary)
        except BaseException as error:
            if primary is None and parent_error is not None:
                raise BaseExceptionGroup('family_parent_and_cleanup', [parent_error, error])
            raise


def owned_family_evidence(cgroup: tuple, proc: tuple, pid: str, target: bytes) -> tuple:
    """Return child, parent, original ordinary parent error and exact read transcript.

    Parent reads are nested by process_family_snapshot, never taken from another
    census iteration. Root descriptors remain borrowed. No unobserved ABA claim.
    """
    observe.check_no_descendants(*cgroup)
    census = observe.read_observation_file(*cgroup, 'cgroup.procs')
    members = observe.parse_procs(census)
    if type(pid) is not str or pid not in members:
        raise ObservationError('pid_not_in_census')
    observe._check_directory(*proc)
    reader = _OwnedFamily(proc, members, pid)
    parent_error = None
    try:
        child, parent, parent_error = observe.process_family_snapshot(
            census, pid, target, reader.read)
        for descriptor in reader.pins.values():
            observe._check_directory(*descriptor)
        observe._check_directory(*proc)
        observe.check_no_descendants(*cgroup)
    except BaseException as error:
        if parent_error is not None:
            error = BaseExceptionGroup('family_parent_and_guard', [parent_error, error])
        reader.finish(error, None)
    reader.finish(None, parent_error)
    return child, parent, parent_error, tuple(reader.raw)


def identity_evidence(result: tuple) -> tuple:
    """Encode exact parent reads and bounded failure metadata, never error text."""
    child, parent, failure, transcript = result
    own = [(name, data) for pid, name, data in transcript if pid == child['pid']]
    encode = lambda data: base64.b64encode(data).decode('ascii')
    parent_raw = [[name, encode(data)] for pid, name, data in transcript
                  if pid != child['pid']]
    if parent is not None:
        evidence = ['observed', parent_raw]
    elif failure is None:
        evidence = ['not_observed', []]
    else:
        number = failure.errno if type(failure) in (
            OSError, FileNotFoundError, PermissionError) else None
        if type(number) is not int or not 0 <= number <= 65535:
            number = None
        reason = ('vanished' if number == errno.ENOENT else 'denied'
                  if number == errno.EACCES else 'io_failure'
                  if isinstance(failure, OSError) else 'parse_error')
        evidence = ['unavailable', reason, number, parent_raw]
    return child, own[0][1], own[4][1], own[1][1], evidence
