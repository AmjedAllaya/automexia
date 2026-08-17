#!/usr/bin/env python3
"""Validate DCO, documentation, changelog, and protected-path review policy."""

from __future__ import annotations

import os
import re
import subprocess
import sys


PROTECTED_PREFIXES = (
    ".github/workflows/",
    "packaging/",
    "assets/brand/ASSET-MANIFEST.toml",
    "SECURITY.md",
    "GOVERNANCE.md",
    "RELEASING.md",
    "NOTICE.md",
    "CODEOWNERS",
    "corcovado/",
    "librio/",
    "librio-wasm/",
    "rio-backend/",
    "rio-fonts/",
    "rio-graphics/",
    "rio-grapheme-width/",
    "rio-notifier/",
    "rio-vt/",
    "rio-window/",
    "sugarloaf/",
    "teletypewriter/",
)
ENGINE_PREFIXES = PROTECTED_PREFIXES[-12:]
EXEMPT_LABELS = {"documentation", "tests-only", "internal-maintenance"}
TITLE = re.compile(
    r"^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)"
    r"(?:\([a-z0-9._/-]+\))?!?: .+"
)
SIGNOFF = re.compile(r"(?im)^Signed-off-by:\s*.+\s+<[^<>\s]+@[^<>\s]+>$")
DOCUMENTATION_REQUIRED_SUFFIXES = (
    ".bash",
    ".cmd",
    ".fish",
    ".json",
    ".lock",
    ".ps1",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".wgsl",
    ".wxs",
    ".xml",
    ".yaml",
    ".yml",
    ".zsh",
)
DOCUMENTATION_REQUIRED_PREFIXES = ("assets/", "packaging/")


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], check=True, text=True, stdout=subprocess.PIPE
    )
    return result.stdout


def missing_documentation_for(changed: set[str]) -> list[str]:
    if any(
        path.startswith("docs/") and path.endswith(".md") for path in changed
    ):
        return []
    return sorted(
        path
        for path in changed
        if not path.startswith(("docs/", "changes/"))
        and (
            path.endswith(DOCUMENTATION_REQUIRED_SUFFIXES)
            or path.startswith(DOCUMENTATION_REQUIRED_PREFIXES)
        )
    )


def main() -> int:
    base = os.environ["BASE_SHA"]
    head = os.environ.get("HEAD_SHA", "HEAD")
    title = os.environ.get("PR_TITLE", "")
    labels = {
        label.strip() for label in os.environ.get("PR_LABELS", "").split(",") if label
    }
    approvals = {
        login.strip()
        for login in os.environ.get("APPROVAL_LOGINS", "").split(",")
        if login.strip()
    }

    failures: list[str] = []
    if not TITLE.fullmatch(title):
        failures.append("PR title is not Conventional Commits compatible")

    changed = set(git("diff", "--name-only", f"{base}...{head}").splitlines())
    has_fragment = any(
        path.startswith("changes/") and path != "changes/README.md" for path in changed
    )
    if not has_fragment and not labels.intersection(EXEMPT_LABELS):
        failures.append("PR needs a changes/ fragment or an explicit exempt label")
    undocumented = missing_documentation_for(changed)
    if undocumented:
        failures.append(
            "behavior/configuration/test changes require an affected docs/*.md "
            "update in the same pull request: " + ", ".join(undocumented)
        )

    messages = git("log", "--format=%H%x00%B%x00", f"{base}..{head}").split("\x00")
    for index in range(0, len(messages) - 1, 2):
        commit = messages[index].strip()
        body = messages[index + 1]
        if commit and not SIGNOFF.search(body):
            failures.append(f"commit {commit[:12]} lacks a valid DCO Signed-off-by line")

    protected = sorted(
        path
        for path in changed
        if any(path == prefix or path.startswith(prefix) for prefix in PROTECTED_PREFIXES)
    )
    if protected and len(approvals) < 2:
        failures.append(
            "protected paths require two distinct approving reviewers: "
            + ", ".join(protected)
        )

    engine_sources = {
        path
        for path in changed
        if path.endswith(".rs")
        and any(path.startswith(prefix) for prefix in ENGINE_PREFIXES)
        and "/tests/" not in path
    }
    if engine_sources:
        diff = git("diff", "--unified=0", f"{base}...{head}", "--", "*.rs")
        changed_test_file = any(
            "/tests/" in path or path.endswith("/tests.rs") for path in changed
        )
        added_inline_test = any(
            line.startswith("+") and not line.startswith("+++") and "#[test]" in line
            for line in diff.splitlines()
        )
        if not changed_test_file and not added_inline_test:
            failures.append(
                "engine Rust changes require a focused integration test or a newly added inline test: "
                + ", ".join(sorted(engine_sources))
            )

    if failures:
        print("PR policy failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("PASS: title, DCO, documentation, changelog, and protected-path review policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
