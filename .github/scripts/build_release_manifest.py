#!/usr/bin/env python3
"""Copy an exact, bounded release package set and write its deterministic manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys


PACKAGE_SUFFIXES = (".msi", ".zip", ".dmg", ".deb", ".rpm", ".tar.gz", ".tgz")
SBOM_NAMES = frozenset(
    {"automexia-terminal.spdx.json", "automexia-terminal.cdx.json"}
)
MAX_ARTIFACTS = 32
MAX_ARTIFACT_BYTES = 512 * 1024 * 1024
MAX_TOTAL_BYTES = 2 * 1024 * 1024 * 1024
MAX_MANIFEST_BYTES = 4 * 1024 * 1024


class ReleaseManifestError(ValueError):
    """Raised when release inputs are ambiguous, unsafe, or incomplete."""


def digest(path: Path, *, maximum: int = MAX_ARTIFACT_BYTES) -> tuple[str, int]:
    metadata = _regular_unlinked(path, "release artifact", maximum)
    hasher = hashlib.sha256()
    bytes_read = 0
    with path.open("rb") as source_file:
        for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
            bytes_read += len(chunk)
            if bytes_read > maximum:
                raise ReleaseManifestError("release artifact grew beyond its byte limit")
            hasher.update(chunk)
    if bytes_read != metadata.st_size:
        raise ReleaseManifestError("release artifact changed while it was hashed")
    return hasher.hexdigest(), bytes_read


def _regular_unlinked(path: Path, label: str, maximum: int) -> os.stat_result:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise ReleaseManifestError(f"{label} is missing") from error
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise ReleaseManifestError(f"{label} must be a regular unlinked file")
    if metadata.st_size > maximum:
        raise ReleaseManifestError(f"{label} exceeds its {maximum}-byte limit")
    return metadata


def _artifact_patterns(version: str) -> dict[str, re.Pattern[str]]:
    escaped = re.escape(version)
    return {
        "Windows x64 MSI": re.compile(
            rf"^automexia-terminal-{escaped}-x86_64-pc-windows-msvc\.msi$"
        ),
        "Windows x64 ZIP": re.compile(
            rf"^automexia-terminal-{escaped}-x86_64-pc-windows-msvc\.zip$"
        ),
        "Windows ARM64 MSI": re.compile(
            rf"^automexia-terminal-{escaped}-aarch64-pc-windows-msvc\.msi$"
        ),
        "Windows ARM64 ZIP": re.compile(
            rf"^automexia-terminal-{escaped}-aarch64-pc-windows-msvc\.zip$"
        ),
        "macOS Universal DMG": re.compile(
            rf"^automexia-terminal-{escaped}-universal\.dmg$"
        ),
        "Linux x64 DEB": re.compile(
            rf"^.*{escaped}.*(?:x86_64|amd64).*\.deb$", re.IGNORECASE
        ),
        "Linux x64 RPM": re.compile(
            rf"^.*{escaped}.*(?:x86_64|amd64).*\.rpm$", re.IGNORECASE
        ),
        "Linux x64 tarball": re.compile(
            rf"^.*{escaped}.*(?:x86_64-unknown-linux-gnu|x86_64|amd64).*\.(?:tar\.gz|tgz)$",
            re.IGNORECASE,
        ),
        "Linux ARM64 DEB": re.compile(
            rf"^.*{escaped}.*(?:aarch64|arm64).*\.deb$", re.IGNORECASE
        ),
        "Linux ARM64 RPM": re.compile(
            rf"^.*{escaped}.*(?:aarch64|arm64).*\.rpm$", re.IGNORECASE
        ),
        "Linux ARM64 tarball": re.compile(
            rf"^.*{escaped}.*(?:aarch64-unknown-linux-gnu|aarch64|arm64).*\.(?:tar\.gz|tgz)$",
            re.IGNORECASE,
        ),
    }


def _validate_exact_packages(paths: list[Path], version: str) -> None:
    patterns = _artifact_patterns(version)
    slots: dict[str, list[str]] = {label: [] for label in patterns}
    unexpected: list[str] = []
    for path in paths:
        matches = [label for label, pattern in patterns.items() if pattern.fullmatch(path.name)]
        if len(matches) != 1:
            unexpected.append(path.name)
            continue
        slots[matches[0]].append(path.name)
    incomplete = [label for label, names in slots.items() if not names]
    duplicate = [label for label, names in slots.items() if len(names) != 1]
    if incomplete:
        raise ReleaseManifestError(
            "release package allowlist is incomplete: " + ", ".join(incomplete)
        )
    if duplicate:
        raise ReleaseManifestError(
            "every release package slot must contain exactly one artifact: "
            + ", ".join(duplicate)
        )
    if unexpected:
        raise ReleaseManifestError(
            "unexpected release package artifacts: " + ", ".join(sorted(unexpected))
        )


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ReleaseManifestError(f"duplicate SBOM key: {key}")
        result[key] = value
    return result


def _validate_sboms(paths: list[Path]) -> None:
    names = {path.name for path in paths}
    if names != SBOM_NAMES:
        raise ReleaseManifestError(
            "SBOM allowlist must contain exactly " + ", ".join(sorted(SBOM_NAMES))
        )
    for path in paths:
        try:
            document = json.loads(
                path.read_text(encoding="utf-8"), object_pairs_hook=_reject_duplicate_keys
            )
        except json.JSONDecodeError as error:
            raise ReleaseManifestError(f"SBOM is not valid JSON: {path.name}") from error
        if not isinstance(document, dict):
            raise ReleaseManifestError(f"SBOM must be a JSON object: {path.name}")
        if path.name.endswith(".spdx.json") and not str(
            document.get("spdxVersion", "")
        ).startswith("SPDX-"):
            raise ReleaseManifestError("SPDX SBOM identity is missing")
        if path.name.endswith(".cdx.json") and document.get("bomFormat") != "CycloneDX":
            raise ReleaseManifestError("CycloneDX SBOM identity is missing")


def _scan_artifacts(
    root: Path,
    *,
    recursive: bool,
    include_sbom: bool,
    max_artifacts: int,
    max_artifact_bytes: int,
    max_total_bytes: int,
) -> tuple[list[Path], dict[Path, tuple[str, int]]]:
    iterator = root.rglob("*") if recursive else root.iterdir()
    candidates = sorted(
        path
        for path in iterator
        if any(path.name.endswith(suffix) for suffix in PACKAGE_SUFFIXES)
        or (include_sbom and path.name.endswith((".spdx.json", ".cdx.json")))
    )
    if len(candidates) > max_artifacts:
        raise ReleaseManifestError("release artifact count exceeds its limit")
    if not candidates:
        raise ReleaseManifestError("no release package files found")
    records: dict[Path, tuple[str, int]] = {}
    total = 0
    for path in candidates:
        if any(ord(character) < 32 for character in path.name):
            raise ReleaseManifestError("release filename contains control data")
        record = digest(path, maximum=max_artifact_bytes)
        total += record[1]
        if total > max_total_bytes:
            raise ReleaseManifestError("release artifact total byte limit exceeded")
        records[path] = record
    packages = [
        path
        for path in candidates
        if any(path.name.endswith(suffix) for suffix in PACKAGE_SUFFIXES)
    ]
    if include_sbom:
        _validate_sboms([path for path in candidates if path.name not in {p.name for p in packages}])
    return candidates, records


def _prepare_directory(path: Path, label: str) -> None:
    if path.exists() or path.is_symlink():
        if path.is_symlink() or not path.is_dir():
            raise ReleaseManifestError(f"{label} must be a real directory")
    else:
        path.mkdir(parents=True)


def _atomic_write(path: Path, payload: str) -> None:
    if path.exists() or path.is_symlink():
        _regular_unlinked(path, "release manifest", MAX_MANIFEST_BYTES)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    if temporary.exists() or temporary.is_symlink():
        raise ReleaseManifestError("release manifest temporary path already exists")
    try:
        temporary.write_text(payload, encoding="utf-8", newline="\n")
        os.replace(temporary, path)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def build_release_manifest(
    source: Path,
    output: Path,
    version: str,
    commit: str,
    *,
    copy_packages: bool = False,
    include_sbom: bool = False,
    max_artifacts: int = MAX_ARTIFACTS,
    max_artifact_bytes: int = MAX_ARTIFACT_BYTES,
    max_total_bytes: int = MAX_TOTAL_BYTES,
) -> dict[str, object]:
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ReleaseManifestError("version must be stable SemVer X.Y.Z")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ReleaseManifestError("commit must be a lowercase 40-character Git SHA")
    if include_sbom and copy_packages:
        raise ReleaseManifestError("package copying and SBOM finalization are separate stages")
    if source.is_symlink() or not source.is_dir():
        raise ReleaseManifestError("release source must be a real directory")
    source = source.resolve()
    output_resolved = output.resolve(strict=False)
    if copy_packages:
        if output_resolved == source or source in output_resolved.parents:
            raise ReleaseManifestError("release output must be outside the package source")
        candidates, records = _scan_artifacts(
            source,
            recursive=True,
            include_sbom=False,
            max_artifacts=max_artifacts,
            max_artifact_bytes=max_artifact_bytes,
            max_total_bytes=max_total_bytes,
        )
        _validate_exact_packages(candidates, version)
        _prepare_directory(output, "release output")
        names: dict[str, tuple[str, int]] = {}
        for source_path in candidates:
            record = records[source_path]
            prior = names.get(source_path.name)
            if prior is not None:
                raise ReleaseManifestError(
                    f"duplicate release filename across inputs: {source_path.name}"
                )
            names[source_path.name] = record
            destination = output / source_path.name
            if destination.exists() or destination.is_symlink():
                if digest(destination, maximum=max_artifact_bytes) != record:
                    raise ReleaseManifestError(
                        f"conflicting destination release filename: {source_path.name}"
                    )
                continue
            temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
            try:
                shutil.copy2(source_path, temporary)
                if digest(temporary, maximum=max_artifact_bytes) != record:
                    raise ReleaseManifestError("copied release artifact identity changed")
                os.replace(temporary, destination)
            finally:
                try:
                    temporary.unlink()
                except FileNotFoundError:
                    pass
    else:
        _prepare_directory(output, "release output")

    final_paths, final_records = _scan_artifacts(
        output,
        recursive=False,
        include_sbom=include_sbom,
        max_artifacts=max_artifacts,
        max_artifact_bytes=max_artifact_bytes,
        max_total_bytes=max_total_bytes,
    )
    packages = [
        path
        for path in final_paths
        if any(path.name.endswith(suffix) for suffix in PACKAGE_SUFFIXES)
    ]
    _validate_exact_packages(packages, version)
    if include_sbom:
        sboms = [path for path in final_paths if path.name in SBOM_NAMES or path.name.endswith((".spdx.json", ".cdx.json"))]
        _validate_sboms(sboms)
    manifest: dict[str, object] = {
        "schema": 1,
        "project": "automexia-terminal",
        "version": version,
        "tag": f"v{version}",
        "source_commit": commit,
        "artifacts": [
            {"file": path.name, "sha256": final_records[path][0], "size": final_records[path][1]}
            for path in sorted(final_paths, key=lambda item: item.name)
        ],
    }
    _atomic_write(
        output / "release-manifest.json",
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--copy-packages", action="store_true")
    parser.add_argument("--include-sbom", action="store_true")
    arguments = parser.parse_args()
    try:
        manifest = build_release_manifest(
            arguments.source,
            arguments.output,
            arguments.version,
            arguments.commit,
            copy_packages=arguments.copy_packages,
            include_sbom=arguments.include_sbom,
        )
    except (ReleaseManifestError, OSError, UnicodeError) as error:
        print(f"release manifest validation failed: {error}", file=sys.stderr)
        return 1
    print(
        f"Validated {len(manifest['artifacts'])} release files and wrote "
        "release-manifest.json"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
