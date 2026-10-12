#!/usr/bin/env python3
"""Owner-authorized first alpha: bounded identity, brand and publication checks.

This contract does not relax stable_release.py or claim native OS signing.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import tomllib

from release_identity import CURRENT_ALPHA_TAG as ALPHA_TAG, CURRENT_ALPHA_VERSION as ALPHA_VERSION

ROOT = Path(__file__).resolve().parents[2]
REQUIRED_GATES = frozenset({
    'preflight', 'release-quality', 'native-platform-qa', 'verify-windows-final',
    'verify-macos-final', 'verify-linux-final', 'reproducibility-linux',
    'reproducibility-desktop',
})
TARGETS = frozenset({
    'x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc',
    'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu',
    'x86_64-apple-darwin', 'aarch64-apple-darwin',
})


def validate_identity(version: str, tag: str, commit: str) -> None:
    if version != ALPHA_VERSION or tag != ALPHA_TAG:
        raise ValueError('this workflow is restricted to the currently authorized alpha')
    if re.fullmatch(r'[0-9a-f]{40}', commit) is None:
        raise ValueError('a full immutable source commit is required')


def authorize(expected: str, actual: str, ref: str, acknowledgement: str) -> None:
    validate_identity(ALPHA_VERSION, ALPHA_TAG, expected)
    if actual != expected or ref != 'refs/heads/main':
        raise ValueError('publication must use the exact current main revision')
    if acknowledgement != 'unsigned-first-alpha':
        raise ValueError('explicit unsigned-first-alpha acknowledgement required')


def validate_results(results: dict) -> None:
    for name in REQUIRED_GATES:
        if not isinstance(results.get(name), dict) or results[name].get('result') != 'success':
            raise ValueError(f'required alpha gate did not pass: {name}')


def signing_disclosure() -> dict:
    return {'windows': 'unsigned', 'macos': 'ad-hoc; not notarized',
            'linux': 'unsigned packages', 'checksum_manifest': 'minisign'}


def verify_disclosure(manifest: dict) -> None:
    validate_identity(manifest['version'], manifest['tag'], manifest['source_commit'])
    if manifest.get('channel') != 'first-alpha' or manifest.get('os_signing') != signing_disclosure():
        raise ValueError('alpha signing disclosure is missing or misleading')
    if not isinstance(manifest.get('branding'), dict) or manifest['branding'].get('stable_approval') is not False:
        raise ValueError('alpha must not claim stable branding approval')


def _bytes(path: Path, maximum: int = 8 * 1024 * 1024) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError('release input must be a bounded regular file')
    data = path.read_bytes()
    if len(data) > maximum:
        raise ValueError('release input exceeded its byte limit')
    return data


def verify_brand(root: Path) -> dict:
    base = root / 'assets/brand'
    manifest = tomllib.loads(_bytes(base / 'ASSET-MANIFEST.toml', 64 * 1024).decode('utf-8'))
    source = _bytes(base / 'automexia-terminal-source-512.png')
    if hashlib.sha256(source).hexdigest() != manifest['source']['sha256']:
        raise ValueError('brand source digest mismatch')
    approval = manifest.get('first_alpha', {})
    if approval.get('version') != ALPHA_VERSION or approval.get('owner_authorized') is not True:
        raise ValueError('first-alpha brand authorization is missing')
    if approval.get('source_sha256') != manifest['source']['sha256']:
        raise ValueError('brand approval does not bind the source image')
    records = []
    for size in (16, 32, 48, 64, 128, 256, 512, 1024):
        path = base / 'png' / f'automexia-terminal-{size}.png'
        data = _bytes(path)
        if len(data) < 24 or data[:8] != b'\x89PNG\r\n\x1a\n' or data[12:16] != b'IHDR':
            raise ValueError('invalid platform PNG')
        if struct.unpack('>II', data[16:24]) != (size, size):
            raise ValueError('platform PNG dimensions do not match its name')
        records.append({'file': path.relative_to(root).as_posix(), 'sha256': hashlib.sha256(data).hexdigest()})
    ico = _bytes(base / 'automexia-terminal.ico')
    if len(ico) < 6 or ico[:4] != b'\0\0\1\0':
        raise ValueError('invalid Windows icon container')
    count = struct.unpack('<H', ico[4:6])[0]
    if not 1 <= count <= 64 or len(ico) < 6 + count * 16:
        raise ValueError('invalid Windows icon entries')
    sizes = set()
    for index in range(count):
        entry = ico[6 + index * 16:22 + index * 16]
        width, height = entry[0] or 256, entry[1] or 256
        length, offset = struct.unpack('<II', entry[8:16])
        if width != height or offset < 6 + count * 16 or not length or offset + length > len(ico):
            raise ValueError('invalid Windows icon bounds')
        sizes.add(width)
    if not {16, 32, 48, 256}.issubset(sizes):
        raise ValueError('Windows icon lacks required sizes')
    icns = _bytes(base / 'automexia-terminal.icns')
    if len(icns) < 8 or icns[:4] != b'icns' or struct.unpack('>I', icns[4:8])[0] != len(icns):
        raise ValueError('invalid macOS icon container')
    offset = 8
    while offset < len(icns):
        if offset + 8 > len(icns):
            raise ValueError('truncated macOS icon entry')
        length = struct.unpack('>I', icns[offset + 4:offset + 8])[0]
        if length < 8 or offset + length > len(icns):
            raise ValueError('invalid macOS icon bounds')
        offset += length
    for name, data in [('automexia-terminal.ico', ico), ('automexia-terminal.icns', icns)]:
        records.append({'file': 'assets/brand/' + name, 'sha256': hashlib.sha256(data).hexdigest()})
    return {'source_sha256': manifest['source']['sha256'], 'assets': records,
            'owner_authorized_alpha': True, 'stable_approval': False}


def validate_workflow(workflow: dict) -> None:
    jobs = workflow.get('jobs', {})
    if set(jobs.get('final-gate', {}).get('needs', [])) != REQUIRED_GATES:
        raise ValueError('alpha final gate must require every native and quality gate')
    targets = {item.get('target') for item in jobs['build']['strategy']['matrix']['include']}
    if targets != TARGETS:
        raise ValueError('alpha must build all six native targets')
    if 'publish-release' in jobs or workflow.get('permissions') != {'contents': 'read'}:
        raise ValueError('alpha CI prepares verified bundles; publication belongs to the owner CLI')
    if jobs['native-platform-qa'].get('uses') != './.github/workflows/accessibility-native.yml':
        raise ValueError('native QA must retain its existing owner')
    for job, command, condition in (
        ('verify-linux-final', 'tools/ci/test_packaged_shells.py', None),
        ('verify-linux-final', 'tools/ci/test_linux_distros.py', None),
        ('verify-macos-final', 'tools/ci/test_packaged_shells.py', None),
        ('verify-windows-final', 'tests/integration/windows-package-review.ps1', "matrix.arch == 'x86_64'"),
    ):
        steps = [step for step in jobs[job]['steps'] if command in step.get('run', '')]
        if len(steps) != 1 or steps[0].get('if') != condition or steps[0].get('continue-on-error', False):
            raise ValueError('alpha must exercise the shipped shell and settings resources')
    for name, dependencies in {
        'publish': {'preflight', 'package-windows', 'package-macos', 'package-linux', 'final-gate', 'reproducibility-linux'},
        'sign-release-bundle': {'publish', 'preflight'},
    }.items():
        if set(jobs[name].get('needs', [])) != dependencies:
            raise ValueError('publication/signing dependency chain drifted')
    for job in jobs.values():
        if job.get('permissions', {'contents': 'read'}) != {'contents': 'read'}:
            raise ValueError('alpha CI must never hold repository write or administration permissions')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['brand', 'authorize', 'gate', 'manifest', 'verify-manifest', 'check-workflow'])
    parser.add_argument('--commit')
    parser.add_argument('--main')
    parser.add_argument('--ref')
    parser.add_argument('--acknowledgement')
    parser.add_argument('--file', type=Path)
    args = parser.parse_args()
    if args.action == 'brand':
        print(json.dumps(verify_brand(ROOT), sort_keys=True))
    elif args.action == 'authorize':
        authorize(args.commit, args.main, args.ref, args.acknowledgement)
    elif args.action == 'gate':
        if args.file is None:
            parser.error('--file required')
        validate_results(json.loads(_bytes(args.file, 256 * 1024)))
    elif args.action == 'check-workflow':
        import yaml
        validate_workflow(yaml.safe_load(_bytes(ROOT / '.github/workflows/first-alpha.yml').decode('utf-8')))
    else:
        if args.file is None:
            parser.error('--file required')
        manifest = json.loads(_bytes(args.file, 4 * 1024 * 1024))
        validate_identity(manifest['version'], manifest['tag'], manifest['source_commit'])
        if args.action == 'verify-manifest':
            verify_disclosure(manifest)
            return
        manifest['channel'] = 'first-alpha'
        manifest['os_signing'] = signing_disclosure()
        manifest['branding'] = verify_brand(ROOT)
        manifest['certification'] = 'Native CI and package tests; not stable or long-duration certification'
        args.file.write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n', encoding='utf-8')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError) as error:
        raise SystemExit(f'First-alpha validation failed: {error}') from error
