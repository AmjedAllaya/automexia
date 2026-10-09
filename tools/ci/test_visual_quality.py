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

    def test_capture_runs_each_exact_test_with_individual_deadline(self):
        log = self.root / 'tests.log'
        success = b'test result: ok. 1 passed; 0 failed; 0 ignored;'
        with patch.object(visual, 'command', side_effect=[
                (0, b'a::visual_quality_one: test\nb::visual_quality_two: test\n'),
                (0, success), (0, success)]) as command:
            visual.capture_tests(self.root / 'harness', {}, log)
        self.assertEqual(command.call_args_list[1].args[0][1:],
                         ['a::visual_quality_one', '--exact', '--test-threads=1'])
        self.assertEqual(command.call_args_list[2].args[0][1:],
                         ['b::visual_quality_two', '--exact', '--test-threads=1'])
        self.assertTrue(all(call.args[1] <= 120 for call in command.call_args_list))
        self.assertEqual(log.read_bytes().count(success), 2)

    def test_capture_rejects_empty_duplicate_malformed_and_ignored_tests(self):
        for listing in (b'', b'elsewhere: test', b'a::visual_quality: test\n' * 2,
                        b'a::visual_quality: benchmark', b'a::visual_quality: test\n' * 65):
            with self.subTest(listing=listing[:60]), patch.object(visual, 'command', return_value=(0, listing)), self.assertRaises(ValueError):
                visual.capture_tests(self.root, {}, self.root / 'log')
        for result in (b'test result: ok. 0 passed; 0 failed; 1 ignored;',
                       b'test result: ok. 2 passed; 0 failed; 0 ignored;'):
            with patch.object(visual, 'command', side_effect=[(0, b'a::visual_quality: test'), (0, result)]), self.assertRaises(ValueError):
                visual.capture_tests(self.root, {}, self.root / 'log')

    def test_capture_retains_partial_failure_log_and_stops_after_deadline(self):
        log = self.root / 'tests.log'
        with patch.object(visual, 'command', side_effect=[(0, b'a::visual_quality: test'),
                visual.CommandFailure(['fixture'], True, False, b'partial fixture output')]), self.assertRaises(visual.CommandFailure):
            visual.capture_tests(self.root, {}, log)
        self.assertIn(b'partial fixture output', log.read_bytes())
        with patch.object(visual, 'command', return_value=(0, b'a::visual_quality: test')) as command, \
                patch.object(visual.time, 'monotonic', side_effect=[1, 1202]), self.assertRaises(visual.CommandFailure):
            visual.capture_tests(self.root, {}, log)
        self.assertEqual(command.call_count, 1)

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
                if '--list' in argv:
                    return 0, b'a::visual_quality: test'
                (target / 'captures' / 'case.capture.json').write_bytes(b'{}')
                return 0, b'test result: ok. 1 passed; 0 failed; 0 ignored;'
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
        with patch.object(visual, 'ROOT', self.root), \
                patch.object(visual, 'hash_source_diff', side_effect=lambda digest: digest.update(b'diff')), \
                patch.object(visual, 'command', return_value=(0, b'fixture\0')):
            before = visual.source_fingerprint('a' * 40)
            path.write_bytes(b'changed')
            self.assertNotEqual(before, visual.source_fingerprint('a' * 40))

    def test_source_diff_streams_large_fixtures_and_rejects_overflow_or_failure(self):
        from types import SimpleNamespace
        chunk = b'fixture-patch' * 8192
        def git(*args, **kwargs):
            self.assertFalse(kwargs['merge_stderr'])
            self.assertEqual(kwargs['timeout_seconds'], 30)
            for _ in range(24):
                kwargs['consume'](chunk)
            return SimpleNamespace(timed_out=False, error=None, return_code=0)
        digest = hashlib.sha256()
        with patch.object(visual.qa_process, 'run', side_effect=git):
            visual.hash_source_diff(digest)
        self.assertEqual(digest.digest(), hashlib.sha256(chunk * 24).digest())
        with patch.object(visual.qa_process, 'run', side_effect=git), \
                patch.object(visual, 'MAX_SOURCE_DIFF', len(chunk)), self.assertRaises(ValueError):
            visual.hash_source_diff(hashlib.sha256())
        for result in (SimpleNamespace(timed_out=True, error=None, return_code=None),
                       SimpleNamespace(timed_out=False, error='failed', return_code=None),
                       SimpleNamespace(timed_out=False, error=None, return_code=1)):
            with patch.object(visual.qa_process, 'run', return_value=result), self.assertRaises((ValueError, visual.CommandFailure)):
                visual.hash_source_diff(hashlib.sha256())

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

    def test_native_captures_cannot_precreate_the_visual_comparator_output(self):
        import shlex

        predecessors = 0
        for name in ('ci.yml', 'accessibility-native.yml'):
            source = (visual.ROOT / '.github/workflows' / name).read_text(encoding='utf8')
            lines = source.splitlines()
            comparator = next(index for index, line in enumerate(lines)
                              if line.strip().startswith('python tools/ci/visual_quality.py '))
            args = shlex.split(lines[comparator].strip())
            output = self.root / name / args[args.index('--output') + 1]
            for line in lines[:comparator]:
                if 'tests/integration/unix-session-ui.py ' not in line:
                    continue
                args = shlex.split(line.strip())
                capture = self.root / name / args[args.index('--captures') + 1]
                capture.mkdir(parents=True, exist_ok=False)
                predecessors += 1
            # Reproduce the real comparator's exclusive creation after earlier
            # native captures. Neither owner may remove or reuse stale output.
            output.mkdir(parents=True, exist_ok=False)
        self.assertGreater(predecessors, 0)

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
