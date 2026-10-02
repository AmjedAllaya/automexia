#!/usr/bin/env python3
"""Bounded, non-activating verification for the SSH planning library.

Uses the repository's existing QA process owner. No SSH server, GUI, or remote
machine is contacted. A passing run is not native SSH/ConPTY evidence.
"""
from __future__ import annotations
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
MAX_LOG_BYTES = 2 * 1024 * 1024
ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')


def clean(output: bytes | str) -> str:
    if isinstance(output, bytes):
        output = output.decode('utf-8', errors='replace')
    return ANSI.sub('', output).replace('\r\n', '\n').replace('\r', '\n')


def test_count(output: bytes | str, kind: str) -> int:
    text = clean(output)
    if kind == 'python':
        counts = re.findall(r'(?m)^Ran ([1-9][0-9]*) tests? in .+$', text)
        if len(counts) != 1 or re.search(r'(?m)^OK\s*$', text) is None:
            raise ValueError('Python tests did not report nonzero success')
        if re.search(r'(?m)^(?:FAILED|ERROR|FAIL|OK \()', text):
            raise ValueError('Python failure/skip or conflicting summary')
        return int(counts[0])
    summaries = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    if not summaries or any(state != 'ok' or int(failed) or int(ignored)
                            for state, _, failed, ignored in summaries):
        raise ValueError('Rust failures, ignored tests, or missing summary')
    count = sum(int(passed) for _, passed, _, _ in summaries)
    if count == 0:
        raise ValueError('Rust filter matched zero passing tests')
    return count


# Reviewed workload inventory: prevents a renamed/empty Criterion filter from
# turning a zero-benchmark run into successful performance evidence.
EXPECTED_BENCHMARKS = (
    'classify_native_arguments', 'disabled_passthrough', 'denied_no_bootstrap',
    'bounded_bootstrap', 'accept_scoped_receipt', 'reject_wrong_pane',
    'reject_unimplemented_capability', 'maximum_remote_path', 'reconnect_and_close',
)


def benchmark_count(output: bytes | str, *, smoke: bool) -> int:
    text = clean(output)
    for name in EXPECTED_BENCHMARKS:
        identifier = re.escape('ssh_integration/' + name)
        pattern = (rf'(?m)^Testing {identifier}[ \t]*\nSuccess[ \t]*$' if smoke else
                   rf'(?m)^{identifier}[ \t]*(?:\n[ \t]*)?time:[ \t]*\[[0-9][^\]\n]*\]')
        if re.search(pattern, text) is None:
            raise ValueError('missing completed benchmark evidence: ' + name)
    if re.search(r'(?im)^.*(?:panicked at|^FAILED|^ERROR:)', text):
        raise ValueError('benchmark output contains a failure')
    return len(EXPECTED_BENCHMARKS)


def load_owner(root: Path):
    path = root / 'tools/ci/qa_process.py'
    if path.is_symlink() or not path.is_file():
        raise ValueError('existing tools/ci/qa_process.py is required')
    name = 'automexia_ssh_qa_process'
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ValueError('cannot load existing QA owner')
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def check_boundaries(root: Path) -> None:
    # The canonical static validator is shared with xtask and repository policy.
    path = root / 'tools/ci/check_ssh_boundaries.py'
    spec = importlib.util.spec_from_file_location('automexia_ssh_boundaries', path)
    if spec is None or spec.loader is None: raise ValueError('SSH boundary owner missing')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.validate_repository(root)


def available_bash() -> str | None:
    return shutil.which('bash') if os.name == 'posix' else None


def verify(root: Path, report_dir: Path, *, baseline=False, ready=False,
           require_bash=False, timeout=3600, shell_only=False, bench=False, bench_smoke=False) -> dict:
    if bench and bench_smoke:
        raise ValueError('choose measurement or benchmark smoke, not both')
    if shell_only and (ready or bench or bench_smoke or baseline):
        raise ValueError('shell-only scope cannot be combined with build/benchmark scopes')
    root = root.resolve()
    if os.name == 'posix' and re.match(r'^/mnt/[a-z](?:/|$)', str(root)):
        raise ValueError('Linux Cargo checks require a Linux-filesystem checkout, not /mnt/<drive>; use Windows Python in this Windows checkout')
    report_dir.mkdir(parents=True, exist_ok=True)
    owner = load_owner(root)
    cargo = shutil.which('cargo')
    if cargo is None:
        raise ValueError('cargo is required; install the repository toolchain first')
    env = dict(os.environ, CARGO_TERM_COLOR='never', PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    result = {'schema': 1, 'scope': 'planning-library-not-live-ssh', 'steps': [], 'external': [
        'Rust/SSH host matrix outside this run', 'Windows/WSL/ConPTY and native GUI behavior',
        'full remote prompt/status/provider parity', 'controlled native latency and resource baselines']}
    if baseline:
        steps = [('baseline-connectivity', [cargo, 'test', '--locked', '-p', 'automexia-connectivity', '--tests'], 'rust', 1)]
    else:
        check_boundaries(root)
        steps = [
            ('checker-self-tests', [sys.executable, str(root / 'tools/ci/test_check_ssh_library.py')], 'python', 35),
            ('library-unit', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--lib'], 'rust', 3),
            ('library-hardening', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'hardening'], 'rust', 12),
            ('boundary-mutations', [sys.executable, str(root / 'tools/ci/test_ssh_boundaries.py')], 'python', 28),
            ('xtask-dependency-mutations', [cargo, 'test', '--locked', '-p', 'xtask', 'ssh_planning_dependency_tests'], 'rust', 4),
            ('library-contracts', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'contracts'], 'rust', 34),
            ('application-cli', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--test', 'ssh_integration'], 'rust', 6),
            ('application-gate', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--bin', 'automexia', 'ssh_library_gate'], 'rust', 1),
            ('existing-review-regressions', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--lib', 'connections::direct_openssh::tests'], 'rust', 1),
            ('feature-reinforcement', [sys.executable, str(root / 'tools/ci/check_feature_test_reinforcement.py')], None, 0),
            ('reinforcement-mutations', [sys.executable, str(root / 'tools/ci/test_feature_test_reinforcement.py')], 'python', 1),
            ('feature-assurance', [sys.executable, str(root / 'tools/ci/check_feature_assurance.py')], None, 0),
            ('documentation-coverage', [sys.executable, str(root / 'tools/ci/check_documentation_coverage.py')], None, 0),
            ('documentation-self-tests', [sys.executable, str(root / 'tools/ci/test_documentation_coverage.py')], 'python', 1),
        ]
        if shell_only: steps = []
        bash = available_bash()
        generated = report_dir / 'actual-rust-generator.fixture'
        if bash is not None:
            steps.extend([
                ('export-actual-generator', [cargo, 'run', '--quiet', '--locked', '-p', 'automexia-ssh-integration', '--example', 'export_shell_fixture'], None, 0),
                ('bash-core-local', [sys.executable, str(root / 'tools/ci/test_ssh_bash_core.py'), '--bash', bash, '--generated-fixture', str(generated)], 'python', 34),
            ])
        elif require_bash or shell_only:
            raise ValueError('this scope requires a Unix-native Bash test host')
        else:
            result['external'].append('Bash controlling-PTY and actual generated bootstrap unavailable on this host')
            print('UNAVAILABLE: native Unix Bash adapter evidence; not a passing result.')
        if bench or bench_smoke:
            command = [cargo, 'bench', '--locked', '-p', 'automexia-terminal', '--bench', 'automexia_services', '--', 'ssh_integration']
            if bench_smoke: command.append('--test')
            steps.append(('ssh-benchmark-smoke' if bench_smoke else 'ssh-benchmarks-measured', command, None, 0))
        if ready:
            steps.append(('cargo-ready', [cargo, 'ready'], None, 0))
    for label, command, kind, minimum in steps:
        print(f'[ssh-library] {label}', flush=True)
        path = report_dir / f'{label}.log'
        output = bytearray()
        with path.open('wb') as log:
            def consume(chunk: bytes) -> None:
                if len(output) + len(chunk) > MAX_LOG_BYTES:
                    raise ValueError('verification log exceeded its fixed byte ceiling')
                log.write(chunk)
                output.extend(chunk)
            outcome = owner.run(command, cwd=root, timeout_seconds=timeout,
                                consume=consume, environment=env, merge_stderr=label != 'export-actual-generator')
        entry = {'name': label, 'return_code': outcome.return_code,
                 'timed_out': outcome.timed_out, 'error': outcome.error, 'log': path.name}
        result['steps'].append(entry)
        try:
            if outcome.return_code != 0 or outcome.timed_out or outcome.error:
                raise ValueError('command failed, timed out, or cleanup was incomplete')
            if label == 'export-actual-generator':
                import base64
                lines = bytes(output).splitlines()
                if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
                    raise ValueError('actual Rust generator did not produce its fixture')
                for line in lines[1:]: base64.b64decode(line, validate=True).decode('utf-8')
                generated.write_bytes(bytes(output))
                entry['fixture_source'] = 'actual-compiled-Rust-generator'
            if label in {'ssh-benchmark-smoke', 'ssh-benchmarks-measured'}:
                entry['benchmarks'] = benchmark_count(bytes(output), smoke=label == 'ssh-benchmark-smoke')
                entry['measurement'] = label == 'ssh-benchmarks-measured'
            if kind:
                count = test_count(bytes(output), kind)
                if count < minimum:
                    raise ValueError(f'only {count} tests ran; expected at least {minimum}')
                entry['tests'] = count
                print(f'PASS ({count} tests)', flush=True)
            else:
                print('PASS (command exit status)', flush=True)
            entry['status'] = 'passed'
        except ValueError as error:
            entry['status'] = 'failed'
            entry['diagnostic'] = str(error)
            (report_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
            raise ValueError(f'{label}: {error}. Log: {path}') from error
    (report_dir / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    return result


def main() -> int:
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, 'reconfigure'):
            stream.reconfigure(encoding='utf-8', errors='backslashreplace')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ready', action='store_true')
    parser.add_argument('--shell-only', action='store_true')
    parser.add_argument('--bench', action='store_true', help='measure the existing Criterion SSH benchmark group')
    parser.add_argument('--bench-smoke', action='store_true', help='run benchmark correctness smoke, not latency measurement')
    parser.add_argument('--require-bash', action='store_true')
    parser.add_argument('--timeout', type=int, default=3600)
    parser.add_argument('--report-dir', type=Path, default=ROOT / 'target/ssh-library-checks')
    args = parser.parse_args()
    if not 1 <= args.timeout <= 14400:
        parser.error('timeout must be 1..14400 seconds')
    try:
        verify(ROOT, args.report_dir, ready=args.ready, require_bash=args.require_bash, timeout=args.timeout, shell_only=args.shell_only, bench=args.bench, bench_smoke=args.bench_smoke)
    except (OSError, ValueError) as error:
        print(f'FAILED: {error}', file=sys.stderr)
        return 1
    print('Focused planning-library checks passed. Enhanced SSH is still disabled; native UX was not measured.')
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
