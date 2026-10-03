use super::*;

fn selection() -> ThemeSelection {
    theme_gallery::builtins().remove(0).selection().unwrap()
}
fn flag() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn themes_scan_is_read_only_and_bad_local_files_do_not_hide_builtins() {
    let root = tempfile::tempdir().unwrap();
    let ThemeOutcome::Inventory { entries, limited } =
        scan(root.path(), &flag()).unwrap()
    else {
        panic!()
    };
    assert_eq!(entries.len(), 5);
    assert!(!limited);
    assert!(!root.path().join("themes").exists());
    fs::create_dir(root.path().join("themes")).unwrap();
    fs::write(
        root.path().join("themes/invalid.toml"),
        "[colors]\nforeground='bad'",
    )
    .unwrap();
    let ThemeOutcome::Inventory { entries, .. } = scan(root.path(), &flag()).unwrap()
    else {
        panic!()
    };
    assert_eq!(entries.len(), 6);
    assert!(entries[..5].iter().all(|entry| entry.theme.is_some()));
    assert!(entries[5].theme.is_none());
    assert!(!entries[5]
        .description
        .contains(root.path().to_str().unwrap()));
}

#[test]
fn themes_import_copy_export_are_canonical_and_collision_safe() {
    let root = tempfile::tempdir().unwrap();
    let cancel = flag();
    let source = selection();
    let first = save_copy(root.path(), source.clone(), &cancel).unwrap();
    let second = save_copy(root.path(), source.clone(), &cancel).unwrap();
    assert_ne!(first.id, second.id);
    let files: Vec<_> = fs::read_dir(root.path().join("themes"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 2);
    for path in &files {
        assert_eq!(read_theme(path).unwrap(), source);
    }
    let destination = root.path().join("export.toml");
    let result = perform(
        root.path(),
        ThemeOperation::Export {
            selection: source.clone(),
            destination: destination.clone(),
        },
        &cancel,
    )
    .unwrap();
    assert!(matches!(result, ThemeOutcome::Exported));
    assert_eq!(read_theme(&destination).unwrap(), source);
    assert!(matches!(
        perform(root.path(), ThemeOperation::Import(destination), &cancel).unwrap(),
        ThemeOutcome::Imported(_)
    ));
    assert_eq!(fs::read_dir(root.path().join("themes")).unwrap().count(), 3);
    assert!(fs::read_dir(root.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".theme-")));
}

#[test]
fn themes_bad_oversized_cancelled_and_directory_sources_never_write() {
    let root = tempfile::tempdir().unwrap();
    let bad = root.path().join("bad.toml");
    fs::write(&bad, "[fonts]\nsize=16").unwrap();
    assert_eq!(read_theme(&bad), Err(ThemeIoError::InvalidTheme));
    fs::write(&bad, vec![b' '; MAX_IMPORT_BYTES + 1]).unwrap();
    assert_eq!(read_theme(&bad), Err(ThemeIoError::TooLarge));
    assert_eq!(read_theme(root.path()), Err(ThemeIoError::UnsafeFile));
    assert!(matches!(
        save_copy(root.path(), selection(), &AtomicBool::new(true)),
        Err(ThemeIoError::Cancelled)
    ));
    assert!(!root.path().join("themes").exists());
}

#[test]
fn themes_worker_delivers_once_and_cancelled_generation_cannot_publish() {
    let root = tempfile::tempdir().unwrap();
    let mut owner = ThemeLibrary::new(root.path().to_owned(), None);
    assert_eq!(
        owner.submit(ThemeOperation::Scan, WindowId::from(0), Instant::now()),
        RefreshSubmission::Queued
    );
    assert_eq!(
        owner.submit(ThemeOperation::Scan, WindowId::from(0), Instant::now()),
        RefreshSubmission::Busy
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let result = loop {
        if let Some(result) = owner.take(Instant::now()) {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(matches!(result, Ok(ThemeOutcome::Inventory { .. })));
    assert!(owner.take(Instant::now()).is_none());
    assert_eq!(
        owner.submit(ThemeOperation::Scan, WindowId::from(0), Instant::now()),
        RefreshSubmission::Queued
    );
    owner.cancel();
    while owner.pending.is_some() {
        assert!(owner.take(Instant::now()).is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(unix)]
#[test]
fn themes_reject_symlinked_sources_and_destinations() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("source.toml");
    fs::write(&file, canonical(&selection()).unwrap()).unwrap();
    let link = root.path().join("link.toml");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    assert_eq!(read_theme(&link), Err(ThemeIoError::UnsafeFile));
    assert_eq!(
        write_file(&link, "bad", true, &flag()),
        Err(ThemeIoError::UnsafeFile)
    );
    assert_eq!(read_theme(&file).unwrap(), selection());
}

#[test]
fn themes_name_roundtrip_limits_and_local_library_failure_keep_builtins() {
    let root = tempfile::tempdir().unwrap();
    let mut chosen = selection();
    chosen.name = "Custom palette".into();
    save_copy(root.path(), chosen.clone(), &flag()).unwrap();
    let ThemeOutcome::Inventory { entries, .. } = scan(root.path(), &flag()).unwrap()
    else {
        panic!()
    };
    assert_eq!(entries[5].selection(), Some(chosen));
    for index in 1..MAX_LOCAL_THEMES {
        fs::write(
            root.path().join(format!("themes/extra-{index}.toml")),
            "[colors]",
        )
        .unwrap();
    }
    assert!(matches!(
        save_copy(root.path(), selection(), &flag()),
        Err(ThemeIoError::Limit)
    ));
    fs::write(root.path().join("themes/overflow.toml"), "[colors]").unwrap();
    let ThemeOutcome::Inventory { entries, limited } =
        scan(root.path(), &flag()).unwrap()
    else {
        panic!()
    };
    assert!(limited);
    assert_eq!(entries.len(), 5 + MAX_LOCAL_THEMES);
    let bad = tempfile::tempdir().unwrap();
    fs::write(bad.path().join("themes"), "not a directory").unwrap();
    let ThemeOutcome::Inventory { entries, .. } =
        perform(bad.path(), ThemeOperation::Scan, &flag()).unwrap()
    else {
        panic!()
    };
    assert_eq!(entries.len(), 6);
    assert!(entries[..5].iter().all(|entry| entry.theme.is_some()));
    assert!(entries[5].theme.is_none());
}

#[test]
fn themes_timeout_keeps_admission_until_completion_and_discards_late_result() {
    let root = tempfile::tempdir().unwrap();
    let mut owner = ThemeLibrary::new(root.path().to_owned(), None);
    // Model an admitted operation stuck in an OS read, without a sleeping thread.
    let cancelled = Arc::new(AtomicBool::new(false));
    owner.pending = Some(Pending {
        id: 42,
        cancelled: cancelled.clone(),
        deadline: Instant::now(),
        timed_out: false,
    });
    assert!(matches!(
        owner.take(Instant::now()),
        Some(Err(ThemeIoError::TimedOut))
    ));
    assert!(cancelled.load(Ordering::Acquire));
    assert_eq!(
        owner.submit(ThemeOperation::Scan, WindowId::from(0), Instant::now()),
        RefreshSubmission::Busy
    );
    assert!(owner.take(Instant::now()).is_none());
    assert!(owner.deadline().is_none());
    let (sender, receiver) = mpsc::sync_channel(1);
    owner.completed = receiver;
    sender
        .send(Completion {
            id: 42,
            result: Ok(ThemeOutcome::Exported),
        })
        .unwrap();
    assert!(owner.take(Instant::now()).is_none());
    assert!(owner.pending.is_none());
}
