#!/usr/bin/env python3
"""Run controlled renderer captures and the existing strict image oracle.

This does not certify native windows, hardware, IME or assistive technologies.
It never creates/updates a reviewed baseline. Missing baselines remain missing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import sys
import tempfile

import qa_process

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / 'tests/assurance/visual-diff-policy-v1.json'
MAX_CASES = 512
MAX_JSON = 65536
MAX_IMAGE = 64 * 1024 * 1024
MAX_OUTPUT = 1024 * 1024
TOKEN = re.compile(r'[A-Za-z0-9._+-]{1,96}\Z')
MUTATIONS = {'baseline-one-pixel', 'missing-glyph', 'clipped-glyph', 'wrong-foreground', 'cell-shift',
             'cursor-one-pixel', 'cursor-hidden', 'cursor-low-contrast', 'cell-padding', 'line-height',
             'underline-one-pixel', 'ansi-red-one-channel', 'background-gap', 'selection-shift', 'box-seam', 'powerline-shift'}


class CommandFailure(RuntimeError):
    def __init__(self, argv: list[str], timed_out: bool, process_error: bool):
        super().__init__('visual command did not complete under process ownership')
        self.command_kind = ('git-' + argv[1] if argv and argv[0] == 'git' and len(argv) > 1
                             and argv[1] in ('diff', 'ls-files', 'rev-parse', 'status') else 'visual-tool')
        self.reason = 'deadline' if timed_out else ('process-owner' if process_error else 'missing-exit-status')


def read(path: Path, limit: int) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError('visual evidence is not a bounded regular file')
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    if len(data) > limit:
        raise ValueError('visual evidence grew beyond its bound')
    return data


def file_digest(path: Path, limit: int) -> str:
    """Hash large binaries in bounded chunks, never a whole-binary allocation."""
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError('digest input is not a bounded regular file')
    digest = hashlib.sha256()
    total = 0
    with path.open('rb') as stream:
        while chunk := stream.read(65536):
            total += len(chunk)
            if total > limit:
                raise ValueError('digest input grew beyond its bound')
            digest.update(chunk)
    return digest.hexdigest()


def atomic_json(path: Path, value: object) -> None:
    data = json.dumps(value, indent=2, sort_keys=True).encode() + b'\n'
    if len(data) > 2 * MAX_OUTPUT:
        raise ValueError('visual report exceeds its bound')
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(data)
        stream.flush()
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def command(argv: list[str], timeout: int, environment: dict[str, str] | None = None,
            *, merge_stderr: bool = True) -> tuple[int, bytes]:
    output = bytearray()
    overflow = False
    def consume(chunk: bytes) -> None:
        nonlocal overflow
        overflow |= len(output) + len(chunk) > MAX_OUTPUT
        output.extend(chunk[:max(0, MAX_OUTPUT - len(output))])
    result = qa_process.run(argv, cwd=ROOT, timeout_seconds=timeout, consume=consume,
                            environment=environment, merge_stderr=merge_stderr)
    if result.timed_out or result.error or result.return_code is None:
        raise CommandFailure(argv, result.timed_out, bool(result.error))
    if overflow:
        raise ValueError('visual command exceeded its output bound')
    return result.return_code, bytes(output)


def source_fingerprint(commit: str) -> str:
    """Bind local edits without putting source text or filenames in reports."""
    digest = hashlib.sha256(commit.encode('ascii'))
    # Conversion warnings are diagnostics, not source bytes. Their presence can
    # vary after an index refresh even when the actual patch is unchanged.
    code, changes = command(['git', 'diff', '--no-ext-diff', '--binary', 'HEAD', '--'], 30, merge_stderr=False)
    if code:
        raise ValueError('source diff unavailable')
    digest.update(changes)
    code, names = command(['git', 'ls-files', '--others', '--exclude-standard', '-z'], 15, merge_stderr=False)
    entries = sorted(name for name in names.split(b'\0') if name)
    if code or len(entries) > 1024:
        raise ValueError('untracked source inventory unavailable or too large')
    total = 0
    for name in entries:
        path = ROOT / os.fsdecode(name)
        if not path.resolve().is_relative_to(ROOT) or path.is_symlink():
            raise ValueError('source entry escapes the repository')
        total += path.stat().st_size
        if total > 128 * 1024 * 1024:
            raise ValueError('untracked source exceeds its bound')
        digest.update(len(name).to_bytes(8, 'big'))
        digest.update(name)
        digest.update(bytes.fromhex(file_digest(path, 64 * 1024 * 1024)))
    return digest.hexdigest()


def environment_identity() -> dict[str, str]:
    system = {'Windows': 'windows', 'Linux': 'linux', 'Darwin': 'macos'}.get(platform.system())
    arch = {'AMD64': 'x86_64', 'x86_64': 'x86_64', 'arm64': 'aarch64', 'aarch64': 'aarch64'}.get(platform.machine())
    version = platform.version() if system == 'windows' else platform.release()
    # Kernel/build only; never machine name, username, path or full platform().
    version = version.replace(' ', '-')
    if not system or not arch or not TOKEN.fullmatch(version):
        raise ValueError('unsupported or unbounded visual environment identity')
    return {'platform': system, 'os_version': version, 'architecture': arch,
            'display_server': 'controlled', 'gpu': 'software', 'driver': 'cpu-v1',
            'shell': 'fixture', 'shell_version': '1', 'locale': 'fixed-fixture'}


def build_test_binary() -> Path:
    """Cargo identifies its own harness; never guess a hash or reuse a glob hit."""
    pending = bytearray()
    binaries: set[Path] = set()
    def consume(chunk: bytes) -> None:
        pending.extend(chunk)
        while b'\n' in pending:
            line, _, rest = pending.partition(b'\n')
            pending[:] = rest
            if len(line) > MAX_OUTPUT:
                raise ValueError('Cargo record exceeded its bound')
            if not line.startswith(b'{'):
                continue
            record = json.loads(line)
            if (record.get('reason') == 'compiler-artifact' and record.get('target', {}).get('name') == 'automexia'
                    and record.get('profile', {}).get('test') and record.get('executable')):
                binaries.add(Path(record['executable']).resolve())
        if len(pending) > MAX_OUTPUT:
            raise ValueError('unterminated Cargo record exceeded its bound')
    result = qa_process.run(['cargo', 'test', '-p', 'automexia-terminal', '--bin', 'automexia',
                             '--features', 'visual-test-hooks', '--locked', '--no-run', '--message-format=json'],
                            cwd=ROOT, timeout_seconds=3600, consume=consume)
    if result.return_code != 0 or result.error or result.timed_out or len(binaries) != 1:
        raise RuntimeError('visual harness build failed or produced ambiguous artifacts')
    return binaries.pop()


def seal(facts_path: Path, source: str, dirty: bool, environment: dict[str, str]) -> tuple[str, str, Path]:
    facts = json.loads(read(facts_path, MAX_JSON))
    expected = {'schema', 'scenario', 'variant', 'fixture', 'evidence_kind', 'renderer', 'width', 'height',
                'scale_milli', 'font_size_milli', 'theme', 'image_sha256', 'frame_generation', 'font_hashes', 'geometry'}
    if not isinstance(facts, dict) or set(facts) != expected or type(facts['schema']) is not int or facts['schema'] != 1:
        raise ValueError('capture facts schema mismatch')
    scenario, variant = facts['scenario'], facts['variant']
    if (not isinstance(scenario, str) or not TOKEN.fullmatch(scenario) or not isinstance(variant, str)
            or variant not in MUTATIONS | {'correct', 'restored'}):
        raise ValueError('unknown visual scenario or mutation')
    filename = scenario if variant == 'correct' else f'{scenario}--{variant}'
    if facts_path.name != f'{filename}.capture.json':
        raise ValueError('visual scenario filename mismatch')
    if facts['evidence_kind'] != 'controlled-raster' or facts['renderer'] != 'cpu' or facts['fixture'] != 'visual-quality-v1':
        raise ValueError('controlled tests cannot claim native capture')
    image = facts_path.parent / f'{filename}.png'
    if hashlib.sha256(read(image, MAX_IMAGE)).hexdigest() != facts['image_sha256']:
        raise ValueError('capture image is stale or corrupt')
    identity = {key: facts[key] for key in ('scenario', 'fixture', 'evidence_kind', 'renderer', 'width', 'height',
                                         'scale_milli', 'font_size_milli', 'theme', 'font_hashes')}
    identity.update(environment)
    identity['configuration_sha256'] = hashlib.sha256(json.dumps(identity, sort_keys=True).encode()).hexdigest()
    metadata = {'schema': 1, 'source_commit': source, 'source_dirty': dirty, 'identity': identity,
                'image_sha256': facts['image_sha256'], 'frame_generation': facts['frame_generation'],
                'geometry': facts['geometry']}
    path = facts_path.parent / f'{filename}.json'
    atomic_json(path, metadata)
    return scenario, variant, path


def compare(xtask: Path, expected: Path, actual: Path, destination: Path, *, receipt: bool = False,
            expected_image: Path | None = None) -> dict:
    if destination.with_suffix('.png').exists() or destination.with_suffix('.json').exists():
        raise ValueError('comparison outputs must be fresh; stale evidence cannot pass')
    argv = ([str(xtask), 'visual-diff', '--compare-receipts', '--expected-metadata', str(expected),
             '--actual-metadata', str(actual), '--actual', str(actual.with_suffix('.png')),
             '--report', str(destination.with_suffix('.json'))] if receipt else
            [str(xtask), 'visual-diff', '--expected', str(expected_image or expected.with_suffix('.png')),
                       '--actual', str(actual.with_suffix('.png')), '--config', str(POLICY),
                       '--expected-metadata', str(expected), '--actual-metadata', str(actual),
                       '--diff', str(destination.with_suffix('.png')), '--report', str(destination.with_suffix('.json'))])
    code, _ = command(argv, 60)
    report_path = destination.with_suffix('.json')
    if not report_path.is_file():
        raise ValueError('visual comparison did not produce a compatible evidence report')
    report = json.loads(read(report_path, MAX_JSON))
    if report.get('schema') != (3 if receipt else 2) or report.get('status') not in ('passed', 'failed') or (code == 0) != (report['status'] == 'passed'):
        raise ValueError('visual command and report disagree')
    if not report.get('binding', {}).get('identity_verified'):
        raise ValueError('unbound visual evidence')
    return report


def load_receipts(path: Path, scenarios: set[str]) -> dict:
    """Inventory wrapper only; Rust remains the sole metadata/schema validator."""
    if not path.exists():
        return {}
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate baseline receipt field')
            result[key] = value
        return result
    value = json.loads(read(path, 2 * MAX_OUTPUT), object_pairs_hook=unique)
    if (not isinstance(value, dict) or set(value) != {'schema', 'captures'}
            or type(value['schema']) is not int or value['schema'] != 1
            or not isinstance(value['captures'], dict) or set(value['captures']) != scenarios):
        raise ValueError('baseline receipt inventory must match every correct scenario')
    for name, metadata in value['captures'].items():
        if (not TOKEN.fullmatch(name) or not isinstance(metadata, dict)
                or metadata.get('source_dirty') is not False
                or not isinstance(metadata.get('identity'), dict)
                or metadata.get('identity', {}).get('scenario') != name
                or len(json.dumps(metadata).encode()) > MAX_JSON):
            raise ValueError('baseline receipt must identify a clean reviewed capture')
    return value['captures']


def read_baseline_image(directory: Path, metadata: dict) -> bytes:
    digest = metadata.get('image_sha256')
    if (not isinstance(digest, str) or not re.fullmatch('[a-f0-9]{64}', digest)
            or directory.is_symlink() or not directory.is_dir()):
        raise ValueError('invalid reviewed image store or digest')
    image = directory / (digest + '.png')
    # The canonical Rust decoder also validates dimensions and metadata. Check
    # the stored bytes before preserving them as expected-image diagnostics.
    data = read(image, MAX_IMAGE)
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError('reviewed image store contains stale or corrupt bytes')
    return data


def validate_inventory(cases: list[tuple[str, str, Path]]) -> None:
    keys = [(name, variant) for name, variant, _ in cases]
    if len(set(keys)) != len(keys):
        raise ValueError('duplicate captures')
    themes = ('aurora-night', 'solar-dusk', 'forest-operator', 'arctic-glass', 'arctic-day')
    pages = ('header', 'footer', 'panes', 'background', 'window-controls', 'fonts', 'timestamps', 'tables',
             'tags', 'output', 'kubernetes', 'customizations', 'terminal-interface', 'theme-gallery', 'color-picker', 'font-picker')
    expected = {(f'terminal-{theme}-{scale}', 'correct') for theme in themes for scale in (100, 125, 150, 175, 200)}
    expected |= {(f'settings-{page}-{theme}-{scale}', 'correct') for page in pages for theme in themes for scale in (100, 125, 150, 175, 200)}
    expected |= {('glyph-mutations', variant) for variant in MUTATIONS | {'correct', 'restored'}}
    expected |= {(f'unicode-fallback-{scale}', 'correct') for scale in (100, 125, 150, 175, 200)}
    if set(keys) != expected:
        raise ValueError('missing or unexpected visual scenarios; zero-test/partial capture is not success')


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    binary_source = parser.add_mutually_exclusive_group(required=True)
    binary_source.add_argument('--test-binary', type=Path, help='prebuilt harness; caller owns source/build correspondence')
    binary_source.add_argument('--build', action='store_true', help='build and resolve the current source harness using Cargo')
    parser.add_argument('--xtask', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    baseline_source = parser.add_mutually_exclusive_group()
    baseline_source.add_argument('--baseline', type=Path)
    baseline_source.add_argument('--baseline-receipts', type=Path)
    parser.add_argument('--baseline-images', type=Path, help='reviewed PNGs named by digest; requires --baseline-receipts')
    parser.add_argument('--require-baseline', action='store_true')
    args = parser.parse_args(argv)
    if args.baseline_images and not args.baseline_receipts:
        parser.error('--baseline-images requires --baseline-receipts')
    output = args.output.resolve()
    # Fresh directory prevents a previous test/report from satisfying this run.
    output.mkdir(parents=True, exist_ok=False)
    captures = output / 'captures'
    captures.mkdir()
    diffs = output / 'diffs'
    diffs.mkdir()
    summary: dict = {'schema': 1, 'scope': 'controlled-raster', 'status': 'failed', 'native_certification': 'not-run', 'cases': []}
    try:
        summary['stage'] = 'source-identity'
        code, source = command(['git', 'rev-parse', 'HEAD'], 15)
        commit = source.decode('ascii').strip()
        if code or not re.fullmatch('[a-f0-9]{40,64}', commit):
            raise ValueError('source commit unavailable')
        code, dirty = command(['git', 'status', '--porcelain', '--untracked-files=normal'], 15)
        if code:
            raise ValueError('source status unavailable')
        summary.update(source_commit=commit, source_dirty=bool(dirty.strip()))
        fingerprint = source_fingerprint(commit)
        summary['source_fingerprint'] = fingerprint
        summary['stage'] = 'build-harness'
        binary = build_test_binary() if args.build else args.test_binary.resolve()
        summary['build_source'] = 'cargo-current-source' if args.build else 'caller-supplied-binary'
        summary['test_binary_sha256'] = file_digest(binary, 1024 * 1024 * 1024)
        env = dict(os.environ, AUTOMEXIA_VISUAL_CAPTURE_DIR=str(captures))
        summary['stage'] = 'capture'
        code, log = command([str(binary), 'visual_quality', '--test-threads=1'], 600, env)
        (output / 'controlled-tests.log').write_bytes(log)
        if code or not re.search(rb'test result: ok\. [1-9][0-9]* passed;', log):
            raise ValueError('controlled renderer tests failed or executed no tests')
        files = sorted(captures.glob('*.capture.json'))
        if not 1 <= len(files) <= MAX_CASES:
            raise ValueError('visual capture count is outside bounds')
        identity = environment_identity()
        summary['stage'] = 'seal-captures'
        cases = [seal(path, commit, bool(dirty.strip()), identity) for path in files]
        validate_inventory(cases)
        summary['environment'] = identity
        summary['stage'] = 'load-baseline'
        receipts = (load_receipts(args.baseline_receipts, {name for name, variant, _ in cases if variant == 'correct'})
                    if args.baseline_receipts else {})
        receipt_dir = output / 'expected-receipts'
        if receipts:
            receipt_dir.mkdir()
        expected_images = output / 'expected-images'
        if args.baseline_images:
            expected_images.mkdir()
        baseline_failed = False
        summary['stage'] = 'validate-and-compare'
        for name, variant, actual in cases:
            summary['active_scenario'] = name
            summary['active_variant'] = variant
            if variant != 'correct':
                expected = captures / f'{name}.json'
                report = compare(args.xtask.resolve(), expected, actual, diffs / f'{name}--{variant}')
                wanted = 'passed' if variant == 'restored' else 'failed'
                if report['status'] != wanted:
                    raise ValueError('mutation was not detected or restoration changed pixels')
                summary['cases'].append({'scenario': name, 'variant': variant, 'status': 'verified',
                                         'changed_pixels': report['changed_pixels']})
            else:
                expected = args.baseline.resolve() / actual.name if args.baseline else receipt_dir / actual.name
                if name in receipts:
                    atomic_json(expected, receipts[name])
                if (args.baseline or args.baseline_receipts) and expected.is_file():
                    image = None
                    if args.baseline_images:
                        # Keep expected pixels even if incompatible metadata
                        # prevents a meaningful pixel-difference statistic.
                        image = expected_images / (name + '.png')
                        image.write_bytes(read_baseline_image(args.baseline_images, receipts[name]))
                    report = compare(args.xtask.resolve(), expected, actual, diffs / name,
                                     receipt=bool(args.baseline_receipts and not args.baseline_images), expected_image=image)
                    summary['cases'].append({'scenario': name, 'status': report['status']})
                    baseline_failed |= report['status'] != 'passed'
                else:
                    # Comparison already validates its inputs. Decode separately
                    # only when no reviewed baseline is available for this case.
                    code, _ = command([str(args.xtask.resolve()), 'visual-diff', '--validate-metadata', str(actual),
                                       '--image', str(actual.with_suffix('.png'))], 60)
                    if code:
                        raise ValueError('capture rejected by the canonical metadata/image validator')
                    if args.baseline or args.baseline_receipts:
                        summary['cases'].append({'scenario': name, 'status': 'baseline-missing'})
                        baseline_failed = True
        summary['captures'] = len(cases)
        summary.pop('active_scenario', None)
        summary.pop('active_variant', None)
        summary['stage'] = 'verify-source-identity'
        code, final_commit = command(['git', 'rev-parse', 'HEAD'], 15)
        final_fingerprint = source_fingerprint(commit)
        summary['final_source_fingerprint'] = final_fingerprint
        if code or final_commit.decode('ascii').strip() != commit or final_fingerprint != fingerprint:
            raise ValueError('source changed during visual capture; evidence must be regenerated')
        has_baseline = bool(args.baseline or args.baseline_receipts)
        summary['baseline'] = 'failed-or-missing' if baseline_failed else ('passed' if has_baseline else 'unavailable')
        summary['status'] = 'passed' if has_baseline and not baseline_failed else 'captured-not-certified'
        if baseline_failed:
            raise ValueError('reviewed platform baseline is missing or changed')
        if args.require_baseline and not has_baseline:
            raise ValueError('reviewed platform baseline is required')
        summary['stage'] = 'complete'
    except (ValueError, OSError, RuntimeError, UnicodeError, json.JSONDecodeError) as error:
        # Do not publish paths, shell output or arbitrary decoder exception text.
        summary['status'] = 'failed'
        summary['error_kind'] = type(error).__name__
        if isinstance(error, CommandFailure):
            summary['command_kind'], summary['failure_reason'] = error.command_kind, error.reason
        atomic_json(output / 'summary.json', summary)
        print('FAIL: visual quality run; inspect bounded local evidence.', file=sys.stderr)
        return 1
    atomic_json(output / 'summary.json', summary)
    print(f"Visual captures: {summary['captures']}; baseline: {summary['baseline']}; native certification: not run")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
