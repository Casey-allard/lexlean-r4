"""One absolute guest-clock schedule; no thread, sleep, launch or compiler handle."""

import boundary
from observe import ObservationError

TICK = 100000000
SLOW = 5000000000
SYNC = 1000000000


def _elapsed(time_ns, due_ns):
    return 0 if time_ns < due_ns else 1+(time_ns-due_ns)//TICK


def _next_phase(origin, time_ns, period):
    future = origin+((time_ns-origin)//period+1)*period
    if future >= 2**64:
        raise ObservationError('cadence_overflow')
    return future


class Cadence:
    """Carry post-write misses to the next row or expose them as incomplete coverage."""

    def __init__(self, sampler):
        if type(sampler) is not boundary.SampleBoundary:
            raise ObservationError('cadence_sampler')
        self.sampler, self.origin = sampler, sampler.origin
        self.due_ns = self.slow_due_ns = self.origin
        self.sync_due_ns = _next_phase(self.origin, self.origin, SYNC)
        self.pending_misses = 0
        self.accounted_ns = self.origin
        self.failed = self.ended = self.busy = False

    def _enter(self, terminal_ok=False):
        if self.busy:
            self.failed = self.sampler.failed = self.sampler.journal.stopped = True
            raise ObservationError('cadence_reentrant')
        if not terminal_ok:
            self._active()
        self.busy = True

    def _active(self):
        if self.failed or self.ended:
            raise ObservationError('cadence_stopped')

    def _advance(self, now):
        if type(now) is not int or not self.accounted_ns <= now < 2**64:
            raise ObservationError('cadence_clock')
        extra = _elapsed(now, self.due_ns)
        future = self.due_ns+extra*TICK
        if future >= 2**64 or self.pending_misses+extra >= 2**53:
            raise ObservationError('cadence_overflow')
        self.due_ns, self.pending_misses = future, self.pending_misses+extra
        self.accounted_ns = now

    def _commit(self, result):
        self.due_ns, self.pending_misses = result['next_due'], 0
        self._advance(result['finished_ns'])
        if result['slow'] and result['started_ns'] >= self.slow_due_ns:
            self.slow_due_ns = _next_phase(self.origin, result['started_ns'], SLOW)
        if result['ack'] is not None:
            self.sync_due_ns = _next_phase(self.origin, result['finished_ns'], SYNC)
        elif result['finished_ns'] >= self.sync_due_ns:
            self._advance(self.sampler.synchronize_owned())
            self.sync_due_ns = _next_phase(self.origin, self.accounted_ns, SYNC)

    def step(self, request=None):
        self._enter()
        before = (self.due_ns, self.pending_misses, self.accounted_ns,
                  self.slow_due_ns, self.sync_due_ns)
        try:
            now = self.sampler.now()
            self._active()
            if request is not None:
                result = self.sampler.capture_boundary(
                    request, due_ns=self.due_ns, pending_misses=self.pending_misses)
            elif now >= self.due_ns:
                result = self.sampler.capture_periodic(
                    self.due_ns, self.pending_misses, self.slow_due_ns)
            else:
                return None
            self._active()
            self._commit(result)
            self._active()
            return result['ack']
        except BaseException:
            (self.due_ns, self.pending_misses, self.accounted_ns,
             self.slow_due_ns, self.sync_due_ns) = before
            self.failed = True
            raise
        finally:
            self.busy = False

    def finish_snapshot(self):
        return {'next_due_ns': self.due_ns, 'pending_misses': self.pending_misses,
                'accounted_through_ns': self.accounted_ns, 'failed': self.failed,
                'ended': self.ended, 'incomplete': self.failed or self.pending_misses > 0}

    def finish(self):
        self._enter(terminal_ok=True)
        try:
            if not self.failed and not self.ended:
                now = self.sampler.now()
                self._active()
                self._advance(now)
            self.ended = True
            return self.finish_snapshot()
        except BaseException:
            self.failed = True
            raise
        finally:
            self.busy = False
