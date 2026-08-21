#!/usr/bin/env python3
"""Validate DCO, documentation, changelog, and protected-path review policy."""

from __future__ import annotations

import json
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
    "apps/automexia-terminal/src/context/",
    "automexia-extension-api/",
    "automexia-extension-runtime/",
    "docs/adr/",
    "docs/project/adr/",
    "extensions/devops-ssh/",
    "tests/fixtures/session-launch/",
)
PROTECTED_EXACT_PATHS = {
    ".github/BRANCH-PROTECTION.md",
    "docs/SESSION-LAUNCH-BROKER.md",
    "tools/ci/check_pr_policy.py",
    "tools/ci/test_pr_policy.py",
    "tools/ci/check_session_launch_d0.py",
    "tools/ci/test_session_launch_d0.py",
}
ENGINE_PREFIXES = (
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
    "apps/automexia-terminal/src/context/",
    "automexia-extension-api/",
    "automexia-extension-runtime/",
)
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
MAX_REVIEW_PAYLOAD_BYTES = 4 * 1024 * 1024
MAX_REVIEW_COUNT = 10_000
REVIEW_LOGIN = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})\Z")
REVIEW_COMMIT = re.compile(r"(?:[0-9A-Fa-f]{40}|[0-9A-Fa-f]{64})\Z")


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], check=True, text=True, stdout=subprocess.PIPE
    )
    return result.stdout


def protected_paths(changed: set[str]) -> list[str]:
    return sorted(
        path
        for path in changed
        if path in PROTECTED_EXACT_PATHS
        or any(path.startswith(prefix) for prefix in PROTECTED_PREFIXES)
    )


def independent_approval_logins(raw: str, author: str, head: str) -> set[str]:
    author_login = author.strip().casefold()
    head_commit = head.strip().casefold()
    if not REVIEW_COMMIT.fullmatch(head_commit):
        return set()

    approvals: set[str] = set()
    for value in raw.split(","):
        login_value, separator, commit_value = value.strip().partition("|")
        login = login_value.strip().casefold()
        commit = commit_value.strip().casefold()
        if (
            separator
            and REVIEW_LOGIN.fullmatch(login)
            and commit == head_commit
            and login != author_login
            and not login.endswith("[bot]")
        ):
            approvals.add(login)
    return approvals


def approval_records_from_review_pages(pages: object) -> str:
    if not isinstance(pages, list):
        raise ValueError("review payload must contain a list of pages")

    latest: dict[str, tuple[tuple[str, int], str, str, str, str]] = {}
    review_count = 0
    for page in pages:
        if not isinstance(page, list):
            raise ValueError("each review page must be a list")
        for review in page:
            review_count += 1
            if review_count > MAX_REVIEW_COUNT:
                raise ValueError("review payload exceeds the bounded review count")
            if not isinstance(review, dict) or not isinstance(review.get("user"), dict):
                continue
            user = review["user"]
            login = user.get("login")
            user_type = user.get("type")
            state = review.get("state")
            commit = review.get("commit_id")
            submitted_at = review.get("submitted_at")
            review_id = review.get("id")
            if not (
                isinstance(login, str)
                and REVIEW_LOGIN.fullmatch(login)
                and isinstance(user_type, str)
                and isinstance(state, str)
                and isinstance(commit, str)
                and REVIEW_COMMIT.fullmatch(commit)
                and isinstance(submitted_at, str)
                and submitted_at
                and isinstance(review_id, int)
            ):
                continue
            order = (submitted_at, review_id)
            key = login.casefold()
            current = latest.get(key)
            if current is None or order > current[0]:
                latest[key] = (order, login, user_type, state, commit)

    records = [
        f"{login}|{commit}"
        for _, login, user_type, state, commit in latest.values()
        if user_type == "User" and state == "APPROVED"
    ]
    return ",".join(sorted(records, key=str.casefold))


def format_approvals_from_stdin() -> int:
    payload = sys.stdin.buffer.read(MAX_REVIEW_PAYLOAD_BYTES + 1)
    if len(payload) > MAX_REVIEW_PAYLOAD_BYTES:
        print("approval review payload exceeds the byte limit", file=sys.stderr)
        return 1
    try:
        pages = json.loads(payload)
        print(approval_records_from_review_pages(pages))
    except (json.JSONDecodeError, UnicodeDecodeError, ValueError) as error:
        print(f"invalid approval review payload: {error}", file=sys.stderr)
        return 1
    return 0


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
    approvals = independent_approval_logins(
        os.environ.get("APPROVAL_LOGINS", ""),
        os.environ.get("PR_AUTHOR", ""),
        head,
    )

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

    protected = protected_paths(changed)
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
    if sys.argv[1:] == ["--format-approvals"]:
        raise SystemExit(format_approvals_from_stdin())
    if sys.argv[1:]:
        print("unsupported arguments", file=sys.stderr)
        raise SystemExit(2)
    raise SystemExit(main())
