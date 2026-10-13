"""Failure/coverage controls for the native startup runtime comparison."""
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

import check_windows_shell_startup as startup


class StartupRuntimeTests(unittest.TestCase):
    def exercise(self, results, compare=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            executable = root / "fixture.exe"
            executable.write_bytes(b"fixture")
            stage = mock.Mock(executable=executable, version="fixture", cleanup_safe=True)
            outputs = [(SimpleNamespace(return_code=0, error=None, timed_out=False), b"")]
            for code, count, error in results:
                output = f"test result: ok. {count} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n".encode()
                outputs.append((SimpleNamespace(return_code=code, error=error, timed_out=False), output))
            with mock.patch.object(startup, "os", SimpleNamespace(name="nt")), \
                 mock.patch.object(startup, "run", side_effect=outputs) as run, \
                 mock.patch.object(startup, "compiled_executable", return_value=str(executable)), \
                 mock.patch.object(startup, "NativeTestStage", return_value=stage):
                result = startup.verify(root / "report", compare)
            summary = json.loads((root / "report/summary.json").read_text())
            stage.close.assert_called_once()
            return result, summary, run.call_count, stage.cleanup_safe

    def test_bundled_pass_requires_all_three_native_cases(self):
        result, summary, calls, cleanup = self.exercise([(0, 3, None)])
        self.assertTrue(result)
        self.assertEqual([item["runtime"] for item in summary], ["bundled"])
        self.assertEqual(calls, 2)
        self.assertTrue(cleanup)

    def test_inbox_failure_still_collects_bundled_control_and_fails(self):
        result, summary, calls, _ = self.exercise([(1, 1, None), (0, 3, None)], compare=True)
        self.assertFalse(result)
        self.assertEqual([item["passed"] for item in summary], [False, True])
        self.assertEqual(calls, 3)

    def test_missing_native_case_is_not_a_pass(self):
        result, _, _, _ = self.exercise([(0, 2, None)])
        self.assertFalse(result)

    def test_unresolved_owner_retains_staged_runtime(self):
        result, _, _, cleanup = self.exercise([(1, 0, "unresolved fixture owner")])
        self.assertFalse(result)
        self.assertFalse(cleanup)


if __name__ == "__main__":
    unittest.main()
