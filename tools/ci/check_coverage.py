#!/usr/bin/env python3
"""Enforce non-decreasing global and 80% changed-line Rust coverage."""

from __future__ import annotations

import json
import os
from pathlib import Path
import re
import subprocess
import sys


OWNED_PREFIXES = (
    "apps/automexia-terminal/src/automexia/",
    "rio-backend/src/config/product.rs",
    "tools/xtask/src/",
)


def parse_lcov(path: Path) -> dict[str, dict[int, int]]:
    result: dict[str, dict[int, int]] = {}
    current: str | None = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        if raw.startswith("SF:"):
            source = Path(raw[3:]).as_posix()
            marker = "/automexia-terminal/"
            current = source.split(marker, 1)[-1] if marker in source else source
            result.setdefault(current, {})
        elif raw.startswith("DA:") and current:
            line, hits, *_ = raw[3:].split(",")
            result[current][int(line)] = max(result[current].get(int(line), 0), int(hits))
    return result


def changed_lines(base: str, head: str) -> dict[str, set[int]]:
    output = subprocess.run(
        ["git", "diff", "--unified=0", f"{base}...{head}", "--", "*.rs"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout
    current: str | None = None
    changed: dict[str, set[int]] = {}
    for line in output.splitlines():
        if line.startswith("+++ b/"):
            current = line[6:]
        elif current and line.startswith("@@"):
            match = re.search(r"\+(\d+)(?:,(\d+))?", line)
            if match:
                start = int(match.group(1))
                count = int(match.group(2) or "1")
                changed.setdefault(current, set()).update(range(start, start + count))
    return changed


def main() -> int:
    coverage = parse_lcov(Path(os.environ.get("LCOV_FILE", "lcov.info")))
    found = sum(len(lines) for lines in coverage.values())
    hit = sum(sum(value > 0 for value in lines.values()) for lines in coverage.values())
    global_percent = (100.0 * hit / found) if found else 0.0
    baseline = json.loads(
        Path(".github/coverage-baseline.json").read_text(encoding="utf-8")
    )["line_percent"]

    base = os.environ["BASE_SHA"]
    head = os.environ.get("HEAD_SHA", "HEAD")
    changed = changed_lines(base, head)
    relevant: list[int] = []
    for path, lines in changed.items():
        if not any(path.startswith(prefix) for prefix in OWNED_PREFIXES):
            continue
        covered_lines = coverage.get(path, {})
        relevant.extend(covered_lines[line] for line in lines if line in covered_lines)
    changed_percent = (
        100.0 * sum(value > 0 for value in relevant) / len(relevant)
        if relevant
        else 100.0
    )

    print(f"global line coverage: {global_percent:.2f}% (baseline {baseline:.2f}%)")
    print(f"changed owned line coverage: {changed_percent:.2f}%")
    if global_percent + 1e-9 < baseline:
        print("global coverage regressed", file=sys.stderr)
        return 1
    if changed_percent + 1e-9 < 80.0:
        print("changed Automexia-owned line coverage is below 80%", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
