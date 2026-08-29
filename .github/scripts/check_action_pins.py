#!/usr/bin/env python3
"""Require every external GitHub Action reference to use a full commit SHA."""
from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path('.github/workflows')
USE_RE = re.compile(r'^\s*(?:-\s*)?uses:\s*["\']?([^\s"\']+)', re.MULTILINE)
SHA_RE = re.compile(r'^[^@]+@[0-9a-fA-F]{40}$')

failures: list[str] = []
for path in sorted([*ROOT.glob('*.yml'), *ROOT.glob('*.yaml')]):
    text = path.read_text(encoding='utf-8')
    for match in USE_RE.finditer(text):
        ref = match.group(1).strip().strip('"\'')
        if ref.startswith('./'):
            continue
        if not SHA_RE.fullmatch(ref):
            line = text.count('\n', 0, match.start()) + 1
            failures.append(f'{path}:{line}: unpinned action reference: {ref}')

if failures:
    print('\n'.join(failures), file=sys.stderr)
    raise SystemExit(1)
print('All external GitHub Action references are pinned to full commit SHAs.')
