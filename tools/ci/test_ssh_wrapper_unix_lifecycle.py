#!/usr/bin/env python3
"""Opt-in actual CLI / controlling-PTY Linux job-control regressions.

The fixed fake OpenSSH never contacts a host or uses credentials. Every helper
self-expires; pidfds pin identities for cleanup, including intentional native
background connections. Supply --application with the compiled Linux executable.
This is Linux process evidence, not a native macOS or SSH-server claim.
"""
from __future__ import annotations

import argparse
import base64
import json
import os
from pathlib import Path
import re
import select
import shlex
import signal
import struct
import subprocess
import sys
import tempfile
import time
import unittest

import test_ssh_bash_core as pty_owner

APPLICATION: Path | None = None
IMAGE: str | None = None
READY = b'SSH_FIXTURE_READY\r\n'
SCOPE = rb'\x1b\]1337;SetUserVar=terminal_scope_v1=([^\x07]+)\x07'
FAKE = r'''#!/usr/bin/python3
import json, os, signal, sys, termios, time, tty
from pathlib import Path
root = Path(os.environ['HOME'])
mode = (root / 'mode').read_text()
if '-G' in sys.argv[1:]:
    if mode == 'exec-error': Path(__file__).unlink()
    if mode.startswith('pipe-'):
        with open('/dev/tty', 'rb', buffering=0) as terminal:
            (root / 'caller-modes').write_text(repr(termios.tcgetattr(terminal.fileno())))
    print('requesttty auto\nsessiontype default\nstdinnull no\nforkafterauthentication no\ncontrolmaster false')
    sys.exit(0)
signal.alarm(15)
record = {'leader': os.getpid(), 'wrapper': os.getppid(), 'group': os.getpgrp(),
          'wrapper_group': os.getpgid(os.getppid())}
if mode.startswith('proxy'):
    child = os.fork()
    if child == 0:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGHUP, signal.SIG_IGN)
        signal.signal(signal.SIGTSTP, signal.SIG_IGN)
        signal.alarm(15)
        while not (root / 'proxy-check').exists():
            if mode == 'proxy-stop':
                (root / 'counter.new').write_text(str(time.monotonic_ns()))
                (root / 'counter.new').replace(root / 'counter')
            time.sleep(0.02)
        (root / 'proxy-survived').write_text('yes')
        os.write(1, b'PROXY_SURVIVED\n')
        time.sleep(10)
        os._exit(0)
    record['proxy'] = child
(root / 'identities.new').write_text(json.dumps(record))
(root / 'identities.new').replace(root / 'identities')
if mode == 'early': os._exit(7)
if mode == 'early-stop': os.kill(os.getpid(), signal.SIGSTOP)
if mode == 'pipe-auth':
    with open('/dev/tty', 'rb', buffering=0) as terminal:
        if terminal.readline().strip() != b'AUTH_LINE': os._exit(43)
if mode == 'initial-read':
    if sys.stdin.readline().strip() != 'INITIAL': os._exit(43)
if mode == 'raw': tty.setraw(0)
os.write(1, b'SSH_FIXTURE_READY\n')
if mode in ('proxy-no-tty', 'pipe-only', 'pipe-auth'):
    while not (root / 'release').exists(): time.sleep(0.01)
    os._exit(7)
if mode == 'raw':
    os.read(0, 1)
    os._exit(7)
def interrupted(*_): os.write(1, b'INTERRUPTED\n')
signal.signal(signal.SIGINT, interrupted)
while True:
    line = sys.stdin.readline()
    if not line: os._exit(7)
    if line.strip() == 'EXIT': os._exit(7)
    if line.strip() == 'SIZE':
        import fcntl, struct
        rows, cols, _, _ = struct.unpack('HHHH', fcntl.ioctl(0, termios.TIOCGWINSZ, bytes(8)))
        os.write(1, ('SIZE:%d:%d\n' % (rows, cols)).encode())
    if line.strip() == 'STOP':
        os.killpg(os.getpgrp(), signal.SIGTSTP)
        os.write(1, b'RESUMED\n')
'''


def frames(data: bytes) -> list[str]:
    return [base64.b64decode(value, validate=True).decode('ascii')
            for value in re.findall(SCOPE, data)]


class PinnedProcess:
    def __init__(self, pid: int) -> None:
        self.fd = os.pidfd_open(pid)

    def live(self) -> bool:
        return not select.select([self.fd], [], [], 0)[0]

    def send(self, number: int) -> None:
        if self.live():
            try:
                signal.pidfd_send_signal(self.fd, number)
            except ProcessLookupError:
                pass

    def close(self) -> None:
        try:
            self.send(signal.SIGKILL)
            if not select.select([self.fd], [], [], 5)[0]:
                raise TimeoutError('owned fixture process did not retire')
        finally:
            os.close(self.fd)


class Session:
    def __init__(self, mode: str = 'io', integration: str = 'required',
                 *, pipe_input: bool = False) -> None:
        import fcntl
        import termios
        if APPLICATION is None:
            raise RuntimeError('compiled application is required')
        profile = ('set +H\nHISTFILE=/dev/null\n'
                   'run_fixture() { ' + ("printf 'PIPE_ONLY\\n' | " if pipe_input else '') +
                   shlex.quote(str(APPLICATION)) +
                   ' +ssh --shell bash --integration ' + integration +
                   (' --force-tty' if pipe_input else '') +
                   ' -- fixture-host; printf "LOCAL_EXIT:%s\\n" "$?"; }\n')
        self.processes: dict[str, PinnedProcess] = {}
        self.shell = pty_owner.Shell(profile, extra_bins={'ssh': FAKE})
        fcntl.ioctl(self.shell.fd, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
        self.initial_modes = termios.tcgetattr(self.shell.fd)
        (self.shell.root / 'mode').write_text(mode, encoding='ascii')

    def start(self, *, early: bool = False, initial_input: bytes = b'') -> bytes:
        output = self.shell.send(b'run_fixture\n' + initial_input,
                                 pty_owner.PROMPT if early else READY)
        record = json.loads((self.shell.root / 'identities').read_text())
        self.identities = record
        for key in ('leader', 'wrapper', 'proxy'):
            if key in record:
                try:
                    self.processes[key] = PinnedProcess(record[key])
                except ProcessLookupError:
                    if not early:
                        raise
        return output

    def close(self) -> None:
        errors = []
        try:
            for process in self.processes.values():
                try:
                    process.close()
                except (OSError, TimeoutError) as error:
                    errors.append(error)
        finally:
            self.shell.close()
        if errors:
            raise RuntimeError('fixture cleanup did not retire every pinned process') from errors[0]

    def __enter__(self) -> 'Session':
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


class UnixLifecycle(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if APPLICATION is None:
            raise unittest.SkipTest('supply --application; no actual process evidence')
        if sys.platform != 'linux' or not hasattr(os, 'pidfd_open'):
            raise unittest.SkipTest('requires Linux controlling PTY and pinned process identities')

    def assert_scope_closed(self, output: bytes) -> None:
        controls = frames(output)
        self.assertEqual(len(controls), 2)
        self.assertIn('|begin|', controls[0])
        self.assertIn('|end|', controls[1])

    def test_enhanced_normal_exit_retires_proxy_before_scope_end(self) -> None:
        with Session('proxy') as session:
            session.start()
            session.shell.send(b'EXIT\n')
            self.assert_scope_closed(bytes(session.shell.all_output))
            self.assertFalse(session.processes['proxy'].live(),
                             'enhanced descendant remained alive after scope_end')

    def test_enhanced_cancel_and_hup_retire_proxy_before_scope_end(self) -> None:
        for number in (signal.SIGTERM, signal.SIGHUP):
            with self.subTest(signal=number), Session('proxy') as session:
                session.start()
                session.processes['wrapper'].send(number)
                session.shell.until(pty_owner.PROMPT)
                self.assert_scope_closed(bytes(session.shell.all_output))
                self.assertFalse(session.processes['proxy'].live(),
                                 'cancelled enhanced descendant survived local restoration')

    def test_native_off_preserves_intentional_background_connection(self) -> None:
        with Session('proxy', 'off') as session:
            session.start()
            session.shell.send(b'EXIT\n')
            self.assertEqual(frames(bytes(session.shell.all_output)), [])
            self.assertTrue(session.processes['proxy'].live())
            (session.shell.root / 'proxy-check').write_text('ack', encoding='ascii')
            session.shell.until(b'PROXY_SURVIVED\r\n')

    def test_initial_read_ctrl_c_resize_and_exit_keep_terminal_ownership(self) -> None:
        import fcntl
        import termios
        with Session() as session:
            session.start()
            session.shell.send(b'\x03', b'INTERRUPTED\r\n')
            self.assertTrue(session.processes['wrapper'].live())
            fcntl.ioctl(session.shell.fd, termios.TIOCSWINSZ,
                        struct.pack('HHHH', 41, 119, 0, 0))
            session.shell.send(b'SIZE\n', b'SIZE:41:119\r\n')
            result = session.shell.send(b'EXIT\n')
            self.assertIn(b'LOCAL_EXIT:7', result)
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            self.assert_scope_closed(bytes(session.shell.all_output))

    def test_raw_mode_abrupt_exit_and_fast_exit_restore_parent(self) -> None:
        import termios
        for mode in ('raw', 'early'):
            with self.subTest(mode=mode), Session(mode) as session:
                if mode == 'early':
                    session.start(early=True)
                else:
                    # Raw mode disables output CR translation too.
                    output = session.shell.send(b'run_fixture\n', b'SSH_FIXTURE_READY\n')
                    self.assertTrue(output)
                    session.shell.send(b'x')
                self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
                self.assertEqual(termios.tcgetattr(session.shell.fd), session.initial_modes)
                self.assert_scope_closed(bytes(session.shell.all_output))

    def test_failed_exec_preserves_parent_terminal(self) -> None:
        import termios
        with Session('exec-error') as session:
            session.shell.send(b'run_fixture\n')
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            self.assertEqual(termios.tcgetattr(session.shell.fd), session.initial_modes)
            self.assert_scope_closed(bytes(session.shell.all_output))

    def test_fast_initial_reads_receive_foreground_before_input(self) -> None:
        for attempt in range(8):
            with self.subTest(attempt=attempt), Session('initial-read') as session:
                session.start(initial_input=b'INITIAL\n')
                output = session.shell.send(b'SIZE\n', b'SIZE:')
                self.assertNotIn(b'Stopped', output)
                self.assertNotEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
                session.shell.send(b'EXIT\n')
                self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)

    def test_explicit_early_stop_is_preserved_until_foreground_resume(self) -> None:
        with Session('early-stop') as session:
            stopped = session.start(early=True)
            self.assertNotIn(READY, stopped)
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            session.shell.send(b'fg\n', READY)
            session.shell.send(b'EXIT\n')
            self.assert_scope_closed(bytes(session.shell.all_output))

    def test_no_controlling_tty_still_retires_owned_group(self) -> None:
        with tempfile.TemporaryDirectory(prefix='automexia-ssh-no-tty-') as temporary:
            root = Path(temporary)
            fake = root / 'ssh'
            fake.write_text(FAKE, encoding='utf-8')
            fake.chmod(0o700)
            (root / 'mode').write_text('proxy-no-tty', encoding='ascii')
            environment = {'HOME': str(root), 'PATH': str(root), 'LC_ALL': 'C.UTF-8'}
            handles: list[PinnedProcess] = []
            with tempfile.TemporaryFile() as output:
                child = subprocess.Popen([str(APPLICATION), '+ssh', '--shell', 'bash',
                                          '--integration', 'required', '--force-tty',
                                          '--', 'fixture-host'], env=environment,
                                         start_new_session=True, stdin=subprocess.DEVNULL,
                                         stdout=output, stderr=output)
                try:
                    deadline = time.monotonic() + 5
                    while not (root / 'identities').exists():
                        if time.monotonic() >= deadline:
                            raise TimeoutError('no-TTY fixture did not start')
                        time.sleep(0.01)
                    record = json.loads((root / 'identities').read_text())
                    handles = [PinnedProcess(record[key]) for key in ('leader', 'proxy')]
                    (root / 'release').write_text('exit', encoding='ascii')
                    self.assertEqual(child.wait(timeout=5), 7)
                    output.seek(0)
                    data = output.read(64 * 1024 + 1)
                    self.assertLessEqual(len(data), 64 * 1024)
                    self.assert_scope_closed(data)
                    self.assertFalse(handles[1].live(), 'no-TTY enhanced proxy survived')
                finally:
                    if child.poll() is None:
                        child.kill()
                    child.wait(timeout=5)
                    for handle in handles:
                        handle.close()

    def test_pipe_only_child_does_not_change_shared_terminal(self) -> None:
        import termios
        with Session('pipe-only', pipe_input=True) as session:
            session.start()
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.identities['wrapper_group'])
            expected = (session.shell.root / 'caller-modes').read_text()
            self.assertEqual(repr(termios.tcgetattr(session.shell.fd)), expected)
            (session.shell.root / 'release').write_text('exit', encoding='ascii')
            session.shell.until(pty_owner.PROMPT)
            self.assert_scope_closed(bytes(session.shell.all_output))

    def test_pipe_auth_read_claims_terminal_after_actual_io_stop(self) -> None:
        import termios
        with Session('pipe-auth', pipe_input=True) as session:
            session.start(initial_input=b'AUTH_LINE\n')
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.identities['group'])
            (session.shell.root / 'release').write_text('exit', encoding='ascii')
            session.shell.until(pty_owner.PROMPT)
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            self.assertEqual(termios.tcgetattr(session.shell.fd), session.initial_modes)
            self.assert_scope_closed(bytes(session.shell.all_output))

    def test_stop_bg_and_fg_do_not_steal_local_terminal(self) -> None:
        with Session('proxy-stop') as session:
            session.start()
            deadline = time.monotonic() + 5
            while not (session.shell.root / 'counter').exists():
                if time.monotonic() >= deadline:
                    raise TimeoutError('proxy heartbeat did not start')
                time.sleep(0.01)
            session.shell.send(b'STOP\n')
            self.assertTrue(session.processes['wrapper'].live())
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            before = (session.shell.root / 'counter').read_bytes()
            time.sleep(0.12)
            self.assertEqual((session.shell.root / 'counter').read_bytes(), before,
                             'stopped enhanced job left a proxy running behind the local prompt')
            session.shell.send(b'bg\n')
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.shell.pid)
            # The resumed background child attempts a read and stops itself;
            # require a second local prompt before the outer shell's fg.
            session.shell.send(b'\n')
            session.shell.send(b'fg\nSIZE\n', b'SIZE:24:80\r\n')
            self.assertEqual(os.tcgetpgrp(session.shell.fd), session.identities['group'])
            session.shell.send(b'EXIT\n')
            self.assert_scope_closed(bytes(session.shell.all_output))


class ActualOpenSshJobControl(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if IMAGE is None or APPLICATION is None:
            raise unittest.SkipTest('supply immutable --image and --application for real SSH evidence')
        if sys.platform != 'linux' or not hasattr(os, 'pidfd_open'):
            raise unittest.SkipTest('requires Linux controlling PTY and pinned process identities')

    def test_ssh_escape_suspend_bg_fg_interrupt_resize_and_exit(self) -> None:
        import fcntl
        import termios
        import test_ssh_wrapper_runtime as runtime
        local_prompt = b'LOCAL_JOB_PROMPT> '
        with runtime.Fixture(IMAGE) as fixture:
            profile = ('set +H\nHISTFILE=/dev/null\nrun_fixture() { ' +
                       shlex.quote(str(APPLICATION)) +
                       ' +ssh --shell bash --integration required -- -F ' +
                       shlex.quote(str(fixture.config)) +
                       ' fixture-host; printf "LOCAL_EXIT:%s\\n" "$?"; }\n')
            with pty_owner.Shell(profile) as shell:
                shell.send(b"PS1='LOCAL_JOB_PROMPT> '\n", local_prompt)
                local_modes = termios.tcgetattr(shell.fd)
                shell.send(b'run_fixture\n')
                children = Path('/proc') / str(shell.pid) / 'task' / str(shell.pid) / 'children'
                wrapper_ids = children.read_text().split()
                self.assertLessEqual(len(wrapper_ids), 4)
                handles = []
                for pid in wrapper_ids:
                    # This strict fixture has no ProxyCommand: the direct SSH
                    # child is the only extra cleanup identity we need to pin.
                    descendants = Path('/proc') / pid / 'task' / pid / 'children'
                    child_ids = descendants.read_text().split()
                    self.assertLessEqual(len(child_ids), 4)
                    handles.extend(PinnedProcess(int(child_id)) for child_id in child_ids)
                    handles.append(PinnedProcess(int(pid)))
                self.assertTrue(handles, 'actual wrapper must be owned by the local shell')
                try:
                    # OpenSSH interprets this local escape after a newline; it
                    # does not forward Ctrl+Z as a remote command editor key.
                    shell.send(b'\n~\x1a', local_prompt)
                    self.assertEqual(os.tcgetpgrp(shell.fd), shell.pid)
                    self.assertEqual(termios.tcgetattr(shell.fd), local_modes)
                    shell.send(b'bg\n', local_prompt)
                    self.assertEqual(os.tcgetpgrp(shell.fd), shell.pid)
                    shell.send(b'\n', local_prompt)
                    shell.send(b"fg\nprintf 'JOB_RESUME_OK\\n'\n", b'JOB_RESUME_OK\r\n')
                    shell.until(pty_owner.PROMPT)
                    self.assertNotEqual(os.tcgetpgrp(shell.fd), shell.pid)
                    shell.send(b"printf 'SLEEP_READY\\n'; sleep 30\n", b'SLEEP_READY\r\n')
                    interrupted = shell.send(b'\x03')
                    self.assertIn(b'\x1b]133;D;130\x07', interrupted)
                    fcntl.ioctl(shell.fd, termios.TIOCSWINSZ, struct.pack('HHHH', 41, 119, 0, 0))
                    size = shell.command('stty size')
                    self.assertIn(b'41 119\r\n', size)
                    shell.send(b'exit 7\n', local_prompt)
                    # Suspending finished the original shell function. `fg`
                    # now owns this completion status, not that old function.
                    status = shell.send(b'printf "FINAL_JOB_STATUS:%s\\n" "$?"\n', local_prompt)
                    self.assertIn(b'FINAL_JOB_STATUS:7\r\n', status)
                    self.assertEqual(os.tcgetpgrp(shell.fd), shell.pid)
                    self.assertEqual(termios.tcgetattr(shell.fd), local_modes)
                    controls = frames(bytes(shell.all_output))
                    self.assertEqual(len(controls), 2)
                    self.assertIn('|begin|', controls[0])
                    self.assertIn('|end|', controls[1])
                finally:
                    for handle in handles:
                        handle.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--application', type=Path, required=True)
    parser.add_argument('--image', help='existing immutable local fixture image; never pulled')
    arguments, remaining = parser.parse_known_args()
    APPLICATION = arguments.application.resolve(strict=True)
    IMAGE = arguments.image
    unittest.main(argv=[sys.argv[0], *remaining])
