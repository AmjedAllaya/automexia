//! Paste delivery belongs to an exact existing terminal, never to later focus.

use super::{Context, ContextManager};
use crate::event::Msg;
use rio_backend::crosswords::{grid::Scroll, Mode};
use rio_backend::event::EventListener;
use std::sync::atomic::Ordering;

/// Bound app-owned conversion and each queued transaction, not OS clipboard allocation.
pub(crate) const MAX_PASTE_BYTES: usize = 1024 * 1024;

pub(crate) fn prepare_paste(
    text: &str,
    bracketed: bool,
    mode: Mode,
) -> Result<Vec<u8>, PasteError> {
    if text.len() > MAX_PASTE_BYTES {
        return Err(PasteError::TooLarge);
    }
    if text.is_empty() {
        return Ok(Vec::new());
    }
    if bracketed && mode.contains(Mode::BRACKETED_PASTE) {
        let mut payload = Vec::with_capacity(text.len() + 12);
        payload.extend_from_slice(b"\x1b[200~");
        // Preserve UTF-8 and newlines; neither ESC nor ETX can terminate the
        // bracket from inside its untrusted body. Do not append an Enter.
        payload.extend(text.bytes().filter(|byte| !matches!(byte, 0x1b | 0x03)));
        payload.extend_from_slice(b"\x1b[201~");
        Ok(payload)
    } else if bracketed {
        Ok(text.replace("\r\n", "\r").replace('\n', "\r").into_bytes())
    } else {
        Ok(text.as_bytes().to_vec())
    }
}

/// Consumed on delivery; no clone/replay API. The second identity rejects a
/// replaced context even if a caller mistakenly reuses a route identifier.
pub(crate) struct PasteTarget {
    route_id: usize,
    rich_text_id: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PasteError {
    StaleTarget,
    Closed,
    TooLarge,
}

impl<T: EventListener> Context<T> {
    pub(crate) fn paste_target(&self) -> PasteTarget {
        PasteTarget {
            route_id: self.route_id,
            rich_text_id: self.rich_text_id,
        }
    }
}

impl<T: EventListener + Clone + Send + 'static> ContextManager<T> {
    pub(crate) fn deliver_paste(
        &mut self,
        target: PasteTarget,
        text: &str,
        bracketed: bool,
    ) -> Result<bool, PasteError> {
        let context = self
            .get_by_route_id(target.route_id)
            .filter(|context| context.rich_text_id == target.rich_text_id)
            .ok_or(PasteError::StaleTarget)?;
        if context.shutdown_requested.load(Ordering::Acquire) {
            return Err(PasteError::Closed);
        }
        let mut terminal = context.terminal.lock();
        let payload = prepare_paste(text, bracketed, terminal.mode())?;
        if payload.is_empty() {
            return Ok(false);
        }
        // One queue message prevents resize/keyboard producers from entering
        // between bracket delimiters. A disconnected receiver changes no UI state.
        context
            .messenger
            .channel
            .send(Msg::Input(payload.into()))
            .map_err(|_| PasteError::Closed)?;
        // Keep terminal mutation under the same lock as mode observation and
        // queue publication: the awakened PTY parser cannot publish first.
        terminal.scroll_display(Scroll::Bottom);
        terminal.selection.take();
        drop(terminal);
        context.set_selection(None);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::VoidListener;
    use crate::messenger::Messenger;
    use rio_backend::event::WindowId;

    #[test]
    fn paste_encoding_bounds_and_raw_input_are_independent_of_shell_identity() {
        for mode in [
            Mode::empty(),
            Mode::BRACKETED_PASTE,
            Mode::ALT_SCREEN | Mode::BRACKETED_PASTE,
        ] {
            for bracketed in [false, true] {
                assert_eq!(prepare_paste("", bracketed, mode), Ok(Vec::new()));
                for size in [MAX_PASTE_BYTES - 1, MAX_PASTE_BYTES] {
                    assert!(prepare_paste(&"x".repeat(size), bracketed, mode).is_ok());
                }
                assert_eq!(
                    prepare_paste(&"x".repeat(MAX_PASTE_BYTES + 1), bracketed, mode),
                    Err(PasteError::TooLarge)
                );
                assert_eq!(
                    prepare_paste(&"界".repeat(MAX_PASTE_BYTES / 3 + 1), bracketed, mode),
                    Err(PasteError::TooLarge)
                );
            }
            assert_eq!(
                prepare_paste("a\r\nb\nc\x1b\x03", false, mode).unwrap(),
                b"a\r\nb\nc\x1b\x03"
            );
        }
        assert_eq!(
            prepare_paste("a\r\nb\nc", true, Mode::empty()).unwrap(),
            b"a\rb\rc"
        );
        assert_eq!(
            prepare_paste("界e\u{301}\n", true, Mode::BRACKETED_PASTE).unwrap(),
            "\x1b[200~界e\u{301}\n\x1b[201~".as_bytes()
        );
    }

    #[test]
    fn rejected_paste_keeps_selection_and_never_falls_back_to_another_route() {
        use rio_backend::crosswords::pos::{Column, Line, Pos, Side};
        use rio_backend::selection::{Selection, SelectionType};
        let mut manager =
            ContextManager::start_with_capacity(3, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        let selection = Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(0)),
            Side::Left,
        );
        manager.current().terminal.lock().selection = Some(selection);
        let target = manager.current().paste_target();
        assert_eq!(
            manager.deliver_paste(target, &"x".repeat(MAX_PASTE_BYTES + 1), true),
            Err(PasteError::TooLarge)
        );
        assert!(manager.current().terminal.lock().selection.is_some());
        let target = manager.current().paste_target();
        assert_eq!(manager.deliver_paste(target, "", true), Ok(false));
        assert!(manager.current().terminal.lock().selection.is_some());
        let target = manager.current().paste_target();
        manager.current_mut().rich_text_id += 1;
        assert_eq!(
            manager.deliver_paste(target, "not sent", true),
            Err(PasteError::StaleTarget)
        );
        assert!(receiver.try_recv().is_err());
        let target = manager.current().paste_target();
        manager.current_mut().route_id += 100;
        assert_eq!(
            manager.deliver_paste(target, "not sent", true),
            Err(PasteError::StaleTarget)
        );
        let target = manager.current().paste_target();
        manager.current().request_pty_shutdown();
        assert_eq!(
            manager.deliver_paste(target, "not sent", true),
            Err(PasteError::Closed)
        );
        assert!(matches!(receiver.try_recv(), Ok(Msg::Shutdown)));
        assert!(receiver.try_recv().is_err());
        assert!(manager.current().terminal.lock().selection.is_some());
    }

    #[test]
    fn disconnected_paste_changes_no_selection() {
        use rio_backend::crosswords::pos::{Column, Line, Pos, Side};
        use rio_backend::selection::{Selection, SelectionType};
        let mut manager =
            ContextManager::start_with_capacity(1, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        drop(receiver);
        manager.current().terminal.lock().selection = Some(Selection::new(
            SelectionType::Simple,
            Pos::new(Line(0), Column(0)),
            Side::Left,
        ));
        let target = manager.current().paste_target();
        assert_eq!(
            manager.deliver_paste(target, "not sent", true),
            Err(PasteError::Closed)
        );
        assert!(manager.current().terminal.lock().selection.is_some());
    }

    #[test]
    fn bracketed_paste_is_one_exact_transaction_to_the_captured_owner() {
        let mut manager =
            ContextManager::start_with_capacity(3, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        let mut parser = rio_backend::performer::handler::Processor::default();
        parser.advance(&mut *manager.current().terminal.lock(), b"\x1b[?2004h");
        let target = manager.current().paste_target();
        manager.add_context(true, 0);
        let (sibling_sender, sibling_receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sibling_sender);

        // A clipboard reply may arrive after focus changes. Capture actual
        // queued bytes, not a mocked "sent" count or the current route.
        assert_eq!(
            manager.deliver_paste(target, "one\r\ntwo\x1b[201~\x03", true),
            Ok(true)
        );
        let Msg::Input(bytes) = receiver.try_recv().unwrap() else {
            panic!("expected PTY input")
        };
        assert_eq!(&*bytes, b"\x1b[200~one\r\ntwo[201~\x1b[201~");
        assert!(receiver.try_recv().is_err());
        assert!(sibling_receiver.try_recv().is_err());
    }
}
