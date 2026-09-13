//! App-owned private transient kubeconfig lifecycle for M11.
//!
//! Official provider CLIs may emit a kubeconfig only into an exact path owned
//! by this manager. The file and path stay outside renderer, persistence,
//! logging, clipboard, and provider-capsule models. Publication is bounded and
//! parsed before use; cleanup is session/generation scoped and fail closed.

use std::{
    collections::VecDeque,
    fmt, fs,
    io::Write as _,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};

use automexia_connectivity::connections::{OpaqueReference, ProviderKind};
use automexia_devops_kubernetes::{
    parse_private_transient_source, KubeconfigSourceSnapshot, MAX_KUBECONFIG_BYTES,
};

use crate::automexia::private_fs;

pub const MAX_PROVIDER_TRANSIENTS: usize = 16;
pub const MAX_PROVIDER_TRANSIENT_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const ROOT_PREFIX: &str = "provider-transients-";
const FILE_PREFIX: &str = "kubeconfig-";
const MAX_RESERVE_ATTEMPTS: usize = 64;
const MAX_SWEEP_ROOTS: usize = 64;
const MAX_SWEEP_FILES: usize = 64;

static MANAGER_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderTransientErrorCode {
    InvalidBinding,
    InvalidSource,
    CapacityExceeded,
    StaleGeneration,
    CrossSession,
    Expired,
    SourceChanged,
    PrivateStorageUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderTransientError {
    code: ProviderTransientErrorCode,
}

impl ProviderTransientError {
    const fn new(code: ProviderTransientErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> ProviderTransientErrorCode {
        self.code
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderTransientBinding {
    pub capsule_id: String,
    pub session_id: u64,
    pub capsule_revision: u64,
    pub generation: u64,
    pub provider: ProviderKind,
    pub source_reference: OpaqueReference,
    pub grant_reference: OpaqueReference,
    pub expires_at_ms: u64,
}

#[derive(Clone)]
pub struct ProviderTransientHandle {
    id: String,
    snapshot: KubeconfigSourceSnapshot,
}

impl ProviderTransientHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn public_snapshot(&self) -> &KubeconfigSourceSnapshot {
        &self.snapshot
    }
}

impl fmt::Debug for ProviderTransientHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderTransientHandle")
            .field("id", &self.id)
            .field("path", &"<redacted>")
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

struct ProviderTransientRecord {
    binding: ProviderTransientBinding,
    path: PathBuf,
    handle: ProviderTransientHandle,
    retired: bool,
}

pub struct ProviderTransientManager {
    root: PathBuf,
    records: VecDeque<ProviderTransientRecord>,
}

impl fmt::Debug for ProviderTransientManager {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderTransientManager")
            .field("root", &"<redacted>")
            .field("active_count", &self.records.len())
            .finish()
    }
}

impl ProviderTransientManager {
    pub fn open(
        connections_root: impl AsRef<Path>,
    ) -> Result<Self, ProviderTransientError> {
        let connections_root = connections_root.as_ref();
        if !connections_root.is_absolute() {
            return Err(storage_error());
        }
        private_fs::ensure_private_child_directory(connections_root)
            .map_err(|_| storage_error())?;
        private_fs::validate_private_child_directory(connections_root)
            .map_err(|_| storage_error())?;
        sweep_stale_roots(connections_root);
        let root = reserve_manager_root_with(connections_root, || {
            MANAGER_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        })?;
        Ok(Self {
            root,
            records: VecDeque::with_capacity(MAX_PROVIDER_TRANSIENTS),
        })
    }

    pub fn publish(
        &mut self,
        binding: ProviderTransientBinding,
        bytes: &[u8],
        observed_at_ms: u64,
    ) -> Result<ProviderTransientHandle, ProviderTransientError> {
        validate_binding(&binding, observed_at_ms)?;
        if bytes.len() > MAX_KUBECONFIG_BYTES {
            return Err(ProviderTransientError::new(
                ProviderTransientErrorCode::InvalidSource,
            ));
        }
        self.cleanup_expired(observed_at_ms);
        if self.records.len() >= MAX_PROVIDER_TRANSIENTS {
            return Err(ProviderTransientError::new(
                ProviderTransientErrorCode::CapacityExceeded,
            ));
        }
        if self.records.iter().any(|record| {
            record.binding.capsule_id == binding.capsule_id
                && record.binding.provider == binding.provider
                && record.binding.generation >= binding.generation
        }) {
            return Err(ProviderTransientError::new(
                ProviderTransientErrorCode::StaleGeneration,
            ));
        }
        let snapshot = parse_private_transient_source(
            binding.source_reference.clone(),
            binding.grant_reference.clone(),
            binding.provider,
            bytes,
            observed_at_ms,
        )
        .map_err(|_| {
            ProviderTransientError::new(ProviderTransientErrorCode::InvalidSource)
        })?;
        let (id, path) = self.reserve_path()?;
        let mut file =
            private_fs::create_private_file(&path).map_err(|_| storage_error())?;
        if file
            .write_all(bytes)
            .and_then(|()| file.sync_data())
            .is_err()
        {
            drop(file);
            let _ = remove_owned_file(&self.root, &path);
            return Err(storage_error());
        }
        drop(file);
        if private_fs::inspect_private_file(&path).is_err() {
            let _ = remove_owned_file(&self.root, &path);
            return Err(storage_error());
        }
        let handle = ProviderTransientHandle { id, snapshot };
        self.records.push_back(ProviderTransientRecord {
            binding,
            path,
            handle: handle.clone(),
            retired: false,
        });
        Ok(handle)
    }

    pub fn revalidate(
        &self,
        handle: &ProviderTransientHandle,
        capsule_id: &str,
        session_id: u64,
        capsule_revision: u64,
        generation: u64,
        now_ms: u64,
    ) -> Result<(), ProviderTransientError> {
        let record = self
            .records
            .iter()
            .find(|record| record.handle.id == handle.id && !record.retired)
            .ok_or_else(|| {
                ProviderTransientError::new(ProviderTransientErrorCode::SourceChanged)
            })?;
        validate_owner(record, capsule_id, session_id, capsule_revision, generation)?;
        if now_ms >= record.binding.expires_at_ms {
            return Err(ProviderTransientError::new(
                ProviderTransientErrorCode::Expired,
            ));
        }
        let exact_path = self.exact_path(
            handle,
            capsule_id,
            session_id,
            capsule_revision,
            generation,
        )?;
        let bytes = private_fs::read_bounded_regular(exact_path, MAX_KUBECONFIG_BYTES)
            .map_err(|_| {
                ProviderTransientError::new(ProviderTransientErrorCode::SourceChanged)
            })?
            .ok_or_else(|| {
                ProviderTransientError::new(ProviderTransientErrorCode::SourceChanged)
            })?;
        let snapshot = parse_private_transient_source(
            record.binding.source_reference.clone(),
            record.binding.grant_reference.clone(),
            record.binding.provider,
            &bytes,
            now_ms,
        )
        .map_err(|_| {
            ProviderTransientError::new(ProviderTransientErrorCode::SourceChanged)
        })?;
        if snapshot.source_record().source_revision()
            != record.handle.snapshot.source_record().source_revision()
        {
            return Err(ProviderTransientError::new(
                ProviderTransientErrorCode::SourceChanged,
            ));
        }
        Ok(())
    }

    pub(crate) fn exact_path(
        &self,
        handle: &ProviderTransientHandle,
        capsule_id: &str,
        session_id: u64,
        capsule_revision: u64,
        generation: u64,
    ) -> Result<&Path, ProviderTransientError> {
        let record = self
            .records
            .iter()
            .find(|record| record.handle.id == handle.id && !record.retired)
            .ok_or_else(|| {
                ProviderTransientError::new(ProviderTransientErrorCode::SourceChanged)
            })?;
        validate_owner(record, capsule_id, session_id, capsule_revision, generation)?;
        Ok(&record.path)
    }

    pub fn revoke(
        &mut self,
        handle_id: &str,
        capsule_id: &str,
        session_id: u64,
        capsule_revision: u64,
        generation: u64,
    ) -> Result<bool, ProviderTransientError> {
        let Some(index) = self
            .records
            .iter()
            .position(|record| record.handle.id == handle_id)
        else {
            return Ok(false);
        };
        validate_owner(
            &self.records[index],
            capsule_id,
            session_id,
            capsule_revision,
            generation,
        )?;
        // Revoke access immediately, but retain deletion ownership until the
        // native filesystem accepts cleanup. A retry must not revive a handle.
        self.records[index].retired = true;
        remove_owned_file(&self.root, &self.records[index].path)
            .map_err(|_| storage_error())?;
        self.records.remove(index);
        Ok(true)
    }

    pub fn cleanup_expired(&mut self, now_ms: u64) -> usize {
        let mut removed = 0;
        let mut retained = VecDeque::with_capacity(self.records.len());
        while let Some(mut record) = self.records.pop_front() {
            if record.retired || now_ms >= record.binding.expires_at_ms {
                record.retired = true;
                if remove_owned_file(&self.root, &record.path).is_ok() {
                    removed += 1;
                } else {
                    retained.push_back(record);
                }
            } else {
                retained.push_back(record);
            }
        }
        self.records = retained;
        removed
    }

    pub fn disable_provider(&mut self, provider: ProviderKind) -> usize {
        self.remove_matching(|record| record.binding.provider == provider)
    }

    pub fn revoke_session(&mut self, capsule_id: &str, session_id: u64) -> usize {
        self.remove_matching(|record| {
            record.binding.capsule_id == capsule_id
                && record.binding.session_id == session_id
        })
    }

    pub fn shutdown(&mut self) -> usize {
        let removed = self.remove_matching(|_| true);
        let _ = fs::remove_dir(&self.root);
        removed
    }

    fn remove_matching(
        &mut self,
        predicate: impl Fn(&ProviderTransientRecord) -> bool,
    ) -> usize {
        let mut removed = 0;
        let mut retained = VecDeque::with_capacity(self.records.len());
        while let Some(mut record) = self.records.pop_front() {
            if predicate(&record) {
                record.retired = true;
                if remove_owned_file(&self.root, &record.path).is_ok() {
                    removed += 1;
                } else {
                    retained.push_back(record);
                }
            } else {
                retained.push_back(record);
            }
        }
        self.records = retained;
        removed
    }

    fn reserve_path(&self) -> Result<(String, PathBuf), ProviderTransientError> {
        for _ in 0..MAX_RESERVE_ATTEMPTS {
            let sequence = FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let id = format!("transient-{sequence:016x}");
            let path = self.root.join(format!("{FILE_PREFIX}{sequence:016x}.yaml"));
            if !path.exists() {
                return Ok((id, path));
            }
        }
        Err(storage_error())
    }
}

fn reserve_manager_root_with(
    connections_root: &Path,
    mut next_sequence: impl FnMut() -> u64,
) -> Result<PathBuf, ProviderTransientError> {
    for _ in 0..MAX_RESERVE_ATTEMPTS {
        let sequence = next_sequence();
        let root = connections_root.join(format!(
            "{ROOT_PREFIX}{}-{sequence:016x}",
            std::process::id()
        ));
        match fs::create_dir(&root) {
            Ok(()) => {
                if private_fs::ensure_private_child_directory(&root).is_err()
                    || private_fs::validate_private_child_directory(&root).is_err()
                {
                    let _ = fs::remove_dir(&root);
                    return Err(storage_error());
                }
                return Ok(root);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(storage_error()),
        }
    }
    Err(storage_error())
}

impl Drop for ProviderTransientManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn storage_error() -> ProviderTransientError {
    ProviderTransientError::new(ProviderTransientErrorCode::PrivateStorageUnavailable)
}

fn validate_binding(
    binding: &ProviderTransientBinding,
    observed_at_ms: u64,
) -> Result<(), ProviderTransientError> {
    if binding.capsule_id.is_empty()
        || binding.capsule_id.len() > 128
        || !binding.capsule_id.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-')
        })
        || binding.session_id == 0
        || binding.capsule_revision == 0
        || binding.generation == 0
        || binding.expires_at_ms <= observed_at_ms
        || !matches!(
            binding.provider,
            ProviderKind::Aws
                | ProviderKind::Azure
                | ProviderKind::Gcp
                | ProviderKind::Kubernetes
                | ProviderKind::OpenShift
                | ProviderKind::Teleport
        )
    {
        return Err(ProviderTransientError::new(
            ProviderTransientErrorCode::InvalidBinding,
        ));
    }
    Ok(())
}

fn validate_owner(
    record: &ProviderTransientRecord,
    capsule_id: &str,
    session_id: u64,
    capsule_revision: u64,
    generation: u64,
) -> Result<(), ProviderTransientError> {
    if record.binding.capsule_id != capsule_id || record.binding.session_id != session_id
    {
        return Err(ProviderTransientError::new(
            ProviderTransientErrorCode::CrossSession,
        ));
    }
    if record.binding.capsule_revision != capsule_revision
        || record.binding.generation != generation
    {
        return Err(ProviderTransientError::new(
            ProviderTransientErrorCode::StaleGeneration,
        ));
    }
    Ok(())
}

fn remove_owned_file(root: &Path, path: &Path) -> std::io::Result<()> {
    if path.parent() != Some(root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "provider transient escaped its owning root",
        ));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !name.starts_with(FILE_PREFIX) || !name.ends_with(".yaml") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "provider transient name is not owned",
        ));
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            fs::remove_file(path)
        }
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "provider transient is not a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn sweep_stale_roots(connections_root: &Path) {
    let Ok(entries) = fs::read_dir(connections_root) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.take(MAX_SWEEP_ROOTS).flatten() {
        let root = entry.path();
        if !entry.file_name().to_string_lossy().starts_with(ROOT_PREFIX) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&root) else {
            continue;
        };
        let expired = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= MAX_PROVIDER_TRANSIENT_AGE);
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || !expired
            || private_fs::validate_private_child_directory(&root).is_err()
        {
            continue;
        }
        let Ok(files) = fs::read_dir(&root) else {
            continue;
        };
        for file in files.take(MAX_SWEEP_FILES).flatten() {
            let _ = remove_owned_file(&root, &file.path());
        }
        let _ = fs::remove_dir(&root);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const KUBECONFIG: &[u8] = br#"apiVersion: v1
kind: Config
clusters:
- name: cluster-one
  cluster:
    server: https://cluster.example.invalid
    certificate-authority-data: ignored-private-material
contexts:
- name: context-one
  context:
    cluster: cluster-one
    user: user-one
    namespace: default
users:
- name: user-one
  user:
    token: ignored-private-token
current-context: context-one
"#;

    fn binding(generation: u64, expiry: u64) -> ProviderTransientBinding {
        ProviderTransientBinding {
            capsule_id: "capsule-one".into(),
            session_id: 7,
            capsule_revision: 3,
            generation,
            provider: ProviderKind::Aws,
            source_reference: OpaqueReference::new("source-one"),
            grant_reference: OpaqueReference::new("grant-one"),
            expires_at_ms: expiry,
        }
    }

    fn manager() -> (tempfile::TempDir, ProviderTransientManager) {
        let temporary = tempfile::tempdir().unwrap();
        let manager =
            ProviderTransientManager::open(temporary.path().join("connections")).unwrap();
        (temporary, manager)
    }

    #[test]
    fn publish_revalidate_and_revoke_keep_path_and_secrets_private() {
        let (_temporary, mut manager) = manager();
        let handle = manager.publish(binding(1, 500), KUBECONFIG, 100).unwrap();
        manager
            .revalidate(&handle, "capsule-one", 7, 3, 1, 200)
            .unwrap();
        let path = manager
            .exact_path(&handle, "capsule-one", 7, 3, 1)
            .unwrap()
            .to_path_buf();
        assert!(path.exists());
        let debug = format!("{handle:?} {manager:?}");
        assert!(!debug.contains(path.to_string_lossy().as_ref()));
        assert!(!debug.contains("ignored-private-token"));
        let tampered = String::from_utf8_lossy(KUBECONFIG).replace("default", "changed");
        fs::write(&path, tampered).unwrap();
        assert_eq!(
            manager
                .revalidate(&handle, "capsule-one", 7, 3, 1, 200)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::SourceChanged
        );
        assert!(manager.revoke(handle.id(), "capsule-one", 7, 3, 1).unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn stale_cross_session_invalid_oversized_and_expired_fail_closed() {
        let (_temporary, mut manager) = manager();
        assert_eq!(
            manager
                .publish(binding(1, 500), b"not kubeconfig", 100)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::InvalidSource
        );
        let oversized = vec![b'x'; MAX_KUBECONFIG_BYTES + 1];
        assert_eq!(
            manager
                .publish(binding(1, 500), &oversized, 100)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::InvalidSource
        );
        let handle = manager.publish(binding(2, 250), KUBECONFIG, 100).unwrap();
        assert_eq!(
            manager
                .revalidate(&handle, "capsule-one", 7, 3, 1, 200)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::StaleGeneration
        );
        assert_eq!(
            manager
                .revalidate(&handle, "capsule-one", 8, 3, 2, 200)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::CrossSession
        );
        assert_eq!(
            manager
                .revalidate(&handle, "capsule-one", 7, 4, 2, 200)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::StaleGeneration
        );
        assert_eq!(
            manager
                .revalidate(&handle, "capsule-one", 7, 3, 2, 250)
                .unwrap_err()
                .code(),
            ProviderTransientErrorCode::Expired
        );
        assert_eq!(manager.cleanup_expired(250), 1);
    }

    #[test]
    fn every_m8_m12_kubeconfig_relation_uses_the_same_private_lifecycle() {
        let (_temporary, mut manager) = manager();
        for (index, provider) in [
            ProviderKind::Aws,
            ProviderKind::Azure,
            ProviderKind::Gcp,
            ProviderKind::Kubernetes,
            ProviderKind::OpenShift,
            ProviderKind::Teleport,
        ]
        .into_iter()
        .enumerate()
        {
            let mut item = binding(index as u64 + 1, 500);
            item.capsule_id = format!("capsule-relation-{index}");
            item.session_id = index as u64 + 1;
            item.provider = provider;
            let handle = manager.publish(item, KUBECONFIG, 100).unwrap();
            assert!(!handle.public_snapshot().contexts().is_empty());
        }
        for provider in [
            ProviderKind::Aws,
            ProviderKind::Azure,
            ProviderKind::Gcp,
            ProviderKind::Kubernetes,
            ProviderKind::OpenShift,
            ProviderKind::Teleport,
        ] {
            assert_eq!(manager.disable_provider(provider), 1);
        }
    }
    #[test]
    #[cfg(windows)]
    fn failed_revoke_retains_file_ownership_until_cleanup_can_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        for _ in 0..4 {
            let (_temporary, mut manager) = manager();
            let handle = manager
                .publish(binding(1, 10_000), KUBECONFIG, 100)
                .unwrap();
            let path = manager
                .exact_path(&handle, "capsule-one", 7, 3, 1)
                .unwrap()
                .to_owned();
            // A real native sharing violation must not release the bookkeeping
            // slot while private material still exists on disk.
            let locked = fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&path)
                .unwrap();
            assert_eq!(
                manager
                    .revoke(handle.id(), "capsule-one", 7, 3, 1)
                    .unwrap_err()
                    .code(),
                ProviderTransientErrorCode::PrivateStorageUnavailable
            );
            assert_eq!(
                manager.records.len(),
                1,
                "failed deletion retains cleanup ownership"
            );
            assert!(path.exists());
            assert_eq!(manager.shutdown(), 0);
            assert_eq!(manager.records.len(), 1);
            drop(locked);
            assert!(manager.revoke(handle.id(), "capsule-one", 7, 3, 1).unwrap());
            assert!(!path.exists());
            assert!(manager.records.is_empty());
            manager.shutdown();
            assert!(!manager.root.exists());
        }
    }

    #[test]
    #[cfg(windows)]
    fn failed_cleanup_revokes_access_before_retry_for_every_retirement_path() {
        use std::os::windows::fs::OpenOptionsExt;
        let mut observations = Vec::new();
        for operation in ["revoke", "disable", "session", "expiry", "shutdown"] {
            let (_temporary, mut manager) = manager();
            let handle = manager
                .publish(binding(1, 10_000), KUBECONFIG, 100)
                .unwrap();
            let path = manager
                .exact_path(&handle, "capsule-one", 7, 3, 1)
                .unwrap()
                .to_owned();
            let locked = fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&path)
                .unwrap();
            assert_eq!(
                manager
                    .revoke(handle.id(), "sibling", 7, 3, 1)
                    .unwrap_err()
                    .code(),
                ProviderTransientErrorCode::CrossSession
            );
            assert_eq!(
                manager
                    .revoke(handle.id(), "capsule-one", 7, 3, 2)
                    .unwrap_err()
                    .code(),
                ProviderTransientErrorCode::StaleGeneration
            );
            assert!(
                manager.exact_path(&handle, "capsule-one", 7, 3, 1).is_ok(),
                "invalid callers cannot retire another owner"
            );
            match operation {
                "revoke" => {
                    assert!(manager.revoke(handle.id(), "capsule-one", 7, 3, 1).is_err())
                }
                "disable" => assert_eq!(manager.disable_provider(ProviderKind::Aws), 0),
                "session" => assert_eq!(manager.revoke_session("capsule-one", 7), 0),
                "expiry" => assert_eq!(manager.cleanup_expired(10_000), 0),
                "shutdown" => assert_eq!(manager.shutdown(), 0),
                _ => unreachable!(),
            }
            let denied_path =
                manager.exact_path(&handle, "capsule-one", 7, 3, 1).is_err();
            drop(locked);
            assert!(
                path.is_file(),
                "access revocation and physical deletion are independent"
            );
            // Restoring filesystem access or observing an earlier clock must
            // never reactivate the handle retained solely for cleanup.
            let denied_source = manager
                .revalidate(&handle, "capsule-one", 7, 3, 1, 100)
                .is_err_and(|error| {
                    error.code() == ProviderTransientErrorCode::SourceChanged
                });
            let cleaned = manager.cleanup_expired(100);
            observations.push((operation, denied_path, denied_source, cleaned));
            manager.shutdown();
            assert!(!path.exists());
            assert!(!manager.root.exists());
        }
        for (operation, denied_path, denied_source, cleaned) in observations {
            assert!(
                denied_path && denied_source,
                "{operation} retained an active handle after cleanup failure"
            );
            assert_eq!(
                cleaned, 1,
                "{operation} pending cleanup must retry before its original expiry"
            );
        }
    }

    #[test]
    fn capacity_disable_and_repeated_shutdown_are_bounded() {
        // A native run stalled in this lifecycle with no operation evidence.
        // Captured numeric checkpoints identify the next blocked filesystem
        // phase without exposing transient contents, paths or host identities.
        for cycle in 0..64_u64 {
            eprintln!("provider-transient lifecycle cycle={cycle} phase=open");
            let (_temporary, mut manager) = manager();
            let owned_root = manager.root.clone();
            for index in 0..MAX_PROVIDER_TRANSIENTS {
                let mut item = binding(index as u64 + 1, 10_000);
                item.capsule_id = format!("capsule-{cycle}-{index}");
                item.session_id = index as u64 + 1;
                eprintln!("provider-transient lifecycle cycle={cycle} phase=publish item={index}");
                manager.publish(item, KUBECONFIG, 100).unwrap();
            }
            assert_eq!(
                fs::read_dir(&owned_root).unwrap().count(),
                MAX_PROVIDER_TRANSIENTS,
                "every live transient must have exactly one owned file"
            );
            let mut extra = binding(99, 10_000);
            extra.capsule_id = "capacity-extra".into();
            assert_eq!(
                manager.publish(extra, KUBECONFIG, 100).unwrap_err().code(),
                ProviderTransientErrorCode::CapacityExceeded
            );
            eprintln!("provider-transient lifecycle cycle={cycle} phase=disable");
            assert_eq!(
                manager.disable_provider(ProviderKind::Aws),
                MAX_PROVIDER_TRANSIENTS
            );
            assert_eq!(fs::read_dir(&owned_root).unwrap().count(), 0);
            eprintln!("provider-transient lifecycle cycle={cycle} phase=shutdown");
            assert_eq!(manager.shutdown(), 0);
            assert!(!owned_root.exists(), "shutdown must remove its owned root");
            assert_eq!(manager.shutdown(), 0);
            assert!(
                !owned_root.exists(),
                "repeated shutdown must not recreate state"
            );
            eprintln!("provider-transient lifecycle cycle={cycle} phase=complete");
        }
    }

    #[test]
    fn manager_root_reservation_never_adopts_a_preexisting_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let connections = temporary.path().join("connections");
        private_fs::ensure_private_child_directory(&connections).unwrap();
        let colliding = connections.join(format!(
            "{ROOT_PREFIX}{}-{:016x}",
            std::process::id(),
            41_u64
        ));
        private_fs::ensure_private_child_directory(&colliding).unwrap();
        let sequence = AtomicU64::new(41);
        let reserved = reserve_manager_root_with(&connections, || {
            sequence.fetch_add(1, Ordering::Relaxed)
        })
        .unwrap();
        assert_ne!(reserved, colliding);
        assert!(reserved.ends_with(format!(
            "{ROOT_PREFIX}{}-{:016x}",
            std::process::id(),
            42_u64
        )));
    }

    #[cfg(windows)]
    #[test]
    fn long_local_paths_publish_revalidate_and_revoke_real_provider_transients() {
        let temporary = tempfile::tempdir().unwrap();
        let long_parent = temporary.path().join("p".repeat(180));
        fs::create_dir(&long_parent).unwrap();
        let connections = long_parent.join("connections");
        let mut manager = ProviderTransientManager::open(&connections).unwrap();
        let item = binding(41, 500);
        let handle = manager.publish(item, KUBECONFIG, 100).unwrap();
        let path = manager
            .exact_path(&handle, "capsule-one", 7, 3, 41)
            .unwrap();
        assert!(path.to_string_lossy().encode_utf16().count() > 260);
        manager
            .revalidate(&handle, "capsule-one", 7, 3, 41, 200)
            .unwrap();
        manager
            .revoke(handle.id(), "capsule-one", 7, 3, 41)
            .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_connections_root_is_rejected() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = temporary.path().join("connections");
        symlink(outside.path(), &link).unwrap();
        assert_eq!(
            ProviderTransientManager::open(&link).unwrap_err().code(),
            ProviderTransientErrorCode::PrivateStorageUnavailable
        );
    }
}
