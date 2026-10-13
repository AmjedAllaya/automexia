//! ARM64 MSI adapter for the repository-pinned WiX compiler.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::{
    command_available, normalize_canonical_path, require, root, run, run_command,
    windows_installer_version, ProductIdentity, TaskResult,
};

pub(super) fn package_windows_arm64(
    identity: &ProductIdentity,
    binary: &Path,
    output: &Path,
    target: &str,
) -> TaskResult {
    require(
        command_available("dotnet"),
        "ARM64 MSI packaging requires the .NET SDK for the repository-pinned WiX 5 tool",
    )?;
    run("dotnet", &["tool", "restore"])?;
    // Restore and select the same UI extension version as the pinned compiler.
    run(
        "dotnet",
        &[
            "tool",
            "run",
            "wix",
            "--",
            "extension",
            "add",
            "--global",
            "WixToolset.UI.wixext/5.0.2",
        ],
    )?;

    // WiX is a .NET application and interprets its source argument as a URI.
    // `std::fs::canonicalize` adds the Windows `\\?\` verbatim prefix used by
    // `root()` which `System.Uri` rejects before WiX can parse the source.
    // Keep filesystem resolution canonical while presenting regular Win32
    // paths to this external tool.
    let workspace = normalize_canonical_path(root());
    let binary = normalize_canonical_path(binary.to_path_buf());
    let output = normalize_canonical_path(output.to_path_buf());

    let msi = output.join(format!(
        "automexia-terminal-{}-{target}.msi",
        identity.version
    ));
    let intermediate = output.join(".wix-arm64");
    fs::create_dir_all(&intermediate).map_err(|error| {
        format!(
            "could not create WiX intermediate directory {}: {error}",
            intermediate.display()
        )
    })?;

    let define = |name: &str, path: &Path| format!("{name}={}", path.display());
    let mut command = Command::new("dotnet");
    command.args([
        "tool",
        "run",
        "wix",
        "--",
        "build",
        "-arch",
        "arm64",
        "-pdbtype",
        "none",
        "-intermediateFolder",
    ]);
    command.arg(&intermediate);
    command.args(["-ext", "WixToolset.UI.wixext/5.0.2", "-culture", "en-us"]);
    command.arg("-loc");
    command.arg(workspace.join("packaging/windows/automexia-arm64-en-us.wxl"));
    command.args([
        "-d",
        &define(
            "InstallerBrandRoot",
            &workspace.join("assets/brand/windows-installer"),
        ),
    ]);
    command.args([
        "-d",
        &define(
            "InstallerLicensePath",
            &workspace.join("packaging/windows/license.rtf"),
        ),
    ]);
    command.args([
        "-d",
        &format!(
            "ProductVersion={}",
            windows_installer_version(&identity.version)?
        ),
    ]);
    command.args(["-d", &define("BinaryPath", &binary)]);
    command.args([
        "-d",
        &define(
            "ConptyRuntimeRoot",
            binary.parent().ok_or("release binary has no parent")?,
        ),
    ]);
    command.args([
        "-d",
        &define("CliBinaryPath", &binary.with_file_name("amx.exe")),
    ]);
    command.args([
        "-d",
        &define(
            "SuggestionHelperPath",
            &binary.with_file_name("automexia-suggestion-helper.exe"),
        ),
    ]);
    command.args([
        "-d",
        &define(
            "SshHelperPath",
            &binary.with_file_name("automexia-ssh-helper.exe"),
        ),
    ]);
    command.args([
        "-d",
        &define(
            "IconPath",
            &workspace.join("assets/brand/automexia-terminal.ico"),
        ),
    ]);
    command.args(["-d", &define("LicensePath", &workspace.join("LICENSE"))]);
    command.args(["-d", &define("NoticePath", &workspace.join("NOTICE.md"))]);
    command.args([
        "-d",
        &define("ThirdPartyPath", &workspace.join("THIRD_PARTY_NOTICES.md")),
    ]);
    command.args(["-d", &define("ReadmePath", &workspace.join("README.md"))]);
    command.args([
        "-d",
        &define("ShellIntegrationRoot", &workspace.join("shell-integration")),
    ]);
    command.args(["-o"]);
    command.arg(&msi);
    command.arg(workspace.join("packaging/windows/automexia-arm64.wxs"));
    command.current_dir(&workspace);
    run_command(command, "WiX 5 ARM64 MSI")?;
    require(
        msi.is_file(),
        &format!("WiX 5 did not produce {}", msi.display()),
    )
}
