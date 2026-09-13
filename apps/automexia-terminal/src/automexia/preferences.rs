//! Versioned, application-owned persistence for runtime-editable preferences.
//!
//! The hand-edited `config.toml` remains the canonical configuration. This
//! small overlay stores only settings changed from the running UI, so a zoom
//! shortcut never rewrites comments or unrelated configuration keys.

use crate::automexia::private_fs::{self, PrivateFsErrorCode};
use rio_backend::config::{theme::AppearanceTheme, Config};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tempfile::Builder;

const SCHEMA_VERSION: u16 = 1;
const STATE_DIRECTORY: &str = "state";
const PRIMARY_FILE: &str = "user-preferences-v1.toml";
const PREVIOUS_FILE: &str = "user-preferences-v1.previous.toml";
const LOCK_FILE: &str = "user-preferences-v1.lock";
const STAGING_PREFIX: &str = ".user-preferences-";
pub const MAX_PREFERENCE_BYTES: usize = 16 * 1024;
pub const MIN_FONT_POINTS: f32 = 6.0;
pub const MAX_FONT_POINTS: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceErrorCode {
    Io,
    InvalidData,
    LinkRejected,
    NotRegularFile,
    PrivatePermissions,
    SourceTooLarge,
    SourceChanged,
    InvalidRoot,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreferenceError {
    code: PreferenceErrorCode,
}

impl PreferenceError {
    const fn new(code: PreferenceErrorCode) -> Self {
        Self { code }
    }

    fn io(_error: std::io::Error) -> Self {
        Self::new(PreferenceErrorCode::Io)
    }

    pub const fn code(self) -> PreferenceErrorCode {
        self.code
    }
}

impl From<crate::automexia::private_fs::PrivateFsError> for PreferenceError {
    fn from(error: crate::automexia::private_fs::PrivateFsError) -> Self {
        let code = match error.code() {
            PrivateFsErrorCode::Io => PreferenceErrorCode::Io,
            PrivateFsErrorCode::LinkRejected => PreferenceErrorCode::LinkRejected,
            PrivateFsErrorCode::NotDirectory | PrivateFsErrorCode::InvalidRoot => {
                PreferenceErrorCode::InvalidRoot
            }
            PrivateFsErrorCode::NotRegularFile => PreferenceErrorCode::NotRegularFile,
            PrivateFsErrorCode::PrivatePermissions => {
                PreferenceErrorCode::PrivatePermissions
            }
            PrivateFsErrorCode::SourceTooLarge => PreferenceErrorCode::SourceTooLarge,
            PrivateFsErrorCode::SourceChanged => PreferenceErrorCode::SourceChanged,
        };
        Self::new(code)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserPreferences {
    pub font_size: Option<f32>,
    pub appearance_theme: Option<AppearanceTheme>,
    pub shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
}

impl UserPreferences {
    pub fn apply_to(&self, base: &Config) -> Config {
        let mut effective = base.clone();
        if let Some(font_size) = self.font_size {
            effective.fonts.size = font_size;
        }
        if let Some(theme) = self.appearance_theme {
            effective.force_theme = Some(theme);
        }
        effective.bindings.ui_shortcuts = self.shortcuts.clone();
        effective
    }

    fn validate(&self) -> Result<(), PreferenceError> {
        crate::automexia::shortcut_preferences::validate_records(&self.shortcuts)
            .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
        if self.font_size.is_some_and(|value| {
            !value.is_finite() || !(MIN_FONT_POINTS..=MAX_FONT_POINTS).contains(&value)
        }) {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredPreferences {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shortcuts: Vec<rio_backend::config::bindings::UiShortcut>,
    #[serde(default, rename = "font-size", skip_serializing_if = "Option::is_none")]
    font_size: Option<f32>,
    #[serde(
        default,
        rename = "appearance-theme",
        skip_serializing_if = "Option::is_none"
    )]
    appearance_theme: Option<AppearanceTheme>,
}

impl TryFrom<StoredPreferences> for UserPreferences {
    type Error = PreferenceError;

    fn try_from(stored: StoredPreferences) -> Result<Self, Self::Error> {
        if stored.schema_version != SCHEMA_VERSION {
            return Err(PreferenceError::new(PreferenceErrorCode::InvalidData));
        }
        let preferences = Self {
            font_size: stored.font_size,
            appearance_theme: stored.appearance_theme,
            shortcuts: stored.shortcuts,
        };
        preferences.validate()?;
        Ok(preferences)
    }
}

impl From<&UserPreferences> for StoredPreferences {
    fn from(preferences: &UserPreferences) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            shortcuts: preferences.shortcuts.clone(),
            font_size: preferences.font_size,
            appearance_theme: preferences.appearance_theme,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceSource {
    Defaults,
    Primary,
    Previous,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadOutcome {
    pub preferences: UserPreferences,
    pub source: PreferenceSource,
    pub warning: Option<PreferenceErrorCode>,
}

pub fn load() -> LoadOutcome {
    load_from_root(&rio_backend::config::config_dir_path())
}

pub fn writer() -> PreferenceWriter {
    PreferenceWriter::new(rio_backend::config::config_dir_path())
}

fn state_root(root: &Path) -> PathBuf {
    root.join(STATE_DIRECTORY)
}

fn primary_path(root: &Path) -> PathBuf {
    state_root(root).join(PRIMARY_FILE)
}

fn previous_path(root: &Path) -> PathBuf {
    state_root(root).join(PREVIOUS_FILE)
}

fn lock_path(root: &Path) -> PathBuf {
    state_root(root).join(LOCK_FILE)
}

fn parse_snapshot(bytes: &[u8]) -> Result<UserPreferences, PreferenceError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    let stored: StoredPreferences = toml::from_str(text)
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?;
    stored.try_into()
}

fn read_snapshot(path: &Path) -> Result<Option<UserPreferences>, PreferenceError> {
    private_fs::read_bounded_regular(path, MAX_PREFERENCE_BYTES)
        .map_err(Into::into)
        .and_then(|bytes| bytes.map(|bytes| parse_snapshot(&bytes)).transpose())
}

pub fn load_from_root(root: &Path) -> LoadOutcome {
    match fs::symlink_metadata(state_root(root)) {
        Ok(_) => {
            if let Err(error) =
                private_fs::validate_private_child_directory(&state_root(root))
            {
                return LoadOutcome {
                    preferences: UserPreferences::default(),
                    source: PreferenceSource::Defaults,
                    warning: Some(PreferenceError::from(error).code()),
                };
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return LoadOutcome {
                preferences: UserPreferences::default(),
                source: PreferenceSource::Defaults,
                warning: Some(PreferenceError::io(error).code()),
            };
        }
    }
    match read_snapshot(&primary_path(root)) {
        Ok(Some(preferences)) => LoadOutcome {
            preferences,
            source: PreferenceSource::Primary,
            warning: None,
        },
        Ok(None) => match read_snapshot(&previous_path(root)) {
            Ok(Some(preferences)) => LoadOutcome {
                preferences,
                source: PreferenceSource::Previous,
                warning: Some(PreferenceErrorCode::InvalidData),
            },
            Ok(None) => LoadOutcome {
                preferences: UserPreferences::default(),
                source: PreferenceSource::Defaults,
                warning: None,
            },
            Err(error) => LoadOutcome {
                preferences: UserPreferences::default(),
                source: PreferenceSource::Defaults,
                warning: Some(error.code()),
            },
        },
        Err(primary_error) => match read_snapshot(&previous_path(root)) {
            Ok(Some(preferences)) => LoadOutcome {
                preferences,
                source: PreferenceSource::Previous,
                warning: Some(primary_error.code()),
            },
            Ok(None) | Err(_) => LoadOutcome {
                preferences: UserPreferences::default(),
                source: PreferenceSource::Defaults,
                warning: Some(primary_error.code()),
            },
        },
    }
}

fn ensure_state_root(root: &Path) -> Result<PathBuf, PreferenceError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err(PreferenceError::new(PreferenceErrorCode::InvalidRoot)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root).map_err(PreferenceError::io)?;
        }
        Err(error) => return Err(PreferenceError::io(error)),
    }
    let state = state_root(root);
    private_fs::ensure_private_child_directory(&state)?;
    Ok(state)
}

fn serialize(preferences: &UserPreferences) -> Result<Vec<u8>, PreferenceError> {
    preferences.validate()?;
    let bytes = toml::to_string_pretty(&StoredPreferences::from(preferences))
        .map_err(|_| PreferenceError::new(PreferenceErrorCode::InvalidData))?
        .into_bytes();
    if bytes.len() > MAX_PREFERENCE_BYTES {
        return Err(PreferenceError::new(PreferenceErrorCode::SourceTooLarge));
    }
    Ok(bytes)
}

fn persist_bytes(
    parent: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), PreferenceError> {
    if bytes.len() > MAX_PREFERENCE_BYTES {
        return Err(PreferenceError::new(PreferenceErrorCode::SourceTooLarge));
    }
    let mut staged = Builder::new()
        .prefix(STAGING_PREFIX)
        .tempfile_in(parent)
        .map_err(PreferenceError::io)?;
    private_fs::apply_private_file_permissions(staged.path())?;
    staged.write_all(bytes).map_err(PreferenceError::io)?;
    staged
        .as_file_mut()
        .sync_all()
        .map_err(PreferenceError::io)?;
    private_fs::reject_link_or_non_file(destination)?;
    let file = staged
        .persist(destination)
        .map_err(|error| PreferenceError::io(error.error))?;
    private_fs::apply_private_file_permissions(destination)?;
    file.sync_all().map_err(PreferenceError::io)?;
    private_fs::sync_directory(parent)?;
    Ok(())
}

pub fn write_to_root(
    root: &Path,
    preferences: &UserPreferences,
) -> Result<(), PreferenceError> {
    let state = ensure_state_root(root)?;
    let lock = private_fs::open_private_lock(&lock_path(root))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Err(PreferenceError::new(PreferenceErrorCode::Busy));
        }
        Err(TryLockError::Error(error)) => return Err(PreferenceError::io(error)),
    }
    let primary = primary_path(root);
    if let Ok(Some(current)) = read_snapshot(&primary) {
        let previous = serialize(&current)?;
        persist_bytes(&state, &previous_path(root), &previous)?;
    }
    let candidate = serialize(preferences)?;
    persist_bytes(&state, &primary, &candidate)
}

#[derive(Debug, Default)]
struct WriterState {
    pending: Option<(u64, UserPreferences)>,
    submitted: u64,
    completed: Option<(u64, Result<(), PreferenceErrorCode>)>,
    writing: bool,
    stopping: bool,
    stopped: bool,
    maximum_pending_depth: usize,
    last_error: Option<PreferenceErrorCode>,
    write_failed: bool,
}

type SharedWriterState = Arc<(Mutex<WriterState>, Condvar)>;

pub struct PreferenceWriter {
    root: PathBuf,
    shared: SharedWriterState,
    worker: Option<JoinHandle<()>>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl PreferenceWriter {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            shared: Arc::new((Mutex::new(WriterState::default()), Condvar::new())),
            worker: None,
            wake: None,
        }
    }

    fn ensure_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let root = self.root.clone();
        let shared = Arc::clone(&self.shared);
        let notify = self.wake.clone();
        match std::thread::Builder::new()
            .name("automexia-preferences".into())
            .spawn(move || writer_loop(root, shared, notify))
        {
            Ok(worker) => self.worker = Some(worker),
            Err(_) => {
                let (lock, wake) = &*self.shared;
                let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                state.last_error = Some(PreferenceErrorCode::Io);
                state.write_failed = true;
                state.stopped = true;
                wake.notify_all();
            }
        }
    }

    pub fn set_wake(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.wake = Some(wake);
    }

    pub fn completion(&self) -> Option<(u64, Result<(), PreferenceErrorCode>)> {
        self.shared
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .completed
    }

    pub fn submit(&mut self, preferences: UserPreferences) -> u64 {
        if preferences.validate().is_err() {
            let (lock, _) = &*self.shared;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.last_error = Some(PreferenceErrorCode::InvalidData);
            state.write_failed = true;
            return 0;
        }
        self.ensure_worker();
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.stopping || state.stopped {
            state.last_error = Some(PreferenceErrorCode::Io);
            state.write_failed = true;
            return 0;
        }
        state.submitted = state.submitted.saturating_add(1);
        let revision = state.submitted;
        state.pending = Some((revision, preferences));
        state.maximum_pending_depth = state.maximum_pending_depth.max(1);
        wake.notify_one();
        revision
    }

    pub fn take_error(&self) -> Option<PreferenceErrorCode> {
        let (lock, _) = &*self.shared;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .last_error
            .take()
    }

    pub fn maximum_pending_depth(&self) -> usize {
        let (lock, _) = &*self.shared;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .maximum_pending_depth
    }

    pub fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        while state.pending.is_some() || state.writing {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, result) = wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if result.timed_out() && (state.pending.is_some() || state.writing) {
                return false;
            }
        }
        !state.write_failed
    }

    pub fn shutdown(&mut self, timeout: Duration) -> bool {
        if self.worker.is_none() {
            return true;
        }
        let deadline = Instant::now() + timeout;
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.stopping = true;
        wake.notify_all();
        while !state.stopped {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, result) = wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if result.timed_out() && !state.stopped {
                return false;
            }
        }
        let clean = !state.write_failed;
        drop(state);
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                return false;
            }
        }
        clean
    }
}

impl Drop for PreferenceWriter {
    fn drop(&mut self) {
        let _ = self.shutdown(Duration::from_millis(250));
    }
}

fn writer_loop(
    root: PathBuf,
    shared: SharedWriterState,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    loop {
        let (revision, preferences) = {
            let (lock, wake) = &*shared;
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            while state.pending.is_none() && !state.stopping {
                state = wake.wait(state).unwrap_or_else(|error| error.into_inner());
            }
            if state.pending.is_none() && state.stopping {
                state.stopped = true;
                wake.notify_all();
                return;
            }
            state.writing = true;
            state.pending.take().expect("pending preference exists")
        };

        let result = write_to_root(&root, &preferences);
        let (lock, wake) = &*shared;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.writing = false;
        let result = result.map_err(PreferenceError::code);
        state.last_error = result.err();
        state.write_failed = result.is_err();
        state.completed = Some((revision, result));
        wake.notify_all();
        drop(state);
        if let Some(notify) = &notify {
            notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_store_uses_config_without_creating_files() {
        let root = tempfile::tempdir().unwrap();
        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.source, PreferenceSource::Defaults);
        assert!(outcome.warning.is_none());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn canonical_round_trip_preserves_font_and_theme() {
        let root = tempfile::tempdir().unwrap();
        let expected = UserPreferences {
            font_size: Some(21.5),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Light),
            ..UserPreferences::default()
        };

        write_to_root(root.path(), &expected).unwrap();
        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, expected);
        assert_eq!(outcome.source, PreferenceSource::Primary);
        assert!(outcome.warning.is_none());
    }

    #[test]
    fn invalid_primary_recovers_the_last_known_good_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let first = UserPreferences {
            font_size: Some(17.0),
            appearance_theme: None,
            ..UserPreferences::default()
        };
        let second = UserPreferences {
            font_size: Some(19.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Dark),
            ..UserPreferences::default()
        };
        write_to_root(root.path(), &first).unwrap();
        write_to_root(root.path(), &second).unwrap();
        std::fs::write(
            primary_path(root.path()),
            b"schema-version = 1\nfont-size = nan",
        )
        .unwrap();

        let outcome = load_from_root(root.path());

        assert_eq!(outcome.preferences, first);
        assert_eq!(outcome.source, PreferenceSource::Previous);
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::InvalidData));
    }

    #[test]
    fn malformed_oversized_unknown_future_and_out_of_range_inputs_fail_closed() {
        let root = tempfile::tempdir().unwrap();
        let cases = [
            b"not toml".to_vec(),
            b"schema-version = 2".to_vec(),
            b"schema-version = 1\nunknown = true".to_vec(),
            b"schema-version = 1\nfont-size = 5.99".to_vec(),
            b"schema-version = 1\nfont-size = 100.01".to_vec(),
            vec![b'x'; MAX_PREFERENCE_BYTES + 1],
        ];

        for bytes in cases {
            remove_store(root.path());
            ensure_state_root(root.path()).unwrap();
            std::fs::write(primary_path(root.path()), bytes).unwrap();
            crate::automexia::private_fs::apply_private_file_permissions(&primary_path(
                root.path(),
            ))
            .unwrap();
            let outcome = load_from_root(root.path());
            assert_eq!(outcome.preferences, UserPreferences::default());
            assert_eq!(outcome.source, PreferenceSource::Defaults);
            assert!(outcome.warning.is_some());
        }
    }

    #[test]
    fn application_overlay_changes_only_runtime_owned_fields() {
        let mut base = rio_backend::config::Config::default();
        base.fonts.size = 14.0;
        base.confirm_before_quit = false;
        let preferences = UserPreferences {
            font_size: Some(20.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Light),
            ..UserPreferences::default()
        };

        let effective = preferences.apply_to(&base);

        assert_eq!(base.fonts.size, 14.0);
        assert_eq!(effective.fonts.size, 20.0);
        assert_eq!(
            effective.force_theme,
            Some(rio_backend::config::theme::AppearanceTheme::Light)
        );
        assert!(!effective.confirm_before_quit);
    }

    #[test]
    fn bounded_writer_coalesces_to_latest_and_flushes_for_restart() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().to_path_buf());
        for size in 6..=100 {
            writer.submit(UserPreferences {
                font_size: Some(size as f32),
                appearance_theme: None,
                ..UserPreferences::default()
            });
        }

        assert!(writer.flush(std::time::Duration::from_secs(5)));
        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences.font_size, Some(100.0));
        assert!(writer.maximum_pending_depth() <= 1);
        assert!(writer.shutdown(std::time::Duration::from_secs(5)));
    }

    #[test]
    fn writer_notification_follows_publication_and_consuming_error_cannot_fake_durability(
    ) {
        let root = tempfile::tempdir().unwrap();
        write_to_root(root.path(), &UserPreferences::default()).unwrap();
        let held =
            crate::automexia::private_fs::open_private_lock(&lock_path(root.path()))
                .unwrap();
        held.try_lock().unwrap();
        let mut writer = PreferenceWriter::new(root.path().into());
        let shared = writer.shared.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        writer.set_wake(Arc::new(move || {
            let completed = shared.0.lock().unwrap().completed;
            sender.send(completed).unwrap();
        }));
        let candidate = UserPreferences {
            font_size: Some(22.0),
            ..UserPreferences::default()
        };
        let failed = writer.submit(candidate.clone());
        let outcome = receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert_eq!(outcome.0, failed);
        assert!(outcome.1.is_err());
        assert!(writer.take_error().is_some());
        assert!(!writer.flush(Duration::from_secs(5)));
        assert_eq!(
            load_from_root(root.path()).preferences,
            UserPreferences::default()
        );
        drop(held);
        let saved = writer.submit(candidate.clone());
        assert!(saved > failed);
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(5)).unwrap(),
            Some((saved, Ok(())))
        );
        assert!(writer.flush(Duration::from_secs(5)));
        assert_eq!(load_from_root(root.path()).preferences, candidate);
        assert!(writer.shutdown(Duration::from_secs(5)));
    }

    #[test]
    fn reset_snapshot_survives_restart_without_overriding_config() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = PreferenceWriter::new(root.path().to_path_buf());
        writer.submit(UserPreferences {
            font_size: Some(24.0),
            appearance_theme: Some(rio_backend::config::theme::AppearanceTheme::Dark),
            ..UserPreferences::default()
        });
        writer.submit(UserPreferences::default());
        assert!(writer.shutdown(std::time::Duration::from_secs(5)));

        let mut base = rio_backend::config::Config::default();
        base.fonts.size = 15.0;
        let effective = load_from_root(root.path()).preferences.apply_to(&base);
        assert_eq!(effective.fonts.size, 15.0);
        assert_eq!(effective.force_theme, base.force_theme);
    }

    #[test]
    fn concurrent_writer_is_rejected_without_mutating_the_primary_snapshot() {
        let root = tempfile::tempdir().unwrap();
        let original = UserPreferences {
            font_size: Some(16.0),
            appearance_theme: None,
            ..UserPreferences::default()
        };
        write_to_root(root.path(), &original).unwrap();
        let lock =
            crate::automexia::private_fs::open_private_lock(&lock_path(root.path()))
                .unwrap();
        lock.try_lock().unwrap();

        let error = write_to_root(
            root.path(),
            &UserPreferences {
                font_size: Some(22.0),
                appearance_theme: None,
                ..UserPreferences::default()
            },
        )
        .unwrap_err();

        assert_eq!(error.code(), PreferenceErrorCode::Busy);
        assert_eq!(load_from_root(root.path()).preferences, original);
    }

    #[test]
    fn durable_files_use_private_permissions_and_leave_no_staging_artifacts() {
        let root = tempfile::tempdir().unwrap();
        write_to_root(
            root.path(),
            &UserPreferences {
                font_size: Some(18.0),
                appearance_theme: None,
                ..UserPreferences::default()
            },
        )
        .unwrap();

        crate::automexia::private_fs::inspect_private_file(&primary_path(root.path()))
            .unwrap();
        crate::automexia::private_fs::inspect_private_file(&lock_path(root.path()))
            .unwrap();
        let staging = std::fs::read_dir(state_root(root.path()))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(STAGING_PREFIX)
            })
            .count();
        assert_eq!(staging, 0);
    }

    #[cfg(unix)]
    #[test]
    fn linked_preference_file_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        ensure_state_root(root.path()).unwrap();
        let outside = root.path().join("outside.toml");
        std::fs::write(&outside, b"schema-version = 1\nfont-size = 42.0").unwrap();
        symlink(&outside, primary_path(root.path())).unwrap();

        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::LinkRejected));
    }

    #[cfg(unix)]
    #[test]
    fn linked_state_directory_is_rejected_before_reading_a_regular_child() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(
            outside.path().join(PRIMARY_FILE),
            b"schema-version = 1\nfont-size = 42.0",
        )
        .unwrap();
        symlink(outside.path(), state_root(root.path())).unwrap();

        let outcome = load_from_root(root.path());
        assert_eq!(outcome.preferences, UserPreferences::default());
        assert_eq!(outcome.warning, Some(PreferenceErrorCode::LinkRejected));
    }

    fn remove_store(root: &Path) {
        let state = state_root(root);
        if !state.exists() {
            return;
        }
        for path in [primary_path(root), previous_path(root)] {
            let _ = std::fs::remove_file(path);
        }
    }
}
