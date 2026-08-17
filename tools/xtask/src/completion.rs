use super::TaskResult;
use process_wrap::std::CommandWrap;
#[cfg(windows)]
use process_wrap::std::JobObject;
#[cfg(unix)]
use process_wrap::std::ProcessGroup;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

const SCHEMA_VERSION: u8 = 1;
const PROVIDER_DEADLINE: Duration = Duration::from_millis(750);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_PROVIDER_OUTPUT: usize = 1024 * 1024;
const MAX_PROVIDER_STDERR: usize = 256 * 1024;
const MAX_VERSION_OUTPUT: usize = 16 * 1024;
const MAX_CONFIG_PATH_BYTES: usize = 4096;
const MAX_ARTIFACT_BYTES: usize = MAX_PROVIDER_OUTPUT + 64 * 1024;
// A refresh briefly publishes the previous and candidate digests together so
// either complete artifact remains usable if the process is interrupted. The
// steady-state sidecar still contains exactly one digest.
const MAX_DIGEST_BYTES: usize = 192;
const MAX_METADATA_BYTES: usize = 64 * 1024;
const MAX_OVERRIDE_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompletionShell {
    PowerShell,
    Bash,
    Zsh,
    Fish,
    Cmd,
}

impl CompletionShell {
    const ALL: [Self; 5] = [
        Self::PowerShell,
        Self::Bash,
        Self::Zsh,
        Self::Fish,
        Self::Cmd,
    ];

    fn parse(value: &str) -> TaskResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "powershell" | "pwsh" => Ok(Self::PowerShell),
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "cmd" => Ok(Self::Cmd),
            _ => Err(format!(
                "unsupported shell {value:?}; expected powershell, bash, zsh, fish, or cmd"
            )),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::PowerShell => "powershell",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Cmd => "cmd",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::PowerShell => "ps1",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Cmd => "cmd",
        }
    }

    fn supports_programmable_completion(self) -> bool {
        !matches!(self, Self::Cmd)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Provider {
    Git,
    Docker,
    Kubernetes,
    OpenShift,
    Helm,
    Terraform,
    OpenTofu,
    Aws,
    Azure,
    Gcp,
    OpenSsh,
}

impl Provider {
    const ALL: [Self; 11] = [
        Self::Git,
        Self::Docker,
        Self::Kubernetes,
        Self::OpenShift,
        Self::Helm,
        Self::Terraform,
        Self::OpenTofu,
        Self::Aws,
        Self::Azure,
        Self::Gcp,
        Self::OpenSsh,
    ];

    fn parse(value: &str) -> TaskResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "git" => Ok(Self::Git),
            "docker" => Ok(Self::Docker),
            "kubernetes" | "kubectl" => Ok(Self::Kubernetes),
            "openshift" | "oc" => Ok(Self::OpenShift),
            "helm" => Ok(Self::Helm),
            "terraform" => Ok(Self::Terraform),
            "opentofu" | "tofu" => Ok(Self::OpenTofu),
            "aws" => Ok(Self::Aws),
            "azure" | "az" => Ok(Self::Azure),
            "gcp" | "gcloud" => Ok(Self::Gcp),
            "openssh" | "ssh" => Ok(Self::OpenSsh),
            _ => Err(format!("unknown completion provider {value:?}")),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Docker => "docker",
            Self::Kubernetes => "kubernetes",
            Self::OpenShift => "openshift",
            Self::Helm => "helm",
            Self::Terraform => "terraform",
            Self::OpenTofu => "opentofu",
            Self::Aws => "aws",
            Self::Azure => "azure",
            Self::Gcp => "gcp",
            Self::OpenSsh => "openssh",
        }
    }

    fn command(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Docker => "docker",
            Self::Kubernetes => "kubectl",
            Self::OpenShift => "oc",
            Self::Helm => "helm",
            Self::Terraform => "terraform",
            Self::OpenTofu => "tofu",
            Self::Aws => "aws_completer",
            Self::Azure => "az",
            Self::Gcp => "gcloud",
            Self::OpenSsh => "ssh",
        }
    }

    fn support(self, shell: CompletionShell) -> ProviderSupport {
        use ProviderSupport::{ExternalOwned, Generated, ManualConsent, Unsupported};
        if !shell.supports_programmable_completion() {
            return Unsupported("CMD has no context-aware programmable completion API");
        }
        match self {
            Self::Docker
                if matches!(
                    shell,
                    CompletionShell::Bash | CompletionShell::Zsh | CompletionShell::Fish
                ) =>
            {
                Generated
            }
            Self::Kubernetes | Self::OpenShift | Self::Helm => Generated,
            Self::Terraform | Self::OpenTofu => ManualConsent,
            Self::Git | Self::Aws | Self::Azure | Self::Gcp | Self::OpenSsh => {
                ExternalOwned
            }
            _ => Unsupported(
                "the installed provider does not publish this shell generator",
            ),
        }
    }

    fn generator_args(self, shell: CompletionShell) -> TaskResult<Vec<OsString>> {
        if self.support(shell) != ProviderSupport::Generated {
            return Err(format!(
                "{} completion for {} is not a cacheable official generator",
                self.id(),
                shell.id()
            ));
        }
        let args = match self {
            Self::Docker | Self::Kubernetes | Self::OpenShift | Self::Helm => {
                vec![OsString::from("completion"), OsString::from(shell.id())]
            }
            _ => unreachable!("support() admitted only reviewed generators"),
        };
        Ok(args)
    }

    fn version_args(self) -> &'static [&'static str] {
        match self {
            Self::Docker => &["--version"],
            Self::Kubernetes => &["version", "--client=true"],
            Self::OpenShift => &["version", "--client"],
            Self::Helm => &["version", "--short"],
            _ => &["--version"],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderSupport {
    Generated,
    ExternalOwned,
    ManualConsent,
    Unsupported(&'static str),
}

impl ProviderSupport {
    fn label(self) -> &'static str {
        match self {
            Self::Generated => "explicit-refresh",
            Self::ExternalOwned => "native/provider-owned",
            Self::ManualConsent => "manual-consent-required",
            Self::Unsupported(_) => "unsupported",
        }
    }
}

#[derive(Debug)]
struct Captured {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    elapsed: Duration,
}

#[derive(Debug, Deserialize, Serialize)]
struct ArtifactMetadata {
    schema_version: u8,
    provider: String,
    command: String,
    shell: String,
    executable: String,
    executable_length: u64,
    executable_modified_unix_ms: u128,
    tool_version: String,
    source_sha256: String,
    artifact_sha256: String,
    generated_unix_ms: u128,
    deadline_ms: u128,
    output_limit_bytes: usize,
    native_override: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostPlatform {
    Windows,
    MacOs,
    Unix,
}

#[derive(Debug, Eq, PartialEq)]
enum ArtifactHealth {
    Missing,
    Healthy,
    Invalid(String),
}

#[derive(Debug, Eq, PartialEq)]
struct ExecutableFingerprint {
    length: u64,
    modified_unix_ms: u128,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    creation_time: u64,
    #[cfg(windows)]
    last_write_time: u64,
    #[cfg(windows)]
    attributes: u32,
}

#[derive(Debug)]
struct ParsedOperation {
    provider: Provider,
    shell: CompletionShell,
    allow_native_override: bool,
}

pub(super) fn dispatch(args: &[String]) -> TaskResult {
    match args {
        [command] if command == "doctor" => report_health(),
        [command] if command == "enable" => set_enabled(true),
        [command] if command == "disable" => set_enabled(false),
        [command, rest @ ..] if command == "refresh" => refresh(parse_operation(rest)?),
        [command, rest @ ..] if command == "remove" => remove(parse_operation(rest)?),
        _ => Err(completion_usage()),
    }
}

fn completion_usage() -> String {
    "usage: cargo xtask completion <doctor|enable|disable|refresh --provider ID --shell SHELL [--allow-native-override]|remove --provider ID --shell SHELL>".into()
}

fn parse_operation(args: &[String]) -> TaskResult<ParsedOperation> {
    let mut provider = None;
    let mut shell = None;
    let mut allow_native_override = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--provider" if index + 1 < args.len() => {
                if provider.is_some() {
                    return Err("--provider may be specified only once".into());
                }
                provider = Some(Provider::parse(&args[index + 1])?);
                index += 2;
            }
            "--shell" if index + 1 < args.len() => {
                if shell.is_some() {
                    return Err("--shell may be specified only once".into());
                }
                shell = Some(CompletionShell::parse(&args[index + 1])?);
                index += 2;
            }
            "--allow-native-override" => {
                if allow_native_override {
                    return Err(
                        "--allow-native-override may be specified only once".into()
                    );
                }
                allow_native_override = true;
                index += 1;
            }
            value => {
                return Err(format!(
                    "unknown completion option {value:?}\n{}",
                    completion_usage()
                ))
            }
        }
    }
    let provider = provider.ok_or_else(completion_usage)?;
    let shell = shell.ok_or_else(completion_usage)?;
    if allow_native_override && shell != CompletionShell::PowerShell {
        return Err(
            "--allow-native-override is PowerShell-only; Bash, Zsh, and Fish detect native definitions at shell startup".into(),
        );
    }
    Ok(ParsedOperation {
        provider,
        shell,
        allow_native_override,
    })
}

pub(super) fn report_health() -> TaskResult {
    let root = completion_root()?;
    let mut report = String::new();
    let _ = writeln!(report, "completion root    {}", root.display());
    let disabled_path = root.join(".disabled");
    let state = match read_regular_bounded(&disabled_path, MAX_OVERRIDE_BYTES) {
        Ok(Some(bytes)) if bytes == b"disabled-by-user-v1\n" => {
            "disabled/native fallback"
        }
        Ok(Some(_)) | Err(_) => "unsafe-state/native fallback",
        Ok(None) => match validate_managed_directory_chain(&root) {
            Ok(()) => "enabled",
            Err(_) => "unsafe-path/native fallback",
        },
    };
    let _ = writeln!(report, "completion state   {}", state);
    for shell in CompletionShell::ALL {
        let mut healthy = 0;
        let mut invalid = 0;
        let mut missing = 0;
        let mut issues = Vec::new();
        for provider in Provider::ALL {
            if provider.support(shell) != ProviderSupport::Generated {
                continue;
            }
            match inspect_artifact(&root, provider, shell) {
                ArtifactHealth::Healthy => healthy += 1,
                ArtifactHealth::Missing => missing += 1,
                ArtifactHealth::Invalid(reason) => {
                    invalid += 1;
                    issues.push(format!("{}/{}={reason}", shell.id(), provider.id()));
                }
            }
        }
        let _ = writeln!(
            report,
            "completion {:<8} healthy={healthy} invalid={invalid} missing={missing}",
            shell.id(),
        );
        for issue in issues {
            let _ = writeln!(report, "completion issue    {issue}");
        }
    }
    for provider in Provider::ALL {
        let available = resolve_executable(provider.command()).is_ok();
        let policies = CompletionShell::ALL
            .iter()
            .map(|shell| provider.support(*shell).label())
            .collect::<Vec<_>>();
        let uniform = policies.iter().all(|policy| policy == &policies[0]);
        let _ = writeln!(
            report,
            "provider {:<11} {:<11} {}",
            provider.id(),
            if available { "available" } else { "missing" },
            if uniform {
                policies[0]
            } else {
                "shell-specific"
            }
        );
    }
    let _ = writeln!(
        report,
        "completion safety  read-only health; provider commands run only through explicit `completion refresh`"
    );
    emit_output(&report)
}

fn emit_output(output: &str) -> TaskResult {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    tolerate_broken_pipe(stdout.write_all(output.as_bytes()))
}

fn tolerate_broken_pipe(result: io::Result<()>) -> TaskResult {
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(format!("could not write completion output: {error}")),
    }
}

fn refresh(operation: ParsedOperation) -> TaskResult {
    if operation.shell == CompletionShell::Cmd {
        return Err("CMD retains native DOSKEY behavior and has no programmable provider completion API".into());
    }
    match operation.provider.support(operation.shell) {
        ProviderSupport::Generated => {}
        ProviderSupport::ExternalOwned => {
            return Err(format!(
                "{} completion is owned by the installed shell/provider package; Automexia diagnoses it but does not replace it",
                operation.provider.id()
            ));
        }
        ProviderSupport::ManualConsent => {
            return Err(format!(
                "{} uses a profile-mutating installer; preview and run the provider's documented install flow explicitly instead of caching it through Automexia",
                operation.provider.id()
            ));
        }
        ProviderSupport::Unsupported(reason) => return Err(reason.into()),
    }
    if operation.shell == CompletionShell::PowerShell && !operation.allow_native_override
    {
        return Err(
            "PowerShell has no public read-only registry for native argument completers; pass --allow-native-override only after reviewing the active shell collision report"
                .into(),
        );
    }

    // Validate and create only the bounded managed destination before granting
    // the provider any process time. A malformed, linked, or unwritable state
    // root must fail without invoking an external executable.
    let root = completion_root()?;
    let (artifact, digest_path, metadata_path, override_path) =
        artifact_paths(&root, operation.provider, operation.shell);
    let directory = artifact
        .parent()
        .ok_or_else(|| "completion artifact has no parent".to_owned())?;
    create_secure_directory(directory)?;

    let executable = resolve_executable(operation.provider.command())?;
    validate_refresh_executable(&executable)?;
    let executable_handle = File::open(&executable)
        .map_err(|error| format!("cannot open {}: {error}", executable.display()))?;
    let executable_metadata = executable_handle
        .metadata()
        .map_err(|error| format!("cannot stat {}: {error}", executable.display()))?;
    if !executable_metadata.is_file() {
        return Err(format!(
            "provider executable is not a regular file: {}",
            executable.display()
        ));
    }
    let executable_fingerprint =
        ExecutableFingerprint::from_metadata(&executable_metadata);
    let version_args = operation
        .provider
        .version_args()
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>();
    let version = run_bounded(
        &executable,
        &version_args,
        PROVIDER_DEADLINE,
        MAX_VERSION_OUTPUT,
        MAX_PROVIDER_STDERR,
    )?;
    require_success("provider version", &version)?;
    revalidate_executable(&executable, &executable_handle, &executable_fingerprint)?;
    let tool_version =
        summarize_version(bounded_utf8(&version.stdout, "provider version")?);
    if tool_version.is_empty() {
        return Err("provider version output was empty".into());
    }

    let args = operation.provider.generator_args(operation.shell)?;
    let generated = run_bounded(
        &executable,
        &args,
        PROVIDER_DEADLINE,
        MAX_PROVIDER_OUTPUT,
        MAX_PROVIDER_STDERR,
    )?;
    require_success("completion generator", &generated)?;
    revalidate_executable(&executable, &executable_handle, &executable_fingerprint)?;
    validate_generated_output(&generated.stdout)?;
    let source_digest = sha256_hex(&generated.stdout);
    let header = generated_header(operation.provider, operation.shell, &source_digest);
    let mut body = header.into_bytes();
    body.extend_from_slice(&generated.stdout);
    if !body.ends_with(b"\n") {
        body.push(b'\n');
    }
    let artifact_digest = sha256_hex(&body);
    let metadata = ArtifactMetadata {
        schema_version: SCHEMA_VERSION,
        provider: operation.provider.id().to_owned(),
        command: operation.provider.command().to_owned(),
        shell: operation.shell.id().to_owned(),
        executable: executable.to_string_lossy().into_owned(),
        executable_length: executable_metadata.len(),
        executable_modified_unix_ms: modified_unix_ms(&executable_metadata),
        tool_version,
        source_sha256: source_digest,
        artifact_sha256: artifact_digest.clone(),
        generated_unix_ms: unix_ms(SystemTime::now()),
        deadline_ms: PROVIDER_DEADLINE.as_millis(),
        output_limit_bytes: MAX_PROVIDER_OUTPUT,
        native_override: operation.allow_native_override,
    };
    let mut metadata_bytes = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| format!("could not serialize completion metadata: {error}"))?;
    metadata_bytes.push(b'\n');

    // Publish a two-digest transition before replacing the executable shell
    // artifact. At every interruption point an adapter can verify either the
    // previous complete artifact or the new candidate.
    let previous_digest = runtime_artifact_digest(&artifact, &digest_path);
    if operation.allow_native_override {
        atomic_write(&override_path, b"explicit-native-override-v1\n")?;
    } else {
        remove_regular_file_if_present(&override_path)?;
    }
    atomic_write(&metadata_path, &metadata_bytes)?;
    let transition =
        transition_digest_bytes(previous_digest.as_deref(), &artifact_digest);
    atomic_write(&digest_path, &transition)?;
    atomic_write(&artifact, &body)?;
    atomic_write(&digest_path, format!("{artifact_digest}\n").as_bytes())?;
    emit_output(&format!(
        "PASS: cached {} completion for {} ({} bytes, generator {}, version {}, {} ms)\n",
        operation.provider.id(),
        operation.shell.id(),
        generated.stdout.len(),
        executable.display(),
        metadata.tool_version,
        generated.elapsed.as_millis()
    ))
}

fn remove(operation: ParsedOperation) -> TaskResult {
    let root = completion_root()?;
    let paths = artifact_paths(&root, operation.provider, operation.shell);
    let directory = paths
        .0
        .parent()
        .ok_or_else(|| "completion artifact has no parent".to_owned())?;
    validate_managed_directory_chain(directory)?;
    for path in [&paths.0, &paths.1, &paths.2, &paths.3] {
        remove_regular_file_if_present(path)?;
    }
    emit_output(&format!(
        "PASS: removed only Automexia's cached {} completion for {}\n",
        operation.provider.id(),
        operation.shell.id()
    ))
}

fn set_enabled(enabled: bool) -> TaskResult {
    let root = completion_root()?;
    create_secure_directory(&root)?;
    let marker = root.join(".disabled");
    if enabled {
        remove_regular_file_if_present(&marker)?;
        emit_output(
            "PASS: managed completion enabled; native collision policy remains authoritative\n",
        )?;
    } else {
        atomic_write(&marker, b"disabled-by-user-v1\n")?;
        emit_output(
            "PASS: managed completion disabled; native shell completion remains unchanged\n",
        )?;
    }
    Ok(())
}

fn completion_root() -> TaskResult<PathBuf> {
    let platform = if cfg!(target_os = "windows") {
        HostPlatform::Windows
    } else if cfg!(target_os = "macos") {
        HostPlatform::MacOs
    } else {
        HostPlatform::Unix
    };
    let root = select_config_root(
        platform,
        env::var_os("AUTOMEXIA_CONFIG_HOME").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
    )?;
    Ok(root.join("generated").join("completion"))
}

fn select_config_root(
    platform: HostPlatform,
    override_root: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
) -> TaskResult<PathBuf> {
    let root = if let Some(override_root) = override_root {
        override_root
    } else {
        match platform {
            HostPlatform::Windows => local_app_data
                .ok_or_else(|| "LOCALAPPDATA is unavailable".to_owned())?
                .join("Automexia")
                .join("Terminal"),
            HostPlatform::MacOs => home
                .ok_or_else(|| "HOME is unavailable".to_owned())?
                .join("Library")
                .join("Application Support")
                .join("io.github.AmjedAllaya.AutomexiaTerminal"),
            HostPlatform::Unix => xdg_config_home
                .or_else(|| home.map(|path| path.join(".config")))
                .ok_or_else(|| "HOME is unavailable".to_owned())?
                .join("automexia"),
        }
    };
    if root.as_os_str().as_encoded_bytes().len() > MAX_CONFIG_PATH_BYTES {
        return Err("completion configuration root exceeds 4096 bytes".into());
    }
    if root.as_os_str().is_empty() {
        return Err("completion configuration root is empty".into());
    }
    if !root.is_absolute() {
        return Err("completion configuration root must be absolute".into());
    }
    #[cfg(windows)]
    if platform == HostPlatform::Windows && !is_local_windows_path(&root) {
        return Err(
            "completion configuration root must be on a local Windows drive".into(),
        );
    }
    Ok(root)
}

fn artifact_paths(
    root: &Path,
    provider: Provider,
    shell: CompletionShell,
) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let directory = root.join(shell.id());
    let name = format!("{}.{}", provider.command(), shell.extension());
    let artifact = directory.join(name);
    let digest = artifact.with_extension(format!("{}.sha256", shell.extension()));
    let metadata = artifact.with_extension(format!("{}.json", shell.extension()));
    let native_override =
        artifact.with_extension(format!("{}.allow-override", shell.extension()));
    (artifact, digest, metadata, native_override)
}

fn inspect_artifact(
    root: &Path,
    provider: Provider,
    shell: CompletionShell,
) -> ArtifactHealth {
    let (artifact_path, digest_path, metadata_path, override_path) =
        artifact_paths(root, provider, shell);
    let Some(directory) = artifact_path.parent() else {
        return ArtifactHealth::Invalid("missing-parent".into());
    };
    if validate_managed_directory_chain(directory).is_err() {
        return ArtifactHealth::Invalid("unsafe-path".into());
    }

    let artifact = match read_regular_bounded(&artifact_path, MAX_ARTIFACT_BYTES) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            let orphaned = [&digest_path, &metadata_path, &override_path]
                .iter()
                .any(|path| fs::symlink_metadata(path).is_ok());
            return if orphaned {
                ArtifactHealth::Invalid("orphaned-sidecar".into())
            } else {
                ArtifactHealth::Missing
            };
        }
        Err(_) => return ArtifactHealth::Invalid("artifact-bounds-or-type".into()),
    };
    let digest = match read_regular_bounded(&digest_path, MAX_DIGEST_BYTES) {
        Ok(Some(bytes)) => bytes,
        _ => return ArtifactHealth::Invalid("digest-missing-or-unsafe".into()),
    };
    let digests = match parse_digest_sidecar(&digest) {
        Ok(digests) => digests,
        Err(reason) => return ArtifactHealth::Invalid(reason.into()),
    };
    let artifact_digest = sha256_hex(&artifact);
    if !digests.iter().any(|digest| *digest == artifact_digest) {
        return ArtifactHealth::Invalid("digest-mismatch".into());
    }

    let metadata = match read_regular_bounded(&metadata_path, MAX_METADATA_BYTES) {
        Ok(Some(bytes)) => bytes,
        _ => return ArtifactHealth::Invalid("metadata-missing-or-unsafe".into()),
    };
    let metadata: ArtifactMetadata = match serde_json::from_slice(&metadata) {
        Ok(metadata) => metadata,
        Err(_) => return ArtifactHealth::Invalid("metadata-invalid".into()),
    };
    let expected_override = shell == CompletionShell::PowerShell;
    if metadata.schema_version != SCHEMA_VERSION
        || metadata.provider != provider.id()
        || metadata.command != provider.command()
        || metadata.shell != shell.id()
        || metadata.artifact_sha256 != artifact_digest
        || metadata.deadline_ms != PROVIDER_DEADLINE.as_millis()
        || metadata.output_limit_bytes != MAX_PROVIDER_OUTPUT
        || metadata.native_override != expected_override
        || metadata.tool_version.is_empty()
        || metadata.executable.is_empty()
    {
        return ArtifactHealth::Invalid("metadata-mismatch".into());
    }
    let source_digest = metadata.source_sha256.as_str();
    if source_digest.len() != 64
        || !source_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || !artifact
            .starts_with(generated_header(provider, shell, source_digest).as_bytes())
    {
        return ArtifactHealth::Invalid("source-provenance-mismatch".into());
    }

    match read_regular_bounded(&override_path, MAX_OVERRIDE_BYTES) {
        Ok(Some(bytes))
            if expected_override && bytes == b"explicit-native-override-v1\n" => {}
        Ok(None) if !expected_override => {}
        _ => return ArtifactHealth::Invalid("native-override-mismatch".into()),
    }
    ArtifactHealth::Healthy
}

fn read_regular_bounded(path: &Path, limit: usize) -> TaskResult<Option<Vec<u8>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!("could not inspect {}: {error}", path.display()))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "managed state is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(format!(
            "managed state exceeds {limit} bytes: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|file| file.take(limit as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!(
            "managed state exceeds {limit} bytes: {}",
            path.display()
        ));
    }
    Ok(Some(bytes))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_digest_sidecar(bytes: &[u8]) -> Result<Vec<&str>, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "digest-encoding")?;
    let digests = text.lines().collect::<Vec<_>>();
    if !(1..=2).contains(&digests.len())
        || digests.iter().any(|digest| !is_sha256_hex(digest))
        || !text.ends_with('\n')
    {
        return Err("digest-format");
    }
    Ok(digests)
}

fn runtime_artifact_digest(artifact: &Path, digest_path: &Path) -> Option<String> {
    let artifact = read_regular_bounded(artifact, MAX_ARTIFACT_BYTES)
        .ok()
        .flatten()?;
    let digest = read_regular_bounded(digest_path, MAX_DIGEST_BYTES)
        .ok()
        .flatten()?;
    let actual = sha256_hex(&artifact);
    parse_digest_sidecar(&digest)
        .ok()?
        .iter()
        .any(|digest| *digest == actual)
        .then_some(actual)
}

fn transition_digest_bytes(previous: Option<&str>, candidate: &str) -> Vec<u8> {
    let mut sidecar = String::new();
    if let Some(previous) = previous.filter(|previous| *previous != candidate) {
        let _ = writeln!(sidecar, "{previous}");
    }
    let _ = writeln!(sidecar, "{candidate}");
    sidecar.into_bytes()
}

fn generated_header(provider: Provider, shell: CompletionShell, digest: &str) -> String {
    let prefix = if shell == CompletionShell::Cmd {
        "rem"
    } else {
        "#"
    };
    format!(
        "{prefix} AUTOMEXIA-MANAGED-COMPLETION schema={SCHEMA_VERSION} provider={} shell={} source-sha256={digest}\n{prefix} Generated explicitly; edit the source provider configuration, not this disposable file.\n",
        provider.id(),
        shell.id()
    )
}

fn validate_generated_output(output: &[u8]) -> TaskResult {
    if output.is_empty() {
        return Err("completion generator returned an empty artifact".into());
    }
    if output.len() > MAX_PROVIDER_OUTPUT {
        return Err(format!(
            "completion generator exceeded the {} byte output ceiling",
            MAX_PROVIDER_OUTPUT
        ));
    }
    let text = bounded_utf8(output, "completion generator")?;
    if text.contains('\0') {
        return Err("completion generator emitted NUL".into());
    }
    if text.chars().any(|character| {
        character.is_control() && !matches!(character, '\n' | '\r' | '\t')
    }) {
        return Err("completion generator emitted unsupported control characters".into());
    }
    if text.chars().any(is_bidi_control) {
        return Err(
            "completion generator emitted bidirectional control characters".into(),
        );
    }
    Ok(())
}

fn require_success(label: &str, captured: &Captured) -> TaskResult {
    if captured.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&captured.stderr);
    let summary =
        sanitize_diagnostic(stderr.lines().next().unwrap_or("no stderr").trim(), 256);
    Err(format!(
        "{label} failed with {} after {} ms: {summary}",
        captured.status,
        captured.elapsed.as_millis()
    ))
}

fn bounded_utf8<'a>(bytes: &'a [u8], label: &str) -> TaskResult<&'a str> {
    std::str::from_utf8(bytes).map_err(|_| format!("{label} output is not valid UTF-8"))
}

fn summarize_version(output: &str) -> String {
    let summary = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    sanitize_diagnostic(&summary, 512)
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn sanitize_diagnostic(value: &str, maximum_chars: usize) -> String {
    value
        .chars()
        .take(maximum_chars)
        .map(|character| {
            if character.is_control() || is_bidi_control(character) {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}

fn is_safe_search_directory(directory: &Path) -> bool {
    if !directory.is_absolute() {
        return false;
    }
    #[cfg(windows)]
    if !is_local_windows_path(directory) {
        return false;
    }
    true
}

fn filtered_provider_path(path: &OsStr) -> TaskResult<OsString> {
    env::join_paths(env::split_paths(path).filter(|path| is_safe_search_directory(path)))
        .map_err(|error| format!("could not construct bounded provider PATH: {error}"))
}

fn apply_provider_environment(command: &mut Command) -> TaskResult {
    // Official completion generators are static, local operations. Do not pass
    // cloud tokens, kubeconfig, Docker config, proxy credentials, HOME/profile
    // state, or arbitrary application environment into the provider process.
    command.env_clear();
    #[cfg(windows)]
    let public_keys = ["SystemRoot", "WINDIR", "ComSpec", "TEMP", "TMP", "PATHEXT"];
    #[cfg(not(windows))]
    let public_keys = ["TMPDIR"];
    for key in public_keys {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    if let Some(path) = env::var_os("PATH") {
        command.env("PATH", filtered_provider_path(&path)?);
    }
    command.env("NO_COLOR", "1").env("TERM", "dumb");
    #[cfg(unix)]
    command.env("LANG", "C").env("LC_ALL", "C");
    Ok(())
}

fn run_bounded(
    executable: &Path,
    args: &[OsString],
    deadline: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
) -> TaskResult<Captured> {
    let mut command = Command::new(executable);
    apply_provider_environment(&mut command)?;
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut command = CommandWrap::from(command);
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(JobObject);
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", executable.display()))?;
    let stdout = child
        .stdout()
        .take()
        .ok_or_else(|| "provider stdout pipe is unavailable".to_owned())?;
    let stderr = child
        .stderr()
        .take()
        .ok_or_else(|| "provider stderr pipe is unavailable".to_owned())?;
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_overflow = Arc::clone(&overflow);
    let stderr_overflow = Arc::clone(&overflow);
    let stdout_thread =
        thread::spawn(move || read_bounded(stdout, stdout_limit, stdout_overflow));
    let stderr_thread =
        thread::spawn(move || read_bounded(stderr, stderr_limit, stderr_overflow));
    let started = Instant::now();
    let status = loop {
        if overflow.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err("provider output exceeded its bounded capture ceiling".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // A provider leader can exit while a helper still owns the inherited
                // output pipes. Terminate the remaining process group/Job Object
                // before joining readers so refresh cannot hang past its deadline.
                let _ = child.start_kill();
                break status;
            }
            Ok(None) if started.elapsed() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                let _ = child.kill();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(format!(
                    "provider process exceeded the {} ms deadline and was terminated",
                    deadline.as_millis()
                ));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(format!("could not poll provider process: {error}"));
            }
        }
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| "provider stdout reader panicked".to_owned())??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| "provider stderr reader panicked".to_owned())??;
    Ok(Captured {
        status,
        stdout,
        stderr,
        elapsed: started.elapsed(),
    })
}

fn read_bounded(
    mut reader: impl Read,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> TaskResult<Vec<u8>> {
    let mut retained = Vec::with_capacity(limit.min(64 * 1024));
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("could not read provider output: {error}"))?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(retained.len());
        let keep = remaining.min(read);
        retained.extend_from_slice(&buffer[..keep]);
        if keep < read {
            overflow.store(true, Ordering::Release);
        }
    }
    Ok(retained)
}

fn resolve_executable(command: &str) -> TaskResult<PathBuf> {
    if command.is_empty()
        || command.contains('/')
        || command.contains('\\')
        || command.contains('\0')
    {
        return Err("provider executable identity is invalid".into());
    }
    let path = env::var_os("PATH").ok_or_else(|| "PATH is unavailable".to_owned())?;
    #[cfg(windows)]
    let extensions = env::var_os("PATHEXT")
        .unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"))
        .to_string_lossy()
        .split(';')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for directory in env::split_paths(&path) {
        // Empty and relative PATH components search the current working
        // directory. Refresh can run from an untrusted workspace, so those
        // components must never select the provider executable.
        if !is_safe_search_directory(&directory) {
            continue;
        }
        #[cfg(windows)]
        let candidates = extensions
            .iter()
            .map(|extension| directory.join(format!("{command}{extension}")))
            .chain(std::iter::once(directory.join(command)))
            .collect::<Vec<_>>();
        #[cfg(not(windows))]
        let candidates = vec![directory.join(command)];
        for candidate in candidates {
            let Ok(metadata) = fs::metadata(&candidate) else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            let resolved = candidate.canonicalize().map_err(|error| {
                format!("cannot resolve {}: {error}", candidate.display())
            })?;
            #[cfg(windows)]
            if !is_local_windows_path(&resolved) {
                continue;
            }
            return Ok(resolved);
        }
    }
    Err(format!(
        "{} is not installed on an absolute local PATH entry",
        command
    ))
}

#[cfg(windows)]
fn is_local_windows_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};

    path.is_absolute()
        && matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
}

fn validate_refresh_executable(executable: &Path) -> TaskResult {
    #[cfg(windows)]
    {
        let extension = executable
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !extension.eq_ignore_ascii_case("exe")
            && !extension.eq_ignore_ascii_case("com")
        {
            return Err(format!(
                "refusing completion provider launcher without a native .exe/.com image: {}",
                executable.display()
            ));
        }
    }
    #[cfg(not(windows))]
    let _ = executable;
    Ok(())
}

impl ExecutableFingerprint {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;
        #[cfg(windows)]
        use std::os::windows::fs::MetadataExt as _;

        Self {
            length: metadata.len(),
            modified_unix_ms: modified_unix_ms(metadata),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(windows)]
            creation_time: metadata.creation_time(),
            #[cfg(windows)]
            last_write_time: metadata.last_write_time(),
            #[cfg(windows)]
            attributes: metadata.file_attributes(),
        }
    }
}

fn revalidate_executable(
    executable: &Path,
    held_file: &File,
    expected: &ExecutableFingerprint,
) -> TaskResult {
    let held = held_file.metadata().map_err(|error| {
        format!("cannot revalidate open {}: {error}", executable.display())
    })?;
    let current = fs::metadata(executable).map_err(|error| {
        format!("cannot revalidate {}: {error}", executable.display())
    })?;
    if !current.is_file()
        || ExecutableFingerprint::from_metadata(&held) != *expected
        || ExecutableFingerprint::from_metadata(&current) != *expected
    {
        return Err(format!(
            "provider executable identity changed during refresh: {}",
            executable.display()
        ));
    }
    Ok(())
}

fn create_secure_directory(directory: &Path) -> TaskResult {
    validate_managed_directory_chain(directory)?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    validate_managed_directory_chain(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for managed in managed_directory_chain(directory)? {
            fs::set_permissions(&managed, fs::Permissions::from_mode(0o700)).map_err(
                |error| format!("could not restrict {}: {error}", managed.display()),
            )?;
        }
    }
    Ok(())
}

fn validate_managed_directory_chain(directory: &Path) -> TaskResult {
    for candidate in managed_directory_chain(directory)? {
        match fs::symlink_metadata(&candidate) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(format!(
                    "completion directory contains a linked or non-directory component: {}",
                    candidate.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "could not inspect {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Ok(())
}

fn managed_directory_chain(directory: &Path) -> TaskResult<Vec<PathBuf>> {
    let mut managed = Vec::new();
    let mut found_generated = false;
    for ancestor in directory.ancestors() {
        managed.push(ancestor.to_path_buf());
        if ancestor.file_name().is_some_and(|name| name == "generated") {
            if let Some(config_root) = ancestor.parent() {
                managed.push(config_root.to_path_buf());
            }
            found_generated = true;
            break;
        }
    }
    if !found_generated {
        return Err("completion directory is outside the generated state root".into());
    }
    managed.reverse();
    Ok(managed)
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> TaskResult {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(
            "managed completion write exceeds the absolute artifact ceiling".into(),
        );
    }
    let directory = destination
        .parent()
        .ok_or_else(|| format!("{} has no parent", destination.display()))?;
    create_secure_directory(directory)?;
    if let Ok(metadata) = fs::symlink_metadata(destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "refusing to replace non-regular completion artifact {}",
                destination.display()
            ));
        }
    }
    let mut temporary = NamedTempFile::new_in(directory)
        .map_err(|error| format!("could not stage {}: {error}", destination.display()))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("could not flush {}: {error}", destination.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| {
                format!("could not restrict {}: {error}", destination.display())
            })?;
    }
    temporary.persist(destination).map_err(|error| {
        format!(
            "could not atomically replace {}: {}",
            destination.display(),
            error.error
        )
    })?;
    Ok(())
}

fn remove_regular_file_if_present(path: &Path) -> TaskResult {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(format!(
                "refusing to remove non-regular completion artifact {}",
                path.display()
            ))
        }
        Ok(_) => fs::remove_file(path)
            .map_err(|error| format!("could not remove {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn modified_unix_ms(metadata: &fs::Metadata) -> u128 {
    metadata.modified().map(unix_ms).unwrap_or_default()
}

fn unix_ms(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn provider_matrix_is_explicit_and_cmd_never_claims_parity() {
        assert_eq!(Provider::ALL.len(), 11);
        for provider in Provider::ALL {
            assert!(!provider.command().is_empty());
            assert!(matches!(
                provider.support(CompletionShell::Cmd),
                ProviderSupport::Unsupported(_)
            ));
        }
        assert_eq!(
            Provider::Docker.support(CompletionShell::PowerShell),
            ProviderSupport::Unsupported(
                "the installed provider does not publish this shell generator"
            )
        );
        assert_eq!(
            Provider::Kubernetes.support(CompletionShell::PowerShell),
            ProviderSupport::Generated
        );
    }

    #[test]
    fn operation_parser_rejects_unknown_and_non_powershell_override() {
        let valid = parse_operation(&[
            "--provider".into(),
            "kubectl".into(),
            "--shell".into(),
            "powershell".into(),
            "--allow-native-override".into(),
        ])
        .unwrap();
        assert_eq!(valid.provider, Provider::Kubernetes);
        assert_eq!(valid.shell, CompletionShell::PowerShell);
        assert!(valid.allow_native_override);
        assert!(parse_operation(&[
            "--provider".into(),
            "docker".into(),
            "--shell".into(),
            "bash".into(),
            "--allow-native-override".into(),
        ])
        .is_err());
        assert!(parse_operation(&["--provider".into(), "unknown".into()]).is_err());
        assert!(parse_operation(&[
            "--provider".into(),
            "kubectl".into(),
            "--provider".into(),
            "helm".into(),
            "--shell".into(),
            "bash".into(),
        ])
        .unwrap_err()
        .contains("only once"));
        assert!(parse_operation(&[
            "--provider".into(),
            "kubectl".into(),
            "--shell".into(),
            "powershell".into(),
            "--allow-native-override".into(),
            "--allow-native-override".into(),
        ])
        .unwrap_err()
        .contains("only once"));
    }

    #[test]
    fn bounded_reader_retains_only_the_ceiling_and_flags_overflow() {
        let overflow = Arc::new(AtomicBool::new(false));
        let retained =
            read_bounded(Cursor::new(vec![b'x'; 4097]), 4096, Arc::clone(&overflow))
                .unwrap();
        assert_eq!(retained.len(), 4096);
        assert!(overflow.load(Ordering::Acquire));
    }

    #[test]
    fn generated_output_rejects_empty_nul_control_and_invalid_utf8() {
        assert!(validate_generated_output(b"").is_err());
        assert!(validate_generated_output(b"complete\0bad").is_err());
        assert!(validate_generated_output(b"complete\x1bbad").is_err());
        assert!(validate_generated_output("complete \u{202e}bad".as_bytes()).is_err());
        assert!(validate_generated_output(&[0xff]).is_err());
        assert!(validate_generated_output(b"complete -F _tool tool\n").is_ok());
    }

    #[test]
    fn artifact_names_are_fixed_and_never_derived_from_user_text() {
        let root = Path::new("safe");
        let paths = artifact_paths(root, Provider::Kubernetes, CompletionShell::Bash);
        assert_eq!(paths.0, root.join("bash").join("kubectl.bash"));
        assert_eq!(paths.1, root.join("bash").join("kubectl.bash.sha256"));
        assert_eq!(paths.2, root.join("bash").join("kubectl.bash.json"));
        assert_eq!(
            paths.3,
            root.join("bash").join("kubectl.bash.allow-override")
        );
    }

    #[test]
    fn transition_digest_preserves_last_known_good_across_interruption_points() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory
            .path()
            .join("config")
            .join("generated")
            .join("completion");
        let (artifact, digest, _, _) =
            artifact_paths(&root, Provider::Kubernetes, CompletionShell::Bash);
        let previous = b"previous-completion\n";
        let candidate = b"candidate-completion\n";
        let previous_digest = sha256_hex(previous);
        let candidate_digest = sha256_hex(candidate);

        atomic_write(&artifact, previous).unwrap();
        atomic_write(&digest, format!("{previous_digest}\n").as_bytes()).unwrap();
        assert_eq!(
            runtime_artifact_digest(&artifact, &digest).as_deref(),
            Some(previous_digest.as_str())
        );

        let transition =
            transition_digest_bytes(Some(&previous_digest), &candidate_digest);
        assert_eq!(parse_digest_sidecar(&transition).unwrap().len(), 2);
        atomic_write(&digest, &transition).unwrap();
        assert_eq!(
            runtime_artifact_digest(&artifact, &digest).as_deref(),
            Some(previous_digest.as_str())
        );

        atomic_write(&artifact, candidate).unwrap();
        assert_eq!(
            runtime_artifact_digest(&artifact, &digest).as_deref(),
            Some(candidate_digest.as_str())
        );
        assert!(
            parse_digest_sidecar(format!("{candidate_digest}\nextra\n").as_bytes())
                .is_err()
        );
        assert!(parse_digest_sidecar(candidate_digest.as_bytes()).is_err());
    }

    #[test]
    fn atomic_write_rejects_a_link_destination_and_preserves_external_data() {
        let directory = tempfile::tempdir().unwrap();
        let outside = directory.path().join("outside");
        fs::write(&outside, b"keep").unwrap();
        let link = directory.path().join("managed");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&outside, &link).is_err() {
            return;
        }
        assert!(atomic_write(&link, b"replace").is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"keep");
    }

    #[test]
    fn atomic_write_is_bounded_and_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("config")
            .join("generated")
            .join("completion")
            .join("bash")
            .join("artifact");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read(path).unwrap(), b"second");
    }

    fn shell_command(script: &str) -> (PathBuf, Vec<OsString>) {
        #[cfg(windows)]
        {
            (
                PathBuf::from(env::var_os("ComSpec").expect("ComSpec")),
                vec![
                    OsString::from("/D"),
                    OsString::from("/C"),
                    OsString::from(script),
                ],
            )
        }
        #[cfg(not(windows))]
        {
            (
                PathBuf::from("/bin/sh"),
                vec![OsString::from("-c"), OsString::from(script)],
            )
        }
    }

    #[test]
    fn bounded_process_captures_success_without_inheriting_stdin() {
        let (executable, args) = shell_command("echo automexia-provider");
        let captured =
            run_bounded(&executable, &args, Duration::from_secs(2), 1024, 1024).unwrap();
        assert!(captured.status.success());
        assert!(String::from_utf8(captured.stdout)
            .unwrap()
            .contains("automexia-provider"));
    }

    #[test]
    fn provider_path_excludes_relative_workspace_entries() {
        #[cfg(windows)]
        let absolute = PathBuf::from(r"C:\Program Files\Provider");
        #[cfg(not(windows))]
        let absolute = PathBuf::from("/opt/provider/bin");
        let source =
            env::join_paths([PathBuf::from("relative-bin"), absolute.clone()]).unwrap();
        let filtered = filtered_provider_path(&source).unwrap();
        assert_eq!(
            env::split_paths(&filtered).collect::<Vec<_>>(),
            vec![absolute]
        );
    }

    #[test]
    fn provider_process_does_not_inherit_ambient_secret_environment() {
        const SECRET: &str = "AUTOMEXIA_CP1_SECRET_FIXTURE";
        env::set_var(SECRET, "must-not-reach-provider");
        #[cfg(windows)]
        let probe =
            "if defined AUTOMEXIA_CP1_SECRET_FIXTURE (exit /B 7) else (echo isolated)";
        #[cfg(not(windows))]
        let probe = "if [ -n \"${AUTOMEXIA_CP1_SECRET_FIXTURE+x}\" ]; then exit 7; else echo isolated; fi";
        let (executable, args) = shell_command(probe);
        let captured =
            run_bounded(&executable, &args, Duration::from_secs(2), 1024, 1024);
        env::remove_var(SECRET);
        let captured = captured.unwrap();
        assert!(captured.status.success());
        assert!(String::from_utf8(captured.stdout)
            .unwrap()
            .contains("isolated"));
    }

    #[test]
    fn bounded_process_terminates_deadline_and_output_overflow() {
        #[cfg(windows)]
        let spin = "for /L %i in (1,1,100000000) do @rem";
        #[cfg(not(windows))]
        let spin = "while :; do :; done";
        let (executable, args) = shell_command(spin);
        let timeout =
            run_bounded(&executable, &args, Duration::from_millis(25), 1024, 1024)
                .unwrap_err();
        assert!(timeout.contains("deadline"));

        #[cfg(windows)]
        let flood = "for /L %i in (1,1,4096) do @echo 1234567890123456";
        #[cfg(not(windows))]
        let flood =
            "i=0; while [ $i -lt 4096 ]; do echo 1234567890123456; i=$((i+1)); done";
        let (executable, args) = shell_command(flood);
        let overflow =
            run_bounded(&executable, &args, Duration::from_secs(2), 1024, 1024)
                .unwrap_err();
        assert!(overflow.contains("bounded capture ceiling"));
    }

    #[test]
    fn bounded_process_terminates_descendants_holding_output_pipes() {
        #[cfg(windows)]
        let tree = "start /B ping -n 6 127.0.0.1 & for /L %i in (1,1,100000000) do @rem";
        #[cfg(unix)]
        let tree = "(sleep 5) & while :; do :; done";
        let (executable, args) = shell_command(tree);
        let started = Instant::now();
        let timeout =
            run_bounded(&executable, &args, Duration::from_millis(25), 1024, 1024)
                .unwrap_err();
        assert!(timeout.contains("deadline"));
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "descendant process kept provider pipes alive after group termination"
        );
    }

    #[test]
    fn bounded_process_terminates_pipe_holders_after_leader_exit() {
        #[cfg(windows)]
        let tree = "start /B ping -n 6 127.0.0.1 & exit /B 0";
        #[cfg(unix)]
        let tree = "(sleep 5) & exit 0";
        let (executable, args) = shell_command(tree);
        let started = Instant::now();
        let captured =
            run_bounded(&executable, &args, Duration::from_secs(2), 1024, 1024).unwrap();
        assert!(captured.status.success());
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "a provider helper retained output pipes after its leader exited"
        );
    }

    #[test]
    fn secure_directory_rejects_a_link_inside_managed_state() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("config");
        let outside = directory.path().join("outside");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let generated = config.join("generated");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &generated).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_dir(&outside, &generated).is_err() {
            return;
        }
        assert!(
            create_secure_directory(&generated.join("completion").join("bash")).is_err()
        );
        assert!(!outside.join("completion").exists());
    }

    #[cfg(unix)]
    #[test]
    fn secure_directory_restricts_every_managed_component() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("config");
        let shell = config.join("generated").join("completion").join("bash");
        create_secure_directory(&shell).unwrap();
        for managed in [
            config,
            directory.path().join("config/generated"),
            directory.path().join("config/generated/completion"),
            shell,
        ] {
            assert_eq!(
                fs::metadata(managed).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn config_roots_are_absolute_and_macos_matches_the_product_contract() {
        #[cfg(windows)]
        let home = PathBuf::from(r"C:\Users\amjed");
        #[cfg(not(windows))]
        let home = PathBuf::from("/Users/amjed");
        let mac =
            select_config_root(HostPlatform::MacOs, None, None, Some(home.clone()), None)
                .unwrap();
        assert_eq!(
            mac,
            home.join("Library")
                .join("Application Support")
                .join("io.github.AmjedAllaya.AutomexiaTerminal")
        );
        #[cfg(windows)]
        assert!(select_config_root(
            HostPlatform::Windows,
            Some(PathBuf::from(r"\\server\share\automexia")),
            None,
            Some(home.clone()),
            None,
        )
        .unwrap_err()
        .contains("local Windows drive"));
        assert!(select_config_root(
            HostPlatform::Unix,
            Some(PathBuf::from("relative-config")),
            None,
            Some(home),
            None,
        )
        .unwrap_err()
        .contains("must be absolute"));
    }

    #[test]
    fn doctor_health_validates_digest_metadata_and_provenance() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory
            .path()
            .join("config")
            .join("generated")
            .join("completion");
        let provider = Provider::Kubernetes;
        let shell = CompletionShell::Bash;
        let (artifact, digest, metadata_path, _) = artifact_paths(&root, provider, shell);
        let source = b"complete -W managed kubectl\n";
        let source_digest = sha256_hex(source);
        let mut body = generated_header(provider, shell, &source_digest).into_bytes();
        body.extend_from_slice(source);
        let artifact_digest = sha256_hex(&body);
        atomic_write(&artifact, &body).unwrap();
        atomic_write(&digest, format!("{artifact_digest}\n").as_bytes()).unwrap();
        let metadata = ArtifactMetadata {
            schema_version: SCHEMA_VERSION,
            provider: provider.id().into(),
            command: provider.command().into(),
            shell: shell.id().into(),
            executable: "/fixture/kubectl".into(),
            executable_length: 42,
            executable_modified_unix_ms: 1,
            tool_version: "v1.0.0".into(),
            source_sha256: source_digest,
            artifact_sha256: artifact_digest,
            generated_unix_ms: 1,
            deadline_ms: PROVIDER_DEADLINE.as_millis(),
            output_limit_bytes: MAX_PROVIDER_OUTPUT,
            native_override: false,
        };
        atomic_write(
            &metadata_path,
            &serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
        assert_eq!(
            inspect_artifact(&root, provider, shell),
            ArtifactHealth::Healthy
        );
        fs::write(&artifact, b"tampered").unwrap();
        assert_eq!(
            inspect_artifact(&root, provider, shell),
            ArtifactHealth::Invalid("digest-mismatch".into())
        );
    }

    #[test]
    fn executable_revalidation_detects_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("provider");
        fs::write(&executable, b"first").unwrap();
        let held = File::open(&executable).unwrap();
        let expected = ExecutableFingerprint::from_metadata(&held.metadata().unwrap());
        revalidate_executable(&executable, &held, &expected).unwrap();
        fs::write(&executable, b"changed-length").unwrap();
        assert!(revalidate_executable(&executable, &held, &expected).is_err());
    }

    #[test]
    fn sha256_is_stable() {
        assert_eq!(
            sha256_hex(b"automexia"),
            "aa0072bbb611bed430699b37c0c05c1d0097e1a5cf385f95bb6571cca014a876"
        );
    }

    #[test]
    fn generated_headers_are_reviewable_and_shell_specific() {
        let header = generated_header(Provider::Docker, CompletionShell::Bash, "abc");
        assert!(header.contains("schema=1 provider=docker shell=bash source-sha256=abc"));
        assert!(!header.contains("eval"));
    }

    #[test]
    fn provider_output_and_path_ceilings_are_immutable() {
        assert_eq!(PROVIDER_DEADLINE, Duration::from_millis(750));
        assert_eq!(MAX_PROVIDER_OUTPUT, 1_048_576);
        assert_eq!(MAX_PROVIDER_STDERR, 262_144);
        assert_eq!(MAX_CONFIG_PATH_BYTES, 4096);
        assert_eq!(MAX_DIGEST_BYTES, 192);
    }

    #[test]
    fn refresh_launcher_policy_avoids_implicit_windows_shell_parsing() {
        #[cfg(windows)]
        {
            assert!(is_local_windows_path(Path::new(r"C:\Tools")));
            assert!(!is_local_windows_path(Path::new(r"relative\Tools")));
            assert!(!is_local_windows_path(Path::new(r"\\server\share\Tools")));
            assert!(validate_refresh_executable(Path::new("kubectl.exe")).is_ok());
            assert!(validate_refresh_executable(Path::new("legacy.com")).is_ok());
            assert!(validate_refresh_executable(Path::new("kubectl.cmd")).is_err());
            assert!(validate_refresh_executable(Path::new("kubectl.bat")).is_err());
        }
        #[cfg(not(windows))]
        assert!(validate_refresh_executable(Path::new("/usr/bin/kubectl")).is_ok());
    }

    #[test]
    fn multiline_provider_versions_are_bounded_and_informative() {
        let output = "clientVersion:\n  gitVersion: v1.35.0\n  platform: linux/amd64\n  extra: ignored-after-four\n  fifth: omitted\n";
        let summary = summarize_version(output);
        assert_eq!(summary, "clientVersion: gitVersion: v1.35.0 platform: linux/amd64 extra: ignored-after-four");
        assert!(summary.len() <= 512);
        let hostile =
            summarize_version("v1\u{1b}]8;;https://spoof.invalid\u{7}link\u{202e}txt");
        assert!(!hostile.contains('\u{1b}'));
        assert!(!hostile.contains('\u{7}'));
        assert!(!hostile.contains('\u{202e}'));
        assert!(hostile.contains('\u{fffd}'));
        assert_eq!(
            Provider::Kubernetes.version_args(),
            &["version", "--client=true"]
        );
    }

    #[test]
    fn completion_output_treats_a_closed_consumer_as_success_only() {
        assert!(tolerate_broken_pipe(Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "consumer closed"
        )))
        .is_ok());
        let error = tolerate_broken_pipe(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "blocked",
        )))
        .unwrap_err();
        assert!(error.contains("could not write completion output"));
    }
}
