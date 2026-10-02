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

#[cfg(test)]
use automexia_extension_api::SegmentRole;
use automexia_extension_api::{ContextContribution, IconKind, SessionFacts};
pub(super) use automexia_ui_model::information_bar::prompt_tag_metrics;
use automexia_ui_model::information_bar::{
    tag_surface_paint_layers, BarVisualStyle, ResolvedBarItem, TagSurfaceGeometry,
};
use automexia_ui_model::{self, IconOptics, Segment};

use super::session_metadata::MetadataReadiness;
use crate::automexia::runtime;
use crate::automexia::ui::{PromptAnchor, MAX_PROMPT_CONTEXT_HISTORY};

#[cfg(feature = "native-gui-test-hooks")]
pub(crate) type NativePromptContextPaint = (Option<u64>, u64, [f32; 4]);

pub(crate) const LIVE_REFRESH_MILLIS: u64 = 3_000;
const REFRESH_INTERVAL: Duration = Duration::from_millis(LIVE_REFRESH_MILLIS);
const REFRESH_IN_FLIGHT_TIMEOUT: Duration = Duration::from_secs(10);
const ORDER: u8 = 19;
const PROMPT_TAG_LEFT_INSET: f32 = 2.0;

/// One already-projected prompt item and its row-projection geometry. Keeping
/// these inputs together prevents paint from consulting a newer context frame.
pub(super) struct PromptFragmentPaint<'a> {
    pub item: &'a ResolvedBarItem,
    pub visual: BarVisualStyle,
    pub anchor: &'a PromptAnchor,
    pub fragment: &'a crate::automexia::ui::command_info::Fragment,
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
    remote_active: bool,
    remote_context: Option<super::remote_session_metadata::RemotePresentation>,
    metadata_readiness: MetadataReadiness,
    contribution: Option<ContextContribution>,
    metadata_session: Option<usize>,
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
    pub(super) fn set_remote_context(
        &mut self,
        session: &SessionFacts,
        active: bool,
        remote: Option<&super::remote_session_metadata::RemotePresentation>,
    ) {
        if self.remote_active == active && self.remote_context.as_ref() == remote {
            return;
        }
        // Local provider leases and labels cannot cross an SSH or nested scope.
        // Historical prompt rows stay intact; only the live projection changes.
        self.set_metadata_readiness(session.session_id, MetadataReadiness::Unavailable);
        self.remote_active = active;
        self.remote_context = remote.cloned();
        if active && remote.is_some_and(|remote| remote.ready) {
            self.set_metadata_readiness(session.session_id, MetadataReadiness::Complete);
        }
        self.live_segments_session = None;
    }

    pub(super) fn set_metadata_readiness(
        &mut self,
        session_id: usize,
        readiness: super::session_metadata::MetadataReadiness,
    ) {
        use super::session_metadata::MetadataReadiness;
        if self
            .metadata_session
            .is_some_and(|owner| owner != session_id)
        {
            // The status is route-owned. A reused owner must not lend another
            // route live labels or completed prompt history.
            *self = Self::default();
        }
        self.metadata_session = Some(session_id);
        if self.metadata_readiness == readiness {
            return;
        }
        self.metadata_readiness = readiness;
        if readiness == MetadataReadiness::Unavailable {
            runtime::invalidate_devops_session(session_id);
            self.contribution = None;
            self.live_segments.clear();
            self.live_segments_session = None;
            self.live_segments_snapshot_revision = 0;
            self.live_segments_revision = self.live_segments_revision.wrapping_add(1);
            self.last_refresh_request = None;
            self.last_session = None;
            self.observed_global_generation = 0;
            self.snapshot_revision = 0;
            self.request_in_flight = false;
        } else if readiness == MetadataReadiness::Complete {
            // Even equal facts must refresh a prompt created while pending.
            self.live_segments_session = None;
        }
        if readiness != MetadataReadiness::Complete {
            self.refresh_pending = false;
        }
    }

    fn metadata_complete(&self) -> bool {
        self.metadata_readiness == super::session_metadata::MetadataReadiness::Complete
    }

    fn initial_prompt_segments(&self) -> Vec<Segment> {
        if self.metadata_complete() {
            self.live_segments.clone()
        } else {
            Vec::new()
        }
    }

    /// Revoking optional discovery must not erase renderer-owned OS/user tags
    /// or lend the next active session's identity to historical prompt rows.
    pub(super) fn clear_optional_contribution(&mut self) {
        fn core_identity(segment: &Segment) -> bool {
            matches!(
                segment.role,
                automexia_extension_api::SegmentRole::Windows
                    | automexia_extension_api::SegmentRole::UbuntuWsl
                    | automexia_extension_api::SegmentRole::User
            )
        }
        self.contribution = None;
        self.live_segments.retain(core_identity);
        self.live_segments_session = None;
        self.live_segments_snapshot_revision = 0;
        self.live_segments_revision = self.live_segments_revision.wrapping_add(1);
        for snapshot in &mut self.prompt_history {
            snapshot.segments.retain(core_identity);
        }
        if let Some(active) = &mut self.active_prompt {
            active.segments.retain(core_identity);
            active.segments_revision = 0;
        }
        self.last_refresh_request = None;
        self.last_session = None;
        self.observed_global_generation = 0;
        self.snapshot_revision = 0;
        self.refresh_pending = false;
        self.request_in_flight = false;
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

        if prompt_active && self.metadata_complete() {
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
                            segments: self.initial_prompt_segments(),
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
                segments: self.initial_prompt_segments(),
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
            || !self.metadata_complete()
        {
            if let Some(previous) = self.active_prompt.take() {
                self.remember_prompt(previous);
            }
            self.active_prompt = Some(ActivePrompt {
                session_id: session.session_id,
                generation: anchor.generation,
                key: anchor.key,
                segments: self.initial_prompt_segments(),
                segments_revision: self.live_segments_revision,
            });
            return true;
        }

        // Complete legacy integrations without `aid` cannot distinguish a
        // reflowed row from a new row by identity alone. Preserve that geometry
        // compatibility only while fresh metadata can identify the live shell.
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
        if self
            .metadata_session
            .is_some_and(|owner| owner != session_id)
        {
            return &[];
        }
        if let Some(active) = self.active_prompt.as_ref().filter(|active| {
            active.session_id == session_id
                && same_prompt_identity(
                    active.generation,
                    active.key,
                    anchor.generation,
                    anchor.key,
                )
        }) {
            return if self.live_segments_session.is_some() {
                &active.segments
            } else {
                &[]
            };
        }
        self.cached_segments(session_id, anchor).unwrap_or_else(|| {
            if self.metadata_complete() {
                &self.live_segments
            } else {
                &[]
            }
        })
    }

    pub(super) fn draw_prompt_fragment(
        &self,
        sugarloaf: &mut Sugarloaf,
        colors: Colors,
        appearance: &rio_backend::config::presentation::TagAppearance,
        inputs: PromptFragmentPaint<'_>,
    ) {
        let PromptFragmentPaint {
            item,
            visual,
            anchor,
            fragment,
        } = inputs;
        let Some(text) = item.value.get(fragment.bytes.clone()) else {
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
        let paint = prompt_bar_item_colors(colors, item, appearance, visual);
        let color = paint.foreground;
        draw_tag_surface_in_run(
            sugarloaf,
            visual,
            [x, y, fragment.width, metrics.height],
            paint,
            fragment.shape_position,
        );
        if let Some(anchor) = kubernetes_freshness_marker(item) {
            let marker =
                automexia_ui_model::context_tag_colors(colors.background.0, anchor, 0);
            let size = (metrics.height * 0.22).clamp(2.0, 4.0);
            sugarloaf.rounded_rect(
                None,
                x + fragment.width - size,
                y,
                size,
                size,
                marker.foreground,
                0.0,
                size * 0.5,
                ORDER,
            );
        }
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
            if let Some(kind) = item.icon {
                draw_icon_in_slot(
                    sugarloaf,
                    kind,
                    x + fragment.padding,
                    y + (metrics.height - icon) * 0.5,
                    slot,
                    icon,
                    color,
                );
            }
        }
        if !item.icon_only && !text.is_empty() {
            super::command_info::draw_fragment_text(
                sugarloaf.text_mut(),
                text,
                fragment,
                [anchor.x, anchor.y],
                [anchor.height, metrics.font_size, metrics.height],
                color_to_u8(color),
            );
        }
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
        if self.remote_active || !self.metadata_complete() {
            return;
        }
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
        if self.remote_active || !self.metadata_complete() {
            return;
        }
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
        if self.remote_active || !self.metadata_complete() {
            return;
        }
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
        if !self.metadata_complete() {
            return;
        }
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
        if self.remote_active {
            return self
                .remote_context
                .as_ref()
                .filter(|remote| remote.ready)
                .map_or_else(Vec::new, |remote| {
                    remote_segments(session.session_id, remote)
                });
        }
        self.contribution.as_ref().map_or_else(
            || automexia_ui_model::immediate_session_segments(session),
            |contribution| automexia_ui_model::project_status(session, contribution),
        )
    }
}

fn remote_segments(
    session_id: usize,
    remote: &super::remote_session_metadata::RemotePresentation,
) -> Vec<Segment> {
    use automexia_extension_api::{
        ExtensionId, Freshness, SegmentRole, SessionId, StatusSegment,
    };
    use automexia_ssh_integration::helper::ContextField as Discovered;
    use automexia_ssh_integration::session::RemoteContextField as Field;
    let value = |field| {
        remote
            .context
            .as_ref()
            .and_then(|context| context.value(field))
    };
    let mut segments = Vec::new();
    let discovered = |field| {
        remote
            .discovered
            .as_ref()
            .and_then(|context| context.value(field))
    };
    let mut push = |id: &str, label: String, accessible: String, role, icon, priority| {
        if let Ok(segment) = StatusSegment::new(
            id,
            label,
            accessible,
            role,
            icon,
            priority,
            if role == SegmentRole::Kubernetes {
                Freshness::Stale
            } else {
                Freshness::Current
            },
        ) {
            segments.push(segment);
        }
    };
    let environment = value(Field::Environment).unwrap_or(&remote.shell);
    push(
        "ssh",
        format!(
            "SSH · {}",
            automexia_ui_model::compact_label(environment, 20)
        ),
        format!("Remote SSH environment {environment}"),
        SegmentRole::Environment,
        IconKind::Environment,
        10,
    );
    for (field, id, role, icon, priority) in [
        (Field::GitBranch, "git", SegmentRole::Git, IconKind::Git, 30),
        (
            Field::DockerContext,
            "docker",
            SegmentRole::Docker,
            IconKind::Docker,
            50,
        ),
        (
            Field::TerraformWorkspace,
            "terraform",
            SegmentRole::Terraform,
            IconKind::Terraform,
            60,
        ),
    ] {
        if let Some(value) = value(field) {
            push(
                id,
                automexia_ui_model::compact_label(value, 24),
                format!("Remote {id} {value}"),
                role,
                icon,
                priority,
            );
        }
    }
    for (legacy, profile, region, id, role, priority) in [
        (
            Field::AwsProfile,
            Discovered::AwsProfile,
            Discovered::AwsRegion,
            "aws",
            SegmentRole::Aws,
            70,
        ),
        (
            Field::AzureCloud,
            Discovered::AzureSubscription,
            Discovered::AzureRegion,
            "azure",
            SegmentRole::Azure,
            71,
        ),
        (
            Field::GcpProject,
            Discovered::GcpProject,
            Discovered::GcpRegion,
            "gcp",
            SegmentRole::Gcp,
            72,
        ),
    ] {
        let selection = discovered(profile).or_else(|| value(legacy));
        let region = discovered(region);
        let label = match (selection, region) {
            (Some(selection), Some(region)) => format!("{selection} · {region}"),
            (Some(selection), None) => selection.to_owned(),
            (None, Some(region)) => region.to_owned(),
            (None, None) => continue,
        };
        push(
            id,
            automexia_ui_model::compact_label(&label, 24),
            format!("Remote {id} {label}"),
            role,
            IconKind::Cloud,
            priority,
        );
    }
    if discovered(Discovered::Production) == Some("1") {
        push(
            "production",
            "prod".into(),
            "Remote production context".into(),
            SegmentRole::Production,
            IconKind::Production,
            0,
        );
    }
    if let Some(context) = value(Field::KubernetesContext) {
        let namespace = value(Field::KubernetesNamespace);
        let combined = namespace.map_or_else(
            || context.to_owned(),
            |namespace| format!("{context} · {namespace}"),
        );
        let accessible = namespace.map_or_else(
            || format!("Remote Kubernetes context {context}"),
            |namespace| {
                format!("Remote Kubernetes context {context}, namespace {namespace}")
            },
        );
        push(
            "kubernetes",
            format!("{}?", automexia_ui_model::compact_label(&combined, 27)),
            format!(
                "{accessible}; remote configured selection, cluster existence unverified"
            ),
            SegmentRole::Kubernetes,
            IconKind::Kubernetes,
            40,
        );
    }
    if let Some(user) = &remote.user {
        push(
            "user",
            automexia_ui_model::compact_label(user, 16),
            format!("Remote user {user}"),
            SegmentRole::User,
            IconKind::User,
            90,
        );
    }
    let Ok(extension) = ExtensionId::new("automexia.ssh") else {
        return Vec::new();
    };
    let Ok(contribution) = ContextContribution::new(
        extension,
        SessionId::new(session_id as u64),
        remote.key.generation(),
        1,
        Freshness::Current,
        segments,
    ) else {
        return Vec::new();
    };
    automexia_ui_model::project_context_contribution(&contribution)
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

#[cfg(test)]
fn prompt_tag_colors(
    colors: Colors,
    role: SegmentRole,
    appearance: &rio_backend::config::presentation::TagAppearance,
) -> automexia_ui_model::ContextTagColors {
    use rio_backend::config::presentation::TagStyle;
    let opacity = match appearance.style {
        TagStyle::Plain => 0,
        TagStyle::Tinted => appearance.opacity.get(),
    };
    automexia_ui_model::context_tag_colors(
        colors.background.0,
        crate::automexia::presentation::tag_anchor(appearance, role),
        opacity,
    )
}

fn prompt_bar_item_colors(
    colors: Colors,
    item: &ResolvedBarItem,
    appearance: &rio_backend::config::presentation::TagAppearance,
    visual: BarVisualStyle,
) -> automexia_ui_model::ContextTagColors {
    use rio_backend::config::presentation::TagStyle;
    let foreground = color_to_u8(colors.foreground);
    let anchor = item.color.unwrap_or_else(|| {
        item.source_role
            .map_or([foreground[0], foreground[1], foreground[2]], |role| {
                crate::automexia::presentation::tag_anchor(appearance, role)
            })
    });
    let opacity =
        if appearance.style == TagStyle::Plain || visual == BarVisualStyle::Underline {
            0
        } else {
            appearance.opacity.get()
        };
    automexia_ui_model::context_tag_colors(colors.background.0, anchor, opacity)
}

/// An icon-only recipe has no text in which to show the unverified `?`.
/// Keep its freshness visible inside the existing tag rectangle.
fn kubernetes_freshness_marker(item: &ResolvedBarItem) -> Option<[u8; 3]> {
    if item.source_role != Some(automexia_extension_api::SegmentRole::Kubernetes) {
        return None;
    }
    match item.freshness {
        automexia_extension_api::Freshness::Current => None,
        automexia_extension_api::Freshness::Refreshing => Some([80, 213, 255]),
        automexia_extension_api::Freshness::Stale
        | automexia_extension_api::Freshness::Expired => Some([255, 194, 67]),
        automexia_extension_api::Freshness::Unavailable
        | automexia_extension_api::Freshness::Error => Some([255, 98, 115]),
    }
}

/// Every vertex stays inside the fragment rectangle supplied by the shared
/// packer. A shape never increases the prompt band or its terminal-grid rows.
trait TagSurfaceCanvas {
    fn fill_rounded(&mut self, rect: [f32; 4], radius: f32, color: [f32; 4]);
    fn fill_rect(&mut self, rect: [f32; 4], color: [f32; 4]);
    fn fill_polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]);
    fn stroke_line(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        color: [f32; 4],
    );
    fn stroke_arc(
        &mut self,
        center: (f32, f32),
        radius: f32,
        angles: (f32, f32),
        width: f32,
        color: [f32; 4],
    );
}

impl TagSurfaceCanvas for Sugarloaf<'_> {
    fn fill_rounded(
        &mut self,
        [x, y, width, height]: [f32; 4],
        radius: f32,
        color: [f32; 4],
    ) {
        self.rounded_rect(None, x, y, width, height, color, 0.0, radius, ORDER - 1);
    }

    fn fill_rect(&mut self, [x, y, width, height]: [f32; 4], color: [f32; 4]) {
        self.rect(None, x, y, width, height, color, 0.0, ORDER - 1);
    }

    fn fill_polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
        self.polygon_with_order(points, 0.0, color, ORDER - 1);
    }

    fn stroke_line(
        &mut self,
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        color: [f32; 4],
    ) {
        self.line(from.0, from.1, to.0, to.1, width, 0.0, color, ORDER - 1);
    }

    fn stroke_arc(
        &mut self,
        center: (f32, f32),
        radius: f32,
        angles: (f32, f32),
        width: f32,
        color: [f32; 4],
    ) {
        self.arc(
            center.0, center.1, radius, angles.0, angles.1, width, 0.0, color,
        );
    }
}

#[cfg(test)]
fn draw_tag_surface(
    sugarloaf: &mut impl TagSurfaceCanvas,
    visual: BarVisualStyle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    paint: automexia_ui_model::ContextTagColors,
) {
    draw_tag_surface_in_run(
        sugarloaf,
        visual,
        [x, y, width, height],
        paint,
        Default::default(),
    );
}

fn draw_tag_surface_in_run(
    sugarloaf: &mut impl TagSurfaceCanvas,
    visual: BarVisualStyle,
    [x, y, width, height]: [f32; 4],
    paint: automexia_ui_model::ContextTagColors,
    position: automexia_ui_model::information_bar::TagShapePosition,
) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    let Some(geometry) = automexia_ui_model::information_bar::tag_fragment_geometry(
        visual, width, height, position,
    ) else {
        return;
    };
    let mut outline = paint.foreground;
    outline[3] = 0.45;
    let layers = tag_surface_paint_layers(visual, paint.background[3]);
    match geometry {
        TagSurfaceGeometry::Connected(shape) => {
            let offset = |p: (f32, f32)| (x + p.0, y + p.1);
            if layers.fill {
                for triangle in shape.triangles() {
                    sugarloaf.fill_polygon(&triangle.map(offset), paint.background);
                }
            } else {
                for (a, b) in shape
                    .points()
                    .iter()
                    .zip(shape.points().iter().cycle().skip(1))
                {
                    sugarloaf.stroke_line(offset(*a), offset(*b), shape.stroke, outline);
                }
            }
            if layers.fill {
                if let Some(fold) = shape.fold {
                    let color = [
                        paint.background[0] * 0.55,
                        paint.background[1] * 0.55,
                        paint.background[2] * 0.55,
                        paint.background[3],
                    ];
                    sugarloaf.fill_polygon(&fold.map(offset), color);
                }
            }
        }
        TagSurfaceGeometry::Capsule { radius, stroke } => {
            if layers.fill {
                sugarloaf.fill_rounded([x, y, width, height], radius, paint.background);
            } else {
                draw_rounded_outline(
                    sugarloaf, x, y, width, height, radius, stroke, outline,
                );
            }
        }
        TagSurfaceGeometry::Flat { stroke } => {
            if layers.fill {
                sugarloaf.fill_rect([x, y, width, height], paint.background);
            } else {
                draw_rect_outline(sugarloaf, x, y, width, height, stroke, outline);
            }
        }
        TagSurfaceGeometry::Chevron { points, stroke }
        | TagSurfaceGeometry::Hexagon { points, stroke } => {
            let points = points.map(|(px, py)| (x + px, y + py));
            if layers.fill {
                sugarloaf.fill_polygon(&points, paint.background);
            } else {
                draw_polygon_outline(sugarloaf, &points, stroke, outline);
            }
        }
        TagSurfaceGeometry::Card { radius, stroke } => {
            if layers.fill {
                sugarloaf.fill_rounded([x, y, width, height], radius, paint.background);
            }
            draw_rounded_outline(sugarloaf, x, y, width, height, radius, stroke, outline);
        }
        TagSurfaceGeometry::Underline { baseline_y, stroke } => {
            sugarloaf.stroke_line(
                (x + stroke * 0.5, y + baseline_y),
                (x + width - stroke * 0.5, y + baseline_y),
                stroke,
                paint.foreground,
            );
        }
    }
}

fn draw_polygon_outline(
    sugarloaf: &mut impl TagSurfaceCanvas,
    points: &[(f32, f32)],
    stroke: f32,
    color: [f32; 4],
) {
    for index in 0..points.len() {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        sugarloaf.stroke_line(from, to, stroke, color);
    }
}

fn draw_rect_outline(
    sugarloaf: &mut impl TagSurfaceCanvas,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    stroke: f32,
    color: [f32; 4],
) {
    let inset = stroke * 0.5;
    let points = [
        (x + inset, y + inset),
        (x + width - inset, y + inset),
        (x + width - inset, y + height - inset),
        (x + inset, y + height - inset),
    ];
    for index in 0..points.len() {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        sugarloaf.stroke_line(from, to, stroke, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rounded_outline(
    sugarloaf: &mut impl TagSurfaceCanvas,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    stroke: f32,
    color: [f32; 4],
) {
    if radius < stroke * 0.5 {
        draw_rect_outline(sugarloaf, x, y, width, height, stroke, color);
        return;
    }
    let inset = stroke * 0.5;
    let left = x + inset + radius;
    let right = x + width - inset - radius;
    let top = y + inset + radius;
    let bottom = y + height - inset - radius;
    for (from, to) in [
        ((left, y + inset), (right, y + inset)),
        ((x + width - inset, top), (x + width - inset, bottom)),
        ((right, y + height - inset), (left, y + height - inset)),
        ((x + inset, bottom), (x + inset, top)),
    ] {
        sugarloaf.stroke_line(from, to, stroke, color);
    }
    for (cx, cy, start, end) in [
        (right, top, -90.0, 0.0),
        (right, bottom, 0.0, 90.0),
        (left, bottom, 90.0, 180.0),
        (left, top, 180.0, 270.0),
    ] {
        sugarloaf.stroke_arc((cx, cy), radius, (start, end), stroke, color);
    }
}

#[cfg(test)]
fn segment_color(colors: Colors, role: SegmentRole) -> [f32; 4] {
    prompt_tag_colors(colors, role, &Default::default()).foreground
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

    #[derive(Default)]
    struct ShapeRecorder {
        fills: Vec<[f32; 4]>,
        polygons: Vec<Vec<(f32, f32)>>,
        strokes: usize,
    }

    impl TagSurfaceCanvas for ShapeRecorder {
        fn fill_rounded(&mut self, _rect: [f32; 4], _radius: f32, color: [f32; 4]) {
            self.fills.push(color);
        }

        fn fill_rect(&mut self, _rect: [f32; 4], color: [f32; 4]) {
            self.fills.push(color);
        }

        fn fill_polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
            self.polygons.push(points.to_vec());
            self.fills.push(color);
        }

        fn stroke_line(
            &mut self,
            _from: (f32, f32),
            _to: (f32, f32),
            _width: f32,
            _color: [f32; 4],
        ) {
            self.strokes += 1;
        }

        fn stroke_arc(
            &mut self,
            _center: (f32, f32),
            _radius: f32,
            _angles: (f32, f32),
            _width: f32,
            _color: [f32; 4],
        ) {
            self.strokes += 1;
        }
    }

    #[test]
    fn every_plain_shape_has_a_real_outline_without_any_fill() {
        let plain = automexia_ui_model::ContextTagColors {
            foreground: [0.8, 0.9, 1.0, 1.0],
            background: [0.1, 0.3, 0.5, 0.0],
        };
        for style in BarVisualStyle::ALL {
            let mut recorder = ShapeRecorder::default();
            draw_tag_surface(&mut recorder, style, 2.0, 3.0, 80.0, 18.0, plain);
            assert!(recorder.fills.is_empty(), "{style:?} painted a plain fill");
            assert!(recorder.strokes > 0, "{style:?} lost its silhouette");
        }
    }

    #[test]
    fn connected_terminal_shapes_keep_their_cutouts_and_exact_tint_alpha() {
        use automexia_ui_model::information_bar::{
            tag_fragment_geometry, TagShapePosition,
        };
        for style in BarVisualStyle::ALL
            .into_iter()
            .filter(|style| !style.is_legacy())
        {
            for ordinal in 0..3 {
                let position = TagShapePosition {
                    first: ordinal == 0,
                    last: ordinal == 2,
                    ordinal,
                    padding: 12.0,
                };
                let TagSurfaceGeometry::Connected(shape) =
                    tag_fragment_geometry(style, 90.0, 24.0, position).unwrap()
                else {
                    panic!("connected style lost its geometry")
                };
                for alpha in [0.12, 0.75, 1.0] {
                    let paint = automexia_ui_model::ContextTagColors {
                        foreground: [0.8, 0.9, 1.0, 1.0],
                        background: [0.1, 0.3, 0.5, alpha],
                    };
                    let mut recorder = ShapeRecorder::default();
                    draw_tag_surface_in_run(
                        &mut recorder,
                        style,
                        [2.0, 3.0, 90.0, 24.0],
                        paint,
                        position,
                    );
                    let mut expected: Vec<Vec<_>> = shape
                        .triangles()
                        .map(|triangle| triangle.map(|p| (p.0 + 2.0, p.1 + 3.0)).to_vec())
                        .collect();
                    if let Some(fold) = shape.fold {
                        expected.push(fold.map(|p| (p.0 + 2.0, p.1 + 3.0)).to_vec());
                    }
                    assert_eq!(
                        recorder.polygons, expected,
                        "{style:?}, position {ordinal}"
                    );
                    assert!(recorder.fills.iter().all(|color| color[3] == alpha));
                }
            }
        }
    }

    #[test]
    fn tinted_card_keeps_exact_tint_and_real_outline_while_chevron_uses_notch() {
        let tinted = automexia_ui_model::ContextTagColors {
            foreground: [0.8, 0.9, 1.0, 1.0],
            background: [0.1, 0.3, 0.5, 0.12],
        };
        let mut card = ShapeRecorder::default();
        draw_tag_surface(
            &mut card,
            BarVisualStyle::Card,
            2.0,
            3.0,
            80.0,
            18.0,
            tinted,
        );
        assert_eq!(card.fills, vec![tinted.background]);
        assert!(card.strokes >= 4);

        let mut chevron = ShapeRecorder::default();
        draw_tag_surface(
            &mut chevron,
            BarVisualStyle::Chevron,
            2.0,
            3.0,
            80.0,
            18.0,
            tinted,
        );
        assert_eq!(chevron.fills, vec![tinted.background]);
        assert_eq!(chevron.polygons.len(), 1);
        let notch = chevron.polygons[0][0];
        assert!(notch.0 > 2.0 && notch.1 > 3.0);
        assert!(chevron.polygons[0].iter().all(|point| {
            point.0 >= 2.0 && point.0 <= 82.0 && point.1 >= 3.0 && point.1 <= 21.0
        }));
    }

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

    fn history_anchor() -> PromptAnchor {
        PromptAnchor {
            generation: Some(7),
            key: 3,
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 24.0,
        }
    }

    fn history_status(route: usize, historical: &str, live: &str) -> DevOpsStatus {
        let mut facts = session("", Some("Ubuntu"));
        facts.session_id = route;
        let mut status = DevOpsStatus::default();
        for (revision, value) in [(1, historical), (2, live)] {
            status.contribution = Some(
                ContextContribution::new(
                    ExtensionId::new("automexia.devops").unwrap(),
                    SessionId::new(route as u64),
                    1,
                    revision,
                    Freshness::Current,
                    vec![status_segment(
                        "kubernetes",
                        value,
                        SegmentRole::Kubernetes,
                        IconKind::Kubernetes,
                        100,
                    )],
                )
                .unwrap(),
            );
            status.snapshot_revision = revision as u32;
            if revision == 1 {
                status.prepare_prompt_rows(&facts, true, &[], Some(history_anchor()));
                status.prepare_prompt_rows(&facts, false, &[history_anchor()], None);
            } else {
                status.ensure_live_segments(&facts);
            }
        }
        status.last_session = Some(facts);
        status.last_refresh_request = Some(Instant::now());
        status.request_in_flight = true;
        status
    }

    fn historical_value(status: &DevOpsStatus, route: usize) -> Option<&str> {
        status
            .segments_for_prompt(route, &history_anchor())
            .iter()
            .find(|segment| segment.role == SegmentRole::Kubernetes)
            .map(|segment| segment.value.as_str())
    }

    #[test]
    fn ssh_scope_blocks_host_discovery_and_cached_contributions_even_when_remote_ready() {
        let mut facts = session("fixture-local", Some("Fixture-Linux"));
        facts.session_id = 10;
        facts.cwd = Some("/fixture/local".into());
        facts.shell_user = Some("local-user".into());
        let mut status = history_status(10, "local-history", "local-live");
        let remote = super::super::remote_session_metadata::RemotePresentation {
            key: automexia_ssh_integration::GenerationKey::new(1, 2).unwrap(),
            shell: "bash".into(),
            ready: true,
            directory: None,
            user: Some("remote-user".into()),
            context: None,
            discovered: None,
        };
        status.set_remote_context(&facts, true, Some(&remote));
        assert!(!status.refresh_session_context(&facts, || panic!(
            "remote scope ran host provider"
        )));
        status
            .request_prompt_refresh(&facts, || panic!("remote prompt ran host provider"));
        assert!(!status.refresh_visible_session(&facts, || panic!(
            "remote inactive pane ran host provider"
        )));
        status.accept_cached_snapshot(
            &facts,
            99,
            Some(&facts),
            contribution(vec![status_segment(
                "kubernetes",
                "injected-local",
                SegmentRole::Kubernetes,
                IconKind::Kubernetes,
                100,
            )]),
        );
        assert!(status.contribution.is_none());
        assert!(!status.request_in_flight);
        assert_eq!(historical_value(&status, 10), Some("local-history"));
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.value == "SSH · bash"));
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.value == "remote-user"));
        assert!(!status.live_segments.iter().any(|segment| matches!(
            segment.role,
            SegmentRole::Windows
                | SegmentRole::UbuntuWsl
                | SegmentRole::Kubernetes
                | SegmentRole::Git
        )));
        assert!(!status
            .live_segments
            .iter()
            .any(|segment| segment.value == "local-user"));
        status.set_remote_context(&facts, false, None);
        status.set_metadata_readiness(10, MetadataReadiness::Complete);
        status.ensure_live_segments(&facts);
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.value == "local-user"));
        assert!(!status
            .live_segments
            .iter()
            .any(|segment| segment.value == "remote-user"));
        assert_eq!(historical_value(&status, 10), Some("local-history"));
    }

    #[test]
    fn ssh_scope_unready_and_quarantine_have_no_local_live_identity() {
        let mut facts = session("fixture-local", None);
        facts.session_id = 10;
        let mut status = history_status(10, "local-history", "local-live");
        status.set_remote_context(&facts, true, None);
        status.refresh_session_context(&facts, || {
            panic!("unready scope ran host provider")
        });
        assert!(status.live_segments.is_empty());
        let pending = super::super::remote_session_metadata::RemotePresentation {
            key: automexia_ssh_integration::GenerationKey::new(1, 2).unwrap(),
            shell: "pwsh".into(),
            ready: false,
            directory: None,
            user: None,
            context: None,
            discovered: None,
        };
        status.set_remote_context(&facts, true, Some(&pending));
        status.refresh_visible_session(&facts, || {
            panic!("pending scope ran host provider")
        });
        assert!(status.live_segments.is_empty());
        assert_eq!(historical_value(&status, 10), Some("local-history"));
    }

    #[test]
    fn ssh_scope_projects_scoped_context_roles_and_clears_changed_values() {
        use automexia_ssh_integration::{session::RemoteContext, GenerationKey};
        let key = GenerationKey::new(1, 2).unwrap();
        let snapshot = "AMXSSHCTX1|1|2|\ngit_branch=remote-main\nkubernetes_context=fixture-cluster\nkubernetes_namespace=fixture-ns\ndocker_context=fixture-docker\nterraform_workspace=fixture-workspace\nenvironment=staging\naws_profile=fixture-aws\nazure_cloud=fixture-azure\ngcp_project=fixture-gcp\n";
        let mut remote = super::super::remote_session_metadata::RemotePresentation {
            key,
            shell: "bash".into(),
            ready: true,
            directory: None,
            user: Some("remote-user".into()),
            context: RemoteContext::decode(key, snapshot).unwrap(),
            discovered: None,
        };
        let facts = session("", None);
        let mut status = DevOpsStatus::default();
        status.set_remote_context(&facts, true, Some(&remote));
        status.ensure_live_segments(&facts);
        assert_eq!(status.live_segments.len(), 9);
        for role in [
            SegmentRole::Git,
            SegmentRole::Kubernetes,
            SegmentRole::Docker,
            SegmentRole::Terraform,
            SegmentRole::Environment,
            SegmentRole::Aws,
            SegmentRole::Azure,
            SegmentRole::Gcp,
            SegmentRole::User,
        ] {
            assert_eq!(
                status
                    .live_segments
                    .iter()
                    .filter(|segment| segment.role == role)
                    .count(),
                1
            );
        }
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.accessibility_label.contains("namespace fixture-ns")));
        assert!(status
            .live_segments
            .iter()
            .any(|segment| segment.value == "SSH · staging"));
        remote.context = None;
        status.set_remote_context(&facts, true, Some(&remote));
        status.ensure_live_segments(&facts);
        assert_eq!(status.live_segments.len(), 2);
        assert!(!status
            .live_segments
            .iter()
            .any(|segment| segment.role == SegmentRole::Kubernetes));
    }

    #[test]
    fn ssh_discovered_clouds_keep_regions_and_production_without_duplicate_tags() {
        use automexia_ssh_integration::{
            helper::{ContextField, ContextUpdate},
            GenerationKey,
        };
        let key = GenerationKey::new(1, 2).unwrap();
        let mut discovered = ContextUpdate::new(key, 3).unwrap();
        for (field, value) in [
            (ContextField::AzureCloud, "fixture-cloud"),
            (ContextField::AzureSubscription, "fixture-subscription"),
            (ContextField::AzureRegion, "region-a"),
            (ContextField::AwsProfile, "fixture-profile"),
            (ContextField::AwsRegion, "region-b"),
            (ContextField::GcpProject, "fixture-project"),
            (ContextField::GcpRegion, "region-c"),
            (ContextField::Production, "1"),
        ] {
            discovered.set(field, value).unwrap();
        }
        let mut remote = super::super::remote_session_metadata::RemotePresentation {
            key,
            shell: "bash".into(),
            ready: true,
            directory: None,
            user: None,
            context: Some(discovered.base_context()),
            discovered: Some(discovered),
        };
        let segments = remote_segments(1, &remote);
        for (role, label) in [
            (SegmentRole::Azure, "fixture-subscription · region-a"),
            (SegmentRole::Aws, "fixture-profile · region-b"),
            (SegmentRole::Gcp, "fixture-project · region-c"),
        ] {
            let matching: Vec<_> = segments
                .iter()
                .filter(|segment| segment.role == role)
                .collect();
            assert_eq!(matching.len(), 1);
            assert!(matching[0].accessibility_label.ends_with(label));
        }
        assert_eq!(
            segments
                .iter()
                .filter(|segment| segment.role == SegmentRole::Production)
                .count(),
            1
        );
        assert!(!segments
            .iter()
            .any(|segment| segment.accessibility_label.contains("fixture-cloud")));
        remote.context = None;
        remote.discovered = None;
        let cleared = remote_segments(1, &remote);
        assert_eq!(cleared.len(), 1);
        assert_eq!(cleared[0].role, SegmentRole::Environment);
    }

    #[test]
    fn disabling_optional_discovery_keeps_historical_core_identity_only() {
        let mut status = history_status(10, "old-namespace", "new-namespace");
        assert_eq!(historical_value(&status, 10), Some("old-namespace"));
        status.clear_optional_contribution();
        let historical = status.segments_for_prompt(10, &history_anchor());
        assert!(historical
            .iter()
            .any(|segment| segment.role == SegmentRole::UbuntuWsl));
        assert!(!historical
            .iter()
            .any(|segment| segment.role == SegmentRole::Kubernetes));
        assert!(!status.request_in_flight);
        assert!(!status.refresh_pending);
    }

    fn dead_history_context(
        route: usize,
    ) -> crate::context::Context<rio_backend::event::VoidListener> {
        crate::context::create_dead_context(
            rio_backend::event::VoidListener {},
            rio_backend::event::WindowId::from(0),
            route,
            route,
            crate::context::ContextDimension::default(),
        )
    }

    fn history_manager(
    ) -> crate::context::ContextManager<rio_backend::event::VoidListener> {
        let mut manager = crate::context::ContextManager::start_with_capacity(
            4,
            rio_backend::event::VoidListener {},
            rio_backend::event::WindowId::from(0),
        )
        .unwrap();
        manager.contexts_mut().clear();
        manager.contexts_mut().push(history_grid(10));
        manager.select_route_from_current_grid();
        manager
    }

    fn history_grid(
        route: usize,
    ) -> crate::context::ContextGrid<rio_backend::event::VoidListener> {
        crate::context::ContextGrid::new(
            dead_history_context(route),
            rio_backend::config::layout::Margin::default(),
            [0.0; 4],
            [0.0; 4],
            rio_backend::config::layout::Panel::default(),
        )
    }

    fn core_prompt_pane(
        route: usize,
        user: &str,
        distro: Option<&str>,
        is_active: bool,
    ) -> super::super::SemanticPaneRenderState {
        let mut session = session("", distro);
        session.session_id = route;
        session.shell_user = Some(user.into());
        super::super::SemanticPaneRenderState {
            origin_y: 0.0,
            bottom_y: 240.0,
            cell_height: 24.0,
            completion_labels: Vec::new(),
            session,
            metadata_readiness:
                super::super::session_metadata::MetadataReadiness::Complete,
            remote_context: None,
            remote_active: false,
            prompt_active: true,
            historical_anchors: Vec::new(),
            live_anchor: Some(history_anchor()),
            command_results: Vec::new(),
            output_background_protected: Vec::new(),
            allow_result_animation: false,
            is_active,
        }
    }

    #[test]
    fn core_prompt_tags_keep_route_state_without_optional_devops() {
        let mut manager = history_manager();
        let mut renderer =
            super::super::Renderer::new(&rio_backend::config::Config::default());
        renderer.devops_context_enabled = false;
        renderer.sync_devops_routes(&manager);
        assert_eq!(renderer.devops_status_route, Some(10));
        assert!(manager
            .current_grid_mut()
            .split_right_core(dead_history_context(20)));
        assert!(manager.current_grid_mut().select_active_route(10));
        let host = core_prompt_pane(10, "alice", None, true);
        let guest = core_prompt_pane(20, "bob", Some("Ubuntu"), false);
        renderer.prepare_prompt_context(&mut manager, &host, &[guest]);
        let host_segments = renderer
            .prompt_status_for_route(10, 10)
            .unwrap()
            .segments_for_prompt(10, &host.live_anchor.unwrap());
        assert!(host_segments.iter().any(|s| s.role == SegmentRole::Windows));
        assert!(host_segments.iter().any(|s| s.value == "alice"));
        assert!(!host_segments.iter().any(|s| s.value == "bob"));
        let guest_segments = renderer
            .prompt_status_for_route(20, 10)
            .unwrap()
            .segments_for_prompt(20, &history_anchor());
        assert!(guest_segments
            .iter()
            .any(|s| s.role == SegmentRole::UbuntuWsl));
        assert!(guest_segments.iter().any(|s| s.value == "bob"));
        assert!(!guest_segments.iter().any(|s| s.value == "alice"));

        assert!(manager.current_grid_mut().select_active_route(20));
        let guest = core_prompt_pane(20, "bob", Some("Ubuntu"), true);
        let host = core_prompt_pane(10, "alice", None, false);
        renderer.prepare_prompt_context(&mut manager, &guest, &[host]);
        assert_eq!(renderer.devops_status_route, Some(20));
        assert!(renderer.devops_statuses.contains_key(&10));
        assert!(renderer
            .prompt_status_for_route(20, 20)
            .unwrap()
            .segments_for_prompt(20, &history_anchor())
            .iter()
            .any(|s| s.value == "bob"));
    }

    #[test]
    fn context_ownership_unchanged_active_route_retains_history_and_refresh_lease() {
        let manager = history_manager();
        let mut renderer =
            super::super::Renderer::new(&rio_backend::config::Config::default());
        renderer.devops_context_enabled = true;
        renderer.sync_devops_routes(&manager);
        renderer.devops_status = history_status(10, "alpha-old", "alpha-now");
        let original_request = renderer.devops_status.last_refresh_request;
        for _ in 0..3 {
            renderer.sync_devops_routes(&manager);
            assert_eq!(
                historical_value(&renderer.devops_status, 10),
                Some("alpha-old")
            );
            assert_eq!(
                renderer.devops_status.last_refresh_request,
                original_request
            );
            assert!(renderer.devops_status.request_in_flight);
            assert!(renderer.devops_statuses.is_empty());
        }
    }

    #[test]
    fn context_ownership_split_focus_round_trip_retains_each_prompt_and_refresh_lease() {
        let mut manager = history_manager();
        assert!(manager
            .current_grid_mut()
            .split_right_core(dead_history_context(20)));
        assert!(manager.current_grid_mut().select_active_route(10));
        let mut renderer =
            super::super::Renderer::new(&rio_backend::config::Config::default());
        renderer.devops_context_enabled = true;
        renderer.sync_devops_routes(&manager);
        renderer.devops_status = history_status(10, "alpha-old", "alpha-now");
        renderer
            .devops_statuses
            .insert(20, history_status(20, "beta-old", "beta-now"));
        let original_request = renderer.devops_status.last_refresh_request;
        assert!(manager.current_grid_mut().select_active_route(20));
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_status, 20),
            Some("beta-old")
        );
        assert!(manager.current_grid_mut().select_active_route(10));
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_status, 10),
            Some("alpha-old")
        );
        assert_eq!(
            historical_value(&renderer.devops_statuses[&20], 20),
            Some("beta-old")
        );
        assert_eq!(
            renderer.devops_status.last_refresh_request,
            original_request
        );
        assert!(renderer.devops_status.request_in_flight);
        assert_eq!(renderer.devops_statuses.len(), 1);
    }

    #[test]
    fn context_ownership_hidden_local_tab_retains_history_and_closed_tab_is_pruned() {
        let mut manager = history_manager();
        let mut renderer =
            super::super::Renderer::new(&rio_backend::config::Config::default());
        renderer.devops_context_enabled = true;
        renderer.sync_devops_routes(&manager);
        renderer.devops_status = history_status(10, "alpha-old", "alpha-now");
        manager
            .current_grid_mut()
            .contexts_mut()
            .values_mut()
            .next()
            .unwrap()
            .push_tab_core(dead_history_context(20));
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_statuses[&10], 10),
            Some("alpha-old")
        );
        renderer.devops_status = history_status(20, "beta-old", "beta-now");
        assert!(manager.current_grid_mut().remove_parked_route(20));
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_status, 10),
            Some("alpha-old")
        );
        assert!(renderer.devops_statuses.is_empty());
    }

    #[test]
    fn context_ownership_hidden_top_level_tab_retained_only_while_route_is_live() {
        let mut manager = history_manager();
        manager.contexts_mut().push(history_grid(20));
        let mut renderer =
            super::super::Renderer::new(&rio_backend::config::Config::default());
        renderer.devops_context_enabled = true;
        renderer.sync_devops_routes(&manager);
        renderer.devops_status = history_status(10, "alpha-old", "alpha-now");
        renderer
            .devops_statuses
            .insert(20, history_status(20, "beta-old", "beta-now"));
        renderer
            .devops_statuses
            .insert(99, history_status(99, "closed", "closed"));
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_statuses[&20], 20),
            Some("beta-old")
        );
        assert!(!renderer.devops_statuses.contains_key(&99));
        manager.set_current(1);
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_status, 20),
            Some("beta-old")
        );
        manager.set_current(0);
        manager.contexts_mut().pop();
        renderer.sync_devops_routes(&manager);
        assert_eq!(
            historical_value(&renderer.devops_status, 10),
            Some("alpha-old")
        );
        assert!(renderer.devops_statuses.is_empty());
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
            compact_label("dev-\u{1f600}-cluster-name", 10),
            automexia_ui_model::compact_label("dev-\u{1f600}-cluster-name", 10)
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
    #[test]
    fn metadata_readiness_other_route_cannot_borrow_live_segments() {
        let mut status = history_status(417, "historic", "live");
        status.set_metadata_readiness(417, MetadataReadiness::Complete);
        assert!(status
            .segments_for_prompt(418, &history_anchor())
            .is_empty());
        assert_eq!(historical_value(&status, 417), Some("historic"));
    }

    #[test]
    fn metadata_readiness_repeated_pending_does_not_revoke_each_frame() {
        let mut status = history_status(419, "historic", "live");
        status.set_metadata_readiness(419, MetadataReadiness::Pending);
        let revision = status.live_segments_revision;
        for _ in 0..128 {
            status.set_metadata_readiness(419, MetadataReadiness::Pending);
            assert_eq!(status.live_segments_revision, revision);
        }
    }

    mod metadata_readiness {
        include!("devops_metadata_readiness_tests.rs");
    }
}

#[cfg(test)]
mod visual_tag_render_tests {
    use super::*;
    use rio_backend::config::{presentation::TagStyle, Config};

    #[test]
    fn icon_only_kubernetes_tag_keeps_unverified_state_visible() {
        let mut item = ResolvedBarItem {
            slot_id: "cluster".into(),
            value: String::new(),
            icon_only: true,
            accessibility_label:
                "Configured Kubernetes namespace sandbox; cluster existence unverified"
                    .into(),
            source_role: Some(SegmentRole::Kubernetes),
            icon: Some(IconKind::Kubernetes),
            lane: automexia_ui_model::information_bar::BarLane::Leading,
            color: None,
            freshness: automexia_extension_api::Freshness::Stale,
            observed_at_ms: 1,
            details_action: None,
        };
        assert!(kubernetes_freshness_marker(&item).is_some());
        item.freshness = automexia_extension_api::Freshness::Current;
        assert!(kubernetes_freshness_marker(&item).is_none());
        item.source_role = Some(SegmentRole::Docker);
        item.freshness = automexia_extension_api::Freshness::Stale;
        assert!(kubernetes_freshness_marker(&item).is_none());
    }

    #[test]
    fn visual_tag_render_custom_anchor_and_opacity_reach_prompt_paint_data() {
        let config: Config = toml::from_str("[presentation.tags]\nopacity = 40\n[presentation.tags.colors]\nkubernetes = '#f1c284'\n").unwrap();
        let renderer = super::super::Renderer::new(&config);
        let expected = automexia_ui_model::context_tag_colors(
            renderer.named_colors.background.0,
            [241, 194, 132],
            40,
        );
        assert_eq!(
            prompt_tag_colors(
                renderer.named_colors,
                SegmentRole::Kubernetes,
                &renderer.presentation.tags
            )
            .foreground,
            expected.foreground
        );
        assert_eq!(
            prompt_tag_colors(
                renderer.named_colors,
                SegmentRole::Kubernetes,
                &renderer.presentation.tags
            )
            .background,
            expected.background
        );
    }
    #[test]
    fn visual_tag_render_plain_and_zero_opacity_preserve_readable_foreground() {
        for style in ["plain", "tinted"] {
            let opacity = if style == "plain" { 61 } else { 0 };
            let config: Config = toml::from_str(&format!(
                "[presentation.tags]\nstyle = '{style}'\nopacity = {opacity}\n"
            ))
            .unwrap();
            assert_eq!(
                config.presentation.tags.style == TagStyle::Plain,
                style == "plain"
            );
            let renderer = super::super::Renderer::new(&config);
            for role in [SegmentRole::Kubernetes, SegmentRole::User] {
                let expected = automexia_ui_model::context_tag_colors(
                    renderer.named_colors.background.0,
                    segment_anchor_rgb(role),
                    0,
                );
                assert_eq!(
                    prompt_tag_colors(
                        renderer.named_colors,
                        role,
                        &renderer.presentation.tags
                    )
                    .background,
                    expected.background,
                    "role {role:?}, style {style}"
                );
                assert_eq!(
                    prompt_tag_colors(
                        renderer.named_colors,
                        role,
                        &renderer.presentation.tags
                    )
                    .foreground,
                    expected.foreground
                );
            }
        }
    }
}
