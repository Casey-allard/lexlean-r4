"""Test-only finite owner/reader bridge; no production import or guest authority."""
import json
import os
import pathlib
import signal
import sys
import tempfile
import types
from unittest import mock

import observer_entry
import observer_owner as owner
import observe
import test_live_context
from test_identity import stat, status
from test_observer_entry import EntryTests
from test_observer_owner import finite_worker
from test_owned_io import pin
from test_sample import SampleTests


def run(base):
    def action():
        # Node owns the complete temporary root until both actual waits finish.
        def temporary(**unused):
            return types.SimpleNamespace(name=tempfile.mkdtemp(dir=base), cleanup=lambda: None)
        with mock.patch.object(test_live_context.tempfile, 'TemporaryDirectory', temporary):
            fixture = EntryTests(); fixture.setUp()
        files = {'memory.events': SampleTests.events(), 'memory.events.local': SampleTests.events(),
                 'memory.current': b'123\n', 'memory.swap.current': b'0\n',
                 'memory.peak': b'456\n', 'memory.stat': b'anon 100\nfile 23\n',
                 'memory.pressure': b'some avg10=0.00 avg60=0.00 avg300=0.00 total=0\n'}
        for name, raw in files.items(): (fixture.fixture.cg/name).write_bytes(raw)
        (fixture.fixture.proc/'41'/'cmdline').write_bytes(b'fixture-init\0')
        original = owner._capture_identity
        def capture(spec, pid):
            process = fixture.fixture.proc/str(pid); process.mkdir()
            (process/'stat').write_bytes(stat(str(pid), str(os.getpid()), '222'))
            (process/'status').write_bytes(status(str(pid), str(os.getpid())))
            (process/'cgroup').write_bytes(b'0::/observer\n')
            result = original(spec, pid)
            identity = [*spec.docker[:3], spec.boot,
                        [spec.target.decode(), *spec.cgroup[1][:2], spec.cgroup[1][3]],
                        [str(pid), '222', '/observer']]
            print(json.dumps({'directory': str(fixture.output),
                              'pin': [str(n) for n in pin(fixture.output)],
                              'nonce': fixture.data['nonce'], 'identity': identity,
                              'contextDigest': owner._context_digest(spec)}), flush=True)
            return result
        def finite_child(read, write, arguments):
            signal.alarm(5)
            try:
                os.close(write)
                null = os.open(os.devnull, os.O_RDWR)
                for fd in (0, 1, 2): os.dup2(null, fd)
                os.closerange(3, read); os.closerange(read+1, os.sysconf('SC_OPEN_MAX'))
                if os.read(read, 1) != b'G': os._exit(125)
                os.close(read)
                os._exit(observer_entry.cli(fixture.argv))
            except BaseException:
                os._exit(127)
        try:
            with mock.patch.object(owner, '_capture_identity', capture), \
                 mock.patch.object(owner, '_child_entry', finite_child), \
                 mock.patch.object(owner, '_exec_argv', return_value=('/unused/fixed-fixture',)):
                owner.main(fixture.argv, 0)
            # Independent Python receipt admission; child status is in oe, not stdout.
            observe.decode_owner((fixture.output/'observer-launch.json').read_bytes())
            observe.decode_owner((fixture.output/'observer-exit.json').read_bytes())
        finally:
            fixture.doCleanups()
    finite_worker(action)


if __name__ == '__main__':
    if len(sys.argv) != 2: raise SystemExit(64)
    base = pathlib.Path(sys.argv[1])
    if not base.is_dir() or base.resolve() != base: raise SystemExit(65)
    run(str(base))
