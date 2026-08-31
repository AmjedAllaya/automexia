#!/usr/bin/env python3
"""Fail closed when runtime or release trust boundaries drift."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import time
from typing import Any
import xml.etree.ElementTree as ET

import yaml


ROOT = Path(__file__).resolve().parents[2]


class RuntimeTrustError(ValueError):
    """A production trust invariant is absent or ambiguous."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeTrustError(message)


def section(source: str, start: str, end: str) -> str:
    start_index = source.find(start)
    end_index = source.find(end, start_index + len(start))
    require(start_index >= 0 and end_index > start_index, f"missing bounded section {start!r}")
    return source[start_index:end_index]


def validate_launcher(source: str) -> None:
    launch = section(source, "fn launch_plan(", "fn build_debug_app(")
    require(
        "InstallShellIntegration" not in launch
        and "install-windows.ps1" not in launch
        and "ExecutionPolicy" not in launch
        and "powershell" not in launch.casefold(),
        "normal cargo launch must not provision or execute shell installers",
    )
    spawn = section(source, "fn launch_debug_app(", "fn check(")
    require(
        '.env(\n            "AUTOMEXIA_SHELL_INTEGRATION_ROOT"' in spawn,
        "development launch must pass a session-only integration root",
    )
    require(
        '"-ExecutionPolicy"' not in source and '"Bypass"' not in source,
        "maintainer launch and verification commands must honor PowerShell execution policy",
    )


def validate_qa_runner(source: str) -> None:
    require(
        '"-ExecutionPolicy"' not in source
        and '"Bypass"' not in source
        and source.count('"powershell.exe"') >= 3
        and source.count('"-NonInteractive"') >= 3,
        "controlled QA PowerShell commands must be non-interactive and honor execution policy",
    )


def validate_application(main: str, shell: str, integration: str, cmd: str) -> None:
    require(
        main.find("for env_config in config.env_vars.iter()")
        < main.find("shell_integration::prepare_session_environment()"),
        "session integration trust resolution must override config-supplied paths",
    )
    require(
        "run_persistent(" in main
        and "PersistentOperation::Install" in main
        and "if let Some(command) = &args.command" in main,
        "persistent integration must be reachable only through explicit CLI dispatch",
    )
    require(
        "#[cfg(debug_assertions)]\n    if let Some(root) = env::var_os(ROOT_ENV)" in integration,
        "arbitrary integration roots must be accepted only by debug builds",
    )
    require(
        "dunce::canonicalize(path)" in integration
        and "#[cfg(not(windows))]\n    let canonical = path.canonicalize()" in integration,
        "validated Windows integration roots must use a PowerShell-compatible canonical path",
    )
    require(
        '"-ExecutionPolicy"' not in integration
        and '"Bypass"' not in integration
        and 'Command::new("powershell.exe")' in integration,
        "explicit persistent integration must honor PowerShell execution policy",
    )
    require(
        "integration_available: bool" in shell
        and "AUTOMEXIA_SHELL_INTEGRATION_ROOT" in shell
        and "LOCALAPPDATA" not in shell,
        "child shell injection must be gated and use validated session resources",
    )
    bootstrap = section(
        shell, "const POWERSHELL_SESSION_BOOTSTRAP", "pub fn normalized_program"
    )
    require(
        bootstrap.count("Test-Path -LiteralPath $p") == 2
        and "PSSecurityException" in bootstrap
        and "$env:AUTOMEXIA_SHELL_INTEGRATION='0'" in bootstrap
        and "ExecutionPolicy" not in bootstrap
        and "Invoke-Expression" not in bootstrap,
        "PowerShell bootstrap must use literal paths, preserve policy, and fail quietly",
    )
    require(
        "ExecutionPolicy Bypass" not in cmd,
        "shipped CMD helpers must not bypass PowerShell execution policy",
    )


def validate_wsl(source: str) -> None:
    require(
        "AutomexiaWslPayloadBytesLimit" in source
        and "RedirectStandardInput" in source
        and "--exec sh -s" in source,
        "WSL integration must use bounded raw standard input and a fixed shell command",
    )
    require(
        "base64 -d" not in source
        and "sh -c" not in source
        and "Invoke-AutomexiaWslBase64Script" not in source,
        "WSL integration must not restore encoded or nested shell execution",
    )


def validate_manifest(source: str, build_script: str) -> None:
    root = ET.fromstring(source)
    execution = root.find(
        ".//{urn:schemas-microsoft-com:asm.v3}requestedExecutionLevel"
    )
    require(
        execution is not None
        and execution.attrib.get("level") == "asInvoker"
        and execution.attrib.get("uiAccess") == "false",
        "Windows manifest must declare non-elevated asInvoker execution",
    )
    supported = root.find(
        ".//{urn:schemas-microsoft-com:compatibility.v1}supportedOS"
    )
    require(
        supported is not None
        and supported.attrib.get("Id") == "{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}",
        "Windows 10/11 compatibility identity is missing",
    )
    values = {element.tag.rsplit("}", 1)[-1]: (element.text or "").strip() for element in root.iter()}
    require(values.get("dpiAwareness") == "PerMonitorV2,PerMonitor", "PerMonitorV2 DPI awareness is missing")
    require(values.get("longPathAware") == "true", "Windows long-path awareness is missing")
    require(values.get("activeCodePage") == "UTF-8", "Windows UTF-8 active code page is missing")
    require(
        "set_manifest_file(manifest_path)" in build_script
        and 'replace("@AUTOMEXIA_VERSION@"' in build_script,
        "the version-derived Windows manifest must be embedded by the build",
    )


def workflow_steps(job: dict[str, Any]) -> list[dict[str, Any]]:
    steps = job.get("steps")
    require(isinstance(steps, list), "release job has no steps")
    return [step for step in steps if isinstance(step, dict)]


def workflow_commands(job: dict[str, Any]) -> str:
    return "\n".join(str(step.get("run", "")) for step in workflow_steps(job))


def validate_release(workflow: dict[str, Any]) -> None:
    jobs = workflow.get("jobs")
    require(isinstance(jobs, dict), "release workflow has no jobs")
    windows = jobs.get("package-windows")
    unsigned_windows = jobs.get("package-windows-unsigned")
    runtime_signing = jobs.get("sign-windows-runtime")
    publish = jobs.get("publish")
    publish_release = jobs.get("publish-release")
    reproducibility = jobs.get("reproducibility-linux")
    require(
        isinstance(windows, dict)
        and isinstance(unsigned_windows, dict)
        and isinstance(runtime_signing, dict)
        and isinstance(publish, dict)
        and isinstance(publish_release, dict)
        and isinstance(reproducibility, dict),
        "release trust jobs are missing",
    )
    require(
        "check_reproducible_build.sh" in workflow_commands(reproducibility)
        and "reproducibility-linux" in publish.get("needs", []),
        "publication must depend on independent cold-build reproducibility evidence",
    )

    runtime_steps = workflow_steps(runtime_signing)
    script_signers = [
        step
        for step in runtime_steps
        if "azure/artifact-signing-action@" in str(step.get("uses", ""))
        and step.get("with", {}).get("files-folder") == "signing-input"
    ]
    require(
        len(script_signers) == 1
        and script_signers[0].get("with", {}).get("files-folder-filter") == "exe,ps1,ps1xml"
        and script_signers[0].get("with", {}).get("files-folder-recurse") is True,
        "Windows release must recursively sign PowerShell assets before packaging",
    )
    runtime_names = [str(step.get("name", "")) for step in runtime_steps]
    require(
        runtime_names.index("Verify every signed runtime input without executing it")
        < runtime_names.index("Upload signed runtime inputs"),
        "PowerShell runtime inputs must be verified before publication to packaging",
    )
    runtime_commands = workflow_commands(runtime_signing)
    require(
        "Set-AuthenticodeSignature" in runtime_commands
        and "TimeStamperCertificate" in runtime_commands,
        "PFX signing and timestamp verification for PowerShell assets are missing",
    )
    unsigned_commands = workflow_commands(unsigned_windows)
    require(
        "sign-windows-runtime" in unsigned_windows.get("needs", [])
        and "Download signed runtime inputs" in [
            str(step.get("name", "")) for step in workflow_steps(unsigned_windows)
        ]
        and "Get-AuthenticodeSignature" in unsigned_commands,
        "Windows packaging must consume and revalidate isolated signed runtime inputs",
    )
    require(
        "package-windows-unsigned" in windows.get("needs", []),
        "final Windows installer signing must consume the credential-free package output",
    )

    publish_release_commands = workflow_commands(publish_release)
    require(
        "sign-release-bundle" in publish_release.get("needs", [])
        and 'gh release create "$tag"' in publish_release_commands
        and 'gh release upload "$tag"' in publish_release_commands,
        "publication must create the release only from the signed immutable bundle",
    )
    require(
        "--clobber" not in publish_release_commands
        and "refusing mutation" in publish_release_commands
        and 'gh release view "$tag"' in publish_release_commands,
        "publication must reject existing releases and asset overwrites",
    )
    publish_commands = workflow_commands(publish)
    require(
        "sbom-input/Cargo.lock" in publish_commands,
        "SBOM generation must include the locked Rust dependency graph",
    )
    sbom_steps = [
        step
        for step in workflow_steps(publish)
        if "anchore/sbom-action@" in str(step.get("uses", ""))
    ]
    require(
        len(sbom_steps) == 2
        and all(step.get("with", {}).get("path") == "sbom-input" for step in sbom_steps),
        "SPDX and CycloneDX must be generated from enriched final-package inputs",
    )


def validate_packaging(xtask: str, arm_wix: str) -> None:
    require(
        "copy_bounded_resource_tree(\n        &root().join" in xtask
        and '&staging.join("shell-integration")' in xtask
        and "portable shell resources exceed their file-count or byte ceiling" in xtask,
        "portable packages must include a bounded, non-linked integration tree",
    )
    require(
        "ShellIntegrationRoot" in xtask
        and "AutomexiaShellRootComponent" in arm_wix
        and "AutomexiaShellCompletionPowerShellComponent" in arm_wix,
        "ARM64 MSI must include the complete shell integration resource tree",
    )


def validate_repository() -> float:
    started = time.perf_counter()
    read = lambda path: (ROOT / path).read_text(encoding="utf-8")
    validate_launcher(read("tools/xtask/src/main.rs"))
    validate_qa_runner(read("tools/ci/qa.py"))
    validate_application(
        read("apps/automexia-terminal/src/main.rs"),
        read("apps/automexia-terminal/src/automexia/shell.rs"),
        read("apps/automexia-terminal/src/automexia/shell_integration.rs"),
        read("shell-integration/cmd/automexia-ls.cmd"),
    )
    validate_wsl(read("shell-integration/windows-wsl.ps1"))
    validate_manifest(
        read("packaging/windows/automexia.manifest"),
        read("apps/automexia-terminal/build.rs"),
    )
    validate_packaging(
        read("tools/xtask/src/main.rs"),
        read("packaging/windows/automexia-arm64.wxs"),
    )
    workflow = yaml.safe_load(read(".github/workflows/release.yml"))
    require(isinstance(workflow, dict), "release workflow is not a mapping")
    validate_release(workflow)
    return (time.perf_counter() - started) * 1000


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.parse_args()
    try:
        elapsed = validate_repository()
    except (OSError, ET.ParseError, yaml.YAMLError, RuntimeTrustError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"PASS: runtime, package, and release trust boundaries are fail-closed ({elapsed:.2f} ms)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
