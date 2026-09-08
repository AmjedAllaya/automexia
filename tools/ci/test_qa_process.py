"""Real native ownership tests; output, receipts and process identity are separate oracles."""

from __future__ import annotations

import ctypes
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

import qa_process as PROCESS


class NativeIdentity:
    def __init__(self, pid: int):
        if os.name == 'nt':
            from ctypes import wintypes
            self.api = ctypes.WinDLL('kernel32', use_last_error=True)
            self.api.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
            self.api.OpenProcess.restype = wintypes.HANDLE
            self.api.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
            self.api.WaitForSingleObject.restype = wintypes.DWORD
            self.api.CloseHandle.argtypes = [wintypes.HANDLE]
            self.api.CloseHandle.restype = wintypes.BOOL
            self.handle = self.api.OpenProcess(0x100000, False, pid)
            if not self.handle:
                raise OSError('fixture identity acquisition failed')
        else:
            self.handle = os.pidfd_open(pid)

    def exited(self) -> bool:
        if os.name == 'nt':
            return self.api.WaitForSingleObject(self.handle, 1000) == 0
        return bool(select.select([self.handle], [], [], 1)[0])

    def close(self):
        if os.name == 'nt':
            self.api.CloseHandle(self.handle)
        else:
            os.close(self.handle)


class QaProcessTests(unittest.TestCase):
    def tearDown(self):
        self.assertIsNone(PROCESS._quarantine)
        self.assertFalse(PROCESS._launch_lock.locked())
        self.assertFalse(any(t.name.startswith('automexia-qa-') for t in threading.enumerate()))

    def run_code(self, code, *, timeout=5, consume=None, **options):
        chunks = []
        with tempfile.TemporaryDirectory() as temporary:
            result = PROCESS.run(
                [sys.executable, '-c', code], cwd=Path(temporary),
                timeout_seconds=timeout, consume=consume or chunks.append, **options,
            )
        return result, b''.join(chunks)

    def test_exact_arguments_environment_and_nonzero_receipt(self):
        arguments = ['space value', '', '界', '; echo forbidden', '$(not-evaluated)', 'a"b']
        code = 'import json,os,sys;print(json.dumps([sys.argv[1:],os.getenv("QA_FIXTURE")],ensure_ascii=True));sys.exit(7)'
        chunks = []
        with tempfile.TemporaryDirectory() as temporary:
            result = PROCESS.run(
                [sys.executable, '-c', code, *arguments], cwd=Path(temporary),
                timeout_seconds=5, consume=chunks.append,
                environment={**os.environ, 'QA_FIXTURE': 'fixture-value'},
            )
        self.assertEqual(json.loads(b''.join(chunks)), [arguments, 'fixture-value'])
        self.assertEqual(result, PROCESS.Result(7, False, None))

    def test_final_tail_and_stderr_are_preserved_without_a_newline(self):
        result, raw = self.run_code('import os;os.write(1,b"first");os.write(2,b"last")')
        self.assertEqual((result, raw), (PROCESS.Result(0, False, None), b'firstlast'))
        result, raw = self.run_code('import os;os.write(1,b"data\\0");os.write(2,b"ignored")', merge_stderr=False)
        self.assertEqual((result, raw), (PROCESS.Result(0, False, None), b'data\0'))

    def test_chunking_and_reader_retirement_across_repeated_launches(self):
        for _ in range(12):
            chunks = []
            result, _ = self.run_code('import os;os.write(1,b"x"*131073)', consume=chunks.append)
            self.assertEqual(result, PROCESS.Result(0, False, None))
            self.assertEqual(b''.join(chunks), b'x' * 131073)
            self.assertTrue(all(0 < len(chunk) <= PROCESS.CHUNK_BYTES for chunk in chunks))
            self.assertFalse(any(t.name.startswith('automexia-qa-') for t in threading.enumerate()))

    def test_invalid_and_over_limit_inputs_never_spawn(self):
        invalid = [[], [''], ['bad\0name'], [None], ['x'] * 513, ['x' * 65537]]
        with mock.patch.object(PROCESS.subprocess, 'Popen') as spawn:
            for command in invalid:
                with self.subTest(kind='argv'), self.assertRaises(ValueError):
                    PROCESS.run(command, cwd=Path('.'), timeout_seconds=1, consume=lambda _: None)
            for timeout in (0, -1, float('nan'), float('inf')):
                with self.subTest(kind='deadline'), self.assertRaises(ValueError):
                    PROCESS.run(['fixture'], cwd=Path('.'), timeout_seconds=timeout, consume=lambda _: None)
            spawn.assert_not_called()

    def test_missing_executable_returns_a_redacted_failure(self):
        result, raw = self.run_code('import sys;sys.exit(0)')
        self.assertEqual(result.return_code, 0)
        with tempfile.TemporaryDirectory() as temporary:
            result = PROCESS.run([str(Path(temporary) / 'absent-tool')], cwd=Path(temporary), timeout_seconds=5, consume=lambda _: None)
        self.assertEqual(result, PROCESS.Result(None, False, 'QA command start or exit protocol failed'))

    def test_malformed_exit_receipts_fail_closed_with_native_cleanup(self):
        original = PROCESS.subprocess.Popen
        for receipt in (b'', b'E\n', b'Xno\n', b'X4294967296\n', b'X-2147483649\n', b'x' * 64):
            with self.subTest(receipt=receipt):
                code = f'import os;os.read(0,1);os.write(2,{receipt!r});os.close(2);os.read(0,1)'
                def supervisor(_argv, **kwargs):
                    return original([sys.executable, '-c', code], **kwargs)
                with mock.patch.object(PROCESS.subprocess, 'Popen', side_effect=supervisor):
                    result, _ = self.run_code('pass')
                self.assertEqual(result, PROCESS.Result(None, False, 'QA command start or exit protocol failed'))

    def test_incomplete_cleanup_retains_owner_and_blocks_following_launches(self):
        # Isolate injected cleanup failure: the tested helper kills/reaps its
        # native group, but cannot assert reader retirement. Process exit is the
        # final owner; a later command must not launder this into success.
        code = (
            'import pathlib,sys;from unittest import mock;'
            f'sys.path.insert(0,{str(Path(PROCESS.__file__).parent)!r});'
            'import qa_process as p;'
            '\nwith mock.patch.object(p.threading.Thread,"is_alive",return_value=True):\n'
            ' result=p.run([sys.executable,"-c","pass"],cwd=pathlib.Path("."),timeout_seconds=5,consume=lambda _:None)\n'
            'assert result.error and p._quarantine is not None and p._launch_lock.locked();'
            '\ntry:\n p.run([sys.executable,"-c","pass"],cwd=pathlib.Path("."),timeout_seconds=5,consume=lambda _:None)\n'
            'except OSError: print("blocked",flush=True)\n'
            'else: raise AssertionError("cleanup failure was ignored")\n'
        )
        result, raw = self.run_code(code, timeout=8)
        self.assertEqual(result, PROCESS.Result(0, False, None))
        self.assertEqual(raw.strip(), b'blocked')

    def test_launch_failure_and_interruption_release_admission(self):
        with mock.patch.object(PROCESS.subprocess, 'Popen', side_effect=OSError('fixture')):
            with self.assertRaises(OSError):
                self.run_code('pass')
        # Interrupt the supervisor's host wait, not a fake child completion.
        original = threading.Condition.wait
        def interrupt(condition, *args, **kwargs):
            if threading.current_thread() is threading.main_thread() and args:
                raise KeyboardInterrupt
            return original(condition, *args, **kwargs)
        with mock.patch.object(PROCESS.threading.Condition, 'wait', new=interrupt):
            with self.assertRaises(KeyboardInterrupt):
                self.run_code('import time;time.sleep(15)')

    def test_concurrent_launch_is_rejected_without_creating_another_owner(self):
        failures = []
        def consume(_):
            try:
                self.run_code('pass')
            except OSError:
                failures.append('occupied')
        result, _ = self.run_code('print("fixture",flush=True)', consume=consume)
        self.assertEqual(result, PROCESS.Result(0, False, None))
        self.assertEqual(failures, ['occupied'])

    @unittest.skipUnless(os.name == 'nt' or hasattr(os, 'pidfd_open'), 'exact native identity oracle needs Windows handles or Linux pidfds')
    def test_descendant_identity_retires_on_parent_exit_timeout_and_consumer_failure(self):
        for mode in ('exit', 'timeout', 'overflow'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                acknowledgment = Path(temporary) / 'identity-acquired'
                child = 'import os,time;print(os.getpid(),flush=True);time.sleep(15)'
                parent = (
                    'import pathlib,subprocess,sys,time;'
                    f'p=subprocess.Popen([sys.executable,"-c",{child!r}],stdout=subprocess.PIPE);'
                    'print(p.stdout.readline().decode().strip(),flush=True);'
                    f'ack=pathlib.Path({str(acknowledgment)!r});'
                    'deadline=time.monotonic()+8;'
                    '\nwhile not ack.exists() and time.monotonic()<deadline: time.sleep(0.005)\n'
                    'assert ack.exists();'
                    + ('sys.exit(7)' if mode == 'exit' else 'print("overflow",flush=True);time.sleep(15)')
                )
                identity = []
                pending = bytearray()
                def consume(chunk):
                    pending.extend(chunk)
                    if not identity and b'\n' in pending:
                        pid, _, rest = pending.partition(b'\n')
                        identity.append(NativeIdentity(int(pid)))
                        pending[:] = rest
                        # Parent cannot exit before the independent handle/pidfd
                        # is pinned. No PID is logged, snapshotted or reopened.
                        acknowledgment.touch()
                    if mode == 'overflow' and b'overflow' in pending:
                        raise ValueError('fixture ceiling')
                started = time.monotonic()
                try:
                    result = PROCESS.run([sys.executable, '-c', parent], cwd=Path(temporary), timeout_seconds=2, consume=consume)
                    self.assertEqual(len(identity), 1, 'native readiness must actually execute')
                    self.assertTrue(identity[0].exited(), 'exact child identity remains alive')
                    self.assertLess(time.monotonic() - started, 7)
                    if mode == 'exit':
                        self.assertEqual(result, PROCESS.Result(7, False, None))
                    elif mode == 'timeout':
                        self.assertTrue(result.timed_out)
                    else:
                        self.assertEqual(result.error, 'QA output consumer failed')
                finally:
                    for handle in identity:
                        handle.close()

    @unittest.skipUnless(os.name == 'nt', 'Windows job admission boundary')
    def test_failed_job_admission_cannot_execute_the_requested_command(self):
        with tempfile.TemporaryDirectory() as temporary:
            sentinel = Path(temporary) / 'unexpected-execution'
            code = f'import pathlib;pathlib.Path({str(sentinel)!r}).touch()'
            with mock.patch.object(PROCESS._WindowsJob, 'assign', side_effect=OSError('fixture admission failure')):
                with self.assertRaises(OSError):
                    self.run_code(code)
            self.assertFalse(sentinel.exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
