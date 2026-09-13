"""Shared heading-anchor extraction for repository-owned documentation checks.

This recognizes the repository's ATX heading subset, not a general Markdown
renderer. Fenced examples cannot supply evidence anchors; repeated headings
receive unique suffixes. File scope and reference policy stay with the caller.
"""

from __future__ import annotations

from pathlib import Path
import re


def markdown_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    occurrences: dict[str, int] = {}
    fence: tuple[str, int] | None = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if fence is not None:
            marker, minimum = fence
            if re.fullmatch(rf" {{0,3}}{re.escape(marker)}{{{minimum},}}\s*", line):
                fence = None
            continue
        opener = re.match(r"^ {0,3}(`{3,}|~{3,})(.*)$", line)
        if opener:
            marker = opener.group(1)
            fence = (marker[0], len(marker))
            continue
        match = re.match(r"^#{1,6}\s+(.+?)\s*#*\s*$", line)
        if not match:
            continue
        heading = re.sub(r"<[^>]+>", "", match.group(1)).strip().lower()
        slug = re.sub(r"[^\w\- ]", "", heading, flags=re.UNICODE)
        slug = re.sub(r"[\s-]+", "-", slug).strip("-")
        duplicate = occurrences.get(slug, 0)
        candidate = slug if duplicate == 0 else f"{slug}-{duplicate}"
        # Literal titles such as "Name-1" can consume a generated suffix first.
        while candidate in anchors:
            duplicate += 1
            candidate = f"{slug}-{duplicate}"
        occurrences[slug] = duplicate + 1
        anchors.add(candidate)
    return anchors
