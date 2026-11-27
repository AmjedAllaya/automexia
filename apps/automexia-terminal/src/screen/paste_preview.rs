use super::Screen;
use crate::context::paste::PasteTarget;
use crate::event::PasteRequest;
use crate::renderer::paste_preview::{BatchReview, Choice, Layout, LineIndex};
use crate::renderer::ui_theme::UiTheme;
use rio_backend::crosswords::{grid::row::PromptInputShell, Mode, PasteDestination};
use rio_window::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent,
};
use rio_window::keyboard::{Key, NamedKey, PhysicalKey};

pub(super) struct PastePreview {
    target: PasteTarget,
    destination: PasteDestination,
    paste: PasteRequest,
    lines: LineIndex,
    batch_review: BatchReview,
    selected: Choice,
    hovered: Option<Choice>,
    pressed_button: Option<Choice>,
    held_enter: Option<(PhysicalKey, Choice)>,
    wheel_remainder: (f64, f64),
}

impl PastePreview {
    fn layout(&self, dimensions: (f32, f32, f32)) -> Layout {
        Layout::new(dimensions, self.lines.len(), self.batch_review)
    }

    fn prepare_bash_batch(&mut self) -> Option<()> {
        let script =
            crate::automexia::shell::prepare_bash_paste_batch(self.paste.text())?;
        // Legacy editors receive this as literal input. Never queue a control
        // character that could submit or edit the shell line on their behalf.
        if script.bytes().any(|byte| !matches!(byte, b' '..=b'~')) {
            return None;
        }
        let mut paste = PasteRequest::from_clipboard(script)?;
        paste.guard_clipboard_review(self.destination);
        self.lines = LineIndex::new(paste.text());
        self.paste = paste;
        self.batch_review = BatchReview::Prepared;
        self.selected = Choice::Paste;
        self.hovered = None;
        self.pressed_button = None;
        self.held_enter = None;
        self.wheel_remainder = (0.0, 0.0);
        Some(())
    }
}

fn initial_batch_review(mode: Mode, shell: PromptInputShell) -> BatchReview {
    let bracketed = mode.contains(Mode::BRACKETED_PASTE);
    if mode.intersects(Mode::ALT_SCREEN | Mode::VI)
        || shell == PromptInputShell::Cmd
        || (!bracketed && mode.contains(Mode::WIN32_INPUT))
    {
        BatchReview::Literal
    } else {
        // The explicit Bash action also works in a normal legacy line editor.
        // Do not guess the running shell from a profile or from a prompt string.
        BatchReview::Available { bracketed }
    }
}

impl Screen<'_> {
    pub(super) fn open_paste_preview(
        &mut self,
        target: PasteTarget,
        destination: PasteDestination,
        mut paste: PasteRequest,
        mode: Mode,
        shell: PromptInputShell,
    ) {
        paste.guard_clipboard_review(destination);
        let lines = LineIndex::for_shell(paste.text(), shell);
        if self.mouse.left_button_state == ElementState::Pressed
            || self.renderer.scrollbar.is_dragging()
            || self.resize_state.is_some()
        {
            self.mouse.capture_left_for_overlay();
            self.renderer.scrollbar.end_drag();
            self.resize_state = None;
        }
        self.paste_preview = Some(PastePreview {
            target,
            destination,
            paste,
            lines,
            batch_review: initial_batch_review(mode, shell),
            selected: Choice::default(),
            hovered: None,
            pressed_button: None,
            held_enter: None,
            wheel_remainder: (0.0, 0.0),
        });
        self.cancel_terminal_composition();
        self.dismiss_suggestions(
            crate::automexia::suggestions::SuggestionInvalidation::ModalOpened,
        );
        self.context_manager.request_render();
    }

    fn dismiss_paste_preview(&mut self) {
        if self.paste_preview.take().is_some() {
            self.mark_dirty();
        }
    }

    fn complete_paste_preview(&mut self, choice: Choice) {
        if choice == Choice::BashBatch {
            let context = self.context_manager.current();
            let prepared = self.paste_preview.as_mut().is_some_and(|preview| {
                if !preview.batch_review.is_available()
                    || !preview.target.matches(context)
                {
                    return false;
                }
                let current = preview.paste.is_for_terminal(&context.terminal.lock());
                current && preview.prepare_bash_batch().is_some()
            });
            if prepared {
                self.mark_dirty();
            } else {
                self.dismiss_paste_preview();
                self.report_paste_rejected();
            }
            return;
        }

        if let Some(preview) = self.paste_preview.take() {
            self.mark_dirty();
            if choice == Choice::Paste
                && preview.target.matches(self.context_manager.current())
                && self
                    .context_manager
                    .deliver_paste_request(preview.target, preview.paste)
                    .is_err()
            {
                self.report_paste_rejected();
            }
        }
    }

    fn paste_preview_dimensions(&self) -> (f32, f32, f32) {
        let size = self.sugarloaf.window_size();
        (size.width, size.height, self.sugarloaf.scale_factor())
    }

    fn discard_stale_paste_preview(&mut self) {
        if self.paste_preview.as_ref().is_some_and(|preview| {
            !preview.target.matches(self.context_manager.current())
        }) || self.renderer.confirm_quit.is_active()
        {
            self.dismiss_paste_preview();
        }
    }

    pub(super) fn handle_paste_preview_key(&mut self, event: &KeyEvent) -> bool {
        self.discard_stale_paste_preview();
        if self.paste_preview.is_none() {
            return false;
        }
        let dimensions = self.paste_preview_dimensions();
        let shift = self.modifiers.state().shift_key();
        let Some(preview) = &mut self.paste_preview else {
            return false;
        };
        let layout = preview.layout(dimensions);

        if event.state == ElementState::Pressed
            && !self.overlay_key_releases.contains(&event.physical_key)
            && self.overlay_key_releases.len() < 512
        {
            self.overlay_key_releases.push(event.physical_key);
        }
        if event.state == ElementState::Released {
            let choice = take_enter_choice(&mut preview.held_enter, event.physical_key)
                .filter(|choice| layout.is_enabled(*choice));
            let released_overlay_key = self.consume_overlay_key_release(event);
            #[cfg(windows)]
            if !released_overlay_key {
                // The shortcut that opened the modal may already belong to
                // the ordinary Windows shortcut path.
                self.consumed_win32_key_releases
                    .take_release(&event.physical_key);
            }
            #[cfg(not(windows))]
            let _ = released_overlay_key;
            if let Some(choice) = choice {
                self.complete_paste_preview(choice);
            }
            return true;
        }

        let page = layout.page_size;
        let horizontal_step = if shift { 32 } else { 8 };
        match &event.logical_key {
            Key::Named(NamedKey::Escape) if !event.repeat => self.dismiss_paste_preview(),
            Key::Named(NamedKey::Enter) if !event.repeat => {
                preview.held_enter =
                    Some((event.physical_key, layout.focused_choice(preview.selected)));
            }
            Key::Named(NamedKey::Tab) if !event.repeat => {
                preview.held_enter = None;
                preview.selected = layout.next_choice(preview.selected, shift);
            }
            Key::Named(NamedKey::ArrowLeft) => {
                preview.lines.scroll_horizontal(-horizontal_step)
            }
            Key::Named(NamedKey::ArrowRight) => {
                preview.lines.scroll_horizontal(horizontal_step)
            }
            Key::Named(NamedKey::ArrowUp) => preview.lines.scroll(-1, page),
            Key::Named(NamedKey::ArrowDown) => preview.lines.scroll(1, page),
            Key::Named(NamedKey::PageUp) => preview.lines.scroll(-(page as isize), page),
            Key::Named(NamedKey::PageDown) => preview.lines.scroll(page as isize, page),
            Key::Named(NamedKey::Home) => preview.lines.scroll(isize::MIN, page),
            Key::Named(NamedKey::End) => preview.lines.scroll(isize::MAX, page),
            _ => return true,
        }
        self.mark_dirty();
        true
    }

    pub(crate) fn handle_paste_preview_window_event(
        &mut self,
        event: &WindowEvent,
    ) -> bool {
        if self.paste_preview.is_none() {
            return false;
        }
        if let WindowEvent::KeyboardInput { event, .. } = event {
            return self.handle_paste_preview_key(event);
        }
        self.discard_stale_paste_preview();
        let dimensions = self.paste_preview_dimensions();
        let Some(preview) = &mut self.paste_preview else {
            return false;
        };
        let layout = preview.layout(dimensions);
        match event {
            WindowEvent::Focused(false) | WindowEvent::CloseRequested => {
                self.dismiss_paste_preview();
                return false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.x = position.x;
                self.mouse.y = position.y;
                self.mouse.raw_y = position.y;
                self.mouse.last_cell = None;
                let hovered = layout.hit((position.x as f32, position.y as f32));
                if preview.hovered != hovered {
                    preview.hovered = hovered;
                    self.mark_dirty();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                preview.pressed_button = None;
                if preview.hovered.take().is_some() {
                    self.mark_dirty();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse.palette_consumes_button(*button, *state, true);
                if *button == MouseButton::Left {
                    let hovered = layout.hit((self.mouse.x as f32, self.mouse.y as f32));
                    if *state == ElementState::Pressed {
                        preview.pressed_button = hovered;
                    } else if let Some(choice) = preview
                        .pressed_button
                        .take()
                        .filter(|choice| Some(*choice) == hovered)
                    {
                        self.complete_paste_preview(choice);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (f64::from(*x), f64::from(*y)),
                    MouseScrollDelta::PixelDelta(position) => {
                        let stride = f64::from(dimensions.2.max(0.1) * 24.0);
                        (position.x / stride, position.y / stride)
                    }
                };
                let x = wheel_steps(&mut preview.wheel_remainder.0, x);
                let y = wheel_steps(&mut preview.wheel_remainder.1, y);
                if x != 0 || y != 0 {
                    preview.lines.scroll_horizontal(-x * 8);
                    preview.lines.scroll(-y, layout.page_size);
                    self.mark_dirty();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.set_modifiers(*modifiers),
            WindowEvent::Ime(_) | WindowEvent::DroppedFile(_) | WindowEvent::Touch(_) => {
            }
            _ => return false,
        }
        true
    }

    pub(super) fn render_paste_preview(&mut self) {
        self.discard_stale_paste_preview();
        if let Some(preview) = &self.paste_preview {
            preview.layout(self.paste_preview_dimensions()).render(
                &mut self.sugarloaf,
                UiTheme::from_colors(&self.renderer.named_colors),
                &preview.lines,
                preview.paste.text(),
                preview.selected,
                preview.hovered,
            );
        }
    }

    pub(super) fn paste_preview_accessibility(
        &self,
    ) -> Option<automexia_ui_model::accessibility::Surface> {
        let preview = self.paste_preview.as_ref()?;
        if !preview.target.matches(self.context_manager.current()) {
            return None;
        }
        let dimensions = self.paste_preview_dimensions();
        Some(preview.layout(dimensions).accessibility_surface(
            dimensions,
            &preview.lines,
            preview.paste.text(),
            preview.selected,
        ))
    }
}

fn take_enter_choice(
    held: &mut Option<(PhysicalKey, Choice)>,
    released: PhysicalKey,
) -> Option<Choice> {
    if held.is_some_and(|(pressed, _)| pressed == released) {
        held.take().map(|(_, choice)| choice)
    } else {
        None
    }
}

fn wheel_steps(remainder: &mut f64, delta: f64) -> isize {
    let total = *remainder + delta;
    *remainder = if total.is_finite() {
        total.fract()
    } else {
        0.0
    };
    (total as isize).clamp(-32, 32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rio_window::keyboard::KeyCode;

    #[test]
    fn paste_preview_requires_the_matching_fresh_enter_release() {
        let enter = PhysicalKey::Code(KeyCode::Enter);
        let other = PhysicalKey::Code(KeyCode::KeyA);
        let mut held = None;
        assert_eq!(take_enter_choice(&mut held, enter), None);
        held = Some((enter, Choice::Paste));
        assert_eq!(take_enter_choice(&mut held, other), None);
        assert_eq!(take_enter_choice(&mut held, enter), Some(Choice::Paste));
        assert_eq!(take_enter_choice(&mut held, enter), None);
    }

    #[test]
    fn paste_preview_resize_cannot_retarget_an_enter_that_started_on_cancel() {
        let enter = PhysicalKey::Code(KeyCode::Enter);
        let batch_review = BatchReview::Available { bracketed: true };
        let compact = Layout::new((100.0, 80.0, 1.0), 3, batch_review);
        let expanded = Layout::new((900.0, 400.0, 1.0), 3, batch_review);
        let mut held = Some((enter, compact.focused_choice(Choice::Paste)));
        assert!(expanded.is_enabled(Choice::Paste));
        assert_eq!(take_enter_choice(&mut held, enter), Some(Choice::Cancel));
    }

    #[test]
    fn bash_batch_paste_prepares_a_second_review_before_queuing_any_input() {
        use crate::context::ContextManager;
        use crate::event::{Msg, VoidListener};
        use crate::messenger::Messenger;
        use rio_backend::event::WindowId;

        let mut manager =
            ContextManager::start_with_capacity(1, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        let context = manager.current();
        let destination = {
            let terminal = context.terminal.lock();
            assert!(!terminal.mode().contains(Mode::BRACKETED_PASTE));
            terminal.paste_destination()
        };
        let text = "echo first\ncommand __fixture_missing\necho last";
        let mut paste = PasteRequest::from_clipboard(text.into()).unwrap();
        paste.guard_clipboard_review(destination);
        let mut preview = PastePreview {
            target: context.paste_target(),
            destination,
            paste,
            lines: LineIndex::new(text),
            batch_review: BatchReview::Available { bracketed: false },
            selected: Choice::BashBatch,
            hovered: Some(Choice::BashBatch),
            pressed_button: Some(Choice::BashBatch),
            held_enter: None,
            wheel_remainder: (0.0, 0.0),
        };
        preview.prepare_bash_batch().unwrap();
        assert_eq!(preview.batch_review, BatchReview::Prepared);
        assert_eq!(preview.selected, Choice::Paste);
        assert_eq!(preview.pressed_button, None);
        assert!(receiver.try_recv().is_err());
        let reviewed = preview.paste.text().to_owned();
        assert!(reviewed.contains("echo first\\ncommand __fixture_missing\\necho last"));
        assert!(reviewed.bytes().all(|byte| matches!(byte, b' '..=b'~')));
        assert_eq!(
            manager.deliver_paste_request(preview.target, preview.paste),
            Ok(true)
        );
        let Msg::Paste(queued) = receiver.try_recv().unwrap() else {
            panic!("expected one paste transaction");
        };
        assert_eq!(queued.text(), reviewed);
        assert!(queued.is_bracketed_request());
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn bash_batch_review_preserves_terminal_editor_ownership() {
        let shell = PromptInputShell::Posix;
        assert_eq!(
            initial_batch_review(Mode::empty(), shell),
            BatchReview::Available { bracketed: false }
        );
        assert_eq!(
            initial_batch_review(Mode::BRACKETED_PASTE | Mode::WIN32_INPUT, shell),
            BatchReview::Available { bracketed: true }
        );
        for mode in [Mode::ALT_SCREEN, Mode::VI, Mode::WIN32_INPUT] {
            assert_eq!(initial_batch_review(mode, shell), BatchReview::Literal);
        }
        assert_eq!(
            initial_batch_review(Mode::BRACKETED_PASTE, PromptInputShell::Cmd),
            BatchReview::Literal
        );
    }

    #[test]
    fn paste_preview_accumulates_trackpad_motion_without_unbounded_scroll() {
        let mut remainder = 0.0;
        assert_eq!(wheel_steps(&mut remainder, 0.25), 0);
        assert_eq!(wheel_steps(&mut remainder, 0.5), 0);
        assert_eq!(wheel_steps(&mut remainder, 0.5), 1);
        assert_eq!(wheel_steps(&mut remainder, -1e6), -32);
    }
}
