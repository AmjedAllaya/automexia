#!/usr/bin/env python3
"""Unit tests for native completion adapter performance measurement."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "measure_completion_adapter", ROOT / "tools/ci/measure_completion_adapter.py"
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load completion adapter benchmark")
BENCHMARK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCHMARK)


class CompletionAdapterBenchmarkTests(unittest.TestCase):
    def test_p95_uses_nearest_rank(self) -> None:
        self.assertEqual(BENCHMARK.percentile_95(list(range(1, 21))), 19)
        with self.assertRaisesRegex(ValueError, "at least one"):
            BENCHMARK.percentile_95([])

    def test_budget_and_sample_contract_are_fixed(self) -> None:
        self.assertEqual(BENCHMARK.WARMUPS, 5)
        self.assertEqual(BENCHMARK.SAMPLES, 20)
        self.assertEqual(BENCHMARK.MAX_P95_MS, 50.0)
        self.assertEqual(BENCHMARK.DEADLINE_SECONDS, 2.0)

    def test_fish_wall_time_parser_is_unit_safe_and_bounded(self) -> None:
        self.assertEqual(
            BENCHMARK.parse_fish_wall_time("Executed in 750 micros"), 0.75
        )
        self.assertEqual(
            BENCHMARK.parse_fish_wall_time("Executed in 12.5 millis"), 12.5
        )
        self.assertEqual(
            BENCHMARK.parse_fish_wall_time("Executed in 0.02 secs"), 20.0
        )
        with self.assertRaisesRegex(RuntimeError, "bounded wall-clock"):
            BENCHMARK.parse_fish_wall_time("untrusted diagnostic")

    def test_alias_reload_batch_is_fixed(self) -> None:
        self.assertEqual(BENCHMARK.ALIAS_RELOAD_BATCH, 3)

    def test_exact_runner_accepts_success_and_rejects_failure(self) -> None:
        environment = {"PATH": str(Path(sys.executable).parent)}
        elapsed = BENCHMARK.run_exact(
            [sys.executable, "-c", "raise SystemExit(0)"], environment
        )
        self.assertGreaterEqual(elapsed, 0.0)
        with self.assertRaisesRegex(RuntimeError, "failed with 7"):
            BENCHMARK.run_exact(
                [sys.executable, "-c", "raise SystemExit(7)"], environment
            )

    def test_exact_runner_rejects_unbounded_diagnostics(self) -> None:
        environment = {"PATH": str(Path(sys.executable).parent)}
        with self.assertRaisesRegex(RuntimeError, "stderr ceiling"):
            BENCHMARK.run_exact(
                [
                    sys.executable,
                    "-c",
                    f"import sys; sys.stderr.write('x' * {BENCHMARK.MAX_STDERR_BYTES + 1})",
                ],
                environment,
            )


if __name__ == "__main__":
    unittest.main()
