#!/usr/bin/env python3
"""Validate redacted S1 native, visual, resource, and accessibility evidence.

The repository owns the bounded schema and release decision. Controlled native
runners and human reviewers own the actual evidence. This tool never launches
the application, installs assistive technology, captures pixels, or uploads
artifacts.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
from urllib.parse import urlsplit
import tempfile
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests/assurance/s1-assurance-policy-v1.json"
MAX_MANIFEST_BYTES = 1_048_576
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}(?:[0-9a-f]{24})?$")
IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,95}$")
SAFE_METADATA = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+() /:-]{0,191}$")
DOMAINS = {"native", "resource", "visual", "accessibility"}
REQUIRED_NATIVE_SCENARIOS = {
    "startup-prompt",
    "multiple-panes-tabs",
    "unicode-path",
    "output-burst",
    "history-navigation",
    "minimize-restore",
    "scale-change",
    "tiny-viewport",
    "large-viewport",
    "final-grid-invariants",
    "route-recovery",
    "command-shape-matrix",
    "command-result-height-boundaries",
    "command-result-viewport-overflow",
    "modal-interaction-matrix",
    "clipboard-ime-pointer",
    "compact-window-chrome",
    "command-palette-scroll",
    "connection-hub-direct-entry",
    "command-result-datetime",
    "user-preference-restart",
    "command-boundary-navigation",
    "connection-hub-workspaces-review",
    "connection-hub-providers-review",
}
REQUIRED_RESOURCE_SCENARIOS = {
    "tab-split-clone-close",
    "resize-storm",
    "process-tree-cleanup",
    "handle-thread-stability",
    "private-working-set-stability",
    "pty-route-snapshot-gpu-release",
    "scoped-search-switching",
    "connection-hub-open-close",
    "quick-actions-cancel",
    "image-preview-lifecycle",
    "long-output-storm",
    "command-palette-scroll-storm",
    "connection-hub-direct-entry",
    "user-preference-restart",
    "command-boundary-navigation",
    "connection-hub-workspaces-replacement",
    "connection-hub-providers-replacement",
}
REQUIRED_VISUAL_THEMES = {"dark", "light", "high-contrast"}
REQUIRED_VISUAL_SCALES = {"1.0", "1.25", "1.5", "2.0", "3.0", "4.0"}
REQUIRED_VISUAL_MOTION_PROFILES = {"enabled", "reduced"}
REQUIRED_VISUAL_VIEWPORTS = {
    "300x200",
    "compact",
    "normal",
    "portrait",
    "ultrawide",
    "split",
    "4k",
    "8k",
}
REQUIRED_VISUAL_SURFACES = {
    "startup-welcome",
    "tabs-and-panes",
    "window-controls",
    "command-palette",
    "scoped-search-local",
    "scoped-search-global",
    "connection-hub-catalog",
    "connection-hub-review",
    "connection-hub-workspaces-catalog",
    "connection-hub-workspace-restore",
    "connection-hub-workspace-broadcast",
    "connection-hub-providers-review",
    "quick-actions",
    "compatibility-inspector",
    "confirmation-dialogs",
    "file-listing",
    "command-result-short",
    "command-result-multiline",
    "command-result-viewport-overflow",
    "image-preview",
    "context-identity",
    "path",
    "footer",
    "smallest-pane",
    "compact-top-shelf",
    "diagnostic-assistant",
    "tab-appearance-picker",
    "command-palette-overflow",
    "connection-hub-setup",
    "connection-hub-direct-entry",
    "command-result-datetime",
    "scrollbar",
    "saved-preferences-restart",
}
REQUIRED_ACCESSIBILITY_TASKS = {
    "launch",
    "identify-tabs-active-pane",
    "open-filter-activate-dismiss-palette",
    "split-select-close",
    "keyboard-only",
    "focus-order-restoration",
    "200-percent-scale",
    "known-limitations-confirmed",
    "switch-scoped-search",
    "connection-hub-navigation",
    "connection-hub-workspaces-review",
    "connection-hub-providers-review",
    "quick-actions-navigation",
    "modal-stack-focus",
    "window-controls",
    "command-result-announcement",
    "reduced-motion-high-contrast",
    "compact-window-chrome",
    "diagnostic-assistant-actions",
    "compatibility-inspector-redaction",
    "tab-appearance-picker",
    "quit-confirmation",
    "command-palette-scroll-position",
    "image-preview-open-close",
    "connection-hub-direct-entry",
    "command-result-datetime",
    "saved-preferences-restart",
    "command-boundary-navigation",
    "scrollbar-position",
}

POLICY_KEYS = {
    "schema",
    "phase",
    "status",
    "freshness_days",
    "required_visual_fixture",
    "allowed",
    "limits",
    "privacy",
    "environment_requirements",
    "required_suites",
}
ALLOWED_KEYS = {
    "platforms",
    "architectures",
    "display_servers",
    "renderer_backends",
    "gpu_vendors",
    "power_modes",
    "artifact_privacy",
}
LIMIT_KEYS = {
    "max_manifest_bytes",
    "max_environments",
    "max_suites",
    "max_reviews",
    "max_artifacts_per_suite",
    "max_artifact_bytes",
    "max_suite_duration_ms",
    "max_identifier_bytes",
    "max_metadata_bytes",
    "max_shell_versions",
}
PRIVACY_KEYS = {"persist", "forbidden"}
ENV_REQUIREMENT_KEYS = {
    "id",
    "platform",
    "architecture",
    "display_server",
    "renderer_backend",
    "gpu_vendor",
}
REQUIRED_SUITE_KEYS = {
    "id",
    "domain",
    "environment_id",
    "tool",
    "artifact_kind",
    "coverage",
}
MANIFEST_KEYS = {
    "schema",
    "evidence_kind",
    "synthetic",
    "policy_sha256",
    "source_commit",
    "application",
    "environments",
    "suites",
    "reviews",
    "redaction",
}
APPLICATION_KEYS = {
    "version",
    "binary_sha256",
    "package_sha256",
    "visual_fixture",
}
ENVIRONMENT_KEYS = {
    *ENV_REQUIREMENT_KEYS,
    "adapter_sha256",
    "driver_version",
    "os_build",
    "display",
    "power_mode",
    "shell_versions",
}
SUITE_KEYS = {
    "id",
    "domain",
    "environment_id",
    "tool",
    "tool_version",
    "started_at_utc",
    "duration_ms",
    "result",
    "coverage",
    "artifacts",
    "operator",
}
ARTIFACT_KEYS = {"kind", "sha256", "bytes", "privacy"}
REVIEW_KEYS = {
    "scope",
    "suite_ids",
    "reviewer",
    "reviewed_at_utc",
    "review_url",
    "result",
}
REDACTION_KEYS = {"canaries_checked", "canary_leaks", "forbidden_fields_absent"}


class S1AssuranceError(ValueError):
    """S1 policy or evidence violated the fail-closed contract."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise S1AssuranceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def bounded_bytes(path: Path, maximum: int) -> bytes:
    descriptor: int | None = None
    try:
        before = path.lstat()
        if not stat.S_ISREG(before.st_mode):
            raise S1AssuranceError("evidence must be one non-linked regular file")
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if (
            not stat.S_ISREG(opened.st_mode)
            or (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino)
            or opened.st_size <= 0
            or opened.st_size > maximum
        ):
            raise S1AssuranceError(f"evidence size must be within 1..{maximum} bytes")
        data = bytearray()
        while len(data) <= maximum:
            chunk = os.read(descriptor, min(65_536, maximum + 1 - len(data)))
            if not chunk:
                break
            data.extend(chunk)
        after = os.fstat(descriptor)
        if (
            (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino)
            or after.st_size != opened.st_size
            or len(data) != opened.st_size
            or len(data) > maximum
        ):
            raise S1AssuranceError("evidence changed or grew while reading")
        return bytes(data)
    except S1AssuranceError:
        raise
    except OSError as error:
        raise S1AssuranceError("evidence is unavailable") from error
    finally:
        if descriptor is not None:
            os.close(descriptor)


def _json(path: Path, maximum: int, label: str) -> dict[str, Any]:
    try:
        document = json.loads(
            bounded_bytes(path, maximum).decode("utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except UnicodeDecodeError as error:
        raise S1AssuranceError(f"{label} must be UTF-8") from error
    except json.JSONDecodeError as error:
        raise S1AssuranceError(f"{label} must be valid JSON") from error
    if not isinstance(document, dict):
        raise S1AssuranceError(f"{label} must contain one JSON object")
    return document


def _exact(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise S1AssuranceError(f"{label} keys changed")
    return value


def _integer(value: Any, minimum: int, maximum: int, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not minimum <= value <= maximum:
        raise S1AssuranceError(f"{label} must be within {minimum}..{maximum}")
    return value


def _identifier(value: Any, label: str) -> str:
    if not isinstance(value, str) or IDENTIFIER.fullmatch(value) is None:
        raise S1AssuranceError(f"{label} must be a bounded portable identifier")
    return value


def _metadata(value: Any, label: str) -> str:
    if not isinstance(value, str) or SAFE_METADATA.fullmatch(value) is None:
        raise S1AssuranceError(f"{label} is missing or unsafe")
    return value


def _hash(value: Any, label: str) -> str:
    if not isinstance(value, str) or HEX_64.fullmatch(value) is None:
        raise S1AssuranceError(f"{label} must be a lowercase SHA-256 digest")
    return value


def _utc(value: Any, label: str) -> dt.datetime:
    if not isinstance(value, str) or not value.endswith("Z"):
        raise S1AssuranceError(f"{label} must be a UTC timestamp ending in Z")
    try:
        parsed = dt.datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as error:
        raise S1AssuranceError(f"{label} is invalid") from error
    if parsed.tzinfo != dt.timezone.utc:
        raise S1AssuranceError(f"{label} must be UTC")
    return parsed


def _unique_strings(value: Any, label: str, maximum: int = 1024) -> list[str]:
    if not isinstance(value, list) or not value or len(value) > maximum:
        raise S1AssuranceError(f"{label} must be a bounded non-empty list")
    result = [_identifier(item, label) for item in value]
    if len(set(result)) != len(result):
        raise S1AssuranceError(f"{label} contains duplicates")
    return result


def policy_sha256(policy: dict[str, Any]) -> str:
    payload = json.dumps(policy, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(payload.encode("ascii")).hexdigest()


def validate_policy(policy: dict[str, Any]) -> dict[str, Any]:
    _exact(policy, POLICY_KEYS, "S1 policy")
    if policy["schema"] != 1 or policy["phase"] != "S1" or policy["status"] != "collecting":
        raise S1AssuranceError("S1 policy identity changed")
    _integer(policy["freshness_days"], 1, 30, "S1 freshness days")
    _identifier(policy["required_visual_fixture"], "visual fixture")

    allowed = _exact(policy["allowed"], ALLOWED_KEYS, "S1 allowed values")
    for key, values in allowed.items():
        _unique_strings(values, f"allowed {key}", 32)

    limits = _exact(policy["limits"], LIMIT_KEYS, "S1 limits")
    for key, value in limits.items():
        _integer(value, 1, 2**63 - 1, f"S1 limit {key}")
    if limits["max_manifest_bytes"] != MAX_MANIFEST_BYTES:
        raise S1AssuranceError("S1 manifest byte ceiling changed without code review")

    privacy = _exact(policy["privacy"], PRIVACY_KEYS, "S1 privacy")
    if privacy["persist"] != "allowlisted-redacted-aggregate-metadata-and-digests-only":
        raise S1AssuranceError("S1 persistence boundary changed")
    _unique_strings(privacy["forbidden"], "S1 forbidden privacy fields", 32)

    requirements = policy["environment_requirements"]
    if not isinstance(requirements, list) or not requirements or len(requirements) > limits["max_environments"]:
        raise S1AssuranceError("S1 environment requirements are outside bounds")
    environment_ids: set[str] = set()
    for index, value in enumerate(requirements):
        requirement = _exact(value, ENV_REQUIREMENT_KEYS, f"environment requirement {index}")
        environment_id = _identifier(requirement["id"], "environment requirement id")
        if environment_id in environment_ids:
            raise S1AssuranceError("S1 environment requirement IDs are duplicated")
        environment_ids.add(environment_id)
        for field, allowed_key in (
            ("platform", "platforms"),
            ("architecture", "architectures"),
            ("display_server", "display_servers"),
            ("renderer_backend", "renderer_backends"),
            ("gpu_vendor", "gpu_vendors"),
        ):
            if requirement[field] not in allowed[allowed_key]:
                raise S1AssuranceError(f"environment requirement {field} is unsupported")

    suites = policy["required_suites"]
    if not isinstance(suites, list) or not suites or len(suites) > limits["max_suites"]:
        raise S1AssuranceError("S1 required suite count is outside bounds")
    suite_ids: set[str] = set()
    tools: set[str] = set()
    for index, value in enumerate(suites):
        suite = _exact(value, REQUIRED_SUITE_KEYS, f"required suite {index}")
        suite_id = _identifier(suite["id"], "required suite id")
        if suite_id in suite_ids:
            raise S1AssuranceError("S1 required suite IDs are duplicated")
        suite_ids.add(suite_id)
        if suite["domain"] not in DOMAINS:
            raise S1AssuranceError("S1 suite domain is unsupported")
        if suite["environment_id"] not in environment_ids:
            raise S1AssuranceError("S1 suite references an unknown environment")
        tools.add(_identifier(suite["tool"], "S1 suite tool"))
        _identifier(suite["artifact_kind"], "S1 artifact kind")
        if not isinstance(suite["coverage"], dict) or not suite["coverage"]:
            raise S1AssuranceError("S1 suite coverage must be a non-empty object")
        coverage = suite["coverage"]
        if suite["domain"] == "native":
            scenarios = set(
                _unique_strings(coverage.get("scenarios"), f"{suite_id} scenarios")
            )
            if not REQUIRED_NATIVE_SCENARIOS.issubset(scenarios):
                raise S1AssuranceError(f"{suite_id} omits a required native scenario")
        elif suite["domain"] == "resource" and suite["tool"] == "native-resource":
            scenarios = set(
                _unique_strings(coverage.get("scenarios"), f"{suite_id} scenarios")
            )
            if not REQUIRED_RESOURCE_SCENARIOS.issubset(scenarios):
                raise S1AssuranceError(f"{suite_id} omits a required resource scenario")
        elif suite["domain"] == "visual":
            themes = set(_unique_strings(coverage.get("themes"), f"{suite_id} themes"))
            scales = set(_unique_strings(coverage.get("scales"), f"{suite_id} scales"))
            viewports = set(
                _unique_strings(coverage.get("viewports"), f"{suite_id} viewports")
            )
            surfaces = set(
                _unique_strings(coverage.get("surfaces"), f"{suite_id} surfaces")
            )
            motion_profiles = set(
                _unique_strings(
                    coverage.get("motion_profiles"),
                    f"{suite_id} motion profiles",
                )
            )
            if not REQUIRED_VISUAL_THEMES.issubset(themes):
                raise S1AssuranceError(f"{suite_id} omits a required visual theme")
            if not REQUIRED_VISUAL_SCALES.issubset(scales):
                raise S1AssuranceError(f"{suite_id} omits a required visual scale")
            if not REQUIRED_VISUAL_VIEWPORTS.issubset(viewports):
                raise S1AssuranceError(f"{suite_id} omits a required visual viewport")
            if not REQUIRED_VISUAL_SURFACES.issubset(surfaces):
                raise S1AssuranceError(f"{suite_id} omits a required visual surface")
            if not REQUIRED_VISUAL_MOTION_PROFILES.issubset(motion_profiles):
                raise S1AssuranceError(
                    f"{suite_id} omits a required visual motion profile"
                )
            expected = (
                len(themes)
                * len(scales)
                * len(viewports)
                * len(surfaces)
                * len(motion_profiles)
            )
            if coverage.get("cross_product_complete") is not True or coverage.get("capture_count") != expected:
                raise S1AssuranceError("visual coverage must declare the complete matrix cross product")
        elif suite["domain"] == "accessibility":
            tasks = set(_unique_strings(coverage.get("tasks"), f"{suite_id} tasks"))
            if not REQUIRED_ACCESSIBILITY_TASKS.issubset(tasks):
                raise S1AssuranceError(
                    f"{suite_id} omits a required accessibility task"
                )
    if not {"narrator", "nvda", "voiceover", "orca-x11", "orca-wayland"}.issubset(tools):
        raise S1AssuranceError("S1 assistive-technology matrix is incomplete")
    if not {"application-verifier", "wpr", "visual-diff", "native-gui"}.issubset(tools):
        raise S1AssuranceError("S1 native/resource/visual tool matrix is incomplete")
    return policy


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    return validate_policy(_json(path, 65_536, "S1 assurance policy"))


def load_manifest(path: Path) -> dict[str, Any]:
    document = _json(path, MAX_MANIFEST_BYTES, "S1 assurance evidence")
    return _exact(document, MANIFEST_KEYS, "S1 assurance evidence")


def current_source_commit(root: Path = ROOT) -> str:
    def run(arguments: list[str]) -> subprocess.CompletedProcess[bytes]:
        try:
            return subprocess.run(
                arguments,
                cwd=root,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                timeout=3,
                check=False,
            )
        except (OSError, subprocess.SubprocessError) as error:
            raise S1AssuranceError("current source state is unavailable") from error

    revision = run(["git", "rev-parse", "--verify", "HEAD"])
    try:
        candidate = revision.stdout[:129].decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise S1AssuranceError("current source commit is unavailable") from error
    if revision.returncode != 0 or COMMIT.fullmatch(candidate) is None:
        raise S1AssuranceError("current source commit is unavailable")
    status = run(["git", "diff-index", "--quiet", "HEAD", "--"])
    if status.returncode != 0:
        raise S1AssuranceError("tracked source tree is not clean at the evidence commit")
    return candidate


def _validate_environment(
    value: Any,
    requirement: dict[str, Any],
    policy: dict[str, Any],
    index: int,
) -> dict[str, Any]:
    environment = _exact(value, ENVIRONMENT_KEYS, f"environment {index}")
    if environment["id"] != requirement["id"]:
        raise S1AssuranceError("environment identity changed")
    for field in ENV_REQUIREMENT_KEYS - {"id"}:
        if environment[field] != requirement[field]:
            raise S1AssuranceError(f"environment {environment['id']} {field} changed")
    _hash(environment["adapter_sha256"], "environment adapter")
    for field in ("driver_version", "os_build", "display"):
        _metadata(environment[field], f"environment {field}")
    if environment["power_mode"] not in policy["allowed"]["power_modes"]:
        raise S1AssuranceError("environment power mode is unsupported")
    shells = environment["shell_versions"]
    if not isinstance(shells, list) or not shells or len(shells) > policy["limits"]["max_shell_versions"]:
        raise S1AssuranceError("environment shell version count is outside bounds")
    for shell in shells:
        _metadata(shell, "environment shell version")
    if len(set(shells)) != len(shells):
        raise S1AssuranceError("environment shell versions are duplicated")
    return environment


def _validate_artifacts(
    value: Any,
    required_kind: str,
    policy: dict[str, Any],
    label: str,
) -> None:
    if not isinstance(value, list) or not value or len(value) > policy["limits"]["max_artifacts_per_suite"]:
        raise S1AssuranceError(f"{label} artifacts are outside bounds")
    kinds: set[str] = set()
    hashes: set[str] = set()
    for index, item in enumerate(value):
        artifact = _exact(item, ARTIFACT_KEYS, f"{label} artifact {index}")
        kind = _identifier(artifact["kind"], "artifact kind")
        sha256 = _hash(artifact["sha256"], "artifact")
        if kind in kinds or sha256 in hashes:
            raise S1AssuranceError(f"{label} artifacts are duplicated")
        kinds.add(kind)
        hashes.add(sha256)
        _integer(artifact["bytes"], 1, policy["limits"]["max_artifact_bytes"], "artifact bytes")
        if artifact["privacy"] not in policy["allowed"]["artifact_privacy"]:
            raise S1AssuranceError("artifact privacy classification is unsupported")
    if required_kind not in kinds:
        raise S1AssuranceError(f"{label} lacks its required artifact kind")


def validate_manifest(
    path: Path,
    *,
    allow_synthetic: bool = False,
    expected_commit: str | None = None,
    require_complete: bool = False,
    now: dt.datetime | None = None,
    root: Path = ROOT,
) -> dict[str, Any]:
    policy = load_policy(root / POLICY_PATH.relative_to(ROOT))
    document = load_manifest(path)
    if document["schema"] != 1 or document["evidence_kind"] != "s1-release-assurance":
        raise S1AssuranceError("S1 evidence identity changed")
    if not isinstance(document["synthetic"], bool):
        raise S1AssuranceError("synthetic must be a boolean")
    if document["synthetic"] and not allow_synthetic:
        raise S1AssuranceError("synthetic evidence cannot satisfy the S1 release gate")
    if document["policy_sha256"] != policy_sha256(policy):
        raise S1AssuranceError("S1 evidence is not bound to the active policy")
    source_commit = document["source_commit"]
    if not isinstance(source_commit, str) or COMMIT.fullmatch(source_commit) is None:
        raise S1AssuranceError("source_commit must be an exact lowercase commit digest")
    if expected_commit is not None:
        if COMMIT.fullmatch(expected_commit) is None or source_commit != expected_commit:
            raise S1AssuranceError("S1 evidence is not bound to the expected release commit")
    if not document["synthetic"] and source_commit != current_source_commit(root):
        raise S1AssuranceError("S1 evidence is not bound to the current clean source commit")

    application = _exact(document["application"], APPLICATION_KEYS, "S1 application")
    _metadata(application["version"], "application version")
    for field in ("binary_sha256", "package_sha256"):
        digest = _hash(application[field], f"application {field}")
        if not document["synthetic"] and digest == "0" * 64:
            raise S1AssuranceError("release application hashes cannot use a sentinel")
    if application["visual_fixture"] != policy["required_visual_fixture"]:
        raise S1AssuranceError("S1 visual fixture changed")

    required_environments = {
        requirement["id"]: requirement
        for requirement in policy["environment_requirements"]
    }
    environments_value = document["environments"]
    if not isinstance(environments_value, list) or len(environments_value) > policy["limits"]["max_environments"]:
        raise S1AssuranceError("S1 environment count is outside bounds")
    environments: dict[str, dict[str, Any]] = {}
    for index, value in enumerate(environments_value):
        if not isinstance(value, dict):
            raise S1AssuranceError("S1 environment must be an object")
        environment_id = value.get("id")
        if environment_id not in required_environments or environment_id in environments:
            raise S1AssuranceError("S1 environment is unknown or duplicated")
        environments[environment_id] = _validate_environment(
            value, required_environments[environment_id], policy, index
        )

    required_suites = {suite["id"]: suite for suite in policy["required_suites"]}
    suites_value = document["suites"]
    if not isinstance(suites_value, list) or len(suites_value) > policy["limits"]["max_suites"]:
        raise S1AssuranceError("S1 suite count is outside bounds")
    suites: dict[str, dict[str, Any]] = {}
    operators: set[str] = set()
    suite_times: dict[str, dt.datetime] = {}
    check_time = now or dt.datetime.now(dt.timezone.utc)
    if check_time.tzinfo is None:
        raise S1AssuranceError("S1 validation clock must be timezone-aware")
    check_time = check_time.astimezone(dt.timezone.utc)
    freshness = dt.timedelta(days=policy["freshness_days"])
    for index, value in enumerate(suites_value):
        suite = _exact(value, SUITE_KEYS, f"S1 suite {index}")
        suite_id = _identifier(suite["id"], "S1 suite id")
        if suite_id not in required_suites or suite_id in suites:
            raise S1AssuranceError("S1 suite is unknown or duplicated")
        required = required_suites[suite_id]
        for field in ("domain", "environment_id", "tool"):
            if suite[field] != required[field]:
                raise S1AssuranceError(f"S1 suite {suite_id} {field} changed")
        if suite["environment_id"] not in environments:
            raise S1AssuranceError(f"S1 suite {suite_id} references a missing environment")
        _metadata(suite["tool_version"], "S1 suite tool version")
        started = _utc(suite["started_at_utc"], "S1 suite start")
        if started > check_time + dt.timedelta(minutes=5) or check_time - started > freshness:
            raise S1AssuranceError(f"S1 suite {suite_id} is stale or future-dated")
        _integer(suite["duration_ms"], 0, policy["limits"]["max_suite_duration_ms"], "S1 suite duration")
        if suite["result"] != "pass":
            raise S1AssuranceError(f"S1 suite did not pass: {suite_id}")
        if suite["coverage"] != required["coverage"]:
            raise S1AssuranceError(f"S1 suite coverage changed: {suite_id}")
        _validate_artifacts(
            suite["artifacts"], required["artifact_kind"], policy, f"S1 suite {suite_id}"
        )
        operator = _identifier(suite["operator"], "S1 suite operator")
        operators.add(operator)
        suites[suite_id] = suite
        suite_times[suite_id] = started

    missing = sorted(set(required_suites) - set(suites))
    manual_present = {
        suite_id
        for suite_id, suite in suites.items()
        if suite["domain"] in {"visual", "accessibility"}
    }
    reviews_value = document["reviews"]
    if not isinstance(reviews_value, list) or len(reviews_value) > policy["limits"]["max_reviews"]:
        raise S1AssuranceError("S1 review count is outside bounds")
    reviewed: set[str] = set()
    for index, value in enumerate(reviews_value):
        review = _exact(value, REVIEW_KEYS, f"S1 review {index}")
        _identifier(review["scope"], "S1 review scope")
        suite_ids = _unique_strings(review["suite_ids"], "S1 reviewed suite IDs", 64)
        if not set(suite_ids).issubset(manual_present) or reviewed.intersection(suite_ids):
            raise S1AssuranceError("S1 review references missing, non-manual, or duplicated suites")
        reviewer = _identifier(review["reviewer"], "S1 reviewer")
        if reviewer in operators:
            raise S1AssuranceError("S1 human review must be independent from suite operators")
        reviewed_at = _utc(review["reviewed_at_utc"], "S1 review time")
        if any(reviewed_at < suite_times[suite_id] for suite_id in suite_ids):
            raise S1AssuranceError("S1 review predates reviewed evidence")
        if (
            reviewed_at > check_time + dt.timedelta(minutes=5)
            or check_time - reviewed_at > freshness
        ):
            raise S1AssuranceError("S1 review is stale or future-dated")
        review_url = review["review_url"]
        try:
            encoded_review_url = review_url.encode("ascii")
            parsed_review_url = urlsplit(review_url)
            parsed_review_url.port
        except (AttributeError, UnicodeEncodeError, ValueError) as error:
            raise S1AssuranceError("S1 review requires a bounded HTTPS URL") from error
        if (
            len(encoded_review_url) > 512
            or parsed_review_url.scheme != "https"
            or not parsed_review_url.hostname
            or parsed_review_url.username is not None
            or parsed_review_url.password is not None
        ):
            raise S1AssuranceError("S1 review requires a bounded HTTPS URL")
        if review["result"] != "approved":
            raise S1AssuranceError("S1 review is not approved")
        reviewed.update(suite_ids)
    if reviewed != manual_present:
        raise S1AssuranceError("S1 visual/accessibility evidence lacks exact human review")

    redaction = _exact(document["redaction"], REDACTION_KEYS, "S1 redaction")
    _integer(redaction["canaries_checked"], 1, 1_000_000, "S1 redaction canaries")
    if redaction["canary_leaks"] != 0 or redaction["forbidden_fields_absent"] is not True:
        raise S1AssuranceError("S1 evidence redaction failed")

    status = "pass" if not missing else "external"
    if require_complete and missing:
        raise S1AssuranceError(
            "S1 evidence is incomplete: " + ", ".join(missing)
        )
    return {
        "schema": 1,
        "status": status,
        "source_commit": source_commit,
        "policy_sha256": document["policy_sha256"],
        "suite_count": len(suites),
        "missing_suites": missing,
        "domains": {
            domain: sum(1 for suite in suites.values() if suite["domain"] == domain)
            for domain in sorted(DOMAINS)
        },
    }


def write_report(path: Path, report: dict[str, Any]) -> None:
    payload = json.dumps(report, indent=2, sort_keys=True).encode("utf-8") + b"\n"
    if len(payload) > 262_144:
        raise S1AssuranceError("S1 report exceeds its byte ceiling")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", prefix=f".{path.name}.", suffix=".tmp", dir=path.parent, delete=False
        ) as output:
            temporary = Path(output.name)
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    commands.add_parser("check-policy")
    validate = commands.add_parser("validate")
    validate.add_argument("--manifest", type=Path, required=True)
    validate.add_argument("--expected-commit")
    validate.add_argument("--require-complete", action="store_true")
    validate.add_argument("--allow-synthetic", action="store_true")
    validate.add_argument("--output", type=Path)
    return root


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "check-policy":
            policy = load_policy()
            print(
                "PASS: S1 assurance policy is bounded and collecting "
                f"({len(policy['required_suites'])} required native/visual/resource/accessibility suites)"
            )
            return 0
        report = validate_manifest(
            arguments.manifest,
            allow_synthetic=arguments.allow_synthetic,
            expected_commit=arguments.expected_commit,
            require_complete=arguments.require_complete,
        )
        if arguments.output is not None:
            write_report(arguments.output, report)
        print(
            f"S1 assurance: {report['status']} "
            f"({report['suite_count']} suites, {len(report['missing_suites'])} external)"
        )
        return 0 if report["status"] == "pass" else 2
    except (S1AssuranceError, OSError) as error:
        print(f"S1 assurance failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
