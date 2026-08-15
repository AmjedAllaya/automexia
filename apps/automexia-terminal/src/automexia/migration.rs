//! One-release, non-destructive import of safe Rio configuration state.

use rio_backend::config::product;
pub use rio_backend::config::product::validate_legacy_config;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

const COMPLETE_MARKER: &str = ".migrated-from-rio-v0.4";
const PROGRESS_MARKER: &str = ".migration-in-progress";
const MAX_MARKER_BYTES: u64 = 64 * 1024;

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
    if !source_config.is_file() {
        return Ok(MigrationStatus::NoLegacyConfiguration);
    }

    let progress = destination.join(PROGRESS_MARKER);
    let continuing = progress.is_file();
    if continuing {
        let recorded_source = fs::read_to_string(&progress)
            .map_err(|error| format!("could not read {}: {error}", progress.display()))?;
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

    let config_text = fs::read_to_string(&source_config).map_err(|error| {
        format!("could not read {}: {error}", source_config.display())
    })?;
    validate_legacy_config(&config_text)?;

    fs::create_dir_all(destination).map_err(|error| {
        format!("could not create {}: {error}", destination.display())
    })?;
    atomic_write(&progress, source.to_string_lossy().as_bytes())?;
    atomic_copy_if_missing(&source_config, &destination.join("config.toml"))?;
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

fn copy_theme_tree(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("theme entry failed: {error}"))?;
        let file_type = entry.file_type().map_err(|error| {
            format!("could not inspect {}: {error}", entry.path().display())
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_theme_tree(&entry.path(), &target)?;
        } else if file_type.is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some("toml")
        {
            atomic_copy_if_missing(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_extension_state(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Ok(());
    }
    for extension in fs::read_dir(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?
    {
        let extension =
            extension.map_err(|error| format!("extension entry failed: {error}"))?;
        if !extension
            .file_type()
            .map_err(|error| format!("could not inspect extension: {error}"))?
            .is_dir()
        {
            continue;
        }
        for marker in ["installed", "disabled"] {
            let source_marker = extension.path().join(marker);
            let metadata = match fs::symlink_metadata(&source_marker) {
                Ok(metadata)
                    if metadata.file_type().is_file()
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
            let _ = metadata;
            atomic_copy_if_missing(
                &source_marker,
                &destination.join(extension.file_name()).join(marker),
            )?;
        }
    }
    Ok(())
}

fn atomic_copy_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Ok(());
    }
    let bytes = fs::read(source)
        .map_err(|error| format!("could not read {}: {error}", source.display()))?;
    atomic_write_if_missing(destination, &bytes)
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
