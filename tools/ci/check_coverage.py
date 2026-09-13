#!/usr/bin/env python3
"""Enforce non-decreasing global and 80% changed-line Rust coverage."""

from __future__ import annotations

import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import sys


REPO_ROOT = Path(__file__).resolve().parents[2]
MAX_LCOV_BYTES = 512 * 1024 * 1024
MAX_LCOV_FILES = 100_000
MAX_LCOV_LINES = 20_000_000
MIN_CHANGED_PERCENT = 80.0
MAX_UNCOVERED_CHANGED_DIAGNOSTICS = 64

# Automexia-owned product and extension code is held to the changed-line ratchet.
# Inherited Rio engine sources remain covered by the global non-regression ratchet.
OWNED_PREFIXES = (
    "apps/automexia-terminal/src/",
    "automexia-command-productivity/src/",
    "automexia-connectivity/src/",
    "automexia-devops/src/",
    "automexia-ecosystem/src/",
    "automexia-ecosystem-runtime/src/",
    "automexia-extension-api/src/",
    "automexia-extension-runtime/src/",
    "automexia-image/src/",
    "automexia-keybindings/src/",
    "automexia-ui-model/src/",
    "extensions/devops-aws/src/",
    "extensions/devops-azure/src/",
    "extensions/devops-gcp/src/",
    "extensions/devops-kubernetes/src/",
    "extensions/devops-openshift/src/",
    "extensions/devops-ssh/src/",
    "extensions/devops-teleport/src/",
    "tools/xtask/src/",
)
OWNED_FILES = frozenset({"rio-backend/src/config/product.rs"})


class CoveragePolicyError(ValueError):
    """Raised when coverage evidence is malformed, unsafe, or incomplete."""


def is_owned(path: str) -> bool:
    normalized = path.replace("\\", "/")
    return normalized in OWNED_FILES or normalized.startswith(OWNED_PREFIXES)


def _require_regular_unlinked(path: Path, label: str, maximum: int) -> int:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise CoveragePolicyError(f"{label} is missing") from error
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise CoveragePolicyError(f"{label} must be a regular unlinked file")
    if metadata.st_size > maximum:
        raise CoveragePolicyError(f"{label} exceeds its {maximum}-byte limit")
    return metadata.st_size


def _relative_source(raw_source: str, repo_root: Path) -> str:
    if not raw_source or any(ord(character) < 32 for character in raw_source):
        raise CoveragePolicyError("LCOV source path contains invalid control data")
    root = repo_root.resolve()
    source = Path(raw_source.replace("\\", os.sep))
    candidate = source if source.is_absolute() else root / source
    try:
        relative = candidate.resolve(strict=False).relative_to(root)
    except ValueError as error:
        raise CoveragePolicyError("LCOV source is outside the repository") from error
    if not relative.parts or any(part in {"", ".", ".."} for part in relative.parts):
        raise CoveragePolicyError("LCOV source is outside the repository")
    return relative.as_posix()


def parse_lcov(
    path: Path,
    *,
    repo_root: Path = REPO_ROOT,
    max_bytes: int = MAX_LCOV_BYTES,
    max_files: int = MAX_LCOV_FILES,
    max_lines: int = MAX_LCOV_LINES,
) -> dict[str, dict[int, int]]:
    expected_size = _require_regular_unlinked(path, "LCOV report", max_bytes)
    result: dict[str, dict[int, int]] = {}
    current: str | None = None
    executable_lines = 0
    bytes_read = 0
    with path.open("r", encoding="utf-8", newline="") as source_file:
        for record_number, raw in enumerate(source_file, start=1):
            bytes_read += len(raw.encode("utf-8"))
            if bytes_read > max_bytes:
                raise CoveragePolicyError("LCOV report grew beyond its byte limit")
            line = raw.rstrip("\r\n")
            if line.startswith("SF:"):
                current = _relative_source(line[3:], repo_root)
                if current not in result and len(result) >= max_files:
                    raise CoveragePolicyError("LCOV file limit exceeded")
                result.setdefault(current, {})
            elif line.startswith("DA:"):
                if current is None:
                    raise CoveragePolicyError("LCOV DA record appears before SF")
                fields = line[3:].split(",")
                if len(fields) < 2:
                    raise CoveragePolicyError(
                        f"malformed DA record at LCOV line {record_number}"
                    )
                try:
                    line_number = int(fields[0], 10)
                    hits = int(fields[1], 10)
                except ValueError as error:
                    raise CoveragePolicyError(
                        f"malformed DA record at LCOV line {record_number}"
                    ) from error
                if line_number <= 0:
                    raise CoveragePolicyError("invalid executable line in LCOV report")
                if hits < 0:
                    raise CoveragePolicyError("invalid hit count in LCOV report")
                lines = result[current]
                if line_number not in lines:
                    executable_lines += 1
                    if executable_lines > max_lines:
                        raise CoveragePolicyError("LCOV executable line limit exceeded")
                lines[line_number] = max(lines.get(line_number, 0), hits)
    if bytes_read != expected_size:
        raise CoveragePolicyError("LCOV report changed while it was being read")
    if not result or executable_lines == 0:
        raise CoveragePolicyError("LCOV report contains no executable lines")
    return result


def _repo_relative_git_path(raw: str) -> str:
    normalized = raw.replace("\\", "/")
    path = PurePosixPath(normalized)
    if path.is_absolute() or not path.parts or any(part == ".." for part in path.parts):
        raise CoveragePolicyError("Git reported a path outside the repository")
    return path.as_posix()


def changed_lines(
    base: str, head: str, *, repo_root: Path = REPO_ROOT
) -> dict[str, set[int]]:
    if not re.fullmatch(r"(?:[0-9a-f]{40}|HEAD)", base):
        raise CoveragePolicyError("BASE_SHA must be HEAD or a lowercase 40-character SHA")
    if not re.fullmatch(r"(?:[0-9a-f]{40}|HEAD|WORKTREE)", head):
        raise CoveragePolicyError(
            "HEAD_SHA must be HEAD, WORKTREE, or a lowercase 40-character SHA"
        )
    root = repo_root.resolve()
    git = ["git", "-c", f"safe.directory={root.as_posix()}"]
    diff_range = base if head == "WORKTREE" else f"{base}...{head}"
    output = subprocess.run(
        [*git, "diff", "--unified=0", diff_range, "--", "*.rs"],
        check=True,
        text=True,
        encoding="utf-8",
        errors="strict",
        stdout=subprocess.PIPE,
        cwd=root,
        timeout=120,
    ).stdout
    current: str | None = None
    changed: dict[str, set[int]] = {}
    for line in output.splitlines():
        if line.startswith("+++ b/"):
            current = _repo_relative_git_path(line[6:])
        elif current and line.startswith("@@"):
            match = re.search(r"\+(\d+)(?:,(\d+))?", line)
            if match:
                start = int(match.group(1))
                count = int(match.group(2) or "1")
                if count > MAX_LCOV_LINES:
                    raise CoveragePolicyError("Git changed-line range exceeds its limit")
                changed.setdefault(current, set()).update(range(start, start + count))

    if head == "WORKTREE":
        untracked = subprocess.run(
            [*git, "ls-files", "--others", "--exclude-standard", "--", "*.rs"],
            check=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            stdout=subprocess.PIPE,
            cwd=root,
            timeout=120,
        ).stdout.splitlines()
        if len(untracked) > MAX_LCOV_FILES:
            raise CoveragePolicyError("untracked Rust file count exceeds its limit")
        for raw_path in untracked:
            relative = _repo_relative_git_path(raw_path)
            source = root / Path(relative)
            _require_regular_unlinked(source, "untracked Rust source", MAX_LCOV_BYTES)
            with source.open("r", encoding="utf-8") as source_file:
                line_count = sum(1 for _ in source_file)
            if line_count > MAX_LCOV_LINES:
                raise CoveragePolicyError("untracked Rust source exceeds its line limit")
            changed[relative] = set(range(1, line_count + 1))
    return changed


def evaluate(
    coverage: dict[str, dict[int, int]],
    changed: dict[str, set[int]],
    *,
    baseline: float,
    platform: str,
) -> tuple[dict[str, object], str | None]:
    found = sum(len(lines) for lines in coverage.values())
    hit = sum(sum(value > 0 for value in lines.values()) for lines in coverage.values())
    global_percent = (100.0 * hit / found) if found else 0.0
    relevant: list[tuple[str, int, int]] = []
    owned_changed_lines = 0
    owned_changed_files = 0
    for path in sorted(changed):
        lines = changed[path]
        if not is_owned(path):
            continue
        owned_changed_files += 1
        owned_changed_lines += len(lines)
        covered_lines = coverage.get(path, {})
        relevant.extend(
            (path, line, covered_lines[line])
            for line in sorted(lines)
            if line in covered_lines
        )
    changed_percent = (
        100.0 * sum(hits > 0 for _, _, hits in relevant) / len(relevant)
        if relevant
        else 100.0
    )
    uncovered = [f"{path}:{line}" for path, line, hits in relevant if hits == 0]
    failure = None
    if global_percent + 1e-9 < baseline:
        failure = "global coverage regressed"
    elif changed_percent + 1e-9 < MIN_CHANGED_PERCENT:
        failure = "changed Automexia-owned line coverage is below 80%"
    summary: dict[str, object] = {
        "schema_version": 2,
        "status": "pass" if failure is None else "fail",
        "platform": platform,
        "global_line_percent": round(global_percent, 4),
        "global_baseline_percent": baseline,
        "changed_owned_line_percent": round(changed_percent, 4),
        "changed_owned_files": owned_changed_files,
        "changed_owned_lines": owned_changed_lines,
        "changed_owned_executable_lines": len(relevant),
        "uncovered_changed_owned_line_count": len(uncovered),
        "uncovered_changed_owned_lines": uncovered[
            :MAX_UNCOVERED_CHANGED_DIAGNOSTICS
        ],
        "uncovered_changed_owned_lines_truncated": (
            len(uncovered) > MAX_UNCOVERED_CHANGED_DIAGNOSTICS
        ),
        "executable_lines": found,
        "covered_lines": hit,
        "failure": failure,
    }
    return summary, failure


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise CoveragePolicyError(f"duplicate coverage baseline key: {key}")
        result[key] = value
    return result


def load_baseline(path: Path) -> dict[str, object]:
    _require_regular_unlinked(path, "coverage baseline", 64 * 1024)
    try:
        record = json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=_reject_duplicate_keys
        )
    except json.JSONDecodeError as error:
        raise CoveragePolicyError("coverage baseline is not valid JSON") from error
    if not isinstance(record, dict):
        raise CoveragePolicyError("coverage baseline must be an object")
    baseline = record.get("line_percent")
    platform = record.get("platform")
    if (
        isinstance(baseline, bool)
        or not isinstance(baseline, (int, float))
        or not 0.0 <= float(baseline) <= 100.0
    ):
        raise CoveragePolicyError("coverage baseline line_percent is invalid")
    if not isinstance(platform, str) or not re.fullmatch(r"[a-z0-9_-]{1,64}", platform):
        raise CoveragePolicyError("coverage baseline platform is invalid")
    return record


def write_summary(destination: Path, payload: dict[str, object]) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() or destination.is_symlink():
        _require_regular_unlinked(destination, "coverage summary", 4 * 1024 * 1024)
    temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
    if temporary.exists() or temporary.is_symlink():
        raise CoveragePolicyError("coverage summary temporary path already exists")
    try:
        temporary.write_text(
            json.dumps(payload, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        os.replace(temporary, destination)
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def main() -> int:
    try:
        platform = os.environ.get("COVERAGE_PLATFORM", "")
        base = os.environ.get("BASE_SHA", "")
        head = os.environ.get("HEAD_SHA", "HEAD")
        if not platform:
            raise CoveragePolicyError("COVERAGE_PLATFORM is required")
        if not base:
            raise CoveragePolicyError("BASE_SHA is required")
        baseline_record = load_baseline(REPO_ROOT / ".github/coverage-baseline.json")
        if platform != baseline_record["platform"]:
            raise CoveragePolicyError(
                "coverage platform does not match the recorded baseline: "
                f"{platform!r} != {baseline_record['platform']!r}"
            )
        lcov_path = Path(os.environ.get("LCOV_FILE", "lcov.info"))
        if not lcov_path.is_absolute():
            lcov_path = REPO_ROOT / lcov_path
        coverage = parse_lcov(lcov_path)
        changed = changed_lines(base, head)
        summary, failure = evaluate(
            coverage,
            changed,
            baseline=float(baseline_record["line_percent"]),
            platform=platform,
        )
        print(
            f"global line coverage: {summary['global_line_percent']:.2f}% "
            f"(baseline {summary['global_baseline_percent']:.2f}%)"
        )
        print(
            "changed owned line coverage: "
            f"{summary['changed_owned_line_percent']:.2f}% "
            f"({summary['changed_owned_executable_lines']} executable lines)"
        )
        uncovered = summary["uncovered_changed_owned_lines"]
        if uncovered:
            print(
                "uncovered changed owned lines: " + ", ".join(uncovered),
                file=sys.stderr,
            )
            if summary["uncovered_changed_owned_lines_truncated"]:
                hidden = summary["uncovered_changed_owned_line_count"] - len(uncovered)
                print(f"uncovered diagnostics truncated: {hidden} more", file=sys.stderr)
        summary_path = os.environ.get("COVERAGE_SUMMARY")
        if summary_path:
            destination = Path(summary_path)
            if not destination.is_absolute():
                destination = REPO_ROOT / destination
            write_summary(destination, summary)
        if failure:
            print(failure, file=sys.stderr)
            return 1
        return 0
    except (CoveragePolicyError, OSError, UnicodeError, subprocess.SubprocessError) as error:
        print(f"coverage policy failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
