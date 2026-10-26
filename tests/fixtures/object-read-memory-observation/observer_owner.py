"""Fixed observer child custody; no compiler handle or arbitrary command API."""

import errno
import hashlib
import os
import select
import signal
import sys
import threading
import time

import live_context
import observe
import observer_entry
import observer_session
from observe import ObservationError


_SOURCE_ROOT = '/Users/casey.allard/lexlean-r4-worktrees/model-port/tests/fixtures/object-read-memory-observation'
_MODULES = (
    ('observer_entry', 6189, 'ab6f4c8ef96e297dce0b8a3acd24ddae6d3833cfc2ef393347d583e5340b5d1e'),
    ('observer_session', 8169, 'eb84e076c2809003406bdc9665aa12a7d6482c335b4c67914f3e9b0736b669c5'),
    ('live_context', 7977, 'df74150b428a0ad72f3ec4ac1a3638dacd786fc1e4220adc9c69bcfd77508a55'),
    ('boundary', 26535, 'b457b8e290158a45766ddec34a2518e8215a1c9fed35e31b1353bd7e5b76dddf'),
    ('family', 4288, '7a5f1135d1cea44c1fb02dd721a8eff2422708442fd69c272c1e4aa685a072b0'),
    ('observe', 32926, '341fdc5f4def127af11954b2cbfb0251932923b06eb6e227fc9a132553eb0e13'),
    ('cadence', 4264, 'aefa62d3c242f53013299ab362b2895c9c9e7ed6fc2e72c09307952542823c46'),
    ('control_slots', 5571, 'dfa6ef3ea8c12c60c5b37e48bb944f819bfa387afe2302c8b8ca2cf3cac1d79a'),
)
_BOOTSTRAP_CHECKS = 'ROOT = '+repr(_SOURCE_ROOT)+'\nMODULES = '+repr(_MODULES)+r'''
import sys, os, stat, io, json, base64, errno, datetime, hashlib, uuid, time
import dataclasses, typing, importlib.util
def stable(item):
    return (item.st_dev, item.st_ino, item.st_uid, item.st_mode, item.st_nlink,
            item.st_size, item.st_mtime_ns, item.st_ctime_ns)
def root_fd():
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        for part in ROOT.split('/')[1:]:
            new = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                          dir_fd=fd)
            os.close(fd); fd = new
        info = os.fstat(fd)
        if info.st_uid != os.getuid() or info.st_mode & 0o022:
            raise ValueError('source custody')
        return fd
    except BaseException:
        os.close(fd)
        raise
def verify(fd):
    for name, length, digest in MODULES:
        path = ROOT+'/'+name+'.py'
        for cache in (importlib.util.cache_from_source(path), ROOT+'/'+name+'.pyc'):
            if os.path.lexists(cache): raise ValueError('source cache')
        src = os.open(name+'.py', os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                      dir_fd=fd)
        try:
            before = os.fstat(src)
            if (not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid()
                    or before.st_mode & 0o022 or before.st_nlink != 1 or before.st_size != length):
                raise ValueError('source identity')
            raw = os.read(src, length + 1)
            if (len(raw) != length or hashlib.sha256(raw).hexdigest() != digest
                    or stable(os.fstat(src)) != stable(before)
                    or stable(os.stat(name+'.py', dir_fd=fd, follow_symlinks=False)) != stable(before)):
                raise ValueError('source bytes')
        finally:
            os.close(src)
'''
_BOOTSTRAP = _BOOTSTRAP_CHECKS + r'''
try:
    source = root_fd()
    try:
        initial = stable(os.fstat(source))
        verify(source)
        sys.path.insert(0, ROOT)
        import observer_entry
        names = {row[0] for row in MODULES}
        for name in names:
            if getattr(sys.modules.get(name), '__file__', None) != ROOT+'/'+name+'.py':
                raise ValueError('module path')
        for name, module in tuple(sys.modules.items()):
            path = getattr(module, '__file__', None)
            if isinstance(path, str) and path.startswith(ROOT+'/') and name not in names:
                raise ValueError('extra dependency')
        verify(source)
        again = root_fd()
        try:
            if stable(os.fstat(again)) != initial: raise ValueError('source replacement')
        finally:
            os.close(again)
    finally:
        os.close(source)
except Exception:
    sys.stderr.write('observer-owner: bootstrap\n')
    sys.exit(74)
sys.exit(observer_entry.cli(sys.argv[1:]))
'''


def _source_preflight():
    # Execute only the fixed verifier definitions, shared verbatim with -I child.
    namespace = {}
    exec(_BOOTSTRAP_CHECKS, namespace)
    fd = namespace['root_fd']()
    try:
        namespace['verify'](fd)
        for name, _, _ in _MODULES:
            if getattr(sys.modules.get(name), '__file__', None) != _SOURCE_ROOT+'/'+name+'.py':
                raise ObservationError('owner_import_path')
    finally:
        os.close(fd)


def _context_digest(spec):
    return hashlib.sha256(observe._encoded(
        [list(spec.docker), spec.boot, spec.init_start, spec.cgroup, spec.proc,
         spec.boot_directory, spec.target.decode('utf8')], 16384)).hexdigest()


def _capture_identity(spec, pid):
    """Linux proc identity from pinned roots, independent of child self-report."""
    context = live_context.open_context(spec)
    primary, result = None, None
    try:
        key = str(pid)
        context.processes[key] = context._open_process(key)
        parent = context.read_identity()[5]
        child = context._snapshot(key)
        actual = observe.parse_stat(observe.read_observation_file(
            *context.processes[key], 'stat'))
        if (actual != (key, str(os.getpid()), child[0]) or child[1] == spec.target
                or child[1].startswith(spec.target+b'/')):
            raise ObservationError('owner_child_identity')
        context._names()
        if context._snapshot(key) != child or context.read_identity()[5] != parent:
            raise ObservationError('owner_identity_changed')
        result = [parent[0], parent[1]], [key, child[0]], spec.boot
    except BaseException as error:
        primary = error
    context.close(primary)
    return result


def main(arguments, host_input=0):
    """Own fixed receipts and one fork; host_input is borrowed and never closed."""
    spec, output, nonce = observer_entry.load_config(arguments)
    _preconditions()
    _source_preflight()
    digest, retention = _context_digest(spec), observe.Retention()
    cleanup, errors, end, start = [], [], None, None
    child = _OwnedChild()
    try:
        fd = observe.open_pinned_directory(*output)
        cleanup.append(lambda: os.close(fd))
        directory = fd, output[1]
        launch = observer_session._output(directory, 'observer-launch.json', cleanup)
        end = observer_session._output(directory, 'observer-exit.json', cleanup)
        child.start(arguments)
        if child.poll():
            raise ObservationError('owner_exited_before_capture')
        parent, identity, boot = _capture_identity(spec, child.pid)
        start = identity[1]
        if child.poll():
            raise ObservationError('owner_exited_before_go')
        retention.write_owner('launch', observe.encode_owner_launch(
            nonce, digest, parent, identity, boot), launch, lambda: os.fsync(launch.fileno()))
        child.release()
        child.supervise(host_input)
    except BaseException as error:
        errors.append(error)
    try:
        child.stop('owner_error' if errors else 'host_eof')
    except BaseException as error:
        errors.append(error)
    try:
        child.close_go()
    except BaseException as error:
        errors.append(error)
    if end is not None:
        try:
            reason = child.reason
            if child.process[0] == 'unknown' and reason == 'natural': reason = 'owner_error'
            retention.write_owner('exit', observe.encode_owner_exit(
                nonce, digest, None if child.pid is None else str(child.pid), start,
                child.process, reason), end, lambda: os.fsync(end.fileno()))
        except BaseException as error:
            errors.append(error)
    try:
        observer_session._close_owned(cleanup, None)
    except BaseException as error:
        errors.append(error)
    observe._raise_io_errors(errors)
    return child.process


def cli(arguments):
    try:
        return 0 if main(arguments) == ['exited', 0, None] else 70
    except observer_entry.EntryError as error:
        code = error.code
    except Exception:
        code = 70
    sys.stderr.write('observer-owner: failed\n')
    return code

def _preconditions():
    # Sole reaper is a caller obligation; reject detectable violations.
    if signal.getsignal(signal.SIGCHLD) != signal.SIG_DFL or threading.active_count() != 1:
        raise ObservationError('owner_reaper')


def _exec_argv(arguments):
    return ('/usr/bin/python3.12', '-I', '-B', '-c', _BOOTSTRAP, *arguments)


def _child_entry(read, write, argv):
    try:
        os.close(write)
        null = os.open(os.devnull, os.O_RDWR)
        for fd in (0, 1, 2):
            os.dup2(null, fd)
        # read is an anonymous, non-inheritable GO descriptor above stdio.
        os.closerange(3, read)
        os.closerange(read + 1, os.sysconf('SC_OPEN_MAX'))
        if os.read(read, 1) != b'G':
            os._exit(125)
        os.close(read)
        os.execve(argv[0], argv, {})
    except BaseException:
        os._exit(127)


class _OwnedChild:
    """Only start() can acquire a signalable handle; final wait destroys it."""

    def __init__(self):
        self.state, self.pid, self.go = 'NEW', None, None
        self.process, self.reason = ['not_run', None, None], 'not_started'
        self.clock_failed = False

    def start(self, arguments):
        if self.state != 'NEW':
            raise ObservationError('owner_start_once')
        self.state = 'PREPARING'
        _preconditions()
        argv = _exec_argv(arguments)
        read, write = os.pipe()
        if min(read, write) < 3:
            os.close(read); os.close(write)
            raise ObservationError('owner_stdio_closed')
        try:
            pid = os.fork()
        except BaseException as error:
            self.reason = 'fork_failed'
            observe._close_fds([read, write], error)
        if pid == 0:
            _child_entry(read, write, argv)
            os._exit(127)
        self.pid, self.go, self.state = pid, write, 'OWNED'
        self.process, self.reason = ['unknown', None, None], 'natural'
        os.close(read)

    def close_go(self):
        if self.go is not None:
            fd, self.go = self.go, None
            os.close(fd)

    def release(self):
        if self.state != 'OWNED' or self.go is None:
            raise ObservationError('owner_go')
        errors = []
        try:
            if os.write(self.go, b'G') != 1:
                raise ObservationError('owner_go_write')
        except BaseException as error:
            errors.append(error)
        try:
            self.close_go()
        except BaseException as error:
            errors.append(error)
        observe._raise_io_errors(errors)

    def poll(self):
        if self.state != 'OWNED':
            return True
        previous, outcome = self.reason, 'uncertain'
        # waitpid may consume the child before Python delivers an exception.
        # No signal authority exists during the syscall-through-publication span.
        self.state = 'WAITING'
        try:
            try:
                pid, status = os.waitpid(self.pid, os.WNOHANG)
            except InterruptedError as error:
                if error.errno != errno.EINTR:
                    raise
                outcome = 'nonconsuming'
                self.state = 'OWNED'
                return False
            except OSError as error:
                if error.errno != errno.ECHILD:
                    raise
                self.state, self.reason = 'LOST', 'lost_custody'
                return True
            if type(pid) is not int or type(status) is not int:
                raise ObservationError('owner_wait_result')
            if pid == 0:
                outcome = 'nonconsuming'
                self.state = 'OWNED'
                return False
            if pid != self.pid:
                raise ObservationError('owner_foreign_wait')
            if os.WIFEXITED(status):
                outcome, self.state = 'reaped', 'REAPED'
                self.process = ['exited', os.WEXITSTATUS(status), None]
            elif os.WIFSIGNALED(status):
                outcome, self.state = 'reaped', 'REAPED'
                self.process = ['signaled', None, os.WTERMSIG(status)]
            elif os.WIFSTOPPED(status) or os.WIFCONTINUED(status):
                outcome = 'nonconsuming'
                self.state = 'OWNED'
                return False
            else:
                raise ObservationError('owner_wait_status')
            return True
        except BaseException:
            if outcome == 'nonconsuming':
                self.state, self.reason = 'OWNED', previous
            else:
                self.state = 'REAPED' if outcome == 'reaped' else 'LOST'
                self.process, self.reason = ['unknown', None, None], 'lost_custody'
            raise

    def wait_until(self, deadline):
        while True:
            try:
                now = time.monotonic()
            except BaseException:
                self.clock_failed = True
                raise
            if now >= deadline:
                return self.poll()
            if self.poll():
                return True
            time.sleep(min(.005, deadline-now))

    def stop(self, reason):
        errors, started, timed_out = [], None, False
        if self.state == 'OWNED' and not self.clock_failed:
            try:
                started = time.monotonic()
            except BaseException as error:
                self.clock_failed = True
                errors.append(error)
        try:
            if self.state == 'OWNED' and not self.poll():
                self.reason = reason
        except BaseException as error:
            errors.append(error)
        try:
            self.close_go()
        except BaseException as error:
            errors.append(error)
        if self.state == 'OWNED':
            for sig, offset in ((signal.SIGTERM, 2), (signal.SIGKILL, 4)):
                if self.state != 'OWNED':
                    break
                try:
                    os.kill(self.pid, sig)
                except ProcessLookupError:
                    pass  # ESRCH is not exit evidence; an exact wait is required.
                except BaseException as error:
                    errors.append(error)
                finished = False
                while self.state == 'OWNED':
                    fallback = self.clock_failed
                    try:
                        if fallback:
                            finished = self.poll()  # One no-clock attempt.
                        else:
                            finished = self.wait_until(started + offset)
                        timed_out = sig == signal.SIGKILL and not fallback and not finished
                        break
                    except BaseException as error:
                        errors.append(error)
                        # Only final polling continues: never resend a signal,
                        # renew a deadline, or repeat the no-clock fallback.
                        if sig != signal.SIGKILL or fallback:
                            break
                if finished:
                    break
            if self.state == 'OWNED':
                self.state = 'LOST'
                self.reason = 'reap_timeout' if timed_out else 'owner_error'
        observe._raise_io_errors(errors)

    def supervise(self, host_input):
        while not self.poll():
            ready, _, _ = select.select([host_input], [], [], .01)
            if ready:
                reason = 'host_eof' if os.read(host_input, 1) == b'' else 'owner_error'
                self.stop(reason)
                return


if __name__ == '__main__':
    sys.exit(cli(sys.argv[1:]))
