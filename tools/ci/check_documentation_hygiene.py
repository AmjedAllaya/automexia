#!/usr/bin/env python3
"""Validate repository Markdown encoding and structural hygiene."""

from __future__ import annotations

import os
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
EXCLUDED_PARTS = {
    ".git",
    ".cargo-packager",
    ".automexia-private",
    ".automexia-tools",
    "target",
}
MAX_MARKDOWN_BYTES = 8 * 1024 * 1024
FENCE_OPEN = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
HEADING = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")


class DocumentationHygieneError(ValueError):
    """A Markdown file violates the repository text contract."""


def markdown_files(root: Path = ROOT) -> list[Path]:
    files: list[Path] = []
    for directory, child_directories, filenames in os.walk(root):
        child_directories[:] = [
            name for name in child_directories if name not in EXCLUDED_PARTS
        ]
        base = Path(directory)
        files.extend(
            base / filename
            for filename in filenames
            if Path(filename).suffix.lower() == ".md"
        )
    return sorted(files)


def validate_markdown_payload(
    name: str,
    payload: bytes,
    *,
    require_single_h1: bool = False,
    require_heading_order: bool = False,
) -> int:
    if len(payload) > MAX_MARKDOWN_BYTES:
        raise DocumentationHygieneError(f"{name} exceeds the Markdown byte limit")
    if payload.startswith(b"\xef\xbb\xbf"):
        raise DocumentationHygieneError(f"{name} contains a UTF-8 byte-order mark")
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise DocumentationHygieneError(f"{name} is not strict UTF-8") from error
    if "\x00" in text:
        raise DocumentationHygieneError(f"{name} contains a NUL byte")
    if "\r" in text:
        raise DocumentationHygieneError(f"{name} must use LF line endings")
    if payload and not payload.endswith(b"\n"):
        raise DocumentationHygieneError(f"{name} must end with a newline")

    fence: tuple[str, int, int] | None = None
    headings: list[tuple[int, int, str]] = []
    for line_number, line in enumerate(text.splitlines(), 1):
        if fence is None:
            opener = FENCE_OPEN.match(line)
            if opener:
                marker = opener.group(1)
                fence = (marker[0], len(marker), line_number)
                continue
            heading = HEADING.match(line)
            if heading:
                headings.append((line_number, len(heading.group(1)), heading.group(2)))
            continue

        marker, minimum, _ = fence
        if re.fullmatch(rf" {{0,3}}{re.escape(marker)}{{{minimum},}}\s*", line):
            fence = None

    if fence is not None:
        raise DocumentationHygieneError(
            f"{name}:{fence[2]} has an unclosed fenced code block"
        )
    if require_single_h1:
        h1 = [heading for heading in headings if heading[1] == 1]
        if not headings or headings[0][1] != 1 or len(h1) != 1:
            raise DocumentationHygieneError(
                f"{name} must start its heading hierarchy with exactly one H1"
            )
    if require_heading_order:
        previous = 0
        for line_number, level, title in headings:
            if previous and level > previous + 1:
                raise DocumentationHygieneError(
                    f"{name}:{line_number} skips from H{previous} to H{level}: {title}"
                )
            previous = level
    return len(text.splitlines())


def validate(root: Path = ROOT) -> dict[str, int]:
    files = markdown_files(root)
    lines = 0
    for path in files:
        if path.is_symlink() or not path.is_file():
            raise DocumentationHygieneError(
                f"Markdown must be a regular non-link file: {path.relative_to(root)}"
            )
        lines += validate_markdown_payload(
            path.relative_to(root).as_posix(), path.read_bytes()
        )
    return {"files": len(files), "lines": lines}


def main() -> int:
    try:
        counts = validate()
    except (DocumentationHygieneError, OSError) as error:
        print(f"documentation hygiene validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: repository Markdown is strict UTF-8/LF, newline-terminated, "
        f"and fence-balanced (files={counts['files']}, lines={counts['lines']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
