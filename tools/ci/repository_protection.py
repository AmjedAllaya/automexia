#!/usr/bin/env python3
"""Validate, audit, and safely apply Automexia's GitHub protection contract."""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Any, Callable, NamedTuple

import yaml


ROOT = Path(__file__).resolve().parents[2]
CODEOWNERS_PATH = ROOT / "CODEOWNERS"
POLICY_PATH = ROOT / ".github" / "repository-protection.json"
WORKFLOW_ROOT = ROOT / ".github" / "workflows"
MAX_POLICY_BYTES = 256 * 1024
MAX_API_BYTES = 2 * 1024 * 1024
MAX_API_SECONDS = 45
API_VERSION = "2026-03-10"
CODEOWNER_LOGIN = re.compile(
    r"@[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})(?:/[A-Za-z0-9_.-]{1,100})?\Z"
)
EXPECTED_REPOSITORY = "AmjedAllaya/automexia-terminal"
EXPECTED_WORKFLOWS = {
    "ci.yml": "CI",
    "f5-openssh-assurance.yml": "F5 controlled native OpenSSH assurance",
    "nightly.yml": "Deep assurance (manual)",
    "release.yml": "Stable release",
    "s1-assurance.yml": "S1 controlled assurance",
    "s2-assurance.yml": "S2 controlled activation",
}
EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS = [
    "ci.yml",
]
EXPECTED_REPOSITORY_SETTINGS = {
    "allow_auto_merge": False,
    "allow_merge_commit": False,
    "allow_rebase_merge": False,
    "allow_squash_merge": True,
    "allow_update_branch": True,
    "delete_branch_on_merge": True,
    "squash_merge_commit_message": "PR_BODY",
    "squash_merge_commit_title": "PR_TITLE",
    "web_commit_signoff_required": True,
}
EXPECTED_ACTION_PERMISSIONS = {
    "allowed_actions": "selected",
    "enabled": True,
    "sha_pinning_required": True,
}
EXPECTED_WORKFLOW_PERMISSIONS = {
    "can_approve_pull_request_reviews": False,
    "default_workflow_permissions": "read",
}
EXPECTED_SECURITY = {
    "artifact_attestations": "release-workflow",
    "codeql": "pinned-advanced-workflow",
    "dependabot_alerts": True,
    "dependabot_security_updates": True,
    "dependency_graph": True,
    "immutable_releases": True,
    "private_vulnerability_reporting": "required-when-public",
    "secret_scanning": "required-when-available",
    "secret_scanning_push_protection": "required-when-available",
}
FULL_ACTION_PIN = re.compile(r"[^\s@]+@[0-9a-f]{40}\Z")
REPOSITORY_NAME = re.compile(r"[A-Za-z0-9_.-]{1,100}/[A-Za-z0-9_.-]{1,100}\Z")


class ProtectionPolicyError(ValueError):
    """Raised when the repository-owned protection contract is invalid."""


class Finding(NamedTuple):
    identifier: str
    status: str
    detail: str


class GhResult(NamedTuple):
    returncode: int
    stdout: str
    stderr: str


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ProtectionPolicyError(f"repository protection JSON has duplicate key {key!r}")
        result[key] = value
    return result


def parse_json(raw: str) -> Any:
    try:
        return json.loads(raw, object_pairs_hook=reject_duplicate_keys)
    except json.JSONDecodeError as error:
        raise ProtectionPolicyError(f"invalid repository protection JSON: {error}") from error


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) > MAX_POLICY_BYTES:
        raise ProtectionPolicyError("repository protection policy exceeds 256 KiB")
    try:
        decoded = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProtectionPolicyError("repository protection policy must be UTF-8") from error
    policy = parse_json(decoded)
    if not isinstance(policy, dict):
        raise ProtectionPolicyError("repository protection policy must be an object")
    return policy


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ProtectionPolicyError(message)


def parse_codeowners(raw: str) -> list[str]:
    require("\x00" not in raw, "CODEOWNERS must not contain NUL bytes")
    owners: set[str] = set()
    wildcard_owners: set[str] = set()
    for line_number, line in enumerate(raw.splitlines(), start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        fields = stripped.split()
        require(
            len(fields) >= 2,
            f"CODEOWNERS line {line_number} must contain a pattern and owner",
        )
        pattern, *line_owners = fields
        require(
            all(CODEOWNER_LOGIN.fullmatch(owner) is not None for owner in line_owners),
            f"CODEOWNERS line {line_number} contains an invalid owner",
        )
        owners.update(line_owners)
        if pattern == "*":
            wildcard_owners.update(line_owners)
    require(wildcard_owners, "CODEOWNERS must include a repository-wide '*' owner")
    return sorted(owners)


def validate_codeowners(path: Path = CODEOWNERS_PATH) -> int:
    payload = path.read_bytes()
    require(len(payload) <= 64 * 1024, "CODEOWNERS exceeds 64 KiB")
    try:
        decoded = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProtectionPolicyError("CODEOWNERS must be UTF-8") from error
    return len(parse_codeowners(decoded))


def exact_mapping(value: Any, expected: dict[str, Any], label: str) -> None:
    require(isinstance(value, dict), f"{label} must be an object")
    require(value == expected, f"{label} must equal the fail-closed repository contract")


def rule_by_type(ruleset: dict[str, Any], rule_type: str) -> dict[str, Any] | None:
    rules = ruleset.get("rules", [])
    if not isinstance(rules, list):
        return None
    return next(
        (
            rule
            for rule in rules
            if isinstance(rule, dict) and rule.get("type") == rule_type
        ),
        None,
    )


def validate_main_ruleset(ruleset: dict[str, Any], required_checks: list[str]) -> None:
    require(ruleset.get("name") == "Protect main", "main ruleset name must be Protect main")
    require(ruleset.get("target") == "branch", "Protect main must target branches")
    require(ruleset.get("enforcement") == "active", "Protect main must be active")
    require(ruleset.get("bypass_actors") == [], "Protect main must have no bypass actors")
    require(
        ruleset.get("conditions")
        == {"ref_name": {"exclude": [], "include": ["~DEFAULT_BRANCH"]}},
        "Protect main must target only the default branch",
    )
    for required_type in (
        "deletion",
        "non_fast_forward",
        "required_linear_history",
        "required_signatures",
        "pull_request",
        "required_status_checks",
    ):
        require(
            rule_by_type(ruleset, required_type) is not None,
            f"Protect main is missing {required_type}",
        )

    pull_request = rule_by_type(ruleset, "pull_request") or {}
    exact_mapping(
        pull_request.get("parameters"),
        {
            "allowed_merge_methods": ["squash"],
            "dismiss_stale_reviews_on_push": True,
            "require_code_owner_review": True,
            "require_last_push_approval": True,
            "required_approving_review_count": 1,
            "required_review_thread_resolution": True,
        },
        "Protect main pull_request parameters",
    )

    status_rule = rule_by_type(ruleset, "required_status_checks") or {}
    parameters = status_rule.get("parameters")
    require(isinstance(parameters, dict), "Protect main status check parameters are missing")
    contexts = parameters.get("required_status_checks")
    require(isinstance(contexts, list), "Protect main required checks must be a list")
    actual_checks = [
        item.get("context") for item in contexts if isinstance(item, dict)
    ]
    require(
        actual_checks == required_checks,
        "Protect main status checks must exactly match hosted_ci required checks",
    )
    require(
        parameters.get("strict_required_status_checks_policy") is True,
        "Protect main required checks must use strict latest-base evaluation",
    )
    require(
        parameters.get("do_not_enforce_on_create") is False,
        "Protect main must not bypass checks on branch creation",
    )


def validate_tag_ruleset(ruleset: dict[str, Any]) -> None:
    require(
        ruleset.get("name") == "Protect release tags",
        "release tag ruleset name must be Protect release tags",
    )
    require(ruleset.get("target") == "tag", "Protect release tags must target tags")
    require(ruleset.get("enforcement") == "active", "Protect release tags must be active")
    require(
        ruleset.get("bypass_actors") == [],
        "Protect release tags must have no bypass actors",
    )
    require(
        ruleset.get("conditions")
        == {"ref_name": {"exclude": [], "include": ["refs/tags/v*"]}},
        "Protect release tags must target only v* tags",
    )
    for required_type in ("deletion", "non_fast_forward"):
        require(
            rule_by_type(ruleset, required_type) is not None,
            f"Protect release tags is missing {required_type}",
        )
    require(
        len(ruleset.get("rules", [])) == 2,
        "Protect release tags must contain only deletion and non-fast-forward rules",
    )


def workflow(name: str) -> dict[str, Any]:
    path = WORKFLOW_ROOT / name
    with path.open("r", encoding="utf-8") as source:
        value = yaml.safe_load(source)
    require(isinstance(value, dict), f"{path.relative_to(ROOT)} must be a mapping")
    require(value.get("name") == EXPECTED_WORKFLOWS[name], f"{name} workflow name drifted")
    require(isinstance(value.get("jobs"), dict), f"{name} must contain jobs")
    return value


def ci_check_names(ci: dict[str, Any], codeql: dict[str, Any]) -> list[str]:
    jobs = ci["jobs"]
    required_static = {
        "policy": "Policy and repository contracts",
        "coverage": "Coverage",
        "dependencies": "Dependency policy",
        "dependency-review": "Dependency review",
    }
    names: set[str] = set()
    for key, expected_name in required_static.items():
        value = jobs.get(key)
        require(isinstance(value, dict), f"ci.yml is missing {key}")
        require(value.get("name") == expected_name, f"ci.yml {key} job name drifted")
        names.add(expected_name)

    native = jobs.get("native")
    require(isinstance(native, dict), "ci.yml is missing native")
    require(native.get("name") == "Native ${{ matrix.os }}", "native job name drifted")
    native_os = native.get("strategy", {}).get("matrix", {}).get("os", [])
    require(isinstance(native_os, list), "native OS matrix must be a list")
    names.update(f"Native {item}" for item in native_os)

    linux = jobs.get("linux-features")
    require(isinstance(linux, dict), "ci.yml is missing linux-features")
    require(linux.get("name") == "Linux ${{ matrix.features }}", "Linux job name drifted")
    linux_include = linux.get("strategy", {}).get("matrix", {}).get("include", [])
    require(isinstance(linux_include, list), "Linux feature matrix must be a list")
    names.update(
        f"Linux {item['features']}"
        for item in linux_include
        if isinstance(item, dict) and isinstance(item.get("features"), str)
    )

    cross = jobs.get("cross-checks")
    require(isinstance(cross, dict), "ci.yml is missing cross-checks")
    require(cross.get("name") == "Cross-check ${{ matrix.target }}", "cross-check job name drifted")
    cross_include = cross.get("strategy", {}).get("matrix", {}).get("include", [])
    require(isinstance(cross_include, list), "cross-check matrix must be a list")
    names.update(
        f"Cross-check {item['target']}"
        for item in cross_include
        if isinstance(item, dict) and isinstance(item.get("target"), str)
    )

    codeql_job = codeql["jobs"].get("rust")
    require(isinstance(codeql_job, dict), "codeql.yml is missing rust")
    require(codeql_job.get("name") == "CodeQL / Rust", "CodeQL job name must be explicit and unique")
    names.add("CodeQL / Rust")
    return sorted(names)


def workflow_action_uses() -> list[str]:
    uses: list[str] = []
    for path in sorted(WORKFLOW_ROOT.glob("*.y*ml")):
        value = workflow(path.name)
        for job_value in value["jobs"].values():
            if not isinstance(job_value, dict):
                continue
            steps = job_value.get("steps", [])
            if not isinstance(steps, list):
                continue
            uses.extend(
                str(step["uses"])
                for step in steps
                if isinstance(step, dict) and isinstance(step.get("uses"), str)
            )
    return uses


def validate_action_allowlist(actions: dict[str, Any]) -> int:
    exact_mapping(
        actions.get("permissions"),
        EXPECTED_ACTION_PERMISSIONS,
        "Actions permissions",
    )
    exact_mapping(
        actions.get("workflow_permissions"),
        EXPECTED_WORKFLOW_PERMISSIONS,
        "workflow token permissions",
    )
    selected = actions.get("selected_actions")
    require(isinstance(selected, dict), "selected Actions policy must be an object")
    require(selected.get("github_owned_allowed") is True, "GitHub-owned Actions must remain allowed")
    require(selected.get("verified_allowed") is False, "verified publishers must not be broadly allowed")
    patterns = selected.get("patterns_allowed")
    require(isinstance(patterns, list) and patterns, "selected Actions patterns must be nonempty")
    require(
        patterns == sorted(set(patterns)),
        "selected Actions patterns must be unique and sorted",
    )
    require(
        all(isinstance(pattern, str) and pattern.endswith("@*") for pattern in patterns),
        "selected Actions patterns must name exact actions with @* revisions",
    )

    uses = workflow_action_uses()
    require(uses, "workflows must use at least one pinned Action")
    for action in uses:
        require(FULL_ACTION_PIN.fullmatch(action) is not None, f"Action is not SHA pinned: {action}")
        owner = action.split("/", 1)[0].casefold()
        if owner in {"actions", "github"}:
            continue
        require(
            any(fnmatch.fnmatchcase(action, pattern) for pattern in patterns),
            f"third-party Action is absent from the exact allowlist: {action}",
        )
    for pattern in patterns:
        require(
            any(fnmatch.fnmatchcase(action, pattern) for action in uses),
            f"selected Actions pattern is unused: {pattern}",
        )
    return len(patterns)


def validate_github_free_private_policy(policy: dict[str, Any]) -> dict[str, int]:
    """Validate the no-cost private-repository contract without paid controls."""
    expected_keys = {
        "actions_policy",
        "mode",
        "ordinary_ci",
        "paid_github_features_not_used",
        "release",
        "release_platforms",
        "schema",
        "server_side_paid_controls_required",
    }
    require(
        set(policy) == expected_keys,
        "GitHub-Free/private policy top-level keys drifted",
    )
    require(
        policy.get("schema") == 2 and policy.get("mode") == "github-free-private",
        "GitHub-Free/private policy identity drifted",
    )
    require(
        policy.get("server_side_paid_controls_required") is False,
        "GitHub-Free/private policy must not require paid server-side controls",
    )

    ordinary = policy.get("ordinary_ci")
    require(isinstance(ordinary, dict), "ordinary_ci must be an object")
    require(
        set(ordinary)
        == {
            "hosted_os",
            "reason",
            "release_branch_extra_check",
            "required_checks",
        },
        "ordinary_ci keys drifted",
    )
    require(
        ordinary.get("hosted_os") == ["ubuntu-24.04"],
        "ordinary CI must use only the GitHub-Free Ubuntu runner",
    )
    expected_checks = {
        "Repository and workflow policy",
        "Rust quality and tests",
        "Dependency security",
    }
    required_checks = ordinary.get("required_checks")
    require(
        isinstance(required_checks, list)
        and set(required_checks) == expected_checks
        and len(required_checks) == len(expected_checks),
        "GitHub-Free ordinary required checks drifted",
    )
    ci = workflow("ci.yml")
    actual_checks = {
        str(value.get("name", ""))
        for name, value in ci.get("jobs", {}).items()
        if isinstance(value, dict)
        and name not in {"release-candidate", "release-candidate-coverage"}
    }
    require(
        actual_checks == expected_checks,
        "GitHub-Free ordinary required checks must match ci.yml",
    )
    require(
        ordinary.get("release_branch_extra_check") == "Release candidate gate",
        "GitHub-Free release candidate check drifted",
    )
    require(
        isinstance(ordinary.get("reason"), str) and ordinary["reason"],
        "GitHub-Free ordinary CI must retain its bounded-runner rationale",
    )

    release = policy.get("release")
    require(isinstance(release, dict), "release policy must be an object")
    expected_release = {
        "default_minimum_human_approvals": 1,
        "default_require_distinct_merger": True,
        "distinct_merger_variable": "AUTOMEXIA_RELEASE_REQUIRE_DISTINCT_MERGER",
        "minimum_approvals_variable": "AUTOMEXIA_RELEASE_MIN_APPROVALS",
        "must_equal_current_main": True,
        "same_repository_only": True,
        "stable_semver_only": True,
        "trigger": "merged internal release/X.Y.Z pull request into main",
        "workflow_creates_tag_after_all_gates": True,
    }
    for key, expected in expected_release.items():
        require(release.get(key) == expected, f"GitHub-Free release {key} drifted")
    protected_paths = release.get("protected_release_pr_paths")
    require(
        protected_paths
        == [
            ".github/",
            "tools/ci/",
            "tools/xtask/",
            "packaging/",
            "shell-integration/",
        ],
        "GitHub-Free protected release paths drifted",
    )
    require(
        set(release) == set(expected_release) | {"protected_release_pr_paths"},
        "GitHub-Free release policy keys drifted",
    )

    paid_features = policy.get("paid_github_features_not_used")
    require(
        paid_features
        == [
            "private artifact attestations",
            "private protected-environment reviewers",
            "GitHub Code Security dependency review",
            "private CodeQL result upload",
            "private ruleset enforcement",
        ],
        "GitHub-Free excluded feature inventory drifted",
    )
    require(
        policy.get("release_platforms")
        == [
            "windows-2025 x86_64",
            "windows-11-arm aarch64",
            "ubuntu-22.04 x86_64 build baseline",
            "ubuntu-22.04-arm aarch64 build baseline",
            "macos-26-intel x86_64",
            "macos-26 arm64",
        ],
        "GitHub-Free release platform inventory drifted",
    )

    actions_policy = policy.get("actions_policy")
    expected_patterns = [
        "anchore/sbom-action@*",
        "azure/artifact-signing-action@*",
        "azure/login@*",
        "taiki-e/install-action@*",
    ]
    require(
        isinstance(actions_policy, dict)
        and actions_policy
        == {
            "mode": "selected-actions",
            "github_owned_actions_allowed": True,
            "third_party_patterns": expected_patterns,
            "full_length_sha_required": True,
            "zizmor_delivery": (
                "zizmor@1.21.0 via taiki-e/install-action; "
                "no zizmorcore/zizmor-action use"
            ),
        },
        "GitHub-Free selected Action policy drifted",
    )
    uses = workflow_action_uses()
    require(uses, "workflows must use at least one pinned Action")
    for action in uses:
        require(
            FULL_ACTION_PIN.fullmatch(action) is not None,
            f"Action is not SHA pinned: {action}",
        )
        owner = action.split("/", 1)[0].casefold()
        if owner not in {"actions", "github"}:
            require(
                any(fnmatch.fnmatchcase(action, pattern) for pattern in expected_patterns),
                f"third-party Action is absent from the GitHub-Free allowlist: {action}",
            )
    return {
        "action_patterns": len(expected_patterns),
        "required_checks": len(expected_checks),
        "rulesets": 0,
    }


def validate_policy(policy: dict[str, Any]) -> dict[str, int]:
    if policy.get("mode") == "github-free-private":
        return validate_github_free_private_policy(policy)
    expected_keys = {
        "actions",
        "default_branch",
        "hosted_ci",
        "repository",
        "repository_settings",
        "review",
        "rulesets",
        "schema",
        "security",
    }
    require(set(policy) == expected_keys, "repository protection top-level keys drifted")
    require(policy.get("schema") == 1, "repository protection schema must be 1")
    require(
        policy.get("repository") == EXPECTED_REPOSITORY,
        "repository identity must remain AmjedAllaya/automexia-terminal",
    )
    require(policy.get("default_branch") == "main", "protected default branch must remain main")
    exact_mapping(
        policy.get("repository_settings"),
        EXPECTED_REPOSITORY_SETTINGS,
        "repository merge settings",
    )
    action_patterns = validate_action_allowlist(policy.get("actions", {}))
    exact_mapping(policy.get("security"), EXPECTED_SECURITY, "security feature policy")
    require(
        policy["security"].get("secret_scanning") == "required-when-available"
        and policy["security"].get("secret_scanning_push_protection")
        == "required-when-available",
        "secret scanning and push protection must fail closed when available",
    )

    hosted = policy.get("hosted_ci")
    require(isinstance(hosted, dict), "hosted_ci must be an object")
    require(
        set(hosted)
        == {
            "default_branch_evidence_workflows",
            "required_pull_request_checks",
            "required_workflows",
            "success_recency_days",
        },
        "hosted_ci keys drifted",
    )
    require(
        hosted.get("default_branch_evidence_workflows")
        == EXPECTED_DEFAULT_BRANCH_EVIDENCE_WORKFLOWS,
        "default-branch evidence workflows drifted",
    )
    require(
        hosted.get("required_workflows") == list(EXPECTED_WORKFLOWS.values()),
        "required hosted workflows drifted",
    )
    required_checks = hosted.get("required_pull_request_checks")
    require(
        isinstance(required_checks, list)
        and required_checks == sorted(set(required_checks)),
        "required checks must be unique and sorted",
    )
    require(hosted.get("success_recency_days") == 7, "hosted success recency must be seven days")
    actual_checks = ci_check_names(workflow("ci.yml"), workflow("codeql.yml"))
    require(
        required_checks == actual_checks,
        "required checks must exactly match all CI and CodeQL pull-request job names",
    )

    review = policy.get("review")
    exact_mapping(
        review,
        {
            "minimum_independent_approvals_for_protected_paths": 2,
            "minimum_total_human_reviewers": 3,
        },
        "protected review capacity",
    )
    rulesets = policy.get("rulesets")
    require(isinstance(rulesets, list) and len(rulesets) == 2, "exactly two rulesets are required")
    require(all(isinstance(item, dict) for item in rulesets), "rulesets must be objects")
    validate_main_ruleset(rulesets[0], required_checks)
    validate_tag_ruleset(rulesets[1])
    return {
        "action_patterns": action_patterns,
        "required_checks": len(required_checks),
        "rulesets": len(rulesets),
    }


def validate_repository(policy: dict[str, Any] | None = None) -> dict[str, int]:
    counts = validate_policy(policy or load_policy())
    counts["codeowners"] = validate_codeowners()
    return counts


def classify_api_error(status: int, message: str) -> str:
    normalized = message.casefold()
    if status == 403 and "upgrade to github pro or make this repository public" in normalized:
        return "external-plan"
    if status in {402, 403} and ("payment" in normalized or "spending limit" in normalized):
        return "external-billing"
    if status in {401, 403}:
        return "external-permission"
    if status == 404:
        return "unavailable"
    return "error"


def classify_hosted_job(job: dict[str, Any]) -> str:
    conclusion = str(job.get("conclusion", "")).casefold()
    steps = job.get("steps", [])
    annotation = str(job.get("annotation", "")).casefold()
    if (
        conclusion == "failure"
        and job.get("runner_id") in {0, None}
        and steps == []
        and ("payments have failed" in annotation or "spending limit" in annotation)
    ):
        return "external-billing"
    if conclusion == "success":
        return "pass"
    if conclusion == "skipped":
        return "skip"
    return "fail"


def hosted_run_is_recent(
    run: dict[str, Any], recency_days: int, *, now: dt.datetime | None = None
) -> bool:
    timestamp = run.get("updated_at") or run.get("created_at")
    if not isinstance(timestamp, str):
        return False
    try:
        measured = dt.datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
    except ValueError:
        return False
    if measured.tzinfo is None:
        return False
    current = now or dt.datetime.now(dt.timezone.utc)
    age = current.astimezone(dt.timezone.utc) - measured.astimezone(dt.timezone.utc)
    return dt.timedelta(0) <= age <= dt.timedelta(days=recency_days)


def hosted_run_matches_head(run: dict[str, Any], expected_head_sha: str) -> bool:
    head_sha = run.get("head_sha")
    return (
        isinstance(head_sha, str)
        and re.fullmatch(r"[0-9a-f]{40,64}", head_sha) is not None
        and head_sha == expected_head_sha
    )


def hosted_jobs_pass(classified: list[str]) -> bool:
    return bool(classified) and all(item == "pass" for item in classified)


def recursive_subset(expected: Any, actual: Any) -> bool:
    if isinstance(expected, dict):
        return isinstance(actual, dict) and all(
            key in actual and recursive_subset(value, actual[key])
            for key, value in expected.items()
        )
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(expected) != len(actual):
            return False
        if all(isinstance(item, dict) and "type" in item for item in expected):
            actual_by_type = {
                item.get("type"): item for item in actual if isinstance(item, dict)
            }
            return all(
                item["type"] in actual_by_type
                and recursive_subset(item, actual_by_type[item["type"]])
                for item in expected
            )
        return all(recursive_subset(left, right) for left, right in zip(expected, actual))
    return expected == actual


def rulesets_match(expected: list[dict[str, Any]], actual: Any) -> bool:
    if not isinstance(actual, list):
        return False
    actual_by_name = {
        item.get("name"): item for item in actual if isinstance(item, dict)
    }
    return all(
        item["name"] in actual_by_name
        and recursive_subset(item, actual_by_name[item["name"]])
        for item in expected
    )


def synthetic_compliant_snapshot(policy: dict[str, Any]) -> dict[str, Any]:
    return {
        "actions_permissions": copy_json(policy["actions"]["permissions"]),
        "automated_security_fixes": True,
        "hosted_ci": "pass",
        "hosted_workflows_active": True,
        "human_reviewer_count": policy["review"]["minimum_total_human_reviewers"],
        "immutable_releases": True,
        "private_vulnerability_reporting": True,
        "repository_settings": copy_json(policy["repository_settings"]),
        "rulesets": copy_json(policy["rulesets"]),
        "rulesets_availability": "available",
        "secret_scanning": True,
        "secret_scanning_availability": "available",
        "secret_scanning_push_protection": True,
        "selected_actions": copy_json(policy["actions"]["selected_actions"]),
        "visibility": "public",
        "vulnerability_alerts": True,
        "workflow_permissions": copy_json(policy["actions"]["workflow_permissions"]),
    }


def copy_json(value: Any) -> Any:
    return json.loads(json.dumps(value))


def finding(identifier: str, passed: bool, pass_detail: str, fail_detail: str) -> Finding:
    return Finding(identifier, "pass" if passed else "fail", pass_detail if passed else fail_detail)


def evaluate_remote_snapshot(policy: dict[str, Any], snapshot: dict[str, Any]) -> list[Finding]:
    findings = [
        finding(
            "repository-settings",
            recursive_subset(policy["repository_settings"], snapshot.get("repository_settings")),
            "squash-only, DCO, update, and cleanup settings match",
            "repository merge/DCO settings drift from policy",
        ),
        finding(
            "actions-permissions",
            recursive_subset(policy["actions"]["permissions"], snapshot.get("actions_permissions")),
            "Actions use selected publishers and full-SHA enforcement",
            "Actions execution permissions drift from policy",
        ),
        finding(
            "selected-actions",
            recursive_subset(policy["actions"]["selected_actions"], snapshot.get("selected_actions")),
            "third-party Action allowlist matches",
            "selected Action allowlist drifts from policy",
        ),
        finding(
            "workflow-token",
            recursive_subset(policy["actions"]["workflow_permissions"], snapshot.get("workflow_permissions")),
            "workflow token defaults to read and cannot approve reviews",
            "workflow token permissions drift from policy",
        ),
        finding(
            "dependabot-alerts",
            snapshot.get("vulnerability_alerts") is True,
            "dependency graph and Dependabot alerts are enabled",
            "dependency graph or Dependabot alerts are disabled",
        ),
        finding(
            "dependabot-security-updates",
            snapshot.get("automated_security_fixes") is True,
            "Dependabot security updates are enabled",
            "Dependabot security updates are disabled",
        ),
        finding(
            "immutable-releases",
            snapshot.get("immutable_releases") is True,
            "future releases are immutable",
            "immutable releases are disabled",
        ),
        finding(
            "hosted-workflows",
            snapshot.get("hosted_workflows_active") is True,
            "all required workflows are active",
            "one or more required workflows are absent or disabled",
        ),
    ]

    ruleset_availability = snapshot.get("rulesets_availability")
    if ruleset_availability == "external-plan":
        findings.append(Finding("rulesets", "external", "private repository plan cannot enforce rulesets"))
    elif ruleset_availability != "available":
        findings.append(Finding("rulesets", "fail", "rulesets could not be audited"))
    else:
        findings.append(
            finding(
                "rulesets",
                rulesets_match(policy["rulesets"], snapshot.get("rulesets")),
                "main and release-tag rulesets match with no bypass",
                "main or release-tag ruleset drifts from policy",
            )
        )

    reviewer_count = snapshot.get("human_reviewer_count", 0)
    required_reviewers = policy["review"]["minimum_total_human_reviewers"]
    if isinstance(reviewer_count, int) and reviewer_count >= required_reviewers:
        findings.append(Finding("reviewer-capacity", "pass", "independent protected-path reviewer pool exists"))
    else:
        findings.append(Finding("reviewer-capacity", "external", "fewer than three human collaborators are available"))

    hosted = snapshot.get("hosted_ci")
    if hosted in {"external-billing", "external-no-run"}:
        findings.append(Finding("hosted-ci", "external", "hosted jobs cannot provide executed current evidence"))
    else:
        findings.append(
            finding(
                "hosted-ci",
                hosted == "pass",
                "current hosted CI, CodeQL, and Nightly runs executed successfully",
                "current hosted CI, CodeQL, or Nightly evidence is failing or stale",
            )
        )

    visibility = str(snapshot.get("visibility", "")).casefold()
    if visibility == "public":
        findings.append(
            finding(
                "private-vulnerability-reporting",
                snapshot.get("private_vulnerability_reporting") is True,
                "private vulnerability reporting is enabled",
                "public repository private vulnerability reporting is disabled",
            )
        )
    else:
        findings.append(
            Finding(
                "private-vulnerability-reporting",
                "external",
                "GitHub private vulnerability reporting is available only after public visibility",
            )
        )

    scanning_available = snapshot.get("secret_scanning_availability") == "available"
    if not scanning_available:
        findings.append(
            Finding(
                "secret-scanning",
                "external",
                "private repository lacks Secret Protection entitlement",
            )
        )
    else:
        findings.append(
            finding(
                "secret-scanning",
                snapshot.get("secret_scanning") is True
                and snapshot.get("secret_scanning_push_protection") is True,
                "secret scanning and push protection are enabled",
                "secret scanning or push protection is disabled",
            )
        )
    return findings


def default_runner(args: list[str], payload: str | None = None) -> GhResult:
    executable = shutil.which("gh")
    if executable is None:
        return GhResult(127, "", "GitHub CLI is not installed")
    command = [executable, *args]
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            input=payload,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=MAX_API_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return GhResult(124, "", "GitHub API request timed out")
    stdout = completed.stdout[: MAX_API_BYTES + 1]
    stderr = completed.stderr[: MAX_API_BYTES + 1]
    if len(stdout.encode("utf-8")) > MAX_API_BYTES or len(stderr.encode("utf-8")) > MAX_API_BYTES:
        return GhResult(1, "", "GitHub API response exceeded 2 MiB")
    return GhResult(completed.returncode, stdout, stderr)


def api(
    runner: Callable[[list[str], str | None], GhResult],
    endpoint: str,
    *,
    method: str = "GET",
    payload: Any | None = None,
) -> tuple[GhResult, Any | None]:
    args = [
        "api",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        f"X-GitHub-Api-Version: {API_VERSION}",
    ]
    if method != "GET":
        args.extend(["--method", method])
    encoded = None
    if payload is not None:
        args.extend(["--input", "-"])
        encoded = json.dumps(payload, separators=(",", ":"))
    args.append(endpoint)
    result = runner(args, encoded)
    parsed = None
    if result.stdout.strip():
        try:
            parsed = json.loads(result.stdout)
        except json.JSONDecodeError:
            parsed = None
    return result, parsed


def error_text(result: GhResult, parsed: Any) -> str:
    if isinstance(parsed, dict) and isinstance(parsed.get("message"), str):
        return parsed["message"]
    return result.stderr.strip()[:500]


def status_from_error(result: GhResult, parsed: Any) -> str:
    message = error_text(result, parsed)
    match = re.search(r"HTTP\s+(\d{3})", result.stderr)
    status = int(match.group(1)) if match else (403 if result.returncode != 0 else 200)
    return classify_api_error(status, message)


def normalize_selected_actions(value: Any) -> Any:
    if not isinstance(value, dict):
        return value
    normalized = dict(value)
    patterns = normalized.get("patterns_allowed")
    if isinstance(patterns, str):
        normalized["patterns_allowed"] = sorted(
            item.strip() for item in patterns.split(",") if item.strip()
        )
    return normalized


def selected_actions_apply_payload(policy: dict[str, Any]) -> dict[str, Any]:
    return copy_json(policy["actions"]["selected_actions"])


def security_status(repository: dict[str, Any], key: str) -> bool | None:
    security = repository.get("security_and_analysis")
    if not isinstance(security, dict):
        return None
    value = security.get(key)
    if not isinstance(value, dict):
        return None
    return value.get("status") == "enabled"


def active_workflows(
    runner: Callable[[list[str], str | None], GhResult], repository: str
) -> bool:
    result, value = api(runner, f"repos/{repository}/actions/workflows?per_page=100")
    if result.returncode != 0 or not isinstance(value, dict):
        return False
    workflows = value.get("workflows", [])
    actual = {
        item.get("name")
        for item in workflows
        if isinstance(item, dict) and item.get("state") == "active"
    }
    return set(EXPECTED_WORKFLOWS.values()).issubset(actual)


def hosted_run_state(
    runner: Callable[[list[str], str | None], GhResult],
    repository: str,
    default_branch: str,
    expected_head_sha: str,
    recency_days: int,
    workflow_files: list[str],
) -> str:
    states: list[str] = []
    for workflow_file in workflow_files:
        endpoint = (
            f"repos/{repository}/actions/workflows/{workflow_file}/runs"
            f"?branch={default_branch}&per_page=1"
        )
        result, value = api(runner, endpoint)
        if result.returncode != 0 or not isinstance(value, dict):
            states.append("external-no-run")
            continue
        runs = value.get("workflow_runs", [])
        if not isinstance(runs, list) or not runs:
            states.append("external-no-run")
            continue
        run = runs[0]
        if not isinstance(run, dict) or not isinstance(run.get("id"), int):
            states.append("external-no-run")
            continue
        jobs_result, jobs_value = api(
            runner, f"repos/{repository}/actions/runs/{run['id']}/jobs?per_page=100"
        )
        if jobs_result.returncode != 0 or not isinstance(jobs_value, dict):
            states.append("fail")
            continue
        jobs = jobs_value.get("jobs", [])
        if not isinstance(jobs, list) or not jobs:
            states.append("fail")
            continue
        classified: list[str] = []
        for raw_job in jobs:
            if not isinstance(raw_job, dict) or raw_job.get("conclusion") == "skipped":
                continue
            job = dict(raw_job)
            if job.get("conclusion") == "failure" and job.get("steps") == []:
                check_url = str(job.get("check_run_url", ""))
                check_id = check_url.rsplit("/", 1)[-1]
                if check_id.isdigit():
                    annotation_result, annotations = api(
                        runner,
                        f"repos/{repository}/check-runs/{check_id}/annotations?per_page=1",
                    )
                    if annotation_result.returncode == 0 and isinstance(annotations, list) and annotations:
                        first = annotations[0]
                        if isinstance(first, dict):
                            job["annotation"] = str(first.get("message", ""))
            classified.append(classify_hosted_job(job))
        if "external-billing" in classified:
            states.append("external-billing")
        elif (
            run.get("conclusion") == "success"
            and hosted_jobs_pass(classified)
            and hosted_run_matches_head(run, expected_head_sha)
            and hosted_run_is_recent(run, recency_days)
        ):
            states.append("pass")
        else:
            states.append("fail")
    if "external-billing" in states:
        return "external-billing"
    if "external-no-run" in states:
        return "external-no-run"
    return "pass" if states and all(item == "pass" for item in states) else "fail"


def audit_snapshot(
    policy: dict[str, Any],
    runner: Callable[[list[str], str | None], GhResult] = default_runner,
) -> dict[str, Any]:
    repository = policy["repository"]
    repo_result, repository_value = api(runner, f"repos/{repository}")
    if repo_result.returncode != 0 or not isinstance(repository_value, dict):
        raise ProtectionPolicyError("could not read the configured GitHub repository")
    if not repository_value.get("permissions", {}).get("admin"):
        raise ProtectionPolicyError("authenticated GitHub identity lacks repository admin access")

    snapshot: dict[str, Any] = {
        "repository_settings": {
            key: repository_value.get(key) for key in policy["repository_settings"]
        },
        "visibility": repository_value.get("visibility", ""),
        "hosted_workflows_active": active_workflows(runner, repository),
    }
    for key, endpoint in (
        ("actions_permissions", f"repos/{repository}/actions/permissions"),
        ("workflow_permissions", f"repos/{repository}/actions/permissions/workflow"),
    ):
        result, value = api(runner, endpoint)
        snapshot[key] = value if result.returncode == 0 else None
    selected_result, selected_value = api(
        runner, f"repos/{repository}/actions/permissions/selected-actions"
    )
    snapshot["selected_actions"] = (
        normalize_selected_actions(selected_value)
        if selected_result.returncode == 0
        else None
    )

    alerts_result, _ = api(runner, f"repos/{repository}/vulnerability-alerts")
    snapshot["vulnerability_alerts"] = alerts_result.returncode == 0
    fixes_result, fixes = api(runner, f"repos/{repository}/automated-security-fixes")
    snapshot["automated_security_fixes"] = (
        fixes_result.returncode == 0
        and isinstance(fixes, dict)
        and fixes.get("enabled") is True
    )
    immutable_result, immutable = api(runner, f"repos/{repository}/immutable-releases")
    snapshot["immutable_releases"] = (
        immutable_result.returncode == 0
        and isinstance(immutable, dict)
        and immutable.get("enabled") is True
    )

    private_result, private_reporting = api(
        runner, f"repos/{repository}/private-vulnerability-reporting"
    )
    snapshot["private_vulnerability_reporting"] = (
        private_result.returncode == 0
        and isinstance(private_reporting, dict)
        and private_reporting.get("enabled") is True
    )
    secret = security_status(repository_value, "secret_scanning")
    push = security_status(repository_value, "secret_scanning_push_protection")
    snapshot["secret_scanning_availability"] = (
        "available" if secret is not None and push is not None else "external-license"
    )
    snapshot["secret_scanning"] = secret is True
    snapshot["secret_scanning_push_protection"] = push is True

    ruleset_result, ruleset_values = api(runner, f"repos/{repository}/rulesets")
    if ruleset_result.returncode != 0:
        snapshot["rulesets_availability"] = status_from_error(
            ruleset_result, ruleset_values
        )
        snapshot["rulesets"] = []
    else:
        snapshot["rulesets_availability"] = "available"
        detailed: list[Any] = []
        for summary in ruleset_values if isinstance(ruleset_values, list) else []:
            if not isinstance(summary, dict) or not isinstance(summary.get("id"), int):
                continue
            detail_result, detail = api(
                runner, f"repos/{repository}/rulesets/{summary['id']}"
            )
            if detail_result.returncode == 0 and isinstance(detail, dict):
                detailed.append(detail)
        snapshot["rulesets"] = detailed

    collaborators_result, collaborators = api(
        runner, f"repos/{repository}/collaborators?affiliation=direct&per_page=100"
    )
    snapshot["human_reviewer_count"] = (
        sum(
            1
            for item in collaborators
            if isinstance(item, dict) and item.get("type") == "User"
        )
        if collaborators_result.returncode == 0 and isinstance(collaborators, list)
        else 0
    )
    branch_result, branch = api(
        runner, f"repos/{repository}/branches/{policy['default_branch']}"
    )
    expected_head_sha = ""
    if branch_result.returncode == 0 and isinstance(branch, dict):
        commit = branch.get("commit")
        if isinstance(commit, dict) and isinstance(commit.get("sha"), str):
            expected_head_sha = commit["sha"]
    snapshot["hosted_ci"] = hosted_run_state(
        runner,
        repository,
        policy["default_branch"],
        expected_head_sha,
        policy["hosted_ci"]["success_recency_days"],
        policy["hosted_ci"]["default_branch_evidence_workflows"],
    )
    return snapshot


def require_api_success(result: GhResult, parsed: Any, operation: str) -> None:
    if result.returncode != 0:
        classification = status_from_error(result, parsed)
        raise ProtectionPolicyError(f"{operation} failed ({classification})")


def apply_policy(
    policy: dict[str, Any],
    repository: str,
    runner: Callable[[list[str], str | None], GhResult] = default_runner,
) -> list[str]:
    require(repository == policy["repository"], "apply confirmation must match repository identity")
    repo_result, repository_value = api(runner, f"repos/{repository}")
    require_api_success(repo_result, repository_value, "repository administration check")
    require(
        isinstance(repository_value, dict)
        and repository_value.get("permissions", {}).get("admin") is True,
        "apply requires repository administrator access",
    )
    applied: list[str] = []

    result, value = api(
        runner,
        f"repos/{repository}",
        method="PATCH",
        payload=policy["repository_settings"],
    )
    require_api_success(result, value, "repository merge settings")
    applied.append("repository-settings")

    for identifier, endpoint, payload in (
        (
            "actions-permissions",
            f"repos/{repository}/actions/permissions",
            policy["actions"]["permissions"],
        ),
        (
            "selected-actions",
            f"repos/{repository}/actions/permissions/selected-actions",
            selected_actions_apply_payload(policy),
        ),
        (
            "workflow-token",
            f"repos/{repository}/actions/permissions/workflow",
            policy["actions"]["workflow_permissions"],
        ),
    ):
        result, value = api(runner, endpoint, method="PUT", payload=payload)
        require_api_success(result, value, identifier)
        applied.append(identifier)

    for identifier, endpoint in (
        ("dependabot-alerts", f"repos/{repository}/vulnerability-alerts"),
        ("dependabot-security-updates", f"repos/{repository}/automated-security-fixes"),
        ("immutable-releases", f"repos/{repository}/immutable-releases"),
    ):
        result, value = api(runner, endpoint, method="PUT")
        require_api_success(result, value, identifier)
        applied.append(identifier)

    visibility = str(repository_value.get("visibility", "")).casefold()
    secret_available = security_status(repository_value, "secret_scanning") is not None
    if visibility == "public":
        result, value = api(
            runner,
            f"repos/{repository}/private-vulnerability-reporting",
            method="PUT",
        )
        require_api_success(result, value, "private vulnerability reporting")
        applied.append("private-vulnerability-reporting")
    if visibility == "public" or secret_available:
        result, value = api(
            runner,
            f"repos/{repository}",
            method="PATCH",
            payload={
                "security_and_analysis": {
                    "secret_scanning": {"status": "enabled"},
                    "secret_scanning_push_protection": {"status": "enabled"},
                }
            },
        )
        require_api_success(result, value, "secret scanning and push protection")
        applied.append("secret-scanning")

    ruleset_result, ruleset_values = api(runner, f"repos/{repository}/rulesets")
    if ruleset_result.returncode == 0:
        existing = {
            item.get("name"): item
            for item in ruleset_values
            if isinstance(item, dict) and isinstance(item.get("id"), int)
        } if isinstance(ruleset_values, list) else {}
        for expected in policy["rulesets"]:
            current = existing.get(expected["name"])
            if current is None:
                endpoint = f"repos/{repository}/rulesets"
                method = "POST"
            else:
                endpoint = f"repos/{repository}/rulesets/{current['id']}"
                method = "PUT"
            result, value = api(runner, endpoint, method=method, payload=expected)
            require_api_success(result, value, f"ruleset {expected['name']}")
        applied.append("rulesets")
    elif status_from_error(ruleset_result, ruleset_values) != "external-plan":
        require_api_success(ruleset_result, ruleset_values, "ruleset availability")
    return applied


def render_findings(findings: list[Finding], as_json: bool) -> None:
    if as_json:
        print(
            json.dumps(
                [finding._asdict() for finding in findings],
                indent=2,
                sort_keys=True,
            )
        )
        return
    for item in findings:
        print(f"{item.status.upper():8} {item.identifier}: {item.detail}")


def exit_for_findings(findings: list[Finding]) -> int:
    if any(item.status == "fail" for item in findings):
        return 1
    if any(item.status == "external" for item in findings):
        return 2
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("check", help="validate repository-owned policy and workflows")
    audit_parser = subparsers.add_parser("audit", help="audit authenticated GitHub state")
    audit_parser.add_argument("--json", action="store_true", dest="as_json")
    apply_parser = subparsers.add_parser("apply", help="apply reversible available settings")
    apply_parser.add_argument("--confirm-repository", required=True)
    apply_parser.add_argument("--json", action="store_true", dest="as_json")
    args = parser.parse_args()

    try:
        policy = load_policy()
        counts = validate_repository(policy)
        if args.command == "check":
            print(
                "repository protection contract passed: "
                f"{counts['rulesets']} rulesets, {counts['required_checks']} required checks, "
                f"{counts['action_patterns']} third-party Action patterns, "
                f"{counts['codeowners']} CODEOWNERS identities"
            )
            return 0
        if policy.get("mode") == "github-free-private":
            if args.command == "apply":
                raise ProtectionPolicyError(
                    "GitHub-Free/private governance must be configured manually; "
                    "this tool will not simulate unavailable paid controls"
                )
            findings = [
                Finding(
                    "github-free-manual-governance",
                    "external",
                    "independent release review and authenticated repository audit "
                    "remain external on GitHub Free/private",
                )
            ]
            render_findings(findings, args.as_json)
            return exit_for_findings(findings)
        if args.command == "apply":
            if REPOSITORY_NAME.fullmatch(args.confirm_repository) is None:
                parser.error("--confirm-repository must be an owner/repository name")
            applied = apply_policy(policy, args.confirm_repository)
            if not args.as_json:
                print("applied available settings: " + ", ".join(applied))
        snapshot = audit_snapshot(policy)
        findings = evaluate_remote_snapshot(policy, snapshot)
        render_findings(findings, args.as_json)
        return exit_for_findings(findings)
    except (OSError, ProtectionPolicyError) as error:
        print(f"repository protection error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
