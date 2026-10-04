//! Paste delivery belongs to an exact existing terminal, never to later focus.

use super::{Context, ContextManager};
use crate::event::{Msg, PasteRequest};
use rio_backend::crosswords::grid::Scroll;
use rio_backend::event::EventListener;
use std::sync::atomic::Ordering;

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
    pub(crate) fn deliver_action_insert(
        &mut self,
        target: PasteTarget,
        text: &str,
    ) -> Result<bool, PasteError> {
        let paste =
            PasteRequest::reviewed_action_insert(text).ok_or(PasteError::TooLarge)?;
        self.deliver_paste_request(target, paste, None)
    }
    pub(crate) fn deliver_workflow_command(
        &mut self,
        target: PasteTarget,
        text: &str,
        receipt: rio_backend::crosswords::command_actions::WorkflowPrompt,
    ) -> Result<bool, PasteError> {
        let paste = PasteRequest::reviewed_workflow_command(text, receipt)
            .ok_or(PasteError::TooLarge)?;
        self.deliver_paste_request(target, paste, Some(receipt))
    }

    pub(crate) fn deliver_paste(
        &mut self,
        target: PasteTarget,
        text: &str,
        bracketed: bool,
    ) -> Result<bool, PasteError> {
        let paste = PasteRequest::new(text, bracketed).ok_or(PasteError::TooLarge)?;
        self.deliver_paste_request(target, paste, None)
    }

    fn deliver_paste_request(
        &mut self,
        target: PasteTarget,
        paste: PasteRequest,
        receipt: Option<rio_backend::crosswords::command_actions::WorkflowPrompt>,
    ) -> Result<bool, PasteError> {
        let context = self
            .get_by_route_id(target.route_id)
            .filter(|context| context.rich_text_id == target.rich_text_id)
            .ok_or(PasteError::StaleTarget)?;
        if context.shutdown_requested.load(Ordering::Acquire) {
            return Err(PasteError::Closed);
        }
        if paste.text().is_empty() {
            return Ok(false);
        }
        let mut terminal = context.terminal.lock();
        if receipt.is_some_and(|receipt| terminal.workflow_prompt() != Some(receipt)) {
            return Err(PasteError::StaleTarget);
        }
        // One queue message prevents resize/keyboard producers from entering
        // between bracket delimiters. A disconnected receiver changes no UI state.
        context
            .messenger
            .channel
            .send(Msg::Paste(paste))
            .map_err(|_| PasteError::Closed)?;
        // Keep selection and scroll mutation under the same lock as publication.
        // Mode-dependent encoding occurs only in the receiving PTY worker.
        terminal.scroll_display(Scroll::Bottom);
        terminal.selection.take();
        drop(terminal);
        context.renderable_content.command_rows.follow();
        context
            .renderable_content
            .pending_update
            .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
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

    fn scrolled_header(content: &mut crate::context::renderable::RenderableContent) {
        content.command_rows.rebuild(3, &[(0, 5)]);
        assert_eq!(content.command_rows.scroll(-2, 0, 0), 0);
        content.command_rows.rebuild(3, &[(0, 5)]);
        content.command_rows.settle(None, 3);
        assert_eq!(content.command_rows.top(), 2);
    }

    #[test]
    fn paste_request_bound_is_independent_of_shell_identity() {
        for bracketed in [false, true] {
            for size in [PasteRequest::MAX_BYTES - 1, PasteRequest::MAX_BYTES] {
                let text = "x".repeat(size);
                assert_eq!(PasteRequest::new(&text, bracketed).unwrap().text(), text);
            }
            assert!(PasteRequest::new(
                &"x".repeat(PasteRequest::MAX_BYTES + 1),
                bracketed
            )
            .is_none());
            assert!(PasteRequest::new(
                &"界".repeat(PasteRequest::MAX_BYTES / 3 + 1),
                bracketed
            )
            .is_none());
        }
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
            manager.deliver_paste(target, &"x".repeat(PasteRequest::MAX_BYTES + 1), true),
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
        scrolled_header(&mut manager.current_mut().renderable_content);
        let projection = manager.current().renderable_content.command_rows.clone();
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
        assert_eq!(
            manager.current().renderable_content.command_rows,
            projection
        );
    }

    #[test]
    fn bracketed_paste_is_one_exact_transaction_to_the_captured_owner() {
        let mut manager =
            ContextManager::start_with_capacity(3, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        let target = manager.current().paste_target();
        let target_route = target.route_id;
        scrolled_header(&mut manager.current_mut().renderable_content);
        manager.add_context(true, 0);
        scrolled_header(&mut manager.current_mut().renderable_content);
        let (sibling_sender, sibling_receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sibling_sender);

        // A clipboard reply may arrive after focus changes. The exact route
        // receives one semantic transaction; only its PTY worker may encode it.
        assert_eq!(
            manager.deliver_paste(target, "one\r\ntwo\x1b[201~\x03", true),
            Ok(true)
        );
        let Msg::Paste(paste) = receiver.try_recv().unwrap() else {
            panic!("expected PTY paste")
        };
        assert_eq!(paste.text(), "one\r\ntwo\x1b[201~\x03");
        assert!(paste.is_bracketed_request());
        assert!(receiver.try_recv().is_err());
        assert!(sibling_receiver.try_recv().is_err());
        assert_eq!(manager.current().renderable_content.command_rows.top(), 2);
        assert_eq!(
            manager
                .get_by_route_id(target_route)
                .unwrap()
                .renderable_content
                .command_rows
                .top(),
            0
        );
    }
}
