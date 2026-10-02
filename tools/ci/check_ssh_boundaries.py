#!/usr/bin/env python3
"""Static SSH planning-boundary checks; no builds, network, or command execution.

Called by existing repository/xtask validators. These are reviewed drift
sentinels, NOT a Rust parser, a hostile-code sandbox, or proof of runtime safety.
Cargo metadata in xtask independently validates resolved dependency declarations.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = 'automexia-ssh-integration'
PROTOCOL_PACKAGE = 'automexia-terminal-protocol'
MAX_SOURCE_FILES = 64
MAX_SOURCE_BYTES = 1024 * 1024


def ordinary(path: Path) -> None:
    if path.is_symlink() or getattr(path, 'is_junction', lambda: False)():
        raise ValueError('SSH source must not be a linked/reparse path')


def text(path: Path) -> str:
    ordinary(path)
    if not path.is_file() or path.stat().st_size > MAX_SOURCE_BYTES:
        raise ValueError('missing or oversized SSH boundary input: ' + path.name)
    return path.read_text(encoding='utf-8')


def source_files(directory: Path) -> list[Path]:
    ordinary(directory)
    files = []
    size = 0
    def fail(error: OSError) -> None:
        raise ValueError('cannot traverse SSH source') from error
    for base, dirs, names in os.walk(directory, followlinks=False, onerror=fail):
        for name in dirs:
            ordinary(Path(base) / name)
        for name in names:
            p = Path(base) / name
            ordinary(p)
            if p.suffix == '.rs':
                files.append(p)
                size += p.stat().st_size
                if len(files) > MAX_SOURCE_FILES or size > MAX_SOURCE_BYTES:
                    raise ValueError('SSH source review budget exceeded')
    if not files:
        raise ValueError('SSH production sources missing')
    return sorted(files)


def validate_manifest(manifest: dict) -> None:
    package = manifest.get('package', {})
    if package.get('name') != PACKAGE or package.get('publish') is not False:
        raise ValueError('SSH planning crate must remain private')
    if package.get('build') is not False:
        raise ValueError('automatic build scripts must be explicitly disabled')
    allowed = {'automexia-connectivity', 'automexia-terminal-protocol', 'base64'}
    if set(manifest.get('dependencies', {})) != allowed:
        raise ValueError('unexpected normal SSH dependency')
    for name in allowed:
        if manifest['dependencies'][name] != {'workspace': True}:
            raise ValueError('SSH dependency must use the reviewed workspace declaration')
    if manifest.get('dev-dependencies', {}) != {'proptest': {'workspace': True}}:
        raise ValueError('unexpected development SSH dependency')
    if manifest.get('build-dependencies'):
        raise ValueError('SSH build dependencies are forbidden')
    # Target-specific dependencies of ALL kinds were missed by the old checker.
    if manifest.get('target'):
        raise ValueError('SSH target-specific tables require a new boundary review')
    if manifest.get('features') or manifest.get('patch') or manifest.get('replace'):
        raise ValueError('SSH feature/override expansion requires review')
    if manifest.get('bin') or manifest.get('lib', {}).get('proc-macro'):
        raise ValueError('SSH planning crate cannot become an executable or proc macro')
    kind = manifest.get('lib', {}).get('crate-type', ['lib'])
    if kind not in (['lib'], ['rlib']):
        raise ValueError('SSH planning crate must be statically linked')


def validate_protocol_repository(root: Path) -> int:
    """Keep the shared wire decoder incapable of acquiring execution authority."""
    crate = root / PROTOCOL_PACKAGE
    ordinary(crate)
    manifest = tomllib.loads(text(crate / 'Cargo.toml'))
    package = manifest.get('package', {})
    if package.get('name') != PROTOCOL_PACKAGE or package.get('publish') is not False or package.get('build') is not False:
        raise ValueError('terminal protocol must remain private with build scripts disabled')
    for table in ('dependencies', 'dev-dependencies', 'build-dependencies', 'target', 'features', 'patch', 'replace', 'bin', 'example', 'bench'):
        if manifest.get(table):
            raise ValueError('terminal protocol must remain dependency-free and capability-free')
    library = manifest.get('lib', {})
    if library.get('path', 'src/lib.rs') != 'src/lib.rs' or library.get('proc-macro') or library.get('crate-type', ['lib']) not in (['lib'], ['rlib']):
        raise ValueError('terminal protocol library target changed')
    for path in (crate / 'build.rs', crate / 'src/main.rs', crate / 'src/bin', crate / 'examples', crate / 'benches'):
        if path.exists() or path.is_symlink():
            raise ValueError('unexpected terminal protocol executable/build-script source')
    files = source_files(crate / 'src')
    lib = text(crate / 'src/lib.rs')
    if '#![no_std]' not in lib or '#![forbid(unsafe_code)]' not in lib:
        raise ValueError('terminal protocol must forbid unsafe code and remain no_std')
    forbidden = re.compile(r'\b(?:std|alloc)\s*::|\bextern\s+(?:"|crate\b)|\binclude(?:_str|_bytes)?\s*!|#\s*\[\s*path\s*=')
    for path in files:
        if forbidden.search(text(path)):
            raise ValueError('terminal protocol effect/code-inclusion sentinel changed')
    return len(files)


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    root = root.resolve()
    protocol_files = validate_protocol_repository(root)
    crate = root / PACKAGE
    ordinary(crate)
    manifest = tomllib.loads(text(crate / 'Cargo.toml'))
    validate_manifest(manifest)
    # No alternate source root, build script, or production binary escape hatch.
    if manifest.get('lib', {}).get('path', 'src/lib.rs') != 'src/lib.rs':
        raise ValueError('SSH library source root changed')
    for p in [crate / 'build.rs', crate / 'src/main.rs', crate / 'src/bin']:
        if p.exists() or p.is_symlink():
            raise ValueError('unexpected executable/build-script source')
    files = source_files(crate / 'src')
    forbidden = re.compile(r'\b(?:std|tokio|async_std)\s*::\s*(?:process|fs|net|env|thread|time|os|io)\b|\bextern\s+"|\bextern\s+crate\b|\b(?:include|include_bytes)\s*!|#\s*\[\s*path\s*=|\bstd\s*::\s*\{')
    core_resources = {
        '"../resources/bash-core.bash"', '"../resources/zsh-core.zsh"',
        '"../resources/fish-core.fish"', '"../resources/powershell-core.ps1"',
    }
    upload_resources = {
        '"../../resources/upload-posix.sh.in"',
        '"../../resources/upload-powershell.ps1.in"',
    }
    resources = {('bootstrap.rs', resource) for resource in core_resources}
    resources.update(('bootstrap/upload.rs', resource) for resource in upload_resources)
    included = []
    for p in files:
        s = text(p)
        if forbidden.search(s):
            raise ValueError('unexpected effect/code-inclusion sentinel in ' + str(p.relative_to(root)))
        for include in re.findall(r'include_str!\s*\((.*?)\)', s, re.S):
            owner = (p.relative_to(crate / 'src').as_posix(), include.strip())
            if owner not in resources:
                raise ValueError('unexpected embedded SSH source owner')
            included.append(owner)
    if len(included) != len(resources) or set(included) != resources:
        raise ValueError('exactly one canonical resource per reviewed shell or upload adapter is required')
    lib = text(crate / 'src/lib.rs')
    if '#![forbid(unsafe_code)]' not in lib:
        raise ValueError('SSH library unsafe prohibition is missing')
    bootstrap = text(crate / 'src/bootstrap.rs')
    if 'STANDARD.encode' not in bootstrap or re.search(r'fn\s+base64\s*\(', bootstrap):
        raise ValueError('use the existing Base64 dependency, not a copied encoder')
    if re.search(r'format!\s*\(\s*"sh -c', bootstrap):
        raise ValueError('redundant bootstrap shell layer returned')
    if (crate / 'src/reviewed.rs').exists():
        raise ValueError('redundant reviewed-preparation forwarding module remains')
    gate = text(root / 'apps/automexia-terminal/src/context/launch_broker.rs')
    if not re.search(r'pub const MANAGED_SESSION_LAUNCH_ENABLED:\s*bool\s*=\s*false\s*;', gate) or 'assert!(!MANAGED_SESSION_LAUNCH_ENABLED)' not in gate:
        raise ValueError('protected managed launch denial changed')
    adapter = text(root / 'apps/automexia-terminal/src/automexia/ssh_integration.rs')
    if '"enhanced_execution_enabled": false' not in adapter or 'assess_reviewed' in adapter:
        raise ValueError('application preview boundary changed')
    direct = text(root / 'apps/automexia-terminal/src/automexia/connections/direct_openssh.rs')
    if 'ssh_integration::assess_reviewed' in direct:
        raise ValueError('native review path still detours through a duplicate validator')
    for resource in core_resources:
        core = text(crate / resource.strip('"').removeprefix('../'))
        # PowerShell's [char]7 is BEL, not an OSC 7 introducer.
        if re.search(r'(?<!char)\]7;', core, re.I) or 'file://remote' in core:
            raise ValueError('remote CWD must not enter the local OSC 7 path')
        if 'automexia_ssh_cwd' not in core or 'AMXSSHCWD1|' not in core:
            raise ValueError('scoped remote directory envelope missing')
    for resource in upload_resources:
        text(crate / resource.strip('"').removeprefix('../../'))
    # Wiring is an invariant, not a separate hand-maintained success claim.
    xtask = text(root / 'tools/xtask/src/main.rs')
    if '"automexia-ssh-integration"' not in xtask or 'run_python("tools/ci/check_ssh_boundaries.py")?;' not in xtask:
        raise ValueError('canonical xtask registration is absent')
    repository = text(root / 'tools/ci/validate_repository.py')
    if 'validate_ssh_boundaries()' not in repository:
        raise ValueError('repository validator registration is absent')
    reinforcement = json.loads(text(root / 'tests/assurance/feature-test-reinforcement-v1.json'))
    features = [f for f in reinforcement['features'] if f.get('id') == 'extension-contract-runtime']
    if len(features) != 1 or not any('SSH planning' in s for s in features[0]['needed_tests']):
        raise ValueError('SSH scenario reinforcement is absent')
    return {'source_files': len(files), 'runtime_dependencies': 3, 'protocol_source_files': protocol_files, 'protocol_runtime_dependencies': 0, 'library_live_execution': 0}


def main() -> int:
    try:
        result = validate_repository()
    except (OSError, ValueError, KeyError) as error:
        print('SSH boundary validation failed: ' + str(error), file=sys.stderr)
        return 1
    print('PASS: SSH planning boundaries ' + json.dumps(result, sort_keys=True))
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
