#!/usr/bin/env python3
"""Reject incompatible vendored assets and missing non-Cargo notices."""

from __future__ import annotations

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]
FORBIDDEN_ROOTS = (
    ROOT / "misc",
    ROOT / "sugarloaf" / "src" / "components" / "filters" / "builtin",
)
FORBIDDEN_MARKERS = (
    "creativecommons.org/licenses/by-nc",
    "licensed under gnu 3.0",
    "gnu general public license",
)


def text_files(root: Path):
    if not root.exists():
        return
    for path in root.rglob("*"):
        if path.is_file():
            try:
                yield path, path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue


def main() -> int:
    failures: list[str] = []
    for root in FORBIDDEN_ROOTS:
        for path, content in text_files(root):
            lowered = content.lower()
            for marker in FORBIDDEN_MARKERS:
                if marker in lowered:
                    failures.append(
                        f"{path.relative_to(ROOT).as_posix()} contains forbidden license marker {marker!r}"
                    )

    required = {
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
    for relative, markers in required.items():
        path = ROOT / relative
        if not path.is_file():
            failures.append(f"missing required third-party license file {relative}")
            continue
        content = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in content:
                failures.append(f"{relative} is missing {marker!r}")

    runtime = ROOT / "sugarloaf/src/components/filters/runtime"
    for path in runtime.glob("*.rs"):
        if "MPL-2.0" not in path.read_text(encoding="utf-8"):
            failures.append(
                f"{path.relative_to(ROOT).as_posix()} lacks its MPL-2.0 source notice"
            )

    if failures:
        print("vendored license validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("PASS: vendored source, font, and asset licenses are compatible and noticed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
