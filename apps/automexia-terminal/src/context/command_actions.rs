//! Route-bound command reinsertion through the existing single-message paste path.
use super::{Context, ContextManager};
use crate::event::{Msg, PasteRequest};
use rio_backend::crosswords::{
    command_actions::{CommandActionError, CommandHandle},
    grid::Scroll,
};
use rio_backend::event::EventListener;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CommandTarget {
    pub route_id: usize,
    pub rich_text_id: usize,
    pub handle: CommandHandle,
}
impl<T: EventListener> Context<T> {
    pub(crate) fn command_target(&self) -> Option<CommandTarget> {
        if self.shutdown_requested.load(Ordering::Acquire) {
            return None;
        }
        self.terminal
            .lock()
            .last_command_handle()
            .map(|handle| CommandTarget {
                route_id: self.route_id,
                rich_text_id: self.rich_text_id,
                handle,
            })
    }
    pub(crate) fn matches_command_target(&self, target: CommandTarget) -> bool {
        self.route_id == target.route_id
            && self.rich_text_id == target.rich_text_id
            && !self.shutdown_requested.load(Ordering::Acquire)
    }
}
impl<T: EventListener + Clone + Send + 'static> ContextManager<T> {
    pub(crate) fn reinsert_command(
        &mut self,
        target: CommandTarget,
    ) -> Result<(), CommandActionError> {
        let context = self.current_mut();
        if !context.matches_command_target(target) {
            return Err(CommandActionError::Stale);
        }
        let mut terminal = context.terminal.lock();
        // Validation and queue publication share the grid lock. PTY metadata
        // cannot replace this editable prompt between the check and enqueue.
        if terminal.mode().contains(rio_backend::crosswords::Mode::VI) {
            return Err(CommandActionError::Busy);
        }
        let text = terminal.command_text_for_reinsert(target.handle)?;
        let paste = PasteRequest::new(&text, true).ok_or(CommandActionError::Limit)?;
        context
            .messenger
            .channel
            .send(Msg::Paste(paste))
            .map_err(|_| CommandActionError::Busy)?;
        terminal.scroll_display(Scroll::Bottom);
        terminal.selection.take();
        drop(terminal);
        context.renderable_content.command_rows.follow();
        context
            .renderable_content
            .pending_update
            .set_terminal_damage(rio_backend::event::TerminalDamage::Full);
        context.set_selection(None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::VoidListener;
    use crate::messenger::Messenger;
    use rio_backend::crosswords::{
        pos::{Column, Line, Pos, Side},
        CrosswordsSize,
    };
    use rio_backend::event::WindowId;
    use rio_backend::performer::handler::Processor;
    use rio_backend::selection::{Selection, SelectionType};

    fn setup() -> (
        ContextManager<VoidListener>,
        corcovado::channel::Receiver<Msg>,
        CommandTarget,
    ) {
        let mut manager =
            ContextManager::start_with_capacity(3, VoidListener {}, WindowId::from(0))
                .unwrap();
        let (sender, receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        {
            let mut t = manager.current().terminal.lock();
            t.resize(CrosswordsSize::new(80, 12));
            Processor::default().advance(&mut *t, b"\x1b]133;A;aid=1\x07> \x1b]133;B\x07echo hello\r\n\x1b]133;C\x07hello\r\n\x1b]133;D;0\x07\x1b]133;A;aid=2\x07> \x1b]133;B\x07");
            t.selection = Some(Selection::new(
                SelectionType::Simple,
                Pos::new(Line(0), Column(0)),
                Side::Left,
            ));
        }
        let target = manager.current().command_target().unwrap();
        (manager, receiver, target)
    }
    #[test]
    fn command_actions_reinsert_one_paste_without_execution() {
        let (mut manager, receiver, target) = setup();
        assert_eq!(manager.reinsert_command(target), Ok(()));
        let Msg::Paste(paste) = receiver.try_recv().unwrap() else {
            panic!("expected one paste");
        };
        assert_eq!(paste.text(), "echo hello");
        assert!(paste.is_bracketed_request());
        assert!(!paste.text().contains(['\r', '\n']));
        assert!(receiver.try_recv().is_err());
        assert!(manager.current().terminal.lock().selection.is_none());
    }
    #[test]
    fn command_actions_reinsert_rejects_replaced_closed_busy_or_cleared_targets() {
        for change in 0..6 {
            let (mut manager, receiver, target) = setup();
            match change {
                0 => manager.current_mut().route_id += 100,
                1 => manager.current_mut().rich_text_id += 1,
                2 => manager
                    .current()
                    .shutdown_requested
                    .store(true, Ordering::Release),
                3 => Processor::default()
                    .advance(&mut *manager.current().terminal.lock(), b"pending"),
                4 => manager.current().terminal.lock().clear_screen_and_history(),
                _ => manager.current().terminal.lock().toggle_vi_mode(),
            }
            assert!(manager.reinsert_command(target).is_err());
            assert!(receiver.try_recv().is_err());
            if change != 4 {
                assert!(manager.current().terminal.lock().selection.is_some());
            }
        }
    }
    #[test]
    fn command_actions_reinsert_never_follows_changed_focus_or_disconnected_channel() {
        let (mut manager, receiver, target) = setup();
        manager.add_context(true, 0);
        let (sender, sibling_receiver) = corcovado::channel::channel();
        manager.current_mut().messenger = Messenger::new(sender);
        assert_eq!(
            manager.reinsert_command(target),
            Err(CommandActionError::Stale)
        );
        assert!(receiver.try_recv().is_err());
        assert!(sibling_receiver.try_recv().is_err());
        let (mut manager, receiver, target) = setup();
        drop(receiver);
        assert_eq!(
            manager.reinsert_command(target),
            Err(CommandActionError::Busy)
        );
        assert!(manager.current().terminal.lock().selection.is_some());
    }
}
