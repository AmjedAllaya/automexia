"""Stage the pinned Microsoft editor beside Windows builds, never globally.

Downloads run in the existing bounded QA process owner. Installed applications
only read the staged module; they never fetch packages or edit user profiles.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
import time
import urllib.request
import urllib.parse
import zipfile

import dev_cache
import qa_process
from windows_conpty_runtime import _directory, _safe_path, _digest

ROOT = Path(__file__).resolve().parents[2]
RECIPE = ROOT / 'packaging/windows/powershell-editor.json'
MODULE_PATH = Path('powershell-modules/PSReadLine/2.4.5')
MAX_ARCHIVE = 2 * 1024 * 1024
MAX_EXPANDED = 4 * 1024 * 1024
DOWNLOAD_SECONDS = 120
FILES = frozenset({'PSReadLine.psd1', 'PSReadLine.psm1', 'PSReadLine.format.ps1xml',
    'Microsoft.PowerShell.PSReadLine.dll', 'Microsoft.PowerShell.Pager.dll',
    'netstd/Microsoft.PowerShell.PSReadLine.Polyfiller.dll',
    'net6plus/Microsoft.PowerShell.PSReadLine.Polyfiller.dll', 'License.txt'})
_quarantine: tuple[Path, object] | None = None


class EditorError(ValueError):
    """Bounded, content-free editor preparation error."""


def load_recipe() -> dict:
    if RECIPE.stat().st_size > 16 * 1024:
        raise EditorError('Editor recipe exceeds its bound')
    recipe = json.loads(RECIPE.read_text(encoding='utf-8'))
    if not isinstance(recipe, dict):
        raise EditorError('Editor recipe must be an object')
    package = recipe.get('package', {})
    files = recipe.get('files', [])
    if (not isinstance(package, dict) or not isinstance(files, list)
            or any(not isinstance(item, dict) for item in files)):
        raise EditorError('Editor recipe structure is invalid')
    if (recipe.get('schema_version') != 1 or package.get('id') != 'PSReadLine'
            or package.get('version') != '2.4.5'
            or package.get('url') != 'https://www.powershellgallery.com/api/v2/package/PSReadLine/2.4.5'
            or package.get('publisher') != 'Microsoft Corporation' or package.get('license') != 'MIT'
            or not re.fullmatch(r'[0-9a-f]{64}', str(package.get('sha256', '')))
            or len(files) != len(FILES) or {item.get('path') for item in files} != FILES):
        raise EditorError('Editor recipe identity or file allowlist is invalid')
    for item in files:
        if (type(item.get('size')) is not int or not 0 < item['size'] <= MAX_ARCHIVE
                or not re.fullmatch(r'[0-9a-f]{64}', str(item.get('sha256', '')))
                or item.get('signed') is not (item['path'] != 'License.txt')):
            raise EditorError('Editor recipe file contract is invalid')
    return recipe


def verify_runtime(destination: Path) -> tuple[Path, ...]:
    module = destination / MODULE_PATH
    files = load_recipe()['files']
    pending, actual, entries = [module], set(), 0
    while pending:
        directory = pending.pop()
        _safe_path(directory)
        for path in directory.iterdir():
            entries += 1
            if entries > 16:
                raise EditorError('Editor directory contains too many entries')
            _safe_path(path)
            if path.is_dir():
                pending.append(path)
            else:
                actual.add(path.relative_to(module).as_posix())
    if actual != FILES:
        raise EditorError('Editor directory differs from its file allowlist')
    for item in files:
        if _digest(module / item['path'], MAX_ARCHIVE) != (item['size'], item['sha256']):
            raise EditorError('Editor file differs from the pinned package')
    return tuple(module / item['path'] for item in files)


def extract_archive(archive: Path, destination: Path, recipe: dict) -> None:
    if _digest(archive, MAX_ARCHIVE)[1] != recipe['package']['sha256']:
        raise EditorError('Editor archive checksum differs from its pin')
    with zipfile.ZipFile(archive) as source:
        entries = source.infolist()
        seen = set()
        expanded = 0
        if len(entries) > 64:
            raise EditorError('Editor archive member count exceeds its bound')
        for entry in entries:
            name = entry.filename.rstrip('/')
            expanded += entry.file_size
            if (not name or name.startswith('/') or '\\' in name or ':' in name
                    or any(part in ('', '.', '..') for part in name.split('/'))
                    or name.casefold() in seen or stat.S_ISLNK(entry.external_attr >> 16)
                    or entry.flag_bits & 1 or entry.file_size > MAX_ARCHIVE or expanded > MAX_EXPANDED):
                raise EditorError('Editor archive contains an unsafe member')
            seen.add(name.casefold())
        for item in recipe['files']:
            entry = source.getinfo(item['path'])
            if entry.is_dir() or entry.file_size != item['size']:
                raise EditorError('Editor archive member size differs from its pin')
            target = destination / item['path']
            _directory(target.parent)
            with target.open('xb') as output:
                output.write(source.read(entry))
            if _digest(target, MAX_ARCHIVE) != (item['size'], item['sha256']):
                raise EditorError('Editor archive member checksum differs from its pin')


def download_worker(destination: Path) -> None:
    staging = Path(os.path.abspath(dev_cache.staging_root(root=ROOT)))
    destination = Path(os.path.abspath(destination))
    _safe_path(destination)
    if (destination.name != 'editor.nupkg' or destination.parent.parent != staging
            or not re.fullmatch(r'psreadline-[a-z0-9_]{8}', destination.parent.name)):
        raise EditorError('Editor download requires its owned staging directory')
    deadline = time.monotonic() + DOWNLOAD_SECONDS
    url = load_recipe()['package']['url']
    with urllib.request.urlopen(url, timeout=30) as response, destination.open('xb') as output:
        # PowerShell Gallery redirects package bytes to its Microsoft CDN.
        final = urllib.parse.urlsplit(response.geturl())
        if final.scheme != 'https' or final.hostname not in ('www.powershellgallery.com', 'cdn.powershellgallery.com'):
            raise EditorError('Editor download redirected outside the pinned publisher')
        total = 0
        while chunk := response.read(64 * 1024):
            total += len(chunk)
            if total > MAX_ARCHIVE or time.monotonic() > deadline:
                raise EditorError('Editor download exceeded its byte or time bound')
            output.write(chunk)


def prepare_runtime(destination: Path, *, offline=False, archive: Path | None = None) -> tuple[Path, ...]:
    global _quarantine
    if os.name != 'nt':
        raise EditorError('Editor installation requires a native Windows builder')
    if _quarantine is not None:
        raise EditorError('Editor download cleanup remains unresolved')
    recipe = load_recipe()
    destination = Path(os.path.abspath(destination))
    module = destination / MODULE_PATH
    _safe_path(module)
    if module.exists():
        return verify_runtime(destination)
    lease = dev_cache.cache_lease('windows-powershell-editor', root=ROOT)
    lease.__enter__()
    temporary = None
    try:
        cached = dev_cache.downloads_root(root=ROOT) / 'psreadline' / (recipe['package']['sha256'] + '.nupkg')
        _directory(cached.parent)
        staging = dev_cache.staging_root(root=ROOT)
        _directory(staging)
        temporary = Path(tempfile.mkdtemp(prefix='psreadline-', dir=staging))
        source = archive or cached
        if archive is None and not cached.exists():
            if offline:
                raise EditorError('Pinned editor is not cached; prepare online or supply --archive')
            source = temporary / 'editor.nupkg'
            result = qa_process.run([sys.executable, str(Path(__file__).resolve()), '--download-worker', str(source)],
                                    cwd=ROOT, timeout_seconds=DOWNLOAD_SECONDS, consume=lambda _: None)
            if qa_process._quarantine is not None:
                _quarantine = (temporary, lease)
                raise EditorError('Editor download cleanup remains unresolved')
            if result.return_code or result.timed_out or result.error:
                raise EditorError('Pinned editor download failed; retry or supply --archive')
        members = temporary / 'members'
        extract_archive(source, members, recipe)
        if not cached.exists():
            shutil.copyfile(source, temporary / 'verified.nupkg')
            os.replace(temporary / 'verified.nupkg', cached)
        # Stage on the destination volume, then publish the complete module in
        # one rename. Never replace a pre-existing or partially modified module.
        _directory(module.parent)
        with tempfile.TemporaryDirectory(prefix='.editor-stage-', dir=module.parent) as staged:
            candidate = Path(staged) / 'module'
            shutil.copytree(members, candidate)
            if module.exists():
                return verify_runtime(destination)
            os.rename(candidate, module)
        return verify_runtime(destination)
    finally:
        if _quarantine is None:
            if temporary is not None:
                shutil.rmtree(temporary)
            lease.__exit__(*sys.exc_info())


def main() -> int:
    if len(sys.argv) == 3 and sys.argv[1] == '--download-worker':
        download_worker(Path(sys.argv[2]))
        return 0
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('prepare', 'verify'))
    parser.add_argument('--destination', required=True, type=Path)
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    if args.action == 'prepare':
        prepare_runtime(args.destination, archive=args.archive, offline=args.offline)
    else:
        verify_runtime(args.destination)
    print('PASS: pinned Microsoft PowerShell editor verified')
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile):
        print('FAIL: pinned PowerShell editor could not be safely prepared or verified', file=sys.stderr)
        raise SystemExit(1)
