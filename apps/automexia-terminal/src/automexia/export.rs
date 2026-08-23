//! Bounded, current-user-only terminal text export lifecycle.

use crate::automexia::private_fs;
use std::collections::VecDeque;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

pub const MAX_EXPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_EXPORT_FILES: usize = 16;
const MAX_STALE_ROOTS_PER_SWEEP: usize = 64;
const MAX_STALE_FILES_PER_ROOT: usize = 64;
const MAX_EXPORT_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const ROOT_PREFIX: &str = "automexia-screen-exports-";
const FILE_PREFIX: &str = "export-";

static MANAGER_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static FILE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    Plain,
    Escaped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportErrorCode {
    CapacityExceeded,
    InvalidTemporaryRoot,
    PrivatePermissions,
    Io,
}

#[derive(Debug)]
pub struct ExportManager {
    root: PathBuf,
    files: VecDeque<PathBuf>,
}

impl ExportManager {
    pub fn new() -> Self {
        let manager = MANAGER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("{ROOT_PREFIX}{}-{manager}", std::process::id()));
        Self {
            root,
            files: VecDeque::new(),
        }
    }

    pub fn write(
        &mut self,
        text: &str,
        format: ExportFormat,
    ) -> Result<PathBuf, ExportErrorCode> {
        let bytes = normalize_export(text, format)?;
        self.prepare_root()?;
        self.expire_owned_files();

        while self.files.len() >= MAX_EXPORT_FILES {
            if let Some(path) = self.files.pop_front() {
                let _ = remove_owned_file(&self.root, &path);
            }
        }

        for _ in 0..32 {
            let sequence = FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = self.root.join(format!("{FILE_PREFIX}{sequence:016x}.txt"));
            let mut file = match private_fs::create_private_file(&path) {
                Ok(file) => file,
                Err(error)
                    if error.code() == private_fs::PrivateFsErrorCode::Io
                        && path.exists() =>
                {
                    continue;
                }
                Err(error) => return Err(map_private_error(error)),
            };
            file.write_all(&bytes).map_err(|_| ExportErrorCode::Io)?;
            file.sync_data().map_err(|_| ExportErrorCode::Io)?;
            self.files.push_back(path.clone());
            return Ok(path);
        }

        Err(ExportErrorCode::Io)
    }

    fn prepare_root(&self) -> Result<(), ExportErrorCode> {
        sweep_stale_roots(&self.root);
        private_fs::ensure_private_child_directory(&self.root)
            .map_err(map_private_error)?;
        private_fs::validate_private_child_directory(&self.root)
            .map_err(map_private_error)
    }

    fn expire_owned_files(&mut self) {
        let now = SystemTime::now();
        self.files.retain(|path| {
            let expired = fs::symlink_metadata(path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age >= MAX_EXPORT_AGE);
            if expired {
                let _ = remove_owned_file(&self.root, path);
                false
            } else {
                true
            }
        });
    }
}

impl Default for ExportManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExportManager {
    fn drop(&mut self) {
        while let Some(path) = self.files.pop_front() {
            let _ = remove_owned_file(&self.root, &path);
        }
        let _ = fs::remove_dir(&self.root);
    }
}

fn normalize_export(
    text: &str,
    format: ExportFormat,
) -> Result<Vec<u8>, ExportErrorCode> {
    let mut normalized = String::with_capacity(text.len().min(MAX_EXPORT_BYTES));
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        let character = if character == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            '\n'
        } else {
            character
        };
        match format {
            ExportFormat::Plain => {
                if character == '\n'
                    || character == '\t'
                    || (!character.is_control() && character != '\u{7f}')
                {
                    normalized.push(character);
                }
            }
            ExportFormat::Escaped => {
                normalized.extend(character.escape_default());
            }
        }
        if normalized.len() > MAX_EXPORT_BYTES {
            return Err(ExportErrorCode::CapacityExceeded);
        }
    }
    Ok(normalized.into_bytes())
}

fn map_private_error(error: private_fs::PrivateFsError) -> ExportErrorCode {
    match error.code() {
        private_fs::PrivateFsErrorCode::PrivatePermissions => {
            ExportErrorCode::PrivatePermissions
        }
        private_fs::PrivateFsErrorCode::InvalidRoot
        | private_fs::PrivateFsErrorCode::LinkRejected
        | private_fs::PrivateFsErrorCode::NotDirectory
        | private_fs::PrivateFsErrorCode::NotRegularFile
        | private_fs::PrivateFsErrorCode::SourceChanged => {
            ExportErrorCode::InvalidTemporaryRoot
        }
        private_fs::PrivateFsErrorCode::Io
        | private_fs::PrivateFsErrorCode::SourceTooLarge => ExportErrorCode::Io,
    }
}

fn remove_owned_file(root: &Path, path: &Path) -> std::io::Result<()> {
    if path.parent() != Some(root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "export path escaped its owning root",
        ));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !name.starts_with(FILE_PREFIX) || !name.ends_with(".txt") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "unowned export file name",
        ));
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            fs::remove_file(path)
        }
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "export target is not a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn sweep_stale_roots(current: &Path) {
    let Some(parent) = current.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.take(MAX_STALE_ROOTS_PER_SWEEP).flatten() {
        let path = entry.path();
        if path == current {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(ROOT_PREFIX) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        let old_enough = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= MAX_EXPORT_AGE);
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || !old_enough
            || private_fs::validate_private_child_directory(&path).is_err()
        {
            continue;
        }
        let Ok(files) = fs::read_dir(&path) else {
            continue;
        };
        for file in files.take(MAX_STALE_FILES_PER_ROOT).flatten() {
            let _ = remove_owned_file(&path, &file.path());
        }
        let _ = fs::remove_dir(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_bounded_and_removes_plain_control_bytes() {
        assert_eq!(
            normalize_export("a\r\nb\rc\u{1b}[31m", ExportFormat::Plain).unwrap(),
            b"a\nb\nc[31m"
        );
        assert_eq!(
            normalize_export("\u{1b}\n", ExportFormat::Escaped).unwrap(),
            b"\\u{1b}\\n"
        );
        assert!(matches!(
            normalize_export(&"x".repeat(MAX_EXPORT_BYTES + 1), ExportFormat::Plain),
            Err(ExportErrorCode::CapacityExceeded)
        ));
    }

    #[test]
    fn manager_creates_collision_safe_private_files_and_cleans_on_drop() {
        let (root, first, second) = {
            let mut manager = ExportManager::new();
            let first = manager.write("one", ExportFormat::Plain).unwrap();
            let second = manager.write("two", ExportFormat::Plain).unwrap();
            assert_ne!(first, second);
            assert_eq!(fs::read(&first).unwrap(), b"one");
            assert_eq!(fs::read(&second).unwrap(), b"two");
            (manager.root.clone(), first, second)
        };
        assert!(!first.exists());
        assert!(!second.exists());
        assert!(!root.exists());
    }

    #[test]
    fn cleanup_rejects_paths_outside_the_exact_owned_root() {
        let manager = ExportManager::new();
        let outside = std::env::temp_dir().join("not-an-automexia-export.txt");
        assert_eq!(
            remove_owned_file(&manager.root, &outside)
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
}
