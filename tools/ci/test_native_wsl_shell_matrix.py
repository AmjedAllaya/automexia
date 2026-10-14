import base64
import argparse
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import native_wsl_shell_matrix as matrix

from native_wsl_shell_matrix import decode_inventory, identity, parse_case, selected_version, source_digest, successful, timings


class WslMatrixTests(unittest.TestCase):
    def test_localized_utf16_inventory_and_exact_names(self):
        data = '\ufeff  NAME STATE VERSION\r\n* Ubuntu-24.04 Arrêté 1\r\n  Test Distro En cours 2\r\n'
        for encoded in (data.encode('utf-16-le'), data.encode('utf-8')):
            self.assertEqual(selected_version(encoded, 'Ubuntu-24.04'), 1)
            self.assertEqual(selected_version(encoded, 'Test Distro'), 2)
            with self.assertRaises(ValueError):
                selected_version(encoded, 'Ubuntu')

    def test_missing_duplicate_and_oversized_inventory_fail(self):
        for data in (b'', b'X Running 1\nX Running 2\n', b'x' * 65537):
            with self.assertRaises(ValueError):
                selected_version(data, 'X')
        with self.assertRaises(UnicodeError):
            decode_inventory(b'\xff\xfe\x00')

    def test_cases_require_version_and_literal_name(self):
        self.assertEqual(parse_case('Test Distro=2'), ('Test Distro', 2))
        for case in ('Ubuntu', 'Ubuntu=3', '=1', '-d=2', 'bad\nname=1'):
            with self.assertRaises(argparse.ArgumentTypeError):
                parse_case(case)

    def test_identity_requires_complete_matching_guest_frame(self):
        values = {'automexia_shell': '1', 'automexia_shell_name': 'fish',
                  'automexia_distro': 'Fixture', 'automexia_shell_user': 'alice',
                  'automexia_prompt_active': '1', 'automexia_os_name': 'Alpine Linux',
                  'automexia_shell_path': '/usr/bin/fish'}
        output = b''.join(b'\x1b]1337;SetUserVar=' + name.encode() + b'=' +
                          base64.b64encode(value.encode()) + b'\x07' for name, value in values.items())
        output += b'\x1b]7;file://localhost/tmp\x07\x1b]133;A\x07'
        identity(output, 'fish', 'Fixture')
        with self.assertRaises(ValueError):
            identity(output, 'fish', 'Fixture', 'different-user')
        for shell, distro in [('bash', 'Fixture'), ('fish', 'Other')]:
            with self.assertRaises(ValueError):
                identity(output, shell, distro)
        with self.assertRaises(ValueError):
            identity(b'', 'fish', 'Fixture')

    def test_benchmark_requires_real_finite_samples(self):
        self.assertEqual(timings(list(range(1, 31)))['p95_ms'], 29)
        self.assertEqual(timings(list(range(1, 31)))['median_ms'], 15.5)
        for samples in ([], [1] * 29, [float('nan')] * 30, [-1] * 30):
            with self.assertRaises(ValueError):
                timings(samples)

    def test_shell_budget_receipts_preserve_units_and_failures(self):
        receipts = (b'SHELL_BENCHMARK shell=bash operation=adapter p95=0.010 unit=s\n'
                    b'SHELL_BENCHMARK shell=bash operation=alias-reload p95=50.001 unit=ms\n')
        values = matrix.shell_benchmarks(receipts, 'bash')
        self.assertEqual(values['adapter']['p95_ms'], 10)
        self.assertEqual(values['adapter']['status'], 'passed')
        self.assertEqual(values['alias-reload']['status'], 'failed')
        self.assertEqual(matrix.shell_benchmarks(receipts + b'quoted: ' + receipts.splitlines()[0], 'bash'), values)
        for invalid in (b'', receipts + receipts, receipts.replace(b'50.001', b'nan'),
                        receipts.replace(b'unit=ms', b'unit=minutes')):
            with self.assertRaises(ValueError):
                matrix.shell_benchmarks(invalid, 'bash')
        with self.assertRaises(ValueError):
            matrix.shell_benchmarks(receipts, 'fish')

    def test_fish_receipts_must_not_join_human_summaries(self):
        lines = [b'SHELL_BENCHMARK shell=fish operation=adapter p95=45.52 unit=ms',
                 b'shell=fish adapter-p95=45.52ms budget=50ms',
                 b'SHELL_BENCHMARK shell=fish operation=alias-reload p95=53.62 unit=ms',
                 b'shell=fish alias-reload-p95=53.62ms budget=50ms']
        values = matrix.shell_benchmarks(b'\n'.join(lines) + b'\n', 'fish')
        self.assertEqual(values['adapter']['status'], 'passed')
        self.assertEqual(values['alias-reload']['status'], 'failed')
        # Quoting Fish's command-substitution list joined each receipt with its
        # human summary. A missing measurement must never turn into a pass.
        with self.assertRaises(ValueError):
            matrix.shell_benchmarks(b' '.join(lines[:2]) + b'\n' +
                                    b' '.join(lines[2:]) + b'\n', 'fish')

    def test_source_identity_changes_with_content_and_relative_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tree = root / 'shell-integration'
            tree.mkdir()
            file = tree / 'fixture.sh'
            file.write_text('first')
            first = source_digest(root)
            self.assertEqual(first, source_digest(root))
            file.write_text('second')
            second = source_digest(root)
            self.assertNotEqual(first, second)
            file.rename(tree / 'renamed.sh')
            self.assertNotEqual(second, source_digest(root))
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(ValueError):
                source_digest(Path(temporary))

    def test_a_failed_or_missing_case_cannot_report_success(self):
        self.assertTrue(successful([{'status': 'passed'}, {'status': 'passed'}]))
        for rows in ([], [{}], [{'status': 'passed'}, {'status': 'failed'}],
                     [{'status': 'passed'}, {'status': 'not_run'}]):
            self.assertFalse(successful(rows))

    def test_zsh_prompt_markers_do_not_hide_a_complete_behavioral_receipt(self):
        receipt = b'PASS: Zsh behavioral assertions completed'
        self.assertTrue(matrix.behavioral_receipt(receipt + b'\n', 'zsh'))
        self.assertTrue(matrix.behavioral_receipt(b'\x1b[0m\x1b]133;C\x07' + receipt + b'\r\n', 'zsh'))
        self.assertFalse(matrix.behavioral_receipt(b'fixture text: ' + receipt, 'zsh'))
        self.assertFalse(matrix.behavioral_receipt(receipt, 'bash'))

    def test_failed_native_suite_retains_report_and_fails_the_windows_gate(self):
        with tempfile.TemporaryDirectory() as temporary:
            report = Path(temporary) / 'report.json'
            payload = {'status': 'failed', 'shells': [{'shell': 'bash', 'suite': 'failed',
                                                      'identity': 'passed'}]}
            arguments = ['matrix', '--case', 'Fixture=1', '--report', str(report)]
            with patch.object(matrix.sys, 'platform', 'win32'), patch.object(matrix.sys, 'argv', arguments), \
                 patch.object(matrix, 'run', side_effect=[b'Fixture Running 1', b'/mnt/c/source\n', json.dumps(payload).encode()]):
                self.assertEqual(matrix.main(), 1)
            data = json.loads(report.read_text())
            self.assertEqual(data['status'], 'failed')
            self.assertEqual(data['cases'][0]['shells'][0]['identity'], 'passed')

    def test_wrong_wsl_version_stops_before_any_suite_and_replaces_stale_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            report = Path(temporary) / 'report.json'
            report.write_text('{"status":"passed"}')
            arguments = ['matrix', '--case', 'Fixture=2', '--report', str(report)]
            with patch.object(matrix.sys, 'platform', 'win32'), patch.object(matrix.sys, 'argv', arguments), \
                 patch.object(matrix, 'run', return_value=b'Fixture Running 1') as run:
                self.assertEqual(matrix.main(), 1)
                self.assertEqual(run.call_count, 1)
            self.assertEqual(json.loads(report.read_text())['status'], 'failed')


if __name__ == '__main__':
    unittest.main()
