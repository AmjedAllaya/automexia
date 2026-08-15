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
    require("test_windows_installer.ps1" in commands(windows), "Windows release must run install/upgrade/uninstall smoke")

    macos = job(workflow, "package-macos", "release.yml")
    macos_commands = commands(macos)
    for fragment in ("codesign --verify", "notarytool submit", "stapler validate", "spctl --assess", "hdiutil attach"):
        require(fragment in macos_commands, f"macOS release validation is missing {fragment!r}")

    linux = job(workflow, "package-linux", "release.yml")
    linux_commands = commands(linux)
    for fragment in ("desktop-file-validate", "appstreamcli validate", "lintian", "test_linux_package.sh"):
        require(fragment in linux_commands, f"Linux release validation is missing {fragment!r}")

    publish = job(workflow, "publish", "release.yml")
    dependencies = {str(item) for item in publish.get("needs", [])}
    require(
        {"package-windows", "package-macos", "package-linux", "hardware-smoke"}.issubset(dependencies),
        "publication must depend on every platform package and controlled hardware smoke",
    )


def validate_repository_workflows() -> None:
    validate_ci(load_workflow("ci.yml"))
    validate_nightly(load_workflow("nightly.yml"))
    validate_release(load_workflow("release.yml"))


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
