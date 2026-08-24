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
use crate::automexia::ui::{
    CommandResultAnchor, PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY,
};

pub(crate) const LIVE_REFRESH_MILLIS: u64 = 3_000;
const REFRESH_INTERVAL: Duration = Duration::from_millis(LIVE_REFRESH_MILLIS);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const ORDER: u8 = 19;
const PROMPT_TAG_FONT_ROW_RATIO: f32 = 0.62;
const PROMPT_TAG_MAX_FONT_SIZE: f32 = 14.0;
const PROMPT_TAG_MIN_FONT_SIZE: f32 = 4.0;
const PROMPT_TAG_LEFT_INSET: f32 = 2.0;
const PROMPT_RESULT_RESERVE: f32 = 112.0;
const RESULT_DIVIDER_ALPHA: f32 = 0.32;

#[derive(Clone, Copy, Debug, PartialEq)]
struct PromptTagMetrics {
    font_size: f32,
    icon_size: f32,
    icon_slot: f32,
    height: f32,
    padding_x: f32,
    icon_gap: f32,
    tag_gap: f32,
    radius: f32,
}

fn prompt_tag_metrics(row_height: f32) -> PromptTagMetrics {
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

/// Keep the result boundary inside the following prompt's already-reserved
/// context row. Nothing is inserted into the terminal grid or PTY stream.
fn command_result_divider(anchor: &CommandResultAnchor) -> Option<[f32; 4]> {
    if !anchor.separates_next_prompt {
        return None;
    }
    let inset = (anchor.height * 0.1).clamp(1.0, 2.0);
    let height = (anchor.height * 0.05).clamp(1.0, 1.5);
    let width = anchor.width - inset * 2.0;
    (width >= 1.0).then_some([
        anchor.x + inset,
        anchor.y + (anchor.height * 0.08).clamp(0.5, 1.5),
        width,
        height,
    ])
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
    pub fn render_prompt_rows(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        session: &SessionFacts,
        prompt_active: bool,
        historical_anchors: &[PromptAnchor],
        live_anchor: Option<PromptAnchor>,
    ) -> bool {
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

        // A newly visible historical row can predate this renderer instance
        // (for example after restoring a route or deep scrollback). Never
        // leave its context strip empty: use the current local snapshot until
        // a prompt-specific snapshot exists.
        for anchor in historical_anchors {
            if live_anchor.is_some_and(|live| {
                same_prompt_identity(
                    live.generation,
                    live.key,
                    anchor.generation,
                    anchor.key,
                )
            }) {
                continue;
            }
            let segments = self
                .cached_segments(session.session_id, anchor)
                .unwrap_or(&self.live_segments);
            self.draw_prompt_segments(sugarloaf, colors, anchor, segments);
        }

        if prompt_active {
            if let (Some(anchor), Some(active)) =
                (live_anchor, self.active_prompt.as_ref())
            {
                if active.session_id == session.session_id
                    && same_prompt_identity(
                        active.generation,
                        active.key,
                        anchor.generation,
                        anchor.key,
                    )
                {
                    self.draw_prompt_segments(
                        sugarloaf,
                        colors,
                        &anchor,
                        &active.segments,
                    );
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

    fn draw_prompt_segments(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchor: &PromptAnchor,
        segments: &[Segment],
    ) {
        let metrics = prompt_tag_metrics(anchor.height);
        let Some(top_inset) = automexia_ui_model::prompt_context_top_inset(
            anchor.height,
            metrics.height,
            !segments.is_empty(),
        ) else {
            return;
        };
        let tag_y = anchor.y + top_inset;
        let text_y = tag_y + (metrics.height - metrics.font_size) * 0.5 - 1.0;
        let icon_y = tag_y + (metrics.height - metrics.icon_size) * 0.5;
        let mut cursor_x = anchor.x + PROMPT_TAG_LEFT_INSET;
        let right_edge = anchor.x + (anchor.width - PROMPT_RESULT_RESERVE).max(80.0);

        for segment in segments {
            let color = segment_color(colors, segment.role);
            let text_opts = DrawOpts {
                font_size: metrics.font_size,
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let text_width = sugarloaf.text_mut().measure(&segment.value, &text_opts);
            let segment_width = metrics.padding_x * 2.0
                + metrics.icon_slot
                + metrics.icon_gap
                + text_width;
            if cursor_x + segment_width > right_edge {
                break;
            }

            sugarloaf.rounded_rect(
                None,
                cursor_x,
                tag_y,
                segment_width,
                metrics.height,
                segment_tag_background(segment.role),
                0.0,
                metrics.radius,
                ORDER - 1,
            );
            let content_x = cursor_x + metrics.padding_x;
            draw_icon_in_slot(
                sugarloaf,
                segment.icon,
                content_x,
                icon_y,
                metrics.icon_slot,
                metrics.icon_size,
                color,
            );
            let label_x = content_x + metrics.icon_slot + metrics.icon_gap;
            sugarloaf
                .text_mut()
                .draw(label_x, text_y, &segment.value, &text_opts);
            cursor_x += segment_width + metrics.tag_gap;
        }
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

    /// Draw completion state on the semantic row that owns the command.
    pub fn render_command_results(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        anchors: &[CommandResultAnchor],
    ) {
        for anchor in anchors {
            let success = anchor.exit_code == 0;
            let status = if success { "✓" } else { "×" };
            let label = format!("{status}  {}", format_duration(anchor.elapsed_ms));
            let metrics = prompt_tag_metrics(anchor.height);
            let Some(top_inset) = automexia_ui_model::prompt_context_top_inset(
                anchor.height,
                metrics.height,
                true,
            ) else {
                continue;
            };
            let opts = DrawOpts {
                font_size: metrics.font_size,
                color: color_to_u8(if success { colors.green } else { colors.red }),
                ..DrawOpts::default()
            };
            if let Some([x, y, width, height]) = command_result_divider(anchor) {
                let mut divider_color = if success { colors.green } else { colors.red };
                divider_color[3] = RESULT_DIVIDER_ALPHA;
                sugarloaf.rect(None, x, y, width, height, divider_color, 0.0, ORDER - 2);
            }
            let text_width = sugarloaf.text_mut().measure(&label, &opts);
            let x = anchor.x + anchor.width - text_width - 10.0;
            if x <= anchor.x + 24.0 {
                continue;
            }
            let tag_y = anchor.y + top_inset;
            let y = tag_y + (metrics.height - metrics.font_size) * 0.5 - 1.0;
            sugarloaf.text_mut().draw(x, y, &label, &opts);
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
        let session_changed = self.last_session.as_ref() != Some(session);
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
        match snapshot_candidate(
            self.snapshot_revision,
            revision,
            cached_session.as_ref(),
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
        self.refresh_pending = false;
        self.request_in_flight = false;
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
    } else if cached_session != Some(current_session) {
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
    let glyph = icon_glyph(icon);
    let opts = DrawOpts {
        font_size: icon_font_size(base_size, icon),
        color: color_to_u8(color),
        ..DrawOpts::default()
    };
    let measured_width = sugarloaf.text_mut().measure(glyph, &opts);
    let x = slot_x + (slot_width - measured_width) * 0.5;
    sugarloaf
        .text_mut()
        .draw(x, icon_draw_y(base_y, base_size, icon), glyph, &opts);
}

fn format_duration(elapsed_ms: u64) -> String {
    if elapsed_ms < 1_000 {
        format!("{elapsed_ms}ms")
    } else if elapsed_ms < 60_000 {
        format!("{:.1}s", elapsed_ms as f64 / 1_000.0)
    } else {
        format!(
            "{}m {:02}s",
            elapsed_ms / 60_000,
            (elapsed_ms % 60_000) / 1_000
        )
    }
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
    fn command_duration_uses_compact_units() {
        assert_eq!(format_duration(18), "18ms");
        assert_eq!(format_duration(1_250), "1.2s");
        assert_eq!(format_duration(62_000), "1m 02s");
    }

    #[test]
    fn result_divider_is_bounded_and_only_marks_a_following_prompt() {
        let mut anchor = CommandResultAnchor {
            x: 4.0,
            y: 40.0,
            width: 720.0,
            height: 20.0,
            separates_next_prompt: true,
            exit_code: 0,
            elapsed_ms: 18,
        };
        let [x, y, width, height] = command_result_divider(&anchor).unwrap();
        assert!(x >= anchor.x);
        assert!(y >= anchor.y);
        assert!(x + width <= anchor.x + anchor.width);
        assert!(y + height <= anchor.y + anchor.height);

        anchor.separates_next_prompt = false;
        assert_eq!(command_result_divider(&anchor), None);
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
        let session = session("amjed@host:/work", Some("Ubuntu"));
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
        let session = session("amjed@host:/work", Some("Ubuntu"));
        let status = DevOpsStatus {
            contribution: Some(contribution(vec![
                status_segment(
                    "docker",
                    "docker",
                    SegmentRole::Docker,
                    IconKind::Docker,
                    60,
                ),
                status_segment("user", "amjed", SegmentRole::User, IconKind::User, 90),
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
    fn completed_refreshes_schedule_the_next_live_poll() {
        assert_eq!(next_context_wake_millis(true), 100);
        assert_eq!(next_context_wake_millis(false), LIVE_REFRESH_MILLIS);
    }

    #[test]
    fn stale_startup_contribution_is_retried_without_timeout() {
        let current = session("amjed@host:/work/current", Some("Ubuntu"));
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
