#!/usr/bin/env python3
"""Regression tests for the bounded stable-release artifact manifest."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).resolve().parents[2] / ".github/scripts/build_release_manifest.py"
SPEC = importlib.util.spec_from_file_location("automexia_release_manifest", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load .github/scripts/build_release_manifest.py")
MANIFEST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MANIFEST)


VERSION = "1.2.3"
COMMIT = "0123456789abcdef0123456789abcdef01234567"


def package_names() -> tuple[str, ...]:
    return (
        f"automexia-terminal-{VERSION}-x86_64-pc-windows-msvc.msi",
        f"automexia-terminal-{VERSION}-x86_64-pc-windows-msvc.zip",
        f"automexia-terminal-{VERSION}-aarch64-pc-windows-msvc.msi",
        f"automexia-terminal-{VERSION}-aarch64-pc-windows-msvc.zip",
        f"automexia-terminal-{VERSION}-universal.dmg",
        f"automexia-terminal_{VERSION}-1_amd64.deb",
        f"automexia-terminal-{VERSION}-1.x86_64.rpm",
        f"automexia-terminal-{VERSION}-x86_64-unknown-linux-gnu.tar.gz",
        f"automexia-terminal_{VERSION}-1_arm64.deb",
        f"automexia-terminal-{VERSION}-1.aarch64.rpm",
        f"automexia-terminal-{VERSION}-aarch64-unknown-linux-gnu.tar.gz",
    )


def write_packages(root: Path) -> None:
    for index, name in enumerate(package_names()):
        path = root / f"download-{index}" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(f"fixture-{index}\n".encode())


class ReleaseManifestTests(unittest.TestCase):
    def test_canonical_copy_and_sbom_finalization_are_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            output = root / "release"
            source.mkdir()
            write_packages(source)

            first = MANIFEST.build_release_manifest(
                source, output, VERSION, COMMIT, copy_packages=True
            )
            self.assertEqual(len(first["artifacts"]), 11)
            # Publication is deliberately two-stage: packages are consolidated
            # before independently generated SBOMs finalize the same directory.
            (output / "automexia-terminal.spdx.json").write_text(
                '{"spdxVersion":"SPDX-2.3"}\n', encoding="utf-8"
            )
            (output / "automexia-terminal.cdx.json").write_text(
                '{"bomFormat":"CycloneDX"}\n', encoding="utf-8"
            )
            final = MANIFEST.build_release_manifest(
                output, output, VERSION, COMMIT, include_sbom=True
            )
            self.assertEqual(len(final["artifacts"]), 13)
            names = [entry["file"] for entry in final["artifacts"]]
            self.assertEqual(names, sorted(names))
            disk = json.loads((output / "release-manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(final, disk)

    def test_missing_duplicate_and_unexpected_package_slots_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            write_packages(source)
            missing = next(source.rglob("*.dmg"))
            missing.unlink()
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "incomplete"):
                MANIFEST.build_release_manifest(
                    source, root / "out-missing", VERSION, COMMIT, copy_packages=True
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            write_packages(source)
            duplicate = source / f"duplicate-{VERSION}-amd64.deb"
            duplicate.write_bytes(b"duplicate\n")
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "exactly one"):
                MANIFEST.build_release_manifest(
                    source, root / "out-duplicate", VERSION, COMMIT, copy_packages=True
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            write_packages(source)
            (source / f"automexia-terminal-{VERSION}-debug.zip").write_bytes(b"debug\n")
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "unexpected"):
                MANIFEST.build_release_manifest(
                    source, root / "out-extra", VERSION, COMMIT, copy_packages=True
                )

    def test_sbom_names_are_exact_and_complete(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_packages(root)
            for path in tuple(root.rglob("*")):
                if path.is_file():
                    path.replace(root / path.name)
            (root / "unexpected.spdx.json").write_text("{}\n", encoding="utf-8")
            (root / "automexia-terminal.cdx.json").write_text("{}\n", encoding="utf-8")
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "SBOM"):
                MANIFEST.build_release_manifest(
                    root, root, VERSION, COMMIT, include_sbom=True
                )

    def test_linked_inputs_and_resource_limits_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            write_packages(source)
            original = next(source.rglob("*.msi"))
            linked = original.with_name("linked-" + original.name)
            try:
                os.link(original, linked)
            except OSError:
                self.skipTest("hard links are unavailable on this host")
            # Equal bytes are insufficient release identity when another name
            # can mutate the same underlying file after validation.
            with self.assertRaisesRegex(
                MANIFEST.ReleaseManifestError, "regular unlinked file"
            ):
                MANIFEST.build_release_manifest(
                    source, root / "out-linked", VERSION, COMMIT, copy_packages=True
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            write_packages(source)
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "total byte"):
                MANIFEST.build_release_manifest(
                    source,
                    root / "out-large",
                    VERSION,
                    COMMIT,
                    copy_packages=True,
                    max_total_bytes=1,
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
