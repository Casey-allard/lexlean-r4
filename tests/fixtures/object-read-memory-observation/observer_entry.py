"""Bounded guest configuration adapter; no compiler or launch authority."""

import io
import json
import os
import stat
import sys

import boundary
import live_context
import observe
import observer_session


class EntryError(ValueError):
    def __init__(self, code: int) -> None:
        self.code = code
        super().__init__('observer_entry_'+str(code))


def _keys(value, names):
    if type(value) is not dict or set(value) != set(names):
        raise EntryError(65)


def _text(value):
    if type(value) is not str or any(c in value for c in '\x00\r\n'):
        raise EntryError(65)
    value.encode('utf8')
    return value


def _decimal(value):
    return int(observe.u64(_text(value).encode('ascii')))


def _directory(value):
    if type(value) is not list or len(value) != 2:
        raise EntryError(65)
    path, wire = value
    if type(wire) is not list or len(wire) != 4:
        raise EntryError(65)
    pair = (_text(path), tuple(_decimal(n) for n in wire))
    observe._directory_parts(*pair)
    return pair


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result: raise EntryError(65)
        result[key] = value
    return result


def _no_number(value):
    raise EntryError(65)


def decode_config(raw: bytes) -> tuple[live_context.ContextSpec, tuple, str]:
    """Decode assertions only; never acquire roots or grant launch authority."""
    if type(raw) is not bytes or not 1 <= len(raw) <= 16384:
        raise EntryError(65)
    try:
        data = json.loads(raw.decode('utf8'), object_pairs_hook=_object,
                          parse_int=_no_number, parse_float=_no_number,
                          parse_constant=_no_number)
        _keys(data, ('schema', 'nonce', 'context', 'output_directory'))
        if data['schema'] != 'lexlean-r4/observer-config/1': raise EntryError(65)
        context = data['context']
        _keys(context, ('docker', 'boot', 'init_start', 'cgroup', 'proc',
                        'boot_directory', 'target'))
        if type(context['docker']) is not list or len(context['docker']) != 4:
            raise EntryError(65)
        spec = live_context.ContextSpec(
            tuple(_text(x) for x in context['docker']), _text(context['boot']),
            _text(context['init_start']), _directory(context['cgroup']),
            _directory(context['proc']), _directory(context['boot_directory']),
            _text(context['target']).encode('utf8'))
        output = _directory(data['output_directory'])
        if stat.S_IMODE(output[1][3]) != 0o700: raise EntryError(65)
        live_context._validate(spec)
        boundary._hex64(data['nonce'])
        return spec, output, data['nonce']
    except (ValueError, TypeError, UnicodeError, RecursionError) as error:
        if type(error) is EntryError: raise
        raise EntryError(65) from error


def _arguments(argv):
    if (type(argv) is not list or len(argv) != 7
            or any(type(x) is not str for x in argv)
            or argv[0] != '--config-directory' or argv[2] != '--parent-pin'):
        raise EntryError(64)
    try:
        pair = _directory([argv[1], argv[3:]])
        if pair[1][2] != os.getuid() or stat.S_IMODE(pair[1][3]) != 0o700:
            raise EntryError(64)
        return pair
    except (ValueError, TypeError, UnicodeError) as error:
        if type(error) is EntryError and error.code == 64: raise
        raise EntryError(64) from error


def _identity(item):
    return (item.st_dev, item.st_ino, item.st_uid, item.st_mode, item.st_nlink,
            item.st_size, item.st_mtime_ns, item.st_ctime_ns)


def _file(fd):
    item = os.fstat(fd)
    if (not stat.S_ISREG(item.st_mode) or stat.S_IMODE(item.st_mode) != 0o600
            or item.st_uid != os.getuid() or item.st_nlink != 1
            or not 1 <= item.st_size <= 16384):
        raise EntryError(74)
    return _identity(item)


def _acquire(parent, cleanup):
    root = observe.open_pinned_directory(*parent)
    cleanup.append(lambda: os.close(root))
    fd = os.open('observer-config.json', os.O_RDONLY | os.O_NOFOLLOW |
                 os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=root)
    cleanup.append(lambda: os.close(fd))
    before = _file(fd)
    observe._check_directory(root, parent[1])
    stream = io.FileIO(fd, 'rb', closefd=False)
    cleanup.append(stream.close)
    raw = observe.read_bounded(stream, 16384)
    if len(raw) != before[5] or _file(fd) != before:
        raise EntryError(74)
    observe._check_directory(root, parent[1])
    again = observe.open_pinned_directory(*parent)
    cleanup.append(lambda: os.close(again))
    named = os.stat('observer-config.json', dir_fd=again, follow_symlinks=False)
    if _identity(named) != before: raise EntryError(74)
    return raw


def load_config(argv: list[str]) -> tuple[live_context.ContextSpec, tuple, str]:
    """Close every config handle before returning consumer arguments."""
    parent = _arguments(argv)
    cleanup, primary, decoded = [], None, None
    try:
        try:
            raw = _acquire(parent, cleanup)
        except (OSError, observe.ObservationError, EntryError, ExceptionGroup) as error:
            raise EntryError(74) from error
        decoded = decode_config(raw)
        if decoded[1][1][2] != os.getuid(): raise EntryError(65)
    except BaseException as error:
        primary = error
    try:
        observer_session._close_owned(cleanup, primary)
    except Exception as error:
        if error is primary: raise
        raise EntryError(74) from error
    return decoded


def main(argv: list[str]) -> int:
    spec, output, nonce = load_config(argv)
    observer_session.run_owned(spec, output, nonce)
    return 0


def cli(argv: list[str]) -> int:
    try:
        return main(argv)
    except EntryError as error:
        code = error.code
        category = 'config-io-compound' if isinstance(error.__cause__, ExceptionGroup) else {
            64: 'arguments', 65: 'configuration', 74: 'config-io'}[code]
    except Exception:
        code, category = 70, 'reader-or-internal'
    sys.stderr.write('observer-entry: '+category+'\n')
    return code


if __name__ == '__main__':
    sys.exit(cli(sys.argv[1:]))
