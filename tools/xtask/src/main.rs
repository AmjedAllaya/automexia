use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type TaskResult<T = ()> = Result<T, String>;

const RIO_BASE_SHA: &str = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2";

#[derive(Debug)]
struct ProductIdentity {
    version: String,
    package_name: String,
    frontend_path: String,
    product_name: String,
    executable: String,
    application_id: String,
    url_scheme: String,
    wm_class: String,
    term_program: String,
    config_home_environment: String,
    log_level_environment: String,
    shell_integration_environment: String,
}

fn main() {
    if let Err(error) = dispatch(env::args().skip(1).collect()) {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn dispatch(args: Vec<String>) -> TaskResult {
    match args.as_slice() {
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => {
            println!("{}", usage());
            Ok(())
        }
        [command] if command == "dev" => dev(&[]),
        [command, separator, app_args @ ..] if command == "dev" && separator == "--" => {
            dev(app_args)
        }
        [command] if command == "ready" => ready(),
        [command] if command == "run" => run_app(&[]),
        [command, separator, app_args @ ..] if command == "run" && separator == "--" => {
            run_app(app_args)
        }
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
        [command, scope] if command == "verify" && scope == "all" => verify_all(),
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
    "usage: cargo xtask <dev [-- APP_ARGS...]|ready|run [-- APP_ARGS...]|doctor|check|ci|verify architecture|verify identity|verify provenance|verify all|test conformance|package --check|package --target TARGET|release --version VERSION>".into()
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

fn python_program() -> Option<&'static str> {
    if command_available("python3") {
        Some("python3")
    } else if command_available("python") {
        Some("python")
    } else {
        None
    }
}

fn python_yaml_available(program: &str) -> bool {
    Command::new(program)
        .args(["-c", "import yaml"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn doctor() -> TaskResult {
    let required = ["cargo", "rustc", "rustfmt", "git", "cargo-deny"];
    let optional = ["cargo-llvm-cov", "cargo-packager", "nfpm"];
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
    let python = python_program();
    println!(
        "{:<18} {}",
        "python3",
        if python.is_some() { "ok" } else { "missing" }
    );
    if python.is_none() {
        missing.push("python3");
    }
    let yaml_available = python.is_some_and(python_yaml_available);
    println!(
        "{:<18} {}",
        "Python PyYAML",
        if yaml_available { "ok" } else { "missing" }
    );
    if !yaml_available {
        missing.push("Python PyYAML");
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

fn ready() -> TaskResult {
    println!("Automexia local readiness: tools, policy, tests, build, and smoke");
    doctor()?;
    run_python("tools/ci/validate_repository.py")?;
    ci()?;
    run(
        "cargo",
        &[
            "deny",
            "--locked",
            "--color",
            "never",
            "check",
            "--hide-inclusion-graph",
        ],
    )?;
    build_debug_app()?;
    smoke_debug_app()?;
    println!("PASS: Automexia is locally ready to run and submit");
    Ok(())
}

fn dev(app_args: &[String]) -> TaskResult {
    ready()?;
    launch_debug_app(app_args)
}

fn run_app(app_args: &[String]) -> TaskResult {
    build_debug_app()?;
    smoke_debug_app()?;
    launch_debug_app(app_args)
}

fn build_debug_app() -> TaskResult {
    let identity = product_identity()?;
    run(
        "cargo",
        &[
            "build",
            "--locked",
            "-p",
            &identity.package_name,
            "--bin",
            &identity.executable,
        ],
    )
}

fn debug_binary(identity: &ProductIdentity) -> PathBuf {
    let binary = if cfg!(windows) {
        format!("{}.exe", identity.executable)
    } else {
        identity.executable.clone()
    };
    root().join("target").join("debug").join(binary)
}

fn smoke_debug_app() -> TaskResult {
    let identity = product_identity()?;
    let binary = debug_binary(&identity);
    require(
        binary.is_file(),
        &format!("debug executable is missing: {}", binary.display()),
    )?;
    println!("+ {} --version", binary.display());
    let output = Command::new(&binary)
        .arg("--version")
        .output()
        .map_err(|error| format!("could not smoke {}: {error}", binary.display()))?;
    require(
        output.status.success(),
        &format!(
            "{} --version exited with {}",
            identity.executable, output.status
        ),
    )?;
    let reported = String::from_utf8_lossy(&output.stdout);
    let expected = format!("{} {}", identity.executable, identity.version);
    require(
        reported.trim() == expected,
        &format!(
            "{} --version reported {:?}; expected {expected:?}",
            identity.executable,
            reported.trim()
        ),
    )?;
    println!("PASS: debug executable reports {expected}");
    Ok(())
}

fn launch_debug_app(app_args: &[String]) -> TaskResult {
    let identity = product_identity()?;
    let binary = debug_binary(&identity);
    require(
        binary.is_file(),
        &format!("debug executable is missing: {}", binary.display()),
    )?;
    println!(
        "+ {}{}",
        binary.display(),
        if app_args.is_empty() {
            String::new()
        } else {
            format!(" {}", app_args.join(" "))
        }
    );
    let child = Command::new(&binary)
        .args(app_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not launch {}: {error}", binary.display()))?;
    println!(
        "PASS: launched {} as process {}; Cargo is free for the next command",
        identity.executable,
        child.id()
    );
    Ok(())
}

fn check() -> TaskResult {
    verify_all()?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run_quiet("cargo", &["metadata", "--locked", "--format-version", "1"])?;
    run(
        "cargo",
        &["check", "--workspace", "--all-targets", "--locked"],
    )
}

fn verify_all() -> TaskResult {
    verify_identity()?;
    verify_provenance()?;
    verify_architecture()?;
    package_check()
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
    run_summarized(
        "cargo",
        &["test", "--workspace", "--locked"],
        "workspace unit, integration, and documentation tests passed",
    )
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

fn run_quiet(program: &str, args: &[&str]) -> TaskResult {
    println!("+ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .current_dir(root())
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn run_summarized(program: &str, args: &[&str], success_message: &str) -> TaskResult {
    println!("+ {program} {}", args.join(" "));
    let output = Command::new(program)
        .args(args)
        .current_dir(root())
        .output()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    if output.status.success() {
        println!("PASS: {success_message}");
        return Ok(());
    }

    // Successful test output is intentionally summarized, but failures retain
    // the complete harness and compiler diagnostics needed for investigation.
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Err(format!("{program} exited with {}", output.status))
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

fn product_identity() -> TaskResult<ProductIdentity> {
    let manifest_text = read(&root().join("Cargo.toml"))?;
    let manifest: toml::Value = toml::from_str(&manifest_text)
        .map_err(|error| format!("invalid workspace Cargo.toml: {error}"))?;
    let workspace = manifest
        .get("workspace")
        .ok_or("Cargo.toml is missing [workspace]")?;
    let version = workspace
        .get("package")
        .and_then(|value| value.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or("Cargo.toml is missing workspace.package.version")?
        .to_owned();
    let product = workspace
        .get("metadata")
        .and_then(|value| value.get("automexia"))
        .ok_or("Cargo.toml is missing workspace.metadata.automexia")?;
    let field = |name: &str| -> TaskResult<String> {
        product
            .get(name)
            .and_then(toml::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("workspace.metadata.automexia.{name} is missing"))
    };
    Ok(ProductIdentity {
        version,
        package_name: field("package-name")?,
        frontend_path: field("frontend-path")?,
        product_name: field("product-name")?,
        executable: field("executable")?,
        application_id: field("application-id")?,
        url_scheme: field("url-scheme")?,
        wm_class: field("wm-class")?,
        term_program: field("term-program")?,
        config_home_environment: field("config-home-environment")?,
        log_level_environment: field("log-level-environment")?,
        shell_integration_environment: field("shell-integration-environment")?,
    })
}

fn verify_architecture() -> TaskResult {
    let identity = product_identity()?;
    let metadata = metadata()?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata omitted packages")?;
    let frontend = packages
        .iter()
        .find(|package| package["name"].as_str() == Some(identity.package_name.as_str()))
        .ok_or("Automexia frontend package is missing")?;
    let manifest = frontend["manifest_path"]
        .as_str()
        .ok_or("frontend manifest path is missing")?
        .replace('\\', "/");
    require(
        manifest.ends_with(&format!("{}/Cargo.toml", identity.frontend_path)),
        "frontend package is outside apps/automexia-terminal",
    )?;

    let app = root().join(&identity.frontend_path);
    for path in files_under(&app.join("src/renderer"))? {
        if path.extension().and_then(OsStr::to_str) != Some("rs") {
            continue;
        }
        let renderer = read(&path)?;
        for forbidden in ["std::process::Command", "TcpStream", "UdpSocket", "reqwest"] {
            require(
                !renderer.contains(forbidden),
                &format!(
                    "{} performs extension I/O through {forbidden}",
                    display_relative(&path)
                ),
            )?;
        }
    }
    let runtime = read(&app.join("src/automexia/runtime.rs"))?;
    require(
        runtime.contains("sync_channel") && runtime.contains("try_send"),
        "extension runtime does not expose a bounded non-blocking worker queue",
    )?;
    require(
        runtime.contains("MAX_SESSION_TITLE_BYTES")
            && runtime.contains("shutdown_background_services")
            && runtime.contains("handle.is_finished()"),
        "extension worker lacks input bounds, restart detection, or joined shutdown",
    )?;
    let renderable = read(&app.join("src/context/renderable.rs"))?;
    let renderer = read(&app.join("src/renderer/mod.rs"))?;
    for field in [
        "current_directory",
        "terminal_title",
        "shell_integration",
        "shell_prompt_active",
    ] {
        require(
            renderable.contains(&format!("pub {field}:"))
                && renderer.contains(&format!("renderable_content.{field} =")),
            &format!("shell/prompt readiness metadata {field} is not snapshotted"),
        )?;
    }
    let devops_manifest = read(&app.join("src/automexia/builtins/devops/mod.rs"))?;
    for capability in [
        "Capability::FilesystemRead",
        "Capability::EnvironmentRead",
        "Capability::TerminalOutputRead",
        "Capability::UiOverlay",
    ] {
        require(
            devops_manifest.contains(capability),
            &format!("DevOps manifest is missing {capability}"),
        )?;
    }
    for excessive in [
        "Capability::ProcessSpawn",
        "Capability::Network",
        "Capability::Clipboard",
    ] {
        require(
            !devops_manifest.contains(excessive),
            &format!("DevOps manifest declares excessive privilege {excessive}"),
        )?;
    }

    for engine in ["rio-vt", "teletypewriter", "sugarloaf", "rio-window"] {
        let package = packages
            .iter()
            .find(|package| package["name"] == engine)
            .ok_or_else(|| format!("engine package {engine} is missing"))?;
        let depends_on_frontend =
            package["dependencies"]
                .as_array()
                .is_some_and(|dependencies| {
                    dependencies.iter().any(|dependency| {
                        dependency["name"].as_str()
                            == Some(identity.package_name.as_str())
                    })
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
    let identity = product_identity()?;
    let required = vec![
        ("Cargo.toml", format!("version = \"{}\"", identity.version)),
        ("Cargo.toml", identity.application_id.clone()),
        ("README.md", identity.product_name.clone()),
        (
            "packaging/linux/automexia-terminal.desktop",
            format!("Exec={}", identity.executable),
        ),
        (
            "packaging/linux/automexia-terminal.desktop",
            format!("StartupWMClass={}", identity.wm_class),
        ),
        (
            "packaging/linux/automexia-terminal.desktop",
            format!("x-scheme-handler/{}", identity.url_scheme),
        ),
        (
            "packaging/macos/Info.plist",
            identity.application_id.clone(),
        ),
        (
            "packaging/macos/Info.plist",
            "<string>@AUTOMEXIA_VERSION@</string>".to_owned(),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("TERM_PROGRAM: &str = \"{}\"", identity.term_program),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("PRODUCT_NAME: &str = \"{}\"", identity.product_name),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("EXECUTABLE_NAME: &str = \"{}\"", identity.executable),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("APPLICATION_ID: &str = \"{}\"", identity.application_id),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("WM_CLASS: &str = \"{}\"", identity.wm_class),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!("URL_SCHEME: &str = \"{}\"", identity.url_scheme),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!(
                "CONFIG_HOME_ENV: &str = \"{}\"",
                identity.config_home_environment
            ),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!(
                "LOG_LEVEL_ENV: &str = \"{}\"",
                identity.log_level_environment
            ),
        ),
        (
            "rio-backend/src/config/product.rs",
            format!(
                "SHELL_INTEGRATION_ENV: &str = \"{}\"",
                identity.shell_integration_environment
            ),
        ),
        (
            "packaging/windows/automexia.wxs",
            "AutomexiaContextMenuComponents".to_owned(),
        ),
    ];
    for (path, needle) in required {
        let content = read(&root.join(path))?;
        require(
            content.contains(&needle),
            &format!("{path} is missing canonical identity {needle:?}"),
        )?;
    }

    let scopes = [
        root.join("apps/automexia-terminal"),
        root.join("packaging"),
        root.join("shell-integration"),
        root.join("sugarloaf"),
        root.join("teletypewriter"),
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
        "RIO_VULKAN_",
        "TERM=rio",
        "xterm-rio",
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
    require(
        notice.contains("THIRD_PARTY_NOTICES.md")
            && root.join("THIRD_PARTY_NOTICES.md").is_file(),
        "NOTICE.md is missing the vendored-component license index",
    )?;
    run_python("tools/ci/check_vendored_licenses.py")?;

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
    let identity = product_identity()?;
    for required in [
        "apps/automexia-terminal/Cargo.toml",
        "apps/automexia-terminal/build.rs",
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
    let frontend_manifest_text =
        read(&root.join(&identity.frontend_path).join("Cargo.toml"))?;
    let frontend_manifest: toml::Value = toml::from_str(&frontend_manifest_text)
        .map_err(|error| format!("invalid frontend Cargo.toml: {error}"))?;
    let packager = frontend_manifest
        .get("package")
        .and_then(|value| value.get("metadata"))
        .and_then(|value| value.get("packager"))
        .ok_or("frontend Cargo.toml is missing package.metadata.packager")?;
    require(
        packager.get("product-name").and_then(toml::Value::as_str)
            == Some(identity.product_name.as_str()),
        "cargo-packager product name is not canonical",
    )?;
    require(
        packager.get("identifier").and_then(toml::Value::as_str)
            == Some(identity.application_id.as_str()),
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
    let windows_resources = read(&root.join("apps/automexia-terminal/build.rs"))?;
    require(
        windows_resources.contains("automexia-terminal.ico")
            && windows_resources.contains("Automexia Terminal")
            && windows_resources.contains("automexia.exe"),
        "Windows executable resources do not use the canonical Automexia identity",
    )?;
    let window_adapter =
        read(&root.join("apps/automexia-terminal/src/router/window.rs"))?;
    require(
        window_adapter.contains("Icon::from_resource(1, None)")
            && window_adapter.contains("automexia-terminal-256.png"),
        "platform window adapters do not load the Automexia application icon",
    )?;
    let terminfo = read(&root.join("packaging/linux/automexia.terminfo"))?;
    require(
        terminfo.contains("automexia|") && terminfo.contains("xterm-automexia|"),
        "terminfo must define both automexia and xterm-automexia",
    )?;
    let linux_package = read(&root.join("packaging/linux/nfpm.yaml"))?;
    for size in [16, 32, 48, 64, 128, 256, 512] {
        require(
            linux_package.contains(&format!(
                "/usr/share/icons/hicolor/{size}x{size}/apps/automexia-terminal.png"
            )),
            &format!("Linux package is missing the {size}px application icon"),
        )?;
    }
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
        require(
            release.get("placeholder").and_then(toml::Value::as_bool).is_some(),
            "non-release brand manifest must explicitly declare whether assets are placeholders",
        )?;
        for path in [
            "assets/brand/automexia-terminal-source-512.png",
            "assets/brand/automexia-terminal-1024.png",
            "assets/brand/automexia-terminal.ico",
            "assets/brand/automexia-terminal.icns",
        ] {
            require(
                root.join(path).is_file(),
                &format!("non-release packaging asset is missing: {path}"),
            )?;
        }
        for size in [16, 32, 48, 64, 128, 256, 512] {
            let path = format!("assets/brand/png/automexia-terminal-{size}.png");
            require(
                root.join(&path).is_file(),
                &format!("non-release Linux icon is missing: {path}"),
            )?;
        }
        println!("PASS: package metadata and supplied raster assets verified (stable release remains blocked on final brand approval)");
    }
    Ok(())
}

fn package_target(target: &str) -> TaskResult {
    package_check()?;
    let identity = product_identity()?;
    let skip_build = env::var_os("AUTOMEXIA_PACKAGE_SKIP_BUILD").is_some();
    if !skip_build {
        run(
            "cargo",
            &[
                "build",
                "--release",
                "--locked",
                "-p",
                &identity.package_name,
                "--target",
                target,
            ],
        )?;
    }

    let binary_name = if target.contains("windows") {
        format!("{}.exe", identity.executable)
    } else {
        identity.executable.clone()
    };
    let binary = root()
        .join("target")
        .join(target)
        .join("release")
        .join(&binary_name);
    require(
        binary.is_file(),
        &format!("release binary is missing: {}", binary.display()),
    )?;
    let output = root().join("target/package").join(target);
    fs::create_dir_all(&output)
        .map_err(|error| format!("could not create {}: {error}", output.display()))?;

    if target.contains("windows") {
        require(
            cfg!(windows),
            "Windows packages must be produced on Windows",
        )?;
        run(
            "cargo",
            &[
                "packager",
                "--manifest-path",
                "apps/automexia-terminal/Cargo.toml",
                "--release",
                "--binaries-dir",
                binary
                    .parent()
                    .and_then(Path::to_str)
                    .ok_or("binary directory is not UTF-8")?,
                "--out-dir",
                output.to_str().ok_or("package path is not UTF-8")?,
                "--target",
                target,
                "--formats",
                "wix",
            ],
        )?;
        portable_archive(&identity, target, &binary, &output, "zip")?;
    } else if target.contains("apple-darwin") {
        require(
            cfg!(target_os = "macos"),
            "macOS packages must be produced on macOS",
        )?;
        run(
            "cargo",
            &[
                "packager",
                "--manifest-path",
                "apps/automexia-terminal/Cargo.toml",
                "--release",
                "--binaries-dir",
                binary
                    .parent()
                    .and_then(Path::to_str)
                    .ok_or("binary directory is not UTF-8")?,
                "--out-dir",
                output.to_str().ok_or("package path is not UTF-8")?,
                "--target",
                target,
                "--formats",
                "app,dmg",
            ],
        )?;
    } else if target.contains("linux") {
        require(
            cfg!(target_os = "linux"),
            "Linux packages must be produced on Linux",
        )?;
        package_linux(&identity, target, &binary, &output)?;
    } else {
        return Err(format!("unsupported package target: {target}"));
    }
    println!("PASS: packages staged in {}", output.display());
    Ok(())
}

fn portable_archive(
    identity: &ProductIdentity,
    target: &str,
    binary: &Path,
    output: &Path,
    extension: &str,
) -> TaskResult {
    let staging = output.join("portable");
    fs::create_dir_all(&staging)
        .map_err(|error| format!("could not create {}: {error}", staging.display()))?;
    let binary_name = binary.file_name().ok_or("release binary has no filename")?;
    fs::copy(binary, staging.join(binary_name))
        .map_err(|error| format!("could not stage {}: {error}", binary.display()))?;
    for document in [
        "LICENSE",
        "NOTICE.md",
        "THIRD_PARTY_NOTICES.md",
        "README.md",
    ] {
        fs::copy(root().join(document), staging.join(document))
            .map_err(|error| format!("could not stage {document}: {error}"))?;
    }
    let archive = output.join(format!(
        "{}-{}-{target}.{extension}",
        identity.package_name, identity.version
    ));
    let flag = if extension == "zip" { "-acf" } else { "-czf" };
    let mut command = Command::new("tar");
    command
        .args([flag])
        .arg(&archive)
        .args(["-C"])
        .arg(&staging)
        .arg(".")
        .current_dir(root());
    run_command(command, "portable archive")
}

fn package_linux(
    identity: &ProductIdentity,
    target: &str,
    binary: &Path,
    output: &Path,
) -> TaskResult {
    let manpage = output.join("automexia.1");
    let source = File::open(root().join("packaging/linux/automexia.1.scd"))
        .map_err(|error| format!("could not open manpage source: {error}"))?;
    let rendered = File::create(&manpage)
        .map_err(|error| format!("could not create {}: {error}", manpage.display()))?;
    let mut scdoc = Command::new("scdoc");
    scdoc
        .stdin(Stdio::from(source))
        .stdout(Stdio::from(rendered))
        .current_dir(root());
    run_command(scdoc, "scdoc")?;

    let terminfo = output.join("terminfo");
    fs::create_dir_all(&terminfo)
        .map_err(|error| format!("could not create {}: {error}", terminfo.display()))?;
    let mut tic = Command::new("tic");
    tic.args(["-x", "-o"])
        .arg(&terminfo)
        .arg("packaging/linux/automexia.terminfo")
        .current_dir(root());
    run_command(tic, "terminfo compiler")?;

    let arch = if target.starts_with("x86_64-") {
        "amd64"
    } else if target.starts_with("aarch64-") {
        "arm64"
    } else {
        return Err(format!("unsupported Linux package architecture: {target}"));
    };
    for format in ["deb", "rpm"] {
        let mut nfpm = Command::new("nfpm");
        nfpm.args([
            "package",
            "--config",
            "packaging/linux/nfpm.yaml",
            "--packager",
            format,
            "--target",
        ])
        .arg(output)
        .env("NFPM_ARCH", arch)
        .env("AUTOMEXIA_VERSION", &identity.version)
        .env("AUTOMEXIA_BINARY", binary)
        .env("AUTOMEXIA_MANPAGE", &manpage)
        .env("AUTOMEXIA_TERMINFO_ROOT", &terminfo)
        .current_dir(root());
        run_command(nfpm, &format!("nFPM {format}"))?;
    }
    portable_archive(identity, target, binary, output, "tar.gz")
}

fn run_command(mut command: Command, label: &str) -> TaskResult {
    println!("+ {label}");
    let status = command
        .status()
        .map_err(|error| format!("could not start {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} exited with {status}"))
    }
}

fn release(version: &str) -> TaskResult {
    let identity = product_identity()?;
    require(
        version == identity.version,
        &format!("release version must match workspace {}", identity.version),
    )?;
    if assemble_changelog(version)? {
        return Err(
            "CHANGELOG.md was assembled and consumed fragments; review and commit those release-preparation changes before tagging"
                .into(),
        );
    }
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
    ci()?;
    println!("PASS: release {version} preconditions verified; publication is CI-only");
    Ok(())
}

fn assemble_changelog(version: &str) -> TaskResult<bool> {
    const CATEGORIES: [&str; 6] = [
        "Added",
        "Changed",
        "Fixed",
        "Security",
        "Deprecated",
        "Removed",
    ];
    let directory = root().join("changes");
    let mut fragments = fs::read_dir(&directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(OsStr::to_str) == Some("md")
                && path.file_name().and_then(OsStr::to_str) != Some("README.md")
        })
        .collect::<Vec<_>>();
    fragments.sort();

    let changelog_path = root().join("CHANGELOG.md");
    let changelog = read(&changelog_path)?;
    let version_heading = format!("## [{version}]");
    if fragments.is_empty() {
        require(
            changelog.contains(&version_heading),
            "release has neither changelog fragments nor an assembled version section",
        )?;
        return Ok(false);
    }
    require(
        !changelog.contains(&version_heading),
        "release version is already assembled but unconsumed fragments remain",
    )?;

    let marker = "## [Unreleased]\n";
    let marker_index = changelog
        .find(marker)
        .ok_or("CHANGELOG.md is missing the Unreleased heading")?;
    let after_marker = marker_index + marker.len();
    let next_section = changelog[after_marker..]
        .find("\n## [")
        .map(|offset| after_marker + offset)
        .unwrap_or(changelog.len());
    require(
        changelog[after_marker..next_section].trim().is_empty(),
        "move Unreleased changelog text into reviewed fragments before assembly",
    )?;

    let mut grouped: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for path in &fragments {
        let content = read(path)?;
        let mut lines = content.lines();
        let heading = lines.next().unwrap_or_default().trim();
        let category = CATEGORIES
            .iter()
            .copied()
            .find(|category| *category == heading)
            .ok_or_else(|| {
                format!(
                    "{} has invalid changelog category {heading:?}",
                    display_relative(path)
                )
            })?;
        let body = lines.collect::<Vec<_>>().join("\n").trim().to_owned();
        require(
            body.starts_with("- "),
            &format!(
                "{} must contain Markdown bullet entries",
                display_relative(path)
            ),
        )?;
        grouped.entry(category).or_default().push(body);
    }
    let date = env::var("AUTOMEXIA_RELEASE_DATE").unwrap_or_else(|_| {
        Command::new("git")
            .args(["show", "-s", "--format=%cs", "HEAD"])
            .current_dir(root())
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "DATE-REQUIRED".to_owned())
    });
    require(
        date.len() == 10 && date.as_bytes()[4] == b'-' && date.as_bytes()[7] == b'-',
        "AUTOMEXIA_RELEASE_DATE must use YYYY-MM-DD",
    )?;

    let mut release_notes = format!("\n## [{version}] - {date}\n");
    for category in CATEGORIES {
        let Some(entries) = grouped.get(category) else {
            continue;
        };
        release_notes.push_str(&format!("\n### {category}\n\n"));
        release_notes.push_str(&entries.join("\n"));
        release_notes.push('\n');
    }
    let assembled = format!(
        "{}{}{}{}",
        &changelog[..after_marker],
        release_notes,
        if changelog[next_section..].starts_with('\n') {
            ""
        } else {
            "\n"
        },
        &changelog[next_section..]
    );
    fs::write(&changelog_path, assembled).map_err(|error| {
        format!("could not write {}: {error}", changelog_path.display())
    })?;
    for path in fragments {
        fs::remove_file(&path)
            .map_err(|error| format!("could not consume {}: {error}", path.display()))?;
    }
    println!(
        "PASS: assembled {version} changelog; commit before creating the release tag"
    );
    Ok(true)
}

fn run_python(script: &str) -> TaskResult {
    let program =
        python_program().ok_or("Python 3 is required for repository policy checks")?;
    run(program, &[script])
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
        assert!(usage().contains("dev [-- APP_ARGS...]"));
        assert!(usage().contains("ready"));
        assert!(usage().contains("run [-- APP_ARGS...]"));
        assert!(usage().contains("verify architecture"));
        assert!(usage().contains("test conformance"));
        assert!(usage().contains("release --version"));
        assert!(usage().contains("verify all"));
    }

    #[test]
    fn canonical_identity_matches_release_contract() {
        let identity = product_identity().unwrap();
        assert_eq!(identity.product_name, "Automexia Terminal");
        assert_eq!(
            identity.application_id,
            "io.github.AmjedAllaya.AutomexiaTerminal"
        );
        assert_eq!(RIO_BASE_SHA.len(), 40);
    }
}
