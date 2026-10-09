#!/usr/bin/env python3
"""Actual generated helper hooks over native shell PTYs and local datagrams."""
from __future__ import annotations
import argparse
import base64
import json
import errno
import select
import platform
import time
import os
from pathlib import Path
import re
import shlex
import shutil
import socket
import tempfile
import unittest

import test_ssh_bash_core as owner

FIXTURES: Path | None = None
HELPER: Path | None = None
TIMINGS: list[dict] = []
FIELDS = ('cwd HOME KUBECONFIG HOMEDRIVE HOMEPATH USERPROFILE DOCKER_CONTEXT '
          'DOCKER_HOST_PRESENT AWS_PROFILE AWS_DEFAULT_PROFILE AWS_REGION AWS_DEFAULT_REGION '
          'AZURE_CLOUD_NAME CLOUDSDK_ACTIVE_CONFIG_NAME CLOUDSDK_CORE_PROJECT CLOUDSDK_COMPUTE_REGION '
          'TF_WORKSPACE AUTOMEXIA_ENV ENVIRONMENT APP_ENV NODE_ENV GIT_BRANCH KUBECONTEXT KUBE_CONTEXT KUBE_NAMESPACE').split()
REVISION = rb'\x1b\]1337;SetUserVar=automexia_ssh_revision=([^\x07]*)\x07'

LAUNCHER = '''import os, pathlib, shlex, sys
shell, init, fd, response = sys.argv[1:]
home = pathlib.Path(os.environ['HOME'])
os.environ['AMX_SSH_HELPER_FD'] = fd
os.environ.pop('AMX_SSH_HELPER_RESPONSE_FD', None)
if response != 'none':
    os.environ['AMX_SSH_HELPER_RESPONSE_FD'] = response
os.environ['XDG_CONFIG_HOME'] = str(home / '.config')
if shell.endswith('bash'):
    (home / '.bashrc').write_text('PS1="AMX_AUDIT_PROMPT> "\\n')
    argv = [shell, '--noprofile', '--rcfile', str(pathlib.Path(init) / 'rc.bash'), '-i']
elif shell.endswith('zsh'):
    (home / '.zshrc').write_text('PROMPT="AMX_AUDIT_PROMPT> "\\n')
    os.environ['ZDOTDIR'] = init
    argv = [shell, '-d', '-i']
else:
    raise RuntimeError('unsupported helper shell fixture')
os.execve(shell, argv, os.environ)
'''


def generated(shell: str) -> dict[str, str]:
    if FIXTURES is None:
        raise unittest.SkipTest('requires canonical --fixtures exported by Rust')
    data = (FIXTURES / (shell + '.fixture')).read_bytes()
    if len(data) > 128 * 1024:
        raise ValueError('generated helper fixture exceeds its bound')
    lines = data.splitlines()
    if lines[0] != b'AMXSSH-HELPER-FIXTURE-1':
        raise ValueError('invalid canonical helper fixture')
    files = {}
    allowed = {'rc.bash', '.zshenv', '.zshrc', 'rc.ps1'}
    for line in lines[1:]:
        key, encoded = line.split(b'=', 1)
        name = key.decode().removeprefix('file:')
        source = base64.b64decode(encoded, validate=True).decode('utf-8')
        if name not in allowed or name in files or '@@' in source or len(source.encode()) > 32 * 1024:
            raise ValueError('invalid generated helper source')
        files[name] = source
    return files


def revisions(output: bytes) -> list[str]:
    return [base64.b64decode(value, validate=True).decode('ascii') if value else ''
            for value in re.findall(REVISION, output)]


class HelperShell(owner.Shell):
    def __init__(self, shell: str, *, prompt_output: bool = False) -> None:
        import fcntl
        self.files = tempfile.TemporaryDirectory(prefix='automexia-helper-source-')
        self.initialization = Path(self.files.name)
        self.receiver, self.sender = socket.socketpair(socket.AF_UNIX, socket.SOCK_DGRAM)
        for endpoint in (self.receiver, self.sender):
            endpoint.setblocking(False)
            endpoint.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 65536)
            endpoint.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 65536)
        self.writer = fcntl.fcntl(self.sender.fileno(), fcntl.F_DUPFD, 10)
        os.set_inheritable(self.writer, True)
        self.response = None
        self.response_fd = None
        if prompt_output:
            send, receive = socket.socketpair()
            send.setblocking(False)
            receive.setblocking(False)
            self.response = send
            self.response_fd = fcntl.fcntl(receive.fileno(), fcntl.F_DUPFD, 10)
            os.set_inheritable(self.response_fd, True)
            receive.close()
        self.helper_closed = False
        self.shell_name = shell
        try:
            for name, source in generated(shell).items():
                (self.initialization / name).write_text(source, encoding='utf-8')
            launcher = self.initialization / 'launch.py'
            launcher.write_text(LAUNCHER, encoding='utf-8')
            executable = shutil.which(shell)
            if executable is None:
                raise unittest.SkipTest('native shell unavailable: ' + shell)
            super().__init__(launch_arguments=['/usr/bin/python3', str(launcher), executable,
                                              str(self.initialization), str(self.writer),
                                              str(self.response_fd) if self.response_fd is not None else 'none'])
        except BaseException:
            self.close_transport()
            raise

    def close_transport(self) -> None:
        if self.helper_closed:
            return
        self.helper_closed = True
        os.close(self.writer)
        if self.response_fd is not None:
            os.close(self.response_fd)
        if self.response is not None:
            self.response.close()
        self.sender.close()
        self.receiver.close()
        self.files.cleanup()

    def close(self) -> None:
        try:
            super().close()
        finally:
            self.close_transport()

    def requests(self) -> list[tuple[int, dict[str, str]]]:
        data = bytearray()
        while True:
            try:
                data.extend(self.receiver.recv(16384))
            except BlockingIOError:
                break
            if len(data) > 512 * 1024:
                raise ValueError('helper request output exceeded fixture bound')
        result = []
        for frame in data.split(b'\0'):
            if not frame:
                continue
            lines = frame.decode('utf-8').splitlines()
            header = lines[0].split('|')
            if len(header) != 4 or header[:3] != ['AMXREQ1', '3', '7'] or len(lines) != 26:
                raise AssertionError('invalid helper frame structure')
            entries = [line.split('=', 1) for line in lines[1:]]
            if [entry[0] for entry in entries] != FIELDS or len(frame) + 2 > 16384:
                raise AssertionError('invalid helper fields/budget')
            result.append((int(header[3]), dict(entries)))
        return result

    def change(self, name: str, value: str) -> bytes:
        # Fixture values are encoded by the native shell's fixed printf format;
        # no user command is evaluated by the test runner.
        escaped = ''.join('\\%03o' % byte for byte in value.encode('utf-8'))
        command = f"export {name}=$(builtin printf '{escaped}')"
        return self.command(command)


class PosixHelperContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if os.name != 'posix' or FIXTURES is None:
            raise unittest.SkipTest('requires native Unix shells and actual generated fixtures')

    def test_actual_helper_discovers_publishes_at_prompt_and_retires_owned_files(self) -> None:
        if HELPER is None:
            self.skipTest('requires an explicitly built native helper')
        launcher_source = """import os, pathlib, sys
helper, shell = sys.argv[1:]
home = pathlib.Path(os.environ['HOME'])
(home / '.bashrc').write_text('PS1="AMX_AUDIT_PROMPT> "\\n')
(home / '.zshenv').write_text('skip_global_compinit=1\\n')
(home / '.zshrc').write_text('PROMPT="AMX_AUDIT_PROMPT> "\\n')
git = pathlib.Path.cwd() / '.git'
git.mkdir()
(git / 'HEAD').write_text('ref: refs/heads/channel-fixture\\n')
os.execve(helper, [helper, '--session-v1', shell, '3', '7'], os.environ)
"""
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), tempfile.TemporaryDirectory(prefix='automexia-helper-launch-') as temporary:
                launcher = Path(temporary) / 'launch.py'
                launcher.write_text(launcher_source, encoding='utf-8')
                with owner.Shell(launch_arguments=['/usr/bin/python3', str(launcher), str(HELPER), name]) as shell:
                    deadline = time.monotonic() + 8
                    values = []
                    while time.monotonic() < deadline:
                        shell.command('true')
                        values = [base64.b64decode(value, validate=True)
                                  for value in re.findall(
                                      rb'\x1b\]1337;SetUserVar=automexia_ssh_context_v2=([^\x07]+)\x07',
                                      bytes(shell.all_output))]
                        if any(b'git_branch=channel-fixture\n' in value for value in values):
                            break
                        time.sleep(.05)
                    self.assertTrue(any(b'git_branch=channel-fixture\n' in value for value in values))
                    output = shell.command("builtin printf 'UNTOUCHED_OUTPUT\\n'")
                    without_osc = re.sub(rb'\x1b\][^\x07]*\x07', b'', output)
                    self.assertIn(b'UNTOUCHED_OUTPUT', without_osc)
                    os.write(shell.fd, b'exit 7\n')
                    deadline = time.monotonic() + 8
                    # Keep the exact helper unreaped until the fixture owner
                    # retires it; no process identifier can be recycled here.
                    while True:
                        self.assertLess(time.monotonic(), deadline, 'native helper did not exit')
                        if not select.select([shell.fd], [], [], .1)[0]:
                            continue
                        try:
                            chunk = os.read(shell.fd, 8192)
                        except OSError as error:
                            if error.errno != errno.EIO:
                                raise
                            break
                        if not chunk:
                            break
                    self.assertEqual(list(shell.tmp.glob('automexia-ssh*')), [])

    def test_prompt_channel_waits_for_whole_frame_and_keeps_command_output_outside_osc(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name, prompt_output=True) as shell:
                shell.requests()
                prefix = b'\x1b]1337;SetUserVar=automexia_ssh_context_v2='
                frame = prefix + b'A' * (6144 - len(prefix) - 1) + b'\x07'
                # A genuinely incomplete response is already in the stream.
                # No notification exists, so the prompt must not consume it.
                shell.response.sendall(frame[:3000])
                first = shell.command("builtin printf 'COMMAND_BEFORE\\n'")
                self.assertIn(b'COMMAND_BEFORE', first)
                self.assertNotIn(prefix, first)
                shell.response.sendall(frame[3000:] + b'\0')
                shell.receiver.send(b'1')
                second = shell.command("builtin printf 'COMMAND_AFTER\\n'")
                self.assertEqual(second.count(frame), 1)
                self.assertEqual(shell.response.recv(1), b'1')
                self.assertLess(second.index(b'COMMAND_AFTER'), second.index(frame))
                self.assertNotIn(prefix, shell.command('true'))
                result = shell.command('builtin printf "RESPONSE:%s\\n" "${AMX_SSH_HELPER_RESPONSE_FD-unset}"')
                self.assertIn(b'RESPONSE:unset', result)

    def test_prompt_channel_coalesces_complete_frames_and_rejects_foreign_controls(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name, prompt_output=True) as shell:
                shell.requests()
                prefix = b'\x1b]1337;SetUserVar=automexia_ssh_context_v2='
                frames = [prefix + base64.b64encode(f'fixture-{n}'.encode()) + b'\x07'
                          for n in range(4)]
                for frame in frames:
                    shell.response.sendall(frame + b'\0')
                    shell.receiver.send(b'1')
                result = shell.command('true')
                self.assertEqual(result.count(prefix), 1)
                self.assertIn(frames[-1], result)
                shell.response.sendall(prefix + b'A\x1b[31m' + b'\x07\0')
                shell.receiver.send(b'1')
                result = shell.command('true')
                self.assertNotIn(b'\x1b[31m', result)
                self.assertEqual(revisions(result)[-1], '')
                self.assertIn(owner.PROMPT, shell.command('true'))

    def test_prompt_channel_maximum_frame_roundtrip_samples(self) -> None:
        prefix = b'\x1b]1337;SetUserVar=automexia_ssh_context_v2='
        frame = prefix + b'A' * (6144 - len(prefix) - 1) + b'\x07'
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name, prompt_output=True) as shell:
                shell.requests()
                shell.command('export PATH=/fixture/no-programs')
                shell.requests()
                samples = []
                for index in range(13):
                    shell.response.sendall(frame + b'\0')
                    shell.receiver.send(b'1')
                    started = time.monotonic()
                    output = shell.command('true')
                    elapsed = (time.monotonic() - started) * 1000
                    self.assertEqual(output.count(frame), 1)
                    shell.requests()
                    if index >= 3:
                        samples.append(round(elapsed, 3))
                TIMINGS.append({
                    'scenario': 'maximum-frame-through-native-prompt',
                    'shell': name, 'system': platform.system(),
                    'architecture': platform.machine(), 'frame_bytes': len(frame),
                    'timing_kind': 'diagnostic-native-pty-roundtrip-not-cross-machine-baseline',
                    'warmup_samples': 3, 'samples_ms': samples,
                })

    def test_prompt_channel_missing_and_oversized_acknowledged_frames_disable_without_wait(self) -> None:
        for name in ('bash', 'zsh'):
            for payload in (b'', b'x' * 6145 + b'\0'):
                with self.subTest(shell=name, size=len(payload)), HelperShell(name, prompt_output=True) as shell:
                    shell.requests()
                    if payload:
                        shell.response.sendall(payload)
                    shell.receiver.send(b'1')
                    result = shell.command('true')
                    self.assertEqual(revisions(result)[-1], '')
                    self.assertIn(owner.PROMPT, shell.command('true'))

    def test_complete_snapshot_revision_stability_and_no_endpoint_leak(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                initial = shell.requests()
                self.assertTrue(initial)
                self.assertEqual(initial[-1][0], 1)
                self.assertEqual(Path(initial[-1][1]['cwd']).resolve(), shell.cwd.resolve())
                self.assertTrue(all(value.endswith('|1') for value in revisions(shell.initial)))
                shell.command('true')
                self.assertEqual(shell.requests()[-1][0], 1)
                shell.change('DOCKER_HOST', 'unix:///fixture.sock')
                revision, fields = shell.requests()[-1]
                self.assertEqual(revision, 2)
                self.assertEqual(fields['DOCKER_HOST_PRESENT'], '1')
                self.assertNotIn('unix:///fixture.sock', json.dumps(fields))

    def test_changed_values_controls_and_utf8_bounds_clear_whole_snapshot(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                shell.change('GIT_BRANCH', 'feature/fixture')
                self.assertEqual(shell.requests()[-1][1]['GIT_BRANCH'], 'feature/fixture')
                shell.change('AWS_PROFILE', '\x01bad')
                revision, fields = shell.requests()[-1]
                self.assertTrue(all(value == '' for value in fields.values()))
                shell.command('true')
                self.assertEqual(shell.requests()[-1][0], revision)
                shell.change('AWS_PROFILE', '猫' * 86)
                self.assertTrue(all(value == '' for value in shell.requests()[-1][1].values()))
                shell.change('AWS_PROFILE', '猫' * 85)
                self.assertEqual(shell.requests()[-1][1]['AWS_PROFILE'], '猫' * 85)

    def test_builtin_only_prompt_and_revision_exhaustion(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                shell.command('export PATH=/fixture/no-programs')
                shell.requests()
                shell.change('GIT_BRANCH', 'builtins-only')
                self.assertEqual(shell.requests()[-1][1]['GIT_BRANCH'], 'builtins-only')
                shell.command('__amx_ssh_helper_revision=4294967295')
                shell.requests()
                output = shell.change('GIT_BRANCH', 'exhausted')
                self.assertEqual(revisions(output)[-1], '')
                self.assertEqual(shell.requests(), [])

    def test_dead_receiver_disables_without_signal_or_block(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                shell.receiver.close()
                output = shell.command('true')
                self.assertEqual(revisions(output)[-1], '')
                self.assertNotIn(b'Broken pipe', output)
                self.assertIn(owner.PROMPT, shell.command('true'))

    def test_full_receiver_has_bounded_fail_closed_backpressure(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                filled = 0
                while True:
                    try:
                        filled += shell.sender.send(b'x' * 4096)
                    except BlockingIOError:
                        break
                    self.assertLess(filled, 2 * 1024 * 1024)
                output = shell.command('true')
                self.assertEqual(revisions(output)[-1], '')
                self.assertIn(owner.PROMPT, shell.command('true'))

    def test_near_limit_snapshot_and_total_overflow(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                for field, length in [('KUBECONFIG', 4096), ('HOMEDRIVE', 4096),
                                      ('HOMEPATH', 4096), ('USERPROFILE', 2400)]:
                    shell.command(f"builtin printf -v {field} '%*s' {length} ''; {field}=${{{field}// /x}}")
                    received = shell.requests()
                    self.assertEqual(received[-1][1][field], 'x' * length)
                self.assertGreater(sum(len(value) for value in received[-1][1].values()), 14688)
                shell.command("builtin printf -v USERPROFILE '%*s' 4096 ''; USERPROFILE=${USERPROFILE// /x}")
                self.assertTrue(all(value == '' for value in shell.requests()[-1][1].values()))

    def test_duplicate_source_endpoint_privacy_and_cwd_change(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), HelperShell(name) as shell:
                shell.requests()
                output = shell.command('builtin printf "ENDPOINT:%s\\n" "${AMX_SSH_HELPER_FD-unset}"')
                self.assertIn(b'ENDPOINT:unset', output)
                shell.requests()
                shell.command('mkdir -p "space quote\'"; cd "space quote\'"')
                self.assertTrue(shell.requests()[-1][1]['cwd'].endswith("/space quote'"))
                init = shell.initialization / ('rc.bash' if name == 'bash' else '.zshrc')
                output = shell.command('builtin source ' + shlex.quote(str(init)))
                self.assertNotIn('AMXSSHREV1|3|7|1', revisions(output))
                self.assertIn(owner.PROMPT, output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--helper', type=Path)
    parser.add_argument('--timings', type=Path)
    arguments, remaining = parser.parse_known_args()
    FIXTURES = arguments.fixtures.resolve(strict=True)
    HELPER = arguments.helper.resolve(strict=True) if arguments.helper else None
    result = unittest.main(argv=[__file__, *remaining], exit=False).result
    if arguments.timings:
        arguments.timings.parent.mkdir(parents=True, exist_ok=True)
        arguments.timings.write_text(json.dumps(TIMINGS, indent=2) + '\n', encoding='utf-8')
    raise SystemExit(not result.wasSuccessful())
