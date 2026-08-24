#!/usr/bin/env python3
"""Bounded S1 benchmark normalization and S2 release-ratchet enforcement."""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import statistics
import subprocess
import sys
import tempfile
from typing import Any
import unicodedata
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests" / "assurance" / "performance-ratchet-policy-v1.json"
BASELINE_PATH = ROOT / "tests" / "fixtures" / "performance" / "s2-baseline-v1.json"
WAIVERS_PATH = ROOT / "tests" / "fixtures" / "performance" / "s2-waivers-v1.json"

EXPECTED_THRESHOLDS = {"latency_percent": 5.0, "memory_percent": 10.0}
EXPECTED_REQUIRED_CLAIMS = [
    "startup",
    "input",
    "prompt",
    "context",
    "listing",
    "parser",
    "renderer",
    "resize",
    "memory",
    "workers",
    "images",
    "actions",
    "pty",
    "build-storage",
]
EXPECTED_COMPARABILITY_FIELDS = [
    "runner_class",
    "os",
    "arch",
    "cpu",
    "gpu",
    "power_mode",
    "toolchain",
    "profile",
]
EXPECTED_LIMITS = {
    "max_policy_bytes": 65_536,
    "max_baseline_bytes": 8_388_608,
    "max_evidence_bytes": 2_097_152,
    "max_waiver_bytes": 262_144,
    "max_report_bytes": 262_144,
    "max_criterion_file_bytes": 262_144,
    "max_criterion_sample_file_bytes": 1_048_576,
    "max_criterion_entries": 4_096,
    "max_criterion_depth": 16,
    "max_metrics": 512,
    "max_unclassified_metrics": 512,
    "max_metric_id_bytes": 192,
    "max_metadata_value_bytes": 192,
}
EXPECTED_QUALITY = {
    "candidate_max_age_hours": 24,
    "maximum_future_skew_minutes": 5,
    "minimum_criterion_samples": 50,
    "maximum_criterion_samples": 10_000,
    "criterion_confidence_level": 0.95,
    "maximum_latency_interval_percent": 10.0,
    "maximum_memory_interval_percent": 100.0,
}
EXPECTED_POLICY_SHA256 = "b0b54438f9813e8e85d3b21d0427287a4c703964401572d6b90032dfb01a2f2a"
MAX_REPORT_BYTES = EXPECTED_LIMITS["max_report_bytes"]
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")
METRIC_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._/@+-]*(?:/[A-Za-z0-9._@+-]+)*$")
OPERATOR_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._@+-]{0,127}$")
ABSOLUTE_PATH_RE = re.compile(r"(?:^[A-Za-z]:[\\/]|^/|^\\\\|[\\/]Users[\\/]|[\\/]home[\\/])")
SECRET_RE = re.compile(r"(?:-----BEGIN|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{20,})")


class AssuranceError(ValueError):
    """A performance evidence or policy contract was invalid."""


def _duplicate_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AssuranceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _is_link_or_reparse(info: os.stat_result) -> bool:
    attributes = getattr(info, "st_file_attributes", 0)
    reparse = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    return stat.S_ISLNK(info.st_mode) or bool(attributes & reparse)


def read_json(path: Path, maximum_bytes: int, label: str) -> Any:
    descriptor: int | None = None
    try:
        before = path.lstat()
        if _is_link_or_reparse(before) or not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise AssuranceError(f"{label} must be one non-linked regular file")
        if before.st_size <= 0 or before.st_size > maximum_bytes:
            raise AssuranceError(f"{label} size is outside 1..{maximum_bytes} bytes")
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if (
            _is_link_or_reparse(opened)
            or not stat.S_ISREG(opened.st_mode)
            or opened.st_nlink != 1
            or (opened.st_dev, opened.st_ino, opened.st_size)
            != (before.st_dev, before.st_ino, before.st_size)
        ):
            raise AssuranceError(f"{label} changed or resolved through a link while opening")
        payload = bytearray()
        while len(payload) <= maximum_bytes:
            chunk = os.read(descriptor, min(65_536, maximum_bytes + 1 - len(payload)))
            if not chunk:
                break
            payload.extend(chunk)
        after = os.fstat(descriptor)
        if (
            (after.st_dev, after.st_ino, after.st_size)
            != (opened.st_dev, opened.st_ino, opened.st_size)
            or len(payload) != opened.st_size
            or len(payload) > maximum_bytes
        ):
            raise AssuranceError(f"{label} changed or grew while it was read")
    except AssuranceError:
        raise
    except OSError as error:
        raise AssuranceError(f"could not read {label}") from error
    finally:
        if descriptor is not None:
            os.close(descriptor)
    try:
        return json.loads(bytes(payload).decode("utf-8"), object_pairs_hook=_duplicate_object)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise AssuranceError(f"{label} is not strict UTF-8 JSON: {error}") from error


def _exact_keys(value: object, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        raise AssuranceError(f"{label} fields changed")
    return value


def _bounded_text(value: object, maximum_bytes: int, label: str) -> str:
    if not isinstance(value, str):
        raise AssuranceError(f"{label} must be text")
    encoded = value.encode("utf-8")
    if (
        not encoded
        or len(encoded) > maximum_bytes
        or any(unicodedata.category(char) in {"Cc", "Cs"} for char in value)
    ):
        raise AssuranceError(f"{label} is empty, oversized, or contains controls")
    return value


def _operator(value: object, label: str) -> str:
    text = _bounded_text(value, 128, label)
    if OPERATOR_RE.fullmatch(text) is None:
        raise AssuranceError(f"{label} must be a portable operator identity")
    return text


def _portable_metadata(value: object, maximum_bytes: int, label: str) -> str:
    text = _bounded_text(value, maximum_bytes, label)
    try:
        encoded = text.encode("ascii")
    except UnicodeEncodeError as error:
        raise AssuranceError(f"{label} metadata must be portable ASCII") from error
    if (
        len(encoded) > maximum_bytes
        or ABSOLUTE_PATH_RE.search(text) is not None
        or SECRET_RE.search(text) is not None
        or "\\" in text
    ):
        raise AssuranceError(f"{label} metadata contains a path or credential pattern")
    return text


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or DIGEST_RE.fullmatch(value) is None:
        raise AssuranceError(f"{label} must be one lowercase SHA-256 digest")
    return value


def _utc(value: object, label: str) -> dt.datetime:
    text = _bounded_text(value, 64, label)
    if not text.endswith("Z"):
        raise AssuranceError(f"{label} must be an explicit UTC timestamp")
    try:
        parsed = dt.datetime.fromisoformat(text[:-1] + "+00:00")
    except ValueError as error:
        raise AssuranceError(f"{label} is not a valid timestamp") from error
    if parsed.utcoffset() != dt.timedelta(0):
        raise AssuranceError(f"{label} must be UTC")
    return parsed


def _commit(value: object, label: str = "commit") -> str:
    if not isinstance(value, str) or COMMIT_RE.fullmatch(value) is None:
        raise AssuranceError(f"{label} must be one lowercase 40-character commit")
    return value


def _digest(value: object) -> str:
    return hashlib.sha256(_canonical(value)).hexdigest()


def validate_source_state(current_commit: str, dirty: bool, expected_commit: str) -> None:
    _commit(current_commit, "current source commit")
    _commit(expected_commit, "expected source commit")
    if current_commit != expected_commit:
        raise AssuranceError("S2 evidence is not bound to the expected source commit")
    if dirty:
        raise AssuranceError("S2 evidence cannot be produced from a dirty tracked source tree")


def current_source_state(root: Path = ROOT) -> tuple[str, bool]:
    try:
        commit_result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=root,
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
        unstaged = subprocess.run(
            ["git", "diff", "--quiet", "--ignore-submodules", "--"],
            cwd=root,
            check=False,
            capture_output=True,
            timeout=15,
        )
        staged = subprocess.run(
            ["git", "diff", "--cached", "--quiet", "--ignore-submodules", "--"],
            cwd=root,
            check=False,
            capture_output=True,
            timeout=15,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise AssuranceError("current S2 source state is unavailable") from error
    commit = commit_result.stdout.strip()
    if commit_result.returncode != 0 or COMMIT_RE.fullmatch(commit) is None:
        raise AssuranceError("current S2 source commit is unavailable")
    if unstaged.returncode not in {0, 1} or staged.returncode not in {0, 1}:
        raise AssuranceError("current S2 tracked-tree state is unavailable")
    return commit, unstaged.returncode == 1 or staged.returncode == 1


def require_clean_source(expected_commit: str, root: Path = ROOT) -> None:
    current, dirty = current_source_state(root)
    validate_source_state(current, dirty, expected_commit)


def validate_candidate_context(
    candidate: dict[str, Any],
    policy: dict[str, Any],
    *,
    expected_commit: str,
    now: dt.datetime,
) -> None:
    if candidate["commit"] != _commit(expected_commit, "expected source commit"):
        raise AssuranceError("candidate evidence does not match the expected source commit")
    measured = _utc(candidate["measured_at_utc"], "candidate measurement time")
    future_skew = dt.timedelta(minutes=policy["quality"]["maximum_future_skew_minutes"])
    maximum_age = dt.timedelta(hours=policy["quality"]["candidate_max_age_hours"])
    if measured > now + future_skew:
        raise AssuranceError("candidate evidence is future-dated")
    if now - measured > maximum_age:
        raise AssuranceError("candidate evidence is stale")


def validate_active_baseline_context(
    baseline: dict[str, Any], policy: dict[str, Any], *, now: dt.datetime
) -> None:
    if baseline["status"] != "active":
        raise AssuranceError("performance baseline is not active")
    accepted_at = _utc(baseline["accepted"]["at_utc"], "baseline acceptance time")
    if accepted_at > now + dt.timedelta(
        minutes=policy["quality"]["maximum_future_skew_minutes"]
    ):
        raise AssuranceError("baseline acceptance is future-dated")


def policy_digest(policy: dict[str, Any]) -> str:
    validate_policy(policy)
    return _digest(policy)


def baseline_digest(baseline: dict[str, Any]) -> str:
    return _digest(baseline)


def validate_policy(value: object) -> dict[str, Any]:
    policy = _exact_keys(
        value,
        {
            "schema",
            "phase",
            "status",
            "thresholds",
            "quality",
            "required_claims",
            "comparability_fields",
            "criterion_claim_rules",
            "baseline",
            "waivers",
            "limits",
            "privacy",
            "activation",
        },
        "performance policy",
    )
    if policy["schema"] != 1 or policy["phase"] != "S1-S2":
        raise AssuranceError("performance policy schema or phase changed")
    if policy["status"] != "collecting":
        raise AssuranceError("repository performance policy must remain collecting")
    if policy["thresholds"] != EXPECTED_THRESHOLDS:
        raise AssuranceError("5% latency or 10% memory threshold changed")
    if policy["quality"] != EXPECTED_QUALITY:
        raise AssuranceError("performance evidence quality contract changed")
    if policy["required_claims"] != EXPECTED_REQUIRED_CLAIMS:
        raise AssuranceError("required performance claims changed")
    if policy["comparability_fields"] != EXPECTED_COMPARABILITY_FIELDS:
        raise AssuranceError("runner comparability fields changed")
    if policy["limits"] != EXPECTED_LIMITS:
        raise AssuranceError("performance evidence limits changed")
    if policy["baseline"] != {
        "minimum_consecutive_days": 30,
        "maximum_days": 90,
        "maximum_acceptance_delay_days": 7,
        "minimum_metrics_per_claim": 1,
    }:
        raise AssuranceError("30-day baseline eligibility changed")
    if policy["waivers"] != {
        "maximum_duration_days": 30,
        "maximum_reason_bytes": 512,
        "https_review_required": True,
        "independent_approver_required": True,
        "wildcards_forbidden": True,
    }:
        raise AssuranceError("waiver boundary changed")
    if policy["privacy"] != {
        "persist": "allowlisted-runner-and-aggregate-metrics-only",
        "forbidden": [
            "environment-dump",
            "terminal-content",
            "command-history",
            "clipboard",
            "credentials",
            "user-paths",
            "raw-etl",
        ],
    }:
        raise AssuranceError("performance evidence privacy boundary changed")
    if policy["activation"] != {
        "automatic": False,
        "independent_review_required": True,
        "requires_reviewed_baseline": True,
        "release_mode": "fail-closed-require-active",
    }:
        raise AssuranceError("performance enforcement activation changed")
    rules = policy["criterion_claim_rules"]
    if not isinstance(rules, list) or not rules or len(rules) > 64:
        raise AssuranceError("Criterion claim rules are invalid")
    seen_globs: set[str] = set()
    for index, raw in enumerate(rules):
        rule = _exact_keys(raw, {"glob", "claim"}, f"Criterion claim rule {index}")
        pattern = _bounded_text(rule["glob"], 128, f"Criterion rule {index} glob")
        if pattern in seen_globs or rule["claim"] not in EXPECTED_REQUIRED_CLAIMS:
            raise AssuranceError("Criterion claim rule is duplicate or unknown")
        seen_globs.add(pattern)
    if _digest(policy) != EXPECTED_POLICY_SHA256:
        raise AssuranceError("reviewed performance policy content changed")
    return policy


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    document = read_json(path, EXPECTED_LIMITS["max_policy_bytes"], "performance policy")
    return validate_policy(document)


def validate_runner(value: object, policy: dict[str, Any]) -> dict[str, str]:
    runner = _exact_keys(value, set(policy["comparability_fields"]), "runner fingerprint")
    maximum = policy["limits"]["max_metadata_value_bytes"]
    for field in policy["comparability_fields"]:
        _portable_metadata(runner[field], maximum, f"runner {field}")
    return runner


def validate_metric(value: object, policy: dict[str, Any], label: str) -> dict[str, Any]:
    metric = _exact_keys(
        value,
        {"id", "claim", "family", "unit", "point", "lower", "upper", "samples"},
        label,
    )
    metric_id = _bounded_text(
        metric["id"], policy["limits"]["max_metric_id_bytes"], f"{label} id"
    )
    if METRIC_ID_RE.fullmatch(metric_id) is None or ".." in metric_id:
        raise AssuranceError(f"{label} metric id is unsafe")
    if metric["claim"] not in policy["required_claims"]:
        raise AssuranceError(f"{label} claim is unknown")
    if metric["family"] not in {"latency", "memory"}:
        raise AssuranceError(f"{label} family is unknown")
    expected_unit = "ns" if metric["family"] == "latency" else "bytes"
    if metric["unit"] != expected_unit:
        raise AssuranceError(f"{label} unit must be {expected_unit}")
    values: list[float] = []
    for field in ("lower", "point", "upper"):
        raw = metric[field]
        if isinstance(raw, bool) or not isinstance(raw, (int, float)):
            raise AssuranceError(f"{label} {field} must be numeric")
        number = float(raw)
        if not math.isfinite(number) or number <= 0:
            raise AssuranceError(f"{label} {field} must be finite and positive")
        values.append(number)
    if not values[0] <= values[1] <= values[2]:
        raise AssuranceError(f"{label} confidence bounds do not contain point")
    samples = metric["samples"]
    minimum_samples = (
        policy["quality"]["minimum_criterion_samples"]
        if metric["family"] == "latency"
        else 2
    )
    if (
        isinstance(samples, bool)
        or not isinstance(samples, int)
        or not minimum_samples <= samples <= policy["quality"]["maximum_criterion_samples"]
    ):
        raise AssuranceError(f"{label} samples are outside the reviewed quality bounds")
    interval_percent = (values[2] - values[0]) / values[1] * 100.0
    maximum_interval = policy["quality"][
        "maximum_latency_interval_percent"
        if metric["family"] == "latency"
        else "maximum_memory_interval_percent"
    ]
    if interval_percent > maximum_interval + 1e-9:
        raise AssuranceError(f"{label} confidence interval is too wide")
    return metric


def _validate_metrics(value: object, policy: dict[str, Any], label: str) -> list[dict[str, Any]]:
    if not isinstance(value, list) or not value or len(value) > policy["limits"]["max_metrics"]:
        raise AssuranceError(f"{label} metric count is outside bounds")
    metrics = [validate_metric(item, policy, f"{label} metric {index}") for index, item in enumerate(value)]
    identities = [metric["id"] for metric in metrics]
    if len(set(identities)) != len(identities):
        raise AssuranceError(f"{label} contains duplicate metric identities")
    if identities != sorted(identities):
        raise AssuranceError(f"{label} metrics must be sorted by identity")
    return metrics


def validate_evidence(value: object, policy: dict[str, Any]) -> dict[str, Any]:
    evidence = _exact_keys(
        value,
        {
            "schema",
            "kind",
            "commit",
            "measured_at_utc",
            "operator",
            "runner",
            "metrics",
            "unclassified_metrics",
        },
        "candidate evidence",
    )
    if evidence["schema"] != 1 or evidence["kind"] != "candidate":
        raise AssuranceError("candidate evidence schema or kind changed")
    _commit(evidence["commit"], "candidate commit")
    _utc(evidence["measured_at_utc"], "candidate measured_at_utc")
    _operator(evidence["operator"], "candidate operator")
    validate_runner(evidence["runner"], policy)
    _validate_metrics(evidence["metrics"], policy, "candidate")
    unknown = evidence["unclassified_metrics"]
    if not isinstance(unknown, list) or len(unknown) > policy["limits"]["max_unclassified_metrics"]:
        raise AssuranceError("unclassified metric count is outside bounds")
    if unknown != sorted(set(unknown)):
        raise AssuranceError("unclassified metrics must be unique and sorted")
    for index, metric_id in enumerate(unknown):
        text = _bounded_text(
            metric_id,
            policy["limits"]["max_metric_id_bytes"],
            f"unclassified metric {index}",
        )
        if METRIC_ID_RE.fullmatch(text) is None or ".." in text:
            raise AssuranceError("unclassified metric identity is unsafe")
    return evidence


def _validate_acceptance(value: object) -> dict[str, Any]:
    accepted = _exact_keys(value, {"by", "at_utc", "review_url"}, "baseline acceptance")
    _operator(accepted["by"], "baseline approver")
    _utc(accepted["at_utc"], "baseline acceptance time")
    _https_url(accepted["review_url"], "baseline review URL")
    return accepted


def validate_baseline(
    value: object, policy: dict[str, Any], *, require_active: bool
) -> dict[str, Any]:
    baseline = _exact_keys(
        value,
        {"schema", "status", "policy_sha256", "runner", "days", "accepted"},
        "performance baseline",
    )
    if baseline["schema"] != 1 or baseline["status"] not in {"collecting", "active"}:
        raise AssuranceError("performance baseline schema or status changed")
    if baseline["status"] == "collecting":
        if any(
            (
                baseline["policy_sha256"] is not None,
                baseline["runner"] is not None,
                baseline["days"] != [],
                baseline["accepted"] is not None,
            )
        ):
            raise AssuranceError("collecting baseline must not claim accepted evidence")
        if require_active:
            raise AssuranceError("performance baseline is not active")
        return baseline

    if baseline["policy_sha256"] != policy_digest(policy):
        raise AssuranceError("active baseline is not bound to the current policy")
    validate_runner(baseline["runner"], policy)
    accepted = _validate_acceptance(baseline["accepted"])
    days = baseline["days"]
    minimum = policy["baseline"]["minimum_consecutive_days"]
    maximum = policy["baseline"]["maximum_days"]
    if not isinstance(days, list) or not minimum <= len(days) <= maximum:
        raise AssuranceError(f"active baseline requires {minimum} consecutive days")
    parsed_dates: list[dt.date] = []
    measurements: list[dt.datetime] = []
    operators: set[str] = set()
    expected_metric_ids: set[str] | None = None
    claims: dict[str, int] = {claim: 0 for claim in policy["required_claims"]}
    for index, raw_day in enumerate(days):
        day = _exact_keys(
            raw_day,
            {
                "date",
                "commit",
                "measured_at_utc",
                "operator",
                "evidence_sha256",
                "metrics",
            },
            f"baseline day {index}",
        )
        try:
            parsed_date = dt.date.fromisoformat(_bounded_text(day["date"], 10, "baseline date"))
        except ValueError as error:
            raise AssuranceError("baseline date is invalid") from error
        if parsed_date.isoformat() != day["date"]:
            raise AssuranceError("baseline date must use YYYY-MM-DD")
        parsed_dates.append(parsed_date)
        _commit(day["commit"], f"baseline day {index} commit")
        metrics = _validate_metrics(day["metrics"], policy, f"baseline day {index}")
        measurement = _utc(day["measured_at_utc"], f"baseline day {index} measurement")
        if measurement.date() != parsed_date:
            raise AssuranceError("baseline measurement does not match its UTC date")
        measurements.append(measurement)
        operators.add(_operator(day["operator"], f"baseline day {index} operator"))
        _sha256(day["evidence_sha256"], f"baseline day {index} evidence digest")
        metric_ids = {metric["id"] for metric in metrics}
        if expected_metric_ids is None:
            expected_metric_ids = metric_ids
            for metric in metrics:
                claims[metric["claim"]] += 1
        elif metric_ids != expected_metric_ids:
            raise AssuranceError("baseline metric set changed between days")
    if len(set(parsed_dates)) != len(parsed_dates) or parsed_dates != sorted(parsed_dates):
        raise AssuranceError("baseline dates must be unique and ordered")
    if any(
        current - previous != dt.timedelta(days=1)
        for previous, current in zip(parsed_dates, parsed_dates[1:])
    ):
        raise AssuranceError("baseline dates must be consecutive")
    minimum_per_claim = policy["baseline"]["minimum_metrics_per_claim"]
    missing = [claim for claim, count in claims.items() if count < minimum_per_claim]
    if missing:
        raise AssuranceError(f"baseline metric set is missing required claims: {', '.join(missing)}")
    if measurements != sorted(measurements):
        raise AssuranceError("baseline measurements must be ordered")
    accepted_at = _utc(accepted["at_utc"], "baseline acceptance time")
    if accepted["by"].casefold() in {operator.casefold() for operator in operators}:
        raise AssuranceError("baseline acceptance must be independent from evidence operators")
    if accepted_at < measurements[-1]:
        raise AssuranceError("baseline acceptance predates its final measurement")
    if accepted_at - measurements[-1] > dt.timedelta(
        days=policy["baseline"]["maximum_acceptance_delay_days"]
    ):
        raise AssuranceError("baseline acceptance is too late for the measured evidence")
    return baseline


def load_baseline(
    path: Path, policy: dict[str, Any], *, require_active: bool
) -> dict[str, Any]:
    value = read_json(
        path, policy["limits"]["max_baseline_bytes"], "performance baseline"
    )
    return validate_baseline(value, policy, require_active=require_active)


def load_evidence(path: Path, policy: dict[str, Any]) -> dict[str, Any]:
    value = read_json(
        path, policy["limits"]["max_evidence_bytes"], "candidate evidence"
    )
    return validate_evidence(value, policy)


def _https_url(value: object, label: str) -> str:
    text = _bounded_text(value, 512, label)
    try:
        encoded = text.encode("ascii")
        parsed = urlsplit(text)
        parsed.port
    except (UnicodeEncodeError, ValueError) as error:
        raise AssuranceError(f"{label} must be a portable HTTPS URL") from error
    if (
        len(encoded) > 512
        or parsed.scheme != "https"
        or not parsed.hostname
        or parsed.username is not None
        or parsed.password is not None
        or any(char.isspace() for char in text)
        or "\\" in text
    ):
        raise AssuranceError(f"{label} must be an HTTPS URL without credentials")
    return text


def _validate_waiver(
    raw: object,
    policy: dict[str, Any],
    *,
    metric_id: str,
    candidate_commit: str,
    candidate_operator: str,
    accepted_baseline_sha256: str,
    regression_percent: float,
    now: dt.datetime,
) -> dict[str, Any]:
    waiver = _exact_keys(
        raw,
        {
            "metric_id",
            "candidate_commit",
            "baseline_sha256",
            "maximum_regression_percent",
            "reason",
            "review_url",
            "approved_by",
            "approved_at_utc",
            "expires_at_utc",
        },
        "performance waiver",
    )
    if waiver["metric_id"] != metric_id or "*" in str(waiver["metric_id"]):
        raise AssuranceError("performance waiver metric is not exact")
    if waiver["candidate_commit"] != candidate_commit:
        raise AssuranceError("performance waiver commit does not match candidate")
    if waiver["baseline_sha256"] != accepted_baseline_sha256:
        raise AssuranceError("performance waiver baseline does not match")
    maximum = waiver["maximum_regression_percent"]
    if isinstance(maximum, bool) or not isinstance(maximum, (int, float)):
        raise AssuranceError("performance waiver maximum must be numeric")
    maximum = float(maximum)
    if not math.isfinite(maximum) or maximum < regression_percent or maximum > 100.0:
        raise AssuranceError("performance waiver maximum is below the regression or unsafe")
    _bounded_text(
        waiver["reason"], policy["waivers"]["maximum_reason_bytes"], "waiver reason"
    )
    _https_url(waiver["review_url"], "waiver review URL")
    approver = _operator(waiver["approved_by"], "waiver approver")
    if approver.casefold() == candidate_operator.casefold():
        raise AssuranceError("performance waiver approval must be independent from the operator")
    approved = _utc(waiver["approved_at_utc"], "waiver approval time")
    expires = _utc(waiver["expires_at_utc"], "waiver expiry time")
    if approved > now:
        raise AssuranceError("performance waiver approval is in the future")
    if expires <= now:
        raise AssuranceError("performance waiver expired")
    if expires - approved > dt.timedelta(days=policy["waivers"]["maximum_duration_days"]):
        raise AssuranceError("performance waiver duration exceeds policy")
    return waiver


def _validate_waiver_document(value: object, policy: dict[str, Any]) -> list[dict[str, Any]]:
    document = _exact_keys(value, {"schema", "waivers"}, "waiver document")
    if document["schema"] != 1 or not isinstance(document["waivers"], list):
        raise AssuranceError("waiver document schema changed")
    if len(document["waivers"]) > policy["limits"]["max_metrics"]:
        raise AssuranceError("waiver count exceeds policy")
    required = {
        "metric_id",
        "candidate_commit",
        "baseline_sha256",
        "maximum_regression_percent",
        "reason",
        "review_url",
        "approved_by",
        "approved_at_utc",
        "expires_at_utc",
    }
    for index, raw in enumerate(document["waivers"]):
        waiver = _exact_keys(raw, required, f"performance waiver {index}")
        metric_id = _bounded_text(
            waiver["metric_id"],
            policy["limits"]["max_metric_id_bytes"],
            f"performance waiver {index} metric",
        )
        if "*" in metric_id or METRIC_ID_RE.fullmatch(metric_id) is None or ".." in metric_id:
            raise AssuranceError("performance waiver metric is not exact")
        _commit(waiver["candidate_commit"], "performance waiver candidate commit")
        baseline_sha = waiver["baseline_sha256"]
        if not isinstance(baseline_sha, str) or re.fullmatch(r"[0-9a-f]{64}", baseline_sha) is None:
            raise AssuranceError("performance waiver baseline digest is invalid")
        maximum = waiver["maximum_regression_percent"]
        if isinstance(maximum, bool) or not isinstance(maximum, (int, float)):
            raise AssuranceError("performance waiver maximum must be numeric")
        if not math.isfinite(float(maximum)) or not 0.0 < float(maximum) <= 100.0:
            raise AssuranceError("performance waiver maximum is unsafe")
        _bounded_text(
            waiver["reason"],
            policy["waivers"]["maximum_reason_bytes"],
            "waiver reason",
        )
        _https_url(waiver["review_url"], "waiver review URL")
        _operator(waiver["approved_by"], "waiver approver")
        _utc(waiver["approved_at_utc"], "waiver approval time")
        _utc(waiver["expires_at_utc"], "waiver expiry time")
    return document["waivers"]


def load_waivers(path: Path, policy: dict[str, Any]) -> dict[str, Any]:
    value = read_json(path, policy["limits"]["max_waiver_bytes"], "waiver document")
    _validate_waiver_document(value, policy)
    return value


def evaluate(
    policy: dict[str, Any],
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    waiver_document: dict[str, Any],
    *,
    now: dt.datetime,
    require_active: bool,
    expected_commit: str,
) -> dict[str, Any]:
    validate_policy(policy)
    validate_baseline(baseline, policy, require_active=require_active)
    validate_evidence(candidate, policy)
    validate_candidate_context(
        candidate, policy, expected_commit=expected_commit, now=now
    )
    waiver_values = _validate_waiver_document(waiver_document, policy)
    if baseline["status"] != "active":
        return {
            "schema": 1,
            "status": "collecting",
            "candidate_commit": candidate["commit"],
            "baseline_sha256": None,
            "thresholds": dict(policy["thresholds"]),
            "comparisons": [],
            "regressions": [],
            "waivers_applied": [],
            "missing_claims": sorted(
                set(policy["required_claims"])
                - {metric["claim"] for metric in candidate["metrics"]}
            ),
            "unclassified_metric_count": len(candidate["unclassified_metrics"]),
        }
    if baseline["runner"] != candidate["runner"]:
        raise AssuranceError("candidate runner fingerprint does not match accepted baseline")
    if candidate["unclassified_metrics"]:
        raise AssuranceError("candidate contains unclassified metrics")
    validate_active_baseline_context(baseline, policy, now=now)
    daily_by_id: dict[str, list[float]] = {}
    baseline_metadata: dict[str, tuple[str, str, str]] = {}
    for day in baseline["days"]:
        for item in day["metrics"]:
            daily_by_id.setdefault(item["id"], []).append(float(item["point"]))
            baseline_metadata[item["id"]] = (item["claim"], item["family"], item["unit"])
    candidate_by_id = {item["id"]: item for item in candidate["metrics"]}
    if set(candidate_by_id) != set(daily_by_id):
        raise AssuranceError("candidate metric set does not match accepted baseline")

    accepted_sha256 = baseline_digest(baseline)
    comparisons: list[dict[str, Any]] = []
    regressions: list[dict[str, Any]] = []
    applied: list[dict[str, str]] = []
    used_waivers: set[int] = set()
    for metric_id in sorted(daily_by_id):
        candidate_metric = candidate_by_id[metric_id]
        claim, family, unit = baseline_metadata[metric_id]
        if (candidate_metric["claim"], candidate_metric["family"], candidate_metric["unit"]) != (
            claim,
            family,
            unit,
        ):
            raise AssuranceError(f"candidate metric {metric_id} family, claim, or unit changed")
        baseline_point = float(statistics.median(daily_by_id[metric_id]))
        candidate_point = float(candidate_metric["point"])
        change = (candidate_point / baseline_point - 1.0) * 100.0
        allowed = float(policy["thresholds"][f"{family}_percent"])
        verdict = "pass"
        if change > allowed + 1e-9:
            matching = [
                (index, raw)
                for index, raw in enumerate(waiver_values)
                if isinstance(raw, dict) and raw.get("metric_id") == metric_id
            ]
            if len(matching) > 1:
                raise AssuranceError(f"multiple waivers target metric {metric_id}")
            if matching:
                index, raw = matching[0]
                waiver = _validate_waiver(
                    raw,
                    policy,
                    metric_id=metric_id,
                    candidate_commit=candidate["commit"],
                    candidate_operator=candidate["operator"],
                    accepted_baseline_sha256=accepted_sha256,
                    regression_percent=change,
                    now=now,
                )
                used_waivers.add(index)
                applied.append(
                    {
                        "metric_id": metric_id,
                        "review_url": waiver["review_url"],
                        "expires_at_utc": waiver["expires_at_utc"],
                    }
                )
                verdict = "waived"
            else:
                verdict = "regression"
                regressions.append(
                    {
                        "metric_id": metric_id,
                        "claim": claim,
                        "family": family,
                        "change_percent": round(change, 6),
                        "allowed_percent": allowed,
                    }
                )
        comparisons.append(
            {
                "metric_id": metric_id,
                "claim": claim,
                "family": family,
                "unit": unit,
                "baseline_point": baseline_point,
                "candidate_point": candidate_point,
                "candidate_lower": float(candidate_metric["lower"]),
                "candidate_upper": float(candidate_metric["upper"]),
                "samples": candidate_metric["samples"],
                "change_percent": round(change, 6),
                "allowed_percent": allowed,
                "verdict": verdict,
            }
        )
    if len(used_waivers) != len(waiver_values):
        raise AssuranceError("waiver document contains an unused or non-regression waiver")
    status = "fail" if regressions else ("pass-with-waiver" if applied else "pass")
    return {
        "schema": 1,
        "status": status,
        "candidate_commit": candidate["commit"],
        "baseline_sha256": accepted_sha256,
        "thresholds": dict(policy["thresholds"]),
        "comparisons": comparisons,
        "regressions": regressions,
        "waivers_applied": applied,
    }


def build_baseline(
    evidence_documents: list[dict[str, Any]],
    accepted: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    minimum = policy["baseline"]["minimum_consecutive_days"]
    maximum = policy["baseline"]["maximum_days"]
    if not minimum <= len(evidence_documents) <= maximum:
        raise AssuranceError(
            f"baseline builder requires {minimum}..{maximum} daily evidence files"
        )
    validate_policy(policy)
    _validate_acceptance(accepted)
    validated: list[dict[str, Any]] = []
    for index, evidence in enumerate(evidence_documents):
        validate_evidence(evidence, policy)
        if evidence["unclassified_metrics"]:
            raise AssuranceError(f"baseline evidence {index} contains unclassified metrics")
        validated.append(evidence)
    validated.sort(key=lambda item: _utc(item["measured_at_utc"], "baseline measurement"))
    first_runner = validated[0]["runner"]
    if any(item["runner"] != first_runner for item in validated[1:]):
        raise AssuranceError("baseline evidence runner fingerprint changed")
    latest_measurement = _utc(validated[-1]["measured_at_utc"], "latest baseline measurement")
    if _utc(accepted["at_utc"], "baseline acceptance time") < latest_measurement:
        raise AssuranceError("baseline acceptance predates its final measurement")

    baseline = {
        "schema": 1,
        "status": "active",
        "policy_sha256": policy_digest(policy),
        "runner": first_runner,
        "days": [
            {
                "date": _utc(item["measured_at_utc"], "baseline measurement")
                .date()
                .isoformat(),
                "commit": item["commit"],
                "measured_at_utc": item["measured_at_utc"],
                "operator": item["operator"],
                "evidence_sha256": _digest(item),
                "metrics": item["metrics"],
            }
            for item in validated
        ],
        "accepted": accepted,
    }
    return validate_baseline(baseline, policy, require_active=True)


def write_document(path: Path, document: dict[str, Any], maximum_bytes: int) -> None:
    payload = json.dumps(document, indent=2, sort_keys=True, allow_nan=False).encode("utf-8") + b"\n"
    if len(payload) > maximum_bytes:
        raise AssuranceError(f"performance document exceeds {maximum_bytes} bytes")
    path.parent.mkdir(parents=True, exist_ok=True)
    cursor = path.parent
    while True:
        try:
            info = cursor.lstat()
        except OSError as error:
            raise AssuranceError("performance output parent is unavailable") from error
        if _is_link_or_reparse(info) or not stat.S_ISDIR(info.st_mode):
            raise AssuranceError("performance output parent must not traverse a link")
        if cursor == cursor.parent:
            break
        cursor = cursor.parent
    if path.exists() or path.is_symlink():
        info = path.lstat()
        if _is_link_or_reparse(info) or not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
            raise AssuranceError("performance output must not replace a linked file")
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


def write_report(path: Path, report: dict[str, Any]) -> None:
    write_document(path, report, MAX_REPORT_BYTES)


def _classify(metric_id: str, policy: dict[str, Any]) -> str | None:
    for rule in policy["criterion_claim_rules"]:
        if fnmatch.fnmatchcase(metric_id.casefold(), rule["glob"].casefold()):
            return rule["claim"]
    return None


def _criterion_estimate(document: object, label: str) -> tuple[float, float, float]:
    if not isinstance(document, dict):
        raise AssuranceError(f"{label} must contain a JSON object")
    estimate = document.get("slope") or document.get("mean")
    if not isinstance(estimate, dict):
        raise AssuranceError(f"{label} lacks a slope or mean estimate")
    interval = estimate.get("confidence_interval")
    if not isinstance(interval, dict):
        raise AssuranceError(f"{label} lacks a confidence interval")
    if interval.get("confidence_level") != EXPECTED_QUALITY["criterion_confidence_level"]:
        raise AssuranceError(f"{label} confidence level changed")
    raw_values = (
        interval.get("lower_bound"),
        estimate.get("point_estimate"),
        interval.get("upper_bound"),
    )
    values: list[float] = []
    for raw in raw_values:
        if isinstance(raw, bool) or not isinstance(raw, (int, float)):
            raise AssuranceError(f"{label} estimate is not numeric")
        number = float(raw)
        if not math.isfinite(number) or number <= 0:
            raise AssuranceError(f"{label} estimate is not finite and positive")
        values.append(number)
    if not values[0] <= values[1] <= values[2]:
        raise AssuranceError(f"{label} confidence interval is invalid")
    return values[0], values[1], values[2]

def _criterion_sample_count(path: Path, policy: dict[str, Any], label: str) -> int:
    sample = read_json(
        path,
        policy["limits"]["max_criterion_sample_file_bytes"],
        f"{label} samples",
    )
    sample = _exact_keys(sample, {"sampling_mode", "iters", "times"}, f"{label} samples")
    if sample["sampling_mode"] not in {"Linear", "Flat"}:
        raise AssuranceError(f"{label} sampling mode is unsupported")
    iterations = sample["iters"]
    times = sample["times"]
    minimum = policy["quality"]["minimum_criterion_samples"]
    maximum = policy["quality"]["maximum_criterion_samples"]
    if (
        not isinstance(iterations, list)
        or not isinstance(times, list)
        or len(iterations) != len(times)
        or not minimum <= len(iterations) <= maximum
    ):
        raise AssuranceError(f"{label} samples are outside {minimum}..{maximum}")
    for raw in (*iterations, *times):
        if isinstance(raw, bool) or not isinstance(raw, (int, float)):
            raise AssuranceError(f"{label} samples must be numeric")
        number = float(raw)
        if not math.isfinite(number) or number <= 0:
            raise AssuranceError(f"{label} samples must be finite and positive")
    return len(iterations)


def _discover_criterion_results(
    criterion_root: Path, policy: dict[str, Any]
) -> list[Path]:
    try:
        root_info = criterion_root.lstat()
    except OSError as error:
        raise AssuranceError("could not inspect Criterion root") from error
    if _is_link_or_reparse(root_info) or not stat.S_ISDIR(root_info.st_mode):
        raise AssuranceError("Criterion root must be one non-symlinked directory")
    maximum_entries = policy["limits"]["max_criterion_entries"]
    maximum_depth = policy["limits"]["max_criterion_depth"]
    entries = 0
    results: list[Path] = []
    stack: list[tuple[Path, int]] = [(criterion_root, 0)]
    while stack:
        directory, depth = stack.pop()
        if depth > maximum_depth:
            raise AssuranceError("Criterion result tree exceeds the reviewed depth")
        try:
            directory_info = directory.lstat()
        except OSError as error:
            raise AssuranceError("could not inspect Criterion directory") from error
        if _is_link_or_reparse(directory_info) or not stat.S_ISDIR(directory_info.st_mode):
            raise AssuranceError("Criterion result tree contains a linked directory")
        try:
            with os.scandir(directory) as children:
                for child in children:
                    entries += 1
                    if entries > maximum_entries:
                        raise AssuranceError("Criterion result tree exceeds the entry ceiling")
                    child_info = child.stat(follow_symlinks=False)
                    if _is_link_or_reparse(child_info) or child.is_symlink():
                        raise AssuranceError("Criterion result tree contains a link")
                    child_path = Path(child.path)
                    if child.is_dir(follow_symlinks=False):
                        stack.append((child_path, depth + 1))
                    elif child.is_file(follow_symlinks=False):
                        if child.name == "estimates.json" and child_path.parent.name == "new":
                            results.append(child_path)
                    else:
                        raise AssuranceError("Criterion result tree contains a special file")
        except AssuranceError:
            raise
        except OSError as error:
            raise AssuranceError("could not traverse Criterion results") from error
    results.sort()
    if not results or len(results) > policy["limits"]["max_metrics"]:
        raise AssuranceError("Criterion result count is outside bounds")
    return results


def collect_criterion(
    criterion_root: Path,
    runner: dict[str, str],
    commit: str,
    measured_at_utc: str,
    operator: str,
    policy: dict[str, Any],
) -> dict[str, Any]:
    validate_policy(policy)
    validate_runner(runner, policy)
    _commit(commit)
    _utc(measured_at_utc, "measurement time")
    _operator(operator, "candidate operator")
    paths = _discover_criterion_results(criterion_root, policy)
    metrics: list[dict[str, Any]] = []
    unclassified: list[str] = []
    for path in paths:
        relative = path.parent.parent.relative_to(criterion_root).as_posix()
        _bounded_text(relative, policy["limits"]["max_metric_id_bytes"], "Criterion metric id")
        if any(parent.is_symlink() for parent in (path, *path.parents) if parent != criterion_root.parent):
            raise AssuranceError(f"Criterion result {relative} traverses a symlink")
        document = read_json(
            path,
            policy["limits"]["max_criterion_file_bytes"],
            f"Criterion result {relative}",
        )
        lower, point, upper = _criterion_estimate(document, f"Criterion result {relative}")
        samples = _criterion_sample_count(path.with_name("sample.json"), policy, f"Criterion result {relative}")
        claim = _classify(relative, policy)
        if claim is None:
            unclassified.append(relative)
            continue
        metrics.append(
            {
                "id": relative,
                "claim": claim,
                "family": "latency",
                "unit": "ns",
                "point": point,
                "lower": lower,
                "upper": upper,
                "samples": samples,
            }
        )
    metrics.sort(key=lambda item: item["id"])
    unclassified.sort()
    if not metrics:
        raise AssuranceError("Criterion results contain no classified metrics")
    evidence = {
        "schema": 1,
        "kind": "candidate",
        "commit": commit,
        "measured_at_utc": measured_at_utc,
        "operator": operator,
        "runner": runner,
        "metrics": metrics,
        "unclassified_metrics": unclassified,
    }
    return validate_evidence(evidence, policy)


def collect_native_resource(
    report_path: Path,
    runner: dict[str, str],
    commit: str,
    measured_at_utc: str,
    operator: str,
    policy: dict[str, Any],
) -> dict[str, Any]:
    validate_policy(policy)
    validate_runner(runner, policy)
    _commit(commit)
    _utc(measured_at_utc, "measurement time")
    _operator(operator, "candidate operator")
    document = read_json(
        report_path,
        policy["limits"]["max_evidence_bytes"],
        "native resource report",
    )
    report = _exact_keys(
        document,
        {
            "schema_version",
            "panel_count_at_final_sample",
            "baseline",
            "final",
            "delta",
            "ceilings",
            "typography_frame",
            "fullscreen_brightness",
            "painted_frame",
            "modal_composition",
            "image_preview_pixels",
            "image_preview_lifecycle",
        },
        "native resource report",
    )
    if report["schema_version"] != 1:
        raise AssuranceError("native resource report schema changed")
    sample_keys = {
        "timestamp_utc",
        "handle_count",
        "thread_count",
        "private_bytes",
        "working_set_bytes",
        "descendant_process_count",
    }
    samples: dict[str, dict[str, Any]] = {}
    for label in ("baseline", "final"):
        sample = _exact_keys(report[label], sample_keys, f"native resource {label}")
        _utc(sample["timestamp_utc"], f"native resource {label} time")
        for field in sample_keys - {"timestamp_utc"}:
            value = sample[field]
            if isinstance(value, bool) or not isinstance(value, int) or value < 0:
                raise AssuranceError(f"native resource {label} {field} is invalid")
        samples[label] = sample
    if _utc(samples["final"]["timestamp_utc"], "native resource final time") < _utc(
        samples["baseline"]["timestamp_utc"], "native resource baseline time"
    ):
        raise AssuranceError("native resource samples are out of order")

    metrics = []
    for field in ("private_bytes", "working_set_bytes"):
        baseline = float(samples["baseline"][field])
        final = float(samples["final"][field])
        if baseline <= 0 or final <= 0:
            raise AssuranceError(f"native resource {field} must be positive")
        metrics.append(
            {
                "id": f"native_resize/{field}",
                "claim": "memory",
                "family": "memory",
                "unit": "bytes",
                "point": final,
                "lower": min(baseline, final),
                "upper": max(baseline, final),
                "samples": 2,
            }
        )
    evidence = {
        "schema": 1,
        "kind": "candidate",
        "commit": commit,
        "measured_at_utc": measured_at_utc,
        "operator": operator,
        "runner": runner,
        "metrics": metrics,
        "unclassified_metrics": [],
    }
    return validate_evidence(evidence, policy)


def merge_candidate_evidence(
    primary: dict[str, Any],
    supplemental: list[dict[str, Any]],
    policy: dict[str, Any],
) -> dict[str, Any]:
    """Merge independently measured, already-normalized evidence fail closed."""
    validate_evidence(primary, policy)
    merged_metrics = list(primary["metrics"])
    unclassified = list(primary["unclassified_metrics"])
    identity = {
        "commit": primary["commit"],
        "measured_at_utc": primary["measured_at_utc"],
        "operator": primary["operator"],
        "runner": primary["runner"],
    }
    for index, evidence in enumerate(supplemental):
        validate_evidence(evidence, policy)
        if any(evidence[field] != value for field, value in identity.items()):
            raise AssuranceError(
                f"supplemental evidence {index} does not match commit, time, and runner"
            )
        if evidence["unclassified_metrics"]:
            raise AssuranceError(
                f"supplemental evidence {index} contains unclassified metrics"
            )
        merged_metrics.extend(evidence["metrics"])
    if len(merged_metrics) > policy["limits"]["max_metrics"]:
        raise AssuranceError("merged performance metric count exceeds policy")
    merged_metrics.sort(key=lambda item: item["id"])
    merged = {
        "schema": 1,
        "kind": "candidate",
        **identity,
        "metrics": merged_metrics,
        "unclassified_metrics": sorted(set(unclassified)),
    }
    return validate_evidence(merged, policy)


def _runner_from_args(args: argparse.Namespace) -> dict[str, str]:
    return {field: str(getattr(args, field)) for field in EXPECTED_COMPARABILITY_FIELDS}


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("check-policy")

    collect = subcommands.add_parser("collect-criterion")
    collect.add_argument("--criterion-root", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--commit", required=True)
    collect.add_argument("--operator", required=True)
    collect.add_argument("--measured-at-utc", required=True)
    collect.add_argument("--supplemental", type=Path, action="append", default=[])
    collect.add_argument("--require-classified", action="store_true")
    for field in EXPECTED_COMPARABILITY_FIELDS:
        collect.add_argument(f"--{field.replace('_', '-')}", dest=field, required=True)

    native = subcommands.add_parser("collect-native-resource")
    native.add_argument("--report", type=Path, required=True)
    native.add_argument("--output", type=Path, required=True)
    native.add_argument("--commit", required=True)
    native.add_argument("--operator", required=True)
    native.add_argument("--measured-at-utc", required=True)
    for field in EXPECTED_COMPARABILITY_FIELDS:
        native.add_argument(f"--{field.replace('_', '-')}", dest=field, required=True)

    build = subcommands.add_parser("build-baseline")
    build.add_argument("--evidence", type=Path, action="append", required=True)
    build.add_argument("--accepted-by", required=True)
    build.add_argument("--accepted-at-utc", required=True)
    build.add_argument("--review-url", required=True)
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--expected-source-commit", required=True)

    compare = subcommands.add_parser("evaluate")
    compare.add_argument("--candidate", type=Path, required=True)
    compare.add_argument("--baseline", type=Path, default=BASELINE_PATH)
    compare.add_argument("--waivers", type=Path, default=WAIVERS_PATH)
    compare.add_argument("--output", type=Path, required=True)
    compare.add_argument("--expected-commit", required=True)
    compare.add_argument("--require-active", action="store_true")

    validate = subcommands.add_parser("validate-baseline")
    validate.add_argument("--baseline", type=Path, default=BASELINE_PATH)
    validate.add_argument("--expected-source-commit", required=True)
    validate.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        policy = load_policy()
        if args.command == "check-policy":
            baseline = load_baseline(BASELINE_PATH, policy, require_active=False)
            load_waivers(WAIVERS_PATH, policy)
            print(
                "PASS: S1/S2 performance policy is bounded and remains "
                f"{baseline['status']} (latency 5%, memory 10%, 30 consecutive days)"
            )
            return 0
        if args.command == "collect-criterion":
            require_clean_source(args.commit)
            evidence = collect_criterion(
                args.criterion_root,
                _runner_from_args(args),
                args.commit,
                args.measured_at_utc,
                args.operator,
                policy,
            )
            supplemental = [load_evidence(path, policy) for path in args.supplemental]
            evidence = merge_candidate_evidence(evidence, supplemental, policy)
            if args.require_classified and evidence["unclassified_metrics"]:
                raise AssuranceError("controlled candidate contains unclassified metrics")
            write_report(args.output, evidence)
            print(
                f"PASS: normalized {len(evidence['metrics'])} performance metrics; "
                f"{len(evidence['unclassified_metrics'])} require classification"
            )
            return 0
        if args.command == "collect-native-resource":
            require_clean_source(args.commit)
            evidence = collect_native_resource(
                args.report,
                _runner_from_args(args),
                args.commit,
                args.measured_at_utc,
                args.operator,
                policy,
            )
            write_report(args.output, evidence)
            print(f"PASS: normalized {len(evidence['metrics'])} native memory metrics")
            return 0
        if args.command == "build-baseline":
            require_clean_source(args.expected_source_commit)
            evidences = [load_evidence(path, policy) for path in args.evidence]
            baseline = build_baseline(
                evidences,
                {
                    "by": args.accepted_by,
                    "at_utc": args.accepted_at_utc,
                    "review_url": args.review_url,
                },
                policy,
            )
            write_document(args.output, baseline, policy["limits"]["max_baseline_bytes"])
            print(f"PASS: built reviewed active baseline from {len(baseline['days'])} days")
            return 0
        if args.command == "validate-baseline":
            require_clean_source(args.expected_source_commit)
            baseline = load_baseline(args.baseline, policy, require_active=True)
            validate_active_baseline_context(
                baseline, policy, now=dt.datetime.now(dt.timezone.utc)
            )
            digest = baseline_digest(baseline)
            write_report(
                args.output,
                {
                    "schema": 1,
                    "status": "active",
                    "source_commit": args.expected_source_commit,
                    "baseline_sha256": digest,
                    "days": len(baseline["days"]),
                    "accepted": baseline["accepted"],
                },
            )
            print(
                f"PASS: active S2 baseline has {len(baseline['days'])} reviewed days "
                f"and digest {digest}"
            )
            return 0
        if args.command == "evaluate":
            require_clean_source(args.expected_commit)
            baseline = load_baseline(args.baseline, policy, require_active=args.require_active)
            candidate = load_evidence(args.candidate, policy)
            waivers = load_waivers(args.waivers, policy)
            report = evaluate(
                policy,
                baseline,
                candidate,
                waivers,
                now=dt.datetime.now(dt.timezone.utc),
                require_active=args.require_active,
                expected_commit=args.expected_commit,
            )
            write_report(args.output, report)
            print(
                f"S2 performance assurance: {report['status']} "
                f"({len(report['comparisons'])} comparisons, "
                f"{len(report['regressions'])} regressions)"
            )
            return 1 if report["status"] == "fail" else 0
        raise AssuranceError("unknown command")
    except (AssuranceError, OSError) as error:
        print(f"S1/S2 performance assurance failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
