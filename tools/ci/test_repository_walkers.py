#!/usr/bin/env python3
"""Regression checks for bounded repository-tree walkers."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TEST_TEMP_PARENT = Path(ROOT.anchor) if os.name == "nt" else None


def load(name: str, relative: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


VALIDATOR = load("automexia_validate_repository", "tools/ci/validate_repository.py")
HYGIENE = load("automexia_documentation_hygiene", "tools/ci/check_documentation_hygiene.py")


class RepositoryWalkerTests(unittest.TestCase):
    def test_ignored_local_assurance_cache_is_never_treated_as_repository_source(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            (root / "tracked.yml").write_text("name: tracked\n", encoding="utf-8")
            local_cache = root / ".automexia-tools"
            local_cache.mkdir()
            (local_cache / "malformed.yml").write_text("invalid: [\n", encoding="utf-8")
            (root / "tracked.md").write_text("# Tracked\n", encoding="utf-8")
            (local_cache / "malformed.md").write_bytes(b"\xff")

            with mock.patch.object(VALIDATOR, "ROOT", root):
                structured = {
                    path.relative_to(root).as_posix()
                    for path in VALIDATOR.files_with_suffixes(".yml")
                }
            documentation = {
                path.relative_to(root).as_posix()
                for path in HYGIENE.markdown_files(root)
            }

            self.assertEqual(structured, {"tracked.yml"})
            self.assertEqual(documentation, {"tracked.md"})

    def test_testing_overview_link_uses_an_existing_markdown_anchor(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            docs = root / "docs"
            docs.mkdir()
            (docs / "index.md").write_text(
                "# Documentation\n\n[Assurance](TESTING.md#nightly-and-release-depth)\n",
                encoding="utf-8",
            )
            (docs / "TESTING.md").write_text(
                "# Testing\n\n## Nightly and release depth\n",
                encoding="utf-8",
            )
            with mock.patch.object(VALIDATOR, "ROOT", root):
                self.assertEqual(VALIDATOR.validate_markdown_links(), 2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
