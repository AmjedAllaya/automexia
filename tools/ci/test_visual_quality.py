"""Evidence-contract regressions; renderer and pixel tests live with their owners."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import visual_quality as visual


class VisualQualityTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        # Hosted Windows/macOS temporary directories can have canonical aliases
        # (for example /var -> /private/var). Match the production path owner.
        self.root = Path(self.temporary.name).resolve()

    def fixture(self, **updates):
        image = b'owned-image-fixture'
        (self.root / 'case.png').write_bytes(image)
        value = {'schema': 1, 'scenario': 'case', 'variant': 'correct', 'fixture': 'visual-quality-v1',
                 'evidence_kind': 'controlled-raster', 'renderer': 'cpu', 'width': 80, 'height': 24,
                 'scale_milli': 1000, 'font_size_milli': 16000, 'theme': 'aurora-night',
                 'image_sha256': hashlib.sha256(image).hexdigest(), 'frame_generation': 1,
                 'font_hashes': ['a' * 64], 'geometry': [{'id': 'cell', 'rect_milli': [0, 0, 10000, 24000]}]}
        value.update(updates)
        path = self.root / 'case.capture.json'
        path.write_text(json.dumps(value), encoding='utf8')
        return path

    def test_sealed_metadata_contains_only_allowlisted_capture_and_environment_facts(self):
        path = self.fixture()
        name, variant, sealed = visual.seal(path, 'b' * 40, True, visual.environment_identity())
        metadata = json.loads(sealed.read_bytes())
        self.assertEqual((name, variant), ('case', 'correct'))
        self.assertEqual(metadata['source_commit'], 'b' * 40)
        self.assertTrue(metadata['source_dirty'])
        self.assertEqual(metadata['identity']['evidence_kind'], 'controlled-raster')
        self.assertEqual(len(metadata['identity']['configuration_sha256']), 64)
        self.assertNotIn(str(self.root), sealed.read_text())

    def test_stale_unknown_path_and_native_claims_are_rejected(self):
        for update in ({'schema': 2}, {'schema': True}, {'variant': []}, {'secret': 'not-admitted'}, {'scenario': '../outside'},
                       {'image_sha256': '0' * 64}, {'variant': 'auto-approved'},
                       {'evidence_kind': 'native-window'}, {'renderer': 'wgpu-dx12'}):
            with self.subTest(update=update), self.assertRaises(ValueError):
                visual.seal(self.fixture(**update), 'b' * 40, False, visual.environment_identity())

    def test_empty_partial_and_duplicate_matrices_cannot_pass(self):
        for cases in ([], [('case', 'correct', self.root)], [('case', 'correct', self.root)] * 2):
            with self.assertRaises(ValueError):
                visual.validate_inventory(cases)

    def test_stale_missing_or_unbound_comparator_reports_do_not_pass(self):
        destination = self.root / 'diff'
        with patch.object(visual, 'command', return_value=(0, b'')):
            with self.assertRaises(ValueError):
                visual.compare(self.root, self.root / 'expected', self.root / 'actual', destination)
            for report in ({'schema': 1, 'status': 'passed'},
                           {'schema': 2, 'status': 'passed'},
                           {'schema': 2, 'status': 'passed', 'binding': {'identity_verified': True}},
                           {'schema': 2, 'status': 'failed', 'binding': {'identity_verified': True}}):
                destination.with_suffix('.json').write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    visual.compare(self.root, self.root / 'expected', self.root / 'actual', destination)

    def test_bounded_regular_input_and_atomic_report(self):
        path = self.root / 'evidence.json'
        visual.atomic_json(path, {'schema': 1})
        self.assertEqual(json.loads(visual.read(path, 64)), {'schema': 1})
        with self.assertRaises(ValueError):
            visual.read(path, 1)
        with self.assertRaises(ValueError):
            visual.read(self.root, 64)
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ['evidence.json'])

    def test_zero_test_success_is_not_a_capture_pass(self):
        target = self.root / 'run'
        (self.root / 'test').write_bytes(b'fixture-binary')
        with patch.object(visual, 'source_fingerprint', return_value='c' * 64), patch.object(visual, 'command', side_effect=[(0, b'b' * 40), (0, b''),
                                                        (0, b'test result: ok. 0 passed;')]):
            result = visual.main(['--test-binary', str(self.root / 'test'), '--xtask', str(self.root / 'xtask'), '--output', str(target)])
        self.assertEqual(result, 1)
        self.assertEqual(json.loads((target / 'summary.json').read_bytes())['status'], 'failed')
        self.assertEqual(json.loads((target / 'summary.json').read_bytes())['stage'], 'capture')

    def test_receipts_reject_dirty_partial_duplicate_and_unbounded_inventories(self):
        path = self.root / 'receipts.json'
        self.assertEqual(visual.load_receipts(path, {'case'}), {})
        metadata = {'source_dirty': False, 'identity': {'scenario': 'case'}}
        good = {'schema': 1, 'captures': {'case': metadata}}
        path.write_text(json.dumps(good), encoding='utf8')
        self.assertEqual(visual.load_receipts(path, {'case'}), good['captures'])
        for bad in ({}, {'schema': 2, 'captures': {}}, {'schema': True, 'captures': {'case': metadata}}, {'schema': 1, 'captures': {}},
                    {'schema': 1, 'captures': {'case': dict(metadata, source_dirty=True)}},
                    {'schema': 1, 'captures': {'case': dict(metadata, identity=[])}},
                    {'schema': 1, 'captures': {'case': dict(metadata, extra='x' * visual.MAX_JSON)}}):
            path.write_text(json.dumps(bad), encoding='utf8')
            with self.assertRaises(ValueError):
                visual.load_receipts(path, {'case'})
        path.write_text('{"schema":1,"schema":1,"captures":{}}', encoding='utf8')
        with self.assertRaises(ValueError):
            visual.load_receipts(path, {'case'})

    def test_missing_receipts_preserve_validated_candidates_and_verify_source_before_failing(self):
        target = self.root / 'run'
        binary = self.root / 'test'
        binary.write_bytes(b'fixture-binary')
        def run(argv, *_args, **_kwargs):
            if argv[:3] == ['git', 'rev-parse', 'HEAD']:
                return 0, b'b' * 40
            if argv[0] == 'git':
                return 0, b''
            if argv[0] == str(binary):
                (target / 'captures' / 'case.capture.json').write_bytes(b'{}')
                return 0, b'test result: ok. 1 passed;'
            self.assertIn('--validate-metadata', argv)
            return 0, b''
        with patch.object(visual, 'command', side_effect=run), \
                patch.object(visual, 'source_fingerprint', return_value='c' * 64) as fingerprint, \
                patch.object(visual, 'seal', return_value=('case', 'correct', target / 'captures/case.json')), \
                patch.object(visual, 'validate_inventory'):
            result = visual.main(['--test-binary', str(binary), '--xtask', str(self.root / 'xtask'),
                                  '--output', str(target), '--baseline-receipts', str(self.root / 'missing.json'), '--require-baseline'])
        self.assertEqual(result, 1)
        self.assertEqual(fingerprint.call_count, 2)
        summary = json.loads((target / 'summary.json').read_bytes())
        self.assertEqual(summary['baseline'], 'failed-or-missing')
        self.assertEqual(summary['cases'], [{'scenario': 'case', 'status': 'baseline-missing'}])
        self.assertEqual(summary['final_source_fingerprint'], summary['source_fingerprint'])
        self.assertTrue((target / 'captures/case.capture.json').is_file())

    def test_reviewed_image_store_requires_exact_named_bytes_without_path_traversal(self):
        data = b'reviewed image fixture'
        digest = hashlib.sha256(data).hexdigest()
        image = self.root / (digest + '.png')
        image.write_bytes(data)
        self.assertEqual(visual.read_baseline_image(self.root, {'image_sha256': digest}), data)
        for bad in ('../escape', '', 'A' * 64, None, 10):
            with self.assertRaises(ValueError):
                visual.read_baseline_image(self.root, {'image_sha256': bad})
        image.write_bytes(b'changed')
        with self.assertRaises(ValueError):
            visual.read_baseline_image(self.root, {'image_sha256': digest})
        image.unlink()
        with self.assertRaises(ValueError):
            visual.read_baseline_image(self.root, {'image_sha256': digest})

    def test_stream_digest_and_source_fingerprint_track_untracked_contents(self):
        path = self.root / 'fixture'
        data = b'abc' * 65536
        path.write_bytes(data)
        self.assertEqual(visual.file_digest(path, len(data)), hashlib.sha256(data).hexdigest())
        with self.assertRaises(ValueError):
            visual.file_digest(path, len(data) - 1)
        with patch.object(visual, 'ROOT', self.root), patch.object(visual, 'command', side_effect=[(0, b'diff'), (0, b'fixture\0')] * 2):
            before = visual.source_fingerprint('a' * 40)
            path.write_bytes(b'changed')
            self.assertNotEqual(before, visual.source_fingerprint('a' * 40))

    def test_command_output_overflow_is_not_silently_truncated(self):
        from types import SimpleNamespace
        def oversized(*args, **kwargs):
            kwargs['consume'](b'x' * (visual.MAX_OUTPUT + 1))
            return SimpleNamespace(timed_out=False, error=None, return_code=0)
        with patch.object(visual.qa_process, 'run', side_effect=oversized), self.assertRaises(ValueError):
            visual.command(['fixture'], 1)

    def test_source_digest_does_not_include_git_conversion_warnings(self):
        from types import SimpleNamespace
        def git(*args, **kwargs):
            self.assertFalse(kwargs['merge_stderr'])
            kwargs['consume'](b'patch' if args[0][1] == 'diff' else b'')
            return SimpleNamespace(timed_out=False, error=None, return_code=0)
        with patch.object(visual.qa_process, 'run', side_effect=git):
            self.assertEqual(visual.source_fingerprint('a' * 40),
                             hashlib.sha256(b'a' * 40 + b'patch').hexdigest())

    def test_command_failure_reports_only_allowlisted_stage_and_reason(self):
        from types import SimpleNamespace
        with patch.object(visual.qa_process, 'run', return_value=SimpleNamespace(timed_out=True, error='private output', return_code=None)):
            with self.assertRaises(visual.CommandFailure) as failure:
                visual.command(['git', 'diff'], 1)
        self.assertEqual((failure.exception.command_kind, failure.exception.reason), ('git-diff', 'deadline'))
        self.assertNotIn('private', str(failure.exception))

    def test_build_resolves_chunked_cargo_artifact_and_rejects_ambiguity(self):
        from types import SimpleNamespace
        record = {'reason': 'compiler-artifact', 'target': {'name': 'automexia'},
                  'profile': {'test': True}, 'executable': str(self.root / 'harness')}
        def build(*args, **kwargs):
            payload = json.dumps(record).encode() + b'\n'
            for byte in payload:
                kwargs['consume'](bytes([byte]))
            return SimpleNamespace(timed_out=False, error=None, return_code=0)
        with patch.object(visual.qa_process, 'run', side_effect=build):
            self.assertEqual(visual.build_test_binary(), (self.root / 'harness').resolve())
        with patch.object(visual.qa_process, 'run', return_value=SimpleNamespace(timed_out=False, error=None, return_code=0)), self.assertRaises(RuntimeError):
            visual.build_test_binary()

    def test_windows_persistence_oracles_share_the_current_preference_owner(self):
        # A stale literal once made save checks read v12 after migration to v14.
        # Empty obsolete files must never satisfy no-save/cancel checks.
        integration = visual.ROOT / 'tests/integration'
        for name in ('resize-stress-windows.ps1', 'theme-gallery-windows.ps1',
                     'font-picker-windows.ps1', 'shared-color-picker-windows.ps1'):
            source = (integration / name).read_text(encoding='utf8')
            self.assertNotRegex(source, r"['\"]state/user-preferences-v[0-9]+\.toml['\"]")
            self.assertIn('$preferenceRelativePath', source)
        owner = (visual.ROOT / 'apps/automexia-terminal/src/automexia/preferences.rs').read_text(encoding='utf8')
        self.assertRegex(owner, r'const PRIMARY_FILE: &str = "user-preferences-v[1-9][0-9]*\.toml";')

    def test_hosted_visual_runs_cannot_silently_become_capture_only(self):
        for name in ('ci.yml', 'accessibility-native.yml'):
            source = (visual.ROOT / '.github/workflows' / name).read_text(encoding='utf8')
            commands = [line.strip() for line in source.splitlines()
                        if line.strip().startswith('python tools/ci/visual_quality.py ')]
            self.assertEqual(len(commands), 1, name)
            for required in ('--build ', '--baseline-receipts ', '--require-baseline',
                             '--baseline-images tests/fixtures/visual-baselines/images',
                             'tests/fixtures/visual-baselines/', '--output artifacts/visual-quality'):
                self.assertIn(required, commands[0], name)
            self.assertNotIn('||', commands[0], name)


if __name__ == '__main__':
    unittest.main()
