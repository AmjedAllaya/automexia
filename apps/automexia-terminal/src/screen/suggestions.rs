//! Thin screen adapter for the application-owned CP5 suggestion controller.
//!
//! The adapter handles only host-owned overlay navigation and invalidation.
//! Candidate insertion stays with the authenticated native-editor bridge and
//! never travels through the PTY input path.

use automexia_ui_model::suggestions::Navigation;
use rio_window::event::{ElementState, KeyEvent};
use rio_window::keyboard::{Key, NamedKey};

use super::Screen;
use crate::automexia::suggestions::SuggestionInvalidation;

impl Screen<'_> {
    pub(crate) fn sync_suggestions(&mut self) {
        let next = self.suggestions.surface().cloned();
        if self.renderer.suggestions.surface() == next.as_ref() {
            return;
        }
        match next {
            Some(surface) => self.renderer.suggestions.set_surface(surface),
            None => self.renderer.suggestions.dismiss(),
        }
        self.mark_dirty();
    }

    pub(crate) fn process_suggestion_host_key(&mut self, key: &KeyEvent) -> bool {
        if key.state != ElementState::Pressed || self.suggestions.surface().is_none() {
            return false;
        }
        let navigation = match key.logical_key {
            Key::Named(NamedKey::ArrowUp) => Some(Navigation::Previous),
            Key::Named(NamedKey::ArrowDown) => Some(Navigation::Next),
            Key::Named(NamedKey::PageUp) => Some(Navigation::PageUp),
            Key::Named(NamedKey::PageDown) => Some(Navigation::PageDown),
            Key::Named(NamedKey::Home) => Some(Navigation::Home),
            Key::Named(NamedKey::End) => Some(Navigation::End),
            _ => None,
        };
        if let Some(navigation) = navigation {
            let _ = self.suggestions.navigate(navigation);
            self.sync_suggestions();
            return true;
        }

        match key.logical_key {
            Key::Named(NamedKey::Escape) => {
                self.dismiss_suggestions(SuggestionInvalidation::Typing);
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.dismiss_suggestions(SuggestionInvalidation::PromptCompleted);
                false
            }
            Key::Named(NamedKey::Tab) | Key::Named(NamedKey::ArrowRight) => {
                // The native adapter owns these keys. With no advertised
                // acceptance binding, close the stale surface and forward the
                // key unchanged to the editor.
                self.dismiss_suggestions(SuggestionInvalidation::Typing);
                false
            }
            _ => {
                self.dismiss_suggestions(SuggestionInvalidation::Typing);
                false
            }
        }
    }

    pub(crate) fn dismiss_suggestions(&mut self, reason: SuggestionInvalidation) {
        self.suggestions.dismiss(reason);
        self.renderer.suggestions.dismiss();
        self.mark_dirty();
    }
}
