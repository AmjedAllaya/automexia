use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type TaskResult<T = ()> = Result<T, String>;

const FRONTEND_PACKAGE: &str = "automexia-terminal";
const FRONTEND_PATH: &str = "apps/automexia-terminal";
const PRODUCT_NAME: &str = "Automexia Terminal";
const APPLICATION_ID: &str = "io.github.AmjedAllaya.AutomexiaTerminal";
const RIO_BASE_SHA: &str = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2";

fn main() {
    if let Err(error) = dispatch(env::args().skip(1).collect()) {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn dispatch(args: Vec<String>) -> TaskResult {
    match args.as_slice() {
        [command] if command == "doctor" => doctor(),
        [command] if command == "check" => check(),
        [command] if command == "ci" => ci(),
        [command, scope] if command == "verify" && scope == "architecture" => {
            verify_architecture()
        }
        [command, scope] if command == "verify" && scope == "identity" => {
            verify_identity()
        }
        [command, scope] if command == "verify" && scope == "provenance" => {
            verify_provenance()
        }
        [command, scope] if command == "test" && scope == "conformance" => {
            test_conformance()
        }
        [command, flag] if command == "package" && flag == "--check" => package_check(),
        [command, flag, target] if command == "package" && flag == "--target" => {
            package_target(target)
        }
        [command, flag, version] if command == "release" && flag == "--version" => {
            release(version)
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo xtask <doctor|check|ci|verify architecture|verify identity|verify provenance|test conformance|package --check|package --target TARGET|release --version VERSION>".into()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn doctor() -> TaskResult {
    let required = ["cargo", "rustc", "rustfmt", "git"];
    let optional = ["cargo-deny", "cargo-llvm-cov", "cargo-packager", "nfpm"];
    let mut missing = Vec::new();
    for program in required {
        let available = command_available(program);
        println!("{program:<18} {}", if available { "ok" } else { "missing" });
        if !available {
            missing.push(program);
        }
    }
    for program in optional {
        println!(
            "{program:<18} {}",
            if command_available(program) {
                "ok"
            } else {
                "optional/missing"
            }
        );
    }
    #[cfg(target_os = "windows")]
    println!("platform           Windows: Visual Studio Build Tools and WiX are required for MSI builds");
    #[cfg(target_os = "macos")]
    println!("platform           macOS: Xcode CLI tools and Apple signing credentials are required for releases");
    #[cfg(target_os = "linux")]
    println!("platform           Linux: X11, Wayland, fontconfig, and audio development packages are required");
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing required tools: {}", missing.join(", ")))
    }
}

fn check() -> TaskResult {
    verify_identity()?;
    verify_provenance()?;
    verify_architecture()?;
    package_check()?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["metadata", "--locked", "--format-version", "1"])?;
    run(
        "cargo",
        &["check", "--workspace", "--all-targets", "--locked"],
    )
}

fn ci() -> TaskResult {
    check()?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace", "--locked"])
}

fn test_conformance() -> TaskResult {
    run(
        "cargo",
        &[
            "test",
            "-p",
            "automexia-terminal",
            "-p",
            "rio-vt",
            "-p",
            "rio-backend",
            "-p",
            "rio-window",
            "-p",
            "sugarloaf",
            "-p",
            "teletypewriter",
            "--locked",
        ],
    )
}

fn run(program: &str, args: &[&str]) -> TaskResult {
    println!("+ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .status()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn metadata() -> TaskResult<Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root())
        .output()
        .map_err(|error| format!("could not start cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid cargo metadata JSON: {error}"))
}

fn verify_architecture() -> TaskResult {
    let metadata = metadata()?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata omitted packages")?;
    let frontend = packages
        .iter()
        .find(|package| package["name"] == FRONTEND_PACKAGE)
        .ok_or("Automexia frontend package is missing")?;
    let manifest = frontend["manifest_path"]
        .as_str()
        .ok_or("frontend manifest path is missing")?
        .replace('\\', "/");
    require(
        manifest.ends_with(&format!("{FRONTEND_PATH}/Cargo.toml")),
        "frontend package is outside apps/automexia-terminal",
    )?;

    let app = root().join(FRONTEND_PATH);
    let renderer = read(&app.join("src/renderer/devops_status.rs"))?;
    for forbidden in ["std::process::Command", "TcpStream", "UdpSocket", "reqwest"] {
        require(
            !renderer.contains(forbidden),
            &format!("renderer performs extension I/O through {forbidden}"),
        )?;
    }
    let runtime = read(&app.join("src/automexia/runtime.rs"))?;
    require(
        runtime.contains("sync_channel") && runtime.contains("try_send"),
        "extension runtime does not expose a bounded non-blocking worker queue",
    )?;

    for engine in ["rio-vt", "teletypewriter", "sugarloaf", "rio-window"] {
        let package = packages
            .iter()
            .find(|package| package["name"] == engine)
            .ok_or_else(|| format!("engine package {engine} is missing"))?;
        let depends_on_frontend =
            package["dependencies"]
                .as_array()
                .is_some_and(|dependencies| {
                    dependencies
                        .iter()
                        .any(|dependency| dependency["name"] == FRONTEND_PACKAGE)
                });
        require(
            !depends_on_frontend,
            &format!("{engine} depends on the Automexia frontend"),
        )?;
    }
    println!("PASS: dependency graph and render/PTY/extension boundaries verified");
    Ok(())
}

fn verify_identity() -> TaskResult {
    let root = root();
    let required = [
        ("Cargo.toml", "version = \"0.4.0\""),
        ("Cargo.toml", APPLICATION_ID),
        ("README.md", PRODUCT_NAME),
        (
            "packaging/linux/automexia-terminal.desktop",
            "Exec=automexia",
        ),
        ("packaging/macos/Info.plist", APPLICATION_ID),
        (
            "packaging/windows/automexia.wxs",
            "AutomexiaContextMenuComponents",
        ),
    ];
    for (path, needle) in required {
        let content = read(&root.join(path))?;
        require(
            content.contains(needle),
            &format!("{path} is missing canonical identity {needle:?}"),
        )?;
    }

    let scopes = [
        root.join("apps/automexia-terminal"),
        root.join("packaging"),
        root.join("shell-integration"),
    ];
    let forbidden = [
        "\"Rio Terminal",
        "\"Rio Settings",
        "Rio: Runtime Error",
        "rioterm.com",
        "rio://",
        "com.rioterm",
        "frontends/rioterm",
        "target/release/rio",
    ];
    let mut failures = Vec::new();
    for scope in scopes {
        for path in files_under(&scope)? {
            if !is_text_file(&path) {
                continue;
            }
            let content = read(&path)?;
            for token in forbidden {
                if content.contains(token) {
                    failures.push(format!(
                        "{} contains forbidden user-facing token {token:?}",
                        display_relative(&path)
                    ));
                }
            }
        }
    }
    if failures.is_empty() {
        println!("PASS: canonical Automexia product identity verified");
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

fn verify_provenance() -> TaskResult {
    let root = root();
    let license = read(&root.join("LICENSE"))?;
    require(
        license.contains("Copyright (c) 2022-present Raphael Amorim"),
        "LICENSE lost the upstream Rio copyright notice",
    )?;
    require(
        license.contains("Copyright (c) 2026-present Automexia Contributors"),
        "LICENSE is missing Automexia's separate copyright line",
    )?;
    let notice = read(&root.join("NOTICE.md"))?;
    require(
        notice.contains(RIO_BASE_SHA),
        "NOTICE.md has the wrong fork SHA",
    )?;
    require(
        notice.contains("https://github.com/raphamorim/rio"),
        "NOTICE.md is missing the upstream repository",
    )?;

    let metadata = metadata()?;
    for package in metadata["packages"]
        .as_array()
        .ok_or("cargo metadata omitted packages")?
    {
        let publish = package.get("publish");
        require(
            publish.is_some_and(|value| value.as_array().is_some_and(Vec::is_empty)),
            &format!(
                "workspace crate {} is publishable; desktop artifacts are the only v0.4 release surface",
                package["name"].as_str().unwrap_or("unknown")
            ),
        )?;
    }
    println!("PASS: MIT attribution, fork provenance, and private crates verified");
    Ok(())
}

fn package_check() -> TaskResult {
    let root = root();
    for required in [
        "apps/automexia-terminal/Cargo.toml",
        "packaging/windows/automexia.wxs",
        "packaging/macos/Info.plist",
        "packaging/linux/automexia-terminal.desktop",
        "packaging/linux/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml",
        "packaging/linux/nfpm.yaml",
        "packaging/linux/automexia.terminfo",
        "assets/brand/ASSET-MANIFEST.toml",
    ] {
        require(
            root.join(required).is_file(),
            &format!("missing {required}"),
        )?;
    }
    let manifest_text = read(&root.join("assets/brand/ASSET-MANIFEST.toml"))?;
    let manifest: toml::Value = toml::from_str(&manifest_text)
        .map_err(|error| format!("invalid brand asset manifest: {error}"))?;
    let release_ready = manifest
        .get("release")
        .and_then(|value| value.get("ready"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    let release = manifest
        .get("release")
        .and_then(toml::Value::as_table)
        .ok_or("brand manifest is missing [release]")?;
    let frontend_manifest_text = read(&root.join("apps/automexia-terminal/Cargo.toml"))?;
    let frontend_manifest: toml::Value = toml::from_str(&frontend_manifest_text)
        .map_err(|error| format!("invalid frontend Cargo.toml: {error}"))?;
    let packager = frontend_manifest
        .get("package")
        .and_then(|value| value.get("metadata"))
        .and_then(|value| value.get("packager"))
        .ok_or("frontend Cargo.toml is missing package.metadata.packager")?;
    require(
        packager.get("product-name").and_then(toml::Value::as_str) == Some(PRODUCT_NAME),
        "cargo-packager product name is not canonical",
    )?;
    require(
        packager.get("identifier").and_then(toml::Value::as_str) == Some(APPLICATION_ID),
        "cargo-packager application ID is not canonical",
    )?;
    require(
        packager
            .get("wix")
            .and_then(|value| value.get("component-group-refs"))
            .and_then(toml::Value::as_array)
            .is_some_and(|refs| {
                refs.iter()
                    .any(|value| value.as_str() == Some("AutomexiaContextMenuComponents"))
            }),
        "cargo-packager does not include the Automexia WiX context-menu fragment",
    )?;
    if release_ready {
        require(
            release.get("placeholder").and_then(toml::Value::as_bool) == Some(false),
            "release-ready brand manifest still declares placeholder assets",
        )?;
        require(
            release
                .get("rights_verified")
                .and_then(toml::Value::as_bool)
                == Some(true),
            "release-ready brand manifest has no verified redistribution rights",
        )?;
        for field in ["approved_by", "approved_at"] {
            require(
                release
                    .get(field)
                    .and_then(toml::Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()),
                &format!("release-ready brand manifest is missing {field}"),
            )?;
        }
        let assets = manifest
            .get("asset")
            .and_then(toml::Value::as_array)
            .ok_or("brand manifest has no asset entries")?;
        let required_kinds = [
            "editable-logo-svg",
            "standalone-app-mark-svg",
            "light-dark-monochrome-variants",
            "master-png-1024",
            "windows-ico-16-through-256",
            "macos-icns-16-through-1024",
            "linux-png-16-32-48-64-128-256-512",
            "redistribution-rights-proof",
        ];
        for kind in required_kinds {
            let asset = assets
                .iter()
                .find(|asset| {
                    asset.get("kind").and_then(toml::Value::as_str) == Some(kind)
                })
                .ok_or_else(|| {
                    format!("release-ready brand manifest is missing {kind}")
                })?;
            require(
                asset.get("status").and_then(toml::Value::as_str) == Some("final"),
                &format!("brand asset {kind} is not final"),
            )?;
            let path = asset
                .get("path")
                .and_then(toml::Value::as_str)
                .filter(|path| path.starts_with("assets/brand/"))
                .ok_or_else(|| format!("brand asset {kind} has an invalid path"))?;
            require(
                root.join(path).exists(),
                &format!("brand asset {kind} does not exist at {path}"),
            )?;
        }
        println!("PASS: package metadata and final brand manifest verified");
    } else {
        println!("PASS: package metadata verified (stable release remains blocked on final brand assets)");
    }
    Ok(())
}

fn package_target(target: &str) -> TaskResult {
    package_check()?;
    run(
        "cargo",
        &[
            "build",
            "--release",
            "--locked",
            "-p",
            FRONTEND_PACKAGE,
            "--target",
            target,
        ],
    )
}

fn release(version: &str) -> TaskResult {
    require(
        version == "0.4.0",
        "release version must match workspace 0.4.0",
    )?;
    ci()?;
    package_check()?;
    let brand = read(&root().join("assets/brand/ASSET-MANIFEST.toml"))?;
    let brand: toml::Value = toml::from_str(&brand)
        .map_err(|error| format!("invalid brand asset manifest: {error}"))?;
    require(
        brand
            .get("release")
            .and_then(|value| value.get("ready"))
            .and_then(toml::Value::as_bool)
            == Some(true),
        "stable release is blocked until the final brand asset manifest is approved",
    )?;
    let security = read(&root().join("SECURITY.md"))?;
    require(
        !security.contains("CONDUCT_CONTACT_REQUIRED"),
        "stable release is blocked until a private conduct contact is configured",
    )?;
    for variable in [
        "AUTOMEXIA_WINDOWS_CERTIFICATE",
        "AUTOMEXIA_WINDOWS_CERTIFICATE_PASSWORD",
        "APPLE_CERTIFICATE",
        "APPLE_CERTIFICATE_PASSWORD",
        "APPLE_ID",
        "APPLE_PASSWORD",
        "APPLE_TEAM_ID",
        "APPLE_SIGNING_IDENTITY",
    ] {
        require(
            env::var_os(variable).is_some(),
            &format!("stable release credential {variable} is unavailable"),
        )?;
    }
    println!("PASS: release {version} preconditions verified; publication is CI-only");
    Ok(())
}

fn files_under(root: &Path) -> TaskResult<Vec<PathBuf>> {
    if !root.exists() {
        return Err(format!("required directory is missing: {}", root.display()));
    }
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("could not read {}: {error}", directory.display()))?
        {
            let entry =
                entry.map_err(|error| format!("directory entry failed: {error}"))?;
            let kind = entry.file_type().map_err(|error| {
                format!("could not inspect {}: {error}", entry.path().display())
            })?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}

fn is_text_file(path: &Path) -> bool {
    let extensions: BTreeSet<&str> = [
        "rs", "toml", "md", "yml", "yaml", "json", "xml", "plist", "desktop", "wxs",
        "ps1", "sh", "zsh", "bash", "terminfo",
    ]
    .into_iter()
    .collect();
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extensions.contains(extension))
}

fn read(path: &Path) -> TaskResult<String> {
    fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))
}

fn require(condition: bool, message: &str) -> TaskResult {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn display_relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_surface_is_stable() {
        assert!(usage().contains("verify architecture"));
        assert!(usage().contains("test conformance"));
        assert!(usage().contains("release --version"));
    }

    #[test]
    fn identity_constants_match_release_contract() {
        assert_eq!(PRODUCT_NAME, "Automexia Terminal");
        assert_eq!(APPLICATION_ID, "io.github.AmjedAllaya.AutomexiaTerminal");
        assert_eq!(RIO_BASE_SHA.len(), 40);
    }
}
