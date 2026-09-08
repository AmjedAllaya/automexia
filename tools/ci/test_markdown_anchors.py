#!/usr/bin/env python3
"""Independent characterization of the three documentation-anchor consumers."""

from __future__ import annotations

from pathlib import Path
import tempfile
import unittest
from unittest import mock

import check_feature_assurance as assurance
import check_feature_test_reinforcement as reinforcement
import validate_repository as repository
from markdown_anchors import markdown_anchors


CONSUMERS = (
    repository.markdown_anchors,
    assurance.markdown_anchors,
    reinforcement._markdown_anchors,
)


class MarkdownAnchorTests(unittest.TestCase):
    def test_all_consumers_use_the_single_parser_owner(self) -> None:
        for consumer in CONSUMERS:
            self.assertIs(consumer, markdown_anchors)

    def check_document(self, text: str, expected: set[str]) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "guide.md"
            path.write_text(text, encoding="utf-8", newline="\n")
            for consumer in CONSUMERS:
                with self.subTest(consumer=consumer.__module__):
                    self.assertEqual(consumer(path), expected)

    def test_plain_repeated_and_closing_markers(self) -> None:
        self.check_document(
            "# Guide\n## Section\n### Section ###\n###### Section\n",
            {"guide", "section", "section-1", "section-2"},
        )

    def test_unicode_punctuation_and_inline_markup(self) -> None:
        self.check_document(
            "# Café 中文\n## **Bold** `code` <em>text</em>!\n"
            "## under_score / dash--separated\n",
            {"café-中文", "bold-code-text", "under_score-dash-separated"},
        )

    def test_non_headings_and_missing_title(self) -> None:
        self.check_document("#\n####### Not a heading\n#No space\nordinary\n", set())
        self.check_document("", set())

    def test_legacy_empty_and_indented_heading_behavior(self) -> None:
        self.check_document("# \n# !!!\n  ## Indented\n", {""})

    def test_fenced_examples_never_prove_a_documentation_anchor(self) -> None:
        for opening, closing in (("```text", "```"), ("  ~~~~", "   ~~~~~")):
            self.check_document(
                f"# Real\n{opening}\n## Example\n{closing}\n# After\n",
                {"real", "after"},
            )
        # Short or mismatched delimiters cannot expose headings inside a fence.
        self.check_document(
            "# Real\n````\n```\n# Hidden\n~~~~\n## Also hidden\n````\n# After\n",
            {"real", "after"},
        )
        self.check_document("# Real\n```\n# Hidden\n", {"real"})

    def test_natural_suffixes_and_duplicate_headings_remain_distinct(self) -> None:
        self.check_document(
            "# Name\n# Name\n# Name-1\n# Name\n# Name-1\n",
            {"name", "name-1", "name-1-1", "name-2", "name-1-2"},
        )
        self.check_document(
            "# Name-1\n# Name\n# Name\n# Name\n",
            {"name-1", "name", "name-2", "name-3"},
        )

    def test_legacy_links_and_explicit_anchor_behavior(self) -> None:
        self.check_document(
            '<a id="custom"></a>\n# [Link](example.md)\n# Title {#explicit}\n',
            {"linkexamplemd", "title-explicit"},
        )

    def test_invalid_utf8_and_missing_files_do_not_become_empty_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "guide.md"
            for consumer in CONSUMERS:
                with self.assertRaises(FileNotFoundError):
                    consumer(path)
            path.write_bytes(b"# Title\n\xff")
            for consumer in CONSUMERS:
                with self.assertRaises(UnicodeDecodeError):
                    consumer(path)

    def test_real_consumers_reject_missing_and_stale_anchors(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            guide = root / "guide.md"
            guide.write_text(
                "# Title\n## Repeated\n## Repeated\n```\n## Hidden\n```\n",
                encoding="utf-8",
            )
            links = root / "links.md"
            cases = (
                ("title", True), ("repeated-1", True),
                ("repeated-2", False), ("missing", False), ("hidden", False),
            )
            for anchor, valid in cases:
                links.write_text(f"# Links\n[section](guide.md#{anchor})\n", encoding="utf-8")
                with self.subTest(anchor=anchor), mock.patch.object(
                    repository, "ROOT", root
                ):
                    if valid:
                        self.assertEqual(repository.validate_markdown_links(), 2)
                        assurance.validate_fragment(guide, f"guide.md#{anchor}")
                    else:
                        with self.assertRaisesRegex(ValueError, "missing Markdown anchor"):
                            repository.validate_markdown_links()
                        with self.assertRaisesRegex(assurance.AssuranceError, "missing Markdown anchor"):
                            assurance.validate_fragment(guide, f"guide.md#{anchor}")

    def test_repository_parses_each_target_once_without_retaining_stale_results(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            guide = root / "guide.md"
            guide.write_text("# First\n## Second\n", encoding="utf-8")
            (root / "links.md").write_text(
                "[first](guide.md#first)\n[second](guide.md#second)\n"
                "[again](guide.md#first)\n", encoding="utf-8"
            )
            with mock.patch.object(repository, "ROOT", root), mock.patch.object(
                repository, "markdown_anchors", wraps=markdown_anchors
            ) as parser:
                self.assertEqual(repository.validate_markdown_links(), 2)
                self.assertEqual(parser.call_count, 1)
                # A later validation must read changed bytes, not reuse prior evidence.
                guide.write_text("# Replacement\n", encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "missing Markdown anchor"):
                    repository.validate_markdown_links()
                self.assertEqual(parser.call_count, 2)


if __name__ == "__main__":
    unittest.main()
