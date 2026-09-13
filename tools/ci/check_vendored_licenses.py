#!/usr/bin/env python3
"""Reject incompatible vendored assets and missing non-Cargo notices."""

from __future__ import annotations

import os
from pathlib import Path
import stat
import sys


ROOT = Path(__file__).resolve().parents[2]
MAX_FILES = 20_000
MAX_FILE_BYTES = 16 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024
FORBIDDEN_MARKERS = (
    "creativecommons.org/licenses/by-nc",
    "licensed under gnu 3.0",
    "gnu general public license",
)
REQUIRED = {
    "THIRD_PARTY_NOTICES.md": (
        "Mozilla Public License 2.0",
        "SIL Open Font License 1.1",
        "Nerd Fonts Symbols Only",
    ),
    "rio-fonts/resources/SymbolsNerdFontMono/LICENSE": (
        "SIL OPEN FONT LICENSE Version 1.1",
    ),
    "sugarloaf/src/font/resources/CascadiaCode/LICENSE": (
        "SIL Open Font License, Version 1.1",
    ),
}


def _relative(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def _read_regular(
    path: Path, root: Path, *, max_file_bytes: int
) -> tuple[str | None, str | None, int]:
    relative = _relative(path, root)
    try:
        metadata = path.lstat()
        if (
            path.is_symlink()
            or not stat.S_ISREG(metadata.st_mode)
            or metadata.st_nlink != 1
        ):
            return None, f"{relative} must be a regular unlinked file", 0
        if metadata.st_size > max_file_bytes:
            return None, f"{relative} exceeds its {max_file_bytes}-byte limit", 0
        raw = path.read_bytes()
        if len(raw) != metadata.st_size:
            return None, f"{relative} changed while it was read", 0
        try:
            return raw.decode("utf-8"), None, len(raw)
        except UnicodeDecodeError:
            return None, None, len(raw)
    except OSError as error:
        return None, f"{relative} could not be inspected: {error}", 0


def _bounded_files(
    directory: Path,
    root: Path,
    *,
    max_files: int,
) -> tuple[list[Path], list[str]]:
    if not directory.exists():
        return [], []
    files: list[Path] = []
    failures: list[str] = []
    for current, child_directories, filenames in os.walk(directory, followlinks=False):
        current_path = Path(current)
        retained: list[str] = []
        for name in child_directories:
            child = current_path / name
            if child.is_symlink():
                failures.append(
                    f"{_relative(child, root)} must be a real directory, not a link"
                )
            else:
                retained.append(name)
        child_directories[:] = retained
        for name in filenames:
            files.append(current_path / name)
            if len(files) > max_files:
                failures.append(f"vendored file count exceeds its {max_files}-file limit")
                return files, failures
    return sorted(files), failures


def validate(
    root: Path = ROOT,
    *,
    max_files: int = MAX_FILES,
    max_file_bytes: int = MAX_FILE_BYTES,
    max_total_bytes: int = MAX_TOTAL_BYTES,
) -> list[str]:
    root = root.resolve()
    failures: list[str] = []
    total_bytes = 0
    scanned_files = 0

    for relative_root in (
        "misc",
        "sugarloaf/src/components/filters/builtin",
    ):
        paths, walker_failures = _bounded_files(
            root / relative_root, root, max_files=max_files
        )
        failures.extend(walker_failures)
        for path in paths:
            scanned_files += 1
            content, error, size = _read_regular(
                path, root, max_file_bytes=max_file_bytes
            )
            total_bytes += size
            if error:
                failures.append(error)
                continue
            if total_bytes > max_total_bytes:
                failures.append("vendored text exceeds its total byte limit")
                return failures
            if content is None:
                continue
            lowered = content.lower()
            for marker in FORBIDDEN_MARKERS:
                if marker in lowered:
                    failures.append(
                        f"{_relative(path, root)} contains forbidden license marker {marker!r}"
                    )

    for relative, markers in REQUIRED.items():
        path = root / relative
        if not path.exists() and not path.is_symlink():
            failures.append(f"missing required third-party license file {relative}")
            continue
        scanned_files += 1
        content, error, size = _read_regular(path, root, max_file_bytes=max_file_bytes)
        total_bytes += size
        if error:
            failures.append(error)
            continue
        if content is None:
            failures.append(f"{relative} must be UTF-8 text")
            continue
        for marker in markers:
            if marker not in content:
                failures.append(f"{relative} is missing {marker!r}")

    runtime = root / "sugarloaf/src/components/filters/runtime"
    if runtime.exists():
        for path in sorted(runtime.glob("*.rs")):
            scanned_files += 1
            content, error, size = _read_regular(
                path, root, max_file_bytes=max_file_bytes
            )
            total_bytes += size
            if error:
                failures.append(error)
            elif content is None or "MPL-2.0" not in content:
                failures.append(
                    f"{_relative(path, root)} lacks its MPL-2.0 source notice"
                )

    if scanned_files > max_files:
        failures.append(f"vendored file count exceeds its {max_files}-file limit")
    if total_bytes > max_total_bytes:
        failures.append("vendored text exceeds its total byte limit")
    return failures


def main() -> int:
    failures = validate(ROOT)
    if failures:
        print("vendored license validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("PASS: vendored source, font, and asset licenses are compatible and noticed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
