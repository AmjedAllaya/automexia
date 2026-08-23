#!/usr/bin/env python3
"""Deterministic tests for the S1/S2 performance evidence and ratchet."""

from __future__ import annotations

import copy
import datetime as dt
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("performance_assurance.py")
SPEC = importlib.util.spec_from_file_location("automexia_performance_assurance", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/performance_assurance.py")
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)


COMMIT = "1" * 40
NOW = dt.datetime(2026, 8, 23, 12, 0, tzinfo=dt.timezone.utc)


def runner() -> dict[str, str]:
    return {
        "runner_class": "linux-x64-benchmark-a",
        "os": "linux",
        "arch": "x86_64",
        "cpu": "fixture-cpu",
        "gpu": "headless",
        "power_mode": "performance-ac",
        "toolchain": "rustc-1.96.1",
        "profile": "criterion-release",
    }


def metric(metric_id: str, claim: str, family: str, point: float) -> dict[str, object]:
    unit = "bytes" if family == "memory" else "ns"
    return {
        "id": metric_id,
        "claim": claim,
        "family": family,
        "unit": unit,
        "point": point,
        "lower": point * 0.99,
        "upper": point * 1.01,
    }


class PerformanceAssuranceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.policy = PERF.load_policy()
        self.metric_templates = sorted(
            [
                metric(
                    f"fixture/{claim}",
                    claim,
                    "memory" if claim == "memory" else "latency",
                    1_000_000.0 if claim == "memory" else 1_000.0,
                )
                for claim in self.policy["required_claims"]
            ],
            key=lambda value: str(value["id"]),
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_json(self, name: str, value: object) -> Path:
        path = self.root / name
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def active_baseline(self, *, days: int = 30) -> dict[str, object]:
        start = dt.date(2026, 7, 1)
        entries = []
        for offset in range(days):
            entries.append(
                {
                    "date": (start + dt.timedelta(days=offset)).isoformat(),
                    "commit": f"{offset + 1:040x}",
                    "metrics": copy.deepcopy(self.metric_templates),
                }
            )
        return {
            "schema": 1,
            "status": "active",
            "policy_sha256": PERF.policy_digest(self.policy),
            "runner": runner(),
            "days": entries,
            "accepted": {
                "by": "release-maintainer",
                "at_utc": "2026-08-01T12:00:00Z",
                "review_url": "https://github.com/AmjedAllaya/automexia-terminal/pull/1",
            },
        }

    def candidate(self, multiplier: float = 1.0) -> dict[str, object]:
        metrics = copy.deepcopy(self.metric_templates)
        for item in metrics:
            item["point"] = float(item["point"]) * multiplier
            item["lower"] = float(item["lower"]) * multiplier
            item["upper"] = float(item["upper"]) * multiplier
        return {
            "schema": 1,
            "kind": "candidate",
            "commit": COMMIT,
            "measured_at_utc": "2026-08-23T11:00:00Z",
            "runner": runner(),
            "metrics": metrics,
            "unclassified_metrics": [],
        }

    def evaluate(
        self,
        baseline: dict[str, object],
        candidate: dict[str, object],
        waivers: list[dict[str, object]] | None = None,
    ) -> dict[str, object]:
        return PERF.evaluate(
            self.policy,
            baseline,
            candidate,
            {"schema": 1, "waivers": waivers or []},
            now=NOW,
            require_active=True,
        )

    def test_repository_policy_and_collecting_template_validate(self) -> None:
        self.assertEqual(self.policy["thresholds"], {"latency_percent": 5.0, "memory_percent": 10.0})
        template = PERF.load_baseline(PERF.BASELINE_PATH, self.policy, require_active=False)
        self.assertEqual(template["status"], "collecting")
        with self.assertRaisesRegex(PERF.AssuranceError, "not active"):
            PERF.load_baseline(PERF.BASELINE_PATH, self.policy, require_active=True)

    def test_reviewed_baseline_builder_requires_exact_complete_daily_evidence(self) -> None:
        start = dt.datetime(2026, 7, 1, 12, 0, tzinfo=dt.timezone.utc)
        evidence = []
        for offset in range(30):
            item = self.candidate()
            item["commit"] = f"{offset + 1:040x}"
            item["measured_at_utc"] = (start + dt.timedelta(days=offset)).isoformat().replace(
                "+00:00", "Z"
            )
            evidence.append(item)
        accepted = {
            "by": "release-maintainer",
            "at_utc": "2026-08-01T12:00:00Z",
            "review_url": "https://github.com/AmjedAllaya/automexia-terminal/pull/2",
        }
        baseline = PERF.build_baseline(evidence, accepted, self.policy)
        self.assertEqual(baseline["status"], "active")
        self.assertEqual(len(baseline["days"]), 30)

        evidence[10]["runner"]["cpu"] = "different"
        with self.assertRaisesRegex(PERF.AssuranceError, "runner"):
            PERF.build_baseline(evidence, accepted, self.policy)

    def test_duplicate_json_keys_and_oversized_documents_are_rejected(self) -> None:
        duplicate = self.root / "duplicate.json"
        duplicate.write_text('{"schema":1,"schema":2}', encoding="utf-8")
        with self.assertRaisesRegex(PERF.AssuranceError, "duplicate"):
            PERF.read_json(duplicate, 1024, "fixture")

        oversized = self.root / "oversized.json"
        oversized.write_bytes(b" " * 1025)
        with self.assertRaisesRegex(PERF.AssuranceError, "size"):
            PERF.read_json(oversized, 1024, "fixture")

    def test_policy_threshold_or_required_claim_drift_is_rejected(self) -> None:
        for mutate in (
            lambda value: value["thresholds"].update(latency_percent=5.1),
            lambda value: value["thresholds"].update(memory_percent=10.1),
            lambda value: value["required_claims"].remove("startup"),
            lambda value: value["limits"].update(max_metrics=10_000),
            lambda value: value["criterion_claim_rules"][0].update(glob="*"),
            lambda value: value["privacy"]["forbidden"].remove("raw-etl"),
        ):
            changed = copy.deepcopy(self.policy)
            mutate(changed)
            with self.assertRaises(PERF.AssuranceError):
                PERF.validate_policy(changed)

    def test_active_baseline_requires_thirty_consecutive_complete_days(self) -> None:
        with self.assertRaisesRegex(PERF.AssuranceError, "30 consecutive"):
            PERF.validate_baseline(self.active_baseline(days=29), self.policy, require_active=True)

        gap = self.active_baseline()
        for index in range(15, len(gap["days"])):
            shifted = dt.date.fromisoformat(gap["days"][index]["date"]) + dt.timedelta(days=1)
            gap["days"][index]["date"] = shifted.isoformat()
        with self.assertRaisesRegex(PERF.AssuranceError, "consecutive"):
            PERF.validate_baseline(gap, self.policy, require_active=True)

        incomplete = self.active_baseline()
        incomplete["days"][0]["metrics"].pop()
        with self.assertRaisesRegex(PERF.AssuranceError, "metric set"):
            PERF.validate_baseline(incomplete, self.policy, require_active=True)

    def test_runner_and_metric_identity_must_match_exactly(self) -> None:
        baseline = self.active_baseline()
        candidate = self.candidate()
        candidate["runner"]["power_mode"] = "battery"
        with self.assertRaisesRegex(PERF.AssuranceError, "runner fingerprint"):
            self.evaluate(baseline, candidate)

        candidate = self.candidate()
        candidate["metrics"][0]["unit"] = "milliseconds"
        with self.assertRaisesRegex(PERF.AssuranceError, "unit"):
            self.evaluate(baseline, candidate)

    def test_candidate_rejects_unknown_fields_nonfinite_and_unclassified_metrics(self) -> None:
        candidate = self.candidate()
        candidate["secret"] = "must-not-pass"
        with self.assertRaisesRegex(PERF.AssuranceError, "fields"):
            PERF.validate_evidence(candidate, self.policy)

        candidate = self.candidate()
        candidate["metrics"][0]["point"] = float("nan")
        with self.assertRaisesRegex(PERF.AssuranceError, "finite"):
            PERF.validate_evidence(candidate, self.policy)

        candidate = self.candidate()
        candidate["unclassified_metrics"] = ["unknown/benchmark"]
        with self.assertRaisesRegex(PERF.AssuranceError, "unclassified"):
            self.evaluate(self.active_baseline(), candidate)

    def test_latency_and_memory_thresholds_fail_above_exact_limits(self) -> None:
        result = self.evaluate(self.active_baseline(), self.candidate(1.05))
        self.assertEqual(result["status"], "pass")

        latency = self.candidate()
        latency["metrics"][0]["point"] = 1_050.1
        latency["metrics"][0]["lower"] = 1_040.0
        latency["metrics"][0]["upper"] = 1_060.0
        result = self.evaluate(self.active_baseline(), latency)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["regressions"][0]["allowed_percent"], 5.0)

        memory = self.candidate()
        memory_metric = next(item for item in memory["metrics"] if item["claim"] == "memory")
        memory_metric.update(point=1_100_001.0, lower=1_090_000.0, upper=1_110_000.0)
        result = self.evaluate(self.active_baseline(), memory)
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["regressions"][0]["allowed_percent"], 10.0)

    def test_waiver_is_exact_bounded_expiring_and_baseline_bound(self) -> None:
        baseline = self.active_baseline()
        candidate = self.candidate()
        candidate["metrics"][0].update(point=1_060.0, lower=1_050.0, upper=1_070.0)
        metric_id = candidate["metrics"][0]["id"]
        waiver = {
            "metric_id": metric_id,
            "candidate_commit": COMMIT,
            "baseline_sha256": PERF.baseline_digest(baseline),
            "maximum_regression_percent": 6.5,
            "reason": "Accepted short-term regression while issue 42 is fixed.",
            "review_url": "https://github.com/AmjedAllaya/automexia-terminal/issues/42",
            "approved_by": "release-maintainer",
            "approved_at_utc": "2026-08-22T12:00:00Z",
            "expires_at_utc": "2026-08-30T12:00:00Z",
        }
        result = self.evaluate(baseline, candidate, [waiver])
        self.assertEqual(result["status"], "pass-with-waiver")

        for field, value, message in (
            ("metric_id", "*", "metric"),
            ("candidate_commit", "2" * 40, "commit"),
            ("baseline_sha256", "0" * 64, "baseline"),
            ("maximum_regression_percent", 5.5, "maximum"),
            ("expires_at_utc", "2026-08-23T11:59:59Z", "expired"),
        ):
            changed = copy.deepcopy(waiver)
            changed[field] = value
            with self.assertRaisesRegex(PERF.AssuranceError, message):
                self.evaluate(baseline, candidate, [changed])

    def test_report_is_atomic_bounded_and_contains_no_unapproved_metadata(self) -> None:
        report = self.evaluate(self.active_baseline(), self.candidate())
        destination = self.root / "nested" / "report.json"
        PERF.write_report(destination, report)
        parsed = json.loads(destination.read_text(encoding="utf-8"))
        self.assertEqual(parsed["status"], "pass")
        self.assertNotIn("cpu", json.dumps(parsed).casefold())
        self.assertFalse(list(destination.parent.glob("*.tmp")))
        self.assertLess(destination.stat().st_size, self.policy["limits"]["max_report_bytes"])

    def test_collect_criterion_normalizes_bounds_and_reports_unclassified(self) -> None:
        criterion = self.root / "criterion"
        known = criterion / "generic_context_projection" / "new"
        known.mkdir(parents=True)
        (known / "estimates.json").write_text(
            json.dumps(
                {
                    "slope": {
                        "confidence_interval": {"lower_bound": 99.0, "upper_bound": 101.0},
                        "point_estimate": 100.0,
                        "standard_error": 0.1,
                    }
                }
            ),
            encoding="utf-8",
        )
        unknown = criterion / "mystery_measurement" / "new"
        unknown.mkdir(parents=True)
        (unknown / "estimates.json").write_text(
            (known / "estimates.json").read_text(encoding="utf-8"), encoding="utf-8"
        )

        evidence = PERF.collect_criterion(
            criterion,
            runner(),
            COMMIT,
            "2026-08-23T11:00:00Z",
            self.policy,
        )
        self.assertEqual(evidence["metrics"][0]["claim"], "context")
        self.assertEqual(evidence["metrics"][0]["unit"], "ns")
        self.assertEqual(evidence["unclassified_metrics"], ["mystery_measurement"])

    def test_native_resource_report_normalizes_only_allowlisted_memory(self) -> None:
        sample = {
            "timestamp_utc": "2026-08-23T10:00:00Z",
            "handle_count": 100,
            "thread_count": 10,
            "private_bytes": 10_000_000,
            "working_set_bytes": 8_000_000,
            "descendant_process_count": 1,
        }
        final = copy.deepcopy(sample)
        final.update(
            timestamp_utc="2026-08-23T10:30:00Z",
            private_bytes=11_000_000,
            working_set_bytes=8_500_000,
        )
        report = {
            "schema_version": 1,
            "panel_count_at_final_sample": 4,
            "baseline": sample,
            "final": final,
            "delta": {},
            "ceilings": {},
            "typography_frame": {},
            "fullscreen_brightness": {},
            "painted_frame": {},
            "modal_composition": {},
            "image_preview_pixels": {},
            "image_preview_lifecycle": {},
        }
        path = self.write_json("native-resource.json", report)
        evidence = PERF.collect_native_resource(
            path,
            runner(),
            COMMIT,
            "2026-08-23T11:00:00Z",
            self.policy,
        )
        self.assertEqual([item["family"] for item in evidence["metrics"]], ["memory", "memory"])
        self.assertNotIn("handle_count", json.dumps(evidence))

        report["unexpected"] = "not allowlisted"
        path = self.write_json("native-resource-extra.json", report)
        with self.assertRaisesRegex(PERF.AssuranceError, "fields"):
            PERF.collect_native_resource(
                path,
                runner(),
                COMMIT,
                "2026-08-23T11:00:00Z",
                self.policy,
            )

    def test_supplemental_memory_evidence_requires_exact_run_identity(self) -> None:
        primary = self.candidate()
        primary["metrics"] = [
            item for item in primary["metrics"] if item["claim"] != "memory"
        ]
        supplemental = self.candidate()
        supplemental["metrics"] = [
            item for item in supplemental["metrics"] if item["claim"] == "memory"
        ]
        merged = PERF.merge_candidate_evidence(primary, [supplemental], self.policy)
        self.assertEqual(len(merged["metrics"]), len(self.metric_templates))
        self.assertEqual(
            next(item for item in merged["metrics"] if item["claim"] == "memory")["unit"],
            "bytes",
        )

        changed = copy.deepcopy(supplemental)
        changed["runner"]["power_mode"] = "battery"
        with self.assertRaisesRegex(PERF.AssuranceError, "does not match"):
            PERF.merge_candidate_evidence(primary, [changed], self.policy)

        duplicate = copy.deepcopy(supplemental)
        duplicate["metrics"] = [copy.deepcopy(primary["metrics"][0])]
        with self.assertRaisesRegex(PERF.AssuranceError, "duplicate"):
            PERF.merge_candidate_evidence(primary, [duplicate], self.policy)

    def test_collect_criterion_rejects_too_many_or_oversized_results(self) -> None:
        criterion = self.root / "criterion"
        result = criterion / "generic_context_projection" / "new"
        result.mkdir(parents=True)
        (result / "estimates.json").write_bytes(
            b" " * (self.policy["limits"]["max_criterion_file_bytes"] + 1)
        )
        with self.assertRaisesRegex(PERF.AssuranceError, "size"):
            PERF.collect_criterion(
                criterion,
                runner(),
                COMMIT,
                "2026-08-23T11:00:00Z",
                self.policy,
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
