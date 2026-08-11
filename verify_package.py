#!/usr/bin/env python3
from __future__ import annotations

import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "MANIFEST.sha256"


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> None:
    if not MANIFEST.is_file():
        raise SystemExit("FAIL: MANIFEST.sha256 is missing")

    expected: dict[str, str] = {}
    for line in MANIFEST.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        sha, relative = line.split(None, 1)
        relative = relative.strip()
        if relative.startswith("./"):
            relative = relative[2:]
        expected[relative] = sha

    actual_files = {
        str(path.relative_to(ROOT)).replace("\\", "/")
        for path in ROOT.rglob("*")
        if path.is_file()
        and path != MANIFEST
        and ".git" not in path.parts
        and "__pycache__" not in path.parts
        and path.suffix != ".pyc"
    }
    if actual_files != set(expected):
        missing = sorted(actual_files - set(expected))
        stale = sorted(set(expected) - actual_files)
        raise SystemExit(f"FAIL: manifest file set mismatch; missing entries={missing}, stale entries={stale}")

    for relative, sha in expected.items():
        actual = digest(ROOT / relative)
        if actual != sha:
            raise SystemExit(f"FAIL: manifest hash mismatch for {relative}")

    print(f"PASS: package manifest verified ({len(expected)} files)")


if __name__ == "__main__":
    main()
