#!/usr/bin/env python3
"""Mutation coverage for the GitHub-Free/private workflow contract."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / ".github" / "scripts" / "check_free_plan_contract.py"
TEST_TEMP_PARENT = Path(ROOT.anchor) if os.name == "nt" else None


class FreePlanContractTests(unittest.TestCase):
    def run_checker(self, mutation: tuple[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            if mutation is not None:
                relative, addition = mutation
                path = root / ".github" / relative
                path.write_text(
                    path.read_text(encoding="utf-8") + addition,
                    encoding="utf-8",
                )
            return subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )

    def test_current_contract_passes(self) -> None:
        completed = self.run_checker()
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_private_environment_is_rejected(self) -> None:
        completed = self.run_checker(
            ("workflows/s2-assurance.yml", "\n  environment: stable-release\n")
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("private GitHub environments", completed.stderr)

    def test_paid_or_unbounded_workflows_are_rejected(self) -> None:
        cases = (
            (
                "workflows/ci.yml",
                "\n      - uses: actions/attest@0123456789012345678901234567890123456789\n",
                "private GitHub artifact attestations",
            ),
            (
                "workflows/nightly.yml",
                "\nschedule:\n  - cron: '0 0 * * *'\n",
                "manual-only",
            ),
        )
        for relative, addition, expected in cases:
            with self.subTest(expected=expected):
                completed = self.run_checker((relative, addition))
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn(expected, completed.stderr)

    def test_only_the_release_coverage_job_may_use_a_hosted_windows_runner(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            source = workflow.read_text(encoding="utf-8")
            source = source.replace(
                "  quality:\n", "  unexpected-windows:\n    runs-on: windows-2025\n  quality:\n", 1
            )
            workflow.write_text(source, encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("only release-candidate-coverage", completed.stderr)

    def test_required_local_assurance_scanners_cannot_be_removed(self) -> None:
        completed = self.run_checker(
            (
                "workflows/ci.yml",
                "\n# semgrep==1.175.0\n",
            )
        )
        # Appending a fragment must not make a complete workflow less strict.
        self.assertEqual(completed.returncode, 0, completed.stderr)
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            workflow.write_text(
                workflow.read_text(encoding="utf-8").replace(
                    "GITLEAKS_VERSION: '8.30.1'", "GITLEAKS_VERSION: '0.0.0'", 1
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("GitHub-Free local assurance", completed.stderr)

    def test_ordinary_ci_cannot_be_changed_back_to_all_history_scanning(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            workflow.write_text(
                workflow.read_text(encoding="utf-8").replace(
                    '--log-opts="$log_opts"', "--log-opts=--all", 1
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("legacy history", completed.stderr)

    def test_secret_scan_requires_full_history_but_quality_stays_shallow(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            source = workflow.read_text(encoding="utf-8")
            dependency = source.index("  dependency-security:\n")
            fetch = source.index("          fetch-depth: 0\n", dependency)
            workflow.write_text(
                source[:fetch] + source[fetch + len("          fetch-depth: 0\n") :],
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("fetch complete history", completed.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
