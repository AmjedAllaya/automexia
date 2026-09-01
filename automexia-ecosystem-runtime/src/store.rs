use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use automexia_ecosystem::{
    decode_strict_json, portable_identifier, valid_digest, LifecycleState, Limits,
    VerificationReceipt,
};
use serde::{Deserialize, Serialize};

use crate::VerifiedBundle;

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const STATE_FILE: &str = "store-state.v1.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreErrorCode {
    InvalidRoot,
    LinkRejected,
    NotDirectory,
    Io,
    DiskPreflight,
    CacheLimit,
    ExtensionLimit,
    UnsafeReceipt,
    Collision,
    State,
    NotInstalled,
    PrivatePermissions,
}

#[derive(Debug)]
pub struct StoreError {
    pub code: StoreErrorCode,
    pub detail: String,
}

impl StoreError {
    fn new(code: StoreErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    fn io(error: std::io::Error) -> Self {
        Self::new(StoreErrorCode::Io, error.to_string())
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.detail)
    }
}

impl std::error::Error for StoreError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledGeneration {
    pub extension_id: String,
    pub version: String,
    pub package_sha256: String,
    pub installed_at_unix: u64,
    pub lifecycle: LifecycleState,
    pub grant_generation: u64,
    pub last_known_good: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreState {
    schema_version: u32,
    global_generation: u64,
    installed: Vec<InstalledGeneration>,
}

impl Default for StoreState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            global_generation: 1,
            installed: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupReceipt {
    pub affected_extensions: usize,
    pub cancelled_generation: u64,
    pub retained_verified_bundles: bool,
    pub selected_input_cleared: bool,
    pub endpoints_closed: bool,
    pub fallback_available: bool,
}

#[derive(Debug)]
pub struct PackageStore {
    root: PathBuf,
    state: StoreState,
}

impl PackageStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        if !root.is_absolute() || root.file_name().is_none_or(|name| name != "ecosystem")
        {
            return Err(StoreError::new(
                StoreErrorCode::InvalidRoot,
                "store root must be an absolute application-owned ecosystem directory",
            ));
        }
        ensure_directory(&root)?;
        ensure_directory(&root.join("packages"))?;
        ensure_directory(&root.join("staging"))?;
        let state = read_state(&root)?.unwrap_or_default();
        validate_store_state(&state)?;
        let mut store = Self { root, state };
        store.recover()?;
        Ok(store)
    }

    pub fn installed(&self) -> &[InstalledGeneration] {
        &self.state.installed
    }

    pub fn install_verified(
        &mut self,
        bundle: &VerifiedBundle,
        available_bytes: u64,
        now_unix: u64,
    ) -> Result<InstalledGeneration, StoreError> {
        let required = u64::try_from(bundle.expanded_bytes()).unwrap_or(u64::MAX);
        if available_bytes < required.saturating_add(Limits::MANIFEST_BYTES as u64) {
            return Err(StoreError::new(StoreErrorCode::DiskPreflight, "available disk space is below the bounded extraction and receipt requirement"));
        }
        let total_cache =
            self.state
                .installed
                .iter()
                .try_fold(required, |total, installed| {
                    let path = self.generation_path(installed);
                    directory_bytes(&path).map(|size| total.saturating_add(size))
                })?;
        if total_cache > Limits::CACHE_BYTES as u64 {
            return Err(StoreError::new(
                StoreErrorCode::CacheLimit,
                "package cache limit would be exceeded",
            ));
        }
        let unique_extensions = self
            .state
            .installed
            .iter()
            .map(|item| item.extension_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if !unique_extensions.contains(bundle.receipt.extension_id.as_str())
            && unique_extensions.len() >= Limits::INSTALLED_EXTENSIONS
        {
            return Err(StoreError::new(
                StoreErrorCode::ExtensionLimit,
                "installed extension limit reached",
            ));
        }
        validate_receipt(&bundle.receipt)?;

        let staging = self.root.join("staging").join(format!(
            ".install-{}-{}-{}",
            &bundle.receipt.package_sha256[..12],
            std::process::id(),
            STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging).map_err(StoreError::io)?;
        apply_private_permissions(&staging, true)?;
        let result = (|| {
            for (name, bytes) in bundle.entries() {
                write_entry(&staging, name, bytes)?;
            }
            let receipt_bytes =
                serde_json::to_vec_pretty(&bundle.receipt).map_err(|error| {
                    StoreError::new(StoreErrorCode::State, error.to_string())
                })?;
            write_entry(&staging, "verification-receipt.json", &receipt_bytes)?;
            sync_directory(&staging)?;

            let extension = self
                .root
                .join("packages")
                .join(&bundle.receipt.extension_id);
            ensure_directory(&extension)?;
            let version = extension.join(&bundle.receipt.version);
            ensure_directory(&version)?;
            let final_path = version.join(&bundle.receipt.package_sha256);
            if final_path.exists() {
                let existing = read_receipt(&final_path)?;
                if existing.package_sha256 != bundle.receipt.package_sha256 {
                    return Err(StoreError::new(
                        StoreErrorCode::Collision,
                        "existing generation receipt does not match",
                    ));
                }
                remove_owned_tree(&staging, &self.root.join("staging"))?;
            } else {
                fs::rename(&staging, &final_path).map_err(StoreError::io)?;
                sync_directory(&version)?;
            }

            for item in self
                .state
                .installed
                .iter_mut()
                .filter(|item| item.extension_id == bundle.receipt.extension_id)
            {
                item.last_known_good = false;
            }
            let generation = InstalledGeneration {
                extension_id: bundle.receipt.extension_id.clone(),
                version: bundle.receipt.version.clone(),
                package_sha256: bundle.receipt.package_sha256.clone(),
                installed_at_unix: now_unix,
                lifecycle: LifecycleState::InstalledDisabled,
                grant_generation: self.state.global_generation.saturating_add(1),
                last_known_good: true,
            };
            self.state
                .installed
                .retain(|item| item.package_sha256 != generation.package_sha256);
            self.state.installed.push(generation.clone());
            self.state.global_generation = generation.grant_generation;
            self.prune_retained_versions(&generation.extension_id)?;
            write_state(&self.root, &self.state)?;
            Ok(generation)
        })();
        if result.is_err() && staging.exists() {
            let _ = remove_owned_tree(&staging, &self.root.join("staging"));
        }
        result
    }

    pub fn disable(&mut self, extension_id: &str) -> Result<CleanupReceipt, StoreError> {
        let mut affected = 0;
        self.state.global_generation = self.state.global_generation.saturating_add(1);
        for item in self
            .state
            .installed
            .iter_mut()
            .filter(|item| item.extension_id == extension_id)
        {
            item.lifecycle = LifecycleState::Disabled;
            item.grant_generation = self.state.global_generation;
            affected += 1;
        }
        if affected == 0 {
            return Err(StoreError::new(
                StoreErrorCode::NotInstalled,
                "extension is not installed",
            ));
        }
        write_state(&self.root, &self.state)?;
        Ok(cleanup_receipt(
            affected,
            self.state.global_generation,
            true,
        ))
    }

    pub fn kill_switch(&mut self) -> Result<CleanupReceipt, StoreError> {
        self.state.global_generation = self.state.global_generation.saturating_add(1);
        for item in &mut self.state.installed {
            item.lifecycle = LifecycleState::Disabled;
            item.grant_generation = self.state.global_generation;
        }
        write_state(&self.root, &self.state)?;
        Ok(cleanup_receipt(
            self.state.installed.len(),
            self.state.global_generation,
            true,
        ))
    }

    pub fn uninstall(
        &mut self,
        extension_id: &str,
    ) -> Result<CleanupReceipt, StoreError> {
        if !portable_identifier(extension_id) {
            return Err(StoreError::new(
                StoreErrorCode::InvalidRoot,
                "extension ID is invalid",
            ));
        }
        let extension_path = self.root.join("packages").join(extension_id);
        if !extension_path.exists() {
            return Err(StoreError::new(
                StoreErrorCode::NotInstalled,
                "extension is not installed",
            ));
        }
        let affected = self
            .state
            .installed
            .iter()
            .filter(|item| item.extension_id == extension_id)
            .count();
        remove_owned_tree(&extension_path, &self.root.join("packages"))?;
        self.state
            .installed
            .retain(|item| item.extension_id != extension_id);
        self.state.global_generation = self.state.global_generation.saturating_add(1);
        write_state(&self.root, &self.state)?;
        Ok(cleanup_receipt(
            affected,
            self.state.global_generation,
            false,
        ))
    }

    pub fn recover(&mut self) -> Result<(), StoreError> {
        let staging = self.root.join("staging");
        let mut staging_entries = 0usize;
        for entry in fs::read_dir(&staging).map_err(StoreError::io)? {
            let entry = entry.map_err(StoreError::io)?;
            staging_entries = staging_entries.saturating_add(1);
            if staging_entries > Limits::QUEUED_CALLS_PER_EXTENSION {
                return Err(StoreError::new(
                    StoreErrorCode::ExtensionLimit,
                    "staging entry count exceeds its bounded recovery limit",
                ));
            }
            let name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| {
                    StoreError::new(
                        StoreErrorCode::LinkRejected,
                        "staging name is not UTF-8",
                    )
                })?
                .to_owned();
            if !name.starts_with(".install-") {
                return Err(StoreError::new(
                    StoreErrorCode::LinkRejected,
                    "staging contains an entry not owned by an interrupted install",
                ));
            }
            remove_owned_tree(&entry.path(), &staging)?;
        }

        let mut receipts = BTreeMap::new();
        let mut generation_counts = BTreeMap::<String, usize>::new();
        let packages = self.root.join("packages");
        for extension in bounded_directories(&packages, Limits::INSTALLED_EXTENSIONS)? {
            let extension_id = exact_utf8_file_name(&extension, "extension directory")?;
            if !portable_identifier(&extension_id) {
                return Err(StoreError::new(
                    StoreErrorCode::UnsafeReceipt,
                    "extension directory identity is invalid",
                ));
            }
            for version in bounded_directories(
                &extension,
                Limits::RETAINED_VERSIONS.saturating_mul(4),
            )? {
                let version_id = exact_utf8_file_name(&version, "version directory")?;
                for generation in bounded_directories(
                    &version,
                    Limits::RETAINED_VERSIONS.saturating_mul(4),
                )? {
                    let count =
                        generation_counts.entry(extension_id.clone()).or_default();
                    *count = count.saturating_add(1);
                    if *count > Limits::RETAINED_VERSIONS.saturating_add(1) {
                        return Err(StoreError::new(
                            StoreErrorCode::ExtensionLimit,
                            "recovery found more generations than retention plus one interrupted publish",
                        ));
                    }
                    let receipt = read_receipt(&generation)?;
                    validate_receipt(&receipt)?;
                    if extension_id != receipt.extension_id
                        || version_id != receipt.version
                        || generation.file_name().and_then(|name| name.to_str())
                            != Some(&receipt.package_sha256)
                    {
                        return Err(StoreError::new(
                            StoreErrorCode::UnsafeReceipt,
                            "generation path and receipt identity differ",
                        ));
                    }
                    if receipts
                        .insert(receipt.package_sha256.clone(), receipt)
                        .is_some()
                    {
                        return Err(StoreError::new(
                            StoreErrorCode::Collision,
                            "duplicate package digest exists in the managed store",
                        ));
                    }
                }
            }
        }
        self.state
            .installed
            .retain(|item| receipts.contains_key(&item.package_sha256));
        for receipt in receipts.values() {
            if self
                .state
                .installed
                .iter()
                .all(|item| item.package_sha256 != receipt.package_sha256)
            {
                self.state.global_generation =
                    self.state.global_generation.saturating_add(1);
                self.state.installed.push(InstalledGeneration {
                    extension_id: receipt.extension_id.clone(),
                    version: receipt.version.clone(),
                    package_sha256: receipt.package_sha256.clone(),
                    installed_at_unix: receipt.verified_at_unix,
                    lifecycle: LifecycleState::InstalledDisabled,
                    grant_generation: self.state.global_generation,
                    last_known_good: false,
                });
            }
        }
        let extension_ids = self
            .state
            .installed
            .iter()
            .map(|item| item.extension_id.clone())
            .collect::<BTreeSet<_>>();
        for extension_id in extension_ids {
            self.prune_retained_versions(&extension_id)?;
        }

        let mut newest = BTreeMap::<String, (usize, u64)>::new();
        for (index, item) in self.state.installed.iter().enumerate() {
            let candidate = (index, item.installed_at_unix);
            newest
                .entry(item.extension_id.clone())
                .and_modify(|current| {
                    if candidate.1 > current.1 {
                        *current = candidate;
                    }
                })
                .or_insert(candidate);
        }
        for item in &mut self.state.installed {
            item.last_known_good = false;
        }
        for (index, _) in newest.into_values() {
            self.state.installed[index].last_known_good = true;
        }
        write_state(&self.root, &self.state)
    }

    fn prune_retained_versions(&mut self, extension_id: &str) -> Result<(), StoreError> {
        let mut indices = self
            .state
            .installed
            .iter()
            .enumerate()
            .filter(|(_, item)| item.extension_id == extension_id)
            .map(|(index, item)| (index, item.installed_at_unix))
            .collect::<Vec<_>>();
        indices.sort_by_key(|(_, installed)| std::cmp::Reverse(*installed));
        let remove = indices
            .into_iter()
            .skip(Limits::RETAINED_VERSIONS)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for index in remove.into_iter().rev() {
            let generation = self.state.installed.remove(index);
            let path = self.generation_path(&generation);
            let version_path = path.parent().map(Path::to_path_buf);
            remove_owned_tree(&path, &self.root.join("packages"))?;
            if let Some(version_path) = version_path {
                remove_directory_if_empty(&version_path, &self.root.join("packages"))?;
            }
        }
        Ok(())
    }

    fn generation_path(&self, generation: &InstalledGeneration) -> PathBuf {
        self.root
            .join("packages")
            .join(&generation.extension_id)
            .join(&generation.version)
            .join(&generation.package_sha256)
    }
}

fn cleanup_receipt(
    affected_extensions: usize,
    generation: u64,
    retain: bool,
) -> CleanupReceipt {
    CleanupReceipt {
        affected_extensions,
        cancelled_generation: generation,
        retained_verified_bundles: retain,
        selected_input_cleared: true,
        endpoints_closed: true,
        fallback_available: true,
    }
}

fn validate_receipt(receipt: &VerificationReceipt) -> Result<(), StoreError> {
    if receipt.schema_version != 1
        || !portable_identifier(&receipt.extension_id)
        || !valid_digest(&receipt.package_sha256)
        || !valid_digest(&receipt.content_sha256)
        || !valid_digest(&receipt.provenance_sha256)
        || !valid_digest(&receipt.sbom_sha256)
        || !valid_digest(&receipt.licenses_sha256)
        || !portable_identifier(&receipt.key_id)
        || !receipt.source_uri.starts_with("https://")
        || !automexia_ecosystem::safe_text(&receipt.source_uri, false)
        || !automexia_ecosystem::safe_text(&receipt.source_revision, false)
        || !portable_identifier(&receipt.builder_id)
        || receipt.built_at_unix == 0
        || receipt.built_at_unix > receipt.verified_at_unix
        || receipt.signature_expires_at_unix <= receipt.verified_at_unix
        || receipt.revocation_sequence == 0
        || receipt.extension_id != receipt.manifest.extension_id
        || receipt.version != receipt.manifest.version
        || receipt.publisher_id != receipt.manifest.publisher_id
        || receipt.manifest.validate(0).is_err()
    {
        return Err(StoreError::new(
            StoreErrorCode::UnsafeReceipt,
            "verification receipt is malformed or identity-mismatched",
        ));
    }
    Ok(())
}

fn validate_store_state(state: &StoreState) -> Result<(), StoreError> {
    if state.schema_version != 1 || state.global_generation == 0 {
        return Err(StoreError::new(
            StoreErrorCode::State,
            "store state schema or generation is invalid",
        ));
    }
    let mut digests = BTreeSet::new();
    for item in &state.installed {
        if !portable_identifier(&item.extension_id)
            || !valid_digest(&item.package_sha256)
            || item.grant_generation == 0
            || item.grant_generation > state.global_generation
            || !digests.insert(item.package_sha256.as_str())
        {
            return Err(StoreError::new(
                StoreErrorCode::State,
                "store state contains an invalid identity, generation, or duplicate digest",
            ));
        }
    }
    Ok(())
}

fn read_state(root: &Path) -> Result<Option<StoreState>, StoreError> {
    let path = root.join(STATE_FILE);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StoreError::io(error)),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > Limits::MANIFEST_BYTES as u64
    {
        return Err(StoreError::new(
            StoreErrorCode::State,
            "store state is linked, non-regular, or oversized",
        ));
    }
    let bytes = fs::read(&path).map_err(StoreError::io)?;
    decode_strict_json(&bytes, Limits::MANIFEST_BYTES)
        .map(Some)
        .map_err(|error| StoreError::new(StoreErrorCode::State, error.to_string()))
}

fn write_state(root: &Path, state: &StoreState) -> Result<(), StoreError> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| StoreError::new(StoreErrorCode::State, error.to_string()))?;
    let temporary = root.join(format!(".{STATE_FILE}.next"));
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(StoreError::io)?;
    }
    write_new_file(&temporary, &bytes)?;
    let destination = root.join(STATE_FILE);
    replace_file(&temporary, &destination)?;
    sync_directory(root)
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StoreError> {
    fs::rename(source, destination).map_err(StoreError::io)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), StoreError> {
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = windows_local_wide_path(source)?;
    let destination = windows_local_wide_path(destination)?;
    // SAFETY: both paths are owned NUL-terminated UTF-16 buffers and the flags
    // request an atomic same-volume replacement with write-through semantics.
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(StoreError::io(std::io::Error::last_os_error()));
    }
    Ok(())
}

fn read_receipt(generation: &Path) -> Result<VerificationReceipt, StoreError> {
    let path = generation.join("verification-receipt.json");
    let metadata = fs::symlink_metadata(&path).map_err(StoreError::io)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > Limits::MANIFEST_BYTES as u64
    {
        return Err(StoreError::new(
            StoreErrorCode::UnsafeReceipt,
            "receipt is linked, non-regular, or oversized",
        ));
    }
    let bytes = fs::read(path).map_err(StoreError::io)?;
    decode_strict_json(&bytes, Limits::MANIFEST_BYTES).map_err(|error| {
        StoreError::new(StoreErrorCode::UnsafeReceipt, error.to_string())
    })
}

fn exact_utf8_file_name(path: &Path, label: &str) -> Result<String, StoreError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            StoreError::new(
                StoreErrorCode::UnsafeReceipt,
                format!("{label} is missing or not UTF-8"),
            )
        })
}

fn remove_directory_if_empty(path: &Path, owner: &Path) -> Result<(), StoreError> {
    if path == owner || !path.starts_with(owner) {
        return Err(StoreError::new(
            StoreErrorCode::InvalidRoot,
            "refusing to remove an empty directory outside the owned store",
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreError::new(
            StoreErrorCode::LinkRejected,
            "empty-directory cleanup encountered a linked or non-directory path",
        ));
    }
    if fs::read_dir(path).map_err(StoreError::io)?.next().is_none() {
        fs::remove_dir(path).map_err(StoreError::io)?;
    }
    Ok(())
}

fn write_entry(root: &Path, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
    let mut path = root.to_path_buf();
    let segments = name.split('/').collect::<Vec<_>>();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        path.push(segment);
        ensure_directory(&path)?;
    }
    path.push(segments.last().ok_or_else(|| {
        StoreError::new(StoreErrorCode::InvalidRoot, "entry path is empty")
    })?);
    write_new_file(&path, bytes)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600).custom_flags(0x20000);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.custom_flags(0x0020_0000);
    }
    let mut file = options.open(path).map_err(StoreError::io)?;
    file.write_all(bytes).map_err(StoreError::io)?;
    file.sync_all().map_err(StoreError::io)?;
    apply_private_permissions(path, false)
}

fn ensure_directory(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(StoreError::new(
            StoreErrorCode::LinkRejected,
            "managed directory is a link or reparse point",
        )),
        Ok(metadata) if !metadata.is_dir() => Err(StoreError::new(
            StoreErrorCode::NotDirectory,
            "managed path is not a directory",
        )),
        Ok(_) => apply_private_permissions(path, true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let parent_metadata =
                        fs::symlink_metadata(parent).map_err(StoreError::io)?;
                    if parent_metadata.file_type().is_symlink()
                        || !parent_metadata.is_dir()
                    {
                        return Err(StoreError::new(
                            StoreErrorCode::LinkRejected,
                            "managed parent is not a safe directory",
                        ));
                    }
                }
            }
            fs::create_dir(path).map_err(StoreError::io)?;
            apply_private_permissions(path, true)
        }
        Err(error) => Err(StoreError::io(error)),
    }
}

fn bounded_directories(path: &Path, maximum: usize) -> Result<Vec<PathBuf>, StoreError> {
    let mut output = Vec::new();
    for entry in fs::read_dir(path).map_err(StoreError::io)? {
        let entry = entry.map_err(StoreError::io)?;
        let metadata = entry.metadata().map_err(StoreError::io)?;
        if entry.file_type().map_err(StoreError::io)?.is_symlink() || !metadata.is_dir() {
            return Err(StoreError::new(
                StoreErrorCode::LinkRejected,
                "managed store contains a link or non-directory",
            ));
        }
        output.push(entry.path());
        if output.len() > maximum {
            return Err(StoreError::new(
                StoreErrorCode::ExtensionLimit,
                "managed directory count exceeds its limit",
            ));
        }
    }
    Ok(output)
}

fn directory_bytes(path: &Path) -> Result<u64, StoreError> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    let mut files = 0usize;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory).map_err(StoreError::io)? {
            let entry = entry.map_err(StoreError::io)?;
            let file_type = entry.file_type().map_err(StoreError::io)?;
            if file_type.is_symlink() {
                return Err(StoreError::new(
                    StoreErrorCode::LinkRejected,
                    "managed store contains a link",
                ));
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                files += 1;
                if files > Limits::PACKAGE_FILES.saturating_mul(Limits::RETAINED_VERSIONS)
                {
                    return Err(StoreError::new(
                        StoreErrorCode::CacheLimit,
                        "managed generation file count exceeds its limit",
                    ));
                }
                total =
                    total.saturating_add(entry.metadata().map_err(StoreError::io)?.len());
            } else {
                return Err(StoreError::new(
                    StoreErrorCode::LinkRejected,
                    "managed store contains a special file",
                ));
            }
        }
    }
    Ok(total)
}

fn remove_owned_tree(path: &Path, owner: &Path) -> Result<(), StoreError> {
    if path.parent().is_none() || !path.starts_with(owner) || path == owner {
        return Err(StoreError::new(
            StoreErrorCode::InvalidRoot,
            "refusing to remove outside the exact owned subtree",
        ));
    }
    let metadata = fs::symlink_metadata(path).map_err(StoreError::io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(StoreError::new(
            StoreErrorCode::LinkRejected,
            "refusing to traverse linked or non-directory owned state",
        ));
    }
    for entry in fs::read_dir(path).map_err(StoreError::io)? {
        let entry = entry.map_err(StoreError::io)?;
        let file_type = entry.file_type().map_err(StoreError::io)?;
        if file_type.is_symlink() {
            return Err(StoreError::new(
                StoreErrorCode::LinkRejected,
                "refusing to remove a linked owned entry",
            ));
        }
        if file_type.is_dir() {
            remove_owned_tree(&entry.path(), path)?;
        } else if file_type.is_file() {
            fs::remove_file(entry.path()).map_err(StoreError::io)?;
        } else {
            return Err(StoreError::new(
                StoreErrorCode::LinkRejected,
                "refusing to remove a special owned entry",
            ));
        }
    }
    fs::remove_dir(path).map_err(StoreError::io)
}

#[cfg(unix)]
fn apply_private_permissions(path: &Path, directory: bool) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
    )
    .map_err(StoreError::io)
}

#[cfg(windows)]
fn apply_private_permissions(path: &Path, _directory: bool) -> Result<(), StoreError> {
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, GENERIC_ALL},
        Security::{
            Authorization::{
                SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
                NO_MULTIPLE_TRUSTEE, SET_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID,
                TRUSTEE_IS_USER, TRUSTEE_W,
            },
            GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION, NO_INHERITANCE,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: OpenProcessToken returned this owned handle.
                unsafe { CloseHandle(self.0) };
            }
        }
    }

    struct LocalAcl(*mut windows_sys::Win32::Security::ACL);
    impl Drop for LocalAcl {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: SetEntriesInAclW allocates the ACL with LocalAlloc.
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    let denied = |stage: &str, result: Option<u32>| {
        let suffix = result.map_or_else(String::new, |code| format!(" (error {code})"));
        StoreError::new(
            StoreErrorCode::PrivatePermissions,
            format!(
                "failed to enforce a protected current-user-only Windows ACL at {stage}{suffix}"
            ),
        )
    };
    let mut raw_token = ptr::null_mut();
    // SAFETY: output storage is valid and the returned handle is owned by Handle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw_token) } == 0
    {
        return Err(denied("OpenProcessToken", None));
    }
    let token = Handle(raw_token);
    let mut required = 0;
    // SAFETY: the zero-length probe is the documented way to obtain the size.
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(denied("GetTokenInformation size", None));
    }
    let mut buffer = vec![0u8; required as usize];
    // SAFETY: the byte buffer has the size returned by the probe.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(denied("GetTokenInformation value", None));
    }
    // SAFETY: TOKEN_USER may be unaligned in the byte buffer; the buffer
    // remains alive until the ACL has been installed.
    let user = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<TOKEN_USER>()) };
    let trustee = TRUSTEE_W {
        pMultipleTrustee: ptr::null_mut(),
        MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
        TrusteeForm: TRUSTEE_IS_SID,
        TrusteeType: TRUSTEE_IS_USER,
        ptstrName: user.User.Sid.cast(),
    };
    let access = EXPLICIT_ACCESS_W {
        grfAccessPermissions: GENERIC_ALL,
        grfAccessMode: SET_ACCESS,
        grfInheritance: NO_INHERITANCE,
        Trustee: trustee,
    };
    let mut raw_acl = ptr::null_mut();
    // SAFETY: all inputs have the exact Windows layouts and raw_acl is an
    // owned output released by LocalAcl.
    let result = unsafe { SetEntriesInAclW(1, &access, ptr::null(), &mut raw_acl) };
    if result != 0 {
        return Err(denied("SetEntriesInAclW", Some(result)));
    }
    let acl = LocalAcl(raw_acl);
    let wide = windows_local_wide_path(path)?;
    // SAFETY: wide is NUL-terminated and the ACL remains live for the call.
    let result = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            acl.0,
            ptr::null(),
        )
    };
    if result != 0 {
        return Err(denied("SetNamedSecurityInfoW", Some(result)));
    }
    if !private_permissions_are_safe(path)? {
        return Err(denied("post-write verification", None));
    }
    Ok(())
}

#[cfg(windows)]
fn windows_local_wide_path(path: &Path) -> Result<Vec<u16>, StoreError> {
    use std::{
        os::windows::ffi::OsStrExt as _,
        path::{Component, Prefix},
    };

    let prefix = match path.components().next() {
        Some(Component::Prefix(prefix)) if path.is_absolute() => prefix.kind(),
        _ => {
            return Err(StoreError::new(
                StoreErrorCode::InvalidRoot,
                "managed Windows paths must be absolute local drive paths",
            ))
        }
    };
    let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if wide.contains(&0) {
        return Err(StoreError::new(
            StoreErrorCode::InvalidRoot,
            "managed Windows paths cannot contain NUL",
        ));
    }
    // Rust accepts either separator in ordinary drive paths, while the Win32
    // verbatim namespace deliberately performs no slash normalization.
    for unit in &mut wide {
        if *unit == b'/' as u16 {
            *unit = b'\\' as u16;
        }
    }
    match prefix {
        Prefix::Disk(_) => {
            wide.splice(
                0..0,
                [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16],
            );
        }
        Prefix::VerbatimDisk(_) => {}
        _ => {
            return Err(StoreError::new(
                StoreErrorCode::InvalidRoot,
                "managed Windows paths must stay on a local drive",
            ))
        }
    };
    wide.push(0);
    Ok(wide)
}

#[cfg(not(any(unix, windows)))]
fn apply_private_permissions(_path: &Path, _directory: bool) -> Result<(), StoreError> {
    Err(StoreError::new(
        StoreErrorCode::PrivatePermissions,
        "private package-store permissions are unsupported on this platform",
    ))
}

#[cfg(windows)]
fn private_permissions_are_safe(path: &Path) -> Result<bool, StoreError> {
    use std::{mem, ptr};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            AclSizeInformation,
            Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
            GetAclInformation, GetSecurityDescriptorControl, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
        },
    };

    struct Descriptor(PSECURITY_DESCRIPTOR);
    impl Drop for Descriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: GetNamedSecurityInfoW returns LocalAlloc memory.
                unsafe { LocalFree(self.0.cast()) };
            }
        }
    }

    let wide = windows_local_wide_path(path)?;
    let mut acl = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    // SAFETY: the path is NUL-terminated and all requested outputs are valid.
    let result = unsafe {
        GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 || descriptor.is_null() || acl.is_null() {
        return Ok(false);
    }
    let descriptor = Descriptor(descriptor);
    let mut control = 0;
    let mut revision = 0;
    // SAFETY: the descriptor and scalar outputs remain valid for the call.
    if unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) }
        == 0
    {
        return Ok(false);
    }
    let mut information = ACL_SIZE_INFORMATION::default();
    // SAFETY: the ACL is owned by the descriptor and output layout is exact.
    if unsafe {
        GetAclInformation(
            acl,
            (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
            mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return Ok(false);
    }
    Ok(control & SE_DACL_PROTECTED != 0 && information.AceCount == 1)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(StoreError::io)
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use automexia_ecosystem::{
        Compatibility, EcosystemManifest, ExtensionKind, WIT_WORLD,
    };

    use super::*;

    fn verified(
        id: &str,
        version: &str,
        digest: char,
        installed_at: u64,
    ) -> VerifiedBundle {
        let package_sha256 = digest.to_string().repeat(64);
        let manifest = EcosystemManifest {
            schema_version: 1,
            extension_id: id.into(),
            display_name: "Example".into(),
            description: "Example".into(),
            publisher_id: "example.publisher".into(),
            version: version.into(),
            kind: ExtensionKind::Component,
            compatibility: Compatibility {
                sdk_major: 1,
                sdk_minor_minimum: 0,
                sdk_minor_maximum: 0,
            },
            world: WIT_WORLD.into(),
            imports: vec![],
            capabilities: vec![],
            action_pack_entry: None,
        };
        let receipt = VerificationReceipt {
            schema_version: 1,
            extension_id: id.into(),
            version: version.into(),
            publisher_id: "example.publisher".into(),
            package_sha256,
            content_sha256: "f".repeat(64),
            provenance_sha256: "e".repeat(64),
            sbom_sha256: "d".repeat(64),
            licenses_sha256: "c".repeat(64),
            key_id: "example.key".into(),
            source_uri: "https://example.invalid/source".into(),
            source_revision: "0123456789abcdef".into(),
            builder_id: "example.builder".into(),
            built_at_unix: installed_at.saturating_sub(1),
            signature_expires_at_unix: installed_at.saturating_add(100),
            verified_at_unix: installed_at,
            revocation_sequence: 1,
            manifest,
        };
        VerifiedBundle::from_verified_parts_for_test(
            receipt,
            BTreeMap::from([("manifest.json".into(), b"{}".to_vec())]),
        )
    }

    #[test]
    fn store_publishes_disabled_keeps_two_lkg_generations_and_uninstalls_only_owned_state(
    ) {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let mut store = PackageStore::open(&root).unwrap();
        for (version, digest, now) in
            [("1.0.0", 'a', 10), ("1.1.0", 'b', 20), ("1.2.0", 'c', 30)]
        {
            let generation = store
                .install_verified(
                    &verified("example.extension", version, digest, now),
                    10_000_000,
                    now,
                )
                .unwrap();
            assert_eq!(generation.lifecycle, LifecycleState::InstalledDisabled);
        }
        assert_eq!(store.installed().len(), 2);
        assert_eq!(
            store
                .installed()
                .iter()
                .filter(|item| item.last_known_good)
                .count(),
            1
        );
        let unrelated = temporary.path().join("user-file");
        fs::write(&unrelated, b"preserve").unwrap();
        let cleanup = store.uninstall("example.extension").unwrap();
        assert!(!cleanup.retained_verified_bundles);
        assert_eq!(fs::read(&unrelated).unwrap(), b"preserve");
        assert!(store.installed().is_empty());
    }

    #[test]
    fn recovery_reconstructs_an_atomically_published_generation_after_state_loss() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let mut store = PackageStore::open(&root).unwrap();
        store
            .install_verified(
                &verified("example.extension", "1.0.0", 'a', 10),
                10_000_000,
                10,
            )
            .unwrap();
        fs::remove_file(root.join(STATE_FILE)).unwrap();
        let recovered = PackageStore::open(root).unwrap();
        assert_eq!(recovered.installed().len(), 1);
        assert_eq!(
            recovered.installed()[0].lifecycle,
            LifecycleState::InstalledDisabled
        );
        assert!(recovered.installed()[0].last_known_good);
    }

    fn publish_without_state(root: &Path, extension_path: &str, bundle: &VerifiedBundle) {
        let generation = root
            .join("packages")
            .join(extension_path)
            .join(&bundle.receipt.version)
            .join(&bundle.receipt.package_sha256);
        ensure_directory(generation.parent().unwrap().parent().unwrap()).unwrap();
        ensure_directory(generation.parent().unwrap()).unwrap();
        ensure_directory(&generation).unwrap();
        for (name, bytes) in bundle.entries() {
            write_entry(&generation, name, bytes).unwrap();
        }
        let receipt = serde_json::to_vec_pretty(&bundle.receipt).unwrap();
        write_entry(&generation, "verification-receipt.json", &receipt).unwrap();
    }

    #[test]
    fn recovery_prunes_one_interrupted_publish_and_removes_its_empty_version_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let mut store = PackageStore::open(&root).unwrap();
        store
            .install_verified(
                &verified("example.extension", "1.0.0", 'a', 10),
                10_000_000,
                10,
            )
            .unwrap();
        store
            .install_verified(
                &verified("example.extension", "1.1.0", 'b', 20),
                10_000_000,
                20,
            )
            .unwrap();
        let interrupted = verified("example.extension", "1.2.0", 'c', 30);
        publish_without_state(&root, "example.extension", &interrupted);
        fs::remove_file(root.join(STATE_FILE)).unwrap();

        let recovered = PackageStore::open(&root).unwrap();
        assert_eq!(recovered.installed().len(), Limits::RETAINED_VERSIONS);
        assert!(recovered
            .installed()
            .iter()
            .any(|item| item.package_sha256 == interrupted.receipt.package_sha256));
        assert!(!root.join("packages/example.extension/1.0.0").exists());
    }

    #[test]
    fn recovery_rejects_cross_identity_generation_placement() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let _store = PackageStore::open(&root).unwrap();
        let foreign = verified("other.extension", "1.0.0", 'a', 10);
        publish_without_state(&root, "example.extension", &foreign);
        let error = PackageStore::open(root).unwrap_err();
        assert_eq!(error.code, StoreErrorCode::UnsafeReceipt);
    }

    #[test]
    fn recovery_rejects_unowned_staging_entries() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let _store = PackageStore::open(&root).unwrap();
        ensure_directory(&root.join("staging/not-an-install")).unwrap();
        let error = PackageStore::open(root).unwrap_err();
        assert_eq!(error.code, StoreErrorCode::LinkRejected);
    }
    #[test]
    fn disable_and_kill_rotate_generations_clear_transients_and_retain_verified_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let mut store = PackageStore::open(temporary.path().join("ecosystem")).unwrap();
        store
            .install_verified(
                &verified("example.extension", "1.0.0", 'a', 10),
                10_000_000,
                10,
            )
            .unwrap();
        let disabled = store.disable("example.extension").unwrap();
        assert!(
            disabled.retained_verified_bundles
                && disabled.selected_input_cleared
                && disabled.endpoints_closed
                && disabled.fallback_available
        );
        let killed = store.kill_switch().unwrap();
        assert!(killed.cancelled_generation > disabled.cancelled_generation);
    }

    #[cfg(windows)]
    #[test]
    fn windows_store_root_has_a_protected_current_user_only_acl() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("ecosystem");
        let _store = PackageStore::open(&root).unwrap();
        assert!(private_permissions_are_safe(&root).unwrap());
        assert!(private_permissions_are_safe(&root.join("packages")).unwrap());
        assert!(private_permissions_are_safe(&root.join("staging")).unwrap());
    }

    #[cfg(windows)]
    #[test]
    fn long_local_store_paths_keep_private_acl_and_atomic_recovery() {
        let temporary = tempfile::tempdir().unwrap();
        let long_parent = temporary.path().join("p".repeat(180));
        fs::create_dir(&long_parent).unwrap();
        let root = long_parent.join("ecosystem");
        let mut store = PackageStore::open(&root).unwrap();
        store
            .install_verified(
                &verified("example.extension", "1.0.0", 'a', 10),
                10_000_000,
                10,
            )
            .unwrap();
        drop(store);

        let recovered = PackageStore::open(&root).unwrap();
        assert_eq!(recovered.installed().len(), 1);
        assert!(private_permissions_are_safe(&root).unwrap());
        assert!(
            root.join("packages/example.extension/1.0.0")
                .to_string_lossy()
                .encode_utf16()
                .count()
                > 260
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_acl_paths_reject_relative_and_remote_namespaces() {
        for path in [
            Path::new("relative"),
            Path::new(r"\\server\share\ecosystem"),
        ] {
            assert_eq!(
                windows_local_wide_path(path).unwrap_err().code,
                StoreErrorCode::InvalidRoot
            );
        }
        let mixed = Path::new(r"D:\store").join("staging/not-an-install");
        assert!(!windows_local_wide_path(&mixed)
            .unwrap()
            .contains(&(b'/' as u16)));
    }

    #[cfg(unix)]
    #[test]
    fn linked_store_root_fails_closed_without_touching_target() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let target = temporary.path().join("target");
        fs::create_dir(&target).unwrap();
        let root = temporary.path().join("ecosystem");
        symlink(&target, &root).unwrap();
        assert_eq!(
            PackageStore::open(root).unwrap_err().code,
            StoreErrorCode::LinkRejected
        );
    }
}
