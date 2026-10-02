#!/usr/bin/env python3
"""CRLF/ANSI, nonzero-evidence and failure-rejection tests for the SSH checker."""
import unittest
from check_ssh_library import test_count, clean

class EvidenceTests(unittest.TestCase):
    def test_python_lf(self):
        self.assertEqual(test_count('Ran 8 tests in 0.01s\n\nOK\n', 'python'), 8)
    def test_python_crlf(self):
        self.assertEqual(test_count(b'Ran 8 tests in 0.01s\r\n\r\nOK\r\n', 'python'), 8)
    def test_python_colored(self):
        self.assertEqual(test_count('Ran 8 tests in 0.01s\n\x1b[32mOK\x1b[0m\n', 'python'), 8)
    def test_python_singular(self):
        self.assertEqual(test_count('Ran 1 test in 0.01s\nOK\n', 'python'), 1)
    def test_python_zero_rejected(self):
        with self.assertRaises(ValueError): test_count('Ran 0 tests in 0s\nOK\n', 'python')
    def test_python_skip_rejected(self):
        with self.assertRaises(ValueError): test_count('Ran 8 tests in 0s\nOK (skipped=1)\n', 'python')
    def test_python_conflicting_summary_rejected(self):
        with self.assertRaises(ValueError): test_count('Ran 8 tests in 0s\nFAILED (failures=1)\nOK\n', 'python')
    def test_python_duplicate_rejected(self):
        with self.assertRaises(ValueError): test_count('Ran 8 tests in 0s\nOK\nRan 3 tests in 0s\nOK\n', 'python')
    def test_rust_crlf_color(self):
        self.assertEqual(test_count('\x1b[32mtest result: ok.\x1b[0m 35 passed; 0 failed; 0 ignored; 0 measured;\r\n', 'rust'), 35)
    def test_rust_zero_rejected(self):
        with self.assertRaises(ValueError): test_count('test result: ok. 0 passed; 0 failed; 0 ignored;', 'rust')
    def test_rust_failure_rejected(self):
        with self.assertRaises(ValueError): test_count('test result: FAILED. 1 passed; 1 failed; 0 ignored;', 'rust')
    def test_rust_ignore_rejected(self):
        with self.assertRaises(ValueError): test_count('test result: ok. 1 passed; 0 failed; 1 ignored;', 'rust')
    def test_rust_mixed_failure_rejected(self):
        with self.assertRaises(ValueError): test_count('test result: ok. 1 passed; 0 failed; 0 ignored;\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;', 'rust')
    def test_missing_rejected(self):
        for kind in ('rust', 'python'):
            with self.assertRaises(ValueError): test_count('build completed', kind)
    def test_ansi_crlf_normalization(self):
        self.assertEqual(clean(b'\x1b[31mhello\x1b[0m\r\n'), 'hello\n')

class BenchmarkEvidenceTests(unittest.TestCase):
    @staticmethod
    def smoke():
        from check_ssh_library import EXPECTED_BENCHMARKS
        return ''.join(f'Testing ssh_integration/{name}\nSuccess\n' for name in EXPECTED_BENCHMARKS)
    @staticmethod
    def measured():
        from check_ssh_library import EXPECTED_BENCHMARKS
        return ''.join(f'ssh_integration/{name}\n                        time: [1.0 us 1.2 us 1.4 us]\n' for name in EXPECTED_BENCHMARKS)
    def test_all_smoke_workloads_required(self):
        from check_ssh_library import benchmark_count
        self.assertEqual(benchmark_count(self.smoke(),smoke=True),9)
    def test_smoke_crlf(self):
        from check_ssh_library import benchmark_count
        self.assertEqual(benchmark_count(self.smoke().replace('\n','\r\n'),smoke=True),9)
    def test_smoke_ansi(self):
        from check_ssh_library import benchmark_count
        self.assertEqual(benchmark_count(self.smoke().replace('Success','\x1b[32mSuccess\x1b[0m'),smoke=True),9)
    def test_empty_filter_is_not_a_pass(self):
        from check_ssh_library import benchmark_count
        with self.assertRaises(ValueError):benchmark_count('Finished bench profile',smoke=True)
    def test_missing_workload_is_not_a_pass(self):
        from check_ssh_library import benchmark_count
        with self.assertRaises(ValueError):benchmark_count(self.smoke().replace('maximum_remote_path','different'),smoke=True)
    def test_missing_success_is_not_a_pass(self):
        from check_ssh_library import benchmark_count
        with self.assertRaises(ValueError):benchmark_count(self.smoke().replace('Success','Success',1).rsplit('Success',1)[0],smoke=True)
    def test_measurements_are_distinct_from_smoke(self):
        from check_ssh_library import benchmark_count
        self.assertEqual(benchmark_count(self.measured(),smoke=False),9)
        with self.assertRaises(ValueError):benchmark_count(self.smoke(),smoke=False)
    def test_measurement_crlf_and_color(self):
        from check_ssh_library import benchmark_count
        text=self.measured().replace('ssh_integration/','\x1b[32mssh_integration/').replace('\n','\x1b[0m\r\n')
        self.assertEqual(benchmark_count(text,smoke=False),9)
    def test_measurements_cannot_masquerade_as_smoke(self):
        from check_ssh_library import benchmark_count
        with self.assertRaises(ValueError):benchmark_count(self.measured(),smoke=True)
    def test_panicked_benchmark_is_not_a_pass(self):
        from check_ssh_library import benchmark_count
        with self.assertRaises(ValueError):benchmark_count(self.smoke()+"thread main panicked at fixture\n",smoke=True)


class OrchestrationTests(unittest.TestCase):
    def invoke(self, *, output=None, code=0, timed_out=False, cleanup=None, **options):
        import check_ssh_library as runner
        from pathlib import Path
        import tempfile
        from types import SimpleNamespace
        from unittest.mock import patch
        calls=[]
        def run(command, **kwargs):
            calls.append((command,kwargs.get('merge_stderr')))
            data = output
            if data is None:
                if '--example' in command:
                    data=b'AMXSSH-FIXTURE-1\nY29yZQ==\nYm9vdHN0cmFw\n'
                else:
                    data=b'Ran 34 tests in 0.01s\n\nOK\n'
            kwargs['consume'](data)
            return SimpleNamespace(return_code=code,timed_out=timed_out,error=cleanup)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            with patch.object(runner,'load_owner',return_value=SimpleNamespace(run=run)), \
                 patch.object(runner,'check_boundaries'), \
                 patch.object(runner.shutil,'which',side_effect=lambda n:n), \
                 patch.object(runner,'available_bash',return_value='bash'):
                result=runner.verify(root,root/'reports',shell_only=True,require_bash=True,**options)
        return result,calls
    def test_actual_generator_precedes_shell_tests(self):
        result,calls=self.invoke()
        self.assertEqual([s['name'] for s in result['steps']],['export-actual-generator','bash-core-local'])
        self.assertIn('--example',calls[0][0])
        self.assertIn('--generated-fixture',calls[1][0])
    def test_generator_stderr_cannot_pollute_fixture(self):
        _,calls=self.invoke()
        self.assertFalse(calls[0][1]);self.assertTrue(calls[1][1])
    def test_shell_scope_never_invokes_ready(self):
        _,calls=self.invoke()
        self.assertFalse(any('ready' in c for c,_ in calls))
    def test_generator_nonzero_stops_before_shell(self):
        with self.assertRaises(ValueError):self.invoke(code=1)
    def test_generator_timeout_cannot_pass(self):
        with self.assertRaises(ValueError):self.invoke(timed_out=True)
    def test_unretired_cleanup_cannot_pass(self):
        with self.assertRaises(ValueError):self.invoke(cleanup='not retired')
    def test_wrong_fixture_header_rejected(self):
        with self.assertRaises(ValueError):self.invoke(output=b'wrong\nYQ==\nYg==\n')
    def test_invalid_base64_fixture_rejected(self):
        with self.assertRaises(ValueError):self.invoke(output=b'AMXSSH-FIXTURE-1\n!\nYg==\n')
    def test_benchmark_modes_are_mutually_exclusive(self):
        import check_ssh_library as runner
        from pathlib import Path
        with self.assertRaises(ValueError):runner.verify(Path('.'),Path('.'),bench=True,bench_smoke=True)
    def test_shell_scope_rejects_recursive_full_gate(self):
        with self.assertRaises(ValueError):self.invoke(ready=True)

if __name__ == '__main__': unittest.main()