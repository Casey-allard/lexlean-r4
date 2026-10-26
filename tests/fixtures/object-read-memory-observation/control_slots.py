"""Owned fixed request/ACK files; atomic visibility, not power-loss durability."""

import errno
import os
import stat

import boundary
import observe
from observe import ObservationError


def _close(fd, primary=None):
    try:
        os.close(fd)
    except BaseException as cleanup:
        if primary is not None:
            raise BaseExceptionGroup('control_operation_and_close', [primary, cleanup])
        raise
    if primary is not None:
        raise primary


def _pin(fd):
    value = os.fstat(fd)
    if (not stat.S_ISREG(value.st_mode) or stat.S_IMODE(value.st_mode) != 0o600
            or value.st_uid != os.getuid() or value.st_nlink != 1
            or not 0 <= value.st_size <= 512):
        raise ObservationError('control_file_owner_type_size')
    return value.st_dev, value.st_ino, value.st_uid, value.st_mode, value.st_size


def _read(fd):
    raw = bytearray()
    while len(raw) <= 512:
        chunk = os.read(fd, 513-len(raw))
        if type(chunk) is not bytes or len(chunk) > 513-len(raw):
            raise ObservationError('control_read_protocol')
        if not chunk:
            return bytes(raw)
        raw.extend(chunk)
    raise ObservationError('control_file_oversize')


class OwnedControlSlots:
    """Borrow one pinned fresh directory; own only one fixed publication role.

    Journal synchronization and nonce/sequence admission belong to the caller.
    A failed attempt is terminal; its bounded temporary file is never retried.
    """

    def __init__(self, directory: tuple, role: str):
        boundary._root_pair(directory)
        if type(role) is not str or role not in ('host', 'guest'):
            raise ObservationError('control_role')
        self.directory = directory
        self.name, self.peer = ('request', 'ack') if role == 'host' else ('ack', 'request')
        self.decode = boundary.decode_request if role == 'host' else boundary.decode_ack
        self.decode_peer = boundary.decode_ack if role == 'host' else boundary.decode_request
        self.previous, self.failed = None, False
        self._guard()

    def _guard(self):
        if self.failed:
            raise ObservationError('control_owner_stopped')
        observe._check_directory(*self.directory)
        pin = self.directory[1]
        if pin[2] != os.getuid() or stat.S_IMODE(pin[3]) != 0o700:
            raise ObservationError('control_directory_owner_mode')

    def _read_file(self, name, missing_ok=False):
        self._guard()
        try:
            fd = os.open(name, os.O_RDONLY|os.O_NOFOLLOW|os.O_CLOEXEC|os.O_NONBLOCK,
                         dir_fd=self.directory[0])
        except FileNotFoundError as error:
            if not missing_ok or error.errno != errno.ENOENT:
                raise
            self._guard()
            return None
        try:
            before = _pin(fd)
            raw = _read(fd)
            if _pin(fd) != before or len(raw) != before[4]:
                raise ObservationError('control_file_changed')
            self._guard()
        except BaseException as primary:
            _close(fd, primary)
        _close(fd)
        return raw, before

    def read_peer(self) -> bytes | None:
        try:
            self._guard()
            result = self._read_file(self.peer, missing_ok=True)
            if result is None:
                return None
            raw, _ = result
            self.decode_peer(raw)
            return raw
        except BaseException:
            self.failed = True
            raise

    def _destination(self):
        current = self._read_file(self.name, missing_ok=True)
        if current is None:
            if self.previous is not None:
                raise ObservationError('control_destination_missing')
            return
        if current != self.previous:
            raise ObservationError('control_destination_changed')

    def _prepare(self, raw, temporary):
        fd = os.open(temporary, os.O_RDWR|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,
                     0o600, dir_fd=self.directory[0])
        try:
            _pin(fd)
            remaining = memoryview(raw)
            while remaining:
                count = os.write(fd, remaining)
                if type(count) is not int or not 0 < count <= len(remaining):
                    raise ObservationError('control_write_protocol')
                remaining = remaining[count:]
            pin = _pin(fd)
            os.lseek(fd, 0, os.SEEK_SET)
            if _read(fd) != raw or pin[4] != len(raw) or _pin(fd) != pin:
                raise ObservationError('control_readback')
            if os.fsync(fd) is not None:
                raise ObservationError('control_sync_protocol')
        except BaseException as primary:
            _close(fd, primary)
        _close(fd)
        return raw, pin

    def publish(self, raw: bytes) -> None:
        try:
            self._guard()
            self.decode(raw)
            self._destination()
            temporary = self.name+'_temp'
            prepared = self._prepare(raw, temporary)
            self._guard()
            self._destination()
            if self._read_file(temporary) != prepared:
                raise ObservationError('control_prepared_temp_changed')
            # Publication is last: no directory fsync or other fallible action after it.
            os.replace(temporary, self.name, src_dir_fd=self.directory[0],
                       dst_dir_fd=self.directory[0])
            self.previous = prepared
        except BaseException:
            self.failed = True
            raise
