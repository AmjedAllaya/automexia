//! Secure, asynchronous local-image quick look for terminal path hints.
//!
//! Terminal output is untrusted. A preview is therefore requested only after
//! an explicit modifier-hover or keyboard action, is restricted to local
//! raster files, and is decoded on one bounded worker away from the render
//! and PTY threads.

use crate::automexia::image::{
    decode_candidate, DecodeOutcome, ImagePreviewError as PreviewError, ThumbnailCache,
};
pub use crate::automexia::image::{
    has_supported_extension, path_token_at_line, ImageCandidate as PreviewCandidate,
};
use automexia_extension_runtime::{BoundedWorker, CompletionWake, RefreshSubmission};
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::{GraphicDataEntry, GraphicOverlay, Sugarloaf};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const HOVER_DELAY: Duration = Duration::from_millis(350);
const WORK_QUEUE_CAPACITY: usize = 16;
const PREVIEW_KEY_PREFIX: u64 = 0xFFFF_FFFE_0000_0000;
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
    owner_id: u64,
    candidate: PreviewCandidate,
    route_id: usize,
    generation: u64,
    current_generation: Arc<AtomicU64>,
    completion: Arc<Mutex<Option<PreviewResult>>>,
    wake: CompletionWake,
}

impl PreviewRequest {
    fn is_current(&self) -> bool {
        self.current_generation.load(Ordering::Acquire) == self.generation
    }
}

#[derive(Debug)]
struct PreviewResult {
    route_id: usize,
    generation: u64,
    result: Result<DecodeOutcome, PreviewError>,
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

#[derive(Debug)]
pub struct ImagePreview {
    owner_id: u64,
    generation: u64,
    current_generation: Arc<AtomicU64>,
    completion: Arc<Mutex<Option<PreviewResult>>>,
    pending: Option<PendingPreview>,
    ready: Option<ReadyPreview>,
    failed: Option<FailedPreview>,
}

impl Default for ImagePreview {
    fn default() -> Self {
        static NEXT_OWNER_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            owner_id: NEXT_OWNER_ID.fetch_add(1, Ordering::Relaxed),
            generation: 0,
            current_generation: Arc::new(AtomicU64::new(0)),
            completion: Arc::new(Mutex::new(None)),
            pending: None,
            ready: None,
            failed: None,
        }
    }
}

impl ImagePreview {
    fn advance_generation(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.current_generation
            .store(self.generation, Ordering::Release);
        self.generation
    }

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
        let generation = self.advance_generation();
        self.pending = Some(PendingPreview {
            candidate,
            route_id,
            generation,
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
        let generation = self.advance_generation();
        self.pending = Some(PendingPreview {
            candidate,
            route_id,
            generation,
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
        self.advance_generation();
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
        let completed = self.pending.as_ref().and_then(|pending| {
            take_result(&self.completion, pending.route_id, pending.generation)
        });
        if let Some(result) = completed {
            changed = true;
            if let Some(old) = self.ready.take() {
                remove_preview_overlay(sugarloaf, old.image_key);
            }
            match result.result {
                Ok(outcome) => {
                    let thumbnail = &outcome.thumbnail;
                    let image_key = preview_image_key(thumbnail.render_id);
                    sugarloaf.image_data.insert(
                        image_key,
                        GraphicDataEntry::from_shared_rgba(
                            thumbnail.render_id,
                            thumbnail.width,
                            thumbnail.height,
                            Arc::clone(&thumbnail.rgba),
                            thumbnail.decoded_at,
                        ),
                    );
                    let path = outcome.path;
                    let (anchor, candidate) = self
                        .pending
                        .as_ref()
                        .map(|pending| (pending.anchor, pending.candidate.clone()))
                        .unwrap_or_else(|| {
                            (
                                PreviewAnchor::default(),
                                PreviewCandidate {
                                    text: path.to_string_lossy().into_owned(),
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
                        path,
                        width: thumbnail.width,
                        height: thumbnail.height,
                        original_width: thumbnail.original_width,
                        original_height: thumbnail.original_height,
                        file_bytes: thumbnail.file_bytes,
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
            owner_id: self.owner_id,
            candidate: pending.candidate.clone(),
            route_id,
            generation,
            current_generation: Arc::clone(&self.current_generation),
            completion: Arc::clone(&self.completion),
            wake: completion(route_id),
        };
        match submit_request(request) {
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

#[derive(Default)]
struct PreviewWorkQueue {
    pending: VecDeque<PreviewRequest>,
}

impl PreviewWorkQueue {
    fn submit(&mut self, request: PreviewRequest) -> Option<bool> {
        if let Some(index) = self
            .pending
            .iter()
            .position(|queued| queued.owner_id == request.owner_id)
        {
            self.pending.remove(index);
            self.pending.push_back(request);
            return Some(true);
        }
        if self.pending.len() >= WORK_QUEUE_CAPACITY {
            return None;
        }
        self.pending.push_back(request);
        Some(false)
    }

    fn take(&mut self) -> Option<PreviewRequest> {
        self.pending.pop_front()
    }

    fn remove(&mut self, owner_id: u64, generation: u64) {
        self.pending.retain(|request| {
            request.owner_id != owner_id || request.generation != generation
        });
    }
}

fn worker() -> &'static BoundedWorker<()> {
    static WORKER: OnceLock<BoundedWorker<()>> = OnceLock::new();
    WORKER.get_or_init(|| {
        BoundedWorker::new("automexia-image-preview", 1, process_pending_requests)
    })
}

fn work_queue() -> &'static Mutex<PreviewWorkQueue> {
    static QUEUE: OnceLock<Mutex<PreviewWorkQueue>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(PreviewWorkQueue::default()))
}

fn thumbnail_cache() -> &'static Mutex<ThumbnailCache> {
    static CACHE: OnceLock<Mutex<ThumbnailCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ThumbnailCache::default()))
}

fn submit_request(request: PreviewRequest) -> RefreshSubmission {
    let owner_id = request.owner_id;
    let generation = request.generation;
    {
        let mut queue = work_queue()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if queue.submit(request).is_none() {
            return RefreshSubmission::Rejected;
        }
    }

    match worker().try_submit(()) {
        RefreshSubmission::Queued | RefreshSubmission::Busy => RefreshSubmission::Queued,
        failure => {
            work_queue()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(owner_id, generation);
            failure
        }
    }
}

fn process_pending_requests(_: ()) {
    loop {
        let request = work_queue()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(request) = request else {
            break;
        };
        if !request.is_current() {
            continue;
        }

        let result = {
            let mut cache = thumbnail_cache()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            decode_candidate(&request.candidate, &mut cache)
        };
        if !request.is_current() {
            continue;
        }

        let mut completion = request
            .completion
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *completion = Some(PreviewResult {
            route_id: request.route_id,
            generation: request.generation,
            result,
        });
        drop(completion);
        request.wake.wake();
    }
}

fn take_result(
    completion: &Mutex<Option<PreviewResult>>,
    route_id: usize,
    generation: u64,
) -> Option<PreviewResult> {
    let result = completion
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()?;
    (result.route_id == route_id && result.generation == generation).then_some(result)
}
fn preview_image_key(render_id: u64) -> u64 {
    PREVIEW_KEY_PREFIX | (render_id & 0xFFFF_FFFF)
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

    fn request(owner_id: u64, generation: u64, route_id: usize) -> PreviewRequest {
        let current_generation = Arc::new(AtomicU64::new(generation));
        PreviewRequest {
            owner_id,
            candidate: PreviewCandidate::new(
                format!("owner-{owner_id}.png"),
                Some(PathBuf::from("C:/preview")),
                None,
            )
            .unwrap(),
            route_id,
            generation,
            current_generation,
            completion: Arc::new(Mutex::new(None)),
            wake: Box::new(|| {}),
        }
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
    fn generations_cancel_obsolete_work_without_sharing_between_owners() {
        let mut first = ImagePreview::default();
        let second = ImagePreview::default();
        let first_generation = first.advance_generation();
        let obsolete = PreviewRequest {
            owner_id: first.owner_id,
            candidate: PreviewCandidate::new(
                "first.png",
                Some(PathBuf::from("C:/preview")),
                None,
            )
            .unwrap(),
            route_id: 1,
            generation: first_generation,
            current_generation: Arc::clone(&first.current_generation),
            completion: Arc::clone(&first.completion),
            wake: Box::new(|| {}),
        };
        assert!(obsolete.is_current());
        first.advance_generation();
        assert!(!obsolete.is_current());
        assert_eq!(second.current_generation.load(Ordering::Acquire), 0);
        assert_ne!(first.owner_id, second.owner_id);
    }

    #[test]
    fn work_queue_keeps_only_latest_request_per_owner_and_preserves_fairness() {
        let mut queue = PreviewWorkQueue::default();
        assert!(matches!(queue.submit(request(1, 1, 10)), Some(false)));
        assert!(matches!(queue.submit(request(2, 1, 20)), Some(false)));
        assert!(matches!(queue.submit(request(1, 2, 11)), Some(true)));
        assert_eq!(queue.pending.len(), 2);

        let other = queue.take().unwrap();
        let latest = queue.take().unwrap();
        assert_eq!((other.owner_id, other.route_id), (2, 20));
        assert_eq!((latest.owner_id, latest.generation), (1, 2));
    }

    #[test]
    fn work_queue_rejects_new_owners_at_its_hard_capacity() {
        let mut queue = PreviewWorkQueue::default();
        for owner in 0..WORK_QUEUE_CAPACITY as u64 {
            assert!(queue.submit(request(owner, 1, owner as usize)).is_some());
        }
        assert!(queue
            .submit(request(WORK_QUEUE_CAPACITY as u64 + 1, 1, 99))
            .is_none());
        assert_eq!(queue.pending.len(), WORK_QUEUE_CAPACITY);
    }

    #[test]
    fn completion_mailboxes_discard_stale_results_without_cross_owner_delivery() {
        let first = Arc::new(Mutex::new(Some(PreviewResult {
            route_id: 1,
            generation: 3,
            result: Err(PreviewError::NotFound),
        })));
        let second = Arc::new(Mutex::new(Some(PreviewResult {
            route_id: 1,
            generation: 4,
            result: Err(PreviewError::NotFound),
        })));

        assert!(take_result(&first, 1, 4).is_none());
        assert!(take_result(&first, 1, 3).is_none());
        let result = take_result(&second, 1, 4).unwrap();
        assert_eq!(result.generation, 4);
        assert!(take_result(&second, 1, 4).is_none());
    }

    #[test]
    fn stable_preview_keys_live_outside_terminal_protocol_namespaces() {
        let key = preview_image_key(9);
        assert!(key >= PREVIEW_KEY_PREFIX);
        assert_eq!(key, preview_image_key(9));
        assert_ne!(key, preview_image_key(10));
        assert_ne!(key, rio_backend::sugarloaf::kitty_image_key(9));
        assert_ne!(key, rio_backend::sugarloaf::atlas_image_key(9));
    }
}
