#!/usr/bin/env python3
"""Regression tests for the feature assurance ledger validator."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_feature_assurance.py")
SPEC = importlib.util.spec_from_file_location("automexia_feature_assurance", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_feature_assurance.py")
ASSURANCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ASSURANCE)


class FeatureAssuranceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = json.loads(ASSURANCE.DEFAULT_MATRIX.read_text(encoding="utf-8"))

    def test_canonical_matrix_covers_every_workspace_member_and_required_surface(self) -> None:
        counts = ASSURANCE.validate_document(copy.deepcopy(self.document))
        self.assertGreaterEqual(counts["features"], 12)
        self.assertGreaterEqual(counts["components"], 20)
        self.assertGreater(counts["evidence"], counts["features"] * 9)
        self.assertEqual(counts["benchmarks"], 5)
        self.assertEqual(counts["fuzz_targets"], 7)
        self.assertGreaterEqual(counts["documentation"], counts["features"] * 3)

    def test_missing_documentation_type_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        del document["features"][0]["documentation"]["explanation"]
        with self.assertRaisesRegex(
            ASSURANCE.AssuranceError, "documentation must define exactly"
        ):
            ASSURANCE.validate_document(document)

    def test_missing_documentation_anchor_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["documentation"]["reference"] = [
            "docs/CONFIGURATION.md#missing-section"
        ]
        with self.assertRaisesRegex(
            ASSURANCE.AssuranceError, "missing Markdown anchor"
        ):
            ASSURANCE.validate_document(document)

    def test_non_markdown_documentation_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["documentation"]["guide"] = ["Cargo.toml"]
        with self.assertRaisesRegex(
            ASSURANCE.AssuranceError, "must reference Markdown"
        ):
            ASSURANCE.validate_document(document)

    def test_missing_quality_dimension_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        del document["features"][0]["quality"]["security"]
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "quality must define exactly"):
            ASSURANCE.validate_document(document)

    def test_missing_native_platform_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        del document["features"][0]["platforms"]["macos"]
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "platforms must define exactly"):
            ASSURANCE.validate_document(document)

    def test_unverifiable_evidence_path_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["quality"]["correctness"]["evidence"] = [
            "tests/does-not-exist.rs"
        ]
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "missing evidence"):
            ASSURANCE.validate_document(document)

    def test_unverifiable_workflow_job_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["platforms"]["windows"]["evidence"] = [
            ".github/workflows/ci.yml#missing-job"
        ]
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "missing workflow job"):
            ASSURANCE.validate_document(document)
    def test_orphaned_benchmark_target_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        feature = next(
            item
            for item in document["features"]
            if item["id"] == "pty-scheduler-process-lifecycle"
        )
        feature["quality"]["performance"]["evidence"].remove(
            "teletypewriter/benches/pty_io.rs"
        )
        with self.assertRaisesRegex(
            ASSURANCE.AssuranceError, "benchmark targets missing"
        ):
            ASSURANCE.validate_document(document)

    def test_orphaned_fuzz_target_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        feature = next(
            item
            for item in document["features"]
            if item["id"] == "identity-config-migration"
        )
        feature["quality"]["security"]["evidence"].remove(
            "fuzz/fuzz_targets/config_migration.rs"
        )
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "fuzz targets missing"):
            ASSURANCE.validate_document(document)
    def test_not_applicable_requires_an_explicit_rationale(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["quality"]["performance"] = {
            "level": "not_applicable",
            "evidence": [],
        }
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "requires a rationale"):
            ASSURANCE.validate_document(document)

    def test_duplicate_feature_id_is_rejected(self) -> None:
        document = copy.deepcopy(self.document)
        duplicate = copy.deepcopy(document["features"][0])
        document["features"].append(duplicate)
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "duplicate feature id"):
            ASSURANCE.validate_document(document)


if __name__ == "__main__":
    unittest.main(verbosity=2)
