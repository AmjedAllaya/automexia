#!/usr/bin/env python3
"""Validate the invariants of the GitHub-Free/private production CI contract."""
from __future__ import annotations
from pathlib import Path
import json
import re
import sys

root = Path('.github')
wf = root / 'workflows'
errors: list[str] = []

EXPECTED_WORKFLOWS = {
    'ci.yml',
    'f5-openssh-assurance.yml',
    'nightly.yml',
    'release.yml',
    's1-assurance.yml',
    's2-assurance.yml',
}
actual_workflows = {p.name for p in wf.iterdir() if p.is_file() and p.suffix in {'.yml', '.yaml'}}
if actual_workflows != EXPECTED_WORKFLOWS:
    errors.append(
        'workflow inventory drift: expected ' + repr(sorted(EXPECTED_WORKFLOWS)) +
        ', found ' + repr(sorted(actual_workflows))
    )

for forbidden in ('codeql.yml', 'codeql.yaml', 'release-drafter.yml', 'release-drafter.yaml', 'workflow-security.yml', 'workflow-security.yaml'):
    if (wf / forbidden).exists():
        errors.append(f'forbidden/stale workflow exists: .github/workflows/{forbidden}')

all_text = '\n'.join(p.read_text(encoding='utf-8') for p in sorted([*wf.glob('*.yml'), *wf.glob('*.yaml')]))
for needle, reason in [
    ('actions/attest@', 'private GitHub artifact attestations are paid-only'),
    ('actions/dependency-review-action@', 'private dependency review is paid-only'),
]:
    if needle in all_text:
        errors.append(f'{reason}: found {needle!r}')
if re.search(r'^\s*environment\s*:', all_text, re.MULTILINE):
    errors.append('private GitHub environments are unavailable on the Free/private edition')

release = (wf/'release.yml').read_text(encoding='utf-8')
required_release_fragments = [
    'types:', '- closed', "startsWith(github.event.pull_request.head.ref, 'release/')",
    'github.event.pull_request.head.repo.full_name == github.repository',
    "github.event.pull_request.merged == true", 'contents: write',
    'stable-release-${{ github.repository }}',
]
for fragment in required_release_fragments:
    if fragment not in release:
        errors.append(f'release workflow is missing required fragment: {fragment}')


if 'run-name: "Release gate · PR #' not in release:
    errors.append('release workflow must declare a deterministic human-readable run-name')
if re.search(r'^    name:.*\$\{\{\s*matrix\.', release, re.MULTILINE):
    errors.append('release job display names must not expose raw matrix expressions in skipped runs')
if not re.search(
    r"release-final-gate:\n(?:.*\n){0,18}?\s*- authorize\n(?:.*\n){0,18}?\s*if: \$\{\{ always\(\) && needs\.authorize\.result == 'success' \}\}",
    release,
    re.MULTILINE,
):
    errors.append('release-final-gate must depend on authorize and skip non-release PRs cleanly')

if release.count('contents: write') != 1:
    errors.append(f'release workflow must contain exactly one contents: write grant; found {release.count("contents: write")}')

if not re.search(r'^  publish:\n(?:.*\n){0,15}?    - reproducibility-linux$', release, re.MULTILINE):
    errors.append('release publication preparation must directly depend on reproducibility-linux')

ci = (wf/'ci.yml').read_text(encoding='utf-8')
for fragment in (
    'tools/ci/github_free_assurance.py check-policy',
    'tools/ci/validate_repository.py',
    "python3 -m unittest discover -s tools/ci -p 'test_*.py'",
    'PyYAML==6.0.3',
    'semgrep==1.175.0',
    'semgrep scan --config tools/ci/semgrep-rules.yml',
    'cargo-vet@0.10.2',
    'cargo vet --locked',
    "GITLEAKS_VERSION: '8.30.1'",
    "GITLEAKS_SHA256: '551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb'",
    'gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz',
    'Scan introduced commits and working tree for secrets',
    'log_opts="${BASE_SHA}..${HEAD_SHA}"',
    'gitleaks git --redact --no-banner --timeout=900 --max-target-megabytes=16 --config .gitleaks.toml --log-opts="$log_opts"',
    'gitleaks dir --redact --no-banner --timeout=900 --max-target-megabytes=16 --config .gitleaks.toml .',
):
    if fragment not in ci:
        errors.append(f'CI is missing required GitHub-Free local assurance fragment: {fragment}')
if '--log-opts=--all' in ci:
    errors.append('ordinary CI must not rescan unresolved legacy history; use the explicit local history-audit command')
dependency_job = re.search(
    r'(?ms)^  dependency-security:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)',
    ci,
)
if dependency_job is None or 'fetch-depth: 0' not in dependency_job.group('body'):
    errors.append('dependency-security must fetch complete history for its validated introduced-commit range')
quality_job = re.search(
    r'(?ms)^  quality:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)',
    ci,
)
if quality_job is None or 'fetch-depth: 0' in quality_job.group('body'):
    errors.append('quality must keep the economical shallow checkout')
# Ordinary PR CI stays on Linux. The sole standard-hosted Windows exception is
# release-only coverage because the recorded non-regression baseline is MSVC.
ci_jobs = {
    match.group('name'): match.group('body')
    for match in re.finditer(
        r'(?ms)^  (?P<name>[A-Za-z0-9_-]+):\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)',
        ci,
    )
}
for job_name, body in ci_jobs.items():
    runner = re.search(r'^    runs-on:\s*((?:windows|macos)-[^\s#]+)', body, re.MULTILINE)
    if runner and not (job_name == 'release-candidate-coverage' and runner.group(1) == 'windows-2025'):
        errors.append(
            'only release-candidate-coverage may use the standard hosted Windows runner; '
            f'{job_name} uses {runner.group(1)}'
        )

nightly = (wf/'nightly.yml').read_text(encoding='utf-8')
if re.search(r'^\s*schedule:\s*$', nightly, re.MULTILINE):
    errors.append('deep/nightly assurance must be manual-only on the Free/private edition')

contract = json.loads((root/'repository-protection.json').read_text(encoding='utf-8'))
if contract.get('mode') != 'github-free-private':
    errors.append('repository-protection.json mode must be github-free-private')
if contract.get('release', {}).get('trigger') != 'merged internal release/X.Y.Z pull request into main':
    errors.append('repository-protection.json release trigger contract drifted')

if errors:
    print('\n'.join(f'ERROR: {e}' for e in errors), file=sys.stderr)
    raise SystemExit(1)
print('GitHub-Free/private CI contract passed.')
