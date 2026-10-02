#!/usr/bin/env python3
"""Native PowerShell contract tests over actual Rust-generated adapter source.

These verify interpreter/hook behavior, not a remote server or a physical editor.
Interactive SSH/ConPTY evidence remains a separate integration gate.
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

import qa_process

ROOT = Path(__file__).resolve().parents[2]
FIXTURE: Path | None = None
POWERSHELL: str | None = None


def source() -> str:
    if FIXTURE is None or POWERSHELL is None:
        raise unittest.SkipTest('requires native PowerShell and actual generated fixture')
    lines = FIXTURE.read_bytes().splitlines()
    if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
        raise ValueError('invalid generated fixture')
    value = base64.b64decode(lines[1], validate=True).decode('utf-8')
    if len(value.encode()) > 4096 or '@@' in value:
        raise ValueError('invalid generated PowerShell source')
    return value


def run_script(before: str, after: str, *, no_module: bool = False) -> bytes:
    core = source()
    if no_module:
        # PowerShell 7 adds PSHOME modules during startup; set the test-only empty
        # search path after startup as well, so absence is real on both runtimes.
        before = "Import-Module Microsoft.PowerShell.Utility\nImport-Module Microsoft.PowerShell.Management\n$env:PSModulePath=Join-Path $PSScriptRoot no-modules\n$PSModuleAutoLoadingPreference='None'\nRemove-Module PSReadLine -ErrorAction Ignore\nRemove-Item Function:PSConsoleHostReadLine -ErrorAction Ignore\n" + before
    with tempfile.TemporaryDirectory(prefix='automexia-ssh-powershell-') as temporary:
        root = Path(temporary)
        (root / 'core.ps1').write_text(core, encoding='utf-8-sig')
        script = root / 'test.ps1'
        script.write_text("$ErrorActionPreference='Stop'\n" + before +
                          "\n. (Join-Path $PSScriptRoot core.ps1)\n" + after, encoding='utf-8-sig')
        environment = os.environ.copy()
        output = bytearray()
        def consume(chunk: bytes) -> None:
            if len(output) + len(chunk) > 64 * 1024:
                raise ValueError('PowerShell fixture output exceeded budget')
            output.extend(chunk)
        result = qa_process.run([POWERSHELL, '-NoLogo', '-NoProfile', '-NonInteractive', '-File', str(script)],
                                cwd=ROOT, timeout_seconds=20, consume=consume, environment=environment)
        if result.return_code != 0 or result.error or result.timed_out:
            raise AssertionError('native PowerShell fixture failed: ' + bytes(output).decode(errors='replace'))
        return bytes(output)


def masks(output: bytes) -> list[int]:
    return [int(base64.b64decode(value, validate=True).decode().rsplit('|', 1)[1])
            for value in re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_ready=([^\x07]+)\x07', output)]


class PowerShellAdapterTests(unittest.TestCase):
    def test_unchanged_prompt_recovers_optional_metadata_after_rejected_replacement(self):
        after = """
prompt
[Console]::Write("$([char]27)]1337;SetUserVar=automexia_ssh_user=!!!!$([char]7)$([char]27)]1337;SetUserVar=automexia_ssh_context=!!!!$([char]7)")
prompt
"""
        output = run_script("function global:prompt { 'NATIVE_PROMPT>' }", after, no_module=True)
        before, recovery = output.split(b'\x1b]1337;SetUserVar=automexia_ssh_user=!!!!\x07\x1b]1337;SetUserVar=automexia_ssh_context=!!!!\x07', 1)
        for name in (b'automexia_ssh_user', b'automexia_ssh_context'):
            pattern = rb'\x1b]1337;SetUserVar=' + name + rb'=([^\x07]+)\x07'
            self.assertEqual(re.findall(pattern, recovery), re.findall(pattern, before)[-1:])

    def test_readline_loaded_during_bootstrap_has_command_status_capability(self):
        output = run_script("Remove-Module PSReadLine -ErrorAction Ignore\nRemove-Item Function:PSConsoleHostReadLine -ErrorAction Ignore\nfunction global:prompt { 'NATIVE_PROMPT>' }", "prompt\n")
        self.assertEqual(masks(output), [7])

    def test_explicit_readline_import_when_automatic_loading_is_disabled(self):
        output = run_script("Import-Module Microsoft.PowerShell.Utility\nImport-Module Microsoft.PowerShell.Management\n$PSModuleAutoLoadingPreference='None'\nRemove-Module PSReadLine -ErrorAction Ignore\nRemove-Item Function:PSConsoleHostReadLine -ErrorAction Ignore\nfunction global:prompt { 'NATIVE_PROMPT>' }", "prompt\n")
        self.assertEqual(masks(output), [7])

    def test_native_prompt_sees_the_original_command_success(self):
        output = run_script("function global:prompt { $ok=$?; 'NATIVE_STATUS='+$ok }", "Write-Error fixture-failure -ErrorAction SilentlyContinue\nprompt\n", no_module=True)
        self.assertIn(b'NATIVE_STATUS=False', output)

    def test_native_failure_status_does_not_mutate_native_exitcode(self):
        before = """
Import-Module PSReadLine -ErrorAction Stop
function global:PSConsoleHostReadLine { 'fixture-command' }
function global:prompt { $ok=$?; 'NATIVE_STATUS='+$ok }
$fixture_exe=Join-Path $PSHOME $(if($PSVersionTable.PSVersion.Major -ge 6){'pwsh.exe'}else{'powershell.exe'})
"""
        after = """
prompt
PSConsoleHostReadLine | Out-Null
& $fixture_exe -NoLogo -NoProfile -NonInteractive -Command 'exit 7'
prompt
[Console]::Write('UNCHANGED_EXIT='+$global:LASTEXITCODE)
"""
        output = run_script(before, after)
        self.assertEqual(output.count(b'\x1b]133;D;1\x07'), 1)
        self.assertIn(b'NATIVE_STATUS=False', output)
        self.assertIn(b'UNCHANGED_EXIT=7', output)

    def test_empty_and_cancelled_editor_reads_do_not_fabricate_commands(self):
        before = """
Import-Module PSReadLine -ErrorAction Stop
$global:fixture_cancel=$false
function global:PSConsoleHostReadLine { if($global:fixture_cancel){throw 'fixture-cancel'}; '   ' }
function global:prompt { 'NATIVE_PROMPT>' }
"""
        after = """
prompt
PSConsoleHostReadLine | Out-Null
prompt
$global:fixture_cancel=$true
try { PSConsoleHostReadLine | Out-Null } catch {}
prompt
"""
        output = run_script(before, after)
        self.assertNotIn(b'\x1b]133;C\x07', output)
        self.assertNotIn(b'\x1b]133;D;', output)
        self.assertEqual(output.count(b'\x1b]133;A;aid=1\x07'), 1)

    def test_context_is_scoped_complete_cached_and_cleared(self):
        before = """
$env:GIT_BRANCH='fixture-branch';$env:KUBECONTEXT='fixture-cluster'
$env:TF_WORKSPACE='fixture-workspace';$env:AWS_PROFILE='fixture-cloud'
function global:prompt { 'NATIVE_PROMPT>' }
"""
        after = """
prompt
prompt
$env:GIT_BRANCH='';$env:KUBECONTEXT="bad`nvalue";$env:TF_WORKSPACE=('x'*257);$env:AWS_PROFILE=''
prompt
[Console]::Write("IDENTITY=$env:COLORTERM|$env:TERM_PROGRAM")
"""
        output = run_script(before, after, no_module=True)
        self.assertLess(output.index(b'SetUserVar=automexia_ssh_ready='), output.index(b'SetUserVar=automexia_ssh_user='))
        values = re.findall(rb'\x1b]1337;SetUserVar=automexia_ssh_context=([^\x07]+)\x07', output)
        self.assertEqual(len(values), 3)
        decoded = [base64.b64decode(value, validate=True).decode().splitlines() for value in values]
        for lines in decoded:
            self.assertEqual(lines[0], 'AMXSSHCTX1|3|7|')
            self.assertEqual(len(lines), 10)
        first, unchanged, last = [dict(line.split('=', 1) for line in lines[1:]) for lines in decoded]
        self.assertEqual(first, unchanged)
        self.assertEqual(first['git_branch'], 'fixture-branch')
        self.assertEqual(first['kubernetes_context'], 'fixture-cluster')
        self.assertEqual(first['terraform_workspace'], 'fixture-workspace')
        self.assertEqual(last['git_branch'], '')
        self.assertEqual(last['kubernetes_context'], '')
        self.assertEqual(last['terraform_workspace'], '')
        self.assertIn(b'IDENTITY=truecolor|Automexia', output)

    def test_missing_readline_is_truthfully_prompt_and_cwd_only(self):
        output = run_script("function global:prompt { 'NATIVE_PROMPT>' }", "prompt\nprompt\n", no_module=True)
        self.assertEqual(masks(output), [3])
        self.assertIn(b'NATIVE_PROMPT>', output)
        self.assertEqual(output.count(b'\x1b]133;A;aid=1\x07'), 1)
        self.assertNotIn(b'\x1b]133;D;', output)

    def test_real_readline_module_hook_preserves_prompt_and_command_status(self):
        before = """
Import-Module PSReadLine -ErrorAction Stop
function global:prompt { 'NATIVE_PROMPT>' }
$global:fixture_line='fixture-command'
function global:PSConsoleHostReadLine { $global:fixture_line }
"""
        after = """
prompt
PSConsoleHostReadLine | Out-Null
$global:LASTEXITCODE=7
Write-Error 'fixture-failure' -ErrorAction SilentlyContinue
prompt
PSConsoleHostReadLine | Out-Null
$global:LASTEXITCODE=0
Write-Output 'fixture-success' | Out-Null
prompt
$global:fixture_line=''
PSConsoleHostReadLine | Out-Null
prompt
"""
        output = run_script(before, after)
        self.assertEqual(masks(output), [7])
        self.assertEqual(output.count(b'\x1b]133;C\x07'), 2)
        # Match the local PowerShell integration: success/failure is truthful;
        # LASTEXITCODE can belong to a previous native command and is not reused.
        self.assertEqual(output.count(b'\x1b]133;D;1\x07'), 1)
        self.assertNotIn(b'\x1b]133;D;7\x07', output)
        self.assertEqual(output.count(b'\x1b]133;D;0\x07'), 1)
        self.assertEqual(output.count(b'NATIVE_PROMPT>'), 4)

    def test_namespace_collision_leaves_native_prompt_untouched(self):
        output = run_script("$global:__amxS='owned'\nfunction global:prompt { 'NATIVE_PROMPT>' }", "prompt\n", no_module=True)
        self.assertEqual(masks(output), [])
        self.assertIn(b'NATIVE_PROMPT>', output)

    def test_strict_native_profile_and_unset_exitcode_are_supported(self):
        output = run_script("Set-StrictMode -Version Latest\nfunction global:prompt { 'NATIVE_PROMPT>' }", "prompt\n", no_module=True)
        self.assertEqual(masks(output), [3])
        self.assertIn(b'\x1b]133;P;k=c;aid=1\x07', output)
        self.assertNotIn(b'\x1b]7;', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--powershell', required=True)
    args, rest = parser.parse_known_args()
    FIXTURE = args.fixture
    POWERSHELL = shutil.which(args.powershell)
    if not POWERSHELL:
        parser.error('native PowerShell executable is unavailable')
    unittest.main(argv=[__file__, *rest])
