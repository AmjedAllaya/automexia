//! Secure, asynchronous local-image quick look for terminal path hints.
//!
//! Terminal output is untrusted. A preview is therefore requested only after
//! an explicit modifier-hover or keyboard action, is restricted to local
//! raster files, and is decoded on one bounded worker away from the render
//! and PTY threads.

use automexia_extension_runtime::{BoundedWorker, CompletionWake, RefreshSubmission};
use image_rs::ImageReader;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::{GraphicDataEntry, GraphicOverlay, Sugarloaf};
use std::collections::VecDeque;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const HOVER_DELAY: Duration = Duration::from_millis(350);
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_DECODED_PIXELS: u64 = 16_777_216;
const MAX_UPLOAD_WIDTH: u32 = 1_280;
const MAX_UPLOAD_HEIGHT: u32 = 960;
const RESULT_CAPACITY: usize = 8;
const PREVIEW_KEY_PREFIX: u64 = 0xFFFF_FFFE_0000_0000;

const EXTENSIONS: &[&str] = &[
    "bmp", "gif", "ico", "jpeg", "jpg", "pbm", "pgm", "png", "pnm", "ppm", "tif", "tiff",
    "webp",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewCandidate {
    pub text: String,
    pub cwd: Option<PathBuf>,
    pub wsl_distro: Option<String>,
}

impl PreviewCandidate {
    pub fn new(
        text: impl Into<String>,
        cwd: Option<PathBuf>,
        wsl_distro: Option<String>,
    ) -> Option<Self> {
        let text = trim_path_token(&text.into()).to_string();
        if is_non_local_token(&text)
            || text.chars().any(char::is_control)
            || !has_supported_extension(&text)
        {
            return None;
        }
        Some(Self {
            text,
            cwd,
            wsl_distro,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewAnchor {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
struct PendingPreview {
    candidate: PreviewCandidate,
    route_id: usize,
    generation: u64,
    anchor: PreviewAnchor,
    armed_at: Instant,
    submitted: bool,
    wake_scheduled: bool,
}

struct PreviewRequest {
    candidate: PreviewCandidate,
    route_id: usize,
    generation: u64,
    wake: CompletionWake,
}

#[derive(Debug)]
struct PreviewPixels {
    path: PathBuf,
    width: u32,
    height: u32,
    original_width: u32,
    original_height: u32,
    file_bytes: u64,
    rgba: Vec<u8>,
}

#[derive(Debug)]
struct PreviewResult {
    route_id: usize,
    generation: u64,
    result: Result<PreviewPixels, PreviewError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewError {
    NotLocal,
    NotFound,
    NotRegularFile,
    Symlink,
    TooLarge,
    DecodeFailed,
}

#[derive(Debug)]
struct ReadyPreview {
    route_id: usize,
    image_key: u64,
    candidate: PreviewCandidate,
    anchor: PreviewAnchor,
    path: PathBuf,
    width: u32,
    height: u32,
    original_width: u32,
    original_height: u32,
    file_bytes: u64,
}

#[derive(Debug)]
struct FailedPreview {
    route_id: usize,
    candidate: PreviewCandidate,
}

#[derive(Debug, Default)]
pub struct ImagePreview {
    generation: u64,
    pending: Option<PendingPreview>,
    ready: Option<ReadyPreview>,
    failed: Option<FailedPreview>,
}

impl ImagePreview {
    pub fn arm_hover(
        &mut self,
        candidate: Option<PreviewCandidate>,
        route_id: usize,
        anchor: PreviewAnchor,
        sugarloaf: &mut Sugarloaf,
    ) -> bool {
        if candidate.is_none() && self.is_idle() {
            return false;
        }
        if candidate
            .as_ref()
            .is_some_and(|candidate| self.has_target(candidate, route_id))
        {
            return false;
        }

        let dismissed = self.dismiss(sugarloaf);
        let Some(candidate) = candidate else {
            return dismissed;
        };
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(PendingPreview {
            candidate,
            route_id,
            generation: self.generation,
            anchor,
            armed_at: Instant::now(),
            submitted: false,
            wake_scheduled: false,
        });
        true
    }

    pub fn show_selection(
        &mut self,
        candidate: PreviewCandidate,
        route_id: usize,
        anchor: PreviewAnchor,
        sugarloaf: &mut Sugarloaf,
    ) {
        let _ = self.dismiss(sugarloaf);
        self.generation = self.generation.wrapping_add(1);
        self.pending = Some(PendingPreview {
            candidate,
            route_id,
            generation: self.generation,
            anchor,
            armed_at: Instant::now() - HOVER_DELAY,
            submitted: false,
            wake_scheduled: false,
        });
    }

    fn is_idle(&self) -> bool {
        self.pending.is_none() && self.ready.is_none() && self.failed.is_none()
    }

    fn has_target(&self, candidate: &PreviewCandidate, route_id: usize) -> bool {
        self.pending.as_ref().is_some_and(|pending| {
            pending.route_id == route_id && pending.candidate == *candidate
        }) || self.ready.as_ref().is_some_and(|ready| {
            ready.route_id == route_id && ready.candidate == *candidate
        }) || self.failed.as_ref().is_some_and(|failed| {
            failed.route_id == route_id && failed.candidate == *candidate
        })
    }

    pub fn dismiss(&mut self, sugarloaf: &mut Sugarloaf) -> bool {
        if self.is_idle() {
            return false;
        }
        self.generation = self.generation.wrapping_add(1);
        self.pending = None;
        self.failed = None;
        if let Some(ready) = self.ready.take() {
            remove_preview_overlay(sugarloaf, ready.image_key);
        }
        true
    }

    pub fn is_visible(&self) -> bool {
        self.ready.is_some()
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub fn native_test_state(
        &self,
        sugarloaf: &Sugarloaf,
    ) -> (bool, bool, Option<[u32; 2]>) {
        let Some(ready) = self.ready.as_ref() else {
            return (false, false, None);
        };
        let overlay_present = sugarloaf
            .image_overlays
            .values()
            .flatten()
            .any(|overlay| overlay.image_id == ready.image_key);
        (true, overlay_present, Some([ready.width, ready.height]))
    }

    pub fn take_wake_in(&mut self) -> Option<Duration> {
        let pending = self.pending.as_mut()?;
        if pending.submitted || pending.wake_scheduled {
            return None;
        }
        pending.wake_scheduled = true;
        Some(HOVER_DELAY.saturating_sub(pending.armed_at.elapsed()))
    }

    pub fn poll_and_submit(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        completion: impl FnOnce(usize) -> CompletionWake,
    ) -> bool {
        let mut changed = false;
        let completed = self
            .pending
            .as_ref()
            .and_then(|pending| take_result(pending.route_id, pending.generation));
        if let Some(result) = completed {
            changed = true;
            if let Some(old) = self.ready.take() {
                remove_preview_overlay(sugarloaf, old.image_key);
            }
            match result.result {
                Ok(pixels) => {
                    let image_key = preview_image_key(result.route_id, result.generation);
                    let data = rio_backend::sugarloaf::GraphicData {
                        id: rio_backend::sugarloaf::GraphicId::new(image_key),
                        width: pixels.width as usize,
                        height: pixels.height as usize,
                        color_type: rio_backend::sugarloaf::ColorType::Rgba,
                        pixels: pixels.rgba,
                        is_opaque: false,
                        resize: None,
                        display_width: None,
                        display_height: None,
                        transmit_time: std::time::Instant::now(),
                    };
                    sugarloaf
                        .image_data
                        .insert(image_key, GraphicDataEntry::from_graphic_data(data));
                    let (anchor, candidate) = self
                        .pending
                        .as_ref()
                        .map(|pending| (pending.anchor, pending.candidate.clone()))
                        .unwrap_or_else(|| {
                            (
                                PreviewAnchor::default(),
                                PreviewCandidate {
                                    text: pixels.path.to_string_lossy().into_owned(),
                                    cwd: None,
                                    wsl_distro: None,
                                },
                            )
                        });
                    self.ready = Some(ReadyPreview {
                        route_id: result.route_id,
                        image_key,
                        candidate,
                        anchor,
                        path: pixels.path,
                        width: pixels.width,
                        height: pixels.height,
                        original_width: pixels.original_width,
                        original_height: pixels.original_height,
                        file_bytes: pixels.file_bytes,
                    });
                    self.pending = None;
                    self.failed = None;
                }
                Err(error) => {
                    tracing::debug!(
                        ?error,
                        "local image quick look rejected a candidate"
                    );
                    self.failed = self.pending.as_ref().map(|pending| FailedPreview {
                        route_id: pending.route_id,
                        candidate: pending.candidate.clone(),
                    });
                    self.pending = None;
                }
            }
        }

        let Some(pending) = self.pending.as_mut() else {
            return changed;
        };
        if pending.submitted || pending.armed_at.elapsed() < HOVER_DELAY {
            return changed;
        }
        let route_id = pending.route_id;
        let generation = pending.generation;
        let request = PreviewRequest {
            candidate: pending.candidate.clone(),
            route_id,
            generation,
            wake: completion(route_id),
        };
        match worker().try_submit(request) {
            RefreshSubmission::Queued => pending.submitted = true,
            RefreshSubmission::Busy => {
                pending.armed_at = Instant::now();
                pending.wake_scheduled = false;
            }
            RefreshSubmission::Rejected | RefreshSubmission::Unavailable => {
                tracing::debug!("local image quick look worker is unavailable");
                self.failed = Some(FailedPreview {
                    route_id,
                    candidate: pending.candidate.clone(),
                });
                self.pending = None;
            }
        }
        true
    }

    pub fn draw(
        &self,
        sugarloaf: &mut Sugarloaf,
        route_id: usize,
        rich_text_id: usize,
        pane: [f32; 4],
    ) {
        let Some(ready) = self.ready.as_ref() else {
            return;
        };
        for overlays in sugarloaf.image_overlays.values_mut() {
            overlays.retain(|overlay| overlay.image_id != ready.image_key);
        }
        if ready.route_id != route_id || pane[2] <= 0.0 || pane[3] <= 0.0 {
            return;
        }
        let scale = sugarloaf.scale_factor().max(1.0);
        let Some(geometry) = preview_geometry(
            pane,
            ready.anchor,
            ready.width as f32,
            ready.height as f32,
            scale,
        ) else {
            return;
        };
        let card = geometry.card.map(|value| value / scale);
        let image = geometry.image;
        sugarloaf.rounded_rect(
            None,
            card[0] + 4.0,
            card[1] + 6.0,
            card[2],
            card[3],
            [0.0, 0.0, 0.0, 0.42],
            0.0,
            12.0,
            34,
        );
        sugarloaf.rounded_rect(
            None,
            card[0],
            card[1],
            card[2],
            card[3],
            [0.008, 0.027, 0.050, 0.985],
            0.0,
            12.0,
            35,
        );
        sugarloaf.line(
            card[0],
            card[1],
            card[0] + card[2],
            card[1],
            1.25,
            0.0,
            [0.063, 0.72, 0.96, 0.92],
            36,
        );
        sugarloaf.push_image_overlay(
            rich_text_id,
            GraphicOverlay {
                image_id: ready.image_key,
                x: image[0],
                y: image[1],
                width: image[2],
                height: image[3],
                z_index: 2_000,
                source_rect: GraphicOverlay::FULL_SOURCE_RECT,
            },
        );

        let title = ready
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Image preview");
        let title = elide_middle(title, 48);
        let details = format!(
            "{}x{}  {}",
            ready.original_width,
            ready.original_height,
            human_bytes(ready.file_bytes)
        );
        let title_opts = DrawOpts {
            font_size: 13.0,
            color: [220, 239, 250, 255],
            bold: true,
            ..DrawOpts::default()
        };
        let detail_opts = DrawOpts {
            font_size: 10.5,
            color: [116, 155, 180, 255],
            ..DrawOpts::default()
        };
        sugarloaf
            .text_mut()
            .draw(card[0] + 12.0, card[1] + 8.0, &title, &title_opts);
        let detail_width = sugarloaf.text_mut().measure(&details, &detail_opts);
        sugarloaf.text_mut().draw(
            (card[0] + card[2] - detail_width - 12.0).max(card[0] + 12.0),
            card[1] + 10.0,
            &details,
            &detail_opts,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PreviewGeometry {
    card: [f32; 4],
    image: [f32; 4],
}

fn preview_geometry(
    pane: [f32; 4],
    anchor: PreviewAnchor,
    width: f32,
    height: f32,
    scale: f32,
) -> Option<PreviewGeometry> {
    if pane[2] < 120.0 * scale || pane[3] < 120.0 * scale {
        return None;
    }
    let margin = 12.0 * scale;
    let title = 34.0 * scale;
    let padding = 10.0 * scale;
    let max_w = (pane[2] * 0.48)
        .clamp(180.0 * scale, 520.0 * scale)
        .min((pane[2] - margin * 2.0).max(1.0));
    let max_h = (pane[3] * 0.62)
        .clamp(120.0 * scale, 420.0 * scale)
        .min((pane[3] - title - padding * 2.0 - margin * 2.0).max(1.0));
    let ratio = (max_w / width.max(1.0))
        .min(max_h / height.max(1.0))
        .min(1.0);
    let image_w = (width * ratio).max(1.0);
    let image_h = (height * ratio).max(1.0);
    let card_w = image_w + padding * 2.0;
    let card_h = image_h + title + padding;
    let right = pane[0] + pane[2] - margin;
    let bottom = pane[1] + pane[3] - margin;
    let mut x = anchor.x + 18.0 * scale;
    if x + card_w > right {
        x = (anchor.x - card_w - 18.0 * scale).max(pane[0] + margin);
    }
    let mut y = anchor.y + 18.0 * scale;
    if y + card_h > bottom {
        y = (anchor.y - card_h - 18.0 * scale).max(pane[1] + margin);
    }
    x = x.clamp(pane[0] + margin, (right - card_w).max(pane[0] + margin));
    y = y.clamp(pane[1] + margin, (bottom - card_h).max(pane[1] + margin));
    Some(PreviewGeometry {
        card: [x, y, card_w, card_h],
        image: [x + padding, y + title, image_w, image_h],
    })
}

fn worker() -> &'static BoundedWorker<PreviewRequest> {
    static WORKER: OnceLock<BoundedWorker<PreviewRequest>> = OnceLock::new();
    WORKER
        .get_or_init(|| BoundedWorker::new("automexia-image-preview", 2, process_request))
}

fn results() -> &'static Mutex<VecDeque<PreviewResult>> {
    static RESULTS: OnceLock<Mutex<VecDeque<PreviewResult>>> = OnceLock::new();
    RESULTS.get_or_init(|| Mutex::new(VecDeque::with_capacity(RESULT_CAPACITY)))
}

fn process_request(request: PreviewRequest) {
    let result = decode_candidate(&request.candidate);
    let mut results = results()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if results.len() >= RESULT_CAPACITY {
        results.pop_front();
    }
    results.push_back(PreviewResult {
        route_id: request.route_id,
        generation: request.generation,
        result,
    });
    drop(results);
    request.wake.wake();
}

fn take_result(route_id: usize, generation: u64) -> Option<PreviewResult> {
    let mut results = results()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    results
        .retain(|result| result.route_id != route_id || result.generation >= generation);
    let index = results.iter().position(|result| {
        result.route_id == route_id && result.generation == generation
    })?;
    results.remove(index)
}

fn decode_candidate(candidate: &PreviewCandidate) -> Result<PreviewPixels, PreviewError> {
    let path = resolve_candidate_path(candidate)?;
    let (file, metadata) = open_regular_file_without_links(&path)?;
    if metadata.len() == 0 || metadata.len() > MAX_FILE_BYTES {
        return Err(PreviewError::TooLarge);
    }

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PreviewError::DecodeFailed)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(PreviewError::TooLarge);
    }

    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| PreviewError::DecodeFailed)?;
    let mut limits = image_rs::Limits::default();
    limits.max_image_width = Some(4_096);
    limits.max_image_height = Some(4_096);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| PreviewError::DecodeFailed)?;
    let original_width = image.width();
    let original_height = image.height();
    if u64::from(original_width).saturating_mul(u64::from(original_height))
        > MAX_DECODED_PIXELS
    {
        return Err(PreviewError::TooLarge);
    }
    let image =
        if original_width > MAX_UPLOAD_WIDTH || original_height > MAX_UPLOAD_HEIGHT {
            image.thumbnail(MAX_UPLOAD_WIDTH, MAX_UPLOAD_HEIGHT)
        } else {
            image
        }
        .into_rgba8();
    Ok(PreviewPixels {
        path,
        width: image.width(),
        height: image.height(),
        original_width,
        original_height,
        file_bytes: metadata.len(),
        rgba: image.into_raw(),
    })
}

fn open_regular_file_without_links(
    path: &Path,
) -> Result<(std::fs::File, std::fs::Metadata), PreviewError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| PreviewError::NotFound)?;
    if metadata.file_type().is_symlink() {
        return Err(PreviewError::Symlink);
    }
    if !metadata.is_file() {
        return Err(PreviewError::NotRegularFile);
    }

    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_FLAG_OPEN_REPARSE_POINT keeps a last-moment symlink/junction
        // swap from being followed between metadata validation and open.
        options.custom_flags(0x0020_0000);
    }
    let file = options.open(path).map_err(|_| PreviewError::NotFound)?;
    let opened_metadata = file.metadata().map_err(|_| PreviewError::NotFound)?;
    if opened_metadata.file_type().is_symlink() {
        return Err(PreviewError::Symlink);
    }
    if !opened_metadata.is_file() {
        return Err(PreviewError::NotRegularFile);
    }
    Ok((file, opened_metadata))
}

fn is_non_local_token(token: &str) -> bool {
    token.contains("://")
        || token
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
}

fn resolve_candidate_path(candidate: &PreviewCandidate) -> Result<PathBuf, PreviewError> {
    let token = trim_path_token(&candidate.text);
    if is_non_local_token(token) || token.chars().any(char::is_control) {
        return Err(PreviewError::NotLocal);
    }
    #[cfg(windows)]
    if token.starts_with("\\\\") || token.starts_with("//") {
        return Err(PreviewError::NotLocal);
    }

    let raw = PathBuf::from(token);
    let path = if raw.is_absolute() {
        raw
    } else {
        candidate
            .cwd
            .as_ref()
            .ok_or(PreviewError::NotLocal)?
            .join(raw)
    };
    #[cfg(windows)]
    let path =
        if candidate.wsl_distro.is_some() || path.to_string_lossy().starts_with('/') {
            map_wsl_path(&path, candidate.wsl_distro.as_deref())?
        } else {
            path
        };
    Ok(path)
}

#[cfg(windows)]
fn map_wsl_path(path: &Path, distro: Option<&str>) -> Result<PathBuf, PreviewError> {
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(rest) = text.strip_prefix("/mnt/") {
        let mut parts = rest.splitn(2, '/');
        let drive = parts.next().unwrap_or_default();
        if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
            let tail = parts.next().unwrap_or_default().replace('/', "\\");
            return Ok(PathBuf::from(format!(
                "{}:\\{}",
                drive.to_ascii_uppercase(),
                tail
            )));
        }
    }
    let distro = distro
        .filter(|value| {
            !value.is_empty()
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                })
        })
        .ok_or(PreviewError::NotLocal)?;
    Ok(PathBuf::from(format!(
        "\\\\wsl.localhost\\{}\\{}",
        distro,
        text.trim_start_matches('/').replace('/', "\\")
    )))
}

pub fn has_supported_extension(text: &str) -> bool {
    Path::new(trim_path_token(text))
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

/// Return the path-shaped token under `column`. Quoted names keep spaces;
/// unquoted names follow normal shell whitespace boundaries. The filesystem
/// is deliberately not touched here because this runs on pointer movement.
pub fn path_token_at_line(line: &str, column: usize) -> Option<String> {
    let chars = line.chars().collect::<Vec<_>>();
    if chars.is_empty() || column >= chars.len() {
        return None;
    }

    for quote in ['\'', '"', '`'] {
        let left = chars[..=column]
            .iter()
            .rposition(|character| *character == quote);
        let right = chars[column..]
            .iter()
            .position(|character| *character == quote)
            .map(|offset| column + offset);
        if let (Some(left), Some(right)) = (left, right) {
            if left < right {
                let candidate = chars[left + 1..right].iter().collect::<String>();
                if has_supported_extension(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }

    let is_boundary = |character: char| {
        character.is_whitespace() || matches!(character, '|' | '<' | '>')
    };
    let mut start = column;
    while start > 0 && !is_boundary(chars[start - 1]) {
        start -= 1;
    }
    let mut end = column + 1;
    while end < chars.len() && !is_boundary(chars[end]) {
        end += 1;
    }
    let candidate =
        trim_path_token(&chars[start..end].iter().collect::<String>()).to_string();
    has_supported_extension(&candidate).then_some(candidate)
}

fn trim_path_token(text: &str) -> &str {
    text.trim().trim_matches(|character| {
        matches!(
            character,
            '\'' | '"' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
        )
    })
}

fn preview_image_key(route_id: usize, generation: u64) -> u64 {
    PREVIEW_KEY_PREFIX | ((route_id as u64 & 0xFFFF) << 16) | (generation & 0xFFFF)
}

fn remove_preview_overlay(sugarloaf: &mut Sugarloaf, image_key: u64) {
    for overlays in sugarloaf.image_overlays.values_mut() {
        overlays.retain(|overlay| overlay.image_id != image_key);
    }
    sugarloaf.remove_image(image_key);
}

fn human_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn elide_middle(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    let left = (max_chars.saturating_sub(1)) / 2;
    let right = max_chars.saturating_sub(left + 1);
    chars[..left]
        .iter()
        .chain(std::iter::once(&'…'))
        .chain(chars[chars.len() - right..].iter())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_extensions_are_case_insensitive_and_svg_is_rejected() {
        assert!(has_supported_extension("photo.PNG"));
        assert!(has_supported_extension("'folder/a picture.webp'"));
        assert!(!has_supported_extension("script.svg"));
        assert!(!has_supported_extension("https://example.com/a.png?x=1"));
    }

    #[test]
    fn geometry_stays_inside_tiny_and_large_panes() {
        for pane in [[0.0, 0.0, 260.0, 180.0], [40.0, 80.0, 7680.0, 4320.0]] {
            for anchor in [
                PreviewAnchor {
                    x: pane[0],
                    y: pane[1],
                },
                PreviewAnchor {
                    x: pane[0] + pane[2],
                    y: pane[1] + pane[3],
                },
            ] {
                let geometry =
                    preview_geometry(pane, anchor, 1200.0, 800.0, 1.0).unwrap();
                assert!(geometry.card[0] >= pane[0]);
                assert!(geometry.card[1] >= pane[1]);
                assert!(geometry.card[0] + geometry.card[2] <= pane[0] + pane[2] + 0.01);
                assert!(geometry.card[1] + geometry.card[3] <= pane[1] + pane[3] + 0.01);
            }
        }
    }

    #[test]
    fn geometry_hides_on_physically_unusable_panes() {
        assert!(preview_geometry(
            [0.0, 0.0, 80.0, 60.0],
            PreviewAnchor { x: 40.0, y: 30.0 },
            640.0,
            480.0,
            1.0,
        )
        .is_none());
    }

    #[test]
    fn idle_pointer_motion_is_a_noop_and_failed_targets_are_debounced() {
        let candidate =
            PreviewCandidate::new("broken.png", Some(PathBuf::from("C:/preview")), None)
                .unwrap();
        let mut preview = ImagePreview::default();
        assert!(preview.is_idle());
        assert!(!preview.has_target(&candidate, 7));

        preview.failed = Some(FailedPreview {
            route_id: 7,
            candidate: candidate.clone(),
        });
        assert!(!preview.is_idle());
        assert!(preview.has_target(&candidate, 7));
        assert!(!preview.has_target(&candidate, 8));
    }

    #[test]
    fn candidates_reject_remote_and_control_character_inputs() {
        assert!(PreviewCandidate::new("https://example.com/a.png", None, None).is_none());
        assert!(PreviewCandidate::new("file:///tmp/a.png", None, None).is_none());
        assert!(PreviewCandidate::new("FILE:C:\\temp\\a.png", None, None).is_none());
        assert!(PreviewCandidate::new("image.png\nnext-command", None, None).is_none());
        assert!(PreviewCandidate::new("image.png\u{1b}", None, None).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn wsl_paths_map_only_through_validated_local_routes() {
        assert_eq!(
            map_wsl_path(Path::new("/mnt/d/work/image.png"), Some("Ubuntu")).unwrap(),
            PathBuf::from(r"D:\work\image.png")
        );
        assert_eq!(
            map_wsl_path(Path::new("/home/user/image.png"), Some("Ubuntu-24.04"))
                .unwrap(),
            PathBuf::from(r"\\wsl.localhost\Ubuntu-24.04\home\user\image.png")
        );
        assert_eq!(
            map_wsl_path(Path::new("/home/user/image.png"), Some("bad\\name")),
            Err(PreviewError::NotLocal)
        );
    }
    #[test]
    fn decoder_rejects_oversized_files_before_image_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.png");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();
        let candidate =
            PreviewCandidate::new(path.to_string_lossy(), None, None).unwrap();
        assert!(matches!(
            decode_candidate(&candidate),
            Err(PreviewError::TooLarge)
        ));
    }

    #[test]
    fn decoder_keeps_small_png_at_native_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.png");
        image_rs::RgbaImage::from_pixel(4, 3, image_rs::Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        let candidate =
            PreviewCandidate::new(path.to_string_lossy(), None, None).unwrap();
        let decoded = decode_candidate(&candidate).unwrap();
        assert_eq!((decoded.width, decoded.height), (4, 3));
        assert_eq!(decoded.rgba.len(), 4 * 3 * 4);
    }

    #[test]
    fn stale_results_are_discarded_by_generation() {
        let mut queue = results()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.clear();
        queue.push_back(PreviewResult {
            route_id: 1,
            generation: 3,
            result: Err(PreviewError::NotFound),
        });
        queue.push_back(PreviewResult {
            route_id: 1,
            generation: 4,
            result: Err(PreviewError::NotFound),
        });
        drop(queue);
        let result = take_result(1, 4).unwrap();
        assert_eq!(result.generation, 4);
        assert!(take_result(1, 3).is_none());
    }

    #[test]
    fn preview_keys_live_outside_terminal_protocol_namespaces() {
        let key = preview_image_key(2, 9);
        assert!(key >= PREVIEW_KEY_PREFIX);
        assert_ne!(key, rio_backend::sugarloaf::kitty_image_key(9));
        assert_ne!(key, rio_backend::sugarloaf::atlas_image_key(9));
    }

    #[test]
    fn path_token_supports_bare_paths_and_quoted_spaces_without_io() {
        assert_eq!(
            path_token_at_line("  photo.png  ", 4).as_deref(),
            Some("photo.png")
        );
        assert_eq!(
            path_token_at_line("open 'folder/my image.JPEG' now", 16).as_deref(),
            Some("folder/my image.JPEG")
        );
        assert!(path_token_at_line("README.md", 4).is_none());
    }
}
