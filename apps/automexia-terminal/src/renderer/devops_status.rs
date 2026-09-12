//! Renderer-owned operational context for semantic prompt rows.
//!
//! Context is attached to every prompt generation so PTY output, prompt
//! editing, scrollback and resize/reflow cannot erase it. Discovery is
//! asynchronous and local-only; this renderer never contacts a daemon,
//! cluster or cloud API.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use rio_backend::config::colors::Colors;
use rio_backend::sugarloaf::text::DrawOpts;
use rio_backend::sugarloaf::Sugarloaf;

use automexia_extension_api::{ContextContribution, IconKind, SegmentRole, SessionFacts};
use automexia_ui_model::{self, IconOptics, Segment};

use crate::automexia::runtime;
use crate::automexia::ui::{PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY};

#[cfg(feature = "native-gui-test-hooks")]
pub(crate) type NativePromptContextPaint = (Option<u64>, u64, [f32; 4]);

pub(crate) const LIVE_REFRESH_MILLIS: u64 = 3_000;
const REFRESH_INTERVAL: Duration = Duration::from_millis(LIVE_REFRESH_MILLIS);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const ORDER: u8 = 19;
const PROMPT_TAG_FONT_ROW_RATIO: f32 = 0.62;
const PROMPT_TAG_MAX_FONT_SIZE: f32 = 14.0;
const PROMPT_TAG_MIN_FONT_SIZE: f32 = 4.0;
const PROMPT_TAG_LEFT_INSET: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PromptTagMetrics {
    pub font_size: f32,
    pub icon_size: f32,
    pub icon_slot: f32,
    pub height: f32,
    pub padding_x: f32,
    pub icon_gap: f32,
    pub tag_gap: f32,
    pub radius: f32,
}

pub(super) fn prompt_tag_metrics(row_height: f32) -> PromptTagMetrics {
    let row_height = row_height.max(1.0);
    let font_size = (row_height * PROMPT_TAG_FONT_ROW_RATIO)
        .clamp(PROMPT_TAG_MIN_FONT_SIZE, PROMPT_TAG_MAX_FONT_SIZE)
        .min(row_height);
    let vertical_padding = (row_height * 0.08).clamp(1.0, 2.0);
    let height = (font_size + vertical_padding * 2.0).min(row_height);
    let padding_x = (font_size * 0.35).clamp(3.0, 6.0);
    let icon_gap = (font_size * 0.28).clamp(2.0, 5.0);
    let tag_gap = (font_size * 0.35).clamp(3.0, 6.0);
    let icon_size = font_size * 1.05;
    let icon_slot = font_size * 1.20;
    let radius = (height * 0.22).clamp(1.0, 5.0).min(height * 0.5);
    PromptTagMetrics {
        font_size,
        icon_size,
        icon_slot,
        height,
        padding_x,
        icon_gap,
        tag_gap,
        radius,
    }
}

struct PromptSnapshot {
    session_id: usize,
    generation: Option<u64>,
    key: u64,
    segments: Vec<Segment>,
}

struct ActivePrompt {
    session_id: usize,
    generation: Option<u64>,
    key: u64,
    segments: Vec<Segment>,
    segments_revision: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotCandidate {
    Unchanged,
    StaleSession,
    Current,
}

#[derive(Default)]
pub struct DevOpsStatus {
    contribution: Option<ContextContribution>,
    /// Materialized segment labels shared by live and historical prompt rows.
    /// Rebuilt only when session facts or the async discovery revision change.
    live_segments: Vec<Segment>,
    live_segments_session: Option<SessionFacts>,
    live_segments_snapshot_revision: u32,
    live_segments_revision: u32,
    prompt_history: VecDeque<PromptSnapshot>,
    active_prompt: Option<ActivePrompt>,
    last_refresh_request: Option<Instant>,
    last_session: Option<SessionFacts>,
    observed_global_generation: u32,
    snapshot_revision: u32,
    refresh_pending: bool,
    request_in_flight: bool,
    #[cfg(feature = "native-gui-test-hooks")]
    native_prompt_paints: std::cell::RefCell<Vec<NativePromptContextPaint>>,
}

impl DevOpsStatus {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_context(&self) -> Option<(usize, Vec<String>)> {
        let session_id = self.live_segments_session.as_ref()?.session_id;
        Some((
            session_id,
            self.live_segments
                .iter()
                .map(|segment| segment.value.clone())
                .collect(),
        ))
    }

    #[cfg(feature = "native-gui-test-hooks")]
    pub(crate) fn native_test_prompt_paints(&self) -> Vec<NativePromptContextPaint> {
        self.native_prompt_paints.borrow().clone()
    }

    /// Keep asynchronous context discovery warm for semantic prompt rows.
    ///
    /// The former persistent context/status header duplicated the same facts
    /// above every command. The chrome row is now action-focused and rendered
    /// by `Island`, while this method keeps prompt metadata current without
    /// submitting any global-header drawing primitives.
    pub fn refresh_session_context<F>(
        &mut self,
        session: &SessionFacts,
        completion: F,
    ) -> bool
    where
        F: FnOnce() -> runtime::DevOpsRefreshCompletion,
    {
        self.request_refresh_if_needed(session, false, completion);
        self.sync_cached_snapshot(session);
        self.ensure_live_segments(session);
        self.refresh_pending
    }

    /// Draw a renderer-owned context row above every semantic shell prompt.
    ///
    /// The shell reserves a blank `Prompt` row, a complete-path continuation,
    /// and a short editable continuation. Context is therefore durable grid
    /// metadata rather than prompt text: typing cannot erase it, scrollback
    /// retains it, and resize/reflow resolves its current geometry each frame.
    pub(super) fn prepare_prompt_rows(
        &mut self,
        session: &SessionFacts,
        prompt_active: bool,
        historical_anchors: &[PromptAnchor],
        live_anchor: Option<PromptAnchor>,
    ) -> bool {
        #[cfg(feature = "native-gui-test-hooks")]
        self.native_prompt_paints.borrow_mut().clear();
        self.ensure_live_segments(session);
        let new_prompt = self.sync_active_prompt(
            session,
            prompt_active,
            live_anchor,
            historical_anchors,
        );

        if prompt_active {
            if let Some(anchor) = live_anchor {
                let segments_revision = self.live_segments_revision;
                match self.active_prompt.as_mut() {
                    Some(active)
                        if active.session_id == session.session_id
                            && same_prompt_identity(
                                active.generation,
                                active.key,
                                anchor.generation,
                                anchor.key,
                            ) =>
                    {
                        active.generation = anchor.generation;
                        active.key = anchor.key;
                        if active.segments_revision != segments_revision {
                            active.segments.clone_from(&self.live_segments);
                            active.segments_revision = segments_revision;
                        }
                    }
                    _ => {
                        self.active_prompt = Some(ActivePrompt {
                            session_id: session.session_id,
                            generation: anchor.generation,
                            key: anchor.key,
                            segments: self.live_segments.clone(),
                            segments_revision,
                        });
                    }
                }
            }
        }

        new_prompt
    }

    /// Queue fresh external context for a newly emitted prompt. This is kept
    /// separate from row drawing so renderer geometry stays independent from
    /// the event-loop wake-up mechanism.
    pub fn request_prompt_refresh<F>(&mut self, session: &SessionFacts, completion: F)
    where
        F: FnOnce() -> runtime::DevOpsRefreshCompletion,
    {
        self.request_refresh_if_needed(session, true, completion);
        self.sync_cached_snapshot(session);
        self.ensure_live_segments(session);
    }

    /// Keep an inactive but visible pane's operational snapshot current.
    ///
    /// This small preparation entry point performs asynchronous cache
    /// synchronization without painting global chrome.
    pub fn refresh_visible_session<F>(
        &mut self,
        session: &SessionFacts,
        completion: F,
    ) -> bool
    where
        F: FnOnce() -> runtime::DevOpsRefreshCompletion,
    {
        self.request_refresh_if_needed(session, false, completion);
        self.sync_cached_snapshot(session);
        self.ensure_live_segments(session);
        self.refresh_pending
    }

    fn sync_active_prompt(
        &mut self,
        session: &SessionFacts,
        prompt_active: bool,
        newest: Option<PromptAnchor>,
        historical_anchors: &[PromptAnchor],
    ) -> bool {
        if !prompt_active {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous);
            }
            return false;
        }

        let Some(anchor) = newest else {
            return false;
        };
        let Some(active) = self.active_prompt.as_ref() else {
            self.active_prompt = Some(ActivePrompt {
                session_id: session.session_id,
                generation: anchor.generation,
                key: anchor.key,
                segments: self.live_segments.clone(),
                segments_revision: self.live_segments_revision,
            });
            return true;
        };

        if active.session_id == session.session_id
            && same_prompt_identity(
                active.generation,
                active.key,
                anchor.generation,
                anchor.key,
            )
        {
            if let Some(active) = self.active_prompt.as_mut() {
                active.generation = anchor.generation;
                active.key = anchor.key;
            }
            return false;
        }

        let generation_proves_new_prompt = active.session_id == session.session_id
            && active.generation.is_some()
            && anchor.generation.is_some();
        let previous_still_visible = historical_anchors.iter().any(|candidate| {
            same_prompt_identity(
                active.generation,
                active.key,
                candidate.generation,
                candidate.key,
            )
        });
        if active.session_id != session.session_id
            || generation_proves_new_prompt
            || previous_still_visible
        {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous);
            }
            self.active_prompt = Some(ActivePrompt {
                session_id: session.session_id,
                generation: anchor.generation,
                key: anchor.key,
                segments: self.live_segments.clone(),
                segments_revision: self.live_segments_revision,
            });
            return true;
        }

        // Legacy integrations without `aid` cannot distinguish a reflowed row
        // from a new row by identity alone. If the old key vanished, preserve
        // the live prompt and adopt its recomputed geometry key.
        if let Some(active) = self.active_prompt.as_mut() {
            active.generation = anchor.generation;
            active.key = anchor.key;
        }
        false
    }

    fn cached_segments(
        &self,
        session_id: usize,
        anchor: &PromptAnchor,
    ) -> Option<&[Segment]> {
        self.prompt_history
            .iter()
            .rev()
            .find(|entry| {
                entry.session_id == session_id
                    && same_prompt_identity(
                        entry.generation,
                        entry.key,
                        anchor.generation,
                        anchor.key,
                    )
            })
            .map(|entry| entry.segments.as_slice())
    }

    pub(super) fn segments_for_prompt(
        &self,
        session_id: usize,
        anchor: &PromptAnchor,
    ) -> &[Segment] {
        if let Some(active) = self.active_prompt.as_ref().filter(|active| {
            active.session_id == session_id
                && same_prompt_identity(
                    active.generation,
                    active.key,
                    anchor.generation,
                    anchor.key,
                )
        }) {
            return &active.segments;
        }
        self.cached_segments(session_id, anchor)
            .unwrap_or(&self.live_segments)
    }

    pub(super) fn draw_prompt_fragment(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchor: &PromptAnchor,
        session_id: usize,
        fragment: &crate::automexia::ui::command_info::Fragment,
    ) {
        let Some(segment) = self
            .segments_for_prompt(session_id, anchor)
            .get(fragment.item)
        else {
            return;
        };
        let Some(text) = segment.value.get(fragment.bytes.clone()) else {
            return;
        };
        let metrics = prompt_tag_metrics(anchor.height);
        let top_inset = automexia_ui_model::prompt_context_top_inset(
            anchor.height,
            metrics.height,
            true,
        )
        .unwrap_or(0.0);
        let x = anchor.x + PROMPT_TAG_LEFT_INSET + fragment.x;
        let y = anchor.y + fragment.row as f32 * anchor.height + top_inset;
        let color = segment_color(colors, segment.role);
        sugarloaf.rounded_rect(
            None,
            x,
            y,
            fragment.width,
            metrics.height,
            segment_tag_background(segment.role),
            0.0,
            metrics.radius,
            ORDER - 1,
        );
        #[cfg(feature = "native-gui-test-hooks")]
        self.native_prompt_paints.borrow_mut().push((
            anchor.generation,
            anchor.key,
            [x, y, fragment.width, metrics.height],
        ));
        if fragment.leading > 0.0 {
            let slot = fragment.leading * metrics.icon_slot
                / (metrics.icon_slot + metrics.icon_gap);
            let icon = metrics.icon_size.min(slot);
            draw_icon_in_slot(
                sugarloaf,
                segment.icon,
                x + fragment.padding,
                y + (metrics.height - icon) * 0.5,
                slot,
                icon,
                color,
            );
        }
        super::command_info::draw_fragment_text(
            sugarloaf.text_mut(),
            text,
            fragment,
            [anchor.x, anchor.y],
            [anchor.height, metrics.font_size, metrics.height],
            color_to_u8(color),
        );
    }

    fn remember_prompt(&mut self, prompt: ActivePrompt) {
        if let Some(existing) = self.prompt_history.iter_mut().find(|entry| {
            entry.session_id == prompt.session_id
                && same_prompt_identity(
                    entry.generation,
                    entry.key,
                    prompt.generation,
                    prompt.key,
                )
        }) {
            existing.generation = prompt.generation;
            existing.key = prompt.key;
            existing.segments = prompt.segments;
            return;
        }
        self.prompt_history.push_back(PromptSnapshot {
            session_id: prompt.session_id,
            generation: prompt.generation,
            key: prompt.key,
            segments: prompt.segments,
        });
        while self.prompt_history.len() > MAX_PROMPT_CONTEXT_HISTORY {
            self.prompt_history.pop_front();
        }
    }

    fn request_refresh_if_needed<F>(
        &mut self,
        session: &SessionFacts,
        force: bool,
        completion: F,
    ) where
        F: FnOnce() -> runtime::DevOpsRefreshCompletion,
    {
        let session_changed = self
            .last_session
            .as_ref()
            .is_none_or(|previous| !runtime::same_devops_context(previous, session));
        let expired = self
            .last_refresh_request
            .is_none_or(|instant| instant.elapsed() >= REFRESH_INTERVAL);
        if !force && !session_changed && !expired && !self.refresh_pending {
            return;
        }

        let in_flight_fresh = self.request_in_flight
            && self
                .last_refresh_request
                .is_some_and(|instant| instant.elapsed() < REFRESH_IN_FLIGHT_TIMEOUT);
        if in_flight_fresh && !session_changed {
            return;
        }
        if self.request_in_flight && !in_flight_fresh {
            self.request_in_flight = false;
        }

        if session_changed {
            self.contribution = None;
            self.observed_global_generation = 0;
            self.snapshot_revision = 0;
            self.request_in_flight = false;
        }

        match runtime::request_devops_refresh(session, Some(completion())) {
            runtime::RefreshSubmission::Queued => {
                self.last_session = Some(session.clone());
                self.last_refresh_request = Some(Instant::now());
                self.refresh_pending = true;
                self.request_in_flight = true;
            }
            runtime::RefreshSubmission::Busy => {
                self.refresh_pending = true;
                self.request_in_flight = false;
            }
            runtime::RefreshSubmission::Rejected
            | runtime::RefreshSubmission::Unavailable => {
                self.last_session = Some(session.clone());
                self.last_refresh_request = Some(Instant::now());
                self.refresh_pending = false;
                self.request_in_flight = false;
            }
        }
    }

    fn sync_cached_snapshot(&mut self, session: &SessionFacts) {
        let global_generation = runtime::devops_generation();
        if global_generation == self.observed_global_generation {
            return;
        }
        self.observed_global_generation = global_generation;

        let (revision, cached_session, contribution) =
            runtime::context_contribution(session.session_id);
        self.accept_cached_snapshot(
            session,
            revision,
            cached_session.as_ref(),
            contribution,
        );
    }

    fn accept_cached_snapshot(
        &mut self,
        session: &SessionFacts,
        revision: u32,
        cached_session: Option<&SessionFacts>,
        contribution: ContextContribution,
    ) {
        match snapshot_candidate(
            self.snapshot_revision,
            revision,
            cached_session,
            session,
        ) {
            SnapshotCandidate::Unchanged => return,
            SnapshotCandidate::StaleSession => {
                // Shell startup can finish an early request after OSC metadata has
                // changed the session from the native shell to WSL. Release the
                // in-flight latch so the next repaint immediately requests the
                // current session instead of waiting for the timeout fallback.
                self.refresh_pending = true;
                self.request_in_flight = false;
                return;
            }
            SnapshotCandidate::Current => {}
        }
        self.snapshot_revision = revision;
        self.refresh_pending =
            contribution.freshness == automexia_extension_api::Freshness::Refreshing;
        self.request_in_flight = self.refresh_pending;
        self.contribution = Some(contribution);
    }

    fn ensure_live_segments(&mut self, session: &SessionFacts) {
        if self.live_segments_session.as_ref() == Some(session)
            && self.live_segments_snapshot_revision == self.snapshot_revision
        {
            return;
        }

        self.live_segments = self.build_live_segments(session);
        self.live_segments_session = Some(session.clone());
        self.live_segments_snapshot_revision = self.snapshot_revision;
        self.live_segments_revision = self.live_segments_revision.wrapping_add(1);
    }

    fn build_live_segments(&self, session: &SessionFacts) -> Vec<Segment> {
        self.contribution.as_ref().map_or_else(
            || automexia_ui_model::immediate_session_segments(session),
            |contribution| automexia_ui_model::project_status(session, contribution),
        )
    }
}

fn snapshot_candidate(
    current_revision: u32,
    candidate_revision: u32,
    cached_session: Option<&SessionFacts>,
    current_session: &SessionFacts,
) -> SnapshotCandidate {
    if candidate_revision == current_revision || cached_session.is_none() {
        SnapshotCandidate::Unchanged
    } else if cached_session
        .is_some_and(|cached| !runtime::same_devops_context(cached, current_session))
    {
        SnapshotCandidate::StaleSession
    } else {
        SnapshotCandidate::Current
    }
}

fn same_prompt_identity(
    left_generation: Option<u64>,
    left_key: u64,
    right_generation: Option<u64>,
    right_key: u64,
) -> bool {
    match (left_generation, right_generation) {
        (Some(left), Some(right)) => left == right,
        _ => left_key == right_key,
    }
}

/// Symbols from the Nerd Font vocabulary used by the reference project.
fn icon_glyph(icon: IconKind) -> &'static str {
    automexia_ui_model::icon_glyph(icon)
}

/// Optical corrections measured against the bundled Symbols Nerd Font.
/// Codepoints share an advance cell but not an ink box: Docker occupies only
/// about two thirds of the height used by Git or Kubernetes, while cloud and
/// environment marks are also deliberately compact.
#[inline]
fn icon_optics(icon: IconKind) -> IconOptics {
    automexia_ui_model::icon_optics(icon)
}

#[inline]
fn icon_font_size(base_size: f32, icon: IconKind) -> f32 {
    base_size * icon_optics(icon).scale
}

#[inline]
fn icon_draw_y(base_y: f32, base_size: f32, icon: IconKind) -> f32 {
    let optics = icon_optics(icon);
    base_y + (base_size - base_size * optics.scale) * 0.5 + optics.y_shift
}

fn draw_icon_in_slot(
    sugarloaf: &mut Sugarloaf,
    icon: IconKind,
    slot_x: f32,
    base_y: f32,
    slot_width: f32,
    base_size: f32,
    color: [f32; 4],
) {
    draw_icon_text_in_slot(
        sugarloaf.text_mut(),
        icon,
        slot_x,
        base_y,
        slot_width,
        base_size,
        color,
    );
}

fn draw_icon_text_in_slot(
    text: &mut rio_backend::sugarloaf::text::Text,
    icon: IconKind,
    slot_x: f32,
    base_y: f32,
    slot_width: f32,
    base_size: f32,
    color: [f32; 4],
) {
    let glyph = icon_glyph(icon);
    let opts = DrawOpts {
        font_size: icon_font_size(base_size, icon),
        color: color_to_u8(color),
        ..DrawOpts::default()
    };
    let measured_width = text.measure(glyph, &opts);
    let x = slot_x + (slot_width - measured_width) * 0.5;
    text.draw(x, icon_draw_y(base_y, base_size, icon), glyph, &opts);
}
pub(crate) fn next_context_wake_millis(refresh_pending: bool) -> u64 {
    if refresh_pending {
        100
    } else {
        LIVE_REFRESH_MILLIS
    }
}

#[cfg(test)]
fn compact_label(value: &str, max_chars: usize) -> String {
    automexia_ui_model::compact_label(value, max_chars)
}

#[cfg(test)]
fn compact_middle(value: &str, max_chars: usize) -> String {
    automexia_ui_model::compact_middle(value, max_chars)
}

#[cfg(test)]
fn segment_anchor_rgb(role: SegmentRole) -> [u8; 3] {
    automexia_ui_model::segment_anchor_rgb(role)
}

#[cfg(test)]
fn segment_anchor(role: SegmentRole) -> [f32; 4] {
    automexia_ui_model::segment_anchor(role)
}

fn segment_color(colors: Colors, role: SegmentRole) -> [f32; 4] {
    automexia_ui_model::segment_tag_color(colors.background.0, role)
}

fn segment_tag_background(role: SegmentRole) -> [f32; 4] {
    automexia_ui_model::segment_tag_background(role)
}

#[cfg(test)]
fn contrast_ratio(left: [f32; 4], right: [f32; 4]) -> f32 {
    automexia_ui_model::contrast_ratio(left, right)
}

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    color.map(|value| (value.clamp(0.0, 1.0) * 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use automexia_extension_api::{ExtensionId, Freshness, SessionId, StatusSegment};

    const ALL_SEGMENT_ROLES: [SegmentRole; 13] = [
        SegmentRole::Production,
        SegmentRole::UbuntuWsl,
        SegmentRole::Windows,
        SegmentRole::Git,
        SegmentRole::Kubernetes,
        SegmentRole::Docker,
        SegmentRole::Azure,
        SegmentRole::Aws,
        SegmentRole::Gcp,
        SegmentRole::UnknownCloud,
        SegmentRole::Terraform,
        SegmentRole::Environment,
        SegmentRole::User,
    ];
    const ALL_ICON_KINDS: [IconKind; 10] = [
        IconKind::Wsl,
        IconKind::Windows,
        IconKind::Docker,
        IconKind::Kubernetes,
        IconKind::Cloud,
        IconKind::Terraform,
        IconKind::Git,
        IconKind::Environment,
        IconKind::User,
        IconKind::Production,
    ];

    fn colors_with_background(background: [f32; 4]) -> Colors {
        let mut colors = Colors::default();
        colors.background.0 = background;
        colors
    }

    fn quantized(color: [f32; 4]) -> [f32; 4] {
        color_to_u8(color).map(|channel| f32::from(channel) / 255.0)
    }

    fn session(title: &str, distro: Option<&str>) -> SessionFacts {
        SessionFacts {
            session_id: 1,
            cwd: None,
            title: title.to_string(),
            distro: distro.map(str::to_string),
            os_version: None,
            shell_name: Some(
                if distro.is_some() {
                    "bash"
                } else {
                    "PowerShell"
                }
                .to_string(),
            ),
            shell_user: None,
            shell_path: None,
            shell_integration: true,
            shell_pid: 42,
            environment: Default::default(),
        }
    }

    fn contribution(segments: Vec<StatusSegment>) -> ContextContribution {
        ContextContribution::new(
            ExtensionId::new("automexia.devops").unwrap(),
            SessionId::new(1),
            1,
            1,
            Freshness::Current,
            segments,
        )
        .unwrap()
    }

    fn status_segment(
        id: &str,
        value: &str,
        role: SegmentRole,
        icon: IconKind,
        priority: u16,
    ) -> StatusSegment {
        StatusSegment::new(
            id,
            value,
            format!("{id} {value}"),
            role,
            icon,
            priority,
            Freshness::Current,
        )
        .unwrap()
    }

    #[test]
    fn renderer_adapter_uses_the_shared_brand_anchors_and_contrast() {
        let backgrounds = [
            [0.01, 0.02, 0.03, 1.0],
            [0.98, 0.98, 0.96, 1.0],
            [0.32, 0.34, 0.37, 1.0],
        ];
        for background in backgrounds {
            let colors = colors_with_background(background);
            for role in ALL_SEGMENT_ROLES {
                assert_eq!(
                    segment_anchor_rgb(role),
                    automexia_ui_model::segment_anchor_rgb(role)
                );
                let rendered = quantized(segment_color(colors, role));
                let surface =
                    quantized(automexia_ui_model::segment_tag_surface(background, role));
                assert!(contrast_ratio(rendered, surface) >= 4.5);
            }
        }
        assert_eq!(
            segment_anchor(SegmentRole::User),
            automexia_ui_model::segment_anchor(SegmentRole::User)
        );
    }

    #[test]
    fn prompt_tags_are_secondary_compact_and_fit_their_rows() {
        let comfortable = prompt_tag_metrics(24.0);
        assert_eq!(PROMPT_TAG_MAX_FONT_SIZE, 14.0);
        assert_eq!(comfortable.font_size, 14.0);
        assert!(
            comfortable.font_size
                < rio_backend::sugarloaf::font::fonts::default_font_size()
        );
        assert!(comfortable.height < 24.0);
        assert!(comfortable.icon_slot >= comfortable.icon_size);
        assert!(comfortable.tag_gap < comfortable.icon_slot);

        for row_height in [2.0, 8.0, 16.0, 24.0, 48.0] {
            let metrics = prompt_tag_metrics(row_height);
            let top_inset = automexia_ui_model::prompt_context_top_inset(
                row_height,
                metrics.height,
                true,
            )
            .unwrap();
            assert!(metrics.font_size <= row_height);
            assert!(metrics.height <= row_height);
            assert!(top_inset + metrics.height <= row_height + f32::EPSILON);
            assert!(metrics.radius <= metrics.height * 0.5);
            assert!(metrics.padding_x > 0.0);
            assert!(metrics.icon_gap > 0.0);
        }
        assert!(
            automexia_ui_model::prompt_context_top_inset(24.0, comfortable.height, true,)
                .unwrap()
                > (24.0 - comfortable.height) * 0.5
        );
    }

    #[test]
    fn renderer_icon_adapter_uses_shared_glyphs_and_optics() {
        let docker = icon_optics(IconKind::Docker);
        assert_eq!(docker.scale, 1.85);
        let prompt_icon_size = prompt_tag_metrics(24.0).icon_size;
        for kind in ALL_ICON_KINDS {
            assert_eq!(icon_glyph(kind), automexia_ui_model::icon_glyph(kind));
            assert!(icon_glyph(kind)
                .chars()
                .all(|character| character as u32 >= 0xe000));
            let optics = icon_optics(kind);
            assert!((0.90..=1.90).contains(&optics.scale));
            let prompt_size = icon_font_size(prompt_icon_size, kind);
            assert!(prompt_size > 0.0);
            let prompt_center = icon_draw_y(20.0, prompt_icon_size, kind)
                + prompt_size * 0.5
                - optics.y_shift;
            assert!((prompt_center - (20.0 + prompt_icon_size * 0.5)).abs() < 0.001);
        }
    }

    #[test]
    fn kubernetes_icon_raster_grows_without_touching_its_label() {
        use rio_backend::sugarloaf::{
            font::{FontData, FontLibrary, FontLibraryData},
            text::Text,
        };
        use std::sync::Arc;
        let mut data = FontLibraryData::default();
        data.insert(FontData::from_static_slice(include_bytes!("../../../../rio-fonts/resources/SymbolsNerdFontMono/SymbolsNerdFontMono-Regular.ttf")).unwrap());
        let fonts = FontLibrary {
            inner: Arc::new(parking_lot::RwLock::new(data)),
        };
        for scale in [1.0, 1.5, 2.0, 3.0] {
            for row_height in [12.0, 18.0, 24.0, 40.0] {
                let metrics = prompt_tag_metrics(row_height);
                let mut text = Text::new(&fonts);
                text.set_scale_factor(scale);
                text.init_cpu();
                let x = 12.0;
                let y = 12.0;
                draw_icon_text_in_slot(
                    &mut text,
                    IconKind::Kubernetes,
                    x,
                    y,
                    metrics.icon_slot,
                    metrics.icon_size,
                    [0.31, 0.84, 1.0, 1.0],
                );
                assert_eq!(
                    text.instance_count(),
                    1,
                    "bundled Kubernetes glyph must rasterize"
                );
                let glyph = text.instances()[0];
                let right = glyph.pos[0]
                    + f32::from(glyph.bearings[0])
                    + glyph.glyph_size[0] as f32;
                let label_start = (x + metrics.icon_slot + metrics.icon_gap) * scale;
                assert!(
                    right < label_start,
                    "icon must not reach the namespace text"
                );
                assert!(
                    glyph.glyph_size[1] as f32 <= row_height * scale,
                    "ink stays inside its row"
                );
                if row_height == 24.0 && scale == 1.0 {
                    let mut pixels = vec![0u32; 96 * 48];
                    text.render_cpu_base(&mut pixels, 96, 48);
                    // This standalone Text fixture has no frame finalizer;
                    // render both partitions just like the other CPU fixtures.
                    text.render_cpu_modal(&mut pixels, 96, 48);
                    let ink = pixels.iter().filter(|pixel| **pixel != 0).count();
                    // Literal previous size is an independent regression oracle,
                    // not a second call to the current optical-sizing policy.
                    let mut previous = Text::new(&fonts);
                    previous.init_cpu();
                    previous.draw(
                        x,
                        y,
                        icon_glyph(IconKind::Kubernetes),
                        &DrawOpts {
                            font_size: metrics.icon_size * 0.94,
                            ..Default::default()
                        },
                    );
                    let mut old_pixels = vec![0u32; 96 * 48];
                    previous.render_cpu_base(&mut old_pixels, 96, 48);
                    previous.render_cpu_modal(&mut old_pixels, 96, 48);
                    let old_ink = old_pixels.iter().filter(|pixel| **pixel != 0).count();
                    assert!(
                        old_ink > 0 && ink > old_ink * 5 / 4,
                        "logo needs a visible ink-area increase"
                    );
                    if let Some(path) =
                        std::env::var_os("AUTOMEXIA_KUBERNETES_ICON_PREVIEW")
                    {
                        image_rs::RgbImage::from_fn(96, 48, |x, y| {
                            let pixel = pixels[y as usize * 96 + x as usize];
                            image_rs::Rgb([
                                (pixel >> 16) as u8,
                                (pixel >> 8) as u8,
                                pixel as u8,
                            ])
                        })
                        .save(path)
                        .unwrap();
                    }
                }
            }
        }
    }

    #[test]
    fn renderer_compaction_delegates_to_grapheme_safe_ui_policy() {
        assert_eq!(
            compact_label("dev-😀-cluster-name", 10),
            automexia_ui_model::compact_label("dev-😀-cluster-name", 10)
        );
        assert_eq!(
            compact_middle("feature/very-long-branch", 12),
            automexia_ui_model::compact_middle("feature/very-long-branch", 12)
        );
    }

    #[test]
    fn live_segments_rebuild_only_when_generic_inputs_change() {
        let session = session("alice@host:/work", Some("Ubuntu"));
        let mut status = DevOpsStatus::default();
        status.ensure_live_segments(&session);
        let initial_revision = status.live_segments_revision;

        status.ensure_live_segments(&session);
        assert_eq!(status.live_segments_revision, initial_revision);

        status.contribution = Some(contribution(vec![status_segment(
            "docker",
            "docker",
            SegmentRole::Docker,
            IconKind::Docker,
            60,
        )]));
        status.snapshot_revision = 1;
        status.ensure_live_segments(&session);
        assert_eq!(
            status.live_segments_revision,
            initial_revision.wrapping_add(1)
        );
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.icon == IconKind::Docker));
    }

    #[test]
    fn generic_projection_preserves_priority_and_user_is_final() {
        let session = session("alice@host:/work", Some("Ubuntu"));
        let status = DevOpsStatus {
            contribution: Some(contribution(vec![
                status_segment(
                    "docker",
                    "docker",
                    SegmentRole::Docker,
                    IconKind::Docker,
                    60,
                ),
                status_segment("user", "alice", SegmentRole::User, IconKind::User, 90),
            ])),
            ..DevOpsStatus::default()
        };
        let segments = status.build_live_segments(&session);
        assert_eq!(
            segments.last().map(|segment| segment.icon),
            Some(IconKind::User)
        );
        assert!(
            segments
                .iter()
                .any(|segment| segment.icon == IconKind::Docker
                    && segment.value == "docker")
        );
    }

    #[test]
    fn title_change_accepts_progress_without_releasing_the_pending_latch() {
        let facts = session("starting", Some("Ubuntu"));
        let mut renamed = facts.clone();
        renamed.title = "command in progress".into();
        let mut status = DevOpsStatus::default();
        let mut contribution = ContextContribution::empty(
            ExtensionId::new("automexia.devops").unwrap(),
            SessionId::new(facts.session_id as u64),
        );
        contribution.freshness = Freshness::Refreshing;
        status.accept_cached_snapshot(&renamed, 11, Some(&facts), contribution.clone());
        assert_eq!(status.snapshot_revision, 11);
        assert!(status.refresh_pending && status.request_in_flight);
        assert_eq!(next_context_wake_millis(status.refresh_pending), 100);
        contribution.freshness = Freshness::Current;
        status.accept_cached_snapshot(&renamed, 12, Some(&facts), contribution);
        assert_eq!(status.snapshot_revision, 12);
        assert!(!status.refresh_pending && !status.request_in_flight);
        assert_eq!(next_context_wake_millis(status.refresh_pending), 3000);
    }

    #[test]
    fn completed_refreshes_schedule_the_next_live_poll() {
        assert_eq!(next_context_wake_millis(true), 100);
        assert_eq!(next_context_wake_millis(false), LIVE_REFRESH_MILLIS);
    }

    #[test]
    fn stale_startup_contribution_is_retried_without_timeout() {
        let current = session("alice@host:/work/current", Some("Ubuntu"));
        let stale = session("Automexia", None);
        assert_eq!(
            snapshot_candidate(0, 2, Some(&stale), &current),
            SnapshotCandidate::StaleSession
        );
        assert_eq!(
            snapshot_candidate(0, 2, Some(&current), &current),
            SnapshotCandidate::Current
        );
        assert_eq!(
            snapshot_candidate(2, 2, Some(&current), &current),
            SnapshotCandidate::Unchanged
        );
    }

    #[test]
    fn stable_prompt_identity_survives_reflow_key_changes() {
        assert!(same_prompt_identity(Some(7), 12, Some(7), 99));
        assert!(!same_prompt_identity(Some(7), 12, Some(8), 12));
        assert!(same_prompt_identity(None, 12, None, 12));
        assert!(!same_prompt_identity(None, 12, None, 99));
    }
}
