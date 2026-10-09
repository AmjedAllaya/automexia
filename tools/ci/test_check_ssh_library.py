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
    def invoke(self, *, output=None, code=0, timed_out=False, cleanup=None,
               bash='bash', windows_shells=None, adapter_output=None, upload_output=None,
               producer_output=None, posix_upload_output=None, **options):
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
                if command[1] == 'build':
                    import json
                    kind = 'example' if '--example' in command else 'bin'
                    name = command[command.index('--' + kind) + 1]
                    data = json.dumps({'reason': 'compiler-artifact', 'target': {'name': name, 'kind': [kind]},
                                       'executable': str(root / (name + '.exe'))}).encode() + b'\n'
                elif '--files' in command:
                    names = {'bash': ['rc.bash'], 'zsh': ['.zshenv', '.zshrc']}.get(command[-1], ['rc.ps1'])
                    data=b'AMXSSH-HELPER-FIXTURE-1\n' + b''.join(b'file:' + name.encode() + b'=Y29yZQ==\n' for name in names)
                elif '--example' in command:
                    data=b'AMXSSH-FIXTURE-1\nY29yZQ==\nYm9vdHN0cmFw\n'
                elif command[1] == 'test':
                    data=b'test result: ok. 100 passed; 0 failed; 0 ignored;\n'
                elif any(str(value).endswith('test_ssh_helper_native.py') for value in command):
                    data=b'test result: ok. 1 passed; 0 failed; 0 ignored;\n'
                elif any(str(value).endswith('test_ssh_powershell_adapter.py') for value in command) and adapter_output is not None:
                    data=adapter_output
                elif any(str(value).endswith('test_ssh_helper_powershell_transfer.py') for value in command) and upload_output is not None:
                    data=upload_output
                elif any(str(value).endswith('test_ssh_helper_transfer.py') for value in command) and posix_upload_output is not None:
                    data=posix_upload_output
                elif any(str(value).endswith(('test_ssh_helper_powershell.py', 'test_ssh_helper_adapters.py')) for value in command) and producer_output is not None:
                    data=producer_output
                else:
                    data=b'Ran 100 tests in 0.01s\n\nOK\n'
            kwargs['consume'](data)
            return SimpleNamespace(return_code=code,timed_out=timed_out,error=cleanup)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            with patch.object(runner,'load_owner',return_value=SimpleNamespace(run=run)), \
                 patch.object(runner,'check_boundaries'), \
                 patch.object(runner.shutil,'which',side_effect=lambda n:n), \
                 patch.object(runner,'available_bash',return_value=bash), \
                 patch.object(runner,'native_windows_host',return_value=bool(windows_shells)), \
                 patch.object(runner,'available_windows_powershells',return_value=windows_shells or {}):
                options.setdefault('shell_only',True)
                options.setdefault('require_bash',bash is not None)
                result=runner.verify(root,root/'reports',**options)
        return result,calls
    def test_actual_generator_precedes_shell_tests(self):
        result,calls=self.invoke()
        self.assertEqual([s['name'] for s in result['steps']],['export-actual-generator','bash-core-local'])
        self.assertIn('--example',calls[0][0])
        self.assertIn('--generated-fixture',calls[1][0])
    def test_generator_stderr_cannot_pollute_fixture(self):
        _,calls=self.invoke()
        self.assertFalse(calls[0][1]);self.assertTrue(calls[1][1])
    def test_all_posix_shells_export_actual_sources_before_their_tests(self):
        result,calls=self.invoke(require_posix_shells=True)
        self.assertEqual([step['name'] for step in result['steps']], [
            'export-actual-generator', 'bash-core-local', 'export-actual-generator-zsh',
            'export-actual-generator-fish', 'other-shells-local',
            'export-helper-files-bash', 'export-helper-files-zsh', 'build-upload-helper', 'posix-helper-producers-local',
            'build-upload-exporter', 'posix-helper-upload-local'])
        self.assertEqual(calls[2][0][-2:], ['--', 'zsh'])
        self.assertEqual(calls[3][0][-2:], ['--', 'fish'])
        self.assertFalse(calls[2][1]);self.assertFalse(calls[3][1])
        self.assertIn('--fixtures', calls[4][0])
        self.assertEqual(calls[5][0][-2:], ['--files', 'bash'])
        self.assertEqual(calls[6][0][-2:], ['--files', 'zsh'])
        self.assertFalse(calls[5][1]);self.assertFalse(calls[6][1])
        self.assertTrue(any(str(value).endswith('test_ssh_helper_adapters.py') for value in calls[8][0]))
        self.assertIn('--message-format=json', calls[7][0])
        self.assertIn('--message-format=json', calls[9][0])
        self.assertTrue(any(str(value).endswith('test_ssh_helper_transfer.py') for value in calls[10][0]))
        self.assertFalse(any(value.startswith('@upload-') for value in calls[10][0]))
    def test_posix_producers_receive_the_built_native_helper(self):
        _, calls = self.invoke(require_posix_shells=True)
        build_index = next(i for i, (cmd, _) in enumerate(calls)
                           if 'build' in cmd and 'automexia-ssh-helper' in cmd)
        test_index = next(i for i, (cmd, _) in enumerate(calls)
                          if any(str(value).endswith('test_ssh_helper_adapters.py') for value in cmd))
        command = calls[test_index][0]
        self.assertLess(build_index, test_index)
        self.assertIn('--message-format=json', calls[build_index][0])
        self.assertIn('--helper', command)
        self.assertTrue(command[command.index('--helper') + 1].endswith('automexia-ssh-helper.exe'))
        self.assertFalse(any(value.startswith('@upload-') for value in command))
    def test_posix_producers_reject_missing_or_skipped_helper_evidence(self):
        for summary in (b'Ran 12 tests in 1s\nOK (skipped=1)\n',
                        b'Ran 11 tests in 1s\nOK\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(require_posix_shells=True, producer_output=summary)
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
    def test_windows_scope_exports_both_actual_adapters_without_bash(self):
        result,calls=self.invoke(bash=None, windows_shells={'powershell':'ps5', 'pwsh':'ps7'},
                                 require_windows_powershells=True)
        self.assertEqual([step['name'] for step in result['steps']], [
            'build-upload-exporter', 'build-upload-helper',
            'export-actual-generator-powershell', 'powershell-adapter-local', 'powershell-upload-local',
            'export-helper-files-powershell', 'powershell-helper-producer-local',
            'export-actual-generator-pwsh', 'pwsh-adapter-local', 'pwsh-upload-local',
            'export-helper-files-pwsh', 'pwsh-helper-producer-local'])
        for offset,shell,executable in ((2,'powershell','ps5'), (7,'pwsh','ps7')):
            self.assertEqual(calls[offset][0][-2:], ['--',shell])
            self.assertFalse(calls[offset][1])
            self.assertTrue(calls[offset+1][1])
            self.assertEqual(calls[offset+1][0][-2:], ['--powershell',executable])
            self.assertTrue(any(str(value).endswith(shell+'.fixture') for value in calls[offset+1][0]))
            self.assertEqual(calls[offset+2][0][-4:], ['--powershell', executable, '--shell', shell])
            self.assertFalse(any(value.startswith('@upload-') for value in calls[offset+2][0]))
            self.assertFalse(calls[offset+3][1])
            self.assertIn('--files', calls[offset+3][0])
            self.assertIn('--fixtures', calls[offset+4][0])
    def test_required_windows_scope_rejects_each_missing_interpreter(self):
        for present in ({}, {'powershell':'ps5'}, {'pwsh':'ps7'}):
            with self.subTest(present=present), self.assertRaises(ValueError):
                self.invoke(bash=None, windows_shells=present, require_windows_powershells=True)
    def test_optional_windows_runtime_unavailability_is_explicit(self):
        result,_=self.invoke(bash=None,windows_shells={'pwsh':'ps7'})
        self.assertEqual([step['name'] for step in result['steps']],
                         ['build-upload-exporter', 'build-upload-helper',
                          'export-actual-generator-pwsh','pwsh-adapter-local', 'pwsh-upload-local',
                          'export-helper-files-pwsh', 'pwsh-helper-producer-local'])
        self.assertTrue(any('Windows powershell' in gap for gap in result['external']))
    def test_no_native_shell_is_not_a_passing_shell_scope(self):
        with self.assertRaises(ValueError):self.invoke(bash=None)
    def test_windows_adapters_require_nonzero_complete_unskipped_evidence(self):
        for summary in (b'Ran 0 tests in 0s\nOK\n', b'Ran 11 tests in 0s\nOK (skipped=1)\n',
                        b'Ran 10 tests in 0s\nOK\n', b'Ran 11 tests in 0s\nFAILED (failures=1)\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(bash=None,windows_shells={'powershell':'ps5','pwsh':'ps7'},
                            require_windows_powershells=True,adapter_output=summary)
    def test_wrapper_scope_excludes_only_the_ignored_executable_fixture(self):
        _,calls=self.invoke(shell_only=False)
        wrappers=[command for command,_ in calls if 'ssh_wrapper' in command]
        self.assertEqual(len(wrappers),1)
        self.assertEqual(wrappers[0][-3:],['--test','ssh_wrapper','explicit_ssh'])
        codecs=[command for command,_ in calls if 'automexia-terminal-protocol' in command]
        self.assertEqual(len(codecs), 1)
        self.assertEqual(codecs[0][-2:], ['--test', 'revision'])
        revision=[command for command,_ in calls if 'revision_contract' in command]
        self.assertEqual(len(revision), 1)
        self.assertEqual(revision[0][-4:], ['-p', 'automexia-ssh-integration', '--test', 'revision_contract'])

    def test_windows_upload_requires_nonzero_complete_unskipped_evidence(self):
        for summary in (b'Ran 0 tests in 0s\nOK\n', b'Ran 10 tests in 0s\nOK (skipped=1)\n',
                        b'Ran 9 tests in 0s\nOK\n', b'Ran 10 tests in 0s\nFAILED (failures=1)\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(bash=None, windows_shells={'powershell':'ps5', 'pwsh':'ps7'},
                            require_windows_powershells=True, upload_output=summary)

    def test_windows_helper_producer_requires_nonzero_complete_unskipped_evidence(self):
        for summary in (b'Ran 0 tests in 0s\nOK\n', b'Ran 5 tests in 0s\nOK (skipped=1)\n',
                        b'Ran 4 tests in 0s\nOK\n', b'Ran 5 tests in 0s\nFAILED (failures=1)\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(bash=None, windows_shells={'powershell':'ps5', 'pwsh':'ps7'},
                            require_windows_powershells=True, producer_output=summary)

    def test_posix_upload_requires_nonzero_complete_unskipped_evidence(self):
        for summary in (b'Ran 0 tests in 0s\nOK\n', b'Ran 8 tests in 0s\nOK (skipped=1)\n',
                        b'Ran 7 tests in 0s\nOK\n', b'Ran 8 tests in 0s\nFAILED (failures=1)\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(require_posix_shells=True, posix_upload_output=summary)

    def test_posix_helper_producer_requires_nonzero_complete_unskipped_evidence(self):
        for summary in (b'Ran 0 tests in 0s\nOK\n', b'Ran 7 tests in 0s\nOK (skipped=1)\n',
                        b'Ran 6 tests in 0s\nOK\n', b'Ran 7 tests in 0s\nFAILED (failures=1)\n'):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                self.invoke(require_posix_shells=True, producer_output=summary)

    def test_full_windows_scope_requires_native_helper_but_shell_only_stays_interpreter_only(self):
        shells = {'powershell':'ps5', 'pwsh':'ps7'}
        result,calls = self.invoke(bash=None, windows_shells=shells, shell_only=False)
        native = [command for command,_ in calls if any(str(value).endswith('test_ssh_helper_native.py') for value in command)]
        self.assertEqual(len(native), 1)
        self.assertIn('--helper', native[0])
        self.assertNotIn('@upload-helper@', native[0])
        self.assertEqual(next(step['tests'] for step in result['steps'] if step['name']=='windows-helper-conpty'), 1)
        _,calls = self.invoke(bash=None, windows_shells=shells)
        self.assertFalse(any(any(str(value).endswith('test_ssh_helper_native.py') for value in command) for command,_ in calls))
        with self.assertRaises(ValueError):
            self.invoke(bash=None, windows_shells={'powershell':'ps5'}, shell_only=False)


class NativeHelperRunnerTests(unittest.TestCase):
    def invoke(self, output, *, host=True, shells=None):
        from pathlib import Path
        from unittest.mock import patch, MagicMock
        import tempfile
        import test_ssh_helper_native as native
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            helper=root/'helper.exe'; helper.touch()
            fixture=root/'fixture.exe'; fixture.touch()
            stage=MagicMock(executable=fixture)
            with patch.object(native, 'native_windows_host', return_value=host), \
                 patch.object(native, 'available_windows_powershells', return_value=shells if shells is not None else {'powershell':'ps5','pwsh':'ps7'}), \
                 patch.object(native, 'NativeTestStage', return_value=stage), \
                 patch.object(native, 'run', return_value=output) as execute:
                try:
                    result=native.verify(root/'report', helper=helper, test_binary=fixture, offline=True)
                finally:
                    if execute.called:
                        stage.close.assert_called_once()
                stage.prepare.assert_called_once_with(fixture, offline=True)
                self.assertIn('--exact', execute.call_args.args[0])
                return result

    def test_native_helper_accepts_only_actual_single_nonignored_test(self):
        output=b'test result: ok. 1 passed; 0 failed; 0 ignored;\n'
        self.assertEqual(self.invoke(output), output)

    def test_native_helper_zero_skip_failure_and_wrong_count_fail_after_cleanup(self):
        for output in (b'test result: ok. 0 passed; 0 failed; 0 ignored;\n',
                       b'test result: ok. 1 passed; 0 failed; 1 ignored;\n',
                       b'test result: FAILED. 0 passed; 1 failed; 0 ignored;\n',
                       b'test result: ok. 2 passed; 0 failed; 0 ignored;\n'):
            with self.subTest(output=output), self.assertRaises(ValueError): self.invoke(output)

    def test_native_helper_unavailable_host_or_runtime_cannot_pass(self):
        for options in ({'host':False}, {'shells':{}}, {'shells':{'powershell':'ps5'}}, {'shells':{'pwsh':'ps7'}}):
            with self.subTest(options=options), self.assertRaises(ValueError): self.invoke(b'', **options)


class ArtifactTests(unittest.TestCase):
    def test_helper_fixture_requires_exact_owned_source_set(self):
        from check_ssh_library import validate_helper_fixture
        valid = b'AMXSSH-HELPER-FIXTURE-1\nfile:rc.ps1=Y29yZQ==\n'
        validate_helper_fixture(valid, 'powershell')
        validate_helper_fixture(valid, 'pwsh')
        for output in (b'', valid.replace(b'1\n', b'2\n'), valid + valid.splitlines(True)[1],
                       valid.replace(b'rc.ps1', b'../rc.ps1'), valid.replace(b'Y29yZQ==', b'!'),
                       valid.replace(b'file:rc.ps1', b'rc.ps1'), valid.replace(b'Y29yZQ==', b'QEBQQVRIAEA='),
                       valid.splitlines(True)[0]):
            with self.subTest(output=output), self.assertRaises(ValueError):
                validate_helper_fixture(output, 'powershell')
        with self.assertRaises(ValueError): validate_helper_fixture(valid, 'fish')

    def test_helper_fixture_enforces_source_and_total_byte_limits(self):
        import base64
        from check_ssh_library import validate_helper_fixture
        for source in (b'@@NAME@@', b'x' * (32 * 1024 + 1)):
            output = b'AMXSSH-HELPER-FIXTURE-1\nfile:rc.ps1=' + base64.b64encode(source) + b'\n'
            with self.assertRaises(ValueError): validate_helper_fixture(output, 'powershell')
        with self.assertRaises(ValueError): validate_helper_fixture(b'x' * (128 * 1024 + 1), 'powershell')

    def test_exact_cargo_artifact_is_required(self):
        import json
        from pathlib import Path
        from check_ssh_library import compiled_executable
        expected = str(Path.cwd() / 'fixture.exe')
        item = {'reason': 'compiler-artifact', 'target': {'name': 'helper', 'kind': ['bin']}, 'executable': expected}
        raw = json.dumps(item).encode() + b'\n'
        self.assertEqual(compiled_executable(raw, 'helper', 'bin'), expected)
        for data in (b'', raw + raw, raw.replace(b'helper', b'other'), raw.replace(b'"bin"', b'"lib"')):
            with self.assertRaises(ValueError): compiled_executable(data, 'helper', 'bin')

    def test_null_relative_and_invalid_artifacts_are_rejected(self):
        import json
        from check_ssh_library import compiled_executable
        for path in (None, 12, 'relative.exe', 'bad\0path'):
            item = {'reason': 'compiler-artifact', 'target': {'name': 'helper', 'kind': ['bin']}, 'executable': path}
            with self.assertRaises(ValueError): compiled_executable(json.dumps(item).encode(), 'helper', 'bin')
        for item in ([], None, {'target': 42}, {'target': {'name':'helper', 'kind': None}}):
            with self.assertRaises(ValueError): compiled_executable(json.dumps(item).encode(), 'helper', 'bin')

class FailureDiagnosticTests(unittest.TestCase):
    def test_python_failure_keeps_test_shell_and_exception_type_only(self):
        from check_ssh_library import failure_summary
        output = ("ERROR: test_prompt (__main__.AdapterTests.test_prompt) (shell='fish')\n"
                  "Traceback: private-path\nTimeoutError: private-output\n"
                  "FAIL: test_status (__main__.AdapterTests.test_status) (profile='private-profile')\n"
                  "AssertionError: private-value\n")
        self.assertEqual(failure_summary(output), [
            'ERROR test_prompt [fish]: TimeoutError', 'FAIL test_status: AssertionError'])

    def test_python_failure_bounds_records_and_rejects_free_text(self):
        from check_ssh_library import failure_summary
        self.assertEqual(failure_summary('private output\nTimeoutError: private output'), [])
        self.assertEqual(failure_summary('ERROR: private/name (fixture)\n'), [])
        output = 'ERROR: test_prompt (__main__.Tests.test_prompt)\n' * 1000
        self.assertEqual(len(failure_summary(output)), 20)

    def test_failed_orchestrated_suite_reports_redacted_diagnostic(self):
        from contextlib import redirect_stdout
        from io import StringIO
        output = StringIO()
        fixture = b'ERROR: test_prompt (__main__.Tests.test_prompt)\nTimeoutError: private-value\n'
        with redirect_stdout(output), self.assertRaises(ValueError):
            OrchestrationTests().invoke(bash=None, windows_shells={'pwsh': 'pwsh'}, adapter_output=fixture)
        self.assertIn('ERROR test_prompt: TimeoutError', output.getvalue())
        self.assertNotIn('private-value', output.getvalue())


if __name__ == '__main__': unittest.main()
