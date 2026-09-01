#!/usr/bin/env python3
"""Mutation tests for bounded vendored-license and notice validation."""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_vendored_licenses.py")
SPEC = importlib.util.spec_from_file_location("automexia_vendored_licenses", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_vendored_licenses.py")
LICENSES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(LICENSES)


def fixture(root: Path) -> None:
    notices = root / "THIRD_PARTY_NOTICES.md"
    notices.write_text(
        "Mozilla Public License 2.0\nSIL Open Font License 1.1\n"
        "Nerd Fonts Symbols Only\n",
        encoding="utf-8",
    )
    nerd = root / "rio-fonts/resources/SymbolsNerdFontMono/LICENSE"
    nerd.parent.mkdir(parents=True)
    nerd.write_text("SIL OPEN FONT LICENSE Version 1.1\n", encoding="utf-8")
    cascadia = root / "sugarloaf/src/font/resources/CascadiaCode/LICENSE"
    cascadia.parent.mkdir(parents=True)
    cascadia.write_text("SIL Open Font License, Version 1.1\n", encoding="utf-8")
    runtime = root / "sugarloaf/src/components/filters/runtime/filter.rs"
    runtime.parent.mkdir(parents=True)
    runtime.write_text("// SPDX-License-Identifier: MPL-2.0\n", encoding="utf-8")


class VendoredLicenseTests(unittest.TestCase):
    def test_minimal_compatible_fixture_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture(root)
            self.assertEqual(LICENSES.validate(root), [])

    def test_forbidden_marker_missing_notice_and_runtime_notice_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture(root)
            forbidden = root / "misc/asset.txt"
            forbidden.parent.mkdir()
            forbidden.write_text("Licensed under GNU 3.0\n", encoding="utf-8")
            (root / "THIRD_PARTY_NOTICES.md").write_text(
                "Mozilla Public License 2.0\n", encoding="utf-8"
            )
            runtime = root / "sugarloaf/src/components/filters/runtime/filter.rs"
            runtime.write_text("// missing source notice\n", encoding="utf-8")

            failures = LICENSES.validate(root)

            self.assertTrue(any("forbidden license marker" in item for item in failures))
            self.assertTrue(any("Nerd Fonts Symbols Only" in item for item in failures))
            self.assertTrue(any("lacks its MPL-2.0" in item for item in failures))

    def test_linked_and_oversized_inputs_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture(root)
            notice = root / "THIRD_PARTY_NOTICES.md"
            linked = root / "misc/linked.txt"
            linked.parent.mkdir()
            try:
                os.link(notice, linked)
            except OSError:
                self.skipTest("hard links are unavailable on this host")
            # A valid notice can still be unsafe evidence when another path can
            # change the same file between validation and packaging.
            failures = LICENSES.validate(root)
            self.assertTrue(any("regular unlinked" in item for item in failures))

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture(root)
            failures = LICENSES.validate(root, max_file_bytes=8)
            self.assertTrue(any("byte limit" in item for item in failures))


if __name__ == "__main__":
    unittest.main(verbosity=2)
