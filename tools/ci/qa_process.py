"""Serial, contained process/output ownership for contributor QA commands.

This is a lifecycle boundary for trusted local tools, not a hostile-code sandbox.
The admitted supervisor stays unreaped until cleanup, preserving native identity
even when the command exits before descendants release their inherited pipes.
"""

from __future__ import annotations

import math
import os
import pathlib
import re
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from typing import Callable

CHUNK_BYTES = 8192
CONTROL_BYTES = 32
CLEANUP_SECONDS = 5.0
MAX_ARGUMENTS = 512
MAX_COMMAND_BYTES = 65536
_launch_lock = threading.Lock()
# An unreaped native failure retains its owner and forbids further launches.
_quarantine: object | None = None


@dataclass(frozen=True)
class Result:
    return_code: int | None
    timed_out: bool
    error: str | None


class _WindowsJob:
    def __init__(self) -> None:
        import ctypes
        from ctypes import wintypes

        class BasicLimits(ctypes.Structure):
            _fields_ = [
                ('process_time', ctypes.c_int64), ('job_time', ctypes.c_int64),
                ('flags', wintypes.DWORD), ('minimum_working_set', ctypes.c_size_t),
                ('maximum_working_set', ctypes.c_size_t), ('active_limit', wintypes.DWORD),
                ('affinity', ctypes.c_size_t), ('priority', wintypes.DWORD),
                ('scheduling', wintypes.DWORD),
            ]

        class ExtendedLimits(ctypes.Structure):
            _fields_ = [
                ('basic', BasicLimits), ('io_counters', ctypes.c_uint64 * 6),
                ('process_memory', ctypes.c_size_t), ('job_memory', ctypes.c_size_t),
                ('peak_process_memory', ctypes.c_size_t), ('peak_job_memory', ctypes.c_size_t),
            ]

        class Accounting(ctypes.Structure):
            _fields_ = [
                ('times', ctypes.c_int64 * 4), ('page_faults', wintypes.DWORD),
                ('total', wintypes.DWORD), ('active', wintypes.DWORD),
                ('terminated', wintypes.DWORD),
            ]

        self._ctypes = ctypes
        self._accounting = Accounting
        self._api = ctypes.WinDLL('kernel32', use_last_error=True)
        signatures = {
            'CreateJobObjectW': ([ctypes.c_void_p, wintypes.LPCWSTR], wintypes.HANDLE),
            'SetInformationJobObject': ([wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD], wintypes.BOOL),
            'AssignProcessToJobObject': ([wintypes.HANDLE, wintypes.HANDLE], wintypes.BOOL),
            'TerminateJobObject': ([wintypes.HANDLE, wintypes.UINT], wintypes.BOOL),
            'QueryInformationJobObject': ([wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p], wintypes.BOOL),
            'CloseHandle': ([wintypes.HANDLE], wintypes.BOOL),
        }
        for name, (arguments, result) in signatures.items():
            function = getattr(self._api, name)
            function.argtypes, function.restype = arguments, result
        self._handle = self._api.CreateJobObjectW(None, None)
        if not self._handle:
            raise OSError('QA job creation failed')
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE; no breakaway.
        if not self._api.SetInformationJobObject(self._handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close()
            raise OSError('QA job limits failed')

    def assign(self, process: subprocess.Popen[bytes]) -> None:
        # CPython's owned handle is an exact identity; reopening a numeric PID
        # would permit reuse. The helper is still blocked at its admission gate.
        handle = getattr(process, '_handle', None)
        if handle is None or not self._api.AssignProcessToJobObject(self._handle, int(handle)):
            raise OSError('QA job admission failed')

    def terminate(self) -> None:
        if not self._api.TerminateJobObject(self._handle, 1):
            raise OSError('QA job termination failed')

    def active(self) -> int:
        value = self._accounting()
        if not self._api.QueryInformationJobObject(self._handle, 1, self._ctypes.byref(value), self._ctypes.sizeof(value), None):
            raise OSError('QA job accounting failed')
        return value.active

    def close(self) -> None:
        if self._handle:
            if not self._api.CloseHandle(self._handle):
                raise OSError('QA job handle closure failed')
            self._handle = None


def _worker(command: list[str], merge_stderr: bool) -> int:
    if sys.stdin.buffer.read(1) != b'G':
        return 126
    try:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=sys.stdout, stderr=sys.stdout if merge_stderr else subprocess.DEVNULL, close_fds=True)
        code = child.wait()
        message = f'X{code}\n'.encode('ascii')
    except (OSError, ValueError):
        message = b'E\n'
    # Data EOF and child completion are independent. Descendants may still own
    # the data handle; host retirement closes them before joining its reader.
    sys.stdout.close()
    sys.stderr.buffer.write(message)
    sys.stderr.buffer.flush()
    sys.stderr.close()
    sys.stdin.buffer.read(1)
    return 0


def run(
    command: list[str],
    *,
    cwd: pathlib.Path,
    timeout_seconds: float,
    consume: Callable[[bytes], None],
    environment: dict[str, str] | None = None,
    merge_stderr: bool = True,
) -> Result:
    """Run one exact argv; consumers own capture limits, formatting and redaction.

The command deadline includes output collection. Native creation may itself be
uninterruptible; cleanup has a separate five-second ceiling and never reports
success while an owned process or reader remains unretired.
"""
    global _quarantine
    if (
        not isinstance(command, list) or not 0 < len(command) <= MAX_ARGUMENTS
        or any(not isinstance(arg, str) or '\0' in arg for arg in command)
        or not command[0]
        or sum(len(arg.encode('utf-8')) for arg in command) > MAX_COMMAND_BYTES
        or not math.isfinite(timeout_seconds) or timeout_seconds <= 0
        or not isinstance(merge_stderr, bool)
    ):
        raise ValueError('invalid bounded QA command')
    if not _launch_lock.acquire(blocking=False):
        raise OSError('QA process owner is already occupied')
    process = None
    job = None
    assigned = False
    readers: list[threading.Thread] = []
    condition = threading.Condition()
    state: dict[str, object] = {'done': False, 'code': None, 'error': None}
    deadline = time.monotonic() + timeout_seconds
    timed_out = False
    error = None
    cleanup_error = None

    def control() -> None:
        try:
            message = process.stderr.readline(CONTROL_BYTES)
            match = re.fullmatch(rb'X(-?[0-9]{1,10})\n', message)
            code = int(match[1]) if match else None
            with condition:
                if code is None or not -(2**31) <= code < 2**32:
                    state['error'] = 'QA command start or exit protocol failed'
                else:
                    state['code'] = code
                state['done'] = True
                condition.notify_all()
        except (OSError, ValueError):
            with condition:
                state['error'] = 'QA exit receipt failed'
                state['done'] = True
                condition.notify_all()

    def output() -> None:
        try:
            while chunk := process.stdout.read(CHUNK_BYTES):
                consume(chunk)
        except BaseException:
            with condition:
                state['error'] = 'QA output consumer failed'
                condition.notify_all()

    try:
        if _quarantine is not None:
            raise OSError('QA cleanup remains unresolved')
        if os.name == 'nt':
            job = _WindowsJob()
        options = {'creationflags': subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == 'nt' else {'start_new_session': True}
        process = subprocess.Popen(
            [sys.executable, str(pathlib.Path(__file__).resolve()), '--worker' if merge_stderr else '--worker-quiet', *command],
            cwd=cwd, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, bufsize=0, close_fds=True, **options,
        )
        if job is not None:
            job.assign(process)
            assigned = True
        readers = [
            threading.Thread(target=output, name='automexia-qa-output', daemon=True),
            threading.Thread(target=control, name='automexia-qa-exit', daemon=True),
        ]
        for reader in readers:
            reader.start()
        if time.monotonic() < deadline:
            process.stdin.write(b'G')
            process.stdin.flush()
        with condition:
            while not state['done'] and state['error'] is None:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    timed_out = True
                    error = 'TimeoutExpired: QA command deadline exceeded'
                    break
                condition.wait(remaining)
            if error is None:
                error = state['error']
    finally:
        cleanup_deadline = time.monotonic() + CLEANUP_SECONDS
        try:
            if process is not None:
                if job is not None and assigned:
                    job.terminate()
                elif os.name != 'nt':
                    # Never poll/reap the supervisor before this operation: its
                    # owned, unreaped PID keeps the process-group identity valid.
                    os.killpg(process.pid, signal.SIGKILL)
                else:
                    process.kill()  # Admission failed; the helper spawned nothing.
                process.wait(timeout=max(0.001, cleanup_deadline - time.monotonic()))
                if job is not None and assigned:
                    while job.active():
                        remaining = cleanup_deadline - time.monotonic()
                        if remaining <= 0:
                            raise OSError('QA native cleanup deadline exceeded')
                        time.sleep(min(0.01, remaining))
                for reader in readers:
                    if reader.ident is not None:
                        reader.join(timeout=max(0, cleanup_deadline - time.monotonic()))
                if any(reader.is_alive() for reader in readers):
                    raise OSError('QA reader cleanup deadline exceeded')
                for stream in (process.stdin, process.stdout, process.stderr):
                    if stream is not None:
                        stream.close()
                handle = getattr(process, '_handle', None)
                if handle is not None:
                    handle.Close()
            if job is not None:
                job.close()
        except (OSError, subprocess.TimeoutExpired):
            # Keep the complete cleanup owner, not just an is_finished hint. The
            # occupied serial slot prevents growth or a false later success.
            _quarantine = (process, job, readers)
            cleanup_error = 'QA native cleanup is incomplete; further launches blocked'
        if cleanup_error is None:
            _launch_lock.release()
    return Result(state['code'], timed_out, cleanup_error or error or state['error'])


if __name__ == '__main__':
    sys.exit(_worker(sys.argv[2:], sys.argv[1] == '--worker') if len(sys.argv) > 2 and sys.argv[1] in ('--worker', '--worker-quiet') else 126)
