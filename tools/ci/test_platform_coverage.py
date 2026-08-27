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
        self.s1_assurance = PLATFORM.load_workflow("s1-assurance.yml")
        self.s2_assurance = PLATFORM.load_workflow("s2-assurance.yml")
        self.f5_openssh_assurance = PLATFORM.load_workflow("f5-openssh-assurance.yml")

    def test_current_workflows_satisfy_the_contract(self) -> None:
        PLATFORM.validate_ci(self.ci)
        PLATFORM.validate_nightly(self.nightly)
        PLATFORM.validate_release(self.release)
        PLATFORM.validate_s1_assurance(self.s1_assurance)
        PLATFORM.validate_s2_assurance(self.s2_assurance)
        PLATFORM.validate_f5_openssh_assurance(self.f5_openssh_assurance)
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

    def test_release_cannot_bypass_complete_s1_assurance(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["jobs"]["preflight"]["needs"].remove("s1-assurance")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "S1 assurance"):
            PLATFORM.validate_release(altered)

    def test_release_preflight_cannot_bypass_any_controlled_assurance_gate(self) -> None:
        required = {
            "native-gui-resilience",
            "native-wsl-resilience",
            "performance-assurance",
            "s1-assurance",
        }
        for gate in sorted(required):
            with self.subTest(gate=gate, mutation="dependency"):
                altered = copy.deepcopy(self.release)
                altered["jobs"]["preflight"]["needs"].remove(gate)
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError, "controlled assurance gates"
                ):
                    PLATFORM.validate_release(altered)

            with self.subTest(gate=gate, mutation="success-result"):
                altered = copy.deepcopy(self.release)
                condition = altered["jobs"]["preflight"]["if"]
                altered["jobs"]["preflight"]["if"] = condition.replace(
                    f"needs.{gate}.result == 'success'", "true"
                )
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError, "fail closed.*controlled assurance"
                ):
                    PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        altered["jobs"]["preflight"]["if"] += " || true"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "fail closed.*controlled assurance"
        ):
            PLATFORM.validate_release(altered)

    def test_release_s2_ratchet_cannot_be_informational_or_unbound(self) -> None:
        for fragment in ("--require-active", "--expected-commit $env:GITHUB_SHA"):
            with self.subTest(fragment=fragment):
                altered = copy.deepcopy(self.release)
                step = PLATFORM.step_for_command(
                    altered["jobs"]["performance-assurance"],
                    "performance_assurance.py evaluate",
                )
                self.assertIsNotNone(step)
                step["run"] = str(step["run"]).replace(fragment, "")
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError, "S2 performance assurance"
                ):
                    PLATFORM.validate_release(altered)

    def test_release_s1_and_s2_jobs_retain_controlled_execution_and_artifacts(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["jobs"]["s1-assurance"]["runs-on"].remove("automexia-assurance")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "controlled assurance runner"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        altered["jobs"]["s1-assurance"]["if"] = "true"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "S1 assurance activation"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["s1-assurance"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["name"] = "redirected-summary"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "S1 assurance summary"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        altered["jobs"]["performance-assurance"]["if"] = "true"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "S2 performance activation"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(
            altered["jobs"]["performance-assurance"], "collect-criterion"
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace("--require-classified", "")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "S2 performance assurance"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["performance-assurance"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["retention-days"] = 1
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "S2 performance evidence"
        ):
            PLATFORM.validate_release(altered)

    def test_s1_assurance_cannot_accept_incomplete_or_unbound_evidence(self) -> None:
        altered = copy.deepcopy(self.s1_assurance)
        step = PLATFORM.step_for_command(
            altered["jobs"]["validate"], "s1_assurance.py validate"
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace("--require-complete", "")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "complete matrix"
        ):
            PLATFORM.validate_s1_assurance(altered)

    def test_s1_assurance_workflow_remains_manual_controlled_and_bounded(self) -> None:
        altered = copy.deepcopy(self.s1_assurance)
        trigger_key = True if True in altered else "on"
        altered[trigger_key]["pull_request"] = {}
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "manual dispatch"):
            PLATFORM.validate_s1_assurance(altered)

        altered = copy.deepcopy(self.s1_assurance)
        altered["jobs"]["validate"]["runs-on"].remove("automexia-assurance")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "controlled assurance runner"
        ):
            PLATFORM.validate_s1_assurance(altered)

        altered = copy.deepcopy(self.s1_assurance)
        altered["jobs"]["validate"]["timeout-minutes"] = 31
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "bounded timeout"):
            PLATFORM.validate_s1_assurance(altered)

        altered = copy.deepcopy(self.s1_assurance)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["validate"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["path"] = "target/s1-assurance/*.json"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "exact summary"):
            PLATFORM.validate_s1_assurance(altered)

    def test_s2_activation_must_remain_manual_protected_and_exact_commit_bound(self) -> None:
        altered = copy.deepcopy(self.s2_assurance)
        trigger_key = True if True in altered else "on"
        altered[trigger_key]["pull_request"] = {}
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "manual dispatch"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        altered["jobs"]["validate-active-baseline"].pop("environment")
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "protected environment"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        step = PLATFORM.step_for_command(
            altered["jobs"]["validate-active-baseline"],
            "validate-baseline",
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace(
            '--expected-source-commit "$GITHUB_SHA"', ""
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "exact source commit"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        altered["concurrency"]["cancel-in-progress"] = True
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "serialized activation"
        ):
            PLATFORM.validate_s2_assurance(altered)

    def test_s2_activation_cannot_drop_mutations_or_bounded_artifact_retention(self) -> None:
        altered = copy.deepcopy(self.s2_assurance)
        step = PLATFORM.step_for_command(
            altered["jobs"]["validate-active-baseline"],
            "test_performance_assurance.py",
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace(
            "python tools/ci/test_performance_assurance.py", "true"
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "mutation suite"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["validate-active-baseline"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["retention-days"] = 1
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "90-day"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["validate-active-baseline"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["name"] = "redirected-summary"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "90-day"):
            PLATFORM.validate_s2_assurance(altered)

    def test_f5_assurance_cannot_drop_protected_exact_artifact_binding(self) -> None:
        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["runs-on"]["group"] = "shared"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "restricted self-hosted runner group"
        ):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        trigger_key = True if True in altered else "on"
        altered[trigger_key]["pull_request"] = {}
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "manual dispatch only"
        ):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"].pop("environment")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "protected environment"
        ):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["env"].pop(
            "AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY"
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "artifact paths"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["if"] += " || true"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "operator-enabled"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["timeout-minutes"] = 400
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "timeout"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["runs-on"]["labels"] += "-redirected"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "runner group"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["continue-on-error"] = True
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "fail closed"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["concurrency"]["cancel-in-progress"] = True
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "concurrency"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        upload = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["validate"])
            if "upload-artifact@" in str(step.get("uses", ""))
        )
        upload["with"]["retention-days"] = 1
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "90-day"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["env"][
            "AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE"
        ] = "${{ vars.AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE }}"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "artifact paths"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        unix = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["validate"])
            if step.get("shell") == "bash"
        )
        unix["run"] = unix["run"].replace(
            "python3 tools/ci/test_session_launch_d0.py", "true"
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "D0 mutation"):
            PLATFORM.validate_f5_openssh_assurance(altered)

        for key, value in (
            ("name", "redirected-summary"),
            ("path", "target/private-manifest.json"),
        ):
            altered = copy.deepcopy(self.f5_openssh_assurance)
            upload = next(
                step
                for step in PLATFORM.steps(altered["jobs"]["validate"])
                if "upload-artifact@" in str(step.get("uses", ""))
            )
            upload["with"][key] = value
            with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "90-day"):
                PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["steps"].append(
            {
                "uses": "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
                "with": {
                    "name": "private-evidence",
                    "path": "target/private-manifest.json",
                    "if-no-files-found": "error",
                    "retention-days": 1,
                },
            }
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "90-day"):
            PLATFORM.validate_f5_openssh_assurance(altered)

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
