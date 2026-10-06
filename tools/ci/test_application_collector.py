"""Collector isolation, workload identity, native resources and failure coverage."""
from pathlib import Path
import os
import io
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'renderer-benchmarks/application'))
import collector
import native
import workload
from benchmark_model import BenchmarkError, METRICS


class CollectorTests(unittest.TestCase):
    def test_native_search_query_matches_the_actual_fixture_history(self):
        with TemporaryDirectory() as temporary:
            output = io.BytesIO()
            environment = {'AUTOMEXIA_BENCHMARK_TOKEN': 'a' * 32,
                           'AUTOMEXIA_BENCHMARK_FIXTURE_OUTPUT': str(Path(temporary) / 'fixture.json')}
            with patch.dict(os.environ, environment), \
                 patch.object(workload.sys, 'stdout', SimpleNamespace(buffer=output)), \
                 patch.object(workload.time, 'sleep'), patch.object(workload, 'interactive'):
                self.assertEqual(workload.main(['interactive', '--seconds', '5']), 0)
            rows = output.getvalue().splitlines()
            self.assertEqual(len(rows), 1000)
            self.assertTrue(all(collector.SEARCH_QUERY.encode() in row for row in rows))

    def test_hidden_and_infrastructure_windows_are_never_input_targets(self):
        self.assertTrue(native.application_surface('native-window', 1280, 720, 0, 10))
        for args in [('Winit Thread Event Target', 1280, 720, 0, 10),
                     ('native-window', 1280, 720, 1, 10), ('native-window', 0, 0, 0, 10),
                     ('native-window', 1280, 720, 0, 0)]:
            self.assertFalse(native.application_surface(*args))

    def test_scenarios_cover_exact_metric_roster(self):
        self.assertEqual(set(collector.SCENARIOS), set(collector.SCENARIO_METRICS))
        self.assertEqual(set(METRICS), {name for names in collector.SCENARIO_METRICS.values() for name in names})

    def test_dirty_source_identity_includes_current_untracked_bytes(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'fixture.py'
            source.write_text('before', encoding='utf-8')
            with patch.object(collector, 'ROOT', root), \
                 patch.object(collector.qa, 'git_value', return_value='a' * 40), \
                 patch.object(collector.qa, 'source_status_bytes', return_value=b'?? fixture.py\0'):
                before = collector.source_identity()
                source.write_text('changed', encoding='utf-8')
                after = collector.source_identity()
            self.assertTrue(before[1])
            self.assertEqual(before[:2], after[:2])
            self.assertNotEqual(before[2], after[2])

    def test_missing_source_inventory_cannot_be_clean_evidence(self):
        with patch.object(collector.qa, 'git_value', return_value='a' * 40), \
             patch.object(collector.qa, 'source_status_bytes', return_value=None):
            with self.assertRaises(BenchmarkError):
                collector.source_identity()

    def test_fingerprint_rejects_empty_or_nonregular_input(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            empty = root / 'empty'
            empty.touch()
            for path in (root, empty):
                with self.assertRaises(BenchmarkError):
                    collector.sha_file(path)

    def test_environment_drops_credentials_and_user_settings(self):
        with patch.dict(os.environ, {'FAKE_CREDENTIAL': 'never-copy', 'AUTOMEXIA_CONFIG_HOME': 'user',
                                     'RIO_CONFIG_HOME': 'legacy', 'SSH_AUTH_SOCK': 'agent'}, clear=True):
            env = collector.environment(Path('isolated'), 'idle')
        self.assertNotIn('FAKE_CREDENTIAL', env)
        self.assertNotIn('RIO_CONFIG_HOME', env)
        self.assertNotIn('SSH_AUTH_SOCK', env)
        self.assertEqual(env['AUTOMEXIA_CONFIG_HOME'], 'isolated')
        self.assertEqual(env['HOME'], 'isolated')
        self.assertEqual(len(env['AUTOMEXIA_BENCHMARK_TOKEN']), 32)

    def test_config_is_valid_toml_exact_argv_and_recovery_disabled(self):
        import json
        import tomllib
        value = tomllib.loads(collector.CONFIG.format(cwd=json.dumps('fixture directory'),
                 python=json.dumps('python executable'), workload=json.dumps('workload.py'),
                 mode=json.dumps('interactive'), seconds=json.dumps('45')))
        self.assertEqual(value['shell']['args'], ['workload.py', 'interactive', '--seconds', '45'])
        self.assertFalse(value['session-recovery']['enabled'])
        self.assertFalse(value['confirm-before-quit'])
        self.assertEqual(len(value['bindings']['keys']), 3)

    def test_font_fingerprint_is_content_based_not_checkout_path(self):
        with TemporaryDirectory() as temporary:
            left, right = Path(temporary)/'one', Path(temporary)/'two'
            left.write_bytes(b'fixture-font'); right.write_bytes(b'fixture-font')
            self.assertEqual(collector.fingerprints([left]), collector.fingerprints([right]))
            right.write_bytes(b'changed-font')
            self.assertNotEqual(collector.fingerprints([left])['fonts_sha256'], collector.fingerprints([right])['fonts_sha256'])

    def test_native_resource_sensor_tracks_owned_process_then_rejects_exit(self):
        if sys.platform not in ('win32', 'linux', 'darwin'):
            self.skipTest('native resource adapter unavailable')
        with subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(2)'],
                              stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) as process:
            try:
                counter = native.Resources(process)
                rss, cpu, clock = counter.sample()
                self.assertGreater(rss, 0); self.assertGreaterEqual(cpu, 0); self.assertGreater(clock, 0)
                process.wait(timeout=5)
                with self.assertRaises(BenchmarkError):
                    counter.sample()
            finally:
                if process.poll() is None:
                    process.terminate(); process.wait(timeout=5)

    def test_no_interactive_driver_is_silently_emulated_on_unsupported_os(self):
        with patch.object(native.os, 'name', 'posix'), patch.object(native.sys, 'platform', 'unsupported'):
            with self.assertRaises(BenchmarkError):
                native.driver(object())


if __name__ == '__main__':
    unittest.main()
