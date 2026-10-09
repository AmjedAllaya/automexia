#!/usr/bin/env python3
"""Controlling-PTY regressions. No SSH server or personal profile is accessed.

The existing CI unittest discovery runs core resource tests. The canonical SSH
scope additionally supplies ACTUAL Rust output from examples/export_shell_fixture.
No Rust-template regex reconstruction is used by this repository test runner.
"""
from __future__ import annotations
import argparse
import base64
import errno
import os
from pathlib import Path
import re
import select
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

import qa_process

ROOT = Path(__file__).resolve().parents[2]
CORE = ''
BOOTSTRAP = ''
BASH = shutil.which('bash') if os.name == 'posix' else None
TIMEOUT = 5.0
MAX_OUTPUT = 256 * 1024
PROMPT = b'AMX_AUDIT_PROMPT> '
READY = b'\x1b]1337;SetUserVar=automexia_ssh_ready='
CWD = b'\x1b]1337;SetUserVar=automexia_ssh_cwd='
USER = b'\x1b]1337;SetUserVar=automexia_ssh_user='
CONTEXT = b'\x1b]1337;SetUserVar=automexia_ssh_context='
CONTEXT_FIELDS = ('git_branch', 'kubernetes_context', 'kubernetes_namespace', 'docker_context',
                  'terraform_workspace', 'environment', 'aws_profile', 'azure_cloud', 'gcp_project')
EMPTY_CONTEXT = 'AMXSSHCTX1|3|7|\n' + ''.join(field + '=\n' for field in CONTEXT_FIELDS)


def fixture_core() -> str:
    source = (ROOT / 'automexia-ssh-integration/resources/bash-core.bash').read_text(encoding='utf-8')
    values = {
        '@@RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|1|7').decode(),
        '@@PROMPT_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|1|5').decode(),
        '@@DOWNGRADE_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|2|5').decode(),
        '@@REVOKE_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|3|0').decode(),
        '@@CLEAR_CWD@@': base64.b64encode(b'AMXSSHCWD1|3|7|').decode(),
        '@@CLEAR_USER@@': base64.b64encode(b'AMXSSHUSER1|3|7|').decode(),
        '@@CLEAR_CONTEXT@@': base64.b64encode(EMPTY_CONTEXT.encode()).decode(),
        '@@PANE@@': '3', '@@GENERATION@@': '7', '@@PATH_LIMIT@@': '4000',
    }
    for key, value in values.items(): source = source.replace(key, value)
    if '@@' in source: raise ValueError('unknown shell fixture token')
    return source


def load_generated(path: Path) -> None:
    global CORE, BOOTSTRAP
    if path.stat().st_size > 64 * 1024: raise ValueError('oversized generated fixture')
    lines = path.read_bytes().splitlines()
    if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
        raise ValueError('invalid ACTUAL-generator fixture framing')
    core = base64.b64decode(lines[1], validate=True).decode('utf-8')
    bootstrap = base64.b64decode(lines[2], validate=True).decode('utf-8')
    if core != fixture_core(): raise ValueError('compiled core differs from canonical resource fixture')
    if core not in bootstrap or len(bootstrap.encode()) > 12 * 1024 or '@@' in bootstrap:
        raise ValueError('invalid generated bootstrap identity/budget')
    CORE, BOOTSTRAP = core, bootstrap


def setUpModule() -> None:
    global CORE
    if not BASH: raise unittest.SkipTest('requires native Unix Bash and a controlling PTY')
    version = subprocess.run([BASH, '--noprofile', '--norc', '-c', 'printf "%s.%s" "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}"'],
                             capture_output=True, timeout=TIMEOUT, check=True).stdout.decode()
    if tuple(map(int, version.split('.'))) < (5, 1):
        raise unittest.SkipTest('requires Bash >=5.1; no shell adapter evidence')
    if not CORE: CORE = fixture_core()

class Shell:
    def __init__(self, profile: str = '', *, bootstrap: bool = False,
                 fail_mktemp: bool = False, cwd_name: str = 'project', initial_umask: int = 0o022, extra_bins: dict[str, str] | None = None,
                 echo_input: bool = False, user_name: str = 'remote-user',
                 launch_arguments: list[str] | None = None) -> None:
        import pty
        import termios
        self.temporary = tempfile.TemporaryDirectory(prefix='automexia-ssh-audit-')
        self.root = Path(self.temporary.name)
        self.cwd = self.root / cwd_name
        self.cwd.mkdir()
        self.tmp = self.root / 'tmp'
        self.tmp.mkdir()
        self.all_output = bytearray()
        self.pending = bytearray()
        self.closed = False
        rc = self.root / 'fixture.rc'
        startup = 'PS1="AMX_AUDIT_PROMPT> "\n' + profile + '\n'
        core = CORE
        if bootstrap:
            (self.root / '.bashrc').write_text(startup, encoding='utf-8')
            script = BOOTSTRAP
            if not script:
                self.temporary.cleanup()
                raise unittest.SkipTest('actual Rust-generated bootstrap fixture is not supplied')
            argv = ['/bin/sh', '-c', script, 'automexia-ssh-audit-bootstrap']
        else:
            rc.write_text(startup + core, encoding='utf-8')
            argv = [BASH, '--noprofile', '--rcfile', str(rc), '-i']
        if launch_arguments is not None:
            if not launch_arguments or len(launch_arguments) > 256 or sum(len(arg.encode()) for arg in launch_arguments) > 65536:
                self.temporary.cleanup()
                raise ValueError('Fixture launch argument budget exceeded')
            argv = launch_arguments
        bin_dir = self.root / 'bin'
        bin_dir.mkdir()
        # Constrain bash resolution to the explicitly selected test executable.
        (bin_dir / 'bash').symlink_to(BASH)
        if fail_mktemp:
            shim = bin_dir / 'mktemp'
            shim.write_text('#!/bin/sh\nexit 1\n', encoding='utf-8')
            shim.chmod(0o700)
        for name, body in (extra_bins or {}).items():
            target = bin_dir / name
            target.unlink(missing_ok=True)
            target.write_text(body, encoding='utf-8')
            target.chmod(0o700)
        env = {'HOME': str(self.root), 'PATH': f'{bin_dir}:/usr/bin:/bin',
               'TERM': 'xterm-256color', 'TMPDIR': str(self.tmp), 'LC_ALL': 'C.UTF-8',
               'USER': user_name}
        try:
            self.pid, self.fd = pty.fork()
            if self.pid == 0:
                os.chdir(self.cwd)
                os.umask(initial_umask)
                try:
                    os.execve(argv[0], argv, env)
                finally:
                    os._exit(127)
            attributes = termios.tcgetattr(self.fd)
            if echo_input:
                attributes[3] |= termios.ECHO
            else:
                attributes[3] &= ~(termios.ECHO | termios.ECHONL)
            termios.tcsetattr(self.fd, termios.TCSANOW, attributes)
            self.initial = self.until(PROMPT)
        except BaseException:
            if getattr(self, 'pid', 0) > 0:
                self.close()
            else:
                self.temporary.cleanup()
            raise

    def until(self, marker: bytes) -> bytes:
        deadline = time.monotonic() + TIMEOUT
        while True:
            at = self.pending.find(marker)
            if at >= 0:
                end = at + len(marker)
                result = bytes(self.pending[:end])
                del self.pending[:end]
                return result
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError('Bash fixture did not reach its prompt within five seconds')
            if not select.select([self.fd], [], [], remaining)[0]:
                continue
            try:
                data = os.read(self.fd, 8192)
            except OSError as error:
                if error.errno == errno.EIO:
                    raise RuntimeError('Bash exited before the expected prompt') from error
                raise
            if not data:
                raise RuntimeError('Bash output ended before the expected prompt')
            self.all_output.extend(data)
            self.pending.extend(data)
            if len(self.all_output) > MAX_OUTPUT:
                raise RuntimeError('Bash fixture exceeded the output ceiling')

    def command(self, command: str) -> bytes:
        return self.send((command + '\n').encode('utf-8'), PROMPT)

    def send(self, data: bytes, marker: bytes = PROMPT) -> bytes:
        if len(data) > 8192:
            raise ValueError('Fixture input budget exceeded')
        while data:
            count = os.write(self.fd, data)
            data = data[count:]
        return self.until(marker)

    def wait_for_exit(self, timeout: float = TIMEOUT) -> int:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            status = qa_process.pinned_exit_status(self.pid)
            if status is not None:
                return status
            time.sleep(.005)
        raise TimeoutError('Fixture child did not exit')

    def close(self) -> None:
        if self.closed:
            return
        # Keep our child unreaped until its fixture process group is retired.
        qa_process.terminate_pinned_group(self.pid)
        deadline = time.monotonic() + TIMEOUT
        while True:
            child, _ = os.waitpid(self.pid, os.WNOHANG)
            if child:
                break
            if time.monotonic() >= deadline:
                raise TimeoutError('Fixture child did not retire')
            time.sleep(0.005)
        self.closed = True
        os.close(self.fd)
        self.temporary.cleanup()

    def __enter__(self) -> 'Shell':
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


def advertised(output: bytes) -> int:
    masks = []
    for encoded in re.findall(re.escape(READY) + rb'([^\x07]+)\x07', output):
        try:
            text = base64.b64decode(encoded, validate=True).decode('ascii')
            masks.append(int(text.split('|')[-1]))
        except (ValueError, UnicodeError):
            raise AssertionError('Malformed readiness emitted by fixture')
    return masks[-1] if masks else 0


class ActivationRegressions(unittest.TestCase):
    def test_01_core_has_valid_bash_syntax(self) -> None:
        result = subprocess.run([BASH, '-n'], input=CORE.encode(), capture_output=True,
                                timeout=TIMEOUT, env={'PATH': '/usr/bin:/bin'})
        self.assertEqual(result.returncode, 0, 'Bash syntax check failed')

    def test_02_noninteractive_source_stays_inert(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / 'core.bash'
            p.write_text(CORE, encoding='utf-8')
            code = f"source {shlex.quote(str(p))}; printf '%s' CLEAN"
            result = subprocess.run([BASH, '--noprofile', '--norc', '-c', code],
                capture_output=True, timeout=TIMEOUT,
                env={'HOME': tmp, 'PATH': '/usr/bin:/bin'})
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, b'CLEAN')

    def test_03_normal_prompt_remains_visible_and_marked(self) -> None:
        with Shell() as shell:
            output = shell.initial + shell.command('true')
            self.assertIn(PROMPT, output)
            self.assertIn(b'\x1b]133;A;aid=1\x07', output)
            self.assertIn(b'\x1b]133;B\x07', output)
            self.assertEqual(advertised(output), 7)

    def test_04_existing_hook_receives_actual_exit_status(self) -> None:
        with Shell("PROMPT_COMMAND='printf \"AMX_STATUS=%s\\n\" \"$?\"'") as shell:
            output = shell.command('(exit 23)')
            self.assertIn(b'AMX_STATUS=23', output)

    def test_05_dynamic_prompt_must_not_false_advertise_markers(self) -> None:
        with Shell("PROMPT_COMMAND='PS1=\"AMX_AUDIT_PROMPT> \"'") as shell:
            output = shell.initial + shell.command('true')
            self.assertTrue(not (advertised(output) & 1) or b'\x1b]133;A;aid=' in output,
                'Readiness claims prompt markers, but the real prompt loop emits none')

    def test_06_readonly_helper_collision_must_not_false_advertise_cwd(self) -> None:
        profile = '__amx_ssh_prompt() { :; }; readonly -f __amx_ssh_prompt'
        with Shell(profile) as shell:
            output = shell.initial + shell.command('true')
            self.assertTrue(not (advertised(output) & 2) or CWD in output,
                'A failed helper installation still advertises working-directory support')

    def test_07_redirected_stdout_must_not_receive_metadata(self) -> None:
        with Shell() as shell:
            shell.command('exec > "$HOME/captured-output"')
            shell.command('true')
            shell.command('exec > /dev/tty')
            captured = (shell.root / 'captured-output').read_bytes()
            self.assertNotIn(b'\x1b]', captured,
                'The prompt hook wrote OSC metadata into redirected stdout')

    def test_08_bootstrap_must_restore_inherited_umask(self) -> None:
        with Shell(bootstrap=True) as shell:
            shell.command('umask > "$HOME/observed-mask"')
            value = int((shell.root / 'observed-mask').read_text(encoding='utf-8').strip(), 8)
            self.assertEqual(f'{value:04o}', '0022', 'Private setup mask leaked into the user shell')

    def test_09_profile_must_observe_original_umask(self) -> None:
        with Shell('umask > "$HOME/profile-mask"', bootstrap=True) as shell:
            value = int((shell.root / 'profile-mask').read_text(encoding='utf-8').strip(), 8)
            self.assertEqual(f'{value:04o}', '0022', 'User .bashrc runs under altered setup permissions')

    def test_10_setup_failure_fallback_must_restore_umask(self) -> None:
        with Shell(bootstrap=True, fail_mktemp=True) as shell:
            shell.command('umask > "$HOME/fallback-mask"')
            value = int((shell.root / 'fallback-mask').read_text(encoding='utf-8').strip(), 8)
            self.assertEqual(f'{value:04o}', '0022', 'Plain fallback retains the private setup mask')

    def test_11_cwd_escaping_does_not_emit_literal_hostile_controls(self) -> None:
        with Shell(cwd_name='a b#界') as shell:
            output = shell.initial + shell.command('true')
            frames = cwd_frames(output)
            self.assertTrue(any(value.endswith('/a b#界') for value in frames))
            self.assertNotIn(b'\x1b]7;', output)
            self.assertNotIn(b'a b#\xe7\x95\x8c', output)

    def test_12_successful_bootstrap_cleans_its_owned_temp_directory(self) -> None:
        with Shell(bootstrap=True) as shell:
            self.assertEqual(list(shell.tmp.glob('automexia-ssh.*')), [])

    def test_13_unwrapped_core_does_not_change_umask(self) -> None:
        with Shell() as shell:
            shell.command('umask > "$HOME/control-mask"')
            value = int((shell.root / 'control-mask').read_text(encoding='utf-8').strip(), 8)
            self.assertEqual(value, 0o022)



def cwd_frames(output: bytes) -> list[str]:
    return [base64.b64decode(value, validate=True).decode('utf-8')
            for value in re.findall(re.escape(CWD) + rb'([^\x07]*)\x07', output)]


class AdditionalContracts(unittest.TestCase):
    def test_dynamic_prompt_is_marked_on_every_iteration_without_duplication(self):
        with Shell('PROMPT_COMMAND=\'PS1="AMX_AUDIT_PROMPT> "\'') as shell:
            for _ in range(8):
                output = shell.command('true')
                self.assertEqual(output.count(b'\x1b]133;A;aid='), 1)
    def test_hook_array_preserves_order_and_first_status(self):
        with Shell('PROMPT_COMMAND=(\'printf "FIRST=%s\\n" "$?"\' \'printf "SECOND\\n"\')') as shell:
            data = shell.command('(exit 23)')
            self.assertIn(b'FIRST=23', data)
            self.assertLess(data.index(b'FIRST=23'), data.index(b'SECOND'))
    def test_readonly_global_namespace_collision_is_inert(self):
        with Shell('readonly __amx_ssh_cwd=unchanged') as shell:
            self.assertEqual(advertised(shell.initial), 0)
            self.assertIn(b'unchanged', shell.command('printf "%s\\n" "$__amx_ssh_cwd"'))
    def test_nameref_prompt_command_is_not_reassigned(self):
        with Shell("original=':'; declare -n PROMPT_COMMAND=original") as shell:
            self.assertEqual(advertised(shell.initial), 0)
            self.assertIn(b'original', shell.command('declare -p PROMPT_COMMAND'))
    def test_readonly_ps1_declines_without_advertising(self):
        with Shell('readonly PS1') as shell:
            self.assertEqual(advertised(shell.initial), 0)
    def test_associative_prompt_command_declines(self):
        with Shell('declare -A PROMPT_COMMAND=([key]=:)') as shell:
            self.assertEqual(advertised(shell.initial), 0)
    def test_stdout_and_stderr_redirection_cannot_capture_hook_metadata(self):
        with Shell() as shell:
            shell.command('exec 2>"$HOME/err"; __amx_ssh_prompt; exec 2>/dev/tty')
            self.assertEqual((shell.root/'err').read_bytes(), b'')
    def test_repeated_source_does_not_duplicate_hooks(self):
        with Shell() as shell:
            source = shell.root/'core'
            source.write_text(CORE, encoding='utf-8')
            shell.command('source "$HOME/core"')
            data = shell.command('declare -p PROMPT_COMMAND')
            self.assertEqual(data.count(b'__amx_ssh_prompt'), 1)
    def test_status_hooks_are_not_invented_for_empty_enter(self):
        with Shell() as shell:
            data = shell.command('')
            self.assertNotIn(b'\x1b]133;C', data)
            self.assertNotIn(b'\x1b]133;D', data)
    def test_cwd_frame_is_connection_scoped_and_never_osc7(self):
        with Shell() as shell:
            values = cwd_frames(shell.initial)
            self.assertTrue(values)
            self.assertTrue(all(v.startswith('AMXSSHCWD1|3|7|/') for v in values))
            self.assertNotIn(b'\x1b]7;', shell.initial)
    def test_invalid_cwd_emits_an_explicit_scoped_clear(self):
        with Shell(cwd_name='a\x1bb') as shell:
            self.assertIn('AMXSSHCWD1|3|7|', cwd_frames(shell.initial))
            self.assertNotIn(b'a\x1bb', shell.initial)
    def test_stable_cwd_does_not_spawn_encoder_per_prompt(self):
        wrapper = '#!/bin/sh\nprintf x >> "$HOME/encodes"\nexec /usr/bin/base64 "$@"\n'
        with Shell(extra_bins={'base64': wrapper}) as shell:
            before = (shell.root/'encodes').read_bytes()
            for _ in range(12): shell.command('true')
            self.assertEqual((shell.root/'encodes').read_bytes(), before)
    def test_encoding_failure_degrades_without_osc7_or_false_cwd(self):
        with Shell(extra_bins={'base64': '#!/bin/sh\nexit 1\n'}) as shell:
            self.assertEqual(advertised(shell.initial) & 2, 0)
            self.assertNotIn(b'\x1b]7;', shell.initial)
    def test_later_foreign_prompt_marker_revokes_our_capabilities(self):
        with Shell() as shell:
            data = shell.command("PS1='\\[\\e]133;A\\a\\]AMX_AUDIT_PROMPT> '")
            self.assertEqual(advertised(data), 0)
    def test_profile_umask_change_is_not_undone_by_setup_cleanup(self):
        with Shell('umask 0002', bootstrap=True) as shell:
            shell.command('umask > "$HOME/mask"')
            self.assertEqual(int((shell.root/'mask').read_text(encoding='utf-8').strip(), 8), 0o002)
    def test_profile_is_sourced_once_in_this_bootstrap(self):
        with Shell('printf x >> "$HOME/profile-count"', bootstrap=True) as shell:
            self.assertEqual((shell.root/'profile-count').read_bytes(), b'x')
    def test_success_cleanup_preserves_unrelated_sentinel(self):
        with Shell('printf untouched > "$TMPDIR/sentinel"', bootstrap=True) as shell:
            self.assertEqual((shell.tmp/'sentinel').read_text(encoding='utf-8'), 'untouched')
            self.assertEqual(list(shell.tmp.glob('automexia-ssh.*')), [])
    def test_nondefault_inherited_mask_survives(self):
        with Shell(bootstrap=True, initial_umask=0o027) as shell:
            shell.command('umask > "$HOME/mask"')
            self.assertEqual(int((shell.root/'mask').read_text(encoding='utf-8').strip(), 8), 0o027)

    def test_own_alias_collision_declines_without_expansion(self):
        with Shell("alias __amx_ssh_prompt='printf ALIAS_CANARY'") as shell:
            self.assertEqual(advertised(shell.initial), 0)
            self.assertNotIn(b'ALIAS_CANARY', shell.initial)
    def test_user_printf_alias_cannot_rewrite_metadata(self):
        with Shell("alias printf='echo PRINTF_CANARY'") as shell:
            self.assertEqual(advertised(shell.initial), 7)
            self.assertNotIn(b'PRINTF_CANARY', shell.initial)
    def test_bootstrap_cleanup_does_not_modify_similarly_named_profile_variables(self):
        with Shell("readonly __amx_rc=profile_owned __amx_dir=profile_owned", bootstrap=True) as shell:
            data=shell.command('builtin printf "%s:%s\\n" "$__amx_rc" "$__amx_dir"')
            self.assertIn(b'profile_owned:profile_owned', data)


class CommandLifecycle(unittest.TestCase):
    def test_context_spacer_and_editor_share_one_stable_prompt_identity(self):
        with Shell(echo_input=True) as shell:
            self.assertIn(b'\x1b]133;A;aid=1\x07 \r\n', shell.initial)
            self.assertIn(b'\x1b]133;P;k=c;aid=1\x07' + PROMPT, shell.initial)
            for data in [shell.command(''), shell.send(b'\x0c'), shell.send(b'cancel-me\x03')]:
                self.assertNotIn(b'aid=2', data)
                self.assertIn(b'\x1b]133;P;k=c;aid=1\x07', data)
            data = shell.command('true')
            self.assertIn(b'\x1b]133;A;aid=2\x07 \r\n', data)
            self.assertIn(b'\x1b]133;P;k=c;aid=2\x07', data)

    def test_real_commands_publish_start_and_original_exit_status(self):
        profile = 'PROMPT_COMMAND=(\'printf "NATIVE_STATUS=%s\\n" "$?"\' true)'
        with Shell(profile) as shell:
            self.assertNotIn(b'\x1b]133;D;', shell.initial)
            for command, status in [('true', 0), ('false', 1), ('(exit 23)', 23), ('false | true', 0),
                                    ('set -o pipefail; false | true', 1)]:
                with self.subTest(command=command):
                    data = shell.command(command)
                    self.assertEqual(data.count(b'\x1b]133;C'), 1)
                    self.assertEqual(re.findall(rb'\x1b]133;D;(\d+)\x07', data), [str(status).encode()])
                    self.assertIn(f'NATIVE_STATUS={status}'.encode(), data)
                    self.assertLess(data.index(b'\x1b]133;C'), data.index(b'\x1b]133;D;'))

    def test_empty_enter_does_not_complete_the_previous_command_twice(self):
        with Shell() as shell:
            shell.command('false')
            for _ in range(3):
                data = shell.command('')
                self.assertNotIn(b'\x1b]133;C', data)
                self.assertNotIn(b'\x1b]133;D', data)

    def test_readline_clear_redraws_without_a_new_prompt_or_command(self):
        with Shell(echo_input=True) as shell:
            for _ in range(3):
                data = shell.send(b'\x0c')
                self.assertNotIn(b'\x1b]133;A', data)
                self.assertIn(b'\x1b]133;P;k=c;aid=1\x07', data)
                self.assertNotIn(b'\x1b]133;C', data)
                self.assertNotIn(b'\x1b]133;D', data)

    def test_cancelled_edit_does_not_publish_a_completed_command(self):
        with Shell() as shell:
            data = shell.send(b'not-executed\x03')
            self.assertNotIn(b'\x1b]133;C', data)
            self.assertNotIn(b'\x1b]133;D', data)
            self.assertIn(b'\x1b]133;A', data)

    def test_interrupting_a_running_command_reports_its_real_failure(self):
        with Shell() as shell:
            # Readiness must come from the blocking child itself. A shell
            # printf before exec does not prove the interrupt target is running.
            program = ("import os,time; os.write(1,bytes((82,85,78,78,73,78,71))+b':'"
                       "+str(os.getpgrp()).encode()+b'\\n'); time.sleep(30)")
            command = f'({shlex.quote(sys.executable)} -c {shlex.quote(program)})\n'
            before = shell.send(command.encode(), b'RUNNING:')
            self.assertIn(b'\x1b]133;C', before)
            child_group = int(shell.until(b'\n').strip())
            self.assertGreater(child_group, 0)
            deadline = time.monotonic() + TIMEOUT
            while os.tcgetpgrp(shell.fd) != child_group:
                if time.monotonic() >= deadline:
                    self.fail('running interrupt fixture did not acquire the terminal')
                time.sleep(.001)
            data = shell.send(b'\x03')
            self.assertEqual(re.findall(rb'\x1b]133;D;(\d+)\x07', data), [b'130'])
            self.assertNotIn(b'\x1b]133;D', shell.command(''))

    def test_unset_optional_prompts_with_nounset_preserve_native_shell(self):
        with Shell('unset PS0 PS2; set -u') as shell:
            data = shell.command('true')
            self.assertIn(b'\x1b]133;C', data)
            self.assertIn(b'\x1b]133;D;0\x07', data)
            self.assertNotIn(b'unbound variable', shell.initial + data)

    def test_native_hooks_can_update_preexecution_text_without_duplicates(self):
        with Shell('PROMPT_COMMAND=\'PS0="NATIVE_PREEXEC\\n"\'') as shell:
            for _ in range(3):
                data = shell.command('true')
                self.assertEqual(data.count(b'NATIVE_PREEXEC'), 1)
                self.assertEqual(data.count(b'\x1b]133;C'), 1)

    def test_later_foreign_prompt_revokes_all_owned_prompt_markers(self):
        with Shell() as shell:
            data = shell.command("PS1='\\[\\e]133;A\\a\\]AMX_AUDIT_PROMPT> '")
            self.assertEqual(advertised(data), 0)
            data = shell.command('true')
            self.assertNotIn(b'\x1b]133;C', data)
            self.assertNotIn(b'\x1b]133;D', data)
            self.assertNotIn(b'\x1b]133;P;k=c', data)

    def test_native_preexecution_text_and_secondary_prompt_are_preserved(self):
        with Shell('PS0="NATIVE_PREEXEC\\n"; PS2="NATIVE_SECONDARY> "') as shell:
            first = shell.send(b'printf "%s\\n" "first\n', b'NATIVE_SECONDARY> ')
            self.assertIn(b'\x1b]133;P;k=s;aid=1\x07', first)
            self.assertNotIn(b'\x1b]133;C', first)
            data = shell.send(b'second"\n')
            self.assertIn(b'NATIVE_PREEXEC', data)
            self.assertIn(b'first\r\nsecond', data)
            self.assertEqual(data.count(b'\x1b]133;C'), 1)
            self.assertIn(b'\x1b]133;D;0\x07', data)

    def test_conflicting_preexecution_and_secondary_prompts_decline(self):
        for profile in ['readonly PS0="native"', 'readonly PS2="secondary"',
                        "PS0='\\[\\e]133;C\\a\\]'", "PS2='\\[\\e]133;P;k=s\\a\\]secondary'",
                        "original='native'; declare -n PS0=original"]:
            with self.subTest(profile=profile), Shell(profile) as shell:
                self.assertEqual(advertised(shell.initial), 0)

    def test_redirected_terminal_stream_does_not_capture_prompt_markers(self):
        with Shell() as shell:
            # Restore stderr after a prompt without depending on its hidden text.
            shell.send(b'exec 2>"$HOME/captured-stderr"\nprintf SYNC\n', b'SYNC')
            shell.send(b'exec 2>/dev/tty\n')
            captured = (shell.root / 'captured-stderr').read_bytes()
            self.assertNotIn(b'\x1b]', captured)
            self.assertIn(PROMPT, captured)


def user_frames(output: bytes) -> list[str]:
    return [base64.b64decode(value, validate=True).decode('utf-8')
            for value in re.findall(re.escape(USER) + rb'([^\x07]*)\x07', output)]


class RemoteUserMetadata(unittest.TestCase):
    def test_remote_user_is_scoped_encoded_and_replayed(self):
        with Shell(user_name='remote-user') as shell:
            for data in [shell.initial, shell.command('true')]:
                self.assertIn('AMXSSHUSER1|3|7|remote-user', user_frames(data))
                self.assertNotIn(b'remote-user', data)

    def test_utf8_names_are_bounded_in_bytes(self):
        for user, expected in [('a' * 256, 'a' * 256), ('ü' * 128, 'ü' * 128),
                               ('a' * 257, ''), ('ü' * 129, '')]:
            with self.subTest(bytes=len(user.encode())), Shell(user_name=user) as shell:
                self.assertEqual(set(user_frames(shell.initial)), {'AMXSSHUSER1|3|7|' + expected})

    def test_control_characters_clear_the_scoped_user(self):
        for user in ['a\nb', 'a\x1bb', 'a\x7fb', 'a\u0085b']:
            with self.subTest(user=repr(user)), Shell(user_name=user) as shell:
                self.assertEqual(set(user_frames(shell.initial)), {'AMXSSHUSER1|3|7|'})

    def test_missing_user_uses_remote_logname_or_explicit_clear(self):
        with Shell('unset USER; LOGNAME=remote-login') as shell:
            self.assertIn('AMXSSHUSER1|3|7|remote-login', user_frames(shell.initial))
        with Shell('unset USER LOGNAME') as shell:
            self.assertEqual(set(user_frames(shell.initial)), {'AMXSSHUSER1|3|7|'})

    def test_encoder_failure_clears_identity_without_leaking_raw_text(self):
        with Shell(user_name='remote-user', extra_bins={'base64': '#!/bin/sh\nexit 1\n'}) as shell:
            self.assertEqual(set(user_frames(shell.initial)), {'AMXSSHUSER1|3|7|'})
            self.assertNotIn(b'remote-user', shell.initial)

    def test_later_foreign_prompt_revokes_remote_identity(self):
        with Shell() as shell:
            data = shell.command("PS1='\\[\\e]133;A\\a\\]AMX_AUDIT_PROMPT> '")
            self.assertEqual(user_frames(data), ['AMXSSHUSER1|3|7|'])


def context_frames(output: bytes) -> list[dict[str, str]]:
    frames = []
    for encoded in re.findall(re.escape(CONTEXT) + rb'([^\x07]*)\x07', output):
        lines = base64.b64decode(encoded, validate=True).decode('utf-8').splitlines()
        if lines[0] != 'AMXSSHCTX1|3|7|' or len(lines) != 10:
            raise AssertionError('context scope or complete snapshot contract changed')
        values = dict(line.split('=', 1) for line in lines[1:])
        if tuple(values) != CONTEXT_FIELDS:
            raise AssertionError('context fields changed')
        frames.append(values)
    return frames


class RemoteContextMetadata(unittest.TestCase):
    def test_environment_snapshot_is_complete_scoped_and_updates(self):
        profile = ('GIT_BRANCH=feature; KUBECONTEXT=cluster; KUBE_NAMESPACE=namespace; '
                   'DOCKER_CONTEXT=engine; TF_WORKSPACE=workspace; AUTOMEXIA_ENV=development; '
                   'AWS_PROFILE=profile; AZURE_CLOUD_NAME=cloud; CLOUDSDK_CORE_PROJECT=project')
        expected = dict(zip(CONTEXT_FIELDS, ('feature', 'cluster', 'namespace', 'engine',
                        'workspace', 'development', 'profile', 'cloud', 'project')))
        with Shell(profile) as shell:
            self.assertEqual(context_frames(shell.initial)[-1], expected)
            data = shell.command('unset GIT_BRANCH KUBECONTEXT KUBE_NAMESPACE DOCKER_CONTEXT TF_WORKSPACE AUTOMEXIA_ENV AWS_PROFILE AZURE_CLOUD_NAME CLOUDSDK_CORE_PROJECT')
            self.assertEqual(context_frames(data)[-1], dict.fromkeys(CONTEXT_FIELDS, ''))

    def test_context_utf8_byte_limits_and_hostile_values_clear_one_field(self):
        for value, expected in [('ü' * 128, 'ü' * 128), ('ü' * 129, ''), ('a\nb', ''),
                                ('a\x1bb', ''), ('a\u0085b', ''), ('a\u202eb', ''),
                                ('a\u2067b', ''), ('a\u200fb', ''), ('a\u061cb', '')]:
            with self.subTest(bytes=len(value.encode())), Shell('GIT_BRANCH=' + shlex.quote(value) + '; TF_WORKSPACE=valid') as shell:
                result = context_frames(shell.initial)[-1]
                self.assertEqual(result['git_branch'], expected)
                self.assertEqual(result['terraform_workspace'], 'valid')

    def test_fallback_names_and_fixed_terminal_identity(self):
        with Shell('KUBE_CONTEXT=fallback; ENVIRONMENT=development; AWS_DEFAULT_PROFILE=profile') as shell:
            result = context_frames(shell.initial)[-1]
            self.assertEqual((result['kubernetes_context'], result['environment'], result['aws_profile']),
                             ('fallback', 'development', 'profile'))
            data = shell.command("printf 'ENV=%s:%s\\n' \"$COLORTERM\" \"$TERM_PROGRAM\"")
            self.assertIn(b'ENV=truecolor:Automexia', data)

    def test_readonly_or_indirect_terminal_identity_preserves_native_shell(self):
        for setup in ['readonly COLORTERM=native', 'readonly TERM_PROGRAM=native',
                      'native=native; declare -n TERM_PROGRAM=native']:
            with self.subTest(setup=setup), Shell(setup) as shell:
                self.assertNotIn(READY, shell.initial)
                self.assertNotIn(b'\x1b]133;', shell.initial)
                self.assertNotIn(b'readonly variable', shell.initial)
                self.assertIn(b'NATIVE_OK', shell.command('printf NATIVE_OK'))

    def test_unchanged_context_is_cached_and_changed_context_encoded_once(self):
        wrapper = '#!/bin/sh\nprintf x >> "$HOME/encodes"\nexec /usr/bin/base64 "$@"\n'
        with Shell(extra_bins={'base64': wrapper}) as shell:
            count = len((shell.root / 'encodes').read_bytes())
            for _ in range(4): self.assertTrue(context_frames(shell.command('true')))
            self.assertEqual(len((shell.root / 'encodes').read_bytes()), count)
            self.assertEqual(context_frames(shell.command('GIT_BRANCH=changed'))[-1]['git_branch'], 'changed')
            self.assertEqual(len((shell.root / 'encodes').read_bytes()), count + 1)

    def test_encoder_failure_and_revocation_explicitly_clear_context(self):
        with Shell('GIT_BRANCH=private', extra_bins={'base64': '#!/bin/sh\nexit 1\n'}) as shell:
            self.assertEqual(context_frames(shell.initial)[-1], dict.fromkeys(CONTEXT_FIELDS, ''))
            self.assertNotIn(b'private', shell.initial)
        with Shell('GIT_BRANCH=feature') as shell:
            data = shell.command("PS1='\\[\\e]133;A\\a\\]AMX_AUDIT_PROMPT> '")
            self.assertEqual(context_frames(data)[-1], dict.fromkeys(CONTEXT_FIELDS, ''))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bash')
    parser.add_argument('--generated-fixture', type=Path)
    opts, rest = parser.parse_known_args()
    if opts.bash: BASH = opts.bash
    if opts.generated_fixture: load_generated(opts.generated_fixture)
    unittest.main(argv=[sys.argv[0], *rest])
