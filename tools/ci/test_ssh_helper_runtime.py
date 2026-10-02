#!/usr/bin/env python3
"""Opt-in actual helper upload/provider tests on an owned strict loopback SSH server."""
from __future__ import annotations

import argparse
import base64
import os
from pathlib import Path
import re
import select
import signal
import struct
import sys
import time
import unittest

import test_ssh_bash_core as pty_owner
import test_ssh_wrapper_runtime as runtime

IMAGE = ''
APPLICATION: Path | None = None
HELPER: Path | None = None
CONTEXT = re.compile(rb'\x1b\]1337;SetUserVar=automexia_ssh_context_v2=([^\x07]+)\x07')
FIELDS = ('git_branch kubernetes_context kubernetes_namespace docker_context terraform_workspace '
          'environment aws_profile azure_cloud gcp_project aws_region azure_subscription '
          'azure_region gcp_region production').split()


def contexts(output: bytes) -> list[tuple[tuple[str, str, int], dict[str, str]]]:
    result = []
    for encoded in CONTEXT.findall(output):
        value = base64.b64decode(encoded, validate=True)
        if len(value) > 8192:
            raise AssertionError('helper context exceeds fixture output bound')
        lines = value.decode('utf-8').splitlines()
        header = lines[0].split('|')
        fields = [line.split('=', 1) for line in lines[1:]]
        if len(header) != 4 or header[0] != 'AMXSSHCTX2' or [field[0] for field in fields] != FIELDS:
            raise AssertionError('invalid actual helper context contract')
        result.append(((header[1], header[2], int(header[3])), dict(fields)))
    return result


def wait_context(shell: pty_owner.Shell, expected: dict[str, str], *, after: int = 0,
                 timeout: float = 8) -> tuple[tuple[str, str, int], dict[str, str]]:
    deadline = time.monotonic() + timeout
    while True:
        for header, fields in reversed(contexts(bytes(shell.all_output[after:]))):
            if all(fields.get(key) == value for key, value in expected.items()):
                return header, fields
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            found = [fields for _, fields in contexts(bytes(shell.all_output[after:]))]
            raise AssertionError('helper did not publish expected fictional fixture facts: ' + repr(found[-2:]))
        if select.select([shell.fd], [], [], min(0.2, remaining))[0]:
            chunk = os.read(shell.fd, 8192)
            if not chunk:
                raise AssertionError('helper exited before publishing context')
            shell.all_output.extend(chunk)
            shell.pending.extend(chunk)
            if len(shell.all_output) > pty_owner.MAX_OUTPUT:
                raise AssertionError('helper fixture output limit')


class HelperRuntimeContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not IMAGE and APPLICATION is None and HELPER is None:
            raise unittest.SkipTest('requires explicit image, application and helper fixtures')
        if not IMAGE or APPLICATION is None or HELPER is None:
            raise ValueError('all image, application and helper fixtures are required')
        if sys.platform != 'linux':
            raise RuntimeError('configured helper runtime tests require native Linux')
        cls.fixture = runtime.Fixture(IMAGE, executable_temp=True)
        cls.fixture.__enter__()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.fixture.close()

    def put(self, name: str, value: str) -> None:
        # All paths are fixed fixture-relative constants; no remote path output.
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('invalid owned fixture path')
        source = self.fixture.root / 'server' / 'fixture-data'
        source.write_text(value, encoding='utf-8')
        target = '/home/fixture/' + name
        runtime.run(['docker', 'exec', self.fixture.container, 'mkdir', '-p', str(Path(target).parent)])
        runtime.run(['docker', 'exec', self.fixture.container, 'cp', '/fixture/fixture-data', target])
        runtime.run(['docker', 'exec', self.fixture.container, 'chown', '1001:1001', target])

    def shell(self, name: str) -> pty_owner.Shell:
        return pty_owner.Shell(launch_arguments=[str(APPLICATION), '+ssh', '--shell', name,
            '--integration', 'required', '--helper-upload', str(HELPER), '--', '-F',
            str(self.fixture.config), 'fixture-host'], echo_input=False)

    def assert_no_remote_leases(self) -> None:
        _, output = runtime.run(['docker', 'exec', self.fixture.container, 'find', '/tmp', '-maxdepth', '1',
                                 '-name', 'automexia-ssh*', '-print'])
        self.assertEqual(output, b'', 'helper left uploaded/startup files behind')

    def test_passive_discovery_idle_refresh_revision_changes_and_exact_cleanup(self) -> None:
        import fcntl
        import termios
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name):
                self.put('.git/HEAD', 'ref: refs/heads/fixture-main\n')
                self.put('.kube/config', 'current-context: fixture-cluster\ncontexts:\n- name: fixture-cluster\n  context:\n    namespace: fixture-namespace\n')
                self.put('.docker/config.json', '{"currentContext":"fixture-docker"}\n')
                self.put('.terraform/environment', 'fixture-workspace\n')
                with self.shell(name) as shell:
                    initial, _ = wait_context(shell, {'git_branch': 'fixture-main',
                        'kubernetes_context': 'fixture-cluster', 'kubernetes_namespace': 'fixture-namespace',
                        'docker_context': 'fixture-docker', 'terraform_workspace': 'fixture-workspace'})
                    self.assertTrue(any(frame.endswith('|fixture') for frame in pty_owner.user_frames(shell.initial)))
                    before = len(shell.all_output)
                    os.write(shell.fd, b'printf TYPED_SAFE')
                    self.put('.git/HEAD', 'ref: refs/heads/fixture-idle\n')
                    refreshed, _ = wait_context(shell, {'git_branch': 'fixture-idle'}, after=before)
                    self.assertEqual(initial, refreshed, 'idle provider refresh changed the shell revision')
                    self.assertIn(b'TYPED_SAFE', shell.send(b'\n'))
                    before = len(shell.all_output)
                    shell.command('export DOCKER_CONTEXT=selected-docker TF_WORKSPACE=selected-workspace')
                    changed, _ = wait_context(shell, {'docker_context': 'selected-docker',
                        'terraform_workspace': 'selected-workspace'}, after=before)
                    self.assertGreater(changed[2], initial[2])
                    before = len(shell.all_output)
                    shell.command('mkdir -p "space quote\'"; cd "space quote\'"')
                    moved, _ = wait_context(shell, {'git_branch': 'fixture-idle'}, after=before)
                    self.assertGreater(moved[2], changed[2])
                    self.assertTrue(any(path.endswith("/space quote'") for path in pty_owner.cwd_frames(bytes(shell.all_output))))
                    self.assertIn(b'\x1b]133;D;1\x07', shell.command('false'))
                    shell.send(b'printf RUNNING; sleep 30\n', b'RUNNING')
                    self.assertIn(b'\x1b]133;D;130\x07', shell.send(b'\x03'))
                    fcntl.ioctl(shell.fd, termios.TIOCSWINSZ, struct.pack('HHHH', 41, 119, 0, 0))
                    # Zsh redraws its prompt on SIGWINCH before consuming input.
                    # Require the remote stty result, not that earlier prompt.
                    shell.send(b'stty size\n', b'41 119')
                    shell.until(pty_owner.PROMPT)
                    before = len(shell.all_output)
                    shell.command("export AWS_PROFILE=$(builtin printf '\\001invalid')")
                    _, cleared = wait_context(shell, {field: '0' if field == 'production' else '' for field in FIELDS}, after=before)
                    self.assertTrue(all(not value or field == 'production' for field, value in cleared.items()))
                    self.assertEqual(runtime.exit_status(shell, 7), 7)
                self.assert_no_remote_leases()

    def test_cancelled_helper_session_retires_remote_shell_and_owned_files(self) -> None:
        for name in ('bash', 'zsh'):
            with self.subTest(shell=name), self.shell(name) as shell:
                wait_context(shell, {})
                output = shell.command("builtin printf 'REMOTE_PIDS:%s:%s\\n' $$ $PPID")
                identity = re.search(rb'REMOTE_PIDS:([0-9]+):([0-9]+)', output)
                self.assertIsNotNone(identity)
                remote_pids = [int(value) for value in identity.groups()]
                os.kill(shell.pid, signal.SIGTERM)
                deadline = time.monotonic() + 8
                while os.waitid(os.P_PID, shell.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is None:
                    if time.monotonic() >= deadline:
                        self.fail('cancelled local wrapper did not retire')
                    time.sleep(0.05)
            deadline = time.monotonic() + 8
            while True:
                _, output = runtime.run(['docker', 'exec', self.fixture.container, 'find', '/tmp', '-maxdepth', '1',
                                         '-name', 'automexia-ssh*', '-print'])
                if not output:
                    break
                if time.monotonic() >= deadline:
                    self.fail('cancelled SSH helper left owned remote files behind')
                time.sleep(0.05)
            for pid in remote_pids:
                code, state = runtime.run(['docker', 'exec', self.fixture.container, 'cat',
                                          f'/proc/{pid}/stat'], check=False)
                if code == 0:
                    self.assertEqual(state.rsplit(b')', 1)[-1].split()[0], b'Z',
                                     'cancelled remote helper or shell remained alive')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--application', type=Path, required=True)
    parser.add_argument('--helper', type=Path, required=True)
    arguments, remaining = parser.parse_known_args()
    if sys.platform != 'linux':
        parser.error('this opt-in fixture requires native Linux process identity checks')
    IMAGE = arguments.image
    APPLICATION = arguments.application.resolve(strict=True)
    HELPER = arguments.helper.resolve(strict=True)
    unittest.main(argv=[__file__, *remaining])
