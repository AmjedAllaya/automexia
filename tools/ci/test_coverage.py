#!/usr/bin/env python3
"""Regression and negative tests for the Rust coverage release gate."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_coverage.py")
SPEC = importlib.util.spec_from_file_location("automexia_coverage", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_coverage.py")
COVERAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COVERAGE)


class CoveragePolicyTests(unittest.TestCase):
    def test_absolute_checkout_sources_are_normalized_to_repository_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source = root / "automexia-connectivity" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("pub fn covered() {}\n", encoding="utf-8")
            report = root / "lcov.info"
            report.write_text(
                f"SF:{source}\nDA:1,1\nend_of_record\n", encoding="utf-8"
            )

            parsed = COVERAGE.parse_lcov(report, repo_root=root)

            self.assertEqual(
                parsed, {"automexia-connectivity/src/lib.rs": {1: 1}}
            )

    def test_windows_separators_do_not_bypass_changed_line_matching(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source = root / "extensions" / "devops-ssh" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("pub fn covered() {}\n", encoding="utf-8")
            report = root / "lcov.info"
            report.write_text(
                "SF:" + str(source).replace("/", "\\") + "\nDA:1,1\nend_of_record\n",
                encoding="utf-8",
            )

            parsed = COVERAGE.parse_lcov(report, repo_root=root)

            self.assertIn("extensions/devops-ssh/src/lib.rs", parsed)

    def test_outside_workspace_and_traversal_sources_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            report = root / "lcov.info"
            for source in (root.parent / "outside.rs", Path("../outside.rs")):
                with self.subTest(source=source):
                    report.write_text(
                        f"SF:{source}\nDA:1,1\nend_of_record\n", encoding="utf-8"
                    )
                    with self.assertRaisesRegex(
                        COVERAGE.CoveragePolicyError, "outside the repository"
                    ):
                        COVERAGE.parse_lcov(report, repo_root=root)

    def test_malformed_and_over_limit_reports_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source = root / "automexia-ui-model" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("pub fn covered() {}\n", encoding="utf-8")
            report = root / "lcov.info"
            cases = (
                (f"SF:{source}\nDA:zero,1\n", "malformed DA record"),
                (f"SF:{source}\nDA:0,1\n", "invalid executable line"),
                (f"SF:{source}\nDA:1,-1\n", "invalid hit count"),
            )
            for payload, expected in cases:
                with self.subTest(expected=expected):
                    report.write_text(payload, encoding="utf-8")
                    with self.assertRaisesRegex(COVERAGE.CoveragePolicyError, expected):
                        COVERAGE.parse_lcov(report, repo_root=root)

            report.write_text(
                f"SF:{source}\nDA:1,1\nDA:2,1\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(COVERAGE.CoveragePolicyError, "line limit"):
                COVERAGE.parse_lcov(report, repo_root=root, max_lines=1)

    def test_every_automexia_owned_crate_is_subject_to_changed_line_policy(self) -> None:
        expected = {
            "apps/automexia-terminal/src/main.rs",
            "automexia-command-productivity/src/lib.rs",
            "automexia-connectivity/src/lib.rs",
            "automexia-devops/src/lib.rs",
            "automexia-ecosystem/src/lib.rs",
            "automexia-ecosystem-runtime/src/lib.rs",
            "automexia-extension-api/src/lib.rs",
            "automexia-extension-runtime/src/lib.rs",
            "automexia-image/src/lib.rs",
            "automexia-keybindings/src/lib.rs",
            "automexia-ui-model/src/lib.rs",
            "extensions/devops-aws/src/lib.rs",
            "extensions/devops-azure/src/lib.rs",
            "extensions/devops-gcp/src/lib.rs",
            "extensions/devops-kubernetes/src/lib.rs",
            "extensions/devops-openshift/src/lib.rs",
            "extensions/devops-ssh/src/lib.rs",
            "extensions/devops-teleport/src/lib.rs",
            "tools/xtask/src/main.rs",
            "rio-backend/src/config/product.rs",
        }
        for path in sorted(expected):
            with self.subTest(path=path):
                self.assertTrue(COVERAGE.is_owned(path), path)
        self.assertFalse(COVERAGE.is_owned("rio-vt/src/lib.rs"))

    def test_changed_owned_lines_below_threshold_fail(self) -> None:
        coverage = {
            "automexia-connectivity/src/lib.rs": {1: 1, 2: 0, 3: 0, 4: 0}
        }
        changed = {"automexia-connectivity/src/lib.rs": {1, 2, 3, 4}}

        summary, failure = COVERAGE.evaluate(
            coverage,
            changed,
            baseline=20.0,
            platform="windows-x86_64-msvc",
        )

        self.assertEqual(summary["changed_owned_line_percent"], 25.0)
        self.assertEqual(summary["changed_owned_executable_lines"], 4)
        self.assertEqual(summary["uncovered_changed_owned_line_count"], 3)
        self.assertEqual(
            summary["uncovered_changed_owned_lines"],
            [
                "automexia-connectivity/src/lib.rs:2",
                "automexia-connectivity/src/lib.rs:3",
                "automexia-connectivity/src/lib.rs:4",
            ],
        )
        self.assertFalse(summary["uncovered_changed_owned_lines_truncated"])
        self.assertIn("below 80%", failure or "")

    def test_uncovered_changed_line_diagnostics_are_sorted_and_bounded(self) -> None:
        path = "automexia-connectivity/src/lib.rs"
        coverage = {path: {line: 0 for line in range(1, 81)}}

        summary, failure = COVERAGE.evaluate(
            coverage,
            {path: set(range(1, 81))},
            baseline=0.0,
            platform="windows-x86_64-msvc",
        )

        # The total remains exact, while persisted/logged identities are bounded.
        self.assertEqual(summary["uncovered_changed_owned_line_count"], 80)
        self.assertEqual(len(summary["uncovered_changed_owned_lines"]), 64)
        self.assertEqual(summary["uncovered_changed_owned_lines"][0], f"{path}:1")
        self.assertEqual(summary["uncovered_changed_owned_lines"][-1], f"{path}:64")
        self.assertTrue(summary["uncovered_changed_owned_lines_truncated"])
        self.assertIn("below 80%", failure or "")

    def test_global_regression_fails_even_when_no_owned_executable_line_changed(self) -> None:
        coverage = {"rio-vt/src/lib.rs": {1: 1, 2: 0}}

        summary, failure = COVERAGE.evaluate(
            coverage,
            {"README.rs": {1}},
            baseline=75.0,
            platform="windows-x86_64-msvc",
        )

        self.assertEqual(summary["global_line_percent"], 50.0)
        self.assertEqual(summary["changed_owned_executable_lines"], 0)
        self.assertEqual(failure, "global coverage regressed")

    def test_summary_write_is_atomic_and_rejects_link_targets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "summary.json"
            payload = {"schema_version": 2, "status": "pass"}
            COVERAGE.write_summary(destination, payload)
            self.assertEqual(json.loads(destination.read_text(encoding="utf-8")), payload)
            linked = root / "linked.json"
            os.link(destination, linked)
            with self.assertRaisesRegex(
                COVERAGE.CoveragePolicyError, "regular unlinked file"
            ):
                COVERAGE.write_summary(linked, payload)


if __name__ == "__main__":
    unittest.main(verbosity=2)
