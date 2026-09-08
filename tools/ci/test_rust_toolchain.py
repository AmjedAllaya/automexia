#!/usr/bin/env python3
"""Compiler-policy mutations and real bounded identity-probe regressions."""

from __future__ import annotations

import copy
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
import tomllib
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_toolchain as contract


class RustToolchainTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        (contract.ROOT / "target").mkdir(exist_ok=True)

    def test_current_policy(self):
        self.assertGreater(contract.validate_policy(), 0)

    def test_compiler_jobs_bootstrap_supported_python_before_verification(self):
        for name in contract.RUST_WORKFLOWS:
            workflow = contract.load_workflow(contract.ROOT / '.github/workflows' / name)
            for index, job in enumerate(workflow['jobs'].values()):
                steps = job.get('steps', [])
                for position, step in enumerate(steps):
                    if 'python tools/ci/rust_toolchain.py verify' not in step.get('run', ''):
                        continue
                    with self.subTest(workflow=name, job=index):
                        # Ubuntu 22.04 ships Python 3.10, without stdlib tomllib.
                        self.assertTrue(any(
                            previous.get('uses') == 'actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1'
                            and previous.get('with', {}).get('python-version') == '3.12'
                            for previous in steps[:position]
                        ))

    def test_current_documentation_matches_repository_identity(self):
        pin = tomllib.loads((contract.ROOT / 'rust-toolchain.toml').read_text(encoding='utf-8'))['toolchain']['channel']
        expected = f'automexia-rust-{pin}-v2'
        for name in ('docs/CI-ASSURANCE.md', 'docs/CI-BUILD-PERFORMANCE-PLAN.md', '.github/FREE-PRIVATE-PRODUCTION-SETUP.md'):
            with self.subTest(owner=name):
                source = (contract.ROOT / name).read_text(encoding='utf-8')
                self.assertEqual(set(re.findall(r'automexia-rust-[0-9.]+-v[0-9]+', source)), {expected})
        for name in ('.github/TOOLCHAIN-POLICY.md', '.github/PATCH-REPORT.md'):
            with self.subTest(owner=name):
                source = (contract.ROOT / name).read_text(encoding='utf-8')
                self.assertIn('rust-toolchain.toml', source)
                self.assertTrue(all(value == pin for value in re.findall(r'\bRust\s+\*{0,2}([0-9]+\.[0-9]+\.[0-9]+)\b', source)))

    def test_later_compiler_executable_override_is_rejected(self):
        cases = (
            lambda w: w['env'].update(RUSTC='unreviewed-compiler'),
            lambda w: w['jobs']['quality']['env'].update(CARGO_BUILD_RUSTC='unreviewed-compiler'),
            lambda w: w['jobs']['quality']['steps'].append({'env': {'RUSTDOC': 'unreviewed-compiler'}, 'run': 'cargo doc --locked'}),
            lambda w: w['jobs']['quality']['steps'].append({'run': 'export RUSTC=unreviewed-compiler\ncargo check --locked'}),
        )
        for index, mutate in enumerate(cases):
            with self.subTest(override=index):
                self.assert_mutation_rejected(mutate, 'compiler executable')

    def test_python_bootstrap_cannot_be_removed_reordered_or_bypassed(self):
        def setup(w):
            return next(step for step in w['jobs']['quality']['steps']
                        if str(step.get('uses', '')).startswith('actions/setup-python@'))
        def reorder(w):
            steps = w['jobs']['quality']['steps']
            previous = setup(w)
            steps.remove(previous)
            steps.append(previous)
        mutations = (
            lambda w: setup(w).update(uses='removed'),
            reorder,
            lambda w: setup(w).update({'if': 'false'}),
            lambda w: setup(w).update({'continue-on-error': True}),
            lambda w: setup(w).update({'with': {'python-version': '3.10'}}),
            lambda w: setup(w).update(uses='actions/setup-python@' + '1' * 40),
        )
        for index, mutation in enumerate(mutations):
            with self.subTest(mutation=index):
                self.assert_mutation_rejected(mutation, 'Python bootstrap')

    def test_documentation_drift_cannot_bypass_repository_validation(self):
        original = contract.read_text
        pin, _ = contract.versions()
        owners = ('docs/CI-ASSURANCE.md', 'docs/CI-BUILD-PERFORMANCE-PLAN.md',
                  '.github/FREE-PRIVATE-PRODUCTION-SETUP.md', '.github/TOOLCHAIN-POLICY.md',
                  '.github/PATCH-REPORT.md')
        for name in owners:
            def mutated(path):
                source = original(path)
                if path == contract.ROOT / name:
                    return source.replace(pin, '1.98.0').replace('rust-toolchain.toml', 'removed-policy-reference')
                return source
            with self.subTest(owner=name), patch.object(contract, 'read_text', side_effect=mutated):
                with self.assertRaisesRegex(contract.ToolchainError, 'documentation'):
                    contract.validate_policy()

    def assert_mutation_rejected(self, mutate, message):
        original = contract.load_workflow
        def mutated(path):
            value = copy.deepcopy(original(path))
            if path.name == "ci.yml":
                mutate(value)
            return value
        with patch.object(contract, "load_workflow", side_effect=mutated):
            with self.assertRaisesRegex(contract.ToolchainError, message):
                contract.validate_policy()

    def test_effective_selection_and_cache_drift(self):
        cases = (
            (lambda w: w["env"].update(RUSTUP_TOOLCHAIN="1.98.0"), "repository compiler pin"),
            (lambda w: w["env"].pop("RUSTUP_TOOLCHAIN"), "repository compiler pin"),
            (lambda w: w["jobs"]["quality"]["env"].update(RUSTUP_TOOLCHAIN="stable"), "job compiler override"),
            (lambda w: w["jobs"]["quality"]["env"].update(SCCACHE_GHA_VERSION="old-cache"), "cache identity"),
        )
        for mutate, reason in cases:
            with self.subTest(reason=reason):
                self.assert_mutation_rejected(mutate, reason)

    @staticmethod
    def verification_step(workflow):
        return next(step for step in workflow["jobs"]["quality"]["steps"]
                    if "verify --kind development" in step.get("run", ""))

    def test_cache_initializer_retains_exact_bytes_and_order(self):
        initializer = r'''printf 'SCCACHE_GHA_VERSION=automexia-rust-%s-v2\n' "$RUSTUP_TOOLCHAIN" >> "$GITHUB_ENV"'''
        for replacement in ("", "# " + initializer, initializer.replace(">>", ">"),
                            initializer + "\n" + initializer,
                            initializer.replace('"$RUSTUP_TOOLCHAIN"', '"stable"')):
            def mutate(workflow):
                step = self.verification_step(workflow)
                self.assertIn(initializer, step["run"])
                step["run"] = step["run"].replace(initializer, replacement)
            with self.subTest(replacement=replacement):
                self.assert_mutation_rejected(mutate, "compiler cache initializer")

    def test_missing_skipped_ignored_and_fabricated_verification(self):
        def mutate_command(value):
            return lambda w: self.verification_step(w).update(run=value)
        verify = "python tools/ci/rust_toolchain.py verify --kind development"
        cases = (
            (mutate_command("echo verified"), "precedes compiler verification"),
            (mutate_command("# " + verify), "precedes compiler verification"),
            (mutate_command(verify + " || true"), "mandatory command"),
            (mutate_command("echo '" + verify + "'"), "mandatory command"),
            (lambda w: self.verification_step(w).update({"if": "false"}), "skipped or ignored"),
            (lambda w: self.verification_step(w).update({"continue-on-error": True}), "skipped or ignored"),
            (lambda w: self.verification_step(w).update(env={"RUSTUP_TOOLCHAIN": "stable"}), "shadows"),
            (mutate_command("rustup default 1.96.1"), "global rustup default"),
        )
        for mutate, reason in cases:
            with self.subTest(reason=reason):
                self.assert_mutation_rejected(mutate, reason)

    def test_verification_must_precede_rust_in_same_step(self):
        def mutate(w):
            step = self.verification_step(w)
            step["run"] = "cargo check --locked\n" + step["run"]
        self.assert_mutation_rejected(mutate, "precedes compiler verification")

    def test_later_shell_override_cannot_replace_verified_compiler(self):
        def mutate(w):
            w["jobs"]["quality"]["steps"].append({"run": "export RUSTUP_TOOLCHAIN=stable\ncargo check"})
        self.assert_mutation_rejected(mutate, "mutate compiler selection")

    def test_malformed_workflow_is_rejected_without_echoing_values(self):
        self.assert_mutation_rejected(lambda w: w["jobs"].update(quality=None), "structure is invalid")

    def test_untrusted_job_identity_is_not_copied_into_diagnostics(self):
        original = contract.load_workflow
        canary = 'private-job-identity-canary'
        def mutated(path):
            value = copy.deepcopy(original(path))
            if path.name == 'ci.yml':
                job = value['jobs'].pop('quality')
                job['env']['RUSTUP_TOOLCHAIN'] = 'unreviewed-compiler'
                value['jobs'][canary] = job
            return value
        with patch.object(contract, 'load_workflow', side_effect=mutated):
            with self.assertRaises(contract.ToolchainError) as raised:
                contract.validate_policy()
        self.assertNotIn(canary, str(raised.exception))

    def test_failure_propagation_cannot_be_disabled_after_probe(self):
        def mutate(w):
            step = self.verification_step(w)
            step["run"] = step["run"].replace("set -euo pipefail", "set +e")
        self.assert_mutation_rejected(mutate, "failure propagation")

    def test_msrv_is_independent_and_mandatory(self):
        def msrv(w):
            return next(step for step in w["jobs"]["quality"]["steps"] if step.get("id") == "msrv")
        for mutate, message in (
            (lambda w: msrv(w).update(id="removed"), "explicit MSRV"),
            (lambda w: msrv(w).update(env={"RUSTUP_TOOLCHAIN": "1.98.0"}), "shadows"),
            (lambda w: msrv(w).update(run="echo passed"), "MSRV must verify"),
            (lambda w: msrv(w).update(run=msrv(w)["run"].replace("--all-features", "")), "MSRV must verify"),
        ):
            with self.subTest(message=message):
                self.assert_mutation_rejected(mutate, message)

    def test_duplicate_keys_and_oversized_policy_fail_redacted(self):
        with tempfile.TemporaryDirectory(dir=contract.ROOT / "target") as directory:
            path = Path(directory) / "workflow.yml"
            path.write_text("jobs: {}\nenv: {}\nenv: {}\n", encoding="utf-8")
            with self.assertRaisesRegex(contract.ToolchainError, "duplicate"):
                contract.load_workflow(path)
            path.write_text("jobs: &cycle\n  nested: *cycle\n", encoding="utf-8")
            with self.assertRaisesRegex(contract.ToolchainError, "aliases"):
                contract.load_workflow(path)
            path.write_text("jobs: " + "[" * 50 + "0" + "]" * 50, encoding="utf-8")
            with self.assertRaisesRegex(contract.ToolchainError, "structural bounds"):
                contract.load_workflow(path)
            path.write_bytes(b"x" * (contract.MAX_FILE_BYTES + 1))
            with self.assertRaisesRegex(contract.ToolchainError, "byte limit"):
                contract.load_workflow(path)

    def test_exact_version_fields_not_self_reported_success(self):
        pin, _ = contract.versions()
        good = f"release: {pin}\ncommit-hash: {'a' * 40}\n"
        for response in ("pass", good + good, good.replace(pin, "1.98.0"), good.replace("a" * 40, "unknown")):
            with self.subTest(response_kind=len(response)), patch.dict(os.environ, {"RUSTUP_TOOLCHAIN": pin}), patch.object(
                contract, "probe", side_effect=[f"{pin}-x86_64-pc-windows-msvc (override)", response]
            ):
                with self.assertRaises(contract.ToolchainError):
                    contract.verify()

    def test_native_probe_success_error_overflow_and_deadline_cleanup(self):
        before = {thread.ident for thread in threading.enumerate()}
        self.assertEqual(contract.probe([sys.executable, "-c", "import sys; sys.stdout.buffer.write(b'fixture\\n')"]), "fixture\n")
        # Actual children exercise the capture/termination owner, not mocked exits.
        for program, timeout, maximum in (
            ("raise SystemExit(3)", 5, 4096),
            ("import sys; sys.stdout.write('x' * 8192); sys.stdout.flush()", 5, 4096),
            ("import threading; threading.Event().wait()", 0.1, 4096),
        ):
            with self.subTest(timeout=timeout, program_kind=len(program)):
                with self.assertRaises(contract.ToolchainError):
                    contract.probe([sys.executable, "-c", program], timeout=timeout, maximum=maximum)
        self.assertEqual({thread.ident for thread in threading.enumerate()}, before)

    def test_cli_does_not_echo_untrusted_environment(self):
        environment = dict(os.environ, RUSTUP_TOOLCHAIN="private-environment-canary")
        result = subprocess.run([sys.executable, str(Path(contract.__file__)), "verify"],
                                env=environment, text=True, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("private-environment-canary", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
