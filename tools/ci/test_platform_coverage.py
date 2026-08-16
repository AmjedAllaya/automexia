#!/usr/bin/env python3
"""Regression tests for the cross-platform workflow coverage contract."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).with_name("check_platform_coverage.py")
SPEC = importlib.util.spec_from_file_location("automexia_platform_coverage", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_platform_coverage.py")
PLATFORM = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLATFORM)


class PlatformCoverageTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ci = PLATFORM.load_workflow("ci.yml")
        self.nightly = PLATFORM.load_workflow("nightly.yml")
        self.release = PLATFORM.load_workflow("release.yml")

    def test_current_workflows_satisfy_the_contract(self) -> None:
        PLATFORM.validate_ci(self.ci)
        PLATFORM.validate_nightly(self.nightly)
        PLATFORM.validate_release(self.release)
        PLATFORM.validate_macos_runtime_contract(
            PLATFORM.MACOS_BUILD_SCRIPT.read_text(encoding="utf-8")
        )
        PLATFORM.validate_windows_release_trust_contract(
            PLATFORM.WINDOWS_RELEASE_TRUST_SCRIPT.read_text(encoding="utf-8")
        )

    def test_macos_frontend_cannot_restore_a_hard_framework_link(self) -> None:
        source = PLATFORM.MACOS_BUILD_SCRIPT.read_text(encoding="utf-8")
        altered = source.replace("-weak_framework", "-framework")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "weak-link CoreGraphics"
        ):
            PLATFORM.validate_macos_runtime_contract(altered)

    def test_missing_native_macos_is_rejected(self) -> None:
        altered = copy.deepcopy(self.ci)
        altered["jobs"]["native"]["strategy"]["matrix"]["os"].remove("macos-latest")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "Ubuntu, Windows, and macOS"):
            PLATFORM.validate_ci(altered)

    def test_missing_wayland_only_variant_is_rejected(self) -> None:
        altered = copy.deepcopy(self.ci)
        matrix = altered["jobs"]["linux-features"]["strategy"]["matrix"]["include"]
        altered["jobs"]["linux-features"]["strategy"]["matrix"]["include"] = [
            item for item in matrix if item["features"] != "wayland"
        ]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "Wayland-only"):
            PLATFORM.validate_ci(altered)

    def test_windows_shell_contract_cannot_move_to_an_unrelated_host(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(altered["jobs"]["native"], "test_powershell.ps1")
        self.assertIsNotNone(step)
        step["if"] = "runner.os == 'Linux'"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "runner.os == 'Windows'"):
            PLATFORM.validate_ci(altered)

    def test_librio_c_abi_gate_cannot_leave_native_linux(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(
            altered["jobs"]["native"], "test_librio_c_api.sh"
        )
        self.assertIsNotNone(step)
        step["if"] = "runner.os == 'macOS'"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "runner.os == 'Linux'"):
            PLATFORM.validate_ci(altered)

    def test_missing_release_architecture_is_rejected(self) -> None:
        altered = copy.deepcopy(self.release)
        matrix = altered["jobs"]["build"]["strategy"]["matrix"]["include"]
        altered["jobs"]["build"]["strategy"]["matrix"]["include"] = [
            item for item in matrix if item["target"] != "aarch64-apple-darwin"
        ]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "both architectures"):
            PLATFORM.validate_release(altered)

    def test_native_gui_depth_cannot_be_presented_as_a_hosted_check(self) -> None:
        altered = copy.deepcopy(self.nightly)
        altered["jobs"]["native-gui-resize-stress"]["runs-on"] = ["windows-latest"]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "controlled self-hosted"):
            PLATFORM.validate_nightly(altered)

    def test_native_clippy_cannot_drop_all_feature_coverage(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(altered["jobs"]["native"], "cargo clippy")
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace(" --all-features", "")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "all-features"):
            PLATFORM.validate_ci(altered)

    def test_macos_release_cannot_drop_notarization(self) -> None:
        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(altered["jobs"]["package-macos"], "notarytool submit")
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace("notarytool submit", "notarytool omitted")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "notarytool submit"):
            PLATFORM.validate_release(altered)

    def test_release_cannot_restore_global_write_permissions(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["permissions"] = {"contents": "write", "id-token": "write"}
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "contents: read"):
            PLATFORM.validate_release(altered)

    def test_preflight_cannot_receive_raw_signing_secret(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["jobs"]["preflight"]["env"]["APPLE_CERTIFICATE"] = (
            "${{ secrets.APPLE_CERTIFICATE }}"
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "presence flag"):
            PLATFORM.validate_release(altered)

    def test_publish_cannot_download_unsigned_build_intermediates(self) -> None:
        altered = copy.deepcopy(self.release)
        publish = altered["jobs"]["publish"]
        step = next(
            step
            for step in PLATFORM.steps(publish)
            if step.get("with", {}).get("pattern") == "packages-*"
        )
        del step["with"]["pattern"]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "unsigned build intermediates"):
            PLATFORM.validate_release(altered)

    def test_publish_cannot_bypass_reproducibility(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["jobs"]["publish"]["needs"].remove("reproducibility-linux")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "reproducibility"):
            PLATFORM.validate_release(altered)

    def test_windows_executable_signing_cannot_escape_isolated_input(self) -> None:
        altered = copy.deepcopy(self.release)
        signing = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["package-windows"])
            if "artifact-signing-action@" in str(step.get("uses", ""))
            and step.get("with", {}).get("files-folder-filter") == "exe"
        )
        signing["with"]["files-folder"] = "staged"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "isolated flat"):
            PLATFORM.validate_release(altered)

    def test_windows_release_cannot_drop_script_signing(self) -> None:
        altered = copy.deepcopy(self.release)
        windows = altered["jobs"]["package-windows"]
        windows["steps"] = [
            step
            for step in windows["steps"]
            if step.get("name") != "Sign PowerShell assets with Azure Artifact Signing"
        ]
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "EXE, scripts, and MSI|PowerShell asset"
        ):
            PLATFORM.validate_release(altered)

    def test_publication_cannot_overwrite_or_skip_immutability(self) -> None:
        altered = copy.deepcopy(self.release)
        publish = altered["jobs"]["publish"]
        for step in publish["steps"]:
            if step.get("name") == "Require immutable GitHub releases":
                step["run"] = "echo skipped"
            if step.get("name") == "Publish protected-tag assets":
                step["run"] = 'gh release upload "$GITHUB_REF_NAME" --clobber'
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "immutable-releases|overwrite"
        ):
            PLATFORM.validate_release(altered)

    def test_controlled_smoke_cannot_restore_unsigned_build_artifacts(self) -> None:
        altered = copy.deepcopy(self.release)
        hardware = altered["jobs"]["hardware-smoke"]
        download = next(
            step
            for step in PLATFORM.steps(hardware)
            if step.get("with", {}).get("name") == "packages-windows-x86_64"
        )
        download["with"]["name"] = "windows-x86_64"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "final packages"):
            PLATFORM.validate_release(altered)

    def test_windows_release_cannot_drop_defender_gate(self) -> None:
        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(
            altered["jobs"]["hardware-smoke"], "test_release_trust_windows.ps1"
        )
        self.assertIsNotNone(step)
        step["run"] = "echo skipped"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "scan and launch"):
            PLATFORM.validate_release(altered)

    def test_windows_release_scanner_cannot_drop_bounds_or_add_exclusions(self) -> None:
        source = PLATFORM.WINDOWS_RELEASE_TRUST_SCRIPT.read_text(encoding="utf-8")
        for original, replacement, message in (
            ("MaximumArchiveEntries", "UnboundedEntries", "MaximumArchiveEntries"),
            ("$expectedPackageNames", "$unreviewedPackages", "expectedPackageNames"),
            ("Get-FileHash", "Get-UntrustedHash", "Get-FileHash"),
            ("$expectedFiles", "$unreviewedFiles", "expectedFiles"),
            ("VersionInfo.ProductVersion", "UnverifiedVersion", "ProductVersion"),
        ):
            with self.subTest(contract=message):
                altered = source.replace(original, replacement)
                with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, message):
                    PLATFORM.validate_windows_release_trust_contract(altered)

        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "must not weaken"):
            PLATFORM.validate_windows_release_trust_contract(
                source + "\nAdd-MpPreference -ExclusionPath C:\\\n"
            )

    def test_release_cannot_drop_sbom_attestation(self) -> None:
        altered = copy.deepcopy(self.release)
        publish = altered["jobs"]["publish"]
        publish["steps"] = [
            step
            for step in publish["steps"]
            if "sbom-path" not in step.get("with", {})
        ]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "SBOM attestations"):
            PLATFORM.validate_release(altered)

    def test_macos_release_cannot_use_deep_signing(self) -> None:
        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(
            altered["jobs"]["package-macos"], "codesign --force --options runtime"
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace(
            "--timestamp --sign", "--timestamp --deep --sign", 1
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "inside-out"):
            PLATFORM.validate_release(altered)

    def test_linux_release_cannot_drop_install_smoke(self) -> None:
        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(altered["jobs"]["package-linux"], "test_linux_package.sh")
        self.assertIsNotNone(step)
        step["run"] = "echo skipped"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "test_linux_package.sh"):
            PLATFORM.validate_release(altered)

    def test_nightly_windows_cannot_drop_arm64(self) -> None:
        altered = copy.deepcopy(self.nightly)
        targets = altered["jobs"]["unsigned-windows"]["strategy"]["matrix"]["target"]
        targets.remove("aarch64-pc-windows-msvc")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "x86_64 and ARM64"):
            PLATFORM.validate_nightly(altered)

    def test_nightly_benchmarks_must_remain_bounded(self) -> None:
        altered = copy.deepcopy(self.nightly)
        del altered["jobs"]["benchmark-build"]["timeout-minutes"]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "hard timeout"):
            PLATFORM.validate_nightly(altered)

    def test_limitations_remain_explicit_and_nonempty(self) -> None:
        limitations = "\n".join(PLATFORM.EXTERNAL_LIMITATIONS)
        self.assertIn("BSD", limitations)
        self.assertIn("does not certify every distribution", limitations)
        self.assertIn("controlled native runners", limitations)


if __name__ == "__main__":
    unittest.main()
