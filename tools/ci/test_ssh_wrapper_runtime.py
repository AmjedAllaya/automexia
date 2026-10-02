#!/usr/bin/env python3
"""Opt-in loopback OpenSSH tests using an existing, immutable fixture image.

The image must contain OpenSSH, Bash, Zsh, Fish, and a non-locked `fixture` user
(uid 1001, Bash account shell). This runner never pulls images or installs host
services. It owns one dedicated Docker network/container and temporary keys;
personal SSH configuration, agents, profiles and credentials are not used.

Supply --image sha256:... and --fixture bash=... (repeat for zsh/fish). Fixtures
are actual export_shell_fixture output. --application optionally exercises the
compiled CLI. This is Linux fixture evidence, not Windows server/release proof.
"""
from __future__ import annotations

import argparse
import base64
import json
import os
from pathlib import Path
import re
import shutil
import socket
import tempfile
import time
import unittest
import uuid

import qa_process
import test_ssh_bash_core as pty_owner

ROOT = Path(__file__).resolve().parents[2]
IMAGE: str | None = None
FIXTURES: dict[str, Path] = {}
APPLICATION: Path | None = None
LIMIT = 256 * 1024


def run(command: list[str], *, timeout: float = 30, check: bool = True) -> tuple[int, bytes]:
    output = bytearray()

    def consume(chunk: bytes) -> None:
        if len(output) + len(chunk) > LIMIT:
            raise ValueError('fixture command output exceeded its bound')
        output.extend(chunk)

    environment = dict(os.environ)
    environment.pop('SSH_AUTH_SOCK', None)
    result = qa_process.run(command, cwd=ROOT, timeout_seconds=timeout,
                            consume=consume, environment=environment)
    if result.timed_out or result.error or result.return_code is None:
        raise RuntimeError('fixture command did not complete with owned cleanup')
    if check and result.return_code != 0:
        raise RuntimeError(f'fixture command failed ({result.return_code}): {bytes(output)[-2048:]!r}')
    return result.return_code, bytes(output)


class Fixture:
    def __init__(self, image: str, *, executable_temp: bool = False) -> None:
        if not re.fullmatch(r'sha256:[0-9a-f]{64}', image):
            raise ValueError('fixture image must be an existing immutable image ID')
        self.image = image
        # Temporary helper upload alone opts into executable container tmpfs.
        # Ordinary SSH fixtures keep Docker's default noexec temporary mount.
        self.executable_temp = executable_temp
        self.token = uuid.uuid4().hex
        self.name = 'automexia-ssh-fixture-' + self.token
        self.network = ''
        self.container = ''
        self.network_requested = False
        self.container_requested = False
        self.temp = tempfile.TemporaryDirectory(prefix='automexia-ssh-runtime-')
        self.root = Path(self.temp.name)
        self.ssh = shutil.which('ssh')
        self.keygen = shutil.which('ssh-keygen')
        if not self.ssh or not self.keygen:
            self.temp.cleanup()
            raise RuntimeError('native OpenSSH fixture tools are unavailable')

    def __enter__(self) -> 'Fixture':
        try:
            _, identity = run(['docker', 'image', 'inspect', self.image, '--format', '{{.Id}}'])
            if identity.decode().strip() != self.image:
                raise RuntimeError('fixture image identity changed')
            for key in ['client', 'host']:
                run([self.keygen, '-q', '-t', 'ed25519', '-N', '', '-C', 'fixture', '-f', str(self.root / key)])
            server = self.root / 'server'
            server.mkdir(mode=0o755)
            shutil.copy2(self.root / 'host', server / 'host')
            shutil.copy2(self.root / 'client.pub', server / 'authorized_keys')
            (server / 'authorized_keys').chmod(0o644)
            (server / 'sshd_config').write_text(
                'Port 22\nHostKey /fixture/host\nPidFile /run/sshd.pid\n'
                'AuthorizedKeysFile /run/authorized_keys\nAllowUsers fixture\n'
                'PermitRootLogin no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n'
                'UsePAM no\nStrictModes yes\nAllowAgentForwarding no\nAllowTcpForwarding no\n'
                'X11Forwarding no\nPermitTunnel no\nPrintMotd no\nPrintLastLog no\nLogLevel ERROR\n',
                encoding='utf-8')
            home = self.root / 'home'
            (home / '.config/fish').mkdir(parents=True)
            (home / '.bashrc').write_text('PS1="AMX_AUDIT_PROMPT> "; PS2="SECONDARY> "\n', encoding='utf-8')
            (home / '.zshrc').write_text("PROMPT='AMX_AUDIT_PROMPT> '; RPROMPT=''\n", encoding='utf-8')
            (home / '.config/fish/config.fish').write_text(
                "function fish_prompt; printf 'AMX_AUDIT_PROMPT> '; end\n", encoding='utf-8')
            self.network = self.name
            self.network_requested = True
            run(['docker', 'network', 'create', '--driver', 'bridge', '--opt',
                 'com.docker.network.bridge.enable_ip_masquerade=false',
                 '--label', 'automexia.fixture=' + self.token, self.name])
            self.container_requested = True
            _, container = run(['docker', 'run', '--detach', '--pull=never', '--name', self.name,
                                '--label', 'automexia.fixture=' + self.token,
                                '--network', self.network, '--publish', '127.0.0.1::22',
                                '--read-only', '--tmpfs', '/run:rw,noexec,nosuid,size=8m',
                                '--mount', f'type=bind,source={server},target=/fixture,readonly',
                                '--mount', f'type=bind,source={home},target=/fixture-home,readonly',
                                '--tmpfs', '/tmp:rw,nosuid,size=16m' + (',exec' if self.executable_temp else ''),
                                '--tmpfs', '/home/fixture:rw,nosuid,size=16m,uid=1001,gid=1001',
                                '--pids-limit', '64', '--memory', '256m', '--cpus', '1',
                                '--security-opt', 'no-new-privileges=true', '--entrypoint', '/bin/sleep',
                                self.image, 'infinity'])
            self.container = container.decode().strip()
            if not re.fullmatch('[0-9a-f]{64}', self.container):
                raise RuntimeError('invalid fixture container identity')
            run(['docker', 'exec', self.container, 'cp', '-R', '/fixture-home/.', '/home/fixture/'])
            run(['docker', 'exec', self.container, 'chown', '-R', '1001:1001', '/home/fixture'])
            run(['docker', 'exec', self.container, 'mkdir', '-p', '/run/sshd'])
            # StrictModes requires root/the target account ownership. Bind mounts
            # retain the host uid, so copy this public key into root-owned tmpfs.
            run(['docker', 'exec', self.container, 'cp', '/fixture/authorized_keys', '/run/authorized_keys'])
            run(['docker', 'exec', self.container, 'chmod', '644', '/run/authorized_keys'])
            run(['docker', 'exec', '--detach', self.container, '/usr/sbin/sshd', '-D', '-f', '/fixture/sshd_config'])
            _, ports = run(['docker', 'inspect', '--format', '{{json .NetworkSettings.Ports}}', self.container])
            bindings = json.loads(ports)['22/tcp']
            if not isinstance(bindings, list) or len(bindings) != 1 or bindings[0]['HostIp'] != '127.0.0.1':
                raise RuntimeError('fixture listener is not exclusively loopback')
            self.port = int(bindings[0]['HostPort'])
            if not 1 <= self.port <= 65535:
                raise RuntimeError('invalid loopback fixture port')
            host_key = (self.root / 'host.pub').read_text(encoding='ascii').split()[:2]
            known = self.root / 'known_hosts'
            known.write_text(f'[127.0.0.1]:{self.port} ' + ' '.join(host_key) + '\n', encoding='ascii')
            self.config = self.root / 'ssh_config'
            self.config.write_text(
                f'Host fixture-host\n HostName 127.0.0.1\n Port {self.port}\n User fixture\n'
                f' IdentityFile {self.root / "client"}\n UserKnownHostsFile {known}\n'
                ' GlobalKnownHostsFile /dev/null\n StrictHostKeyChecking yes\n CheckHostIP yes\n'
                ' IdentityAgent none\n IdentitiesOnly yes\n BatchMode yes\n ForwardAgent no\n'
                ' ConnectTimeout 5\n ConnectionAttempts 1\n ControlMaster no\n ControlPath none\n'
                ' LogLevel ERROR\n', encoding='utf-8')
            deadline = time.monotonic() + 10
            while True:
                try:
                    with socket.create_connection(('127.0.0.1', self.port), timeout=0.5):
                        break
                except OSError:
                    if time.monotonic() >= deadline:
                        raise RuntimeError('isolated SSH listener did not become ready') from None
                    time.sleep(0.05)
            return self
        except BaseException:
            self.close()
            raise

    def close(self) -> None:
        errors = []
        if self.container_requested:
            target = self.container or self.name
            code, label = run(['docker', 'inspect', '--format', '{{index .Config.Labels "automexia.fixture"}}', target], check=False)
            if code == 0 and label.decode().strip() == self.token:
                code, _ = run(['docker', 'rm', '--force', target], check=False)
                if code: errors.append('container cleanup failed')
            elif code == 0:
                errors.append('container ownership changed')
            elif b'No such object' not in label:
                errors.append('container cleanup could not verify absence')
        if self.network_requested:
            code, label = run(['docker', 'network', 'inspect', '--format', '{{index .Labels "automexia.fixture"}}', self.network], check=False)
            if code == 0 and label.decode().strip() == self.token:
                code, _ = run(['docker', 'network', 'rm', self.network], check=False)
                if code: errors.append('network cleanup failed')
            elif code == 0:
                errors.append('network ownership changed')
            elif b'not found' not in label and b'No such network' not in label:
                errors.append('network cleanup could not verify absence')
        if errors:
            raise RuntimeError('; '.join(errors))
        self.container = self.network = ''
        self.container_requested = self.network_requested = False
        self.temp.cleanup()

    def __exit__(self, *_: object) -> None:
        self.close()

    def shell(self, name: str, application: Path | None = None) -> pty_owner.Shell:
        if application is not None:
            args = [str(application), '+ssh', '--shell', name, '--integration', 'required', '--', '-F', str(self.config), 'fixture-host']
        else:
            path = FIXTURES[name]
            if path.stat().st_size > 64 * 1024:
                raise ValueError('oversized actual-generator fixture')
            lines = path.read_bytes().splitlines()
            if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
                raise ValueError('invalid actual-generator fixture')
            source = base64.b64decode(lines[2], validate=True).decode('utf-8')
            if len(source.encode()) > 12 * 1024 or '\0' in source or '@@' in source:
                raise ValueError('invalid generated bootstrap')
            args = [self.ssh, '-F', str(self.config), '-tt', 'fixture-host', source]
        return pty_owner.Shell(launch_arguments=args, echo_input=False)


def exit_status(shell: pty_owner.Shell, code: int) -> int:
    os.write(shell.fd, f'exit {code}\n'.encode())
    deadline = time.monotonic() + 5
    while True:
        status = os.waitid(os.P_PID, shell.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        if status is not None:
            if status.si_code != os.CLD_EXITED:
                raise AssertionError('SSH child did not exit normally')
            return status.si_status
        if time.monotonic() >= deadline:
            raise TimeoutError('SSH child did not exit within the fixture deadline')
        time.sleep(0.01)


class RuntimeContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not IMAGE or not FIXTURES or os.name != 'posix':
            raise unittest.SkipTest('requires explicit Unix Docker image and actual Rust shell fixtures')
        cls.fixture = Fixture(IMAGE)
        cls.fixture.__enter__()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.fixture.close()

    def test_generated_shell_lifecycle_cwd_identity_and_exit(self) -> None:
        for name in FIXTURES:
            with self.subTest(shell=name), self.fixture.shell(name) as shell:
                ready = shell.initial.find(pty_owner.READY)
                self.assertGreaterEqual(ready, 0)
                ready_end = shell.initial.index(b'\x07', ready) + 1
                self.assertIn('AMXSSHUSER1|3|7|fixture', pty_owner.user_frames(shell.initial[ready_end:]))
                data = shell.command('false')
                self.assertIn(b'\x1b]133;C', data)
                self.assertIn(b'\x1b]133;D;1\x07', data)
                data = shell.command('mkdir -p "$HOME/space \' quote"; cd "$HOME/space \' quote"')
                self.assertTrue(any(path.endswith("/space ' quote") for path in pty_owner.cwd_frames(data)))
                data = shell.command('export GIT_BRANCH=fixture-branch KUBECONTEXT=fixture-cluster TF_WORKSPACE=fixture-workspace')
                context = pty_owner.context_frames(data)[-1]
                self.assertEqual(context['git_branch'], 'fixture-branch')
                self.assertEqual(context['kubernetes_context'], 'fixture-cluster')
                self.assertEqual(context['terraform_workspace'], 'fixture-workspace')
                data = shell.command("printf 'TERMINAL=%s:%s\\n' \"$COLORTERM\" \"$TERM_PROGRAM\"")
                self.assertIn(b'TERMINAL=truecolor:Automexia', data)
                self.assertEqual(exit_status(shell, 7), 7)

    def test_generated_shell_cancellation(self) -> None:
        for name in FIXTURES:
            with self.subTest(shell=name), self.fixture.shell(name) as shell:
                shell.send(b'printf RUNNING; sleep 30\n', b'RUNNING')
                data = shell.send(b'\x03')
                self.assertIn(b'\x1b]133;D;130\x07', data)
                self.assertNotIn(b'\x1b]133;D', shell.command(''))

    def test_native_remote_command_exit_is_preserved(self) -> None:
        args = [self.fixture.ssh, '-F', str(self.fixture.config), '-T', 'fixture-host', 'printf NATIVE_RESULT; exit 7']
        if APPLICATION is not None:
            args = [str(APPLICATION), '+ssh', '--integration', 'off', '--', *args[1:]]
        code, output = run(args, check=False)
        self.assertEqual(code, 7, output[-2048:])
        self.assertEqual(output, b'NATIVE_RESULT')

    def test_application_interactive_wrapper(self) -> None:
        if APPLICATION is None:
            self.skipTest('compiled application was not supplied; generator evidence only')
        for name in FIXTURES:
            with self.subTest(shell=name), self.fixture.shell(name, APPLICATION) as shell:
                user = pty_owner.user_frames(shell.initial)[-1].split('|', 3)
                self.assertEqual((user[0], user[3]), ('AMXSSHUSER1', 'fixture'))
                scope = re.search(rb'\x1b\]1337;SetUserVar=terminal_scope_v1=([^\x07]+)\x07', shell.initial)
                self.assertIsNotNone(scope)
                begin = base64.b64decode(scope[1], validate=True).decode().split('|')
                self.assertEqual((begin[0], begin[1], begin[3], begin[4], begin[5]),
                                 ('AMXSCOPE1', 'begin', user[1], user[2], name))
                self.assertIn(b'\x1b]133;D;1\x07', shell.command('false'))
                data = shell.command('export GIT_BRANCH=fixture-branch')
                contexts = re.findall(re.escape(pty_owner.CONTEXT) + rb'([^\x07]*)\x07', data)
                expected = f'AMXSSHCTX1|{user[1]}|{user[2]}|\ngit_branch=fixture-branch\n'
                self.assertTrue(any(base64.b64decode(value, validate=True).decode().startswith(expected)
                                    for value in contexts))
                shell.send(b'printf RUNNING; sleep 30\n', b'RUNNING')
                self.assertIn(b'\x1b]133;D;130\x07', shell.send(b'\x03'))
                self.assertEqual(exit_status(shell, 9), 9)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--fixture', action='append', required=True, metavar='SHELL=FILE')
    parser.add_argument('--application', type=Path)
    options, remaining = parser.parse_known_args()
    IMAGE = options.image
    APPLICATION = options.application.resolve() if options.application else None
    for value in options.fixture:
        name, separator, path = value.partition('=')
        if not separator or name not in {'bash', 'zsh', 'fish'} or name in FIXTURES:
            parser.error('each fixture must specify a unique supported shell and generated file')
        FIXTURES[name] = Path(path).resolve()
    unittest.main(argv=[__file__, *remaining])
