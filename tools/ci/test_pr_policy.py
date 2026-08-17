#!/usr/bin/env python3
"""Unit tests for pull-request documentation update policy."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_pr_policy.py")
SPEC = importlib.util.spec_from_file_location("automexia_pr_policy", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load pull-request policy checker")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class PullRequestDocumentationPolicyTests(unittest.TestCase):
    def test_source_change_without_documentation_is_rejected(self) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for({"apps/automexia-terminal/src/main.rs"}),
            ["apps/automexia-terminal/src/main.rs"],
        )

    def test_documentation_in_same_change_satisfies_policy(self) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for(
                {
                    "apps/automexia-terminal/src/main.rs",
                    "docs/CLI-REFERENCE.md",
                    "changes/feature.md",
                }
            ),
            [],
        )

    def test_test_workflow_and_configuration_changes_require_docs(self) -> None:
        changed = {
            ".github/workflows/ci.yml",
            "Cargo.lock",
            "tests/fixtures/example.json",
            "tools/ci/check_example.py",
        }
        self.assertEqual(
            POLICY.missing_documentation_for(changed),
            sorted(changed),
        )

    def test_assets_and_packaging_changes_require_docs(self) -> None:
        changed = {"assets/brand/icon.svg", "packaging/windows/main.wxs"}
        self.assertEqual(
            POLICY.missing_documentation_for(changed),
            sorted(changed),
        )

    def test_changelog_and_documentation_only_changes_do_not_require_more_docs(
        self,
    ) -> None:
        self.assertEqual(
            POLICY.missing_documentation_for(
                {"changes/docs.md", "docs/DOCUMENTATION.md"}
            ),
            [],
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
