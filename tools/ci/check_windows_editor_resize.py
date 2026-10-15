#!/usr/bin/env python3
"""Check native terminal editing with the exact packaged Microsoft runtimes."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys

import qa_process
import windows_powershell_editor as EDITOR
from check_inline_pipeline import NativeTestStage
from check_ssh_library import compiled_executable, test_count

ROOT = Path(__file__).resolve().parents[2]
MAX_LOG = 8 * 1024 * 1024
TARGETS = ('conformance_fixtures', 'live_resize', 'pane_editor_resize', 'resize_repaint')
EXPECTED_COUNTS = {'conformance_fixtures': 5, 'live_resize': 11, 'pane_editor_resize': 5, 'resize_repaint': 19}
OPTIONAL_WSL_CASES = (
    'native_live_wsl_resize_preserves_output_and_prompt_adjacency',
    'native_live_wsl_multicolumn_icons_preserve_layout',
    'native_live_wsl_eza_listing_restores_columns',
    'native_live_wsl_eza_extreme_resize_restores_columns',
    'native_completed_ultrawide_eza_listing_survives_first_shrink',
)


def powershell_environment(environment: dict[str, str], module_path: str | None) -> dict[str, str]:
    # Windows names are case insensitive; Python exposes inherited names in
    # uppercase. Do not retain a competing key or the build shell's Core modules.
    child = {key: value for key, value in environment.items() if key.casefold() != 'psmodulepath'}
    if module_path is not None:
        child['PSModulePath'] = module_path
    return child


def run(command, report: Path, label: str, environment: dict, timeout: int):
    print(f'CHECK: {label}', flush=True)
    captured = bytearray()
    with (report / (label + '.log')).open('xb') as log:
        def consume(chunk):
            if len(captured) + len(chunk) > MAX_LOG:
                raise ValueError('Native editor output exceeded its bound')
            captured.extend(chunk)
            log.write(chunk)
        result = qa_process.run(command, cwd=ROOT, environment=environment,
                                timeout_seconds=timeout, consume=consume)
    if result.return_code or result.timed_out or result.error:
        raise ValueError(f'Native editor check failed: {label}; inspect the bounded log')
    return bytes(captured)


def verify(report: Path, *, offline=False) -> None:
    if os.name != 'nt':
        raise ValueError('Native Windows is required')
    report.mkdir(parents=True, exist_ok=False)
    environment = dict(os.environ)
    environment['PATH'] = str(Path(sys.executable).parent) + os.pathsep + environment.get('PATH', '')
    command = ['cargo', 'test', '--locked', '-p', 'rio-vt', '--features', 'graphics', '--no-run', '--message-format=json']
    for target in TARGETS:
        command += ['--test', target]
    output = run(command, report, 'build', environment, 1800)
    powershell = str(Path(os.environ['SystemRoot']) / 'System32/WindowsPowerShell/v1.0/powershell.exe')
    verifier = str(ROOT / 'tools/ci/verify_windows_powershell_editor.ps1')
    base = [powershell, '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'RemoteSigned', '-File', verifier]
    # Keep default/redirected user, shared and system module locations without
    # persisting their private values in the report or changing the parent.
    module_paths = bytearray()
    print('CHECK: native Windows PowerShell module paths', flush=True)
    def capture_paths(chunk):
        if len(module_paths) + len(chunk) > 32 * 1024:
            raise ValueError('Module path output exceeds its bound')
        module_paths.extend(chunk)
    paths_result = qa_process.run(base + ['-ModulePathOnly'], cwd=ROOT,
                                 environment=powershell_environment(environment, None),
                                 timeout_seconds=30, consume=capture_paths)
    if (paths_result.return_code or paths_result.timed_out or paths_result.error
            or not 0 < len(module_paths) <= 32 * 1024):
        raise ValueError('Windows PowerShell default module paths are unavailable')
    existing_paths = bytes(module_paths).decode('utf-8').strip()
    records = []
    for target in TARGETS:
        stage = NativeTestStage(report)
        try:
            print(f'CHECK: {target}-conpty-staging', flush=True)
            stage.prepare(Path(compiled_executable(output, target, 'test')), offline=offline)
            print(f'CHECK: {target}-editor-staging', flush=True)
            EDITOR.prepare_runtime(stage.directory, offline=offline)
            child = powershell_environment(environment, str(stage.directory / 'powershell-modules') + ';' + existing_paths)
            run(base + ['-Root', str(stage.directory), '-Load'], report, target + '-vendor', child, 120)
            arguments = [str(stage.executable)]
            if target == 'live_resize':
                # These existing opt-in WSL/eza cases have a separate owner and
                # prerequisites. Keep them visible in evidence; every selected
                # Windows case must pass with no ignored or zero-test success.
                for name in OPTIONAL_WSL_CASES:
                    arguments += ['--skip', name]
            result = run(arguments, report, target, child, 180)
            count = test_count(result, 'rust')
            if count != EXPECTED_COUNTS[target]:
                raise ValueError('Native editor fixture workload differs from its reviewed inventory')
            records.append({'target': target, 'passed': count, 'conpty_version': stage.version,
                            'editor_version': EDITOR.load_recipe()['package']['version'],
                            'optional_wsl_cases_not_run': list(OPTIONAL_WSL_CASES) if target == 'live_resize' else []})
            print(f'PASS: {target}: {count} tests')
        finally:
            if qa_process._quarantine is not None:
                stage.cleanup_safe = False
            stage.close()
    (report / 'summary.json').write_text(json.dumps(records, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report-dir', type=Path, default=ROOT / 'target/native-windows-editor')
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    try:
        verify(args.report_dir.resolve(), offline=args.offline)
    except EDITOR.EditorError as error:
        # This adapter's typed errors contain only fixed, content-free text.
        print(f'FAIL: {error}', file=sys.stderr)
        raise SystemExit(1)
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        # Error types and owned phase names are safe; exception text may contain
        # private paths. Detailed fixture output stays in bounded CI evidence.
        print(f'FAIL: native Windows editor verification ({type(error).__name__}); inspect retained bounded logs', file=sys.stderr)
        raise SystemExit(1)
