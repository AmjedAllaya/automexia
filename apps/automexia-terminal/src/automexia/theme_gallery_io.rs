//! One lazy, bounded owner of local theme files and inventory work.
use super::{
    private_fs,
    theme_gallery::{
        self, ThemeDescriptor, ThemeSelection, ThemeSource, MAX_IMPORT_BYTES,
        MAX_LOCAL_THEMES,
    },
};
use automexia_extension_runtime::{BoundedWorker, RefreshSubmission};
use rio_backend::{
    config::theme::Theme,
    event::{EventProxy, RioEvent, RioEventType, WindowId},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

const MAX_SCAN_ENTRIES: usize = 512;
const MAX_SCAN_BYTES: usize = 4 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone, Debug)]
pub enum ThemeOperation {
    Scan,
    Import(PathBuf),
    SaveCopy(ThemeSelection),
    Export {
        selection: ThemeSelection,
        destination: PathBuf,
    },
}
#[derive(Clone, Debug)]
pub enum ThemeOutcome {
    Inventory {
        entries: Vec<ThemeDescriptor>,
        limited: bool,
    },
    Imported(ThemeDescriptor),
    Saved(ThemeDescriptor),
    Exported,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeIoError {
    InvalidTheme,
    UnsafeFile,
    TooLarge,
    Limit,
    Io,
    Cancelled,
    TimedOut,
    Worker,
}
impl ThemeIoError {
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidTheme => "Choose a valid theme TOML with a [colors] table and supported color names.",
            Self::UnsafeFile => "Choose a regular file in an accessible, unlinked folder.",
            Self::TooLarge => "This theme is too large (maximum 64 KiB).",
            Self::Limit => "The local theme limit was reached. Remove unused theme files and refresh.",
            Self::Io => "Theme file could not be read or saved. Check its location and try again.",
            Self::Cancelled => "Theme operation cancelled.",
            Self::TimedOut => "Theme operation timed out. The previous choice remains active.",
            Self::Worker => "Theme service is unavailable. Reopen Themes to retry.",
        }
    }
}
struct Request {
    id: u64,
    operation: ThemeOperation,
    cancelled: Arc<AtomicBool>,
    window: WindowId,
}
struct Completion {
    id: u64,
    result: Result<ThemeOutcome, ThemeIoError>,
}
struct Pending {
    id: u64,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    timed_out: bool,
}
pub struct ThemeLibrary {
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
impl ThemeLibrary {
    pub fn new(root: PathBuf, proxy: Option<EventProxy>) -> Self {
        let (sender, completed) = mpsc::sync_channel(1);
        Self {
            worker: BoundedWorker::new("theme-library", 1, move |request: Request| {
                let mut publisher = Publisher {
                    sender: &sender,
                    value: Some(Completion {
                        id: request.id,
                        result: Err(ThemeIoError::Worker),
                    }),
                    proxy: proxy.as_ref(),
                    window: request.window,
                };
                let result = perform(&root, request.operation, &request.cancelled);
                publisher.value = Some(Completion {
                    id: request.id,
                    result: if request.cancelled.load(Ordering::Acquire) {
                        Err(ThemeIoError::Cancelled)
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
        operation: ThemeOperation,
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
    pub fn take(&mut self, now: Instant) -> Option<Result<ThemeOutcome, ThemeIoError>> {
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
                return Some(Err(ThemeIoError::TimedOut));
            }
        }
        None
    }
}
impl Drop for ThemeLibrary {
    fn drop(&mut self) {
        self.cancel();
        self.worker.request_shutdown();
    }
}

fn check_cancel(cancelled: &AtomicBool) -> Result<(), ThemeIoError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ThemeIoError::Cancelled)
    } else {
        Ok(())
    }
}
fn map_io(error: private_fs::PrivateFsError) -> ThemeIoError {
    use private_fs::PrivateFsErrorCode as Code;
    match error.code() {
        Code::SourceTooLarge => ThemeIoError::TooLarge,
        Code::LinkRejected
        | Code::NotDirectory
        | Code::NotRegularFile
        | Code::SourceChanged
        | Code::InvalidRoot => ThemeIoError::UnsafeFile,
        _ => ThemeIoError::Io,
    }
}
fn read_theme(path: &Path) -> Result<ThemeSelection, ThemeIoError> {
    let bytes = private_fs::read_bounded_untrusted_regular(path, MAX_IMPORT_BYTES)
        .map_err(map_io)?
        .ok_or(ThemeIoError::Io)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| ThemeIoError::InvalidTheme)?;
    let theme = Theme::parse_for_gallery(text).map_err(|_| ThemeIoError::InvalidTheme)?;
    let name = text
        .lines()
        .take(4)
        .find_map(|line| line.strip_prefix("# Automexia theme: "))
        .filter(|name| theme_gallery::valid_name(name))
        .map(str::to_owned)
        .unwrap_or_else(|| local_name(path));
    Ok(ThemeSelection { name, theme })
}
fn local_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| theme_gallery::valid_name(s))
        .unwrap_or("Local theme")
        .replace('-', " ")
}
fn scan(root: &Path, cancelled: &AtomicBool) -> Result<ThemeOutcome, ThemeIoError> {
    let mut entries = theme_gallery::builtins();
    let directory = root.join("themes");
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ThemeOutcome::Inventory {
                entries,
                limited: false,
            })
        }
        Err(_) => return Err(ThemeIoError::Io),
        Ok(_) => private_fs::inspect_directory(&directory).map_err(map_io)?,
    }
    let mut paths = Vec::new();
    let mut limited = false;
    for (index, entry) in fs::read_dir(&directory)
        .map_err(|_| ThemeIoError::Io)?
        .enumerate()
    {
        check_cancel(cancelled)?;
        if index >= MAX_SCAN_ENTRIES {
            limited = true;
            break;
        }
        let entry = entry.map_err(|_| ThemeIoError::Io)?;
        if entry
            .path()
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    limited |= paths.len() > MAX_LOCAL_THEMES;
    let mut bytes = 0usize;
    for path in paths.into_iter().take(MAX_LOCAL_THEMES) {
        check_cancel(cancelled)?;
        let file_bytes = fs::symlink_metadata(&path).map_or(MAX_IMPORT_BYTES, |m| {
            usize::try_from(m.len())
                .unwrap_or(MAX_IMPORT_BYTES)
                .min(MAX_IMPORT_BYTES)
        });
        if bytes.saturating_add(file_bytes) > MAX_SCAN_BYTES {
            limited = true;
            break;
        }
        bytes = bytes.saturating_add(file_bytes);
        let name = local_name(&path);
        let id = format!(
            "local:{}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unreadable")
        );
        // Display and retain only bounded safe filenames, never arbitrary paths.
        if id.len() > theme_gallery::MAX_NAME_BYTES + 16
            || !theme_gallery::valid_name(&name)
        {
            limited = true;
            continue;
        }
        entries.push(match read_theme(&path) {
            Ok(selection) => ThemeDescriptor::parsed(
                id,
                selection.name,
                "Your local palette".into(),
                ThemeSource::Local,
                selection.theme,
            ),
            Err(error) => ThemeDescriptor::invalid(id, name, error.message().into()),
        });
    }
    Ok(ThemeOutcome::Inventory { entries, limited })
}
fn canonical(selection: &ThemeSelection) -> Result<String, ThemeIoError> {
    if !selection.is_valid() {
        return Err(ThemeIoError::InvalidTheme);
    }
    let text = format!(
        "# Automexia theme: {}\n{}",
        selection.name,
        selection
            .theme
            .to_toml()
            .map_err(|_| ThemeIoError::InvalidTheme)?
    );
    if text.len() > MAX_IMPORT_BYTES {
        return Err(ThemeIoError::TooLarge);
    }
    Ok(text)
}
fn write_file(
    destination: &Path,
    text: &str,
    replace: bool,
    cancelled: &AtomicBool,
) -> Result<bool, ThemeIoError> {
    check_cancel(cancelled)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(ThemeIoError::UnsafeFile)?;
    private_fs::inspect_directory(parent).map_err(map_io)?;
    private_fs::reject_link_or_non_file(destination).map_err(map_io)?;
    let mut stage = tempfile::Builder::new()
        .prefix(".theme-")
        .tempfile_in(parent)
        .map_err(|_| ThemeIoError::Io)?;
    private_fs::apply_private_file_permissions(stage.path()).map_err(map_io)?;
    stage
        .write_all(text.as_bytes())
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| ThemeIoError::Io)?;
    check_cancel(cancelled)?;
    let persisted = if replace {
        stage.persist(destination)
    } else {
        stage.persist_noclobber(destination)
    };
    match persisted {
        Ok(file) => {
            file.sync_all().map_err(|_| ThemeIoError::Io)?;
            private_fs::sync_directory(parent).map_err(map_io)?;
            Ok(true)
        }
        Err(error)
            if !replace && error.error.kind() == std::io::ErrorKind::AlreadyExists =>
        {
            Ok(false)
        }
        Err(_) => Err(ThemeIoError::Io),
    }
}
fn save_copy(
    root: &Path,
    selection: ThemeSelection,
    cancelled: &AtomicBool,
) -> Result<ThemeDescriptor, ThemeIoError> {
    let text = canonical(&selection)?;
    check_cancel(cancelled)?;
    // The application supplies its configuration root; user names never join it.
    if !root.exists() {
        fs::create_dir_all(root).map_err(|_| ThemeIoError::Io)?;
    }
    private_fs::inspect_directory(root).map_err(map_io)?;
    let directory = root.join("themes");
    private_fs::ensure_private_child_directory(&directory).map_err(map_io)?;
    if fs::read_dir(&directory)
        .map_err(|_| ThemeIoError::Io)?
        .take(MAX_LOCAL_THEMES)
        .count()
        >= MAX_LOCAL_THEMES
    {
        return Err(ThemeIoError::Limit);
    }
    let slug: String = selection
        .name
        .chars()
        .take(40)
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    for suffix in 1..=MAX_LOCAL_THEMES {
        check_cancel(cancelled)?;
        let filename = format!("user-{}-{suffix}.toml", slug.trim_matches('-'));
        if write_file(&directory.join(&filename), &text, false, cancelled)? {
            return Ok(ThemeDescriptor::parsed(
                format!("local:{filename}"),
                selection.name,
                "Your local palette".into(),
                ThemeSource::Local,
                selection.theme,
            ));
        }
    }
    Err(ThemeIoError::Limit)
}
fn perform(
    root: &Path,
    operation: ThemeOperation,
    cancelled: &AtomicBool,
) -> Result<ThemeOutcome, ThemeIoError> {
    check_cancel(cancelled)?;
    match operation {
        ThemeOperation::Scan => match scan(root, cancelled) {
            Err(error) if error != ThemeIoError::Cancelled => {
                let mut entries = theme_gallery::builtins();
                entries.push(ThemeDescriptor::invalid(
                    "local-library".into(),
                    "Local library unavailable".into(),
                    error.message().into(),
                ));
                Ok(ThemeOutcome::Inventory {
                    entries,
                    limited: false,
                })
            }
            result => result,
        },
        ThemeOperation::Import(path) => {
            let selection = read_theme(&path)?;
            save_copy(root, selection, cancelled).map(ThemeOutcome::Imported)
        }
        ThemeOperation::SaveCopy(selection) => {
            save_copy(root, selection, cancelled).map(ThemeOutcome::Saved)
        }
        ThemeOperation::Export {
            selection,
            destination,
        } => {
            let text = canonical(&selection)?;
            write_file(&destination, &text, true, cancelled)?;
            Ok(ThemeOutcome::Exported)
        }
    }
}

#[cfg(test)]
#[path = "theme_gallery_io_tests.rs"]
mod tests;
