#!/usr/bin/env python3
"""Fail-closed stable-release source provenance checks.

This checker intentionally owns only facts that can be proven from the checked-
out Git source and the versioned policy. Hosted repository state remains owned
by repository_protection.py and is invoked separately by the release workflow.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import NoReturn


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests/assurance/stable-release-policy-v1.json"
RELEASE_WORKFLOW = ROOT / ".github/workflows/release.yml"
EXPECTED_POLICY_SHA256 = "b04f07dc25af975721246fe8ff29b88061fcdd77e0b79d8890aebdbdbc82fd5a"
MAX_POLICY_BYTES = 128 * 1024
MAX_GIT_OUTPUT_BYTES = 16 * 1024 * 1024
MAX_RELEASE_COMMITS = 100_000
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
EXPECTED_TAG_PATTERN = r"^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$"
POLICY_KEYS = {
    "schema",
    "repository",
    "default_branch",
    "fork_provenance",
    "release_source",
    "hosted_repository",
    "external_prerequisites",
}
FORK_KEYS = {"tag", "commit", "require_annotated_tag", "require_remote_tag"}
SOURCE_KEYS = {
    "tag_pattern",
    "require_annotated_tag",
    "require_remote_tag",
    "require_exact_default_branch_head",
    "require_clean_tracked_source",
    "require_complete_history",
    "require_linear_history",
    "require_dco_after_fork",
    "require_author_matching_signoff",
}
HOSTED_KEYS = {
    "require_authenticated_audit",
    "required_result",
    "audit_secret",
    "protected_environment",
}
EXTERNAL_KEYS = {"id", "owner", "evidence"}
EXPECTED_EXTERNAL_PREREQUISITES = {
    "brand-rights-and-final-assets": ("release-owner", "assets/brand/ASSET-MANIFEST.toml"),
    "private-conduct-contact": ("governance-owner", "SECURITY.md"),
    "windows-production-signing": ("release-owner", "docs/RELEASE-TRUST.md"),
    "apple-developer-id-and-notarization": ("release-owner", "docs/RELEASE-TRUST.md"),
    "github-plan-or-public-visibility": ("repository-owner", ".github/BRANCH-PROTECTION.md"),
    "independent-reviewer-capacity": ("repository-owner", "CODEOWNERS"),
    "github-actions-billing": ("repository-owner", ".github/BRANCH-PROTECTION.md"),
    "github-security-entitlements": ("repository-owner", ".github/repository-protection.json"),
    "s1-controlled-native-visual-resource-accessibility": ("assurance-owner", "tests/assurance/s1-assurance-policy-v1.json"),
    "s2-active-thirty-day-baseline": ("performance-owner", "tests/assurance/performance-ratchet-policy-v1.json"),
    "historical-linearity-and-dco-resolution": ("repository-owner", "docs/READINESS-AUDIT.md"),
}
SIGNOFF_RE = re.compile(
    r"(?im)^Signed-off-by:\s*[^\r\n<>]+\s*<([^\r\n<>]+)>\s*$"
)


class ReleaseError(RuntimeError):
    """A redacted, actionable release-policy failure."""


def fail(message: str) -> NoReturn:
    raise ReleaseError(message)


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            fail(f"stable-release policy contains duplicate key {key!r}")
        result[key] = value
    return result


def load_policy(path: Path = POLICY_PATH) -> dict[str, object]:
    try:
        size = path.stat().st_size
    except OSError as error:
        raise ReleaseError("stable-release policy is unavailable") from error
    if size <= 0 or size > MAX_POLICY_BYTES:
        fail("stable-release policy exceeds its byte limit")
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ReleaseError("stable-release policy could not be read") from error
    if path.resolve() == POLICY_PATH.resolve():
        digest = hashlib.sha256(raw).hexdigest()
        if EXPECTED_POLICY_SHA256 != "TO_BE_REPLACED" and digest != EXPECTED_POLICY_SHA256:
            fail("stable-release policy digest drifted")
    try:
        loaded = json.loads(raw, object_pairs_hook=_reject_duplicate_keys)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ReleaseError("stable-release policy is not valid UTF-8 JSON") from error
    if not isinstance(loaded, dict):
        fail("stable-release policy root must be an object")
    return loaded


def _require_bool(mapping: dict[str, object], key: str) -> None:
    if mapping.get(key) is not True:
        fail(f"stable-release policy must require {key}")


def _require_mapping(policy: dict[str, object], key: str) -> dict[str, object]:
    value = policy.get(key)
    if not isinstance(value, dict):
        fail(f"stable-release policy section {key!r} is missing")
    return value


def _require_exact_keys(
    mapping: dict[str, object], expected: set[str], label: str
) -> None:
    if set(mapping) != expected:
        fail(f"stable-release {label} keys drifted")


def validate_policy(policy: dict[str, object]) -> None:
    _require_exact_keys(policy, POLICY_KEYS, "policy")
    if policy.get("schema") != 1:
        fail("stable-release policy schema must be 1")
    if policy.get("repository") != "AmjedAllaya/automexia-terminal":
        fail("stable-release repository identity drifted")
    if policy.get("default_branch") != "main":
        fail("stable-release default branch drifted")

    fork = _require_mapping(policy, "fork_provenance")
    _require_exact_keys(fork, FORK_KEYS, "fork provenance")
    if fork.get("tag") != "rio-base-0.5.20-7d595af":
        fail("stable-release fork tag drifted")
    commit = fork.get("commit")
    if commit != "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2":
        fail("stable-release fork commit drifted")
    _require_bool(fork, "require_annotated_tag")
    _require_bool(fork, "require_remote_tag")

    source = _require_mapping(policy, "release_source")
    _require_exact_keys(source, SOURCE_KEYS, "release source")
    pattern = source.get("tag_pattern")
    if pattern != EXPECTED_TAG_PATTERN:
        fail("stable-release tag pattern drifted")
    try:
        re.compile(pattern)
    except re.error as error:
        raise ReleaseError("stable-release tag pattern is invalid") from error
    for key in (
        "require_annotated_tag",
        "require_remote_tag",
        "require_exact_default_branch_head",
        "require_clean_tracked_source",
        "require_complete_history",
        "require_linear_history",
        "require_dco_after_fork",
        "require_author_matching_signoff",
    ):
        _require_bool(source, key)

    hosted = _require_mapping(policy, "hosted_repository")
    _require_exact_keys(hosted, HOSTED_KEYS, "hosted repository")
    _require_bool(hosted, "require_authenticated_audit")
    if hosted.get("required_result") != "pass":
        fail("stable-release hosted audit must require pass")
    if hosted.get("audit_secret") != "AUTOMEXIA_REPOSITORY_AUDIT_TOKEN":
        fail("stable-release audit credential name drifted")
    if hosted.get("protected_environment") != "stable-release":
        fail("stable-release protected environment drifted")

    prerequisites = policy.get("external_prerequisites")
    if not isinstance(prerequisites, list) or len(prerequisites) != 11:
        fail("stable-release external prerequisite inventory drifted")
    identifiers: set[str] = set()
    for item in prerequisites:
        if not isinstance(item, dict):
            fail("stable-release external prerequisite must be an object")
        _require_exact_keys(item, EXTERNAL_KEYS, "external prerequisite")
        identifier = item.get("id")
        owner = item.get("owner")
        evidence = item.get("evidence")
        if not isinstance(identifier, str) or not re.fullmatch(r"[a-z0-9-]{3,80}", identifier):
            fail("stable-release external prerequisite has an invalid id")
        if identifier in identifiers:
            fail("stable-release external prerequisite ids must be unique")
        identifiers.add(identifier)
        if not isinstance(owner, str) or not owner:
            fail("stable-release external prerequisite has no owner")
        if not isinstance(evidence, str) or not evidence or Path(evidence).is_absolute():
            fail("stable-release external prerequisite has invalid evidence")
        expected_prerequisite = EXPECTED_EXTERNAL_PREREQUISITES.get(identifier)
        if expected_prerequisite is not None and expected_prerequisite != (owner, evidence):
            fail(f"stable-release external prerequisite contract drifted for {identifier}")
        if not (ROOT / evidence).exists():
            fail(f"stable-release evidence authority is missing for {identifier}")
    if identifiers != set(EXPECTED_EXTERNAL_PREREQUISITES):
        fail("stable-release external prerequisite identities drifted")


def validate_release_workflow(
    policy: dict[str, object], path: Path = RELEASE_WORKFLOW
) -> None:
    validate_policy(policy)
    try:
        workflow = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise ReleaseError("stable-release workflow is unavailable") from error
    if len(workflow.encode("utf-8")) > 512 * 1024:
        fail("stable-release workflow exceeds its byte limit")
    required = {
        "protected environment": "environment: stable-release",
        "audit credential": "AUTOMEXIA_REPOSITORY_AUDIT_TOKEN",
        "source policy": "python tools/ci/stable_release.py check-policy",
        "source mutation tests": "python tools/ci/test_stable_release.py",
        "source validation": "python tools/ci/stable_release.py validate-source",
        "exact source commit": '--expected-commit "$GITHUB_SHA"',
        "exact release tag": '--release-tag "$GITHUB_REF_NAME"',
        "repository audit": "python tools/ci/repository_protection.py audit --json",
    }
    for label, token in required.items():
        if token not in workflow:
            fail(f"stable-release workflow is missing {label}")
    preflight_match = re.search(
        r"(?ms)^  preflight:\n(?P<body>.*?)(?=^  [a-zA-Z0-9_-]+:\n|\Z)", workflow
    )
    if preflight_match is None:
        fail("stable-release workflow has no preflight job")
    body = preflight_match.group("body")
    if "environment: stable-release" not in body:
        fail("stable-release preflight is outside its protected environment")
    if "GH_TOKEN: ${{ secrets.AUTOMEXIA_REPOSITORY_AUDIT_TOKEN }}" not in body:
        fail("stable-release preflight does not bind the audit credential")
    if "fetch-depth: 0" not in body:
        fail("stable-release preflight must fetch complete Git history")


def _run_git(root: Path, *args: str) -> str:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=root,
            check=False,
            capture_output=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ReleaseError(f"git {args[0]} operation could not complete") from error
    if len(completed.stdout) > MAX_GIT_OUTPUT_BYTES or len(completed.stderr) > MAX_GIT_OUTPUT_BYTES:
        fail(f"git {args[0]} output exceeded its byte limit")
    if completed.returncode != 0:
        fail(f"git {args[0]} operation failed")
    try:
        return completed.stdout.decode("utf-8", errors="strict").strip()
    except UnicodeDecodeError as error:
        raise ReleaseError(f"git {args[0]} output was not UTF-8") from error


def _local_tag_commit(root: Path, tag: str, *, label: str) -> str:
    object_type = _run_git(root, "cat-file", "-t", f"refs/tags/{tag}")
    if object_type != "tag":
        fail(f"{label} must be an annotated tag")
    commit = _run_git(root, "rev-parse", f"refs/tags/{tag}^{{commit}}")
    if not COMMIT_RE.fullmatch(commit):
        fail(f"{label} does not resolve to a commit")
    return commit


def _remote_tag_commit(root: Path, remote: str, tag: str, *, label: str) -> str:
    output = _run_git(
        root,
        "ls-remote",
        "--tags",
        remote,
        f"refs/tags/{tag}",
        f"refs/tags/{tag}^{{}}",
    )
    direct: str | None = None
    peeled: str | None = None
    for line in output.splitlines():
        parts = line.split("\t", 1)
        if len(parts) != 2 or not COMMIT_RE.fullmatch(parts[0]):
            fail(f"{label} remote response is malformed")
        if parts[1] == f"refs/tags/{tag}":
            direct = parts[0]
        elif parts[1] == f"refs/tags/{tag}^{{}}":
            peeled = parts[0]
    if direct is None or peeled is None:
        fail(f"remote {label} is missing or is not annotated")
    return peeled


def _validate_dco(root: Path, base: str, expected_commit: str) -> None:
    records = _run_git(
        root,
        "log",
        "-z",
        "--reverse",
        "--format=%H%x00%ae%x00%B",
        f"{base}..{expected_commit}",
    ).split("\x00")
    if records and records[-1] == "":
        records.pop()
    if not records or len(records) % 3 != 0:
        fail("stable-release DCO history is empty or malformed")
    commit_count = len(records) // 3
    if commit_count > MAX_RELEASE_COMMITS:
        fail("stable-release commit range exceeds its limit")
    for index in range(0, len(records), 3):
        commit, author_email, message = records[index : index + 3]
        if not COMMIT_RE.fullmatch(commit):
            fail("stable-release history contains an invalid commit id")
        if not author_email:
            fail("stable-release DCO record is malformed")
        signoffs = [email.casefold() for email in SIGNOFF_RE.findall(message)]
        if author_email.casefold() not in signoffs:
            fail("stable-release history contains a commit without author-matching DCO")


def validate_source(
    policy: dict[str, object],
    root: Path,
    expected_commit: str,
    release_tag: str,
    remote: str = "origin",
) -> None:
    if not COMMIT_RE.fullmatch(expected_commit):
        fail("expected release commit must be a full lowercase SHA-1")
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,80}", remote):
        fail("release remote name is invalid")
    source = _require_mapping(policy, "release_source")
    pattern = source.get("tag_pattern")
    if not isinstance(pattern, str) or re.fullmatch(pattern, release_tag) is None:
        fail("release tag does not match the stable-release policy")
    if _run_git(root, "rev-parse", "--is-shallow-repository") != "false":
        fail("stable release requires complete Git history")
    head = _run_git(root, "rev-parse", "HEAD")
    if head != expected_commit:
        fail("checked-out HEAD does not match the expected release commit")
    if _run_git(root, "status", "--porcelain", "--untracked-files=no"):
        fail("stable release requires clean tracked source")

    release_commit = _local_tag_commit(root, release_tag, label="release tag")
    if release_commit != expected_commit:
        fail("release tag does not identify the expected release commit")
    if _remote_tag_commit(root, remote, release_tag, label="release tag") != expected_commit:
        fail("remote release tag does not identify the expected release commit")

    default_branch = policy.get("default_branch")
    if not isinstance(default_branch, str):
        fail("stable-release default branch is invalid")
    remote_head = _run_git(root, "ls-remote", remote, f"refs/heads/{default_branch}")
    fields = remote_head.split()
    if len(fields) != 2 or not COMMIT_RE.fullmatch(fields[0]):
        fail("remote default branch head is unavailable")
    if fields[0] != expected_commit:
        fail("release commit is not the exact remote default branch head")

    fork = _require_mapping(policy, "fork_provenance")
    fork_tag = fork.get("tag")
    fork_commit = fork.get("commit")
    if not isinstance(fork_tag, str) or not isinstance(fork_commit, str):
        fail("stable-release fork provenance is invalid")
    if _local_tag_commit(root, fork_tag, label="fork provenance tag") != fork_commit:
        fail("fork provenance tag does not identify the audited base")
    if _remote_tag_commit(root, remote, fork_tag, label="fork provenance tag") != fork_commit:
        fail("remote fork provenance tag does not identify the audited base")

    ancestry = subprocess.run(
        ["git", "merge-base", "--is-ancestor", fork_commit, expected_commit],
        cwd=root,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if ancestry.returncode != 0:
        fail("audited fork commit is not an ancestor of the release")
    merge_commits = _run_git(
        root, "rev-list", "--min-parents=2", f"{fork_commit}..{expected_commit}"
    )
    if merge_commits:
        fail("stable-release history is not linear")
    _validate_dco(root, fork_commit, expected_commit)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check-policy")
    source = subparsers.add_parser("validate-source")
    source.add_argument("--expected-commit", required=True)
    source.add_argument("--release-tag", required=True)
    source.add_argument("--remote", default="origin")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        policy = load_policy()
        validate_policy(policy)
        if args.command == "check-policy":
            validate_release_workflow(policy)
            print(
                "stable-release policy passed: exact main/tag/fork/DCO provenance, "
                "authenticated repository audit, 11 external prerequisites"
            )
        else:
            validate_source(
                policy,
                ROOT,
                expected_commit=args.expected_commit,
                release_tag=args.release_tag,
                remote=args.remote,
            )
            print("stable-release source provenance passed")
    except ReleaseError as error:
        print(f"stable-release gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
