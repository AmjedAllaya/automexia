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
# Ordinary PR CI must stay on Linux so Windows/macOS minutes are reserved for actual releases.
if re.search(r'^\s*runs-on:\s*(?:windows|macos)-', ci, re.MULTILINE):
    errors.append('ordinary CI must not consume Windows/macOS hosted runners; those belong to Stable release')

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
