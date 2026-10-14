#!/usr/bin/env python3
"""Regression tests for the bounded stable-release artifact manifest."""

from __future__ import annotations

import importlib.util
import hashlib
import json
import os
from pathlib import Path
import tempfile
import subprocess
import sys
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


def write_application_sboms(root: Path, version: str) -> None:
    components = [("automexia-terminal", version), *[(f"dependency-{n}", "1.0.0") for n in range(9)]]
    spdx = {
        "spdxVersion": "SPDX-2.3", "SPDXID": "SPDXRef-DOCUMENT", "dataLicense": "CC0-1.0",
        "documentNamespace": "https://example.invalid/sbom/fixture",
        "creationInfo": {"created": "2026-10-11T00:00:00Z", "creators": ["Tool: fixture"]},
        "packages": [{"name": name, "versionInfo": value, "SPDXID": f"SPDXRef-Package-{index}",
                      "externalRefs": [{"referenceType": "purl", "referenceLocator": f"pkg:cargo/{name}@{value}"}]}
                     for index, (name, value) in enumerate(components)],
    }
    cdx = {
        "bomFormat": "CycloneDX", "specVersion": "1.6", "serialNumber": "urn:uuid:12345678-1234-1234-1234-123456789abc",
        "version": 1, "metadata": {},
        "components": [{"type": "library", "name": name, "version": value, "bom-ref": f"component-{index}",
                        "purl": f"pkg:cargo/{name}@{value}"} for index, (name, value) in enumerate(components)],
    }
    (root / "automexia-terminal.spdx.json").write_text(json.dumps(spdx))
    (root / "automexia-terminal.cdx.json").write_text(json.dumps(cdx))


class ReleaseManifestTests(unittest.TestCase):
    def test_public_names_explain_platform_architecture_and_package(self) -> None:
        expected = (
            f"automexia-{VERSION}-windows-x64-installer.msi",
            f"automexia-{VERSION}-windows-x64-portable.zip",
            f"automexia-{VERSION}-windows-arm64-installer.msi",
            f"automexia-{VERSION}-windows-arm64-portable.zip",
            f"automexia-{VERSION}-macos-apple-silicon-and-intel.dmg",
            f"automexia-{VERSION}-linux-debian-ubuntu-x64.deb",
            f"automexia-{VERSION}-linux-fedora-opensuse-x64.rpm",
            f"automexia-{VERSION}-linux-x64-portable.tar.gz",
            f"automexia-{VERSION}-linux-debian-ubuntu-arm64.deb",
            f"automexia-{VERSION}-linux-fedora-opensuse-arm64.rpm",
            f"automexia-{VERSION}-linux-arm64-portable.tar.gz",
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, output = root / "downloads", root / "release"
            source.mkdir()
            write_packages(source)
            first = MANIFEST.build_release_manifest(source, output, VERSION, COMMIT, copy_packages=True)
            self.assertEqual({entry["file"] for entry in first["artifacts"]}, set(expected))
            for index, name in enumerate(expected):
                payload = f"fixture-{index}\n".encode()
                self.assertEqual((output / name).read_bytes(), payload)
                entry = next(entry for entry in first["artifacts"] if entry["file"] == name)
                self.assertEqual((entry["sha256"], entry["size"]), (hashlib.sha256(payload).hexdigest(), len(payload)))
            self.assertEqual(first, MANIFEST.build_release_manifest(source, output, VERSION, COMMIT, copy_packages=True))
            (output / expected[0]).write_bytes(b"conflicting package")
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "conflicting destination"):
                MANIFEST.build_release_manifest(source, output, VERSION, COMMIT, copy_packages=True)

    def test_first_alpha_retains_exact_public_tag_and_complete_package_set(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "downloads"
            source.mkdir()
            for name in package_names():
                (source / name.replace(VERSION, "0.0.1-alpha")).write_bytes(b"alpha fixture\n")
            manifest = MANIFEST.build_release_manifest(
                source, root / "release", "0.0.1-alpha", COMMIT, copy_packages=True
            )
            self.assertEqual(manifest["tag"], "0.0.1_alpha")
            self.assertEqual(manifest["version"], "0.0.1-alpha")
            self.assertEqual(len(manifest["artifacts"]), 11)

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
            write_application_sboms(output, VERSION)
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "ConPTY"):
                MANIFEST.build_release_manifest(output, output, VERSION, COMMIT, include_sbom=True)
            import release_trust
            release_trust.add_vendor_sbom(output, release_trust.load_policy())
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
            # The old build name and new public name still occupy one slot.
            (source / f"automexia-{VERSION}-windows-x64-installer.msi").write_bytes(b"duplicate")
            with self.assertRaisesRegex(MANIFEST.ReleaseManifestError, "exactly one"):
                MANIFEST.build_release_manifest(source, root / "release", VERSION, COMMIT, copy_packages=True)

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

    def test_public_alpha_sboms_project_locations_and_reject_private_data(self) -> None:
        self._check_public_alpha_sboms("0.0.1-alpha")

    def test_current_alpha_sboms_project_locations_and_reject_private_data(self) -> None:
        self._check_public_alpha_sboms("0.0.2-alpha")
        self._check_public_alpha_sboms("0.0.3-alpha")
        self._check_public_alpha_sboms("0.0.4-alpha")

    def _check_public_alpha_sboms(self, version: str) -> None:
        import hashlib
        with tempfile.TemporaryDirectory() as temporary:
            # Scanner locations use the canonical root, not a Windows TEMP alias.
            root = Path(temporary).resolve()
            output, raw, scan = (root / name for name in ("release", "raw", "scan"))
            for directory in (output, raw, scan):
                directory.mkdir()
            for name in MANIFEST.package_names(version).values():
                (output / name).write_bytes(b"package fixture")
            write_application_sboms(raw, version)
            lock = scan / "Cargo.lock"
            lock.write_bytes(b"lockfile fixture")
            checksum = hashlib.sha256(lock.read_bytes()).hexdigest()
            spdx_path = raw / "automexia-terminal.spdx.json"
            cdx_path = raw / "automexia-terminal.cdx.json"
            spdx, cdx = (json.loads(path.read_text()) for path in (spdx_path, cdx_path))
            spdx["files"] = [{"SPDXID": "SPDXRef-File-lock", "fileName": lock.as_posix(),
                               "checksums": [{"algorithm": "SHA256", "checksumValue": checksum}]}]
            cdx["components"].append({"type": "file", "name": lock.as_posix(), "bom-ref": "file-lock",
                                      "hashes": [{"alg": "SHA-256", "content": checksum}]})
            spdx_path.write_text(json.dumps(spdx))
            cdx_path.write_text(json.dumps(cdx))
            command = [sys.executable, str(MODULE_PATH), "--prepare-sbom", "--source", str(raw),
                       "--output", str(output), "--scan-root", str(scan), "--version", version, "--commit", COMMIT]
            result = subprocess.run(command, capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)
            final = MANIFEST.build_release_manifest(output, output, version, COMMIT, include_sbom=True)
            self.assertEqual(len(final["artifacts"]), 13)
            saved = (output / spdx_path.name).read_bytes()
            self.assertNotIn(str(root).encode(), saved)
            self.assertEqual(json.loads(saved)["files"][0]["fileName"], "Cargo.lock")
            spdx = json.loads(spdx_path.read_text())
            spdx["packages"][0]["comment"] = "private path /home/private-user/credentials"
            spdx_path.write_text(json.dumps(spdx))
            result = subprocess.run(command, capture_output=True, text=True, timeout=15)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((output / spdx_path.name).read_bytes(), saved)

    def test_vendor_only_sboms_cannot_impersonate_application_inventory(self) -> None:
        import release_trust
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in package_names():
                (root / name).write_bytes(b"package fixture")
            paths = [root / "automexia-terminal.spdx.json", root / "automexia-terminal.cdx.json"]
            paths[0].write_text('{"spdxVersion":"SPDX-2.3","packages":[]}')
            paths[1].write_text('{"bomFormat":"CycloneDX","components":[]}')
            release_trust.add_vendor_sbom(root, release_trust.load_policy())
            with self.assertRaises(MANIFEST.ReleaseManifestError):
                MANIFEST._validate_sboms(paths, VERSION)

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
