"""The native smoke must fail closed on process and capture failures."""
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import check_google_command_native as native


class GoogleCommandHarnessTests(unittest.TestCase):
    def test_exact_arguments_environment_deadline_and_exit_are_preserved(self):
        command = ["fixture-executable", "google", "--print-url", "a & b"]
        environment = {"USER": "alice"}
        def process(arguments, **options):
            self.assertEqual(arguments, command)
            self.assertEqual(options["environment"], environment)
            self.assertEqual(options["timeout_seconds"], 25)
            options["consume"](b"first")
            options["consume"](b"second")
            return SimpleNamespace(return_code=7, error=None, timed_out=False)
        with patch.object(native.qa_process, "run", process):
            self.assertEqual(native.run(command, environment), (7, b"firstsecond"))

    def test_process_failure_timeout_and_output_limit_are_not_success(self):
        for error, timed_out in (("private diagnostic canary", False), (None, True)):
            with self.subTest(timed_out=timed_out), patch.object(native.qa_process, "run", return_value=SimpleNamespace(error=error, timed_out=timed_out)):
                with self.assertRaises(AssertionError) as failure:
                    native.run(["fixture-executable"], {})
                self.assertNotIn("private diagnostic canary", str(failure.exception))
        def overflow(arguments, **options):
            options["consume"](b"a" * 65536)
            options["consume"](b"b")
            self.fail("over-limit output was accepted")
        with patch.object(native.qa_process, "run", overflow):
            with self.assertRaisesRegex(RuntimeError, "exceeded its bound"):
                native.run(["fixture-executable"], {})


if __name__ == "__main__":
    unittest.main()
