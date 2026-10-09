#!/usr/bin/env python3
"""Bounded model, CLI and generated-shell verification for SSH integration.

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


def failure_summary(output: bytes | str) -> list[str]:
    """Expose bounded unittest identities, never fixture output or tracebacks."""
    summaries: list[str] = []
    for block in re.split(r'(?m)^(?=(?:ERROR|FAIL): )', clean(output)):
        match = re.match(r'(ERROR|FAIL): (test_[A-Za-z0-9_]{1,160}) '
                         r'\([A-Za-z0-9_.]{1,240}\)([^\n]*)\n', block)
        if match is None:
            continue
        shell = re.search(r"\(shell='(bash|zsh|fish|powershell|pwsh)'\)", match[3])
        kind = re.search(r'(?m)^(AssertionError|TimeoutError|RuntimeError|OSError|'
                         r'ValueError|IndexError|KeyError|TypeError|ImportError):', block)
        summary = f'{match[1]} {match[2]}'
        if shell:
            summary += f' [{shell[1]}]'
        if kind:
            summary += f': {kind[1]}'
        summaries.append(summary)
        if len(summaries) == 20:
            break
    return summaries


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


def native_windows_host() -> bool:
    return os.name == 'nt'


def available_windows_powershells() -> dict[str, str | None]:
    return {shell: shutil.which(shell) if os.name == 'nt' else None
            for shell in ('powershell', 'pwsh')}


def compiled_executable(output: bytes, name: str, kind: str) -> str:
    """Use Cargo's reported artifact, respecting configured target directories."""
    matches = []
    for line in output.splitlines():
        try:
            item = json.loads(line)
        except (ValueError, UnicodeDecodeError):
            continue
        target = item.get('target') if isinstance(item, dict) else None
        if (isinstance(target, dict) and item.get('reason') == 'compiler-artifact'
                and target.get('name') == name
                and isinstance(target.get('kind'), list) and kind in target['kind']):
            executable = item.get('executable')
            if not isinstance(executable, str) or '\0' in executable or not Path(executable).is_absolute():
                raise ValueError('Cargo did not report a valid native executable')
            matches.append(executable)
    if len(matches) != 1:
        raise ValueError('Cargo did not report exactly one requested executable')
    return matches[0]


def validate_helper_fixture(output: bytes, shell: str) -> None:
    import base64
    expected = {'bash': {'rc.bash'}, 'zsh': {'.zshenv', '.zshrc'},
                'powershell': {'rc.ps1'}, 'pwsh': {'rc.ps1'}}
    lines = output.splitlines()
    if len(output) > 128 * 1024 or not lines or lines[0] != b'AMXSSH-HELPER-FIXTURE-1':
        raise ValueError('invalid generated helper fixture')
    observed = set()
    for line in lines[1:]:
        label, separator, value = line.partition(b'=')
        if not separator or not label.startswith(b'file:'):
            raise ValueError('helper fixture must contain only source files')
        name = label.removeprefix(b'file:').decode('ascii')
        source = base64.b64decode(value, validate=True).decode('utf-8')
        if name in observed or name not in expected.get(shell, set()) or '@@' in source or len(source.encode()) > 32 * 1024:
            raise ValueError('invalid helper source ownership or size')
        observed.add(name)
    if observed != expected.get(shell):
        raise ValueError('missing generated helper source')


def verify(root: Path, report_dir: Path, *, baseline=False, ready=False,
           require_bash=False, require_posix_shells=False, require_windows_powershells=False,
           timeout=3600, shell_only=False, bench=False, bench_smoke=False) -> dict:
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
    result = {'schema': 1, 'scope': 'model-cli-shell-not-live-ssh', 'steps': [], 'external': [
        'Rust/SSH host matrix outside this run', 'Windows/WSL/ConPTY and native GUI behavior',
        'full remote prompt/status/provider parity', 'controlled native latency and resource baselines']}
    if baseline:
        steps = [('baseline-connectivity', [cargo, 'test', '--locked', '-p', 'automexia-connectivity', '--tests'], 'rust', 1)]
    else:
        check_boundaries(root)
        steps = [
            ('checker-self-tests', [sys.executable, str(root / 'tools/ci/test_check_ssh_library.py')], 'python', 54),
            ('library-unit', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--lib'], 'rust', 3),
            ('terminal-revision-codec', [cargo, 'test', '--locked', '-p', 'automexia-terminal-protocol', '--test', 'revision'], 'rust', 6),
            ('library-hardening', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'hardening'], 'rust', 12),
            ('boundary-mutations', [sys.executable, str(root / 'tools/ci/test_ssh_boundaries.py')], 'python', 38),
            ('xtask-dependency-mutations', [cargo, 'test', '--locked', '-p', 'xtask', 'ssh_planning_dependency_tests'], 'rust', 7),
            ('library-contracts', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'contracts'], 'rust', 34),
            ('library-revision-contract', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'revision_contract'], 'rust', 2),
            ('library-helper-contracts', [cargo, 'test', '--locked', '-p', 'automexia-ssh-integration', '--test', 'helper_contracts'], 'rust', 8),
            ('application-cli', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--test', 'ssh_integration'], 'rust', 6),
            ('wrapper-cli', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--test', 'ssh_wrapper', 'explicit_ssh'], 'rust', 5),
            ('remote-vt-scope', [cargo, 'test', '--locked', '-p', 'rio-vt', '--lib', 'scope'], 'rust', 9),
            ('remote-renderer-scope', [cargo, 'test', '--locked', '-p', 'automexia-terminal', '--bin', 'automexia', 'ssh_scope'], 'rust', 8),
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
            if require_posix_shells:
                for shell in ('zsh', 'fish'):
                    if shutil.which(shell) is None:
                        raise ValueError('required native shell unavailable: ' + shell)
                    steps.append(('export-actual-generator-' + shell,
                        [cargo, 'run', '--quiet', '--locked', '-p', 'automexia-ssh-integration',
                         '--example', 'export_shell_fixture', '--', shell], None, 0))
                steps.append(('other-shells-local', [sys.executable,
                    str(root / 'tools/ci/test_ssh_adapters.py'), '--fixtures', str(report_dir)], 'python', 8))
                for shell in ('bash', 'zsh'):
                    steps.append(('export-helper-files-' + shell, [cargo, 'run', '--quiet', '--locked', '-p',
                        'automexia-ssh-integration', '--example', 'export_helper_fixture', '--', '--files', shell], None, 0))
                steps.append(('build-upload-helper', [cargo, 'build', '--quiet', '--locked', '-p',
                    'automexia-terminal', '--bin', 'automexia-ssh-helper', '--message-format=json'], None, 0))
                steps.append(('posix-helper-producers-local', [sys.executable,
                    str(root / 'tools/ci/test_ssh_helper_adapters.py'),
                    '--fixtures', str(report_dir / 'helper-adapters'),
                    '--helper', '@upload-helper@'], 'python', 12))
                steps.extend([
                    ('build-upload-exporter', [cargo, 'build', '--quiet', '--locked', '-p',
                     'automexia-ssh-integration', '--example', 'export_helper_fixture', '--message-format=json'], None, 0),
                    ('posix-helper-upload-local', [sys.executable,
                     str(root / 'tools/ci/test_ssh_helper_transfer.py'),
                     '--exporter', '@upload-exporter@'], 'python', 8),
                ])
            else:
                result['external'].append('POSIX helper producer evidence requires --require-posix-shells with native Bash and Zsh')
        elif require_bash or require_posix_shells:
            raise ValueError('this scope requires a Unix-native Bash test host')
        else:
            result['external'].append('Bash controlling-PTY and actual generated bootstrap unavailable on this host')
            print('UNAVAILABLE: native Unix Bash adapter evidence; not a passing result.')
        windows_shells = available_windows_powershells()
        if any(windows_shells.values()):
            if not require_posix_shells:
                steps.append(('build-upload-exporter', [cargo, 'build', '--quiet', '--locked', '-p',
                    'automexia-ssh-integration', '--example', 'export_helper_fixture', '--message-format=json'], None, 0))
            if not require_posix_shells:
                steps.append(('build-upload-helper', [cargo, 'build', '--quiet', '--locked', '-p',
                    'automexia-terminal', '--bin', 'automexia-ssh-helper', '--message-format=json'], None, 0))
        for shell in ('powershell', 'pwsh'):
            executable = windows_shells.get(shell)
            if executable is None:
                if require_windows_powershells:
                    raise ValueError('required native Windows shell unavailable: ' + shell)
                result['external'].append('native Windows ' + shell + ' adapter evidence unavailable on this host')
                print('UNAVAILABLE: native Windows ' + shell + ' adapter evidence; not a passing result.')
                continue
            steps.extend([
                ('export-actual-generator-' + shell,
                 [cargo, 'run', '--quiet', '--locked', '-p', 'automexia-ssh-integration',
                  '--example', 'export_shell_fixture', '--', shell], None, 0),
                (shell + '-adapter-local', [sys.executable,
                 str(root / 'tools/ci/test_ssh_powershell_adapter.py'),
                 '--fixture', str(report_dir / (shell + '.fixture')),
                 '--powershell', executable], 'python', 11),
                (shell + '-upload-local', [sys.executable,
                 str(root / 'tools/ci/test_ssh_helper_powershell_transfer.py'),
                 '--exporter', '@upload-exporter@', '--helper', '@upload-helper@',
                 '--powershell', executable, '--shell', shell], 'python', 10),
                ('export-helper-files-' + shell, [cargo, 'run', '--quiet', '--locked', '-p',
                 'automexia-ssh-integration', '--example', 'export_helper_fixture', '--', '--files', shell], None, 0),
                (shell + '-helper-producer-local', [sys.executable,
                 str(root / 'tools/ci/test_ssh_helper_powershell.py'),
                 '--fixtures', str(report_dir / 'helper-adapters'),
                 '--powershell', executable, '--shell', shell], 'python', 5),
            ])
        if not shell_only and native_windows_host():
            if not all(windows_shells.get(shell) for shell in ('powershell', 'pwsh')):
                raise ValueError('full native Windows SSH scope requires PowerShell 5 and PowerShell 7')
            steps.append(('windows-helper-conpty', [sys.executable,
                str(root / 'tools/ci/test_ssh_helper_native.py'), '--helper', '@upload-helper@',
                '--report-dir', str(report_dir / 'native-helper')], 'rust', 1))
        if shell_only and not steps:
            raise ValueError('shell-only scope requires at least one available native shell')
        if bench or bench_smoke:
            command = [cargo, 'bench', '--locked', '-p', 'automexia-terminal', '--bench', 'automexia_services', '--', 'ssh_integration']
            if bench_smoke: command.append('--test')
            steps.append(('ssh-benchmark-smoke' if bench_smoke else 'ssh-benchmarks-measured', command, None, 0))
        if ready:
            steps.append(('cargo-ready', [cargo, 'ready'], None, 0))
    artifacts: dict[str, str] = {}
    for label, command, kind, minimum in steps:
        command = [artifacts.get(value, value) for value in command]
        if any(value in ('@upload-exporter@', '@upload-helper@') for value in command):
            raise ValueError('native upload tests require both completed build artifacts')
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
                                consume=consume, environment=env,
                                merge_stderr=not label.startswith(('export-actual-generator', 'export-helper-files-')))
        entry = {'name': label, 'return_code': outcome.return_code,
                 'timed_out': outcome.timed_out, 'error': outcome.error, 'log': path.name}
        result['steps'].append(entry)
        try:
            if outcome.return_code != 0 or outcome.timed_out or outcome.error:
                raise ValueError('command failed, timed out, or cleanup was incomplete')
            if label == 'build-upload-exporter':
                artifacts['@upload-exporter@'] = compiled_executable(bytes(output), 'export_helper_fixture', 'example')
            if label == 'build-upload-helper':
                artifacts['@upload-helper@'] = compiled_executable(bytes(output), 'automexia-ssh-helper', 'bin')
            if label.startswith('export-helper-files-'):
                shell = label.removeprefix('export-helper-files-')
                validate_helper_fixture(bytes(output), shell)
                directory = report_dir / 'helper-adapters'
                directory.mkdir(exist_ok=True)
                (directory / (shell + '.fixture')).write_bytes(bytes(output))
                entry['fixture_source'] = 'actual-compiled-Rust-generator'
            if label.startswith('export-actual-generator'):
                import base64
                lines = bytes(output).splitlines()
                if len(lines) != 3 or lines[0] != b'AMXSSH-FIXTURE-1':
                    raise ValueError('actual Rust generator did not produce its fixture')
                for line in lines[1:]: base64.b64decode(line, validate=True).decode('utf-8')
                fixture = generated if label == 'export-actual-generator' else report_dir / (label.removeprefix('export-actual-generator-') + '.fixture')
                fixture.write_bytes(bytes(output))
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
            if kind == 'python':
                entry['test_failures'] = failure_summary(bytes(output))
                for summary in entry['test_failures']:
                    print(f'[ssh-library] {summary}', flush=True)
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
    parser.add_argument('--require-posix-shells', action='store_true', help='require generated Bash, Zsh and Fish PTY checks')
    parser.add_argument('--require-windows-powershells', action='store_true', help='require generated adapters on native Windows PowerShell and PowerShell 7')
    parser.add_argument('--timeout', type=int, default=3600)
    parser.add_argument('--report-dir', type=Path, default=ROOT / 'target/ssh-library-checks')
    args = parser.parse_args()
    if not 1 <= args.timeout <= 14400:
        parser.error('timeout must be 1..14400 seconds')
    try:
        verify(ROOT, args.report_dir, ready=args.ready, require_bash=args.require_bash, require_posix_shells=args.require_posix_shells, require_windows_powershells=args.require_windows_powershells, timeout=args.timeout, shell_only=args.shell_only, bench=args.bench, bench_smoke=args.bench_smoke)
    except (OSError, ValueError) as error:
        print(f'FAILED: {error}', file=sys.stderr)
        return 1
    print('Focused model, CLI and shell checks passed. Native SSH/server and GUI evidence require separate runtime tests.')
    return 0
if __name__ == '__main__':
    raise SystemExit(main())
