//! Pure application interaction state for the CP5 suggestion surface.
//!
//! The screen adapter consumes this model. Acceptance returns only a
//! revalidated native-editor replacement and has no PTY-write or Enter path.

use automexia_devops::suggestions::{
    AcceptanceContext, EditorRequest, NativeEditorReplacement,
};
use automexia_ui_model::suggestions::{
    project_surface, AnnouncementGate, Navigation, Point, SuggestionInteraction,
    SuggestionSurface, SurfaceError, SurfaceRequest,
};

use super::{SuggestionService, SuggestionSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionInvalidation {
    Typing,
    FocusLost,
    PaneChanged,
    PromptCompleted,
    ModalOpened,
    ImeStarted,
    RouteClosed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuggestionAcceptanceKeys {
    pub tab: bool,
    pub right_arrow: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionInteractionKey {
    Escape,
    Enter,
    Tab,
    RightArrow,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuggestionInteractionOutcome {
    Consumed,
    Dismissed,
    ForwardToEditor,
    Replace(NativeEditorReplacement),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionControllerError {
    WrongOwner,
    InvalidCandidate,
    Layout,
}

impl From<SurfaceError> for SuggestionControllerError {
    fn from(_: SurfaceError) -> Self {
        Self::Layout
    }
}

struct ActiveSuggestion {
    request: EditorRequest,
    snapshot: SuggestionSnapshot,
    interaction: SuggestionInteraction,
    layout: SurfaceRequest,
    surface: SuggestionSurface,
}

pub struct SuggestionUiController {
    service: SuggestionService,
    active: Option<ActiveSuggestion>,
    announcements: AnnouncementGate,
    announcement: Option<String>,
}

impl SuggestionUiController {
    pub fn new(service: SuggestionService) -> Self {
        Self {
            service,
            active: None,
            announcements: AnnouncementGate::new(250),
            announcement: None,
        }
    }

    pub fn service(&self) -> &SuggestionService {
        &self.service
    }

    pub fn surface(&self) -> Option<&SuggestionSurface> {
        self.active.as_ref().map(|active| &active.surface)
    }

    pub fn announcement(&self) -> Option<&str> {
        self.announcement.as_deref()
    }

    pub fn publish(
        &mut self,
        request: EditorRequest,
        snapshot: SuggestionSnapshot,
        mut layout: SurfaceRequest,
        now_ms: u64,
    ) -> Result<&SuggestionSurface, SuggestionControllerError> {
        if request.validate().is_err()
            || snapshot.route != request.route()
            || snapshot.request_id != request.request_id
            || snapshot.buffer_generation != request.buffer_generation
            || snapshot.cancellation_id != request.cancellation_id
            || snapshot.source_revision > request.source_revision
            || snapshot
                .candidates
                .iter()
                .any(|ranked| ranked.candidate.validate(&request).is_err())
        {
            return Err(SuggestionControllerError::WrongOwner);
        }
        let Some(interaction) =
            SuggestionInteraction::new(snapshot.candidates.len(), layout.selected)
        else {
            self.dismiss(SuggestionInvalidation::Typing);
            return Err(SuggestionControllerError::InvalidCandidate);
        };
        layout.selected = interaction.selected();
        layout.pointer_highlight = interaction.pointer_highlight();
        let surface = project_surface(&layout, &snapshot.candidates)?;
        let label = surface.accessible_name.clone();
        if let Some(announcement) = self.announcements.publish(now_ms, &label) {
            self.announcement = Some(announcement.to_string());
        }
        let active = self.active.insert(ActiveSuggestion {
            request,
            snapshot,
            interaction,
            layout,
            surface,
        });
        Ok(&active.surface)
    }

    pub fn navigate(
        &mut self,
        navigation: Navigation,
    ) -> Result<Option<&SuggestionSurface>, SuggestionControllerError> {
        let Some(active) = self.active.as_mut() else {
            return Ok(None);
        };
        active.interaction.navigate(navigation);
        reproject(active)?;
        Ok(Some(&active.surface))
    }

    pub fn pointer_moved(
        &mut self,
        point: Point,
    ) -> Result<Option<&SuggestionSurface>, SuggestionControllerError> {
        let Some(active) = self.active.as_mut() else {
            return Ok(None);
        };
        let hit = active.surface.hit_test(point);
        if hit != active.interaction.pointer_highlight() {
            active.interaction.pointer_moved(hit);
            reproject(active)?;
        }
        Ok(Some(&active.surface))
    }

    pub fn pointer_accept(
        &mut self,
        current: &AcceptanceContext,
    ) -> Result<SuggestionInteractionOutcome, SuggestionControllerError> {
        let Some(active) = self.active.as_ref() else {
            return Ok(SuggestionInteractionOutcome::ForwardToEditor);
        };
        let Some(index) = active.interaction.pointer_accept() else {
            return Ok(SuggestionInteractionOutcome::Consumed);
        };
        accept(active, index, current)
    }

    pub fn handle_key(
        &mut self,
        key: SuggestionInteractionKey,
        keys: SuggestionAcceptanceKeys,
        current: &AcceptanceContext,
    ) -> Result<SuggestionInteractionOutcome, SuggestionControllerError> {
        if self.active.is_none() {
            return Ok(SuggestionInteractionOutcome::ForwardToEditor);
        }
        let navigation = match key {
            SuggestionInteractionKey::Up => Some(Navigation::Previous),
            SuggestionInteractionKey::Down => Some(Navigation::Next),
            SuggestionInteractionKey::PageUp => Some(Navigation::PageUp),
            SuggestionInteractionKey::PageDown => Some(Navigation::PageDown),
            SuggestionInteractionKey::Home => Some(Navigation::Home),
            SuggestionInteractionKey::End => Some(Navigation::End),
            _ => None,
        };
        if let Some(navigation) = navigation {
            self.navigate(navigation)?;
            return Ok(SuggestionInteractionOutcome::Consumed);
        }

        match key {
            SuggestionInteractionKey::Escape => {
                self.dismiss(SuggestionInvalidation::Typing);
                Ok(SuggestionInteractionOutcome::Dismissed)
            }
            SuggestionInteractionKey::Enter => {
                self.dismiss(SuggestionInvalidation::PromptCompleted);
                Ok(SuggestionInteractionOutcome::ForwardToEditor)
            }
            SuggestionInteractionKey::Tab if keys.tab => self.accept_selected(current),
            SuggestionInteractionKey::RightArrow if keys.right_arrow => {
                self.accept_selected(current)
            }
            _ => Ok(SuggestionInteractionOutcome::ForwardToEditor),
        }
    }

    pub fn dismiss(&mut self, _reason: SuggestionInvalidation) {
        self.active = None;
        self.announcement = None;
    }

    fn accept_selected(
        &self,
        current: &AcceptanceContext,
    ) -> Result<SuggestionInteractionOutcome, SuggestionControllerError> {
        let active = self
            .active
            .as_ref()
            .ok_or(SuggestionControllerError::WrongOwner)?;
        accept(active, active.interaction.selected(), current)
    }
}

fn reproject(active: &mut ActiveSuggestion) -> Result<(), SuggestionControllerError> {
    active.layout.selected = active.interaction.selected();
    active.layout.pointer_highlight = active.interaction.pointer_highlight();
    active.surface = project_surface(&active.layout, &active.snapshot.candidates)?;
    Ok(())
}

fn accept(
    active: &ActiveSuggestion,
    index: usize,
    current: &AcceptanceContext,
) -> Result<SuggestionInteractionOutcome, SuggestionControllerError> {
    let ranked = active
        .snapshot
        .candidates
        .get(index)
        .ok_or(SuggestionControllerError::InvalidCandidate)?;
    let replacement = NativeEditorReplacement::from_candidate(
        &active.request,
        &ranked.candidate,
        current,
    )
    .map_err(|_| SuggestionControllerError::WrongOwner)?;
    Ok(SuggestionInteractionOutcome::Replace(replacement))
}
