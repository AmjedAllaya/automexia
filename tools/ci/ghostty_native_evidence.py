#!/usr/bin/env python3
"""Validate one redacted three-platform Ghostty compatibility evidence set."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import stat
import sys
from typing import Any


MAX_MANIFEST_BYTES = 512 * 1024
MAX_DURATION_MS = 30 * 60 * 1000
PROFILE = "ghostty-1.3"
PATCH = "1.3.1"
UPSTREAM_COMMIT = "22efb0be2bbea73e5339f5426fa3b20edabcaa11"
PLATFORMS = {"windows", "linux", "macos"}
ARCHITECTURES = {"x86_64", "aarch64"}
SCENARIOS = {
    "fixture-smoke",
    "keyboard-layouts",
    "ime-altgr-dead-keys",
    "profile-reload",
    "inspector-accessibility",
    "topology-history",
    "resource-cycle-1-10-50",
    "package-install-rollback",
}
TOP_LEVEL_KEYS = {
    "schema",
    "evidence_kind",
    "synthetic",
    "source_commit",
    "profile",
    "runs",
    "redaction",
}
RUN_KEYS = {
    "platform",
    "architecture",
    "application_sha256",
    "package_sha256",
    "fixture_kind",
    "fixture_sha256",
    "scenarios",
    "resources",
    "visual",
    "accessibility",
}
RESOURCE_KEYS = {
    "peak_memory_bytes",
    "peak_handles_or_fds",
    "peak_processes",
    "owned_processes_after",
    "handles_or_fds_delta_after",
    "threads_delta_after",
    "parked_sessions_after_clear",
    "duration_ms",
}
VISUAL_KEYS = {"compact", "normal", "split", "hidpi", "text_scale_200", "human_reviewed"}
ACCESSIBILITY_KEYS = {"keyboard_only", "focus_restored", "scope_announced", "native_at_reviewed"}
REDACTION_KEYS = {"canaries_checked", "canary_leaks", "forbidden_fields_absent"}
HEX_40_OR_64 = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
HEX_64 = re.compile(r"^[0-9a-f]{64}$")


class GhosttyNativeEvidenceError(ValueError):
    """Native compatibility evidence is incomplete, unsafe, or malformed."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GhosttyNativeEvidenceError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _bounded_bytes(path: Path) -> bytes:
    descriptor: int | None = None
    try:
        before = path.lstat()
        reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
        attributes = getattr(before, "st_file_attributes", 0)
        if (
            not stat.S_ISREG(before.st_mode)
            or path.is_symlink()
            or bool(reparse and attributes & reparse)
        ):
            raise GhosttyNativeEvidenceError("evidence must be one regular non-linked file")
        if before.st_size <= 0 or before.st_size > MAX_MANIFEST_BYTES:
            raise GhosttyNativeEvidenceError("evidence size is outside the bounded contract")
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino):
            raise GhosttyNativeEvidenceError("evidence identity changed while opening")
        data = os.read(descriptor, MAX_MANIFEST_BYTES + 1)
        after = os.fstat(descriptor)
        if (
            (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino)
            or after.st_size != opened.st_size
            or len(data) != opened.st_size
        ):
            raise GhosttyNativeEvidenceError("evidence changed while reading")
        return data
    except GhosttyNativeEvidenceError:
        raise
    except OSError as error:
        raise GhosttyNativeEvidenceError("native evidence is unavailable") from error
    finally:
        if descriptor is not None:
            os.close(descriptor)


def parse_manifest_bytes(data: bytes) -> Any:
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise GhosttyNativeEvidenceError("native evidence is not strict UTF-8 JSON") from error


def _exact_object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise GhosttyNativeEvidenceError(f"{label} keys changed")
    return value


def _positive_bounded_int(value: Any, maximum: int, label: str, *, allow_zero: bool = True) -> int:
    minimum = 0 if allow_zero else 1
    if type(value) is not int or not minimum <= value <= maximum:
        raise GhosttyNativeEvidenceError(f"{label} is outside {minimum}..{maximum}")
    return value


def validate_manifest(document: Any, expected_commit: str, require_complete: bool) -> dict[str, int]:
    root = _exact_object(document, TOP_LEVEL_KEYS, "evidence")
    if root["schema"] != 1 or root["evidence_kind"] != "ghostty-compatibility-1.3.1":
        raise GhosttyNativeEvidenceError("evidence identity changed")
    if root["synthetic"] is not False:
        raise GhosttyNativeEvidenceError("synthetic evidence cannot satisfy a native gate")
    if not HEX_40_OR_64.fullmatch(root["source_commit"] or "") or root["source_commit"] != expected_commit:
        raise GhosttyNativeEvidenceError("evidence source commit does not match the release candidate")
    if root["profile"] != {
        "id": PROFILE,
        "patch": PATCH,
        "upstream_commit": UPSTREAM_COMMIT,
        "default_profile": "automexia",
        "opt_in": True,
    }:
        raise GhosttyNativeEvidenceError("profile identity or opt-in boundary changed")
    redaction = _exact_object(root["redaction"], REDACTION_KEYS, "redaction")
    if (
        type(redaction["canaries_checked"]) is not int
        or redaction["canaries_checked"] < 3
        or redaction["canary_leaks"] != 0
        or redaction["forbidden_fields_absent"] is not True
    ):
        raise GhosttyNativeEvidenceError("redaction proof is incomplete")
    runs = root["runs"]
    if not isinstance(runs, list) or len(runs) > 3:
        raise GhosttyNativeEvidenceError("native run count is invalid")
    seen: set[str] = set()
    for run_value in runs:
        run = _exact_object(run_value, RUN_KEYS, "native run")
        platform = run["platform"]
        if platform not in PLATFORMS or platform in seen:
            raise GhosttyNativeEvidenceError("platform run is missing, duplicate, or unsupported")
        seen.add(platform)
        if run["architecture"] not in ARCHITECTURES:
            raise GhosttyNativeEvidenceError("native architecture is unsupported")
        for field in ("application_sha256", "package_sha256", "fixture_sha256"):
            if not HEX_64.fullmatch(run[field] or ""):
                raise GhosttyNativeEvidenceError(f"{field} is not a SHA-256 digest")
        expected_fixture = "windows-adapted" if platform == "windows" else "native"
        if run["fixture_kind"] != expected_fixture:
            raise GhosttyNativeEvidenceError("platform fixture kind is not truthful")
        scenarios = run["scenarios"]
        if (
            not isinstance(scenarios, list)
            or {item.get("id") for item in scenarios if isinstance(item, dict)} != SCENARIOS
            or len(scenarios) != len(SCENARIOS)
        ):
            raise GhosttyNativeEvidenceError("native scenario inventory is incomplete")
        for scenario in scenarios:
            if set(scenario) != {"id", "result", "duration_ms"} or scenario["result"] != "pass":
                raise GhosttyNativeEvidenceError("a native scenario did not pass exactly once")
            _positive_bounded_int(scenario["duration_ms"], MAX_DURATION_MS, "scenario duration")
        resources = _exact_object(run["resources"], RESOURCE_KEYS, "resources")
        _positive_bounded_int(resources["peak_memory_bytes"], 4 * 1024**3, "peak memory", allow_zero=False)
        _positive_bounded_int(resources["peak_handles_or_fds"], 65_536, "peak handles", allow_zero=False)
        _positive_bounded_int(resources["peak_processes"], 256, "peak processes", allow_zero=False)
        _positive_bounded_int(resources["duration_ms"], MAX_DURATION_MS, "run duration", allow_zero=False)
        for field in (
            "owned_processes_after",
            "handles_or_fds_delta_after",
            "threads_delta_after",
            "parked_sessions_after_clear",
        ):
            if resources[field] != 0:
                raise GhosttyNativeEvidenceError(f"resource cleanup failed: {field}")
        visual = _exact_object(run["visual"], VISUAL_KEYS, "visual proof")
        accessibility = _exact_object(run["accessibility"], ACCESSIBILITY_KEYS, "accessibility proof")
        if not all(value is True for value in visual.values()):
            raise GhosttyNativeEvidenceError("visual matrix or human review is incomplete")
        if not all(value is True for value in accessibility.values()):
            raise GhosttyNativeEvidenceError("native accessibility matrix is incomplete")
    if require_complete and seen != PLATFORMS:
        raise GhosttyNativeEvidenceError("complete Windows, Linux, and macOS evidence is required")
    return {"platforms": len(seen), "scenarios": len(seen) * len(SCENARIOS)}


def validate_file(path: Path, expected_commit: str, require_complete: bool) -> dict[str, int]:
    return validate_manifest(parse_manifest_bytes(_bounded_bytes(path)), expected_commit, require_complete)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--require-complete", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result = validate_file(args.manifest, args.expected_commit, args.require_complete)
    except GhosttyNativeEvidenceError as error:
        print(f"FAIL: Ghostty native evidence: {error}", file=sys.stderr)
        return 1
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    print(f"PASS: Ghostty native evidence ({result['platforms']} platforms, {result['scenarios']} scenarios)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
