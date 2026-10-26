"""Owned guest identity preflight; no launch entry point or Docker authority."""

from dataclasses import dataclass
import datetime
import os
import time
import uuid

import boundary
import observe
from observe import ObservationError


@dataclass(frozen=True)
class ContextSpec:
    docker: tuple
    boot: str
    init_start: str
    cgroup: tuple
    proc: tuple
    boot_directory: tuple
    target: bytes


def _decimal(value, positive=False):
    if type(value) is not str or not value.isascii():
        raise ObservationError('context_decimal')
    observe.u64(value.encode('ascii'))
    if positive and value == '0':
        raise ObservationError('context_pid')


def _validate(spec):
    if type(spec) is not ContextSpec or type(spec.docker) is not tuple or len(spec.docker) != 4:
        raise ObservationError('context_spec')
    cid, image, started, pid = spec.docker
    boundary._hex64(cid)
    if type(image) is not str or not image.startswith('sha256:'):
        raise ObservationError('context_image')
    boundary._hex64(image[7:])
    if (type(started) is not str or not 1 <= len(started) <= 64
            or not started.isascii() or 'T' not in started or not started.endswith('Z')):
        raise ObservationError('context_started')
    try:
        datetime.datetime.fromisoformat(started)
        if type(spec.boot) is not str or str(uuid.UUID(spec.boot)) != spec.boot:
            raise ValueError('boot')
    except (ValueError, TypeError, AttributeError) as error:
        raise ObservationError('context_epoch') from error
    _decimal(pid, positive=True)
    _decimal(spec.init_start)
    if type(spec.target) is not bytes:
        raise ObservationError('context_target')
    boundary._path(spec.target.decode('utf8'))
    for pair in (spec.cgroup, spec.proc, spec.boot_directory):
        if type(pair) is not tuple or len(pair) != 2:
            raise ObservationError('context_directory')
        observe._directory_parts(*pair)


def _pin(fd):
    item = os.fstat(fd)
    return (item.st_dev, item.st_ino, item.st_uid, item.st_mode)


class Context:
    """Own roots and two process directories; borrowers must not close them."""

    def __init__(self, spec):
        self.spec, self.target = spec, spec.target
        self.owned, self.processes = [], {}
        self.failed = self.closed = self.busy = False
        self.last_clock = None
        self.self_pid = str(os.getpid())

    def _acquire(self, pair):
        fd = observe.open_pinned_directory(*pair)
        self.owned.append(fd)
        return fd, pair[1]

    def _open_process(self, pid):
        observe._check_directory(*self.proc)
        fd = os.open(pid, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                     dir_fd=self.proc[0])
        self.owned.append(fd)
        result = (fd, _pin(fd))
        observe._check_directory(*result)
        return result

    def _enter(self):
        if self.busy:
            self.failed = True
            raise ObservationError('context_reentrant')
        if self.failed or self.closed:
            raise ObservationError('context_stopped')
        self.busy = True

    def _active(self):
        if self.failed or self.closed or str(os.getpid()) != self.self_pid:
            raise ObservationError('context_process_changed')

    def _boot(self):
        raw = observe.read_observation_file(*self.boot_directory, 'boot_id')
        if raw != (self.spec.boot+'\n').encode('ascii'):
            raise ObservationError('context_boot')
        return self.spec.boot

    def _snapshot(self, pid):
        pair = self.processes[pid]
        read = lambda name: observe.read_observation_file(*pair, name)
        before = observe.parse_stat(read('stat'))
        member = observe.parse_membership(read('cgroup'))
        fields = observe.parse_status(read('status'))
        after = observe.parse_stat(read('stat'))
        final_member = observe.parse_membership(read('cgroup'))
        if before != after or before[0] != pid or fields[:2] != before[:2] or member != final_member:
            raise ObservationError('context_process_race')
        return before[2], member

    def _names(self):
        temporary = []
        try:
            temporary.append(observe.open_pinned_directory(*self.spec.cgroup))
            for pid, (_, pin) in self.processes.items():
                observe._check_directory(*self.proc)
                fd = os.open(pid, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                             dir_fd=self.proc[0])
                temporary.append(fd)
                observe._check_directory(fd, pin)
        except BaseException as error:
            observe._close_fds(temporary, error)
        observe._close_fds(temporary)

    def _identity(self):
        self._active()
        for pair in (self.cgroup, self.proc, self.boot_directory):
            observe._check_directory(*pair)
        boot = self._boot()
        census = self._census()
        self._names()
        init_pid = self.spec.docker[3]
        own, init = self._snapshot(self.self_pid), self._snapshot(init_pid)
        if init != (self.spec.init_start, self.target):
            raise ObservationError('context_init_identity')
        current = observe.parse_procs(observe.read_observation_file(*self.cgroup, 'cgroup.procs'))
        if set(current) != set(census):
            raise ObservationError('context_census_changed')
        observe.check_no_descendants(*self.cgroup)
        self._boot()
        self._active()
        pin = self.cgroup[1]
        result = (*self.spec.docker[:3], boot,
                  (self.target.decode('utf8'), pin[0], pin[1], pin[3]),
                  (self.self_pid, own[0], own[1].decode('utf8')))
        return boundary._sample_identity(result, self.target, pin)

    def _census(self):
        observe.check_no_descendants(*self.cgroup)
        census = observe.parse_procs(observe.read_observation_file(*self.cgroup, 'cgroup.procs'))
        if self.spec.docker[3] == self.self_pid or self.spec.docker[3] not in census:
            raise ObservationError('context_init_census')
        return census

    def read_identity(self):
        self._enter()
        try:
            result = self._identity()
            if result != self.identity:
                raise ObservationError('context_identity_changed')
            return result
        except BaseException:
            self.failed = True
            raise
        finally:
            self.busy = False

    def _clock(self):
        self._active()
        value = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
        self._active()
        if (type(value) is not int or not 0 <= value < 2**64
                or (self.last_clock is not None and value < self.last_clock)):
            raise ObservationError('context_clock')
        self.last_clock = value
        return value

    def clock_ns(self):
        self._enter()
        try:
            return self._clock()
        except BaseException:
            self.failed = True
            raise
        finally:
            self.busy = False

    def close(self, primary=None):
        self.closed = True
        observe._close_fds(self.owned, primary)


def open_context(spec):
    _validate(spec)
    context = Context(spec)
    try:
        context.cgroup = context._acquire(spec.cgroup)
        context.proc = context._acquire(spec.proc)
        context.boot_directory = context._acquire(spec.boot_directory)
        if spec.docker[3] == context.self_pid:
            raise ObservationError('context_self_is_init')
        context._boot()
        context._census()
        for pid in (context.self_pid, spec.docker[3]):
            context.processes[pid] = context._open_process(pid)
        context.identity = context._identity()
        context.origin_ns = context.clock_ns()
        context.clock_ns()
        return context
    except BaseException as error:
        context.failed = True
        context.close(error)
