#!/usr/bin/env python3
"""Regression tests for the feature assurance ledger validator."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


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
        # The count is a reviewed drift sentinel in addition to the production
        # checker's requirement that every discovered benchmark has evidence.
        self.assertEqual(counts["benchmarks"], 26)
        discovered = ASSURANCE.benchmark_targets(ASSURANCE.ROOT)
        self.assertIn(
            "automexia-extension-api/benches/text_compaction.rs", discovered
        )
        self.assertIn(
            "automexia-ui-model/benches/semantic_table_presentation.rs", discovered
        )
        self.assertEqual(counts["fuzz_targets"], 19)
        self.assertGreaterEqual(counts["documentation"], counts["features"] * 3)

    def test_benchmark_inventory_ignores_private_generated_and_local_tool_copies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            product = root / "crate" / "benches" / "real.rs"
            private = root / ".automexia-private" / "copy" / "benches" / "real.rs"
            local_tools = root / ".automexia-tools" / "copy" / "benches" / "real.rs"
            generated = root / "target" / "copy" / "benches" / "real.rs"
            for path in (product, private, local_tools, generated):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fn main() {}\n", encoding="utf-8")

            self.assertEqual(
                ASSURANCE.benchmark_targets(root),
                {"crate/benches/real.rs"},
            )

    def test_benchmark_inventory_prunes_caches_before_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "crate" / "benches" / "real.rs"
            source.parent.mkdir(parents=True)
            source.write_text("fn main() {}\n", encoding="utf-8")
            scandir = os.scandir
            expected_visits = {".", "crate", "crate/benches"}
            for cache_size in (1, 32):
                for excluded in ("target", ".git", ".automexia-private", ".automexia-tools"):
                    for index in range(cache_size):
                        cache = root / excluded / str(index) / "benches"
                        cache.mkdir(parents=True, exist_ok=True)
                        (cache / "copy.rs").write_text("fn main() {}\n", encoding="utf-8")
                visited = []

                def observe(directory):
                    relative = Path(directory).relative_to(root).as_posix()
                    visited.append(relative)
                    self.assertFalse(
                        {"target", ".git", ".automexia-private", ".automexia-tools"}.intersection(Path(relative).parts),
                        "cache traversal must be pruned, not filtered afterward",
                    )
                    return scandir(directory)

                # Operation counts remain constant as ignored trees grow. Merely
                # comparing returned paths let cache-size-dependent stalls escape.
                with mock.patch("os.scandir", side_effect=observe):
                    self.assertEqual(ASSURANCE.benchmark_targets(root), {"crate/benches/real.rs"})
                self.assertEqual(set(visited), expected_visits)
                self.assertEqual(len(visited), len(expected_visits))

    def test_benchmark_inventory_does_not_hide_unreadable_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "crate").mkdir()
            scandir = os.scandir

            def deny_source(directory):
                if Path(directory) == root / "crate":
                    raise PermissionError("fixture source denied")
                return scandir(directory)

            with mock.patch("os.scandir", side_effect=deny_source):
                with self.assertRaisesRegex(ASSURANCE.AssuranceError, "benchmark inventory cannot read"):
                    ASSURANCE.benchmark_targets(root)

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

    def test_ui_text_benchmark_requires_the_renderer_performance_owner(self) -> None:
        for benchmark in ("sugarloaf/benches/ui_text.rs",
                          "tools/renderer-benchmarks/benches/text_fit.rs"):
            with self.subTest(benchmark=benchmark):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"]
                               if item["id"] == "renderer-fonts-responsive-ui")
                self.assertIn(benchmark, ASSURANCE.benchmark_targets(ASSURANCE.ROOT))
                evidence = feature["quality"]["performance"]["evidence"]
                evidence[evidence.index(benchmark)] = "sugarloaf/src/text.rs"
                with self.assertRaisesRegex(ASSURANCE.AssuranceError, "benchmark targets missing"):
                    ASSURANCE.validate_document(document)

    def test_prompt_parser_benchmark_requires_performance_evidence(self) -> None:
        document = copy.deepcopy(self.document)
        feature = next(item for item in document["features"]
                       if item["id"] == "prompt-context-devops-semantics")
        benchmark = "automexia-devops/benches/kubernetes_context.rs"
        self.assertIn(benchmark, ASSURANCE.benchmark_targets(ASSURANCE.ROOT))
        evidence = feature["quality"]["performance"]["evidence"]
        # A documented parser is not measured performance evidence: keep the
        # reference count unchanged while removing the actual benchmark owner.
        evidence[evidence.index(benchmark)] = "automexia-devops/src/kubernetes.rs"
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "benchmark targets missing"):
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

    def test_text_benchmark_requires_performance_evidence_not_a_reference_count(self) -> None:
        document = copy.deepcopy(self.document)
        feature = next(
            item for item in document["features"]
            if item["id"] == "extension-contract-runtime"
        )
        benchmark = "automexia-extension-api/benches/text_compaction.rs"
        self.assertIn(benchmark, ASSURANCE.benchmark_targets(ASSURANCE.ROOT))
        evidence = feature["quality"]["performance"]["evidence"]
        index = evidence.index(benchmark)
        # Keep the evidence count and the path elsewhere: neither proves a
        # performance owner when the actual benchmark loses that classification.
        evidence[index] = "automexia-extension-api/tests/text_boundaries.rs"
        feature["quality"]["correctness"]["evidence"].append(benchmark)
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "benchmark targets missing"):
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
