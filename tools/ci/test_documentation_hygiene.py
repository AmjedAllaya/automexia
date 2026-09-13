#!/usr/bin/env python3
"""Regression tests for repository Markdown hygiene validation."""

from __future__ import annotations

import importlib.util
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


MODULE_PATH = Path(__file__).with_name("check_documentation_hygiene.py")
SPEC = importlib.util.spec_from_file_location("automexia_documentation_hygiene", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load documentation hygiene checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class DocumentationHygieneTests(unittest.TestCase):
    def validate_payload(self, payload: bytes) -> dict[str, int]:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "page.md").write_bytes(payload)
            return CHECKER.validate(root)

    def test_valid_markdown_passes(self) -> None:
        self.assertEqual(
            self.validate_payload(b"# Page\n\n```text\nvalue\n```\n"),
            {"files": 1, "lines": 5},
        )

    def test_crlf_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "LF"):
            self.validate_payload(b"# Page\r\n")

    def test_missing_final_newline_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "newline"):
            self.validate_payload(b"# Page")

    def test_utf8_bom_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "byte-order"):
            self.validate_payload(b"\xef\xbb\xbf# Page\n")

    def test_invalid_utf8_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "UTF-8"):
            self.validate_payload(b"# Page\n\xff\n")

    def test_nul_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "NUL"):
            self.validate_payload(b"# Page\n\x00\n")

    def test_unclosed_fence_is_rejected(self) -> None:
        with self.assertRaisesRegex(CHECKER.DocumentationHygieneError, "unclosed"):
            self.validate_payload(b"# Page\n\n```text\nvalue\n")

    def test_merge_markers_are_rejected_even_inside_code_fences(self) -> None:
        for marker in ("<<<<<<< HEAD", ">>>>>>> incoming", "||||||| base"):
            for fenced in (False, True):
                with self.subTest(marker=marker[:7], fenced=fenced):
                    body = marker + "\n"
                    if fenced:
                        body = "```text\n" + body + "```\n"
                    with self.assertRaisesRegex(
                        CHECKER.DocumentationHygieneError, "unresolved merge marker"
                    ) as caught:
                        self.validate_payload(("# Page\n\n" + body).encode())
                    self.assertNotIn(marker, str(caught.exception))

    def test_setext_heading_is_not_a_merge_marker(self) -> None:
        self.assertEqual(
            self.validate_payload(b"Page\n=======\n"), {"files": 1, "lines": 2}
        )

    def test_private_workspace_is_excluded(self) -> None:
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "page.md").write_bytes(b"# Public\n")
            private = root / ".automexia-private"
            private.mkdir()
            (private / "draft.md").write_bytes(b"# Private\r\n")
            self.assertEqual(CHECKER.validate(root), {"files": 1, "lines": 1})


if __name__ == "__main__":
    unittest.main(verbosity=2)
