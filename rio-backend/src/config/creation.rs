//! Initial configuration publication; never replaces an existing destination.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateConfigOutcome {
    Created,
    AlreadyExists,
}

#[derive(Debug)]
pub struct CreateConfigError {
    operation: &'static str,
    kind: io::ErrorKind,
}

impl CreateConfigError {
    fn new(operation: &'static str, error: io::Error) -> Self {
        Self {
            operation,
            kind: error.kind(),
        }
    }
}

impl fmt::Display for CreateConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Could not {} configuration: {}",
            self.operation, self.kind
        )
    }
}

impl std::error::Error for CreateConfigError {}

fn existing_destination(path: &Path) -> io::Result<Option<CreateConfigOutcome>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT: reject file reparse points as well as
        // directories and symbolic links, without following their targets.
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    if metadata.file_type().is_file() && !reparse {
        Ok(Some(CreateConfigOutcome::AlreadyExists))
    } else {
        Err(io::ErrorKind::InvalidInput.into())
    }
}

pub fn create_config_file(
    path: Option<PathBuf>,
) -> Result<CreateConfigOutcome, CreateConfigError> {
    create_config_file_with_root(path, super::product::try_config_dir_path)
}

pub(super) fn create_config_file_with_root(
    path: Option<PathBuf>,
    default_root: impl FnOnce() -> Option<PathBuf>,
) -> Result<CreateConfigOutcome, CreateConfigError> {
    let create_default_directory = path.is_none();
    let destination = match path {
        Some(path) => path,
        None => default_root()
            .ok_or_else(|| {
                CreateConfigError::new(
                    "locate the directory for",
                    io::ErrorKind::NotFound.into(),
                )
            })?
            .join("config.toml"),
    };
    if let Some(outcome) = existing_destination(&destination)
        .map_err(|error| CreateConfigError::new("inspect the destination for", error))?
    {
        return Ok(outcome);
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if create_default_directory {
        fs::create_dir_all(parent)
            .map_err(|error| CreateConfigError::new("create the directory for", error))?;
    }
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| CreateConfigError::new("stage", error))?;
    writeln!(staged, "{}", super::config_file_content())
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|error| CreateConfigError::new("write", error))?;

    // Same-directory staging avoids cross-filesystem publication. No-overwrite
    // remains authoritative even when another creator wins after inspection.
    match staged.persist_noclobber(&destination) {
        Ok(_) => Ok(CreateConfigOutcome::Created),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
            existing_destination(&destination)
                .and_then(|outcome| outcome.ok_or(io::ErrorKind::NotFound.into()))
                .map_err(|error| {
                    CreateConfigError::new("inspect the destination for", error)
                })
        }
        Err(error) => Err(CreateConfigError::new("publish", error.error)),
    }
}
