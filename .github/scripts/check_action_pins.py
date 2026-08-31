#!/usr/bin/env python3
"""Enforce immutable, allowlisted external GitHub Action references."""

from __future__ import annotations

from pathlib import Path
import re
import stat
import sys


WORKFLOWS = Path(__file__).resolve().parents[1] / "workflows"
MAX_WORKFLOWS = 64
MAX_WORKFLOW_BYTES = 2 * 1024 * 1024
USE_RE = re.compile(
    r"^\s*(?:-\s*)?uses:\s*(?:\n\s*)?[\"']?([^\s\"']+)", re.MULTILINE
)
FULL_SHA = re.compile(r"^(?P<action>[^@]+)@(?P<sha>[0-9a-f]{40})$")
ALLOWED_ACTIONS = {
    "actions/cache",
    "actions/checkout",
    "actions/download-artifact",
    "actions/upload-artifact",
    "actions/create-github-app-token",
    "anchore/sbom-action",
    "azure/artifact-signing-action",
    "azure/login",
    "taiki-e/install-action",
}


def validate_workflows(
    workflows: Path,
    *,
    max_workflows: int = MAX_WORKFLOWS,
    max_file_bytes: int = MAX_WORKFLOW_BYTES,
) -> list[str]:
    failures: list[str] = []
    paths = sorted([*workflows.glob("*.yml"), *workflows.glob("*.yaml")])
    if len(paths) > max_workflows:
        return [f"workflow inventory exceeds its {max_workflows}-file limit"]
    for path in paths:
        try:
            metadata = path.lstat()
            if (
                path.is_symlink()
                or not stat.S_ISREG(metadata.st_mode)
                or metadata.st_nlink != 1
            ):
                failures.append(f"{path.name}: workflow must be a regular unlinked file")
                continue
            if metadata.st_size > max_file_bytes:
                failures.append(
                    f"{path.name}: workflow exceeds its {max_file_bytes}-byte limit"
                )
                continue
            raw = path.read_bytes()
            if len(raw) != metadata.st_size:
                failures.append(f"{path.name}: workflow changed while it was read")
                continue
            text = raw.decode("utf-8")
        except (OSError, UnicodeError) as error:
            failures.append(f"{path.name}: workflow could not be read: {error}")
            continue
        for match in USE_RE.finditer(text):
            reference = match.group(1).strip().strip("\"'")
            if reference.startswith("./"):
                continue
            line = text.count("\n", 0, match.start()) + 1
            if reference.startswith("docker://"):
                failures.append(
                    f"{path.name}:{line}: docker:// actions are not allowed by "
                    f"the production policy: {reference}"
                )
                continue
            parsed = FULL_SHA.fullmatch(reference)
            if not parsed:
                failures.append(
                    f"{path.name}:{line}: external action is not pinned to a full "
                    f"lowercase SHA: {reference}"
                )
                continue
            action = parsed.group("action")
            if action not in ALLOWED_ACTIONS:
                failures.append(
                    f"{path.name}:{line}: external action repository is not "
                    f"allowlisted: {action}"
                )
    return failures


def main() -> int:
    failures = validate_workflows(WORKFLOWS)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(
        "Action trust policy passed: every external Action is allowlisted and "
        "pinned to a full lowercase 40-character commit SHA."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
