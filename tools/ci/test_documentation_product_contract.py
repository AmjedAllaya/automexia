#!/usr/bin/env python3
"""Catch drift in cross-platform entry pages and canonical user references."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


class DocumentationProductContractTests(unittest.TestCase):
    def test_product_entry_pages_introduce_all_three_desktop_platforms(self):
        for name in ("README.md", "docs/INSTALLATION.md", "docs/PRODUCT-VISION.md",
                     "docs/PLATFORMS.md", "docs/user-guide/index.md"):
            with self.subTest(page=name):
                intro = read(name)[:1500]
                for platform in ("Windows", "Linux", "macOS"):
                    self.assertIn(platform, intro)
                self.assertIn("cross-platform", intro)

    def test_palette_guides_do_not_document_the_clear_history_key_as_palette(self):
        for name in ("docs/CONFIGURATION.md", "docs/user-guide/customization.md",
                     "docs/user-guide/quick-actions.md"):
            with self.subTest(page=name):
                text = read(name)
                self.assertNotIn("Ctrl/Cmd+K", text)
                self.assertIn("Ctrl+Shift+P", text)
                self.assertIn("Cmd+Shift+P", text)

    def test_reset_instructions_name_the_actual_current_snapshot_pair(self):
        source = read("apps/automexia-terminal/src/automexia/preferences.rs")
        reference = read("docs/CONFIGURATION.md")
        section = reference.split("## Saved runtime preferences", 1)[1].split("\n## ", 1)[0]
        for constant in ("PRIMARY_FILE", "PREVIOUS_FILE"):
            match = re.search(rf'const {constant}: &str = "([^"]+)";', source)
            self.assertIsNotNone(match)
            self.assertIn(match.group(1), section)

    def test_consolidation_preserves_unique_useful_reference_content(self):
        self.assertIn("[presentation.window-controls]", read("docs/CONFIGURATION.md"))
        cli = read("docs/CLI-REFERENCE.md")
        self.assertIn("COLUMNS", cli)
        self.assertIn("amx.exe", cli)

    def test_source_build_guide_uses_current_public_checkout(self):
        self.assertIn("git clone https://github.com/AmjedAllaya/automexia.git", read("docs/INSTALLATION.md"))
        self.assertNotIn("access to the private source repository", read("README.md"))


if __name__ == "__main__":
    unittest.main()
