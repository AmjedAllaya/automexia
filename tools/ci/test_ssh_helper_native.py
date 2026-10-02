#!/usr/bin/env python3
"""Required Windows helper session evidence using the existing native PTY owner."""
import argparse
import os
from pathlib import Path
import sys

import qa_process
from check_inline_pipeline import NativeTestStage, native_test_artifact
from check_ssh_library import available_windows_powershells, compiled_executable, native_windows_host, test_count

ROOT = Path(__file__).resolve().parents[2]
MAX_OUTPUT = 4 * 1024 * 1024


def run(command, report, label, environment):
    output = bytearray()
    with (report / (label + '.log')).open('wb') as log:
        def consume(chunk):
            if len(output) + len(chunk) > MAX_OUTPUT:
                raise ValueError('native helper evidence exceeded the output ceiling')
            output.extend(chunk)
            log.write(chunk)
        outcome = qa_process.run(command, cwd=ROOT, environment=environment,
                                 timeout_seconds=600 if label.endswith('build') else 150,
                                 consume=consume)
    if outcome.return_code != 0 or outcome.timed_out or outcome.error:
        raise ValueError('native helper evidence failed; inspect the bounded ' + label + ' log')
    return bytes(output)


def verify(report, *, helper=None, test_binary=None, offline=False):
    shells = available_windows_powershells()
    if not native_windows_host() or not all(shells.get(name) for name in ('powershell', 'pwsh')):
        raise ValueError('native Windows PowerShell 5 and PowerShell 7 are required')
    report.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, CARGO_TERM_COLOR='never', PYTHONUTF8='1')
    if helper is None:
        output = run(['cargo', 'build', '--locked', '-p', 'automexia-terminal',
                      '--bin', 'automexia-ssh-helper', '--message-format=json'],
                     report, 'helper-build', environment)
        helper = Path(compiled_executable(output, 'automexia-ssh-helper', 'bin'))
    if test_binary is None:
        output = run(['cargo', 'test', '--locked', '-p', 'automexia-terminal',
                      '--test', 'inline_pipeline_native', '--no-run', '--message-format=json'],
                     report, 'native-test-build', environment)
        test_binary = native_test_artifact(output.decode('utf-8'))
    for artifact in (helper, test_binary):
        if not artifact.is_absolute() or not artifact.is_file() or artifact.is_symlink():
            raise ValueError('native helper evidence requires exact regular build artifacts')
    environment['AUTOMEXIA_SSH_HELPER_NATIVE_BINARY'] = str(helper)
    stage = NativeTestStage(report)
    try:
        stage.prepare(test_binary, offline=offline)
        output = run([str(stage.executable), '--exact',
                      'ssh_native::inline_pipeline_native_helper_powershell_discovers_and_retires',
                      '--ignored', '--test-threads=1'], report, 'native-session', environment)
        if test_count(output, 'rust') != 1:
            raise ValueError('native helper session fixture did not execute exactly once')
        return output
    finally:
        stage.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--helper', type=Path)
    parser.add_argument('--test-binary', type=Path)
    parser.add_argument('--report-dir', type=Path, default=ROOT / 'target/ssh-native-helper')
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    try:
        output = verify(args.report_dir, helper=args.helper, test_binary=args.test_binary,
                        offline=args.offline)
    except (OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1
    # Preserve the actual nonzero Rust summary for the enclosing canonical gate.
    sys.stdout.buffer.write(output)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
