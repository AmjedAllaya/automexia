import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import native_wsl_git_prompt as fixture


class WslGitEvidenceTests(unittest.TestCase):
    def test_console_failure_after_nine_checks_still_fails_the_report(self):
        kube = {"context": "fixture", "namespace": "sandbox"}
        values = iter([( "fixture-first", None), ("fixture-first", kube), (None, kube),
                       ("fixture-worktree", None), ("fixture-second", None),
                       (None, None), (None, None), (None, None), ("abcd1234", None)])
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "amx.exe"
            binary.write_bytes(b"fixture image")
            report = Path(temporary) / "report.json"
            def run(argv, **options):
                if argv[0] == str(binary):
                    branch, context = next(values)
                    return json.dumps({"git_branch": branch, "kubernetes": context}).encode()
                if "wslpath" in argv:
                    return b"/fixture/driver.py\n"
                if "create" in argv:
                    return b"/tmp/automexia-git-fixture-test\n"
                return b"abcd1234\n" if "detach" in argv else b""
            with patch.object(fixture, "run", side_effect=run), \
                 patch.object(fixture, "verify_console_entrypoint", side_effect=ValueError("forwarded")):
                with self.assertRaises(ValueError):
                    fixture.verify(binary, "Fixture", report)
            result = json.loads(report.read_text())
            self.assertEqual(result["checks"], 9)
            self.assertEqual(result["cleanup"], "passed")
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["console_entrypoint"], "failed")

    def test_console_entrypoint_requires_direct_bounded_response(self):
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "amx.exe"
            binary.write_bytes(b"fixture image")
            def run(argv, **options):
                self.assertEqual(Path(argv[0]).name, "amx.exe")
                self.assertFalse(Path(argv[0]).with_name("automexia.exe").exists())
                self.assertEqual(options["timeout"], 5)
                self.assertEqual(json.loads(argv[2])["git"], False)
                return b'{"git_branch":null,"kubernetes":null}'
            with patch.object(fixture, "run", side_effect=run):
                fixture.verify_console_entrypoint(binary)
            with patch.object(fixture, "run", return_value=b'{"git_branch":"unexpected","kubernetes":null}'):
                with self.assertRaises(ValueError):
                    fixture.verify_console_entrypoint(binary)

    def test_first_helper_failure_retains_timing_and_cleans_fixture(self):
        calls = []
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "fixture.exe"
            binary.write_bytes(b"fixture image")
            report = Path(temporary) / "report.json"

            def run(argv, **options):
                calls.append(argv)
                if argv[0] == str(binary):
                    raise fixture.CheckFailure("Git prompt check 1: timeout=True", b"")
                if "wslpath" in argv:
                    return b"/fixture/driver.py\n"
                if "create" in argv:
                    return b"/tmp/automexia-git-fixture-test\n"
                return b""

            with patch.object(fixture, "run", side_effect=run):
                with self.assertRaises(fixture.CheckFailure):
                    fixture.verify(binary, "Fixture", report)
            result = json.loads(report.read_text())
            self.assertEqual(result["status"], "failed")
            self.assertEqual(result["checks"], 0)
            self.assertEqual(result["cleanup"], "passed")
            self.assertEqual(result["deadline_seconds"], 5)
            self.assertEqual(result["binary_bytes"], 13)
            self.assertEqual(len(result["binary_sha256"]), 64)
            self.assertEqual(len(result["helper_launches"]), 1)
            self.assertGreaterEqual(result["helper_launches"][0]["elapsed_ms"], 0)
            self.assertIn("delete", calls[-1])
            self.assertNotIn(str(binary), report.read_text())


if __name__ == "__main__":
    unittest.main()
