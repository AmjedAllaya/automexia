#!/usr/bin/env python3
import unittest
from check_inline_pipeline import test_count, groups
class InlinePipelineRunnerTests(unittest.TestCase):
    def test_nonzero_result(self):
        self.assertEqual(test_count("test result: ok. 5 passed; 0 failed; 0 ignored;"), 5)
    def test_zero_is_not_success(self):
        with self.assertRaises(ValueError): test_count("test result: ok. 0 passed; 0 failed; 3 ignored;")
    def test_missing_is_not_success(self):
        with self.assertRaises(ValueError): test_count("build succeeded")
    def test_failure_is_not_success(self):
        with self.assertRaises(ValueError): test_count("test result: ok. 1 passed; 1 failed; 0 ignored;")
    def test_many_test_binaries_count_only_executed_tests(self):
        self.assertEqual(test_count("test result: ok. 0 passed; 0 failed; 1 ignored;\ntest result: ok. 2 passed; 0 failed; 0 ignored;"), 2)
    def test_baseline_does_not_select_new_test_files(self):
        self.assertEqual(len(groups(True, False)), 3)
        self.assertTrue(all("inline_pipeline_native" not in args for _, args, _ in groups(True, False)))
    def test_native_is_selected_by_default(self):
        self.assertTrue(any(n == "native-host" for n, _, _ in groups(False, True)))
    def test_native_skip_is_explicit(self):
        self.assertFalse(any(n == "native-host" for n, _, _ in groups(False, False)))

import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
from unittest import mock
import check_inline_pipeline as runner
from check_inline_pipeline import python_test_count


class PythonResultTests(unittest.TestCase):
    def test_lf(self):
        self.assertEqual(python_test_count("........\nRan 8 tests in 0.000s\n\nOK\n"), 8)

    def test_windows_crlf_regression(self):
        self.assertEqual(python_test_count("........\r\n--------\r\nRan 8 tests in 0.000s\r\n\r\nOK\r\n"), 8)

    def test_mixed_lf_and_crlf(self):
        self.assertEqual(python_test_count("Ran 8 tests in 0.000s\r\n\nOK\r\n"), 8)

    def test_colored_status_lf(self):
        self.assertEqual(python_test_count("Ran 8 tests in 0.000s\n\n\x1b[32mOK\x1b[0m\n"), 8)

    def test_colored_status_crlf(self):
        self.assertEqual(python_test_count("Ran 8 tests in 0.000s\r\n\r\n\x1b[1;32mOK\x1b[0m\r\n"), 8)

    def test_colored_summary(self):
        self.assertEqual(python_test_count("Ran \x1b[32m8\x1b[m tests in 0.000s\nOK\n"), 8)

    def test_no_final_newline(self):
        self.assertEqual(python_test_count("Ran 8 tests in 0.000s\n\nOK"), 8)

    def test_singular_test(self):
        self.assertEqual(python_test_count("Ran 1 test in 0.001s\n\nOK\n"), 1)

    def test_duration_space(self):
        self.assertEqual(python_test_count("Ran 2 tests in 0.001 s\n\nOK\n"), 2)

    def test_zero_is_failure(self):
        with self.assertRaises(ValueError):
            python_test_count("Ran 0 tests in 0.000s\n\nOK\n")

    def test_bare_ok_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("OK\n")

    def test_missing_status_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\n")

    def test_truncated_status_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\nO")

    def test_skip_is_not_full_success(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\nOK (skipped=1)\n")

    def test_expected_failure_is_not_full_success(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\nOK (expected failures=1)\n")

    def test_failed_run_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\nFAILED (failures=1)\n")

    def test_colored_failed_run_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\r\n\x1b[31mFAILED (errors=1)\x1b[0m\r\n")

    def test_duplicate_summary_is_failure(self):
        one = "Ran 8 tests in 0.000s\nOK\n"
        with self.assertRaises(ValueError): python_test_count(one + one)

    def test_earlier_failure_cannot_be_hidden(self):
        with self.assertRaises(ValueError): python_test_count("FAILED (errors=1)\nRan 8 tests in 0.000s\nOK\n")

    def test_earlier_status_cannot_be_hidden(self):
        with self.assertRaises(ValueError): python_test_count("OK\nRan 8 tests in 0.000s\nOK\n")

    def test_later_error_cannot_be_hidden(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\nOK\nERROR: cleanup\n")

    def test_bare_carriage_return_is_not_erased(self):
        with self.assertRaises(ValueError): python_test_count("FAILED\rRan 8 tests in 0.000s\nOK\n")

    def test_cursor_controls_are_not_erased(self):
        with self.assertRaises(ValueError): python_test_count("Ran 8 tests in 0.000s\n\x1b[2KOK\n")

    def test_malformed_count_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("Ran -8 tests in 0.000s\nOK\n")

    def test_non_summary_substring_is_failure(self):
        with self.assertRaises(ValueError): python_test_count("log: Ran 8 tests in 0.000s\nOK\n")


class VerificationResultTests(unittest.TestCase):
    def exercise(self, raw, *, code=0, timed_out=False, error=None, next_step=False):
        calls = []
        def run(argv, **kwargs):
            calls.append((argv, kwargs))
            kwargs['consume'](raw if len(calls) == 1 else b'test result: ok. 5 passed; 0 failed; 0 ignored;\n')
            return SimpleNamespace(return_code=code, timed_out=timed_out, error=error)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report_dir = root / 'reports'
            selected = [('model-fixture', ['test'], True)] if next_step else []
            with mock.patch.object(runner.shutil, 'which', return_value='cargo'), \
                 mock.patch.object(runner, 'load_process_owner', return_value=SimpleNamespace(run=run)), \
                 mock.patch.object(runner, 'groups', return_value=selected), \
                 contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                result = runner.verify(root, report_dir, native=False)
            report = json.loads((report_dir / 'report.json').read_text(encoding='utf-8'))
            saved_log = (report_dir / 'python-runner.log').read_bytes()
            return result, report, saved_log, calls

    def test_real_verify_path_accepts_windows_summary(self):
        raw = b'........\r\n--------\r\nRan 8 tests in 0.000s\r\n\r\nOK\r\n'
        ok, report, saved, calls = self.exercise(raw, next_step=True)
        self.assertTrue(ok)
        self.assertTrue(report['complete'])
        self.assertEqual([entry['passed'] for entry in report['checks']], [8, 5])
        self.assertEqual(saved, raw, 'raw log must not be normalized or recolored')
        self.assertEqual(len(calls), 2)

    def test_real_verify_path_accepts_color(self):
        ok, report, _, _ = self.exercise(b'Ran 8 tests in 0.000s\r\n\x1b[32mOK\x1b[0m\r\n')
        self.assertTrue(ok)
        self.assertEqual(report['checks'][0]['passed'], 8)

    def test_nonzero_exit_cannot_be_masked_by_ok(self):
        ok, report, _, calls = self.exercise(b'Ran 8 tests in 0.000s\nOK\n', code=1, next_step=True)
        self.assertFalse(ok)
        self.assertFalse(report['complete'])
        self.assertEqual(len(calls), 1)

    def test_timeout_cannot_be_masked_by_ok(self):
        ok, report, _, _ = self.exercise(b'Ran 8 tests in 0.000s\nOK\n', timed_out=True)
        self.assertFalse(ok)
        self.assertEqual(report['checks'][0]['reason'], 'timeout')

    def test_cleanup_error_cannot_be_masked_by_ok(self):
        ok, report, _, _ = self.exercise(b'Ran 8 tests in 0.000s\nOK\n', error='cleanup remains unresolved')
        self.assertFalse(ok)
        self.assertEqual(report['checks'][0]['reason'], 'cleanup remains unresolved')

    def test_zero_match_stops_later_steps(self):
        ok, report, _, calls = self.exercise(b'Ran 0 tests in 0.000s\r\nOK\r\n', next_step=True)
        self.assertFalse(ok)
        self.assertFalse(report['complete'])
        self.assertEqual(len(calls), 1)

    def test_child_environment_does_not_change_parent(self):
        with mock.patch.dict(os.environ, {'PYTHON_COLORS': '1', 'FORCE_COLOR': '1'}):
            before = dict(os.environ)
            ok, _, _, calls = self.exercise(b'Ran 8 tests in 0.000s\nOK\n')
            self.assertTrue(ok)
            child = calls[0][1]['environment']
            self.assertEqual(child['PYTHON_COLORS'], '0')
            self.assertEqual(child['NO_COLOR'], '1')
            self.assertEqual(child['PYTHONIOENCODING'], 'utf-8')
            self.assertEqual(child['CARGO_TERM_COLOR'], 'never')
            self.assertEqual(dict(os.environ), before)


if __name__ == '__main__':
    unittest.main()
