#!/usr/bin/env python3
"""Native Windows tests of Rust-generated temporary helper staging commands.

The existing QA owner bounds the entire interpreter/fixture tree. The worker
only supplies a binary file to inherited stdin; it does not supervise children.
This tests staging locally, not Windows OpenSSH-server authentication.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import unittest

import qa_process

ROOT = Path(__file__).resolve().parents[2]
POWERSHELL = ''
EXPORTER = ''
SHELL = ''
HELPER = ''


def bounded(command: list[str], *, environment: dict[str, str] | None = None,
            timeout: float = 20) -> tuple[int, bytes]:
    output = bytearray()
    def consume(chunk: bytes) -> None:
        if len(output) + len(chunk) > 128 * 1024:
            raise ValueError('native transfer fixture output limit')
        output.extend(chunk)
    result = qa_process.run(command, cwd=ROOT, timeout_seconds=timeout,
                            consume=consume, environment=environment)
    if result.error or result.timed_out:
        raise AssertionError('native transfer fixture did not complete within its owned deadline')
    return result.return_code, bytes(output)


def encoded_stage(nonce: str, size: int, digest: str) -> str:
    code, output = bounded([EXPORTER, SHELL, nonce, str(size), digest])
    if code or not output.startswith(b'AMXSSH-HELPER-FIXTURE-1\n'):
        raise AssertionError('actual Rust upload fixture generation failed')
    entries = dict(line.split(b'=', 1) for line in output.splitlines()[1:])
    command = base64.b64decode(entries[b'stage'], validate=True).decode('utf-8')
    match = re.fullmatch(r'(?:powershell\.exe|pwsh) -NoLogo -NoProfile -EncodedCommand ([A-Za-z0-9+/=]+)', command)
    if match is None or len(command) > 8191:
        raise AssertionError('invalid bounded generated PowerShell command')
    return match[1]


class WindowsHelperTransferTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.temporary = tempfile.TemporaryDirectory(prefix='automexia-ssh-upload-tests-')
        cls.root = Path(cls.temporary.name)
        source = cls.root / 'fixture.cs'
        source.write_text(r'''using System;
using System.IO;
using System.Threading;
class Fixture {
 static int Main(string[] args) {
  if (args.Length != 1 || args[0] != "--describe-v1") return 23;
  string marker = Environment.GetEnvironmentVariable("AMX_TEST_PROBE_MARKER");
  if (!String.IsNullOrEmpty(marker)) File.WriteAllText(marker, "probed");
  switch (Environment.GetEnvironmentVariable("AMX_TEST_PROBE_MODE")) {
   case "hang": Thread.Sleep(30000); return 0;
   case "flood": Console.Write(new string('x', 131072)); return 0;
   case "bad": Console.Write("WRONG\n"); return 0;
   case "failed": Console.Write("AMXSSHHELPER1\n"); return 7;
   default: Console.Write("AMXSSHHELPER1\n"); return 0;
  }
 }
}''', encoding='utf-8')
        cls.helper = cls.root / 'fixture.exe'
        compiler = Path(os.environ['WINDIR']) / 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
        if not compiler.is_file():
            raise RuntimeError('native Windows fixture compiler is required')
        code, _ = bounded([str(compiler), '/nologo', '/target:exe', '/out:' + str(cls.helper), str(source)])
        if code or not cls.helper.is_file():
            raise AssertionError('native test helper compilation failed')
        cls.payload = cls.helper.read_bytes()
        cls.digest = hashlib.sha256(cls.payload).hexdigest()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def setUp(self) -> None:
        self.case = Path(tempfile.mkdtemp(prefix='case-', dir=self.root))
        self.nonce = secrets.token_hex(32)
        self.directory = self.case / ('automexia-ssh.' + self.nonce)
        self.marker = self.case / 'probe-marker'

    def stage(self, payload: bytes | None = None, *, digest: str | None = None,
              size: int | None = None, mode: str = '') -> tuple[int, bytes]:
        data = self.payload if payload is None else payload
        input_file = self.case / 'payload'
        input_file.write_bytes(data)
        command = encoded_stage(self.nonce, len(self.payload) if size is None else size,
                                self.digest if digest is None else digest)
        environment = os.environ.copy()
        environment.update(TEMP=str(self.case), TMP=str(self.case),
                           AMX_TEST_PROBE_MODE=mode, AMX_TEST_PROBE_MARKER=str(self.marker))
        return bounded([sys.executable, __file__, '--stage-worker', POWERSHELL,
                        command, str(input_file)], environment=environment)

    def assert_failed_cleanly(self, code: int, output: bytes) -> None:
        self.assertEqual(code, 73)
        self.assertNotIn(b'AMXSSHUPLOAD1|', output)
        self.assertIn(b'Automexia helper staging failed.', output)
        self.assertNotIn(str(self.case).encode(), output)
        self.assertFalse(self.directory.exists())

    def test_exact_binary_receipt_and_private_acl(self) -> None:
        code, output = self.stage()
        self.assertEqual(code, 0, output.decode(errors='replace'))
        prefix = f'AMXSSHUPLOAD1|3|7|{self.nonce}|{len(self.payload)}|{self.digest}|'.encode()
        self.assertTrue(output.startswith(prefix), output.decode(errors='replace'))
        returned = base64.b64decode(output.removeprefix(prefix).strip(), validate=True).decode('utf-8')
        self.assertEqual(Path(returned), self.directory)
        self.assertEqual((self.directory / 'helper.exe').read_bytes(), self.payload)
        self.assertTrue(self.marker.exists())
        script = self.case / 'acl.ps1'
        script.write_text("""param([string]$Directory)
$ErrorActionPreference='Stop'
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
function Read-Acl($item){if($PSVersionTable.PSVersion.Major-lt6){$item.GetAccessControl()}else{[IO.FileSystemAclExtensions]::GetAccessControl($item)}}
$d=Read-Acl (New-Object IO.DirectoryInfo($Directory))
if(!$d.AreAccessRulesProtected -or $d.GetOwner([Security.Principal.SecurityIdentifier]).Value-ne$sid){exit 31}
foreach($item in @((New-Object IO.DirectoryInfo($Directory)),(New-Object IO.FileInfo([IO.Path]::Combine($Directory,'helper.exe'))))){
 $a=Read-Acl $item
 foreach($r in $a.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])){
  if($r.IdentityReference.Value-ne$sid -or $r.AccessControlType-ne'Allow'){exit 32}
 }
}
[Console]::Write('PRIVATE')
""", encoding='utf-8-sig')
        code, output = bounded([POWERSHELL, '-NoLogo', '-NoProfile', '-NonInteractive', '-File', str(script), str(self.directory)])
        self.assertEqual((code, output), (0, b'PRIVATE'))

    def test_truncated_transfer_is_rejected_before_execution(self) -> None:
        self.assert_failed_cleanly(*self.stage(self.payload[:-1]))
        self.assertFalse(self.marker.exists())

    def test_actual_package_helper_description_is_accepted(self) -> None:
        payload = Path(HELPER).read_bytes()
        self.assertGreater(len(payload), 0)
        self.assertLessEqual(len(payload), 64 * 1024 * 1024)
        digest = hashlib.sha256(payload).hexdigest()
        code, output = self.stage(payload, size=len(payload), digest=digest)
        self.assertEqual(code, 0, output.decode(errors='replace'))
        self.assertTrue(output.startswith(f'AMXSSHUPLOAD1|3|7|{self.nonce}|{len(payload)}|{digest}|'.encode()))
        self.assertEqual(hashlib.sha256((self.directory / 'helper.exe').read_bytes()).hexdigest(), digest)

    def test_extra_bytes_are_rejected_before_execution(self) -> None:
        self.assert_failed_cleanly(*self.stage(self.payload + b'extra'))
        self.assertFalse(self.marker.exists())

    def test_hash_mismatch_is_rejected_before_execution(self) -> None:
        self.assert_failed_cleanly(*self.stage(digest='0' * 64))
        self.assertFalse(self.marker.exists())

    def test_preexisting_directory_is_never_modified_or_deleted(self) -> None:
        self.directory.mkdir()
        sentinel = self.directory / 'helper.exe'
        sentinel.write_bytes(b'preexisting')
        code, output = self.stage()
        self.assertEqual(code, 73)
        self.assertNotIn(b'AMXSSHUPLOAD1|', output)
        self.assertEqual(sentinel.read_bytes(), b'preexisting')
        self.assertFalse(self.marker.exists())

    def test_wrong_description_removes_owned_artifacts(self) -> None:
        self.assert_failed_cleanly(*self.stage(mode='bad'))
        self.assertTrue(self.marker.exists())

    def test_failed_description_exit_removes_owned_artifacts(self) -> None:
        self.assert_failed_cleanly(*self.stage(mode='failed'))
        self.assertTrue(self.marker.exists())

    def test_description_flood_is_bounded_and_retired(self) -> None:
        self.assert_failed_cleanly(*self.stage(mode='flood'))
        self.assertTrue(self.marker.exists())

    def test_description_timeout_is_bounded_and_retired(self) -> None:
        self.assert_failed_cleanly(*self.stage(mode='hang'))
        self.assertTrue(self.marker.exists())


if __name__ == '__main__':
    if len(sys.argv) == 5 and sys.argv[1] == '--stage-worker':
        # The caller's existing QA process job/session owns this short-lived
        # relay and its descendants, including all failure/timeout cleanup.
        with open(sys.argv[4], 'rb') as source:
            result = subprocess.run([sys.argv[2], '-NoLogo', '-NoProfile', '-EncodedCommand', sys.argv[3]],
                                    stdin=source, check=False)
        raise SystemExit(result.returncode)
    parser = argparse.ArgumentParser()
    parser.add_argument('--exporter', required=True)
    parser.add_argument('--powershell', required=True)
    parser.add_argument('--shell', choices=('powershell', 'pwsh'), required=True)
    parser.add_argument('--helper', required=True)
    args, rest = parser.parse_known_args()
    if os.name != 'nt':
        parser.error('native Windows is required; no skipped evidence')
    POWERSHELL = shutil.which(args.powershell) or ''
    EXPORTER = shutil.which(args.exporter) or ''
    SHELL = args.shell
    HELPER = shutil.which(args.helper) or ''
    if not POWERSHELL or not EXPORTER or not HELPER:
        parser.error('native PowerShell, compiled Rust exporter and package helper are required')
    unittest.main(argv=[__file__, *rest])
