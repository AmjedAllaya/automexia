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

ROOT = Path(__file__).resolve().parents[2]
CORE = ''
BOOTSTRAP = ''
BASH = shutil.which('bash') if os.name == 'posix' else None
TIMEOUT = 5.0
MAX_OUTPUT = 256 * 1024
PROMPT = b'AMX_AUDIT_PROMPT> '
READY = b'\x1b]1337;SetUserVar=automexia_ssh_ready='
CWD = b'\x1b]1337;SetUserVar=automexia_ssh_cwd='


def fixture_core() -> str:
    source = (ROOT / 'automexia-ssh-integration/resources/bash-core.bash').read_text(encoding='utf-8')
    values = {
        '@@RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|1|3').decode(),
        '@@PROMPT_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|1|1').decode(),
        '@@DOWNGRADE_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|2|1').decode(),
        '@@REVOKE_RECEIPT@@': base64.b64encode(b'AMXSSH1|3|7|3|0').decode(),
        '@@CLEAR_CWD@@': base64.b64encode(b'AMXSSHCWD1|3|7|').decode(),
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
                 fail_mktemp: bool = False, cwd_name: str = 'project', initial_umask: int = 0o022, extra_bins: dict[str, str] | None = None) -> None:
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
               'TERM': 'xterm-256color', 'TMPDIR': str(self.tmp), 'LC_ALL': 'C.UTF-8'}
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
        data = (command + '\n').encode('utf-8')
        if len(data) > 8192:
            raise ValueError('Fixture input budget exceeded')
        while data:
            count = os.write(self.fd, data)
            data = data[count:]
        return self.until(PROMPT)

    def close(self) -> None:
        if self.closed:
            return
        self.closed = True
        # Keep our child unreaped until its fixture process group is retired.
        try:
            os.killpg(self.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        deadline = time.monotonic() + TIMEOUT
        try:
            while True:
                child, _ = os.waitpid(self.pid, os.WNOHANG)
                if child:
                    break
                if time.monotonic() >= deadline:
                    raise TimeoutError('Fixture child did not retire')
                time.sleep(0.005)
        finally:
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
            self.assertIn(b'\x1b]133;A\x07', output)
            self.assertIn(b'\x1b]133;B\x07', output)
            self.assertEqual(advertised(output), 3)

    def test_04_existing_hook_receives_actual_exit_status(self) -> None:
        with Shell("PROMPT_COMMAND='printf \"AMX_STATUS=%s\\n\" \"$?\"'") as shell:
            output = shell.command('(exit 23)')
            self.assertIn(b'AMX_STATUS=23', output)

    def test_05_dynamic_prompt_must_not_false_advertise_markers(self) -> None:
        with Shell("PROMPT_COMMAND='PS1=\"AMX_AUDIT_PROMPT> \"'") as shell:
            output = shell.initial + shell.command('true')
            self.assertTrue(not (advertised(output) & 1) or b'\x1b]133;A\x07' in output,
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
                self.assertEqual(output.count(b'\x1b]133;A\x07'), 1)
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
            self.assertEqual(advertised(shell.initial), 3)
            self.assertNotIn(b'PRINTF_CANARY', shell.initial)
    def test_bootstrap_cleanup_does_not_modify_similarly_named_profile_variables(self):
        with Shell("readonly __amx_rc=profile_owned __amx_dir=profile_owned", bootstrap=True) as shell:
            data=shell.command('builtin printf "%s:%s\\n" "$__amx_rc" "$__amx_dir"')
            self.assertIn(b'profile_owned:profile_owned', data)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bash')
    parser.add_argument('--generated-fixture', type=Path)
    opts, rest = parser.parse_known_args()
    if opts.bash: BASH = opts.bash
    if opts.generated_fixture: load_generated(opts.generated_fixture)
    unittest.main(argv=[sys.argv[0], *rest])
