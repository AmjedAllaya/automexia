//! Non-activated exact-argument session-launch boundary.
//!
//! ADR 0012 is still proposed, so the production module graph excludes this
//! entire broker. Pure validation and lifecycle code is compiled by tests so
//! it can be reviewed and exercised without granting an extension process,
//! PTY, environment, or renderer authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use automexia_extension_api::{
    Capability, CapabilityDecision, CapabilityRequest, Decision, ExecutableId,
    ExtensionId, LaunchKind, LaunchRequest, OperationId, ResourceScope, SessionId,
};

use super::launch::SessionLaunchDescriptor;

/// This must remain false until ADR 0012 receives the protected-path review
/// required by ADR 0003 and its native acceptance evidence is recorded.
pub const MANAGED_SESSION_LAUNCH_ENABLED: bool = false;
const _: () = assert!(!MANAGED_SESSION_LAUNCH_ENABLED);

const REVIEWED_EXTENSION_ID: &str = "automexia.devops-ssh";
const REVIEWED_PUBLISHER: &str = "io.github.AmjedAllaya";
const MAX_PUBLISHER_BYTES: usize = 256;
const REVIEWED_CONTRACT_VERSION: u32 = 1;
const MAX_VERSION_BYTES: usize = 64;
const MAX_TOTAL_ARGUMENT_BYTES: usize = 32 * 1024;
const MAX_DESTINATION_BYTES: usize = 512;
const MAX_TRUSTED_ENVIRONMENT_ENTRIES: usize = 256;
const MAX_ENVIRONMENT_NAME_BYTES: usize = 256;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 16 * 1024;
const MAX_TOTAL_ENVIRONMENT_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenSshExecutable {
    Ssh,
    SshAdd,
    SshKeygen,
}

impl OpenSshExecutable {
    const ALL: [Self; 3] = [Self::Ssh, Self::SshAdd, Self::SshKeygen];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::SshAdd => "ssh-add",
            Self::SshKeygen => "ssh-keygen",
        }
    }

    #[cfg(target_os = "windows")]
    const fn filename(self) -> &'static str {
        match self {
            Self::Ssh => "ssh.exe",
            Self::SshAdd => "ssh-add.exe",
            Self::SshKeygen => "ssh-keygen.exe",
        }
    }

    #[cfg(not(target_os = "windows"))]
    const fn filename(self) -> &'static str {
        self.id()
    }

    fn parse(value: &ExecutableId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| value.as_str() == candidate.id())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchOperationKind {
    OpenSshConnect,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchDenialCode {
    PendingSecurityReview,
    InvalidPrincipal,
    CapabilityMismatch,
    DecisionDenied,
    DecisionMismatch,
    DecisionFromFuture,
    DecisionExpired,
    InvalidScope,
    UnsupportedExecutable,
    UnsupportedOperation,
    InvalidArguments,
    OptionConfusedDestination,
    UnsupportedEncoding,
    ExtensionEnvironmentAccess,
    SecretResolutionUnavailable,
    InvalidTrustedEnvironment,
    InvalidWorkingDirectory,
    ExecutableUnavailable,
    ExecutableIdentityChanged,
    DuplicateOperation,
    ReplayedOperation,
    NonceExhausted,
    Revoked,
    StaleOperationLease,
}

impl fmt::Display for LaunchDenialCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PendingSecurityReview => {
                "managed session launch is disabled pending protected security review"
            }
            Self::InvalidPrincipal => "the extension principal is not reviewed",
            Self::CapabilityMismatch => {
                "the capability request does not match the launch"
            }
            Self::DecisionDenied => "session launch was denied",
            Self::DecisionMismatch => {
                "the capability decision does not match the operation"
            }
            Self::DecisionFromFuture => "the capability decision timestamp is invalid",
            Self::DecisionExpired => "the capability decision has expired",
            Self::InvalidScope => "the launch session or capsule scope is invalid",
            Self::UnsupportedExecutable => "the requested executable is not allowlisted",
            Self::UnsupportedOperation => {
                "the requested OpenSSH operation is not implemented"
            }
            Self::InvalidArguments => "the ordered argument vector is invalid",
            Self::OptionConfusedDestination => {
                "the SSH destination is ambiguous with a command-line option"
            }
            Self::UnsupportedEncoding => "a launch path uses unsupported text encoding",
            Self::ExtensionEnvironmentAccess => {
                "extensions cannot select or read the inherited environment"
            }
            Self::SecretResolutionUnavailable => {
                "managed secret resolution is not enabled for this operation"
            }
            Self::InvalidTrustedEnvironment => {
                "the trusted environment overrides are invalid"
            }
            Self::InvalidWorkingDirectory => "the trusted working directory is invalid",
            Self::ExecutableUnavailable => {
                "the approved system executable is unavailable"
            }
            Self::ExecutableIdentityChanged => {
                "the approved executable changed before process creation"
            }
            Self::DuplicateOperation => "the operation identifier is already active",
            Self::ReplayedOperation => {
                "the operation identifier was already used in this session"
            }
            Self::NonceExhausted => "the operation lease generation is exhausted",
            Self::Revoked => "the extension or session grant was revoked",
            Self::StaleOperationLease => {
                "the operation lease is stale or belongs to another scope"
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageDigest([u8; 32]);

impl PackageDigest {
    pub fn new(bytes: [u8; 32]) -> Result<Self, LaunchDenialCode> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(LaunchDenialCode::InvalidPrincipal);
        }
        Ok(Self(bytes))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageVerification {
    RepositoryReviewed,
    FirstPartySigned,
    Unverified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewedPackagePolicy {
    id: ExtensionId,
    publisher: String,
    version: String,
    package_digest: PackageDigest,
    contract_version: u32,
}

impl ReviewedPackagePolicy {
    pub fn new(
        id: ExtensionId,
        publisher: impl Into<String>,
        version: impl Into<String>,
        package_digest: PackageDigest,
        contract_version: u32,
    ) -> Result<Self, LaunchDenialCode> {
        let publisher = publisher.into();
        let version = version.into();
        if id.as_str() != REVIEWED_EXTENSION_ID
            || publisher != REVIEWED_PUBLISHER
            || version != env!("CARGO_PKG_VERSION")
            || contract_version != REVIEWED_CONTRACT_VERSION
        {
            return Err(LaunchDenialCode::InvalidPrincipal);
        }
        Ok(Self {
            id,
            publisher,
            version,
            package_digest,
            contract_version,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedExtension {
    id: ExtensionId,
    publisher: String,
    version: String,
    package_digest: PackageDigest,
    contract_version: u32,
    verification: PackageVerification,
}

impl VerifiedExtension {
    pub fn new(
        id: ExtensionId,
        publisher: impl Into<String>,
        version: impl Into<String>,
        package_digest: PackageDigest,
        contract_version: u32,
        verification: PackageVerification,
    ) -> Result<Self, LaunchDenialCode> {
        let publisher = publisher.into();
        let version = version.into();
        if !valid_identity_text(&publisher, MAX_PUBLISHER_BYTES)
            || !valid_version(&version)
            || contract_version == 0
        {
            return Err(LaunchDenialCode::InvalidPrincipal);
        }
        Ok(Self {
            id,
            publisher,
            version,
            package_digest,
            contract_version,
            verification,
        })
    }

    fn is_reviewed_first_party(&self, policy: &ReviewedPackagePolicy) -> bool {
        self.id == policy.id
            && self.publisher == policy.publisher
            && self.version == policy.version
            && self.package_digest == policy.package_digest
            && self.contract_version == policy.contract_version
            && matches!(
                self.verification,
                PackageVerification::RepositoryReviewed
                    | PackageVerification::FirstPartySigned
            )
    }
}

fn valid_identity_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn valid_version(value: &str) -> bool {
    valid_identity_text(value, MAX_VERSION_BYTES)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+')
        })
}

pub struct LaunchSubmission<'a> {
    pub principal: &'a VerifiedExtension,
    pub capability: &'a CapabilityRequest,
    pub decision: &'a CapabilityDecision,
    pub launch: &'a LaunchRequest,
    /// Configuration-owned public overrides supplied by core, never read from
    /// extension state or the inherited process environment.
    pub trusted_environment: &'a [(String, String)],
    pub safe_default_working_directory: &'a Path,
    pub now_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditDecision {
    AllowOnce,
    AllowSession,
    Deny,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditResultClass {
    Authorized,
    Denied(LaunchDenialCode),
    Cancelled,
    Completed,
}

/// Deliberately contains no argv, environment value, cwd, terminal content,
/// username, secret reference, process ID, or executable path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchAuditRecord {
    pub extension_id: ExtensionId,
    pub extension_version: String,
    pub publisher: String,
    pub decision: AuditDecision,
    pub operation_kind: LaunchOperationKind,
    pub public_connection_id: Option<String>,
    pub operation_id: OperationId,
    pub session_id: SessionId,
    pub timestamp_ms: u64,
    pub duration_ms: u64,
    pub result: AuditResultClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeniedLaunch {
    pub code: LaunchDenialCode,
    pub audit: LaunchAuditRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationLease {
    operation_id: OperationId,
    session_id: SessionId,
    capsule_revision: u64,
    nonce: u64,
}

impl OperationLease {
    pub const fn operation_id(self) -> OperationId {
        self.operation_id
    }

    pub const fn session_id(self) -> SessionId {
        self.session_id
    }

    pub const fn capsule_revision(self) -> u64 {
        self.capsule_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkingDirectoryDisposition {
    Requested,
    SafeDefaultMissing,
    SafeDefaultNotRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileIdentity {
    canonical_path: PathBuf,
    platform: PlatformFileIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PlatformFileIdentity {
    #[cfg(unix)]
    Unix {
        device: u64,
        inode: u64,
        size: u64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
    },
    #[cfg(target_os = "windows")]
    Windows {
        volume_serial: u32,
        file_index: u64,
        size: u64,
        last_write: u64,
    },
    #[cfg(not(any(unix, target_os = "windows")))]
    Portable {
        size: u64,
        modified_nanoseconds: Option<u128>,
    },
}

pub struct PreparedLaunch {
    executable: ExecutableId,
    executable_identity: FileIdentity,
    arguments: Vec<String>,
    environment: Vec<(String, String)>,
    working_directory: PathBuf,
    safe_default_working_directory: PathBuf,
    working_directory_disposition: WorkingDirectoryDisposition,
    operation_kind: LaunchOperationKind,
    lease: OperationLease,
    audit: LaunchAuditRecord,
}

impl fmt::Debug for PreparedLaunch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedLaunch")
            .field("executable", &self.executable)
            .field("argument_count", &self.arguments.len())
            .field(
                "environment_names",
                &self
                    .environment
                    .iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>(),
            )
            .field(
                "working_directory_disposition",
                &self.working_directory_disposition,
            )
            .field("operation_kind", &self.operation_kind)
            .field("lease", &self.lease)
            .finish()
    }
}

impl PreparedLaunch {
    pub fn lease(&self) -> OperationLease {
        self.lease
    }

    pub fn audit(&self) -> &LaunchAuditRecord {
        &self.audit
    }

    pub fn revalidate_executable(&self) -> Result<(), LaunchDenialCode> {
        let current = identify_executable(&self.executable_identity.canonical_path)
            .map_err(|_| LaunchDenialCode::ExecutableIdentityChanged)?;
        if current == self.executable_identity {
            Ok(())
        } else {
            Err(LaunchDenialCode::ExecutableIdentityChanged)
        }
    }

    /// Revalidate the cwd immediately before launch. If a previously valid
    /// requested directory vanished, return the trusted safe default instead.
    pub fn revalidated_working_directory(&self) -> Result<PathBuf, LaunchDenialCode> {
        canonical_directory(&self.working_directory)
            .or_else(|| canonical_directory(&self.safe_default_working_directory))
            .ok_or(LaunchDenialCode::InvalidWorkingDirectory)
    }

    /// Convert only the already-authorized exact request into the one existing
    /// application launch seam. Executable identity and cwd are revalidated in
    /// this same operation so a future caller cannot accidentally omit either
    /// check. This method does not create a process or PTY.
    pub fn session_launch_descriptor(
        &self,
    ) -> Result<SessionLaunchDescriptor, LaunchDenialCode> {
        self.revalidate_executable()?;
        let program = self
            .executable_identity
            .canonical_path
            .to_str()
            .ok_or(LaunchDenialCode::UnsupportedEncoding)?
            .to_owned();
        let cwd = self
            .revalidated_working_directory()?
            .to_str()
            .ok_or(LaunchDenialCode::UnsupportedEncoding)?
            .to_owned();
        Ok(SessionLaunchDescriptor::new(
            Some(program),
            self.arguments.clone(),
            self.environment.clone(),
            None,
            Some(cwd),
        ))
    }

    #[cfg(test)]
    fn test_command(&self) -> std::process::Command {
        let mut command =
            std::process::Command::new(&self.executable_identity.canonical_path);
        command
            .args(&self.arguments)
            .current_dir(&self.working_directory)
            .envs(self.environment.iter().map(|(name, value)| (name, value)));
        command
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenSshHostPlatform {
    Windows,
    MacOs,
    Linux,
    Wsl,
    Unsupported,
}

#[cfg(target_os = "windows")]
fn current_openssh_host_platform() -> OpenSshHostPlatform {
    OpenSshHostPlatform::Windows
}

#[cfg(target_os = "macos")]
fn current_openssh_host_platform() -> OpenSshHostPlatform {
    OpenSshHostPlatform::MacOs
}

#[cfg(target_os = "linux")]
fn current_openssh_host_platform() -> OpenSshHostPlatform {
    let wsl_environment = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::env::var_os("WSL_INTEROP").is_some();
    let wsl_kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
        .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"));
    if wsl_environment || wsl_kernel {
        OpenSshHostPlatform::Wsl
    } else {
        OpenSshHostPlatform::Linux
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn current_openssh_host_platform() -> OpenSshHostPlatform {
    OpenSshHostPlatform::Unsupported
}

fn platform_candidate_roots(
    platform: OpenSshHostPlatform,
    windows_system_directory: Option<&Path>,
) -> Vec<PathBuf> {
    match platform {
        OpenSshHostPlatform::Windows => windows_system_directory
            .map(|directory| vec![directory.join("OpenSSH")])
            .unwrap_or_default(),
        OpenSshHostPlatform::MacOs => ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        OpenSshHostPlatform::Linux => ["/usr/bin", "/bin", "/usr/local/bin"]
            .into_iter()
            .map(PathBuf::from)
            .collect(),
        // WSL launch remains outside the accepted ADR boundary. A later phase
        // must verify fixed System32/wsl.exe and /usr/bin/ssh identities.
        OpenSshHostPlatform::Wsl | OpenSshHostPlatform::Unsupported => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn host_windows_system_directory() -> Option<PathBuf> {
    windows_system_directory()
}

#[cfg(not(target_os = "windows"))]
fn host_windows_system_directory() -> Option<PathBuf> {
    None
}

#[derive(Clone, Debug, Default)]
pub struct ExecutablePolicy {
    candidates: BTreeMap<&'static str, Vec<PathBuf>>,
}

impl ExecutablePolicy {
    pub fn host_defaults() -> Self {
        let mut policy = Self::default();
        let system_directory = host_windows_system_directory();
        for directory in platform_candidate_roots(
            current_openssh_host_platform(),
            system_directory.as_deref(),
        ) {
            for executable in OpenSshExecutable::ALL {
                policy.add_candidate(executable, directory.join(executable.filename()));
            }
        }
        policy
    }

    /// An explicit override replaces host defaults. Failure never falls back
    /// to a PATH, working-directory, or default candidate.
    pub fn with_configured_path(
        mut self,
        executable: OpenSshExecutable,
        path: PathBuf,
    ) -> Result<Self, LaunchDenialCode> {
        if !path.is_absolute() || !filename_matches(executable, &path) {
            return Err(LaunchDenialCode::UnsupportedExecutable);
        }
        self.candidates.insert(executable.id(), vec![path]);
        Ok(self)
    }

    fn add_candidate(&mut self, executable: OpenSshExecutable, path: PathBuf) {
        self.candidates
            .entry(executable.id())
            .or_default()
            .push(path);
    }

    fn resolve(
        &self,
        executable: &ExecutableId,
    ) -> Result<FileIdentity, LaunchDenialCode> {
        let executable = OpenSshExecutable::parse(executable)
            .ok_or(LaunchDenialCode::UnsupportedExecutable)?;
        let candidates = self
            .candidates
            .get(executable.id())
            .ok_or(LaunchDenialCode::ExecutableUnavailable)?;
        for candidate in candidates {
            if let Ok(identity) = identify_executable(candidate) {
                return Ok(identity);
            }
        }
        Err(LaunchDenialCode::ExecutableUnavailable)
    }
}

fn filename_matches(executable: OpenSshExecutable, path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(executable.filename()))
}

#[cfg(target_os = "windows")]
fn windows_system_directory() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = vec![0_u16; 32_768];
    // SAFETY: `buffer` is writable for its declared length and the API writes
    // at most that many UTF-16 code units.
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 || length as usize >= buffer.len() {
        return None;
    }
    buffer.truncate(length as usize);
    Some(PathBuf::from(OsString::from_wide(&buffer)))
}

fn identify_executable(path: &Path) -> Result<FileIdentity, LaunchDenialCode> {
    let canonical_path =
        canonical_existing(path).map_err(|_| LaunchDenialCode::ExecutableUnavailable)?;
    if canonical_path.to_str().is_none() {
        return Err(LaunchDenialCode::UnsupportedEncoding);
    }
    let metadata = fs::metadata(&canonical_path)
        .map_err(|_| LaunchDenialCode::ExecutableUnavailable)?;
    if !metadata.is_file() || !is_platform_executable(&canonical_path, &metadata) {
        return Err(LaunchDenialCode::ExecutableUnavailable);
    }
    Ok(FileIdentity {
        canonical_path: canonical_path.clone(),
        platform: platform_file_identity(&canonical_path, &metadata)?,
    })
}

#[cfg(target_os = "windows")]
fn canonical_existing(path: &Path) -> std::io::Result<PathBuf> {
    dunce::canonicalize(path)
}

#[cfg(not(target_os = "windows"))]
fn canonical_existing(path: &Path) -> std::io::Result<PathBuf> {
    fs::canonicalize(path)
}

#[cfg(unix)]
fn is_platform_executable(_path: &Path, metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(target_os = "windows")]
fn is_platform_executable(path: &Path, _metadata: &fs::Metadata) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

#[cfg(not(any(unix, target_os = "windows")))]
fn is_platform_executable(_path: &Path, _metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(unix)]
fn platform_file_identity(
    _path: &Path,
    metadata: &fs::Metadata,
) -> Result<PlatformFileIdentity, LaunchDenialCode> {
    use std::os::unix::fs::MetadataExt;
    Ok(PlatformFileIdentity::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    })
}

#[cfg(target_os = "windows")]
fn platform_file_identity(
    path: &Path,
    _metadata: &fs::Metadata,
) -> Result<PlatformFileIdentity, LaunchDenialCode> {
    use std::fs::File;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path).map_err(|_| LaunchDenialCode::ExecutableUnavailable)?;
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the file owns a valid handle for the duration of this call and
    // `information` points to initialized writable storage of the expected ABI.
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle(), &mut information as *mut _)
    };
    if succeeded == 0 {
        return Err(LaunchDenialCode::ExecutableUnavailable);
    }
    let file_index = (u64::from(information.nFileIndexHigh) << 32)
        | u64::from(information.nFileIndexLow);
    let size = (u64::from(information.nFileSizeHigh) << 32)
        | u64::from(information.nFileSizeLow);
    let last_write = (u64::from(information.ftLastWriteTime.dwHighDateTime) << 32)
        | u64::from(information.ftLastWriteTime.dwLowDateTime);
    Ok(PlatformFileIdentity::Windows {
        volume_serial: information.dwVolumeSerialNumber,
        file_index,
        size,
        last_write,
    })
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_file_identity(
    _path: &Path,
    metadata: &fs::Metadata,
) -> Result<PlatformFileIdentity, LaunchDenialCode> {
    use std::time::UNIX_EPOCH;
    Ok(PlatformFileIdentity::Portable {
        size: metadata.len(),
        modified_nanoseconds: metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos()),
    })
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    let canonical = canonical_existing(path).ok()?;
    fs::metadata(&canonical).ok()?.is_dir().then_some(canonical)
}

#[derive(Clone, Debug)]
struct OperationBinding {
    extension_id: ExtensionId,
    lease: OperationLease,
}

#[derive(Clone, Copy, Debug)]
struct SessionRegistration {
    capsule_revision: u64,
    last_operation_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrokerActivation {
    PendingSecurityReview,
    #[cfg(test)]
    ReviewHarness,
}

pub struct CapabilityBroker {
    activation: BrokerActivation,
    executable_policy: ExecutablePolicy,
    package_policy: ReviewedPackagePolicy,
    next_nonce: u64,
    operations: BTreeMap<OperationId, OperationBinding>,
    sessions: BTreeMap<SessionId, SessionRegistration>,
    highest_session_id: u64,
    revoked_extensions: BTreeSet<ExtensionId>,
}

impl CapabilityBroker {
    pub fn pending_security_review(
        executable_policy: ExecutablePolicy,
        package_policy: ReviewedPackagePolicy,
    ) -> Self {
        Self {
            activation: BrokerActivation::PendingSecurityReview,
            executable_policy,
            next_nonce: 1,
            package_policy,
            operations: BTreeMap::new(),
            sessions: BTreeMap::new(),
            highest_session_id: 0,
            revoked_extensions: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    fn review_harness(
        executable_policy: ExecutablePolicy,
        package_policy: ReviewedPackagePolicy,
    ) -> Self {
        let mut broker = Self::pending_security_review(executable_policy, package_policy);
        broker.activation = BrokerActivation::ReviewHarness;
        broker
    }

    /// Register a core-owned session/capsule scope before an extension may
    /// request work for it. Session IDs are monotonic and cannot be reused,
    /// which prevents a stale grant from becoming valid after session close.
    pub fn register_session(
        &mut self,
        session_id: SessionId,
        capsule_revision: u64,
    ) -> Result<(), LaunchDenialCode> {
        if session_id.get() == 0
            || capsule_revision == 0
            || session_id.get() <= self.highest_session_id
            || self.sessions.contains_key(&session_id)
        {
            return Err(LaunchDenialCode::InvalidScope);
        }
        self.highest_session_id = session_id.get();
        self.sessions.insert(
            session_id,
            SessionRegistration {
                capsule_revision,
                last_operation_id: 0,
            },
        );
        Ok(())
    }

    /// Advance an existing session to a new capsule revision and cancel only
    /// operations owned by the previous revision.
    pub fn rebind_session(
        &mut self,
        session_id: SessionId,
        capsule_revision: u64,
    ) -> Result<usize, LaunchDenialCode> {
        let registration = self
            .sessions
            .get_mut(&session_id)
            .ok_or(LaunchDenialCode::InvalidScope)?;
        if capsule_revision == 0 || capsule_revision <= registration.capsule_revision {
            return Err(LaunchDenialCode::InvalidScope);
        }
        registration.capsule_revision = capsule_revision;
        let before = self.operations.len();
        self.operations
            .retain(|_, binding| binding.lease.session_id != session_id);
        Ok(before - self.operations.len())
    }

    pub fn authorize(
        &mut self,
        submission: LaunchSubmission<'_>,
    ) -> Result<PreparedLaunch, Box<DeniedLaunch>> {
        if self.activation == BrokerActivation::PendingSecurityReview
            || (!cfg!(test) && !MANAGED_SESSION_LAUNCH_ENABLED)
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::PendingSecurityReview,
                operation_kind(submission.launch),
            ));
        }
        self.authorize_reviewed(submission)
    }

    fn authorize_reviewed(
        &mut self,
        submission: LaunchSubmission<'_>,
    ) -> Result<PreparedLaunch, Box<DeniedLaunch>> {
        let operation_kind = operation_kind(submission.launch);

        if !submission
            .principal
            .is_reviewed_first_party(&self.package_policy)
            || submission.capability.extension_id != submission.principal.id
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::InvalidPrincipal,
                operation_kind,
            ));
        }
        if self.revoked_extensions.contains(&submission.principal.id) {
            return Err(deny(&submission, LaunchDenialCode::Revoked, operation_kind));
        }
        if submission.launch.operation_id.get() == 0
            || submission.launch.session_id.get() == 0
            || submission.launch.capsule_revision == 0
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::InvalidScope,
                operation_kind,
            ));
        }
        let Some(registration) = self.sessions.get(&submission.launch.session_id) else {
            return Err(deny(&submission, LaunchDenialCode::Revoked, operation_kind));
        };
        let registered_capsule_revision = registration.capsule_revision;
        let last_operation_id = registration.last_operation_id;
        if registered_capsule_revision != submission.launch.capsule_revision {
            return Err(deny(
                &submission,
                LaunchDenialCode::InvalidScope,
                operation_kind,
            ));
        }
        if submission.capability.operation_id != submission.launch.operation_id
            || submission.capability.session_id != submission.launch.session_id
            || submission.capability.capsule_revision
                != submission.launch.capsule_revision
            || submission.capability.capability != Capability::SessionLaunch
            || !matches!(
                &submission.capability.resource,
                ResourceScope::Executable(executable)
                    if executable == &submission.launch.executable
            )
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::CapabilityMismatch,
                operation_kind,
            ));
        }
        if submission.decision.operation_id != submission.capability.operation_id
            || submission.decision.extension_id != submission.capability.extension_id
            || submission.decision.session_id != submission.capability.session_id
            || submission.decision.capsule_revision
                != submission.capability.capsule_revision
            || submission.decision.capability != submission.capability.capability
            || submission.decision.resource != submission.capability.resource
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::DecisionMismatch,
                operation_kind,
            ));
        }
        if submission.decision.decided_at_ms > submission.now_ms {
            return Err(deny(
                &submission,
                LaunchDenialCode::DecisionFromFuture,
                operation_kind,
            ));
        }
        if submission.decision.expires_at_ms < submission.now_ms {
            return Err(deny(
                &submission,
                LaunchDenialCode::DecisionExpired,
                operation_kind,
            ));
        }
        if submission.decision.decision == Decision::Deny {
            return Err(deny(
                &submission,
                LaunchDenialCode::DecisionDenied,
                operation_kind,
            ));
        }
        if !matches!(submission.launch.kind, LaunchKind::Native)
            || submission.launch.profile.is_some()
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::InvalidScope,
                operation_kind,
            ));
        }
        if !submission.launch.inherited_environment.is_empty() {
            return Err(deny(
                &submission,
                LaunchDenialCode::ExtensionEnvironmentAccess,
                operation_kind,
            ));
        }
        if !submission.launch.secret_references.is_empty() {
            return Err(deny(
                &submission,
                LaunchDenialCode::SecretResolutionUnavailable,
                operation_kind,
            ));
        }
        if self
            .operations
            .contains_key(&submission.launch.operation_id)
        {
            return Err(deny(
                &submission,
                LaunchDenialCode::DuplicateOperation,
                operation_kind,
            ));
        }
        if submission.launch.operation_id.get() <= last_operation_id {
            return Err(deny(
                &submission,
                LaunchDenialCode::ReplayedOperation,
                operation_kind,
            ));
        }

        if let Err(code) = validate_operation(submission.launch) {
            return Err(deny(&submission, code, operation_kind));
        }
        let environment =
            match validate_trusted_environment(submission.trusted_environment) {
                Ok(environment) => environment,
                Err(code) => return Err(deny(&submission, code, operation_kind)),
            };
        let (
            working_directory,
            safe_default_working_directory,
            working_directory_disposition,
        ) = match resolve_working_directory(
            submission
                .launch
                .working_directory
                .as_ref()
                .map(|value| value.as_str()),
            submission.safe_default_working_directory,
        ) {
            Ok(resolved) => resolved,
            Err(code) => return Err(deny(&submission, code, operation_kind)),
        };
        let executable_identity = match self
            .executable_policy
            .resolve(&submission.launch.executable)
        {
            Ok(identity) => identity,
            Err(code) => return Err(deny(&submission, code, operation_kind)),
        };
        let next_nonce = match self.next_nonce.checked_add(1) {
            Some(next_nonce) => next_nonce,
            None => {
                return Err(deny(
                    &submission,
                    LaunchDenialCode::NonceExhausted,
                    operation_kind,
                ));
            }
        };
        let lease = OperationLease {
            operation_id: submission.launch.operation_id,
            session_id: submission.launch.session_id,
            capsule_revision: submission.launch.capsule_revision,
            nonce: self.next_nonce,
        };
        let Some(registration) = self.sessions.get_mut(&submission.launch.session_id)
        else {
            return Err(deny(
                &submission,
                LaunchDenialCode::InvalidScope,
                operation_kind,
            ));
        };
        registration.last_operation_id = submission.launch.operation_id.get();
        self.next_nonce = next_nonce;
        self.operations.insert(
            lease.operation_id,
            OperationBinding {
                extension_id: submission.principal.id.clone(),
                lease,
            },
        );
        let audit =
            audit_record(&submission, operation_kind, AuditResultClass::Authorized);
        Ok(PreparedLaunch {
            executable: submission.launch.executable.clone(),
            executable_identity,
            arguments: submission
                .launch
                .arguments
                .iter()
                .map(|argument| argument.as_str().to_owned())
                .collect(),
            environment,
            working_directory,
            safe_default_working_directory,
            working_directory_disposition,
            operation_kind,
            lease,
            audit,
        })
    }

    pub fn complete(
        &mut self,
        lease: OperationLease,
    ) -> Result<AuditResultClass, LaunchDenialCode> {
        self.remove_exact(lease)?;
        Ok(AuditResultClass::Completed)
    }

    pub fn cancel(
        &mut self,
        lease: OperationLease,
    ) -> Result<AuditResultClass, LaunchDenialCode> {
        self.remove_exact(lease)?;
        Ok(AuditResultClass::Cancelled)
    }

    fn remove_exact(&mut self, lease: OperationLease) -> Result<(), LaunchDenialCode> {
        if self
            .operations
            .get(&lease.operation_id)
            .is_some_and(|binding| binding.lease == lease)
        {
            self.operations.remove(&lease.operation_id);
            Ok(())
        } else {
            Err(LaunchDenialCode::StaleOperationLease)
        }
    }

    pub fn revoke_session(&mut self, session_id: SessionId) -> usize {
        self.sessions.remove(&session_id);
        let before = self.operations.len();
        self.operations
            .retain(|_, binding| binding.lease.session_id != session_id);
        before - self.operations.len()
    }

    pub fn revoke_extension(&mut self, extension_id: ExtensionId) -> usize {
        if extension_id.as_str() != REVIEWED_EXTENSION_ID {
            return 0;
        }
        self.revoked_extensions.insert(extension_id.clone());
        let before = self.operations.len();
        self.operations
            .retain(|_, binding| binding.extension_id != extension_id);
        before - self.operations.len()
    }
}

fn operation_kind(request: &LaunchRequest) -> LaunchOperationKind {
    match OpenSshExecutable::parse(&request.executable) {
        Some(OpenSshExecutable::Ssh) => LaunchOperationKind::OpenSshConnect,
        _ => LaunchOperationKind::Unsupported,
    }
}

fn validate_operation(request: &LaunchRequest) -> Result<(), LaunchDenialCode> {
    let executable = OpenSshExecutable::parse(&request.executable)
        .ok_or(LaunchDenialCode::UnsupportedExecutable)?;
    if executable != OpenSshExecutable::Ssh {
        return Err(LaunchDenialCode::UnsupportedOperation);
    }
    if request.arguments.len() != 1 {
        return Err(LaunchDenialCode::InvalidArguments);
    }
    let total_bytes = request
        .arguments
        .iter()
        .try_fold(0_usize, |total, argument| {
            total.checked_add(argument.as_str().len())
        })
        .ok_or(LaunchDenialCode::InvalidArguments)?;
    if total_bytes > MAX_TOTAL_ARGUMENT_BYTES {
        return Err(LaunchDenialCode::InvalidArguments);
    }
    validate_destination(request.arguments[0].as_str())
}

fn validate_destination(destination: &str) -> Result<(), LaunchDenialCode> {
    if destination.starts_with('-') {
        return Err(LaunchDenialCode::OptionConfusedDestination);
    }
    if destination.is_empty()
        || destination.len() > MAX_DESTINATION_BYTES
        || destination
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(LaunchDenialCode::InvalidArguments);
    }
    Ok(())
}

fn validate_trusted_environment(
    environment: &[(String, String)],
) -> Result<Vec<(String, String)>, LaunchDenialCode> {
    if environment.len() > MAX_TRUSTED_ENVIRONMENT_ENTRIES {
        return Err(LaunchDenialCode::InvalidTrustedEnvironment);
    }
    let mut names = BTreeSet::new();
    let mut total_bytes = 0_usize;
    for (name, value) in environment {
        if name.is_empty()
            || name.len() > MAX_ENVIRONMENT_NAME_BYTES
            || name.contains('=')
            || name.chars().any(char::is_control)
            || value.len() > MAX_ENVIRONMENT_VALUE_BYTES
            || value.contains('\0')
        {
            return Err(LaunchDenialCode::InvalidTrustedEnvironment);
        }
        #[cfg(target_os = "windows")]
        let deduplication_name = name.to_ascii_uppercase();
        #[cfg(not(target_os = "windows"))]
        let deduplication_name = name.clone();
        if !names.insert(deduplication_name) {
            return Err(LaunchDenialCode::InvalidTrustedEnvironment);
        }
        total_bytes = total_bytes
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or(LaunchDenialCode::InvalidTrustedEnvironment)?;
        if total_bytes > MAX_TOTAL_ENVIRONMENT_BYTES {
            return Err(LaunchDenialCode::InvalidTrustedEnvironment);
        }
    }
    Ok(environment.to_vec())
}

fn resolve_working_directory(
    requested: Option<&str>,
    safe_default: &Path,
) -> Result<(PathBuf, PathBuf, WorkingDirectoryDisposition), LaunchDenialCode> {
    let safe_default = canonical_directory(safe_default)
        .ok_or(LaunchDenialCode::InvalidWorkingDirectory)?;
    let Some(requested) = requested else {
        return Ok((
            safe_default.clone(),
            safe_default,
            WorkingDirectoryDisposition::SafeDefaultNotRequested,
        ));
    };
    let requested = Path::new(requested);
    if !requested.is_absolute() {
        return Err(LaunchDenialCode::InvalidWorkingDirectory);
    }
    match canonical_directory(requested) {
        Some(requested) => Ok((
            requested,
            safe_default,
            WorkingDirectoryDisposition::Requested,
        )),
        None => Ok((
            safe_default.clone(),
            safe_default,
            WorkingDirectoryDisposition::SafeDefaultMissing,
        )),
    }
}

fn audit_decision(decision: Decision) -> AuditDecision {
    match decision {
        Decision::AllowOnce => AuditDecision::AllowOnce,
        Decision::AllowSession => AuditDecision::AllowSession,
        Decision::Deny => AuditDecision::Deny,
    }
}

fn audit_record(
    submission: &LaunchSubmission<'_>,
    operation_kind: LaunchOperationKind,
    result: AuditResultClass,
) -> LaunchAuditRecord {
    LaunchAuditRecord {
        extension_id: submission.principal.id.clone(),
        extension_version: submission.principal.version.clone(),
        publisher: submission.principal.publisher.clone(),
        decision: audit_decision(submission.decision.decision),
        operation_kind,
        // D4 will provide an opaque public connection ID. Never infer it from
        // the destination argument because it may contain a private username.
        public_connection_id: None,
        operation_id: submission.launch.operation_id,
        session_id: submission.launch.session_id,
        timestamp_ms: submission.now_ms,
        duration_ms: 0,
        result,
    }
}

fn deny(
    submission: &LaunchSubmission<'_>,
    code: LaunchDenialCode,
    operation_kind: LaunchOperationKind,
) -> Box<DeniedLaunch> {
    Box::new(DeniedLaunch {
        code,
        audit: audit_record(submission, operation_kind, AuditResultClass::Denied(code)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_extension_api::{BoundedText, SecretReference};
    use proptest::prelude::*;
    use std::ffi::OsStr;
    use std::io::Write;
    use tempfile::TempDir;

    struct TestRequest {
        principal: VerifiedExtension,
        capability: CapabilityRequest,
        decision: CapabilityDecision,
        launch: LaunchRequest,
        environment: Vec<(String, String)>,
        safe_default: PathBuf,
    }

    impl TestRequest {
        fn new(safe_default: PathBuf, destination: &str) -> Self {
            let operation_id = OperationId::new(41);
            let session_id = SessionId::new(7);
            let capsule_revision = 3;
            let executable = ExecutableId::new("ssh").unwrap();
            let capability = CapabilityRequest::new(
                operation_id,
                ExtensionId::new(REVIEWED_EXTENSION_ID).unwrap(),
                session_id,
                capsule_revision,
                Capability::SessionLaunch,
                ResourceScope::Executable(executable.clone()),
                BoundedText::new("Open a reviewed SSH connection").unwrap(),
            )
            .unwrap();
            let decision = CapabilityDecision::for_request(
                &capability,
                Decision::AllowOnce,
                90,
                110,
            )
            .unwrap();
            Self {
                principal: reviewed_principal(),
                capability,
                decision,
                launch: LaunchRequest::new(
                    operation_id,
                    session_id,
                    capsule_revision,
                    executable,
                    vec![BoundedText::new(destination).unwrap()],
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    LaunchKind::Native,
                )
                .unwrap(),
                environment: vec![("AUTOMEXIA_PUBLIC_PROFILE".into(), "staging".into())],
                safe_default,
            }
        }

        fn refresh_decision(&mut self) {
            self.decision = CapabilityDecision::for_request(
                &self.capability,
                self.decision.decision,
                self.decision.decided_at_ms,
                self.decision.expires_at_ms,
            )
            .unwrap();
        }

        fn set_operation_id(&mut self, operation_id: OperationId) {
            self.launch.operation_id = operation_id;
            self.capability.operation_id = operation_id;
            self.refresh_decision();
        }

        fn set_scope(
            &mut self,
            operation_id: OperationId,
            session_id: SessionId,
            capsule_revision: u64,
        ) {
            self.launch.operation_id = operation_id;
            self.launch.session_id = session_id;
            self.launch.capsule_revision = capsule_revision;
            self.capability.operation_id = operation_id;
            self.capability.session_id = session_id;
            self.capability.capsule_revision = capsule_revision;
            self.refresh_decision();
        }

        fn submission(&self) -> LaunchSubmission<'_> {
            LaunchSubmission {
                principal: &self.principal,
                capability: &self.capability,
                decision: &self.decision,
                launch: &self.launch,
                trusted_environment: &self.environment,
                safe_default_working_directory: &self.safe_default,
                now_ms: 100,
            }
        }
    }

    struct ExecutableFixture {
        _directory: TempDir,
        executable: PathBuf,
        safe_default: PathBuf,
    }

    impl ExecutableFixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let executable = directory.path().join(OpenSshExecutable::Ssh.filename());
            let mut file = fs::File::create(&executable).unwrap();
            file.write_all(b"review-fixture").unwrap();
            file.sync_all().unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
                    .unwrap();
            }
            let safe_default = directory.path().join("safe");
            fs::create_dir(&safe_default).unwrap();
            Self {
                _directory: directory,
                executable,
                safe_default,
            }
        }

        fn policy(&self) -> ExecutablePolicy {
            ExecutablePolicy::default()
                .with_configured_path(OpenSshExecutable::Ssh, self.executable.clone())
                .unwrap()
        }
    }

    fn review_package_digest() -> PackageDigest {
        PackageDigest::new([0xa5; 32]).unwrap()
    }

    fn reviewed_package_policy() -> ReviewedPackagePolicy {
        ReviewedPackagePolicy::new(
            ExtensionId::new(REVIEWED_EXTENSION_ID).unwrap(),
            REVIEWED_PUBLISHER,
            env!("CARGO_PKG_VERSION"),
            review_package_digest(),
            REVIEWED_CONTRACT_VERSION,
        )
        .unwrap()
    }

    fn reviewed_principal() -> VerifiedExtension {
        VerifiedExtension::new(
            ExtensionId::new(REVIEWED_EXTENSION_ID).unwrap(),
            REVIEWED_PUBLISHER,
            env!("CARGO_PKG_VERSION"),
            review_package_digest(),
            REVIEWED_CONTRACT_VERSION,
            PackageVerification::RepositoryReviewed,
        )
        .unwrap()
    }

    fn review_broker(
        fixture: &ExecutableFixture,
        request: &TestRequest,
    ) -> CapabilityBroker {
        let mut broker =
            CapabilityBroker::review_harness(fixture.policy(), reviewed_package_policy());
        broker
            .register_session(request.launch.session_id, request.launch.capsule_revision)
            .unwrap();
        broker
    }

    #[test]
    fn production_broker_is_a_hard_denial_before_resolution() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "prod-alias");
        let mut broker = CapabilityBroker::pending_security_review(
            ExecutablePolicy::default(),
            reviewed_package_policy(),
        );
        let denied = broker.authorize(request.submission()).unwrap_err();
        assert_eq!(denied.code, LaunchDenialCode::PendingSecurityReview);
        assert_eq!(denied.audit.result, AuditResultClass::Denied(denied.code));
    }

    #[test]
    fn reviewed_exact_request_reaches_the_existing_launch_descriptor_seam() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "prod-alias");
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        let descriptor = prepared.session_launch_descriptor().unwrap();
        assert_eq!(descriptor.args(), &["prod-alias"]);
        assert_eq!(descriptor.environment(), request.environment);
        assert_eq!(
            Path::new(descriptor.program().unwrap()),
            canonical_existing(&fixture.executable).unwrap()
        );
        assert_eq!(
            prepared.audit().operation_kind,
            LaunchOperationKind::OpenSshConnect
        );
    }

    #[test]
    fn exact_arguments_are_never_joined_or_sent_through_a_shell() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "user@example.test");
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        let command = prepared.test_command();
        assert_eq!(
            command.get_program(),
            canonical_existing(&fixture.executable).unwrap().as_os_str()
        );
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("user@example.test")]
        );
    }

    #[test]
    fn leading_dash_extra_arguments_and_shell_executables_are_denied() {
        let fixture = ExecutableFixture::new();
        let option = TestRequest::new(fixture.safe_default.clone(), "-oProxyCommand=bad");
        let mut broker = review_broker(&fixture, &option);
        assert_eq!(
            broker.authorize(option.submission()).unwrap_err().code,
            LaunchDenialCode::OptionConfusedDestination
        );

        let mut extra = TestRequest::new(fixture.safe_default.clone(), "host");
        extra
            .launch
            .arguments
            .push(BoundedText::new("remote-command").unwrap());
        assert_eq!(
            broker.authorize(extra.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidArguments
        );

        let mut shell = TestRequest::new(fixture.safe_default.clone(), "host");
        shell.launch.executable = ExecutableId::new("cmd.exe").unwrap();
        shell.capability.resource =
            ResourceScope::Executable(shell.launch.executable.clone());
        shell.refresh_decision();
        assert_eq!(
            broker.authorize(shell.submission()).unwrap_err().code,
            LaunchDenialCode::UnsupportedExecutable
        );
    }

    #[test]
    fn broad_spawn_wsl_environment_and_secret_requests_are_denied() {
        let fixture = ExecutableFixture::new();
        let mut broad = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &broad);
        broad.capability.capability = Capability::ProcessSpawn;
        assert_eq!(
            broker.authorize(broad.submission()).unwrap_err().code,
            LaunchDenialCode::CapabilityMismatch
        );

        let mut wsl = TestRequest::new(fixture.safe_default.clone(), "host");
        wsl.launch.kind = LaunchKind::Wsl {
            distribution: BoundedText::new("Ubuntu").unwrap(),
            user: None,
            shell: None,
        };
        assert_eq!(
            broker.authorize(wsl.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidScope
        );

        let mut environment = TestRequest::new(fixture.safe_default.clone(), "host");
        environment
            .launch
            .inherited_environment
            .push(BoundedText::new("SSH_AUTH_SOCK").unwrap());
        assert_eq!(
            broker.authorize(environment.submission()).unwrap_err().code,
            LaunchDenialCode::ExtensionEnvironmentAccess
        );

        let mut secret = TestRequest::new(fixture.safe_default.clone(), "host");
        secret
            .launch
            .secret_references
            .push(SecretReference::new("vault.prod.key").unwrap());
        assert_eq!(
            broker.authorize(secret.submission()).unwrap_err().code,
            LaunchDenialCode::SecretResolutionUnavailable
        );
    }

    #[test]
    fn mismatched_scope_decision_principal_and_future_timestamp_are_denied() {
        let fixture = ExecutableFixture::new();
        let mut scope = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &scope);
        scope.capability.session_id = SessionId::new(99);
        assert_eq!(
            broker.authorize(scope.submission()).unwrap_err().code,
            LaunchDenialCode::CapabilityMismatch
        );

        let mut decision = TestRequest::new(fixture.safe_default.clone(), "host");
        decision.decision.operation_id = OperationId::new(99);
        assert_eq!(
            broker.authorize(decision.submission()).unwrap_err().code,
            LaunchDenialCode::DecisionMismatch
        );

        let mut future = TestRequest::new(fixture.safe_default.clone(), "host");
        future.decision.decided_at_ms = 101;
        assert_eq!(
            broker.authorize(future.submission()).unwrap_err().code,
            LaunchDenialCode::DecisionFromFuture
        );

        let mut denied = TestRequest::new(fixture.safe_default.clone(), "host");
        denied.decision.decision = Decision::Deny;
        assert_eq!(
            broker.authorize(denied.submission()).unwrap_err().code,
            LaunchDenialCode::DecisionDenied
        );

        let mut zero_scope = TestRequest::new(fixture.safe_default.clone(), "host");
        zero_scope.launch.operation_id = OperationId::new(0);
        zero_scope.capability.operation_id = OperationId::new(0);
        zero_scope.decision.operation_id = OperationId::new(0);
        assert_eq!(
            broker.authorize(zero_scope.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidScope
        );

        let mut unreviewed = TestRequest::new(fixture.safe_default.clone(), "host");
        unreviewed.principal = VerifiedExtension::new(
            ExtensionId::new("third.party").unwrap(),
            "third.party",
            env!("CARGO_PKG_VERSION"),
            review_package_digest(),
            REVIEWED_CONTRACT_VERSION,
            PackageVerification::RepositoryReviewed,
        )
        .unwrap();
        unreviewed.capability.extension_id = unreviewed.principal.id.clone();
        assert_eq!(
            broker.authorize(unreviewed.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidPrincipal
        );

        let mut wrong_version = TestRequest::new(fixture.safe_default.clone(), "host");
        wrong_version.principal.version = "999.0.0".into();
        assert_eq!(
            broker
                .authorize(wrong_version.submission())
                .unwrap_err()
                .code,
            LaunchDenialCode::InvalidPrincipal
        );
    }

    #[test]
    fn recognized_but_unreviewed_openssh_tools_remain_denied() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        for executable in [OpenSshExecutable::SshAdd, OpenSshExecutable::SshKeygen] {
            let mut request = TestRequest::new(fixture.safe_default.clone(), "host");
            request.launch.executable = ExecutableId::new(executable.id()).unwrap();
            request.capability.resource =
                ResourceScope::Executable(request.launch.executable.clone());
            request.refresh_decision();
            assert_eq!(
                broker.authorize(request.submission()).unwrap_err().code,
                LaunchDenialCode::UnsupportedOperation
            );
        }
    }

    #[test]
    fn package_identity_digest_compatibility_and_verification_fail_closed() {
        assert_eq!(
            PackageDigest::new([0; 32]).unwrap_err(),
            LaunchDenialCode::InvalidPrincipal
        );
        assert_eq!(
            ReviewedPackagePolicy::new(
                ExtensionId::new(REVIEWED_EXTENSION_ID).unwrap(),
                REVIEWED_PUBLISHER,
                "999.0.0",
                review_package_digest(),
                REVIEWED_CONTRACT_VERSION,
            )
            .unwrap_err(),
            LaunchDenialCode::InvalidPrincipal
        );

        let fixture = ExecutableFixture::new();
        let seed = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &seed);

        let mut unverified = TestRequest::new(fixture.safe_default.clone(), "host");
        unverified.principal.verification = PackageVerification::Unverified;
        assert_eq!(
            broker.authorize(unverified.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidPrincipal
        );

        let mut wrong_digest = TestRequest::new(fixture.safe_default.clone(), "host");
        wrong_digest.principal.package_digest = PackageDigest::new([0x5a; 32]).unwrap();
        assert_eq!(
            broker
                .authorize(wrong_digest.submission())
                .unwrap_err()
                .code,
            LaunchDenialCode::InvalidPrincipal
        );

        let mut wrong_contract = TestRequest::new(fixture.safe_default.clone(), "host");
        wrong_contract.principal.contract_version = REVIEWED_CONTRACT_VERSION + 1;
        assert_eq!(
            broker
                .authorize(wrong_contract.submission())
                .unwrap_err()
                .code,
            LaunchDenialCode::InvalidPrincipal
        );

        let mut signed = TestRequest::new(fixture.safe_default.clone(), "host");
        signed.principal.verification = PackageVerification::FirstPartySigned;
        let prepared = broker.authorize(signed.submission()).unwrap();
        broker.complete(prepared.lease()).unwrap();
    }

    #[test]
    fn platform_resolution_contract_is_fixed_and_wsl_remains_disabled() {
        let system_directory = PathBuf::from("/fixed/windows/System32");
        assert_eq!(
            platform_candidate_roots(
                OpenSshHostPlatform::Windows,
                Some(system_directory.as_path()),
            ),
            vec![system_directory.join("OpenSSH")]
        );
        assert_eq!(
            platform_candidate_roots(OpenSshHostPlatform::MacOs, None),
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
            ]
        );
        assert_eq!(
            platform_candidate_roots(OpenSshHostPlatform::Linux, None),
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/bin"),
                PathBuf::from("/usr/local/bin"),
            ]
        );
        assert!(platform_candidate_roots(OpenSshHostPlatform::Windows, None).is_empty());
        assert!(platform_candidate_roots(OpenSshHostPlatform::Wsl, None).is_empty());
        assert!(
            platform_candidate_roots(OpenSshHostPlatform::Unsupported, None).is_empty()
        );
    }

    #[test]
    fn resolver_never_accepts_relative_alias_or_unapproved_filename() {
        assert_eq!(
            ExecutablePolicy::default()
                .with_configured_path(OpenSshExecutable::Ssh, PathBuf::from("ssh"))
                .unwrap_err(),
            LaunchDenialCode::UnsupportedExecutable
        );
        let fixture = ExecutableFixture::new();
        assert_eq!(
            ExecutablePolicy::default()
                .with_configured_path(
                    OpenSshExecutable::Ssh,
                    fixture.executable.with_file_name("renamed.exe"),
                )
                .unwrap_err(),
            LaunchDenialCode::UnsupportedExecutable
        );
        assert_eq!(
            ExecutablePolicy::default()
                .resolve(&ExecutableId::new("ssh").unwrap())
                .unwrap_err(),
            LaunchDenialCode::ExecutableUnavailable
        );
    }

    #[test]
    fn host_defaults_are_fixed_absolute_locations_and_never_path_entries() {
        let policy = ExecutablePolicy::host_defaults();
        assert!(policy
            .candidates
            .values()
            .flatten()
            .all(|candidate| candidate.is_absolute()));
        assert!(policy
            .candidates
            .keys()
            .all(|id| OpenSshExecutable::ALL.iter().any(|known| known.id() == *id)));
    }

    #[test]
    fn configured_executable_override_is_fail_closed() {
        let fixture = ExecutableFixture::new();
        let missing = fixture
            ._directory
            .path()
            .join("missing")
            .join(OpenSshExecutable::Ssh.filename());
        let policy = ExecutablePolicy::host_defaults()
            .with_configured_path(OpenSshExecutable::Ssh, missing.clone())
            .unwrap();
        assert_eq!(policy.candidates.get("ssh"), Some(&vec![missing]));
        assert_eq!(
            policy
                .resolve(&ExecutableId::new("ssh").unwrap())
                .unwrap_err(),
            LaunchDenialCode::ExecutableUnavailable
        );
    }

    #[test]
    fn executable_replacement_is_detected_even_when_size_is_unchanged() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        let old = fixture.executable.with_extension("old");
        fs::rename(&fixture.executable, &old).unwrap();
        fs::write(&fixture.executable, b"review-fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&fixture.executable, fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        assert_eq!(
            prepared.revalidate_executable().unwrap_err(),
            LaunchDenialCode::ExecutableIdentityChanged
        );
        assert_eq!(
            prepared.session_launch_descriptor().unwrap_err(),
            LaunchDenialCode::ExecutableIdentityChanged
        );
    }

    #[test]
    fn vanished_absolute_cwd_falls_back_but_relative_cwd_is_rejected() {
        let fixture = ExecutableFixture::new();
        let missing = fixture._directory.path().join("missing");
        let mut request = TestRequest::new(fixture.safe_default.clone(), "host");
        request.launch.working_directory =
            Some(BoundedText::new(missing.to_string_lossy().into_owned()).unwrap());
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        assert_eq!(
            prepared.working_directory_disposition,
            WorkingDirectoryDisposition::SafeDefaultMissing
        );
        assert_eq!(
            prepared.revalidated_working_directory().unwrap(),
            canonical_existing(&fixture.safe_default).unwrap()
        );

        let mut relative = TestRequest::new(fixture.safe_default.clone(), "host");
        relative.set_operation_id(OperationId::new(42));
        relative.launch.working_directory = Some(BoundedText::new("relative").unwrap());
        assert_eq!(
            broker.authorize(relative.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidWorkingDirectory
        );
    }

    #[test]
    fn requested_cwd_that_vanishes_after_authorization_restores_safe_default() {
        let fixture = ExecutableFixture::new();
        let requested = fixture._directory.path().join("requested");
        fs::create_dir(&requested).unwrap();
        let mut request = TestRequest::new(fixture.safe_default.clone(), "host");
        request.launch.working_directory =
            Some(BoundedText::new(requested.to_string_lossy().into_owned()).unwrap());
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        assert_eq!(
            prepared.working_directory_disposition,
            WorkingDirectoryDisposition::Requested
        );
        fs::remove_dir(&requested).unwrap();
        let expected = canonical_existing(&fixture.safe_default).unwrap();
        assert_eq!(prepared.revalidated_working_directory().unwrap(), expected);
        assert_eq!(
            prepared
                .session_launch_descriptor()
                .unwrap()
                .starting_directory(),
            expected.to_str()
        );
    }

    #[test]
    fn trusted_environment_is_bounded_and_never_appears_in_debug_or_audit() {
        let fixture = ExecutableFixture::new();
        let mut request =
            TestRequest::new(fixture.safe_default.clone(), "private-user@host");
        request.environment =
            vec![("SSH_AUTH_SOCK".into(), "CANARY-SECRET-SOCKET".into())];
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        let combined = format!("{prepared:?} {:?}", prepared.audit());
        assert!(!combined.contains("CANARY-SECRET-SOCKET"));
        assert!(!combined.contains("private-user@host"));
        assert!(!combined.contains(&fixture.safe_default.to_string_lossy().to_string()));

        let mut invalid = TestRequest::new(fixture.safe_default.clone(), "host");
        invalid.set_operation_id(OperationId::new(42));
        invalid.environment = vec![("BAD=NAME".into(), "value".into())];
        assert_eq!(
            broker.authorize(invalid.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidTrustedEnvironment
        );

        let mut oversized = TestRequest::new(fixture.safe_default.clone(), "host");
        oversized.set_operation_id(OperationId::new(43));
        oversized.environment = vec![(
            "PUBLIC_VALUE".into(),
            "x".repeat(MAX_ENVIRONMENT_VALUE_BYTES + 1),
        )];
        assert_eq!(
            broker.authorize(oversized.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidTrustedEnvironment
        );
    }

    #[test]
    fn duplicate_replay_cancel_and_revocation_are_scope_safe() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        let first = broker.authorize(request.submission()).unwrap();
        let first_lease = first.lease();
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::DuplicateOperation
        );
        broker.cancel(first_lease).unwrap();
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::ReplayedOperation
        );

        let mut next_request = TestRequest::new(fixture.safe_default.clone(), "host");
        next_request.set_operation_id(OperationId::new(42));
        let second = broker.authorize(next_request.submission()).unwrap();
        let second_lease = second.lease();
        assert_ne!(first_lease.nonce, second_lease.nonce);
        assert_eq!(second_lease.operation_id(), OperationId::new(42));
        assert_eq!(second_lease.session_id(), SessionId::new(7));
        assert_eq!(second_lease.capsule_revision(), 3);
        assert_eq!(
            broker.cancel(first_lease).unwrap_err(),
            LaunchDenialCode::StaleOperationLease
        );
        assert_eq!(broker.revoke_session(second_lease.session_id), 1);
        assert_eq!(
            broker.cancel(second_lease).unwrap_err(),
            LaunchDenialCode::StaleOperationLease
        );
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::Revoked
        );
    }

    #[test]
    fn extension_revocation_does_not_remove_another_extension_binding() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        assert_eq!(
            broker.revoke_extension(ExtensionId::new("unrelated.extension").unwrap()),
            0
        );
        assert!(broker.revoked_extensions.is_empty());
        broker.complete(prepared.lease()).unwrap();
    }

    #[test]
    fn allow_session_decision_is_preserved_without_becoming_a_global_grant() {
        let fixture = ExecutableFixture::new();
        let mut request = TestRequest::new(fixture.safe_default.clone(), "host");
        request.decision.decision = Decision::AllowSession;
        let mut broker = review_broker(&fixture, &request);
        let prepared = broker.authorize(request.submission()).unwrap();
        assert_eq!(prepared.audit().decision, AuditDecision::AllowSession);
        broker.revoke_session(prepared.lease().session_id);
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::Revoked
        );
    }

    #[test]
    fn decisions_are_expiring_and_bound_to_registered_capsule_scope() {
        let fixture = ExecutableFixture::new();
        let mut expired = TestRequest::new(fixture.safe_default.clone(), "host");
        expired.decision.expires_at_ms = 99;
        let mut broker = review_broker(&fixture, &expired);
        assert_eq!(
            broker.authorize(expired.submission()).unwrap_err().code,
            LaunchDenialCode::DecisionExpired
        );

        let unregistered = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker =
            CapabilityBroker::review_harness(fixture.policy(), reviewed_package_policy());
        assert_eq!(
            broker
                .authorize(unregistered.submission())
                .unwrap_err()
                .code,
            LaunchDenialCode::Revoked
        );

        let mut wrong_capsule = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &wrong_capsule);
        wrong_capsule.launch.capsule_revision = 4;
        wrong_capsule.capability.capsule_revision = 4;
        wrong_capsule.refresh_decision();
        assert_eq!(
            broker
                .authorize(wrong_capsule.submission())
                .unwrap_err()
                .code,
            LaunchDenialCode::InvalidScope
        );
    }

    #[test]
    fn capsule_rebind_cancels_old_work_without_reusing_scope() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        let old_lease = broker.authorize(request.submission()).unwrap().lease();
        assert_eq!(broker.rebind_session(SessionId::new(7), 4).unwrap(), 1);
        assert_eq!(
            broker.complete(old_lease).unwrap_err(),
            LaunchDenialCode::StaleOperationLease
        );
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::InvalidScope
        );

        let mut rebound = TestRequest::new(fixture.safe_default.clone(), "host");
        rebound.set_scope(OperationId::new(42), SessionId::new(7), 4);
        let prepared = broker.authorize(rebound.submission()).unwrap();
        assert_eq!(prepared.lease().capsule_revision(), 4);
    }

    #[test]
    fn nonce_exhaustion_is_fail_closed_and_atomic() {
        let fixture = ExecutableFixture::new();
        let request = TestRequest::new(fixture.safe_default.clone(), "host");
        let mut broker = review_broker(&fixture, &request);
        broker.next_nonce = u64::MAX;
        assert_eq!(
            broker.authorize(request.submission()).unwrap_err().code,
            LaunchDenialCode::NonceExhausted
        );
        assert!(broker.operations.is_empty());
        assert_eq!(
            broker.sessions[&request.launch.session_id].last_operation_id,
            0
        );
        broker.next_nonce = 1;
        broker.authorize(request.submission()).unwrap();
    }

    #[test]
    fn one_ten_and_fifty_session_cycles_release_all_bounded_state() {
        let fixture = ExecutableFixture::new();
        for count in [1_u64, 10, 50] {
            let mut broker = CapabilityBroker::review_harness(
                fixture.policy(),
                reviewed_package_policy(),
            );
            for index in 1..=count {
                let mut request = TestRequest::new(fixture.safe_default.clone(), "host");
                request.set_scope(OperationId::new(1), SessionId::new(index), 1);
                broker
                    .register_session(request.launch.session_id, 1)
                    .unwrap();
                let lease = broker.authorize(request.submission()).unwrap().lease();
                broker.complete(lease).unwrap();
                assert_eq!(broker.revoke_session(request.launch.session_id), 0);
            }
            assert!(broker.operations.is_empty());
            assert!(broker.sessions.is_empty());
            assert_eq!(broker.highest_session_id, count);
            assert!(broker.revoked_extensions.is_empty());
        }
    }

    proptest! {
        #[test]
        fn accepted_destination_remains_one_literal_native_argument(
            destination in "[A-Za-z0-9][A-Za-z0-9._@:%+\\[\\]-]{0,120}"
        ) {
            validate_destination(&destination).unwrap();
            let mut command = std::process::Command::new("ssh");
            command.arg(&destination);
            prop_assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                vec![OsStr::new(&destination)]
            );
        }
    }
}
