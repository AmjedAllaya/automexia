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

    preflight = job(workflow, "preflight", "release.yml")
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
        sum("azure/artifact-signing-action@" in value for value in windows_actions) == 2,
        "Windows release must support Azure Artifact Signing for both EXE and MSI",
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
        {"package-windows", "package-macos", "package-linux", "hardware-smoke"}.issubset(dependencies),
        "publication must depend on every platform package and controlled hardware smoke",
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
        "release-assets/*.msi",
        "release-assets/*.tar.gz",
    ):
        require(fragment in publish_commands or fragment in str(publish), f"publication trust is missing {fragment!r}")
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
        "$expectedFiles = @('automexia.exe', 'LICENSE', 'NOTICE.md', 'README.md', 'THIRD_PARTY_NOTICES.md')",
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
    validate_macos_runtime_contract(MACOS_BUILD_SCRIPT.read_text(encoding="utf-8"))
    validate_windows_release_trust_contract(
        WINDOWS_RELEASE_TRUST_SCRIPT.read_text(encoding="utf-8")
    )


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
