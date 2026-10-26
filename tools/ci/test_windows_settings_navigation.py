"""Exercise the actual native focus driver against delayed accessibility frames."""

import os
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "nt", "requires Windows PowerShell")
class SettingsNavigationTests(unittest.TestCase):
    def run_model(self, position, *options):
        powershell = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
        result = subprocess.run(
            [str(powershell), "-NoProfile", "-ExecutionPolicy", "RemoteSigned", "-File",
             str(ROOT / "tests/integration/settings-navigation-model.ps1"),
             "-RepositoryRoot", str(ROOT), "-Position", position, *options],
            capture_output=True, text=True, timeout=30,
        )
        self.assertEqual(result.returncode, 0, "native focus model rejected navigation")
        return result.stdout

    def test_navigation_never_outruns_an_acknowledged_accessibility_frame(self):
        for position in ("first", "second"):
            for options in ((), ("-ImmediateUpdates",)):
                with self.subTest(position=position, options=options):
                    self.assertIn("PASS: one acknowledged move at a time",
                                  self.run_model(position, *options))

    def test_missing_focus_cannot_trigger_more_navigation_or_activation(self):
        for position in ("first", "second"):
            with self.subTest(position=position):
                self.assertIn("PASS: missing focus prevented activation",
                              self.run_model(position, "-SuppressAcknowledgement"))


if __name__ == "__main__":
    unittest.main()
