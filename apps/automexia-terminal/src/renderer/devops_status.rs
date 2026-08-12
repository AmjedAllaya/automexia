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

use crate::automexia::api::SessionFacts;
use crate::automexia::builtins::devops::{CloudContext, DevOpsSnapshot};
use crate::automexia::runtime;
use crate::automexia::ui::{
    CommandResultAnchor, PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY,
};

pub(crate) const LIVE_REFRESH_MILLIS: u64 = 3_000;
const REFRESH_INTERVAL: Duration = Duration::from_millis(LIVE_REFRESH_MILLIS);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const ORDER: u8 = 19;
const CONTEXT_FONT_SIZE: f32 = 18.0;
const PROMPT_CONTEXT_FONT_SIZE: f32 = 18.0;
const PROMPT_CONTEXT_ICON_SIZE: f32 = 23.0;
const PROMPT_CONTEXT_ICON_SLOT: f32 = 27.0;
const PROMPT_CONTEXT_PAD_X: f32 = 4.0;
const PROMPT_CONTEXT_ICON_GAP: f32 = 8.0;
const PROMPT_CONTEXT_SEPARATOR_GAP: f32 = 9.0;
const PROMPT_RESULT_RESERVE: f32 = 112.0;
const MIN_SEGMENT_CONTRAST: f32 = 4.55;

const _: () = {
    assert!(PROMPT_CONTEXT_ICON_SLOT >= PROMPT_CONTEXT_ICON_SIZE);
};

const MAX_WSL_CHARS: usize = 14;
const MAX_CONTEXT_CHARS: usize = 22;
const MAX_CLOUD_CHARS: usize = 22;
const MAX_GIT_CHARS: usize = 24;
const MAX_ENV_CHARS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SegmentRole {
    Production,
    UbuntuWsl,
    Windows,
    Git,
    Kubernetes,
    Docker,
    Azure,
    Aws,
    Gcp,
    UnknownCloud,
    Terraform,
    Environment,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IconKind {
    Wsl,
    Windows,
    Docker,
    Kubernetes,
    Cloud,
    Terraform,
    Git,
    Environment,
    User,
    Production,
}

#[derive(Clone)]
struct Segment {
    value: String,
    role: SegmentRole,
    icon: IconKind,
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct IconOptics {
    /// Compensates for the amount of unused space inside each icon's font
    /// bounding box. The result is an equal perceived height, not an equal
    /// nominal point size.
    scale: f32,
    /// Final optical nudge after point-size centering.
    y_shift: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapshotCandidate {
    Unchanged,
    StaleSession,
    Current,
}

#[derive(Default)]
pub struct DevOpsStatus {
    snapshot: DevOpsSnapshot,
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
        let text_y = anchor.y
            + (anchor.height.max(PROMPT_CONTEXT_FONT_SIZE) - PROMPT_CONTEXT_FONT_SIZE)
                / 2.0
            - 1.0;
        let mut cursor_x = anchor.x + PROMPT_CONTEXT_PAD_X;
        let right_edge = anchor.x + (anchor.width - PROMPT_RESULT_RESERVE).max(80.0);
        let separator = muted(colors.foreground, 0.32);

        for (index, segment) in segments.iter().enumerate() {
            let color = segment_color(colors, segment.role);
            let text_opts = DrawOpts {
                font_size: PROMPT_CONTEXT_FONT_SIZE,
                color: color_to_u8(color),
                ..DrawOpts::default()
            };
            let text_width = sugarloaf.text_mut().measure(&segment.value, &text_opts);
            let separator_width = if index == 0 {
                0.0
            } else {
                PROMPT_CONTEXT_SEPARATOR_GAP * 2.0 + 1.0
            };
            let segment_width = PROMPT_CONTEXT_ICON_SLOT
                + PROMPT_CONTEXT_ICON_GAP
                + text_width
                + PROMPT_CONTEXT_PAD_X;
            if cursor_x + separator_width + segment_width > right_edge {
                break;
            }

            if index != 0 {
                cursor_x += PROMPT_CONTEXT_SEPARATOR_GAP;
                sugarloaf.line(
                    cursor_x,
                    anchor.y + 3.0,
                    cursor_x,
                    anchor.y + anchor.height - 3.0,
                    1.0,
                    0.0,
                    separator,
                    ORDER,
                );
                cursor_x += PROMPT_CONTEXT_SEPARATOR_GAP + 1.0;
            }
            draw_icon_in_slot(
                sugarloaf,
                segment.icon,
                cursor_x,
                text_y - 2.0,
                PROMPT_CONTEXT_ICON_SLOT,
                PROMPT_CONTEXT_ICON_SIZE,
                color,
            );
            cursor_x += PROMPT_CONTEXT_ICON_SLOT + PROMPT_CONTEXT_ICON_GAP;
            sugarloaf
                .text_mut()
                .draw(cursor_x, text_y, &segment.value, &text_opts);
            cursor_x += text_width + PROMPT_CONTEXT_PAD_X;
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
            let opts = DrawOpts {
                font_size: CONTEXT_FONT_SIZE,
                color: color_to_u8(if success { colors.green } else { colors.red }),
                ..DrawOpts::default()
            };
            let text_width = sugarloaf.text_mut().measure(&label, &opts);
            let x = anchor.x + anchor.width - text_width - 10.0;
            if x <= anchor.x + 24.0 {
                continue;
            }
            let y = anchor.y + (anchor.height - CONTEXT_FONT_SIZE) / 2.0;
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
            self.snapshot = DevOpsSnapshot::default();
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

        let (revision, cached_session, snapshot) =
            runtime::devops_snapshot(session.session_id);
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
        self.snapshot = snapshot;
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
        let mut segments = Vec::new();
        if self.snapshot.production {
            segments.push(Segment {
                value: "PRODUCTION".to_string(),
                role: SegmentRole::Production,
                icon: IconKind::Production,
            });
        }
        let immediate_os = immediate_os_value(session);
        let detected_wsl = immediate_os.or_else(|| {
            (!is_native_windows_shell(session))
                .then(|| self.snapshot.wsl.as_ref().map(|wsl| wsl_value(&wsl.distro)))
                .flatten()
        });
        if let Some(os) = detected_wsl {
            segments.push(Segment {
                value: compact_label(&os, MAX_WSL_CHARS),
                role: SegmentRole::UbuntuWsl,
                icon: IconKind::Wsl,
            });
        } else if is_native_windows_shell(session) {
            segments.push(Segment {
                value: "Windows".to_string(),
                role: SegmentRole::Windows,
                icon: IconKind::Windows,
            });
        }
        if let Some(branch) = &self.snapshot.git_branch {
            segments.push(Segment {
                value: compact_middle(branch, MAX_GIT_CHARS),
                role: SegmentRole::Git,
                icon: IconKind::Git,
            });
        }
        if let Some(kubernetes) = &self.snapshot.kubernetes {
            let value =
                if kubernetes.namespace.is_empty() || kubernetes.namespace == "default" {
                    compact_label(&kubernetes.context, MAX_CONTEXT_CHARS)
                } else {
                    compact_label(
                        &format!("{}/{}", kubernetes.context, kubernetes.namespace),
                        MAX_CONTEXT_CHARS,
                    )
                };
            segments.push(Segment {
                value,
                role: SegmentRole::Kubernetes,
                icon: IconKind::Kubernetes,
            });
        }
        for cloud in &self.snapshot.clouds {
            segments.push(Segment {
                value: cloud_value(cloud),
                role: cloud_segment_role(cloud.provider),
                icon: IconKind::Cloud,
            });
        }
        if let Some(context) = &self.snapshot.docker {
            segments.push(Segment {
                value: docker_value(context),
                role: SegmentRole::Docker,
                icon: IconKind::Docker,
            });
        }
        if let Some(workspace) = &self.snapshot.terraform {
            segments.push(Segment {
                value: compact_label(workspace, MAX_CONTEXT_CHARS),
                role: SegmentRole::Terraform,
                icon: IconKind::Terraform,
            });
        }
        if let Some(environment) = &self.snapshot.environment {
            segments.push(Segment {
                value: compact_label(environment, MAX_ENV_CHARS),
                role: SegmentRole::Environment,
                icon: IconKind::Environment,
            });
        }
        if let Some(user) = self
            .snapshot
            .user
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            segments.push(Segment {
                value: compact_label(user, MAX_ENV_CHARS),
                role: SegmentRole::User,
                icon: IconKind::User,
            });
        }
        segments
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
    match icon {
        IconKind::Wsl => "\u{f31b}",
        IconKind::Windows => "\u{e70f}",
        IconKind::Docker => "\u{f308}",
        IconKind::Kubernetes => "\u{f10fe}",
        IconKind::Cloud => "\u{f0c2}",
        IconKind::Terraform => "\u{f1062}",
        IconKind::Git => "\u{e725}",
        IconKind::Environment => "\u{f1b2}",
        IconKind::User => "\u{f007}",
        IconKind::Production => "\u{f071}",
    }
}

/// Optical corrections measured against the bundled Symbols Nerd Font.
/// Codepoints share an advance cell but not an ink box: Docker occupies only
/// about two thirds of the height used by Git or Kubernetes, while cloud and
/// environment marks are also deliberately compact.
#[inline]
fn icon_optics(icon: IconKind) -> IconOptics {
    match icon {
        IconKind::Wsl => IconOptics {
            scale: 1.0,
            y_shift: 0.0,
        },
        IconKind::Windows => IconOptics {
            scale: 1.10,
            y_shift: 0.0,
        },
        IconKind::Docker => IconOptics {
            scale: 1.85,
            y_shift: -0.5,
        },
        IconKind::Kubernetes => IconOptics {
            scale: 0.94,
            y_shift: 0.0,
        },
        IconKind::Cloud => IconOptics {
            scale: 1.16,
            y_shift: 0.5,
        },
        IconKind::Terraform => IconOptics {
            scale: 1.10,
            y_shift: 0.0,
        },
        IconKind::Git => IconOptics {
            scale: 1.02,
            y_shift: 0.0,
        },
        IconKind::Environment => IconOptics {
            scale: 1.12,
            y_shift: 0.0,
        },
        IconKind::User => IconOptics {
            scale: 1.04,
            y_shift: 0.0,
        },
        IconKind::Production => IconOptics {
            scale: 1.08,
            y_shift: 0.0,
        },
    }
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

fn shell_label(session: &SessionFacts) -> &'static str {
    if let Some(name) = session.shell_name.as_deref() {
        if name.eq_ignore_ascii_case("powershell") || name.eq_ignore_ascii_case("pwsh") {
            return "PowerShell";
        }
        if name.eq_ignore_ascii_case("bash") {
            return "bash";
        }
        if name.eq_ignore_ascii_case("zsh") {
            return "zsh";
        }
        if name.eq_ignore_ascii_case("cmd") || name.eq_ignore_ascii_case("command prompt")
        {
            return "CMD";
        }
    }
    if session
        .distro
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return "zsh";
    }
    #[cfg(target_os = "windows")]
    return "PowerShell";
    #[cfg(not(target_os = "windows"))]
    return "zsh";
}

fn is_native_windows_shell(session: &SessionFacts) -> bool {
    matches!(shell_label(session), "PowerShell" | "CMD")
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

fn immediate_os_value(session: &SessionFacts) -> Option<String> {
    let distro = session
        .distro
        .as_ref()
        .filter(|value| !value.trim().is_empty())?;
    let (_, path) = parse_shell_title(&session.title)?;
    path.starts_with('/').then(|| wsl_value(distro))
}

fn parse_shell_title(title: &str) -> Option<(String, String)> {
    let (user, host_and_path) = title.trim().rsplit_once('@')?;
    let (host, path) = host_and_path.split_once(':')?;
    let user = user.split_whitespace().last()?.trim();
    let path = path.trim();
    if user.is_empty() || host.trim().is_empty() || path.is_empty() {
        return None;
    }
    Some((user.to_string(), path.to_string()))
}

fn wsl_value(distro: &str) -> String {
    let distro = distro.trim();
    if distro.eq_ignore_ascii_case("Ubuntu") || distro.starts_with("Ubuntu-") {
        "Ubuntu".to_string()
    } else {
        distro.to_string()
    }
}

fn cloud_value(cloud: &CloudContext) -> String {
    if !cloud.region.trim().is_empty() {
        compact_label(&cloud.region, MAX_CLOUD_CHARS)
    } else if !cloud.profile.trim().is_empty() {
        compact_label(&cloud.profile, MAX_CLOUD_CHARS)
    } else {
        cloud.provider.to_string()
    }
}

fn cloud_segment_role(provider: &str) -> SegmentRole {
    match provider.trim().to_ascii_lowercase().as_str() {
        "aws" | "amazon" | "amazon web services" => SegmentRole::Aws,
        "azure" | "microsoft azure" => SegmentRole::Azure,
        "gcp" | "google" | "google cloud" | "google cloud platform" => SegmentRole::Gcp,
        _ => SegmentRole::UnknownCloud,
    }
}

fn docker_value(context: &str) -> String {
    let context = context.trim();
    if context.is_empty()
        || context.eq_ignore_ascii_case("default")
        || context.eq_ignore_ascii_case("docker")
    {
        "docker".to_string()
    } else {
        compact_label(context, MAX_CONTEXT_CHARS)
    }
}

pub(crate) fn next_context_wake_millis(refresh_pending: bool) -> u64 {
    if refresh_pending {
        100
    } else {
        LIVE_REFRESH_MILLIS
    }
}

fn compact_label(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn compact_middle(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars < 5 {
        return compact_label(value, max_chars);
    }
    let left = (max_chars - 1) / 2;
    let right = max_chars - left - 1;
    let head: String = value.chars().take(left).collect();
    let tail: String = value
        .chars()
        .rev()
        .take(right)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

fn muted(mut color: [f32; 4], alpha: f32) -> [f32; 4] {
    color[3] = alpha;
    color
}

fn segment_anchor_rgb(role: SegmentRole) -> [u8; 3] {
    match role {
        SegmentRole::Production => [0xff, 0x5c, 0x7a],
        SegmentRole::UbuntuWsl => [0xff, 0x6a, 0x00],
        SegmentRole::Windows => [0x62, 0xb0, 0xff],
        SegmentRole::Git => [0xdc, 0x78, 0xff],
        SegmentRole::Kubernetes => [0x50, 0xd5, 0xff],
        SegmentRole::Docker => [0x24, 0x96, 0xed],
        SegmentRole::Azure => [0x14, 0x7d, 0xdb],
        SegmentRole::Aws => [0xff, 0xb0, 0x20],
        SegmentRole::Gcp => [0xf4, 0x6f, 0x61],
        SegmentRole::UnknownCloud => [0xff, 0xd1, 0x66],
        SegmentRole::Terraform => [0xa7, 0x8b, 0xfa],
        SegmentRole::Environment => [0x2d, 0xd4, 0xbf],
        SegmentRole::User => [0xb8, 0xf3, 0x6b],
    }
}

fn segment_anchor(role: SegmentRole) -> [f32; 4] {
    let [red, green, blue] = segment_anchor_rgb(role);
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

/// Resolve every operational identity from one semantic source for live and
/// historical prompt rows. Theme customization may move only HSL lightness;
/// the identity's anchor hue and saturation remain stable while text contrast
/// is brought up to WCAG AA.
fn segment_color(colors: Colors, role: SegmentRole) -> [f32; 4] {
    let anchor = segment_anchor(role);
    let rendered_anchor = color_to_u8(anchor).map(|channel| f32::from(channel) / 255.0);
    if contrast_ratio(rendered_anchor, colors.background.0) >= 4.5 {
        return anchor;
    }
    // Keep a small margin so conversion to the renderer's 8-bit color does
    // not pull the displayed result below the 4.5:1 contract.
    ensure_contrast(anchor, colors.background.0, MIN_SEGMENT_CONTRAST)
}

fn ensure_contrast(anchor: [f32; 4], background: [f32; 4], minimum: f32) -> [f32; 4] {
    if contrast_ratio(anchor, background) >= minimum {
        return anchor;
    }

    let (hue, saturation, lightness) = rgb_to_hsl(anchor);
    let black = hsl_to_rgb(hue, saturation, 0.0);
    let white = hsl_to_rgb(hue, saturation, 1.0);
    let lighten = contrast_ratio(white, background) >= contrast_ratio(black, background);

    // Find the smallest lightness movement that satisfies the contrast floor.
    // One of the black/white endpoints always reaches at least 4.5:1 for a
    // finite sRGB background, so the search remains deterministic.
    let resolved_lightness = if lighten {
        let mut failing = lightness;
        let mut passing = 1.0;
        for _ in 0..24 {
            let candidate = (failing + passing) * 0.5;
            if contrast_ratio(hsl_to_rgb(hue, saturation, candidate), background)
                >= minimum
            {
                passing = candidate;
            } else {
                failing = candidate;
            }
        }
        passing
    } else {
        let mut passing = 0.0;
        let mut failing = lightness;
        for _ in 0..24 {
            let candidate = (passing + failing) * 0.5;
            if contrast_ratio(hsl_to_rgb(hue, saturation, candidate), background)
                >= minimum
            {
                passing = candidate;
            } else {
                failing = candidate;
            }
        }
        passing
    };
    hsl_to_rgb(hue, saturation, resolved_lightness)
}

fn contrast_ratio(left: [f32; 4], right: [f32; 4]) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn relative_luminance(color: [f32; 4]) -> f32 {
    let channel = |value: f32| {
        let value = value.clamp(0.0, 1.0);
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn rgb_to_hsl(color: [f32; 4]) -> (f32, f32, f32) {
    let red = color[0].clamp(0.0, 1.0);
    let green = color[1].clamp(0.0, 1.0);
    let blue = color[2].clamp(0.0, 1.0);
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let lightness = (maximum + minimum) * 0.5;
    if delta <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue_sector = if maximum == red {
        ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    };
    (hue_sector / 6.0, saturation, lightness)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [f32; 4] {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_sector = hue.rem_euclid(1.0) * 6.0;
    let secondary = chroma * (1.0 - (hue_sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue_sector as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = lightness - chroma * 0.5;
    [red + offset, green + offset, blue + offset, 1.0]
}

fn color_to_u8(color: [f32; 4]) -> [u8; 4] {
    color.map(|value| (value.clamp(0.0, 1.0) * 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            shell_name: None,
            shell_user: None,
            shell_path: None,
            shell_integration: true,
            shell_pid: 42,
        }
    }

    #[test]
    fn every_semantic_role_has_the_exact_brand_anchor() {
        let expected = [
            (SegmentRole::Production, [0xff, 0x5c, 0x7a]),
            (SegmentRole::UbuntuWsl, [0xff, 0x6a, 0x00]),
            (SegmentRole::Windows, [0x62, 0xb0, 0xff]),
            (SegmentRole::Git, [0xdc, 0x78, 0xff]),
            (SegmentRole::Kubernetes, [0x50, 0xd5, 0xff]),
            (SegmentRole::Docker, [0x24, 0x96, 0xed]),
            (SegmentRole::Azure, [0x14, 0x7d, 0xdb]),
            (SegmentRole::Aws, [0xff, 0xb0, 0x20]),
            (SegmentRole::Gcp, [0xf4, 0x6f, 0x61]),
            (SegmentRole::UnknownCloud, [0xff, 0xd1, 0x66]),
            (SegmentRole::Terraform, [0xa7, 0x8b, 0xfa]),
            (SegmentRole::Environment, [0x2d, 0xd4, 0xbf]),
            (SegmentRole::User, [0xb8, 0xf3, 0x6b]),
        ];
        for (role, anchor) in expected {
            assert_eq!(
                segment_anchor_rgb(role),
                anchor,
                "wrong anchor for {role:?}"
            );
        }
    }

    #[test]
    fn semantic_colors_reach_contrast_on_dark_light_and_custom_themes() {
        let backgrounds = [
            [0.01, 0.02, 0.03, 1.0],
            [0.98, 0.98, 0.96, 1.0],
            [0.32, 0.34, 0.37, 1.0],
            [0.08, 0.31, 0.28, 1.0],
        ];
        for background in backgrounds {
            let colors = colors_with_background(background);
            let mut distinct = std::collections::HashSet::new();
            for role in ALL_SEGMENT_ROLES {
                let resolved = segment_color(colors, role);
                let rendered = quantized(resolved);
                assert!(
                    contrast_ratio(rendered, background) >= 4.5,
                    "{role:?} resolved to {rendered:?} below 4.5:1 on {background:?}"
                );
                assert!(
                    distinct.insert(color_to_u8(resolved)),
                    "{role:?} duplicated another identity color on {background:?}"
                );
            }
        }
    }

    #[test]
    fn low_contrast_correction_preserves_anchor_hue() {
        for role in ALL_SEGMENT_ROLES {
            let anchor = segment_anchor(role);
            let colors = colors_with_background(anchor);
            let resolved = segment_color(colors, role);
            let (anchor_hue, anchor_saturation, _) = rgb_to_hsl(anchor);
            let (resolved_hue, resolved_saturation, _) = rgb_to_hsl(resolved);
            assert!((anchor_hue - resolved_hue).abs() < 0.0001, "{role:?}");
            assert!(
                (anchor_saturation - resolved_saturation).abs() < 0.0001,
                "{role:?}"
            );
        }
    }

    #[test]
    fn default_theme_keeps_every_identity_visibly_distinct() {
        let colors = Colors::default();
        let mut resolved = std::collections::HashSet::new();
        for role in ALL_SEGMENT_ROLES {
            assert!(
                resolved.insert(color_to_u8(segment_color(colors, role))),
                "{role:?} duplicated another default identity color"
            );
        }
        assert_ne!(
            segment_color(colors, SegmentRole::Windows),
            segment_color(colors, SegmentRole::Docker)
        );
        assert_ne!(
            segment_color(colors, SegmentRole::Docker),
            segment_color(colors, SegmentRole::Azure)
        );
        assert_eq!(segment_anchor_rgb(SegmentRole::User), [0xb8, 0xf3, 0x6b]);
    }

    #[test]
    fn live_and_historical_prompts_share_one_role_resolver() {
        let colors = colors_with_background([0.92, 0.90, 0.86, 1.0]);
        for role in ALL_SEGMENT_ROLES {
            let live_prompt_color = segment_color(colors, role);
            let historical_prompt_color = segment_color(colors, role);
            assert_eq!(live_prompt_color, historical_prompt_color);
        }
    }

    #[test]
    fn cloud_providers_map_to_independent_roles() {
        assert_eq!(cloud_segment_role("AWS"), SegmentRole::Aws);
        assert_eq!(cloud_segment_role("azure"), SegmentRole::Azure);
        assert_eq!(cloud_segment_role("Google Cloud"), SegmentRole::Gcp);
        assert_eq!(
            cloud_segment_role("private-cloud"),
            SegmentRole::UnknownCloud
        );
    }

    #[test]
    fn powershell_never_inherits_a_stale_wsl_badge() {
        let mut native = session(
            "lamjed@DESKTOP: D:/workstation/projects",
            Some("Ubuntu-24.04"),
        );
        native.shell_name = Some("PowerShell".to_string());
        assert_eq!(immediate_os_value(&native), None);
        assert_eq!(shell_label(&native), "PowerShell");
    }

    #[test]
    fn command_prompt_is_a_native_windows_shell() {
        let mut native = session("CMD - D:/workstation/projects", Some("Ubuntu-24.04"));
        native.shell_name = Some("CMD".to_string());
        native.distro = None;
        assert_eq!(shell_label(&native), "CMD");
        assert!(is_native_windows_shell(&native));
        assert_eq!(immediate_os_value(&native), None);
    }

    #[test]
    fn wsl_title_and_distro_produce_the_real_distribution() {
        let wsl = session("lamjed@DESKTOP:/mnt/d/workstation", Some("Ubuntu-24.04"));
        assert_eq!(immediate_os_value(&wsl).as_deref(), Some("Ubuntu"));
    }

    #[test]
    fn command_duration_uses_compact_units() {
        assert_eq!(format_duration(18), "18ms");
        assert_eq!(format_duration(1_250), "1.2s");
        assert_eq!(format_duration(62_000), "1m 02s");
    }

    #[test]
    fn reference_icons_are_real_nerd_font_codepoints() {
        for kind in ALL_ICON_KINDS {
            assert!(icon_glyph(kind)
                .chars()
                .all(|character| character as u32 >= 0xe000));
        }
    }

    #[test]
    fn labels_truncate_on_unicode_boundaries() {
        assert_eq!(compact_label("dev-😀-cluster-name", 10), "dev-😀-clu…");
        assert_eq!(
            compact_middle("feature/very-long-branch", 12)
                .chars()
                .count(),
            12
        );
    }

    #[test]
    fn default_docker_context_uses_the_product_label() {
        assert_eq!(docker_value("default"), "docker");
        assert_eq!(docker_value("desktop-linux"), "desktop-linux");
    }

    #[test]
    fn live_segments_rebuild_only_when_their_inputs_change() {
        let session = session("amjed@host:/work", Some("Ubuntu"));
        let mut status = DevOpsStatus::default();
        status.ensure_live_segments(&session);
        let initial_revision = status.live_segments_revision;

        status.ensure_live_segments(&session);
        assert_eq!(status.live_segments_revision, initial_revision);

        status.snapshot.docker = Some("default".to_string());
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
    fn every_prompt_icon_has_bounded_optical_metrics() {
        let docker = icon_optics(IconKind::Docker);
        assert_eq!(docker.scale, 1.85);
        assert!(ALL_ICON_KINDS
            .iter()
            .all(|kind| docker.scale >= icon_optics(*kind).scale));

        for kind in ALL_ICON_KINDS {
            let optics = icon_optics(kind);
            assert!(
                (0.90..=1.90).contains(&optics.scale),
                "unsafe optical scale for {kind:?}: {}",
                optics.scale
            );
            let prompt_size = icon_font_size(PROMPT_CONTEXT_ICON_SIZE, kind);
            assert!(prompt_size > 0.0);

            // Point-size compensation keeps every glyph centered on the same
            // nominal box before its small intentional optical nudge.
            let prompt_center = icon_draw_y(20.0, PROMPT_CONTEXT_ICON_SIZE, kind)
                + prompt_size * 0.5
                - optics.y_shift;
            assert!(
                (prompt_center - (20.0 + PROMPT_CONTEXT_ICON_SIZE * 0.5)).abs() < 0.001,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn completed_refreshes_schedule_the_next_live_poll() {
        assert_eq!(next_context_wake_millis(true), 100);
        assert_eq!(next_context_wake_millis(false), LIVE_REFRESH_MILLIS);
    }

    #[test]
    fn stale_startup_snapshot_is_retried_without_waiting_for_timeout() {
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
    fn live_user_is_the_final_context_segment() {
        let session = session("amjed@host:/work", Some("Ubuntu"));
        let status = DevOpsStatus {
            snapshot: DevOpsSnapshot {
                docker: Some("default".to_string()),
                user: Some("amjed".to_string()),
                ..DevOpsSnapshot::default()
            },
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
    fn stable_prompt_identity_survives_reflow_key_changes() {
        assert!(same_prompt_identity(Some(7), 12, Some(7), 99));
        assert!(!same_prompt_identity(Some(7), 12, Some(8), 12));
        assert!(same_prompt_identity(None, 12, None, 12));
        assert!(!same_prompt_identity(None, 12, None, 99));
    }
}
