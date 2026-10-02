#!/usr/bin/env python3
"""Actual generated helper producer over a disposable native Windows named pipe."""
from __future__ import annotations

import argparse
import base64
import os
from pathlib import Path
import re
import tempfile
import unittest

import qa_process
import test_ssh_helper_adapters as contract

ROOT = Path(__file__).resolve().parents[2]
POWERSHELL = ''
SHELL = ''

SETUP = r'''
$ErrorActionPreference='Stop'
Import-Module Microsoft.PowerShell.Utility
Import-Module Microsoft.PowerShell.Management
function global:prompt { 'NATIVE_PROMPT>' }
$fixtureName='Automexia.Ssh.'+[guid]::NewGuid().ToString('N')+[guid]::NewGuid().ToString('N')
$fixturePipe=[IO.Pipes.NamedPipeServerStream]::new($fixtureName,[IO.Pipes.PipeDirection]::In,1,[IO.Pipes.PipeTransmissionMode]::Byte,[IO.Pipes.PipeOptions]::Asynchronous,4096,4096)
$fixtureConnection=$fixturePipe.BeginWaitForConnection($null,$null)
$env:AMX_SSH_HELPER_PIPE=$fixtureName
Set-Location $PSScriptRoot
function Read-FixtureFrame {
 $bytes=[byte[]]::new(32768)
 $pending=$fixturePipe.BeginRead($bytes,0,$bytes.Length,$null,$null)
 if(-not $pending.AsyncWaitHandle.WaitOne(2000)){throw 'fixture read timeout'}
 $count=$fixturePipe.EndRead($pending)
 if($count -le 0){throw 'fixture empty read'}
 [Console]::WriteLine('FRAME:'+ [Convert]::ToBase64String($bytes,0,$count))
}
try {
 . (Join-Path $PSScriptRoot rc.ps1)
 if(-not $fixtureConnection.AsyncWaitHandle.WaitOne(2000)){throw 'fixture connect timeout'}
 $fixturePipe.EndWaitForConnection($fixtureConnection)
'''
FINALLY = r'''
} finally {
 if($global:__amx_ssh_helper_state){$global:__amx_ssh_helper_state.Transport.Dispose()}
 $fixturePipe.Dispose()
}
'''


def run(body: str) -> bytes:
    with tempfile.TemporaryDirectory(prefix='automexia-helper-powershell-') as directory:
        root = Path(directory)
        source = contract.generated(SHELL)['rc.ps1']
        (root / 'rc.ps1').write_text(source, encoding='utf-8-sig')
        script = root / 'test.ps1'
        script.write_text(SETUP + body + FINALLY, encoding='utf-8-sig')
        environment = os.environ.copy()
        for name in contract.FIELDS:
            environment.pop(name, None)
        output = bytearray()

        def consume(data: bytes) -> None:
            if len(output) + len(data) > 256 * 1024:
                raise ValueError('native helper fixture output limit')
            output.extend(data)

        result = qa_process.run([POWERSHELL, '-NoLogo', '-NoProfile', '-NonInteractive', '-File', str(script)],
                                cwd=ROOT, timeout_seconds=20, consume=consume, environment=environment)
        if result.return_code or result.error or result.timed_out:
            raise AssertionError('native helper fixture failed: ' + bytes(output).decode(errors='replace'))
        return bytes(output)


def frames(output: bytes) -> list[tuple[int, dict[str, str]]]:
    stream = b''.join(base64.b64decode(value, validate=True)
                      for value in re.findall(rb'FRAME:([A-Za-z0-9+/=]+)', output))
    result = []
    for frame in stream.split(b'\0'):
        if not frame:
            continue
        lines = frame.decode('utf-8').splitlines()
        header = lines[0].split('|')
        if header[:3] != ['AMXREQ1', '3', '7'] or len(header) != 4 or len(lines) != 26:
            raise AssertionError('native helper frame structure')
        fields = [line.split('=', 1) for line in lines[1:]]
        if [field[0] for field in fields] != contract.FIELDS or len(frame) + 2 > 16384:
            raise AssertionError('native helper frame bounds')
        result.append((int(header[3]), dict(fields)))
    return result


class PowerShellHelperContracts(unittest.TestCase):
    def test_complete_stable_and_changed_snapshot_and_endpoint_privacy(self):
        output = run(r'''
Read-FixtureFrame
__amx_ssh_helper_prompt
Read-FixtureFrame
$env:GIT_BRANCH='fixture-branch';$env:DOCKER_HOST='unix:///fixture.sock'
__amx_ssh_helper_prompt
Read-FixtureFrame
if($env:AMX_SSH_HELPER_PIPE){throw 'endpoint remained exported'}
''')
        records = frames(output)
        self.assertEqual([item[0] for item in records], [1, 1, 2])
        self.assertEqual(records[-1][1]['GIT_BRANCH'], 'fixture-branch')
        self.assertEqual(records[-1][1]['DOCKER_HOST_PRESENT'], '1')
        self.assertNotIn('fixture.sock', repr(records))
        self.assertEqual(contract.revisions(output)[-1], 'AMXSSHREV1|3|7|2')

    def test_controls_utf8_and_total_budget_clear_complete_snapshot(self):
        output = run(r'''
Read-FixtureFrame
$env:AWS_PROFILE=[string][char]1+'invalid'
__amx_ssh_helper_prompt;Read-FixtureFrame
__amx_ssh_helper_prompt;Read-FixtureFrame
$env:AWS_PROFILE=([string][char]0x732b)*86
__amx_ssh_helper_prompt;Read-FixtureFrame
$env:AWS_PROFILE=([string][char]0x732b)*85
__amx_ssh_helper_prompt;Read-FixtureFrame
$env:KUBECONFIG='x'*4096;$env:HOMEDRIVE='x'*4096;$env:HOMEPATH='x'*4096;$env:USERPROFILE='x'*4096
__amx_ssh_helper_prompt;Read-FixtureFrame
''')
        records = frames(output)
        self.assertEqual([item[0] for item in records], [1, 2, 2, 2, 3, 4])
        for index in (1, 2, 3, 5):
            self.assertTrue(all(value == '' for value in records[index][1].values()))
        self.assertEqual(records[4][1]['AWS_PROFILE'], '猫' * 85)

    def test_duplicate_source_preserves_transport_and_revision_exhaustion_retires(self):
        output = run(r'''
Read-FixtureFrame
$transport=$global:__amx_ssh_helper_state.Transport
. (Join-Path $PSScriptRoot rc.ps1)
if(-not [Object]::ReferenceEquals($transport,$global:__amx_ssh_helper_state.Transport)){throw 'duplicate transport'}
$global:__amx_ssh_helper_state.Revision=[uint32]::MaxValue
$env:GIT_BRANCH='overflow'
__amx_ssh_helper_prompt
if(-not $global:__amx_ssh_helper_state.Disabled){throw 'revision wrapped'}
prompt
''')
        self.assertEqual(len(frames(output)), 1)
        self.assertEqual(contract.revisions(output)[-1], '')
        self.assertIn(b'NATIVE_PROMPT>', output)

    def test_pending_write_never_waits_and_keeps_one_buffer(self):
        output = run(r'''
Read-FixtureFrame
$env:KUBECONFIG='x'*4096;$env:HOMEDRIVE='x'*4096
for($i=0;$i -lt 32;$i++){
 __amx_ssh_helper_prompt
 if($global:__amx_ssh_helper_state.Pending -and -not $global:__amx_ssh_helper_state.Pending.IsCompleted){break}
}
$h=$global:__amx_ssh_helper_state
if(-not $h.Pending -or $h.Pending.IsCompleted){throw 'fixture failed to apply backpressure'}
$pending=$h.Pending;$buffer=$h.Buffer
$clock=[Diagnostics.Stopwatch]::StartNew()
for($i=0;$i -lt 64;$i++){$env:GIT_BRANCH='pending-'+$i;__amx_ssh_helper_prompt}
$clock.Stop()
if($clock.ElapsedMilliseconds -gt 2000){throw 'prompt waited for pipe capacity'}
if(-not [Object]::ReferenceEquals($pending,$h.Pending) -or -not [Object]::ReferenceEquals($buffer,$h.Buffer)){throw 'more than one pending write'}
if($h.Buffer.Length -gt 16384){throw 'unbounded pending buffer'}
Read-FixtureFrame
''')
        self.assertTrue(frames(output))
        self.assertNotEqual(contract.revisions(output)[-1], '')

    def test_closed_helper_clears_authority_without_prompt_block(self):
        output = run(r'''
Read-FixtureFrame
$fixturePipe.Dispose()
for($i=0;$i -lt 100;$i++){
 __amx_ssh_helper_prompt
 if($global:__amx_ssh_helper_state.Disabled){break}
 [Threading.Thread]::Sleep(5)
}
if(-not $global:__amx_ssh_helper_state.Disabled){throw 'dead pipe retained authority'}
prompt
''')
        self.assertEqual(contract.revisions(output)[-1], '')
        self.assertIn(b'NATIVE_PROMPT>', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--powershell', required=True)
    parser.add_argument('--shell', choices=('powershell', 'pwsh'), required=True)
    arguments, remaining = parser.parse_known_args()
    contract.FIXTURES = arguments.fixtures.resolve(strict=True)
    POWERSHELL = arguments.powershell
    SHELL = arguments.shell
    unittest.main(argv=[__file__, *remaining])
