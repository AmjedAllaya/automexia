#!/usr/bin/env python3
"""Mutation coverage for the GitHub-Free/private workflow contract."""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / ".github" / "scripts" / "check_free_plan_contract.py"


class FreePlanContractTests(unittest.TestCase):
    def run_checker(self, mutation: tuple[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as temporary:
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
                "workflows/ci.yml",
                "\n    runs-on: windows-2025\n",
                "ordinary CI must not consume Windows/macOS",
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
