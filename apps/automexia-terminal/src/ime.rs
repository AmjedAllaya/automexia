use unicode_segmentation::UnicodeSegmentation;
use unicode_width_ui::UnicodeWidthStr;

/// A composition belongs to the pane where it started, even if focus changes
/// before the operating system delivers its final commit.
#[derive(Debug, Default)]
pub(crate) struct CompositionOwner {
    route: Option<usize>,
    cancelled: bool,
}

impl CompositionOwner {
    pub(crate) fn begin(&mut self, route: usize) {
        self.route = Some(route);
        self.cancelled = false;
    }

    pub(crate) fn route(&self) -> Option<usize> {
        self.route
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub(crate) fn reset(&mut self) {
        self.route = None;
        self.cancelled = false;
    }

    pub(crate) fn cancel(&mut self) -> Option<usize> {
        let route = self.route.take();
        // Focus loss with no pending composition cannot invalidate a later
        // direct commit. Repeated cancellation must still reject the old one.
        self.cancelled |= route.is_some();
        route
    }

    pub(crate) fn commit(&mut self, route: usize) -> bool {
        let accepted = !self.cancelled && self.route.is_none_or(|owner| owner == route);
        self.route = None;
        // A rejected old commit remains cancelled until a new composition starts.
        self.cancelled = !accepted;
        accepted
    }
}

/// Composition is transient UI state, never unbounded terminal history.
const MAX_PREEDIT_BYTES: usize = 4096;
#[derive(Debug, Default)]
pub struct Ime {
    /// Whether the IME is enabled.
    enabled: bool,

    /// Current IME preedit.
    preedit: Option<Preedit>,
}

impl Ime {
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn set_enabled(&mut self, is_enabled: bool) {
        if is_enabled {
            self.enabled = is_enabled
        } else {
            // Clear state when disabling IME.
            *self = Default::default();
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn set_preedit(&mut self, preedit: Option<Preedit>) {
        self.preedit = preedit;
    }

    #[inline]
    pub fn preedit(&self) -> Option<&Preedit> {
        self.preedit.as_ref()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Preedit {
    /// The preedit text.
    pub text: String,

    /// Byte offset for cursor start into the preedit text.
    ///
    /// `None` means that the cursor is invisible.
    pub cursor_byte_offset: Option<usize>,

    /// The cursor offset from the end of the preedit in char width.
    pub cursor_end_offset: Option<usize>,
}

impl Preedit {
    pub fn new(mut text: String, cursor_byte_offset: Option<usize>) -> Self {
        if text.len() > MAX_PREEDIT_BYTES {
            let end = text
                .grapheme_indices(true)
                .map(|(start, cluster)| start + cluster.len())
                .take_while(|end| *end <= MAX_PREEDIT_BYTES)
                .last()
                .unwrap_or(0);
            text.truncate(end);
        }
        // Native IME offsets are byte indices. Hide malformed cursors, and
        // snap valid interior offsets to the beginning of their grapheme so
        // combining marks and joined emoji cannot acquire a split caret.
        let cursor_byte_offset = cursor_byte_offset
            .filter(|offset| *offset <= text.len() && text.is_char_boundary(*offset))
            .map(|offset| {
                if offset == text.len() {
                    offset
                } else {
                    text.grapheme_indices(true)
                        .map(|(start, _)| start)
                        .take_while(|start| *start <= offset)
                        .last()
                        .unwrap_or(0)
                }
            });
        let cursor_end_offset = cursor_byte_offset
            .and_then(|offset| text.get(offset..))
            .map(UnicodeWidthStr::width);

        Self {
            text,
            cursor_byte_offset,
            cursor_end_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_uses_grapheme_width_for_cjk_combining_and_zwj() {
        for (text, width) in [("中文", 4), ("e\u{301}", 1), ("👩\u{200d}💻", 2)] {
            let preedit = Preedit::new(text.into(), Some(0));
            assert_eq!(preedit.cursor_end_offset, Some(width), "{text:?}");
        }
    }

    #[test]
    fn malformed_composition_cursor_is_hidden_without_panicking() {
        for offset in [1, 2, 7, usize::MAX] {
            let preedit = Preedit::new("中文".into(), Some(offset));
            assert_eq!(preedit.cursor_byte_offset, None);
            assert_eq!(preedit.cursor_end_offset, None);
        }
    }

    #[test]
    fn composition_cursor_cannot_split_a_grapheme() {
        let preedit = Preedit::new("e\u{301}x".into(), Some(1));
        assert_eq!(preedit.cursor_byte_offset, Some(0));
        assert_eq!(preedit.cursor_end_offset, Some(2));
    }

    #[test]
    fn disabling_ime_clears_composition() {
        let mut ime = Ime::new();
        ime.set_enabled(true);
        ime.set_preedit(Some(Preedit::new("日本語".into(), Some(3))));
        ime.set_enabled(false);
        assert!(!ime.is_enabled());
        assert!(ime.preedit().is_none());
    }

    #[test]
    fn oversized_composition_is_bounded_at_a_grapheme_boundary() {
        let preedit = Preedit::new("👩\u{200d}💻".repeat(1000), Some(0));
        assert!(preedit.text.len() <= 4096);
        assert!(preedit.text.ends_with("👩\u{200d}💻"));
    }

    #[test]
    fn late_commit_cannot_move_to_another_pane_or_survive_cancellation() {
        let mut owner = CompositionOwner::default();
        owner.begin(1);
        assert!(!owner.commit(2));
        assert!(!owner.commit(2));
        owner.begin(2);
        assert!(owner.commit(2));
        owner.begin(2);
        owner.cancel();
        assert!(!owner.commit(2));
        owner.begin(2);
        assert!(owner.commit(2));
    }

    #[test]
    fn idle_focus_loss_does_not_cancel_the_next_direct_ime_commit() {
        let mut owner = CompositionOwner::default();
        assert_eq!(owner.cancel(), None);
        assert!(!owner.is_cancelled());
        assert!(owner.commit(1));
        owner.begin(1);
        assert!(owner.commit(1));
        assert_eq!(owner.cancel(), None);
        assert!(owner.commit(2));
    }

    #[test]
    fn repeated_focus_loss_keeps_an_old_composition_cancelled() {
        let mut owner = CompositionOwner::default();
        owner.begin(1);
        assert_eq!(owner.cancel(), Some(1));
        assert_eq!(owner.cancel(), None);
        assert!(!owner.commit(1));
        owner.begin(2);
        assert!(owner.commit(2));
    }
}
