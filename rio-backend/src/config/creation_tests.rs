use super::{create_config_file, CreateConfigOutcome};
use std::fs;
use std::sync::{Arc, Barrier};

// Independent expected starter bytes, including the existing final blank line.
const STARTER: &[u8] = b"# See the configuration reference: https://github.com/AmjedAllaya/automexia-terminal/tree/main/docs\n\n";

#[test]
fn starter_publication_preserves_existing_bytes_and_reports_the_outcome() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    assert_eq!(
        create_config_file(Some(path.clone())).unwrap(),
        CreateConfigOutcome::Created
    );
    assert_eq!(fs::read(&path).unwrap(), STARTER);
    fs::write(&path, b"# independent existing sentinel\n").unwrap();
    assert_eq!(
        create_config_file(Some(path.clone())).unwrap(),
        CreateConfigOutcome::AlreadyExists
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        b"# independent existing sentinel\n"
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn concurrent_starter_creators_publish_one_complete_file_without_staging_leaks() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let barrier = Arc::new(Barrier::new(4));
    let outcomes = std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = path.clone();
                scope.spawn(move || {
                    barrier.wait();
                    create_config_file(Some(path))
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == CreateConfigOutcome::Created)
            .count(),
        1
    );
    assert_eq!(fs::read(&path).unwrap(), STARTER);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn starter_creation_reports_invalid_destinations_without_partial_files() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing").join("config.toml");
    let error = create_config_file(Some(missing.clone())).unwrap_err();
    assert!(!error.to_string().contains("missing"));
    assert!(!missing.exists());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);

    let occupied = directory.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("sentinel"), b"preserve").unwrap();
    assert!(create_config_file(Some(occupied.clone())).is_err());
    assert_eq!(fs::read(occupied.join("sentinel")).unwrap(), b"preserve");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn starter_publication_refuses_existing_and_dangling_symbolic_links() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.toml");
    let link = directory.path().join("config.toml");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    #[cfg(windows)]
    if let Err(error) = std::os::windows::fs::symlink_file(&target, &link) {
        // Creating Windows symlinks requires Developer Mode or an explicit
        // privilege. The ordinary/directory and race tests still run here.
        assert_eq!(error.raw_os_error(), Some(1314));
        eprintln!("external: Windows symbolic-link fixture privilege unavailable");
        return;
    }
    assert!(create_config_file(Some(link.clone())).is_err());
    assert!(!target.exists());
    fs::write(&target, b"sentinel").unwrap();
    assert!(create_config_file(Some(link.clone())).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"sentinel");
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn starter_file_is_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    create_config_file(Some(path.clone())).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn starter_creation_missing_default_root_is_a_content_free_error() {
    let error = super::creation::create_config_file_with_root(None, || None).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Could not locate the directory for configuration: entity not found"
    );
}

#[test]
fn starter_creation_explicit_destination_never_discovers_the_default_root() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("config.toml");
    assert_eq!(
        super::creation::create_config_file_with_root(
            Some(destination.clone()),
            || panic!("explicit creation discovered a profile root")
        )
        .unwrap(),
        CreateConfigOutcome::Created
    );
    assert_eq!(fs::read(destination).unwrap(), STARTER);
}

#[test]
fn starter_creation_default_root_creates_only_its_owned_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("new-config-root");
    assert_eq!(
        super::creation::create_config_file_with_root(None, || Some(root.clone()))
            .unwrap(),
        CreateConfigOutcome::Created
    );
    assert_eq!(fs::read(root.join("config.toml")).unwrap(), STARTER);
}
