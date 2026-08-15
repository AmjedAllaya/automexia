#!/usr/bin/env python3
"""Regression tests for source-to-documentation coverage validation."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_documentation_coverage.py")
SPEC = importlib.util.spec_from_file_location(
    "automexia_documentation_coverage", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_documentation_coverage.py")
COVERAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COVERAGE)


class DocumentationCoverageTests(unittest.TestCase):
    def test_canonical_documentation_covers_source_registries(self) -> None:
        counts = COVERAGE.validate()
        self.assertGreaterEqual(counts["pages"], 11)
        self.assertGreaterEqual(counts["config_keys"], 100)
        self.assertGreaterEqual(counts["binding_actions"], 60)
        self.assertEqual(counts["cli_flags"], 6)
        self.assertGreaterEqual(counts["xtask_commands"], 20)

    def test_missing_config_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            COVERAGE.DocumentationCoverageError, "scrollback-history-limit"
        ):
            COVERAGE.require_tokens(
                "configuration reference", {"scrollback-history-limit"}, "other"
            )

    def test_missing_binding_action_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            COVERAGE.DocumentationCoverageError, "previewselectedimage"
        ):
            COVERAGE.require_tokens(
                "keyboard reference", {"previewselectedimage"}, "copy paste"
            )

    def test_matching_is_case_insensitive(self) -> None:
        self.assertEqual(
            COVERAGE.require_tokens(
                "keyboard reference", {"reloadconfig"}, "ReloadConfig"
            ),
            1,
        )


    def test_canonical_page_requires_one_level_one_heading(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            page = root / "docs" / "index.md"
            page.parent.mkdir(parents=True)
            page.write_text("# First\n\n# Second\n", encoding="utf-8")
            with self.assertRaisesRegex(
                COVERAGE.DocumentationCoverageError,
                "exactly one level-one heading",
            ):
                COVERAGE.validate_canonical_pages(root, {"docs/index.md"})

if __name__ == "__main__":
    unittest.main(verbosity=2)
