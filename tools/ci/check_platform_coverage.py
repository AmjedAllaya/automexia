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
    native = job(workflow, "native", "ci.yml")
    require(
        {"ubuntu-latest", "windows-latest", "macos-latest"}.issubset(
            matrix_values(native, "os")
        ),
        "native CI must test Ubuntu, Windows, and macOS",
    )
    require(str(native.get("runs-on")) == "${{ matrix.os }}", "native CI must run on matrix.os")
    native_commands = commands(native)
    for fragment in (
        "cargo fmt --all -- --check",
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
        "cargo nextest run --workspace --all-features --locked --profile ci",
        "cargo test --workspace --all-features --doc --locked",
    ):
        require(fragment in native_commands, f"native CI is missing {fragment!r}")

    require_step_condition(
        native,
        "bash tools/ci/test_shell_sources.sh",
        ("runner.os == 'Linux'", "runner.os == 'macOS'"),
    )
    require_step_condition(
        native, "tools/ci/test_powershell.ps1", ("runner.os == 'Windows'",)
    )
    require_step_condition(
        native, "tools/ci/test_librio_c_api.sh", ("runner.os == 'Linux'",)
    )
    power_shell = step_for_command(native, "tools/ci/test_powershell.ps1")
    require(
        str((power_shell or {}).get("shell", "")).lower() == "pwsh",
        "the Windows PowerShell/CMD contract must run under pwsh",
    )
    for fragment in (
        "cargo xtask test resize-stress",
        "cargo xtask test session-clone",
    ):
        require_step_condition(native, fragment, ("runner.os == 'Windows'",))

    linux_features = job(workflow, "linux-features", "ci.yml")
    feature_matrix = linux_features.get("strategy", {}).get("matrix", {}).get("include", [])
    actual_features = {
        (str(item.get("features")), str(item.get("args")))
        for item in feature_matrix
        if isinstance(item, dict)
    }
    expected_features = {
        ("x11", "--no-default-features --features x11"),
        ("wayland", "--no-default-features --features wayland"),
        ("x11-wayland", "--features x11,wayland"),
    }
    require(
        expected_features.issubset(actual_features),
        "Linux CI must check X11-only, Wayland-only, and combined features",
    )
    require(
        "cargo check -p automexia-terminal --all-targets --locked" in commands(linux_features),
        "Linux display-feature checks must cover all frontend targets with the lockfile",
    )

    cross = job(workflow, "cross-checks", "ci.yml")
    require(
        {
            ("windows-latest", "aarch64-pc-windows-msvc"),
            ("macos-latest", "x86_64-apple-darwin"),
            ("macos-latest", "aarch64-apple-darwin"),
        }.issubset(matrix_pairs(cross, "os", "target")),
        "cross-check CI must retain Windows ARM64 and both macOS architecture targets",
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
        workflow.get("permissions") == {"contents": "read"},
        "release workflow must default to contents: read only",
    )
    for job_name, value in workflow["jobs"].items():
        permissions = value.get("permissions", {})
        if job_name != "publish":
            require(
                not isinstance(permissions, dict)
                or permissions.get("contents") != "write",
                f"release job {job_name} must not receive contents: write",
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
    controlled_assurance_gates = {
        "native-gui-resilience",
        "native-wsl-resilience",
        "performance-assurance",
        "s1-assurance",
    }
    require(
        controlled_assurance_gates.issubset(preflight_needs),
        "release preflight controlled assurance gates must include native GUI, "
        "native WSL, S2, and S1 assurance",
    )
    preflight_condition = str(preflight.get("if", ""))
    expected_preflight_condition = " && ".join(
        f"needs.{gate}.result == 'success'"
        for gate in (
            "native-gui-resilience",
            "native-wsl-resilience",
            "performance-assurance",
            "s1-assurance",
        )
    )
    require(
        preflight_condition == expected_preflight_condition,
        "release preflight must fail closed for every controlled assurance gate",
    )
    preflight_environment = preflight.get("env", {})
    for name in (
        "AUTOMEXIA_WINDOWS_CERTIFICATE",
        "AUTOMEXIA_WINDOWS_CERTIFICATE_PASSWORD",
        "AZURE_CLIENT_ID",
        "AZURE_TENANT_ID",
        "AZURE_SUBSCRIPTION_ID",
        "APPLE_CERTIFICATE",
        "APPLE_CERTIFICATE_PASSWORD",
        "APPLE_ID",
        "APPLE_PASSWORD",
        "APPLE_TEAM_ID",
        "APPLE_SIGNING_IDENTITY",
    ):
        value = str(preflight_environment.get(name, ""))
        require(
            "configured" in value and "!= ''" in value,
            f"release preflight must receive only a presence flag for {name}",
        )

    build = job(workflow, "build", "release.yml")
    require(
        {
            ("windows-latest", "x86_64-pc-windows-msvc"),
            ("windows-latest", "aarch64-pc-windows-msvc"),
            ("macos-latest", "x86_64-apple-darwin"),
            ("macos-latest", "aarch64-apple-darwin"),
            ("ubuntu-latest", "x86_64-unknown-linux-gnu"),
            ("ubuntu-latest", "aarch64-unknown-linux-gnu"),
        }.issubset(matrix_pairs(build, "os", "target")),
        "release builds must retain both architectures for Windows, macOS, and Linux",
    )

    windows = job(workflow, "package-windows", "release.yml")
    windows_commands = commands(windows)
    windows_actions = actions(windows)
    require("test_windows_installer.ps1" in windows_commands, "Windows release must run install/upgrade/uninstall smoke")
    for fragment in (
        "signtool verify /pa /all /v /tw",
        "AUTOMEXIA_WINDOWS_PUBLISHER_SUBJECT",
        "TimeStamperCertificate",
    ):
        require(fragment in windows_commands, f"Windows release trust is missing {fragment!r}")
    require(
        sum("azure/artifact-signing-action@" in value for value in windows_actions) == 3,
        "Windows release must support Azure Artifact Signing for EXE, scripts, and MSI",
    )
    executable_signing = [
        step
        for step in steps(windows)
        if "azure/artifact-signing-action@" in str(step.get("uses", ""))
        and step.get("with", {}).get("files-folder-filter") == "exe"
    ]
    require(
        len(executable_signing) == 1
        and executable_signing[0].get("with", {}).get("files-folder") == "signing-input",
        "Windows executable signing must consume the isolated flat signing-input directory",
    )
    script_signing = [
        step
        for step in steps(windows)
        if "azure/artifact-signing-action@" in str(step.get("uses", ""))
        and step.get("with", {}).get("files-folder") == "shell-integration"
    ]
    require(
        len(script_signing) == 1
        and script_signing[0].get("with", {}).get("files-folder-filter") == "ps1,ps1xml"
        and script_signing[0].get("with", {}).get("files-folder-recurse") is True
        and "Set-AuthenticodeSignature" in windows_commands,
        "Windows release must timestamp-sign every distributed PowerShell asset",
    )
    require(
        any("azure/login@" in value for value in windows_actions),
        "Windows Artifact Signing must use OIDC Azure login",
    )
    windows_uploads = [
        step for step in steps(windows) if "actions/upload-artifact@" in str(step.get("uses", ""))
    ]
    require(
        any(
            "signed/*.msi" in str(step.get("with", {}).get("path", ""))
            and "signed/*.zip" in str(step.get("with", {}).get("path", ""))
            and "signed/*\n" not in str(step.get("with", {}).get("path", ""))
            for step in windows_uploads
        ),
        "Windows release upload must contain only final MSI and ZIP packages",
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

    publish = job(workflow, "publish", "release.yml")
    dependencies = {str(item) for item in publish.get("needs", [])}
    require(
        {
            "reproducibility-linux",
            "package-windows",
            "package-macos",
            "package-linux",
            "hardware-smoke",
        }.issubset(dependencies),
        "publication must depend on reproducibility, every platform package, and controlled hardware smoke",
    )
    reproducibility = job(workflow, "reproducibility-linux", "release.yml")
    require(
        "check_reproducible_build.sh" in commands(reproducibility)
        and any(
            step.get("with", {}).get("name") == "release-evidence-reproducibility"
            for step in steps(reproducibility)
        ),
        "release reproducibility must compare cold builds and retain evidence",
    )
    publish_permissions = publish.get("permissions", {})
    require(
        isinstance(publish_permissions, dict)
        and publish_permissions.get("contents") == "write"
        and publish_permissions.get("id-token") == "write"
        and publish_permissions.get("attestations") == "write",
        "publish alone must receive release, OIDC, and attestation write permissions",
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
        "release_trust.py",
        "--verify-final",
        "--expected-windows-publisher",
        "immutable-releases",
        "sbom-input/Cargo.lock",
        "Refusing to modify an existing release",
        "release-assets/*.msi",
        "release-assets/*.tar.gz",
    ):
        require(fragment in publish_commands or fragment in str(publish), f"publication trust is missing {fragment!r}")
    require(
        "--clobber" not in publish_commands,
        "publication must never overwrite existing release assets",
    )
    publish_actions = actions(publish)
    require(
        sum("anchore/sbom-action@" in value for value in publish_actions) == 2,
        "final packages require SPDX and CycloneDX SBOM generation",
    )
    attest_steps = [
        step for step in steps(publish) if "actions/attest@" in str(step.get("uses", ""))
    ]
    require(
        len(attest_steps) == 2
        and any("sbom-path" in step.get("with", {}) for step in attest_steps),
        "final packages require separate provenance and SBOM attestations",
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
        validate.get("runs-on") == "ubuntu-latest",
        "S2 activation validation runner must remain explicit",
    )
    require(
        validate.get("environment") == "stable-release",
        "S2 activation must use the protected environment",
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
    validate = job(workflow, "validate", "f5-openssh-assurance.yml")
    runs_on = validate.get("runs-on", {})
    require(
        isinstance(runs_on, dict)
        and runs_on.get("group") == "automexia-openssh"
        and "inputs.platform" in str(runs_on.get("labels", ""))
        and "inputs.architecture" in str(runs_on.get("labels", "")),
        "F5 OpenSSH assurance must use the restricted self-hosted runner group",
    )
    require(
        validate.get("environment") == "f5-openssh-release",
        "F5 OpenSSH assurance must use the protected environment",
    )
    require(
        "AUTOMEXIA_F5_OPENSSH_RUNNER" in str(validate.get("if", "")),
        "F5 OpenSSH assurance must remain explicitly operator-enabled",
    )
    environment = validate.get("env", {})
    required_environment = {
        "AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE",
        "AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT",
    }
    require(
        isinstance(environment, dict)
        and required_environment.issubset(environment),
        "F5 OpenSSH assurance must receive the private manifest and exact artifact paths",
    )
    source = commands(validate)
    for fragment, message in (
        ("test_native_openssh_evidence.py", "mutation tests"),
        ("--validate-environment", "controlled validator"),
        ("target/native-openssh/summary.json", "redacted summary"),
    ):
        require(fragment in source, f"F5 OpenSSH assurance is missing {message}")
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
