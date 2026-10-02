#!/usr/bin/env python3
"""Native POSIX contracts for the actual Rust-generated helper transfer.

The existing QA process owner supplies the timeout and process cleanup boundary.
The relay only replaces stdin with a fixture file before executing exact argv.
No server, credentials, personal SSH config, or network connection is used.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import stat
import sys
import tempfile
import time
import unittest

import qa_process

ROOT = Path(__file__).resolve().parents[2]
EXPORTER = ''
SHELLS = ('bash', 'zsh')
PAYLOAD = b'''#!/bin/sh
case "$1" in
 --describe-v1) printf '%s\\n' AMXSSHHELPER1;;
 --session-v1) printf 'SESSION:%s:%s:%s\\n' "$2" "$3" "$4"; exit 7;;
 *) exit 64;;
esac
'''


def bounded(command: list[str], *, timeout: float = 20,
            merge_stderr: bool = True) -> tuple[int, bytes]:
    captured = bytearray()
    def consume(chunk: bytes) -> None:
        if len(captured) + len(chunk) > 128 * 1024:
            raise ValueError('native transfer fixture output limit')
        captured.extend(chunk)
    result = qa_process.run(command, cwd=ROOT, timeout_seconds=timeout,
                            consume=consume, merge_stderr=merge_stderr)
    if result.error or result.timed_out:
        raise AssertionError('native transfer fixture exceeded its owned execution budget')
    return result.return_code, bytes(captured)


class PosixHelperTransferTests(unittest.TestCase):
    def test_unsafe_optional_fish_transport_is_rejected_before_staging(self) -> None:
        code, output = bounded([EXPORTER, 'fish'])
        self.assertNotEqual(code, 0)
        self.assertNotIn(b'stage=', output)

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix='automexia-transfer-test-')
        self.local = Path(self.temporary.name)
        self.directories: list[Path] = []

    def tearDown(self) -> None:
        # Only exact fixture leases and known files; never recursive remote cleanup.
        for directory in self.directories:
            if directory.parent != Path('/tmp') or not directory.name.startswith('automexia-ssh.'):
                raise AssertionError('fixture cleanup identity mismatch')
            for name in ('helper', 'describe', 'probe-status', 'sentinel'):
                path = directory / name
                if path.exists() or path.is_symlink():
                    path.unlink()
            if directory.exists():
                directory.rmdir()
        self.temporary.cleanup()

    def fixture(self, shell: str, payload: bytes = PAYLOAD,
                *, size: int | None = None, digest: str | None = None) -> tuple[dict[str, bytes], Path]:
        nonce = secrets.token_hex(32)
        size = len(payload) if size is None else size
        digest = hashlib.sha256(payload).hexdigest() if digest is None else digest
        code, output = bounded([EXPORTER, shell, nonce, str(size), digest])
        self.assertEqual(code, 0, 'actual Rust helper fixture generation failed')
        lines = output.splitlines()
        self.assertEqual(lines[0], b'AMXSSH-HELPER-FIXTURE-1')
        result = {key.decode('ascii'): base64.b64decode(value, validate=True)
                  for key, value in (line.split(b'=', 1) for line in lines[1:])}
        directory = Path('/tmp') / ('automexia-ssh.' + nonce)
        self.directories.append(directory)
        return result, directory

    def run_source(self, shell: str, source: bytes, payload: bytes = b'') -> tuple[int, bytes]:
        stdin = self.local / 'input'
        stdin.write_bytes(payload)
        command = self.local / 'command.json'
        command.write_text(json.dumps([shutil.which(shell), '-c', source.decode('utf-8')]), encoding='utf-8')
        return bounded([sys.executable, str(Path(__file__).resolve()), '--relay', str(stdin), str(command)])

    def test_verified_upload_and_session_preserve_status_and_remove_exact_lease(self) -> None:
        for shell in SHELLS:
            with self.subTest(shell=shell):
                fixture, directory = self.fixture(shell)
                code, output = self.run_source(shell, fixture['stage'], PAYLOAD)
                self.assertEqual(code, 0, 'verified upload rejected by native shell')
                self.assertEqual(stat.S_IMODE(directory.stat().st_mode), 0o700)
                self.assertEqual(stat.S_IMODE((directory / 'helper').stat().st_mode), 0o700)
                self.assertEqual((directory / 'helper').read_bytes(), PAYLOAD)
                fields = output.strip().split(b'|')
                self.assertEqual(len(fields), 7, 'stage must emit exactly one receipt')
                self.assertEqual(fields[:3], [b'AMXSSHUPLOAD1', b'3', b'7'])
                self.assertEqual(base64.b64decode(fields[6], validate=True), str(directory).encode())
                self.assertEqual(fields[4:6], [str(len(PAYLOAD)).encode(), hashlib.sha256(PAYLOAD).hexdigest().encode()])
                code, output = self.run_source(shell, fixture['session'])
                self.assertEqual(code, 7, 'session exit status must survive cleanup')
                self.assertEqual(output, f'SESSION:{shell}:3:7\n'.encode())
                self.assertFalse(directory.exists(), 'successful session left its lease behind')

    def test_invalid_hash_or_payload_length_is_rejected_and_cleaned(self) -> None:
        for shell in SHELLS:
            for mode in ('hash', 'short', 'extra'):
                with self.subTest(shell=shell, mode=mode):
                    fixture, directory = self.fixture(shell, digest='0' * 64 if mode == 'hash' else None)
                    payload = PAYLOAD[:-1] if mode == 'short' else PAYLOAD + b'x' if mode == 'extra' else PAYLOAD
                    code, output = self.run_source(shell, fixture['stage'], payload)
                    self.assertNotEqual(code, 0)
                    self.assertNotIn(b'AMXSSHUPLOAD1', output)
                    self.assertFalse(directory.exists(), 'rejected upload left its lease behind')

    def test_bad_or_oversized_description_is_rejected_and_cleaned(self) -> None:
        for shell in SHELLS:
            for payload in (b'#!/bin/sh\nprintf WRONG\\n\n', b'#!/bin/sh\nhead -c 131072 /dev/zero\n',
                            b'#!/bin/sh\nprintf "AMXSSHHELPER1\\n"\nexit 7\n'):
                with self.subTest(shell=shell, probe=hashlib.sha256(payload).hexdigest()[:8]):
                    fixture, directory = self.fixture(shell, payload)
                    code, output = self.run_source(shell, fixture['stage'], payload)
                    self.assertNotEqual(code, 0)
                    self.assertNotIn(b'AMXSSHUPLOAD1', output)
                    self.assertFalse(directory.exists(), 'bad descriptor left its lease behind')

    def test_hanging_descriptor_has_a_remote_deadline_and_cleanup(self) -> None:
        payload = b'#!/bin/sh\nexec sleep 15\n'
        fixture, directory = self.fixture('bash', payload)
        started = time.monotonic()
        code, output = self.run_source('bash', fixture['stage'], payload)
        self.assertNotEqual(code, 0)
        self.assertNotIn(b'AMXSSHUPLOAD1', output)
        self.assertLess(time.monotonic() - started, 8, 'remote descriptor exceeded its five-second deadline')
        self.assertFalse(directory.exists(), 'timed-out descriptor left its lease behind')

    def test_exclusive_directory_collision_preserves_existing_files(self) -> None:
        for shell in SHELLS:
            with self.subTest(shell=shell):
                fixture, directory = self.fixture(shell)
                directory.mkdir(mode=0o700)
                (directory / 'sentinel').write_bytes(b'untouched')
                code, output = self.run_source(shell, fixture['stage'], PAYLOAD)
                self.assertNotEqual(code, 0)
                self.assertNotIn(b'AMXSSHUPLOAD1', output)
                self.assertEqual((directory / 'sentinel').read_bytes(), b'untouched')
                self.assertFalse((directory / 'helper').exists())

    def test_session_does_not_remove_unowned_files_and_reports_residue(self) -> None:
        for shell in SHELLS:
            with self.subTest(shell=shell):
                fixture, directory = self.fixture(shell)
                code, _ = self.run_source(shell, fixture['stage'], PAYLOAD)
                self.assertEqual(code, 0)
                (directory / 'sentinel').write_bytes(b'untouched')
                code, output = self.run_source(shell, fixture['session'])
                self.assertEqual(code, 7)
                self.assertIn(b'could not be fully removed', output)
                self.assertEqual((directory / 'sentinel').read_bytes(), b'untouched')
                self.assertFalse((directory / 'helper').exists())

    def test_native_openssh_effective_options_disable_transfer_side_effects(self) -> None:
        fixture, _ = self.fixture('bash')
        argv = fixture['argv'].decode('utf-8').split('\0')
        code, output = bounded([shutil.which('ssh'), '-G', *argv], merge_stderr=False)
        self.assertEqual(code, 0, 'generated transfer arguments must be accepted by native OpenSSH')
        # Never print effective config: defaults can contain identifying paths.
        facts = dict(line.split(b' ', 1) for line in output.splitlines() if b' ' in line)
        for key, value in {b'requesttty': b'false', b'forwardagent': b'no', b'forwardx11': b'no',
                           b'permitlocalcommand': b'no', b'clearallforwardings': b'yes',
                           b'controlmaster': b'false', b'controlpersist': b'no', b'tunnel': b'false',
                           b'gssapidelegatecredentials': b'no'}.items():
            self.assertEqual(facts.get(key), value, 'transfer option override is ineffective')
        self.assertNotIn(b'localforward', facts)
        self.assertNotIn(b'remoteforward', facts)


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == '--relay':
        descriptor = os.open(sys.argv[2], os.O_RDONLY)
        os.dup2(descriptor, 0)
        os.close(descriptor)
        argv = json.loads(Path(sys.argv[3]).read_text(encoding='utf-8'))
        os.execv(argv[0], argv)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exporter', required=True)
    arguments, rest = parser.parse_known_args()
    EXPORTER = str(Path(arguments.exporter).resolve())
    if os.name != 'posix' or any(shutil.which(shell) is None for shell in (*SHELLS, 'ssh')):
        parser.error('native Bash, Zsh and OpenSSH are required')
    unittest.main(argv=[sys.argv[0], *rest])
