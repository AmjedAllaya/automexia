#!/usr/bin/env python3
"""Native regression for non-interactive Zsh completion initialization."""

from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
ZSH_CONTRACT = ROOT / "tools/ci/test_zsh_integration.zsh"


@unittest.skipUnless(
    os.name != "nt" and shutil.which("zsh"),
    "native Zsh is required",
)
class ZshNoninteractiveCompinitTests(unittest.TestCase):
    def test_insecure_ambient_fpath_is_ignored_without_a_terminal(self) -> None:
        discovered = subprocess.run(
            ["zsh", "-f", "-c", "print -r -- ${(j.:.)fpath}"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=10,
            check=True,
        ).stdout.strip()
        self.assertTrue(discovered)

        with tempfile.TemporaryDirectory(prefix="automexia-zsh-compinit-") as temporary:
            insecure = Path(temporary) / "world-writable-functions"
            insecure.mkdir()
            (insecure / "_automexia_insecure_canary").write_text(
                "#compdef automexia-test-insecure-canary\n",
                encoding="utf-8",
            )
            insecure.chmod(0o777)

            environment = os.environ.copy()
            environment["FPATH"] = f"{insecure}:{discovered}"
            environment["AUTOMEXIA_TEST_ROOT"] = str(ROOT)
            source = ZSH_CONTRACT.read_text(encoding="utf-8")

            completed = subprocess.run(
                ["zsh", "-f", str(ZSH_CONTRACT)],
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
                timeout=30,
                start_new_session=True,
                check=False,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertIn("PASS: Zsh integration", completed.stdout)

            # This mutation is the exact hosted failure: without safe-ignore
            # mode, compinit tries to ask about the hostile directory through a
            # terminal that a non-interactive CI process does not own.
            self.assertIn("compinit -i -D", source)
            weakened = source.replace("compinit -i -D", "compinit -D", 1)
            rejected = subprocess.run(
                ["zsh", "-f", "-s"],
                cwd=ROOT,
                env=environment,
                input=weakened,
                capture_output=True,
                text=True,
                timeout=10,
                start_new_session=True,
                check=False,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("compinit", rejected.stderr)
            self.assertIn("terminal", rejected.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
