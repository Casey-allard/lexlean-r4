"""One serial reader composition; never owns or signals the compiler process."""

import io
import os
import stat

import boundary
import cadence
import control_slots
import live_context
import observe
import time
from observe import ObservationError


def _output(directory, name, cleanup):
    observe._check_directory(*directory)
    fd = os.open(name, os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,
                 0o600, dir_fd=directory[0])
    cleanup.append(lambda: os.close(fd))
    item = os.fstat(fd)
    if (not stat.S_ISREG(item.st_mode) or stat.S_IMODE(item.st_mode) != 0o600
            or item.st_uid != os.getuid() or item.st_nlink != 1 or item.st_size != 0):
        raise ObservationError('session_output_identity')
    observe._check_directory(*directory)
    stream = io.FileIO(fd, 'wb', closefd=False)
    cleanup.append(stream.close)
    return stream


def _close_owned(cleanup, primary):
    errors = [] if primary is None else [primary]
    while cleanup:
        close = cleanup.pop()
        try:
            close()
        except BaseException as error:
            errors.append(error)
    observe._raise_io_errors(errors)


def run_owned(spec, output_directory, nonce):
    """Own one serial diagnostic's files and descriptors; no launch authority."""
    live_context._validate(spec)
    if type(output_directory) is not tuple or len(output_directory) != 2:
        raise ObservationError('session_output_directory')
    observe._directory_parts(*output_directory)
    expected = output_directory[1]
    if expected[2] != os.getuid() or stat.S_IMODE(expected[3]) != 0o700:
        raise ObservationError('session_output_owner_mode')
    boundary._hex64(nonce)
    cleanup, primary = [], None
    try:
        context = live_context.open_context(spec)
        cleanup.append(context.close)
        fd = observe.open_pinned_directory(*output_directory)
        cleanup.append(lambda: os.close(fd))
        directory = fd, expected
        slots = control_slots.OwnedControlSlots(directory, 'guest')
        journal_file = _output(directory, 'journal.jsonl', cleanup)
        terminal = _output(directory, 'guest-terminal.json', cleanup)
        journal = observe.Journal(journal_file, lambda: os.fsync(journal_file.fileno()))
        sampler = boundary.SampleBoundary(
            journal, nonce, context.identity, context.origin_ns, context.cgroup,
            context.proc, context.target, context.read_identity, context.clock_ns)
        session = ObserverSession(context, cadence.Cadence(sampler), slots,
                                  observe.Retention(), terminal, lambda: os.fsync(terminal.fileno()))
        session.run()
    except BaseException as error:
        primary = error
    _close_owned(cleanup, primary)


class ObserverSession:
    """Borrow context, outputs and control directory; their owner closes all."""

    def __init__(self, context, driver, slots, retention, terminal, terminal_sync):
        if (type(context) is not live_context.Context or type(driver) is not cadence.Cadence
                or type(slots) is not control_slots.OwnedControlSlots
                or type(retention) is not observe.Retention or not callable(terminal_sync)):
            raise ObservationError('session_configuration')
        self.context, self.driver, self.slots = context, driver, slots
        self.sampler, self.retention = driver.sampler, retention
        self.terminal, self.terminal_sync = terminal, terminal_sync
        self.busy = self.failed = self.ended = self.finalized = self.running = False
        self.last_request = None
        if terminal is self.sampler.stream:
            raise ObservationError('session_terminal_alias')
        self._bound()

    def _bound(self):
        c, s = self.context, self.sampler
        if (c.closed or c.failed or self.driver.sampler is not s
                or s.cgroup != c.cgroup or s.proc != c.proc or s.target != c.target
                or s.identity != c.identity or s.origin != c.origin_ns
                or s.read_identity != c.read_identity or s.clock != c.clock_ns
                or self.slots.name != 'ack' or self.slots.peer != 'request'):
            raise ObservationError('session_context_binding')

    def _poison(self):
        self.failed = self.driver.failed = self.sampler.failed = self.sampler.journal.stopped = True

    def _enter(self, finishing=False):
        if self.busy:
            self._poison()
            raise ObservationError('session_reentrant')
        if not finishing and (self.failed or self.ended or self.finalized):
            raise ObservationError('session_stopped')
        self.busy = True

    def _active(self):
        if self.failed or self.ended or self.finalized:
            raise ObservationError('session_stopped')
        self._bound()

    def step(self):
        self._enter()
        try:
            self._active()
            raw = self.slots.read_peer()
            self._active()
            request = None if raw == self.last_request else raw
            ack = self.driver.step(request)
            self._active()
            if ack is not None:
                self.slots.publish(ack)
                self._active()
                self.last_request = request
                if boundary.decode_ack(ack)[1] == 11:
                    self.ended = True
                    self._finish()
            return ack
        except BaseException as error:
            self._finish(error)
            raise
        finally:
            self.busy = False

    def _fields(self):
        j = self.sampler.journal
        return dict(nonce=self.sampler.nonce, binding=self.sampler.binding,
                    final_state='ended' if self.ended and not self.failed else 'stopped',
                    reason='ended' if self.ended and not self.failed else 'unknown',
                    prefix=[str(j.written_offset), str(j.complete_offset), str(j.complete_rows),
                            str(j.synced_offset), j.offset_certain],
                    journal_sync='reported' if j.synced_offset else 'not_attempted',
                    coverage=[None, None, None, None], detailed_gaps='incomplete',
                    reserved=[j.reserved[k] for k in ('samples', 'identities', 'markers', 'terminal')],
                    observer_process=['running', None, None], sdk_process=['unknown', None, None])

    def _finish(self, primary=None):
        if self.finalized:
            if primary is not None:
                raise primary
            return
        self.finalized = True
        errors = [] if primary is None else [primary]
        if primary is not None:
            self._poison()
        try:
            self.driver.finish()
        except BaseException as error:
            errors.append(error)
            self._poison()
        try:
            fields = self._fields()
            self.retention.write_terminal('guest', self.terminal, self.terminal_sync, **fields)
        except BaseException as error:
            errors.append(error)
            self._poison()
        if self.failed and not errors:
            errors.append(ObservationError('session_failed'))
        observe._raise_io_errors(errors)

    def finish(self, primary=None):
        self._enter(finishing=True)
        try:
            self._finish(primary)
        finally:
            self.busy = False

    def _pause(self):
        self._enter()
        try:
            self._active()
            now = self.context.clock_ns()
            self._active()
            delay = min(10000000, max(0, self.driver.due_ns-now))
            time.sleep(delay/1000000000)
            self._active()
        finally:
            self.busy = False

    def run(self):
        if self.running:
            self._poison()
            raise ObservationError('session_reentrant_run')
        self._active()
        self.running = True
        try:
            while not self.ended:
                self.step()
                if not self.ended:
                    self._pause()
        except BaseException as error:
            self.finish(error)
            raise
        finally:
            self.running = False
