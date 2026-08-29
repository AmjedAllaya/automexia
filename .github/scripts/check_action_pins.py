#!/usr/bin/env python3
"""Enforce the external GitHub Action trust policy.

Every external action must be from the reviewed repository allowlist and pinned to
an immutable full 40-character commit SHA. Local actions remain allowed.
"""
from __future__ import annotations
from pathlib import Path
import re
import sys

WORKFLOWS = Path('.github/workflows')
USE_RE = re.compile(r'^\s*(?:-\s*)?uses:\s*(?:\n\s*)?["\']?([^\s"\']+)', re.MULTILINE)
FULL_SHA = re.compile(r'^(?P<action>[^@]+)@(?P<sha>[0-9a-fA-F]{40})$')
ALLOWED_ACTIONS = {
    'actions/cache',
    'actions/checkout',
    'actions/download-artifact',
    'actions/upload-artifact',
    'anchore/sbom-action',
    'azure/artifact-signing-action',
    'azure/login',
    'taiki-e/install-action',
    'zizmorcore/zizmor-action',
}

failures: list[str] = []
for path in sorted([*WORKFLOWS.glob('*.yml'), *WORKFLOWS.glob('*.yaml')]):
    text = path.read_text(encoding='utf-8')
    for match in USE_RE.finditer(text):
        ref = match.group(1).strip().strip('"\'')
        if ref.startswith('./'):
            continue
        line = text.count('\n', 0, match.start()) + 1
        if ref.startswith('docker://'):
            failures.append(f'{path}:{line}: docker:// actions are not allowed by the production policy: {ref}')
            continue
        parsed = FULL_SHA.fullmatch(ref)
        if not parsed:
            failures.append(f'{path}:{line}: external action is not pinned to a full SHA: {ref}')
            continue
        action = parsed.group('action')
        if action not in ALLOWED_ACTIONS:
            failures.append(f'{path}:{line}: external action repository is not allowlisted: {action}')

if failures:
    print('\n'.join(failures), file=sys.stderr)
    raise SystemExit(1)
print('Action trust policy passed: every external Action is allowlisted and pinned to a full 40-character commit SHA.')
