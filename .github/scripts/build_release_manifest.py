#!/usr/bin/env python3
"""Copy allowlisted release packages and generate a deterministic release manifest."""
from __future__ import annotations
from pathlib import Path
import argparse, hashlib, json, re, shutil, sys

PACKAGE_SUFFIXES = ('.msi', '.zip', '.dmg', '.deb', '.rpm', '.tar.gz', '.tgz')
EXTRA_SUFFIXES = ('.spdx.json', '.cdx.json')

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def is_allowed(name: str, include_sbom: bool) -> bool:
    return name.endswith(PACKAGE_SUFFIXES + (EXTRA_SUFFIXES if include_sbom else ()))

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument('--source', type=Path, required=True)
    ap.add_argument('--output', type=Path, required=True)
    ap.add_argument('--version', required=True)
    ap.add_argument('--commit', required=True)
    ap.add_argument('--copy-packages', action='store_true')
    ap.add_argument('--include-sbom', action='store_true')
    args = ap.parse_args()

    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', args.version):
        raise SystemExit('version must be stable SemVer X.Y.Z')
    if not re.fullmatch(r'[0-9a-f]{40}', args.commit):
        raise SystemExit('commit must be a lowercase 40-character Git SHA')

    args.output.mkdir(parents=True, exist_ok=True)
    if args.copy_packages:
        seen: dict[str, str] = {}
        files = sorted(p for p in args.source.rglob('*') if p.is_file() and is_allowed(p.name, False))
        if not files:
            raise SystemExit('no release package files found')
        for src in files:
            if any(c in src.name for c in '\r\n\0'):
                raise SystemExit(f'unsafe release filename: {src.name!r}')
            sha = digest(src)
            existing = seen.get(src.name)
            if existing and existing != sha:
                raise SystemExit(f'conflicting duplicate release filename: {src.name}')
            seen[src.name] = sha
            dest = args.output / src.name
            if dest.exists() and digest(dest) != sha:
                raise SystemExit(f'conflicting destination release filename: {src.name}')
            shutil.copy2(src, dest)

    files = sorted(p for p in args.output.iterdir() if p.is_file() and is_allowed(p.name, args.include_sbom))
    names = [p.name for p in files]
    version = re.escape(args.version)
    requirements = {
        'Windows x64 MSI': rf'^{re.escape("automexia-terminal-")}{version}-x86_64-pc-windows-msvc\.msi$',
        'Windows x64 ZIP': rf'^{re.escape("automexia-terminal-")}{version}-x86_64-pc-windows-msvc\.zip$',
        'Windows ARM64 MSI': rf'^{re.escape("automexia-terminal-")}{version}-aarch64-pc-windows-msvc\.msi$',
        'Windows ARM64 ZIP': rf'^{re.escape("automexia-terminal-")}{version}-aarch64-pc-windows-msvc\.zip$',
        'macOS Universal DMG': rf'^automexia-terminal-{version}-universal\.dmg$',
        'Linux x64 DEB': rf'.*{version}.*(?:x86_64|amd64).*\.deb$',
        'Linux x64 RPM': rf'.*{version}.*(?:x86_64|amd64).*\.rpm$',
        'Linux x64 tarball': rf'.*{version}.*(?:x86_64-unknown-linux-gnu|x86_64|amd64).*\.(?:tar\.gz|tgz)$',
        'Linux ARM64 DEB': rf'.*{version}.*(?:aarch64|arm64).*\.deb$',
        'Linux ARM64 RPM': rf'.*{version}.*(?:aarch64|arm64).*\.rpm$',
        'Linux ARM64 tarball': rf'.*{version}.*(?:aarch64-unknown-linux-gnu|aarch64|arm64).*\.(?:tar\.gz|tgz)$',
    }
    missing = [label for label, pat in requirements.items() if not any(re.fullmatch(pat, n, re.I) for n in names)]
    if missing:
        print('Release package allowlist is incomplete:', file=sys.stderr)
        for label in missing: print(f'  - {label}', file=sys.stderr)
        print('Observed:', *names, sep='\n  ', file=sys.stderr)
        return 1

    manifest = {
        'schema': 1,
        'project': 'automexia-terminal',
        'version': args.version,
        'tag': f'v{args.version}',
        'source_commit': args.commit,
        'artifacts': [
            {'file': p.name, 'sha256': digest(p), 'size': p.stat().st_size}
            for p in files
        ],
    }
    (args.output/'release-manifest.json').write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    print(f'Validated {len(files)} release files and wrote release-manifest.json')
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
