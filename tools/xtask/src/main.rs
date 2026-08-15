use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

type TaskResult<T = ()> = Result<T, String>;

const RIO_BASE_SHA: &str = "7d595af583f6ef1ea6036a66b367ba1e5a84d4a2";
const GIB: u64 = 1024 * 1024 * 1024;
const VERIFICATION_TARGET_PREFIX: &str = "automexia-verification-v1-";
const RUNTIME_TARGET_NAME: &str = "automexia-runtime";
const DEFAULT_VERIFY_MIN_FREE_GIB: u64 = 12;
const DEFAULT_BUILD_MIN_FREE_GIB: u64 = 4;
const DEFAULT_TARGET_WARN_GIB: u64 = 12;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaunchMode {
    Verified,
    Incremental,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LaunchPhase {
    Ready,
    Build,
    Smoke,
    InstallShellIntegration,
    Launch,
}

impl LaunchPhase {
    fn description(self) -> &'static str {
        match self {
            Self::Ready => "complete verification gate",
            Self::Build => "incremental application build",
            Self::Smoke => "executable identity smoke test",
            Self::InstallShellIntegration => "shell integration provisioning",
            Self::Launch => "Automexia process launch",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostShellPlatform {
    Windows,
    Unix,
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
        [command] if command == "storage" => storage_report(),
        [command] if command == "check" => check(),
        [command] if command == "ci" => ci(),
        [command, flag] if command == "qa" && flag == "--full" => qa(false),
        [command, first, second]
            if command == "qa" && first == "--full" && second == "--bundle" =>
        {
            qa(true)
        }
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
        [command, scope] if command == "test" && scope == "resize-stress" => {
            test_resize_stress(false)
        }
        [command, scope] if command == "test" && scope == "image-rendering" => {
            test_image_rendering(false)
        }
        [command, scope, flag]
            if command == "test"
                && scope == "image-rendering"
                && flag == "--native-gui" =>
        {
            test_image_rendering(true)
        }
        [command, scope] if command == "test" && scope == "image-decoder-fuzz" => {
            test_image_decoder_fuzz(120)
        }
        [command, scope, flag, seconds]
            if command == "test"
                && scope == "image-decoder-fuzz"
                && flag == "--seconds" =>
        {
            test_image_decoder_fuzz(parse_fuzz_seconds(seconds)?)
        }
        [command, scope] if command == "test" && scope == "session-clone" => {
            test_session_clone(None)
        }
        [command, scope, flag]
            if command == "test"
                && scope == "session-clone"
                && flag == "--native-windows" =>
        {
            test_session_clone(Some("windows"))
        }
        [command, scope, flag]
            if command == "test"
                && scope == "session-clone"
                && flag == "--native-wsl" =>
        {
            test_session_clone(Some("wsl"))
        }
        [command, scope, flag]
            if command == "test"
                && scope == "resize-stress"
                && flag == "--native-gui" =>
        {
            test_resize_stress(true)
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
    "usage: cargo xtask <dev [-- APP_ARGS...]|ready|run [-- APP_ARGS...]|doctor|storage|check|ci|qa --full [--bundle]|verify architecture|verify identity|verify provenance|verify all|test conformance|test resize-stress [--native-gui]|test image-rendering [--native-gui]|test image-decoder-fuzz [--seconds N]|test session-clone [--native-windows|--native-wsl]|package --check|package --target TARGET|release --version VERSION>".into()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn wsl_windows_drive(path: &Path, running_under_wsl: bool) -> Option<char> {
    if !running_under_wsl {
        return None;
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mounted = normalized.strip_prefix("/mnt/")?;
    let mut characters = mounted.chars();
    let drive = characters.next()?;
    if !drive.is_ascii_alphabetic() || characters.next().is_some_and(|next| next != '/') {
        return None;
    }
    Some(drive.to_ascii_uppercase())
}

#[cfg(target_os = "linux")]
fn running_under_wsl() -> bool {
    env::var_os("WSL_DISTRO_NAME").is_some()
        || env::var_os("WSL_INTEROP").is_some()
        || fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"))
}

#[cfg(not(target_os = "linux"))]
fn running_under_wsl() -> bool {
    false
}

fn workspace_io_issue() -> Option<String> {
    let is_wsl = running_under_wsl();
    let source_drive = wsl_windows_drive(&root(), is_wsl);
    let target = cargo_target_dir();
    let target_drive = wsl_windows_drive(&target, is_wsl);
    if source_drive.is_none() && target_drive.is_none() {
        return None;
    }

    let mut locations = Vec::new();
    if let Some(drive) = source_drive {
        locations.push(format!("source on /mnt/{}", drive.to_ascii_lowercase()));
    }
    if let Some(drive) = target_drive {
        locations.push(format!(
            "Cargo target on /mnt/{}",
            drive.to_ascii_lowercase()
        ));
    }
    Some(locations.join(" and "))
}

fn report_workspace_io_health() {
    if let Some(issue) = workspace_io_issue() {
        println!("workspace I/O      advisory/slow ({issue})");
        println!(
            "WSL workflow        keep Windows/MSVC builds in this Windows checkout; clone the same branch below ~/src/automexia-terminal for Linux Cargo work"
        );
    } else if running_under_wsl() {
        println!("workspace I/O      WSL-native/ok");
    } else {
        println!("workspace I/O      host-native/ok");
    }
}

fn require_native_wsl_workspace(purpose: &str) -> TaskResult {
    let Some(issue) = workspace_io_issue() else {
        return Ok(());
    };
    if environment_truthy("AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT") {
        println!(
            "workspace I/O      override/slow ({issue}; AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT is set)"
        );
        return Ok(());
    }
    Err(format!(
        "{purpose} is I/O-heavy, but {issue}. Linux Cargo builds under /mnt/c or /mnt/d are substantially slower. Keep this checkout for Windows/MSVC commands in PowerShell and clone the same Git branch under ~/src/automexia-terminal for WSL/Linux commands. Set AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT=1 only for a deliberate one-off diagnostic"
    ))
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn command_on_path(program: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|directory| {
        #[cfg(target_os = "windows")]
        {
            let path_ext = env::var_os("PATHEXT")
                .unwrap_or_else(|| std::ffi::OsString::from(".COM;.EXE;.BAT;.CMD"));
            path_ext.to_string_lossy().split(';').any(|extension| {
                directory.join(format!("{program}{extension}")).is_file()
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(directory.join(program)).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        }
    })
}

fn cargo_subcommand_available(subcommand: &str) -> bool {
    Command::new("cargo")
        .args([subcommand, "--version"])
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

#[cfg(target_os = "windows")]
#[derive(Debug, Eq, PartialEq)]
struct WindowsShellHealth {
    host_version: Option<String>,
    psreadline_version: Option<String>,
}

#[cfg(target_os = "windows")]
fn parse_windows_shell_health(output: &str) -> WindowsShellHealth {
    let value = |prefix: &str| {
        output
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix(prefix))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    WindowsShellHealth {
        host_version: value("host="),
        psreadline_version: value("psreadline="),
    }
}

#[cfg(target_os = "windows")]
fn windows_shell_history_is_legacy(health: &WindowsShellHealth) -> bool {
    health
        .host_version
        .as_deref()
        .is_some_and(|version| version.starts_with("5.1."))
        && health
            .psreadline_version
            .as_deref()
            .is_some_and(|version| version == "2.0" || version.starts_with("2.0."))
}

#[cfg(target_os = "windows")]
fn report_windows_shell_health() {
    let output = Command::new("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"Write-Output ('host=' + $PSVersionTable.PSVersion.ToString()); $module = Get-Module PSReadLine -ListAvailable | Sort-Object Version -Descending | Select-Object -First 1; if ($null -ne $module) { Write-Output ('psreadline=' + $module.Version.ToString()) }"#,
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let health =
                parse_windows_shell_health(&String::from_utf8_lossy(&output.stdout));
            println!(
                "PowerShell host    {}",
                health.host_version.as_deref().unwrap_or("unknown")
            );
            println!(
                "PSReadLine         {}",
                health.psreadline_version.as_deref().unwrap_or("not found")
            );
            if windows_shell_history_is_legacy(&health) {
                println!(
                    "shell history      advisory: Windows PowerShell 5.1 + PSReadLine 2.0 can add about 1 s to Up/Ctrl+R; prefer PowerShell 7 or a supported current stable PSReadLine"
                );
            }
        }
        Ok(output) => println!("PowerShell health  unavailable (exit {})", output.status),
        Err(error) => println!("PowerShell health  unavailable ({error})"),
    }

    println!(
        "PowerShell 7       {}",
        if command_available("pwsh") {
            "available"
        } else {
            "optional/missing"
        }
    );
}

fn doctor() -> TaskResult {
    #[cfg(target_os = "windows")]
    let required = ["cargo", "rustc", "rustfmt", "git", "cargo-deny"];
    #[cfg(target_os = "linux")]
    let required = [
        "cargo",
        "rustc",
        "rustfmt",
        "git",
        "cargo-deny",
        "bash",
        "zsh",
        "shellcheck",
        "glslangValidator",
    ];
    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    let required = [
        "cargo",
        "rustc",
        "rustfmt",
        "git",
        "cargo-deny",
        "bash",
        "zsh",
        "shellcheck",
    ];
    let qa_required = [("cargo-nextest", cargo_subcommand_available("nextest"))];
    let optional = [
        ("cargo-insta", cargo_subcommand_available("insta")),
        ("cargo-fuzz", cargo_subcommand_available("fuzz")),
        ("cargo-llvm-cov", cargo_subcommand_available("llvm-cov")),
        ("cargo-packager", cargo_subcommand_available("packager")),
        ("dotnet (ARM MSI)", command_available("dotnet")),
        ("nfpm", command_on_path("nfpm")),
        ("scdoc", command_on_path("scdoc")),
        ("gzip", command_on_path("gzip")),
        ("tic", command_on_path("tic")),
    ];
    let mut missing = Vec::new();
    let mut missing_qa = Vec::new();
    for program in required {
        let available = command_available(program);
        println!("{program:<18} {}", if available { "ok" } else { "missing" });
        if !available {
            missing.push(program);
        }
    }
    for (program, available) in qa_required {
        println!(
            "{program:<18} {}",
            if available {
                "qa-required/ok"
            } else {
                "qa-required/missing"
            }
        );
        if !available {
            missing_qa.push(program);
        }
    }
    for (program, available) in optional {
        println!(
            "{program:<18} {}",
            if available {
                "optional/ok"
            } else {
                "optional/missing"
            }
        );
    }
    if missing_qa.contains(&"cargo-nextest") {
        println!(
            "QA install         cargo install cargo-nextest --version 0.9.137 --locked"
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
    report_windows_shell_health();
    #[cfg(target_os = "windows")]
    println!("platform           Windows: Visual Studio Build Tools, cargo-packager (x64), and the .NET SDK for repository-pinned WiX 5 (ARM64) are required for MSI builds");
    #[cfg(target_os = "macos")]
    println!("platform           macOS: Xcode CLI tools and Apple signing credentials are required for releases");
    #[cfg(target_os = "linux")]
    println!("platform           Linux: install glslang-tools plus X11, Wayland, fontconfig, and audio development packages");
    report_workspace_io_health();
    if let Err(error) = storage_health_summary() {
        println!("storage            unavailable ({error})");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("missing required tools: {}", missing.join(", ")))
    }
}

fn storage_report() -> TaskResult {
    let target = canonical_target_dir()?;
    let used = directory_size(&target)?;
    let available = fs2::available_space(&target).map_err(|error| {
        format!(
            "could not query free space for {}: {error}",
            target.display()
        )
    })?;
    let total = fs2::total_space(&target).map_err(|error| {
        format!(
            "could not query filesystem size for {}: {error}",
            target.display()
        )
    })?;
    let warn_bytes =
        configured_gib("AUTOMEXIA_TARGET_WARN_GIB", DEFAULT_TARGET_WARN_GIB)? * GIB;

    println!("Automexia build storage");
    println!("target             {}", target.display());
    println!("target used        {}", format_bytes(used));
    println!("filesystem free    {}", format_bytes(available));
    println!("filesystem total   {}", format_bytes(total));
    println!("warning threshold  {}", format_bytes(warn_bytes));

    let mut children = Vec::new();
    for entry in fs::read_dir(&target)
        .map_err(|error| format!("could not read {}: {error}", target.display()))?
    {
        let entry = entry.map_err(|error| format!("target entry failed: {error}"))?;
        let size = if entry
            .file_type()
            .map_err(|error| {
                format!("could not inspect {}: {error}", entry.path().display())
            })?
            .is_dir()
        {
            directory_size(&entry.path())?
        } else {
            entry
                .metadata()
                .map_err(|error| {
                    format!("could not inspect {}: {error}", entry.path().display())
                })?
                .len()
        };
        children.push((entry.file_name(), size));
    }
    children.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    for (name, size) in children.into_iter().take(12) {
        println!("  {:<22} {}", name.to_string_lossy(), format_bytes(size));
    }

    if used >= warn_bytes {
        println!(
            "WARNING: the persistent target exceeds its storage threshold; close Automexia and run `cargo purge`"
        );
    } else {
        println!("PASS: persistent build storage is below its warning threshold");
    }
    Ok(())
}

fn storage_health_summary() -> TaskResult {
    let target = canonical_target_dir()?;
    let used = directory_size(&target)?;
    let available = fs2::available_space(&target).map_err(|error| {
        format!(
            "could not query free space for {}: {error}",
            target.display()
        )
    })?;
    let warn =
        configured_gib("AUTOMEXIA_TARGET_WARN_GIB", DEFAULT_TARGET_WARN_GIB)? * GIB;
    println!(
        "storage            {} used, {} free ({})",
        format_bytes(used),
        format_bytes(available),
        target.display()
    );
    if used >= warn {
        println!(
            "storage warning    persistent target exceeds {}; close Automexia and run `cargo purge`",
            format_bytes(warn)
        );
    }
    Ok(())
}

fn directory_size(path: &Path) -> TaskResult<u64> {
    if !path.exists() {
        return Ok(0);
    }
    require(
        !path_is_reparse_point(path)?,
        &format!(
            "refusing to traverse symlink/reparse-point directory {}",
            path.display()
        ),
    )?;

    let mut total = 0u64;
    let mut pending = vec![path.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("could not read {}: {error}", directory.display()))?
        {
            let entry =
                entry.map_err(|error| format!("directory entry failed: {error}"))?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
                format!("could not inspect {}: {error}", entry.path().display())
            })?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                    continue;
                }
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MiB", bytes as f64 / (1024 * 1024) as f64)
    } else if bytes >= 1024 {
        format!("{:.2} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn configured_gib(variable: &str, default: u64) -> TaskResult<u64> {
    match env::var(variable) {
        Ok(value) => value.parse::<u64>().map_err(|_| {
            format!("{variable} must be a non-negative integer number of GiB")
        }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("could not read {variable}: {error}")),
    }
}

fn ensure_free_space(path: &Path, required_gib: u64, purpose: &str) -> TaskResult {
    fs::create_dir_all(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let available = fs2::available_space(path).map_err(|error| {
        format!("could not query free space for {}: {error}", path.display())
    })?;
    let required = required_gib.saturating_mul(GIB);
    require(
        available >= required,
        &format!(
            "{purpose} requires at least {}; only {} is free on the target filesystem. Close Automexia and run `cargo purge`, free disk space, or select a larger CARGO_TARGET_DIR",
            format_bytes(required),
            format_bytes(available)
        ),
    )?;
    println!(
        "storage preflight  {} free for {purpose} (minimum {})",
        format_bytes(available),
        format_bytes(required)
    );
    Ok(())
}

struct VerificationTarget {
    parent: PathBuf,
    path: PathBuf,
    keep: bool,
    finished: bool,
}

impl VerificationTarget {
    fn prepare() -> TaskResult<Self> {
        require_native_wsl_workspace("the exhaustive verification gate")?;
        let parent = canonical_target_dir()?;
        ensure_free_space(
            &parent,
            configured_gib("AUTOMEXIA_VERIFY_MIN_FREE_GIB", DEFAULT_VERIFY_MIN_FREE_GIB)?,
            "the exhaustive isolated verification gate",
        )?;
        let generation = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?
            .as_nanos();
        let name = verification_target_name(std::process::id(), generation);
        let path = verified_target_child(&parent, &name)?;
        fs::create_dir(&path).map_err(|error| {
            format!(
                "could not create verification target {}: {error}",
                path.display()
            )
        })?;
        let keep = environment_truthy("AUTOMEXIA_KEEP_VERIFY_TARGET");
        println!("verification target {}", path.display());
        println!("incremental         disabled for verification artifacts");
        Ok(Self {
            parent,
            path,
            keep,
            finished: false,
        })
    }

    fn finish(&mut self) -> TaskResult {
        let used = directory_size(&self.path)?;
        if self.keep {
            println!(
                "verification target retained at {} ({}) because AUTOMEXIA_KEEP_VERIFY_TARGET is set",
                self.path.display(),
                format_bytes(used)
            );
            self.finished = true;
            return Ok(());
        }
        remove_verification_target(&self.parent, &self.path)?;
        println!(
            "PASS: removed isolated verification artifacts ({})",
            format_bytes(used)
        );
        self.finished = true;
        Ok(())
    }
}

impl Drop for VerificationTarget {
    fn drop(&mut self) {
        if !self.finished && !self.keep {
            let _ = remove_verification_target(&self.parent, &self.path);
        }
    }
}

fn remove_verification_target(parent: &Path, path: &Path) -> TaskResult {
    require(
        path.parent() == Some(parent)
            && path.file_name().is_some_and(is_verification_target_name),
        "refusing to remove a directory that is not a generated verification target",
    )?;
    if !path.exists() {
        return Ok(());
    }
    require(
        !path_is_reparse_point(path)?,
        "refusing to remove a symlink/reparse-point verification directory",
    )?;
    fs::remove_dir_all(path)
        .map_err(|error| format!("could not remove {}: {error}", path.display()))
}

fn verification_target_name(process_id: u32, generation: u128) -> String {
    format!("{VERIFICATION_TARGET_PREFIX}{process_id}-{generation}")
}

fn is_verification_target_name(name: &OsStr) -> bool {
    let Some(suffix) = name
        .to_str()
        .and_then(|name| name.strip_prefix(VERIFICATION_TARGET_PREFIX))
    else {
        return false;
    };
    let Some((process_id, generation)) = suffix.split_once('-') else {
        return false;
    };
    !process_id.is_empty()
        && !generation.is_empty()
        && process_id.bytes().all(|byte| byte.is_ascii_digit())
        && generation.bytes().all(|byte| byte.is_ascii_digit())
}

fn environment_truthy(variable: &str) -> bool {
    env::var(variable).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn with_verification_target<F>(task: F) -> TaskResult
where
    F: FnOnce(&Path) -> TaskResult,
{
    let mut target = VerificationTarget::prepare()?;
    let task_result = task(&target.path);
    let cleanup_result = target.finish();
    match (task_result, cleanup_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(task_error), Ok(())) => Err(task_error),
        (Ok(()), Err(cleanup_error)) => Err(cleanup_error),
        (Err(task_error), Err(cleanup_error)) => Err(format!(
            "{task_error}; verification cleanup also failed: {cleanup_error}"
        )),
    }
}

fn ready() -> TaskResult {
    println!(
        "Automexia local readiness: isolated policy/tests, persistent app build, and smoke"
    );
    complete_ci_gate()?;
    build_debug_app()?;
    smoke_debug_app()?;
    println!("PASS: Automexia is locally ready to run and submit");
    Ok(())
}

fn dev(app_args: &[String]) -> TaskResult {
    execute_launch_plan(LaunchMode::Verified, app_args)
}

fn run_app(app_args: &[String]) -> TaskResult {
    execute_launch_plan(LaunchMode::Incremental, app_args)
}

fn launch_plan(mode: LaunchMode) -> &'static [LaunchPhase] {
    match mode {
        LaunchMode::Verified => &[
            LaunchPhase::Ready,
            LaunchPhase::InstallShellIntegration,
            LaunchPhase::Launch,
        ],
        LaunchMode::Incremental => &[
            LaunchPhase::Build,
            LaunchPhase::Smoke,
            LaunchPhase::InstallShellIntegration,
            LaunchPhase::Launch,
        ],
    }
}

fn execute_launch_plan(mode: LaunchMode, app_args: &[String]) -> TaskResult {
    let plan = launch_plan(mode);
    println!(
        "Automexia {} launch workflow: {} phases; the window opens only after every preceding phase passes",
        match mode {
            LaunchMode::Verified => "verified",
            LaunchMode::Incremental => "incremental",
        },
        plan.len()
    );
    for (index, phase) in plan.iter().enumerate() {
        println!(
            "==> launch phase {}/{}: {}",
            index + 1,
            plan.len(),
            phase.description()
        );
        match phase {
            LaunchPhase::Ready => ready()?,
            LaunchPhase::Build => build_debug_app()?,
            LaunchPhase::Smoke => smoke_debug_app()?,
            LaunchPhase::InstallShellIntegration => install_shell_integration()?,
            LaunchPhase::Launch => launch_debug_app(app_args)?,
        }
    }
    Ok(())
}

fn shell_integration_command(
    platform: HostShellPlatform,
) -> (&'static str, &'static [&'static str]) {
    match platform {
        HostShellPlatform::Windows => (
            "powershell",
            &[
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "shell-integration/install-windows.ps1",
                "-Quiet",
            ],
        ),
        HostShellPlatform::Unix => {
            ("sh", &["shell-integration/install-unix.sh", "--quiet"])
        }
    }
}

fn install_shell_integration() -> TaskResult {
    let platform = if cfg!(windows) {
        HostShellPlatform::Windows
    } else {
        HostShellPlatform::Unix
    };
    let (program, args) = shell_integration_command(platform);
    println!("Preparing Automexia shell integration (source-aware and idempotent)");
    run(program, args)?;
    println!("PASS: shell integration is ready for this launch");
    Ok(())
}

fn build_debug_app() -> TaskResult {
    require_native_wsl_workspace("the persistent Automexia application build")?;
    let identity = product_identity()?;
    ensure_free_space(
        &cargo_target_dir(),
        configured_gib("AUTOMEXIA_BUILD_MIN_FREE_GIB", DEFAULT_BUILD_MIN_FREE_GIB)?,
        "the persistent Automexia application build",
    )?;
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
    cargo_target_dir().join("debug").join(binary)
}

fn release_binary_in(target_dir: &Path, target: &str, executable: &str) -> PathBuf {
    let binary = if target.contains("windows") {
        format!("{executable}.exe")
    } else {
        executable.to_owned()
    };
    target_dir.join(target).join("release").join(binary)
}

fn cargo_target_dir() -> PathBuf {
    // Cargo resolves a relative CARGO_TARGET_DIR from the invocation working
    // directory, which is inherited by every nested command we spawn.
    let invocation_directory = env::current_dir().unwrap_or_else(|_| root());
    resolve_target_dir(
        &invocation_directory,
        env::var_os("CARGO_TARGET_DIR").as_deref(),
    )
}

fn resolve_target_dir(
    invocation_directory: &Path,
    configured: Option<&OsStr>,
) -> PathBuf {
    match configured.map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        Some(path) => invocation_directory.join(path),
        None => root().join("target"),
    }
}

fn canonical_target_dir() -> TaskResult<PathBuf> {
    let target = cargo_target_dir();
    fs::create_dir_all(&target)
        .map_err(|error| format!("could not create {}: {error}", target.display()))?;
    target
        .canonicalize()
        .map(normalize_canonical_path)
        .map_err(|error| format!("could not resolve {}: {error}", target.display()))
}

#[cfg(target_os = "windows")]
fn normalize_canonical_path(path: PathBuf) -> PathBuf {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    // std::fs::canonicalize returns a verbatim path on Windows. Rust accepts
    // it, but MSVC link.exe can misparse object paths below a verbatim
    // CARGO_TARGET_DIR and report LNK1181 for a phantom `.obj` input.
    const VERBATIM: &[u16] = &[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    const VERBATIM_UNC: &[u16] = &[
        b'\\' as u16,
        b'\\' as u16,
        b'?' as u16,
        b'\\' as u16,
        b'U' as u16,
        b'N' as u16,
        b'C' as u16,
        b'\\' as u16,
    ];
    let wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.starts_with(VERBATIM_UNC) {
        let mut normalized = vec![b'\\' as u16, b'\\' as u16];
        normalized.extend_from_slice(&wide[VERBATIM_UNC.len()..]);
        PathBuf::from(OsString::from_wide(&normalized))
    } else if wide.starts_with(VERBATIM) {
        PathBuf::from(OsString::from_wide(&wide[VERBATIM.len()..]))
    } else {
        path
    }
}

#[cfg(not(target_os = "windows"))]
fn normalize_canonical_path(path: PathBuf) -> PathBuf {
    path
}

fn verified_target_child(parent: &Path, name: &str) -> TaskResult<PathBuf> {
    require(
        Path::new(name).components().count() == 1,
        "target child name must be exactly one path component",
    )?;
    let child = parent.join(name);
    require(
        child.parent() == Some(parent) && child.file_name() == Some(OsStr::new(name)),
        "target child escaped its intended parent",
    )?;
    Ok(child)
}

fn path_is_reparse_point(path: &Path) -> TaskResult<bool> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
    }

    #[cfg(not(target_os = "windows"))]
    Ok(false)
}

fn stage_runtime_binary(
    identity: &ProductIdentity,
    build_binary: &Path,
) -> TaskResult<PathBuf> {
    let target = canonical_target_dir()?;
    let runtime = verified_target_child(&target, RUNTIME_TARGET_NAME)?;
    if runtime.exists() {
        require(
            !path_is_reparse_point(&runtime)?,
            "refusing to use a symlink/reparse point as the runtime directory",
        )?;
    } else {
        fs::create_dir(&runtime).map_err(|error| {
            format!(
                "could not create runtime directory {}: {error}",
                runtime.display()
            )
        })?;
    }

    let (removed, retained) = cleanup_runtime_copies(&runtime, &identity.executable)?;
    if removed > 0 || retained > 0 {
        println!(
            "runtime copies      removed {removed} stale, retained {retained} still in use"
        );
    }

    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?
        .as_millis();
    let filename = generation_binary_name(
        &identity.executable,
        std::process::id(),
        generation,
        cfg!(target_os = "windows"),
    );
    let staged = runtime.join(filename);
    fs::copy(build_binary, &staged).map_err(|error| {
        format!(
            "could not stage {} as {}: {error}",
            build_binary.display(),
            staged.display()
        )
    })?;
    Ok(staged)
}

fn generation_binary_name(
    executable: &str,
    process_id: u32,
    generation: u128,
    windows: bool,
) -> String {
    format!(
        "{executable}-{process_id}-{generation}{}",
        if windows { ".exe" } else { "" }
    )
}

fn cleanup_runtime_copies(
    runtime: &Path,
    executable: &str,
) -> TaskResult<(usize, usize)> {
    let prefix = format!("{executable}-");
    let mut removed = 0;
    let mut retained = 0;
    for entry in fs::read_dir(runtime)
        .map_err(|error| format!("could not read {}: {error}", runtime.display()))?
    {
        let entry = entry.map_err(|error| format!("runtime entry failed: {error}"))?;
        let file_type = entry.file_type().map_err(|error| {
            format!("could not inspect {}: {error}", entry.path().display())
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !file_type.is_file() || !name.starts_with(&prefix) {
            continue;
        }
        match fs::remove_file(entry.path()) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                // Windows keeps running executable images locked. They are
                // intentionally retained and retried on the next launch.
                retained += 1;
            }
            Err(error) => {
                return Err(format!(
                    "could not remove stale runtime copy {}: {error}",
                    entry.path().display()
                ));
            }
        }
    }
    Ok((removed, retained))
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
    let build_binary = debug_binary(&identity);
    require(
        build_binary.is_file(),
        &format!("debug executable is missing: {}", build_binary.display()),
    )?;
    let binary = stage_runtime_binary(&identity, &build_binary)?;
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
        .map_err(|error| {
            let _ = fs::remove_file(&binary);
            format!("could not launch {}: {error}", binary.display())
        })?;
    println!(
        "PASS: launched {} as process {} from {}; Cargo's build output remains unlocked",
        identity.executable,
        child.id(),
        display_relative_or_absolute(&binary)
    );
    Ok(())
}

fn check() -> TaskResult {
    with_verification_target(check_in)
}

fn check_in(target: &Path) -> TaskResult {
    verify_all()?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run_quiet("cargo", &["metadata", "--locked", "--format-version", "1"])?;
    run_cargo_in(
        target,
        &["check", "--workspace", "--all-targets", "--locked"],
    )
}

fn verify_all() -> TaskResult {
    verify_identity()?;
    verify_provenance()?;
    verify_architecture()?;
    verify_phase_zero_assurance()?;
    package_check()
}

fn verify_phase_zero_assurance() -> TaskResult {
    let nextest = read(&root().join(".config/nextest.toml"))?;
    require(
        nextest.contains("required = \"0.9.137\"")
            && nextest.contains("flaky-result = \"fail\"")
            && nextest.contains("[profile.ci.junit]")
            && nextest.contains("[test-groups.native-pty]"),
        "Phase 0 Nextest version, flaky-result, JUnit, or resource-group policy is missing",
    )?;

    let qa = read(&root().join("tools/ci/qa.py"))?;
    require(
        qa.contains("MAX_LOG_BYTES = 2 * 1024 * 1024")
            && qa.contains("MAX_BUNDLE_BYTES = 64 * 1024 * 1024")
            && qa.contains("dirty_fingerprint_sha256")
            && qa.contains("environment_dumped\": False")
            && qa.contains("{\".etl\", \".png\", \".info\"}")
            && qa.contains("timeout_seconds")
            && qa.contains("[\"taskkill\", \"/PID\"")
            && qa.contains("os.killpg(process.pid")
            && qa.contains("collect_host_manifest()")
            && qa.contains("AUTOMEXIA_NATIVE_RESOURCE_REPORT")
            && qa.contains("JUnit report exceeds the 8 MiB artifact ceiling")
            && root().join("tools/ci/test_qa.py").is_file(),
        "Phase 0 QA evidence lacks bounds, deadlines, host identity, self-tests, or private-artifact safety",
    )?;

    let ci = read(&root().join(".github/workflows/ci.yml"))?;
    let release_workflow = read(&root().join(".github/workflows/release.yml"))?;
    let nightly_workflow = read(&root().join(".github/workflows/nightly.yml"))?;
    require(
        ci.contains("cargo-nextest@0.9.137")
            && ci.contains(
                "cargo nextest run --workspace --all-features --locked --profile ci",
            )
            && ci.contains("cargo test --workspace --all-features --doc --locked")
            && ci.contains("loom_channel_readiness")
            && ci.contains("python tools/ci/test_qa.py")
            && ci.contains("glslang-tools")
            && release_workflow.contains("glslang-tools")
            && nightly_workflow.contains("glslang-tools"),
        "CI/release workflows do not preserve Linux shader prerequisites, QA self-tests, pinned Nextest/JUnit, Cargo doctests, and Loom coverage",
    )?;

    let codeql_workflow = read(&root().join(".github/workflows/codeql.yml"))?;
    require(
        codeql_workflow.contains("workflow_dispatch:")
            && codeql_workflow.contains("actions: read")
            && codeql_workflow.contains("github/codeql-action/init@ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd # v4.37.7")
            && codeql_workflow.contains("github/codeql-action/analyze@ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd # v4.37.7")
            && codeql_workflow.contains("github.event.repository.private && 'never' || 'always'")
            && codeql_workflow.contains("codeql-results/**/*.sarif")
            && codeql_workflow.contains("if-no-files-found: error"),
        "CodeQL must be dispatchable, pinned to v4, upload findings when entitled, and retain private-repository SARIF without requiring GitHub Code Security",
    )?;
    require(
        nightly_workflow.contains("cargo +nightly fuzz run")
            && nightly_workflow.contains("--component rust-src")
            && nightly_workflow.contains("sanitizer: [address, thread]")
            && nightly_workflow.contains("MIRIFLAGS: -Zmiri-disable-isolation")
            && nightly_workflow.contains("timeout-minutes: 30")
            && nightly_workflow.contains("--locked simd_utf8::tests")
            && nightly_workflow.contains("--locked simd_base64::tests")
            && nightly_workflow.contains("--locked performer::parser::tests")
            && nightly_workflow.contains("test_temp_file_transmission_medium")
            && nightly_workflow.contains("--target x86_64-unknown-linux-gnu")
            && nightly_workflow.contains("tool: cross@0.2.5")
            && nightly_workflow.contains("go install github.com/goreleaser/nfpm/v2/cmd/nfpm@v2.43.4")
            && nightly_workflow.contains("$(go env GOPATH)/bin")
            && release_workflow.contains("go install github.com/goreleaser/nfpm/v2/cmd/nfpm@v2.43.4")
            && release_workflow.contains("$(go env GOPATH)/bin"),
        "Nightly/release workflows must use nightly GNU-target libFuzzer, install sanitizer std sources, enforce the bounded Miri suite and timeout, preserve both sanitizer jobs, and expose the pinned Go-based nFPM tool",
    )?;
    let simd_utf8 = read(&root().join("rio-vt/src/simd_utf8.rs"))?;
    let simd_base64 = read(&root().join("rio-vt/src/simd_base64.rs"))?;
    let parser = read(&root().join("rio-vt/src/performer/parser/mod.rs"))?;
    require(
        simd_utf8.contains("cfg(any(target_arch = \"wasm32\", miri))")
            && simd_base64.contains("cfg(any(target_arch = \"wasm32\", miri))")
            && parser.contains("cfg(any(target_arch = \"wasm32\", miri))")
            && simd_utf8.contains("cfg(all(not(target_arch = \"wasm32\"), not(miri)))")
            && simd_base64.contains("cfg(all(not(target_arch = \"wasm32\"), not(miri)))")
            && parser.contains("cfg(all(not(target_arch = \"wasm32\"), not(miri)))"),
        "Miri must use scalar UTF-8, base64, and parser transcode paths instead of unsupported native simdutf FFI",
    )?;
    for source in [
        "rio-window/src/platform_impl/macos/app.rs",
        "rio-window/src/platform_impl/macos/app_delegate.rs",
        "rio-window/src/platform_impl/macos/view.rs",
        "rio-window/src/platform_impl/macos/window.rs",
        "rio-window/src/platform_impl/macos/window_delegate.rs",
    ] {
        let contents = read(&root().join(source))?;
        require(
            contents.contains("define_class!")
                && contents.contains("#[unsafe(super(")
                && !contents.contains("declare_class!")
                && !contents.contains("DeclaredClass"),
            &format!("{source} does not use the objc2 0.6 class-definition contract"),
        )?;
    }

    let app_manifest = read(&root().join("apps/automexia-terminal/Cargo.toml"))?;
    let channel_manifest = read(&root().join("corcovado/Cargo.toml"))?;
    require(
        app_manifest.contains("insta = { workspace = true }")
            && app_manifest.contains("proptest = { workspace = true }")
            && channel_manifest.contains("loom = { workspace = true }")
            && root()
                .join("apps/automexia-terminal/proptest-regressions/layout/compute_tests.txt")
                .is_file()
            && root()
                .join("corcovado/tests/loom_channel_readiness.rs")
                .is_file(),
        "Phase 0 property/snapshot/model dependencies or persisted regressions are missing",
    )?;

    let appverifier = read(&root().join("tests/integration/appverifier-windows.ps1"))?;
    require(
        appverifier.contains("IsInRole")
            && appverifier.contains("refusing to overwrite maintainer-owned state")
            && appverifier.contains("finally")
            && appverifier.contains("-disable '*'")
            && appverifier.contains("-delete settings")
            && appverifier.contains("automexia.exe"),
        "Application Verifier wrapper lacks preflight refusal, exact target, or guaranteed cleanup",
    )?;
    let wpr = read(&root().join("tests/integration/wpr-windows.ps1"))?;
    require(
        wpr.contains("IsInRole")
            && wpr.contains("finally")
            && wpr.contains("-cancel")
            && wpr.contains("MaximumTraceBytes")
            && wpr.contains("DeleteTraceAfterManifest")
            && wpr.contains("automexia.exe"),
        "WPR wrapper lacks elevation/exact-target checks, trace bounds, or guaranteed cancellation",
    )?;
    let native_resize =
        read(&root().join("tests/integration/resize-stress-windows.ps1"))?;
    let native_window_locator =
        read(&root().join("tests/integration/windows-native-window-locator.cs"))?;
    require(
        native_resize.contains("BitBlt(")
            && native_resize.contains("ClientToScreen")
            && native_resize.contains("SetThreadDpiAwarenessContext")
            && native_resize.contains("SetCaptureTopmost")
            && native_resize.contains("CaptureClientRegionStats")
            && native_resize.contains("overlay_rect")
            && native_resize.contains("MeanLuminance -ge 20")
            && native_resize.contains("[switch]$UseCpuRenderer")
            && native_resize.contains("VisibleApplicationWindows")
            && native_resize.contains("[string]$FrameCapture")
            && native_resize.contains("[string]$TypographyCapture")
            && native_resize.contains("typography_frame = [ordered]@{")
            && native_resize.contains("font_size - 18.0")
            && native_resize.contains("line_height - 1.15")
            && native_resize.contains("$frameDeadline = [DateTime]::UtcNow.AddSeconds(5)")
            && native_resize.contains("DistinctColorBuckets -ge 8")
            && native_resize.contains("did not settle within 5 seconds")
            && native_resize.contains("settle_milliseconds = $frameStopwatch.ElapsedMilliseconds")
            && native_resize.contains("painted_frame = [ordered]@{")
            && native_window_locator.contains("EnumWindows")
            && native_window_locator.contains("IsWindowVisible")
            && native_window_locator.contains("Winit Thread Event Target")
            && native_window_locator.contains("title.Length > 0")
            && native_window_locator.contains("rect.Right - rect.Left >= 100"),
        "Windows resize stress lacks reliable application-HWND discovery, GPU/CPU preview pixel fidelity, strict bounded painted-frame settling, or explicit private artifact control",
    )?;
    require(
        root().join("docs/ACCESSIBILITY.md").is_file()
            && root()
                .join("docs/adr/0013-renderer-independent-accessibility-model.md")
                .is_file(),
        "Phase 0 accessibility baseline or v0.5 model ADR is missing",
    )?;

    println!("PASS: Phase 0 assurance tooling and evidence contracts are present");
    Ok(())
}

fn ci() -> TaskResult {
    complete_ci_gate()
}

fn qa(bundle: bool) -> TaskResult {
    require_native_wsl_workspace("the Phase 0 QA evidence gate")?;
    let program =
        python_program().ok_or("Python 3 is required for the QA evidence runner")?;
    let mut command = Command::new(program);
    command
        .arg("tools/ci/qa.py")
        .arg("--full")
        .current_dir(root());
    if bundle {
        command.arg("--bundle");
    }
    run_command(command, "Phase 0 QA evidence runner")
}

fn complete_ci_gate() -> TaskResult {
    doctor()?;
    run_python("tools/ci/validate_repository.py")?;
    validate_shell_integrations()?;
    with_verification_target(ci_in)?;
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
    )
}

#[cfg(target_os = "windows")]
fn validate_shell_integrations() -> TaskResult {
    run(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "tools/ci/test_powershell.ps1",
        ],
    )
}

#[cfg(not(target_os = "windows"))]
fn validate_shell_integrations() -> TaskResult {
    run("bash", &["tools/ci/test_shell_sources.sh"])
}

fn ci_in(target: &Path) -> TaskResult {
    println!("==> verification phase 1/3: workspace checks");
    check_in(target)?;
    println!("==> verification phase 2/3: warning-denied Clippy");
    run_cargo_in(
        target,
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
    println!(
        "==> verification phase 3/3: workspace tests (a cold isolated target can compile for several minutes)"
    );
    run_cargo_summarized_in(
        target,
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
            "rio-fonts",
            "-p",
            "sugarloaf",
            "-p",
            "teletypewriter",
            "--locked",
        ],
    )
}

fn parse_fuzz_seconds(value: &str) -> TaskResult<u64> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| "--seconds must be an integer from 1 through 86400")?;
    if !(1..=86_400).contains(&seconds) {
        return Err("--seconds must be an integer from 1 through 86400".into());
    }
    Ok(seconds)
}

fn test_image_rendering(native_gui: bool) -> TaskResult {
    run("cargo", &["test", "-p", "automexia-image", "--locked"])?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "sugarloaf",
            "--lib",
            "--locked",
            "image_overlay_tests",
        ],
    )?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "sugarloaf",
            "--lib",
            "--locked",
            "texture_budget_tests",
        ],
    )?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "automexia-terminal",
            "--bin",
            "automexia",
            "--locked",
            "image_preview",
            "--",
            "--test-threads=1",
        ],
    )?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "rio-vt",
            "--lib",
            "--features",
            "graphics",
            "--locked",
        ],
    )?;
    run(
        "cargo",
        &["test", "-p", "rio-backend", "--lib", "--locked", "graphics"],
    )?;
    run(
        "cargo",
        &[
            "check",
            "-p",
            "automexia-terminal",
            "--bench",
            "image_preview",
            "--locked",
        ],
    )?;

    if native_gui {
        test_resize_stress(true)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn image_decoder_fuzz_wsl_script(seconds: u64) -> String {
    format!(
        concat!(
            "set -euo pipefail; ",
            "command -v rustup >/dev/null 2>&1 || ",
            "{{ echo 'rustup is required inside WSL' >&2; exit 2; }}; ",
            "source_root=$1; ",
            "test -d \"$source_root\" || ",
            "{{ echo 'the translated Automexia source root is unavailable' >&2; exit 2; }}; ",
            "command -v tar >/dev/null 2>&1 || ",
            "{{ echo 'tar is required inside WSL' >&2; exit 2; }}; ",
            "rustup toolchain install nightly --profile minimal; ",
            "if ! cargo +nightly fuzz --version >/dev/null 2>&1; then ",
            "cargo install cargo-fuzz --version 0.13.1 --locked; ",
            "fi; ",
            "fuzz_workspace=$(mktemp -d /tmp/automexia-image-fuzz.XXXXXX); ",
            "trap 'rm -rf -- \"$fuzz_workspace\"' EXIT INT TERM; ",
            "mkdir -p \"$fuzz_workspace/source\" \"$fuzz_workspace/target\" \"$fuzz_workspace/corpus\"; ",
            "tar -C \"$source_root\" --exclude='./.git' --exclude='./target' ",
            "--exclude='./fuzz/target' --exclude='./fuzz/corpus' ",
            "--exclude='./fuzz/artifacts' -cf - . | ",
            "tar -C \"$fuzz_workspace/source\" -xf -; ",
            "cd \"$fuzz_workspace/source\"; ",
            "CARGO_TARGET_DIR=\"$fuzz_workspace/target\" cargo +nightly fuzz run ",
            "image_decoder \"$fuzz_workspace/corpus\" -- ",
            "-max_total_time={} -rss_limit_mb=768 -timeout=15"
        ),
        seconds
    )
}

#[cfg(target_os = "windows")]
fn test_image_decoder_fuzz(seconds: u64) -> TaskResult {
    // cargo-fuzz/libFuzzer is officially supported on Unix-like hosts, not
    // native Windows. Route Windows contributors through WSL instead of
    // relying on an unsupported clang_rt.asan_dynamic DLL search.
    let output = Command::new("wsl.exe")
        .args(["--exec", "wslpath", "-a", "-u"])
        .arg(root())
        .output()
        .map_err(|error| {
            format!(
                "could not start WSL for the decoder fuzz campaign: {error}. Install WSL and a Linux distribution"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "WSL could not translate the repository path ({}). Install or repair a default WSL distribution",
            output.status
        ));
    }
    let wsl_root = String::from_utf8(output.stdout)
        .map_err(|_| "wslpath returned a non-UTF-8 repository path")?;
    let wsl_root = wsl_root.trim();
    if wsl_root.is_empty() {
        return Err("wslpath returned an empty repository path".into());
    }

    let script = image_decoder_fuzz_wsl_script(seconds);
    let mut command = Command::new("wsl.exe");
    command
        .arg("--cd")
        .arg(wsl_root)
        .args(["--exec", "bash", "-lc"])
        .arg(script)
        .arg("automexia-image-fuzz")
        .arg(wsl_root);
    run_command(
        command,
        "nightly image-decoder fuzz campaign through supported WSL libFuzzer",
    )
}

#[cfg(not(target_os = "windows"))]
fn test_image_decoder_fuzz(seconds: u64) -> TaskResult {
    let mut rustup = Command::new("rustup");
    rustup.args(["toolchain", "install", "nightly", "--profile", "minimal"]);
    run_command(rustup, "install/update the nightly fuzz toolchain")?;

    let fuzz_available = Command::new("cargo")
        .args(["+nightly", "fuzz", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !fuzz_available {
        let mut install = Command::new("cargo");
        install.args(["install", "cargo-fuzz", "--version", "0.13.1", "--locked"]);
        run_command(install, "install pinned cargo-fuzz 0.13.1")?;
    }

    let workspace = tempfile::Builder::new()
        .prefix("automexia-image-fuzz-")
        .tempdir()
        .map_err(|error| {
            format!("could not create disposable image-fuzz workspace: {error}")
        })?;
    let target = workspace.path().join("target");
    let corpus = workspace.path().join("corpus");
    fs::create_dir_all(&target)
        .map_err(|error| format!("could not create disposable fuzz target: {error}"))?;
    fs::create_dir_all(&corpus)
        .map_err(|error| format!("could not create disposable fuzz corpus: {error}"))?;
    let mut fuzz = Command::new("cargo");
    fuzz.args(["+nightly", "fuzz", "run", "image_decoder"])
        .arg(&corpus)
        .args([
            "--",
            &format!("-max_total_time={seconds}"),
            "-rss_limit_mb=768",
            "-timeout=15",
        ])
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(root());
    run_command(fuzz, "nightly image-decoder fuzz campaign")
}

fn test_resize_stress(native_gui: bool) -> TaskResult {
    run(
        "cargo",
        &[
            "test",
            "-p",
            "rio-vt",
            "--lib",
            "--locked",
            "resize_stress",
            "--",
            "--nocapture",
        ],
    )?;

    if !native_gui {
        return Ok(());
    }

    if !cfg!(target_os = "windows") {
        return Err(
            "test resize-stress --native-gui is currently supported on Windows".into(),
        );
    }

    run(
        "cargo",
        &[
            "build",
            "-p",
            "automexia-terminal",
            "--locked",
            "--features",
            "native-gui-test-hooks",
        ],
    )?;
    let identity = product_identity()?;
    let binary = debug_binary(&identity);
    let binary = binary.to_str().ok_or_else(|| {
        format!(
            "native GUI test binary path is not UTF-8: {}",
            binary.display()
        )
    })?;
    let report_directory = tempfile::Builder::new()
        .prefix("automexia-native-image-")
        .tempdir()
        .map_err(|error| {
            format!("could not create disposable native image evidence: {error}")
        })?;
    let requested_report = env::var_os("AUTOMEXIA_NATIVE_RESOURCE_REPORT");
    if requested_report
        .as_ref()
        .is_some_and(|report| report.is_empty())
    {
        return Err("AUTOMEXIA_NATIVE_RESOURCE_REPORT cannot be empty".into());
    }
    let requested_report_path = requested_report.as_ref().map(PathBuf::from);
    let modal_capture_directory = requested_report_path
        .as_ref()
        .and_then(|report| report.parent())
        .map(|parent| parent.join("modal-captures"));
    let typography_capture_directory = requested_report_path
        .as_ref()
        .and_then(|report| report.parent())
        .map(|parent| parent.join("typography-captures"));
    if let Some(directory) = typography_capture_directory.as_ref() {
        fs::create_dir_all(directory).map_err(|error| {
            format!(
                "could not create native typography capture directory {}: {error}",
                directory.display()
            )
        })?;
    }
    let wgpu_typography_capture = typography_capture_directory
        .as_ref()
        .map(|directory| directory.join("workspace-wgpu.png"));
    let cpu_typography_capture = typography_capture_directory
        .as_ref()
        .map(|directory| directory.join("workspace-cpu.png"));
    let wgpu_report = requested_report_path
        .clone()
        .unwrap_or_else(|| report_directory.path().join("wgpu.json"));
    let cpu_report = report_directory.path().join("cpu.json");
    let mut command = Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "tests/integration/resize-stress-windows.ps1",
            "-Binary",
            binary,
        ])
        .arg("-ResourceReport")
        .arg(&wgpu_report)
        .current_dir(root());
    if let Some(capture_directory) = modal_capture_directory.as_ref() {
        command.arg("-ModalCaptureDirectory").arg(capture_directory);
    }
    if let Some(capture) = wgpu_typography_capture.as_ref() {
        command.arg("-TypographyCapture").arg(capture);
    }
    run_command(command, "native Windows WGPU GUI resize stress")?;

    // The CPU fallback has a separate compositor and pass ordering. Run the
    // same real-window/pixel-fidelity contract there as well so a present but
    // card-obscured preview cannot regress unnoticed.
    let mut cpu_command = Command::new("powershell");
    cpu_command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "tests/integration/resize-stress-windows.ps1",
            "-Binary",
            binary,
            "-UseCpuRenderer",
        ])
        .arg("-ResourceReport")
        .arg(&cpu_report)
        .current_dir(root());
    if let Some(capture_directory) = modal_capture_directory.as_ref() {
        cpu_command
            .arg("-ModalCaptureDirectory")
            .arg(capture_directory);
    }
    if let Some(capture) = cpu_typography_capture.as_ref() {
        cpu_command.arg("-TypographyCapture").arg(capture);
    }
    run_command(cpu_command, "native Windows CPU GUI resize stress")?;
    verify_native_image_backend_equivalence(&wgpu_report, &cpu_report)
}

fn verify_native_image_backend_equivalence(wgpu: &Path, cpu: &Path) -> TaskResult {
    fn report(path: &Path) -> Result<serde_json::Value, String> {
        let bytes = fs::read(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("could not parse {}: {error}", path.display()))
    }
    fn metric(value: &serde_json::Value, name: &str) -> Result<f64, String> {
        value
            .pointer(&format!("/image_preview_pixels/{name}"))
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| format!("native image report is missing numeric {name}"))
    }

    let wgpu = report(wgpu)?;
    let cpu = report(cpu)?;
    for name in ["width", "height", "sample_count"] {
        require(
            metric(&wgpu, name)? == metric(&cpu, name)?,
            &format!("WGPU and CPU preview {name} differ"),
        )?;
    }
    let mean_delta =
        (metric(&wgpu, "mean_luminance")? - metric(&cpu, "mean_luminance")?).abs();
    let spread_delta =
        (metric(&wgpu, "luminance_spread")? - metric(&cpu, "luminance_spread")?).abs();
    let bright_delta =
        (metric(&wgpu, "bright_ratio")? - metric(&cpu, "bright_ratio")?).abs();
    require(
        mean_delta <= 24.0 && spread_delta <= 40.0 && bright_delta <= 0.15,
        &format!(
            "WGPU/CPU preview pixels diverged: mean={mean_delta:.2}, spread={spread_delta:.2}, bright-ratio={bright_delta:.3}"
        ),
    )?;
    println!(
        "PASS: WGPU/CPU preview pixels agree within controlled tolerance (mean {mean_delta:.2}, spread {spread_delta:.2}, bright ratio {bright_delta:.3})"
    );
    Ok(())
}

fn test_session_clone(native: Option<&str>) -> TaskResult {
    run(
        "cargo",
        &[
            "test",
            "-p",
            "automexia-terminal",
            "--locked",
            "context::launch::tests",
            "--",
            "--nocapture",
        ],
    )?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "automexia-terminal",
            "--locked",
            "clone_",
            "--",
            "--nocapture",
        ],
    )?;
    run(
        "cargo",
        &[
            "test",
            "-p",
            "teletypewriter",
            "--locked",
            "command_line_tests",
            "--",
            "--nocapture",
        ],
    )?;

    let Some(native) = native else {
        return Ok(());
    };
    if !cfg!(target_os = "windows") {
        return Err(format!(
            "test session-clone --native-{native} is currently supported on Windows"
        ));
    }

    run(
        "cargo",
        &[
            "build",
            "-p",
            "automexia-terminal",
            "--locked",
            "--features",
            "native-gui-test-hooks",
        ],
    )?;
    let identity = product_identity()?;
    let binary = debug_binary(&identity);
    let binary = binary.to_str().ok_or_else(|| {
        format!(
            "native GUI test binary path is not UTF-8: {}",
            binary.display()
        )
    })?;
    let script = match native {
        "windows" => "tests/integration/resize-stress-windows.ps1",
        "wsl" => "tests/integration/session-clone-wsl-windows.ps1",
        _ => return Err(format!("unsupported native clone suite: {native}")),
    };
    run(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script,
            "-Binary",
            binary,
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

fn cargo_command(target: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("cargo");
    command
        .args(args)
        .current_dir(root())
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_INCREMENTAL", "0");
    command
}

fn run_cargo_in(target: &Path, args: &[&str]) -> TaskResult {
    println!(
        "+ CARGO_INCREMENTAL=0 CARGO_TARGET_DIR={} cargo {}",
        target.display(),
        args.join(" ")
    );
    let status = cargo_command(target, args)
        .status()
        .map_err(|error| format!("could not start cargo: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo exited with {status}"))
    }
}

fn run_cargo_summarized_in(
    target: &Path,
    args: &[&str],
    success_message: &str,
) -> TaskResult {
    println!(
        "+ CARGO_INCREMENTAL=0 CARGO_TARGET_DIR={} cargo {}",
        target.display(),
        args.join(" ")
    );
    let output = cargo_command(target, args)
        // Cargo writes compiler/build-script progress and diagnostics to
        // stderr. Keep that stream attached to the contributor's terminal so
        // a cold native dependency build never looks frozen. Test-harness
        // stdout remains captured and summarized on success below.
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("could not start cargo: {error}"))?;
    if output.status.success() {
        println!("PASS: {success_message}");
        return Ok(());
    }

    // Successful test output is intentionally summarized, but failures retain
    // the complete harness and compiler diagnostics needed for investigation.
    print!("{}", String::from_utf8_lossy(&output.stdout));
    Err(format!("cargo exited with {}", output.status))
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

    let private_crates: [(&str, &[&str]); 5] = [
        ("automexia-extension-api", &["serde", "serde_json"]),
        (
            "automexia-extension-runtime",
            &["automexia-extension-api", "loom"],
        ),
        (
            "automexia-devops",
            &[
                "automexia-extension-api",
                "automexia-ui-model",
                "dirs",
                "serde_json",
            ],
        ),
        ("automexia-image", &["image", "libc", "tempfile"]),
        (
            "automexia-ui-model",
            &["automexia-extension-api", "unicode-segmentation"],
        ),
    ];
    for (name, allowed_dependencies) in private_crates {
        let package = packages
            .iter()
            .find(|package| package["name"].as_str() == Some(name))
            .ok_or_else(|| format!("private Phase 1 package {name} is missing"))?;
        require(
            package["publish"].as_array().is_some_and(Vec::is_empty),
            &format!("{name} must remain publish = false"),
        )?;
        for dependency in package["dependencies"]
            .as_array()
            .ok_or_else(|| format!("{name} dependencies are missing"))?
        {
            let dependency_name = dependency["name"]
                .as_str()
                .ok_or_else(|| format!("{name} has an unnamed dependency"))?;
            require(
                allowed_dependencies.contains(&dependency_name),
                &format!(
                    "{name} has forbidden dependency {dependency_name}; allowed: {allowed_dependencies:?}"
                ),
            )?;
        }
    }

    for manifest in ["rio-backend/Cargo.toml", "sugarloaf/Cargo.toml"] {
        let source = read(&root().join(manifest))?;
        require(
            source.contains("crate-type = [\"rlib\"]")
                && !source.contains("crate-type = [\"cdylib\", \"rlib\"]"),
            &format!(
                "{manifest} must remain rlib-only; duplicate cdylib outputs race in benchmark builds"
            ),
        )?;
    }

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
    let extension_runtime = read(&root().join("automexia-extension-runtime/src/lib.rs"))?;
    require(
        extension_runtime.contains("sync_channel")
            && extension_runtime.contains("try_send")
            && extension_runtime.contains("handle.is_finished()")
            && extension_runtime.contains("pub trait WakeRoute")
            && extension_runtime.contains("pub struct CoalescingSlot")
            && extension_runtime.contains("pub fn try_submit_then")
            && extension_runtime.contains("registration_ready")
            && extension_runtime.contains("pub fn plan_rebind"),
        "private extension runtime lacks bounded queue, restart, injected wake, registration ordering, coalescing, or rebind primitives",
    )?;
    require(
        runtime.contains("MAX_SESSION_TITLE_BYTES")
            && runtime.contains("shutdown_background_services")
            && runtime.contains("CacheKey")
            && runtime.contains("capsule_revision")
            && runtime.contains("CancellationToken")
            && runtime.contains("Freshness::Error")
            && runtime.contains("preserving last truthful snapshot"),
        "application extension adapter lacks bounds, joined shutdown, capsule-key isolation, cancellation, or last-known-good ownership",
    )?;
    let renderable = read(&app.join("src/context/renderable.rs"))?;
    let renderer = read(&app.join("src/renderer/mod.rs"))?;
    for field in [
        "current_directory",
        "terminal_title",
        "shell_distro",
        "shell_name",
        "shell_user",
        "shell_path",
        "shell_integration",
        "shell_prompt_active",
    ] {
        require(
            renderable.contains(&format!("pub {field}:"))
                && snapshots_renderable_field(&renderer, field),
            &format!("shell/prompt readiness metadata {field} is not snapshotted"),
        )?;
    }
    let launch = read(&app.join("src/context/launch.rs"))?;
    let context = read(&app.join("src/context/mod.rs"))?;
    require(
        launch.contains("pub struct SessionLaunchDescriptor")
            && launch.contains("pub fn fresh_clone")
            && context.contains("pub launch_descriptor: SessionLaunchDescriptor")
            && context.contains("pub fn clone_split")
            && context.contains("ContextManager::create_context"),
        "session cloning does not pass through an immutable launch descriptor and fresh context",
    )?;
    require(
        launch.contains("No PowerShell fallback was opened")
            && launch.contains("valid_directory_text")
            && !launch.contains("terminal_title")
            && !launch.contains("visible_text"),
        "WSL session cloning may infer launch identity from presentation text or silently fall back",
    )?;
    require(
        context.contains("report_clone_error")
            && context.contains("RioEvent::ReportToAssistant")
            && context.contains("return false"),
        "session clone failures do not preserve layout and report a user-visible error",
    )?;
    let bindings = read(&app.join("src/bindings/mod.rs"))?;
    let palette = read(&app.join("src/renderer/command_palette.rs"))?;
    require(
        bindings.contains(
            r#""r", ModifiersState::CONTROL, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitRight"#,
        ) && bindings.contains(
            r#""d", ModifiersState::CONTROL, ~BindingMode::SEARCH, ~BindingMode::VI; Action::CloneSplitDown"#,
        ) && bindings.contains(
            r#""r", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitRight"#,
        ) && bindings.contains(
            r#""d", ModifiersState::CONTROL | ModifiersState::SHIFT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::SplitDown"#,
        ) && bindings.contains(
            r#""r", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::Esc("\x12".into())"#,
        ) && bindings.contains(
            r#""d", ModifiersState::CONTROL | ModifiersState::ALT, ~BindingMode::SEARCH, ~BindingMode::VI; Action::Esc("\x04".into())"#,
        ) && palette.contains("Clone Active Session Right")
            && palette.contains("Clone Active Session Down")
            && palette.contains("shortcut: SHORTCUT_CLONE_RIGHT")
            && palette.contains("shortcut: SHORTCUT_CLONE_DOWN"),
        "Automexia classic fresh-split, clone, and explicit shell-control shortcuts are not distinct",
    )?;
    let layout_source = read(&app.join("src/layout/mod.rs"))?;
    require(
        bindings.contains(
            "Key::Named(ArrowLeft), ModifiersState::ALT; Action::SelectPaneLeft",
        ) && bindings.contains(
            "Key::Named(PageDown), ModifiersState::ALT; Action::SelectNextLocalTab",
        ) && bindings.contains(
            "Key::Named(ArrowLeft), ModifiersState::SUPER | ModifiersState::ALT; Action::SelectPaneLeft",
        ) && bindings.contains(
            "\"]\", ModifiersState::SUPER | ModifiersState::ALT; Action::SelectNextLocalTab",
        ) && palette.contains("Focus Pane Left")
            && palette.contains("Next Tab in Selected Pane")
            && layout_source.contains("directional_pane_neighbor")
            && layout_source.contains("outside_beam")
            && layout_source.contains("adjacent_local_tab_index"),
        "pane geometry and pane-local tab navigation are missing cross-platform bindings, discoverability, or deterministic selection",
    )?;
    let context_renderer = read(&app.join("src/renderer/devops_status.rs"))?;
    let renderer_root = read(&app.join("src/renderer/mod.rs"))?;
    let api_model = read(&root().join("automexia-extension-api/src/lib.rs"))?;
    let ui_model = read(&root().join("automexia-ui-model/src/lib.rs"))?;
    require(
        context_renderer.contains("automexia_ui_model::project_status")
            && context_renderer.contains("automexia_ui_model::segment_tag_color")
            && context_renderer.contains("automexia_ui_model::segment_tag_background")
            && !context_renderer.contains("DevOpsSnapshot")
            && !context_renderer.contains("CloudContext")
            && !context_renderer.contains("builtins::devops")
            && !renderer_root.contains("builtins::devops"),
        "renderer depends on a provider implementation instead of generic status segments",
    )?;
    require(
        api_model.contains("pub enum SegmentRole")
            && api_model.contains("pub struct StatusSegment")
            && api_model.contains("pub struct ContextContribution")
            && api_model.contains("pub struct DetailsAction")
            && api_model.contains("observed_at_ms")
            && ui_model.contains("pub fn project_status")
            && ui_model.contains("pub fn layout_segments")
            && ui_model.contains("pub fn hit_test")
            && ui_model.contains("pub fn details_action_at")
            && ui_model.contains("pub fn segment_color")
            && ui_model.contains("pub fn segment_tag_color")
            && ui_model.contains("pub fn segment_tag_background")
            && ui_model.contains("pub fn segment_tag_surface")
            && ui_model.contains("CONTEXT_TAG_BACKGROUND_ALPHA")
            && ui_model.contains("unicode_segmentation"),
        "generic UI model lacks semantic roles, bounded timestamps/actions, responsive layout, hit testing, accessibility, contrast, or grapheme handling",
    )?;
    for contract in [
        "ExtensionId",
        "SessionId",
        "OperationId",
        "ExecutableId",
        "LaunchRequest",
        "EnvironmentCapsule",
        "ContextContribution",
        "StatusSegment",
        "Freshness",
        "CapabilityRequest",
        "CapabilityDecision",
        "SecretReference",
        "PublicDiagnostic",
    ] {
        require(
            api_model.contains(&format!("pub struct {contract}"))
                || api_model.contains(&format!("pub enum {contract}"))
                || api_model.contains(&format!("string_identifier!({contract},")),
            &format!("extension API is missing versioned contract {contract}"),
        )?;
    }
    require(
        api_model.contains("deny_unknown_fields")
            && api_model.contains("try_from = \"LaunchRequestWire\"")
            && api_model.contains("MAX_SECRET_REFERENCES")
            && api_model.contains("UnsupportedVersion")
            && api_model.contains("SecretReference([REDACTED])")
            && launch.contains("pub fn launch_contract")
            && context.contains("pub environment_capsule: EnvironmentCapsule")
            && context.contains("independent session clones must never share an environment capsule"),
        "Phase 1 contracts lack schema rejection, secret redaction, launch adaptation, or per-session capsule ownership",
    )?;

    let island_renderer = read(&app.join("src/renderer/island.rs"))?;
    let pane_layout = read(&app.join("src/layout/mod.rs"))?;
    let responsive_chrome = read(&app.join("src/renderer/responsive.rs"))?;
    let renderer_utils = read(&app.join("src/renderer/utils.rs"))?;
    let screen = read(&app.join("src/screen/mod.rs"))?;
    require(
        screen.contains("fn should_copy_selection_on_ctrl_c")
            && screen.contains("fn has_nonempty_selection")
            && screen.contains("SecondaryClickClipboardAction::CopySelectionAndClear")
            && screen.contains(
                "ctrl_c_copies_only_a_nonempty_selection_and_otherwise_remains_interrupt",
            )
            && screen.contains(
                "secondary_click_copies_and_clears_selection_or_pastes_clipboard_exclusively",
            )
            && bindings.contains(
                "MouseButton::Right,  ~BindingMode::VI;         Action::Paste;",
            )
            && bindings.contains(
                "default_mouse_clipboard_bindings_preserve_primary_selection_ownership",
            ),
        "selection-aware Ctrl+C or safe secondary-click clipboard ownership regressed",
    )?;

    require(
        !island_renderer.contains("UTILITY_ACTIONS")
            && !island_renderer.contains("UtilityActionGeometry")
            && !island_renderer.contains("ChromeAction::Search")
            && !island_renderer.contains("ChromeAction::SplitRight")
            && !island_renderer.contains("ChromeAction::SplitDown")
            && !island_renderer.contains("ChromeAction::NextPane")
            && island_renderer
                .contains("space_below_window_header_has_no_workspace_action_hit_targets")
            && island_renderer.contains("draw_pane_local_tab_rails")
            && island_renderer.contains("local_tab_hit_testing_is_pane_scoped_at_hidpi")
            && pane_layout.contains("pub fn pane_tab_rail_rect")
            && pane_layout.contains("pub fn pane_terminal_rect")
            && responsive_chrome.contains("pub fn content_top(self) -> f32")
            && !responsive_chrome.contains("show_secondary_rail")
            && !renderer_utils.contains("show_secondary_rail")
            && context_renderer.contains("pub fn refresh_session_context")
            && !context_renderer.contains("pub fn render_context_bar"),
        "pane-local tabs are not isolated from window chrome or removed global actions still leak into chrome",
    )?;
    require(
        !screen.contains("ChromeAction::Search =>")
            && !screen.contains("ChromeAction::SplitRight =>")
            && !screen.contains("ChromeAction::SplitDown =>")
            && !screen.contains("ChromeAction::NextPane =>")
            && screen.contains("Act::SearchForward =>")
            && screen.contains("Act::SplitRight =>")
            && screen.contains("Act::SplitDown =>")
            && screen.contains("PaletteAction::SearchForward =>")
            && screen.contains("PaletteAction::SplitRight =>")
            && screen.contains("PaletteAction::SplitDown =>"),
        "removed workspace buttons leaked back into screen routing or their commands became unreachable",
    )?;
    let session_footer = read(&app.join("src/renderer/session_footer.rs"))?;
    let router = read(&app.join("src/router/mod.rs"))?;
    require(
        !session_footer.contains("SessionFooterAction")
            && !session_footer.contains("draw_action_surface")
            && !session_footer.contains("draw_search_icon")
            && !session_footer.contains("draw_live_icon")
            && session_footer.contains("Some(SessionFooterHit { route_id })")
            && session_footer.contains("footer_is_a_passive_status_surface_without_action_regions")
            && session_footer.contains("footer_preserves_vertical_chrome_origin_and_absorbs_outer_horizontal_margins")
            && session_footer.contains("adjacent_split_footers_tile_the_split_seam_without_a_gap")
            && !session_footer.contains("FOOTER_INSET_X")
            && !session_footer.contains("FOOTER_INSET_Y")
            && session_footer.contains("\"UTF-8\"")
            && session_footer.contains("line_ending_for_shell")
            && session_footer.contains("current_clock_label")
            && router.contains("route.window.screen.context_manager.update_titles();")
            && router.contains("route.request_redraw();")
            && !screen.contains("session_footer_action_hovered")
            && !screen.contains("SessionFooterAction"),
        "pane footer must remain passive, useful, and live without visible or hidden action controls",
    )?;
    let powershell_view =
        read(&root().join("shell-integration/powershell/automexia.format.ps1xml"))?;
    require(
        powershell_view.contains("<Label>Mode</Label>")
            && powershell_view.contains("<Label>Last Modified</Label>")
            && powershell_view.contains("<Label>Size</Label>")
            && powershell_view.contains("<Label>Name</Label>")
            && !powershell_view.contains("<Label>Icon</Label>")
            && powershell_view.contains("$glyph $displayName")
            && powershell_view.contains("ReparsePoint")
            && powershell_view.contains("ConvertFromUtf32")
            && powershell_view.contains("0xF0250")
            && powershell_view.contains("0xF107F")
            && powershell_view.contains("0xF0C82")
            && powershell_view.contains("0xF19F6")
            && powershell_view.contains("255;92;122")
            && powershell_view.contains("PSVersionTable.PSVersion.Major -ge 7")
            && powershell_view.contains("[Console]::IsOutputRedirected")
            && powershell_view.contains("WindowSize.Width -ge 96")
            && powershell_view.contains("38;5;${legacyColor}"),
        "PowerShell filesystem view does not keep composite folder badges and width-safe colors beside native object names",
    )?;
    let posix_folder_filter =
        read(&root().join("shell-integration/posix/automexia-eza-filter.pl"))?;
    require(
        posix_folder_filter.contains("generic_folder")
            && posix_folder_filter.contains("0xF0250")
            && posix_folder_filter.contains("0xF107F")
            && posix_folder_filter.contains("0xF19F6")
            && posix_folder_filter.contains("0xF0870"),
        "POSIX eza compatibility path does not provide composite folder badges",
    )?;
    let cmd_integration = read(&root().join("shell-integration/cmd/automexia.cmd"))?;
    let cmd_listing = read(&root().join("shell-integration/cmd/automexia-ls.ps1"))?;
    require(
        cmd_integration.is_ascii()
            && cmd_integration.contains("SetUserVar=automexia_shell_name=Q01E")
            && cmd_integration.contains("set \"AUTOMEXIA_CMD_IDENTITY=")
            && cmd_integration.contains("PROMPT=%AUTOMEXIA_CMD_IDENTITY%")
            && cmd_integration.contains("AUTOMEXIA_CMD_PROMPT_GLYPH")
            && cmd_integration.contains("]7;file:///$P")
            && cmd_integration.contains("]133;A")
            && cmd_integration.contains("]133;P;k=c")
            && cmd_integration.contains("]133;B")
            && cmd_integration.contains("doskey ls=call")
            && !cmd_integration.contains("doskey dir=")
            && cmd_listing.contains("Update-FormatData -PrependPath")
            && cmd_listing.contains("Get-ChildItem @parameters | Format-Table"),
        "CMD integration does not preserve shell identity, semantic prompts, native DIR, and icon-aware ls",
    )?;
    let vt_handler = read(&root().join("rio-vt/src/performer/handler.rs"))?;
    let vt_parser = read(&root().join("rio-vt/src/performer/parser/mod.rs"))?;
    require(
        vt_handler.contains(r#""TN" | "name" => Some("automexia".to_string())"#)
            && vt_handler.contains("MAX_XTGETTCAP_REQUEST_LEN")
            && vt_handler.contains("MAX_APC_SEQUENCE_LEN")
            && vt_handler.contains("warn_control_string_discard")
            && vt_handler.contains("further reports are exponentially rate-limited")
            && !vt_handler.contains("[unhandled osc_dispatch]")
            && vt_parser.contains("MAX_OSC_RAW_LEN")
            && vt_parser.contains("osc_overflowed")
            && vt_parser.contains("warn_oversized_osc")
            && vt_parser.contains("oversized_osc_is_dropped_and_next_sequence_recovers")
            && vt_parser.contains("cancelled_osc_is_not_dispatched_and_next_sequence_recovers")
            && vt_handler.contains("cancelled_xtgettcap_is_not_dispatched_and_next_request_recovers")
            && vt_handler.contains("cancelled_apc_is_not_dispatched_and_next_request_recovers"),
        "PTY control-string identity, hard bounds, or discard/recovery contracts are missing",
    )?;
    let application = read(&app.join("src/application.rs"))?;
    let confirm_quit = read(&app.join("src/renderer/confirm_quit.rs"))?;
    let modal_renderer = read(&root().join("sugarloaf/src/renderer/mod.rs"))?;
    let modal_sugarloaf = read(&root().join("sugarloaf/src/sugarloaf.rs"))?;
    let modal_text = read(&root().join("sugarloaf/src/text.rs"))?;
    require(
        palette.contains("sugarloaf.begin_modal_layer()")
            && palette.contains("sugarloaf.end_modal_layer()")
            && !palette.contains("text_mut().clear()")
            && confirm_quit.contains("const SCRIM:")
            && confirm_quit.contains("ConfirmQuitAction")
            && confirm_quit.contains("hit_test_requires_an_active_explicit_button")
            && confirm_quit.contains("layout_stays_inside_extreme_viewports")
            && renderer_root.contains("if self.confirm_quit.is_active()")
            && application.contains("confirm_quit.hit_test(")
            && screen.contains(r#""confirm-quit" =>"#)
            && screen.contains(r#""dismiss-modal" =>"#)
            && screen.contains(r#""confirm_quit_active": window.confirm_quit_active"#)
            && modal_renderer.contains("fn finish_composition_phases(")
            && modal_renderer.contains(
                "modal_primitives_are_physically_appended_after_base_primitives",
            )
            && modal_renderer.contains("pub fn render_modal")
            && modal_sugarloaf.contains("load: wgpu::LoadOp::Load")
            && modal_sugarloaf.contains("self.renderer.render_modal")
            && modal_text.contains("modal_instance_buffers"),
        "palette and close confirmation lack exclusive input, opaque responsive surfaces, topmost cross-backend composition, or native observability",
    )?;
    require(
        application.contains("enum RuntimeConfigReload")
            && application.contains("RuntimeConfigReload::KeepLastGood")
            && application.contains("prepare_runtime_config_reload")
            && application.contains("prepared_font_library")
            && application.contains("prepare_runtime_font_reload")
            && application.contains("missing_font_reload_is_rejected_before_live_state_mutation")
            && application.contains("replace_quake_hotkeys")
            && !application.contains(
                "Err(error) => (rio_backend::config::Config::default(), Some(error))",
            ),
        "runtime configuration reload can replace last-known-good state after a load failure",
    )?;
    let global_hotkey = read(&app.join("src/global_hotkey.rs"))?;
    require(
        global_hotkey.contains("replace_registered_hotkeys")
            && global_hotkey.contains("parse_quake_hotkeys")
            && global_hotkey.contains("partial_hotkey_addition_is_rolled_back_before_removal")
            && global_hotkey.contains("failed_hotkey_removal_rolls_back_new_registration"),
        "runtime global-hotkey reload lacks validation, transactional replacement, or rollback tests",
    )?;
    let image_model = read(&root().join("automexia-image/src/lib.rs"))?;
    let image_preview = read(&app.join("src/image_preview.rs"))?;
    let image_renderer = read(&root().join("sugarloaf/src/renderer/mod.rs"))?;
    let image_sugarloaf = read(&root().join("sugarloaf/src/sugarloaf.rs"))?;
    let xtask_source = read(&root().join("tools/xtask/src/main.rs"))?;
    let ci_workflow = read(&root().join(".github/workflows/ci.yml"))?;
    let mouse = read(&app.join("src/mouse/mod.rs"))?;
    let native_resize =
        read(&root().join("tests/integration/resize-stress-windows.ps1"))?;
    require(
        image_model.contains("pub fn image_path_tokens_in_line")
            && image_model.contains("row_tokenizer_supports_icons_quotes_unicode_and_multiple_images")
            && image_preview.contains("Duration::from_millis(100)")
            && image_preview.contains("pub fn dismiss_hover")
            && image_preview.contains("pub fn is_pinned")
            && screen.contains("pub fn activate_image_preview_at_pointer")
            && screen.contains("fn navigate_image_preview")
            && screen.contains("NamedKey::ArrowDown")
            && screen.contains("NamedKey::Escape")
            && application.contains("image_preview_click_latched")
            && mouse.contains("pub image_preview_click_latched: bool")
            && native_resize.contains("native image hover click and arrow browsing")
            && native_resize.contains("Plain hover did not decode")
            && native_resize.contains("Click-to-pin and Right Arrow")
            && image_model.contains("every_enabled_raster_codec_decodes_with_exact_rgba_accounting")
            && image_model.contains("deterministic_malformed_and_mutated_input_storm_preserves_all_bounds")
            && image_model.contains("repeated_decode_and_cache_hits_do_not_retain_file_handles_or_write_sidecars")
            && image_model.contains("cache_accounting_remains_bounded_under_replacement_storms")
            && image_preview.contains("native_image_resource_stats")
            && image_preview.contains("thumbnail_cache_entries")
            && image_sugarloaf.contains("pub struct NativeImageResourceStats")
            && image_renderer.contains("native_image_texture_usage")
            && image_renderer.contains("saturating_sub(old.bytes)")
            && native_resize.contains("ImagePreviewLifecycleCycles")
            && native_resize.contains("Test-AutomexiaImageResources")
            && native_resize.contains("image_preview_lifecycle = [ordered]@{")
            && native_resize.contains("CompositingMode.SourceCopy")
            && native_resize.contains("image_preview_pixels = [ordered]@{")
            && xtask_source.contains("verify_native_image_backend_equivalence")
            && ci_workflow.contains("cargo xtask test image-rendering")
            && usage().contains("test image-rendering [--native-gui]"),
        "local image quick look must preserve bounded discovery/decode, exact CPU/GPU lifecycle accounting, repeated leak checks, native pixels, and one required contributor gate",
    )?;
    require(
        native_resize.contains("[string]$ModalCaptureDirectory")
            && native_resize.contains("topmost command palette composition")
            && native_resize.contains("topmost close confirmation composition")
            && native_resize.contains("exclusive modal ownership")
            && native_resize.contains("modal_composition = [ordered]@{")
            && native_resize.contains("SetCaptureTopmost($window, $true)"),
        "native Windows stress does not verify exclusive palette/quit ownership and real composited modal frames",
    )?;
    let control_string_fuzz =
        read(&root().join("fuzz/fuzz_targets/control_string_bounds.rs"))?;
    let nightly = read(&root().join(".github/workflows/nightly.yml"))?;
    require(
        control_string_fuzz.contains("retained_limit + 257")
            && control_string_fuzz.contains("Automexia recovered")
            && nightly.contains("control_string_bounds"),
        "hostile control-string fuzz coverage is missing from the nightly matrix",
    )?;
    let image_decoder_fuzz = read(&root().join("fuzz/fuzz_targets/image_decoder.rs"))?;
    require(
        image_decoder_fuzz.contains("decode_bounded_bytes")
            && image_decoder_fuzz.contains("image_path_tokens_in_line")
            && image_decoder_fuzz.contains("path_token_at_line")
            && read(&root().join("automexia-image/src/lib.rs"))?
                .contains("bytes.len() as u64 > MAX_FILE_BYTES")
            && nightly.contains("image_decoder")
            && nightly.contains("rustup toolchain install nightly --profile minimal")
            && nightly.contains("cargo +nightly fuzz run")
            && nightly.contains("-rss_limit_mb=768 -timeout=15")
            && nightly.contains("cargo +nightly test -p automexia-image --lib")
            && xtask_source.contains("mktemp -d /tmp/automexia-image-fuzz.XXXXXX")
            && xtask_source.contains("fuzz_workspace/corpus")
            && xtask_source.contains("source_root=$1")
            && xtask_source.contains(r#"tar -C \"$source_root\""#)
            && xtask_source.contains(r#"cd \"$fuzz_workspace/source\""#)
            && xtask_source.contains("tempfile::Builder::new()")
            && xtask_source.contains("workspace.path().join(\"corpus\")")
            && usage().contains("test image-decoder-fuzz [--seconds N]"),
        "bounded image decoder/token fuzzing must use explicit nightly, sanitizer/RSS/time limits, disposable build/corpus storage, and a supported local runner",
    )?;
    let wsl_development = read(&root().join("docs/WSL-DEVELOPMENT.md"))?;
    require(
        xtask_source.contains("fn wsl_windows_drive")
            && xtask_source.contains("fn report_workspace_io_health")
            && xtask_source.contains("fn require_native_wsl_workspace")
            && xtask_source.contains("AUTOMEXIA_ALLOW_SLOW_WSL_MOUNT")
            && xtask_source.contains("require_native_wsl_workspace(\"the exhaustive verification gate\")")
            && xtask_source.contains("require_native_wsl_workspace(\"the persistent Automexia application build\")")
            && xtask_source.contains("require_native_wsl_workspace(\"the Phase 0 QA evidence gate\")")
            && wsl_development.contains("## Supported dual-native layout")
            && wsl_development.contains("## Workflow safeguards")
            && wsl_development.contains("## Windows-triggered fuzzing"),
        "WSL-native workspace detection, heavy-workflow preflight, or contributor guidance is incomplete",
    )?;

    let devops_manifest = read(&root().join("automexia-devops/src/lib.rs"))?;
    let devops_capabilities = devops_manifest
        .split("pub const MANIFEST")
        .nth(1)
        .and_then(|manifest| manifest.split("};").next())
        .ok_or("DevOps manifest block is missing")?;
    require(
        !app.join("src/automexia/builtins/devops/context.rs")
            .exists()
            && !app.join("src/automexia/builtins/devops/model.rs").exists()
            && !app
                .join("src/automexia/builtins/devops/semantics.rs")
                .exists(),
        "extracted DevOps implementation files still exist inside the frontend",
    )?;
    for capability in [
        "Capability::FilesystemRead",
        "Capability::EnvironmentRead",
        "Capability::TerminalOutputRead",
        "Capability::UiOverlay",
    ] {
        require(
            devops_capabilities.contains(capability),
            &format!("DevOps manifest is missing {capability}"),
        )?;
    }
    for excessive in [
        "Capability::ProcessSpawn",
        "Capability::Network",
        "Capability::Clipboard",
    ] {
        require(
            !devops_capabilities.contains(excessive),
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

/// Recognize both direct assignment and allocation-preserving `clone_from`
/// snapshots. Whitespace is deliberately ignored so rustfmt layout changes do
/// not weaken or spuriously break this architecture invariant.
fn snapshots_renderable_field(renderer: &str, field: &str) -> bool {
    let compact: String = renderer
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let destination = format!("renderable_content.{field}");

    compact.contains(&format!("{destination}="))
        || compact.contains(&format!("{destination}.clone_from("))
        || compact.contains(&format!(
            "sync_optional_metadata(&mutcontext.{destination},"
        ))
        || compact.contains(&format!("sync_optional_metadata(&mut{destination},"))
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
            "rio-vt/src/performer/handler.rs",
            r#""TN" | "name" => Some("automexia".to_string())"#.to_owned(),
        ),
        (
            "rio-vt/src/error/mod.rs",
            "Error initializing Automexia Terminal".to_owned(),
        ),
        (
            "rio-window/src/platform_impl/windows/event_loop.rs",
            "Close Automexia Terminal?".to_owned(),
        ),
        (
            "rio-window/src/platform_impl/macos/app_delegate.rs",
            "Quit Automexia Terminal?".to_owned(),
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

    for path in [
        "rio-vt/src/error/mod.rs",
        "rio-window/src/platform_impl/windows/event_loop.rs",
        "rio-window/src/platform_impl/macos/app_delegate.rs",
    ] {
        let content = read(&root.join(path))?;
        for stale in ["Rio terminal", "Rio will proceed", "Close Rio", "Quit Rio"] {
            require(
                !content.contains(stale),
                &format!("{path} retains user-facing inherited identity {stale:?}"),
            )?;
        }
    }

    let scopes = [
        root.join("apps/automexia-terminal"),
        root.join("packaging"),
        root.join("shell-integration"),
        root.join("sugarloaf"),
        root.join("teletypewriter"),
        root.join("misc"),
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
    let forbidden_product_phrases = [
        "inside a Rio window",
        "Run inside a Rio window",
        "Rio's terminal app",
        "NULL restores Rio's default theme",
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
            for phrase in forbidden_product_phrases {
                if content.contains(phrase) {
                    failures.push(format!(
                        "{} contains forbidden user-facing Rio product phrase {phrase:?}",
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
        ".config/dotnet-tools.json",
        "packaging/windows/automexia.wxs",
        "packaging/windows/automexia-arm64.wxs",
        "packaging/macos/Info.plist",
        "packaging/linux/automexia-terminal.desktop",
        "packaging/linux/io.github.AmjedAllaya.AutomexiaTerminal.metainfo.xml",
        "packaging/linux/nfpm.yaml",
        "packaging/linux/copyright",
        "packaging/linux/changelog.Debian",
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
    let dotnet_tools = read(&root.join(".config/dotnet-tools.json"))?;
    let arm64_wix = read(&root.join("packaging/windows/automexia-arm64.wxs"))?;
    require(
        dotnet_tools.contains(r#""wix""#)
            && dotnet_tools.contains(r#""version": "5.0.2""#)
            && arm64_wix.contains("ProgramFiles6432Folder")
            && arm64_wix.contains("Bitness=\"always64\"")
            && arm64_wix.contains("AutomexiaExecutable")
            && arm64_wix.contains("Software\\Classes\\automexia")
            && arm64_wix.contains("A4783F75-5705-4F89-B0BF-A74562D6601D"),
        "ARM64 MSI must use the pinned WiX 5 tool, 64-bit directories/components, stable upgrade identity, executable, context menu, and URL protocol",
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
        terminfo.contains("automexia|")
            && terminfo.contains("xterm-automexia|")
            && terminfo.contains(r"Sync=\E[?2026%?%p1%{1}%-%tl%eh%;"),
        "terminfo must define both Automexia names and advertise synchronized updates",
    )?;
    let linux_package = read(&root.join("packaging/linux/nfpm.yaml"))?;
    let linux_smoke = read(&root.join("tools/ci/test_linux_package.sh"))?;
    let linux_copyright = read(&root.join("packaging/linux/copyright"))?;
    let debian_changelog = read(&root.join("packaging/linux/changelog.Debian"))?;
    require(
        linux_package.contains("AmjedAllaya@users.noreply.github.com")
            && linux_package.contains("AUTOMEXIA_CHANGELOG")
            && linux_package.contains("automexia.1.gz")
            && linux_package.contains("/usr/share/doc/automexia-terminal/copyright")
            && linux_package.contains("mode: 0644")
            && linux_package.contains("libfontconfig1")
            && linux_copyright.contains("https://github.com/raphamorim/rio")
            && linux_copyright.contains("Permission is hereby granted")
            && debian_changelog.contains(&format!("({}-1)", identity.version)),
        "Linux packages must contain valid maintainer/description metadata, compressed docs, explicit non-executable modes, runtime dependencies, and attributed copyright",
    )?;
    require(
        linux_smoke.contains("package_dir=$(realpath \"$package_dir\")"),
        "Linux package smoke must normalize the package directory before passing local artifacts to apt",
    )?;
    require(
        linux_smoke.contains("rpm -qpl \"$rpm\" | grep -Fx '/usr/bin/automexia' >/dev/null")
            && !linux_smoke.contains("rpm2cpio \"$rpm\" | cpio"),
        "Linux RPM smoke must query the package manifest directly instead of relying on a fragile cpio pipeline",
    )?;
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

    let binary = release_binary_in(&cargo_target_dir(), target, &identity.executable);
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
        if target == "aarch64-pc-windows-msvc" {
            package_windows_arm64(&identity, &binary, &output, target)?;
        } else {
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
        }
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

fn package_windows_arm64(
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
    command.args(["-d", &format!("ProductVersion={}", identity.version)]);
    command.args(["-d", &define("BinaryPath", &binary)]);
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
    for tool in ["scdoc", "gzip", "tic", "nfpm"] {
        require(
            command_on_path(tool),
            &format!("Linux packaging requires `{tool}` on PATH"),
        )?;
    }
    let rendered_manpage = output.join("automexia.1");
    let source = File::open(root().join("packaging/linux/automexia.1.scd"))
        .map_err(|error| format!("could not open manpage source: {error}"))?;
    let rendered = File::create(&rendered_manpage).map_err(|error| {
        format!("could not create {}: {error}", rendered_manpage.display())
    })?;
    let mut scdoc = Command::new("scdoc");
    scdoc
        .stdin(Stdio::from(source))
        .stdout(Stdio::from(rendered))
        .current_dir(root());
    run_command(scdoc, "scdoc")?;

    let manpage = output.join("automexia.1.gz");
    gzip_file(&rendered_manpage, &manpage, "compressed manpage")?;
    let changelog = output.join("changelog.gz");
    gzip_file(
        &root().join("CHANGELOG.md"),
        &changelog,
        "compressed changelog",
    )?;

    let debian_changelog = output.join("changelog.Debian.gz");
    gzip_file(
        &root().join("packaging/linux/changelog.Debian"),
        &debian_changelog,
        "compressed Debian changelog",
    )?;

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
        .env("AUTOMEXIA_CHANGELOG", &changelog)
        .env("AUTOMEXIA_DEBIAN_CHANGELOG", &debian_changelog)
        .env("AUTOMEXIA_TERMINFO_ROOT", &terminfo)
        .current_dir(root());
        run_command(nfpm, &format!("nFPM {format}"))?;
    }
    portable_archive(identity, target, binary, output, "tar.gz")
}

fn gzip_file(source: &Path, destination: &Path, label: &str) -> TaskResult {
    let input = File::open(source)
        .map_err(|error| format!("could not open {}: {error}", source.display()))?;
    let output = File::create(destination).map_err(|error| {
        format!("could not create {}: {error}", destination.display())
    })?;
    let mut gzip = Command::new("gzip");
    gzip.args(["-n", "-9", "-c"])
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output));
    run_command(gzip, label)
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
    display_relative_or_absolute(path)
}

fn display_relative_or_absolute(path: &Path) -> String {
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
        assert!(usage().contains("storage"));
        assert!(usage().contains("verify architecture"));
        assert!(usage().contains("test conformance"));
        assert!(usage().contains("test resize-stress [--native-gui]"));
        assert!(usage().contains("test image-rendering [--native-gui]"));
        assert!(usage().contains("test image-decoder-fuzz [--seconds N]"));
        assert!(usage().contains("test session-clone [--native-windows|--native-wsl]"));
        assert!(usage().contains("release --version"));
        assert!(usage().contains("verify all"));
    }

    #[test]
    fn path_probe_finds_tools_without_invoking_a_version_flag() {
        assert!(command_on_path("cargo"));
        assert!(!command_on_path("automexia-tool-that-cannot-exist-7f8332"));
    }

    #[test]
    fn fuzz_campaign_duration_is_strictly_bounded() {
        assert_eq!(parse_fuzz_seconds("1").unwrap(), 1);
        assert_eq!(parse_fuzz_seconds("86400").unwrap(), 86_400);
        assert!(parse_fuzz_seconds("0").is_err());
        assert!(parse_fuzz_seconds("86401").is_err());
        assert!(parse_fuzz_seconds("forever").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_fuzz_script_is_nightly_bounded_and_fully_disposable() {
        let script = image_decoder_fuzz_wsl_script(37);
        assert!(script.contains("cargo +nightly fuzz run image_decoder"));
        assert!(script.contains("-max_total_time=37"));
        assert!(script.contains("-rss_limit_mb=768 -timeout=15"));
        assert!(script.contains("mktemp -d /tmp/automexia-image-fuzz.XXXXXX"));
        assert!(script.contains("$fuzz_workspace/target"));
        assert!(script.contains("$fuzz_workspace/corpus"));
        assert!(script.contains("source_root=$1"));
        assert!(script.starts_with("set -euo pipefail;"));
        assert!(script.contains("tar -C \"$source_root\""));
        assert!(script.contains("--exclude='./fuzz/corpus'"));
        assert!(script.contains("--exclude='./fuzz/artifacts'"));
        assert!(script.contains("--exclude='./target'"));
        assert!(script.contains("cd \"$fuzz_workspace/source\""));
        assert!(script.contains("trap 'rm -rf -- \"$fuzz_workspace\"'"));
        assert!(!script.contains("CARGO_TARGET_DIR=\"$fuzz_workspace\" cargo"));
    }

    #[test]
    fn wsl_windows_drive_detection_rejects_cross_filesystem_build_roots() {
        assert_eq!(
            wsl_windows_drive(Path::new("/mnt/d/project"), true),
            Some('D')
        );
        assert_eq!(wsl_windows_drive(Path::new("/mnt/c"), true), Some('C'));
        assert_eq!(
            wsl_windows_drive(Path::new("/home/user/project"), true),
            None
        );
        assert_eq!(
            wsl_windows_drive(Path::new("/mnt/wslg/project"), true),
            None
        );
        assert_eq!(wsl_windows_drive(Path::new("/mnt/d/project"), false), None);
    }

    #[test]
    fn architecture_contract_self_verifies() {
        verify_architecture().unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_shell_health_parses_versions_and_flags_only_legacy_history_stack() {
        let legacy = parse_windows_shell_health(
            "noise\r\nhost=5.1.26100.7019\r\npsreadline=2.0.0\r\n",
        );
        assert_eq!(legacy.host_version.as_deref(), Some("5.1.26100.7019"));
        assert_eq!(legacy.psreadline_version.as_deref(), Some("2.0.0"));
        assert!(windows_shell_history_is_legacy(&legacy));

        let current = parse_windows_shell_health("host=7.5.3\npsreadline=2.3.6\n");
        assert!(!windows_shell_history_is_legacy(&current));
        let missing = parse_windows_shell_health("host=5.1.26100.7019\n");
        assert!(!windows_shell_history_is_legacy(&missing));
    }

    #[test]
    fn every_launch_plan_provisions_shells_immediately_before_spawn() {
        assert_eq!(
            launch_plan(LaunchMode::Verified),
            &[
                LaunchPhase::Ready,
                LaunchPhase::InstallShellIntegration,
                LaunchPhase::Launch,
            ]
        );
        assert_eq!(
            launch_plan(LaunchMode::Incremental),
            &[
                LaunchPhase::Build,
                LaunchPhase::Smoke,
                LaunchPhase::InstallShellIntegration,
                LaunchPhase::Launch,
            ]
        );
        for mode in [LaunchMode::Verified, LaunchMode::Incremental] {
            let phases = launch_plan(mode);
            assert_eq!(phases.last(), Some(&LaunchPhase::Launch));
            assert_eq!(
                phases.get(phases.len() - 2),
                Some(&LaunchPhase::InstallShellIntegration)
            );
        }
    }

    #[test]
    fn automatic_installers_are_quiet_and_repository_owned() {
        let (windows_program, windows_args) =
            shell_integration_command(HostShellPlatform::Windows);
        assert_eq!(windows_program, "powershell");
        assert!(windows_args.contains(&"shell-integration/install-windows.ps1"));
        assert!(windows_args.contains(&"-Quiet"));
        assert!(windows_args.contains(&"-NonInteractive"));

        let (unix_program, unix_args) =
            shell_integration_command(HostShellPlatform::Unix);
        assert_eq!(unix_program, "sh");
        assert_eq!(unix_args, &["shell-integration/install-unix.sh", "--quiet"]);
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

    #[test]
    fn metadata_snapshot_check_accepts_assignments_and_reused_allocations() {
        assert!(snapshots_renderable_field(
            "renderable_content.shell_integration = terminal.enabled;",
            "shell_integration"
        ));
        assert!(snapshots_renderable_field(
            "context\n    .renderable_content\n    .current_directory\n    .clone_from(&terminal.current_directory);",
            "current_directory"
        ));
        assert!(!snapshots_renderable_field(
            "let current_directory = terminal.current_directory.clone();",
            "current_directory"
        ));
    }

    #[test]
    fn debug_binary_respects_relative_and_absolute_cargo_target_dirs() {
        let invocation_directory = Path::new("repo/subdirectory");
        assert_eq!(
            resolve_target_dir(invocation_directory, None),
            root().join("target")
        );
        assert_eq!(
            resolve_target_dir(invocation_directory, Some(OsStr::new("target/isolated"))),
            invocation_directory.join("target/isolated")
        );

        let absolute = root().join("target").join("isolated-absolute");
        assert_eq!(
            resolve_target_dir(invocation_directory, Some(absolute.as_os_str())),
            absolute
        );
    }

    #[test]
    fn release_binary_uses_the_effective_cargo_target_directory() {
        let target_dir = Path::new("D:/isolated-cargo-target");
        assert_eq!(
            release_binary_in(target_dir, "x86_64-pc-windows-msvc", "automexia"),
            target_dir
                .join("x86_64-pc-windows-msvc")
                .join("release")
                .join("automexia.exe")
        );
        assert_eq!(
            release_binary_in(target_dir, "x86_64-unknown-linux-gnu", "automexia"),
            target_dir
                .join("x86_64-unknown-linux-gnu")
                .join("release")
                .join("automexia")
        );
    }

    #[test]
    fn verification_target_is_an_exact_direct_child() {
        let parent = root().join("target");
        let name = verification_target_name(42, 1234);
        assert_eq!(
            verified_target_child(&parent, &name).unwrap(),
            parent.join(&name)
        );
        assert!(is_verification_target_name(OsStr::new(&name)));
        assert!(!is_verification_target_name(OsStr::new(
            "automexia-verification-v1"
        )));
        assert!(!is_verification_target_name(OsStr::new(
            "automexia-verification-v1-active-run"
        )));
        assert!(verified_target_child(&parent, "../outside").is_err());
        assert!(verified_target_child(&parent, "nested/child").is_err());
    }

    #[test]
    fn runtime_binary_names_are_unique_and_platform_correct() {
        assert_eq!(
            generation_binary_name("automexia", 42, 1234, true),
            "automexia-42-1234.exe"
        );
        assert_eq!(
            generation_binary_name("automexia", 42, 1234, false),
            "automexia-42-1234"
        );
    }

    #[test]
    fn storage_units_are_human_readable() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
        assert_eq!(format_bytes(GIB), "1.00 GiB");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn canonical_windows_paths_are_linker_compatible() {
        assert_eq!(
            normalize_canonical_path(PathBuf::from(r"\\?\D:\workspace\target")),
            PathBuf::from(r"D:\workspace\target")
        );
        assert_eq!(
            normalize_canonical_path(PathBuf::from(r"\\?\UNC\server\share\target")),
            PathBuf::from(r"\\server\share\target")
        );
    }
}
