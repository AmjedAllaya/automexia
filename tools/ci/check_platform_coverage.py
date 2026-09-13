#!/usr/bin/env python3
"""Prevent silent drift in Automexia's cross-platform CI and release matrix."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any

import yaml


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_ROOT = ROOT / ".github" / "workflows"
MACOS_BUILD_SCRIPT = ROOT / "apps" / "automexia-terminal" / "build.rs"
WINDOWS_RELEASE_TRUST_SCRIPT = ROOT / "tools" / "ci" / "test_release_trust_windows.ps1"

# These are intentionally reported as external evidence, not represented as a
# passing hosted check. GitHub does not provide BSD runners or interactive GPU,
# accessibility, signing, and notarization hardware for every supported family.
EXTERNAL_LIMITATIONS = (
    "BSD uses the portable Unix contracts but has no native hosted CI runner or release artifact.",
    "Linux and macOS interactive GPU, PTY, and screen-reader evidence requires controlled native runners.",
    "Linux package smoke covers DEB/RPM contracts on clean CI images; it does not certify every distribution.",
    "Windows ARM64 and macOS alternate architectures are compile/package checked; native hardware smoke remains release evidence.",
)


class PlatformCoverageError(ValueError):
    """Raised when a required platform coverage contract is absent."""


def load_workflow(name: str) -> dict[str, Any]:
    path = WORKFLOW_ROOT / name
    with path.open("r", encoding="utf-8") as source:
        workflow = yaml.safe_load(source)
    if not isinstance(workflow, dict) or not isinstance(workflow.get("jobs"), dict):
        raise PlatformCoverageError(f"{path.relative_to(ROOT)} has no jobs mapping")
    return workflow


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PlatformCoverageError(message)


def job(workflow: dict[str, Any], name: str, workflow_name: str) -> dict[str, Any]:
    value = workflow["jobs"].get(name)
    if not isinstance(value, dict):
        raise PlatformCoverageError(f"{workflow_name} is missing the {name!r} job")
    return value


def steps(value: dict[str, Any]) -> list[dict[str, Any]]:
    result = value.get("steps", [])
    require(isinstance(result, list), "workflow job steps must be a list")
    return [step for step in result if isinstance(step, dict)]


def actions(value: dict[str, Any]) -> list[str]:
    return [str(step.get("uses", "")) for step in steps(value) if step.get("uses")]


def commands(value: dict[str, Any]) -> str:
    return "\n".join(str(step.get("run", "")) for step in steps(value))


def matrix_values(value: dict[str, Any], key: str) -> set[str]:
    matrix = value.get("strategy", {}).get("matrix", {})
    result = matrix.get(key, []) if isinstance(matrix, dict) else []
    return {str(item) for item in result} if isinstance(result, list) else set()


def matrix_pairs(value: dict[str, Any], left: str, right: str) -> set[tuple[str, str]]:
    matrix = value.get("strategy", {}).get("matrix", {})
    include = matrix.get("include", []) if isinstance(matrix, dict) else []
    if not isinstance(include, list):
        return set()
    return {
        (str(item[left]), str(item[right]))
        for item in include
        if isinstance(item, dict) and left in item and right in item
    }


def step_for_command(value: dict[str, Any], fragment: str) -> dict[str, Any] | None:
    return next(
        (step for step in steps(value) if fragment in str(step.get("run", ""))),
        None,
    )


def require_step_condition(
    value: dict[str, Any], command_fragment: str, condition_fragments: tuple[str, ...]
) -> None:
    step = step_for_command(value, command_fragment)
    require(step is not None, f"missing workflow command containing {command_fragment!r}")
    condition = str(step.get("if", ""))
    require(
        all(fragment in condition for fragment in condition_fragments),
        f"{command_fragment!r} must be gated by {condition_fragments}; found {condition!r}",
    )


def require_exact_upload(
    value: dict[str, Any],
    *,
    name: str,
    path: str,
    retention_days: int,
    label: str,
) -> dict[str, Any]:
    uploads = [
        step
        for step in steps(value)
        if "actions/upload-artifact@" in str(step.get("uses", ""))
    ]
    expected = {
        "name": name,
        "path": path,
        "if-no-files-found": "error",
        "retention-days": retention_days,
    }
    require(
        len(uploads) == 1 and uploads[0].get("with") == expected,
        f"{label} must upload only the exact summary identity for {retention_days}-day retention",
    )
    return uploads[0]


def validate_ci(workflow: dict[str, Any]) -> None:
    require(
        workflow.get("permissions") == {"contents": "read"},
        "CI must default to contents: read only",
    )
    policy = job(workflow, "policy", "ci.yml")
    quality = job(workflow, "quality", "ci.yml")
    dependency_security = job(workflow, "dependency-security", "ci.yml")
    release_candidate = job(workflow, "release-candidate", "ci.yml")
    release_coverage = job(workflow, "release-candidate-coverage", "ci.yml")

    for name, value, timeout in (
        ("policy", policy, 30),
        ("quality", quality, 90),
        ("dependency-security", dependency_security, 30),
        ("release-candidate", release_candidate, 30),
    ):
        require(
            value.get("runs-on") == "ubuntu-24.04",
            f"CI {name} must remain on the GitHub-Free Ubuntu runner",
        )
        require(
            value.get("timeout-minutes") == timeout,
            f"CI {name} must retain its {timeout}-minute timeout",
        )
        require(
            value.get("permissions") == {"contents": "read"},
            f"CI {name} must be read-only",
        )

    require(
        release_coverage.get("runs-on") == "windows-2025",
        "CI release-candidate-coverage must use the Windows coverage runner that matches the baseline",
    )
    require(
        release_coverage.get("timeout-minutes") == 120,
        "CI release-candidate-coverage must retain its 120-minute timeout",
    )
    require(
        release_coverage.get("permissions") == {"contents": "read"},
        "CI release-candidate-coverage must be read-only",
    )
    require(
        set(release_coverage.get("needs", [])) == {"quality", "release-candidate"},
        "CI release-candidate-coverage must wait for quality and release-candidate validation",
    )
    expected_coverage_environment = {
        "BASE_SHA": "${{ github.event.pull_request.base.sha }}",
        "HEAD_SHA": "${{ github.event.pull_request.head.sha }}",
        "COVERAGE_PLATFORM": "windows-x86_64-msvc",
        "LCOV_FILE": "target/coverage/lcov.info",
        "COVERAGE_SUMMARY": "target/coverage/summary.json",
    }
    require(
        release_coverage.get("env") == expected_coverage_environment,
        "CI release-candidate-coverage must retain its exact coverage evidence environment",
    )

    policy_commands = commands(policy)
    for fragment in (
        "check_action_pins.py",
        "check_free_plan_contract.py",
        "validate_repository.py",
        '"$RUNNER_TEMP/actionlint" -color -shellcheck "$RUNNER_TEMP/shellcheck"',
        "zizmor",
    ):
        require(fragment in policy_commands, f"CI policy is missing {fragment!r}")
    require(
        "python3 -m unittest discover -s tools/ci -p 'test_*.py'" in policy_commands,
        "CI policy must execute the complete Python CI contract suite",
    )

    quality_commands = commands(quality)
    for fragment in (
        "cargo fmt --all -- --check",
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo nextest run --workspace --all-features --locked --profile ci",
        "cargo test --workspace --all-features --doc --locked",
        "bash tools/ci/test_shell_sources.sh",
    ):
        require(fragment in quality_commands, f"CI quality is missing {fragment!r}")

    dependency_commands = commands(dependency_security)
    for fragment in ("cargo audit --deny warnings", "cargo deny --locked"):
        require(fragment in dependency_commands, f"CI dependency security is missing {fragment!r}")

    release_condition = (
        "${{ github.event_name == 'pull_request' && "
        "startsWith(github.head_ref, 'release/') && "
        "!startsWith(github.head_ref, 'release/linux/') && "
        "github.event.pull_request.head.repo.full_name == github.repository }}"
    )
    require(
        str(release_candidate.get("if", "")) == release_condition,
        "CI release-candidate validation must remain limited to internal stable release pull requests",
    )
    require(
        str(release_coverage.get("if", "")) == release_condition,
        "CI release coverage must remain limited to internal stable release pull requests",
    )
    candidate_commands = commands(release_candidate)
    for fragment in (
        "Release PRs must originate from this repository, never a fork.",
        "Release branch must be exactly release/X.Y.Z.",
    ):
        require(
            fragment in candidate_commands,
            f"CI release-candidate validation is missing {fragment!r}",
        )
    require(
        "check_coverage.py" not in candidate_commands,
        "CI release-candidate validation must not compare a Linux report to the Windows baseline",
    )
    coverage_commands = commands(release_coverage)
    for fragment in (
        "cargo llvm-cov --workspace --locked --lcov",
        "python tools/ci/check_coverage.py",
    ):
        require(
            fragment in coverage_commands,
            f"CI release coverage is missing {fragment!r}",
        )


def validate_nightly(workflow: dict[str, Any]) -> None:
    benchmark_build = job(workflow, "benchmark-build", "nightly.yml")
    require(
        benchmark_build.get("timeout-minutes") == 45,
        "nightly benchmark compilation must have a 45-minute hard timeout",
    )
    controlled_benchmarks = job(workflow, "benchmarks-controlled", "nightly.yml")
    require(
        controlled_benchmarks.get("timeout-minutes") == 180,
        "controlled benchmark execution must have a 180-minute hard timeout",
    )

    gui = job(workflow, "native-gui-resize-stress", "nightly.yml")
    require("AUTOMEXIA_NATIVE_GUI_RUNNER" in str(gui.get("if", "")), "native Windows GUI depth must be explicitly opt-in")
    require("self-hosted" in [str(label).lower() for label in gui.get("runs-on", [])], "native Windows GUI depth must use a controlled self-hosted runner")
    require("--native-windows" in commands(gui), "native Windows GUI depth must execute the native clone/resize suite")

    deep = job(workflow, "native-windows-deep", "nightly.yml")
    deep_commands = commands(deep)
    for fragment in ("appverifier-windows.ps1", "wpr-windows.ps1"):
        require(fragment in deep_commands, f"Windows deep evidence is missing {fragment}")

    wsl = job(workflow, "native-wsl-session-clone", "nightly.yml")
    require("--native-wsl" in commands(wsl), "nightly WSL must exercise native distro/user/directory cloning")

    windows = job(workflow, "unsigned-windows", "nightly.yml")
    require(
        {"x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"}.issubset(
            matrix_values(windows, "target")
        ),
        "nightly Windows artifacts must cover x86_64 and ARM64",
    )
    linux = job(workflow, "unsigned-linux", "nightly.yml")
    require(
        {"x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"}.issubset(
            {target for target, _ in matrix_pairs(linux, "target", "arch")}
        ),
        "nightly Linux artifacts must cover x86_64 and ARM64",
    )
    require("*.deb" in commands(linux) or "*.deb" in str(linux), "nightly Linux artifacts must retain DEB output")
    require("*.rpm" in commands(linux) or "*.rpm" in str(linux), "nightly Linux artifacts must retain RPM output")
    require("*.tar.gz" in commands(linux) or "*.tar.gz" in str(linux), "nightly Linux artifacts must retain portable output")

    macos = job(workflow, "unsigned-macos", "nightly.yml")
    macos_commands = commands(macos)
    for fragment in ("x86_64-apple-darwin", "aarch64-apple-darwin", "lipo -create", "--formats app,dmg"):
        require(fragment in macos_commands, f"nightly universal macOS packaging is missing {fragment!r}")


def validate_release(workflow: dict[str, Any]) -> None:
    require(
        workflow.get("permissions")
        == {"contents": "read", "pull-requests": "read"},
        "release workflow must default to read-only contents and pull-request permissions",
    )
    for job_name, value in workflow["jobs"].items():
        permissions = value.get("permissions", {})
        if isinstance(permissions, dict) and permissions.get("contents") == "write":
            require(
                job_name == "publish-release",
                f"release job {job_name} must not receive contents: write",
            )

    publish_release = job(workflow, "publish-release", "release.yml")
    require(
        publish_release.get("permissions") == {"contents": "write"},
        "only immutable publication may receive contents: write",
    )

    assurance = job(workflow, "s1-assurance", "release.yml")
    assurance_commands = commands(assurance)
    for fragment in (
        "s1_assurance.py check-policy",
        "test_s1_assurance.py",
        "s1_assurance.py validate",
        "--expected-commit $env:GITHUB_SHA",
        "--require-complete",
        "--output target/s1-assurance/summary.json",
    ):
        require(
            fragment in assurance_commands,
            f"release S1 assurance is missing {fragment!r}",
        )
    assurance_labels = {
        str(label).lower() for label in assurance.get("runs-on", [])
    }
    require(
        {"self-hosted", "automexia-assurance"}.issubset(assurance_labels),
        "release S1 assurance must use the controlled assurance runner",
    )
    require(
        assurance.get("if") == "vars.AUTOMEXIA_S1_ASSURANCE_RUNNER == '1'",
        "release S1 assurance activation must remain explicit",
    )
    assurance_timeout = assurance.get("timeout-minutes")
    require(
        isinstance(assurance_timeout, int)
        and not isinstance(assurance_timeout, bool)
        and 1 <= assurance_timeout <= 30,
        "release S1 assurance must retain a bounded timeout",
    )
    assurance_steps = steps(assurance)
    require(
        assurance.get("continue-on-error") is not True
        and all(step.get("continue-on-error") is not True for step in assurance_steps),
        "release S1 assurance must fail closed",
    )
    require(
        "AUTOMEXIA_S1_ASSURANCE_EVIDENCE" in str(assurance.get("env", {})),
        "release S1 assurance must consume the controlled evidence path",
    )
    require_exact_upload(
        assurance,
        name="release-evidence-s1-assurance",
        path="target/s1-assurance/summary.json",
        retention_days=90,
        label="release S1 assurance summary",
    )

    performance = job(workflow, "performance-assurance", "release.yml")
    performance_commands = commands(performance)
    for fragment in (
        "cargo xtask test resize-stress --native-gui",
        "cargo xtask qa --full --bundle",
        "performance_assurance.py collect-native-resource",
        "performance_assurance.py collect-criterion",
        "--require-classified",
        "performance_assurance.py evaluate",
        "--expected-commit $env:GITHUB_SHA",
        "--require-active",
    ):
        require(
            fragment in performance_commands,
            f"release S2 performance assurance is missing {fragment!r}",
        )
    performance_labels = {
        str(label).lower() for label in performance.get("runs-on", [])
    }
    require(
        {"self-hosted", "automexia-benchmark"}.issubset(performance_labels),
        "release S2 performance assurance must use the controlled benchmark runner",
    )
    require(
        performance.get("if")
        == "vars.AUTOMEXIA_WINDOWS_PERFORMANCE_RUNNER == '1'",
        "release S2 performance activation must remain explicit",
    )
    performance_timeout = performance.get("timeout-minutes")
    require(
        isinstance(performance_timeout, int)
        and not isinstance(performance_timeout, bool)
        and 1 <= performance_timeout <= 240,
        "release S2 performance assurance must retain a bounded timeout",
    )
    performance_steps = steps(performance)
    require(
        performance.get("continue-on-error") is not True
        and all(step.get("continue-on-error") is not True for step in performance_steps),
        "release S2 performance assurance must fail closed",
    )
    performance_upload = require_exact_upload(
        performance,
        name="release-evidence-performance-assurance",
        path=(
            "target/performance/*.json\n"
            "target/qa/${{ env.AUTOMEXIA_QA_RUN_LABEL }}.zip\n"
        ),
        retention_days=90,
        label="release S2 performance evidence",
    )
    require(
        performance_upload.get("if") == "always()",
        "release S2 performance evidence must be retained on failure",
    )

    preflight = job(workflow, "preflight", "release.yml")
    preflight_needs = {str(item) for item in preflight.get("needs", [])}
    controlled_assurance_gates = (
        "native-gui-resilience",
        "native-wsl-resilience",
        "performance-assurance",
        "s1-assurance",
    )
    required_preflight_gates = {
        "authorize",
        "signing-readiness",
        *controlled_assurance_gates,
    }
    require(
        required_preflight_gates.issubset(preflight_needs),
        "release preflight controlled assurance gates must include authorization, "
        "signing readiness, native GUI, native WSL, S2, and S1 assurance",
    )
    preflight_condition = str(preflight.get("if", ""))
    expected_preflight_condition = " && ".join(
        (
            "always()",
            "needs.authorize.result == 'success'",
            "needs.signing-readiness.result == 'success'",
            *(
                f"(needs.{gate}.result == 'success' || "
                f"needs.{gate}.result == 'skipped')"
                for gate in controlled_assurance_gates
            ),
        )
    )
    require(
        preflight_condition == expected_preflight_condition,
        "release preflight must fail closed for every controlled assurance gate",
    )
    require(
        "secrets." not in str(preflight.get("env", {})),
        "release preflight must not receive raw signing secrets",
    )

    build = job(workflow, "build", "release.yml")
    require(
        {
            ("windows-2025", "x86_64-pc-windows-msvc"),
            ("windows-11-arm", "aarch64-pc-windows-msvc"),
            ("macos-26-intel", "x86_64-apple-darwin"),
            ("macos-26", "aarch64-apple-darwin"),
            ("ubuntu-22.04", "x86_64-unknown-linux-gnu"),
            ("ubuntu-22.04-arm", "aarch64-unknown-linux-gnu"),
        }.issubset(matrix_pairs(build, "os", "target")),
        "release builds must retain both architectures for Windows, macOS, and Linux",
    )

    signed_runtime = job(workflow, "sign-windows-runtime", "release.yml")
    signed_runtime_needs = {str(item) for item in signed_runtime.get("needs", [])}
    require(
        {"preflight", "build", "prepare-windows-package-inputs"}.issubset(
            signed_runtime_needs
        ),
        "Windows runtime signing must consume the exact preflighted binary and package inputs",
    )
    require(
        signed_runtime.get("permissions")
        == {"contents": "read", "id-token": "write"},
        "Windows runtime signing must have read-only contents and OIDC only",
    )
    signed_runtime_commands = commands(signed_runtime)
    signed_runtime_actions = actions(signed_runtime)
    runtime_signing = [
        step
        for step in steps(signed_runtime)
        if "azure/artifact-signing-action@" in str(step.get("uses", ""))
    ]
    require(
        len(runtime_signing) == 1
        and runtime_signing[0].get("with", {}).get("files-folder") == "signing-input"
        and runtime_signing[0].get("with", {}).get("files-folder-filter")
        == "exe,ps1,ps1xml"
        and runtime_signing[0].get("with", {}).get("files-folder-recurse") is True,
        "Windows executable signing must consume the isolated signing-input directory",
    )
    require(
        "Set-AuthenticodeSignature" in signed_runtime_commands,
        "Windows runtime signing must timestamp-sign every distributed PowerShell asset",
    )
    require(
        any("azure/login@" in value for value in signed_runtime_actions),
        "Windows runtime signing must use OIDC Azure login",
    )

    unsigned_windows = job(workflow, "package-windows-unsigned", "release.yml")
    require(
        {"preflight", "sign-windows-runtime"}.issubset(
            {str(item) for item in unsigned_windows.get("needs", [])}
        )
        and "AUTOMEXIA_PACKAGE_SKIP_BUILD" in commands(unsigned_windows)
        and "secrets." not in str(unsigned_windows.get("env", {})),
        "Windows packaging must consume signed inputs without signing credentials",
    )

    windows = job(workflow, "package-windows", "release.yml")
    windows_commands = commands(windows)
    windows_actions = actions(windows)
    require(
        {"preflight", "package-windows-unsigned"}.issubset(
            {str(item) for item in windows.get("needs", [])}
        )
        and windows.get("permissions")
        == {"contents": "read", "id-token": "write"},
        "Windows final MSI signing must be isolated behind the unsigned package gate",
    )
    for fragment in (
        "signtool verify /pa /all /v /tw",
        "AUTOMEXIA_WINDOWS_PUBLISHER_SUBJECT",
        "TimeStamperCertificate",
    ):
        require(fragment in windows_commands, f"Windows release trust is missing {fragment!r}")
    msi_signing = [
        step
        for step in steps(windows)
        if "azure/artifact-signing-action@" in str(step.get("uses", ""))
    ]
    require(
        len(msi_signing) == 1
        and msi_signing[0].get("with", {}).get("files-folder") == "unsigned"
        and msi_signing[0].get("with", {}).get("files-folder-filter") == "msi",
        "Windows release must sign the final MSI from the isolated unsigned directory",
    )
    require(
        any("azure/login@" in value for value in windows_actions),
        "Windows final MSI signing must use OIDC Azure login",
    )
    windows_uploads = [
        step for step in steps(windows) if "actions/upload-artifact@" in str(step.get("uses", ""))
    ]
    require(
        any(
            step.get("with", {}).get("name") == "packages-${{ matrix.artifact }}"
            and "unsigned/*.msi" in str(step.get("with", {}).get("path", ""))
            and "unsigned/*.zip" in str(step.get("with", {}).get("path", ""))
            for step in windows_uploads
        ),
        "Windows release upload must contain only final MSI and ZIP packages",
    )

    verify_windows = job(workflow, "verify-windows-final", "release.yml")
    require(
        {
            ("windows-2025", "x86_64"),
            ("windows-11-arm", "aarch64"),
        }.issubset(matrix_pairs(verify_windows, "runner", "arch")),
        "final Windows verification must retain native x86_64 and ARM64 runners",
    )
    verify_windows_commands = commands(verify_windows)
    for fragment in (
        "Start-Process msiexec.exe",
        "Get-AuthenticodeSignature",
        "MSI uninstall failed",
        "portable executable signature",
    ):
        require(
            fragment in verify_windows_commands,
            f"final Windows verification is missing {fragment!r}",
        )

    macos = job(workflow, "package-macos", "release.yml")
    macos_commands = commands(macos)
    for fragment in (
        "codesign --verify",
        "codesign --force --options runtime --timestamp",
        "notarytool submit",
        "stapler validate",
        "spctl --assess --type execute",
        "hdiutil attach",
        "com.apple.security.get-task-allow",
    ):
        require(fragment in macos_commands, f"macOS release validation is missing {fragment!r}")
    require(
        "--timestamp --deep --sign" not in macos_commands,
        "macOS release signing must be inside-out rather than codesign --deep",
    )

    linux = job(workflow, "package-linux", "release.yml")
    linux_commands = commands(linux)
    for fragment in ("desktop-file-validate", "appstreamcli validate", "lintian", "test_linux_package.sh"):
        require(fragment in linux_commands, f"Linux release validation is missing {fragment!r}")
    linux_uploads = [
        step for step in steps(linux) if "actions/upload-artifact@" in str(step.get("uses", ""))
    ]
    require(
        any(
            all(
                fragment in str(step.get("with", {}).get("path", ""))
                for fragment in ("*.deb", "*.rpm", "*.tar.gz")
            )
            for step in linux_uploads
        ),
        "Linux release upload must contain only DEB, RPM, and portable archives",
    )

    hardware = job(workflow, "hardware-smoke", "release.yml")
    hardware_dependencies = {str(item) for item in hardware.get("needs", [])}
    require(
        {"package-windows", "package-linux"}.issubset(hardware_dependencies),
        "controlled release trust must consume final signed Windows and Linux packages",
    )
    require(
        "defender" in {str(label).lower() for label in hardware.get("runs-on", [])},
        "controlled release trust must use a Defender-enabled runner",
    )
    hardware_commands = commands(hardware)
    require(
        all(
            fragment in hardware_commands
            for fragment in (
                "test_release_trust_windows.ps1",
                "-Version $env:AUTOMEXIA_VERSION",
                "x86_64-pc-windows-msvc.zip",
                "x86_64-unknown-linux-gnu.tar.gz",
            )
        ),
        "controlled release trust must scan and launch final version-bound packages",
    )
    hardware_downloads = {
        str(step.get("with", {}).get("name", ""))
        for step in steps(hardware)
        if "actions/download-artifact@" in str(step.get("uses", ""))
    }
    require(
        {
            "packages-windows-x86_64",
            "packages-windows-arm64",
            "packages-linux-x86_64",
        }.issubset(hardware_downloads)
        and "windows-x86_64" not in hardware_downloads
        and "linux-x86_64" not in hardware_downloads,
        "controlled hardware smoke must consume final packages, not unsigned build intermediates",
    )

    final_gate = job(workflow, "release-final-gate", "release.yml")
    require(
        {
            "authorize",
            "verify-windows-final",
            "verify-macos-final",
            "verify-linux-final",
            "reproducibility-linux",
            "hardware-smoke",
        }.issubset({str(item) for item in final_gate.get("needs", [])}),
        "release final gate must depend on every native verification, reproducibility, and controlled hardware smoke",
    )
    require(
        "always()" in str(final_gate.get("if", ""))
        and "needs.authorize.result == 'success'" in str(final_gate.get("if", "")),
        "release final gate must fail closed after authorization",
    )

    reproducibility = job(workflow, "reproducibility-linux", "release.yml")
    reproducibility_commands = commands(reproducibility)
    reproducibility_uploads = [
        step
        for step in steps(reproducibility)
        if "actions/upload-artifact@" in str(step.get("uses", ""))
    ]
    require(
        "check_reproducible_build.sh" in reproducibility_commands
        and "reproducibility-${{ matrix.arch }}.json" in reproducibility_commands
        and {
            ("ubuntu-22.04", "x86_64"),
            ("ubuntu-22.04-arm", "aarch64"),
        }.issubset(matrix_pairs(reproducibility, "runner", "arch")),
        "release reproducibility must compare cold native x86_64 and ARM64 Linux builds",
    )
    require(
        any(
            step.get("with", {}).get("name")
            == "release-reproducibility-${{ matrix.arch }}"
            and step.get("with", {}).get("path")
            == "trust/evidence/reproducibility-${{ matrix.arch }}.json"
            for step in reproducibility_uploads
        ),
        "release reproducibility must retain a per-architecture cold-build receipt",
    )

    publish = job(workflow, "publish", "release.yml")
    dependencies = {str(item) for item in publish.get("needs", [])}
    require(
        {
            "preflight",
            "reproducibility-linux",
            "package-windows",
            "package-macos",
            "package-linux",
            "release-final-gate",
        }.issubset(dependencies),
        "publication assembly must depend on reproducibility, every package, and the final artifact gate",
    )
    require(
        publish.get("permissions") == {"contents": "read"},
        "publication assembly must remain read-only",
    )
    downloads = [
        step for step in steps(publish) if "actions/download-artifact@" in str(step.get("uses", ""))
    ]
    require(
        any(step.get("with", {}).get("pattern") == "packages-*" for step in downloads),
        "publication must download only package-* artifacts, never unsigned build intermediates",
    )
    publish_commands = commands(publish)
    for fragment in (
        "build_release_manifest.py",
        "--copy-packages",
        "--include-sbom",
        "sbom-input/Cargo.lock",
        "release-assets/*.msi",
        "release-assets/*.tar.gz",
    ):
        require(
            fragment in publish_commands or fragment in str(publish),
            f"publication assembly is missing {fragment!r}",
        )
    require(
        "--clobber" not in publish_commands,
        "publication assembly must never overwrite existing release assets",
    )
    publish_actions = actions(publish)
    require(
        sum("anchore/sbom-action@" in value for value in publish_actions) == 2,
        "final packages require SPDX and CycloneDX SBOM generation",
    )

    signed_bundle = job(workflow, "sign-release-bundle", "release.yml")
    require(
        {"publish", "preflight"}.issubset(
            {str(item) for item in signed_bundle.get("needs", [])}
        )
        and signed_bundle.get("permissions") == {"contents": "read"}
        and "minisign -S" in commands(signed_bundle)
        and "RELEASE_MINISIGN_SECRET_KEY" in str(signed_bundle),
        "release checksum signing must consume the verified read-only publication bundle",
    )

    publication_dependencies = {str(item) for item in publish_release.get("needs", [])}
    publication_commands = commands(publish_release)
    require(
        {"sign-release-bundle", "preflight"}.issubset(publication_dependencies)
        and "gh release create" in publication_commands
        and "gh release upload" in publication_commands
        and "gh release edit" in publication_commands
        and "refusing mutation" in publication_commands
        and "--clobber" not in publication_commands,
        "immutable publication must consume the signed bundle without overwriting releases",
    )


def validate_s1_assurance(workflow: dict[str, Any]) -> None:
    triggers = workflow.get("on", workflow.get(True))
    require(
        isinstance(triggers, dict) and set(triggers) == {"workflow_dispatch"},
        "S1 assurance must use manual dispatch only",
    )
    require(
        workflow.get("permissions") == {"contents": "read"},
        "S1 assurance workflow must be read-only",
    )
    validate = job(workflow, "validate", "s1-assurance.yml")
    labels = {str(label).lower() for label in validate.get("runs-on", [])}
    require(
        {"self-hosted", "automexia-assurance"}.issubset(labels),
        "S1 assurance must use the controlled assurance runner",
    )
    require(
        validate.get("if") == "vars.AUTOMEXIA_S1_ASSURANCE_RUNNER == '1'",
        "S1 assurance activation must remain explicit",
    )
    timeout = validate.get("timeout-minutes")
    require(
        isinstance(timeout, int)
        and not isinstance(timeout, bool)
        and 1 <= timeout <= 30,
        "S1 assurance must retain a bounded timeout",
    )
    validate_steps = steps(validate)
    require(
        validate.get("continue-on-error") is not True
        and all(step.get("continue-on-error") is not True for step in validate_steps),
        "S1 assurance must fail closed",
    )
    source = commands(validate)
    for fragment, message in (
        ("s1_assurance.py check-policy", "reviewed policy"),
        ("test_s1_assurance.py", "mutation tests"),
        ("s1_assurance.py validate", "manifest validator"),
        ("--expected-commit $env:GITHUB_SHA", "source binding"),
        ("--require-complete", "complete matrix"),
        ("--output target/s1-assurance/summary.json", "exact summary"),
    ):
        require(fragment in source, f"S1 assurance is missing {message}")
    require(
        "AUTOMEXIA_S1_ASSURANCE_EVIDENCE" in str(validate.get("env", {})),
        "S1 assurance must consume the controlled evidence path",
    )
    require_exact_upload(
        validate,
        name="s1-assurance-summary",
        path="target/s1-assurance/summary.json",
        retention_days=90,
        label="S1 assurance exact summary",
    )


def validate_s2_assurance(workflow: dict[str, Any]) -> None:
    triggers = workflow.get("on", workflow.get(True))
    require(
        isinstance(triggers, dict) and set(triggers) == {"workflow_dispatch"},
        "S2 activation must use manual dispatch only",
    )
    require(
        workflow.get("permissions") == {"contents": "read"},
        "S2 activation workflow must be read-only",
    )
    require(
        workflow.get("concurrency")
        == {
            "group": "s2-controlled-activation-${{ github.ref }}",
            "cancel-in-progress": False,
        },
        "S2 activation must retain serialized activation per ref",
    )
    validate = job(workflow, "validate-active-baseline", "s2-assurance.yml")
    require(
        validate.get("runs-on") == "ubuntu-24.04",
        "S2 activation validation runner must remain on the GitHub-Free Ubuntu runner",
    )
    require(
        "environment" not in validate,
        "S2 activation must not depend on an unavailable private protected environment",
    )
    require(
        "secrets." not in str(validate.get("env", {})),
        "S2 activation must not consume repository secrets",
    )
    require(
        validate.get("permissions") in (None, {"contents": "read"}),
        "S2 activation must not elevate its read-only permissions",
    )
    timeout = validate.get("timeout-minutes")
    require(
        isinstance(timeout, int) and not isinstance(timeout, bool) and 1 <= timeout <= 30,
        "S2 activation must retain a bounded timeout",
    )
    validate_steps = steps(validate)
    require(
        validate.get("continue-on-error") is not True
        and all(step.get("continue-on-error") is not True for step in validate_steps),
        "S2 activation must fail closed",
    )
    source = commands(validate)
    for fragment, message in (
        ("set -euo pipefail", "fail-closed shell"),
        ("performance_assurance.py check-policy", "reviewed policy"),
        ("test_performance_assurance.py", "mutation suite"),
        ("performance_assurance.py validate-baseline", "baseline validator"),
        ('--expected-source-commit "$GITHUB_SHA"', "exact source commit"),
        ("target/performance/s2-activation-summary.json", "bounded summary"),
    ):
        require(fragment in source, f"S2 activation is missing {message}")
    require_exact_upload(
        validate,
        name="automexia-s2-activation-summary",
        path="target/performance/s2-activation-summary.json",
        retention_days=90,
        label="S2 activation 90-day summary",
    )


def validate_f5_openssh_assurance(workflow: dict[str, Any]) -> None:
    triggers = workflow.get("on", workflow.get(True))
    require(
        isinstance(triggers, dict) and set(triggers) == {"workflow_dispatch"},
        "F5 OpenSSH assurance must use manual dispatch only",
    )
    dispatch = triggers["workflow_dispatch"]
    inputs = dispatch.get("inputs", {}) if isinstance(dispatch, dict) else {}
    require(
        isinstance(inputs, dict)
        and set(inputs) == {"source_commit", "platform", "architecture"}
        and inputs["platform"].get("options") == ["windows", "macos", "linux"]
        and inputs["architecture"].get("options") == ["x86_64", "aarch64"]
        and inputs["source_commit"].get("type") == "string"
        and inputs["platform"].get("type") == "choice"
        and inputs["architecture"].get("type") == "choice"
        and all(
            isinstance(value, dict) and value.get("required") is True
            for value in inputs.values()
        ),
        "F5 OpenSSH assurance dispatch inputs must remain exact and required",
    )
    require(
        workflow.get("permissions") == {"contents": "read"},
        "F5 OpenSSH assurance workflow must be read-only",
    )
    require(
        workflow.get("concurrency")
        == {
            "group": "f5-openssh-${{ inputs.source_commit }}-${{ inputs.platform }}-${{ inputs.architecture }}",
            "cancel-in-progress": False,
        },
        "F5 OpenSSH assurance concurrency must bind the exact request without cancellation",
    )
    validate = job(workflow, "validate", "f5-openssh-assurance.yml")
    runs_on = validate.get("runs-on", {})
    require(
        isinstance(runs_on, dict)
        and runs_on.get("group") == "automexia-openssh"
        and runs_on.get("labels")
        == "automexia-openssh-${{ inputs.platform }}-${{ inputs.architecture }}",
        "F5 OpenSSH assurance must use the restricted self-hosted runner group",
    )
    require(
        "environment" not in validate,
        "F5 OpenSSH assurance must not depend on an unavailable private protected environment",
    )
    require(
        validate.get("permissions") in (None, {"contents": "read"}),
        "F5 OpenSSH assurance must not elevate its read-only permissions",
    )
    require(
        validate.get("if") == "vars.AUTOMEXIA_F5_OPENSSH_RUNNER == '1'",
        "F5 OpenSSH assurance must remain explicitly operator-enabled",
    )
    require(
        validate.get("timeout-minutes") == 40,
        "F5 OpenSSH assurance timeout must remain exactly 40 minutes",
    )
    environment = validate.get("env", {})
    required_environment = {
        "AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE": "${{ secrets.AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE }}",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY": "${{ secrets.AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY }}",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE": "${{ secrets.AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE }}",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW": "${{ secrets.AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW }}",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE": "${{ secrets.AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE }}",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT": "${{ inputs.source_commit }}",
    }
    require(
        environment == required_environment,
        "F5 OpenSSH assurance must receive the private manifest and exact artifact paths",
    )
    validate_steps = steps(validate)
    require(
        validate.get("continue-on-error") is not True
        and all(step.get("continue-on-error") is not True for step in validate_steps),
        "F5 OpenSSH assurance must fail closed",
    )
    platform_steps = {
        str(step.get("shell")): str(step.get("run", ""))
        for step in validate_steps
        if step.get("shell") in {"pwsh", "bash"}
    }
    require(
        set(platform_steps) == {"pwsh", "bash"},
        "F5 OpenSSH assurance must have exact Windows and Unix validation steps",
    )
    for shell, source in platform_steps.items():
        for fragment, message in (
            ("check_session_launch_d0.py", "D0 contract checker"),
            ("test_session_launch_d0.py", "D0 mutation tests"),
            ("test_native_openssh_evidence.py", "native evidence mutation tests"),
            ("--validate-environment", "controlled validator"),
            ("target/native-openssh/summary.json", "redacted summary"),
        ):
            require(
                fragment in source,
                f"F5 OpenSSH {shell} assurance is missing {message}",
            )
        require(
            "|| true" not in source and "continue-on-error" not in source,
            "F5 OpenSSH assurance must fail closed",
        )
    require(
        "set -euo pipefail" in platform_steps["bash"],
        "F5 OpenSSH Unix assurance is missing fail-closed shell settings",
    )
    require(
        platform_steps["pwsh"].count("$LASTEXITCODE -ne 0") == 4,
        "F5 OpenSSH Windows assurance must check every command exit",
    )
    checkout = next(
        (
            step
            for step in steps(validate)
            if str(step.get("uses", "")).startswith("actions/checkout@")
        ),
        None,
    )
    require(
        isinstance(checkout, dict)
        and checkout.get("with", {}).get("persist-credentials") is False
        and "inputs.source_commit" in str(checkout.get("with", {}).get("ref", "")),
        "F5 OpenSSH assurance checkout must bind the requested commit without credentials",
    )
    require_exact_upload(
        validate,
        name="f5-openssh-${{ inputs.platform }}-${{ inputs.architecture }}-summary",
        path="target/native-openssh/summary.json",
        retention_days=90,
        label="F5 OpenSSH 90-day summary",
    )


def validate_macos_runtime_contract(source: str) -> None:
    """Keep the supported macOS deployment floor independent of hard linking."""
    require(
        'CARGO_CFG_TARGET_OS' in source and 'Ok("macos")' in source,
        "the frontend build script must scope the compatibility link to macOS targets",
    )
    require(
        'cargo:rustc-link-arg-bins=-weak_framework' in source
        and 'cargo:rustc-link-arg-bins=CoreGraphics' in source,
        "the macOS frontend must weak-link CoreGraphics for binary targets",
    )


def validate_windows_release_trust_contract(source: str) -> None:
    """Keep native archive, signature, scanner, and cleanup limits fail-closed."""
    for fragment in (
        "MaximumArchiveEntries",
        "$expectedPackageNames = @(",
        "Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256",
        "$expectedFiles = @(",
        "shell-integration/powershell/automexia.ps1",
        "embedded_script_signature_count",
        "VersionInfo.ProductVersion",
        "totalExpandedBytes",
        "CompressedLength * 200",
        "TimeStamperCertificate",
        "1.3.6.1.5.5.7.3.3",
        "MaximumSignatureAgeHours",
        "automexia-portable-$portableIndex.exe",
        "-DisableRemediation",
        "Wait-Job -Job $scanJob -Timeout $ScanTimeoutSeconds",
        "Stop-Job -Job $scanJob",
        "Remove-Job -Job $scanJob",
    ):
        require(
            fragment in source,
            f"Windows release trust script is missing {fragment!r}",
        )
    lowered = source.casefold()
    require(
        "add-mppreference" not in lowered
        and "set-mppreference" not in lowered
        and "-exclusion" not in lowered,
        "Windows release trust must not weaken Defender or create exclusions",
    )


def validate_repository_workflows() -> None:
    validate_ci(load_workflow("ci.yml"))
    validate_nightly(load_workflow("nightly.yml"))
    validate_release(load_workflow("release.yml"))
    validate_s1_assurance(load_workflow("s1-assurance.yml"))
    validate_s2_assurance(load_workflow("s2-assurance.yml"))
    validate_macos_runtime_contract(MACOS_BUILD_SCRIPT.read_text(encoding="utf-8"))
    validate_windows_release_trust_contract(
        WINDOWS_RELEASE_TRUST_SCRIPT.read_text(encoding="utf-8")
    )
    validate_f5_openssh_assurance(load_workflow("f5-openssh-assurance.yml"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit machine-readable evidence")
    arguments = parser.parse_args()
    try:
        validate_repository_workflows()
    except (OSError, yaml.YAMLError, PlatformCoverageError) as error:
        print(f"platform coverage validation failed: {error}", file=sys.stderr)
        return 1

    evidence = {
        "status": "pass",
        "hosted_native": ["windows", "linux", "macos"],
        "linux_display_features": ["x11", "wayland", "x11+wayland"],
        "artifact_architectures": {
            "windows": ["x86_64", "arm64"],
            "macos": ["x86_64", "arm64", "universal"],
            "linux": ["x86_64", "arm64"],
        },
        "external_limitations": list(EXTERNAL_LIMITATIONS),
    }
    if arguments.json:
        print(json.dumps(evidence, indent=2, sort_keys=True))
    else:
        print("PASS: cross-platform CI, shell, display-feature, architecture, and package contracts are present")
        for limitation in EXTERNAL_LIMITATIONS:
            print(f"EXTERNAL: {limitation}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
