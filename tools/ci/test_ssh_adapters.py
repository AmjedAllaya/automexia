#!/usr/bin/env python3
"""Actual generated Zsh/Fish adapters in the existing bounded PTY harness.

Use --fixtures DIR with export_shell_fixture outputs named zsh.fixture and
fish.fixture. Discovery without those artifacts does not claim native evidence.
"""
from __future__ import annotations
import argparse
import base64
import os
from pathlib import Path
import re
import shutil
import tempfile
import unittest

import test_ssh_bash_core as harness

FIXTURES: Path | None = None


def generated(shell: str) -> tuple[str, str]:
    if FIXTURES is None:
        raise unittest.SkipTest('requires actual Rust-generated adapter fixtures')
    data = (FIXTURES / (shell + '.fixture')).read_bytes()
    if len(data) > 64 * 1024:
        raise ValueError('fixture budget exceeded')
    lines = data.splitlines()
    if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
        raise ValueError('invalid generated fixture')
    core, bootstrap = (base64.b64decode(line, validate=True).decode() for line in lines[1:])
    if '@@' in core or '@@' in bootstrap or len(bootstrap.encode()) > 12 * 1024:
        raise ValueError('invalid generated source')
    return core, bootstrap


class NativeShell(harness.Shell):
    """Reuse read/input/output limits and process-group retirement unchanged."""
    def __init__(self, shell: str, profile: str = '', *, initial_umask: int = 0o022, count_encoding: bool = False):
        import pty
        import termios
        executable = shutil.which(shell)
        if os.name != 'posix' or not executable:
            raise unittest.SkipTest('requires native Unix ' + shell)
        _, bootstrap = generated(shell)
        self.temporary = tempfile.TemporaryDirectory(prefix='automexia-ssh-adapter-')
        self.root = Path(self.temporary.name)
        self.cwd = self.root / 'project'
        self.cwd.mkdir()
        self.tmp = self.root / 'tmp'
        self.tmp.mkdir()
        (self.tmp / 'unrelated-sentinel').write_text('preserve', encoding='utf-8')
        self.all_output = bytearray()
        self.pending = bytearray()
        self.closed = False
        if shell == 'zsh':
            (self.root / '.zshrc').write_text('PROMPT="AMX_AUDIT_PROMPT> "\n' + profile, encoding='utf-8')
        else:
            config = self.root / '.config/fish'
            config.mkdir(parents=True)
            (config / 'config.fish').write_text('function fish_prompt\n builtin printf "AMX_AUDIT_PROMPT> "\nend\n' + profile, encoding='utf-8')
        env = {'HOME': str(self.root), 'PATH': '/usr/bin:/bin', 'TERM': 'xterm-256color',
               'TMPDIR': str(self.tmp), 'LC_ALL': 'C.UTF-8', 'USER': 'fixture',
               'XDG_CONFIG_HOME': str(self.root / '.config'), 'ZDOTDIR': str(self.root)}
        if count_encoding:
            binary_dir = self.root / 'bin'
            binary_dir.mkdir()
            encoder = binary_dir / 'base64'
            encoder.write_text('#!/bin/sh\nprintf x >> "$HOME/encode-count"\nexec /usr/bin/base64 "$@"\n', encoding='utf-8')
            encoder.chmod(0o700)
            env['PATH'] = str(binary_dir) + ':/usr/bin:/bin'
        try:
            self.pid, self.fd = pty.fork()
            if self.pid == 0:
                os.chdir(self.cwd)
                os.umask(initial_umask)
                try:
                    os.execve(executable, [executable, '-c', bootstrap], env)
                finally:
                    os._exit(127)
            attributes = termios.tcgetattr(self.fd)
            attributes[3] &= ~(termios.ECHO | termios.ECHONL)
            termios.tcsetattr(self.fd, termios.TCSANOW, attributes)
            self.initial = self.until(harness.PROMPT)
        except BaseException:
            if getattr(self, 'pid', 0) > 0:
                self.close()
            else:
                self.temporary.cleanup()
            raise


@unittest.skipUnless(os.name == 'posix', 'requires native Unix PTY')
class AdapterTests(unittest.TestCase):
    def test_unchanged_prompt_recovers_optional_metadata_without_reencoding(self):
        corrupt = b'\x1b]1337;SetUserVar=automexia_ssh_user=!!!!\x07\x1b]1337;SetUserVar=automexia_ssh_context=!!!!\x07'
        command = "printf '\\e]1337;SetUserVar=automexia_ssh_user=!!!!\\a\\e]1337;SetUserVar=automexia_ssh_context=!!!!\\a'"
        for shell in ('zsh', 'fish'):
            with self.subTest(shell=shell), NativeShell(shell, count_encoding=True) as session:
                initial_context = context_values(session.initial)[-1]
                initial_user = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_user=([^\x07]+)\x07', session.initial)[-1]
                count_before = (session.root / 'encode-count').read_bytes()
                output = session.command(command)
                self.assertIn(corrupt, output)
                recovery = output.split(corrupt, 1)[1]
                self.assertEqual(context_values(recovery), [initial_context])
                users = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_user=([^\x07]+)\x07', recovery)
                self.assertEqual(users, [initial_user])
                self.assertEqual((session.root / 'encode-count').read_bytes(), count_before)

    def test_zsh_native_terminal_identity_ownership_declines_integration(self):
        for profile in ('readonly COLORTERM=fixture-native\n', 'typeset -a TERM_PROGRAM=(fixture-native second)\n'):
            with self.subTest(profile=profile), NativeShell('zsh', profile) as session:
                self.assertEqual(harness.advertised(session.initial), 0)
                self.assertNotIn(b'read-only', session.initial)
                self.assertNotIn(b'readonly', session.initial)
                self.assertIn(b'fixture-native', session.command('print -r -- $COLORTERM $TERM_PROGRAM'))

    def test_context_snapshot_has_fixed_field_positions_and_terminal_identity(self):
        profiles = {
            'zsh': 'export KUBE_CONTEXT=fixture-cluster TF_WORKSPACE=fixture-space\n',
            'fish': 'set -gx KUBE_CONTEXT fixture-cluster\nset -gx TF_WORKSPACE fixture-space\n',
        }
        for shell, profile in profiles.items():
            with self.subTest(shell=shell), NativeShell(shell, profile) as session:
                values = context_values(session.initial)
                self.assertEqual(len(values), 1)
                self.assertEqual(values[0]['git_branch'], '')
                self.assertEqual(values[0]['kubernetes_context'], 'fixture-cluster')
                self.assertEqual(values[0]['terraform_workspace'], 'fixture-space')
                self.assertEqual(values[0]['gcp_project'], '')
                output = session.command('printf "%s|%s|%s\\n" "$TERM" "$COLORTERM" "$TERM_PROGRAM"')
                self.assertIn(b'xterm-256color|truecolor|Automexia', output)
                self.assertEqual(context_values(output), values)

    def test_context_changes_clear_values_and_reject_control_or_overlong_text(self):
        commands = {
            'zsh': ('export GIT_BRANCH=fixture-branch AWS_PROFILE=fixture-cloud',
                    'unset GIT_BRANCH AWS_PROFILE; export KUBECONTEXT=$\'bad\\nvalue\'; export TF_WORKSPACE=' + 'x' * 257),
            'fish': ('set -gx GIT_BRANCH fixture-branch; set -gx AWS_PROFILE fixture-cloud',
                     'set -e GIT_BRANCH AWS_PROFILE; set -gx KUBECONTEXT "bad\\nvalue"; set -gx TF_WORKSPACE ' + 'x' * 257),
        }
        for shell, (set_values, clear_values) in commands.items():
            # Fish double quotes retain a backslash-n, so emit the control with its
            # own builtin string unescaper rather than relying on shell escaping.
            if shell == 'fish':
                clear_values = clear_values.replace('"bad\\nvalue"', '(string unescape "bad\\\\nvalue" | string collect)')
            with self.subTest(shell=shell), NativeShell(shell) as session:
                changed = context_values(session.command(set_values))
                self.assertEqual(changed[-1]['git_branch'], 'fixture-branch')
                self.assertEqual(changed[-1]['aws_profile'], 'fixture-cloud')
                cleared = context_values(session.command(clear_values))
                self.assertEqual(cleared[-1]['git_branch'], '')
                self.assertEqual(cleared[-1]['aws_profile'], '')
                self.assertEqual(cleared[-1]['kubernetes_context'], '')
                self.assertEqual(cleared[-1]['terraform_workspace'], '')

    def test_generated_bootstrap_preserves_visible_native_prompt_and_status(self):
        for shell in ('zsh', 'fish'):
            with self.subTest(shell=shell), NativeShell(shell) as session:
                self.assertEqual(harness.advertised(session.initial), 7)
                self.assertIn(b'\x1b]133;A;aid=1\x07 \r\n', session.initial)
                failed = session.command('false')
                self.assertEqual(failed.count(b'\x1b]133;C\x07'), 1)
                self.assertEqual(failed.count(b'\x1b]133;D;1\x07'), 1)
                succeeded = session.command('true')
                self.assertEqual(succeeded.count(b'\x1b]133;D;0\x07'), 1)

    def test_scoped_directory_and_user_do_not_emit_local_environment_or_osc7(self):
        for shell in ('zsh', 'fish'):
            with self.subTest(shell=shell), NativeShell(shell) as session:
                output = session.initial + session.command('cd /')
                cwd_values = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_cwd=([^\x07]+)\x07', output)
                self.assertTrue(cwd_values)
                self.assertEqual(base64.b64decode(cwd_values[-1]), b'AMXSSHCWD1|3|7|/')
                user_values = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_user=([^\x07]+)\x07', output)
                self.assertTrue(user_values)
                self.assertLess(output.index(b'SetUserVar=automexia_ssh_ready='), output.index(b'SetUserVar=automexia_ssh_user='))
                self.assertTrue(base64.b64decode(user_values[0]).startswith(b'AMXSSHUSER1|3|7|'))
                self.assertNotIn(b'\x1b]7;', output)
                self.assertNotIn(b' automexia_env_', output)

    def test_native_profile_runs_once_and_keeps_its_umask(self):
        for shell in ('zsh', 'fish'):
            profile = 'builtin printf "PROFILE_ONCE\\n"\numask 027\n'
            if shell == 'fish':
                profile = 'if status is-interactive\n' + profile + 'end\n'
            with self.subTest(shell=shell), NativeShell(shell, profile) as session:
                self.assertEqual(session.initial.count(b'PROFILE_ONCE'), 1)
                self.assertIn(b'027', session.command('umask'))
                self.assertFalse(any(path.name.startswith('automexia-ssh.') for path in session.tmp.iterdir()))
                self.assertEqual((session.tmp / 'unrelated-sentinel').read_text(), 'preserve')

    def test_fish_keeps_native_noninteractive_and_interactive_config_loading(self):
        # Fish itself loads config for the SSH account's command interpreter and
        # again for its interactive child. Skipping that child config loses user
        # functions; neither arbitrary profile state nor function source is copied.
        with NativeShell('fish', 'builtin printf "NATIVE_STARTUP\\n"\n') as session:
            self.assertEqual(session.initial.count(b'NATIVE_STARTUP'), 2)

    def test_empty_enter_and_ctrl_l_do_not_fabricate_completion(self):
        for shell in ('zsh', 'fish'):
            with self.subTest(shell=shell), NativeShell(shell) as session:
                for payload in (b'\n', b'\x0c'):
                    output = session.send(payload)
                    self.assertNotIn(b'\x1b]133;C\x07', output)
                    self.assertNotIn(b'\x1b]133;D;', output)

    def test_native_status_hooks_keep_original_failure(self):
        profiles = {
            'zsh': 'native_hook() { builtin printf "NATIVE_STATUS=%s\\n" "$?"; }; precmd_functions+=(native_hook)\n',
            'fish': 'function native_hook --on-event fish_postexec\n builtin printf "NATIVE_STATUS=%s\\n" $status\nend\n',
        }
        for shell, profile in profiles.items():
            with self.subTest(shell=shell), NativeShell(shell, profile) as session:
                output = session.command('false')
                self.assertIn(b'NATIVE_STATUS=1', output)
                self.assertIn(b'\x1b]133;D;1\x07', output)


def context_values(output: bytes) -> list[dict[str, str]]:
    values = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_context=([^\x07]+)\x07', output)
    result = []
    for encoded in values:
        text = base64.b64decode(encoded, validate=True).decode('utf-8')
        header, *lines = text.splitlines()
        if header != 'AMXSSHCTX1|3|7|' or len(lines) != 9:
            raise AssertionError('invalid complete context snapshot')
        result.append(dict(line.split('=', 1) for line in lines))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixtures', type=Path, required=True)
    args, rest = parser.parse_known_args()
    FIXTURES = args.fixtures
    unittest.main(argv=[__file__, *rest])
