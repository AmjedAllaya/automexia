"""Contract tests for raw application evidence, separate from the S2 release gate."""

from copy import deepcopy
import importlib
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

BENCHMARKS = Path(__file__).resolve().parents[1] / "renderer-benchmarks" / "application"
sys.path.insert(0, str(BENCHMARKS))
bench = importlib.import_module("benchmark_model")


def profile():
    return {
        "runner_class": "controlled-fixture", "os": "linux", "os_version": "fixture-1",
        "arch": "x86_64", "cpu": "Fixture CPU", "logical_cpus": 8,
        "gpu": "Fixture GPU", "gpu_driver": "fixture-1", "power_mode": "performance",
        "display_width": 1920, "display_height": 1080, "scale_milli": 1000,
        "refresh_millihertz": 60000, "compositor": "fixture", "renderer": "wgpu",
        "toolchain": "fixture-rust", "build_profile": "release",
        "features": ["application-benchmarks"],
        "config_sha256": "1" * 64, "fonts_sha256": "2" * 64,
        "workload_sha256": "3" * 64,
    }


def evidence(value=100):
    return {
        "schema": 1, "kind": "application-benchmark", "commit": "a" * 40,
        "dirty": False, "artifact_sha256": "4" * 64,
        "measured_at_utc": "2026-10-06T12:00:00Z", "profile": profile(),
        "metrics": {
            name: {"status": "measured", "unit": spec.unit,
                   "runs": [[value] * spec.minimum_samples for _ in range(5)]}
            for name, spec in bench.METRICS.items()
        },
    }


class ApplicationBenchmarkTests(unittest.TestCase):
    def test_roster_covers_application_cost_without_fps(self):
        self.assertEqual(set(bench.METRICS), {
            "startup_window", "startup_terminal_frame", "idle_rss", "idle_cpu",
            "idle_gpu", "input_present", "output_1m", "resize_reflow", "search",
            "four_pane_rss", "four_pane_input_present", "image_kitty",
            "image_iterm2", "image_sixel", "image_rss_delta",
        })
        self.assertEqual(bench.METRICS["output_1m"].unit, "lines/s")
        self.assertEqual(bench.METRICS["idle_rss"].unit, "bytes")

    def test_raw_samples_survive_and_nearest_rank_percentiles_are_explicit(self):
        document = evidence()
        original = deepcopy(document)
        self.assertEqual(bench.validate_evidence(document), original)
        self.assertEqual(document, original)
        self.assertEqual(bench.percentile(list(range(1, 101)), 50), 50)
        self.assertEqual(bench.percentile(list(range(1, 101)), 95), 95)
        self.assertEqual(bench.summarize(document)["input_present"]["p95"], 100)

    def test_missing_and_unknown_metrics_or_fields_fail(self):
        for mutation in (lambda d: d["metrics"].pop("idle_gpu"),
                         lambda d: d["metrics"].update(fps=d["metrics"]["idle_cpu"]),
                         lambda d: d.update(hostname="private-host")):
            document = evidence(); mutation(document)
            with self.assertRaises(bench.BenchmarkError):
                bench.validate_evidence(document)

    def test_invalid_units_numbers_counts_and_unavailable_payloads_fail(self):
        for value in (float("nan"), float("inf"), -1, True):
            with self.subTest(value=value):
                document = evidence(); document["metrics"]["input_present"]["runs"][0][0] = value
                with self.assertRaises(bench.BenchmarkError):
                    bench.validate_evidence(document)
        for replacement in ({"status": "measured", "unit": "ms", "runs": [[1]]},
                            {"status": "measured", "unit": "ns", "runs": []},
                            {"status": "unavailable", "reason": "not-run", "runs": [[0]]}):
            document = evidence(); document["metrics"]["input_present"] = replacement
            with self.assertRaises(bench.BenchmarkError):
                bench.validate_evidence(document)

    def test_missing_sensor_is_visible_and_never_becomes_zero(self):
        document = evidence()
        document["metrics"]["idle_gpu"] = {"status": "unavailable", "reason": "sensor-unavailable"}
        bench.validate_evidence(document)
        summary = bench.summarize(document)["idle_gpu"]
        self.assertEqual(summary, {"status": "unavailable", "reason": "sensor-unavailable"})
        self.assertEqual(bench.compare(evidence(), document)["status"], "incomplete")

    def test_identical_runs_pass_but_regressions_fail_in_both_directions(self):
        baseline = evidence()
        self.assertEqual(bench.compare(baseline, deepcopy(baseline))["status"], "pass")
        for metric, value in (("input_present", 120), ("idle_rss", 120), ("output_1m", 80)):
            with self.subTest(metric=metric):
                candidate = evidence()
                candidate["metrics"][metric]["runs"] = [[value] * bench.METRICS[metric].minimum_samples for _ in range(5)]
                result = bench.compare(baseline, candidate)
                self.assertEqual(result["status"], "regression")
                self.assertIn(metric, result["regressions"])

    def test_zero_idle_counter_uses_absolute_percentage_point_budget(self):
        baseline = evidence(); candidate = evidence()
        for document in (baseline, candidate):
            document["metrics"]["idle_cpu"]["runs"] = [[0] for _ in range(5)]
        self.assertEqual(bench.compare(baseline, candidate)["status"], "pass")
        candidate["metrics"]["idle_cpu"]["runs"] = [[2] for _ in range(5)]
        self.assertEqual(bench.compare(baseline, candidate)["status"], "regression")

    def test_profile_mismatch_dirty_source_and_different_workloads_cannot_pass(self):
        for field, value in (("os_version", "fixture-2"), ("gpu_driver", "fixture-2"),
                             ("config_sha256", "8" * 64), ("fonts_sha256", "9" * 64),
                             ("scale_milli", 2000), ("features", [])):
            with self.subTest(field=field):
                candidate = evidence(); candidate["profile"][field] = value
                self.assertEqual(bench.compare(evidence(), candidate)["status"], "incomparable")
        candidate = evidence(); candidate["dirty"] = True
        self.assertEqual(bench.compare(evidence(), candidate)["status"], "incomparable")

    def test_noisy_runs_are_not_a_successful_regression_check(self):
        candidate = evidence()
        candidate["metrics"]["input_present"]["runs"][-1] = [300] * 50
        result = bench.compare(evidence(), candidate)
        self.assertEqual(result["status"], "inconclusive")
        self.assertIn("input_present", result["unstable"])

    def test_private_paths_unknown_keys_and_secret_shaped_metadata_are_rejected(self):
        for value in ("C:\\Users\\private\\driver", "/home/private/driver", "ghp_" + "x" * 30, "bad\nvalue"):
            document = evidence(); document["profile"]["gpu_driver"] = value
            with self.assertRaises(bench.BenchmarkError):
                bench.validate_evidence(document)

    def test_bounded_json_reader_rejects_duplicates_and_linked_input(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / "sample.json"
            path.write_text('{"schema":1,"schema":2}', encoding="utf-8")
            with self.assertRaises(bench.BenchmarkError):
                bench.load(path)
            path.write_text(json.dumps(evidence()), encoding="utf-8")
            import os
            linked = Path(temporary) / "linked.json"
            os.link(path, linked)
            with self.assertRaises(bench.BenchmarkError):
                bench.load(linked)

    def test_cli_produces_json_and_nonzero_for_a_real_regression(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            baseline = evidence(); candidate = evidence()
            candidate["metrics"]["idle_rss"]["runs"] = [[150] for _ in range(5)]
            for name, document in (("baseline", baseline), ("candidate", candidate)):
                (root / f"{name}.json").write_text(json.dumps(document), encoding="utf-8")
            result = subprocess.run([sys.executable, str(BENCHMARKS / "benchmark.py"), "compare",
                                     "--baseline", str(root / "baseline.json"), "--candidate", str(root / "candidate.json"),
                                     "--output", str(root / "comparison.json")], capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 1, result.stderr.decode())
            self.assertEqual(json.loads((root / "comparison.json").read_text())["status"], "regression")


if __name__ == "__main__":
    unittest.main()
