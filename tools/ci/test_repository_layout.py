#!/usr/bin/env python3
"""Protect canonical file owners without rejecting independent test fixtures."""

from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import validate_repository as validator


class RepositoryLayoutTests(unittest.TestCase):
    def test_current_repository_has_one_layout_owner(self):
        validator.validate_source_layout()

    def test_root_repair_scripts_are_rejected(self):
        for extension in ("py", "ps1", "sh", "bat", "cmd", "PS1"):
            with self.subTest(extension=extension), TemporaryDirectory() as directory:
                root = Path(directory)
                (root / f"one_off.{extension}").write_text("", encoding="utf-8")
                with patch.object(validator, "ROOT", root):
                    with self.assertRaisesRegex(ValueError, "root-level scripts"):
                        validator.validate_source_layout()

    def test_stale_and_identical_adr_mirrors_are_rejected(self):
        for text in ("# Decision\n", "# Outdated decision\n"):
            with self.subTest(text=text), TemporaryDirectory() as directory:
                root = Path(directory)
                canonical = root / "docs/adr"
                duplicate = root / "docs/project/adr"
                canonical.mkdir(parents=True)
                duplicate.mkdir(parents=True)
                (canonical / "0001-boundary.md").write_text("# Decision\n", encoding="utf-8")
                (duplicate / "0001-boundary.md").write_text(text, encoding="utf-8")
                with patch.object(validator, "ROOT", root):
                    with self.assertRaisesRegex(ValueError, "only in docs/adr"):
                        validator.validate_source_layout()

    def test_tools_fixtures_licenses_and_private_work_remain_independent(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("tools/repair.py", "tests/fixture.py", "docs/adr/0001.md",
                         "crate-a/LICENSE", "crate-b/LICENSE",
                         ".automexia-private/old/adr/0001.md"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("independent content\n", encoding="utf-8")
            with patch.object(validator, "ROOT", root):
                validator.validate_source_layout()


if __name__ == "__main__":
    unittest.main()
