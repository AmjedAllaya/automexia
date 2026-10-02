#!/usr/bin/env python3
"""Opt-in generated helper transfer cancellation over isolated real OpenSSH.

Requires an existing immutable fixture image; never pulls or installs services.
Reuses the SSH fixture and PTY lifecycle owners. Linux fixture evidence only.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import sys
import time
import unittest

import test_ssh_wrapper_runtime as runtime
import test_ssh_bash_core as pty_owner

IMAGE = ''
EXPORTER = ''


class HelperTransferRuntimeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not IMAGE and not EXPORTER:
            raise unittest.SkipTest('requires explicit image and actual Rust exporter fixtures')
        if not IMAGE or not EXPORTER:
            raise ValueError('both image and actual Rust exporter fixtures are required')
        if sys.platform != 'linux':
            raise RuntimeError('configured transfer runtime tests require native Linux')

    def test_disconnect_retires_hanging_probe_group_and_exact_lease(self) -> None:
        with runtime.Fixture(IMAGE, executable_temp=True) as fixture:
            nonce = secrets.token_hex(32)
            marker = '/tmp/descriptor-marker-' + secrets.token_hex(16)
            # Self-expiry protects the fixture even before the regression fix.
            payload = ('#!/bin/sh\nprintf "%s\\n" "$$" > ' + marker
                       + '\nexec /bin/sleep 15\n').encode()
            supplied = fixture.root / 'descriptor-fixture'
            supplied.write_bytes(payload)
            _, exported = runtime.run([EXPORTER, 'bash', nonce, str(len(payload)),
                                       hashlib.sha256(payload).hexdigest()])
            lines = exported.splitlines()
            self.assertEqual(lines[0], b'AMXSSH-HELPER-FIXTURE-1')
            records = dict(line.split(b'=', 1) for line in lines[1:])
            stage = base64.b64decode(records[b'stage'], validate=True).decode('utf-8')
            arguments = [fixture.ssh, '-T', '-F', str(fixture.config), 'fixture-host', stage]
            command = fixture.root / 'arguments.json'
            command.write_text(json.dumps(arguments), encoding='utf-8')
            relay = [sys.executable, str(Path(__file__).resolve()), '--relay',
                     str(supplied), str(command)]
            with pty_owner.Shell(launch_arguments=relay) as client:
                deadline = time.monotonic() + 4
                remote = None
                while time.monotonic() < deadline:
                    code, value = runtime.run(['docker', 'exec', fixture.container, 'cat', marker], check=False)
                    if code == 0 and value.strip().isdigit() and len(value.strip()) <= 10:
                        remote = int(value.strip())
                        break
                    time.sleep(.025)
                self.assertIsNotNone(remote, 'the uploaded descriptor did not actually execute')
                _, before = runtime.run(['docker', 'exec', fixture.container, 'cat', f'/proc/{remote}/stat'])
                initial = before.split()
                group = int(initial[4])
                lease = '/tmp/automexia-ssh.' + nonce
                # The PTY owner keeps this exact child unreaped until close, so
                # its PID cannot be reused while this signal is delivered.
                os.kill(client.pid, signal.SIGTERM)
                deadline = time.monotonic() + 6
                alive = True
                while time.monotonic() < deadline:
                    code, after = runtime.run(['docker', 'exec', fixture.container, 'cat', f'/proc/{remote}/stat'], check=False)
                    values = after.split()
                    alive = code == 0 and values[21] == initial[21] and values[2] != b'Z'
                    if not alive:
                        break
                    time.sleep(.025)
                self.assertFalse(alive, 'same remote descriptor survived its deadline after disconnect')
                code, _ = runtime.run(['docker', 'exec', fixture.container, 'test', '-d', lease], check=False)
                self.assertNotEqual(code, 0, 'the disconnected upload retained its exact lease')
                _, processes = runtime.run(['docker', 'exec', fixture.container, 'ps', '-eo', 'pgid=,stat='])
                active = [fields for line in processes.splitlines()
                          if len(fields := line.split()) == 2 and int(fields[0]) == group
                          and not fields[1].startswith(b'Z')]
                self.assertFalse(active, 'the descriptor group retained a live timer or descendant')
                observed = os.waitid(os.P_PID, client.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
                self.assertIsNotNone(observed, 'local OpenSSH did not acknowledge cancellation')


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == '--relay':
        argv = json.loads(Path(sys.argv[3]).read_text(encoding='utf-8'))
        descriptor = os.open(sys.argv[2], os.O_RDONLY)
        os.dup2(descriptor, 0)
        os.close(descriptor)
        os.write(1, pty_owner.PROMPT)
        os.execv(argv[0], argv)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True)
    parser.add_argument('--exporter', required=True)
    arguments, rest = parser.parse_known_args()
    if sys.platform != 'linux':
        parser.error('this opt-in fixture requires native Linux process identity checks')
    IMAGE = arguments.image
    EXPORTER = str(Path(arguments.exporter).resolve())
    unittest.main(argv=[sys.argv[0], *rest])
