"""Versioned raw application measurements; never a replacement for S2 evidence.

Only numeric observations and reviewed public machine/configuration identities
enter this format. Terminal contents, commands, paths and environment are absent.
"""

from dataclasses import dataclass
import datetime as dt
import hashlib
import json
import math
from pathlib import Path
import re
import statistics
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ci"))
import performance_assurance as io

MAX_BYTES = 4 * 1024 * 1024
MAX_RUNS = 30
MAX_SAMPLES = 10_000
MAX_TOTAL_SAMPLES = 150_000
MIN_RUNS = 5
MAX_RUN_SPREAD_PERCENT = 10.0
REASONS = {"not-run", "unsupported", "sensor-unavailable", "timeout", "failed"}


class BenchmarkError(ValueError):
    pass


@dataclass(frozen=True)
class Metric:
    unit: str
    minimum_samples: int = 1
    higher_is_better: bool = False
    regression_percent: float = 5.0
    absolute_budget: float = 0.0


METRICS = {
    "startup_window": Metric("ns"),
    "startup_terminal_frame": Metric("ns"),
    "idle_rss": Metric("bytes", regression_percent=10),
    "idle_cpu": Metric("percent-one-core", absolute_budget=1),
    "idle_gpu": Metric("percent-device", absolute_budget=1),
    "input_present": Metric("ns", minimum_samples=50),
    "output_1m": Metric("lines/s", higher_is_better=True),
    "resize_reflow": Metric("ns", minimum_samples=10),
    "search": Metric("ns", minimum_samples=10),
    "four_pane_rss": Metric("bytes", regression_percent=10),
    "four_pane_input_present": Metric("ns", minimum_samples=50),
    "image_kitty": Metric("ns"),
    "image_iterm2": Metric("ns"),
    "image_sixel": Metric("ns"),
    "image_rss_delta": Metric("bytes", regression_percent=10),
}
PROFILE_TEXT = {
    "runner_class", "os", "os_version", "arch", "cpu", "gpu", "gpu_driver",
    "power_mode", "compositor", "renderer", "toolchain", "build_profile",
}
PROFILE_INTS = {
    "logical_cpus": (1, 4096), "display_width": (320, 16384),
    "display_height": (200, 16384), "scale_milli": (500, 8000),
    "refresh_millihertz": (1000, 1_000_000),
}
PROFILE_DIGESTS = {"config_sha256", "fonts_sha256", "workload_sha256"}
DIGEST = re.compile(r"[0-9a-f]{64}")


def exact(value, keys, label):
    if not isinstance(value, dict) or value.keys() != keys:
        raise BenchmarkError(f"invalid {label} fields")
    return value


def digest(value, label):
    if not isinstance(value, str) or DIGEST.fullmatch(value) is None:
        raise BenchmarkError(f"invalid {label} digest")


def validate_profile(value):
    exact(value, PROFILE_TEXT | set(PROFILE_INTS) | PROFILE_DIGESTS | {"features"}, "profile")
    for field in PROFILE_TEXT:
        text = value[field]
        if (not isinstance(text, str) or not 1 <= len(text.encode("utf-8")) <= 160
                or text != text.strip() or any(ord(c) < 32 or ord(c) == 127 for c in text)
                or io.ABSOLUTE_PATH_RE.search(text) or io.SECRET_RE.search(text)
                or "://" in text or "\\" in text):
            raise BenchmarkError("unsafe public profile metadata")
    if value["os"] not in {"windows", "linux", "macos"}:
        raise BenchmarkError("unsupported measurement OS")
    for field, (low, high) in PROFILE_INTS.items():
        if type(value[field]) is not int or not low <= value[field] <= high:
            raise BenchmarkError(f"invalid profile {field}")
    for field in PROFILE_DIGESTS:
        digest(value[field], field)
    features = value["features"]
    if (not isinstance(features, list) or len(features) > 32
            or any(not isinstance(x, str) or re.fullmatch(r"[a-z0-9][a-z0-9-]{0,63}", x) is None for x in features)
            or features != sorted(set(features))):
        raise BenchmarkError("invalid feature identity")
    if "native-gui-test-hooks" in features or "visual-test-hooks" in features:
        raise BenchmarkError("polling GUI test hooks invalidate application timing")
    return value


def validate_evidence(value):
    exact(value, {"schema", "kind", "commit", "dirty", "artifact_sha256",
                  "measured_at_utc", "profile", "metrics"}, "evidence")
    if type(value["schema"]) is not int or value["schema"] != 1 or value["kind"] != "application-benchmark":
        raise BenchmarkError("unsupported application evidence schema")
    if not isinstance(value["commit"], str) or re.fullmatch(r"[0-9a-f]{40}", value["commit"]) is None:
        raise BenchmarkError("invalid source revision")
    if type(value["dirty"]) is not bool:
        raise BenchmarkError("source cleanliness must be explicit")
    digest(value["artifact_sha256"], "artifact")
    timestamp = value["measured_at_utc"]
    if not isinstance(timestamp, str) or re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", timestamp) is None:
        raise BenchmarkError("invalid UTC measurement time")
    try:
        dt.datetime.strptime(timestamp, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as error:
        raise BenchmarkError("invalid UTC measurement time") from error
    profile = validate_profile(value["profile"])
    exact(value["metrics"], set(METRICS), "metric roster")
    total = 0
    for name, spec in METRICS.items():
        metric = value["metrics"][name]
        if isinstance(metric, dict) and metric.get("status") == "unavailable":
            exact(metric, {"status", "reason"}, "unavailable metric")
            if not isinstance(metric["reason"], str) or metric["reason"] not in REASONS:
                raise BenchmarkError("invalid unavailable reason")
            continue
        exact(metric, {"status", "unit", "runs"}, "measured metric")
        if metric["status"] != "measured" or metric["unit"] != spec.unit:
            raise BenchmarkError("metric status or unit mismatch")
        runs = metric["runs"]
        if not isinstance(runs, list) or not MIN_RUNS <= len(runs) <= MAX_RUNS:
            raise BenchmarkError("independent run count outside bounds")
        for samples in runs:
            if not isinstance(samples, list) or not spec.minimum_samples <= len(samples) <= MAX_SAMPLES:
                raise BenchmarkError("sample count outside bounds")
            total += len(samples)
            if total > MAX_TOTAL_SAMPLES:
                raise BenchmarkError("aggregate sample budget exceeded")
            for sample in samples:
                if type(sample) not in (int, float):
                    raise BenchmarkError("sample must be numeric")
                if sample < 0 or sample > 2 ** 50 or not math.isfinite(sample):
                    raise BenchmarkError("sample outside finite nonnegative bounds")
                if sample == 0 and name not in {"idle_cpu", "idle_gpu", "image_rss_delta"}:
                    raise BenchmarkError("zero duration, rate or resident memory is invalid")
                if name == "idle_cpu" and sample > 100 * profile["logical_cpus"]:
                    raise BenchmarkError("CPU counter exceeds one-core normalization")
                if name == "idle_gpu" and sample > 100:
                    raise BenchmarkError("GPU utilization exceeds device capacity")
    return value


def load(path):
    try:
        return validate_evidence(io.read_json(Path(path), MAX_BYTES, "application evidence"))
    except (io.AssuranceError, OSError, OverflowError, RecursionError) as error:
        raise BenchmarkError("could not read valid bounded application evidence") from error


def write(path, document):
    try:
        io.write_document(Path(path), document, MAX_BYTES)
    except (io.AssuranceError, OSError, ValueError) as error:
        raise BenchmarkError("could not publish bounded benchmark document") from error


def percentile(samples, rank):
    """Nearest rank (no interpolation), including p50 and p95 for raw series."""
    if not samples or not 0 < rank <= 100:
        raise BenchmarkError("percentile needs nonempty samples and a valid rank")
    ordered = sorted(samples)
    return ordered[math.ceil(rank * len(ordered) / 100) - 1]


def summarize(document):
    validate_evidence(document)
    result = {}
    for name, spec in METRICS.items():
        metric = document["metrics"][name]
        if metric["status"] == "unavailable":
            result[name] = dict(metric)
            continue
        samples = [x for run in metric["runs"] for x in run]
        summary = {"status": "measured", "unit": spec.unit,
                   "samples": len(samples), "independent_runs": len(metric["runs"])}
        for rank in (50, 95):
            run_values = [percentile(run, rank) for run in metric["runs"]]
            point = percentile(samples, rank)
            summary[f"p{rank}"] = point
            # These are observed run ranges, explicitly NOT confidence intervals.
            summary[f"p{rank}_run_min"] = min(run_values)
            summary[f"p{rank}_run_max"] = max(run_values)
            span = max(run_values) - min(run_values)
            denominator = max(statistics.median(run_values), spec.absolute_budget)
            summary[f"p{rank}_run_spread_percent"] = span / denominator * 100 if denominator else 0
        result[name] = summary
    return result


def compare(baseline, candidate):
    validate_evidence(baseline); validate_evidence(candidate)
    profile_digest = hashlib.sha256(json.dumps(candidate["profile"], sort_keys=True).encode()).hexdigest()
    report = {"schema": 1, "kind": "application-comparison", "status": "incomparable",
              "baseline_commit": baseline["commit"], "candidate_commit": candidate["commit"],
              "profile_sha256": profile_digest, "regressions": [], "unstable": [],
              "incomplete": [], "uncertain": [], "metrics": {}}
    if baseline["profile"] != candidate["profile"] or baseline["dirty"] or candidate["dirty"]:
        return report
    before, after = summarize(baseline), summarize(candidate)
    for name, spec in METRICS.items():
        left, right = before[name], after[name]
        if left["status"] != "measured" or right["status"] != "measured":
            report["incomplete"].append(name)
            continue
        if [len(x) for x in baseline["metrics"][name]["runs"]] != [len(x) for x in candidate["metrics"][name]["runs"]]:
            report["incomplete"].append(name)
            continue
        report["metrics"][name] = {"baseline": left, "candidate": right}
        if any(summary[f"p{rank}_run_spread_percent"] > MAX_RUN_SPREAD_PERCENT
               for summary in (left, right) for rank in (50, 95)):
            report["unstable"].append(name)
            continue
        regression, uncertain = False, False
        for rank in (50, 95):
            low, high = f"p{rank}_run_min", f"p{rank}_run_max"
            # Conservative comparison of independent run ranges, with an explicit
            # percentage-point budget for idle counters near zero.
            if spec.higher_is_better:
                allowed = 1 - spec.regression_percent / 100
                regression |= right[high] < left[low] * allowed
                uncertain |= right[low] < left[high] * allowed
            else:
                allowed = 1 + spec.regression_percent / 100
                regression |= right[low] > max(left[high] * allowed, left[high] + spec.absolute_budget)
                uncertain |= right[high] > max(left[low] * allowed, left[low] + spec.absolute_budget)
        if regression:
            report["regressions"].append(name)
        elif uncertain:
            report["uncertain"].append(name)
    report["status"] = ("regression" if report["regressions"] else
                        "incomplete" if report["incomplete"] else
                        "inconclusive" if report["unstable"] or report["uncertain"] else "pass")
    return report
