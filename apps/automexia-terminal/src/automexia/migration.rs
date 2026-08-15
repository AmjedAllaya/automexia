//! One-release, non-destructive import of safe Rio configuration state.

use rio_backend::config::product;
pub use rio_backend::config::product::validate_legacy_config;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

const COMPLETE_MARKER: &str = ".migrated-from-rio-v0.4";
const PROGRESS_MARKER: &str = ".migration-in-progress";
const MAX_MARKER_BYTES: u64 = 64 * 1024;
const MAX_CONFIG_BYTES: u64 = product::MAX_CONFIG_FILE_BYTES;
const MAX_THEME_FILE_BYTES: u64 = product::MAX_THEME_FILE_BYTES;
const MAX_THEME_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_THEME_FILES: usize = 256;
const MAX_THEME_ENTRIES: usize = 2_048;
const MAX_THEME_DEPTH: usize = 8;
const MAX_EXTENSION_DIRECTORIES: usize = 256;
const MAX_EXTENSION_ENTRIES: usize = 1_024;
const MAX_EXTENSION_TOTAL_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    Migrated,
    AlreadyMigrated,
    NoLegacyConfiguration,
    DestinationHasConfiguration,
    SameDirectory,
}

pub fn migrate_legacy_configuration() -> Result<MigrationStatus, String> {
    migrate_paths(&product::legacy_config_dir(), &product::config_dir_path())
}

fn migrate_paths(source: &Path, destination: &Path) -> Result<MigrationStatus, String> {
    if source == destination {
        return Ok(MigrationStatus::SameDirectory);
    }
    if destination.join(COMPLETE_MARKER).is_file() {
        return Ok(MigrationStatus::AlreadyMigrated);
    }

    let source_config = source.join("config.toml");
    match fs::symlink_metadata(&source_config) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(MigrationStatus::NoLegacyConfiguration);
        }
        Err(error) => {
            return Err(format!(
                "could not inspect {}: {error}",
                source_config.display()
            ));
        }
        Ok(metadata)
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() =>
        {
            return Err(format!(
                "legacy configuration must be a regular non-symlink file: {}",
                source_config.display()
            ));
        }
        Ok(_) => {}
    }

    let progress = destination.join(PROGRESS_MARKER);
    let continuing = match fs::symlink_metadata(&progress) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!("could not inspect {}: {error}", progress.display()));
        }
        Ok(metadata)
            if metadata.file_type().is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= MAX_MARKER_BYTES =>
        {
            true
        }
        Ok(_) => {
            return Err(format!(
                "migration progress marker is not a bounded regular file: {}",
                progress.display()
            ));
        }
    };
    if continuing {
        let recorded_source = read_bounded_regular_file(
            &progress,
            MAX_MARKER_BYTES,
            "migration progress marker",
        )?;
        let recorded_source = std::str::from_utf8(&recorded_source).map_err(|error| {
            format!("{} is not valid UTF-8: {error}", progress.display())
        })?;
        if recorded_source != source.to_string_lossy() {
            return Err(format!(
                "{} belongs to a different migration source; review the partial destination before retrying",
                progress.display()
            ));
        }
    }
    if destination.join("config.toml").exists() && !continuing {
        return Ok(MigrationStatus::DestinationHasConfiguration);
    }

    let config_bytes = read_bounded_regular_file(
        &source_config,
        MAX_CONFIG_BYTES,
        "legacy configuration",
    )?;
    let config_text = std::str::from_utf8(&config_bytes).map_err(|error| {
        format!("{} is not valid UTF-8: {error}", source_config.display())
    })?;
    validate_legacy_config(config_text)?;

    fs::create_dir_all(destination).map_err(|error| {
        format!("could not create {}: {error}", destination.display())
    })?;
    atomic_write(&progress, source.to_string_lossy().as_bytes())?;
    atomic_write_if_missing(&destination.join("config.toml"), &config_bytes)?;
    copy_theme_tree(&source.join("themes"), &destination.join("themes"))?;
    copy_extension_state(
        &source.join("automexia").join("extensions"),
        &destination.join("extensions"),
    )?;
    atomic_write(
        &destination.join(COMPLETE_MARKER),
        format!(
            "source={}\nversion={}\n",
            source.display(),
            env!("CARGO_PKG_VERSION")
        )
        .as_bytes(),
    )?;
    fs::remove_file(&progress)
        .map_err(|error| format!("could not clear {}: {error}", progress.display()))?;
    Ok(MigrationStatus::Migrated)
}

#[derive(Default)]
struct ThemeCopyBudget {
    entries: usize,
    files: usize,
    bytes: u64,
}

fn copy_theme_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let mut budget = ThemeCopyBudget::default();
    copy_theme_directory(source, destination, 0, &mut budget)
}

fn copy_theme_directory(
    source: &Path,
    destination: &Path,
    depth: usize,
    budget: &mut ThemeCopyBudget,
) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!("could not inspect {}: {error}", source.display()));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Ok(());
    }
    if depth > MAX_THEME_DEPTH {
        return Err(format!(
            "legacy theme tree exceeds the maximum depth of {MAX_THEME_DEPTH}: {}",
            source.display()
        ));
    }

    for entry in fs::read_dir(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?
    {
        budget.entries = budget.entries.saturating_add(1);
        if budget.entries > MAX_THEME_ENTRIES {
            return Err(format!(
                "legacy theme tree exceeds the maximum of {MAX_THEME_ENTRIES} entries"
            ));
        }
        let entry = entry.map_err(|error| format!("theme entry failed: {error}"))?;
        let file_type = entry.file_type().map_err(|error| {
            format!("could not inspect {}: {error}", entry.path().display())
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_theme_directory(&entry.path(), &target, depth + 1, budget)?;
        } else if file_type.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("toml")
        {
            if budget.files >= MAX_THEME_FILES {
                return Err(format!(
                    "legacy theme tree exceeds the maximum of {MAX_THEME_FILES} TOML files"
                ));
            }
            let bytes = read_bounded_regular_file(
                &entry.path(),
                MAX_THEME_FILE_BYTES,
                "legacy theme",
            )?;
            let next_total = budget
                .bytes
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| "legacy theme byte accounting overflowed".to_string())?;
            if next_total > MAX_THEME_TOTAL_BYTES {
                return Err(format!(
                    "legacy themes exceed the maximum aggregate size of {MAX_THEME_TOTAL_BYTES} bytes"
                ));
            }
            budget.files += 1;
            budget.bytes = next_total;
            atomic_write_if_missing(&target, &bytes)?;
        }
    }
    Ok(())
}

fn copy_extension_state(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!("could not inspect {}: {error}", source.display()));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Ok(());
    }

    let mut entries = 0usize;
    let mut extension_directories = 0usize;
    let mut copied_bytes = 0u64;
    for extension in fs::read_dir(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?
    {
        entries = entries.saturating_add(1);
        if entries > MAX_EXTENSION_ENTRIES {
            return Err(format!(
                "legacy extension state exceeds the maximum of {MAX_EXTENSION_ENTRIES} entries"
            ));
        }
        let extension =
            extension.map_err(|error| format!("extension entry failed: {error}"))?;
        if !extension
            .file_type()
            .map_err(|error| format!("could not inspect extension: {error}"))?
            .is_dir()
        {
            continue;
        }
        extension_directories = extension_directories.saturating_add(1);
        if extension_directories > MAX_EXTENSION_DIRECTORIES {
            return Err(format!(
                "legacy extension state exceeds the maximum of {MAX_EXTENSION_DIRECTORIES} directories"
            ));
        }
        for marker in ["installed", "disabled"] {
            let source_marker = extension.path().join(marker);
            let metadata = match fs::symlink_metadata(&source_marker) {
                Ok(metadata)
                    if metadata.file_type().is_file()
                        && !metadata.file_type().is_symlink()
                        && metadata.len() <= MAX_MARKER_BYTES =>
                {
                    metadata
                }
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "could not inspect {}: {error}",
                        source_marker.display()
                    ));
                }
            };
            let bytes = read_bounded_regular_file(
                &source_marker,
                MAX_MARKER_BYTES,
                "legacy extension marker",
            )?;
            debug_assert_eq!(metadata.len(), bytes.len() as u64);
            let next_total =
                copied_bytes
                    .checked_add(bytes.len() as u64)
                    .ok_or_else(|| {
                        "legacy extension byte accounting overflowed".to_string()
                    })?;
            if next_total > MAX_EXTENSION_TOTAL_BYTES {
                return Err(format!(
                    "legacy extension markers exceed the maximum aggregate size of {MAX_EXTENSION_TOTAL_BYTES} bytes"
                ));
            }
            copied_bytes = next_total;
            atomic_write_if_missing(
                &destination.join(extension.file_name()).join(marker),
                &bytes,
            )?;
        }
    }
    Ok(())
}

fn read_bounded_regular_file(
    source: &Path,
    max_bytes: u64,
    purpose: &str,
) -> Result<Vec<u8>, String> {
    let before = fs::symlink_metadata(source)
        .map_err(|error| format!("could not inspect {}: {error}", source.display()))?;
    if before.file_type().is_symlink() || !before.file_type().is_file() {
        return Err(format!(
            "{purpose} must be a regular non-symlink file: {}",
            source.display()
        ));
    }
    if before.len() > max_bytes {
        return Err(format!(
            "{purpose} exceeds the maximum size of {max_bytes} bytes: {}",
            source.display()
        ));
    }

    let mut file = fs::File::open(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&mut file)
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "{purpose} exceeds the maximum size of {max_bytes} bytes: {}",
            source.display()
        ));
    }
    let after = file
        .metadata()
        .map_err(|error| format!("could not recheck {}: {error}", source.display()))?;
    if after.len() != before.len() || after.len() != bytes.len() as u64 {
        return Err(format!("{purpose} changed while it was being read"));
    }
    Ok(bytes)
}
fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid migration path: {}", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("could not stage {}: {error}", destination.display()))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("could not stage {}: {error}", destination.display()))?;
    temporary.persist(destination).map(|_| ()).map_err(|error| {
        format!(
            "could not install {}: {}",
            destination.display(),
            error.error
        )
    })
}

fn atomic_write_if_missing(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid migration path: {}", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("could not stage {}: {error}", destination.display()))?;
    temporary
        .write_all(bytes)
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("could not stage {}: {error}", destination.display()))?;
    match temporary.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!(
            "could not install {}: {}",
            destination.display(),
            error.error
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct TestDirs {
        root: PathBuf,
        source: PathBuf,
        destination: PathBuf,
    }

    impl TestDirs {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "automexia-migration-{name}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let source = root.join("rio");
            let destination = root.join("automexia");
            fs::create_dir_all(&source).unwrap();
            Self {
                root,
                source,
                destination,
            }
        }

        fn valid_config(&self) {
            fs::write(self.source.join("config.toml"), "theme = \"\"\n").unwrap();
        }
    }

    impl Drop for TestDirs {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn missing_legacy_configuration_is_a_noop() {
        let dirs = TestDirs::new("missing");
        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::NoLegacyConfiguration
        );
        assert!(!dirs.destination.exists());
    }

    #[test]
    fn migrates_only_config_themes_and_activation_markers() {
        let dirs = TestDirs::new("safe-content");
        dirs.valid_config();
        fs::create_dir_all(dirs.source.join("themes")).unwrap();
        fs::write(dirs.source.join("themes/custom.toml"), "[colors]\n").unwrap();
        fs::write(dirs.source.join("themes/unsafe.exe"), b"no").unwrap();
        fs::create_dir_all(dirs.source.join("automexia/extensions/devops")).unwrap();
        fs::write(
            dirs.source.join("automexia/extensions/devops/installed"),
            "enabled=true\n",
        )
        .unwrap();
        fs::write(
            dirs.source.join("automexia/extensions/devops/payload.exe"),
            b"no",
        )
        .unwrap();
        fs::create_dir_all(dirs.source.join("logs")).unwrap();
        fs::write(dirs.source.join("logs/rio.log"), b"private").unwrap();

        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::Migrated
        );
        assert!(dirs.destination.join("config.toml").is_file());
        assert!(dirs.destination.join("themes/custom.toml").is_file());
        assert!(dirs
            .destination
            .join("extensions/devops/installed")
            .is_file());
        assert!(!dirs.destination.join("themes/unsafe.exe").exists());
        assert!(!dirs
            .destination
            .join("extensions/devops/payload.exe")
            .exists());
        assert!(!dirs.destination.join("logs").exists());
    }

    #[test]
    fn oversized_config_is_rejected_before_any_destination_write() {
        let dirs = TestDirs::new("oversized-config");
        fs::write(
            dirs.source.join("config.toml"),
            vec![b' '; MAX_CONFIG_BYTES as usize + 1],
        )
        .unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("legacy configuration exceeds the maximum size"));
        assert!(!dirs.destination.exists());
    }

    #[test]
    fn non_utf8_config_is_rejected_before_any_destination_write() {
        let dirs = TestDirs::new("non-utf8-config");
        fs::write(dirs.source.join("config.toml"), [0xff, 0xfe, 0xfd]).unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("is not valid UTF-8"));
        assert!(!dirs.destination.exists());
    }

    #[test]
    fn oversized_progress_marker_cannot_force_an_unbounded_resume_read() {
        let dirs = TestDirs::new("oversized-progress");
        dirs.valid_config();
        fs::create_dir_all(&dirs.destination).unwrap();
        fs::write(
            dirs.destination.join(PROGRESS_MARKER),
            vec![b'x'; MAX_MARKER_BYTES as usize + 1],
        )
        .unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("progress marker is not a bounded regular file"));
        assert!(!dirs.destination.join("config.toml").exists());
    }

    #[test]
    fn oversized_theme_is_never_copied_and_keeps_partial_storage_bounded() {
        let dirs = TestDirs::new("oversized-theme");
        dirs.valid_config();
        fs::create_dir_all(dirs.source.join("themes")).unwrap();
        fs::write(
            dirs.source.join("themes/oversized.toml"),
            vec![b' '; MAX_THEME_FILE_BYTES as usize + 1],
        )
        .unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("legacy theme exceeds the maximum size"));
        assert!(!dirs.destination.join("themes/oversized.toml").exists());
        assert!(dirs.destination.join(PROGRESS_MARKER).is_file());
    }

    #[test]
    fn excessive_theme_depth_is_rejected_without_copying_the_deep_file() {
        let dirs = TestDirs::new("theme-depth");
        dirs.valid_config();
        let mut source = dirs.source.join("themes");
        let mut destination = dirs.destination.join("themes");
        for index in 0..=MAX_THEME_DEPTH {
            source = source.join(format!("level-{index}"));
            destination = destination.join(format!("level-{index}"));
        }
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("deep.toml"), "[colors]\n").unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("maximum depth"));
        assert!(!destination.join("deep.toml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_config_is_rejected_without_reading_or_copying_its_target() {
        use std::os::unix::fs::symlink;

        let dirs = TestDirs::new("symlinked-config");
        let target = dirs.root.join("outside.toml");
        fs::write(&target, "theme = \"sensitive\"\n").unwrap();
        symlink(&target, dirs.source.join("config.toml")).unwrap();

        let error = migrate_paths(&dirs.source, &dirs.destination).unwrap_err();
        assert!(error.contains("regular non-symlink file"));
        assert!(!dirs.destination.exists());
    }

    #[test]
    fn oversized_extension_markers_are_ignored_without_destination_growth() {
        let dirs = TestDirs::new("oversized-extension-marker");
        dirs.valid_config();
        let extension = dirs.source.join("automexia/extensions/example");
        fs::create_dir_all(&extension).unwrap();
        fs::write(
            extension.join("installed"),
            vec![b'x'; MAX_MARKER_BYTES as usize + 1],
        )
        .unwrap();

        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::Migrated
        );
        assert!(!dirs
            .destination
            .join("extensions/example/installed")
            .exists());
    }

    #[test]
    fn malformed_config_is_rejected_without_partial_output() {
        let dirs = TestDirs::new("malformed");
        fs::write(dirs.source.join("config.toml"), "not = [valid").unwrap();
        assert!(migrate_paths(&dirs.source, &dirs.destination).is_err());
        assert!(!dirs.destination.join("config.toml").exists());
    }

    #[test]
    fn conflicting_automexia_config_is_preserved() {
        let dirs = TestDirs::new("conflict");
        dirs.valid_config();
        fs::create_dir_all(&dirs.destination).unwrap();
        fs::write(dirs.destination.join("config.toml"), "theme = \"mine\"\n").unwrap();
        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::DestinationHasConfiguration
        );
        assert_eq!(
            fs::read_to_string(dirs.destination.join("config.toml")).unwrap(),
            "theme = \"mine\"\n"
        );
    }

    #[test]
    fn interrupted_migration_resumes_and_repeated_run_is_idempotent() {
        let dirs = TestDirs::new("resume");
        dirs.valid_config();
        fs::create_dir_all(dirs.source.join("themes")).unwrap();
        fs::write(dirs.source.join("themes/resumed.toml"), "[colors]\n").unwrap();
        fs::create_dir_all(&dirs.destination).unwrap();
        fs::write(
            dirs.destination.join(PROGRESS_MARKER),
            dirs.source.to_string_lossy().as_bytes(),
        )
        .unwrap();
        fs::write(dirs.destination.join("config.toml"), "theme = \"\"\n").unwrap();
        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::Migrated
        );
        assert!(dirs.destination.join("themes/resumed.toml").is_file());
        assert!(!dirs.destination.join(PROGRESS_MARKER).exists());
        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::AlreadyMigrated
        );
    }

    #[test]
    fn read_only_legacy_config_can_be_imported() {
        let dirs = TestDirs::new("read-only");
        dirs.valid_config();
        let config = dirs.source.join("config.toml");
        let mut permissions = fs::metadata(&config).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&config, permissions).unwrap();
        assert_eq!(
            migrate_paths(&dirs.source, &dirs.destination).unwrap(),
            MigrationStatus::Migrated
        );
    }

    #[test]
    fn progress_marker_from_another_source_is_rejected() {
        let dirs = TestDirs::new("wrong-source");
        dirs.valid_config();
        fs::create_dir_all(&dirs.destination).unwrap();
        fs::write(
            dirs.destination.join(PROGRESS_MARKER),
            dirs.root.join("another-rio").to_string_lossy().as_bytes(),
        )
        .unwrap();
        fs::write(dirs.destination.join("config.toml"), "theme = \"mine\"\n").unwrap();

        assert!(migrate_paths(&dirs.source, &dirs.destination).is_err());
        assert_eq!(
            fs::read_to_string(dirs.destination.join("config.toml")).unwrap(),
            "theme = \"mine\"\n"
        );
    }
}
