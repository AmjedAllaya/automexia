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
        # Keep one canonical snapshot of each workflow per test; individual
        # mutations use deep copies so a failure cannot contaminate the next case.
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

    def test_ci_cannot_restore_hosted_non_free_runner(self) -> None:
        for runner in ("windows-2025", "macos-26"):
            with self.subTest(runner=runner):
                altered = copy.deepcopy(self.ci)
                altered["jobs"]["quality"]["runs-on"] = runner
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError, "GitHub-Free Ubuntu runner"
                ):
                    PLATFORM.validate_ci(altered)

    def test_ci_cannot_drop_its_read_only_default_or_release_candidate_gate(self) -> None:
        altered = copy.deepcopy(self.ci)
        altered["permissions"] = {}
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "CI must default to contents: read only"
        ):
            PLATFORM.validate_ci(altered)

        altered = copy.deepcopy(self.ci)
        altered["jobs"]["release-candidate"]["if"] = "true"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "stable release pull requests"
        ):
            PLATFORM.validate_ci(altered)

    def test_stable_release_gates_cannot_capture_linux_early_access_prs(self) -> None:
        for job_name in ("release-candidate", "release-candidate-coverage"):
            with self.subTest(job=job_name):
                altered = copy.deepcopy(self.ci)
                altered["jobs"][job_name]["if"] = str(
                    altered["jobs"][job_name]["if"]
                ).replace(
                    "!startsWith(github.head_ref, 'release/linux/') && ",
                    "",
                )
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError,
                    "stable release pull requests",
                ):
                    PLATFORM.validate_ci(altered)

    def test_release_coverage_is_bound_to_the_windows_baseline_and_exact_commits(self) -> None:
        coverage = self.ci["jobs"]["release-candidate-coverage"]
        self.assertEqual(coverage["runs-on"], "windows-2025")
        self.assertEqual(
            set(coverage["needs"]), {"quality", "release-candidate"}
        )
        self.assertEqual(
            coverage["env"],
            {
                "BASE_SHA": "${{ github.event.pull_request.base.sha }}",
                "HEAD_SHA": "${{ github.event.pull_request.head.sha }}",
                "COVERAGE_PLATFORM": "windows-x86_64-msvc",
                "LCOV_FILE": "target/coverage/lcov.info",
                "COVERAGE_SUMMARY": "target/coverage/summary.json",
            },
        )
        self.assertNotIn(
            "check_coverage.py",
            PLATFORM.commands(self.ci["jobs"]["release-candidate"]),
        )

    def test_release_coverage_cannot_drift_to_an_unmatched_runner_or_evidence(self) -> None:
        cases = (
            ("runs-on", "ubuntu-24.04", "Windows coverage runner"),
            ("timeout-minutes", 0, "120-minute timeout"),
        )
        for key, value, expected in cases:
            with self.subTest(key=key):
                altered = copy.deepcopy(self.ci)
                altered["jobs"]["release-candidate-coverage"][key] = value
                with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, expected):
                    PLATFORM.validate_ci(altered)

        for variable in (
            "BASE_SHA",
            "HEAD_SHA",
            "COVERAGE_PLATFORM",
            "LCOV_FILE",
            "COVERAGE_SUMMARY",
        ):
            with self.subTest(variable=variable):
                altered = copy.deepcopy(self.ci)
                del altered["jobs"]["release-candidate-coverage"]["env"][variable]
                with self.assertRaisesRegex(
                    PLATFORM.PlatformCoverageError, "coverage evidence environment"
                ):
                    PLATFORM.validate_ci(altered)

    def test_hosted_policy_must_execute_the_complete_python_contract_suite(self) -> None:
        altered = copy.deepcopy(self.ci)
        policy = altered["jobs"]["policy"]
        step = PLATFORM.step_for_command(policy, "unittest discover")
        self.assertIsNotNone(step)
        step["run"] = "echo skipped"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "complete Python CI contract suite"
        ):
            PLATFORM.validate_ci(altered)

    def test_hosted_policy_cannot_silently_disable_shellcheck(self) -> None:
        altered = copy.deepcopy(self.ci)
        policy = altered["jobs"]["policy"]
        step = PLATFORM.step_for_command(policy, "-color -shellcheck")
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace(
            '-shellcheck "$RUNNER_TEMP/shellcheck"',
            "-shellcheck=",
        )
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "shellcheck"
        ):
            PLATFORM.validate_ci(altered)

    def test_complete_ci_mutation_discovery_cannot_be_narrowed(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(
            altered["jobs"]["policy"], "unittest discover"
        )
        self.assertIsNotNone(step)
        step["run"] = str(step["run"]).replace("test_*.py", "test_coverage.py")
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "complete Python CI contract suite"
        ):
            PLATFORM.validate_ci(altered)

    def test_ci_cannot_drop_shell_contract_smoke(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(
            altered["jobs"]["quality"], "test_shell_sources.sh"
        )
        self.assertIsNotNone(step)
        step["run"] = "echo skipped"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "test_shell_sources"):
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

    def test_ci_quality_cannot_drop_all_feature_clippy(self) -> None:
        altered = copy.deepcopy(self.ci)
        step = PLATFORM.step_for_command(altered["jobs"]["quality"], "cargo clippy")
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
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "read-only contents"):
            PLATFORM.validate_release(altered)

    def test_release_cannot_drop_review_read_or_publication_write_boundary(self) -> None:
        altered = copy.deepcopy(self.release)
        altered["permissions"] = {"contents": "read"}
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "pull-request permissions"
        ):
            PLATFORM.validate_release(altered)

        altered = copy.deepcopy(self.release)
        altered["jobs"]["publish-release"]["permissions"] = {"contents": "read"}
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "immutable publication"
        ):
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
        # Dependency wiring and the success predicate are separate fail-closed
        # controls, so remove each one independently for every controlled gate.
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
        # PyYAML 1.1 can materialize an unquoted workflow `on` key as True.
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

    def test_s2_activation_must_remain_manual_free_and_exact_commit_bound(self) -> None:
        altered = copy.deepcopy(self.s2_assurance)
        trigger_key = True if True in altered else "on"
        altered[trigger_key]["pull_request"] = {}
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "manual dispatch"):
            PLATFORM.validate_s2_assurance(altered)

        altered = copy.deepcopy(self.s2_assurance)
        altered["jobs"]["validate-active-baseline"]["environment"] = "stable-release"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "unavailable private protected environment"
        ):
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

        altered = copy.deepcopy(self.s2_assurance)
        altered["jobs"]["validate-active-baseline"]["env"] = {
            "UNEXPECTED_SECRET": "secrets.example"
        }
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "must not consume repository secrets"
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

    def test_f5_assurance_cannot_drop_free_exact_artifact_binding(self) -> None:
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
        altered["jobs"]["validate"]["environment"] = "f5-openssh-release"
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "unavailable private protected environment"
        ):
            PLATFORM.validate_f5_openssh_assurance(altered)

        altered = copy.deepcopy(self.f5_openssh_assurance)
        altered["jobs"]["validate"]["permissions"] = {"contents": "write"}
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "must not elevate"
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
        altered["jobs"]["preflight"].setdefault("env", {})["APPLE_CERTIFICATE"] = (
            "secrets.apple_certificate"
        )
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "raw signing secrets"):
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

    def test_reproducibility_cannot_replace_the_canonical_cold_build_contract(self) -> None:
        altered = copy.deepcopy(self.release)
        step = PLATFORM.step_for_command(
            altered["jobs"]["reproducibility-linux"], "check_reproducible_build.sh"
        )
        self.assertIsNotNone(step)
        step["run"] = "echo skipped"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "cold native"):
            PLATFORM.validate_release(altered)

    def test_windows_executable_signing_cannot_escape_isolated_input(self) -> None:
        altered = copy.deepcopy(self.release)
        signing = next(
            step
            for step in PLATFORM.steps(altered["jobs"]["sign-windows-runtime"])
            if "artifact-signing-action@" in str(step.get("uses", ""))
            and step.get("with", {}).get("files-folder-filter") == "exe,ps1,ps1xml"
        )
        signing["with"]["files-folder"] = "staged"
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "isolated signing-input"):
            PLATFORM.validate_release(altered)

    def test_windows_release_cannot_drop_script_signing(self) -> None:
        altered = copy.deepcopy(self.release)
        signing = PLATFORM.step_for_command(
            altered["jobs"]["sign-windows-runtime"], "Set-AuthenticodeSignature"
        )
        self.assertIsNotNone(signing)
        signing["run"] = str(signing["run"]).replace(
            "Set-AuthenticodeSignature", "Set-UntrustedAuthenticodeSignature"
        )
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "PowerShell asset"
        ):
            PLATFORM.validate_release(altered)

    def test_publication_cannot_overwrite_or_skip_immutability(self) -> None:
        altered = copy.deepcopy(self.release)
        publication = PLATFORM.step_for_command(
            altered["jobs"]["publish-release"], "gh release upload"
        )
        self.assertIsNotNone(publication)
        publication["run"] = str(publication["run"]).replace(
            "gh release upload", "gh release upload --clobber", 1
        )
        with self.assertRaisesRegex(
            PLATFORM.PlatformCoverageError, "immutable publication"
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
        # Mutate one independent scanner invariant at a time so a broad textual
        # match cannot hide a missing bound, allowlist, digest, or identity check.
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
            if "anchore/sbom-action@" not in str(step.get("uses", ""))
        ]
        with self.assertRaisesRegex(PLATFORM.PlatformCoverageError, "SBOM generation"):
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
