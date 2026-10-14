"""First-alpha trust boundaries; stable release requirements remain independent."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import first_alpha as alpha

ROOT = Path(__file__).resolve().parents[2]


class FirstAlphaTests(unittest.TestCase):
    def test_exact_owner_authorized_identity_only(self):
        alpha.validate_identity('0.0.4-alpha', 'v0.0.4-alpha', 'a' * 40)
        for version, tag, commit in [('1.0.0', 'v1.0.0', 'a'*40),
                                     ('0.0.1-alpha', '0.0.1_alpha', 'a'*40),
                                     ('0.0.2-alpha', 'v0.0.2-alpha', 'a'*40),
                                     ('0.0.3-alpha', 'v0.0.3-alpha', 'a'*40),
                                     ('0.0.5-alpha', 'v0.0.5-alpha', 'a'*40),
                                     ('0.0.4-alpha', 'v0.0.4-alpha', 'main')]:
            with self.subTest(version=version, tag=tag, commit=commit):
                with self.assertRaises(ValueError):
                    alpha.validate_identity(version, tag, commit)

    def test_publication_requires_exact_main_and_explicit_acknowledgement(self):
        alpha.authorize('a'*40, 'a'*40, 'refs/heads/main', 'unsigned-first-alpha')
        for expected, actual, ref, approval in [
            ('a'*40, 'b'*40, 'refs/heads/main', 'unsigned-first-alpha'),
            ('a'*40, 'a'*40, 'refs/heads/feature', 'unsigned-first-alpha'),
            ('a'*40, 'a'*40, 'refs/heads/main', ''),
        ]:
            with self.assertRaises(ValueError):
                alpha.authorize(expected, actual, ref, approval)

    def test_all_required_jobs_must_pass_never_skip(self):
        results = {key: {'result': 'success'} for key in alpha.REQUIRED_GATES}
        alpha.validate_results(results)
        for key in alpha.REQUIRED_GATES:
            for state in ['skipped', 'failure', 'cancelled', '', None]:
                invalid = copy.deepcopy(results)
                invalid[key]['result'] = state
                with self.assertRaises(ValueError):
                    alpha.validate_results(invalid)
            invalid = copy.deepcopy(results)
            del invalid[key]
            with self.assertRaises(ValueError):
                alpha.validate_results(invalid)

    def test_brand_files_are_verified_not_just_approved(self):
        report = alpha.verify_brand(ROOT)
        self.assertGreaterEqual(len(report['assets']), 10)
        self.assertFalse(report['stable_approval'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises((ValueError, OSError)):
                alpha.verify_brand(root)

    def test_brand_source_mismatch_rejected(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(ROOT / 'assets/brand', root / 'assets/brand')
            (root / 'assets/brand/automexia-terminal-source-512.png').write_bytes(b'wrong')
            with self.assertRaises(ValueError):
                alpha.verify_brand(root)

    def test_signing_disclosure_cannot_claim_os_trust(self):
        report = alpha.signing_disclosure()
        self.assertEqual(report['windows'], 'unsigned')
        self.assertEqual(report['macos'], 'ad-hoc; not notarized')
        self.assertEqual(report['checksum_manifest'], 'minisign')

    def test_publisher_rejects_missing_or_forged_signing_disclosure(self):
        manifest = {'version': '0.0.4-alpha', 'tag': 'v0.0.4-alpha',
                    'source_commit': 'a'*40, 'channel': 'first-alpha',
                    'os_signing': alpha.signing_disclosure(),
                    'branding': {'stable_approval': False}}
        alpha.verify_disclosure(manifest)
        for key in ('windows', 'macos', 'checksum_manifest'):
            bad = copy.deepcopy(manifest)
            bad['os_signing'][key] = 'trusted'
            with self.assertRaises(ValueError):
                alpha.verify_disclosure(bad)
        del manifest['branding']
        with self.assertRaises(ValueError):
            alpha.verify_disclosure(manifest)

    def test_ci_has_no_publication_or_administration_credentials(self):
        import yaml
        workflow = yaml.safe_load((ROOT / '.github/workflows/first-alpha.yml').read_text())
        self.assertNotIn('publish-release', workflow['jobs'])
        self.assertEqual(workflow['permissions'], {'contents': 'read'})
        preflight = next(step['run'] for step in workflow['jobs']['preflight']['steps'] if 'run' in step)
        self.assertIn('first_alpha.py authorize', preflight)
        self.assertNotIn('/immutable-releases', preflight)

    def test_release_quality_installs_required_unicode_fonts(self):
        import yaml
        for name in ('first-alpha.yml', 'release.yml'):
            workflow = yaml.safe_load((ROOT / '.github/workflows' / name).read_text())
            steps = workflow['jobs']['release-quality']['steps']
            setup = next(step['run'] for step in steps if step.get('name') == 'Install native Linux build dependencies')
            for package in ('fonts-noto-cjk', 'fonts-noto-color-emoji', 'fonts-noto-core'):
                with self.subTest(workflow=name, package=package):
                    self.assertIn(package, setup)

    def test_all_universal_verifiers_put_the_input_before_the_architecture_list(self):
        import shlex
        for name in ("first-alpha.yml", "release.yml", "nightly.yml"):
            text = (ROOT / ".github/workflows" / name).read_text(encoding="utf-8")
            invocations = [shlex.split(line.strip()) for line in text.splitlines()
                           if line.strip().startswith("lipo ") and "-verify_arch" in line]
            self.assertTrue(invocations, name)
            for argv in invocations:
                with self.subTest(workflow=name, invocation=argv):
                    self.assertEqual(argv[2:], ["-verify_arch", "x86_64", "arm64"])
                    self.assertFalse(argv[1].startswith("-"))

    def test_workflow_preserves_native_final_and_reproducibility_gates(self):
        import yaml
        workflow = yaml.safe_load((ROOT / '.github/workflows/first-alpha.yml').read_text())
        alpha.validate_workflow(workflow)
        for key in alpha.REQUIRED_GATES:
            invalid = copy.deepcopy(workflow)
            invalid['jobs']['final-gate']['needs'].remove(key)
            with self.assertRaises(ValueError):
                alpha.validate_workflow(invalid)
        invalid = copy.deepcopy(workflow)
        invalid['jobs']['build']['permissions'] = {'contents': 'write'}
        with self.assertRaises(ValueError):
            alpha.validate_workflow(invalid)

    def test_packaged_shell_and_settings_checks_cannot_be_removed_or_skipped(self):
        import yaml
        workflow = yaml.safe_load((ROOT / '.github/workflows/first-alpha.yml').read_text())
        for job, command in (
            ('verify-linux-final', 'tools/ci/test_packaged_shells.py'),
            ('verify-linux-final', 'tools/ci/test_linux_distros.py'),
            ('verify-macos-final', 'tools/ci/test_packaged_shells.py'),
            ('verify-windows-final', 'tests/integration/windows-package-review.ps1'),
        ):
            for mutation in ('remove', 'skip', 'ignore_failure'):
                with self.subTest(job=job, mutation=mutation):
                    invalid = copy.deepcopy(workflow)
                    steps = invalid['jobs'][job]['steps']
                    step = next(step for step in steps if command in step.get('run', ''))
                    if mutation == 'remove':
                        steps.remove(step)
                    elif mutation == 'skip':
                        step['if'] = 'false'
                    else:
                        step['continue-on-error'] = True
                    with self.assertRaises(ValueError):
                        alpha.validate_workflow(invalid)


if __name__ == '__main__':
    unittest.main()
