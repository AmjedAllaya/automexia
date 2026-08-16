#!/usr/bin/env python3
"""Mutation and resource-contract tests for final release trust validation."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("release_trust.py")
SPEC = importlib.util.spec_from_file_location("automexia_release_trust", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/release_trust.py")
TRUST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TRUST)


VALID_NAMES = (
    "automexia-terminal-0.4.0-x86_64-pc-windows-msvc.msi",
    "automexia-terminal-0.4.0-aarch64-pc-windows-msvc.msi",
    "automexia-terminal-0.4.0-x86_64-pc-windows-msvc.zip",
    "automexia-terminal-0.4.0-aarch64-pc-windows-msvc.zip",
    "automexia-terminal-0.4.0-universal.dmg",
    "automexia-terminal_0.4.0_amd64.deb",
    "automexia-terminal_0.4.0_arm64.deb",
    "automexia-terminal-0.4.0-1.x86_64.rpm",
    "automexia-terminal-0.4.0-1.aarch64.rpm",
    "automexia-terminal-0.4.0-x86_64-unknown-linux-gnu.tar.gz",
    "automexia-terminal-0.4.0-aarch64-unknown-linux-gnu.tar.gz",
)


class ReleaseTrustTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.packages = self.root / "packages"
        self.packages.mkdir()
        for index, name in enumerate(VALID_NAMES, start=1):
            (self.packages / name).write_bytes(bytes([index]) * 32)
        self.policy = TRUST.load_policy()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def write_checksums(self, final: Path) -> None:
        lines = [
            f"{TRUST.digest(path, 7)}  {path.name}"
            for path in sorted(final.iterdir(), key=lambda value: value.name.casefold())
            if path.name != "SHA256SUMS"
        ]
        (final / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")

    def prepare_final(self, name: str = "final") -> Path:
        final = self.root / name
        final.mkdir()
        for source in self.packages.iterdir():
            (final / source.name).write_bytes(source.read_bytes())
        TRUST.inventory(
            self.packages,
            "0.4.0",
            final / "release-manifest.json",
            final / "release-trust-benchmark.json",
            self.policy,
        )
        (final / "automexia-terminal.spdx.json").write_text(
            json.dumps({"spdxVersion": "SPDX-2.3", "name": "Automexia Terminal"}),
            encoding="utf-8",
        )
        (final / "automexia-terminal.cdx.json").write_text(
            json.dumps({"bomFormat": "CycloneDX", "specVersion": "1.6"}),
            encoding="utf-8",
        )
        windows_bytes = sum(
            path.stat().st_size
            for path in self.packages.iterdir()
            if path.suffix.casefold() in {".msi", ".zip"}
        )
        (final / "release-trust-windows.json").write_text(
            json.dumps(
                {
                    "schema": 1,
                    "scanner": "Microsoft Defender Antivirus",
                    "scanner_version": "1.1.1",
                    "security_intelligence_version": "1.2.3",
                    "security_intelligence_updated_utc": "2026-08-16T10:00:00Z",
                    "artifact_count": 4,
                    "artifact_bytes": windows_bytes,
                    "signature_count": 4,
                    "publisher": "CN=Automexia Test",
                    "scan_milliseconds": 10,
                    "scan_timeout_seconds": 900,
                    "result": "pass",
                }
            ),
            encoding="utf-8",
        )
        self.write_checksums(final)
        return final

    def test_current_policy_and_complete_matrix_pass(self) -> None:
        artifacts = TRUST.package_files(self.packages, "0.4.0", self.policy)
        self.assertEqual(len(artifacts), 11)

    def test_raw_executable_is_rejected(self) -> None:
        (self.packages / "automexia.exe").write_bytes(b"unsigned")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "forbidden raw"):
            TRUST.package_files(self.packages, "0.4.0", self.policy)

    def test_unknown_sidecar_is_rejected(self) -> None:
        (self.packages / "notes.txt").write_text("not allowlisted", encoding="utf-8")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "unallowlisted"):
            TRUST.package_files(self.packages, "0.4.0", self.policy)

    def test_missing_architecture_is_rejected(self) -> None:
        (self.packages / VALID_NAMES[1]).unlink()
        replacement = self.packages / "automexia-terminal-0.4.0-second-x64.msi"
        replacement.write_bytes(b"replacement")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "architecture token"):
            TRUST.package_files(self.packages, "0.4.0", self.policy)

    def test_one_filename_cannot_satisfy_two_architectures(self) -> None:
        (self.packages / VALID_NAMES[1]).unlink()
        replacement = self.packages / (
            "automexia-terminal-0.4.0-x86_64-pc-windows-msvc-"
            "aarch64-pc-windows-msvc.msi"
        )
        replacement.write_bytes(b"replacement")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "exactly one architecture"):
            TRUST.package_files(self.packages, "0.4.0", self.policy)

    def test_oversized_artifact_is_rejected_without_reading_it(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["artifacts"][0]["max_bytes"] = 8
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "outside"):
            TRUST.package_files(self.packages, "0.4.0", policy)

    def test_empty_artifact_is_rejected(self) -> None:
        (self.packages / VALID_NAMES[0]).write_bytes(b"")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "outside"):
            TRUST.package_files(self.packages, "0.4.0", self.policy)

    def test_inventory_is_sorted_streamed_and_atomic(self) -> None:
        manifest = self.root / "metadata" / "release-manifest.json"
        benchmark = self.root / "metadata" / "benchmark.json"
        result = TRUST.inventory(
            self.packages, "0.4.0", manifest, benchmark, self.policy
        )
        parsed = json.loads(manifest.read_text(encoding="utf-8"))
        names = [entry["name"] for entry in parsed["artifacts"]]
        self.assertEqual(names, sorted(names, key=str.casefold))
        self.assertTrue(all(len(entry["sha256"]) == 64 for entry in parsed["artifacts"]))
        self.assertEqual(result["artifact_count"], 11)
        self.assertGreater(result["throughput_mib_per_second"], 0)
        self.assertFalse(list(manifest.parent.glob("*.tmp")))

    def test_final_assets_require_exact_metadata_and_checksums(self) -> None:
        final = self.prepare_final("final-evidence")
        TRUST.verify_final(final, "0.4.0", self.policy)

    def test_checksum_tampering_is_rejected(self) -> None:
        final = self.prepare_final()
        lines = [
            f"{'0' * 64}  {path.name}"
            for path in sorted(final.iterdir(), key=lambda value: value.name.casefold())
            if path.name != "SHA256SUMS"
        ]
        (final / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "checksum mismatch"):
            TRUST.verify_final(final, "0.4.0", self.policy)

    def test_manifest_tampering_is_rejected_even_with_updated_checksum(self) -> None:
        final = self.prepare_final()
        manifest = json.loads((final / "release-manifest.json").read_text(encoding="utf-8"))
        manifest["artifacts"][0]["sha256"] = "0" * 64
        (final / "release-manifest.json").write_text(
            json.dumps(manifest), encoding="utf-8"
        )
        self.write_checksums(final)
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "does not describe"):
            TRUST.verify_final(final, "0.4.0", self.policy)

    def test_invalid_sbom_and_failed_scan_evidence_are_rejected(self) -> None:
        final = self.prepare_final()
        (final / "automexia-terminal.cdx.json").write_text("{}", encoding="utf-8")
        self.write_checksums(final)
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "CycloneDX"):
            TRUST.verify_final(final, "0.4.0", self.policy)

        final = self.prepare_final("final-failed-evidence")
        evidence_path = final / "release-trust-windows.json"
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
        evidence["result"] = "fail"
        evidence_path.write_text(json.dumps(evidence), encoding="utf-8")
        self.write_checksums(final)
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "not passing"):
            TRUST.verify_final(final, "0.4.0", self.policy)

    def test_oversized_metadata_is_rejected_before_json_parsing(self) -> None:
        final = self.prepare_final()
        policy = copy.deepcopy(self.policy)
        for rule in policy["final_metadata"]:
            if rule["name"] == "automexia-terminal.spdx.json":
                rule["max_bytes"] = 8
        with self.assertRaisesRegex(TRUST.ReleaseTrustError, "outside"):
            TRUST.verify_final(final, "0.4.0", policy)

    def test_policy_rejects_count_token_drift(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["artifacts"][0]["count"] = 3
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as output:
            json.dump(policy, output)
            path = Path(output.name)
        try:
            with self.assertRaisesRegex(TRUST.ReleaseTrustError, "one architecture"):
                TRUST.load_policy(path)
        finally:
            path.unlink()

    def test_policy_rejects_unreviewed_fields_and_metadata_formats(self) -> None:
        for mutation, message in (
            (("unexpected", True), "define exactly"),
            (("final-format", "vendor-private"), "unsupported final metadata"),
        ):
            policy = copy.deepcopy(self.policy)
            if mutation[0] == "unexpected":
                policy[mutation[0]] = mutation[1]
            else:
                policy["final_metadata"][0]["format"] = mutation[1]
            path = self.root / f"{mutation[0]}.json"
            path.write_text(json.dumps(policy), encoding="utf-8")
            with self.assertRaisesRegex(TRUST.ReleaseTrustError, message):
                TRUST.load_policy(path)


if __name__ == "__main__":
    unittest.main()
