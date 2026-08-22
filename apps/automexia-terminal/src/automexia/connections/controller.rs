//! Route-local, renderer-neutral controller for the read-only Connection Hub.

use std::{path::PathBuf, sync::Arc};

use automexia_devops::connections::DirectOpenSshPreparation;
use automexia_devops_ssh::GrantKind;
use automexia_extension_runtime::CompletionWake;

use super::direct_openssh::{
    prepare_literal_direct_openssh, validate_literal_direct_openssh_destination,
};
use automexia_ui_model::connection_hub::{
    apply_hub_key, hub_catalog_controls_visible, project_connection_catalog,
    project_connection_hub, project_direct_openssh_preparation,
    validate_connection_catalog_query, ConnectionCatalogEntry,
    ConnectionCatalogProjection, ConnectionCatalogQuery, ConnectionHubView,
    ConnectionReviewView, ConnectionSummary, HubCatalogGrouping, HubCatalogSource,
    HubContentState, HubFocus, HubKey, HubProjectionRequest, HubRoute,
    HubVisualPreferences, InteractionEffect, InteractionState, Viewport,
    MAX_CATALOG_QUERY_BYTES, MAX_VISIBLE_ROWS,
};

use super::{
    platform_setup_guidance, ConnectionHubRuntime, GrantReviewState, HubLibrarySnapshot,
    HubMetadataChangeState, HubRuntimeErrorCode, HubRuntimeSnapshot, HubRuntimeState,
    HubStoreState, MetadataChangeReview, PlatformFamily, SetupGuidance,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisabledHubAction {
    pub label: &'static str,
    pub reason: &'static str,
    pub disabled: bool,
}

const DISABLED_ACTIONS: [DisabledHubAction; 4] = [
    DisabledHubAction {
        label: "Connect",
        reason: "Connection execution is unavailable in this read-only phase",
        disabled: true,
    },
    DisabledHubAction {
        label: "Login",
        reason: "Authentication is unavailable in this read-only phase",
        disabled: true,
    },
    DisabledHubAction {
        label: "Refresh cloud providers",
        reason: "Provider and network access are unavailable in this read-only phase",
        disabled: true,
    },
    DisabledHubAction {
        label: "Run recipe",
        reason: "Processes and automation are unavailable in this read-only phase",
        disabled: true,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectOpenSshPreparationOrigin {
    Inventory,
    Literal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubControllerEffect {
    None,
    Interaction(InteractionEffect),
    MetadataReviewReady,
    LiteralReviewReady,
    MetadataSubmitted { request: u64 },
    Closed { restore_focus_to: String },
    Error(HubRuntimeErrorCode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HubControllerPresentation {
    pub view: ConnectionHubView,
    pub query: String,
    pub catalog_query: ConnectionCatalogQuery,
    pub row_group_labels: Vec<Option<String>>,
    pub ime_preedit: Option<String>,
    pub literal_destination: Option<String>,
    pub literal_destination_diagnostic: Option<&'static str>,
    pub literal_destination_valid: bool,
    pub grant_review_offset: usize,
    pub metadata_review: Option<MetadataChangeReview>,
    pub tag_editor: Option<String>,
    pub selected_entry: Option<ConnectionCatalogEntry>,
    pub direct_openssh_review: Option<ConnectionReviewView>,
    pub direct_openssh_diagnostic: Option<&'static str>,
    pub grant_review: GrantReviewState,
    pub metadata_change: HubMetadataChangeState,
    pub library: HubLibrarySnapshot,
    pub store_state: HubStoreState,
    pub setup_guidance: SetupGuidance,
    pub disabled_actions: Vec<DisabledHubAction>,
}

pub struct ConnectionHubController {
    runtime: ConnectionHubRuntime,
    runtime_snapshot: HubRuntimeSnapshot,
    query: ConnectionCatalogQuery,
    ime_preedit: Option<String>,
    literal_destination: Option<String>,
    literal_destination_diagnostic: Option<&'static str>,
    grant_review_offset: usize,
    grant_review_request: Option<u64>,
    metadata_review: Option<MetadataChangeReview>,
    tag_editor: Option<String>,
    projection: ConnectionCatalogProjection,
    projected_summaries: Vec<ConnectionSummary>,
    interaction: InteractionState,
    selected_id: Option<String>,
    direct_openssh_preparation: Option<DirectOpenSshPreparation>,
    direct_openssh_preparation_origin: Option<DirectOpenSshPreparationOrigin>,
    direct_openssh_diagnostic: Option<&'static str>,
    #[cfg(test)]
    projection_refresh_count: u64,
    library_preferences_applied: bool,
    active: bool,
}

impl ConnectionHubController {
    pub fn new(runtime: ConnectionHubRuntime) -> Self {
        let runtime_snapshot = runtime.snapshot();
        let query = ConnectionCatalogQuery::default();
        let projection = project_connection_catalog(&runtime_snapshot.catalog, &query)
            .unwrap_or_else(|_| ConnectionCatalogProjection {
                indices: Vec::new(),
                groups: Vec::new(),
                source_revision: 0,
                content_state: HubContentState::Empty,
            });
        let projected_summaries =
            projected_summaries(&runtime_snapshot.catalog, &projection);
        let result_count = projection.indices.len();
        Self {
            runtime,
            runtime_snapshot,
            query,
            ime_preedit: None,
            literal_destination: None,
            literal_destination_diagnostic: None,
            grant_review_offset: 0,
            grant_review_request: None,
            metadata_review: None,
            tag_editor: None,
            projection,
            projected_summaries,
            interaction: InteractionState::new(
                result_count,
                MAX_VISIBLE_ROWS,
                "terminal-grid".into(),
            ),
            selected_id: None,
            direct_openssh_preparation: None,
            direct_openssh_preparation_origin: None,
            direct_openssh_diagnostic: None,
            #[cfg(test)]
            projection_refresh_count: 1,
            library_preferences_applied: false,
            active: false,
        }
    }

    pub fn open(&mut self, opener_id: impl Into<String>) {
        let opener_id = opener_id.into();
        self.cancel_literal_destination_entry();
        self.clear_direct_openssh_preparation();
        self.active = true;
        self.interaction = InteractionState::new(
            self.projection.indices.len(),
            MAX_VISIBLE_ROWS,
            opener_id,
        );
        self.sync();
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub fn catalog_controls_visible(&self) -> bool {
        self.interaction.route == HubRoute::Results
            && hub_catalog_controls_visible(self.content_state())
            && self.owned_grant_review().is_none()
            && self.metadata_review.is_none()
            && self.tag_editor.is_none()
            && self.literal_destination.is_none()
    }

    pub fn close(&mut self) -> String {
        self.clear_direct_openssh_preparation();
        self.cancel_literal_destination_entry();
        self.discard_owned_review();
        self.metadata_review = None;
        self.tag_editor = None;
        self.active = false;
        self.interaction.opener_id.clone()
    }

    pub fn query(&self) -> &str {
        &self.query.text
    }

    pub fn catalog_query(&self) -> &ConnectionCatalogQuery {
        &self.query
    }

    pub fn focus(&self) -> HubFocus {
        self.interaction.focus.clone()
    }

    pub const fn execution_requested(&self) -> bool {
        self.interaction.execution_requested
    }
    pub fn direct_openssh_preparation(&self) -> Option<&DirectOpenSshPreparation> {
        self.direct_openssh_preparation.as_ref()
    }

    pub fn report_direct_openssh_diagnostic(&mut self, diagnostic: &'static str) {
        self.direct_openssh_diagnostic = Some(diagnostic);
    }

    pub fn focus_search(&mut self) {
        self.interaction.focus = HubFocus::Search;
    }

    pub fn can_begin_literal_destination_entry(&self) -> bool {
        self.active
            && self.interaction.route == HubRoute::Results
            && self.literal_destination.is_none()
            && self.owned_grant_review().is_none()
            && self.metadata_review.is_none()
            && self.tag_editor.is_none()
    }

    pub fn begin_literal_destination_entry(&mut self) -> bool {
        if !self.can_begin_literal_destination_entry() {
            return false;
        }
        self.clear_direct_openssh_preparation();
        self.literal_destination = Some(String::new());
        self.literal_destination_diagnostic = None;
        self.ime_preedit = None;
        self.interaction.focus = HubFocus::LiteralDestination;
        true
    }

    pub fn literal_destination_entry_is_active(&self) -> bool {
        self.literal_destination.is_some()
    }

    pub fn literal_destination(&self) -> Option<&str> {
        self.literal_destination.as_deref()
    }

    pub const fn literal_destination_diagnostic(&self) -> Option<&'static str> {
        self.literal_destination_diagnostic
    }

    pub fn literal_destination_is_valid(&self) -> bool {
        self.literal_destination_diagnostic.is_none()
            && self
                .literal_destination
                .as_deref()
                .is_some_and(|destination| {
                    validate_literal_direct_openssh_destination(destination).is_ok()
                })
    }

    pub fn focus_literal_destination(&mut self) {
        if self.literal_destination.is_some() {
            self.interaction.focus = HubFocus::LiteralDestination;
        }
    }

    pub fn append_literal_destination(&mut self, value: &str) -> bool {
        let Some(current) = self.literal_destination.as_ref() else {
            return false;
        };
        if self.interaction.focus != HubFocus::LiteralDestination {
            return false;
        }
        let mut candidate = current.clone();
        candidate.push_str(value);
        match validate_literal_direct_openssh_destination(&candidate) {
            Ok(()) => {
                self.literal_destination = Some(candidate);
                self.literal_destination_diagnostic = None;
                true
            }
            Err(error) => {
                self.literal_destination_diagnostic = Some(error.diagnostic());
                false
            }
        }
    }

    pub fn backspace_literal_destination(&mut self) {
        if self.interaction.focus != HubFocus::LiteralDestination {
            return;
        }
        if let Some(destination) = self.literal_destination.as_mut() {
            destination.pop();
            self.literal_destination_diagnostic = None;
            self.ime_preedit = None;
        }
    }

    pub fn cycle_literal_destination_focus(&mut self, reverse: bool) {
        if self.literal_destination.is_none() {
            return;
        }
        self.interaction.focus = match (reverse, &self.interaction.focus) {
            (false, HubFocus::LiteralDestination) | (true, HubFocus::Back) => {
                HubFocus::PrimaryAction
            }
            (false, HubFocus::PrimaryAction) | (true, HubFocus::LiteralDestination) => {
                HubFocus::Back
            }
            _ => HubFocus::LiteralDestination,
        };
    }

    pub fn confirm_literal_destination(&mut self) -> HubControllerEffect {
        let Some(destination) = self.literal_destination.as_deref() else {
            return HubControllerEffect::None;
        };
        match prepare_literal_direct_openssh(destination) {
            Ok(prepared) => {
                self.literal_destination = None;
                self.literal_destination_diagnostic = None;
                self.ime_preedit = None;
                self.direct_openssh_preparation = Some(prepared);
                self.direct_openssh_preparation_origin =
                    Some(DirectOpenSshPreparationOrigin::Literal);
                self.direct_openssh_diagnostic = None;
                self.interaction.route = HubRoute::Review;
                self.interaction.focus = HubFocus::Review;
                HubControllerEffect::LiteralReviewReady
            }
            Err(error) => {
                self.literal_destination_diagnostic = Some(error.diagnostic());
                self.interaction.focus = HubFocus::LiteralDestination;
                HubControllerEffect::None
            }
        }
    }

    pub fn activate_literal_destination_focus(&mut self) -> HubControllerEffect {
        match self.interaction.focus {
            HubFocus::LiteralDestination | HubFocus::PrimaryAction => {
                self.confirm_literal_destination()
            }
            HubFocus::Back => {
                self.cancel_literal_destination_entry();
                HubControllerEffect::None
            }
            _ => HubControllerEffect::None,
        }
    }

    pub fn cancel_literal_destination_entry(&mut self) {
        self.literal_destination = None;
        self.literal_destination_diagnostic = None;
        self.ime_preedit = None;
        if self.interaction.route == HubRoute::Results {
            self.interaction.focus = HubFocus::Results;
        }
    }

    pub fn select_projected_index(&mut self, index: usize) {
        if index < self.projection.indices.len() {
            self.clear_direct_openssh_preparation();
            self.interaction.selected_index = index;
            self.interaction.focus = HubFocus::Results;
            self.selected_id =
                self.selected_entry().map(|entry| entry.summary.id.clone());
        }
    }

    pub fn set_search_text(&mut self, value: &str) -> bool {
        let mut query = self.query.clone();
        query.text = value.to_owned();
        let Ok(projection) =
            project_connection_catalog(&self.runtime_snapshot.catalog, &query)
        else {
            return false;
        };
        self.query = query;
        self.install_projection(projection);
        true
    }

    pub fn set_ime_preedit(&mut self, value: Option<&str>) -> bool {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            self.ime_preedit = None;
            return true;
        };
        if let Some(destination) = self.literal_destination.as_ref() {
            if self.interaction.focus != HubFocus::LiteralDestination {
                self.ime_preedit = None;
                return false;
            }
            let mut candidate = destination.clone();
            candidate.push_str(value);
            return match validate_literal_direct_openssh_destination(&candidate) {
                Ok(()) => {
                    self.literal_destination_diagnostic = None;
                    self.ime_preedit = Some(value.to_owned());
                    true
                }
                Err(error) => {
                    self.ime_preedit = None;
                    self.literal_destination_diagnostic = Some(error.diagnostic());
                    false
                }
            };
        }
        if let Some(editor) = self.tag_editor.as_ref() {
            if editor.len().saturating_add(value.len()) > MAX_CATALOG_QUERY_BYTES
                || value.chars().any(unsafe_metadata_character)
            {
                return false;
            }
            self.ime_preedit = Some(value.to_owned());
            return true;
        }
        if !self.catalog_controls_visible() || self.interaction.focus != HubFocus::Search
        {
            self.ime_preedit = None;
            return false;
        }
        let mut candidate = self.query.clone();
        candidate.text.push_str(value);
        if validate_connection_catalog_query(&candidate).is_err() {
            return false;
        }
        self.ime_preedit = Some(value.to_owned());
        true
    }

    pub fn commit_ime(&mut self, value: &str) -> bool {
        let accepted = if self.literal_destination.is_some() {
            self.append_literal_destination(value)
        } else if self.tag_editor.is_some() {
            self.append_tag_editor(value)
        } else if self.catalog_controls_visible()
            && self.interaction.focus == HubFocus::Search
        {
            self.append_search_text(value)
        } else {
            false
        };
        if accepted {
            self.ime_preedit = None;
        }
        accepted
    }

    pub fn append_search_text(&mut self, value: &str) -> bool {
        let mut next = self.query.text.clone();
        next.push_str(value);
        self.set_search_text(&next)
    }

    pub fn backspace_search(&mut self) {
        let mut next = self.query.text.clone();
        next.pop();
        let _ = self.set_search_text(&next);
    }

    pub fn toggle_favorites_filter(&mut self) {
        self.query.favorites_only = !self.query.favorites_only;
        self.refresh_projection();
    }

    pub fn toggle_recent_filter(&mut self) {
        self.query.recent_only = !self.query.recent_only;
        self.refresh_projection();
    }

    pub fn cycle_source_filter(&mut self) {
        self.query.source = match self.query.source {
            None => Some(HubCatalogSource::OpenSshUser),
            Some(HubCatalogSource::OpenSshUser) => Some(HubCatalogSource::OpenSshSystem),
            Some(HubCatalogSource::OpenSshSystem) => Some(HubCatalogSource::SavedProfile),
            Some(HubCatalogSource::SavedProfile) => {
                Some(HubCatalogSource::ImportedProfile)
            }
            Some(HubCatalogSource::ImportedProfile) => None,
        };
        self.refresh_projection();
    }

    pub fn cycle_grouping(&mut self) {
        self.query.grouping = match self.query.grouping {
            HubCatalogGrouping::None => HubCatalogGrouping::Source,
            HubCatalogGrouping::Source => HubCatalogGrouping::Environment,
            HubCatalogGrouping::Environment => HubCatalogGrouping::Favorite,
            HubCatalogGrouping::Favorite => HubCatalogGrouping::None,
        };
        self.refresh_projection();
    }

    pub fn clear_filters(&mut self) {
        self.query.text.clear();
        self.ime_preedit = None;
        self.query.favorites_only = false;
        self.query.recent_only = false;
        self.query.tag = None;
        self.query.source = None;
        self.refresh_projection();
    }

    pub fn grant_review_is_pending(&self) -> bool {
        matches!(
            self.owned_grant_review(),
            Some(GrantReviewState::Ready { .. })
        )
    }

    pub fn pending_grant_review_request(&self) -> Option<u64> {
        match self.owned_grant_review() {
            Some(GrantReviewState::Ready { request, .. }) => Some(*request),
            _ => None,
        }
    }

    pub fn jump_grant_review(&mut self, end: bool) {
        let len = match self.owned_grant_review() {
            Some(GrantReviewState::Ready { files, .. }) => files.len(),
            _ => 0,
        };
        self.grant_review_offset = if end { len.saturating_sub(1) } else { 0 };
    }

    pub fn scroll_grant_review(&mut self, delta: isize) {
        let Some(GrantReviewState::Ready { files, .. }) = self.owned_grant_review()
        else {
            self.grant_review_offset = 0;
            return;
        };
        let last = files.len().saturating_sub(1);
        self.grant_review_offset = self
            .grant_review_offset
            .saturating_add_signed(delta)
            .min(last);
    }

    pub fn cancel_grant_review(&mut self) {
        self.discard_owned_review();
    }

    pub fn sync(&mut self) {
        let next_snapshot = self.runtime.snapshot();
        let catalog_changed =
            !Arc::ptr_eq(&self.runtime_snapshot.catalog, &next_snapshot.catalog);
        let review_binding_changed = self.runtime_snapshot.state != next_snapshot.state
            || self.runtime_snapshot.metadata_revision != next_snapshot.metadata_revision
            || catalog_changed;
        self.runtime_snapshot = next_snapshot;
        let mut refresh_projection = catalog_changed;
        if !self.library_preferences_applied
            && !matches!(self.runtime_snapshot.state, HubRuntimeState::Initializing)
        {
            let preferences = &self.runtime_snapshot.library.preferences;
            self.query.favorites_only = preferences.favorites_only;
            self.query.recent_only = preferences.recent_only;
            self.query.tag.clone_from(&preferences.tag);
            self.query.source = preferences.source;
            self.query.grouping = preferences.grouping;
            self.library_preferences_applied = true;
            refresh_projection = true;
        }
        if self.metadata_review.as_ref().is_some_and(|review| {
            review.expected_revision != self.runtime_snapshot.metadata_revision
        }) {
            self.metadata_review = None;
        }
        if self.grant_review_request.is_some()
            && self.grant_review_request
                != grant_review_request(&self.runtime_snapshot.grant_review)
        {
            self.grant_review_request = None;
            self.grant_review_offset = 0;
        }
        if refresh_projection {
            self.refresh_projection();
        }
        if review_binding_changed
            && self.interaction.route == HubRoute::Review
            && self.direct_openssh_preparation_origin
                == Some(DirectOpenSshPreparationOrigin::Inventory)
        {
            self.refresh_direct_openssh_preparation();
        }
        let review_len = match self.owned_grant_review() {
            Some(GrantReviewState::Ready { files, .. }) => files.len(),
            _ => 0,
        };
        self.grant_review_offset =
            self.grant_review_offset.min(review_len.saturating_sub(1));
    }

    pub fn presentation(
        &self,
        viewport: Viewport,
        preferences: HubVisualPreferences,
    ) -> HubControllerPresentation {
        let content_state = self.content_state();
        let view = project_connection_hub(HubProjectionRequest {
            viewport,
            preferences,
            content_state,
            route: self.interaction.route,
            connections: &self.projected_summaries,
            selected_id: self.selected_id.as_deref(),
            focus: self.interaction.focus.clone(),
            opener_id: &self.interaction.opener_id,
            live_announcement: self
                .literal_destination_diagnostic
                .or_else(|| self.live_announcement()),
            literal_destination_entry: self.literal_destination.is_some(),
            literal_destination_valid: self.literal_destination_is_valid(),
        });
        let row_group_labels = view
            .visible_range
            .clone()
            .map(|position| {
                self.projection
                    .groups
                    .iter()
                    .find(|group| group.range.start == position)
                    .map(|group| group.label.clone())
            })
            .collect();
        HubControllerPresentation {
            view,
            query: self.query.text.clone(),
            catalog_query: self.query.clone(),
            row_group_labels,
            ime_preedit: self.ime_preedit.clone(),
            literal_destination: self.literal_destination.clone(),
            literal_destination_diagnostic: self.literal_destination_diagnostic,
            literal_destination_valid: self.literal_destination_is_valid(),
            grant_review_offset: self.grant_review_offset,
            metadata_review: self.metadata_review.clone(),
            tag_editor: self.tag_editor.clone(),
            selected_entry: self.selected_entry().cloned(),
            direct_openssh_review: self
                .direct_openssh_preparation
                .as_ref()
                .map(|prepared| project_direct_openssh_preparation(prepared, viewport)),
            direct_openssh_diagnostic: self.direct_openssh_diagnostic,
            grant_review: self
                .owned_grant_review()
                .cloned()
                .unwrap_or(GrantReviewState::None),
            metadata_change: self.runtime_snapshot.metadata_change.clone(),
            library: self.runtime_snapshot.library.clone(),
            store_state: self.runtime_snapshot.store_state,
            setup_guidance: platform_setup_guidance(current_platform()),
            disabled_actions: DISABLED_ACTIONS.to_vec(),
        }
    }

    pub fn handle_key(
        &mut self,
        key: HubKey,
        wake: CompletionWake,
    ) -> HubControllerEffect {
        if !self.active {
            return HubControllerEffect::None;
        }
        if self.literal_destination.is_some() {
            return match key {
                HubKey::Enter => self.activate_literal_destination_focus(),
                HubKey::Escape => {
                    self.cancel_literal_destination_entry();
                    HubControllerEffect::None
                }
                HubKey::Tab => {
                    self.cycle_literal_destination_focus(false);
                    HubControllerEffect::Interaction(InteractionEffect::FocusChanged(
                        self.interaction.focus.clone(),
                    ))
                }
                HubKey::ShiftTab => {
                    self.cycle_literal_destination_focus(true);
                    HubControllerEffect::Interaction(InteractionEffect::FocusChanged(
                        self.interaction.focus.clone(),
                    ))
                }
                _ => HubControllerEffect::None,
            };
        }
        if self.metadata_review.is_some() {
            return match key {
                HubKey::Enter => self.confirm_metadata_review(wake),
                HubKey::Escape => {
                    self.metadata_review = None;
                    HubControllerEffect::None
                }
                _ => HubControllerEffect::None,
            };
        }
        let effect = apply_hub_key(&mut self.interaction, key);
        match &effect {
            InteractionEffect::SelectionChanged(_) => {
                self.clear_direct_openssh_preparation();
                self.selected_id =
                    self.selected_entry().map(|entry| entry.summary.id.clone());
            }
            InteractionEffect::OpenReview { selected_index } => {
                self.selected_id = self
                    .entry_at(*selected_index)
                    .map(|entry| entry.summary.id.clone());
                self.refresh_direct_openssh_preparation();
            }
            InteractionEffect::BackToResults => {
                self.clear_direct_openssh_preparation();
            }
            InteractionEffect::ToggleFavorite { selected_index } => {
                return self.toggle_favorite_at(*selected_index);
            }
            InteractionEffect::CloseAndRestoreFocus(opener) => {
                self.clear_direct_openssh_preparation();
                self.discard_owned_review();
                self.active = false;
                return HubControllerEffect::Closed {
                    restore_focus_to: opener.clone(),
                };
            }
            _ => {}
        }
        HubControllerEffect::Interaction(effect)
    }

    pub fn toggle_favorite_at(&mut self, projected_index: usize) -> HubControllerEffect {
        let Some(entry) = self.entry_at(projected_index) else {
            return HubControllerEffect::None;
        };
        let review = match self.runtime.review_metadata_change(
            &entry.summary.id,
            Some(!entry.summary.favorite),
            None,
        ) {
            Ok(review) => review,
            Err(error) => return HubControllerEffect::Error(error),
        };
        self.metadata_review = Some(review);
        HubControllerEffect::MetadataReviewReady
    }

    pub fn metadata_review_is_pending(&self) -> bool {
        self.metadata_review.is_some()
    }

    pub fn confirm_metadata_review(
        &mut self,
        wake: CompletionWake,
    ) -> HubControllerEffect {
        let Some(review) = self.metadata_review.take() else {
            return HubControllerEffect::None;
        };
        match self.runtime.apply_metadata_change(review.clone(), wake) {
            Ok(request) => HubControllerEffect::MetadataSubmitted { request },
            Err(error) => {
                self.metadata_review = Some(review);
                HubControllerEffect::Error(error)
            }
        }
    }

    pub fn begin_tag_editor_at(&mut self, projected_index: usize) -> HubControllerEffect {
        let Some(entry) = self.entry_at(projected_index) else {
            return HubControllerEffect::None;
        };
        self.tag_editor = Some(entry.tags.join(", "));
        self.ime_preedit = None;
        HubControllerEffect::None
    }

    pub fn begin_selected_tag_editor(&mut self) -> HubControllerEffect {
        self.begin_tag_editor_at(self.interaction.selected_index)
    }

    pub fn tag_editor_is_active(&self) -> bool {
        self.tag_editor.is_some()
    }

    pub fn append_tag_editor(&mut self, value: &str) -> bool {
        let Some(current) = self.tag_editor.as_mut() else {
            return false;
        };
        if current.len().saturating_add(value.len()) > MAX_CATALOG_QUERY_BYTES
            || value.chars().any(unsafe_metadata_character)
        {
            return false;
        }
        current.push_str(value);
        true
    }

    pub fn backspace_tag_editor(&mut self) {
        if let Some(editor) = self.tag_editor.as_mut() {
            editor.pop();
        }
    }

    pub fn cancel_tag_editor(&mut self) {
        self.tag_editor = None;
        self.ime_preedit = None;
    }

    pub fn finish_tag_editor(&mut self) -> HubControllerEffect {
        let Some(value) = self.tag_editor.take() else {
            return HubControllerEffect::None;
        };
        self.ime_preedit = None;
        let Some(entry) = self.selected_entry() else {
            return HubControllerEffect::None;
        };
        let mut folded = std::collections::BTreeSet::new();
        let tags = value
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .filter(|tag| folded.insert(tag.to_lowercase()))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let review =
            match self
                .runtime
                .review_metadata_change(&entry.summary.id, None, Some(tags))
            {
                Ok(review) => review,
                Err(error) => {
                    self.tag_editor = Some(value);
                    return HubControllerEffect::Error(error);
                }
            };
        self.metadata_review = Some(review);
        HubControllerEffect::MetadataReviewReady
    }

    pub fn review_exact_files(
        &mut self,
        paths: Vec<PathBuf>,
        kind: GrantKind,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        let request = self.runtime.review_exact_files(paths, kind, wake)?;
        self.grant_review_offset = 0;
        self.grant_review_request = Some(request);
        Ok(request)
    }

    pub fn confirm_reviewed_scan(
        &mut self,
        review: u64,
        wake: CompletionWake,
    ) -> Result<u64, HubRuntimeErrorCode> {
        if self.grant_review_request != Some(review) {
            return Err(HubRuntimeErrorCode::StaleReview);
        }
        let result = self.runtime.confirm_reviewed_scan(review, wake);
        if result.is_ok() {
            self.grant_review_offset = 0;
            self.grant_review_request = None;
        }
        result
    }

    pub fn shutdown(&self) {
        self.runtime.shutdown();
    }

    fn refresh_direct_openssh_preparation(&mut self) {
        self.direct_openssh_preparation = None;
        self.direct_openssh_preparation_origin = None;
        self.direct_openssh_diagnostic = None;
        let Some(connection_id) = self.selected_id.as_deref() else {
            self.direct_openssh_diagnostic =
                Some(HubRuntimeErrorCode::UnknownConnection.diagnostic_code());
            return;
        };
        match self.runtime.prepare_direct_openssh(connection_id) {
            Ok(prepared) => {
                self.direct_openssh_preparation = Some(prepared);
                self.direct_openssh_preparation_origin =
                    Some(DirectOpenSshPreparationOrigin::Inventory);
            }
            Err(error) => self.direct_openssh_diagnostic = Some(error.diagnostic_code()),
        }
    }

    fn clear_direct_openssh_preparation(&mut self) {
        self.direct_openssh_preparation = None;
        self.direct_openssh_preparation_origin = None;
        self.direct_openssh_diagnostic = None;
    }

    fn owned_grant_review(&self) -> Option<&GrantReviewState> {
        let request = self.grant_review_request?;
        (grant_review_request(&self.runtime_snapshot.grant_review) == Some(request))
            .then_some(&self.runtime_snapshot.grant_review)
    }

    fn discard_owned_review(&mut self) {
        if let Some(request) = self.grant_review_request.take() {
            let _ = self.runtime.discard_review(request);
        }
        self.grant_review_offset = 0;
    }

    fn refresh_projection(&mut self) {
        let projection =
            project_connection_catalog(&self.runtime_snapshot.catalog, &self.query)
                .unwrap_or_else(|_| ConnectionCatalogProjection {
                    indices: Vec::new(),
                    groups: Vec::new(),
                    source_revision: self.runtime_snapshot.metadata_revision,
                    content_state: HubContentState::Error,
                });
        self.install_projection(projection);
    }

    fn install_projection(&mut self, projection: ConnectionCatalogProjection) {
        self.projected_summaries =
            projected_summaries(&self.runtime_snapshot.catalog, &projection);
        self.projection = projection;
        #[cfg(test)]
        {
            self.projection_refresh_count =
                self.projection_refresh_count.saturating_add(1);
        }
        self.interaction.result_count = self.projection.indices.len();
        self.interaction.selected_index = self
            .selected_id
            .as_deref()
            .and_then(|selected| {
                self.projection.indices.iter().position(|index| {
                    self.runtime_snapshot.catalog[*index].summary.id == selected
                })
            })
            .unwrap_or(self.interaction.selected_index)
            .min(self.projection.indices.len().saturating_sub(1));
        self.selected_id = self.selected_entry().map(|entry| entry.summary.id.clone());
    }

    #[cfg(test)]
    const fn projection_refresh_count(&self) -> u64 {
        self.projection_refresh_count
    }

    fn entry_at(&self, projected_index: usize) -> Option<&ConnectionCatalogEntry> {
        let catalog_index = *self.projection.indices.get(projected_index)?;
        self.runtime_snapshot.catalog.get(catalog_index)
    }

    fn selected_entry(&self) -> Option<&ConnectionCatalogEntry> {
        self.entry_at(self.interaction.selected_index)
    }

    fn content_state(&self) -> HubContentState {
        match self.runtime_snapshot.state {
            HubRuntimeState::Initializing | HubRuntimeState::Loading { .. } => {
                HubContentState::Loading
            }
            HubRuntimeState::InitialSetup => HubContentState::InitialSetup,
            HubRuntimeState::Ready { .. } => {
                if self.runtime_snapshot.store_state == HubStoreState::Recovered {
                    HubContentState::PartialFailure
                } else {
                    self.projection.content_state
                }
            }
            HubRuntimeState::Stale { .. } => HubContentState::Stale,
            HubRuntimeState::Error { .. } | HubRuntimeState::Shutdown => {
                HubContentState::Error
            }
        }
    }

    fn live_announcement(&self) -> Option<&'static str> {
        match self.runtime_snapshot.metadata_change {
            HubMetadataChangeState::Applied { .. } => Some("Connection metadata saved"),
            HubMetadataChangeState::Conflict { .. } => {
                Some("Connection metadata changed elsewhere; current values reloaded")
            }
            HubMetadataChangeState::Error { .. } => {
                Some("Connection metadata was not saved")
            }
            HubMetadataChangeState::Idle | HubMetadataChangeState::Applying { .. } => {
                None
            }
        }
    }
}

fn grant_review_request(state: &GrantReviewState) -> Option<u64> {
    match state {
        GrantReviewState::Reviewing { request }
        | GrantReviewState::Ready { request, .. }
        | GrantReviewState::Error { request, .. } => Some(*request),
        GrantReviewState::None => None,
    }
}

fn projected_summaries(
    catalog: &[ConnectionCatalogEntry],
    projection: &ConnectionCatalogProjection,
) -> Vec<ConnectionSummary> {
    projection
        .indices
        .iter()
        .filter_map(|index| catalog.get(*index))
        .map(|entry| entry.summary.clone())
        .collect()
}

fn unsafe_metadata_character(character: char) -> bool {
    let codepoint = character as u32;
    character.is_control()
        || codepoint == 0x061c
        || (0x200b..=0x200f).contains(&codepoint)
        || (0x202a..=0x202e).contains(&codepoint)
        || (0x2060..=0x206f).contains(&codepoint)
        || codepoint == 0xfeff
}

fn current_platform() -> PlatformFamily {
    #[cfg(target_os = "windows")]
    {
        PlatformFamily::Windows
    }
    #[cfg(target_os = "macos")]
    {
        PlatformFamily::MacOs
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PlatformFamily::Linux
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn initial_setup_does_not_expose_catalog_controls() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");

        assert_eq!(controller.content_state(), HubContentState::InitialSetup);
        assert!(!controller.catalog_controls_visible());
        assert!(!controller.set_ime_preedit(Some("hidden")));
        assert!(!controller.commit_ime("hidden"));
        assert!(controller.query().is_empty());
    }

    #[test]
    fn unchanged_sync_and_presentation_reuse_the_cached_projection() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");
        let refreshes = controller.projection_refresh_count();

        for _ in 0..32 {
            controller.sync();
            let _ = controller.presentation(
                Viewport::new(1_280.0, 800.0, 1.0),
                HubVisualPreferences::default(),
            );
        }
        assert_eq!(controller.projection_refresh_count(), refreshes);

        controller.toggle_favorites_filter();
        assert_eq!(
            controller.projection_refresh_count(),
            refreshes.saturating_add(1)
        );
    }

    #[test]
    fn literal_destination_entry_is_bounded_focus_trapped_transient_and_review_only() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");

        assert!(controller.begin_literal_destination_entry());
        assert!(controller.literal_destination_entry_is_active());
        assert_eq!(controller.focus(), HubFocus::LiteralDestination);
        assert!(controller.append_literal_destination("host"));
        assert!(!controller.append_literal_destination("@operator"));
        assert_eq!(controller.literal_destination(), Some("host"));
        assert_eq!(
            controller.literal_destination_diagnostic(),
            Some("Use one host or alias with letters, numbers, dots, underscores, or hyphens.")
        );
        assert!(controller.append_literal_destination(".example.invalid"));
        assert!(controller.set_ime_preedit(Some("-canary")));
        assert!(controller.commit_ime("-canary"));
        assert_eq!(
            controller.literal_destination(),
            Some("host.example.invalid-canary")
        );
        assert!(controller.literal_destination_is_valid());

        controller.cycle_literal_destination_focus(false);
        assert_eq!(controller.focus(), HubFocus::PrimaryAction);
        controller.cycle_literal_destination_focus(false);
        assert_eq!(controller.focus(), HubFocus::Back);
        controller.cycle_literal_destination_focus(false);
        assert_eq!(controller.focus(), HubFocus::LiteralDestination);

        let before = controller.presentation(
            Viewport::new(1_280.0, 800.0, 1.0),
            HubVisualPreferences::default(),
        );
        assert_eq!(
            before.literal_destination.as_deref(),
            Some("host.example.invalid-canary")
        );
        assert_eq!(before.library.profile_count, 0);
        assert!(before.direct_openssh_review.is_none());

        assert_eq!(
            controller.confirm_literal_destination(),
            HubControllerEffect::LiteralReviewReady
        );
        let prepared = controller.presentation(
            Viewport::new(1_280.0, 800.0, 1.0),
            HubVisualPreferences::default(),
        );
        assert_eq!(prepared.view.route, HubRoute::Review);
        assert!(prepared.literal_destination.is_none());
        assert!(prepared.literal_destination_diagnostic.is_none());
        assert_eq!(prepared.library.profile_count, 0);
        let review = prepared.direct_openssh_review.unwrap();
        assert!(!review.execution_enabled);
        assert!(review.sections.iter().any(|section| {
            section.heading == "Environment risk"
                && section.summary == "Production"
                && section.blocking
        }));
        assert!(review.sections.iter().any(|section| {
            section.heading == "Public target"
                && section.summary == "host.example.invalid-canary"
        }));

        controller.sync();
        let after_sync = controller.presentation(
            Viewport::new(1_280.0, 800.0, 1.0),
            HubVisualPreferences::default(),
        );
        assert!(after_sync.direct_openssh_review.is_some());

        let effect = controller.handle_key(HubKey::Escape, Box::new(|| {}));
        assert_eq!(
            effect,
            HubControllerEffect::Interaction(InteractionEffect::BackToResults)
        );
        assert!(controller
            .presentation(
                Viewport::new(1_280.0, 800.0, 1.0),
                HubVisualPreferences::default(),
            )
            .direct_openssh_review
            .is_none());
    }

    #[test]
    fn literal_destination_cancel_and_close_clear_transient_editor_state() {
        let temporary = tempfile::tempdir().unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");

        assert!(controller.begin_literal_destination_entry());
        assert!(controller.append_literal_destination("cancel-canary"));
        assert_eq!(
            controller.handle_key(HubKey::Tab, Box::new(|| {})),
            HubControllerEffect::Interaction(InteractionEffect::FocusChanged(
                HubFocus::PrimaryAction
            ))
        );
        assert_eq!(
            controller.handle_key(HubKey::ShiftTab, Box::new(|| {})),
            HubControllerEffect::Interaction(InteractionEffect::FocusChanged(
                HubFocus::LiteralDestination
            ))
        );
        assert_eq!(
            controller.handle_key(HubKey::Escape, Box::new(|| {})),
            HubControllerEffect::None
        );
        assert!(!controller.literal_destination_entry_is_active());
        assert!(controller.literal_destination_diagnostic().is_none());
        assert_eq!(controller.focus(), HubFocus::Results);

        assert!(controller.begin_literal_destination_entry());
        assert!(controller.append_literal_destination("close-canary"));
        assert_eq!(controller.close(), "terminal-grid");
        assert!(!controller.is_active());
        assert!(!controller.literal_destination_entry_is_active());
        assert!(controller.literal_destination_diagnostic().is_none());
    }

    #[test]
    fn enter_prepares_a_redacted_disabled_review_and_escape_discards_it() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        std::fs::write(
            &config,
            b"Host private-alias-canary\n  HostName public.example.invalid\n",
        )
        .unwrap();
        let runtime = ConnectionHubRuntime::open_at_root(temporary.path());
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));
        let grant = automexia_devops_ssh::InventoryGrant::new(
            "test-user-config",
            temporary.path(),
            [&config],
            GrantKind::User,
        )
        .unwrap();
        assert!(runtime.request_explicit_scan(vec![grant]) > 0);
        assert!(runtime.wait_for_settled(Duration::from_secs(5)));

        let mut controller = ConnectionHubController::new(runtime);
        controller.open("terminal-grid");
        let effect = controller.handle_key(HubKey::Enter, Box::new(|| {}));
        assert!(matches!(
            effect,
            HubControllerEffect::Interaction(InteractionEffect::OpenReview {
                selected_index: 0
            })
        ));
        assert!(!controller.catalog_controls_visible());
        let presentation = controller.presentation(
            Viewport::new(1_280.0, 800.0, 1.0),
            HubVisualPreferences::default(),
        );
        assert_eq!(presentation.view.route, HubRoute::Review);
        let review = presentation.direct_openssh_review.unwrap();
        assert_eq!(review.sections.len(), 9);
        assert!(!review.execution_enabled);
        assert!(presentation.direct_openssh_diagnostic.is_none());
        assert!(!format!("{review:?}").contains("private-alias-canary"));

        let effect = controller.handle_key(HubKey::Escape, Box::new(|| {}));
        assert_eq!(
            effect,
            HubControllerEffect::Interaction(InteractionEffect::BackToResults)
        );
        let presentation = controller.presentation(
            Viewport::new(1_280.0, 800.0, 1.0),
            HubVisualPreferences::default(),
        );
        assert_eq!(presentation.view.route, HubRoute::Results);
        assert!(presentation.direct_openssh_review.is_none());
    }
}
