//! Lazy bounded profile persistence and launch preparation. The event thread performs no file reads.
use super::{private_fs, theme_gallery};
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use rio_backend::{
    config::{profiles::*, theme::Theme, Config},
    event::{EventProxy, RioEvent, RioEventType, WindowId},
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
const TIMEOUT: Duration = Duration::from_secs(20);
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileSnapshot {
    pub document: ProfileDocument,
    pub bytes: Option<Vec<u8>>,
}
#[derive(Clone)]
pub enum ProfileOperation {
    Load,
    Save {
        document: ProfileDocument,
        expected: Option<Vec<u8>>,
    },
    Import(PathBuf),
    Export {
        document: ProfileDocument,
        destination: PathBuf,
    },
    Resolve {
        profile: NamedProfile,
        base: Box<Config>,
        cwd: Option<String>,
    },
}
pub enum ProfileOutcome {
    Loaded(ProfileSnapshot),
    Imported(ProfileDocument),
    Exported,
    Resolved(Box<Config>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileIoError {
    Invalid,
    Io,
    Conflict,
    Cancelled,
    TimedOut,
    Worker,
    Directory,
    Environment,
    Theme,
    Platform,
}
impl ProfileIoError {
    pub fn message(self) -> &'static str {
        match self {
        Self::Invalid=>"Choose a valid version 1 profile TOML. Saved profiles remain unchanged.",
        Self::Io=>"Profile file is unavailable or unsafe. Check its location and retry.",
        Self::Conflict=>"Profiles changed elsewhere. Reopen Profiles before saving; your draft has not been written.",
        Self::Cancelled=>"Profile operation cancelled.",Self::TimedOut=>"Profile operation timed out. Retry when it finishes.",Self::Worker=>"Profiles service is unavailable. Reopen Profiles to retry.",
        Self::Directory=>"Working directory is missing or is not an absolute directory.",
        Self::Environment=>"An environment reference is unavailable. Set it before opening Automexia.",
        Self::Theme=>"The selected theme is unavailable or invalid.",Self::Platform=>"This profile is for another operating system.",
    }
    }
}
struct Request {
    id: u64,
    operation: ProfileOperation,
    cancelled: Arc<AtomicBool>,
    window: WindowId,
}
struct Completion {
    id: u64,
    result: Result<ProfileOutcome, ProfileIoError>,
}
struct Pending {
    id: u64,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    timed_out: bool,
}
pub struct ProfileLibrary {
    worker: BoundedWorker<Request>,
    completed: mpsc::Receiver<Completion>,
    pending: Option<Pending>,
    next: Option<u64>,
}

struct Publisher<'a> {
    sender: &'a mpsc::SyncSender<Completion>,
    value: Option<Completion>,
    proxy: Option<&'a EventProxy>,
    window: WindowId,
}
impl Drop for Publisher<'_> {
    fn drop(&mut self) {
        if self
            .value
            .take()
            .is_some_and(|value| self.sender.try_send(value).is_ok())
        {
            if let Some(proxy) = self.proxy {
                proxy.send_event(RioEventType::Rio(RioEvent::Render), self.window);
            }
        }
    }
}
impl ProfileLibrary {
    pub fn new(root: PathBuf, proxy: Option<EventProxy>) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        Self {
            worker: BoundedWorker::new("profile-library", 1, move |request: Request| {
                let mut publisher = Publisher {
                    sender: &sender,
                    value: Some(Completion {
                        id: request.id,
                        result: Err(ProfileIoError::Worker),
                    }),
                    proxy: proxy.as_ref(),
                    window: request.window,
                };
                let result = perform(&root, request.operation, &request.cancelled);
                publisher.value = Some(Completion {
                    id: request.id,
                    result: if request.cancelled.load(Ordering::Acquire) {
                        Err(ProfileIoError::Cancelled)
                    } else {
                        result
                    },
                });
            }),
            completed,
            pending: None,
            next: Some(1),
        }
    }
    pub fn submit(
        &mut self,
        operation: ProfileOperation,
        window: WindowId,
        now: Instant,
    ) -> RefreshSubmission {
        if self.pending.is_some() {
            return RefreshSubmission::Busy;
        }
        let Some(id) = self.next else {
            return RefreshSubmission::Unavailable;
        };
        self.next = id.checked_add(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.pending = Some(Pending {
            id,
            cancelled: cancelled.clone(),
            deadline: now + TIMEOUT,
            timed_out: false,
        });
        let result = self.worker.try_submit(Request {
            id,
            operation,
            cancelled,
            window,
        });
        if result != RefreshSubmission::Queued {
            self.pending = None;
        }
        result
    }
    pub fn cancel(&mut self) {
        if let Some(pending) = &self.pending {
            pending.cancelled.store(true, Ordering::Release);
        }
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.pending
            .as_ref()
            .filter(|p| !p.timed_out && !p.cancelled.load(Ordering::Acquire))
            .map(|p| p.deadline)
    }
    pub fn take(
        &mut self,
        now: Instant,
    ) -> Option<Result<ProfileOutcome, ProfileIoError>> {
        if let Ok(completion) = self.completed.try_recv() {
            let pending = self.pending.take()?;
            if pending.id == completion.id && !pending.cancelled.load(Ordering::Acquire) {
                return Some(completion.result);
            }
            return None;
        }
        if let Some(pending) = self.pending.as_mut() {
            if !pending.timed_out
                && !pending.cancelled.load(Ordering::Acquire)
                && now >= pending.deadline
            {
                pending.timed_out = true;
                pending.cancelled.store(true, Ordering::Release);
                return Some(Err(ProfileIoError::TimedOut));
            }
        }
        None
    }
}
impl Drop for ProfileLibrary {
    fn drop(&mut self) {
        self.cancel();
        self.worker.request_shutdown();
    }
}

fn check_cancel(cancelled: &AtomicBool) -> Result<(), ProfileIoError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ProfileIoError::Cancelled)
    } else {
        Ok(())
    }
}
fn load(root: &Path) -> Result<ProfileSnapshot, ProfileIoError> {
    let directory = root.join("profiles");
    if !directory.try_exists().map_err(|_| ProfileIoError::Io)? {
        return Ok(ProfileSnapshot::default());
    }
    private_fs::validate_private_child_directory(&directory)
        .map_err(|_| ProfileIoError::Io)?;
    let bytes = private_fs::read_bounded_regular(
        &directory.join("profiles-v1.toml"),
        MAX_DOCUMENT_BYTES,
    )
    .map_err(|_| ProfileIoError::Io)?;
    let document = bytes
        .as_ref()
        .map(|bytes| {
            std::str::from_utf8(bytes)
                .map_err(|_| ProfileIoError::Invalid)
                .and_then(|s| {
                    ProfileDocument::parse(s).map_err(|_| ProfileIoError::Invalid)
                })
        })
        .transpose()?
        .unwrap_or_default();
    Ok(ProfileSnapshot { document, bytes })
}
fn profile_error(error: ProfileError) -> ProfileIoError {
    match error {
        ProfileError::Platform => ProfileIoError::Platform,
        ProfileError::Environment => ProfileIoError::Environment,
        ProfileError::Directory => ProfileIoError::Directory,
        _ => ProfileIoError::Invalid,
    }
}
fn perform(
    root: &Path,
    operation: ProfileOperation,
    cancelled: &AtomicBool,
) -> Result<ProfileOutcome, ProfileIoError> {
    check_cancel(cancelled)?;
    match operation {
        ProfileOperation::Load => load(root).map(ProfileOutcome::Loaded),
        ProfileOperation::Save { document, expected } => {
            let text = document.to_toml().map_err(profile_error)?;
            let directory = root.join("profiles");
            private_fs::ensure_private_child_directory(&directory)
                .map_err(|_| ProfileIoError::Io)?;
            let lock = private_fs::open_private_lock(&directory.join("profiles.lock"))
                .map_err(|_| ProfileIoError::Io)?;
            let _guard = private_fs::WriteLock::try_acquire(lock)
                .map_err(|_| ProfileIoError::Conflict)?;
            if load(root)?.bytes != expected {
                return Err(ProfileIoError::Conflict);
            }
            check_cancel(cancelled)?;
            private_fs::atomic_write_private(
                &directory,
                &directory.join("profiles-v1.toml"),
                text.as_bytes(),
                MAX_DOCUMENT_BYTES,
                "profile-",
            )
            .map_err(|_| ProfileIoError::Io)?;
            Ok(ProfileOutcome::Loaded(ProfileSnapshot {
                document,
                bytes: Some(text.into_bytes()),
            }))
        }
        ProfileOperation::Import(path) => {
            let bytes =
                private_fs::read_bounded_untrusted_regular(&path, MAX_DOCUMENT_BYTES)
                    .map_err(|_| ProfileIoError::Io)?
                    .ok_or(ProfileIoError::Io)?;
            let text =
                std::str::from_utf8(&bytes).map_err(|_| ProfileIoError::Invalid)?;
            let document = ProfileDocument::parse(text).map_err(profile_error)?;
            check_cancel(cancelled)?;
            Ok(ProfileOutcome::Imported(document))
        }
        ProfileOperation::Export {
            document,
            destination,
        } => {
            let text = document.to_toml().map_err(profile_error)?;
            // Create-new avoids replacing unrelated exports; the native dialog chooses the path.
            let mut file = private_fs::create_private_file(&destination)
                .map_err(|_| ProfileIoError::Io)?;
            use std::io::Write;
            file.write_all(text.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|_| ProfileIoError::Io)?;
            Ok(ProfileOutcome::Exported)
        }
        ProfileOperation::Resolve { profile, base, cwd } => {
            let home = dirs::home_dir().and_then(|p| p.to_str().map(str::to_owned));
            let mut config = profile
                .resolve(
                    &base,
                    ProfilePlatform::current(),
                    cwd.as_deref(),
                    home.as_deref(),
                    |name| std::env::var(name).ok(),
                )
                .map_err(profile_error)?;
            if let Some(directory) = &config.working_dir {
                let path = Path::new(directory);
                if !path.is_absolute() || !path.is_dir() {
                    return Err(ProfileIoError::Directory);
                }
            }
            if let Some(name) = &profile.theme {
                let theme = if let Some(theme) = theme_gallery::builtins()
                    .into_iter()
                    .find(|t| t.id.strip_prefix("builtin:") == Some(name.as_str()))
                    .and_then(|t| t.theme)
                {
                    theme
                } else {
                    let directory = root.join("themes");
                    private_fs::inspect_directory(&directory)
                        .map_err(|_| ProfileIoError::Theme)?;
                    let bytes = private_fs::read_bounded_untrusted_regular(
                        &directory.join(format!("{name}.toml")),
                        64 * 1024,
                    )
                    .map_err(|_| ProfileIoError::Theme)?
                    .ok_or(ProfileIoError::Theme)?;
                    Theme::parse_for_gallery(
                        std::str::from_utf8(&bytes).map_err(|_| ProfileIoError::Theme)?,
                    )
                    .map_err(|_| ProfileIoError::Theme)?
                };
                config.colors = theme.colors;
                config.adaptive_colors = None;
                config.theme = name.clone();
            }
            check_cancel(cancelled)?;
            Ok(ProfileOutcome::Resolved(Box::new(config)))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_profile_work_retains_ownership_until_drained_and_never_delivers_stale_data(
    ) {
        let root = tempfile::tempdir().unwrap();
        let mut library = ProfileLibrary::new(root.path().into(), None);
        assert_eq!(
            library.submit(ProfileOperation::Load, WindowId::from(0), Instant::now()),
            RefreshSubmission::Queued
        );
        library.cancel();
        assert_eq!(
            library.submit(ProfileOperation::Load, WindowId::from(0), Instant::now()),
            RefreshSubmission::Busy
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while library.pending.is_some() && Instant::now() < deadline {
            assert!(library.take(Instant::now()).is_none());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(library.pending.is_none());
        assert_eq!(
            library.submit(ProfileOperation::Load, WindowId::from(0), Instant::now()),
            RefreshSubmission::Queued
        );
        loop {
            if let Some(result) = library.take(Instant::now()) {
                assert!(matches!(result, Ok(ProfileOutcome::Loaded(_))));
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn resolve_builtin_theme_without_local_files_and_reject_missing_cwd() {
        let root = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(false);
        let mut profile = NamedProfile::new("work".into(), "Work".into());
        profile.theme = Some("solar-dusk".into());
        let outcome = perform(
            root.path(),
            ProfileOperation::Resolve {
                profile: profile.clone(),
                base: Box::default(),
                cwd: None,
            },
            &cancelled,
        )
        .unwrap();
        let ProfileOutcome::Resolved(config) = outcome else {
            panic!("expected resolved profile")
        };
        assert_eq!(config.theme, "solar-dusk");
        profile.directory = ProfileDirectory::Fixed {
            path: root.path().join("absent").to_string_lossy().into_owned(),
        };
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Resolve {
                    profile,
                    base: Box::default(),
                    cwd: None
                },
                &cancelled
            ),
            Err(ProfileIoError::Directory)
        ));
    }
    #[test]
    fn private_store_roundtrip_and_conflict_preserve_first_writer() {
        let root = tempfile::tempdir().unwrap();
        let c = AtomicBool::new(false);
        let d = ProfileDocument {
            version: 1,
            profiles: vec![NamedProfile::new("work".into(), "Work".into())],
        };
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Save {
                    document: d.clone(),
                    expected: None
                },
                &c
            ),
            Ok(ProfileOutcome::Loaded(_))
        ));
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Save {
                    document: ProfileDocument::default(),
                    expected: None
                },
                &c
            ),
            Err(ProfileIoError::Conflict)
        ));
        assert_eq!(load(root.path()).unwrap().document, d);
        private_fs::inspect_private_file(&root.path().join("profiles/profiles-v1.toml"))
            .unwrap();
    }
    #[test]
    fn import_never_persists_and_cancelled_save_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("input.toml");
        std::fs::write(&input, "version = 1").unwrap();
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Import(input),
                &AtomicBool::new(false)
            ),
            Ok(ProfileOutcome::Imported(_))
        ));
        assert!(!root.path().join("profiles").exists());
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Save {
                    document: ProfileDocument::default(),
                    expected: None
                },
                &AtomicBool::new(true)
            ),
            Err(ProfileIoError::Cancelled)
        ));
        assert!(!root.path().join("profiles").exists());
    }
    #[test]
    fn invalid_future_file_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let d = root.path().join("profiles");
        private_fs::ensure_private_child_directory(&d).unwrap();
        private_fs::atomic_write_private(
            &d,
            &d.join("profiles-v1.toml"),
            b"version = 2",
            MAX_DOCUMENT_BYTES,
            "profile-",
        )
        .unwrap();
        assert!(matches!(
            perform(
                root.path(),
                ProfileOperation::Save {
                    document: ProfileDocument::default(),
                    expected: Some(b"version = 2".to_vec())
                },
                &AtomicBool::new(false)
            ),
            Err(ProfileIoError::Invalid)
        ));
    }
}
